//! Configuration scaffolding for freestanding (OS-ready) builds.
//!
//! Phase 14 of `ROADMAP.md` requires a separate, `no_std` build path that
//! produces an image with no compiler, Rust `std`, CRT, or LLVM-C in it. This
//! module holds the validated description of such a build: target triple,
//! entry symbol, linker script, and panic strategy.
//!
//! The module is deliberately inert. It does not call into `LlvmBackend`,
//! `linker`, or the CLI, and it changes no hosted behavior. The design for
//! wiring it into a real freestanding pipeline lives in
//! `docs/plans/os-readiness.md`.

use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

/// Entry symbol used when the caller does not supply one.
pub const DEFAULT_ENTRY_SYMBOL: &str = "_start";

/// Architectures the Phase 14 target wrappers cover or are planned to cover.
const KNOWN_ARCHITECTURES: &[&str] = &[
    "x86_64",
    "aarch64",
    "riscv32",
    "riscv64",
    "arm",
    "thumbv7em",
    "wasm32",
    "wasm64",
];

/// How a freestanding image responds to a panic.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PanicStrategy {
    /// Print a short diagnostic and halt. The only strategy a freestanding
    /// image can use: unwinding needs personality routines and exception
    /// tables supplied by a hosted runtime.
    Abort,
    /// Accepted as input so validation can reject it with a precise diagnostic
    /// instead of a generic parse error.
    Unwind,
}

impl PanicStrategy {
    /// Canonical lowercase spelling, used in diagnostics and CLI input.
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

/// A validated, immutable freestanding build configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FreestandingConfig {
    target_triple: String,
    entry_symbol: String,
    linker_script: PathBuf,
    panic_strategy: PanicStrategy,
}

impl FreestandingConfig {
    /// Start a new configuration builder.
    pub fn builder() -> FreestandingConfigBuilder {
        FreestandingConfigBuilder::new()
    }

    /// Target triple, for example `x86_64-unknown-none`.
    pub fn target_triple(&self) -> &str {
        &self.target_triple
    }

    /// Symbol the loader transfers control to, by default `_start`.
    pub fn entry_symbol(&self) -> &str {
        &self.entry_symbol
    }

    /// Linker script that lays out the boot image.
    pub fn linker_script(&self) -> &Path {
        &self.linker_script
    }

    /// Panic strategy; always [`PanicStrategy::Abort`] after validation.
    pub fn panic_strategy(&self) -> PanicStrategy {
        self.panic_strategy
    }
}

/// A validation failure, tagged with the field that caused it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigError {
    field: &'static str,
    message: String,
}

impl ConfigError {
    fn new(field: &'static str, message: impl Into<String>) -> Self {
        Self {
            field,
            message: message.into(),
        }
    }

    /// Name of the invalid field, suitable for a diagnostic.
    pub fn field(&self) -> &'static str {
        self.field
    }

    /// Human-readable explanation of the failure.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

impl Error for ConfigError {}

/// Incremental builder for [`FreestandingConfig`].
///
/// The target triple and linker script are required. The entry symbol defaults
/// to [`DEFAULT_ENTRY_SYMBOL`] and the panic strategy defaults to
/// [`PanicStrategy::Abort`].
#[derive(Clone, Debug, Default)]
pub struct FreestandingConfigBuilder {
    target_triple: Option<String>,
    entry_symbol: Option<String>,
    linker_script: Option<PathBuf>,
    panic_strategy: Option<PanicStrategy>,
}

impl FreestandingConfigBuilder {
    /// Create an empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the target triple, for example `x86_64-unknown-none`.
    pub fn target_triple(mut self, triple: impl Into<String>) -> Self {
        self.target_triple = Some(triple.into());
        self
    }

    /// Set the entry symbol. Defaults to [`DEFAULT_ENTRY_SYMBOL`].
    pub fn entry_symbol(mut self, symbol: impl Into<String>) -> Self {
        self.entry_symbol = Some(symbol.into());
        self
    }

    /// Set the path of the linker script used to lay out the image.
    pub fn linker_script(mut self, path: impl Into<PathBuf>) -> Self {
        self.linker_script = Some(path.into());
        self
    }

    /// Set the panic strategy. Defaults to [`PanicStrategy::Abort`].
    pub fn panic_strategy(mut self, strategy: PanicStrategy) -> Self {
        self.panic_strategy = Some(strategy);
        self
    }

    /// Validate all fields and produce an immutable configuration.
    pub fn build(self) -> Result<FreestandingConfig, ConfigError> {
        let target_triple = self.target_triple.ok_or_else(|| {
            ConfigError::new("target_triple", "required for a freestanding build")
        })?;
        validate_target_triple(&target_triple)?;

        let entry_symbol = self
            .entry_symbol
            .unwrap_or_else(|| DEFAULT_ENTRY_SYMBOL.to_string());
        validate_entry_symbol(&entry_symbol)?;

        let linker_script = self.linker_script.ok_or_else(|| {
            ConfigError::new("linker_script", "required for a freestanding build")
        })?;
        validate_linker_script(&linker_script)?;

        let panic_strategy = self.panic_strategy.unwrap_or(PanicStrategy::Abort);
        validate_panic_strategy(panic_strategy)?;

        Ok(FreestandingConfig {
            target_triple,
            entry_symbol,
            linker_script,
            panic_strategy,
        })
    }
}

/// A freestanding target is `arch-vendor-os[-env]` with `os = none`.
fn validate_target_triple(triple: &str) -> Result<(), ConfigError> {
    if triple.is_empty() {
        return Err(ConfigError::new("target_triple", "must not be empty"));
    }

    let components: Vec<&str> = triple.split('-').collect();
    if !(3..=4).contains(&components.len()) {
        return Err(ConfigError::new(
            "target_triple",
            format!(
                "expected arch-vendor-os[-env] (3 or 4 components), got '{}'",
                triple
            ),
        ));
    }
    if components.iter().any(|component| component.is_empty()) {
        return Err(ConfigError::new(
            "target_triple",
            format!("contains an empty component: '{}'", triple),
        ));
    }
    let valid_component = |component: &&str| {
        component
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
    };
    if !components.iter().all(valid_component) {
        return Err(ConfigError::new(
            "target_triple",
            format!(
                "components may contain only ASCII letters, digits, '_' and '.': '{}'",
                triple
            ),
        ));
    }
    if !KNOWN_ARCHITECTURES.contains(&components[0]) {
        return Err(ConfigError::new(
            "target_triple",
            format!(
                "unsupported architecture '{}'; supported: {}",
                components[0],
                KNOWN_ARCHITECTURES.join(", ")
            ),
        ));
    }
    if components[2] != "none" {
        return Err(ConfigError::new(
            "target_triple",
            format!(
                "freestanding builds require the 'none' operating-system component, got '{}'",
                components[2]
            ),
        ));
    }
    Ok(())
}

/// An entry symbol must be a plain C-style identifier LLVM can name directly.
fn validate_entry_symbol(symbol: &str) -> Result<(), ConfigError> {
    let mut chars = symbol.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' => {}
        _ => {
            return Err(ConfigError::new(
                "entry_symbol",
                format!("'{}' must start with an ASCII letter or '_'", symbol),
            ))
        }
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(ConfigError::new(
            "entry_symbol",
            format!("'{}' may contain only ASCII letters, digits and '_'", symbol),
        ));
    }
    Ok(())
}

/// The linker script path is passed to an external linker, so reject strings
/// that cannot represent a single, ordinary file name.
fn validate_linker_script(path: &Path) -> Result<(), ConfigError> {
    let text = path.as_os_str().to_str().ok_or_else(|| {
        ConfigError::new("linker_script", "path must be valid UTF-8")
    })?;
    if text.is_empty() {
        return Err(ConfigError::new("linker_script", "path must not be empty"));
    }
    if text.contains('\0') {
        return Err(ConfigError::new(
            "linker_script",
            "path must not contain a NUL byte",
        ));
    }
    if path.file_name().is_none() {
        return Err(ConfigError::new(
            "linker_script",
            format!("'{}' must name a file", text),
        ));
    }
    Ok(())
}

/// Unwinding requires a hosted runtime that the freestanding image omits.
fn validate_panic_strategy(strategy: PanicStrategy) -> Result<(), ConfigError> {
    match strategy {
        PanicStrategy::Abort => Ok(()),
        PanicStrategy::Unwind => Err(ConfigError::new(
            "panic_strategy",
            "unwinding needs personality routines and exception tables from a hosted runtime; freestanding images must abort",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal() -> FreestandingConfigBuilder {
        FreestandingConfigBuilder::new()
            .target_triple("x86_64-unknown-none")
            .linker_script("linker/x86_64-none.ld")
    }

    #[test]
    fn defaults_are_applied_when_omitted() {
        let config = minimal().build().expect("minimal config must build");
        assert_eq!(config.entry_symbol(), DEFAULT_ENTRY_SYMBOL);
        assert_eq!(config.panic_strategy(), PanicStrategy::Abort);
    }

    #[test]
    fn explicit_fields_are_preserved() {
        let config = minimal()
            .entry_symbol("kmain")
            .panic_strategy(PanicStrategy::Abort)
            .build()
            .expect("config must build");
        assert_eq!(config.target_triple(), "x86_64-unknown-none");
        assert_eq!(config.entry_symbol(), "kmain");
        assert_eq!(config.linker_script(), Path::new("linker/x86_64-none.ld"));
        assert_eq!(config.panic_strategy(), PanicStrategy::Abort);
    }

    #[test]
    fn config_builder_entry_point_matches_new() {
        let config = FreestandingConfig::builder()
            .target_triple("x86_64-unknown-none")
            .linker_script("linker.ld")
            .build()
            .expect("config must build");
        assert_eq!(config.entry_symbol(), "_start");
    }

    #[test]
    fn missing_target_triple_is_rejected() {
        let err = FreestandingConfigBuilder::new()
            .linker_script("linker.ld")
            .build()
            .expect_err("target triple is required");
        assert_eq!(err.field(), "target_triple");
    }

    #[test]
    fn missing_linker_script_is_rejected() {
        let err = FreestandingConfigBuilder::new()
            .target_triple("x86_64-unknown-none")
            .build()
            .expect_err("linker script is required");
        assert_eq!(err.field(), "linker_script");
    }

    #[test]
    fn malformed_target_triples_are_rejected() {
        let cases = [
            ("", "empty"),
            ("x86_64-none", "two components"),
            ("x86_64-unknown-none-extra-extra", "five components"),
            ("x86_64--none", "empty component"),
            ("sparc-unknown-none", "unknown architecture"),
            ("x86_64-pc-windows-msvc", "hosted OS"),
            ("x86_64-unknown-linux-gnu", "hosted OS with environment"),
            ("x86_64-unknown-no/ne", "invalid component character"),
        ];
        for (triple, reason) in cases {
            let err = minimal().target_triple(triple).build().unwrap_err();
            assert_eq!(err.field(), "target_triple", "case: {}", reason);
        }
    }

    #[test]
    fn triple_with_environment_is_accepted() {
        let config = minimal()
            .target_triple("aarch64-unknown-none-elf")
            .build()
            .expect("four-component none triple must build");
        assert_eq!(config.target_triple(), "aarch64-unknown-none-elf");
    }

    #[test]
    fn malformed_entry_symbols_are_rejected() {
        for symbol in ["", "1start", "has-dash", "has space", "_start.$"] {
            let err = minimal().entry_symbol(symbol).build().unwrap_err();
            assert_eq!(err.field(), "entry_symbol", "symbol: {}", symbol);
        }
    }

    #[test]
    fn malformed_linker_scripts_are_rejected() {
        for path in ["", "dir\0file.ld"] {
            let err = minimal().linker_script(path).build().unwrap_err();
            assert_eq!(err.field(), "linker_script", "path: {:?}", path);
        }
    }

    #[test]
    fn unwind_strategy_is_rejected() {
        let err = minimal()
            .panic_strategy(PanicStrategy::Unwind)
            .build()
            .expect_err("unwinding is not available freestanding");
        assert_eq!(err.field(), "panic_strategy");
        assert!(err.message().contains("freestanding"));
    }

    #[test]
    fn panic_strategy_names_are_stable() {
        assert_eq!(PanicStrategy::Abort.as_str(), "abort");
        assert_eq!(PanicStrategy::Unwind.as_str(), "unwind");
        assert_eq!(PanicStrategy::Abort.to_string(), "abort");
        assert_eq!(PanicStrategy::Unwind.to_string(), "unwind");
    }

    #[test]
    fn config_error_displays_field_and_message() {
        let err = FreestandingConfigBuilder::new()
            .build()
            .expect_err("empty builder must fail");
        assert!(err.to_string().starts_with("target_triple: "));
    }
}
