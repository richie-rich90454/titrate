// Titrate Alpha 0.2 – crafted by richie-rich90454, 2026

pub mod compiler;
pub mod frame;
pub mod opcodes;
pub mod chunk;
pub mod value;
mod value_impl;
pub mod vm;

pub use compiler::{CompiledProgram, Compiler};
pub use opcodes::{CastTarget, OpCode, TypeTag};
pub use chunk::Chunk;
pub use vm::Vm;

use crate::ast;

/// Compile and run a Titrate program through the bytecode VM.
/// Returns captured output lines.
pub fn execute(program: &ast::Program) -> Result<Vec<String>, String> {
    execute_with_root(program, std::path::Path::new("."))
}

/// Compile and run with a root directory for module resolution.
pub fn execute_with_root(program: &ast::Program, root_dir: &std::path::Path) -> Result<Vec<String>, String> {
    let mut compiler = Compiler::new();
    let compiled = if program.imports.is_empty() {
        compiler.compile(program)?
    } else {
        compiler.compile_with_modules(program, root_dir)?
    };

    let mut vm = Vm::new();
    vm.load_program(compiled);
    vm.run()?;
    Ok(vm.output)
}
