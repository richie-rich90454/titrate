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

/// Generic native call with out-pointer pattern. This avoids the hidden sret
/// ABI issue on Windows x64 where returning a 24-byte struct shifts args.
///
/// Signature: void titrate_native_call_out(const u8* name, usize name_len,
///                                         const TitrateValue* args, usize arg_count,
///                                         TitrateValue* out)
#[no_mangle]
pub unsafe extern "C" fn titrate_native_call_out(
    name_ptr: *const u8,
    name_len: usize,
    args: *const TitrateValue,
    arg_count: usize,
    out: *mut TitrateValue,
) {
    let name = if name_ptr.is_null() || name_len == 0 {
        "unknown"
    } else {
        unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(name_ptr, name_len)) }
    };
    let result = native_wrapper(name, args, arg_count);
    unsafe { std::ptr::write_unaligned(out, result); }
}

// Skip "println": titrate_println already exists with a different signature.
#[no_mangle]
pub unsafe extern "C" fn titrate_toString(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("toString", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_parseInt(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("parseInt", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Ok(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Ok", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Err(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Err", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_File_readFile(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("File_readFile", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_File_writeFile(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("File_writeFile", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_File_readLines(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("File_readLines", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_File_open(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("File_open", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_File_readLine(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("File_readLine", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_File_write(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("File_write", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_File_close(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("File_close", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_File_seek(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("File_seek", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_File_tell(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("File_tell", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_File_readBytes(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("File_readBytes", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_File_writeBytes(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("File_writeBytes", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_File_lastModified(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("File_lastModified", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_File_setModified(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("File_setModified", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_File_flush(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("File_flush", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_File_size(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("File_size", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_File_truncate(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("File_truncate", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_File_copy(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("File_copy", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_File_delete(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("File_delete", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_String_split(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("String_split", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Integer_parseOr(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Integer_parseOr", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_String_trim(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("String_trim", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_String_length(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("String_length", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Path_join(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Path_join", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Path_exists(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Path_exists", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Path_isFile(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Path_isFile", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Path_isDir(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Path_isDir", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Path_basename(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Path_basename", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Path_dirname(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Path_dirname", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Path_extension(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Path_extension", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Path_isSymlink(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Path_isSymlink", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Dir_list(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Dir_list", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Dir_create(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Dir_create", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Dir_remove(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Dir_remove", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Dir_removeTree(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Dir_removeTree", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Sys_args(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Sys_args", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Sys_env(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Sys_env", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Sys_setEnv(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Sys_setEnv", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Sys_exit(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Sys_exit", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Sys_workingDir(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Sys_workingDir", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Sys_setWorkingDir(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Sys_setWorkingDir", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Sys_sleep(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Sys_sleep", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Net_connect(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Net_connect", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Net_send(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Net_send", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Net_receive(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Net_receive", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Net_bind(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Net_bind", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Net_accept(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Net_accept", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Net_close(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Net_close", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Net_getLocalPort(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Net_getLocalPort", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Net_getLocalAddress(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Net_getLocalAddress", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Net_getRemoteAddress(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Net_getRemoteAddress", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Net_setTimeout(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Net_setTimeout", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Http_get(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Http_get", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Http_post(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Http_post", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Http_put(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Http_put", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Http_delete(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Http_delete", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Http_patch(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Http_patch", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Http_head(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Http_head", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Http_setTimeout(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Http_setTimeout", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Http_setFollowRedirects(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Http_setFollowRedirects", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Dns_lookup(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Dns_lookup", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Dns_reverseLookup(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Dns_reverseLookup", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Time_now(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Time_now", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Time_sleep(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Time_sleep", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Time_format(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Time_format", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Time_getYear(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Time_getYear", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Time_getMonth(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Time_getMonth", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Time_getDay(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Time_getDay", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Time_getHour(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Time_getHour", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Time_getMinute(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Time_getMinute", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Time_getSecond(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Time_getSecond", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Regex_match(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Regex_match", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Regex_find(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Regex_find", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Regex_replace(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Regex_replace", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_sin(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_sin", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_cos(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_cos", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_tan(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_tan", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_asin(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_asin", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_acos(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_acos", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_atan(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_atan", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_atan2(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_atan2", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_ln(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_ln", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_log10(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_log10", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_log2(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_log2", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_exp(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_exp", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_pow(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_pow", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_sqrt(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_sqrt", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_cbrt(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_cbrt", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_abs(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_abs", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_absInt(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_absInt", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_floor(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_floor", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_ceil(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_ceil", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_round(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_round", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_inf(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_inf", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_nan(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_nan", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_maxDouble(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_maxDouble", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_minDouble(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_minDouble", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_maxInt(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_maxInt", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_minInt(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_minInt", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Random_seed(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Random_seed", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Random_nextLong(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Random_nextLong", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Json_parse(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Json_parse", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Json_stringify(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Json_stringify", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Env_get(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Env_get", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Env_set(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Env_set", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Env_vars(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Env_vars", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Fs_exists(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Fs_exists", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Fs_isFile(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Fs_isFile", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Fs_isDir(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Fs_isDir", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Fs_size(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Fs_size", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Fs_totalSpace(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Fs_totalSpace", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Fs_freeSpace(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Fs_freeSpace", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Process_id(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Process_id", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Process_args(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Process_args", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_name(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_name", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_arch(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_arch", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_family(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_family", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_String_trimStart(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("String_trimStart", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_String_trimEnd(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("String_trimEnd", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_String_startsWith(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("String_startsWith", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_String_endsWith(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("String_endsWith", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_String_padLeft(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("String_padLeft", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_String_padRight(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("String_padRight", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_String_toUpperCase(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("String_toUpperCase", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_String_toLowerCase(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("String_toLowerCase", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_String_replace(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("String_replace", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_String_fromCharCode(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("String_fromCharCode", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_String_charAt(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("String_charAt", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hash_md5(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hash_md5", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hash_sha1(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hash_sha1", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hash_sha256(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hash_sha256", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hash_sha384(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hash_sha384", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hash_sha512(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hash_sha512", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hash_sha3_256(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hash_sha3_256", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hash_sha3_384(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hash_sha3_384", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hash_sha3_512(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hash_sha3_512", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hash_blake2b(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hash_blake2b", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hash_blake2s(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hash_blake2s", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hash_crc32(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hash_crc32", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hash_sha224(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hash_sha224", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hash_sha3_224(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hash_sha3_224", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hash_shake128(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hash_shake128", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hash_shake256(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hash_shake256", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hasher_new(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hasher_new", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hasher_update(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hasher_update", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hasher_digest(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hasher_digest", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hasher_hexDigest(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hasher_hexDigest", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hasher_reset(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hasher_reset", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hasher_close(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hasher_close", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hmac_compareDigest(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hmac_compareDigest", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Base64_encode(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Base64_encode", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Base64_decode(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Base64_decode", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hex_encode(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hex_encode", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Hex_decode(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Hex_decode", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Url_encode(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Url_encode", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Url_decode(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Url_decode", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_nextUp(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_nextUp", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_nextDown(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_nextDown", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_ulp(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_ulp", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_getExponent(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_getExponent", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_scalb(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_scalb", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_random(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_random", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_negInf(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_negInf", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_fma(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_fma", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Regex_groupCount(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Regex_groupCount", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Regex_findGroups(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Regex_findGroups", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Regex_findWithFlags(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Regex_findWithFlags", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Regex_matchWithFlags(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Regex_matchWithFlags", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Dir_walk(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Dir_walk", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Dir_copy(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Dir_copy", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Dir_move(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Dir_move", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Time_dayOfWeek(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Time_dayOfWeek", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Time_dayOfYear(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Time_dayOfYear", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Time_monotonic(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Time_monotonic", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Time_perfCounter(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Time_perfCounter", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Time_epochSeconds(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Time_epochSeconds", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Time_nanos(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Time_nanos", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Time_millis(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Time_millis", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Regex_fullMatch(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Regex_fullMatch", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Regex_subN(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Regex_subN", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Double_parseDouble(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Double_parseDouble", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Double_parse(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Double_parse", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Long_parseLong(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Long_parseLong", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_TypeName_of(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("TypeName_of", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Subprocess_run(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Subprocess_run", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Subprocess_exec(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Subprocess_exec", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Subprocess_popenWrite(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Subprocess_popenWrite", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Tempfile_create(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Tempfile_create", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Thread_spawn(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Thread_spawn", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Thread_spawnRunnable(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Thread_spawnRunnable", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Thread_join(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Thread_join", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Thread_sleep(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Thread_sleep", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Thread_yield(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Thread_yield", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Thread_getId(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Thread_getId", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Thread_currentId(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Thread_currentId", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Thread_detach(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Thread_detach", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Mutex_new(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Mutex_new", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Mutex_lock(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Mutex_lock", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Mutex_unlock(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Mutex_unlock", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Mutex_tryLock(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Mutex_tryLock", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_RecursiveMutex_new(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("RecursiveMutex_new", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_RecursiveMutex_lock(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("RecursiveMutex_lock", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_RecursiveMutex_unlock(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("RecursiveMutex_unlock", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_RecursiveMutex_tryLock(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("RecursiveMutex_tryLock", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_SharedMutex_new(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("SharedMutex_new", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_SharedMutex_sharedLock(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("SharedMutex_sharedLock", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_SharedMutex_sharedUnlock(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("SharedMutex_sharedUnlock", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_SharedMutex_uniqueLock(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("SharedMutex_uniqueLock", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_SharedMutex_uniqueUnlock(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("SharedMutex_uniqueUnlock", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_SharedMutex_trySharedLock(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("SharedMutex_trySharedLock", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_SharedMutex_tryUniqueLock(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("SharedMutex_tryUniqueLock", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_OnceFlag_new(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("OnceFlag_new", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_OnceFlag_callOnce(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("OnceFlag_callOnce", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_CondVar_new(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("CondVar_new", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_CondVar_wait(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("CondVar_wait", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_CondVar_waitFor(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("CondVar_waitFor", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_CondVar_notifyOne(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("CondVar_notifyOne", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_CondVar_notifyAll(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("CondVar_notifyAll", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Semaphore_new(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Semaphore_new", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Semaphore_acquire(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Semaphore_acquire", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Semaphore_release(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Semaphore_release", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Semaphore_tryAcquire(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Semaphore_tryAcquire", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Semaphore_availablePermits(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Semaphore_availablePermits", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicInt_new(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicInt_new", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicInt_get(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicInt_get", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicInt_set(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicInt_set", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicInt_fetchAdd(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicInt_fetchAdd", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicInt_fetchSub(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicInt_fetchSub", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicInt_compareAndSwap(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicInt_compareAndSwap", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicInt_fetchOr(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicInt_fetchOr", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicInt_fetchAnd(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicInt_fetchAnd", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicInt_fetchXor(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicInt_fetchXor", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicInt_exchange(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicInt_exchange", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicBool_new(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicBool_new", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicBool_get(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicBool_get", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicBool_set(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicBool_set", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicBool_compareAndSwap(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicBool_compareAndSwap", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicLong_new(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicLong_new", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicLong_get(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicLong_get", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicLong_set(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicLong_set", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicLong_fetchAdd(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicLong_fetchAdd", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicLong_fetchSub(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicLong_fetchSub", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicLong_compareAndSwap(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicLong_compareAndSwap", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicRef_new(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicRef_new", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicRef_get(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicRef_get", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicRef_set(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicRef_set", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AtomicRef_compareAndSwap(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("AtomicRef_compareAndSwap", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_new(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_new", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_connect(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_connect", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_bind(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_bind", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_listen(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_listen", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_accept(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_accept", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_send(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_send", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_recv(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_recv", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_close(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_close", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_setTimeout(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_setTimeout", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_setNoDelay(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_setNoDelay", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_UdpSocket_new(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("UdpSocket_new", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_UdpSocket_bind(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("UdpSocket_bind", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_UdpSocket_sendTo(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("UdpSocket_sendTo", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_UdpSocket_recvFrom(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("UdpSocket_recvFrom", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_UdpSocket_close(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("UdpSocket_close", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_UdpSocket_setTimeout(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("UdpSocket_setTimeout", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_UdpSocket_lastSenderHost(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("UdpSocket_lastSenderHost", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_UdpSocket_lastSenderPort(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("UdpSocket_lastSenderPort", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_getAddrInfo(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_getAddrInfo", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_inetPton(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_inetPton", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_inetNtop(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_inetNtop", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_createConnection(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_createConnection", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_createServer(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_createServer", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_getLocalAddress(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_getLocalAddress", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_getRemoteAddress(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_getRemoteAddress", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_getLocalPort(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_getLocalPort", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_getRemotePort(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_getRemotePort", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_setReuseAddr(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_setReuseAddr", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_setBroadcast(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_setBroadcast", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_setKeepAlive(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_setKeepAlive", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Socket_setLinger(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Socket_setLinger", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Ssl_contextNew(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Ssl_contextNew", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Ssl_connect(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Ssl_connect", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Ssl_send(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Ssl_send", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Ssl_recv(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Ssl_recv", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Ssl_close(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Ssl_close", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Ssl_peerCertificate(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Ssl_peerCertificate", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Ssl_contextClose(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Ssl_contextClose", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Ssl_getPeerCertHash(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Ssl_getPeerCertHash", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Sqlite_open(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Sqlite_open", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Sqlite_execute(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Sqlite_execute", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Sqlite_query(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Sqlite_query", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Sqlite_close(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Sqlite_close", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Sqlite_lastInsertId(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Sqlite_lastInsertId", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Sqlite_nextRow(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Sqlite_nextRow", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Sqlite_getInt(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Sqlite_getInt", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Sqlite_getString(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Sqlite_getString", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Sqlite_getDouble(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Sqlite_getDouble", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Sqlite_columnCount(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Sqlite_columnCount", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Sqlite_columnName(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Sqlite_columnName", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Sqlite_closeResult(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Sqlite_closeResult", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Sqlite_executePrepared(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Sqlite_executePrepared", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Sqlite_backup(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Sqlite_backup", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Mmap_open(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Mmap_open", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Mmap_close(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Mmap_close", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Mmap_get(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Mmap_get", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Mmap_set(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Mmap_set", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Mmap_size(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Mmap_size", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Mmap_flush(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Mmap_flush", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Signal_register(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Signal_register", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Signal_raise(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Signal_raise", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Zlib_compress(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Zlib_compress", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Zlib_decompress(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Zlib_decompress", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Gzip_compress(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Gzip_compress", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Gzip_decompress(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Gzip_decompress", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ZipFile_open(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ZipFile_open", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ZipFile_entryCount(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ZipFile_entryCount", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ZipFile_entryName(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ZipFile_entryName", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ZipFile_readEntry(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ZipFile_readEntry", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ZipFile_extractAll(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ZipFile_extractAll", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ZipFile_close(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ZipFile_close", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ZipWriter_open(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ZipWriter_open", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ZipWriter_addEntry(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ZipWriter_addEntry", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ZipWriter_close(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ZipWriter_close", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_cpuCount(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_cpuCount", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_userName(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_userName", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_hostName(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_hostName", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_urandom(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_urandom", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_chmod(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_chmod", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_makedirs(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_makedirs", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_symlink(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_symlink", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_readlink(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_readlink", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_kill(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_kill", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_environ(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_environ", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_umask(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_umask", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_scandir(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_scandir", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_environMap(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_environMap", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_getpid(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_getpid", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_getcwd(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_getcwd", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_chdir(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_chdir", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_getenv(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_getenv", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_setenv(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_setenv", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_unsetenv(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_unsetenv", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_system(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_system", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_uname(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_uname", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_getppid(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_getppid", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_strerror(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_strerror", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_removedirs(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_removedirs", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_renames(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_renames", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_replace(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_replace", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_link(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_link", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_utime(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_utime", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_lstat(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_lstat", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_access(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_access", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_release(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_release", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Os_version(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Os_version", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Titrate_version(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Titrate_version", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Gc_collect(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Gc_collect", args, arg_count) }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that calling a wrapper with null args does not crash.
    #[test]
    fn wrapper_math_sin_null_args() {
        let result = unsafe { titrate_Math_sin(std::ptr::null(), 0) };
        // Math_sin with no args should return an error, not crash.
        assert!(result.tag == crate::TV_RESULT_ERR || result.tag == crate::TV_DOUBLE);
    }

    #[test]
    fn wrapper_math_sin_double() {
        let arg = crate::value_to_titrate(&Value::Double(0.0));
        let result = unsafe { titrate_Math_sin(&arg, 1) };
        assert_eq!(result.tag, crate::TV_DOUBLE);
        let back = crate::titrate_to_value(&result);
        match back {
            Value::Double(d) => assert!((d - 0.0).abs() < 1e-12),
            _ => panic!("expected double"),
        }
    }

    #[test]
    fn wrapper_string_length() {
        let arg = crate::value_to_titrate(&Value::String(Rc::new("hello".to_string())));
        let result = unsafe { titrate_String_length(&arg, 1) };
        let back = crate::titrate_to_value(&result);
        match back {
            Value::Int(5) | Value::Long(5) => {}
            other => panic!("expected 5, got {:?}", other),
        }
        // Free the string buffer we allocated.
        let mut arg_mut = arg;
        crate::free_titrate_value(&mut arg_mut);
    }
}

// ---------------------------------------------------------------------------
// ArrayList native wrappers (for LLVM backend)
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn titrate_ArrayList_size(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ArrayList_size", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ArrayList_get(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ArrayList_get", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ArrayList_add(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ArrayList_add", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ArrayList_new(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ArrayList_new", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ArrayList_set(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ArrayList_set", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ArrayList_remove(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ArrayList_remove", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ArrayList_removeAt(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ArrayList_removeAt", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ArrayList_contains(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ArrayList_contains", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ArrayList_indexOf(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ArrayList_indexOf", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ArrayList_isEmpty(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ArrayList_isEmpty", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ArrayList_clear(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ArrayList_clear", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ArrayList_toString(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ArrayList_toString", args, arg_count) }
}

// ---------------------------------------------------------------------------
// HashMap native wrappers (for LLVM backend)
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn titrate_HashMap_new(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("HashMap_new", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_HashMap_size(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("HashMap_size", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_HashMap_get(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("HashMap_get", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_HashMap_put(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("HashMap_put", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_HashMap_containsKey(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("HashMap_containsKey", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_HashMap_containsValue(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("HashMap_containsValue", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_HashMap_remove(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("HashMap_remove", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_HashMap_keys(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("HashMap_keys", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_HashMap_values(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("HashMap_values", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_HashMap_isEmpty(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("HashMap_isEmpty", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_HashMap_clear(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("HashMap_clear", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_HashMap_toString(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("HashMap_toString", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Integer_parseInt(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Integer_parseInt", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_String_indexOf(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("String_indexOf", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_String_substring(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("String_substring", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Boolean_toString(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Boolean_toString", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Integer_toString(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Integer_toString", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Long_toString(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Long_toString", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Double_toString(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Double_toString", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Float_toString(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Float_toString", args, arg_count) }
}
