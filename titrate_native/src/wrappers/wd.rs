use super::native_wrapper;
use crate::TitrateValue;

#[no_mangle]
pub unsafe extern "C" fn titrate_Double_toString(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Double_toString", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Float_toString(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Float_toString", args, arg_count) }
}

// ---- Additional wrappers ----

#[no_mangle]
pub unsafe extern "C" fn titrate_CryptoExt_chacha20Poly1305Decrypt(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("CryptoExt_chacha20Poly1305Decrypt", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_CryptoExt_chacha20Poly1305Encrypt(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("CryptoExt_chacha20Poly1305Encrypt", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_CryptoExt_curve25519KeyExchange(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("CryptoExt_curve25519KeyExchange", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_CryptoExt_ed25519Sign(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("CryptoExt_ed25519Sign", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_CryptoExt_ed25519Verify(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("CryptoExt_ed25519Verify", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_CryptoExt_hkdfExpand(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("CryptoExt_hkdfExpand", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_CryptoExt_hkdfExtract(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("CryptoExt_hkdfExtract", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Ctypes_call(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Ctypes_call", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Ctypes_dlclose(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Ctypes_dlclose", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Ctypes_dlopen(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Ctypes_dlopen", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Ctypes_dlsym(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Ctypes_dlsym", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Ctypes_load(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Ctypes_load", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Ctypes_symclose(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Ctypes_symclose", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Fcntl_fcntl(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Fcntl_fcntl", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Fcntl_flock(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Fcntl_flock", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Fcntl_ioctl(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Fcntl_ioctl", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Fcntl_lockf(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Fcntl_lockf", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_File_append(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("File_append", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_File_exists(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("File_exists", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_File_readChunk(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("File_readChunk", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_File_rename(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("File_rename", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_File_tryLock(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("File_tryLock", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_File_unlock(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("File_unlock", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Float_fromF32Bits(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Float_fromF32Bits", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Float_toF32Bits(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Float_toF32Bits", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Fs_closeWatch(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Fs_closeWatch", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Fs_pollWatchEvents(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Fs_pollWatchEvents", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_HashMap_hasKey(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("HashMap_hasKey", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_JsonValue_asArray(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("JsonValue_asArray", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_JsonValue_asBool(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("JsonValue_asBool", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_JsonValue_asNumber(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("JsonValue_asNumber", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_JsonValue_asObject(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("JsonValue_asObject", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_JsonValue_asString(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("JsonValue_asString", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_JsonValue_hasKey(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("JsonValue_hasKey", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_JsonValue_isArray(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("JsonValue_isArray", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_JsonValue_isBool(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("JsonValue_isBool", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_JsonValue_isNull(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("JsonValue_isNull", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_JsonValue_isNumber(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("JsonValue_isNumber", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_JsonValue_isObject(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("JsonValue_isObject", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_JsonValue_isString(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("JsonValue_isString", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_JsonValue_keys(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("JsonValue_keys", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_JsonValue_size(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("JsonValue_size", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Lz4_compress(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Lz4_compress", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Lz4_compressBound(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Lz4_compressBound", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Lz4_compressWithAcceleration(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Lz4_compressWithAcceleration", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Lz4_decompress(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Lz4_decompress", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Lz4_decompressSafe(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Lz4_decompressSafe", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Lz4_frameBlockMode(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Lz4_frameBlockMode", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Lz4_frameBlockSize(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Lz4_frameBlockSize", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Lz4_frameCompress(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Lz4_frameCompress", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Lz4_frameContentChecksumFlag(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Lz4_frameContentChecksumFlag", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Lz4_frameContentSize(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Lz4_frameContentSize", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Lz4_frameDecompress(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Lz4_frameDecompress", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Lz4_frameDictId(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Lz4_frameDictId", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Lz4_highCompressionCompress(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Lz4_highCompressionCompress", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Lz4_highSpeedCompress(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Lz4_highSpeedCompress", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Lz4_versionNumber(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Lz4_versionNumber", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Lz4_xxhash32(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Lz4_xxhash32", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Lz4_xxhash64(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Lz4_xxhash64", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Lzma_compress(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Lzma_compress", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Lzma_decompress(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Lzma_decompress", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Math_fabs(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Math_fabs", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_MathAdvanced_cbrt(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("MathAdvanced_cbrt", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_MathAdvanced_exp(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("MathAdvanced_exp", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_MathAdvanced_ln(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("MathAdvanced_ln", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_MathAdvanced_log10(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("MathAdvanced_log10", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_MathAdvanced_log2(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("MathAdvanced_log2", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_MathAdvanced_pow(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("MathAdvanced_pow", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_MathAdvanced_sqrt(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("MathAdvanced_sqrt", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Process_join(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Process_join", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Process_spawn(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Process_spawn", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Process_terminate(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Process_terminate", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Pty_fork(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Pty_fork", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Pty_openpty(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Pty_openpty", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Pty_spawn(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Pty_spawn", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Queue_close(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Queue_close", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Queue_new(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Queue_new", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Queue_recv(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Queue_recv", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Queue_send(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Queue_send", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Regex_findAllNamedCaptures(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Regex_findAllNamedCaptures", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Regex_findNamedCapture(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Regex_findNamedCapture", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Regex_split(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Regex_split", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Resource_getrlimit(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Resource_getrlimit", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Resource_getrusage(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Resource_getrusage", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Resource_setrlimit(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Resource_setrlimit", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_String_contains(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("String_contains", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_String_join(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("String_join", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Syslog_closelog(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Syslog_closelog", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Syslog_openlog(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Syslog_openlog", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Syslog_setlogmask(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Syslog_setlogmask", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Syslog_syslog(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Syslog_syslog", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_System_currentTimeMillis(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("System_currentTimeMillis", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Termios_tcdrain(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Termios_tcdrain", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Termios_tcflush(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Termios_tcflush", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Termios_tcgetattr(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Termios_tcgetattr", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Termios_tcsendbreak(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Termios_tcsendbreak", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Termios_tcsetattr(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Termios_tcsetattr", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_WinReg_CloseKey(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("WinReg_CloseKey", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_WinReg_CreateKey(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("WinReg_CreateKey", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_WinReg_DeleteKey(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("WinReg_DeleteKey", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_WinReg_DeleteValue(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("WinReg_DeleteValue", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_WinReg_EnumKey(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("WinReg_EnumKey", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_WinReg_EnumValue(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("WinReg_EnumValue", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_WinReg_OpenKey(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("WinReg_OpenKey", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_WinReg_QueryValue(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("WinReg_QueryValue", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_WinReg_SetValue(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("WinReg_SetValue", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_WinSound_Beep(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("WinSound_Beep", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_WinSound_MessageBeep(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("WinSound_MessageBeep", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_WinSound_PlaySound(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("WinSound_PlaySound", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_ZipFile_entries(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("ZipFile_entries", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Zstd_compress(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Zstd_compress", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Zstd_compressBound(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Zstd_compressBound", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Zstd_compressWithDict(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Zstd_compressWithDict", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Zstd_compressWithParams(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Zstd_compressWithParams", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Zstd_countFrames(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Zstd_countFrames", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Zstd_decompress(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Zstd_decompress", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Zstd_decompressFrame(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Zstd_decompressFrame", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Zstd_decompressWithDict(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Zstd_decompressWithDict", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Zstd_frameBlockSizeMax(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Zstd_frameBlockSizeMax", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Zstd_frameChecksumFlag(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Zstd_frameChecksumFlag", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Zstd_frameHeaderSize(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Zstd_frameHeaderSize", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Zstd_frameWindowSize(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Zstd_frameWindowSize", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Zstd_getDictId(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Zstd_getDictId", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Zstd_getFrameContentSize(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Zstd_getFrameContentSize", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Zstd_isFrame(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Zstd_isFrame", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Zstd_maxCompressionLevel(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Zstd_maxCompressionLevel", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Zstd_minCompressionLevel(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Zstd_minCompressionLevel", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Zstd_trainDictionary(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Zstd_trainDictionary", args, arg_count) }
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Zstd_versionNumber(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("Zstd_versionNumber", args, arg_count) }
}


#[cfg(test)]
mod tests {
    use super::super::wa::*;
    use std::rc::Rc;
    use trc::bytecode::value::Value;

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
