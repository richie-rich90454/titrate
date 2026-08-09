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
pub mod thread;
pub mod mutex;
pub mod condvar;
pub mod semaphore;
pub mod atomic;
pub mod socket;
pub mod ssl;
pub mod sqlite;
pub mod mmap;
pub mod zlib;
pub mod lzma;
pub mod lz4;
pub mod zstd;
pub mod zip;
pub mod cryptoext;
pub mod multiprocessing;
pub mod ctypes;
pub mod platform;

mod lookup;

pub use lookup::lookup_builtin_native;

// ---------------------------------------------------------------------------
// Thread-local working directory for path resolution in native functions
// ---------------------------------------------------------------------------

use std::cell::RefCell;

thread_local! {
    static WORKING_DIR: RefCell<Option<std::path::PathBuf>> = const { RefCell::new(None) };
    static PROGRAM_ARGS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// Set the thread-local working directory used by native path resolution.
/// Called by the VM when `set_working_dir` is invoked.
pub fn set_native_working_dir(dir: Option<std::path::PathBuf>) {
    WORKING_DIR.with(|wd| {
        *wd.borrow_mut() = dir;
    });
}

/// Set the thread-local program arguments exposed to scripts via `Sys_args`.
/// `args[0]` is the program name; the remaining entries are the user-supplied
/// arguments (the script path itself is excluded).
pub fn set_program_args(args: Vec<String>) {
    PROGRAM_ARGS.with(|pa| {
        *pa.borrow_mut() = args;
    });
}

/// Return the current program arguments (`[program_name, user_args...]`).
pub fn get_program_args() -> Vec<String> {
    PROGRAM_ARGS.with(|pa| pa.borrow().clone())
}

/// Resolve a path using the thread-local working directory.
/// If the path is absolute, it is returned as-is.
/// If the path is relative and a working directory is set, the working
/// directory is prepended. Otherwise the path is returned as-is.
pub fn resolve_path(path: &str) -> std::path::PathBuf {
    let p = std::path::Path::new(path);
    if p.is_absolute() {
        return p.to_path_buf();
    }
    WORKING_DIR.with(|wd| {
        match wd.borrow().as_ref() {
            Some(dir) => dir.join(path),
            None => p.to_path_buf(),
        }
    })
}
