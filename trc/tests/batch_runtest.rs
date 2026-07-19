//! Batch runner that runs multiple .tr files in sequence.
//! Usage: TR_FILES="path1.tr,path2.tr" cargo test --test batch_runtest batch_runtest

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
fn batch_runtest() {
    let files = match env::var("TR_FILES") {
        Ok(s) if !s.is_empty() => s,
        _ => {
            eprintln!("Usage: TR_FILES=path1.tr,path2.tr cargo test --test batch_runtest batch_runtest");
            return;
        }
    };
    let mut passes = 0usize;
    let mut fails = 0usize;
    let mut failures: Vec<(String, String)> = Vec::new();
    for f in files.split(',') {
        let f = f.trim();
        if f.is_empty() { continue; }
        let path = PathBuf::from(f);
        eprintln!("[RUN] {}", path.display());
        match run_test_file(&path) {
            Ok(output) => {
                if output.contains("FAIL:") {
                    fails += 1;
                    let sample: Vec<&str> = output.lines().filter(|l| l.contains("FAIL:")).take(3).collect();
                    failures.push((f.to_string(), sample.join("\n")));
                    eprintln!("[FAIL] {} - FAIL lines in output", f);
                } else {
                    passes += 1;
                    eprintln!("[PASS] {}", f);
                }
            }
            Err(e) => {
                fails += 1;
                eprintln!("[FAIL] {} - {}", f, e);
                failures.push((f.to_string(), e));
            }
        }
    }
    eprintln!("\n=== SUMMARY: {} passed, {} failed ===", passes, fails);
    for (f, e) in &failures {
        eprintln!("\n[{}]\n{}", f, e);
    }
    if fails > 0 {
        panic!("{} test(s) failed", fails);
    }
}
