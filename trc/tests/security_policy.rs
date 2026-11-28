//! Phase 4 trust model integration checks.
//!
//! The security module is included by path because it is intentionally
//! self-contained; these tests exercise only its public API, the same surface
//! the VM dispatch gate will use.

#[path = "../src/security/mod.rs"]
mod security;

use security::{
    Capability, CapabilityDenied, NativeCallSite, ResourceBudget, SecurityEnforcer, SecurityPolicy,
};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use trc::bytecode::value::Value;
use trc::bytecode::Vm;
use trc::runtime_policy::RuntimePolicy;
use trc::security::SECURITY_DENIAL_CODE;

static NEXT_ROOT: AtomicUsize = AtomicUsize::new(0);

fn root(label: &str) -> PathBuf {
    let sequence = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "titrate_security_policy_{}_{}_{}",
        std::process::id(),
        label,
        sequence
    ));
    std::fs::create_dir_all(&path).expect("create sandbox root");
    path
}

fn site(name: &str) -> NativeCallSite {
    NativeCallSite::new(name)
}

fn untrusted(label: &str) -> (PathBuf, SecurityEnforcer) {
    let sandbox = root(label);
    let enforcer = SecurityEnforcer::new(SecurityPolicy::untrusted(sandbox.clone()));
    (sandbox, enforcer)
}

#[test]
fn untrusted_policy_denies_process_network_ffi_and_registry_capabilities() {
    let (_sandbox, mut enforcer) = untrusted("dangerous");
    for (native, capability) in [
        ("Subprocess_run", Capability::Process),
        ("Process_spawn", Capability::Process),
        ("Signal_raise", Capability::Signals),
        ("Net_connect", Capability::Network),
        ("Http_get", Capability::Network),
        ("Socket_new", Capability::Network),
        ("Ctypes_dlopen", Capability::Ffi),
        ("Ctypes_call", Capability::Ffi),
        ("WinReg_OpenKey", Capability::Registry),
    ] {
        let denial = enforcer
            .enforce_native(site(native))
            .expect_err("untrusted policy must deny dangerous native");
        assert_eq!(denial.capability_name(), capability.name(), "{}", native);
        assert_eq!(denial.call_site().native_name, native);
    }
}

#[test]
fn untrusted_policy_confines_paths_and_rejects_escapes() {
    let (sandbox, mut enforcer) = untrusted("paths");
    let canonical = std::fs::canonicalize(&sandbox).expect("canonical sandbox");

    assert!(
        enforcer
            .enforce_path(site("File_readFile"), Path::new("../../secrets.txt"))
            .is_err(),
        "lexical escape must be denied"
    );

    let resolved = enforcer
        .enforce_path(site("File_readFile"), Path::new("notes.txt"))
        .expect("in-root path must resolve");
    assert!(resolved.starts_with(&canonical), "resolved: {:?}", resolved);

    std::fs::remove_dir_all(&sandbox).ok();
}

#[test]
fn untrusted_policy_denies_resource_exhaustion() {
    let sandbox = root("budgets");
    let policy = SecurityPolicy::untrusted(sandbox.clone()).with_budgets(ResourceBudget {
        max_steps: 4,
        max_output_bytes: 2,
        max_allocation: 3,
    });
    let mut enforcer = SecurityEnforcer::new(policy);

    assert!(enforcer.enforce_allocation(site("ArrayList_add"), 4).is_err());
    assert!(enforcer.enforce_steps(site("loop"), 5).is_err());
    assert!(enforcer.enforce_output(site("println"), 3).is_err());
    assert_eq!(enforcer.audit().denied_count(), 3);

    std::fs::remove_dir_all(&sandbox).ok();
}

#[test]
fn trusted_policy_is_unchanged() {
    let mut enforcer = SecurityEnforcer::new(SecurityPolicy::trusted());
    for native in ["Subprocess_run", "Net_connect", "Ctypes_dlopen"] {
        assert!(enforcer.enforce_native(site(native)).is_ok());
    }
    let requested = Path::new("/etc/passwd");
    assert_eq!(
        enforcer
            .enforce_path(site("File_readFile"), requested)
            .expect("trusted mode must not confine"),
        requested
    );
    assert!(enforcer
        .enforce_allocation(site("ArrayList_add"), usize::MAX)
        .is_ok());
}

#[test]
fn denial_diagnostic_is_precise_and_registry_overrides_apply() {
    let denial = CapabilityDenied::new(
        Capability::Network,
        NativeCallSite::with_location("Net_connect", "Net", "connect", 7, 3),
        "denied",
    );
    assert_eq!(denial.capability_name(), "NETWORK");
    assert_eq!(denial.call_site().native_name, "Net_connect");
    assert!(denial.to_json().contains("\"call_site\":\"Net.connect (Net_connect) at 7:3\""));

    let sandbox = root("registry");
    let mut enforcer = SecurityEnforcer::new(SecurityPolicy::untrusted(sandbox.clone()));
    enforcer
        .registry_mut()
        .register("Custom_native", Capability::Process);
    let denial = enforcer
        .enforce_native(site("Custom_native"))
        .expect_err("registered capability must be enforced");
    assert_eq!(denial.capability_name(), Capability::Process.name());
    std::fs::remove_dir_all(&sandbox).ok();
}

// The checks below use the real crate VM (not the path-included module above),
// so they exercise native dispatch end to end.

#[test]
fn vm_dispatch_denies_dangerous_natives_only_when_untrusted() {
    let sandbox = root("vm_deny");
    let mut untrusted = Vm::new();
    untrusted.set_policy(RuntimePolicy::untrusted(sandbox.clone()));

    let err = untrusted
        .call_native_by_name("Subprocess_run", &[])
        .expect_err("untrusted VM must deny shell execution");
    assert!(err.contains(SECURITY_DENIAL_CODE), "{}", err);
    assert!(err.contains("PROCESS"), "{}", err);
    assert!(err.contains("Subprocess_run"), "{}", err);

    let err = untrusted
        .call_native_by_name("Os_name", &[])
        .expect_err("untrusted VM must deny system-control natives");
    assert!(err.contains(SECURITY_DENIAL_CODE), "{}", err);
    assert!(err.contains("SYSTEM_CONTROL"), "{}", err);

    let mut trusted = Vm::new();
    let value = trusted
        .call_native_by_name("Os_name", &[])
        .expect("trusted VM must allow the same native");
    assert!(matches!(value, Value::String(_)));

    std::fs::remove_dir_all(&sandbox).ok();
}

#[test]
fn vm_dispatch_confines_relative_paths_only_when_untrusted() {
    let sandbox = root("vm_paths");
    std::fs::write(sandbox.join("note.txt"), "inside").expect("write in-sandbox note");
    let mut untrusted = Vm::new();
    untrusted.set_policy(RuntimePolicy::untrusted(sandbox.clone()));

    let escape = Value::String(Rc::new("../../secret.txt".to_string()));
    let err = untrusted
        .call_native_by_name("File_readFile", &[escape.clone()])
        .expect_err("untrusted VM must deny path escape");
    assert!(err.contains(SECURITY_DENIAL_CODE), "{}", err);
    assert!(err.contains("FILESYSTEM_READ"), "{}", err);

    let note = Value::String(Rc::new("note.txt".to_string()));
    let value = untrusted
        .call_native_by_name("File_readFile", &[note])
        .expect("in-sandbox read must remain allowed");
    assert_eq!(value, Value::String(Rc::new("inside".to_string())));

    let mut trusted = Vm::new();
    let value = trusted
        .call_native_by_name("File_readFile", &[escape])
        .expect("trusted VM must not apply confinement");
    assert!(matches!(value, Value::String(_)));

    std::fs::remove_dir_all(&sandbox).ok();
}
