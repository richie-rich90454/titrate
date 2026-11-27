//! Integration test harness for Titrate stdlib `.tr` test files.
//!
//! Walks `stdlib_test/src/tests/` recursively and, for each `.tr` file:
//!   1. Reads the source.
//!   2. Injects a synthetic `public fn main(): void` that calls every
//!      `run_all_*_tests()` aggregator declared in the file (or, if none
//!      exist, every public parameterless `test_*()` function).
//!   3. Compiles + runs via the bytecode VM (`compile_with_modules`).
//!   4. Asserts that no `FAIL:` line appears and at least one `PASS` marker is
//!      emitted in the captured stdout.
//!
//! Multi-file module resolution uses the project root as `root_dir` so that
//! `import tt::lang::String;` resolves to `lib/tt/lang/String.tr`. The
//! semantic analyzer is skipped (mirroring `mega_test_02.rs` / `mega_test_03.rs`)
//! because the compiler's own module system handles imported symbols.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::Arc;
use std::sync::mpsc;
use std::sync::OnceLock;
use std::thread;
use std::time::{Duration, Instant};

pub mod native_capability;
mod test_sandbox;

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
const TESTS_DIR: &str = "../tests/stdlib/src/tests";

/// Directory containing binary/text fixtures referenced by stdlib tests.
const FIXTURES_DIR: &str = "../tests/fixtures/stdlib";

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
fn build_synthetic_main(source: &str) -> Result<String, String> {
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

    if calls.is_empty() {
        return Err(
            "no run_all_*_tests() or public test_*() functions found; refusing to inject an empty main"
                .to_string(),
        );
    }

    let body: String = calls
        .iter()
        .map(|c| format!("    {}();", c))
        .collect::<Vec<_>>()
        .join("\n");

    Ok(format!("\n\npublic fn main(): void {{\n{}\n}}\n", body))
}

/// A file only passes when its tests actually emitted a PASS marker. Fixtures
/// use both `PASS: ...` and `PASS ...` forms. Files defining their own `main`
/// have custom output contracts (for example `test_fork.tr`) and are checked
/// by their own harnesses.
fn has_pass_marker(path: &Path, output: &str) -> bool {
    let has_own_main = fs::read_to_string(path)
        .is_ok_and(|source| has_main_re().is_match(&source));
    has_own_main
        || output.lines().any(|line| {
            let line = line.trim_start();
            line.starts_with("PASS:") || line.starts_with("PASS ")
        })
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
        let main = build_synthetic_main(&source).map_err(|e| format!("synthetic main: {}", e))?;
        format!("{}\n{}", source, main)
    };

    let tokens = lexer::tokenize(&full_source).map_err(|e| format!("lexer: {}", e))?;
    let ast = parser::parse(tokens).map_err(|e| format!("parser: {}", e))?;

    let root_dir = PathBuf::from(PROJECT_ROOT);
    let mut compiler = bytecode::Compiler::new();
    let compiled = compiler
        .compile_with_modules(&ast, &root_dir)
        .map_err(|e| format!("compiler: {}", e))?;

    let label = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("test");
    let stem = path.file_stem().and_then(|n| n.to_str()).unwrap_or("case");
    let sandbox = test_sandbox::isolated_copy(Path::new(FIXTURES_DIR), &format!("{}_{}", label, stem))?;

    let mut vm = bytecode::Vm::new();
    vm.set_working_dir(sandbox.clone());
    // keep module resolution root
    let _ = root_dir;
    vm.load_program(compiled);
    let result = vm.run().map_err(|e| format!("vm: {}", e));
    test_sandbox::cleanup(&sandbox);
    result?;

    Ok(vm.output.join("\n"))
}

/// Run `run_test_file` in a spawned thread with a hard timeout. If the thread
/// does not produce a result within `PER_FILE_TIMEOUT_SECS`, it is recorded
/// as a timeout failure. (The orphaned thread continues until the test process
/// exits — Rust has no thread cancellation — but it no longer blocks the run.)
fn run_test_file_with_timeout(path: &Path) -> Result<String, String> {
    let (tx, rx) = mpsc::channel::<Result<String, String>>();
    let owned = path.to_path_buf();
    // Give each worker a generous stack; some tests exercise deeply recursive
    // parser/compiler/VM paths. Catch panics so one bad file doesn't abort the
    // whole harness.
    let builder = thread::Builder::new()
        .name(owned.file_stem().unwrap_or_default().to_string_lossy().to_string())
        .stack_size(16 * 1024 * 1024);
    builder.spawn(move || {
        let result = std::panic::catch_unwind(|| run_test_file(&owned));
        let _ = tx.send(match result {
            Ok(r) => r,
            Err(payload) => {
                let msg = if let Some(s) = payload.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = payload.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "worker thread panicked".to_string()
                };
                Err(format!("panic: {}", msg))
            }
        });
    }).map_err(|e| format!("failed to spawn worker: {}", e))?;
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
        files.len() >= 581,
        "expected at least 581 test files, found {} under {}",
        files.len(),
        tests_dir.display()
    );

    // Run test files in parallel to speed up the suite. Each file is independent
    // (own VM instance, own working dir), so N workers is safe. Results are
    // collected and sorted by path for deterministic reporting.
    let num_workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(8);
    let files = Arc::new(files);
    let (tx, rx) = mpsc::channel::<(PathBuf, Result<String, String>)>();

    let mut handles = Vec::new();
    for w in 0..num_workers {
        let files = files.clone();
        let tx = tx.clone();
        let handle = std::thread::Builder::new()
            .name(format!("stdlib-worker-{}", w))
            .stack_size(16 * 1024 * 1024)
            .spawn(move || {
                let mut idx = w;
                while idx < files.len() {
                    let file = files[idx].clone();
                    let result = run_test_file_with_timeout(&file);
                    let _ = tx.send((file, result));
                    idx += num_workers;
                }
            })
            .expect("spawn worker");
        handles.push(handle);
    }
    drop(tx);

    let mut results: Vec<(PathBuf, Result<String, String>)> = rx.iter().collect();
    for h in handles {
        let _ = h.join();
    }
    results.sort_by(|a, b| a.0.cmp(&b.0));

    let mut passed = 0usize;
    let mut compile_failures: Vec<(PathBuf, String)> = Vec::new();
    let mut test_failures: Vec<(PathBuf, String)> = Vec::new();

    for (i, (file, result)) in results.iter().enumerate() {
        eprintln!("[{}/{}] {} ...", i + 1, results.len(), file.display());
        match result {
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
                } else if !has_pass_marker(file, output) {
                    test_failures.push((
                        file.clone(),
                        "no PASS marker in output; tests may not have run".to_string(),
                    ));
                } else {
                    passed += 1;
                    eprintln!("PASS: {}", file.display());
                }
            }
            Err(e) => {
                compile_failures.push((file.clone(), e.clone()));
            }
        }
    }

    // Pass counts are stdout-based: a file counts only if its captured stdout has no FAIL line and at least one PASS marker.
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
            msg.push_str("\n--- COMPILE/RUN ERRORS (all) ---\n");
            for (path, e) in compile_failures.iter().take(500) {
                msg.push_str(&format!("\n[{}]\n{}\n", path.display(), e));
            }
        }

        if !test_failures.is_empty() {
            msg.push_str("\n--- TEST FAILURES (FAIL: lines, all) ---\n");
            for (path, e) in test_failures.iter().take(500) {
                msg.push_str(&format!("\n[{}]\n{}\n", path.display(), e));
            }
        }

        // Write full error report to a file for analysis, then panic.
        let _ = std::fs::write("stdlib_failures_detail.txt", &msg);
        panic!("{}", msg);
    }
}

// ---------------------------------------------------------------------------
// Native (LLVM) backend test
// ---------------------------------------------------------------------------

/// Spawn `cmd`, wait up to `secs`, and collect its output. A hung child is
/// killed and reported as a failure so one blocking fixture cannot stall the
/// whole native harness.
fn run_with_timeout(cmd: &mut Command, secs: u64) -> Result<Output, String> {
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn: {}", e))?;
    let deadline = Instant::now() + Duration::from_secs(secs);
    loop {
        return match child.try_wait() {
            Ok(Some(_)) => child.wait_with_output().map_err(|e| format!("wait: {}", e)),
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("timeout after {}s", secs));
                }
                thread::sleep(Duration::from_millis(25));
                continue;
            }
            Err(e) => Err(format!("try_wait: {}", e)),
        }
    }
}

// ---------------------------------------------------------------------------
// Failure inventory (opt-in diagnostic)
// ---------------------------------------------------------------------------

/// Root-cause buckets from `ROADMAP.md` Phase 2. `Other` keeps the inventory
/// honest for harness-level failures (read errors, synthetic-main errors,
/// timeouts without captured output) that fit no codegen bucket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FailureCategory {
    UnsupportedCallTarget,
    UnsupportedIndex,
    ContainerTypeMismatch,
    MissingVtableMethod,
    HardCrash,
    Other,
}

impl FailureCategory {
    fn label(self) -> &'static str {
        match self {
            Self::UnsupportedCallTarget => "unsupported call target for imported module function",
            Self::UnsupportedIndex => "unsupported Index expression on container result",
            Self::ContainerTypeMismatch => "container type mismatch { i64, ptr } -> { ptr, ptr }",
            Self::MissingVtableMethod => "method missing from generic-class vtable",
            Self::HardCrash => "hard crash (access violation, stack overflow, LLVM signature assertion)",
            Self::Other => "other",
        }
    }
}

/// Classify a native failure by the signatures it emitted. Crashes are checked
/// first because they are the safety priority; the rest follow the roadmap
/// order, with vtable gaps before cast failures so a generic vtable miss is not
/// hidden by a downstream conversion error.
fn categorize_failure(text: &str) -> FailureCategory {
    let lower = text.to_ascii_lowercase();

    if lower.contains("llvm error")
        || lower.contains("bad signature")
        || lower.contains("signature assertion")
        || lower.contains("c++ assertion")
        || lower.contains("assertion failed")
        || lower.contains("access violation")
        || lower.contains("stack overflow")
        || lower.contains("native binary exited")
        || lower.contains("0xc0000005")
        || lower.contains("segfault")
        || lower.contains("native run: timeout")
        || lower.contains("native compile: timeout")
    {
        return FailureCategory::HardCrash;
    }
    if lower.contains("unsupported call target")
        || lower.contains("unsupported static call")
        || lower.contains("unresolved external symbol")
        || lower.contains("lnk1120")
        || lower.contains("lnk2019")
    {
        return FailureCategory::UnsupportedCallTarget;
    }
    if lower.contains("unsupported index")
        || (lower.contains("unsupported expression") && lower.contains("index"))
    {
        return FailureCategory::UnsupportedIndex;
    }
    if lower.contains("vtable") || lower.contains("missing method") {
        return FailureCategory::MissingVtableMethod;
    }
    if lower.contains("cannot convert value of type")
        || lower.contains("cast failed")
        || lower.contains("bitcast failed")
        || lower.contains("bit_cast failed")
        || lower.contains("{i64, ptr}")
        || lower.contains("{ptr, ptr}")
        || lower.contains("{ i64, ptr }")
        || lower.contains("{ ptr, ptr }")
    {
        return FailureCategory::ContainerTypeMismatch;
    }
    FailureCategory::Other
}

/// One native harness failure. `detail` is the exact string rendered by the
/// existing panic message and `STDLIB_NATIVE_DUMP`; stdout/stderr are held
/// separately so the opt-in inventory can write complete output without
/// changing those reports.
struct NativeFailure {
    path: PathBuf,
    detail: String,
    stdout: String,
    stderr: String,
}

impl NativeFailure {
    fn new(path: &Path, detail: String) -> Self {
        Self {
            path: path.to_path_buf(),
            detail,
            stdout: String::new(),
            stderr: String::new(),
        }
    }

    fn with_output(path: &Path, detail: String, stdout: &str, stderr: &str) -> Self {
        Self {
            path: path.to_path_buf(),
            detail,
            stdout: stdout.to_string(),
            stderr: stderr.to_string(),
        }
    }

    fn classify(&self) -> FailureCategory {
        let text = format!("{}\n{}\n{}", self.detail, self.stdout, self.stderr);
        categorize_failure(&text)
    }
}

/// Write every failure with its root-cause category and captured output.
/// Called only when `TITRATE_NATIVE_FAILURE_LOG` is set.
fn write_failure_log(path: &Path, failures: &[NativeFailure]) -> std::io::Result<()> {
    let mut file = fs::File::create(path)?;
    writeln!(file, "# native stdlib failure inventory")?;
    writeln!(file, "total failures: {}", failures.len())?;
    for (i, failure) in failures.iter().enumerate() {
        let category = failure.classify().label();
        writeln!(
            file,
            "\n=== [{}/{}] {} [{}] ===",
            i + 1,
            failures.len(),
            failure.path.display(),
            category
        )?;
        writeln!(file, "category: {}", category)?;
        writeln!(file, "--- detail ---\n{}", failure.detail)?;
        writeln!(file, "--- stdout ---\n{}", failure.stdout)?;
        writeln!(file, "--- stderr ---\n{}", failure.stderr)?;
    }
    Ok(())
}

/// Opt-in: write the complete inventory only when
/// `TITRATE_NATIVE_FAILURE_LOG` names a non-empty path.
fn maybe_write_failure_log(failures: &[NativeFailure]) {
    let log_path = std::env::var("TITRATE_NATIVE_FAILURE_LOG").unwrap_or_default();
    if log_path.is_empty() {
        return;
    }
    if let Err(e) = write_failure_log(Path::new(&log_path), failures) {
        eprintln!(
            "stdlib_runtest_native: failed to write failure log {}: {}",
            log_path, e
        );
    }
}

/// Run the same `.tr` test files through `trc --native` via subprocess, then
/// execute the produced native binary and check for `FAIL:` lines. Skips with
/// a warning when LLVM dev files or the debug `titrate_native` library are
/// absent.
#[test]
#[ignore = "native stdlib parity is incomplete (207/598); see HANDOFF.md section 6. Run with --ignored for a report."]
fn stdlib_runtest_native() {
    if !native_capability::native_ready(native_capability::LibProfile::Debug) {
        return;
    }
    let trc = trc_binary().expect(
        "trc binary not found; run `cargo build -p trc` first",
    );

    let tests_dir = PathBuf::from(TESTS_DIR);
    let files = discover_test_files(&tests_dir);
    assert!(!files.is_empty(), "no test files found");

    // Scratch sources must live under the workspace so `trc` can resolve
    // `tt::` imports against the root `lib/` directory.
    let tmp = workspace_root().join("target").join("trc_stdlib_runtest_native");
    let _ = fs::create_dir_all(&tmp);

    let mut passed = 0usize;
    let mut failures: Vec<NativeFailure> = Vec::new();
    let dump_path = std::env::var("STDLIB_NATIVE_DUMP").unwrap_or_default();
    let mut dump_file = if dump_path.is_empty() {
        None
    } else {
        std::fs::File::create(&dump_path).ok()
    };
    let mut last_dumped = 0usize;

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
                failures.push(NativeFailure::new(file, format!("read error: {}", e)));
                continue;
            }
        };
        let full_source = if has_main_re().is_match(&source) {
            source
        } else {
            match build_synthetic_main(&source) {
                Ok(main) => format!("{}\n{}", source, main),
                Err(e) => {
                    failures.push(NativeFailure::new(file, format!("synthetic main: {}", e)));
                    continue;
                }
            }
        };
        if fs::write(&tmp_src, &full_source).is_err() {
            failures.push(NativeFailure::new(file, "failed to write temp file".to_string()));
            continue;
        }

        // Invoke `trc --native <tmp_src>`. These fixtures are legacy
        // multi-file programs that were never checked by the strict
        // import-aware analyzer; `--compat` keeps this harness focused on
        // native code generation and execution.
        let mut compile_cmd = Command::new(&trc);
        compile_cmd
            .arg("--native")
            .arg("--compat")
            .arg(tmp_src.to_str().unwrap());
        let compile_output = match run_with_timeout(&mut compile_cmd, 30) {
            Ok(o) => o,
            Err(e) => {
                failures.push(NativeFailure::new(file, format!("native compile: {}", e)));
                continue;
            }
        };
        if !compile_output.status.success() {
            let stdout = String::from_utf8_lossy(&compile_output.stdout);
            let stderr = String::from_utf8_lossy(&compile_output.stderr);
            failures.push(NativeFailure::with_output(
                file,
                format!("trc --native failed:\nstdout: {}\nstderr: {}", stdout, stderr),
                &stdout,
                &stderr,
            ));
            continue;
        }

        // `trc --native` writes `<stem>_native[.exe]` beside the source.
        let tmp_stem = tmp_src
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("test");
        let native_exe = if cfg!(windows) {
            tmp.join(format!("{}_native.exe", tmp_stem))
        } else {
            tmp.join(format!("{}_native", tmp_stem))
        };
        let mut run_cmd = Command::new(&native_exe);
        let run_output = match run_with_timeout(&mut run_cmd, 10) {
            Ok(o) => o,
            Err(e) => {
                let _ = fs::remove_file(&native_exe);
                let _ = fs::remove_file(&tmp_src);
                failures.push(NativeFailure::new(file, format!("native run: {}", e)));
                continue;
            }
        };
        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let stderr = String::from_utf8_lossy(&run_output.stderr);
        if !run_output.status.success() {
            failures.push(NativeFailure::with_output(
                file,
                format!(
                    "native binary exited {:?}\nstdout: {}\nstderr: {}",
                    run_output.status.code(),
                    stdout,
                    stderr
                ),
                &stdout,
                &stderr,
            ));
        } else if stdout.contains("FAIL:") {
            let sample: Vec<&str> = stdout
                .lines()
                .filter(|l| l.contains("FAIL:"))
                .take(3)
                .collect();
            failures.push(NativeFailure::with_output(
                file,
                format!("FAIL lines:\n{}", sample.join("\n")),
                &stdout,
                &stderr,
            ));
        } else if !has_pass_marker(file, &stdout) {
            failures.push(NativeFailure::with_output(
                file,
                "native binary produced no PASS marker".to_string(),
                &stdout,
                &stderr,
            ));
        } else {
            passed += 1;
        }

        // Stream failures to the diagnostic dump so a long run can be
        // analysed even if it is interrupted.
        if failures.len() > last_dumped {
            if let Some(ref mut f) = dump_file {
                for failure in &failures[last_dumped..] {
                    let _ = writeln!(f, "[{}]\n{}\n", failure.path.display(), failure.detail);
                    let _ = f.flush();
                }
            }
            last_dumped = failures.len();
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

    // Optional full diagnostic dump for triage. The panic message below is
    // intentionally capped at 20 entries; this writes every failure so the
    // native backend gaps can be grouped by root cause.
    if let Ok(dump_path) = std::env::var("STDLIB_NATIVE_DUMP") {
        let mut dump = String::new();
        for failure in &failures {
            dump.push_str(&format!(
                "[{}]\n{}\n\n",
                failure.path.display(),
                failure.detail
            ));
        }
        let _ = std::fs::write(&dump_path, dump);
    }

    // Complete failure inventory: every failing program with its root-cause
    // category and captured stdout/stderr, not just the 20 in the panic below.
    maybe_write_failure_log(&failures);

    if !failures.is_empty() {
        let mut msg = format!(
            "stdlib_runtest_native: {}/{} files failed\n",
            failures.len(),
            total
        );
        for failure in failures.iter().take(20) {
            msg.push_str(&format!(
                "\n[{}]\n{}\n",
                failure.path.display(),
                failure.detail
            ));
        }
        panic!("{}", msg);
    }
}

// ---------------------------------------------------------------------------
// Temporary single-file debug helper
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires STDLIB_RUNTEST_ONE env var to be set"]
fn stdlib_runtest_one() {
    let path_str = std::env::var("STDLIB_RUNTEST_ONE")
        .expect("set STDLIB_RUNTEST_ONE to a .tr path relative to stdlib_test/src/tests");
    let path = PathBuf::from(TESTS_DIR).join(&path_str);
    eprintln!("Running single file: {}", path.display());
    match run_test_file_with_timeout(&path) {
        Ok(output) => {
            eprintln!("{}", output);
            if output.contains("FAIL:") {
                panic!("FAIL lines detected");
            }
        }
        Err(e) => {
            eprintln!("ERROR: {}", e);
            panic!("{}", e);
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests for the failure inventory helpers
// ---------------------------------------------------------------------------

#[cfg(test)]
mod failure_inventory_tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_LOG_ID: AtomicU64 = AtomicU64::new(0);

    fn temp_log_path() -> PathBuf {
        let id = NEXT_LOG_ID.fetch_add(1, Ordering::Relaxed);
        Path::new("../target").join(format!(
            "native_failure_log_test_{}_{}.txt",
            std::process::id(),
            id
        ))
    }

    #[test]
    fn categorizes_unsupported_call_target() {
        assert_eq!(
            categorize_failure("codegen: unsupported static call: Mystery.frobnicate"),
            FailureCategory::UnsupportedCallTarget
        );
        assert_eq!(
            categorize_failure("codegen: unsupported call target expression"),
            FailureCategory::UnsupportedCallTarget
        );
        assert_eq!(
            categorize_failure("LNK2019: unresolved external symbol clone"),
            FailureCategory::UnsupportedCallTarget
        );
    }

    #[test]
    fn categorizes_unsupported_index_expression() {
        assert_eq!(
            categorize_failure("codegen: unsupported index target type 'ArrayList<int>'"),
            FailureCategory::UnsupportedIndex
        );
        assert_eq!(
            categorize_failure("codegen: unsupported expression: Index(obj, idx)"),
            FailureCategory::UnsupportedIndex
        );
    }

    #[test]
    fn categorizes_container_type_mismatch() {
        assert_eq!(
            categorize_failure(
                "codegen: cannot convert value of type '{ i64, ptr }' to '{ ptr, ptr }'"
            ),
            FailureCategory::ContainerTypeMismatch
        );
        assert_eq!(
            categorize_failure("build_bit_cast failed: {i64, ptr} vs {ptr, ptr}"),
            FailureCategory::ContainerTypeMismatch
        );
    }

    #[test]
    fn categorizes_missing_vtable_method() {
        assert_eq!(
            categorize_failure(
                "codegen: vtable for class 'Foo' is missing method 'bar'; emitting null entry"
            ),
            FailureCategory::MissingVtableMethod
        );
        assert_eq!(
            categorize_failure("method 'next' not found in vtable of 'Iterator'"),
            FailureCategory::MissingVtableMethod
        );
    }

    #[test]
    fn categorizes_hard_crashes() {
        for detail in [
            "native binary exited Some(-1073741819)\nstdout: \nstderr: ",
            "access violation reading address 0x0",
            "stack overflow",
            "LLVM ERROR: Calling a function with a bad signature!",
            "native run: timeout after 10s",
        ] {
            assert_eq!(
                categorize_failure(detail),
                FailureCategory::HardCrash,
                "unexpected category for: {}",
                detail
            );
        }
    }

    #[test]
    fn categorizes_unknown_as_other() {
        assert_eq!(
            categorize_failure("native binary produced no PASS marker"),
            FailureCategory::Other
        );
    }

    #[test]
    fn writes_every_failure_with_category_and_output() {
        let path = temp_log_path();
        let failures = vec![
            NativeFailure::new(Path::new("a.tr"), "read error: denied".to_string()),
            NativeFailure::with_output(
                Path::new("b.tr"),
                "codegen: unsupported static call: Mystery.frobnicate".to_string(),
                "PASS: before failure\n",
                "warning\n",
            ),
            NativeFailure::with_output(
                Path::new("c.tr"),
                "FAIL lines:\nFAIL: expected 1 got 2".to_string(),
                "FAIL: expected 1 got 2\n",
                "",
            ),
        ];

        write_failure_log(&path, &failures).expect("write failure log");
        let written = fs::read_to_string(&path).expect("read failure log");
        let _ = fs::remove_file(&path);

        assert!(
            written.contains("total failures: 3"),
            "missing total in:\n{}",
            written
        );
        for name in ["a.tr", "b.tr", "c.tr"] {
            assert!(written.contains(name), "missing {} in:\n{}", name, written);
        }
        assert!(written.contains("unsupported call target"));
        assert!(written.contains("warning"));
        assert!(written.contains("FAIL: expected 1 got 2"));
        assert_eq!(written.matches("--- stdout ---").count(), 3);
        assert_eq!(written.matches("--- stderr ---").count(), 3);
    }
}
