// Titrate Alpha 0.2 – bytecode virtual machine: ctypes FFI natives
// Precision in every step – richie-rich90454, 2026
//
// Native support for the tt::lang::Ctypes stdlib module.
//
// These natives provide the handle-based FFI binding surface that the
// stdlib Ctypes module builds upon. Real cross-platform dlopen/dlsym plus
// signature-driven function dispatch requires the `libloading` crate and a
// type-tagged argument marshalling layer that the VM does not currently
// expose; accordingly these implementations are stubs that return fake
// handles and record the requested library/symbol names in registries so
// the stdlib API is functional (open/lookup report success, call/load
// return Null). A future revision can swap the registry bodies for real
// `libloading` calls without changing the handle contract.

use super::super::super::value::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{LazyLock, Mutex as StdMutex};

// ---------------------------------------------------------------------------
// Library registry — records dlopen requests keyed by handle
// ---------------------------------------------------------------------------

static LIB_REGISTRY: LazyLock<StdMutex<HashMap<i64, String>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));
static LIB_NEXT_HANDLE: AtomicI64 = AtomicI64::new(1);

// ---------------------------------------------------------------------------
// Symbol registry — records dlsym requests keyed by handle
// ---------------------------------------------------------------------------

// Fields are recorded for future `libloading`-based resolution (see file
// header) and are validated via the registry key, so they are not read yet.
#[allow(dead_code)]
struct SymbolEntry {
    lib_handle: i64,
    name: String,
}

static SYM_REGISTRY: LazyLock<StdMutex<HashMap<i64, SymbolEntry>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));
static SYM_NEXT_HANDLE: AtomicI64 = AtomicI64::new(1);

/// Ctypes_dlopen(path: String) -> Long handle
///
/// Records a request to load the shared library at `path` and returns a
/// handle. On platforms without `libloading`, the library is not actually
/// loaded; the handle is a registry key. Returns an error if `path` is not
/// a string.
pub(crate) fn native_ctypes_dlopen(args: &[Value]) -> Result<Value, String> {
    let path = match args.first() {
        Some(Value::String(s)) => s.as_str().to_string(),
        _ => return Err("Ctypes_dlopen: expected a String library path".to_string()),
    };
    let handle = LIB_NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
    let mut registry = LIB_REGISTRY.lock().unwrap();
    registry.insert(handle, path);
    Ok(Value::Long(handle))
}

/// Ctypes_dlsym(libHandle: Long, name: String) -> Long symHandle
///
/// Records a request to look up `name` in the library referenced by
/// `libHandle` and returns a symbol handle. The lookup is not actually
/// performed; the handle is a registry key. Returns an error if the
/// library handle is invalid or `name` is not a string.
pub(crate) fn native_ctypes_dlsym(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Ctypes_dlsym: expected 2 arguments (libHandle, name)".to_string());
    }
    let lib_handle = match &args[0] {
        Value::Long(h) => *h,
        Value::Int(h) => *h as i64,
        _ => return Err("Ctypes_dlsym: expected an Int/Long library handle".to_string()),
    };
    let name = match &args[1] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Ctypes_dlsym: expected a String symbol name".to_string()),
    };
    // Verify the library handle exists.
    {
        let registry = LIB_REGISTRY.lock().unwrap();
        if !registry.contains_key(&lib_handle) {
            return Err("Ctypes_dlsym: invalid library handle".to_string());
        }
    }
    let handle = SYM_NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
    let mut registry = SYM_REGISTRY.lock().unwrap();
    registry.insert(handle, SymbolEntry { lib_handle, name });
    Ok(Value::Long(handle))
}

/// Ctypes_call(symHandle: Long, [args...]) -> Null
///
/// Stub: real FFI function dispatch requires a type-tagged calling
/// convention that the VM does not currently expose. The symbol handle is
/// validated and the call is reported as successful with a Null return.
pub(crate) fn native_ctypes_call(args: &[Value]) -> Result<Value, String> {
    let sym_handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Ctypes_call: expected an Int/Long symbol handle".to_string()),
    };
    let registry = SYM_REGISTRY.lock().unwrap();
    if !registry.contains_key(&sym_handle) {
        return Err("Ctypes_call: invalid symbol handle".to_string());
    }
    // Arguments (args[1..]) are accepted but ignored — no calling convention.
    Ok(Value::Null)
}

/// Ctypes_load(libHandle: Long, name: String) -> Null
///
/// Stub: loads a global/constant from the referenced library. Real
/// implementation requires symbol resolution with a known type; this stub
/// validates the library handle and returns Null.
pub(crate) fn native_ctypes_load(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Ctypes_load: expected 2 arguments (libHandle, name)".to_string());
    }
    let lib_handle = match &args[0] {
        Value::Long(h) => *h,
        Value::Int(h) => *h as i64,
        _ => return Err("Ctypes_load: expected an Int/Long library handle".to_string()),
    };
    let _name = match &args[1] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Ctypes_load: expected a String symbol name".to_string()),
    };
    let registry = LIB_REGISTRY.lock().unwrap();
    if !registry.contains_key(&lib_handle) {
        return Err("Ctypes_load: invalid library handle".to_string());
    }
    Ok(Value::Null)
}
