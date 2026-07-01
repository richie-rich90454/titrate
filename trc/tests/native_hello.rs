//! Integration test for the LLVM native backend (Phase 0).
//!
//! This test compiles `examples/hello.tr` to a native executable using
//! `trc --native`, runs the resulting binary, and asserts that the output
//! contains the expected "Hello, World!" line.
//!
//! The test is marked `#[ignore]` by default because it requires:
//!   1. LLVM development files (LLVM-C.lib) to be installed and discoverable
//!      via `LLVM_SYS_221_PREFIX` or `C:\Program Files\LLVM`.
//!   2. A working system linker (clang / link.exe / gcc).
//!   3. The `titrate_native` static library to have been built.
//!
//! Run it explicitly with:
//!   cargo test --test native_hello -- --ignored
//!
//! Or, to also build the prerequisites:
//!   cargo build -p titrate_native && cargo test --test native_hello -- --ignored

use std::env;
use std::path::PathBuf;
use std::process::Command;

/// Locate the workspace root by walking up from CARGO_MANIFEST_DIR.
fn workspace_root() -> PathBuf {
    let manifest = env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR should be set by cargo");
    PathBuf::from(manifest)
        .parent()
        .map(|p| p.to_path_buf())
        .expect("trc should be inside the workspace")
}

/// Locate the built `trc` binary in `target/debug` or `target/release`.
fn trc_binary() -> Option<PathBuf> {
    let root = workspace_root();
    let exe_name = if cfg!(windows) { "trc.exe" } else { "trc" };

    for profile in &["debug", "release"] {
        let candidate = root.join("target").join(profile).join(exe_name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Locate the built `titrate_native` static library.
fn native_lib_dir() -> Option<PathBuf> {
    let root = workspace_root();
    let lib_name = if cfg!(windows) {
        "titrate_native.lib"
    } else {
        "libtitrate_native.a"
    };

    for profile in &["debug", "release"] {
        let candidate = root.join("target").join(profile);
        if candidate.join(lib_name).is_file() {
            return Some(candidate);
        }
    }
    None
}

#[test]
#[ignore = "requires LLVM dev files, a system linker, and titrate_native built"]
fn native_hello_world_compiles_and_runs() {
    let trc = trc_binary().expect(
        "trc binary not found; run `cargo build -p trc` first",
    );
    let _native_dir = native_lib_dir().expect(
        "titrate_native static library not found; run \
         `cargo build -p titrate_native` first",
    );

    let hello_src = workspace_root().join("examples").join("hello.tr");
    assert!(
        hello_src.is_file(),
        "examples/hello.tr not found at {}",
        hello_src.display()
    );

    // Invoke `trc --native examples/hello.tr`.
    let compile_output = Command::new(&trc)
        .arg("--native")
        .arg(hello_src.to_str().unwrap())
        .output()
        .expect("failed to invoke trc");

    assert!(
        compile_output.status.success(),
        "trc --native failed.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&compile_output.stdout),
        String::from_utf8_lossy(&compile_output.stderr),
    );

    // The native binary should be written next to the source file.
    let native_exe = if cfg!(windows) {
        hello_src
            .with_file_name("hello_native.exe")
    } else {
        hello_src.with_file_name("hello_native")
    };
    assert!(
        native_exe.is_file(),
        "native binary was not produced at {}",
        native_exe.display()
    );

    // Run the native binary and capture its stdout.
    let run_output = Command::new(&native_exe)
        .output()
        .expect("failed to run the native binary");

    assert!(
        run_output.status.success(),
        "native binary exited with status {:?}\nstdout: {}\nstderr: {}",
        run_output.status.code(),
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr),
    );

    let stdout = String::from_utf8_lossy(&run_output.stdout);
    assert!(
        stdout.contains("Hello, World!"),
        "expected output to contain 'Hello, World!', got:\n{}",
        stdout,
    );

    // Clean up the produced binary so the test is idempotent.
    let _ = std::fs::remove_file(&native_exe);
}

#[test]
#[ignore = "requires LLVM dev files; verifies codegen path without linking"]
fn native_compile_only_produces_object_file() {
    // Lightweight smoke test: just verify that the codegen module can be
    // invoked and produces an object file. This skips the linker entirely,
    // which is useful for environments where a system linker is not on PATH.
    use trc::lexer;
    use trc::parser;
    use trc::analyzer;
    use trc::codegen::llvm;

    let source = r#"
public fn main(): void {
    io::println("Hello, World!");
}
"#;

    let tokens = lexer::tokenize(source).expect("tokenize failed");
    let ast = parser::parse(tokens).expect("parse failed");
    let typed_ast = analyzer::analyze(&ast).expect("analyze failed");

    let obj_path = env::temp_dir().join("trc_native_hello_test.o");
    llvm::compile(&typed_ast, &obj_path, false).expect("LLVM compile failed");

    assert!(
        obj_path.is_file(),
        "object file was not produced at {}",
        obj_path.display()
    );

    let _ = std::fs::remove_file(&obj_path);
}

#[test]
#[ignore = "requires LLVM dev files, a system linker, and titrate_native built"]
fn emit_ir_flag_writes_ll_file() {
    let trc = trc_binary().expect(
        "trc binary not found; run `cargo build -p trc` first",
    );

    let hello_src = workspace_root().join("examples").join("hello.tr");
    assert!(
        hello_src.is_file(),
        "examples/hello.tr not found at {}",
        hello_src.display()
    );

    // Invoke `trc --native --emit-ir examples/hello.tr`. This should produce
    // both the native executable and the `.ll` IR file beside the source.
    let compile_output = Command::new(&trc)
        .arg("--native")
        .arg("--emit-ir")
        .arg(hello_src.to_str().unwrap())
        .output()
        .expect("failed to invoke trc");

    assert!(
        compile_output.status.success(),
        "trc --native --emit-ir failed.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&compile_output.stdout),
        String::from_utf8_lossy(&compile_output.stderr),
    );

    // The `.ll` file is derived from the source stem: examples/hello.ll.
    let ll_path = hello_src.with_file_name("hello.ll");
    assert!(
        ll_path.is_file(),
        "LLVM IR file was not produced at {}",
        ll_path.display()
    );

    let ir = std::fs::read_to_string(&ll_path)
        .expect("failed to read the .ll file");
    assert!(
        ir.contains("define"),
        "expected LLVM IR to contain 'define', got:\n{}",
        ir
    );

    // Clean up the produced artifacts so the test is idempotent.
    let _ = std::fs::remove_file(&ll_path);
    let native_exe = if cfg!(windows) {
        hello_src.with_file_name("hello_native.exe")
    } else {
        hello_src.with_file_name("hello_native")
    };
    let _ = std::fs::remove_file(&native_exe);
}
