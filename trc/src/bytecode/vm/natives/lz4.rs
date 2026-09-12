// Titrate Alpha 0.4 – bytecode virtual machine: lz4 native functions
// Precision in every step – richie-rich90454, 2026
//
// Real LZ4 compression via the pure-Rust `lz4_flex` crate (block + frame).
// XXH32/XXH64 come from `twox-hash`, the same XXH implementation family that
// `lz4_flex` itself uses for frame checksums.
// Binary data is exchanged via Latin-1 encoded strings (each byte → one char),
// preserving all 256 byte values without UTF-8 corruption (same convention as
// zlib.rs). Decompressed output is returned as UTF-8 text when valid, else as
// Latin-1, so both text and binary round-trips reproduce the input exactly.
//
// Wire notes:
// - Block payloads use `compress_prepend_size` / `decompress_size_prepended`
//   (raw LZ4 blocks carry no length, so the 4-byte LE size prefix is what lets
//   `decompress` know how much to emit).
// - `lz4_flex` exposes no speed/compression-level knob, so
//   high-speed / high-compression / acceleration variants all run the same
//   block codec; the acceleration argument is validated (>= 1) and otherwise
//   has no effect on the output.
// - `versionNumber` reports the `lz4_flex` crate version as
//   major*10000+minor*100+patch (0.11.6 → 1106); keep in sync with Cargo.toml.

use super::super::super::value::Value;
use std::io::{Read, Write};
use std::rc::Rc;

/// lz4_flex 0.11.6 → 0*10000 + 11*100 + 6. Keep in sync with Cargo.toml.
const LZ4_FLEX_VERSION_NUMBER: i32 = 1106;

/// Upper bound for a single block/frame decompression allocation (512 MiB).
/// Guards the VM against corrupt size prefixes requesting huge allocations.
const MAX_DECOMPRESSED_SIZE: usize = 512 * 1024 * 1024;

const LZ4_FRAME_MAGIC: u32 = 0x184D_2204;
const LZ4_LEGACY_MAGIC: u32 = 0x184C_2102;

/// Convert a byte slice to a Latin-1 string (each byte becomes one char).
fn bytes_to_string(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| b as char).collect()
}

/// Convert a Latin-1 string back to bytes, or fall back to UTF-8 bytes when the
/// string holds non-Latin-1 text. Pure-ASCII input is identical either way.
fn input_bytes(s: &str) -> Vec<u8> {
    if s.chars().all(|c| (c as u32) <= 0xFF) {
        s.chars().map(|c| c as u8).collect()
    } else {
        s.as_bytes().to_vec()
    }
}

/// Wrap decoded payload bytes: UTF-8 text when valid, Latin-1 otherwise.
fn output_value(bytes: Vec<u8>) -> Value {
    match String::from_utf8(bytes) {
        Ok(s) => Value::String(Rc::new(s)),
        Err(e) => Value::String(Rc::new(bytes_to_string(e.as_bytes()))),
    }
}

fn arg_string(args: &[Value], what: &str) -> Result<Vec<u8>, String> {
    match args.first() {
        Some(Value::String(s)) => Ok(input_bytes(s)),
        Some(other) => Err(format!("{}: expected a string argument, got {}", what, other.type_name())),
        None => Err(format!("{}: expected a string argument", what)),
    }
}

fn arg_int(args: &[Value], idx: usize, what: &str) -> Result<i64, String> {
    match args.get(idx) {
        Some(v) => v.to_i64().ok_or_else(|| {
            format!("{}: argument {} must be an integer, got {}", what, idx, v.type_name())
        }),
        None => Err(format!("{}: expected an integer argument {}", what, idx)),
    }
}

/// Run block decompression without letting a corrupt size prefix panic the VM.
fn safe_block_decompress(input: &[u8], declared: usize) -> Result<Vec<u8>, String> {
    if declared > MAX_DECOMPRESSED_SIZE {
        return Err(format!(
            "Lz4_decompress: declared output size {} exceeds limit of {} bytes",
            declared, MAX_DECOMPRESSED_SIZE
        ));
    }
    let owned = input.to_vec();
    std::panic::catch_unwind(move || lz4_flex::block::decompress(&owned, declared))
        .map_err(|_| "Lz4_decompress: corrupt block data (decompression panicked)".to_string())?
        .map_err(|e| format!("Lz4_decompress: decompression error: {}", e))
}

fn block_compress(data: &[u8]) -> Value {
    Value::String(Rc::new(bytes_to_string(&lz4_flex::block::compress_prepend_size(data))))
}

pub(crate) fn native_lz4_compress(args: &[Value]) -> Result<Value, String> {
    Ok(block_compress(&arg_string(args, "Lz4_compress")?))
}

pub(crate) fn native_lz4_decompress(args: &[Value]) -> Result<Value, String> {
    let data = arg_string(args, "Lz4_decompress")?;
    let (declared, rest) = lz4_flex::block::uncompressed_size(&data)
        .map_err(|e| format!("Lz4_decompress: invalid block header: {}", e))?;
    let rest = rest.to_vec();
    Ok(output_value(safe_block_decompress(&rest, declared)?))
}

pub(crate) fn native_lz4_frame_compress(args: &[Value]) -> Result<Value, String> {
    let data = arg_string(args, "Lz4_frameCompress")?;
    let mut encoder = lz4_flex::frame::FrameEncoder::new(Vec::new());
    encoder
        .write_all(&data)
        .map_err(|e| format!("Lz4_frameCompress: compression error: {}", e))?;
    let compressed = encoder
        .finish()
        .map_err(|e| format!("Lz4_frameCompress: finish error: {}", e))?;
    Ok(Value::String(Rc::new(bytes_to_string(&compressed))))
}

pub(crate) fn native_lz4_frame_decompress(args: &[Value]) -> Result<Value, String> {
    let data = arg_string(args, "Lz4_frameDecompress")?;
    let mut decoder = lz4_flex::frame::FrameDecoder::new(&data[..]);
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .map_err(|e| format!("Lz4_frameDecompress: decompression error: {}", e))?;
    Ok(output_value(decompressed))
}

pub(crate) fn native_lz4_high_speed_compress(args: &[Value]) -> Result<Value, String> {
    Ok(block_compress(&arg_string(args, "Lz4_highSpeedCompress")?))
}

pub(crate) fn native_lz4_high_compression_compress(args: &[Value]) -> Result<Value, String> {
    Ok(block_compress(&arg_string(args, "Lz4_highCompressionCompress")?))
}

pub(crate) fn native_lz4_compress_bound(args: &[Value]) -> Result<Value, String> {
    let n = arg_int(args, 0, "Lz4_compressBound")?;
    if n < 0 {
        return Err("Lz4_compressBound: input size must be >= 0".to_string());
    }
    // Block payloads carry a 4-byte LE size prefix on top of the raw bound.
    Ok(Value::Int(
        (lz4_flex::block::get_maximum_output_size(n as usize) + 4) as i32,
    ))
}

pub(crate) fn native_lz4_version_number(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::Int(LZ4_FLEX_VERSION_NUMBER))
}

pub(crate) fn native_lz4_xxhash32(args: &[Value]) -> Result<Value, String> {
    let data = arg_string(args, "Lz4_xxhash32")?;
    let seed = arg_int(args, 1, "Lz4_xxhash32")? as u32;
    Ok(Value::Int(twox_hash::XxHash32::oneshot(seed, &data) as i32))
}

pub(crate) fn native_lz4_xxhash64(args: &[Value]) -> Result<Value, String> {
    let data = arg_string(args, "Lz4_xxhash64")?;
    let seed = arg_int(args, 1, "Lz4_xxhash64")? as u64;
    Ok(Value::Long(twox_hash::XxHash64::oneshot(seed, &data) as i64))
}

pub(crate) fn native_lz4_compress_with_acceleration(args: &[Value]) -> Result<Value, String> {
    let data = arg_string(args, "Lz4_compressWithAcceleration")?;
    let accel = arg_int(args, 1, "Lz4_compressWithAcceleration")?;
    if accel < 1 {
        return Err("Lz4_compressWithAcceleration: acceleration must be >= 1".to_string());
    }
    Ok(block_compress(&data))
}

pub(crate) fn native_lz4_decompress_safe(args: &[Value]) -> Result<Value, String> {
    let data = arg_string(args, "Lz4_decompressSafe")?;
    let max = arg_int(args, 1, "Lz4_decompressSafe")?;
    if max < 0 {
        return Err("Lz4_decompressSafe: maxOutputSize must be >= 0".to_string());
    }
    let max = (max as usize).min(MAX_DECOMPRESSED_SIZE);
    let (declared, rest) = lz4_flex::block::uncompressed_size(&data)
        .map_err(|e| format!("Lz4_decompressSafe: invalid block header: {}", e))?;
    if declared > max {
        return Err(format!(
            "Lz4_decompressSafe: declared output size {} exceeds maxOutputSize {}",
            declared, max
        ));
    }
    let rest = rest.to_vec();
    Ok(output_value(safe_block_decompress(&rest, declared)?))
}

/// Parsed LZ4 frame descriptor fields.
struct FrameHeader {
    block_max: usize,
    linked: bool,
    checksum: bool,
    content_size: u64,
    dict_id: u32,
}

fn parse_frame_header(data: &[u8]) -> Result<FrameHeader, String> {
    if data.len() < 7 {
        return Err("Lz4 frame info: not an LZ4 frame (too short)".to_string());
    }
    let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    if magic == LZ4_LEGACY_MAGIC {
        return Err("Lz4 frame info: legacy frame format is not supported".to_string());
    }
    if (0x184D_2A50..=0x184D_2A5F).contains(&magic) {
        return Err("Lz4 frame info: skippable frames carry no frame info".to_string());
    }
    if magic != LZ4_FRAME_MAGIC {
        return Err("Lz4 frame info: not an LZ4 frame (bad magic number)".to_string());
    }
    let flg = data[4];
    let bd = data[5];
    if flg & 0xC0 != 0x40 {
        return Err("Lz4 frame info: unsupported frame version".to_string());
    }
    let mut pos = 6usize;
    let content_size = if flg & 0x08 != 0 {
        if data.len() < pos + 8 {
            return Err("Lz4 frame info: truncated content-size field".to_string());
        }
        let v = u64::from_le_bytes(data[pos..pos + 8].try_into().expect("len checked"));
        pos += 8;
        v
    } else {
        0
    };
    let dict_id = if flg & 0x01 != 0 {
        if data.len() < pos + 4 {
            return Err("Lz4 frame info: truncated dictionary-id field".to_string());
        }
        let v = u32::from_le_bytes(data[pos..pos + 4].try_into().expect("len checked"));
        pos += 4;
        v
    } else {
        0
    };
    if data.len() <= pos {
        return Err("Lz4 frame info: truncated header checksum".to_string());
    }
    let expected_hc = (twox_hash::XxHash32::oneshot(0, &data[4..pos]) >> 8) as u8;
    if data[pos] != expected_hc {
        return Err("Lz4 frame info: corrupt frame header checksum".to_string());
    }
    let block_max = match (bd >> 4) & 0x07 {
        4 => 64 * 1024,
        5 => 256 * 1024,
        6 => 1024 * 1024,
        7 => 4 * 1024 * 1024,
        code => return Err(format!("Lz4 frame info: reserved block-size code {}", code)),
    };
    Ok(FrameHeader {
        block_max,
        linked: flg & 0x20 == 0,
        checksum: flg & 0x04 != 0,
        content_size,
        dict_id,
    })
}

pub(crate) fn native_lz4_frame_block_size(args: &[Value]) -> Result<Value, String> {
    let data = arg_string(args, "Lz4_frameBlockSize")?;
    Ok(Value::Int(parse_frame_header(&data)?.block_max as i32))
}

pub(crate) fn native_lz4_frame_block_mode(args: &[Value]) -> Result<Value, String> {
    let data = arg_string(args, "Lz4_frameBlockMode")?;
    Ok(Value::Int(i32::from(parse_frame_header(&data)?.linked)))
}

pub(crate) fn native_lz4_frame_content_checksum_flag(args: &[Value]) -> Result<Value, String> {
    let data = arg_string(args, "Lz4_frameContentChecksumFlag")?;
    Ok(Value::Int(i32::from(parse_frame_header(&data)?.checksum)))
}

pub(crate) fn native_lz4_frame_content_size(args: &[Value]) -> Result<Value, String> {
    let data = arg_string(args, "Lz4_frameContentSize")?;
    Ok(Value::Long(parse_frame_header(&data)?.content_size as i64))
}

pub(crate) fn native_lz4_frame_dict_id(args: &[Value]) -> Result<Value, String> {
    let data = arg_string(args, "Lz4_frameDictId")?;
    Ok(Value::Int(parse_frame_header(&data)?.dict_id as i32))
}

#[cfg(test)]
mod lz4_native_tests {
    use super::*;

    fn s(text: &str) -> Value {
        Value::String(Rc::new(text.to_string()))
    }

    fn latin(bytes: &[u8]) -> Value {
        Value::String(Rc::new(bytes.iter().map(|&b| b as char).collect()))
    }

    fn as_bytes(v: &Value) -> Vec<u8> {
        match v {
            Value::String(s) => {
                if s.chars().all(|c| (c as u32) <= 0xFF) {
                    s.chars().map(|c| c as u8).collect()
                } else {
                    s.as_bytes().to_vec()
                }
            }
            other => panic!("expected string, got {}", other.type_name()),
        }
    }

    fn as_text(v: &Value) -> String {
        match v {
            Value::String(s) => s.as_str().to_string(),
            other => panic!("expected string, got {}", other.type_name()),
        }
    }

    fn as_i64(v: &Value) -> i64 {
        v.to_i64().unwrap_or_else(|| panic!("expected int, got {}", v.type_name()))
    }

    fn block_rt(payload: &[u8]) -> Vec<u8> {
        let c = native_lz4_compress(&[latin(payload)]).expect("compress");
        let d = native_lz4_decompress(&[c]).expect("decompress");
        as_bytes(&d)
    }

    #[test]
    fn block_roundtrip_text() {
        let p = b"LZ4 block round-trip. Repeated. Repeated. Repeated.";
        assert_eq!(block_rt(p), p);
    }

    #[test]
    fn block_roundtrip_empty() {
        assert_eq!(block_rt(b""), b"");
    }

    #[test]
    fn block_roundtrip_binary() {
        let p: Vec<u8> = (0..=255u16).map(|b| b as u8).collect::<Vec<_>>().repeat(3);
        assert_eq!(block_rt(&p), p);
    }

    #[test]
    fn block_roundtrip_large() {
        let p = b"0123456789abcdef".repeat(65536);
        assert_eq!(block_rt(&p), p);
    }

    #[test]
    fn frame_roundtrip_and_magic() {
        let p = b"frame payload with some repetition repetition repetition";
        let c = native_lz4_frame_compress(&[latin(p)]).expect("frame compress");
        let bytes = as_bytes(&c);
        assert_eq!(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]), 0x184D_2204);
        let d = native_lz4_frame_decompress(&[c]).expect("frame decompress");
        assert_eq!(as_bytes(&d), p);
    }

    #[test]
    fn frame_roundtrip_empty() {
        let c = native_lz4_frame_compress(&[s("")]).expect("frame compress");
        let d = native_lz4_frame_decompress(&[c]).expect("frame decompress");
        assert_eq!(as_text(&d), "");
    }

    #[test]
    fn speed_variants_roundtrip() {
        let p = b"speed variant payload 0123456789";
        for f in [native_lz4_high_speed_compress, native_lz4_high_compression_compress] {
            let c = f(&[latin(p)]).expect("variant compress");
            let d = native_lz4_decompress(&[c]).expect("decompress");
            assert_eq!(as_bytes(&d), p);
        }
    }

    #[test]
    fn acceleration_validated() {
        assert!(native_lz4_compress_with_acceleration(&[s("x"), Value::Int(1)]).is_ok());
        assert!(native_lz4_compress_with_acceleration(&[s("x"), Value::Int(0)]).is_err());
        assert!(native_lz4_compress_with_acceleration(&[s("x"), Value::Int(-3)]).is_err());
        let c = native_lz4_compress_with_acceleration(&[s("accel payload"), Value::Int(9)])
            .expect("accel");
        let d = native_lz4_decompress(&[c]).expect("decompress");
        assert_eq!(as_text(&d), "accel payload");
    }

    #[test]
    fn compress_bound_sane() {
        let b = as_i64(&native_lz4_compress_bound(&[Value::Int(1000)]).expect("bound"));
        assert!(b >= 1000);
        assert!(native_lz4_compress_bound(&[Value::Int(-1)]).is_err());
        assert!(native_lz4_compress_bound(&[]).is_err());
    }

    #[test]
    fn version_number_pinned() {
        assert_eq!(
            native_lz4_version_number(&[]).expect("version"),
            Value::Int(1106)
        );
    }

    #[test]
    fn xxhash32_empty_vector() {
        // XXH32("", seed 0) = 0x02CC5D05.
        let h = as_i64(&native_lz4_xxhash32(&[s(""), Value::Int(0)]).expect("xxh32"));
        assert_eq!(h, 0x02CC_5D05);
    }

    #[test]
    fn xxhash32_stable_and_seeded() {
        let a = native_lz4_xxhash32(&[s("hello"), Value::Int(0)]).expect("h");
        let b = native_lz4_xxhash32(&[s("hello"), Value::Int(0)]).expect("h");
        assert_eq!(a, b);
        let c = native_lz4_xxhash32(&[s("hello"), Value::Int(1)]).expect("h");
        assert_ne!(a, c);
        assert_ne!(
            a,
            native_lz4_xxhash32(&[s("hellp"), Value::Int(0)]).expect("h")
        );
    }

    #[test]
    fn xxhash64_empty_vector() {
        // XXH64("", seed 0) = 0xEF46DB3751D8E999.
        let h = as_i64(&native_lz4_xxhash64(&[s(""), Value::Long(0)]).expect("xxh64"));
        assert_eq!(h, 0xEF46_DB3751D8E999u64 as i64);
    }

    #[test]
    fn decompress_garbage_errors() {
        assert!(native_lz4_decompress(&[s("not a block")]).is_err());
        assert!(native_lz4_frame_decompress(&[s("not a frame")]).is_err());
        assert!(native_lz4_decompress(&[]).is_err());
    }

    #[test]
    fn decompress_safe_limit() {
        let p = b"safety-limited payload";
        let c = native_lz4_compress(&[latin(p)]).expect("compress");
        let ok = native_lz4_decompress_safe(&[c.clone(), Value::Int(1024)]).expect("safe");
        assert_eq!(as_bytes(&ok), p);
        assert!(native_lz4_decompress_safe(&[c, Value::Int(2)]).is_err());
        assert!(native_lz4_decompress_safe(&[s("x"), Value::Int(-1)]).is_err());
    }

    #[test]
    fn frame_info_on_real_frame() {
        let p = b"frame info payload ".repeat(100);
        let c = native_lz4_frame_compress(&[latin(&p)]).expect("frame compress");
        let size = as_i64(&native_lz4_frame_block_size(&[c.clone()]).expect("bs"));
        assert!([65536, 262144, 1048576, 4194304].contains(&size));
        let mode = as_i64(&native_lz4_frame_block_mode(&[c.clone()]).expect("bm"));
        assert!(mode == 0 || mode == 1);
        let cksum = as_i64(
            &native_lz4_frame_content_checksum_flag(&[c.clone()]).expect("cc"),
        );
        assert!(cksum == 0 || cksum == 1);
        let dict = as_i64(&native_lz4_frame_dict_id(&[c]).expect("dict"));
        assert_eq!(dict, 0);
    }

    #[test]
    fn frame_info_rejects_non_frames() {
        assert!(native_lz4_frame_block_size(&[s("junk")]).is_err());
        assert!(native_lz4_frame_content_size(&[s("")]).is_err());
        // Legacy magic 0x184C2102.
        let legacy = latin(&[0x02, 0x21, 0x4C, 0x18, 0x40, 0x40, 0x00]);
        assert!(native_lz4_frame_block_size(&[legacy]).is_err());
        // Skippable magic 0x184D2A50.
        let skip = latin(&[0x50, 0x2A, 0x4D, 0x18, 0x00, 0x00, 0x00]);
        assert!(native_lz4_frame_content_size(&[skip]).is_err());
    }

    #[test]
    fn non_string_args_rejected() {
        assert!(native_lz4_compress(&[Value::Int(1)]).is_err());
        assert!(native_lz4_frame_compress(&[Value::Null]).is_err());
        assert!(native_lz4_xxhash32(&[s("x")]).is_err());
    }
}
