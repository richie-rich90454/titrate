use std::fs;
use std::path::{Path, PathBuf};

use trc::bytecode::{Compiler, Vm};
use trc::lexer;
use trc::parser;

use crate::project;

/// Find and run test files (ending in `_test.tr`) and `test_*` functions.
/// If `filter` is provided, only run tests whose file path contains the filter string.
pub fn test(project_dir: &Path, filter: Option<&str>) -> Result<(), String> {
    let _cfg = project::load_config(project_dir)?;

    // Collect test files from src/ and any subdirectories
    let src_dir = project_dir.join("src");
    let mut test_files = Vec::new();
    collect_test_files(&src_dir, &mut test_files)?;

    // Collect all .tr files that may contain test_* functions
    let mut all_tr_files = Vec::new();
    collect_tr_files(&src_dir, &mut all_tr_files)?;

    let mut passed = 0;
    let mut failed = 0;

    // Run test files (files ending in _test.tr)
    for test_file in &test_files {
        let rel = test_file
            .strip_prefix(project_dir)
            .unwrap_or(test_file)
            .display()
            .to_string();

        if let Some(f) = filter {
            if !rel.contains(f) && !test_file.to_string_lossy().contains(f) {
                continue;
            }
        }

        print!("  testing {} ... ", rel);

        match run_test_file(test_file, project_dir) {
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

    // Scan all .tr files for test_* functions and run them individually
    for tr_file in &all_tr_files {
        let rel = tr_file
            .strip_prefix(project_dir)
            .unwrap_or(tr_file)
            .display()
            .to_string();

        if let Some(f) = filter {
            if !rel.contains(f) && !tr_file.to_string_lossy().contains(f) {
                continue;
            }
        }

        let test_fns = extract_test_functions(tr_file)?;
        if test_fns.is_empty() {
            continue;
        }

        for test_fn in &test_fns {
            print!("  testing {}::{} ... ", rel, test_fn.name);

            match run_single_test(tr_file, &test_fn.name, project_dir) {
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
    }

    if test_files.is_empty() && all_tr_files.iter().all(|f| extract_test_functions(f).unwrap_or_default().is_empty()) {
        println!("No test files found (looking for *_test.tr or test_* functions in src/)");
        return Ok(());
    }

    println!(
        "\n{} test(s) passed, {} test(s) failed",
        passed, failed
    );

    if failed > 0 {
        Err(format!("{} test(s) failed", failed))
    } else {
        Ok(())
    }
}

fn collect_test_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }
    let entries = fs::read_dir(dir).map_err(|e| format!("Failed to read directory: {}", e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();
        if path.is_dir() {
            collect_test_files(&path, files)?;
        } else if let Some(name) = path.file_name() {
            let name = name.to_string_lossy();
            if name.ends_with("_test.tr") {
                files.push(path);
            }
        }
    }
    Ok(())
}

fn run_test_file(test_file: &Path, project_dir: &Path) -> Result<(), String> {
    let source = fs::read_to_string(test_file)
        .map_err(|e| format!("Failed to read test file: {}", e))?;

    let tokens = lexer::tokenize(&source)?;
    let ast = parser::parse(tokens)?;

    let mut compiler = Compiler::new();
    let compiled = compiler.compile_with_modules(&ast, project_dir)?;

    let mut vm = Vm::new();
    vm.load_program(compiled);
    vm.run()?;

    Ok(())
}

/// Collect all .tr files recursively from a directory.
fn collect_tr_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }
    let entries = fs::read_dir(dir).map_err(|e| format!("Failed to read directory: {}", e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();
        if path.is_dir() {
            collect_tr_files(&path, files)?;
        } else if let Some(ext) = path.extension() {
            if ext == "tr" {
                files.push(path);
            }
        }
    }
    Ok(())
}

/// A test function extracted from a .tr source file.
struct TestFn {
    name: String,
}

/// Extract top-level functions named `test_*` from a .tr source file
/// by parsing the AST and inspecting declarations.
fn extract_test_functions(file: &Path) -> Result<Vec<TestFn>, String> {
    let source = fs::read_to_string(file)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let tokens = lexer::tokenize(&source)?;
    let ast = parser::parse(tokens)?;

    let mut test_fns = Vec::new();

    for decl in &ast.declarations {
        if let trc::ast::Declaration::Function(fn_decl) = decl {
            if fn_decl.name.starts_with("test_") {
                test_fns.push(TestFn {
                    name: fn_decl.name.clone(),
                });
            }
        }
    }

    Ok(test_fns)
}

/// Run a single test function by name from a .tr file.
/// Compiles the whole file but only invokes the named test function.
fn run_single_test(file: &Path, test_name: &str, project_dir: &Path) -> Result<(), String> {
    let source = fs::read_to_string(file)
        .map_err(|e| format!("Failed to read test file: {}", e))?;

    let tokens = lexer::tokenize(&source)?;
    let ast = parser::parse(tokens)?;

    let mut compiler = Compiler::new();
    let compiled = compiler.compile_with_modules(&ast, project_dir)?;

    // Find the function index by name before loading the program
    let func_idx = compiled
        .functions
        .iter()
        .position(|f| f.name == test_name)
        .ok_or_else(|| format!("Test function '{}' not found in compiled output", test_name))?;

    let mut vm = Vm::new();
    vm.load_program(compiled);

    vm.call_function_by_index(func_idx)?;

    Ok(())
}
