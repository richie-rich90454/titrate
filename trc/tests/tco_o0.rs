//! Regression: compiler-level tail-call elimination must keep the native
//! stack flat in a DEFAULT (debug, -O0) `trc --native` build, not just in
//! `--release`. The `countdown` function recurses 1,000,000 deep; LLVM may
//! ignore a plain `tail` hint at -O0, risking C-stack growth. The backend now
//! rebinds parameter allocas and branches back to a loop header, independent
//! of any optimization pass.
//!
//! The native test is ignored because it needs:
//!   1. LLVM development files (LLVM-C.lib) discoverable via
//!      `LLVM_SYS_221_PREFIX` or `C:\Program Files\LLVM`.
//!   2. The `titrate_native` static library built with
//!      `cargo build -p titrate_native` (the `--native` build here is O0, so
//!      it links the debug profile of the library).
//!   3. A working system linker (clang / link.exe / gcc).
//!
//! Run it explicitly with:
//!   cargo test --test tco_o0 -- --ignored

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;

use trc::bytecode;
use trc::lexer;
use trc::parser;

const DEPTH: u64 = 1_000_000;

fn source() -> String {
    format!(
        r#"fn countdown(n: int): int {{
    if (n <= 0) {{
        return 0;
    }}
    return countdown(n - 1);
}}

public fn main(): void {{
    io::println(countdown({}));
}}
"#,
        DEPTH
    )
}

/// Run the same program on the bytecode VM to get the reference output.
fn vm_output() -> String {
    let src = source();
    let tokens = lexer::tokenize(&src).expect("lex error");
    let ast = parser::parse(tokens).expect("parse error");
    let root_dir = PathBuf::from("..");
    let mut compiler = bytecode::Compiler::new();
    let compiled = compiler
        .compile_with_modules(&ast, &root_dir)
        .expect("compiler error");
    let mut vm = bytecode::Vm::new();
    vm.set_working_dir(root_dir);
    vm.load_program(compiled);
    vm.run().expect("vm run error (TCO frames exhausted?)");
    vm.output.join("\n").trim_end().to_string()
}

// Linking is serialized to avoid LNK1104 on concurrent temp-output writes.
static LINK_LOCK: Mutex<()> = Mutex::new(());

fn workspace_root() -> PathBuf {
    PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("trc inside workspace")
        .to_path_buf()
}

fn trc_binary() -> Option<PathBuf> {
    let root = workspace_root();
    let exe = if cfg!(windows) { "trc.exe" } else { "trc" };
    for profile in &["debug", "release"] {
        let candidate = root.join("target").join(profile).join(exe);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

#[test]
#[ignore = "requires LLVM dev files, a system linker, and a built titrate_native static library"]
fn native_o0_tail_recursion_one_million() {
    let _guard = LINK_LOCK.lock().unwrap();
    let trc = trc_binary().expect("trc binary not found; run `cargo build -p trc` first");
    let dir = env::temp_dir();
    let src_path = dir.join("trc_tco_o0.tr");
    fs::write(&src_path, source()).expect("failed to write repro source");

    // Deliberately no --release: the generated code must be -O0.
    let compile_output = Command::new(&trc)
        .arg("--native")
        .arg(&src_path)
        .current_dir(&dir)
        .output()
        .expect("failed to invoke trc");
    assert!(
        compile_output.status.success(),
        "trc --native (-O0) failed.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&compile_output.stdout),
        String::from_utf8_lossy(&compile_output.stderr),
    );

    let exe_name = if cfg!(windows) {
        "trc_tco_o0_native.exe"
    } else {
        "trc_tco_o0_native"
    };
    let native_exe = dir.join(exe_name);
    assert!(
        native_exe.is_file(),
        "native binary not produced at {}",
        native_exe.display()
    );

    let run_output = Command::new(&native_exe)
        .current_dir(&dir)
        .output()
        .expect("failed to run native binary");
    assert!(
        run_output.status.success(),
        "native -O0 binary exited {:?} (stack overflow?).\nstdout: {}\nstderr: {}",
        run_output.status.code(),
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr),
    );

    let native_stdout = String::from_utf8_lossy(&run_output.stdout)
        .replace("\r\n", "\n")
        .trim_end()
        .to_string();
    assert_eq!(native_stdout, "0");
    assert_eq!(native_stdout, vm_output(), "native -O0 output must match VM");

    let _ = fs::remove_file(&src_path);
    let _ = fs::remove_file(&native_exe);
}
