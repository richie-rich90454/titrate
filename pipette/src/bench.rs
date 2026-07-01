use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use trc::analyzer;
use trc::bytecode::{Compiler, Vm};
use trc::lexer;
use trc::parser;

use crate::project;

/// Find and run benchmark files (ending in `_bench.tr`).
pub fn bench(project_dir: &Path) -> Result<(), String> {
    let _cfg = project::load_config(project_dir)?;

    let src_dir = project_dir.join("src");
    let mut bench_files = Vec::new();
    collect_bench_files(&src_dir, &mut bench_files)?;

    if bench_files.is_empty() {
        println!("No benchmark files found (looking for *_bench.tr in src/)");
        return Ok(());
    }

    let mut passed = 0;
    let mut failed = 0;

    for bench_file in &bench_files {
        let rel = bench_file
            .strip_prefix(project_dir)
            .unwrap_or(bench_file)
            .display()
            .to_string();

        print!("  benchmark {} ... ", rel);

        match run_bench_file(bench_file, project_dir) {
            Ok(()) => {
                println!("ok");
                passed += 1;
            }
            Err(e) => {
                println!("FAILED");
                println!("    {}", e);
                failed += 1;
            }
        }
    }

    println!(
        "\n{} benchmark(s) passed, {} benchmark(s) failed",
        passed, failed
    );

    if failed > 0 {
        Err(format!("{} benchmark(s) failed", failed))
    } else {
        Ok(())
    }
}

/// Run benchmarks comparing native vs. bytecode performance.
///
/// When `compare_native` is true, each benchmark file is compiled both with
/// the bytecode VM and with the native LLVM backend (--native --release),
/// and the wall-clock times are reported side-by-side.
pub fn bench_compare_native(project_dir: &Path) -> Result<(), String> {
    let _cfg = project::load_config(project_dir)?;

    let src_dir = project_dir.join("src");
    let mut bench_files = Vec::new();
    collect_bench_files(&src_dir, &mut bench_files)?;

    if bench_files.is_empty() {
        println!("No benchmark files found (looking for *_bench.tr in src/)");
        return Ok(());
    }

    println!("Running native vs. bytecode comparison benchmarks\n");

    for bench_file in &bench_files {
        let rel = bench_file
            .strip_prefix(project_dir)
            .unwrap_or(bench_file)
            .display()
            .to_string();

        println!("--- {} ---", rel);
        match bench_native_vs_bytecode(bench_file, project_dir) {
            Ok(()) => {}
            Err(e) => {
                println!("    FAILED: {}", e);
            }
        }
        println!();
    }

    Ok(())
}

/// Benchmark an arbitrary program both natively and via the bytecode VM.
///
/// Unlike [`bench_compare_native`], which only iterates `*_bench.tr` files
/// in `src/`, this entry point accepts any `.tr` program path. It locates the
/// enclosing Titrate project (for module resolution) and delegates to
/// [`bench_native_vs_bytecode`] to reuse the existing timing logic.
///
/// Prints a comparison table with the bytecode time, native time, and the
/// speedup ratio (bytecode / native).
pub fn bench_native_vs_bytecode_path(path: &Path) -> Result<(), String> {
    let project_dir = project::find_project().ok_or_else(|| {
        "No Titrate.toml found in current or parent directories".to_string()
    })?;

    let rel = path
        .strip_prefix(&project_dir)
        .unwrap_or(path)
        .display()
        .to_string();
    println!("=== Native vs Bytecode Benchmark ===");
    println!("Program:        {}", rel);
    println!();

    bench_native_vs_bytecode(path, &project_dir)
}

/// Benchmark a single program both natively and via the bytecode VM.
///
/// Compiles the program with `--native --release`, runs the resulting binary
/// and times it, then runs the same program through the bytecode VM and times
/// that. Prints a comparison table.
pub fn bench_native_vs_bytecode(
    program_path: &Path,
    project_dir: &Path,
) -> Result<(), String> {
    // --- Bytecode timing ---
    let bytecode_time = {
        let source = fs::read_to_string(program_path)
            .map_err(|e| format!("Failed to read benchmark file: {}", e))?;
        let tokens = lexer::tokenize(&source)?;
        let ast = parser::parse(tokens)?;
        let typed_ast = analyzer::analyze(&ast).map_err(|errs| errs.join("\n"))?;
        let mut compiler = Compiler::new();
        let compiled = compiler.compile_with_modules(&typed_ast, project_dir)?;

        let start = Instant::now();
        let mut vm = Vm::new();
        vm.load_program(compiled);
        vm.run()?;
        start.elapsed()
    };

    // --- Native timing ---
    // Locate the trc binary.
    let trc_bin = find_trc_binary()
        .ok_or_else(|| "trc binary not found; run `cargo build -p trc` first".to_string())?;

    // Compile with --native --release.
    let compile_out = Command::new(&trc_bin)
        .arg("--native")
        .arg("--release")
        .arg(program_path.to_str().ok_or("invalid program path")?)
        .output()
        .map_err(|e| format!("failed to invoke trc --native: {}", e))?;

    if !compile_out.status.success() {
        return Err(format!(
            "trc --native failed: {}",
            String::from_utf8_lossy(&compile_out.stderr)
        ));
    }

    // The native binary is written next to the source file as <stem>_native.exe.
    let stem = program_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("invalid program file name")?;
    let exe_name = if cfg!(windows) {
        format!("{}_native.exe", stem)
    } else {
        format!("{}_native", stem)
    };
    let native_exe = program_path
        .parent()
        .map(|p| p.join(&exe_name))
        .unwrap_or_else(|| PathBuf::from(&exe_name));

    if !native_exe.is_file() {
        return Err(format!(
            "native binary was not produced at {}",
            native_exe.display()
        ));
    }

    // Run the native binary and time it.
    let native_time = {
        let start = Instant::now();
        let run_out = Command::new(&native_exe)
            .output()
            .map_err(|e| format!("failed to run native binary: {}", e))?;
        let elapsed = start.elapsed();
        if !run_out.status.success() {
            return Err(format!(
                "native binary exited with status {:?}: {}",
                run_out.status.code(),
                String::from_utf8_lossy(&run_out.stderr)
            ));
        }
        elapsed
    };

    // Clean up the native binary.
    let _ = fs::remove_file(&native_exe);

    // Print the comparison table.
    let speedup = bytecode_time.as_secs_f64() / native_time.as_secs_f64();
    println!(
        "{:<15} {:>12} {:>12} {:>10}",
        "", "native", "bytecode", "speedup"
    );
    println!(
        "{:<15} {:>12?} {:>12?} {:>9.2}x",
        "wall-clock", native_time, bytecode_time, speedup
    );

    Ok(())
}

/// Locate the built `trc` binary in `target/debug` or `target/release`.
fn find_trc_binary() -> Option<PathBuf> {
    // Walk up from the current directory looking for a `target` folder.
    let mut dir = std::env::current_dir().ok()?;
    for _ in 0..10 {
        for profile in &["release", "debug"] {
            let exe_name = if cfg!(windows) { "trc.exe" } else { "trc" };
            let candidate = dir.join("target").join(profile).join(exe_name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

fn collect_bench_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }
    let entries = fs::read_dir(dir).map_err(|e| format!("Failed to read directory: {}", e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();
        if path.is_dir() {
            collect_bench_files(&path, files)?;
        } else if let Some(name) = path.file_name() {
            let name = name.to_string_lossy();
            if name.ends_with("_bench.tr") {
                files.push(path);
            }
        }
    }
    Ok(())
}

fn run_bench_file(bench_file: &Path, project_dir: &Path) -> Result<(), String> {
    let source = fs::read_to_string(bench_file)
        .map_err(|e| format!("Failed to read benchmark file: {}", e))?;

    let tokens = lexer::tokenize(&source)?;
    let ast = parser::parse(tokens)?;
    let typed_ast = analyzer::analyze(&ast).map_err(|errs| errs.join("\n"))?;

    let mut compiler = Compiler::new();
    let compiled = compiler.compile_with_modules(&typed_ast, project_dir)?;

    let start = std::time::Instant::now();
    let mut vm = Vm::new();
    vm.load_program(compiled);
    vm.run()?;
    let elapsed = start.elapsed();

    for line in &vm.output {
        println!("{}", line);
    }

    println!("    completed in {:?}", elapsed);

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::bench_native_vs_bytecode_path;

    /// Minimal Titrate program: a tight loop that sums squares, mirroring the
    /// `SUM_SQUARES_SOURCE` used in `trc/tests/native_bench.rs`. It has no
    /// imports, so it compiles even when the project has no `lib/`.
    const BENCH_PATH_TEST_SOURCE: &str = r#"
public fn sumSquares(n: int): int {
    var sum: int = 0;
    var i: int = 0;
    while (i < n) {
        sum = sum + i * i;
        i = i + 1;
    }
    return sum;
}

public fn main(): void {
    let result: int = sumSquares(1000);
    io::println(result);
}
"#;

    /// RAII guard that restores the process working directory on drop, so the
    /// test never leaks a cwd change even if it panics.
    struct CwdGuard {
        prev: PathBuf,
    }

    impl CwdGuard {
        fn enter(new: &Path) -> Self {
            let prev =
                env::current_dir().expect("could not read current working directory");
            env::set_current_dir(new)
                .unwrap_or_else(|e| panic!("failed to change to {}: {}", new.display(), e));
            CwdGuard { prev }
        }
    }

    impl Drop for CwdGuard {
        fn drop(&mut self) {
            let _ = env::set_current_dir(&self.prev);
        }
    }

    /// Verifies that `bench_native_vs_bytecode_path` is wired with the
    /// expected signature and dispatches an arbitrary program path through
    /// both the bytecode VM (in-process) and the native LLVM backend
    /// (`trc --native --release`, subprocess).
    ///
    /// The test is `#[ignore]`'d because a full run requires:
    ///   1. LLVM development files (LLVM-C.lib) for the native backend.
    ///   2. A working system linker.
    ///   3. The `titrate_native` static library built
    ///      (`cargo build -p titrate_native`).
    ///   4. The `trc` binary present in `target/{debug,release}/`.
    ///
    /// Run with:
    ///   cargo test -p pipette --lib bench -- --ignored --nocapture
    #[test]
    #[ignore = "requires LLVM dev files, a system linker, and titrate_native built"]
    fn bench_native_vs_bytecode_path_dispatches_arbitrary_program() {
        // Locate the workspace root via CARGO_MANIFEST_DIR (pipette's own dir).
        let manifest_dir = PathBuf::from(
            env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set by cargo"),
        );
        let workspace_root = manifest_dir
            .parent()
            .expect("pipette should be nested inside the workspace")
            .to_path_buf();

        // Create a throwaway Titrate project under target/ (gitignored) so
        // that both `find_project()` (Titrate.toml) and `find_trc_binary()`
        // (walks up to find target/{debug,release}/trc) succeed when the cwd
        // is set to it.
        let temp_project = workspace_root
            .join("target")
            .join(format!("tmp_bench_path_test_{}", std::process::id()));

        // Remove any stale temp project from a previous run.
        let _ = fs::remove_dir_all(&temp_project);
        fs::create_dir_all(temp_project.join("src"))
            .expect("failed to create temp project src/");

        // Minimal valid Titrate.toml so `find_project()` identifies the dir.
        fs::write(
            temp_project.join("Titrate.toml"),
            "[package]\nname = \"bench_path_test\"\nversion = \"0.0.0\"\nentry = \"src/sum_squares.tr\"\n",
        )
        .expect("failed to write Titrate.toml");

        let program_path = temp_project.join("src").join("sum_squares.tr");
        fs::write(&program_path, BENCH_PATH_TEST_SOURCE)
            .expect("failed to write temp .tr program");

        // `bench_native_vs_bytecode_path` discovers both the project and the
        // `trc` binary from `std::env::current_dir()`, so run it from the temp
        // project and restore the previous cwd afterwards.
        let result = {
            let _guard = CwdGuard::enter(&temp_project);
            bench_native_vs_bytecode_path(&program_path)
            // `_guard` drops here, restoring the cwd even on panic.
        };

        // The cwd has been restored; safe to remove the temp project.
        let _ = fs::remove_dir_all(&temp_project);

        // With LLVM dev files and a built `trc` present, the dispatch should
        // complete successfully through both backends. The assertion verifies
        // the function signature/dispatch path end-to-end.
        assert!(
            result.is_ok(),
            "bench_native_vs_bytecode_path dispatch failed: {:?}",
            result.err()
        );
    }
}
