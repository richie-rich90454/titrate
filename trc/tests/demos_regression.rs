//! Regression: every demo in `demos/` must run to completion without errors.
//!
//! Each file is lexed, parsed, analyzed, and executed through the bytecode VM.
//! A demo fails when any stage errors or when its output contains a `FAIL:`
//! line. Programs that run forever or need a live network are excluded with
//! documented reasons.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use trc::analyzer;
use trc::bytecode;
use trc::lexer;
use trc::parser;

const PER_FILE_TIMEOUT_SECS: u64 = 20;

const EXCLUDED: &[(&str, &str)] = &[
    ("file_watcher.tr", "intentional infinite watcher"),
    ("http_server.tr", "blocks on a live server socket"),
    ("chat_server.tr", "blocks on a live server socket"),
    ("file_server.tr", "blocks on a live server socket"),
    ("async_fetch.tr", "requires live network access"),
];

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("trc should be inside the workspace")
        .to_path_buf()
}

fn run_demo(path: &Path) -> Result<String, String> {
    let source = fs::read_to_string(path).map_err(|e| format!("read: {}", e))?;
    let tokens = lexer::tokenize(&source).map_err(|e| format!("lexer: {}", e))?;
    let ast = parser::parse(tokens).map_err(|e| format!("parser: {}", e))?;
    let root = workspace_root();
    let imported = bytecode::Compiler::load_imported_programs(&ast, &root)
        .map_err(|e| format!("module resolution: {}", e))?;
    let typed_ast = analyzer::analyze_with_imports(&ast, &imported)
        .map(|(typed, _)| typed)
        .map_err(|errs| format!("analyzer: {:?}", errs))?;
    let output = bytecode::execute_with_root(&typed_ast, &root)
        .map_err(|e| format!("execute: {}", e))?;
    Ok(output.join("\n"))
}

fn run_demo_with_timeout(path: &Path) -> Result<String, String> {
    let (tx, rx) = mpsc::channel();
    let owned = path.to_path_buf();
    thread::Builder::new()
        .name(owned.file_stem().unwrap_or_default().to_string_lossy().to_string())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            let result = std::panic::catch_unwind(|| run_demo(&owned));
            let _ = tx.send(match result {
                Ok(r) => r,
                Err(_) => Err("worker panicked".to_string()),
            });
        })
        .map_err(|e| format!("spawn: {}", e))?;
    match rx.recv_timeout(Duration::from_secs(PER_FILE_TIMEOUT_SECS)) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => Err(format!("timeout after {}s", PER_FILE_TIMEOUT_SECS)),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err("worker disconnected".to_string()),
    }
}

#[test]
fn demos_all_run_without_errors() {
    let demos_dir = workspace_root().join("demos");
    let mut files: Vec<PathBuf> = fs::read_dir(&demos_dir)
        .expect("read demos dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|ext| ext == "tr"))
        .filter(|p| {
            let name = p.file_name().and_then(|f| f.to_str()).unwrap_or_default();
            !EXCLUDED.iter().any(|(excluded, _)| *excluded == name)
        })
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no demos found");

    let mut failures = Vec::new();
    for file in &files {
        match run_demo_with_timeout(file) {
            Ok(text) => {
                if text.contains("FAIL:") {
                    failures.push(format!("{} printed FAIL lines", file.display()));
                }
            }
            Err(err) => failures.push(format!("{}: {}", file.display(), err)),
        }
    }
    assert!(
        failures.is_empty(),
        "{} demo(s) failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
    eprintln!("demos_regression: {} demos green", files.len());
}
