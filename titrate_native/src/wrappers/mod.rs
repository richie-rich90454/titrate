#![allow(clippy::missing_safety_doc)]
//! C-ABI wrappers for all VM native functions.
//!
//! Each native function in `lookup.rs` gets a `#[no_mangle] pub extern "C"`
//! wrapper with the signature:
//!   `TitrateValue titrate_<Name>(const TitrateValue* args, size_t arg_count)`
//!
//! The wrapper delegates to `native_wrapper` which converts the C-ABI args,
//! looks up the native function, calls it, and converts the result back.

use std::rc::Rc;

use trc::bytecode::value::Value;
use trc::bytecode::vm::natives::lookup_builtin_native;

use crate::{TitrateValue, titrate_to_value, value_to_titrate};

/// Core dispatch: convert args, look up native, call it, convert result.
/// Uses out pointer pattern to avoid hidden sret ABI issues on Windows x64.
pub unsafe fn native_wrapper(name: &str, args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    // Read args using read_unaligned to handle potential alignment issues
    // between the LLVM-generated TitrateValue structs and the Rust TitrateValue type.
    let values: Vec<Value> = if args.is_null() || arg_count == 0 {
        Vec::new()
    } else {
        let mut vals = Vec::with_capacity(arg_count);
        for i in 0..arg_count {
            let elem_ptr = unsafe { args.add(i) };
            let tv = unsafe { std::ptr::read_unaligned(elem_ptr) };
            vals.push(titrate_to_value(&tv));
        }
        vals
    };

    let func = match lookup_builtin_native(name) {
        Some(f) => f,
        None => {
            return value_to_titrate(&Value::ResultErr(Box::new(Value::String(
                Rc::new(format!("unknown native function '{}'", name)),
            ))));
        }
    };

    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| func(&values))) {
        Ok(Ok(result)) => value_to_titrate(&result),
        Ok(Err(e)) => value_to_titrate(&Value::ResultErr(Box::new(Value::String(Rc::new(e))))),
        Err(_) => value_to_titrate(&Value::ResultErr(Box::new(Value::String(
            Rc::new(format!("native function '{}' panicked", name)),
        )))),
    }
}

/// Helper for out-pointer pattern: wraps native_wrapper and writes result to out.
pub unsafe fn native_wrapper_out(name: &str, args: *const TitrateValue, arg_count: usize, out: *mut TitrateValue) {
    let result = native_wrapper(name, args, arg_count);
    unsafe { std::ptr::write_unaligned(out, result); }
}

pub mod wa;
pub mod wb;
pub mod wc;
pub mod wd;
