//! Integration tests for bytecode-compiler module import resolution.
//!
//! These tests verify that importing a module file makes its public
//! declarations reachable as bare identifiers, mirroring how the stdlib test
//! suite uses them (e.g. `import tt::net::Socket;` then a bare `AF_INET`, or
//! `import tt::math::ndarray::NDArray;` then a bare `fromData(...)` call).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use trc::bytecode;
use trc::lexer;
use trc::parser;

fn cli_case_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("trc_import_{}_{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn run_cli(dir: &Path, extra_args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_trc"))
        .arg(dir.join("main.tr"))
        .args(extra_args)
        .output()
        .expect("run trc")
}

/// Compile with modules from the workspace root and run via the bytecode VM,
/// returning the captured stdout joined by newlines.
fn run(src: &str) -> Result<String, String> {
    let tokens = lexer::tokenize(src).map_err(|e| e.to_string())?;
    let ast = parser::parse(tokens).map_err(|e| e.to_string())?;
    let root = PathBuf::from("..");
    let mut compiler = bytecode::Compiler::new();
    let compiled = compiler
        .compile_with_modules(&ast, &root)
        .map_err(|e| e.to_string())?;
    let mut vm = bytecode::Vm::new();
    vm.set_working_dir(root);
    vm.load_program(compiled);
    vm.run().map_err(|e| e.to_string())?;
    Ok(vm.output.join("\n"))
}

#[test]
fn strict_analysis_rejects_use_after_move_in_file_with_import() {
    let dir = cli_case_dir("use_after_move");
    fs::write(dir.join("helper.tr"), "public fn helper(): void { }\n").expect("write module");
    fs::write(
        dir.join("main.tr"),
        concat!(
            "import helper;\n",
            "public fn main(): void {\n",
            "    let x: Owned<int> = 5;\n",
            "    let y: Owned<int> = x;\n",
            "    let z = x;\n",
            "}\n",
        ),
    )
    .expect("write main");

    let out = run_cli(&dir, &[]);
    assert!(
        !out.status.success(),
        "expected rejection, stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("use of moved variable"), "stderr: {}", stderr);
    assert!(stderr.contains("error[E2001]:"), "stderr: {}", stderr);
}

#[test]
fn strict_analysis_accepts_valid_imported_program() {
    let dir = cli_case_dir("valid_import");
    fs::write(
        dir.join("helper.tr"),
        "public fn twice(x: int): int { return x * 2; }\n",
    )
    .expect("write module");
    fs::write(
        dir.join("main.tr"),
        concat!(
            "import helper;\n",
            "public fn main(): void {\n",
            "    let result: int = twice(21);\n",
            "    io::println(\"ok\");\n",
            "}\n",
        ),
    )
    .expect("write main");

    let out = run_cli(&dir, &[]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("ok"),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn compat_flag_restores_permissive_analysis() {
    let dir = cli_case_dir("compat");
    fs::write(dir.join("helper.tr"), "public fn helper(): void { }\n").expect("write module");
    fs::write(
        dir.join("main.tr"),
        concat!(
            "import helper;\n",
            "public fn main(): void {\n",
            "    let x: Owned<int> = 5;\n",
            "    let y: Owned<int> = x;\n",
            "    let z = x;\n",
            "}\n",
        ),
    )
    .expect("write main");

    let out = run_cli(&dir, &["--compat"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn bare_module_const_resolves_to_initialized_global() {
    let out = run(r#"
import tt::net::Socket;
public fn main(): void {
    io::println("AF_INET=" + Integer.toString(AF_INET));
    io::println("SOCK_STREAM=" + Integer.toString(SOCK_STREAM));
}
"#)
    .expect("run");
    assert!(out.contains("AF_INET=2"), "got: {}", out);
    assert!(out.contains("SOCK_STREAM=1"), "got: {}", out);
}

#[test]
fn bare_module_generic_function_resolves_and_instantiates() {
    let out = run(r#"
import tt::util::ArrayList;
import tt::math::ndarray::NDArray;
public fn main(): void {
    let data: ArrayList<double> = new ArrayList<double>();
    data.add(1.0); data.add(2.0);
    let s: ArrayList<int> = new ArrayList<int>();
    s.add(2); s.add(1);
    let a: NDArray<double> = fromData<double>(s, data);
    io::println("size=" + Integer.toString(a.size()));
}
"#)
    .expect("run");
    assert!(out.contains("size=2"), "got: {}", out);
}
