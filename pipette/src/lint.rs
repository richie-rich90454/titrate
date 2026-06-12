use std::fs;
use std::path::{Path, PathBuf};

use trc::analyzer;
use trc::lexer;
use trc::parser;

use crate::project;

/// Run the analyzer on all .tr files in the project and report diagnostics.
pub fn lint(project_dir: &Path) -> Result<(), String> {
    let _cfg = project::load_config(project_dir)?;

    let src_dir = project_dir.join("src");
    let mut tr_files = Vec::new();
    collect_tr_files(&src_dir, &mut tr_files)?;

    if tr_files.is_empty() {
        println!("No .tr source files found in src/");
        return Ok(());
    }

    let mut total_errors = 0;
    let mut total_warnings = 0;

    for tr_file in &tr_files {
        let rel = tr_file
            .strip_prefix(project_dir)
            .unwrap_or(tr_file)
            .display()
            .to_string();

        let source = fs::read_to_string(tr_file)
            .map_err(|e| format!("Failed to read {}: {}", rel, e))?;

        let tokens = match lexer::tokenize(&source) {
            Ok(t) => t,
            Err(e) => {
                println!("  {} ERROR: {}", rel, e);
                total_errors += 1;
                continue;
            }
        };

        let ast = match parser::parse(tokens) {
            Ok(a) => a,
            Err(e) => {
                println!("  {} ERROR: {}", rel, e);
                total_errors += 1;
                continue;
            }
        };

        match analyzer::analyze(&ast) {
            Ok(_) => {
                println!("  {} OK", rel);
            }
            Err(errs) => {
                for err in &errs {
                    println!("  {} WARNING: {}", rel, err);
                }
                total_warnings += errs.len();
            }
        }
    }

    println!(
        "\nLint complete: {} file(s), {} error(s), {} warning(s)",
        tr_files.len(),
        total_errors,
        total_warnings
    );

    if total_errors > 0 {
        Err(format!("{} error(s) found during lint", total_errors))
    } else {
        Ok(())
    }
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
