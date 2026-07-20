// Titrate Alpha 0.4 – bytecode virtual machine: lz4 native function stubs
// Precision in every step – richie-rich90454, 2026
//
// Stub implementations for LZ4 compression natives. The Titrate
// compression::Lz4 module wraps these calls in user code; when the
// native runtime is unavailable, callers should catch the thrown error
// and degrade gracefully.
//
// Registering the stubs is also required to prevent infinite recursion:
// without an entry in the native table, the VM's STATIC_CALL fallback
// resolves `Lz4_compress(...)` back to the module-level `compress`
// function (matched by the `.Lz4.compress` suffix), which would call
// `Lz4_compress` again indefinitely.

use super::super::super::value::Value;

pub(crate) fn native_lz4_compress(_args: &[Value]) -> Result<Value, String> {
    Err("Lz4_compress: native LZ4 runtime not available".to_string())
}

pub(crate) fn native_lz4_decompress(_args: &[Value]) -> Result<Value, String> {
    Err("Lz4_decompress: native LZ4 runtime not available".to_string())
}

pub(crate) fn native_lz4_frame_compress(_args: &[Value]) -> Result<Value, String> {
    Err("Lz4_frameCompress: native LZ4 runtime not available".to_string())
}

pub(crate) fn native_lz4_frame_decompress(_args: &[Value]) -> Result<Value, String> {
    Err("Lz4_frameDecompress: native LZ4 runtime not available".to_string())
}

pub(crate) fn native_lz4_high_speed_compress(_args: &[Value]) -> Result<Value, String> {
    Err("Lz4_highSpeedCompress: native LZ4 runtime not available".to_string())
}

pub(crate) fn native_lz4_high_compression_compress(_args: &[Value]) -> Result<Value, String> {
    Err("Lz4_highCompressionCompress: native LZ4 runtime not available".to_string())
}

pub(crate) fn native_lz4_compress_bound(_args: &[Value]) -> Result<Value, String> {
    Err("Lz4_compressBound: native LZ4 runtime not available".to_string())
}

pub(crate) fn native_lz4_version_number(_args: &[Value]) -> Result<Value, String> {
    Err("Lz4_versionNumber: native LZ4 runtime not available".to_string())
}

pub(crate) fn native_lz4_xxhash32(_args: &[Value]) -> Result<Value, String> {
    Err("Lz4_xxhash32: native LZ4 runtime not available".to_string())
}

pub(crate) fn native_lz4_xxhash64(_args: &[Value]) -> Result<Value, String> {
    Err("Lz4_xxhash64: native LZ4 runtime not available".to_string())
}

pub(crate) fn native_lz4_compress_with_acceleration(_args: &[Value]) -> Result<Value, String> {
    Err("Lz4_compressWithAcceleration: native LZ4 runtime not available".to_string())
}

pub(crate) fn native_lz4_decompress_safe(_args: &[Value]) -> Result<Value, String> {
    Err("Lz4_decompressSafe: native LZ4 runtime not available".to_string())
}

pub(crate) fn native_lz4_frame_block_size(_args: &[Value]) -> Result<Value, String> {
    Err("Lz4_frameBlockSize: native LZ4 runtime not available".to_string())
}

pub(crate) fn native_lz4_frame_block_mode(_args: &[Value]) -> Result<Value, String> {
    Err("Lz4_frameBlockMode: native LZ4 runtime not available".to_string())
}

pub(crate) fn native_lz4_frame_content_checksum_flag(_args: &[Value]) -> Result<Value, String> {
    Err("Lz4_frameContentChecksumFlag: native LZ4 runtime not available".to_string())
}

pub(crate) fn native_lz4_frame_content_size(_args: &[Value]) -> Result<Value, String> {
    Err("Lz4_frameContentSize: native LZ4 runtime not available".to_string())
}

pub(crate) fn native_lz4_frame_dict_id(_args: &[Value]) -> Result<Value, String> {
    Err("Lz4_frameDictId: native LZ4 runtime not available".to_string())
}
