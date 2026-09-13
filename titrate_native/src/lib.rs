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

pub mod wrappers;

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

/// Deserialize a `Value` from a byte buffer.
///
/// Returns the value and the number of bytes consumed. String data is
/// copied into an owned `Rc<String>`.
fn deserialize_value(buf: &[u8]) -> Result<(Value, usize), String> {
    if buf.len() < 4 {
        return Err("deserialize: buffer too short for tag".to_string());
    }
    let tag = i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
    let mut offset: usize = 4;

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
            let len = i64::from_le_bytes(bytes) as usize;
            offset += 8;
            if buf.len() < offset + len {
                return Err("deserialize: buffer too short for string data".to_string());
            }
            let s = String::from_utf8_lossy(&buf[offset..offset + len]).into_owned();
            offset += len;
            Value::String(Rc::new(s))
        }
        TAG_ARRAY => {
            if buf.len() < offset + 8 {
                return Err("deserialize: buffer too short for array header".to_string());
            }
            let bytes: [u8; 8] = buf[offset..offset + 8].try_into().unwrap();
            let count = i64::from_le_bytes(bytes) as usize;
            offset += 8;
            let mut elements = Vec::with_capacity(count);
            for _ in 0..count {
                let (elem, n) = deserialize_value(&buf[offset..])?;
                offset += n;
                elements.push(elem);
            }
            Value::Array { elements }
        }
        TAG_RESULT_OK => {
            let (inner, n) = deserialize_value(&buf[offset..])?;
            offset += n;
            Value::ResultOk(Box::new(inner))
        }
        TAG_RESULT_ERR => {
            let (inner, n) = deserialize_value(&buf[offset..])?;
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

/// Generic native function dispatch bridge.
///
/// Arguments:
/// - `name_ptr`: pointer to NUL-terminated native function name
/// - `name_len`: length of the name string (not including NUL)
/// - `args_ptr`: pointer to serialized arguments buffer
/// - `args_count`: number of arguments
/// - `result_ptr`: pointer to result buffer (output)
/// - `result_cap`: on input, capacity of result buffer; on output, actual bytes written
///
/// Returns 0 on success, 1 on error (error message is written to result buffer as a string).
#[no_mangle]
pub unsafe extern "C" fn titrate_native_call(
    name_ptr: *const u8,
    name_len: i64,
    args_ptr: *const u8,
    args_count: i64,
    result_ptr: *mut u8,
    result_cap: *mut i64,
) -> i32 {
    if name_ptr.is_null()
        || name_len <= 0
        || args_ptr.is_null()
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
    let buf = unsafe { std::slice::from_raw_parts(args_ptr, 1024 * 1024) }; // 1MB max
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

    // Call the native function.
    match func(&args) {
        Ok(result) => {
            write_value_to_buffer(result_ptr, result_cap, &result);
            0
        }
        Err(e) => {
            write_error_result(result_ptr, result_cap, &e);
            1
        }
    }
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
        Value::Bool(b) => TitrateValue {
            tag: TV_BOOL,
            _pad: 0,
            payload: TitratePayload {
                bool_val: if *b { 1 } else { 0 },
            },
        },
        Value::Byte(b) => TitrateValue {
            tag: TV_BYTE,
            _pad: 0,
            payload: TitratePayload { byte_val: *b },
        },
        Value::Short(s) => TitrateValue {
            tag: TV_SHORT,
            _pad: 0,
            payload: TitratePayload { short_val: *s },
        },
        Value::Int(i) => TitrateValue {
            tag: TV_INT,
            _pad: 0,
            payload: TitratePayload { int_val: *i },
        },
        Value::Long(l) => TitrateValue {
            tag: TV_LONG,
            _pad: 0,
            payload: TitratePayload { long_val: *l },
        },
        Value::Vast(v) => {
            let id = register_handle(Value::Vast(*v));
            TitrateValue {
                tag: TV_VAST,
                _pad: 0,
                payload: TitratePayload {
                    handle: TitrateHandle {
                        id,
                        type_tag: TV_VAST,
                    },
                },
            }
        }
        Value::Uvast(v) => {
            let id = register_handle(Value::Uvast(*v));
            TitrateValue {
                tag: TV_UVAST,
                _pad: 0,
                payload: TitratePayload {
                    handle: TitrateHandle {
                        id,
                        type_tag: TV_UVAST,
                    },
                },
            }
        }
        Value::Float(f) => TitrateValue {
            tag: TV_FLOAT,
            _pad: 0,
            payload: TitratePayload { float_val: *f },
        },
        Value::Double(d) => TitrateValue {
            tag: TV_DOUBLE,
            _pad: 0,
            payload: TitratePayload { double_val: *d },
        },
        Value::Half(h) => TitrateValue {
            tag: TV_HALF,
            _pad: 0,
            payload: TitratePayload { float_val: *h },
        },
        Value::Quad(q) => TitrateValue {
            tag: TV_QUAD,
            _pad: 0,
            payload: TitratePayload { double_val: *q },
        },
        Value::Char(c) => TitrateValue {
            tag: TV_CHAR,
            _pad: 0,
            payload: TitratePayload {
                char_val: *c as u32,
            },
        },
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
            let data = if count == 0 {
                std::ptr::null_mut()
            } else {
                unsafe { std::alloc::alloc_zeroed(layout) as *mut TitrateValue }
            };
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
            let data = if count == 0 {
                std::ptr::null_mut()
            } else {
                unsafe { std::alloc::alloc_zeroed(layout) as *mut TitrateValue }
            };
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
            TitrateValue {
                tag: TV_RESULT_OK,
                _pad: 0,
                payload: TitratePayload {
                    handle: TitrateHandle {
                        id,
                        type_tag: TV_RESULT_OK,
                    },
                },
            }
        }
        Value::ResultErr(inner) => {
            let id = register_handle(Value::ResultErr(inner.clone()));
            TitrateValue {
                tag: TV_RESULT_ERR,
                _pad: 0,
                payload: TitratePayload {
                    handle: TitrateHandle {
                        id,
                        type_tag: TV_RESULT_ERR,
                    },
                },
            }
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
            TitrateValue {
                tag: if matches!(v, Value::ClassInstance { .. }) {
                    TV_CLASS_INSTANCE
                } else if matches!(v, Value::EnumInstance { .. } | Value::EnumVariant { .. }) {
                    TV_ENUM_INSTANCE
                } else {
                    TV_HANDLE
                },
                _pad: 0,
                payload: TitratePayload {
                    handle: TitrateHandle { id, type_tag: 0 },
                },
            }
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
            Value::Long(l)
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
            if s.ptr.is_null() || s.len <= 0 {
                Value::String(Rc::new(String::new()))
            } else {
                let bytes = unsafe { std::slice::from_raw_parts(s.ptr, s.len as usize) };
                let owned = String::from_utf8_lossy(bytes).into_owned();
                Value::String(Rc::new(owned))
            }
        }
        TV_ARRAY | TV_TUPLE => {
            let arr = unsafe { t.payload.array };
            if arr.data.is_null() || arr.len <= 0 {
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
                    let elem = unsafe { &*arr.data.add(i) };
                    elements.push(titrate_to_value(elem));
                }
                if t.tag == TV_TUPLE {
                    Value::Tuple { elements }
                } else {
                    Value::Array { elements }
                }
            }
        }
        TV_RESULT_OK | TV_RESULT_ERR | TV_CLASS_INSTANCE | TV_ENUM_INSTANCE | TV_HANDLE => {
            let h = unsafe { t.payload.handle };
            lookup_handle(h.id).unwrap_or(Value::Null)
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
            if !s.ptr.is_null() {
                unsafe {
                    print::titrate_free(s.ptr);
                }
            }
            t.payload.raw = [0u8; 16];
        }
        TV_ARRAY | TV_TUPLE => {
            let arr = unsafe { t.payload.array };
            if !arr.data.is_null() && arr.len > 0 {
                // Recursively free each element.
                for i in 0..arr.len as usize {
                    let elem = unsafe { &mut *arr.data.add(i) };
                    free_titrate_value(elem);
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

