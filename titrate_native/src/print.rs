// Direct C-ABI print/alloc helpers

use std::io::{self, Write};

use super::{AllocHeader, HEADER_SIZE};

// ---------------------------------------------------------------------------
// Direct C-ABI helpers (print, concat, malloc, free)
// ---------------------------------------------------------------------------

/// Write a UTF-8 string to stdout followed by a newline.
#[no_mangle]
pub unsafe extern "C" fn titrate_println(len: i64, ptr: *const u8) {
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

/// Write a UTF-8 string to stdout without a newline.
#[no_mangle]
pub unsafe extern "C" fn titrate_print(len: i64, ptr: *const u8) {
    if len <= 0 || ptr.is_null() {
        let _ = io::stdout().flush();
        return;
    }
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len as usize) };
    let _ = io::stdout().write_all(bytes);
    let _ = io::stdout().flush();
}

/// Print an integer without a newline.
#[no_mangle]
pub extern "C" fn titrate_print_int(v: i64) {
    print!("{}", v);
}

/// Print a double without a newline.
#[no_mangle]
pub extern "C" fn titrate_print_double(v: f64) {
    print!("{}", v);
}

/// Print a bool without a newline.
#[no_mangle]
pub extern "C" fn titrate_print_bool(v: i32) {
    print!("{}", if v != 0 { "true" } else { "false" });
}

/// Print a char without a newline.
#[no_mangle]
pub extern "C" fn titrate_print_char(v: i32) {
    if let Some(c) = char::from_u32(v as u32) {
        print!("{}", c);
    }
}

/// Concatenate two UTF-8 strings into a freshly allocated buffer.
#[no_mangle]
pub unsafe extern "C" fn titrate_string_concat(
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
    let mut buf: Vec<u8> = Vec::with_capacity(HEADER_SIZE + total);
    buf.resize(HEADER_SIZE, 0);
    buf.extend_from_slice(a_slice);
    buf.extend_from_slice(b_slice);

    let header = AllocHeader {
        cap: buf.capacity(),
        len: total,
    };
    unsafe {
        std::ptr::write_unaligned(buf.as_mut_ptr() as *mut AllocHeader, header);
    }

    if !out_len.is_null() {
        unsafe { *out_len = total as i64 };
    }

    let base = buf.as_mut_ptr();
    std::mem::forget(buf);
    unsafe { base.add(HEADER_SIZE) }
}

/// Free a buffer previously returned by `titrate_string_concat` or `titrate_malloc`.
#[no_mangle]
pub unsafe extern "C" fn titrate_free(ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let base = ptr.sub(HEADER_SIZE);
        let header = std::ptr::read_unaligned(base as *const AllocHeader);
        let _ = Vec::from_raw_parts(base, HEADER_SIZE + header.len, header.cap);
    }
}

/// Allocate `size` bytes of heap memory and return a pointer to it.
#[no_mangle]
pub extern "C" fn titrate_malloc(size: i64) -> *mut u8 {
    if size <= 0 {
        return std::ptr::null_mut();
    }
    let size = size as usize;
    let mut buf: Vec<u8> = vec![0; HEADER_SIZE + size];

    let header = AllocHeader {
        cap: buf.capacity(),
        len: size,
    };
    unsafe {
        std::ptr::write_unaligned(buf.as_mut_ptr() as *mut AllocHeader, header);
    }

    let base = buf.as_mut_ptr();
    std::mem::forget(buf);
    unsafe { base.add(HEADER_SIZE) }
}

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

/// Print a boolean followed by a newline.
#[no_mangle]
pub extern "C" fn titrate_println_bool(v: i32) {
    let _ = writeln!(io::stdout(), "{}", if v != 0 { "true" } else { "false" });
    let _ = io::stdout().flush();
}

/// Print a Unicode character followed by a newline.
#[no_mangle]
pub extern "C" fn titrate_println_char(v: i32) {
    if let Some(c) = char::from_u32(v as u32) {
        let _ = writeln!(io::stdout(), "{}", c);
    } else {
        let _ = writeln!(io::stdout(), "?");
    }
    let _ = io::stdout().flush();
}

