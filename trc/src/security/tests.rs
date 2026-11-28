//! In-file tests: every denied capability named in ROADMAP §10.

use super::audit::AuditDecision;
use super::capability::Capability;
use super::diagnostic::{CapabilityDenied, NativeCallSite, SECURITY_DENIAL_CODE};
use super::policy::{ResourceBudget, SecurityEnforcer, SecurityPolicy};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_ROOT: AtomicUsize = AtomicUsize::new(0);

fn sandbox_root(label: &str) -> PathBuf {
    let sequence = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "titrate_security_{}_{}_{}",
        std::process::id(),
        label,
        sequence
    ));
    std::fs::create_dir_all(&root).expect("create sandbox root");
    root
}

fn site(name: &str) -> NativeCallSite {
    NativeCallSite::new(name)
}

fn untrusted(label: &str) -> (PathBuf, SecurityEnforcer) {
    let root = sandbox_root(label);
    let enforcer = SecurityEnforcer::new(SecurityPolicy::untrusted(root.clone()));
    (root, enforcer)
}

#[test]
fn untrusted_denies_process_native() {
    let (_root, mut enforcer) = untrusted("process");
    for (name, expected) in [
        ("Subprocess_run", Capability::Process),
        ("Process_spawn", Capability::Process),
        ("Sys_exit", Capability::SystemControl),
    ] {
        let denial = enforcer
            .enforce_native(site(name))
            .expect_err("process native must be denied");
        assert_eq!(denial.capability_name(), expected.name());
        assert_eq!(denial.call_site().native_name, name);
    }
}

#[test]
fn untrusted_denies_network_native() {
    let (_root, mut enforcer) = untrusted("network");
    for name in ["Net_connect", "Http_get", "Socket_new", "UdpSocket_new"] {
        let denial = enforcer
            .enforce_native(site(name))
            .expect_err("network native must be denied");
        assert_eq!(denial.capability_name(), Capability::Network.name());
        assert_eq!(denial.call_site().native_name, name);
    }
}

#[test]
fn untrusted_denies_ffi_native() {
    let (_root, mut enforcer) = untrusted("ffi");
    for name in ["Ctypes_dlopen", "Ctypes_call"] {
        let denial = enforcer
            .enforce_native(site(name))
            .expect_err("FFI native must be denied");
        assert_eq!(denial.capability_name(), Capability::Ffi.name());
        assert_eq!(denial.call_site().native_name, name);
    }
}

#[test]
fn untrusted_rejects_lexical_filesystem_escape() {
    let (root, mut enforcer) = untrusted("lexical");
    let denial = enforcer
        .enforce_path(site("File_readFile"), Path::new("../../etc/passwd"))
        .expect_err("parent traversal must be denied");
    assert_eq!(denial.capability_name(), Capability::FileSystemRead.name());
    assert_eq!(denial.call_site().native_name, "File_readFile");

    let allowed = enforcer
        .enforce_path(site("File_readFile"), Path::new("/etc/passwd"))
        .expect("absolute request is interpreted relative to the sandbox root");
    assert!(allowed.starts_with(std::fs::canonicalize(&root).expect("canonical root")));

    std::fs::remove_dir_all(&root).ok();
}

#[test]
#[cfg(unix)]
fn untrusted_rejects_symlink_escape() {
    let root = sandbox_root("symlink");
    let outside = sandbox_root("symlink_outside");
    std::fs::write(outside.join("secret.txt"), "outside").expect("write outside file");
    std::os::unix::fs::symlink(&outside, root.join("escape")).expect("create symlink");

    let mut enforcer = SecurityEnforcer::new(SecurityPolicy::untrusted(root.clone()));
    let denial = enforcer
        .enforce_path(site("File_readFile"), Path::new("escape/secret.txt"))
        .expect_err("symlink escape must be denied");
    assert_eq!(denial.capability_name(), Capability::FileSystemRead.name());
    assert!(denial.to_string().contains("resolves outside sandbox root"));

    std::fs::remove_dir_all(&root).ok();
    std::fs::remove_dir_all(&outside).ok();
}

#[test]
fn untrusted_denies_resource_exhaustion() {
    let root = sandbox_root("budget");
    let policy = SecurityPolicy::untrusted(root.clone()).with_budgets(ResourceBudget {
        max_steps: 10,
        max_output_bytes: 4,
        max_allocation: 8,
    });
    let mut enforcer = SecurityEnforcer::new(policy);

    let allocation = enforcer
        .enforce_allocation(site("ArrayList_add"), 9)
        .expect_err("oversized allocation must be denied");
    assert_eq!(allocation.capability_name(), Capability::ResourceBudget.name());

    let steps = enforcer
        .enforce_steps(site("loop"), 11)
        .expect_err("step overflow must be denied");
    assert_eq!(steps.capability_name(), Capability::ResourceBudget.name());

    let output = enforcer
        .enforce_output(site("println"), 5)
        .expect_err("output overflow must be denied");
    assert_eq!(output.capability_name(), Capability::ResourceBudget.name());

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn trusted_mode_is_unchanged() {
    let mut enforcer = SecurityEnforcer::new(SecurityPolicy::trusted());
    for name in ["Subprocess_run", "Net_connect", "Ctypes_dlopen", "WinReg_OpenKey"] {
        assert!(
            enforcer.enforce_native(site(name)).is_ok(),
            "trusted mode must allow {}",
            name
        );
    }
    let requested = Path::new("/etc/passwd");
    assert_eq!(
        enforcer
            .enforce_path(site("File_readFile"), requested)
            .expect("trusted mode must not confine paths"),
        requested
    );
    assert!(enforcer
        .enforce_allocation(site("ArrayList_add"), usize::MAX)
        .is_ok());
    assert!(enforcer.enforce_steps(site("loop"), u64::MAX).is_ok());
    assert!(enforcer.enforce_output(site("println"), usize::MAX).is_ok());
}

#[test]
fn audit_log_records_denials() {
    let (_root, mut enforcer) = untrusted("audit");
    assert!(enforcer.enforce_native(site("Net_connect")).is_err());
    assert!(enforcer.enforce_native(site("println")).is_ok());

    let audit = enforcer.audit();
    assert_eq!(audit.denied_count(), 1);
    assert_eq!(audit.allowed_count(), 1);
    let event = &audit.events()[0];
    assert_eq!(event.decision, AuditDecision::Denied);
    assert_eq!(event.capability, Capability::Network);
    assert_eq!(event.call_site.native_name, "Net_connect");
    assert!(audit.to_json_array().contains("\"NETWORK\""));
}

#[test]
fn denial_diagnostic_names_capability_and_call_site() {
    let denial = CapabilityDenied::new(
        Capability::Network,
        NativeCallSite::with_location("Net_connect", "Net", "connect", 12, 4),
        "network denied",
    );
    assert_eq!(denial.code, SECURITY_DENIAL_CODE);
    assert_eq!(denial.capability_name(), "NETWORK");
    let display = denial.to_string();
    assert!(display.contains("error[E5001]"));
    assert!(display.contains("'NETWORK'"));
    assert!(display.contains("Net.connect (Net_connect) at 12:4"));
    let json = denial.to_json();
    assert!(json.contains("\"code\":\"E5001\""));
    assert!(json.contains("\"capability\":\"NETWORK\""));
    assert!(json.contains("\"call_site\":\"Net.connect (Net_connect) at 12:4\""));
}

#[test]
fn registry_override_classifies_host_native() {
    let root = sandbox_root("registry");
    let mut enforcer = SecurityEnforcer::new(SecurityPolicy::untrusted(root.clone()));
    enforcer
        .registry_mut()
        .register("Host_added_call", Capability::Network);
    let denial = enforcer
        .enforce_native(site("Host_added_call"))
        .expect_err("host-added native must inherit the registered capability");
    assert_eq!(denial.capability_name(), Capability::Network.name());
    std::fs::remove_dir_all(&root).ok();
}
