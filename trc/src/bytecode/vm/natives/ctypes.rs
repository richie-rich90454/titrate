// Titrate Alpha 0.4 – bytecode virtual machine: ctypes FFI natives
// Precision in every step – richie-rich90454, 2026
//
// Real cross-platform foreign function interface backed by the
// `libloading` crate.
//
// Contract with tt::ffi::Ctypes:
//   - Ctypes_dlopen(path) -> Long handle. Really loads the shared
//     library; the handle owns the loaded library (it stays loaded until
//     Ctypes_dlclose).
//   - Ctypes_dlsym(libHandle, name) -> Long symbol handle. Really resolves
//     the symbol; errors when the library handle is invalid or the symbol
//     is missing.
//   - Ctypes_dlclose(libHandle) -> Null. Unloads the library. Errors when
//     symbols resolved from it are still registered, so a live function
//     pointer can never dangle.
//   - Ctypes_call(symHandle, argsArrayList, returnCType) -> value. Really
//     calls the function pointer with a runtime-typed dispatch over the
//     declared return kind and the runtime argument values. Supported
//     argument cells are integers/bools (i64), doubles (f64) and pointers
//     (strings passed as NUL-terminated C strings, integer addresses
//     passed through). Supported arities: homogeneous int-like 0..=8,
//     homogeneous doubles 0..=4, and string/int mixes up to 4 arguments
//     (covers strlen/strcmp/atoi/puts/Beep/MessageBox-style signatures).
//     Anything else is an explicit error naming the unsupported shape —
//     never a silent Null.
//   - Ctypes_load(libHandle, name) -> Null. Resolves `name` and validates
//     it points at a readable NUL-terminated C string global; kept for
//     API compatibility (reading arbitrary typed globals needs a type,
//     which this entry point does not carry).
//
// Handles are 64-bit Long values. On x86_64 (all supported targets) the
// C and system calling conventions coincide, so `extern "C"` pointers
// are correct for both CDLL and WinDLL.

use super::super::super::value::Value;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_void;
use std::rc::Rc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{LazyLock, Mutex as StdMutex};

// ---------------------------------------------------------------------------
// Registries
// ---------------------------------------------------------------------------

static LIB_REGISTRY: LazyLock<StdMutex<HashMap<i64, libloading::Library>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));
static LIB_NEXT_HANDLE: AtomicI64 = AtomicI64::new(1);

struct SymbolEntry {
    lib_handle: i64,
    /// Raw function pointer as an integer (Send-safe). Valid while the
    /// owning Library lives in LIB_REGISTRY; dlclose refuses to unload
    /// while symbols from it are registered.
    ptr: usize,
}

static SYM_REGISTRY: LazyLock<StdMutex<HashMap<i64, SymbolEntry>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));
static SYM_NEXT_HANDLE: AtomicI64 = AtomicI64::new(1);

// ---------------------------------------------------------------------------
// Helpers: Titrate values <-> C cells
// ---------------------------------------------------------------------------

/// Elements of a Titrate ArrayList (stored as _elements in ClassInstance).
fn extract_array_elements(val: &Value) -> Vec<Value> {
    match val {
        Value::Array { elements } => elements.clone(),
        Value::ClassInstance { fields, .. } => {
            let fields = fields.borrow();
            if let Some(Value::Array { elements }) = fields.get("_elements") {
                elements.clone()
            } else {
                vec![]
            }
        }
        _ => vec![],
    }
}

fn class_field(val: &Value, name: &str) -> Option<Value> {
    match val {
        Value::ClassInstance { fields, .. } => fields.borrow().get(name).cloned(),
        _ => None,
    }
}

fn class_name_of(val: &Value) -> Option<String> {
    match val {
        Value::ClassInstance { class_name, .. } => Some(class_name.clone()),
        _ => None,
    }
}

/// Unwrap a CData instance to its inner `value`; pass other values through.
fn unwrap_cdata(val: &Value) -> Value {
    if class_name_of(val).as_deref() == Some("CData") {
        return class_field(val, "value").unwrap_or(Value::Null);
    }
    val.clone()
}

fn value_to_i64(val: &Value) -> Option<i64> {
    match val {
        Value::Int(i) => Some(*i as i64),
        Value::Long(l) => Some(*l),
        Value::Bool(b) => Some(if *b { 1 } else { 0 }),
        Value::Byte(b) => Some(*b as i64),
        _ => None,
    }
}

fn value_to_f64(val: &Value) -> Option<f64> {
    match val {
        Value::Double(d) => Some(*d),
        Value::Float(f) => Some(*f as f64),
        _ => None,
    }
}

fn value_to_string_bytes(val: &Value) -> Option<Vec<u8>> {
    match val {
        Value::String(s) => Some(s.as_bytes().to_vec()),
        _ => None,
    }
}

/// Declared return kind from the CType ClassInstance (`kind` field:
/// "int" | "uint" | "float" | "char" | "ptr").
fn return_kind(val: &Value) -> String {
    class_field(val, "kind")
        .and_then(|v| match v {
            Value::String(s) => Some(s.as_str().to_string()),
            _ => None,
        })
        .unwrap_or_else(|| "int".to_string())
}

fn return_name(val: &Value) -> String {
    class_field(val, "name")
        .and_then(|v| match v {
            Value::String(s) => Some(s.as_str().to_string()),
            _ => None,
        })
        .unwrap_or_else(|| "int".to_string())
}

// ---------------------------------------------------------------------------
// Library / symbol lifetime management
// ---------------------------------------------------------------------------

/// Ctypes_dlopen(path: String) -> Long handle
pub(crate) fn native_ctypes_dlopen(args: &[Value]) -> Result<Value, String> {
    let path = match args.first() {
        Some(Value::String(s)) => s.as_str().to_string(),
        _ => return Err("Ctypes_dlopen: expected a String library path".to_string()),
    };
    // SAFETY: loading a named shared library is the documented purpose of
    // this entry point; the Library value is retained in the registry so
    // its symbols stay valid until dlclose.
    let lib = unsafe { libloading::Library::new(&path) }
        .map_err(|e| format!("Ctypes_dlopen: failed to load '{}': {}", path, e))?;
    let handle = LIB_NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
    LIB_REGISTRY.lock().unwrap().insert(handle, lib);
    Ok(Value::Long(handle))
}

/// Ctypes_dlsym(libHandle: Long, name: String) -> Long symHandle
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
    let registry = LIB_REGISTRY.lock().unwrap();
    let lib = registry
        .get(&lib_handle)
        .ok_or_else(|| "Ctypes_dlsym: invalid library handle".to_string())?;
    // SAFETY: the pointer stays valid while the owning Library lives in
    // the registry; dlclose refuses to unload while symbols are live.
    // The Symbol is dereferenced immediately and only the address escapes.
    let ptr = unsafe {
        let sym: libloading::Symbol<*mut c_void> = lib
            .get(name.as_bytes())
            .map_err(|_| format!("Ctypes_dlsym: symbol '{}' not found", name))?;
        *sym as usize
    };
    drop(registry);
    let handle = SYM_NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
    SYM_REGISTRY
        .lock()
        .unwrap()
        .insert(handle, SymbolEntry { lib_handle, ptr });
    Ok(Value::Long(handle))
}

/// Ctypes_dlclose(libHandle: Long) -> Null
pub(crate) fn native_ctypes_dlclose(args: &[Value]) -> Result<Value, String> {
    let lib_handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Ctypes_dlclose: expected an Int/Long library handle".to_string()),
    };
    {
        let syms = SYM_REGISTRY.lock().unwrap();
        if syms.values().any(|e| e.lib_handle == lib_handle) {
            return Err(
                "Ctypes_dlclose: library still has live symbols; release them first"
                    .to_string(),
            );
        }
    }
    let mut registry = LIB_REGISTRY.lock().unwrap();
    registry
        .remove(&lib_handle)
        .ok_or_else(|| "Ctypes_dlclose: invalid library handle".to_string())?;
    Ok(Value::Null)
}

/// Ctypes_symclose(symHandle: Long) -> Null
///
/// Releases a resolved symbol without unloading its library.
pub(crate) fn native_ctypes_symclose(args: &[Value]) -> Result<Value, String> {
    let sym_handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Ctypes_symclose: expected an Int/Long symbol handle".to_string()),
    };
    SYM_REGISTRY
        .lock()
        .unwrap()
        .remove(&sym_handle)
        .ok_or_else(|| "Ctypes_symclose: invalid symbol handle".to_string())?;
    Ok(Value::Null)
}

// ---------------------------------------------------------------------------
// Typed call dispatch
// ---------------------------------------------------------------------------

enum Cell {
    Int(i64),
    Float(f64),
    Ptr(*const u8),
}

enum RetKind {
    Void,
    Int,
    Long,
    Double,
    Bool,
    CString,
    Pointer,
}

fn classify_return(kind: &str, name: &str) -> RetKind {
    match (kind, name) {
        (_, "void") => RetKind::Void,
        (_, "bool") => RetKind::Bool,
        (_, "float") | (_, "double") => RetKind::Double,
        (_, "char_p") => RetKind::CString,
        (_, "void_p") => RetKind::Pointer,
        ("float", _) => RetKind::Double,
        ("ptr", _) => RetKind::Pointer,
        (_, "long") | (_, "ulong") | (_, "longlong") | (_, "ulonglong") => RetKind::Long,
        _ => RetKind::Int,
    }
}

fn wrap_int_result(kind: &RetKind, r: i64) -> Value {
    match kind {
        RetKind::Void => Value::Null,
        RetKind::Bool => Value::Bool(r != 0),
        RetKind::Long | RetKind::Pointer => Value::Long(r),
        RetKind::Double => Value::Double(r as f64),
        RetKind::CString => Value::Long(r),
        RetKind::Int => Value::Int(r as i32),
    }
}

fn wrap_float_result(kind: &RetKind, r: f64) -> Value {
    match kind {
        RetKind::Void => Value::Null,
        RetKind::Bool => Value::Bool(r != 0.0),
        RetKind::Long | RetKind::Pointer => Value::Long(r as i64),
        RetKind::Double => Value::Double(r),
        RetKind::CString => Value::Long(r as i64),
        RetKind::Int => Value::Int(r as i32),
    }
}

fn wrap_ptr_result(kind: &RetKind, r: *const u8) -> Value {
    match kind {
        RetKind::Void => Value::Null,
        RetKind::CString => {
            if r.is_null() {
                return Value::Null;
            }
            // SAFETY: the callee promises a NUL-terminated string for
            // char_p returns; read it immediately while the library is
            // loaded (the registry lock is held by the caller path).
            let s = unsafe { CStr::from_ptr(r as *const i8) }
                .to_string_lossy()
                .into_owned();
            Value::String(Rc::new(s))
        }
        RetKind::Bool => Value::Bool(!r.is_null()),
        RetKind::Double => Value::Double(r as usize as f64),
        RetKind::Long | RetKind::Pointer => Value::Long(r as usize as i64),
        RetKind::Int => Value::Int(r as usize as i32),
    }
}

/// Ctypes_call(symHandle: Long, args: ArrayList, returnCType: CType) -> value
pub(crate) fn native_ctypes_call(args: &[Value]) -> Result<Value, String> {
    if args.len() < 3 {
        return Err(
            "Ctypes_call: expected 3 arguments (symHandle, args, returnType)".to_string(),
        );
    }
    let sym_handle = match &args[0] {
        Value::Long(h) => *h,
        Value::Int(h) => *h as i64,
        _ => return Err("Ctypes_call: expected an Int/Long symbol handle".to_string()),
    };
    let ptr: usize = {
        let registry = SYM_REGISTRY.lock().unwrap();
        registry
            .get(&sym_handle)
            .map(|e| e.ptr)
            .ok_or_else(|| "Ctypes_call: invalid symbol handle".to_string())?
    };
    if ptr == 0 {
        return Err("Ctypes_call: null function pointer".to_string());
    }
    let raw_args = extract_array_elements(&args[1]);
    let ret_kind = classify_return(&return_kind(&args[2]), &return_name(&args[2]));

    // Keep C strings alive for the duration of the call.
    let mut cstrings: Vec<CString> = Vec::new();
    let mut cells: Vec<Cell> = Vec::with_capacity(raw_args.len());
    for v in &raw_args {
        let u = unwrap_cdata(v);
        if let Some(i) = value_to_i64(&u) {
            cells.push(Cell::Int(i));
        } else if let Some(f) = value_to_f64(&u) {
            cells.push(Cell::Float(f));
        } else if let Some(bytes) = value_to_string_bytes(&u) {
            let cs = CString::new(bytes)
                .map_err(|_| "Ctypes_call: string argument contains NUL".to_string())?;
            let p = cs.as_ptr() as *const u8;
            cstrings.push(cs);
            cells.push(Cell::Ptr(p));
        } else if matches!(u, Value::Null) {
            cells.push(Cell::Ptr(std::ptr::null()));
        } else {
            return Err(
                "Ctypes_call: unsupported argument type (use int/long/double/bool/string)"
                    .to_string(),
            );
        }
    }

    let has_float = cells.iter().any(|c| matches!(c, Cell::Float(_)));
    let has_ptr = cells.iter().any(|c| matches!(c, Cell::Ptr(_)));
    // SAFETY: the pointer was validated at dlsym time and its library is
    // pinned in the registry; the casts below match the classified cells.
    unsafe {
        if !has_float && !has_ptr {
            let ints: Vec<i64> = cells
                .iter()
                .map(|c| match c {
                    Cell::Int(i) => *i,
                    _ => 0,
                })
                .collect();
            return call_int_arity(ptr, &ret_kind, &ints);
        }
        if !has_float {
            return call_ptr_mixed(ptr, &ret_kind, &cells);
        }
        if !has_ptr {
            let floats: Vec<f64> = cells
                .iter()
                .map(|c| match c {
                    Cell::Float(f) => *f,
                    Cell::Int(i) => *i as f64,
                    _ => 0.0,
                })
                .collect();
            if cells.iter().all(|c| matches!(c, Cell::Float(_))) {
                return call_float_arity(ptr, &ret_kind, &floats);
            }
        }
    }
    Err("Ctypes_call: unsupported signature (mix int/double/pointer freely only as documented)".to_string())
}

unsafe fn call_int_arity(ptr: usize, ret: &RetKind, a: &[i64]) -> Result<Value, String> {
    let ptr = ptr as *mut c_void;
    unsafe {
        let r: i64 = match a.len() {
            0 => {
                let f: extern "C" fn() -> i64 = std::mem::transmute(ptr);
                f()
            }
            1 => {
                let f: extern "C" fn(i64) -> i64 = std::mem::transmute(ptr);
                f(a[0])
            }
            2 => {
                let f: extern "C" fn(i64, i64) -> i64 = std::mem::transmute(ptr);
                f(a[0], a[1])
            }
            3 => {
                let f: extern "C" fn(i64, i64, i64) -> i64 = std::mem::transmute(ptr);
                f(a[0], a[1], a[2])
            }
            4 => {
                let f: extern "C" fn(i64, i64, i64, i64) -> i64 =
                    std::mem::transmute(ptr);
                f(a[0], a[1], a[2], a[3])
            }
            5 => {
                let f: extern "C" fn(i64, i64, i64, i64, i64) -> i64 =
                    std::mem::transmute(ptr);
                f(a[0], a[1], a[2], a[3], a[4])
            }
            6 => {
                let f: extern "C" fn(i64, i64, i64, i64, i64, i64) -> i64 =
                    std::mem::transmute(ptr);
                f(a[0], a[1], a[2], a[3], a[4], a[5])
            }
            7 => {
                let f: extern "C" fn(i64, i64, i64, i64, i64, i64, i64) -> i64 =
                    std::mem::transmute(ptr);
                f(a[0], a[1], a[2], a[3], a[4], a[5], a[6])
            }
            8 => {
                let f: extern "C" fn(i64, i64, i64, i64, i64, i64, i64, i64) -> i64 =
                    std::mem::transmute(ptr);
                f(a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7])
            }
            n => {
                return Err(format!(
                    "Ctypes_call: int-only calls support 0..=8 arguments (got {})",
                    n
                ))
            }
        };
        Ok(wrap_int_result(ret, r))
    }
}

unsafe fn call_float_arity(
    ptr: usize,
    ret: &RetKind,
    a: &[f64],
) -> Result<Value, String> {
    let ptr = ptr as *mut c_void;
    unsafe {
        let r: f64 = match a.len() {
            0 => {
                let f: extern "C" fn() -> f64 = std::mem::transmute(ptr);
                f()
            }
            1 => {
                let f: extern "C" fn(f64) -> f64 = std::mem::transmute(ptr);
                f(a[0])
            }
            2 => {
                let f: extern "C" fn(f64, f64) -> f64 = std::mem::transmute(ptr);
                f(a[0], a[1])
            }
            3 => {
                let f: extern "C" fn(f64, f64, f64) -> f64 = std::mem::transmute(ptr);
                f(a[0], a[1], a[2])
            }
            4 => {
                let f: extern "C" fn(f64, f64, f64, f64) -> f64 =
                    std::mem::transmute(ptr);
                f(a[0], a[1], a[2], a[3])
            }
            n => {
                return Err(format!(
                    "Ctypes_call: double-only calls support 0..=4 arguments (got {})",
                    n
                ))
            }
        };
        Ok(wrap_float_result(ret, r))
    }
}

unsafe fn call_ptr_mixed(
    ptr: usize,
    ret: &RetKind,
    cells: &[Cell],
) -> Result<Value, String> {
    let ptr = ptr as *mut c_void;
    // Cells are Int(i64) or Ptr(*const u8); strings arrived as pointers.
    unsafe {
        match cells.len() {
            1 => match &cells[0] {
                Cell::Ptr(p) => {
                    let f: extern "C" fn(*const u8) -> *const u8 =
                        std::mem::transmute(ptr);
                    Ok(wrap_ptr_result(ret, f(*p)))
                }
                _ => Err("Ctypes_call: unreachable".to_string()),
            },
            2 => {
                let a0 = cell_as_usize(&cells[0]);
                let a1 = cell_as_usize(&cells[1]);
                let f: extern "C" fn(usize, usize) -> usize = std::mem::transmute(ptr);
                Ok(wrap_usize_result(ret, f(a0, a1)))
            }
            3 => {
                let a0 = cell_as_usize(&cells[0]);
                let a1 = cell_as_usize(&cells[1]);
                let a2 = cell_as_usize(&cells[2]);
                let f: extern "C" fn(usize, usize, usize) -> usize =
                    std::mem::transmute(ptr);
                Ok(wrap_usize_result(ret, f(a0, a1, a2)))
            }
            4 => {
                let a0 = cell_as_usize(&cells[0]);
                let a1 = cell_as_usize(&cells[1]);
                let a2 = cell_as_usize(&cells[2]);
                let a3 = cell_as_usize(&cells[3]);
                let f: extern "C" fn(usize, usize, usize, usize) -> usize =
                    std::mem::transmute(ptr);
                Ok(wrap_usize_result(ret, f(a0, a1, a2, a3)))
            }
            n => Err(format!(
                "Ctypes_call: pointer-mixed calls support 1..=4 arguments (got {})",
                n
            )),
        }
    }
}

fn cell_as_usize(c: &Cell) -> usize {
    match c {
        Cell::Int(i) => *i as usize,
        Cell::Float(f) => *f as usize,
        Cell::Ptr(p) => *p as usize,
    }
}

fn wrap_usize_result(kind: &RetKind, r: usize) -> Value {
    match kind {
        RetKind::Void => Value::Null,
        RetKind::Bool => Value::Bool(r != 0),
        RetKind::Long | RetKind::Pointer => Value::Long(r as i64),
        RetKind::Double => Value::Double(r as f64),
        RetKind::CString => wrap_ptr_result(kind, r as *const u8),
        RetKind::Int => Value::Int(r as i32),
    }
}

/// Ctypes_load(libHandle: Long, name: String) -> String
///
/// Reads a NUL-terminated C string global. This is the one global shape
/// that needs no caller-supplied type; other shapes go through
/// Ctypes_call with an explicit CType.
pub(crate) fn native_ctypes_load(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Ctypes_load: expected 2 arguments (libHandle, name)".to_string());
    }
    let lib_handle = match &args[0] {
        Value::Long(h) => *h,
        Value::Int(h) => *h as i64,
        _ => return Err("Ctypes_load: expected an Int/Long library handle".to_string()),
    };
    let name = match &args[1] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Ctypes_load: expected a String symbol name".to_string()),
    };
    let registry = LIB_REGISTRY.lock().unwrap();
    let lib = registry
        .get(&lib_handle)
        .ok_or_else(|| "Ctypes_load: invalid library handle".to_string())?;
    // SAFETY: the Library outlives this read (pinned in the registry).
    let s = unsafe {
        let sym: libloading::Symbol<*const i8> = lib
            .get(name.as_bytes())
            .map_err(|_| format!("Ctypes_load: symbol '{}' not found", name))?;
        if sym.is_null() {
            return Ok(Value::Null);
        }
        CStr::from_ptr(*sym).to_string_lossy().into_owned()
    };
    Ok(Value::String(Rc::new(s)))
}
