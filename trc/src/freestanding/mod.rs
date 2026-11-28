//! Freestanding substrate for Phase 14.
//!
//! The host half validates a `--target` request, describes the serial and value
//! model the image uses, and runs a boot checklist before linking. The
//! core-only half lives in [`runtime`], [`alloc`], [`arch`], and [`idt`]; those
//! four modules are the template for the separate `titrate_rt` crate and use
//! `core` only. The stub never claims to boot a kernel: its entry writes a
//! status line and halts until a kernel loader exists.
//!
//! Hosted builds are unchanged: no hosted panic handler or allocator is
//! replaced, and hardware paths are cfg-gated to `target_os = "none"`.

pub mod alloc;
pub mod arch;
pub mod boot;
pub mod idt;
pub mod runtime;

pub use alloc::{BumpAllocator, Region, Resettable, HEAP_SIZE};
pub use arch::{
    interrupts_disable, interrupts_enable, mmio_read, mmio_write, pause, port_in_u8,
    port_out_u8, read_volatile, write_volatile,
};
pub use boot::{BootCheck, BootChecklist};
pub use idt::{Idt, IdtEntry, IdtError};
pub use runtime::{
    BufferWriter, NoSerial, Serial, SerialError, SerialWriter, Value, ValueOpError,
};

use std::error::Error;
use std::fmt;

/// Entry symbol emitted by the multiboot2 stub.
pub const DEFAULT_ENTRY_SYMBOL: &str = "_start";

/// Target triples with an implemented boot stub. Other freestanding triples
/// are rejected rather than accepted without one.
pub const SUPPORTED_TRIPLES: &[&str] = &[
    "x86_64-unknown-none",
    "x86_64-unknown-none-elf",
];

/// Panic strategy requested for a freestanding build.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PanicStrategy {
    /// The only strategy the image supports.
    Abort,
    /// Accepted as input so validation can reject it by name.
    Unwind,
}

impl PanicStrategy {
    /// Canonical lowercase spelling used in diagnostics.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Abort => "abort",
            Self::Unwind => "unwind",
        }
    }
}

impl fmt::Display for PanicStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Why a freestanding build request was rejected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TargetErrorKind {
    /// Triple is not `arch-vendor-os[-env]`.
    MalformedTriple,
    /// No runtime stub exists for the architecture.
    UnsupportedArchitecture,
    /// The triple names a hosted operating system.
    HostedTarget,
    /// The vendor or environment has not been validated.
    UnsupportedComponent,
    /// An option such as `panic=unwind` needs a hosted runtime.
    UnsupportedOption,
}

/// A precise diagnostic for a rejected build request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TargetError {
    kind: TargetErrorKind,
    option: &'static str,
    value: String,
    reason: String,
}

impl TargetError {
    fn new(
        kind: TargetErrorKind,
        option: &'static str,
        value: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            option,
            value: value.into(),
            reason: reason.into(),
        }
    }

    /// Category of the rejection.
    pub fn kind(&self) -> TargetErrorKind {
        self.kind
    }

    /// Compiler option the rejected value came from.
    pub fn option(&self) -> &'static str {
        self.option
    }

    /// Rejected triple or option value.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Explanation suitable for a compiler diagnostic.
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// Full one-line diagnostic, including the offending option.
    pub fn diagnostic(&self) -> String {
        format!("{} '{}': {}", self.option, self.value, self.reason)
    }
}

impl fmt::Display for TargetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.diagnostic())
    }
}

impl Error for TargetError {}

/// A target the runtime can boot, produced only by [`validate_target`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FreestandingTarget {
    triple: String,
}

impl FreestandingTarget {
    /// Validate and wrap a target triple.
    pub fn parse(triple: &str) -> Result<Self, TargetError> {
        validate_target(triple)
    }

    /// Validated triple.
    pub fn triple(&self) -> &str {
        &self.triple
    }

    /// Symbol the loader transfers control to.
    pub fn entry_symbol(&self) -> &'static str {
        DEFAULT_ENTRY_SYMBOL
    }

    /// Linker script shipped beside this module.
    pub fn linker_script(&self) -> &'static str {
        "x86_64-none.ld"
    }
}

/// Validate a `--target` value.
///
/// Only the x86-64 multiboot2 triple is implemented. Hosted triples are
/// rejected with the component that makes them hosted, and other `none`
/// triples are rejected by name rather than accepted as a no-op.
pub fn validate_target(triple: &str) -> Result<FreestandingTarget, TargetError> {
    if triple.is_empty() {
        return Err(TargetError::new(
            TargetErrorKind::MalformedTriple,
            "--target",
            triple,
            "target triple must not be empty",
        ));
    }

    let components: Vec<&str> = triple.split('-').collect();
    if !(3..=4).contains(&components.len()) || components.iter().any(|part| part.is_empty()) {
        return Err(TargetError::new(
            TargetErrorKind::MalformedTriple,
            "--target",
            triple,
            "expected arch-vendor-os[-env] with 3 or 4 non-empty components",
        ));
    }
    if !components
        .iter()
        .all(|part| part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.'))
    {
        return Err(TargetError::new(
            TargetErrorKind::MalformedTriple,
            "--target",
            triple,
            "components may contain only ASCII letters, digits, '_' and '.'",
        ));
    }

    let arch = components[0];
    let vendor = components[1];
    let os = components[2];
    if arch != "x86_64" {
        return Err(TargetError::new(
            TargetErrorKind::UnsupportedArchitecture,
            "--target",
            triple,
            format!(
                "no freestanding runtime stub for architecture '{}'; supported: x86_64",
                arch
            ),
        ));
    }
    if os != "none" {
        return Err(TargetError::new(
            TargetErrorKind::HostedTarget,
            "--target",
            triple,
            format!(
                "freestanding builds require the 'none' operating-system component, got '{}'",
                os
            ),
        ));
    }
    if vendor != "unknown" {
        return Err(TargetError::new(
            TargetErrorKind::UnsupportedComponent,
            "--target",
            triple,
            format!("only the 'unknown' vendor is validated, got '{}'", vendor),
        ));
    }
    if components.len() == 4 && components[3] != "elf" {
        return Err(TargetError::new(
            TargetErrorKind::UnsupportedComponent,
            "--target",
            triple,
            format!(
                "unsupported environment '{}'; expected 'elf' or no environment",
                components[3]
            ),
        ));
    }

    Ok(FreestandingTarget {
        triple: triple.to_string(),
    })
}

/// Validate the panic strategy for a freestanding build.
pub fn validate_panic_strategy(strategy: PanicStrategy) -> Result<(), TargetError> {
    match strategy {
        PanicStrategy::Abort => Ok(()),
        PanicStrategy::Unwind => Err(TargetError::new(
            TargetErrorKind::UnsupportedOption,
            "-C panic",
            strategy.as_str(),
            "unwinding needs a hosted runtime for personality routines and exception tables; freestanding images must abort",
        )),
    }
}

/// Validate a complete freestanding build request.
pub fn validate_build_request(
    triple: &str,
    panic: PanicStrategy,
) -> Result<FreestandingTarget, TargetError> {
    let target = validate_target(triple)?;
    validate_panic_strategy(panic)?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn serial() -> Serial<BufferWriter<128>> {
        Serial::new(BufferWriter::new())
    }

    // Target validation

    #[test]
    fn supported_triples_validate() {
        for triple in SUPPORTED_TRIPLES {
            let target = validate_target(triple).expect("supported triple must validate");
            assert_eq!(target.triple(), *triple);
            assert_eq!(target.entry_symbol(), "_start");
            assert_eq!(target.linker_script(), "x86_64-none.ld");
        }
    }

    #[test]
    fn malformed_triples_are_rejected_by_name() {
        for triple in ["", "x86_64-none", "x86_64-unknown-none-elf-extra", "x86_64--none"] {
            let err = validate_target(triple).unwrap_err();
            assert_eq!(err.kind(), TargetErrorKind::MalformedTriple, "triple: {}", triple);
            assert_eq!(err.option(), "--target");
        }
        let err = validate_target("x86_64-unknown-no/ne").unwrap_err();
        assert_eq!(err.kind(), TargetErrorKind::MalformedTriple);
        assert!(err.reason().contains("ASCII"));
    }

    #[test]
    fn hosted_target_names_the_offending_component() {
        let err = validate_target("x86_64-pc-windows-msvc").unwrap_err();
        assert_eq!(err.kind(), TargetErrorKind::HostedTarget);
        assert!(err.reason().contains("'windows'"));
    }

    #[test]
    fn unsupported_architecture_is_rejected_instead_of_ignored() {
        let err = validate_target("aarch64-unknown-none").unwrap_err();
        assert_eq!(err.kind(), TargetErrorKind::UnsupportedArchitecture);
        assert!(err.reason().contains("aarch64"));
        assert!(err.reason().contains("x86_64"));
    }

    #[test]
    fn unverified_vendor_and_environment_are_rejected() {
        let vendor = validate_target("x86_64-pc-none").unwrap_err();
        assert_eq!(vendor.kind(), TargetErrorKind::UnsupportedComponent);
        assert!(vendor.reason().contains("'pc'"));

        let env = validate_target("x86_64-unknown-none-uefi").unwrap_err();
        assert_eq!(env.kind(), TargetErrorKind::UnsupportedComponent);
        assert!(env.reason().contains("uefi"));
    }

    #[test]
    fn unwind_is_rejected_with_a_precise_diagnostic() {
        let err = validate_panic_strategy(PanicStrategy::Unwind).unwrap_err();
        assert_eq!(err.kind(), TargetErrorKind::UnsupportedOption);
        assert_eq!(err.option(), "-C panic");
        assert_eq!(err.value(), "unwind");
        assert!(err.reason().contains("personality routines"));
        assert!(validate_panic_strategy(PanicStrategy::Abort).is_ok());
    }

    #[test]
    fn build_request_validates_both_option_and_target() {
        let target = validate_build_request("x86_64-unknown-none", PanicStrategy::Abort)
            .expect("minimal freestanding build is supported");
        assert_eq!(target.triple(), "x86_64-unknown-none");

        let err = validate_build_request("x86_64-unknown-linux-gnu", PanicStrategy::Unwind)
            .unwrap_err();
        assert_eq!(err.kind(), TargetErrorKind::HostedTarget);
        assert!(err.diagnostic().starts_with("--target '"));
        assert!(FreestandingTarget::parse("x86_64-unknown-none").is_ok());
    }

    #[test]
    fn error_display_matches_diagnostic() {
        let err = validate_target("x86_64-unknown-linux-gnu").unwrap_err();
        assert_eq!(err.to_string(), err.diagnostic());
        assert!(err.diagnostic().contains("x86_64-unknown-linux-gnu"));
    }

    // Serial formatting

    #[test]
    fn unsigned_values_format_in_supported_radices() {
        let mut out = serial();
        out.write_u64(255, 10).unwrap();
        out.write_str(" ").unwrap();
        out.write_u64(255, 2).unwrap();
        out.write_str(" ").unwrap();
        out.write_hex(0x2a).unwrap();
        out.write_str(" ").unwrap();
        out.write_u64(0, 10).unwrap();
        assert_eq!(out.writer().as_str(), "255 11111111 2a 0");
    }

    #[test]
    fn signed_values_format_including_the_minimum() {
        let mut out = serial();
        out.write_i64(-42).unwrap();
        out.write_str(" ").unwrap();
        out.write_i64(i64::MIN).unwrap();
        assert_eq!(out.writer().as_str(), "-42 -9223372036854775808");
    }

    #[test]
    fn values_and_lines_format_without_allocation() {
        let mut out = serial();
        out.write_value(Value::Bool(true)).unwrap();
        out.write_str(" ").unwrap();
        out.write_value(Value::Int(-7)).unwrap();
        out.write_str(" ").unwrap();
        out.write_value(Value::UInt(9)).unwrap();
        out.write_line("").unwrap();
        assert_eq!(out.writer().as_str(), "true -7 9\r\n");
    }

    #[test]
    fn fmt_write_integrates_with_core() {
        use core::fmt::Write as _;
        let mut out = serial();
        write!(out, "{} {}", 7, "ok").unwrap();
        assert_eq!(out.writer().as_str(), "7 ok");
    }

    #[test]
    fn buffer_overflow_is_an_explicit_error() {
        let mut out = Serial::new(BufferWriter::<2>::new());
        assert_eq!(out.write_str("abc").unwrap_err().reason(), "serial buffer full");
        assert_eq!(out.writer().as_str(), "ab");
    }

    #[test]
    fn absent_serial_is_an_explicit_error() {
        let mut out = Serial::new(NoSerial);
        let err = out.write_str("x").unwrap_err();
        assert_eq!(err.reason(), "no serial device configured for this target");
        assert_eq!(err.to_string(), err.reason());
    }

    #[test]
    fn invalid_radix_is_an_explicit_error() {
        let mut out = serial();
        assert!(out.write_u64(1, 1).is_err());
        assert!(out.write_u64(1, 17).is_err());
        assert!(out.writer().is_empty());
    }

    // Allocator-free value operations

    #[test]
    fn checked_arithmetic_succeeds_for_matching_widths() {
        assert_eq!(Value::Int(2).checked_add_value(Value::Int(3)), Ok(Value::Int(5)));
        assert_eq!(Value::UInt(9).checked_sub_value(Value::UInt(4)), Ok(Value::UInt(5)));
        assert_eq!(Value::Int(6).checked_mul_value(Value::Int(7)), Ok(Value::Int(42)));
        assert_eq!(Value::Int(4).checked_neg_value(), Ok(Value::Int(-4)));
    }

    #[test]
    fn overflow_is_reported_not_wrapped() {
        assert!(matches!(
            Value::Int(i64::MAX).checked_add_value(Value::Int(1)),
            Err(ValueOpError::Overflow(_))
        ));
        assert!(matches!(
            Value::UInt(0).checked_sub_value(Value::UInt(1)),
            Err(ValueOpError::Overflow(_))
        ));
        assert!(matches!(
            Value::Int(i64::MIN).checked_neg_value(),
            Err(ValueOpError::Overflow(_))
        ));
    }

    #[test]
    fn unsupported_operands_are_reported() {
        assert!(matches!(
            Value::Bool(true).checked_add_value(Value::Int(1)),
            Err(ValueOpError::Unsupported(_))
        ));
        assert!(matches!(
            Value::UInt(1).checked_neg_value(),
            Err(ValueOpError::Unsupported(_))
        ));
        assert!(Value::Bool(true).is_truthy());
        assert!(!Value::Int(0).is_truthy());
        assert!(Value::UInt(1).is_truthy());
    }
}
