use super::native_wrapper;
use crate::TitrateValue;

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
