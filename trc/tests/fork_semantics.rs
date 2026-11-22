//! Integration tests for the genuine fork() primitive (Os_fork / Pty.fork).
//!
//! `fork_parent_and_child_produce_distinct_output` runs a fresh, single-threaded
//! `trc` process so that a real fork is permitted; the in-process stdlib test
//! harness is multi-threaded and would (correctly) have the fork refused.

use trc::bytecode::value::Value;
use trc::bytecode::vm::natives::lookup_builtin_native;

#[cfg(target_os = "linux")]
use std::process::Command;

#[cfg(target_os = "linux")]
fn fork_test_source() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tests")
        .join("stdlib")
        .join("src")
        .join("tests")
        .join("os")
        .join("test_fork.tr")
}

#[test]
fn fork_refused_when_host_is_multithreaded() {
    use std::sync::{Arc, Barrier};

    // Keep an extra thread alive across the fork call so the guard has an
    // unambiguous signal regardless of the host platform.
    let barrier = Arc::new(Barrier::new(2));
    let helper_barrier = barrier.clone();
    let helper = std::thread::spawn(move || {
        helper_barrier.wait();
        std::thread::sleep(std::time::Duration::from_millis(150));
    });
    barrier.wait();

    let native = lookup_builtin_native("Os_fork").expect("Os_fork must be registered");
    let result = native(&[]).expect("Os_fork must not return an error");
    helper.join().unwrap();

    match result {
        Value::Int(-1) => {}
        Value::Int(v) => panic!("expected fork refusal (-1) with active thread, got {}", v),
        _ => panic!("expected Int result from Os_fork"),
    }
}

#[cfg(target_os = "linux")]
#[test]
fn fork_parent_and_child_produce_distinct_output() {
    let exe = env!("CARGO_BIN_EXE_trc");
    let src = fork_test_source();
    let output = Command::new(exe)
        .arg(src.to_str().unwrap())
        .output()
        .expect("failed to run trc bytecode VM");
    assert!(
        output.status.success(),
        "trc failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("FORK_PARENT_PID_POSITIVE"),
        "missing parent marker in output: {}",
        stdout
    );
    assert!(
        stdout.contains("FORK_CHILD_PID_ZERO"),
        "missing child marker in output: {}",
        stdout
    );
    assert!(
        !stdout.contains("FORK_REFUSED"),
        "fork was refused in the single-threaded host: {}",
        stdout
    );
}
