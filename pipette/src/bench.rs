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
