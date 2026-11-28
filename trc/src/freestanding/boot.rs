//! Host-side preflight for a freestanding boot image.
//!
//! Firmware rejects a multiboot2 image silently when the header checksum is
//! wrong or the entry is misaligned. These checks therefore run before linking
//! instead of after a boot that produces no output.

use super::runtime::{
    multiboot2_checksum, Multiboot2Header, MULTIBOOT2_ARCHITECTURE_I386, MULTIBOOT2_HEADER_LEN,
    MULTIBOOT2_MAGIC,
};
use crate::targets::{parse, PanicStrategy};
use super::{validate_build_request, TargetError};

/// Required alignment of the multiboot2 header inside the boot section.
pub const MULTIBOOT2_HEADER_ALIGN: u64 = 8;
/// Stack alignment the `_start` stub establishes before calling the kernel.
pub const ENTRY_STACK_ALIGN: u64 = 16;
/// Entry address used when the caller does not supply one yet.
pub const DEFAULT_ENTRY_ADDRESS: u64 = 0x10_0000;

/// One named preflight result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BootCheck {
    name: &'static str,
    passed: bool,
    detail: String,
}

impl BootCheck {
    fn new(name: &'static str, passed: bool, detail: impl Into<String>) -> Self {
        Self {
            name,
            passed,
            detail: detail.into(),
        }
    }

    /// Stable identifier of the check.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Whether the check passed.
    pub fn passed(&self) -> bool {
        self.passed
    }

    /// Human-readable evidence for the result.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

/// An image configuration that has already passed target validation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BootChecklist {
    triple: String,
    entry_address: u64,
}

impl BootChecklist {
    /// Validate a target and start a checklist at the default entry address.
    pub fn new(triple: &str) -> Result<Self, TargetError> {
        validate_build_request(triple, PanicStrategy::Abort)?;
        Ok(Self {
            triple: triple.to_string(),
            entry_address: DEFAULT_ENTRY_ADDRESS,
        })
    }

    /// Override the entry address used by the alignment check.
    pub fn with_entry(mut self, address: u64) -> Self {
        self.entry_address = address;
        self
    }

    /// Validated triple under test.
    pub fn target(&self) -> &str {
        &self.triple
    }

    /// Entry address under test.
    pub fn entry_address(&self) -> u64 {
        self.entry_address
    }

    /// Run every check in a stable order.
    pub fn run(&self) -> Vec<BootCheck> {
        vec![
            self.header_checksum(),
            self.header_layout(),
            self.entry_alignment(),
            self.target_config(),
        ]
    }

    /// Whether every check passed.
    pub fn all_passed(&self) -> bool {
        self.run().iter().all(BootCheck::passed)
    }

    /// Only the checks that failed, in stable order.
    pub fn failures(&self) -> Vec<BootCheck> {
        self.run().into_iter().filter(|check| !check.passed).collect()
    }

    fn header_checksum(&self) -> BootCheck {
        let sum = MULTIBOOT2_MAGIC
            .wrapping_add(MULTIBOOT2_ARCHITECTURE_I386)
            .wrapping_add(MULTIBOOT2_HEADER_LEN)
            .wrapping_add(multiboot2_checksum());
        BootCheck::new(
            "multiboot2-header-checksum",
            sum == 0,
            format!("header fields sum to {:#010x}", sum),
        )
    }

    fn header_layout(&self) -> BootCheck {
        let size = core::mem::size_of::<Multiboot2Header>() as u32;
        let align = MULTIBOOT2_HEADER_ALIGN as u32;
        let passed = size == MULTIBOOT2_HEADER_LEN && MULTIBOOT2_HEADER_LEN.is_multiple_of(align);
        BootCheck::new(
            "multiboot2-header-layout",
            passed,
            format!("struct size {} bytes, declared length {}", size, MULTIBOOT2_HEADER_LEN),
        )
    }

    fn entry_alignment(&self) -> BootCheck {
        let remainder = self.entry_address % ENTRY_STACK_ALIGN;
        BootCheck::new(
            "entry-alignment",
            remainder == 0,
            format!(
                "entry {:#x} has remainder {:#x} modulo {}",
                self.entry_address, remainder, ENTRY_STACK_ALIGN
            ),
        )
    }

    fn target_config(&self) -> BootCheck {
        match parse(&self.triple) {
            Ok(spec) => {
                let passed = spec.entry_symbol() == "_start"
                    && spec.pointer_width() == 64
                    && spec.boot_protocol() == "multiboot2"
                    && spec.panic_strategy() == PanicStrategy::Abort;
                BootCheck::new(
                    "target-config",
                    passed,
                    format!(
                        "{} enters at {}, {} bit, boot {}",
                        spec.triple(),
                        spec.entry_symbol(),
                        spec.pointer_width(),
                        spec.boot_protocol()
                    ),
                )
            }
            Err(err) => BootCheck::new("target-config", false, err.diagnostic()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_target_passes_every_check() {
        let checklist = BootChecklist::new("x86_64-unknown-none")
            .expect("supported target must pass validation");
        let checks = checklist.run();
        assert_eq!(checks.len(), 4);
        assert!(checklist.all_passed(), "{:?}", checklist.failures());
        for check in checks {
            assert!(check.passed(), "{}: {}", check.name(), check.detail());
            assert!(!check.name().is_empty());
        }
    }

    #[test]
    fn elf_environment_target_passes_every_check() {
        let checklist = BootChecklist::new("x86_64-unknown-none-elf")
            .expect("elf target must pass validation");
        assert!(checklist.all_passed(), "{:?}", checklist.failures());
    }

    #[test]
    fn misaligned_entry_fails_only_the_alignment_check() {
        let checklist = BootChecklist::new("x86_64-unknown-none")
            .unwrap()
            .with_entry(DEFAULT_ENTRY_ADDRESS + 1);
        let failures = checklist.failures();
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].name(), "entry-alignment");
        assert!(failures[0].detail().contains("remainder"));
        assert!(!checklist.all_passed());
    }

    #[test]
    fn checksum_detects_a_corrupted_header() {
        let corrupted = MULTIBOOT2_MAGIC
            .wrapping_add(MULTIBOOT2_ARCHITECTURE_I386)
            .wrapping_add(MULTIBOOT2_HEADER_LEN)
            .wrapping_add(multiboot2_checksum().wrapping_add(1));
        assert_ne!(corrupted, 0);
    }

    #[test]
    fn unsupported_targets_are_rejected_before_a_checklist_exists() {
        let err = BootChecklist::new("x86_64-unknown-linux-gnu").unwrap_err();
        assert_eq!(err.kind(), super::super::TargetErrorKind::HostedTarget);
        assert_eq!(err.option(), "--target");
    }

    #[test]
    fn checklist_accessors_round_trip() {
        let checklist = BootChecklist::new("x86_64-unknown-none")
            .unwrap()
            .with_entry(0x20_0000);
        assert_eq!(checklist.target(), "x86_64-unknown-none");
        assert_eq!(checklist.entry_address(), 0x20_0000);
        assert_eq!(ENTRY_STACK_ALIGN, 16);
        assert_eq!(MULTIBOOT2_HEADER_ALIGN, 8);
    }
}
