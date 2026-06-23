//! Integration tests for the LLVM native backend (Phase 3, Task 3.4).
//!
//! These tests verify that the native bridge correctly maps Titrate native
//! function calls (e.g. `Math.sin(x)`, `String.length(s)`) to their C-ABI
//! wrapper symbols (e.g. `titrate_Math_sin`, `titrate_String_length`) in the
//! generated LLVM IR.
//!
//! They also verify that standard-library `.tr` source files can be parsed
//! and analyzed, and that the `lib/tt/` directory is on the module
//! resolution path.
//!
//! These tests do NOT require a system linker or LLVM dev files — they stop
//! at IR generation and inspect the IR text. The full compile-link-run
//! pipeline is covered by `native_hello.rs` (which is `#[ignore]`d).

use std::path::PathBuf;

use trc::lexer;
use trc::parser;
use trc::analyzer;
use trc::codegen::llvm;

/// Walk up from CARGO_MANIFEST_DIR to find the workspace root (the directory
/// containing `lib/tt/`).
fn workspace_root() -> PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR should be set by cargo");
    PathBuf::from(manifest)
        .parent()
        .map(|p| p.to_path_buf())
        .expect("trc should be inside the workspace")
}

/// Compile a source string to LLVM IR text. The source must define `main`.
fn compile_to_ir(source: &str) -> Result<String, String> {
    let tokens = lexer::tokenize(source).map_err(|e| format!("tokenize: {}", e))?;
    let ast = parser::parse(tokens).map_err(|e| format!("parse: {}", e))?;
    let typed_ast = analyzer::analyze(&ast).map_err(|e| format!("analyze: {:?}", e))?;
    llvm::compile_to_ir_text(&typed_ast)
}

// ---------------------------------------------------------------------------
// Math native function IR tests
// ---------------------------------------------------------------------------

#[test]
fn native_math_sin_emits_titrate_wrapper() {
    let ir = compile_to_ir(r#"
        public fn main(): void {
            let r: double = Math.sin(0.0);
        }
    "#).expect("IR generation should succeed");

    assert!(
        ir.contains("titrate_Math_sin"),
        "expected IR to declare/call titrate_Math_sin, got:\n{}",
        ir,
    );
}

#[test]
fn native_math_sqrt_emits_titrate_wrapper() {
    let ir = compile_to_ir(r#"
        public fn main(): void {
            let r: double = Math.sqrt(4.0);
        }
    "#).expect("IR generation should succeed");

    assert!(
        ir.contains("titrate_Math_sqrt"),
        "expected IR to declare/call titrate_Math_sqrt, got:\n{}",
        ir,
    );
}

#[test]
fn native_math_abs_emits_titrate_wrapper() {
    let ir = compile_to_ir(r#"
        public fn main(): void {
            let r: double = Math.abs(-5.0);
        }
    "#).expect("IR generation should succeed");

    assert!(
        ir.contains("titrate_Math_abs"),
        "expected IR to declare/call titrate_Math_abs, got:\n{}",
        ir,
    );
}

// ---------------------------------------------------------------------------
// String native function IR tests
// ---------------------------------------------------------------------------

#[test]
fn native_string_length_emits_titrate_wrapper() {
    let ir = compile_to_ir(r#"
        public fn main(): void {
            let n: int = String.length("hello");
        }
    "#).expect("IR generation should succeed");

    assert!(
        ir.contains("titrate_String_length"),
        "expected IR to declare/call titrate_String_length, got:\n{}",
        ir,
    );
}

#[test]
fn native_string_char_at_emits_titrate_wrapper() {
    let ir = compile_to_ir(r#"
        public fn main(): void {
            let c: string = String.charAt("hello", 0);
        }
    "#).expect("IR generation should succeed");

    assert!(
        ir.contains("titrate_String_charAt"),
        "expected IR to declare/call titrate_String_charAt, got:\n{}",
        ir,
    );
}

#[test]
fn native_string_to_upper_case_emits_titrate_wrapper() {
    let ir = compile_to_ir(r#"
        public fn main(): void {
            let s: string = String.toUpperCase("hello");
        }
    "#).expect("IR generation should succeed");

    assert!(
        ir.contains("titrate_String_toUpperCase"),
        "expected IR to declare/call titrate_String_toUpperCase, got:\n{}",
        ir,
    );
}

// ---------------------------------------------------------------------------
// Bare (non-qualified) native function IR tests
// ---------------------------------------------------------------------------

#[test]
fn native_parse_int_emits_titrate_wrapper() {
    let ir = compile_to_ir(r#"
        public fn main(): void {
            let n: int = parseInt("42");
        }
    "#).expect("IR generation should succeed");

    assert!(
        ir.contains("titrate_parseInt"),
        "expected IR to declare/call titrate_parseInt, got:\n{}",
        ir,
    );
}

// ---------------------------------------------------------------------------
// Static-call (::) native function IR tests
// ---------------------------------------------------------------------------

#[test]
fn native_static_call_math_sin_emits_titrate_wrapper() {
    let ir = compile_to_ir(r#"
        public fn main(): void {
            let r: double = Math::sin(0.0);
        }
    "#).expect("IR generation should succeed");

    assert!(
        ir.contains("titrate_Math_sin"),
        "expected IR to declare/call titrate_Math_sin via static call, got:\n{}",
        ir,
    );
}

// ---------------------------------------------------------------------------
// Multiple native calls in one function
// ---------------------------------------------------------------------------

#[test]
fn native_multiple_calls_in_one_function() {
    let ir = compile_to_ir(r#"
        public fn main(): void {
            let s: double = Math.sin(0.0);
            let c: double = Math.cos(0.0);
            let len: int = String.length("hello");
        }
    "#).expect("IR generation should succeed");

    assert!(ir.contains("titrate_Math_sin"), "missing titrate_Math_sin");
    assert!(ir.contains("titrate_Math_cos"), "missing titrate_Math_cos");
    assert!(ir.contains("titrate_String_length"), "missing titrate_String_length");
}

// ---------------------------------------------------------------------------
// Object file generation smoke test (no linking)
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires LLVM target machine (native codegen); run with --ignored"]
fn native_object_file_generated_for_native_calls() {
    let source = r#"
        public fn main(): void {
            let r: double = Math.sqrt(16.0);
        }
    "#;

    let tokens = lexer::tokenize(source).expect("tokenize failed");
    let ast = parser::parse(tokens).expect("parse failed");
    let typed_ast = analyzer::analyze(&ast).expect("analyze failed");

    let obj_path = std::env::temp_dir().join("trc_native_stdlib_test.o");
    llvm::compile(&typed_ast, &obj_path, false).expect("LLVM compile failed");

    assert!(
        obj_path.is_file(),
        "object file was not produced at {}",
        obj_path.display(),
    );

    let _ = std::fs::remove_file(&obj_path);
}

// ---------------------------------------------------------------------------
// Standard library module resolution tests
// ---------------------------------------------------------------------------

/// Verify that the `lib/tt/` directory exists and contains `.tr` files
/// (recursively). This confirms the stdlib is present on the module
/// resolution path.
#[test]
fn stdlib_directory_exists_and_contains_tr_files() {
    let lib_tt = workspace_root().join("lib").join("tt");
    assert!(
        lib_tt.is_dir(),
        "lib/tt/ directory not found at {}",
        lib_tt.display(),
    );

    let mut count = 0usize;
    walk_tr_files(&lib_tt, &mut |_| { count += 1; });
    assert!(
        count > 0,
        "expected .tr files under lib/tt/ (recursively), found none",
    );
    eprintln!("Found {} .tr files under lib/tt/", count);
}

/// Verify that key stdlib source files can be parsed.
/// (We test parsing only, not analysis, because the standalone analyzer
/// cannot resolve cross-module imports — that is the compiler's job.)
#[test]
fn stdlib_math_source_parses() {
    let math_file = find_stdlib_file("Math.tr");

    let math_file = match math_file {
        Some(p) => p,
        None => {
            eprintln!("Skipping: Math.tr not found in lib/tt/");
            return;
        }
    };

    let source = std::fs::read_to_string(&math_file)
        .expect("failed to read Math.tr");
    let tokens = lexer::tokenize(&source).expect("tokenize Math.tr failed");
    let _ast = parser::parse(tokens).expect("parse Math.tr failed");
}

/// Verify that the String stdlib source can be parsed.
#[test]
fn stdlib_string_source_parses() {
    let string_file = match find_stdlib_file("String.tr") {
        Some(p) => p,
        None => {
            eprintln!("Skipping: String.tr not found in lib/tt/");
            return;
        }
    };

    let source = std::fs::read_to_string(&string_file)
        .expect("failed to read String.tr");
    let tokens = lexer::tokenize(&source).expect("tokenize String.tr failed");
    let _ast = parser::parse(tokens).expect("parse String.tr failed");
}

/// Verify that the ArrayList stdlib source can be parsed.
#[test]
fn stdlib_arraylist_source_parses() {
    let arraylist_file = match find_stdlib_file("ArrayList.tr") {
        Some(p) => p,
        None => {
            eprintln!("Skipping: ArrayList.tr not found in lib/tt/");
            return;
        }
    };

    let source = std::fs::read_to_string(&arraylist_file)
        .expect("failed to read ArrayList.tr");
    let tokens = lexer::tokenize(&source).expect("tokenize ArrayList.tr failed");
    let _ast = parser::parse(tokens).expect("parse ArrayList.tr failed");
}

/// Verify that the File stdlib source can be parsed.
#[test]
fn stdlib_file_source_parses() {
    let file_file = match find_stdlib_file("File.tr") {
        Some(p) => p,
        None => {
            eprintln!("Skipping: File.tr not found in lib/tt/");
            return;
        }
    };

    let source = std::fs::read_to_string(&file_file)
        .expect("failed to read File.tr");
    let tokens = lexer::tokenize(&source).expect("tokenize File.tr failed");
    let _ast = parser::parse(tokens).expect("parse File.tr failed");
}

/// Count how many stdlib `.tr` files can be successfully parsed.
/// This is a broad smoke test for "compile full standard library natively"
/// at the front-end (lexer + parser) level. Analysis and codegen require
/// import resolution which is the compiler's responsibility.
#[test]
fn stdlib_all_files_parse() {
    let lib_tt = workspace_root().join("lib").join("tt");
    if !lib_tt.is_dir() {
        eprintln!("Skipping: lib/tt/ not found");
        return;
    }

    let mut total = 0usize;
    let mut ok = 0usize;
    let mut failures: Vec<String> = Vec::new();

    walk_tr_files(&lib_tt, &mut |path| {
        total += 1;
        let rel = path.strip_prefix(&lib_tt)
            .unwrap_or(path)
            .display()
            .to_string();
        match std::fs::read_to_string(path) {
            Ok(source) => {
                match lexer::tokenize(&source) {
                    Ok(tokens) => {
                        match parser::parse(tokens) {
                            Ok(_) => { ok += 1; }
                            Err(e) => {
                                failures.push(format!("{}: parse: {}", rel, e));
                            }
                        }
                    }
                    Err(e) => {
                        failures.push(format!("{}: tokenize: {}", rel, e));
                    }
                }
            }
            Err(e) => {
                failures.push(format!("{}: read: {}", rel, e));
            }
        }
    });

    assert!(total > 0, "no .tr files found in lib/tt/");
    // We expect at least 90% of stdlib files to parse cleanly.
    let threshold = (total * 9) / 10;
    assert!(
        ok >= threshold,
        "only {}/{} stdlib files parsed (expected >= {}). Failures:\n{}",
        ok, total, threshold,
        failures.iter().take(20).cloned().collect::<Vec<_>>().join("\n"),
    );
    eprintln!("Parsed {}/{} stdlib .tr files successfully", ok, total);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Recursively find all `.tr` files under `dir` and invoke `f` for each.
fn walk_tr_files(dir: &std::path::Path, f: &mut impl FnMut(&std::path::Path)) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk_tr_files(&path, f);
            } else if path.extension().and_then(|e| e.to_str()) == Some("tr") {
                f(&path);
            }
        }
    }
}

/// Search for a file by name under `lib/tt/`.
fn find_stdlib_file(name: &str) -> Option<PathBuf> {
    let lib_tt = workspace_root().join("lib").join("tt");
    if !lib_tt.is_dir() {
        return None;
    }
    let mut result = None;
    walk_tr_files(&lib_tt, &mut |path| {
        if path.file_name().and_then(|n| n.to_str()) == Some(name) {
            result = Some(path.to_path_buf());
        }
    });
    result
}
