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

/// Neutral values for stdlib `.tr` natives that have no bytecode-VM
/// implementation. Codegen recognizes these names (see
/// `native_bridge::STDLIB_BRIDGE_NATIVE_RETURNS`) and emits native calls; the
/// offline stdlib tests only exercise object construction and guard clauses,
/// so a failed-connect handle / empty payload keeps them behaving like the VM.
pub fn stdlib_native_stub(name: &str, values: &[Value]) -> Option<Value> {
    match name {
        "File_readText" => {
            let path = match values.first() {
                Some(Value::String(s)) => s.as_str(),
                _ => "",
            };
            let content = std::fs::read(path)
                .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
                .unwrap_or_default();
            Some(Value::String(Rc::new(content)))
        }
        "Nntp_connect" | "Pop3_connect" | "Telnet_connect" => Some(Value::Long(-1)),
        "Nntp_readLine" | "Pop3_readLine" | "Pop3_md5Hex" | "Telnet_read"
        | "XmlRpc_post" => Some(Value::String(Rc::new(String::new()))),
        "Smtp_connect" | "Smtp_authenticate" | "Smtp_send" => Some(Value::Bool(false)),
        "Nntp_write" | "Pop3_write" | "Telnet_write" | "Nntp_close" | "Pop3_close"
        | "Telnet_close" | "Telnet_setTimeout" | "Smtp_disconnect" | "XmlRpc_serve"
        | "XmlRpc_stop" => Some(Value::Void),
        _ => None,
    }
}

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
            if let Some(value) = stdlib_native_stub(name, &values) {
                return value_to_titrate(&value);
            }
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
pub mod we;
pub mod wf;
pub mod wg;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stdlib_bridge_natives_return_neutral_values() {
        assert_eq!(
            stdlib_native_stub("Nntp_connect", &[]),
            Some(Value::Long(-1))
        );
        assert_eq!(stdlib_native_stub("Smtp_send", &[]), Some(Value::Bool(false)));
        assert_eq!(stdlib_native_stub("XmlRpc_serve", &[]), Some(Value::Void));
        assert_eq!(
            stdlib_native_stub("Nntp_readLine", &[]),
            Some(Value::String(Rc::new(String::new())))
        );
        assert_eq!(stdlib_native_stub("File_readText", &[]), Some(Value::String(Rc::new(String::new()))));
        assert_eq!(stdlib_native_stub("toString", &[]), None);
    }
}
