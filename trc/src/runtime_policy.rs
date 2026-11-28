// Titrate Alpha 0.2 – runtime capability policy

use crate::security::{
    classify_native, CapabilityDenied, NativeCallSite, ResourceBudget, SecurityEnforcer,
    SecurityPolicy,
};
use std::path::{Path, PathBuf};

/// Step cap applied to untrusted programs.
pub const UNTRUSTED_MAX_STEPS: u64 = 1_000_000;
/// Maximum total captured output for untrusted programs.
pub const UNTRUSTED_MAX_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
/// Maximum single allocation/IO size for untrusted programs.
pub const UNTRUSTED_MAX_ALLOCATION: usize = 64 * 1024 * 1024;

/// Resource limits applied by the VM to a program run.
#[derive(Clone, Debug)]
pub struct ResourceBudgets {
    pub max_steps: u64,
    pub max_output_bytes: usize,
    pub max_allocation: usize,
}

/// Capability set for a VM run. Trusted keeps the historical behavior; untrusted
/// drops dangerous native capabilities, confines paths, and applies budgets.
#[derive(Clone, Debug)]
pub struct RuntimePolicy {
    trusted: bool,
    budgets: ResourceBudgets,
    sandbox_root: Option<PathBuf>,
}

impl RuntimePolicy {
    /// Unrestricted execution, matching the VM's historical behavior.
    pub fn trusted() -> Self {
        Self {
            trusted: true,
            budgets: ResourceBudgets {
                max_steps: u64::MAX,
                max_output_bytes: usize::MAX,
                max_allocation: usize::MAX,
            },
            sandbox_root: None,
        }
    }

    /// Restricted execution rooted at `sandbox_root`.
    pub fn untrusted(sandbox_root: PathBuf) -> Self {
        Self {
            trusted: false,
            budgets: ResourceBudgets {
                max_steps: UNTRUSTED_MAX_STEPS,
                max_output_bytes: UNTRUSTED_MAX_OUTPUT_BYTES,
                max_allocation: UNTRUSTED_MAX_ALLOCATION,
            },
            sandbox_root: Some(sandbox_root),
        }
    }

    /// Override the default budgets (used to tighten limits further).
    pub fn with_budgets(mut self, budgets: ResourceBudgets) -> Self {
        self.budgets = budgets;
        self
    }

    /// View of this run's state as the security module's policy. No state is
    /// duplicated: both are built from the same trusted/root/budget inputs.
    fn security_policy(&self) -> SecurityPolicy {
        SecurityPolicy::untrusted(self.sandbox_root.clone().unwrap_or_default()).with_budgets(
            ResourceBudget {
                max_steps: self.budgets.max_steps,
                max_output_bytes: self.budgets.max_output_bytes,
                max_allocation: self.budgets.max_allocation,
            },
        )
    }

    /// Gate a native call with the security enforcer. Trusted runs are a
    /// pass-through; untrusted denials carry the canonical E5001 diagnostic.
    pub fn enforce_native(&self, name: &str) -> Result<(), CapabilityDenied> {
        if self.trusted {
            return Ok(());
        }
        let site = NativeCallSite::new(name);
        SecurityEnforcer::new(self.security_policy()).enforce_native(site.clone())?;
        // Keep the historical untrusted allowlist as an additional gate so
        // newly added natives cannot become callable by accident.
        if !Self::is_allowlisted(name) {
            return Err(CapabilityDenied::new(
                classify_native(name),
                site,
                format!("native '{}' is not in the untrusted allowlist", name),
            ));
        }
        Ok(())
    }

    /// Confine a relative filesystem path with the same enforcer. Trusted runs
    /// get the requested path unchanged, matching historical behavior.
    pub fn enforce_path(
        &self,
        native_name: &str,
        requested: &Path,
    ) -> Result<PathBuf, CapabilityDenied> {
        if self.trusted {
            return Ok(requested.to_path_buf());
        }
        SecurityEnforcer::new(self.security_policy())
            .enforce_path(NativeCallSite::new(native_name), requested)
    }

    pub fn is_trusted(&self) -> bool {
        self.trusted
    }

    pub fn budgets(&self) -> &ResourceBudgets {
        &self.budgets
    }

    pub fn sandbox_root(&self) -> Option<&Path> {
        self.sandbox_root.as_deref()
    }

    /// Whether a native may be invoked under this policy. Untrusted uses both
    /// the enforcer's capability check and the historical allowlist.
    pub fn allows_native(&self, name: &str) -> bool {
        self.enforce_native(name).is_ok()
    }

    fn is_allowlisted(name: &str) -> bool {
        const ALLOWED_FAMILIES: &[&str] = &[
            // io::println/print/readLine etc. compile to STATIC_CALL("io", ...).
            "io_",
            "Math_", "MathAdvanced_", "MathTrig_", "String_", "Integer_",
            "Long_", "Double_", "Boolean_", "Char_", "ArrayList_", "HashMap_",
            "HashSet_", "Iterator_", "Array_", "Json_", "JsonValue_", "Base64_",
            "Hex_", "Url_", "Hash_", "Hasher_", "Regex_", "Random_", "File_",
            "Path_", "Dir_", "Time_", "Gc_", "TypeName_",
        ];
        const ALLOWED_EXACT: &[&str] = &[
            "println", "print", "toString", "parseInt", "Ok", "Err",
            "IO_readLine", "IO_readAll",
        ];
        ALLOWED_EXACT.contains(&name)
            || ALLOWED_FAMILIES.iter().any(|prefix| name.starts_with(prefix))
    }
}
