//! Single-file runner mirroring stdlib_runtest.rs harness.
//! Usage: cargo test --test single_runtest -- <relative_path_to_.tr_file>
//! e.g.   cargo test --test single_runtest -- ../stdlib_test/src/tests/math/test_ndarray_broadcast.tr

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use regex::Regex;
use trc::bytecode;
use trc::lexer;
use trc::parser;

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
    let mut seen = std::collections::HashSet::new();
    calls.retain(|c| seen.insert(c.clone()));
    let body: String = calls
        .iter()
        .map(|c| format!("    {}();", c))
        .collect::<Vec<_>>()
        .join("\n");
    format!("\n\npublic fn main(): void {{\n{}\n}}\n", body)
}

fn run_test_file(path: &Path) -> Result<String, String> {
    let source = fs::read_to_string(path).map_err(|e| format!("read error: {}", e))?;
    let full_source = if has_main_re().is_match(&source) {
        source
    } else {
        format!("{}\n{}", source, build_synthetic_main(&source))
    };
    let tokens = lexer::tokenize(&full_source).map_err(|e| format!("lexer: {}", e))?;
    let ast = parser::parse(tokens).map_err(|e| format!("parser: {}", e))?;
    let root_dir = PathBuf::from("..");
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

#[test]
fn single_runtest() {
    let target = match env::var("TR_FILE") {
        Ok(p) if !p.is_empty() => p,
        _ => {
            eprintln!("Usage: TR_FILE=path/to/file.tr cargo test --test single_runtest");
            // Skip silently when no file is specified so this doesn't break `cargo test`.
            return;
        }
    };
    let path = PathBuf::from(&target);
    eprintln!("Running: {}", path.display());
    match run_test_file(&path) {
        Ok(output) => {
            println!("--- OUTPUT ---");
            println!("{}", output);
            println!("--- END ---");
            if output.contains("FAIL:") {
                panic!("Test contained FAIL: lines");
            }
        }
        Err(e) => {
            println!("ERROR: {}", e);
            panic!("Test failed: {}", e);
        }
    }
}
