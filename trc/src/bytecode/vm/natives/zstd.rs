// Titrate Alpha 0.4 – bytecode virtual machine: zstd native function stubs
// Precision in every step – richie-rich90454, 2026
//
// Stub implementations for Zstd compression natives. The Titrate
// compression::Zstd module wraps these calls in user code; when the
// native runtime is unavailable, callers should catch the thrown error
// and degrade gracefully.
//
// Registering the stubs is also required to prevent infinite recursion:
// without an entry in the native table, the VM's STATIC_CALL fallback
// resolves `Zstd_compressBound(...)` back to the module-level
// `compressBound` function (matched by the `.Zstd.compressBound` suffix),
// which would call `Zstd_compressBound` again indefinitely.

use super::super::super::value::Value;

pub(crate) fn native_zstd_compress(_args: &[Value]) -> Result<Value, String> {
    Err("Zstd_compress: native Zstd runtime not available".to_string())
}

pub(crate) fn native_zstd_decompress(_args: &[Value]) -> Result<Value, String> {
    Err("Zstd_decompress: native Zstd runtime not available".to_string())
}

pub(crate) fn native_zstd_compress_with_dict(_args: &[Value]) -> Result<Value, String> {
    Err("Zstd_compressWithDict: native Zstd runtime not available".to_string())
}

pub(crate) fn native_zstd_decompress_with_dict(_args: &[Value]) -> Result<Value, String> {
    Err("Zstd_decompressWithDict: native Zstd runtime not available".to_string())
}

pub(crate) fn native_zstd_compress_bound(_args: &[Value]) -> Result<Value, String> {
    Err("Zstd_compressBound: native Zstd runtime not available".to_string())
}

pub(crate) fn native_zstd_get_frame_content_size(_args: &[Value]) -> Result<Value, String> {
    Err("Zstd_getFrameContentSize: native Zstd runtime not available".to_string())
}

pub(crate) fn native_zstd_frame_header_size(_args: &[Value]) -> Result<Value, String> {
    Err("Zstd_frameHeaderSize: native Zstd runtime not available".to_string())
}

pub(crate) fn native_zstd_version_number(_args: &[Value]) -> Result<Value, String> {
    Err("Zstd_versionNumber: native Zstd runtime not available".to_string())
}

pub(crate) fn native_zstd_is_frame(_args: &[Value]) -> Result<Value, String> {
    Err("Zstd_isFrame: native Zstd runtime not available".to_string())
}

pub(crate) fn native_zstd_min_compression_level(_args: &[Value]) -> Result<Value, String> {
    Err("Zstd_minCompressionLevel: native Zstd runtime not available".to_string())
}

pub(crate) fn native_zstd_max_compression_level(_args: &[Value]) -> Result<Value, String> {
    Err("Zstd_maxCompressionLevel: native Zstd runtime not available".to_string())
}

pub(crate) fn native_zstd_train_dictionary(_args: &[Value]) -> Result<Value, String> {
    Err("Zstd_trainDictionary: native Zstd runtime not available".to_string())
}

pub(crate) fn native_zstd_get_dict_id(_args: &[Value]) -> Result<Value, String> {
    Err("Zstd_getDictId: native Zstd runtime not available".to_string())
}

pub(crate) fn native_zstd_frame_block_size_max(_args: &[Value]) -> Result<Value, String> {
    Err("Zstd_frameBlockSizeMax: native Zstd runtime not available".to_string())
}

pub(crate) fn native_zstd_frame_window_size(_args: &[Value]) -> Result<Value, String> {
    Err("Zstd_frameWindowSize: native Zstd runtime not available".to_string())
}

pub(crate) fn native_zstd_frame_checksum_flag(_args: &[Value]) -> Result<Value, String> {
    Err("Zstd_frameChecksumFlag: native Zstd runtime not available".to_string())
}

pub(crate) fn native_zstd_compress_with_params(_args: &[Value]) -> Result<Value, String> {
    Err("Zstd_compressWithParams: native Zstd runtime not available".to_string())
}

pub(crate) fn native_zstd_decompress_frame(_args: &[Value]) -> Result<Value, String> {
    Err("Zstd_decompressFrame: native Zstd runtime not available".to_string())
}

pub(crate) fn native_zstd_count_frames(_args: &[Value]) -> Result<Value, String> {
    Err("Zstd_countFrames: native Zstd runtime not available".to_string())
}
