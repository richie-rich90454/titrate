#![allow(clippy::missing_safety_doc)]
//! titrate_native – C-ABI native runtime bridge for the Titrate LLVM backend.
//!
//! This crate exposes `#[no_mangle] pub extern "C"` functions that the LLVM
//! backend links against. It includes both direct helpers (println, string
//! concat, malloc/free) and a generic native-call dispatch bridge
//! (`titrate_native_call`) that can invoke any of the 350+ VM native
//! functions by name.
//!
//! Value model:
//! - Strings are passed as `(len: i64, ptr: *const u8)` where `ptr` points to
//!   a UTF-8 buffer of exactly `len` bytes (not necessarily NUL-terminated).
//! - `titrate_string_concat` allocates a fresh buffer for the result and
//!   writes the new length through `out_len`. The caller owns the buffer and
//!   must release it with `titrate_free`.
//! - Generic native calls use a serialized buffer format (see
//!   `serialize_value` / `deserialize_value`).

use std::rc::Rc;

use trc::bytecode::value::{NativeFn, Value};
use trc::bytecode::vm::natives::lookup_builtin_native;

pub mod graphics;
pub mod wrappers;
pub mod audio;

// ---------------------------------------------------------------------------
// Allocator header (shared with malloc/free)
// ---------------------------------------------------------------------------

#[repr(C)]
struct AllocHeader {
    cap: usize,
    len: usize,
}

const HEADER_SIZE: usize = std::mem::size_of::<AllocHeader>();

// ---------------------------------------------------------------------------
// C-ABI serialization type tags
// ---------------------------------------------------------------------------

const TAG_VOID: i32 = 0;
const TAG_INT: i32 = 1;
const TAG_LONG: i32 = 2;
const TAG_DOUBLE: i32 = 3;
const TAG_BOOL: i32 = 4;
const TAG_STRING: i32 = 5;
const TAG_NULL: i32 = 6;
const TAG_OBJECT: i32 = 7;
const TAG_ARRAY: i32 = 8;
const TAG_RESULT_OK: i32 = 9;
const TAG_RESULT_ERR: i32 = 10;
const TAG_FLOAT: i32 = 11;
const TAG_CHAR: i32 = 12;


pub mod print;
pub mod array;
#[cfg(test)]
mod tests;
// ---------------------------------------------------------------------------
// Native dispatch bridge
// ---------------------------------------------------------------------------

/// Serialize a `Value` into a byte buffer.
///
/// Returns the number of bytes written. The caller must ensure the buffer
/// is large enough.
fn serialize_value(value: &Value, buf: &mut [u8]) -> usize {
    let mut offset: usize = 0;

    // Write type tag (4 bytes, little-endian).
    let tag: i32 = match value {
        Value::Void => TAG_VOID,
        Value::Null => TAG_NULL,
        Value::Bool(_) => TAG_BOOL,
        Value::Byte(_) | Value::Short(_) | Value::Int(_) => TAG_INT,
        Value::Long(_) => TAG_LONG,
        Value::Float(_) => TAG_FLOAT,
        Value::Double(_) => TAG_DOUBLE,
        Value::Char(_) => TAG_CHAR,
        Value::String(_) => TAG_STRING,
        Value::Array { .. } => TAG_ARRAY,
        Value::ResultOk(_) => TAG_RESULT_OK,
        Value::ResultErr(_) => TAG_RESULT_ERR,
        _ => TAG_OBJECT,
    };
    buf[offset..offset + 4].copy_from_slice(&tag.to_le_bytes());
    offset += 4;

    match value {
        Value::Void | Value::Null => {}
        Value::Bool(b) => {
            buf[offset] = if *b { 1 } else { 0 };
            offset += 1;
        }
        Value::Byte(b) => {
            let v = *b as i32;
            buf[offset..offset + 4].copy_from_slice(&v.to_le_bytes());
            offset += 4;
        }
        Value::Short(s) => {
            let v = *s as i32;
            buf[offset..offset + 4].copy_from_slice(&v.to_le_bytes());
            offset += 4;
        }
        Value::Int(i) => {
            buf[offset..offset + 4].copy_from_slice(&i.to_le_bytes());
            offset += 4;
        }
        Value::Long(l) => {
            buf[offset..offset + 8].copy_from_slice(&l.to_le_bytes());
            offset += 8;
        }
        Value::Float(f) => {
            buf[offset..offset + 4].copy_from_slice(&f.to_le_bytes());
            offset += 4;
        }
        Value::Double(d) => {
            buf[offset..offset + 8].copy_from_slice(&d.to_le_bytes());
            offset += 8;
        }
        Value::Char(c) => {
            let v = *c as u32;
            buf[offset..offset + 4].copy_from_slice(&v.to_le_bytes());
            offset += 4;
        }
        Value::String(s) => {
            let bytes = s.as_bytes();
            let len = bytes.len() as i64;
            buf[offset..offset + 8].copy_from_slice(&len.to_le_bytes());
            offset += 8;
            buf[offset..offset + bytes.len()].copy_from_slice(bytes);
            offset += bytes.len();
        }
        Value::Array { elements } => {
            let count = elements.len() as i64;
            buf[offset..offset + 8].copy_from_slice(&count.to_le_bytes());
            offset += 8;
            for elem in elements {
                let n = serialize_value(elem, &mut buf[offset..]);
                offset += n;
            }
        }
        Value::ResultOk(v) | Value::ResultErr(v) => {
            let n = serialize_value(v, &mut buf[offset..]);
            offset += n;
        }
        _ => {
            // Object: write a null pointer as placeholder.
            let ptr: i64 = 0;
            buf[offset..offset + 8].copy_from_slice(&ptr.to_le_bytes());
            offset += 8;
        }
    }

    offset
}

/// Maximum number of nested containers (arrays / Result values) the C-ABI
/// decoder will walk. Each level adds a stack frame and an allocation chosen
/// by the caller, so an untrusted buffer must not be able to nest freely.
const MAX_DESERIALIZE_DEPTH: usize = 64;

/// Hard cap on the bytes a single serialized value may consume, including
/// every byte consumed by its nested values.
const MAX_DESERIALIZE_BYTES: usize = 16 * 1024 * 1024;

/// Byte pool shared by all levels of one deserialization. A nested value
/// draws from the same pool as its parent rather than getting a fresh
/// allowance for every level.
struct DecodeBudget {
    remaining: usize,
}

impl DecodeBudget {
    fn check(&self, bytes: usize) -> Result<(), String> {
        if bytes > self.remaining {
            return Err(format!(
                "deserialize: total byte budget exceeded ({} needed, {} available)",
                bytes, self.remaining
            ));
        }
        Ok(())
    }

    fn take(&mut self, bytes: usize) -> Result<(), String> {
        self.check(bytes)?;
        self.remaining -= bytes;
        Ok(())
    }
}

/// Deserialize a `Value` from a byte buffer.
///
/// Returns the value and the number of bytes consumed. String data is
/// copied into an owned `Rc<String>`.
fn deserialize_value(buf: &[u8]) -> Result<(Value, usize), String> {
    deserialize_value_budgeted(buf, buf.len().min(MAX_DESERIALIZE_BYTES))
}

/// Deserialize with an explicit total byte budget, shared by every nested level.
fn deserialize_value_budgeted(buf: &[u8], byte_budget: usize) -> Result<(Value, usize), String> {
    let mut budget = DecodeBudget {
        remaining: byte_budget,
    };
    deserialize_value_inner(buf, 0, &mut budget)
}

fn deserialize_value_inner(
    buf: &[u8],
    depth: usize,
    budget: &mut DecodeBudget,
) -> Result<(Value, usize), String> {
    if depth > MAX_DESERIALIZE_DEPTH {
        return Err(format!(
            "deserialize: nesting depth exceeds {} levels",
            MAX_DESERIALIZE_DEPTH
        ));
    }
    if buf.len() < 4 {
        return Err("deserialize: buffer too short for tag".to_string());
    }
    let tag = i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
    let mut offset: usize = 4;
    let mut nested_bytes: usize = 0;

    let value = match tag {
        TAG_VOID => Value::Void,
        TAG_NULL => Value::Null,
        TAG_BOOL => {
            if buf.len() < offset + 1 {
                return Err("deserialize: buffer too short for bool".to_string());
            }
            let v = Value::Bool(buf[offset] != 0);
            offset += 1;
            v
        }
        TAG_INT => {
            if buf.len() < offset + 4 {
                return Err("deserialize: buffer too short for int".to_string());
            }
            let v = Value::Int(i32::from_le_bytes([
                buf[offset],
                buf[offset + 1],
                buf[offset + 2],
                buf[offset + 3],
            ]));
            offset += 4;
            v
        }
        TAG_LONG => {
            if buf.len() < offset + 8 {
                return Err("deserialize: buffer too short for long".to_string());
            }
            let bytes: [u8; 8] = buf[offset..offset + 8].try_into().unwrap();
            let v = Value::Long(i64::from_le_bytes(bytes));
            offset += 8;
            v
        }
        TAG_DOUBLE => {
            if buf.len() < offset + 8 {
                return Err("deserialize: buffer too short for double".to_string());
            }
            let bytes: [u8; 8] = buf[offset..offset + 8].try_into().unwrap();
            let v = Value::Double(f64::from_le_bytes(bytes));
            offset += 8;
            v
        }
        TAG_FLOAT => {
            if buf.len() < offset + 4 {
                return Err("deserialize: buffer too short for float".to_string());
            }
            let bytes: [u8; 4] = buf[offset..offset + 4].try_into().unwrap();
            let v = Value::Float(f32::from_le_bytes(bytes));
            offset += 4;
            v
        }
        TAG_CHAR => {
            if buf.len() < offset + 4 {
                return Err("deserialize: buffer too short for char".to_string());
            }
            let code = u32::from_le_bytes([
                buf[offset],
                buf[offset + 1],
                buf[offset + 2],
                buf[offset + 3],
            ]);
            let v = Value::Char(char::from_u32(code).unwrap_or('\0'));
            offset += 4;
            v
        }
        TAG_STRING => {
            if buf.len() < offset + 8 {
                return Err("deserialize: buffer too short for string header".to_string());
            }
            let bytes: [u8; 8] = buf[offset..offset + 8].try_into().unwrap();
            let raw_len = i64::from_le_bytes(bytes);
            let len = usize::try_from(raw_len)
                .map_err(|_| "deserialize: negative string length".to_string())?;
            offset += 8;
            let available = buf.len().saturating_sub(offset);
            if available < len {
                return Err("deserialize: buffer too short for string data".to_string());
            }
            // Reject before copying so the budget bounds allocation, not just reads.
            budget.check(len)?;
            let s = String::from_utf8_lossy(&buf[offset..offset + len]).into_owned();
            offset += len;
            Value::String(Rc::new(s))
        }
        TAG_ARRAY => {
            if buf.len() < offset + 8 {
                return Err("deserialize: buffer too short for array header".to_string());
            }
            let bytes: [u8; 8] = buf[offset..offset + 8].try_into().unwrap();
            let raw_count = i64::from_le_bytes(bytes);
            let count = usize::try_from(raw_count)
                .map_err(|_| "deserialize: negative array count".to_string())?;
            if count > 1_000_000 {
                return Err("deserialize: array too large".to_string());
            }
            // Every element needs at least a 4-byte tag; fail before walking
            // a count the remaining shared budget cannot possibly cover.
            budget.check(count.saturating_mul(4))?;
            offset += 8;
            let mut elements = Vec::with_capacity(count.min(1024));
            for _ in 0..count {
                let (elem, n) = deserialize_value_inner(&buf[offset..], depth + 1, budget)?;
                nested_bytes += n;
                offset += n;
                elements.push(elem);
            }
            Value::Array { elements }
        }
        TAG_RESULT_OK => {
            let (inner, n) = deserialize_value_inner(&buf[offset..], depth + 1, budget)?;
            nested_bytes += n;
            offset += n;
            Value::ResultOk(Box::new(inner))
        }
        TAG_RESULT_ERR => {
            let (inner, n) = deserialize_value_inner(&buf[offset..], depth + 1, budget)?;
            nested_bytes += n;
            offset += n;
            Value::ResultErr(Box::new(inner))
        }
        TAG_OBJECT => {
            if buf.len() < offset + 8 {
                return Err("deserialize: buffer too short for object".to_string());
            }
            // For now, objects are not supported via serialization; return null.
            offset += 8;
            Value::Null
        }
        _ => return Err(format!("deserialize: unknown type tag {}", tag)),
    };

    // Charge only the bytes read by this level: nested reads were already
    // charged by their own frames from the same shared budget.
    budget.take(offset.saturating_sub(nested_bytes))?;
    Ok((value, offset))
}

/// Compute the serialized size of a `Value` without actually writing.
fn serialized_size(value: &Value) -> usize {
    match value {
        Value::Void | Value::Null => 4,
        Value::Bool(_) => 4 + 1,
        Value::Byte(_) | Value::Short(_) | Value::Int(_) => 4 + 4,
        Value::Long(_) => 4 + 8,
        Value::Float(_) => 4 + 4,
        Value::Double(_) => 4 + 8,
        Value::Char(_) => 4 + 4,
        Value::String(s) => 4 + 8 + s.len(),
        Value::Array { elements } => {
            let mut size = 4 + 8; // tag + count
            for elem in elements {
                size += serialized_size(elem);
            }
            size
        }
        Value::ResultOk(v) | Value::ResultErr(v) => 4 + serialized_size(v),
        _ => 4 + 8, // object: tag + null ptr
    }
}

/// Shared implementation behind the two native-call C entry points.
///
/// Arguments:
/// - `name_ptr`: pointer to NUL-terminated native function name
/// - `name_len`: length of the name string (not including NUL)
/// - `args_ptr`: pointer to serialized arguments buffer
/// - `args_count`: number of arguments
/// - `args_len`: byte length of the serialized arguments buffer
/// - `result_ptr`: pointer to result buffer (output)
/// - `result_cap`: on input, capacity of result buffer; on output, actual bytes written
///
/// Returns 0 on success, 1 on error (error message is written to result buffer as a string).
unsafe fn native_call_impl(
    name_ptr: *const u8,
    name_len: i64,
    args_ptr: *const u8,
    args_count: i64,
    args_len: i64,
    result_ptr: *mut u8,
    result_cap: *mut i64,
) -> i32 {
    if name_ptr.is_null()
        || name_len <= 0
        || args_ptr.is_null()
        || args_len <= 0
        || result_ptr.is_null()
        || result_cap.is_null()
    {
        return 1;
    }

    let name_slice = unsafe { std::slice::from_raw_parts(name_ptr, name_len as usize) };
    let name = match std::str::from_utf8(name_slice) {
        Ok(s) => s,
        Err(e) => {
            let msg = format!("native_call: invalid UTF-8 in name: {}", e);
            write_error_result(result_ptr, result_cap, &msg);
            return 1;
        }
    };

    let func: NativeFn = match lookup_builtin_native(name) {
        Some(f) => f,
        None => {
            let msg = format!("native_call: unknown native function '{}'", name);
            write_error_result(result_ptr, result_cap, &msg);
            return 1;
        }
    };

    // Deserialize arguments.
    let count = args_count as usize;
    let mut args: Vec<Value> = Vec::with_capacity(count);
    let buf = unsafe { std::slice::from_raw_parts(args_ptr, args_len as usize) };
    let mut offset: usize = 0;

    for _ in 0..count {
        if offset >= buf.len() {
            let msg = "native_call: arguments buffer too short".to_string();
            write_error_result(result_ptr, result_cap, &msg);
            return 1;
        }
        match deserialize_value(&buf[offset..]) {
            Ok((val, n)) => {
                args.push(val);
                offset += n;
            }
            Err(e) => {
                write_error_result(result_ptr, result_cap, &e);
                return 1;
            }
        }
    }

    // Call the native function. Never let a native panic unwind across the
    // extern "C" boundary.
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| func(&args)));
    match outcome {
        Ok(Ok(result)) => {
            write_value_to_buffer(result_ptr, result_cap, &result);
            0
        }
        Ok(Err(e)) => {
            write_error_result(result_ptr, result_cap, &e);
            1
        }
        Err(_) => {
            write_error_result(result_ptr, result_cap, "native function panicked");
            1
        }
    }
}

/// Legacy six-argument C ABI, kept for callers built against the original
/// `titrate_native.h`. It carries no serialized-args length, so the buffer is
/// bounded by `MAX_ABI_LEN` while the self-describing values are walked.
/// New callers should use `titrate_native_call_v2`, which passes the real length.
///
/// shortcut: legacy callers may supply a buffer smaller than MAX_ABI_LEN; the
/// deserializer reads only the bytes declared by the `args_count` values, so
/// upgrade by switching those callers to titrate_native_call_v2.
#[no_mangle]
pub unsafe extern "C" fn titrate_native_call(
    name_ptr: *const u8,
    name_len: i64,
    args_ptr: *const u8,
    args_count: i64,
    result_ptr: *mut u8,
    result_cap: *mut i64,
) -> i32 {
    native_call_impl(
        name_ptr,
        name_len,
        args_ptr,
        args_count,
        MAX_ABI_LEN,
        result_ptr,
        result_cap,
    )
}

/// Versioned seven-argument C ABI: same dispatch with an explicit
/// serialized-args buffer length so the buffer is bounds-checked.
#[no_mangle]
pub unsafe extern "C" fn titrate_native_call_v2(
    name_ptr: *const u8,
    name_len: i64,
    args_ptr: *const u8,
    args_count: i64,
    args_len: i64,
    result_ptr: *mut u8,
    result_cap: *mut i64,
) -> i32 {
    native_call_impl(
        name_ptr,
        name_len,
        args_ptr,
        args_count,
        args_len,
        result_ptr,
        result_cap,
    )
}

/// Write a serialized Value to the result buffer.
fn write_value_to_buffer(ptr: *mut u8, cap: *mut i64, value: &Value) {
    let size = serialized_size(value);
    let cap_val = unsafe { *cap };
    if (size as i64) > cap_val {
        unsafe { *cap = size as i64 };
        return;
    }
    let buf = unsafe { std::slice::from_raw_parts_mut(ptr, size) };
    let written = serialize_value(value, buf);
    unsafe { *cap = written as i64 };
}

/// Write an error message as a string to the result buffer.
fn write_error_result(ptr: *mut u8, cap: *mut i64, msg: &str) {
    let err_val = Value::ResultErr(Box::new(Value::String(Rc::new(msg.to_string()))));
    write_value_to_buffer(ptr, cap, &err_val);
}

// ---------------------------------------------------------------------------
// C-ABI tagged-union value model (TitrateValue)
// ---------------------------------------------------------------------------
//
// `TitrateValue` is a `#[repr(C)]` tagged union that mirrors the VM `Value`
// enum. It is the canonical ABI type for passing Titrate values across the
// native bridge: every wrapped native function takes `*const TitrateValue`
// and returns a `TitrateValue`.
//
// Layout: { i32 tag, i32 pad, [16 x i8] payload } = 24 bytes, with the payload
// 8-byte aligned so it can hold i64/f64/pointers/i128 directly.

/// C-ABI string: length + pointer to UTF-8 buffer (not NUL-terminated).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TitrateString {
    pub len: i64,
    pub ptr: *mut u8,
}

/// C-ABI array: length + pointer to a heap-allocated `TitrateValue` buffer.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TitrateArray {
    pub len: i64,
    pub data: *mut TitrateValue,
}

/// C-ABI class/enum instance handle: an opaque index into the handle registry.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TitrateHandle {
    pub id: i64,
    pub type_tag: i32,
}

/// Union of all possible value payloads (16 bytes).
/// NOTE: i128/u128 are intentionally excluded because they cause alignment=16,
/// making the struct 32 bytes instead of the 24 bytes that LLVM expects.
/// Vast/Uvast values are stored via the handle registry instead.
#[repr(C)]
#[derive(Clone, Copy)]
pub union TitratePayload {
    pub bool_val: i8,
    pub byte_val: i8,
    pub short_val: i16,
    pub int_val: i32,
    pub long_val: i64,
    pub float_val: f32,
    pub double_val: f64,
    pub char_val: u32,
    pub string: TitrateString,
    pub array: TitrateArray,
    pub handle: TitrateHandle,
    pub raw: [u8; 16],
}

/// The canonical C-ABI value type: a tagged union.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TitrateValue {
    pub tag: i32,
    pub _pad: i32,
    pub payload: TitratePayload,
}

// Type tags for the TitrateValue union.
pub const TV_VOID: i32 = 0;
pub const TV_NULL: i32 = 1;
pub const TV_BOOL: i32 = 2;
pub const TV_BYTE: i32 = 3;
pub const TV_SHORT: i32 = 4;
pub const TV_INT: i32 = 5;
pub const TV_LONG: i32 = 6;
pub const TV_VAST: i32 = 7;
pub const TV_UVAST: i32 = 8;
pub const TV_FLOAT: i32 = 9;
pub const TV_DOUBLE: i32 = 10;
pub const TV_CHAR: i32 = 11;
pub const TV_STRING: i32 = 12;
pub const TV_ARRAY: i32 = 13;
pub const TV_CLASS_INSTANCE: i32 = 14;
pub const TV_RESULT_OK: i32 = 15;
pub const TV_RESULT_ERR: i32 = 16;
pub const TV_ENUM_INSTANCE: i32 = 17;
pub const TV_TUPLE: i32 = 18;
pub const TV_HALF: i32 = 19;
pub const TV_QUAD: i32 = 20;
pub const TV_HANDLE: i32 = 21; // FileHandle, Socket, Listener, etc.
/// A compiled `{fn_ptr, capture}` closure. The LLVM bridge stores both words
/// in the 16-byte payload; codegen reads them back as a `{i64, ptr}` pair.
pub const TV_CLOSURE: i32 = 22;

// The LLVM bridge occasionally passes an opaque handle id or a stale pointer
// where a length/array is expected. Never trust such payloads: bound the
// accepted length and reject low addresses (null and small handle ids).
const MAX_ABI_LEN: i64 = 1 << 28;

fn valid_abi_slice(ptr: *const u8, len: i64) -> bool {
    (0..=MAX_ABI_LEN).contains(&len) && (len == 0 || ptr as usize >= 0x10000)
}

fn valid_abi_array(ptr: *const TitrateValue, len: i64) -> bool {
    if len <= 0 {
        return false;
    }
    // `len` is an element count; validate the byte extent the elements cover.
    // Element reads use read_unaligned, so an under-aligned buffer from the
    // LLVM bridge must stay acceptable here.
    let nbytes = match (len as usize).checked_mul(std::mem::size_of::<TitrateValue>()) {
        Some(n) => n as i64,
        None => return false,
    };
    valid_abi_slice(ptr as *const u8, nbytes)
}

// Thread-local registry for complex (non-Copy) values that cannot be stored
// directly in the 16-byte payload. Each value is assigned a unique id; the
// TitrateValue payload stores the id as a `TitrateHandle`.
use std::cell::RefCell;
thread_local! {
    static HANDLE_REGISTRY: RefCell<Vec<Value>> = const { RefCell::new(Vec::new()) };
}

fn register_handle(v: Value) -> i64 {
    HANDLE_REGISTRY.with(|reg| {
        let mut reg = reg.borrow_mut();
        let id = reg.len() as i64;
        reg.push(v);
        id
    })
}

fn lookup_handle(id: i64) -> Option<Value> {
    HANDLE_REGISTRY.with(|reg| {
        let reg = reg.borrow();
        reg.get(id as usize).cloned()
    })
}

/// Build a `TitrateValue` with a given tag and zeroed payload.
fn tv_zero(tag: i32) -> TitrateValue {
    TitrateValue {
        tag,
        _pad: 0,
        payload: TitratePayload { raw: [0u8; 16] },
    }
}

/// Build a handle-backed `TitrateValue`. Writing through `raw` keeps every
/// payload byte defined; a union field write leaves the tail uninitialized.
fn tv_handle(tag: i32, id: i64, type_tag: i32) -> TitrateValue {
    let mut raw = [0u8; 16];
    raw[..8].copy_from_slice(&id.to_le_bytes());
    raw[8..12].copy_from_slice(&type_tag.to_le_bytes());
    TitrateValue {
        tag,
        _pad: 0,
        payload: TitratePayload { raw },
    }
}

/// Build a scalar `TitrateValue` whose entire 16-byte payload is defined.
/// A union field write leaves the tail uninitialized; LLVM reinterprets the
/// whole payload for Variant/container-shaped results, so zero the tail.
fn tv_scalar(tag: i32, bits: i64) -> TitrateValue {
    let mut raw = [0u8; 16];
    raw[..8].copy_from_slice(&bits.to_le_bytes());
    TitrateValue {
        tag,
        _pad: 0,
        payload: TitratePayload { raw },
    }
}

/// Build a closure `TitrateValue` carrying both ABI words verbatim. The
/// compiled caller reconstructs `{fn_ptr, capture}` from the payload.
fn tv_closure(fn_ptr: i64, capture: i64) -> TitrateValue {
    let mut raw = [0u8; 16];
    raw[..8].copy_from_slice(&fn_ptr.to_le_bytes());
    raw[8..].copy_from_slice(&capture.to_le_bytes());
    TitrateValue {
        tag: TV_CLOSURE,
        _pad: 0,
        payload: TitratePayload { raw },
    }
}

/// Read the second payload word. It is zero for every scalar the LLVM bridge
/// marshals (tv_scalar zeroes the tail); a non-zero value there means the
/// payload is a two-word `{fn_ptr, capture}` function value.
fn payload_second_word(t: &TitrateValue) -> i64 {
    let raw = unsafe { t.payload.raw };
    i64::from_le_bytes(raw[8..16].try_into().expect("8-byte payload word"))
}

/// Convert a VM `Value` into the C-ABI `TitrateValue` representation.
///
/// Primitive values are stored inline in the payload. Strings and arrays are
/// heap-allocated (the caller owns the buffer). Complex values (class
/// instances, file handles, etc.) are stored in the thread-local handle
/// registry and referenced by id.
pub fn value_to_titrate(v: &Value) -> TitrateValue {
    match v {
        Value::Void => tv_zero(TV_VOID),
        Value::Null => tv_zero(TV_NULL),
        Value::Moved => tv_zero(TV_VOID), // Moved is internal; treat as Void
        Value::Bool(b) => tv_scalar(TV_BOOL, if *b { 1 } else { 0 }),
        Value::Byte(b) => tv_scalar(TV_BYTE, *b as i64),
        Value::Short(s) => tv_scalar(TV_SHORT, *s as i64),
        Value::Int(i) => tv_scalar(TV_INT, *i as i64),
        Value::Long(l) => tv_scalar(TV_LONG, *l),
        Value::Vast(v) => {
            let id = register_handle(Value::Vast(*v));
            tv_handle(TV_VAST, id, TV_VAST)
        }
        Value::Uvast(v) => {
            let id = register_handle(Value::Uvast(*v));
            tv_handle(TV_UVAST, id, TV_UVAST)
        }
        Value::Float(f) => tv_scalar(TV_FLOAT, f.to_bits() as i64),
        Value::Double(d) => tv_scalar(TV_DOUBLE, d.to_bits() as i64),
        Value::Half(h) => tv_scalar(TV_HALF, h.to_bits() as i64),
        Value::Quad(q) => tv_scalar(TV_QUAD, q.to_bits() as i64),
        Value::Char(c) => tv_scalar(TV_CHAR, *c as u32 as i64),
        Value::String(s) => {
            // Allocate a heap buffer for the string bytes.
            let bytes = s.as_bytes();
            let len = bytes.len();
            let mut buf: Vec<u8> = Vec::with_capacity(HEADER_SIZE + len);
            buf.resize(HEADER_SIZE, 0);
            buf.extend_from_slice(bytes);
            let header = AllocHeader {
                cap: buf.capacity(),
                len,
            };
            unsafe {
                std::ptr::write_unaligned(buf.as_mut_ptr() as *mut AllocHeader, header);
            }
            let base = buf.as_mut_ptr();
            std::mem::forget(buf);
            let data_ptr = unsafe { base.add(HEADER_SIZE) };
            TitrateValue {
                tag: TV_STRING,
                _pad: 0,
                payload: TitratePayload {
                    string: TitrateString {
                        len: len as i64,
                        ptr: data_ptr,
                    },
                },
            }
        }
        Value::Array { elements } => {
            let count = elements.len();
            let layout =
                std::alloc::Layout::array::<TitrateValue>(count.max(1)).expect("array layout");
            // Always hand the compiled caller a non-null data pointer. Empty
            // containers are decoded as `{i64, ptr}` and some call sites read
            // element 0 as a scalar; a null pointer there is an access violation.
            let data = unsafe { std::alloc::alloc_zeroed(layout) as *mut TitrateValue };
            for (i, elem) in elements.iter().enumerate() {
                let tv = value_to_titrate(elem);
                unsafe {
                    std::ptr::write(data.add(i), tv);
                }
            }
            TitrateValue {
                tag: TV_ARRAY,
                _pad: 0,
                payload: TitratePayload {
                    array: TitrateArray {
                        len: count as i64,
                        data,
                    },
                },
            }
        }
        Value::Tuple { elements } => {
            // Tuples are represented the same as arrays.
            let count = elements.len();
            let layout =
                std::alloc::Layout::array::<TitrateValue>(count.max(1)).expect("tuple layout");
            let data = unsafe { std::alloc::alloc_zeroed(layout) as *mut TitrateValue };
            for (i, elem) in elements.iter().enumerate() {
                let tv = value_to_titrate(elem);
                unsafe {
                    std::ptr::write(data.add(i), tv);
                }
            }
            TitrateValue {
                tag: TV_TUPLE,
                _pad: 0,
                payload: TitratePayload {
                    array: TitrateArray {
                        len: count as i64,
                        data,
                    },
                },
            }
        }
        Value::ResultOk(inner) => {
            let id = register_handle(Value::ResultOk(inner.clone()));
            tv_handle(TV_RESULT_OK, id, TV_RESULT_OK)
        }
        Value::ResultErr(inner) => {
            let id = register_handle(Value::ResultErr(inner.clone()));
            tv_handle(TV_RESULT_ERR, id, TV_RESULT_ERR)
        }
        // A closure reconstructed from the native ABI carries its two words
        // in a single Long upvalue. Round-trip it with both words intact so
        // compiled callers can rebuild `{fn_ptr, capture}` and invoke it.
        Value::Closure { func_idx, upvalues }
            if upvalues.len() == 1
                && matches!(&*upvalues[0].borrow(), Value::Long(_)) =>
        {
            let capture = match &*upvalues[0].borrow() {
                Value::Long(c) => *c,
                _ => unreachable!(),
            };
            tv_closure(*func_idx as i64, capture)
        }
        Value::ClassInstance { .. }
        | Value::EnumInstance { .. }
        | Value::EnumVariant { .. }
        | Value::Owned(_)
        | Value::Ref(_)
        | Value::RawPtr(_)
        | Value::Function(_)
        | Value::NativeFn(_)
        | Value::FileHandle(_)
        | Value::Socket(_)
        | Value::Listener(_)
        | Value::Closure { .. } => {
            let id = register_handle(v.clone());
            let tag = if matches!(v, Value::ClassInstance { .. }) {
                TV_CLASS_INSTANCE
            } else if matches!(v, Value::EnumInstance { .. } | Value::EnumVariant { .. }) {
                TV_ENUM_INSTANCE
            } else {
                TV_HANDLE
            };
            tv_handle(tag, id, 0)
        }
        Value::Cell(rc) => value_to_titrate(&rc.borrow()),
    }
}

/// Convert a C-ABI `TitrateValue` back into a VM `Value`.
///
/// For string and array payloads, the buffer ownership is *not* consumed —
/// the caller must call `free_titrate_value` to release heap memory.
pub fn titrate_to_value(t: &TitrateValue) -> Value {
    match t.tag {
        TV_VOID => Value::Void,
        TV_NULL => Value::Null,
        TV_BOOL => {
            let b = unsafe { t.payload.bool_val };
            Value::Bool(b != 0)
        }
        TV_BYTE => {
            let b = unsafe { t.payload.byte_val };
            Value::Byte(b)
        }
        TV_SHORT => {
            let s = unsafe { t.payload.short_val };
            Value::Short(s)
        }
        TV_INT => {
            let i = unsafe { t.payload.int_val };
            Value::Int(i)
        }
        TV_LONG => {
            let l = unsafe { t.payload.long_val };
            let second = payload_second_word(t);
            // The LLVM bridge marshals a `{fn_ptr, capture}` function value as
            // TV_LONG with both words in the payload. Scalars always have a
            // zero tail, so a non-zero second word identifies a closure.
            if second != 0 {
                Value::Closure {
                    func_idx: l as usize,
                    upvalues: vec![Rc::new(RefCell::new(Value::Long(second)))],
                }
            } else {
                Value::Long(l)
            }
        }
        TV_CLOSURE => {
            let fn_ptr = unsafe { t.payload.long_val };
            Value::Closure {
                func_idx: fn_ptr as usize,
                upvalues: vec![Rc::new(RefCell::new(Value::Long(payload_second_word(t))))],
            }
        }
        TV_VAST | TV_UVAST => {
            let h = unsafe { t.payload.handle };
            lookup_handle(h.id).unwrap_or(Value::Null)
        }
        TV_FLOAT => {
            let f = unsafe { t.payload.float_val };
            Value::Float(f)
        }
        TV_DOUBLE => {
            let d = unsafe { t.payload.double_val };
            Value::Double(d)
        }
        TV_HALF => {
            let h = unsafe { t.payload.float_val };
            Value::Half(h)
        }
        TV_QUAD => {
            let q = unsafe { t.payload.double_val };
            Value::Quad(q)
        }
        TV_CHAR => {
            let c = unsafe { t.payload.char_val };
            Value::Char(char::from_u32(c).unwrap_or('\0'))
        }
        TV_STRING => {
            let s = unsafe { t.payload.string };
            // A zero-length string may carry a null pointer; from_raw_parts
            // requires a non-null pointer even for an empty slice.
            if s.len <= 0 || !valid_abi_slice(s.ptr, s.len) {
                Value::String(Rc::new(String::new()))
            } else {
                let bytes = unsafe { std::slice::from_raw_parts(s.ptr, s.len as usize) };
                let owned = String::from_utf8_lossy(bytes).into_owned();
                Value::String(Rc::new(owned))
            }
        }
        TV_ARRAY | TV_TUPLE => {
            let arr = unsafe { t.payload.array };
            if !valid_abi_array(arr.data, arr.len) {
                if t.tag == TV_TUPLE {
                    Value::Tuple {
                        elements: Vec::new(),
                    }
                } else {
                    Value::Array {
                        elements: Vec::new(),
                    }
                }
            } else {
                let mut elements = Vec::with_capacity(arr.len as usize);
                for i in 0..arr.len as usize {
                    // read_unaligned: the LLVM bridge may under-align the
                    // element buffer even when the base pointer looks valid.
                    let elem = unsafe { std::ptr::read_unaligned(arr.data.add(i)) };
                    elements.push(titrate_to_value(&elem));
                }
                if t.tag == TV_TUPLE {
                    Value::Tuple { elements }
                } else {
                    Value::Array { elements }
                }
            }
        }
        TV_RESULT_OK | TV_RESULT_ERR | TV_HANDLE => {
            let h = unsafe { t.payload.handle };
            lookup_handle(h.id).unwrap_or(Value::Null)
        }
        TV_CLASS_INSTANCE | TV_ENUM_INSTANCE => {
            let h = unsafe { t.payload.handle };
            // A handle id that is not in the registry is a compiled object
            // pointer passed through the bridge (e.g. storing a JsonValue in a
            // HashMap). Preserve its bits instead of turning it into null.
            lookup_handle(h.id).unwrap_or(Value::Long(h.id))
        }
        _ => Value::Null,
    }
}

/// Convert a slice of C-ABI `TitrateValue` arguments into a `Vec<Value>`.
pub fn args_to_values(args: &[TitrateValue]) -> Vec<Value> {
    args.iter().map(titrate_to_value).collect()
}

/// Free any heap-allocated data owned by a `TitrateValue` (string buffers,
/// array element buffers). After calling this, the `TitrateValue` should not
/// be used again.
pub fn free_titrate_value(t: &mut TitrateValue) {
    match t.tag {
        TV_STRING => {
            let s = unsafe { t.payload.string };
            if valid_abi_slice(s.ptr, s.len) && s.len > 0 {
                unsafe {
                    print::titrate_free(s.ptr);
                }
            }
            t.payload.raw = [0u8; 16];
        }
        TV_ARRAY | TV_TUPLE => {
            let arr = unsafe { t.payload.array };
            if valid_abi_array(arr.data, arr.len) {
                // Recursively free each element.
                for i in 0..arr.len as usize {
                    let mut elem = unsafe { std::ptr::read_unaligned(arr.data.add(i)) };
                    free_titrate_value(&mut elem);
                }
                let layout = std::alloc::Layout::array::<TitrateValue>(arr.len as usize)
                    .expect("array layout");
                unsafe {
                    std::alloc::dealloc(arr.data as *mut u8, layout);
                }
            }
            t.payload.raw = [0u8; 16];
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Graphics host primitives (std only)
// ---------------------------------------------------------------------------
//
// Scalar C-ABI entry points consumed by `wrappers::we`. Their implementations
// live in `graphics` and are complete: the input queue and pixel buffer are
// real, and the window status is an explicit "backend unavailable" result
// because `std` exposes no OS windowing API on any platform.

/// Create a host window. Valid requests report
/// `graphics::STATUS_BACKEND_UNAVAILABLE`; invalid sizes report
/// `graphics::STATUS_INVALID_ARGUMENT`. `tt::graphics::Window.create()`
/// surfaces the former as `Err("windowing backend unavailable")`.
#[no_mangle]
pub unsafe extern "C" fn titrate_graphics_window_create(width: i64, height: i64) -> i64 {
    graphics::window_create(width, height)
}

/// Whether an OS windowing backend exists. Always 0 with a `std`-only host.
#[no_mangle]
pub unsafe extern "C" fn titrate_graphics_window_backend_available() -> i32 {
    if graphics::window_backend_available() {
        1
    } else {
        0
    }
}

/// Enqueue a synthetic input code.
#[no_mangle]
pub unsafe extern "C" fn titrate_graphics_input_push(code: i64) {
    graphics::input_push(code);
}

/// Pop the oldest input code, or 0 when the queue is empty.
#[no_mangle]
pub unsafe extern "C" fn titrate_graphics_input_poll() -> i64 {
    graphics::input_poll()
}

/// Allocate a zero-filled RGBA buffer; returns a positive handle or a
/// negative status.
#[no_mangle]
pub unsafe extern "C" fn titrate_graphics_buffer_create(width: i64, height: i64) -> i64 {
    graphics::buffer_create(width, height)
}

/// Release a buffer. Returns `graphics::STATUS_OK` or
/// `graphics::STATUS_INVALID_ARGUMENT`.
#[no_mangle]
pub unsafe extern "C" fn titrate_graphics_buffer_destroy(handle: i64) -> i64 {
    graphics::buffer_destroy(handle)
}

/// Set one packed RGBA pixel. Bounds are checked.
#[no_mangle]
pub unsafe extern "C" fn titrate_graphics_buffer_set(handle: i64, x: i64, y: i64, rgba: i64) -> i64 {
    graphics::buffer_set(handle, x, y, rgba)
}

/// Read one packed RGBA pixel, or a negative status for invalid coordinates.
#[no_mangle]
pub unsafe extern "C" fn titrate_graphics_buffer_get(handle: i64, x: i64, y: i64) -> i64 {
    graphics::buffer_get(handle, x, y)
}

/// Fill a buffer with one packed RGBA color.
#[no_mangle]
pub unsafe extern "C" fn titrate_graphics_buffer_clear(handle: i64, rgba: i64) -> i64 {
    graphics::buffer_clear(handle, rgba)
}

/// Buffer width, or a negative status for an unknown handle.
#[no_mangle]
pub unsafe extern "C" fn titrate_graphics_buffer_width(handle: i64) -> i64 {
    graphics::buffer_width(handle)
}

/// Buffer height, or a negative status for an unknown handle.
#[no_mangle]
pub unsafe extern "C" fn titrate_graphics_buffer_height(handle: i64) -> i64 {
    graphics::buffer_height(handle)
}

// ---------------------------------------------------------------------------
// Audio host primitives (C ABI, std only)
// ---------------------------------------------------------------------------
//
// These fill a caller-owned f64 buffer with generated PCM samples. They never
// touch a device; availability is reported explicitly by the output helpers.

fn audio_frame_count(frames: i64) -> Option<usize> {
    if frames < 0 || frames as usize > audio::MAX_FRAMES {
        None
    } else {
        Some(frames as usize)
    }
}

/// Fill `out` with `frames` sine samples. Returns a STATUS_* code.
#[no_mangle]
pub unsafe extern "C" fn titrate_audio_sine(
    freq: f64,
    rate: f64,
    frames: i64,
    amp: f64,
    out: *mut f64,
) -> i32 {
    if audio_frame_count(frames).is_none() || out.is_null() {
        return audio::STATUS_INVALID_ARG;
    }
    let samples = audio::sine(freq, rate, frames, amp);
    unsafe {
        std::ptr::copy_nonoverlapping(samples.as_ptr(), out, samples.len());
    }
    audio::STATUS_OK
}

/// Fill `out` with `frames` square-wave samples. Returns a STATUS_* code.
#[no_mangle]
pub unsafe extern "C" fn titrate_audio_square(
    freq: f64,
    rate: f64,
    frames: i64,
    amp: f64,
    duty: f64,
    out: *mut f64,
) -> i32 {
    if audio_frame_count(frames).is_none() || out.is_null() {
        return audio::STATUS_INVALID_ARG;
    }
    let samples = audio::square(freq, rate, frames, amp, duty);
    unsafe {
        std::ptr::copy_nonoverlapping(samples.as_ptr(), out, samples.len());
    }
    audio::STATUS_OK
}

/// Fill `out` with `frames` sawtooth samples. Returns a STATUS_* code.
#[no_mangle]
pub unsafe extern "C" fn titrate_audio_saw(
    freq: f64,
    rate: f64,
    frames: i64,
    amp: f64,
    out: *mut f64,
) -> i32 {
    if audio_frame_count(frames).is_none() || out.is_null() {
        return audio::STATUS_INVALID_ARG;
    }
    let samples = audio::saw(freq, rate, frames, amp);
    unsafe {
        std::ptr::copy_nonoverlapping(samples.as_ptr(), out, samples.len());
    }
    audio::STATUS_OK
}

/// Fill `out` with `frames` deterministic noise samples. Returns a STATUS_* code.
#[no_mangle]
pub unsafe extern "C" fn titrate_audio_noise(
    seed: i64,
    frames: i64,
    amp: f64,
    out: *mut f64,
) -> i32 {
    if audio_frame_count(frames).is_none() || out.is_null() {
        return audio::STATUS_INVALID_ARG;
    }
    let samples = audio::noise(seed, frames, amp);
    unsafe {
        std::ptr::copy_nonoverlapping(samples.as_ptr(), out, samples.len());
    }
    audio::STATUS_OK
}

/// Mix `a` and `b` into `out`, writing the sample count through `out_len`.
/// Returns a STATUS_* code.
#[no_mangle]
pub unsafe extern "C" fn titrate_audio_mix(
    a: *const f64,
    a_len: i64,
    b: *const f64,
    b_len: i64,
    out: *mut f64,
    out_len: *mut i64,
) -> i32 {
    if out_len.is_null()
        || a_len < 0
        || b_len < 0
        || a_len as usize > audio::MAX_FRAMES
        || b_len as usize > audio::MAX_FRAMES
        || out.is_null()
        || (a_len > 0 && a.is_null())
        || (b_len > 0 && b.is_null())
    {
        return audio::STATUS_INVALID_ARG;
    }
    let a_slice = if a_len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(a, a_len as usize) }
    };
    let b_slice = if b_len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(b, b_len as usize) }
    };
    let samples = audio::mix(a_slice, b_slice);
    unsafe {
        *out_len = samples.len() as i64;
        if !samples.is_empty() {
            std::ptr::copy_nonoverlapping(samples.as_ptr(), out, samples.len());
        }
    }
    audio::STATUS_OK
}

/// 1 when an output device is reachable through `std`, 0 when not (always 0).
#[no_mangle]
pub unsafe extern "C" fn titrate_audio_output_available() -> i32 {
    if audio::output_available() {
        1
    } else {
        0
    }
}

/// STATUS_UNAVAILABLE on every std-only host; never reports success.
#[no_mangle]
pub unsafe extern "C" fn titrate_audio_output_status() -> i32 {
    audio::output_status()
}

/// Write the unavailable reason to `buf` and its length to `len`.
/// When `buf` is too small, only the length is written and STATUS_OK is returned.
#[no_mangle]
pub unsafe extern "C" fn titrate_audio_output_reason(
    len: *mut i64,
    buf: *mut u8,
    cap: i64,
) -> i32 {
    if len.is_null() {
        return audio::STATUS_INVALID_ARG;
    }
    let reason = audio::output_reason().as_bytes();
    unsafe {
        *len = reason.len() as i64;
        if !buf.is_null() && cap >= reason.len() as i64 {
            std::ptr::copy_nonoverlapping(reason.as_ptr(), buf, reason.len());
        }
    }
    audio::STATUS_OK
}

#[cfg(test)]
mod abi_tests {
    use super::*;

    #[test]
    fn scalar_payload_tail_is_defined() {
        // LLVM reinterprets the whole 16-byte payload for Variant-shaped
        // results; uninitialized tail bytes make the pointer slot bogus.
        for tv in [
            tv_scalar(TV_INT, -12),
            tv_scalar(TV_DOUBLE, 3.5f64.to_bits() as i64),
            tv_scalar(TV_FLOAT, 1.25f32.to_bits() as i64),
        ] {
            assert!(unsafe { &tv.payload.raw[8..16] }.iter().all(|b| *b == 0));
        }
    }

    #[test]
    fn tv_int_slots_round_trip_exactly() {
        for n in [0i32, 7, -1, -12345, i32::MIN, i32::MAX] {
            let tv = value_to_titrate(&Value::Int(n));
            assert_eq!(tv.tag, TV_INT);
            assert!(matches!(titrate_to_value(&tv), Value::Int(m) if m == n));
            // A Variant/container reinterpretation must see a defined shape:
            // sign-extended integer in the low slot, null pointer in the high.
            let wide = unsafe { tv.payload.long_val };
            assert_eq!(wide, n as i64);
            assert!(unsafe { &tv.payload.raw[8..16] }.iter().all(|b| *b == 0));
        }
    }

    #[test]
    fn tv_long_closure_capture_survives_native_round_trip() {
        // The LLVM backend marshals a `{fn_ptr, capture}` function value as
        // TV_LONG with both words in the payload. A native container stores
        // and returns the decoded Value; dropping the second word made the
        // compiled caller call a closure with a null capture and fault.
        let fn_ptr = 0x1400_0000_1234_i64;
        let capture = 0x2b00_0000_5678_i64;
        let mut incoming = TitrateValue {
            tag: TV_LONG,
            _pad: 0,
            payload: TitratePayload { raw: [0u8; 16] },
        };
        unsafe {
            incoming.payload.raw[..8].copy_from_slice(&fn_ptr.to_le_bytes());
            incoming.payload.raw[8..].copy_from_slice(&capture.to_le_bytes());
        }

        let decoded = titrate_to_value(&incoming);
        let Value::Closure {
            func_idx,
            ref upvalues,
        } = decoded
        else {
            panic!("expected a closure reconstructed from TV_LONG");
        };
        assert_eq!(func_idx as i64, fn_ptr);
        assert_eq!(upvalues.len(), 1);
        assert!(matches!(&*upvalues[0].borrow(), Value::Long(c) if *c == capture));

        let outgoing = value_to_titrate(&decoded);
        assert_eq!(outgoing.tag, TV_CLOSURE);
        assert_eq!(unsafe { outgoing.payload.long_val }, fn_ptr);
        assert_eq!(payload_second_word(&outgoing), capture);

        // A second trip through the decoder must preserve both words again.
        match titrate_to_value(&outgoing) {
            Value::Closure { func_idx, upvalues } => {
                assert_eq!(func_idx as i64, fn_ptr);
                assert!(matches!(&*upvalues[0].borrow(), Value::Long(c) if *c == capture));
            }
            _ => panic!("expected closure from TV_CLOSURE"),
        }
    }

    #[test]
    fn plain_tv_long_stays_a_scalar() {
        let tv = tv_scalar(TV_LONG, -987654321);
        assert!(matches!(titrate_to_value(&tv), Value::Long(n) if n == -987654321));
    }

    #[test]
    fn empty_array_payload_has_non_null_data_pointer() {
        // Compiled callers decode containers as `{i64, ptr}` and may load
        // element 0 as a scalar; a null data pointer faults on that load.
        let mut tv = value_to_titrate(&Value::Array { elements: vec![] });
        assert_eq!(tv.tag, TV_ARRAY);
        assert_eq!(unsafe { tv.payload.array.len }, 0);
        assert!(!unsafe { tv.payload.array.data }.is_null());
        free_titrate_value(&mut tv);
    }

    #[test]
    fn tv_string_round_trips_exact_bytes() {
        let original = "52494646"; // ASCII hex, as File_readBytes returns
        let mut tv = value_to_titrate(&Value::String(Rc::new(original.to_string())));
        assert_eq!(tv.tag, TV_STRING);
        match titrate_to_value(&tv) {
            Value::String(s) => assert_eq!(*s, original),
            _ => panic!("expected string"),
        }
        free_titrate_value(&mut tv);
    }

    #[test]
    fn empty_string_with_null_pointer_is_safe() {
        let tv = TitrateValue {
            tag: TV_STRING,
            _pad: 0,
            payload: TitratePayload {
                string: TitrateString {
                    len: 0,
                    ptr: std::ptr::null_mut(),
                },
            },
        };
        assert!(matches!(titrate_to_value(&tv), Value::String(s) if s.is_empty()));
    }

    #[test]
    fn array_layout_matches_llvm() {
        // LLVM declares TitrateArray as { i64, ptr }.
        assert_eq!(std::mem::size_of::<TitrateArray>(), 16);
        assert_eq!(std::mem::align_of::<TitrateArray>(), 8);
        assert_eq!(std::mem::offset_of!(TitrateArray, len), 0);
        assert_eq!(std::mem::offset_of!(TitrateArray, data), 8);
    }

    #[test]
    fn array_elements_read_from_unaligned_buffer() {
        // The LLVM bridge can hand back an element buffer that is not
        // 8-byte aligned. Element reads use read_unaligned, so validation
        // must not reject the buffer on alignment grounds.
        let mut storage = vec![0u8; 1 + 2 * std::mem::size_of::<TitrateValue>()];
        let data = unsafe { storage.as_mut_ptr().add(1) as *mut TitrateValue };
        assert!(!(data as usize).is_multiple_of(std::mem::align_of::<TitrateValue>()));
        assert!(valid_abi_array(data, 2));
        unsafe {
            std::ptr::write_unaligned(data, tv_scalar(TV_INT, 11));
            std::ptr::write_unaligned(data.add(1), tv_scalar(TV_INT, 22));
        }
        let tv = TitrateValue {
            tag: TV_ARRAY,
            _pad: 0,
            payload: TitratePayload {
                array: TitrateArray { len: 2, data },
            },
        };
        match titrate_to_value(&tv) {
            Value::Array { elements } => {
                assert_eq!(elements.len(), 2);
                assert!(matches!(elements[0], Value::Int(11)));
                assert!(matches!(elements[1], Value::Int(22)));
            }
            _ => panic!("expected array"),
        }
    }

    #[test]
    fn native_call_exports_keep_their_c_abi_signatures() {
        // Typed function-pointer coercions check each exact extern "C"
        // signature at compile time; no linker or C compiler is involved.
        let legacy: unsafe extern "C" fn(
            *const u8,
            i64,
            *const u8,
            i64,
            *mut u8,
            *mut i64,
        ) -> i32 = titrate_native_call;
        let v2: unsafe extern "C" fn(
            *const u8,
            i64,
            *const u8,
            i64,
            i64,
            *mut u8,
            *mut i64,
        ) -> i32 = titrate_native_call_v2;
        assert!((legacy as usize) != 0 && (v2 as usize) != 0);

        // #[no_mangle] makes each Rust name the exported symbol name.
        let source = include_str!("lib.rs").replace("\r\n", "\n");
        for name in ["titrate_native_call", "titrate_native_call_v2"] {
            let decl = format!("#[no_mangle]\npub unsafe extern \"C\" fn {name}(");
            assert!(
                source.contains(&decl),
                "missing #[no_mangle] export for {name}"
            );
        }
    }

    #[test]
    fn native_call_header_declarations_match_exports() {
        let header = include_str!("../titrate_native.h")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        assert!(header.contains(
            "int32_t titrate_native_call(const uint8_t *name_ptr, int64_t name_len, \
             const uint8_t *args_ptr, int64_t args_count, uint8_t *result_ptr, int64_t *result_cap);"
        ));
        assert!(header.contains(
            "int32_t titrate_native_call_v2(const uint8_t *name_ptr, int64_t name_len, \
             const uint8_t *args_ptr, int64_t args_count, int64_t args_len, \
             uint8_t *result_ptr, int64_t *result_cap);"
        ));
        assert!(
            !header.contains(
                "int32_t titrate_native_call(const uint8_t *name_ptr, int64_t name_len, \
                 const uint8_t *args_ptr, int64_t args_count, int64_t args_len,"
            ),
            "legacy declaration must not grow args_len"
        );
        // native_registry.rs collects only `TitrateValue titrate_<name>(...)
        // arg_count);` lines. Both bridges use int32_t, so neither enters the
        // 1:1:1 wrapper/registrar/header correspondence set.
        assert!(!header.contains("TitrateValue titrate_native_call"));

        let generator = include_str!("../../scripts/gen-header.ps1").replace("\r\n", "\n");
        assert!(generator.contains(
            "int32_t titrate_native_call(const uint8_t *name_ptr, int64_t name_len,"
        ));
        assert!(generator.contains(
            "int32_t titrate_native_call_v2(const uint8_t *name_ptr, int64_t name_len,"
        ));
    }

    #[test]
    fn deserialize_rejects_excessive_nesting_depth() {
        // Each single-element array costs 12 bytes (tag + count), so this
        // buffer would otherwise recurse once per level without a bound.
        let mut buf = Vec::new();
        for _ in 0..=MAX_DESERIALIZE_DEPTH {
            buf.extend_from_slice(&TAG_ARRAY.to_le_bytes());
            buf.extend_from_slice(&1i64.to_le_bytes());
        }
        buf.extend_from_slice(&TAG_VOID.to_le_bytes());

        let err = deserialize_value(&buf).expect_err("over-deep buffer must be rejected");
        assert!(err.contains("nesting depth"), "{err}");

        // One level below the limit must still decode.
        buf.truncate(MAX_DESERIALIZE_DEPTH * 12);
        buf.extend_from_slice(&TAG_VOID.to_le_bytes());
        deserialize_value(&buf).expect("buffer at the depth limit must decode");
    }

    #[test]
    fn deserialize_respects_total_byte_budget() {
        // Two sibling strings whose payloads fit each local slice but whose
        // combined size exceeds the budget shared across nesting levels.
        let mut buf = Vec::new();
        buf.extend_from_slice(&TAG_ARRAY.to_le_bytes());
        buf.extend_from_slice(&2i64.to_le_bytes());
        for _ in 0..2 {
            buf.extend_from_slice(&TAG_STRING.to_le_bytes());
            buf.extend_from_slice(&8i64.to_le_bytes());
            buf.extend_from_slice(b"abcdefgh");
        }
        assert_eq!(buf.len(), 52);

        let err = deserialize_value_budgeted(&buf, 40)
            .expect_err("shared byte budget must be enforced across siblings");
        assert!(err.contains("total byte budget"), "{err}");

        // The same buffer decodes when the budget covers the whole value.
        deserialize_value(&buf).expect("buffer within budget must decode");
    }
}

