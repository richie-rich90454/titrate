// Titrate Alpha 0.2 – crafted by richie-rich90454, 2026

pub mod compiler;
pub mod frame;
pub mod opcodes;
pub mod value;
mod value_impl;
pub mod vm;

pub use compiler::{CompiledProgram, Compiler};
pub use opcodes::{CastTarget, Chunk, OpCode, TypeTag};
pub use vm::Vm;

use crate::ast;

/// Compile and run a Titrate program through the bytecode VM.
/// Returns captured output lines.
pub fn execute(program: &ast::Program) -> Result<Vec<String>, String> {
    let mut compiler = Compiler::new();
    let compiled = compiler.compile(program)?;

    let mut vm = Vm::new();
    vm.load_program(compiled);
    vm.run()?;
    Ok(vm.output)
}
