//! Per-VM security policy and the native-dispatch enforcement gate.

use super::audit::{AuditEvent, AuditLog};
use super::capability::{Capability, CapabilityRegistry, CapabilitySet};
use super::diagnostic::{CapabilityDenied, NativeCallSite};
use super::path::confine_path;
use std::path::{Path, PathBuf};

/// Resource limits applied by the gate. Trusted mode uses
/// [`ResourceBudget::UNLIMITED`] so behavior is identical to before.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceBudget {
    pub max_steps: u64,
    pub max_output_bytes: usize,
    pub max_allocation: usize,
}

impl ResourceBudget {
    pub const UNLIMITED: Self = Self {
        max_steps: u64::MAX,
        max_output_bytes: usize::MAX,
        max_allocation: usize::MAX,
    };

    pub const RESTRICTED: Self = Self {
        max_steps: 1_000_000,
        max_output_bytes: 8 * 1024 * 1024,
        max_allocation: 64 * 1024 * 1024,
    };
}

/// The policy fixed at VM construction. Trusted is pass-through; untrusted
/// grants only a safe capability set, confines paths, and applies budgets.
#[derive(Clone, Debug)]
pub struct SecurityPolicy {
    trusted: bool,
    allowed: CapabilitySet,
    budgets: ResourceBudget,
    sandbox_root: Option<PathBuf>,
}

impl SecurityPolicy {
    pub fn trusted() -> Self {
        Self {
            trusted: true,
            allowed: CapabilitySet::all(),
            budgets: ResourceBudget::UNLIMITED,
            sandbox_root: None,
        }
    }

    pub fn untrusted(root: impl Into<PathBuf>) -> Self {
        Self {
            trusted: false,
            allowed: CapabilitySet::safe_default(),
            budgets: ResourceBudget::RESTRICTED,
            sandbox_root: Some(root.into()),
        }
    }

    pub fn with_budgets(mut self, budgets: ResourceBudget) -> Self {
        self.budgets = budgets;
        self
    }

    pub fn allowing(mut self, capability: Capability) -> Self {
        self.allowed = self.allowed.allow(capability);
        self
    }

    pub fn denying(mut self, capability: Capability) -> Self {
        self.allowed = self.allowed.deny(capability);
        self
    }

    pub const fn is_trusted(&self) -> bool {
        self.trusted
    }

    pub const fn allows(&self, capability: Capability) -> bool {
        self.trusted || self.allowed.allows(capability)
    }

    pub const fn budgets(&self) -> &ResourceBudget {
        &self.budgets
    }

    pub fn sandbox_root(&self) -> Option<&Path> {
        self.sandbox_root.as_deref()
    }
}

/// One enforcer per VM. Every decision is appended to the audit log, including
/// allowed calls, so denials can be correlated with the calls that preceded
/// them.
#[derive(Clone, Debug)]
pub struct SecurityEnforcer {
    policy: SecurityPolicy,
    registry: CapabilityRegistry,
    audit: AuditLog,
}

impl SecurityEnforcer {
    pub fn new(policy: SecurityPolicy) -> Self {
        Self {
            policy,
            registry: CapabilityRegistry::new(),
            audit: AuditLog::new(),
        }
    }

    pub fn policy(&self) -> &SecurityPolicy {
        &self.policy
    }

    /// Register an explicit capability for a host-added native before dispatch.
    pub fn registry_mut(&mut self) -> &mut CapabilityRegistry {
        &mut self.registry
    }

    pub fn audit(&self) -> &AuditLog {
        &self.audit
    }

    pub fn into_audit(self) -> AuditLog {
        self.audit
    }

    /// Gate called from native dispatch. Trusted policies allow everything.
    pub fn enforce_native(&mut self, call: NativeCallSite) -> Result<(), CapabilityDenied> {
        let capability = self.registry.capability_for(&call.native_name);
        if self.policy.allows(capability) {
            self.audit.record(AuditEvent::allowed(call, capability));
            return Ok(());
        }
        let reason = format!(
            "native '{}' requires capability '{}' which the active policy does not grant",
            call.native_name,
            capability.name()
        );
        let denial = CapabilityDenied::new(capability, call.clone(), reason.clone());
        self.audit.record(AuditEvent::denied(call, capability, reason));
        Err(denial)
    }

    /// Confine a path used by a filesystem native. Trusted mode returns the
    /// requested path unchanged, preserving historical behavior.
    pub fn enforce_path(
        &mut self,
        call: NativeCallSite,
        requested: &Path,
    ) -> Result<PathBuf, CapabilityDenied> {
        let root = if self.policy.is_trusted() {
            None
        } else {
            self.policy.sandbox_root().map(Path::to_path_buf)
        };
        let capability = match self.registry.capability_for(&call.native_name) {
            Capability::FileSystemWrite => Capability::FileSystemWrite,
            _ => Capability::FileSystemRead,
        };
        let root = match root {
            Some(root) => root,
            None => {
                self.audit.record(AuditEvent::allowed(call, capability));
                return Ok(requested.to_path_buf());
            }
        };
        match confine_path(&root, requested) {
            Ok(resolved) => {
                self.audit.record(AuditEvent::allowed(call, capability));
                Ok(resolved)
            }
            Err(escape) => {
                let reason = escape.to_string();
                let denial = CapabilityDenied::new(capability, call.clone(), reason.clone());
                self.audit
                    .record(AuditEvent::denied(call, capability, reason));
                Err(denial)
            }
        }
    }

    /// Reject a native allocation larger than the untrusted budget.
    pub fn enforce_allocation(
        &mut self,
        call: NativeCallSite,
        bytes: usize,
    ) -> Result<(), CapabilityDenied> {
        let limit = self.policy.budgets.max_allocation;
        if self.policy.is_trusted() || bytes <= limit {
            self.audit
                .record(AuditEvent::allowed(call, Capability::ResourceBudget));
            return Ok(());
        }
        self.budget_denial(call, Capability::ResourceBudget, bytes, limit, "allocation")
    }

    /// Reject a VM step count beyond the untrusted budget.
    pub fn enforce_steps(&mut self, call: NativeCallSite, steps: u64) -> Result<(), CapabilityDenied> {
        let limit = self.policy.budgets.max_steps;
        if self.policy.is_trusted() || steps <= limit {
            self.audit
                .record(AuditEvent::allowed(call, Capability::ResourceBudget));
            return Ok(());
        }
        self.budget_denial(call, Capability::ResourceBudget, steps, limit, "steps")
    }

    /// Reject total captured output beyond the untrusted budget.
    pub fn enforce_output(
        &mut self,
        call: NativeCallSite,
        total_bytes: usize,
    ) -> Result<(), CapabilityDenied> {
        let limit = self.policy.budgets.max_output_bytes;
        if self.policy.is_trusted() || total_bytes <= limit {
            self.audit
                .record(AuditEvent::allowed(call, Capability::ResourceBudget));
            return Ok(());
        }
        self.budget_denial(call, Capability::ResourceBudget, total_bytes, limit, "output")
    }

    fn budget_denial<T: std::fmt::Display>(
        &mut self,
        call: NativeCallSite,
        capability: Capability,
        actual: T,
        limit: T,
        what: &str,
    ) -> Result<(), CapabilityDenied> {
        let reason = format!("{} of {} exceeds budget of {}", what, actual, limit);
        let denial = CapabilityDenied::new(capability, call.clone(), reason.clone());
        self.audit.record(AuditEvent::denied(call, capability, reason));
        Err(denial)
    }
}
