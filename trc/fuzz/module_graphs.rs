//! Module-graph harness: random import graphs (including cycles) are written
//! to a private temporary directory and compiled through the bytecode module
//! resolver. Cycles must terminate; missing modules must produce descriptive
//! errors; panics and hangs are failures.

use std::fs;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use trc::{bytecode, lexer, parser};

use crate::common::Lcg;

const PREFIX: &str = "fzmod";
const MODULE_COUNT: usize = 5;
const TIMEOUT_MS: u64 = 1_000;
const STACK_SIZE: usize = 16 * 1024 * 1024;

fn module_name(index: usize) -> String {
    format!("{}_{}", PREFIX, index)
}

pub fn temp_root(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "trc_fuzz_graphs_{}_{}",
        std::process::id(),
        tag
    ))
}

fn prepare_root(root: &Path) -> Result<(), String> {
    let _ = fs::remove_dir_all(root);
    fs::create_dir_all(root.join("fzmod_sub"))
        .map_err(|error| format!("create temporary module directory: {}", error))?;
    for (name, source) in [
        ("a.tr", "public fn sub_a(): int { return 1; }\n"),
        ("b.tr", "public fn sub_b(): int { return 2; }\n"),
    ] {
        fs::write(root.join("fzmod_sub").join(name), source)
            .map_err(|error| format!("write {}: {}", name, error))?;
    }
    Ok(())
}

/// Write one generated graph into `root`.
pub fn write_graph(root: &Path, files: &[(String, String)]) -> Result<(), String> {
    fs::create_dir_all(root)
        .map_err(|error| format!("create graph directory: {}", error))?;
    for (name, source) in files {
        fs::write(root.join(name), source)
            .map_err(|error| format!("write module '{}': {}", name, error))?;
    }
    Ok(())
}

/// Lex, parse, and compile `main.tr` with its imports rooted at `root`.
pub fn compile_graph_at(root: &Path) -> Result<(), String> {
    let source = fs::read_to_string(root.join("main.tr"))
        .map_err(|error| format!("read main.tr: {}", error))?;
    let tokens = lexer::tokenize(&source).map_err(|error| format!("lexer: {}", error))?;
    let program = parser::parse(tokens).map_err(|error| format!("parser: {}", error))?;
    let mut compiler = bytecode::Compiler::new();
    compiler.compile_with_modules(&program, root)?;
    Ok(())
}

fn random_import(rng: &mut Lcg) -> String {
    let target = module_name(rng.range(0, MODULE_COUNT - 1));
    match rng.range(0, 11) {
        0..=8 => format!("import {};\n", target),
        9 => format!("import {}::missing::Symbol;\n", target),
        10 => "import fzmod_sub::*;\n".to_string(),
        _ => "import missing_module;\n".to_string(),
    }
}

fn random_graph(rng: &mut Lcg) -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = Vec::new();
    for index in 0..MODULE_COUNT {
        let mut source = String::new();
        for _ in 0..rng.range(0, 3) {
            source.push_str(&random_import(rng));
        }
        source.push_str(&format!(
            "public fn f{}(): int {{ return {}; }}\n",
            index, index
        ));
        files.push((format!("{}.tr", module_name(index)), source));
    }

    let mut main_source = String::new();
    for _ in 0..rng.range(1, 4) {
        main_source.push_str(&random_import(rng));
    }
    main_source.push_str("public fn main(): void { }\n");
    files.push(("main.tr".to_string(), main_source));
    files
}

/// A graph whose only defect is an unresolvable import.
pub fn missing_import_case() -> Vec<(String, String)> {
    vec![(
        "main.tr".to_string(),
        "import definitely_missing_module;\npublic fn main(): void { }\n".to_string(),
    )]
}

fn compile_with_timeout(root: PathBuf) -> Result<Result<(), String>, &'static str> {
    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(move || {
            let outcome = catch_unwind(AssertUnwindSafe(|| compile_graph_at(&root)));
            let _ = tx.send(outcome);
        })
        .map_err(|_| "spawn-failed")?;
    match rx.recv_timeout(Duration::from_millis(TIMEOUT_MS)) {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(_)) => Err("panic"),
        Err(mpsc::RecvTimeoutError::Timeout) => Err("timeout"),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err("panic"),
    }
}

pub fn run(iterations: usize, seed: u64) -> Result<(), String> {
    let root = temp_root("run");
    prepare_root(&root)?;
    let mut rng = Lcg::new(seed);
    let mut resolved = 0usize;
    let mut failure: Option<String> = None;

    for iteration in 0..iterations {
        let graph = random_graph(&mut rng);
        write_graph(&root, &graph)?;
        match compile_with_timeout(root.clone()) {
            Ok(Ok(())) => resolved += 1,
            Ok(Err(_)) => {
                // Missing modules and invalid globs are structured errors.
            }
            Err(reason) => {
                failure = Some(format!(
                    "iteration {}: generated import graph {}",
                    iteration, reason
                ));
                break;
            }
        }
    }
    let _ = fs::remove_dir_all(&root);

    if let Some(error) = failure {
        return Err(error);
    }
    if resolved == 0 {
        return Err("generator produced no resolvable import graphs".to_string());
    }
    Ok(())
}
