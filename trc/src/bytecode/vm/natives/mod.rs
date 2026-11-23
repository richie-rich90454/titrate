// Titrate Alpha 0.2 – bytecode virtual machine: native function modules
// Precision in every step – richie-rich90454, 2026

pub mod builtins;
pub mod file;
pub mod path;
pub mod directory;
pub mod system;
pub mod system_os;
pub mod system_containers;
pub mod net;
pub(crate) mod ftp;
pub(crate) mod graphics_audio;
pub mod time;
pub mod regex;
pub mod math;
pub mod random;
pub mod slice;
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
pub mod os_unix;

mod lookup;

pub use lookup::lookup_builtin_native;

// ---------------------------------------------------------------------------
// Thread-local working directory for path resolution in native functions
// ---------------------------------------------------------------------------

use std::cell::RefCell;
use std::io::Read;
use std::sync::{Mutex as StdMutex, MutexGuard};

/// Cap for a single native read/decompression, both to bound memory and to
/// make hostile input fail with an error instead of exhausting the process.
pub(crate) const MAX_IO_BYTES: usize = 512 * 1024 * 1024;

/// Convert a signed count into a bounded length, rejecting negative values.
pub fn checked_len(n: i64, what: &str) -> Result<usize, String> {
    match usize::try_from(n) {
        Ok(v) if v <= effective_io_limit() => Ok(v),
        Ok(_) => Err(format!(
            "{}: size {} exceeds the {} byte limit",
            what, n, effective_io_limit()
        )),
        Err(_) => Err(format!("{}: negative size {}", what, n)),
    }
}

/// Read at most `MAX_IO_BYTES` from `reader`, failing instead of growing
/// without bound when the input is larger.
pub(crate) fn read_bounded<R: Read>(reader: R, what: &str) -> Result<Vec<u8>, String> {
    let limit = effective_io_limit();
    let mut bytes = Vec::new();
    let mut limited = reader.take(limit as u64 + 1);
    limited
        .read_to_end(&mut bytes)
        .map_err(|e| format!("{}: {}", what, e))?;
    if bytes.len() > limit {
        return Err(format!(
            "{}: output exceeds the {} byte limit",
            what, limit
        ));
    }
    Ok(bytes)
}

/// Lock a global registry, turning a poisoned lock into an error instead of a
/// panic. The guard is recovered so the poisoned state stays inspectable.
pub(crate) fn lock_registry<T>(mutex: &StdMutex<T>) -> Result<MutexGuard<'_, T>, String> {
    match mutex.lock() {
        Ok(guard) => Ok(guard),
        Err(poisoned) => {
            let guard = poisoned.into_inner();
            drop(guard);
            Err("internal state lock is poisoned".to_string())
        }
    }
}

/// Recover and return the guard even when a lock is poisoned, for helpers that
/// cannot return an error.
pub(crate) fn lock_registry_recover<T>(mutex: &StdMutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

thread_local! {
    static WORKING_DIR: RefCell<Option<std::path::PathBuf>> = const { RefCell::new(None) };
    static PROGRAM_ARGS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    // VM-wide filesystem sandbox for untrusted runs. Thread-local because the
    // VM is not Send and resolves paths on the thread that runs it.
    static SANDBOX_ROOT: RefCell<Option<std::path::PathBuf>> = const { RefCell::new(None) };
    static ALLOCATION_BUDGET: RefCell<usize> = const { RefCell::new(MAX_IO_BYTES) };
}

/// Install the sandbox root used to confine filesystem paths. `None` restores
/// trusted, unrestricted resolution. Called by `Vm::set_policy`.
pub fn set_sandbox_root(root: Option<std::path::PathBuf>) {
    SANDBOX_ROOT.with(|sr| {
        *sr.borrow_mut() = root;
    });
}

/// Set the per-allocation byte budget applied by `checked_len`/`read_bounded`.
/// Called by `Vm::set_policy`.
pub fn set_allocation_budget(bytes: usize) {
    ALLOCATION_BUDGET.with(|b| {
        *b.borrow_mut() = bytes;
    });
}

fn effective_io_limit() -> usize {
    ALLOCATION_BUDGET.with(|b| (*b.borrow()).min(MAX_IO_BYTES))
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

#[cfg(test)]
mod safety_tests {
    use super::*;
    use crate::bytecode::value::Value;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;
    use std::sync::{Arc, Mutex};

    fn s(text: &str) -> Value {
        Value::String(Rc::new(text.to_string()))
    }

    fn text(value: Value) -> String {
        match value {
            Value::String(s) => s.to_string(),
            _ => panic!("expected a String value"),
        }
    }

    #[test]
    fn bounded_lengths_and_reads() {
        assert_eq!(super::checked_len(5, "test").unwrap(), 5);
        assert!(super::checked_len(-1, "test").is_err());
        assert!(super::checked_len(super::MAX_IO_BYTES as i64 + 1, "test").is_err());
        assert_eq!(super::read_bounded(&b"abc"[..], "test").unwrap(), b"abc");
    }

    #[test]
    fn negative_lengths_are_rejected() {
        let negative = Value::Long(-1);
        assert!(super::file::native_file_read_chunk(
            &[s("missing"), Value::Long(0), negative.clone()]
        )
        .is_err());
        assert!(super::file::native_file_read_bytes(&[s("missing"), negative.clone()]).is_err());
        assert!(super::hash::native_hash_shake128(&[s("x"), negative.clone()]).is_err());
        assert!(super::hash::native_hash_shake256(&[s("x"), negative.clone()]).is_err());
        assert!(super::string::native_string_pad_left(
            &[s("x"), negative.clone(), Value::Char(' ')]
        )
        .is_err());
        assert!(super::string::native_string_pad_right(
            &[s("x"), negative.clone(), Value::Char(' ')]
        )
        .is_err());
        assert!(super::system_os::native_os_urandom(&[negative.clone()]).is_err());
        assert!(super::ssl::native_ssl_recv(&[Value::Long(1), negative.clone()]).is_err());
        assert!(super::socket::native_socket_recv(&[Value::Long(1), negative.clone()]).is_err());
        assert!(super::ftp::native_ftp_read_data(&[Value::Long(1), negative.clone()]).is_err());
        assert!(super::net::native_net_receive(&[Value::Int(0), negative]).is_err());
    }

    #[test]
    fn unicode_hex_input_is_rejected_without_panicking() {
        assert!(super::encoding::native_hex_decode(&[s("\u{e9}")]).is_err());
        assert!(matches!(
            super::file::native_file_write_bytes(&[s("unused"), s("\u{e9}\u{e9}")]),
            Ok(Value::ResultErr(_))
        ));
        assert_eq!(text(super::encoding::native_hex_decode(&[s("41")]).unwrap()), "A");
    }

    #[test]
    fn json_depth_is_bounded() {
        let deep = format!("{}1{}", "[".repeat(300), "]".repeat(300));
        assert!(super::json::native_json_parse(&[s(&deep)]).is_err());
        let shallow = format!("{}1{}", "[".repeat(10), "]".repeat(10));
        assert!(super::json::native_json_parse(&[s(&shallow)]).is_ok());
    }

    #[test]
    fn json_stringify_rejects_cycles_and_escapes_controls() {
        let fields = Rc::new(RefCell::new(HashMap::new()));
        let node = Value::ClassInstance {
            class_name: "Node".to_string(),
            fields: fields.clone(),
            vtable: HashMap::new(),
        };
        fields.borrow_mut().insert("self".to_string(), node.clone());
        assert!(super::json::native_json_stringify(&[node]).is_err());

        let escaped = super::json::native_json_stringify(&[s("a\u{08}b\u{0c}c\u{01}")]).unwrap();
        assert_eq!(text(escaped), "\"a\\bb\\fc\\u0001\"");
    }

    #[test]
    fn json_stringify_depth_is_bounded() {
        let mut value = Value::Int(1);
        for _ in 0..300 {
            value = Value::Array {
                elements: vec![value],
            };
        }
        assert!(super::json::native_json_stringify(&[value]).is_err());
    }

    #[test]
    fn http_configuration_is_applied() {
        assert!(super::net::native_http_set_timeout(&[Value::Long(-1)]).is_err());
        assert!(super::net::native_http_set_timeout(&[Value::Long(100)]).is_ok());
        assert!(super::net::native_http_set_follow_redirects(&[Value::Bool(false)]).is_ok());
        let _ = super::net::native_http_set_timeout(&[Value::Long(30_000)]);
        let _ = super::net::native_http_set_follow_redirects(&[Value::Bool(true)]);
    }

    #[test]
    fn math_random_is_in_unit_interval() {
        for _ in 0..8 {
            match super::math::native_math_random(&[]).unwrap() {
                Value::Double(v) => assert!((0.0..1.0).contains(&v)),
                _ => panic!("expected a Double value"),
            }
        }
    }

    #[test]
    fn poisoned_lock_is_reported_and_recovered() {
        let lock = Arc::new(Mutex::new(7));
        let worker_lock = lock.clone();
        let _ = std::thread::spawn(move || {
            let _guard = worker_lock.lock().unwrap();
            panic!("poison the lock");
        })
        .join();

        assert!(super::lock_registry(&lock).is_err());
        assert_eq!(*super::lock_registry_recover(&lock), 7);
    }
}

/// Reject environment variable names that would make `std::env` panic.
pub fn validate_env_key(key: &str, op: &str) -> Result<(), String> {
    if key.is_empty() || key.contains('=') || key.contains('\0') {
        return Err(format!("{}: invalid environment variable name", op));
    }
    Ok(())
}

/// Resolve a path using the thread-local working directory. When a sandbox root
/// is installed (untrusted mode), the result is confined under it: absolute
/// paths are re-rooted so they cannot address the host filesystem, and `..`
/// traversal cannot escape. Without a root, resolution is unrestricted.
pub fn resolve_path(path: &str) -> std::path::PathBuf {
    let p = std::path::Path::new(path);
    let resolved = if p.is_absolute() {
        p.to_path_buf()
    } else {
        WORKING_DIR.with(|wd| match wd.borrow().as_ref() {
            Some(dir) => dir.join(path),
            None => p.to_path_buf(),
        })
    };
    SANDBOX_ROOT.with(|sr| match sr.borrow().as_ref() {
        Some(root) => confine_to_root(resolved, root),
        None => resolved,
    })
}

pub(crate) fn confine_to_root(
    path: std::path::PathBuf,
    root: &std::path::Path,
) -> std::path::PathBuf {
    use std::path::Component;

    let root = if root.is_absolute() {
        root.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(root)
    };
    let root = normalize_path(&root);
    let path = normalize_path(&path);
    if path.starts_with(&root) {
        return path;
    }
    // Re-root absolute paths and drop parent traversal so neither can escape.
    let mut confined = root;
    for component in path.components() {
        if let Component::Normal(part) = component {
            confined.push(part);
        }
    }
    confined
}

fn normalize_path(path: &std::path::Path) -> std::path::PathBuf {
    use std::path::Component;

    let mut out = std::path::PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => out.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(part) => out.push(part),
        }
    }
    out
}
