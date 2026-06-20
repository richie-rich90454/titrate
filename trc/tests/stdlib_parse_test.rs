use trc::lexer;
use trc::parser;
use std::fs;
use std::path::PathBuf;

/// Walk a directory recursively and collect all `.tr` file paths.
fn collect_tr_files(dir: &PathBuf, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_tr_files(&path, files);
            } else if path.extension().and_then(|s| s.to_str()) == Some("tr") {
                files.push(path);
            }
        }
    }
}

/// Test that every `.tr` file under `lib/tt/` parses without errors.
#[test]
fn test_all_stdlib_files_parse() {
    let lib_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("lib")
        .join("tt");

    let mut files = Vec::new();
    collect_tr_files(&lib_dir, &mut files);

    assert!(!files.is_empty(), "No .tr files found under lib/tt/");

    let mut failures: Vec<(PathBuf, String)> = Vec::new();

    for file in &files {
        let source = match fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => {
                failures.push((file.clone(), format!("read error: {}", e)));
                continue;
            }
        };

        let tokens = match lexer::tokenize(&source) {
            Ok(t) => t,
            Err(e) => {
                failures.push((file.clone(), format!("lexer error: {}", e)));
                continue;
            }
        };

        if let Err(e) = parser::parse(tokens) {
            failures.push((file.clone(), format!("parse error: {}", e)));
        }
    }

    if !failures.is_empty() {
        let mut msg = String::new();
        msg.push_str(&format!(
            "{} stdlib file(s) failed to parse:\n",
            failures.len()
        ));
        for (path, err) in &failures {
            msg.push_str(&format!("  {} : {}\n", path.display(), err));
        }
        panic!("{}", msg);
    }
}
