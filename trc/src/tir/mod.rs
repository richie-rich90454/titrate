//! Shared typed intermediate representation for the Titrate compiler.
//!
//! Foundation only: no codegen or VM backend consumes TIR yet.

mod lower;
pub mod integration;
mod types;

pub use lower::lower;
pub use types::*;
