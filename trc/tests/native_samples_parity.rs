//! Native/VM output parity for every shipped sample program.
//!
//! For each eligible `.tr` file in `examples/` and `demos/`, this harness:
//!   1. runs the bytecode VM through `trc <file>`,
//!   2. copies the source into a scratch dir under `target/`, compiles it with
//!      `trc <file> --native --release`, and runs the produced executable,
//!   3. compares the two stdout streams after normalising line endings only.
//!
//! Both runs use the same working directory so file-I/O samples see identical
//! relative paths. The scratch dir lives inside the workspace so the compiler's
//! import resolver can walk up to the workspace `lib/` for `tt::` stdlib
//! modules, and so native linking finds `titrate_native` release lib.
//!
//! The test checks for LLVM dev files and the release `titrate_native` library
//! at runtime and skips with a warning when either is absent. Build the
//! prerequisites with:
//!   cargo build -p titrate_native --release

pub mod native_capability;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

// Linking is serialized across the whole run; parallel links of distinct
// binaries have flaked before with LNK1104 on Windows.
static LINK_LOCK: Mutex<()> = Mutex::new(());

const PER_PROGRAM_TIMEOUT_SECS: u64 = 30;

const EXCLUDED: &[(&str, &str)] = &[
    ("file_watcher.tr", "intentional infinite watcher"),
    ("http_server.tr", "blocks on a live server socket"),
    ("chat_server.tr", "blocks on a live server socket"),
    ("file_server.tr", "blocks on a live server socket"),
    ("async_fetch.tr", "requires live network access"),
    ("http_client.tr", "requires live network access"),
];

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("trc should be inside the workspace")
        .to_path_buf()
}

fn trc_binary() -> PathBuf {
    let root = workspace_root();
    let exe_name = if cfg!(windows) { "trc.exe" } else { "trc" };
    for profile in &["debug", "release"] {
        let candidate = root.join("target").join(profile).join(exe_name);
        if candidate.is_file() {
            return candidate;
        }
    }
    panic!("trc binary not found; run `cargo build -p trc` first");
}

/// Spawn `cmd`, wait up to `secs`, and return (status code, stdout, stderr).
/// Pipes are drained by `wait_with_output` after exit; samples print modest
/// amounts of output, so the pipe buffer never fills before termination.
fn run_captured(cmd: &mut Command, secs: u64) -> Result<(i32, Vec<u8>, Vec<u8>), String> {
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn: {}", e))?;
    let deadline = Instant::now() + Duration::from_secs(secs);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let out = child
                    .wait_with_output()
                    .map_err(|e| format!("wait_with_output: {}", e))?;
                return Ok((status.code().unwrap_or(-1), out.stdout, out.stderr));
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("timeout after {}s", secs));
                }
                thread::sleep(Duration::from_millis(25));
            }
            Err(e) => return Err(format!("try_wait: {}", e)),
        }
    }
}

fn normalize(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

fn collect_samples() -> Vec<PathBuf> {
    let root = workspace_root();
    let mut files = Vec::new();
    for dir in ["examples", "demos"] {
        let dir_path = root.join(dir);
        for entry in fs::read_dir(&dir_path).unwrap_or_else(|e| panic!("read {}: {}", dir, e)) {
            let path = entry.expect("dir entry").path();
            if path.extension().is_some_and(|ext| ext == "tr") {
                let name = path.file_name().and_then(|f| f.to_str()).unwrap_or_default();
                if EXCLUDED.iter().any(|(ex, _)| *ex == name) {
                    continue;
                }
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

fn run_vm(trc: &Path, source: &Path, cwd: &Path) -> Result<String, String> {
    let mut cmd = Command::new(trc);
    cmd.arg(source).current_dir(cwd);
    let (code, stdout, stderr) =
        run_captured(&mut cmd, PER_PROGRAM_TIMEOUT_SECS).map_err(|e| format!("vm: {}", e))?;
    if code != 0 {
        return Err(format!(
            "vm exit {}\nstdout: {}\nstderr: {}",
            code,
            normalize(&stdout),
            normalize(&stderr)
        ));
    }
    Ok(normalize(&stdout))
}

fn run_native(trc: &Path, source: &Path, cwd: &Path, stem: &str) -> Result<String, String> {
    let _guard = LINK_LOCK.lock().unwrap();

    let copy = cwd.join(format!("{}.tr", stem));
    fs::copy(source, &copy).map_err(|e| format!("copy source: {}", e))?;

    let mut compile = Command::new(trc);
    compile
        .arg("--native")
        .arg("--release")
        .arg(&copy)
        .current_dir(cwd);
    let (code, stdout, stderr) = run_captured(&mut compile, PER_PROGRAM_TIMEOUT_SECS)
        .map_err(|e| format!("native compile: {}", e))?;
    if code != 0 {
        let _ = fs::remove_file(&copy);
        return Err(format!(
            "native compile exit {}\nstdout: {}\nstderr: {}",
            code,
            normalize(&stdout),
            normalize(&stderr)
        ));
    }

    let exe_name = if cfg!(windows) {
        format!("{}_native.exe", stem)
    } else {
        format!("{}_native", stem)
    };
    let exe = cwd.join(exe_name);
    if !exe.is_file() {
        let _ = fs::remove_file(&copy);
        return Err(format!("native binary not produced at {}", exe.display()));
    }

    let mut run = Command::new(&exe);
    run.current_dir(cwd);
    let result = run_captured(&mut run, PER_PROGRAM_TIMEOUT_SECS)
        .map_err(|e| format!("native run: {}", e))
        .and_then(|(code, stdout, stderr)| {
            if code != 0 {
                Err(format!(
                    "native run exit {}\nstdout: {}\nstderr: {}",
                    code,
                    normalize(&stdout),
                    normalize(&stderr)
                ))
            } else {
                Ok(normalize(&stdout))
            }
        });

    let _ = fs::remove_file(&copy);
    let _ = fs::remove_file(&exe);
    result
}

#[test]
fn native_samples_match_vm_bytecode() {
    if !native_capability::native_ready(native_capability::LibProfile::Release) {
        return;
    }
    let root = workspace_root();
    let trc = trc_binary();
    let cwd = root.join("target").join("native_samples_parity");
    if cwd.exists() {
        fs::remove_dir_all(&cwd).expect("clean scratch dir");
    }
    fs::create_dir_all(&cwd).expect("create scratch dir");

    let files = collect_samples();
    assert!(!files.is_empty(), "no sample programs found");

    let mut failures = Vec::new();
    let mut passed = 0usize;
    for file in &files {
        let stem = file.file_stem().and_then(|s| s.to_str()).unwrap();
        let vm = match run_vm(&trc, file, &cwd) {
            Ok(out) => out,
            Err(e) => {
                failures.push(format!("{}: VM failed: {}", file.display(), e));
                continue;
            }
        };
        match run_native(&trc, file, &cwd, stem) {
            Ok(native) => {
                if native == vm {
                    passed += 1;
                } else {
                    failures.push(format!(
                        "{}: output mismatch\n--- VM ---\n{}\n--- NATIVE ---\n{}",
                        file.display(), vm, native
                    ));
                }
            }
            Err(e) => failures.push(format!("{}: {}", file.display(), e)),
        }
    }

    eprintln!(
        "native_samples_parity: {} of {} programs match the VM",
        passed, files.len()
    );
    assert!(
        failures.is_empty(),
        "{} sample(s) differ natively:\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}
