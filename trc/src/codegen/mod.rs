//! Native code generation backends.
//!
//! Phase 0 wires up the LLVM backend via the `inkwell` crate. The module
//! exposes a single entry point, [`llvm::compile`], that lowers a typed
//! Titrate AST to a native object file.

pub mod llvm;
