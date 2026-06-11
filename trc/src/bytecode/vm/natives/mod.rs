// Titrate Alpha 0.2 – bytecode virtual machine: native function modules
// Precision in every step – richie-rich90454, 2026

pub mod builtins;
pub mod file;
pub mod path;
pub mod directory;
pub mod system;
pub mod net;
pub mod time;
pub mod regex;
pub mod math;
pub mod random;
pub mod json;
pub mod string;
pub mod hash;
pub mod encoding;
pub mod subprocess;
pub mod tempfile;

mod lookup;

pub use lookup::lookup_builtin_native;
