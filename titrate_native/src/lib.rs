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

/// Header stored immediately before each allocation returned by
/// `titrate_string_concat` so that `titrate_free` can reconstruct the correct
/// `Vec` layout. Storing the capacity lets us free the buffer safely on every
/// platform without relying on the allocator tracking sizes internally.
#[repr(C)]
struct AllocHeader {
    cap: usize,
    len: usize,
}

const HEADER_SIZE: usize = std::mem::size_of::<AllocHeader>();

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
    // Allocate room for the header + payload so we can recover the layout on free.
    let mut buf: Vec<u8> = Vec::with_capacity(HEADER_SIZE + total);
    buf.resize(HEADER_SIZE, 0);
    buf.extend_from_slice(a_slice);
    buf.extend_from_slice(b_slice);

    debug_assert_eq!(buf.len(), HEADER_SIZE + total);
    debug_assert_eq!(buf.capacity(), HEADER_SIZE + total);

    // Write the header at the start of the buffer.
    let header = AllocHeader { cap: buf.capacity(), len: total };
    unsafe {
        std::ptr::write_unaligned(buf.as_mut_ptr() as *mut AllocHeader, header);
    }

    if !out_len.is_null() {
        unsafe { *out_len = total as i64 };
    }

    // Return a pointer past the header so callers see only the payload.
    let base = buf.as_mut_ptr();
    std::mem::forget(buf);
    unsafe { base.add(HEADER_SIZE) }
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
        let base = ptr.sub(HEADER_SIZE);
        let header = std::ptr::read_unaligned(base as *const AllocHeader);
        // Reconstruct the Vec with the exact length and capacity recorded in
        // the header so the global allocator sees the same layout it allocated.
        let _ = Vec::from_raw_parts(base, HEADER_SIZE + header.len, header.cap);
    }
}

// ---------------------------------------------------------------------------
// Primitive printing helpers (Phase 1)
// ---------------------------------------------------------------------------
//
// These complement `titrate_println` (which takes a UTF-8 buffer) with
// direct printers for primitive types. The LLVM backend uses them when
// `io::println` is called with a non-string argument.

/// Print a 64-bit signed integer followed by a newline.
#[no_mangle]
pub extern "C" fn titrate_println_int(v: i64) {
    let _ = writeln!(io::stdout(), "{}", v);
    let _ = io::stdout().flush();
}

/// Print a 64-bit floating-point value followed by a newline.
#[no_mangle]
pub extern "C" fn titrate_println_double(v: f64) {
    let _ = writeln!(io::stdout(), "{}", v);
    let _ = io::stdout().flush();
}

/// Print a boolean (passed as i32: 0 = false, non-zero = true) followed by
/// a newline.
#[no_mangle]
pub extern "C" fn titrate_println_bool(v: i32) {
    let _ = writeln!(io::stdout(), "{}", if v != 0 { "true" } else { "false" });
    let _ = io::stdout().flush();
}

/// Print a Unicode character (passed as its scalar value as i32) followed by
/// a newline.
#[no_mangle]
pub extern "C" fn titrate_println_char(v: i32) {
    if let Some(c) = char::from_u32(v as u32) {
        let _ = writeln!(io::stdout(), "{}", c);
    } else {
        let _ = writeln!(io::stdout(), "?");
    }
    let _ = io::stdout().flush();
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
