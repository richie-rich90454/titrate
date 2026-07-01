//! Integration test harness for Titrate stdlib `.tr` test files.
//!
//! Walks `stdlib_test/src/tests/` recursively and, for each `.tr` file:
//!   1. Reads the source.
//!   2. Injects a synthetic `public fn main(): void` that calls every
//!      `run_all_*_tests()` aggregator declared in the file (or, if none
//!      exist, every public parameterless `test_*()` function).
//!   3. Compiles + runs via the bytecode VM (`compile_with_modules`).
//!   4. Asserts that no `FAIL:` line appears in the captured stdout.
//!
//! Multi-file module resolution uses the project root as `root_dir` so that
//! `import tt::lang::String;` resolves to `lib/tt/lang/String.tr`. The
//! semantic analyzer is skipped (mirroring `mega_test_02.rs` / `mega_test_03.rs`)
//! because the compiler's own module system handles imported symbols.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;

/// Per-file timeout. Some test files exercise blocking APIs (TCP servers,
/// file watchers, thread joins) that can hang the VM; this keeps the harness
/// moving and records the hang as a failure.
const PER_FILE_TIMEOUT_SECS: u64 = 30;

use regex::Regex;
use trc::bytecode;
use trc::lexer;
use trc::parser;

/// Project root (where `lib/` lives). Cargo runs integration tests with the
/// working directory set to the crate root (`trc/`), so `..` is the workspace
/// root.
const PROJECT_ROOT: &str = "..";

/// Directory containing the `.tr` test files, relative to the crate root.
const TESTS_DIR: &str = "../stdlib_test/src/tests";

// ---------------------------------------------------------------------------
// Regex helpers (compiled once)
// ---------------------------------------------------------------------------

fn run_all_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"fn\s+(run_all_\w+_tests)\s*\(").unwrap())
}

fn test_fn_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"public\s+fn\s+(test_\w+)\s*\(\s*\)").unwrap())
}

fn has_main_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\bfn\s+main\s*\(").unwrap())
}

// ---------------------------------------------------------------------------
// File discovery
// ---------------------------------------------------------------------------

/// Recursively collect every `.tr` file under `dir`, sorted for determinism.
fn discover_test_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk(dir, &mut out);
    out.sort();
    out
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("tr") {
            out.push(path);
        }
    }
}

// ---------------------------------------------------------------------------
// Synthetic main injection
// ---------------------------------------------------------------------------

/// Build a synthetic `public fn main(): void { ... }` that calls every
/// `run_all_*_tests()` aggregator declared in the source. If none are found,
/// falls back to calling every public parameterless `test_*()` function.
fn build_synthetic_main(source: &str) -> String {
    let mut calls: Vec<String> = run_all_re()
        .captures_iter(source)
        .map(|c| c.get(1).unwrap().as_str().to_string())
        .collect();

    if calls.is_empty() {
        calls = test_fn_re()
            .captures_iter(source)
            .map(|c| c.get(1).unwrap().as_str().to_string())
            .collect();
    }

    // Dedup while preserving order.
    let mut seen = std::collections::HashSet::new();
    calls.retain(|c| seen.insert(c.clone()));

    let body: String = calls
        .iter()
        .map(|c| format!("    {}();", c))
        .collect::<Vec<_>>()
        .join("\n");

    format!("\n\npublic fn main(): void {{\n{}\n}}\n", body)
}

// ---------------------------------------------------------------------------
// Bytecode VM execution
// ---------------------------------------------------------------------------

/// Compile and run a single test file through the bytecode VM.
///
/// Returns the captured stdout as a single `String` (lines joined by `'\n'`),
/// or an error message describing where compilation/execution failed.
fn run_test_file(path: &Path) -> Result<String, String> {
    let source = fs::read_to_string(path).map_err(|e| format!("read error: {}", e))?;

    // Inject a synthetic main unless the file already defines one.
    let full_source = if has_main_re().is_match(&source) {
        source
    } else {
        format!("{}\n{}", source, build_synthetic_main(&source))
    };

    let tokens = lexer::tokenize(&full_source).map_err(|e| format!("lexer: {}", e))?;
    let ast = parser::parse(tokens).map_err(|e| format!("parser: {}", e))?;

    let root_dir = PathBuf::from(PROJECT_ROOT);
    let mut compiler = bytecode::Compiler::new();
    let compiled = compiler
        .compile_with_modules(&ast, &root_dir)
        .map_err(|e| format!("compiler: {}", e))?;

    let mut vm = bytecode::Vm::new();
    vm.set_working_dir(root_dir);
    vm.load_program(compiled);
    vm.run().map_err(|e| format!("vm: {}", e))?;

    Ok(vm.output.join("\n"))
}

/// Run `run_test_file` in a spawned thread with a hard timeout. If the thread
/// does not produce a result within `PER_FILE_TIMEOUT_SECS`, it is recorded
/// as a timeout failure. (The orphaned thread continues until the test process
/// exits — Rust has no thread cancellation — but it no longer blocks the run.)
fn run_test_file_with_timeout(path: &Path) -> Result<String, String> {
    let (tx, rx) = mpsc::channel::<Result<String, String>>();
    let owned = path.to_path_buf();
    thread::spawn(move || {
        let _ = tx.send(run_test_file(&owned));
    });
    match rx.recv_timeout(Duration::from_secs(PER_FILE_TIMEOUT_SECS)) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            Err(format!("timeout after {}s", PER_FILE_TIMEOUT_SECS))
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err("worker thread panicked".to_string())
        }
    }
}

// ---------------------------------------------------------------------------
// Workspace / binary helpers (for the native test)
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR should be set by cargo");
    PathBuf::from(manifest)
        .parent()
        .map(|p| p.to_path_buf())
        .expect("trc should be inside the workspace")
}

fn trc_binary() -> Option<PathBuf> {
    let root = workspace_root();
    let exe_name = if cfg!(windows) { "trc.exe" } else { "trc" };
    for profile in &["debug", "release"] {
        let candidate = root.join("target").join(profile).join(exe_name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Primary test: run every .tr file through the bytecode VM
// ---------------------------------------------------------------------------

#[test]
fn stdlib_runtest_all() {
    let tests_dir = PathBuf::from(TESTS_DIR);
    let files = discover_test_files(&tests_dir);
    assert!(
        files.len() >= 400,
        "expected ~444 test files, found {} under {}",
        files.len(),
        tests_dir.display()
    );

    let mut passed = 0usize;
    let mut compile_failures: Vec<(PathBuf, String)> = Vec::new();
    let mut test_failures: Vec<(PathBuf, String)> = Vec::new();

    for (i, file) in files.iter().enumerate() {
        eprintln!("[{}/{}] {} ...", i + 1, files.len(), file.display());
        match run_test_file_with_timeout(file) {
            Ok(output) => {
                if output.contains("FAIL:") {
                    let sample: Vec<&str> = output
                        .lines()
                        .filter(|l| l.contains("FAIL:"))
                        .take(5)
                        .collect();
                    test_failures.push((
                        file.clone(),
                        format!(
                            "{} FAIL line(s) (showing up to 5):\n{}",
                            output.matches("FAIL:").count(),
                            sample.join("\n")
                        ),
                    ));
                } else {
                    passed += 1;
                    eprintln!("PASS: {}", file.display());
                }
            }
            Err(e) => {
                compile_failures.push((file.clone(), e));
            }
        }
    }

    let total = files.len();
    let n_fail = compile_failures.len() + test_failures.len();
    eprintln!(
        "stdlib_runtest: {}/{} passed, {} failed ({} compile/run errors, {} FAIL lines)",
        passed,
        total,
        n_fail,
        compile_failures.len(),
        test_failures.len()
    );

    if n_fail > 0 {
        let mut msg = format!(
            "stdlib_runtest: {}/{} files failed ({} compile/run errors, {} with FAIL lines)\n",
            n_fail,
            total,
            compile_failures.len(),
            test_failures.len()
        );

        if !compile_failures.is_empty() {
            msg.push_str("\n--- COMPILE/RUN ERRORS (first 20) ---\n");
            for (path, e) in compile_failures.iter().take(20) {
                msg.push_str(&format!("\n[{}]\n{}\n", path.display(), e));
            }
        }

        if !test_failures.is_empty() {
            msg.push_str("\n--- TEST FAILURES (FAIL: lines, first 20) ---\n");
            for (path, e) in test_failures.iter().take(20) {
                msg.push_str(&format!("\n[{}]\n{}\n", path.display(), e));
            }
        }

        panic!("{}", msg);
    }
}

// ---------------------------------------------------------------------------
// Native (LLVM) backend test — ignored by default
// ---------------------------------------------------------------------------

/// Run the same `.tr` test files through `trc --native` via subprocess, then
/// execute the produced native binary and check for `FAIL:` lines.
///
/// Ignored by default because it requires LLVM dev files, a system linker,
/// and the `titrate_native` static library. Run with:
///   `cargo test --test stdlib_runtest -- --ignored stdlib_runtest_native`
#[test]
#[ignore = "requires LLVM dev files, a system linker, and titrate_native built"]
fn stdlib_runtest_native() {
    let trc = trc_binary().expect(
        "trc binary not found; run `cargo build -p trc` first",
    );

    let tests_dir = PathBuf::from(TESTS_DIR);
    let files = discover_test_files(&tests_dir);
    assert!(!files.is_empty(), "no test files found");

    let tmp = std::env::temp_dir().join("trc_stdlib_runtest_native");
    let _ = fs::create_dir_all(&tmp);

    let mut passed = 0usize;
    let mut failures: Vec<(PathBuf, String)> = Vec::new();

    for file in &files {
        let stem = file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("test");
        let tmp_src = tmp.join(format!("{}_{}.tr", stem, passed + failures.len()));

        // Write the source with an injected main.
        let source = match fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => {
                failures.push((file.clone(), format!("read error: {}", e)));
                continue;
            }
        };
        let full_source = if has_main_re().is_match(&source) {
            source
        } else {
            format!("{}\n{}", source, build_synthetic_main(&source))
        };
        if fs::write(&tmp_src, &full_source).is_err() {
            failures.push((file.clone(), "failed to write temp file".to_string()));
            continue;
        }

        // Invoke `trc --native <tmp_src>`.
        let compile_output = Command::new(&trc)
            .arg("--native")
            .arg(tmp_src.to_str().unwrap())
            .output();
        let compile_output = match compile_output {
            Ok(o) => o,
            Err(e) => {
                failures.push((file.clone(), format!("spawn error: {}", e)));
                continue;
            }
        };
        if !compile_output.status.success() {
            failures.push((
                file.clone(),
                format!(
                    "trc --native failed:\nstdout: {}\nstderr: {}",
                    String::from_utf8_lossy(&compile_output.stdout),
                    String::from_utf8_lossy(&compile_output.stderr),
                ),
            ));
            continue;
        }

        // The native binary is written next to the source file.
        let native_exe = if cfg!(windows) {
            tmp_src.with_extension("native.exe")
        } else {
            let mut p = tmp_src.clone();
            p.set_extension("native");
            p
        };
        let run_output = match Command::new(&native_exe).output() {
            Ok(o) => o,
            Err(e) => {
                failures.push((file.clone(), format!("native run error: {}", e)));
                continue;
            }
        };
        let stdout = String::from_utf8_lossy(&run_output.stdout);
        if stdout.contains("FAIL:") {
            let sample: Vec<&str> = stdout
                .lines()
                .filter(|l| l.contains("FAIL:"))
                .take(3)
                .collect();
            failures.push((
                file.clone(),
                format!("FAIL lines:\n{}", sample.join("\n")),
            ));
        } else {
            passed += 1;
        }

        // Clean up produced artifacts.
        let _ = fs::remove_file(&native_exe);
        let _ = fs::remove_file(&tmp_src);
    }

    let total = files.len();
    eprintln!(
        "stdlib_runtest_native: {}/{} passed, {} failed",
        passed,
        total,
        failures.len()
    );

    if !failures.is_empty() {
        let mut msg = format!(
            "stdlib_runtest_native: {}/{} files failed\n",
            failures.len(),
            total
        );
        for (path, e) in failures.iter().take(20) {
            msg.push_str(&format!("\n[{}]\n{}\n", path.display(), e));
        }
        panic!("{}", msg);
    }
}
