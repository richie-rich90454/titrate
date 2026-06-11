use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use trc::lexer;
use trc::parser;

use crate::project;

/// Generate Markdown API documentation from doc comments in .tr source files.
/// Writes output to `docs/api/` within the project directory.
pub fn doc(project_dir: &Path) -> Result<(), String> {
    let cfg = project::load_config(project_dir)?;

    // Collect all .tr files from src/
    let src_dir = project_dir.join("src");
    let mut tr_files = Vec::new();
    collect_tr_files(&src_dir, &mut tr_files)?;

    if tr_files.is_empty() {
        println!("No .tr source files found in src/");
        return Ok(());
    }

    // Create output directory
    let doc_dir = project_dir.join("docs").join("api");
    fs::create_dir_all(&doc_dir)
        .map_err(|e| format!("Failed to create docs/api directory: {}", e))?;

    let mut total_entries = 0;

    for tr_file in &tr_files {
        let entries = extract_doc_entries(tr_file)?;
        if entries.is_empty() {
            continue;
        }

        let rel = tr_file
            .strip_prefix(project_dir)
            .unwrap_or(tr_file)
            .display()
            .to_string();

        let md = generate_doc_markdown(&cfg.package.name, &rel, &entries);

        // Derive output file name from the source file name
        let file_stem = tr_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let output_path = doc_dir.join(format!("{}.md", file_stem));

        fs::write(&output_path, md)
            .map_err(|e| format!("Failed to write doc file: {}", e))?;

        total_entries += entries.len();
        println!("  documented {} -> {}", rel, output_path.display());
    }

    // Generate an index page
    let index_md = generate_index_markdown(&cfg.package.name, &tr_files, project_dir, &doc_dir)?;
    let index_path = doc_dir.join("index.md");
    fs::write(&index_path, index_md)
        .map_err(|e| format!("Failed to write index: {}", e))?;

    println!("Generated documentation for {} entries in docs/api/", total_entries);
    Ok(())
}

/// A documentation entry extracted from source.
#[derive(Debug, Clone)]
struct DocEntry {
    /// The kind of item (function, class, enum, interface).
    kind: String,
    /// The name of the item.
    name: String,
    /// The doc comment lines preceding the item.
    doc_lines: Vec<String>,
    /// Function signature (params + return type), if applicable.
    signature: Option<String>,
}

/// Extract doc entries from a .tr source file.
/// Doc comments are lines starting with `//` that immediately precede
/// a function, class, enum, or interface declaration.
fn extract_doc_entries(file: &Path) -> Result<Vec<DocEntry>, String> {
    let source = fs::read_to_string(file)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let tokens = lexer::tokenize(&source)?;
    let ast = parser::parse(tokens)?;

    // Build a map from function name -> signature for all top-level declarations
    let mut entries = Vec::new();

    for decl in &ast.declarations {
        match decl {
            trc::ast::Declaration::Function(fn_decl) => {
                let sig = format_fn_signature(fn_decl);
                entries.push(DocEntry {
                    kind: "function".to_string(),
                    name: fn_decl.name.clone(),
                    doc_lines: Vec::new(),
                    signature: Some(sig),
                });
            }
            trc::ast::Declaration::Class(class_decl) => {
                entries.push(DocEntry {
                    kind: "class".to_string(),
                    name: class_decl.name.clone(),
                    doc_lines: Vec::new(),
                    signature: None,
                });
            }
            trc::ast::Declaration::Enum(enum_decl) => {
                entries.push(DocEntry {
                    kind: "enum".to_string(),
                    name: enum_decl.name.clone(),
                    doc_lines: Vec::new(),
                    signature: None,
                });
            }
            trc::ast::Declaration::Interface(iface_decl) => {
                entries.push(DocEntry {
                    kind: "interface".to_string(),
                    name: iface_decl.name.clone(),
                    doc_lines: Vec::new(),
                    signature: None,
                });
            }
            _ => {}
        }
    }

    // Now extract doc comments from the raw source by scanning for `//` lines
    // that immediately precede declaration keywords.
    let doc_comments = extract_doc_comments_from_source(&source);

    // Match doc comments to declarations by line proximity
    for entry in &mut entries {
        if let Some(doc) = doc_comments.get(&entry.name) {
            entry.doc_lines = doc.clone();
        }
    }

    Ok(entries)
}

/// Scan raw source text for doc comments (lines starting with `//`)
/// that immediately precede a named declaration.
/// Returns a map from declaration name to doc comment lines.
fn extract_doc_comments_from_source(source: &str) -> HashMap<String, Vec<String>> {
    let lines: Vec<&str> = source.lines().collect();
    let mut result: HashMap<String, Vec<String>> = HashMap::new();

    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim();

        // Collect consecutive doc comment lines
        if trimmed.starts_with("//") {
            let mut comment_lines = Vec::new();
            while i < lines.len() && lines[i].trim().starts_with("//") {
                let comment = lines[i].trim().trim_start_matches('/').trim();
                comment_lines.push(comment.to_string());
                i += 1;
            }

            // The next non-comment, non-empty line should be a declaration
            while i < lines.len() && lines[i].trim().is_empty() {
                i += 1;
            }

            if i < lines.len() {
                let next_line = lines[i].trim();
                if let Some(name) = extract_declaration_name(next_line) {
                    result.insert(name, comment_lines);
                }
            }

            continue;
        }

        i += 1;
    }

    result
}

/// Try to extract the name from a declaration line.
/// Handles: `fn name`, `public fn name`, `class Name`, `enum Name`, `interface Name`, etc.
fn extract_declaration_name(line: &str) -> Option<String> {
    let tokens: Vec<&str> = line.split_whitespace().collect();

    // Find the keyword and the name after it
    for i in 0..tokens.len() {
        match tokens[i] {
            "fn" | "class" | "enum" | "interface" => {
                if i + 1 < tokens.len() {
                    // Strip any trailing punctuation (e.g., `(` or `{` or `<`)
                    let name = tokens[i + 1]
                        .trim_end_matches('(')
                        .trim_end_matches('{')
                        .trim_end_matches('<');
                    if !name.is_empty() {
                        return Some(name.to_string());
                    }
                }
            }
            _ => {}
        }
    }

    None
}

/// Format a function declaration as a human-readable signature string.
fn format_fn_signature(fn_decl: &trc::ast::FnDecl) -> String {
    let params: Vec<String> = fn_decl
        .params
        .iter()
        .map(|p| format!("{}: {}", p.name, p.typ))
        .collect();

    let ret = match &fn_decl.return_type {
        Some(t) => format!(" -> {}", t),
        None => String::new(),
    };

    format!("{}({}){}", fn_decl.name, params.join(", "), ret)
}

/// Generate Markdown content for a single source file's doc entries.
fn generate_doc_markdown(package_name: &str, source_path: &str, entries: &[DocEntry]) -> String {
    let mut md = String::new();

    md.push_str(&format!("# API Reference – {}\n\n", source_path));
    md.push_str(&format!("Package: `{}`\n\n", package_name));
    md.push_str("---\n\n");

    for entry in entries {
        // Heading with kind and name
        md.push_str(&format!("### {} `{}`\n\n", capitalize(&entry.kind), entry.name));

        // Signature (for functions)
        if let Some(sig) = &entry.signature {
            md.push_str(&format!("```titrate\n{}\n```\n\n", sig));
        }

        // Doc comment content
        if !entry.doc_lines.is_empty() {
            for line in &entry.doc_lines {
                md.push_str(line);
                md.push('\n');
            }
            md.push('\n');
        } else {
            md.push_str("*No documentation available.*\n\n");
        }
    }

    md
}

/// Generate an index Markdown page listing all documented source files.
fn generate_index_markdown(
    package_name: &str,
    tr_files: &[PathBuf],
    project_dir: &Path,
    _doc_dir: &Path,
) -> Result<String, String> {
    let mut md = String::new();

    md.push_str(&format!("# {} – API Reference Index\n\n", package_name));
    md.push_str("Auto-generated documentation from source doc comments.\n\n");
    md.push_str("---\n\n");

    for tr_file in tr_files {
        let rel = tr_file
            .strip_prefix(project_dir)
            .unwrap_or(tr_file)
            .display()
            .to_string();

        let file_stem = tr_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        md.push_str(&format!("- [{}]({}.md)\n", rel, file_stem));
    }

    md.push('\n');

    Ok(md)
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().chain(c).collect(),
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
