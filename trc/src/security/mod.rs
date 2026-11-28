//! Phase 4 trust model — capability-tagged native dispatch enforcement.
//!
//! This module is self-contained (standard library only) so it can be compiled
//! both as part of `trc` and directly from the integration test in
//! `trc/tests/security_policy.rs` via `#[path]`. It deliberately depends on no
//! private items in the rest of the crate.
//!
//! Intended wiring point: hold one [`SecurityEnforcer`] per VM and call
//! [`SecurityEnforcer::enforce_native`] from `Vm::call_native_fn` before the
//! native is invoked, and [`SecurityEnforcer::enforce_path`] from every native
//! that resolves a path. Trusted policies are pass-through, so historical
//! behavior is preserved.

// The crate wiring is intentionally deferred (see the task limitations), so
// some public helpers are not yet referenced from this crate.
#![allow(dead_code)]

pub mod audit;
pub mod capability;
pub mod diagnostic;
pub mod path;
pub mod policy;

pub use audit::{AuditDecision, AuditEvent, AuditLog};
pub use capability::{classify_native, Capability, CapabilityRegistry, CapabilitySet};
pub use diagnostic::{CapabilityDenied, NativeCallSite, SECURITY_DENIAL_CODE};
pub use path::{confine_path, PathEscape};
pub use policy::{ResourceBudget, SecurityEnforcer, SecurityPolicy};

#[cfg(test)]
mod tests;
