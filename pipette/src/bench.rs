use std::fs;
use std::path::{Path, PathBuf};

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
