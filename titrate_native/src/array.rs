// Dedicated C-ABI array functions (bypass TitrateValue bridge)

use super::{AllocHeader, HEADER_SIZE, TitrateArray, TitratePayload, TitrateString, TitrateValue};
use super::{TV_DOUBLE, TV_FLOAT, TV_INT, TV_LONG, TV_STRING};

// ---------------------------------------------------------------------------
// Dedicated C-ABI array functions (bypass TitrateValue bridge)
// ---------------------------------------------------------------------------
//
// These functions operate directly on TitrateArray, avoiding the alignment
// issues that arise when passing TitrateValue structs through the FFI.

/// Return the length of a TitrateArray.
#[no_mangle]
pub unsafe extern "C" fn titrate_array_length(arr: TitrateArray) -> i64 {
    arr.len
}

/// Return a copy of the string at the given index in the array.
/// The result is written to `out` (out-pointer pattern): returning a 16-byte
/// struct by value is classified differently by LLVM and rustc on Windows
/// x64, which corrupts the call. Writes an empty string if the index is out
/// of bounds or the element is not a string.
///
/// The array is passed by pointer (not by value): a 16-byte struct passed by
/// value is classified differently by LLVM and the Rust/C ABI on Windows x64,
/// which corrupts the fields. A pointer has an unambiguous ABI.
#[no_mangle]
pub unsafe extern "C" fn titrate_array_get_string(
    arr: *const TitrateArray,
    index: i64,
    out: *mut TitrateString,
) {
    let result = string_at(arr, index);
    unsafe { std::ptr::write_unaligned(out, result); }
}

/// Shared body for `titrate_array_get_string`: copy the string element into a
/// new heap buffer, or an empty string when unavailable.
unsafe fn string_at(arr: *const TitrateArray, index: i64) -> TitrateString {
    let arr = unsafe { &*arr };
    if arr.data.is_null() || index < 0 || index >= arr.len {
        return TitrateString {
            len: 0,
            ptr: std::ptr::null_mut(),
        };
    }
    let elem = unsafe { &*arr.data.add(index as usize) };
    if elem.tag != TV_STRING {
        return TitrateString {
            len: 0,
            ptr: std::ptr::null_mut(),
        };
    }
    let s = unsafe { elem.payload.string };
    if s.ptr.is_null() || s.len <= 0 {
        return TitrateString {
            len: 0,
            ptr: std::ptr::null_mut(),
        };
    }
    // Copy the string data into a new heap buffer.
    let bytes = unsafe { std::slice::from_raw_parts(s.ptr, s.len as usize) };
    let mut buf: Vec<u8> = Vec::with_capacity(HEADER_SIZE + bytes.len());
    buf.resize(HEADER_SIZE, 0);
    buf.extend_from_slice(bytes);
    let header = AllocHeader {
        cap: buf.capacity(),
        len: bytes.len(),
    };
    unsafe {
        std::ptr::write_unaligned(buf.as_mut_ptr() as *mut AllocHeader, header);
    }
    let base = buf.as_mut_ptr();
    std::mem::forget(buf);
    let data_ptr = unsafe { base.add(HEADER_SIZE) };
    TitrateString {
        len: bytes.len() as i64,
        ptr: data_ptr,
    }
}

/// Return the int value at the given index in the array.
/// Returns 0 if the index is out of bounds or the element is not an int.
#[no_mangle]
pub unsafe extern "C" fn titrate_array_get_int(arr: *const TitrateArray, index: i64) -> i64 {
    let arr = unsafe { &*arr };
    if arr.data.is_null() || index < 0 || index >= arr.len {
        return 0;
    }
    let elem = unsafe { &*arr.data.add(index as usize) };
    match elem.tag {
        TV_INT => unsafe { elem.payload.int_val as i64 },
        TV_LONG => unsafe { elem.payload.long_val },
        _ => 0,
    }
}

/// Return the long value at the given index in the array.
/// Returns 0 if the index is out of bounds or the element is not numeric.
#[no_mangle]
pub unsafe extern "C" fn titrate_array_get_long(arr: *const TitrateArray, index: i64) -> i64 {
    let arr = unsafe { &*arr };
    if arr.data.is_null() || index < 0 || index >= arr.len {
        return 0;
    }
    let elem = unsafe { &*arr.data.add(index as usize) };
    match elem.tag {
        TV_INT => unsafe { elem.payload.int_val as i64 },
        TV_LONG => unsafe { elem.payload.long_val },
        _ => 0,
    }
}

/// Return the double value at the given index in the array.
/// Returns 0.0 if the index is out of bounds or the element is not a double.
#[no_mangle]
pub unsafe extern "C" fn titrate_array_get_double(arr: *const TitrateArray, index: i64) -> f64 {
    let arr = unsafe { &*arr };
    if arr.data.is_null() || index < 0 || index >= arr.len {
        return 0.0;
    }
    let elem = unsafe { &*arr.data.add(index as usize) };
    match elem.tag {
        TV_DOUBLE => unsafe { elem.payload.double_val },
        TV_FLOAT => unsafe { elem.payload.float_val as f64 },
        TV_INT => unsafe { elem.payload.int_val as f64 },
        TV_LONG => unsafe { elem.payload.long_val as f64 },
        _ => 0.0,
    }
}

/// Append a string element to an array. Returns 0 on success, -1 on error.
#[no_mangle]
pub unsafe extern "C" fn titrate_array_push_string(
    arr_ptr: *mut TitrateArray,
    s_len: i64,
    s_ptr: *const u8,
) -> i32 {
    if arr_ptr.is_null() {
        return -1;
    }
    let arr = unsafe { &mut *arr_ptr };
    if arr.data.is_null() {
        // Allocate initial buffer for 4 elements.
        let layout = std::alloc::Layout::array::<TitrateValue>(4).expect("array layout");
        let data = unsafe { std::alloc::alloc_zeroed(layout) as *mut TitrateValue };
        arr.data = data;
        arr.len = 0;
    }
    // Copy the string into a heap buffer.
    let bytes = if s_len > 0 && !s_ptr.is_null() {
        unsafe { std::slice::from_raw_parts(s_ptr, s_len as usize) }
    } else {
        &[]
    };
    let mut buf: Vec<u8> = Vec::with_capacity(HEADER_SIZE + bytes.len());
    buf.resize(HEADER_SIZE, 0);
    buf.extend_from_slice(bytes);
    let header = AllocHeader {
        cap: buf.capacity(),
        len: bytes.len(),
    };
    unsafe {
        std::ptr::write_unaligned(buf.as_mut_ptr() as *mut AllocHeader, header);
    }
    let base = buf.as_mut_ptr();
    std::mem::forget(buf);
    let data_ptr = unsafe { base.add(HEADER_SIZE) };
    let elem = TitrateValue {
        tag: TV_STRING,
        _pad: 0,
        payload: TitratePayload {
            string: TitrateString {
                len: bytes.len() as i64,
                ptr: data_ptr,
            },
        },
    };
    unsafe {
        std::ptr::write(arr.data.add(arr.len as usize), elem);
    }
    arr.len += 1;
    0
}

