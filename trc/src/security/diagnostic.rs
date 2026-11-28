//! Precise diagnostic for a denied capability, including the native call site.

use super::capability::Capability;
use std::fmt;

/// Stable code reserved for security-policy denials (the compiler catalog in
/// `crate::errors` occupies E0xxx–E3xxx).
pub const SECURITY_DENIAL_CODE: &str = "E5001";

/// Where a native was called from. `native_name` is always present; module,
/// function, and 1-based source coordinates are optional when the dispatch
/// layer knows them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeCallSite {
    pub native_name: String,
    pub module: Option<String>,
    pub function: Option<String>,
    pub location: Option<(usize, usize)>,
}

impl NativeCallSite {
    pub fn new(native_name: impl Into<String>) -> Self {
        Self {
            native_name: native_name.into(),
            module: None,
            function: None,
            location: None,
        }
    }

    pub fn with_location(
        native_name: impl Into<String>,
        module: impl Into<String>,
        function: impl Into<String>,
        line: usize,
        column: usize,
    ) -> Self {
        Self {
            native_name: native_name.into(),
            module: Some(module.into()),
            function: Some(function.into()),
            location: Some((line, column)),
        }
    }
}

impl fmt::Display for NativeCallSite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.module, &self.function) {
            (Some(module), Some(function)) => {
                write!(f, "{}.{} ({})", module, function, self.native_name)?;
            }
            _ => write!(f, "{}", self.native_name)?,
        }
        if let Some((line, column)) = self.location {
            write!(f, " at {}:{}", line, column)?;
        }
        Ok(())
    }
}

/// A capability denial produced at native dispatch. Carries both the required
/// capability name and the native call site so callers get an actionable error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityDenied {
    pub code: &'static str,
    pub capability: String,
    pub call_site: Box<NativeCallSite>,
    pub reason: String,
}

impl CapabilityDenied {
    pub fn new(
        capability: Capability,
        call_site: NativeCallSite,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            code: SECURITY_DENIAL_CODE,
            capability: capability.name().to_string(),
            call_site: Box::new(call_site),
            reason: reason.into(),
        }
    }

    pub fn capability_name(&self) -> &str {
        &self.capability
    }

    pub fn call_site(&self) -> &NativeCallSite {
        &self.call_site
    }

    /// Serialize with a fixed key order: code, capability, call_site, reason.
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        out.push('{');
        out.push_str("\"code\":");
        write_json_string(&mut out, self.code);
        out.push_str(",\"capability\":");
        write_json_string(&mut out, &self.capability);
        out.push_str(",\"call_site\":");
        write_json_string(&mut out, &self.call_site.to_string());
        out.push_str(",\"reason\":");
        write_json_string(&mut out, &self.reason);
        out.push('}');
        out
    }
}

impl fmt::Display for CapabilityDenied {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "error[{}]: capability '{}' denied at native call site '{}': {}",
            self.code, self.capability, self.call_site, self.reason
        )
    }
}

impl std::error::Error for CapabilityDenied {}

/// Escape a string for JSON. Shared with the audit logger so both emit the
/// same deterministic shape without pulling in an external crate.
pub(crate) fn write_json_string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}
