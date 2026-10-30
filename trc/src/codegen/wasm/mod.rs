//! WebAssembly backend (skeleton).
//!
//! Produces a `.wasm` module from a typed AST. Currently a placeholder;
//! full codegen will mirror the LLVM backend using a WASM encoder crate.

pub fn compile_to_wasm(_program: &crate::ast::Program) -> Result<Vec<u8>, String> {
    Err("WASM backend is not yet implemented".to_string())
}