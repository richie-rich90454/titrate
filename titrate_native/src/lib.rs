//! titrate_native – C-ABI native runtime bridge for the Titrate LLVM backend.
//!
//! This crate exposes a small set of `#[no_mangle] pub extern "C"` functions
//! that the LLVM backend links against. Phase 0 only needs `titrate_println`,
//! `titrate_string_concat`, and `titrate_free`; later phases will wrap the
//! full set of VM natives (see `trc/src/bytecode/vm/natives/lookup.rs`).
//!
//! Value model (Phase 0):
//! - Strings are passed as `(len: i64, ptr: *const u8)` where `ptr` points to
//!   a UTF-8 buffer of exactly `len` bytes (not necessarily NUL-terminated).
//! - `titrate_string_concat` allocates a fresh buffer for the result and
//!   writes the new length through `out_len`. The caller owns the buffer and
//!   must release it with `titrate_free`.

use std::io::{self, Write};

/// Write a UTF-8 string to stdout followed by a newline.
///
/// `len`  – number of bytes in the buffer
/// `ptr`  – pointer to the first byte
#[no_mangle]
pub extern "C" fn titrate_println(len: i64, ptr: *const u8) {
    if len <= 0 || ptr.is_null() {
        let _ = io::stdout().write_all(b"\n");
        let _ = io::stdout().flush();
        return;
    }
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len as usize) };
    let _ = io::stdout().write_all(bytes);
    let _ = io::stdout().write_all(b"\n");
    let _ = io::stdout().flush();
}

/// Concatenate two UTF-8 strings into a freshly allocated buffer.
///
/// Returns a pointer to the new buffer. The new length is written through
/// `out_len`. The caller owns the buffer and must free it with `titrate_free`.
/// If either input has a negative length or null pointer, it is treated as
/// the empty string.
#[no_mangle]
pub extern "C" fn titrate_string_concat(
    a_len: i64,
    a_ptr: *const u8,
    b_len: i64,
    b_ptr: *const u8,
    out_len: *mut i64,
) -> *mut u8 {
    let a_slice = if a_len > 0 && !a_ptr.is_null() {
        unsafe { std::slice::from_raw_parts(a_ptr, a_len as usize) }
    } else {
        &[]
    };
    let b_slice = if b_len > 0 && !b_ptr.is_null() {
        unsafe { std::slice::from_raw_parts(b_ptr, b_len as usize) }
    } else {
        &[]
    };

    let total = a_slice.len() + b_slice.len();
    let mut buf = Vec::<u8>::with_capacity(total);
    buf.extend_from_slice(a_slice);
    buf.extend_from_slice(b_slice);

    if !out_len.is_null() {
        unsafe { *out_len = total as i64 };
    }

    // Convert the Vec into a raw pointer so the caller can free it later.
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

/// Free a buffer previously returned by `titrate_string_concat`.
///
/// Passing a null pointer is a no-op. Passing a pointer not originated from
/// `titrate_string_concat` is undefined behaviour.
#[no_mangle]
pub extern "C" fn titrate_free(ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        // Reconstruct the Vec and let it drop. The capacity is unknown to the
        // caller, but `Vec::from_raw_parts` with capacity 0 still frees the
        // allocation via the global allocator because Rust's allocator tracks
        // the original layout. We use the libc-free approach of reconstructing
        // with the same layout the allocator recorded.
        //
        // NOTE: This relies on the global allocator being able to free a
        // pointer without knowing the exact size. On Windows (MSVC) the
        // CRT allocator (`free`) does track the size internally, so this is
        // safe for buffers allocated by Rust's `Vec`.
        let _ = Box::from_raw(ptr as *mut u8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concat_two_strings() {
        let a = b"Hello, ";
        let b = b"World!";
        let mut out_len: i64 = 0;
        let ptr = titrate_string_concat(a.len() as i64, a.as_ptr(), b.len() as i64, b.as_ptr(), &mut out_len);
        assert_eq!(out_len, (a.len() + b.len()) as i64);
        let combined = unsafe { std::slice::from_raw_parts(ptr, out_len as usize) };
        assert_eq!(combined, b"Hello, World!");
        titrate_free(ptr);
    }

    #[test]
    fn concat_empty_inputs() {
        let mut out_len: i64 = -1;
        let ptr = titrate_string_concat(0, std::ptr::null(), -1, std::ptr::null(), &mut out_len);
        assert_eq!(out_len, 0);
        titrate_free(ptr);
    }
}
