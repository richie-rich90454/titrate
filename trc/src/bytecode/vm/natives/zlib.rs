// Titrate Alpha 0.3 – bytecode virtual machine: zlib/gzip native functions
// Precision in every step – richie-rich90454, 2026
//
// Real compression support using the `flate2` crate.
// Binary data is exchanged via Latin-1 encoded strings (each byte → one char),
// preserving all 256 byte values without UTF-8 corruption.

use super::super::super::value::Value;
use super::NativeContext;
use flate2::read::{ZlibDecoder, ZlibEncoder, GzDecoder, GzEncoder};
use flate2::Compression;
use std::rc::Rc;

/// Convert a byte slice to a Latin-1 string (each byte becomes one char).
fn bytes_to_string(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| b as char).collect()
}

/// Convert a Latin-1 string back to a byte vec (each char's lower 8 bits).
fn string_to_bytes(s: &str) -> Vec<u8> {
    s.chars().map(|c| c as u8).collect()
}

pub(crate) fn native_zlib_compress_ctx(ctx: &NativeContext, args: &[Value]) -> Result<Value, String> {
    let data = match args.first() {
        Some(Value::String(s)) => s.as_bytes().to_vec(),
        _ => return Err("Zlib_compress: expected a string argument".to_string()),
    };

    let mut encoder = ZlibEncoder::new(&data[..], Compression::default());
    let compressed = super::read_bounded_ctx(ctx, &mut encoder, "Zlib_compress")?;

    Ok(Value::String(Rc::new(bytes_to_string(&compressed))))
}

pub(crate) fn native_zlib_decompress_ctx(ctx: &NativeContext, args: &[Value]) -> Result<Value, String> {
    let data = match args.first() {
        Some(Value::String(s)) => string_to_bytes(s),
        _ => return Err("Zlib_decompress: expected a string argument".to_string()),
    };

    let decoder = ZlibDecoder::new(&data[..]);
    let decompressed = super::read_bounded_ctx(ctx, decoder, "Zlib_decompress")?;

    let result = String::from_utf8(decompressed)
        .map_err(|e| format!("Zlib_decompress: invalid UTF-8 in decompressed data: {}", e))?;

    Ok(Value::String(Rc::new(result)))
}

pub(crate) fn native_gzip_compress_ctx(ctx: &NativeContext, args: &[Value]) -> Result<Value, String> {
    let data = match args.first() {
        Some(Value::String(s)) => s.as_bytes().to_vec(),
        _ => return Err("Gzip_compress: expected a string argument".to_string()),
    };

    let mut encoder = GzEncoder::new(&data[..], Compression::default());
    let compressed = super::read_bounded_ctx(ctx, &mut encoder, "Gzip_compress")?;

    Ok(Value::String(Rc::new(bytes_to_string(&compressed))))
}

pub(crate) fn native_gzip_decompress_ctx(ctx: &NativeContext, args: &[Value]) -> Result<Value, String> {
    let data = match args.first() {
        Some(Value::String(s)) => string_to_bytes(s),
        _ => return Err("Gzip_decompress: expected a string argument".to_string()),
    };

    let decoder = GzDecoder::new(&data[..]);
    let decompressed = super::read_bounded_ctx(ctx, decoder, "Gzip_decompress")?;

    let result = String::from_utf8(decompressed)
        .map_err(|e| format!("Gzip_decompress: invalid UTF-8 in decompressed data: {}", e))?;

    Ok(Value::String(Rc::new(result)))
}

trusted_natives! {
    native_zlib_compress => native_zlib_compress_ctx,
    native_zlib_decompress => native_zlib_decompress_ctx,
    native_gzip_compress => native_gzip_compress_ctx,
    native_gzip_decompress => native_gzip_decompress_ctx,
}
