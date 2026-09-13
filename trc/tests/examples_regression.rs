//! Regression: every example in `examples/` must run to completion.
//!
//! Each file is lexed, parsed, analyzed, and executed through the bytecode
//! VM in-process. A file fails the suite when any stage errors or when its
//! output contains a `FAIL:` line (the convention used by `all_features.tr`
//! and `file_io.tr`).
//!
//! `http_client.tr` is excluded: it performs live network requests.
//! File IO lands in a temp working directory so examples that write files
//! never pollute the repo.

use std::fs;
use std::path::PathBuf;

use trc::analyzer;
use trc::bytecode;
use trc::lexer;
use trc::parser;

const EXCLUDED: &[&str] = &["http_client.tr"];

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("trc should be inside the workspace")
        .to_path_buf()
}

#[test]
fn examples_all_run_without_errors() {
    let examples_dir = workspace_root().join("examples");
    assert!(
        examples_dir.is_dir(),
        "examples/ not found at {}",
        examples_dir.display()
    );

    // Isolate file IO: examples that write files do so under temp.
    let sandbox = std::env::temp_dir().join("trc_examples_regression");
    fs::create_dir_all(&sandbox).expect("create sandbox dir");
    std::env::set_current_dir(&sandbox).expect("enter sandbox dir");

    let mut files: Vec<PathBuf> = fs::read_dir(&examples_dir)
        .expect("read examples dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|ext| ext == "tr"))
        .filter(|p| {
            !EXCLUDED.iter().any(|name| {
                p.file_name().is_some_and(|f| f == *name)
            })
        })
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no example files found");

    for file in &files {
        let source =
            fs::read_to_string(file).unwrap_or_else(|_| panic!("read {}", file.display()));
        let tokens =
            lexer::tokenize(&source).unwrap_or_else(|_| panic!("tokenize {}", file.display()));
        let ast =
            parser::parse(tokens).unwrap_or_else(|_| panic!("parse {}", file.display()));
        // Mirror the CLI: fall back to the raw AST when analysis fails for
        // a program with imports (the compiler resolves those itself).
        let has_imports = !ast.imports.is_empty();
        let typed_ast = match analyzer::analyze(&ast) {
            Ok(typed) => typed,
            Err(_) if has_imports => ast,
            Err(errs) => panic!("analyze {}: {:?}", file.display(), errs),
        };
        let output = bytecode::execute_with_root(&typed_ast, &examples_dir)
            .unwrap_or_else(|_| panic!("execute {}", file.display()));
        let text = output.join("\n");
        assert!(
            !text.contains("FAIL:"),
            "{} printed FAIL lines:\n{}",
            file.display(),
            text
        );
    }

    eprintln!("examples_regression: {} files green", files.len());
}
