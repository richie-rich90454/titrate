//! Integration tests for bytecode-compiler module import resolution.
//!
//! These tests verify that importing a module file makes its public
//! declarations reachable as bare identifiers, mirroring how the stdlib test
//! suite uses them (e.g. `import tt::net::Socket;` then a bare `AF_INET`, or
//! `import tt::math::ndarray::NDArray;` then a bare `fromData(...)` call).

use std::path::PathBuf;
use trc::bytecode;
use trc::lexer;
use trc::parser;

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
