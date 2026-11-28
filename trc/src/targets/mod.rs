//! Structured target specification for freestanding builds.
//!
//! Acceptance is decided by [`crate::freestanding`] so a triple accepted by
//! `--target` means the same thing everywhere. This module adds the fields a
//! build needs (width, byte order, boot protocol) without re-deriving validity
//! from the triple a second time.

pub use crate::freestanding::{
    validate_build_request, validate_panic_strategy, validate_target, FreestandingTarget,
    PanicStrategy, TargetError, TargetErrorKind, DEFAULT_ENTRY_SYMBOL, SUPPORTED_TRIPLES,
};

use std::fmt;

/// Byte order of values in the target's memory.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Endian {
    Little,
    Big,
}

impl Endian {
    /// Canonical lowercase spelling used in diagnostics.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Little => "little",
            Self::Big => "big",
        }
    }
}

impl fmt::Display for Endian {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A validated target description with the fields a freestanding image needs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TargetSpec {
    triple: String,
    arch: String,
    vendor: String,
    os: String,
    environment: Option<String>,
    pointer_width: u16,
    endian: Endian,
    panic: PanicStrategy,
}

impl TargetSpec {
    fn from_validated(triple: &str, panic: PanicStrategy) -> Self {
        let mut parts = triple.split('-');
        let arch = parts.next().unwrap_or("").to_string();
        let vendor = parts.next().unwrap_or("").to_string();
        let os = parts.next().unwrap_or("").to_string();
        let environment = parts.next().map(str::to_string);
        let pointer_width = if arch.contains("64") { 64 } else { 32 };
        // Validation currently admits only x86_64. The branch keeps the field
        // meaningful when another architecture gains a boot stub.
        let endian = if arch == "x86_64" {
            Endian::Little
        } else {
            Endian::Big
        };
        Self {
            triple: triple.to_string(),
            arch,
            vendor,
            os,
            environment,
            pointer_width,
            endian,
            panic,
        }
    }

    /// Validated triple this spec was built from.
    pub fn triple(&self) -> &str {
        &self.triple
    }

    /// Architecture component, for example `x86_64`.
    pub fn architecture(&self) -> &str {
        &self.arch
    }

    /// Vendor component, for example `unknown`.
    pub fn vendor(&self) -> &str {
        &self.vendor
    }

    /// Operating-system component; always `none` after validation.
    pub fn operating_system(&self) -> &str {
        &self.os
    }

    /// Optional environment component, for example `elf`.
    pub fn environment(&self) -> Option<&str> {
        self.environment.as_deref()
    }

    /// Pointer width in bits.
    pub fn pointer_width(&self) -> u16 {
        self.pointer_width
    }

    /// Memory byte order.
    pub fn endian(&self) -> Endian {
        self.endian
    }

    /// Panic strategy; always abort after validation.
    pub fn panic_strategy(&self) -> PanicStrategy {
        self.panic
    }

    /// Symbol the loader transfers control to.
    pub fn entry_symbol(&self) -> &'static str {
        DEFAULT_ENTRY_SYMBOL
    }

    /// Linker script that lays out this image.
    pub fn linker_script(&self) -> &'static str {
        "x86_64-none.ld"
    }

    /// Boot protocol the stub implements.
    pub fn boot_protocol(&self) -> &'static str {
        "multiboot2"
    }
}

impl fmt::Display for TargetSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({} endian, {} bit, boot {})",
            self.triple,
            self.endian,
            self.pointer_width,
            self.boot_protocol()
        )
    }
}

/// Validate a triple and describe it. Panic strategy defaults to abort.
pub fn parse(triple: &str) -> Result<TargetSpec, TargetError> {
    validate_target(triple)?;
    Ok(TargetSpec::from_validated(triple, PanicStrategy::Abort))
}

/// Validate a complete build request and describe it.
pub fn parse_build_request(
    triple: &str,
    panic: PanicStrategy,
) -> Result<TargetSpec, TargetError> {
    validate_build_request(triple, panic)?;
    Ok(TargetSpec::from_validated(triple, panic))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_triples_produce_usable_specs() {
        for triple in SUPPORTED_TRIPLES {
            let spec = parse(triple).expect("supported triple must produce a spec");
            assert_eq!(spec.triple(), *triple);
            assert_eq!(spec.architecture(), "x86_64");
            assert_eq!(spec.vendor(), "unknown");
            assert_eq!(spec.operating_system(), "none");
            assert_eq!(spec.pointer_width(), 64);
            assert_eq!(spec.endian(), Endian::Little);
            assert_eq!(spec.panic_strategy(), PanicStrategy::Abort);
            assert_eq!(spec.entry_symbol(), "_start");
            assert_eq!(spec.linker_script(), "x86_64-none.ld");
            assert_eq!(spec.boot_protocol(), "multiboot2");
        }
    }

    #[test]
    fn environment_component_is_preserved() {
        let spec = parse("x86_64-unknown-none-elf").expect("elf triple is supported");
        assert_eq!(spec.environment(), Some("elf"));
        assert_eq!(parse("x86_64-unknown-none").unwrap().environment(), None);
    }

    #[test]
    fn unsupported_targets_carry_the_same_precise_errors() {
        let hosted = parse("x86_64-unknown-linux-gnu").unwrap_err();
        assert_eq!(hosted.kind(), TargetErrorKind::HostedTarget);
        assert!(hosted.reason().contains("'linux'"));

        let arch = parse("aarch64-unknown-none").unwrap_err();
        assert_eq!(arch.kind(), TargetErrorKind::UnsupportedArchitecture);

        let env = parse("x86_64-unknown-none-uefi").unwrap_err();
        assert_eq!(env.kind(), TargetErrorKind::UnsupportedComponent);
    }

    #[test]
    fn build_request_rejects_unwind_without_touching_the_spec() {
        let err = parse_build_request("x86_64-unknown-none", PanicStrategy::Unwind).unwrap_err();
        assert_eq!(err.kind(), TargetErrorKind::UnsupportedOption);
        assert_eq!(err.option(), "-C panic");
        let spec = parse_build_request("x86_64-unknown-none", PanicStrategy::Abort)
            .expect("abort is the supported strategy");
        assert_eq!(spec.panic_strategy(), PanicStrategy::Abort);
    }

    #[test]
    fn spec_display_names_triple_and_boot_protocol() {
        let spec = parse("x86_64-unknown-none").unwrap();
        let text = spec.to_string();
        assert!(text.starts_with("x86_64-unknown-none ("));
        assert!(text.contains("multiboot2"));
        assert_eq!(Endian::Little.as_str(), "little");
        assert_eq!(Endian::Big.to_string(), "big");
    }

    #[test]
    fn reexported_validation_is_the_canonical_one() {
        let via_module = validate_target("x86_64-unknown-none").unwrap();
        let via_spec = parse("x86_64-unknown-none").unwrap();
        assert_eq!(via_module.triple(), via_spec.triple());
        let direct = FreestandingTarget::parse("x86_64-unknown-none").unwrap();
        assert_eq!(direct.triple(), via_spec.triple());
    }
}
