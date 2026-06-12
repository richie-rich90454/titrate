use std::fs;
use std::path::{Path, PathBuf};

use crate::project;

/// Format .tr source files with basic indentation normalization.
/// Normalizes indentation to 4-space increments and trims trailing whitespace.
pub fn fmt(project_dir: &Path) -> Result<(), String> {
    let _cfg = project::load_config(project_dir)?;

    let src_dir = project_dir.join("src");
    let mut tr_files = Vec::new();
    collect_tr_files(&src_dir, &mut tr_files)?;

    if tr_files.is_empty() {
        println!("No .tr source files found in src/");
        return Ok(());
    }

    let mut formatted = 0;

    for tr_file in &tr_files {
        let rel = tr_file
            .strip_prefix(project_dir)
            .unwrap_or(tr_file)
            .display()
            .to_string();

        let source = fs::read_to_string(tr_file)
            .map_err(|e| format!("Failed to read {}: {}", rel, e))?;

        let formatted_source = format_tr_source(&source);

        if formatted_source != source {
            fs::write(tr_file, &formatted_source)
                .map_err(|e| format!("Failed to write {}: {}", rel, e))?;
            println!("  formatted {}", rel);
            formatted += 1;
        } else {
            println!("  already formatted {}", rel);
        }
    }

    println!("\nFormatted {} file(s)", formatted);
    Ok(())
}

/// Format a .tr source string by normalizing indentation to 4-space increments
/// and trimming trailing whitespace.
fn format_tr_source(source: &str) -> String {
    let mut result = String::new();
    let indent_size = 4;

    for line in source.lines() {
        let trimmed = line.trim_end();

        if trimmed.is_empty() {
            result.push('\n');
            continue;
        }

        // Determine the indentation change from the *content* of this line
        let leading = trimmed.len() - trimmed.trim_start().len();
        let current_indent = leading / indent_size;

        // Detect closing braces/brackets that decrease indent for this line
        let first_char = trimmed.trim_start().chars().next().unwrap_or(' ');
        let is_closing = first_char == '}' || first_char == ')';

        let display_indent = if is_closing && current_indent > 0 {
            current_indent - 1
        } else {
            current_indent
        };

        // Rebuild the line with normalized indentation
        let content = trimmed.trim_start();
        for _ in 0..display_indent * indent_size {
            result.push(' ');
        }
        result.push_str(content);
        result.push('\n');
    }

    // Remove trailing newline if the original didn't have one
    if !source.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }

    result
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
