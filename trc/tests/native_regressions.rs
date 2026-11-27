//! End-to-end regressions for syntax and runtime bugs that previously made
//! valid programs fail (or silently pass). Each test runs the program through
//! the bytecode VM and, where native codegen is involved, through
//! `trc --native --release` too, asserting the two agree.
//!
//! The native legs check for LLVM dev files and the release `titrate_native`
//! library at runtime and skip with a warning when either is absent.

pub mod native_capability;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

// Native links of distinct binaries have flaked before with LNK1104.
static LINK_LOCK: Mutex<()> = Mutex::new(());

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("trc lives inside the workspace")
        .to_path_buf()
}

fn trc_binary() -> PathBuf {
    let exe = if cfg!(windows) { "trc.exe" } else { "trc" };
    for profile in &["debug", "release"] {
        let path = workspace_root().join("target").join(profile).join(exe);
        if path.is_file() {
            return path;
        }
    }
    panic!("trc binary not found; run `cargo build -p trc` first");
}

/// Scratch directory under the workspace so import resolution can walk up to
/// the `lib/` stdlib tree.
fn scratch(name: &str) -> PathBuf {
    let dir = workspace_root().join("target").join(name);
    fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

fn normalize(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

struct RunResult {
    code: i32,
    stdout: String,
    stderr: String,
}

fn run_trc(dir: &Path, stem: &str, source: &str, native: bool, max_steps: Option<u64>) -> RunResult {
    let src_path = dir.join(format!("{}.tr", stem));
    fs::write(&src_path, source).expect("write repro source");

    let mut cmd = Command::new(trc_binary());
    if native {
        cmd.arg("--native").arg("--release");
    }
    if let Some(cap) = max_steps {
        cmd.env("TITRATE_MAX_STEPS", cap.to_string());
    }
    let out = cmd.arg(&src_path).current_dir(dir).output().expect("invoke trc");
    RunResult {
        code: out.status.code().unwrap_or(-1),
        stdout: normalize(&out.stdout),
        stderr: normalize(&out.stderr),
    }
}

fn vm_stdout(dir: &Path, stem: &str, source: &str) -> String {
    let result = run_trc(dir, stem, source, false, None);
    assert_eq!(
        result.code, 0,
        "bytecode VM failed\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );
    result.stdout.trim_end().to_string()
}

fn native_stdout(dir: &Path, stem: &str, source: &str) -> String {
    let _guard = LINK_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let result = run_trc(dir, stem, source, true, None);
    let exe_name = if cfg!(windows) {
        format!("{}_native.exe", stem)
    } else {
        format!("{}_native", stem)
    };
    let exe = dir.join(exe_name);
    let exe_exists = exe.is_file();
    let run = if result.code == 0 && exe_exists {
        Command::new(&exe).current_dir(dir).output().ok()
    } else {
        None
    };

    let _ = fs::remove_file(&exe);
    let _ = fs::remove_file(dir.join(format!("{}.tr", stem)));

    assert_eq!(
        result.code, 0,
        "native compile failed\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );
    assert!(exe_exists, "native binary was not produced at {}", exe.display());
    let run = run.expect("run native binary");
    assert!(
        run.status.success(),
        "native binary exited {:?}\nstdout: {}\nstderr: {}",
        run.status.code(),
        normalize(&run.stdout),
        normalize(&run.stderr)
    );
    normalize(&run.stdout).trim_end().to_string()
}

#[test]
fn error_propagation_before_subtraction_runs() {
    let source = r#"
fn f(): Result<int, string> { return Ok(41); }
fn compute(): Result<int, string> { return Ok(f()? - 1); }
public fn main(): void {
    let r = compute();
    switch (r) {
        case Ok(v) => io::println(Integer.toString(v));
        case Err(e) => io::println("err " + e);
        case _ => {}
    }
}
"#;
    let dir = scratch("rns_question_minus");
    let vm = vm_stdout(&dir, "question_minus", source);
    assert_eq!(vm, "40", "VM output: {}", vm);

    if !native_capability::native_ready(native_capability::LibProfile::Release) {
        return;
    }
    let native = native_stdout(&dir, "question_minus", source);
    assert_eq!(native, "40", "native output: {}", native);
}

#[test]
fn imported_program_with_use_after_move_is_rejected() {
    let source = r#"
import tt::lang::Integer;
public fn main(): void {
    let x: Owned<int> = Owned(5);
    let y = x;
    io::println(Integer.toString(x));
}
"#;
    let dir = scratch("rns_use_after_move");
    let result = run_trc(&dir, "use_after_move", source, false, None);
    let combined = format!("{}{}", result.stdout, result.stderr);
    assert_ne!(
        result.code, 0,
        "use-after-move in a program with imports must be rejected"
    );
    assert!(
        combined.to_lowercase().contains("move"),
        "expected a use-after-move diagnostic, got:\n{}",
        combined
    );
}

#[test]
fn step_cap_exceeded_is_fatal_and_not_catchable() {
    let source = r#"
public fn main(): void {
    try {
        while (true) {
        }
    } catch (e: string) {
        io::println("caught");
    }
    io::println("done");
}
"#;
    let dir = scratch("rns_step_cap");
    let result = run_trc(&dir, "step_cap", source, false, Some(10_000));
    let combined = format!("{}{}", result.stdout, result.stderr);
    assert_ne!(result.code, 0, "step-limit violation must fail the program");
    assert!(
        !combined.contains("caught"),
        "step-limit error was swallowed by try/catch:\n{}",
        combined
    );
    assert!(
        !combined.contains("done"),
        "execution continued past a fatal step-limit error:\n{}",
        combined
    );
    assert!(
        combined.to_lowercase().contains("step"),
        "expected a step-limit diagnostic, got:\n{}",
        combined
    );
}

#[test]
fn unsigned_right_shift_assignment_runs() {
    let source = r#"
public fn main(): void {
    var x: long = -2;
    x >>>= 1;
    io::println(Integer.toString(x));
}
"#;
    let dir = scratch("rns_unsigned_shr_assign");
    let vm = vm_stdout(&dir, "unsigned_shr_assign", source);
    assert_eq!(vm, "9223372036854775807", "VM output: {}", vm);

    if !native_capability::native_ready(native_capability::LibProfile::Release) {
        return;
    }
    let native = native_stdout(&dir, "unsigned_shr_assign", source);
    assert_eq!(native, "9223372036854775807", "native output: {}", native);
}
