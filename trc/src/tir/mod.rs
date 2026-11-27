//! Shared typed intermediate representation for the Titrate compiler.
//!
//! The WASM backend now consumes TIR directly through
//! [`crate::codegen::wasm::compile_tir_to_wasm`]; the bytecode VM and LLVM
//! backends still lower independently from the AST.

mod lower;
pub mod integration;
mod types;

pub use lower::lower;
pub use types::*;
