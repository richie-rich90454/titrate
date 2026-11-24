// Dedicated C-ABI array functions (bypass TitrateValue bridge)

use super::{
    valid_abi_array, valid_abi_slice, AllocHeader, HEADER_SIZE, TitrateArray, TitratePayload,
    TitrateString, TitrateValue,
};
use super::{TV_DOUBLE, TV_FLOAT, TV_INT, TV_LONG, TV_STRING};

thread_local! {
    /// Allocated element capacity per array data pointer. Arrays are created
    /// with room for four elements; push helpers grow the allocation before
    /// writing so `len == capacity` can never cause an out-of-bounds write.
    static ARRAY_CAPACITIES: std::cell::RefCell<std::collections::HashMap<usize, usize>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

unsafe fn ensure_array_capacity(arr: &mut TitrateArray) -> bool {
    let key = arr.data as usize;
    let cap = ARRAY_CAPACITIES.with(|m| *m.borrow().get(&key).unwrap_or(&4));
    let len = arr.len as usize;
    if len < cap {
        return true;
    }
    if cap >= (1 << 28) {
        return false;
    }
    let new_cap = cap.saturating_mul(2).min(1 << 28);
    let elem = std::mem::size_of::<TitrateValue>();
    let old_layout = match std::alloc::Layout::array::<TitrateValue>(cap) {
        Ok(l) => l,
        Err(_) => return false,
    };
    let new_layout = match std::alloc::Layout::array::<TitrateValue>(new_cap) {
        Ok(l) => l,
        Err(_) => return false,
    };
    let new_ptr = unsafe { std::alloc::realloc(arr.data as *mut u8, old_layout, new_layout.size()) };
    if new_ptr.is_null() {
        return false;
    }
    arr.data = new_ptr as *mut TitrateValue;
    ARRAY_CAPACITIES.with(|m| {
        m.borrow_mut().remove(&key);
        m.borrow_mut().insert(arr.data as usize, new_cap);
    });
    let _ = elem;
    true
}

unsafe fn elem_at(arr: *const TitrateArray, index: i64) -> Option<TitrateValue> {
    if arr.is_null() {
        return None;
    }
    let arr = unsafe { &*arr };
    if index < 0 || !valid_abi_array(arr.data, arr.len) || index >= arr.len {
        return None;
    }
    Some(unsafe { std::ptr::read_unaligned(arr.data.add(index as usize)) })
}

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
    let elem = match unsafe { elem_at(arr, index) } {
        Some(e) => e,
        None => {
            return TitrateString {
                len: 0,
                ptr: std::ptr::null_mut(),
            }
        }
    };
    if elem.tag != TV_STRING {
        return TitrateString {
            len: 0,
            ptr: std::ptr::null_mut(),
        };
    }
    let s = unsafe { elem.payload.string };
    if !valid_abi_slice(s.ptr, s.len) || s.len == 0 {
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

/// Return the raw pointer stored at `index` in the array.
///
/// Class instances and enum values are marshalled into the 24-byte
/// `TitrateValue` as a 64-bit pointer in the first 8 bytes of the payload
/// (tag TV_LONG/TV_CLASS_INSTANCE). The string accessor rejects those tags and
/// yields an empty string, so for-in loops over object arrays need this.
#[no_mangle]
pub unsafe extern "C" fn titrate_array_get_ptr(arr: *const TitrateArray, index: i64) -> *mut u8 {
    let elem = match unsafe { elem_at(arr, index) } {
        Some(e) => e,
        None => return std::ptr::null_mut(),
    };
    let bits = unsafe { elem.payload.long_val };
    bits as *mut u8
}

/// Return the int value at the given index in the array.
/// Returns 0 if the index is out of bounds or the element is not an int.
#[no_mangle]
pub unsafe extern "C" fn titrate_array_get_int(arr: *const TitrateArray, index: i64) -> i64 {
    let elem = match unsafe { elem_at(arr, index) } {
        Some(e) => e,
        None => return 0,
    };
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
    let elem = match unsafe { elem_at(arr, index) } {
        Some(e) => e,
        None => return 0,
    };
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
    let elem = match unsafe { elem_at(arr, index) } {
        Some(e) => e,
        None => return 0.0,
    };
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
    let align = std::mem::align_of::<TitrateValue>();
    if arr.data.is_null() || !(arr.data as usize).is_multiple_of(align) {
        // Allocate initial buffer for 4 elements. A non-null but misaligned
        // pointer is a stale handle, not a real element buffer.
        let layout = std::alloc::Layout::array::<TitrateValue>(4).expect("array layout");
        let data = unsafe { std::alloc::alloc_zeroed(layout) as *mut TitrateValue };
        arr.data = data;
        arr.len = 0;
        ARRAY_CAPACITIES.with(|m| m.borrow_mut().insert(data as usize, 4));
    }
    if arr.len < 0 || arr.len > (1 << 28) {
        return -1;
    }
    if !unsafe { ensure_array_capacity(arr) } {
        return -1;
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
        std::ptr::write_unaligned(arr.data.add(arr.len as usize), elem);
    }
    arr.len += 1;
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build an array struct over a raw element buffer with the requested
    /// byte offset, writing each TitrateValue with write_unaligned.
    fn array_with_elements(elements: &[TitrateValue], offset: usize) -> (Vec<u8>, TitrateArray) {
        let mut storage = vec![0u8; offset + elements.len() * std::mem::size_of::<TitrateValue>()];
        let data = unsafe { storage.as_mut_ptr().add(offset) as *mut TitrateValue };
        for (i, elem) in elements.iter().enumerate() {
            unsafe { std::ptr::write_unaligned(data.add(i), *elem) };
        }
        (
            storage,
            TitrateArray {
                len: elements.len() as i64,
                data,
            },
        )
    }

    #[test]
    fn get_int_round_trips_elements() {
        let (_storage, arr) = array_with_elements(
            &[
                super::super::tv_scalar(TV_INT, 7),
                super::super::tv_scalar(TV_INT, -3),
                super::super::tv_scalar(TV_LONG, 5),
            ],
            0,
        );
        assert_eq!(unsafe { titrate_array_length(arr) }, 3);
        assert_eq!(unsafe { titrate_array_get_int(&arr, 0) }, 7);
        assert_eq!(unsafe { titrate_array_get_int(&arr, 1) }, -3);
        assert_eq!(unsafe { titrate_array_get_int(&arr, 2) }, 5);
        assert_eq!(unsafe { titrate_array_get_int(&arr, 3) }, 0);
        assert_eq!(unsafe { titrate_array_get_int(&arr, -1) }, 0);
    }

    #[test]
    fn get_int_reads_unaligned_element_buffer() {
        // Element reads are explicitly unaligned; validation must agree.
        let (_storage, arr) = array_with_elements(&[super::super::tv_scalar(TV_INT, 42)], 1);
        assert!(!(arr.data as usize).is_multiple_of(std::mem::align_of::<TitrateValue>()));
        assert_eq!(unsafe { titrate_array_get_int(&arr, 0) }, 42);
    }

    #[test]
    fn get_string_round_trips_element() {
        let bytes = b"abcd";
        let mut buf: Vec<u8> = vec![0; HEADER_SIZE + bytes.len()];
        unsafe {
            std::ptr::write_unaligned(
                buf.as_mut_ptr() as *mut AllocHeader,
                AllocHeader {
                    cap: buf.capacity(),
                    len: bytes.len(),
                },
            );
        }
        let ptr = unsafe { buf.as_mut_ptr().add(HEADER_SIZE) };
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len()) };
        let elem = TitrateValue {
            tag: TV_STRING,
            _pad: 0,
            payload: TitratePayload {
                string: TitrateString {
                    len: bytes.len() as i64,
                    ptr,
                },
            },
        };
        let (_storage, arr) = array_with_elements(&[elem], 0);
        let mut out = TitrateString {
            len: -1,
            ptr: std::ptr::null_mut(),
        };
        unsafe { titrate_array_get_string(&arr, 0, &mut out) };
        assert_eq!(out.len, bytes.len() as i64);
        assert_eq!(unsafe { std::slice::from_raw_parts(out.ptr, out.len as usize) }, bytes);
        unsafe { titrate_free(out.ptr) };
        std::mem::forget(buf);
    }
}

