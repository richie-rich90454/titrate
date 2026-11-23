// Titrate Alpha 0.2 – runtime capability policy

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

    pub fn is_trusted(&self) -> bool {
        self.trusted
    }

    pub fn budgets(&self) -> &ResourceBudgets {
        &self.budgets
    }

    pub fn sandbox_root(&self) -> Option<&Path> {
        self.sandbox_root.as_deref()
    }

    /// Whether a native may be invoked under this policy. The check is by
    /// capability prefix so all existing wrappers are covered centrally.
    pub fn allows_native(&self, name: &str) -> bool {
        if self.trusted {
            return true;
        }
        const DENIED: &[&str] = &[
            "Ctypes_",
            "Dll",
            "Libloading",
            "Process_",
            "Subprocess_",
            "Signal_",
            "Os_kill",
            "Sys_exit",
            "Net_",
            "Http_",
            "Socket_",
            "UdpSocket_",
            "Ssl_",
            "Ftp_",
            "WinReg_",
        ];
        !DENIED.iter().any(|prefix| name.starts_with(prefix))
    }
}
