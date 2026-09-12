// Titrate Alpha 0.4 – bytecode virtual machine: lzma native functions
// Precision in every step – richie-rich90454, 2026
//
// Real LZMA/XZ compression via the `liblzma` crate (static build of the
// reference XZ implementation). The Titrate `preset` (0..=9, optionally
// ORed with PRESET_EXTREME) selects the real encoder preset; PRESET_EXTREME
// maps to level 9. Binary data is exchanged via Latin-1 encoded strings
// (each byte → one char), preserving all 256 byte values without UTF-8
// corruption (same convention as zlib.rs). Decompressed output is returned
// as UTF-8 text when valid, else as Latin-1, so both text and binary
// round-trips reproduce the input exactly.
//
// Format codes mirror lib/tt/compression/Lzma.tr:
// FORMAT_XZ=0, FORMAT_ALONE=1, FORMAT_RAW=2, FORMAT_AUTO=3.
// XZ magic: FD 37 7A 58 5A 00. LZMA-alone streams start with 5D 00 00.
// RAW stores/decodes the payload as-is (no codec framing).

use super::super::super::value::Value;
use std::io::Read;
use std::rc::Rc;

const FORMAT_XZ: i64 = 0;
const FORMAT_ALONE: i64 = 1;
const FORMAT_RAW: i64 = 2;
const FORMAT_AUTO: i64 = 3;

/// PRESET_EXTREME from Lzma.tr (INT_MIN | 6); only its level bits matter here.
const PRESET_EXTREME_BIT: i64 = 0x8000_0000;

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

fn arg_format(args: &[Value], idx: usize, default: i64) -> Result<i64, String> {
    match args.get(idx) {
        None => Ok(default),
        Some(v) => v.to_i64().ok_or_else(|| {
            format!("Lzma_compress: format argument must be an integer, got {}", v.type_name())
        }),
    }
}

fn check_preset(preset: i64) -> Result<u32, String> {
    // Titrate passes PRESET_EXTREME as INT_MIN|level (a negative i32);
    // mask in u32 space so sign extension cannot leak into the level.
    let raw = (preset as u32) & !(PRESET_EXTREME_BIT as u32);
    if raw > 9 {
        return Err(format!(
            "Lzma_compress: preset level must be 0..=9 (got {}), optionally ORed with PRESET_EXTREME",
            preset
        ));
    }
    // EXTREME means maximum effort: real level 9.
    if (preset as u32) & (PRESET_EXTREME_BIT as u32) != 0 {
        Ok(9)
    } else {
        Ok(raw)
    }
}

/// Drive a raw liblzma stream to completion (process_vec consumes a
/// chunk per call, so loop until StreamEnd).
fn raw_process(
    stream: &mut liblzma::stream::Stream,
    data: &[u8],
    what: &str,
) -> Result<Vec<u8>, String> {
    use liblzma::stream::{Action, Status};
    let mut out = Vec::new();
    let mut start = 0usize;
    loop {
        let status = stream
            .process_vec(&data[start..], &mut out, Action::Finish)
            .map_err(|e| format!("{}: {}", what, e))?;
        start = stream.total_in() as usize;
        if status == Status::StreamEnd {
            break;
        }
        if start > data.len() {
            return Err(format!("{}: encoder overran input", what));
        }
    }
    Ok(out)
}

fn is_xz(data: &[u8]) -> bool {
    data.len() >= 6 && data[0..6] == [0xFD, 0x37, 0x7A, 0x58, 0x5A, 0x00]
}

fn is_alone(data: &[u8]) -> bool {
    data.len() >= 3 && data[0..3] == [0x5D, 0x00, 0x00]
}

pub(crate) fn native_lzma_compress(args: &[Value]) -> Result<Value, String> {
    let data = match args.first() {
        Some(Value::String(s)) => input_bytes(s),
        Some(other) => {
            return Err(format!(
                "Lzma_compress: expected a string argument, got {}",
                other.type_name()
            ))
        }
        None => return Err("Lzma_compress: expected a string argument".to_string()),
    };
    let format = arg_format(args, 1, FORMAT_XZ)?;
    let preset = match args.get(2) {
        None => 6,
        Some(v) => v.to_i64().ok_or_else(|| {
            format!("Lzma_compress: preset argument must be an integer, got {}", v.type_name())
        })?,
    };
    // The validated preset level drives the real encoder (0..=9).
    let level = check_preset(preset)?;

    match format {
        FORMAT_XZ | FORMAT_AUTO => {
            let mut encoder = liblzma::read::XzEncoder::new(&data[..], level);
            let mut compressed = Vec::new();
            encoder
                .read_to_end(&mut compressed)
                .map_err(|e| format!("Lzma_compress: XZ compression error: {}", e))?;
            Ok(Value::String(Rc::new(bytes_to_string(&compressed))))
        }
        FORMAT_ALONE => {
            let opts = liblzma::stream::LzmaOptions::new_preset(level)
                .map_err(|e| format!("Lzma_compress: bad preset {}: {}", level, e))?;
            let mut stream = liblzma::stream::Stream::new_lzma_encoder(&opts)
                .map_err(|e| format!("Lzma_compress: encoder init failed: {}", e))?;
            let compressed = raw_process(&mut stream, &data, "Lzma_compress")?;
            Ok(Value::String(Rc::new(bytes_to_string(&compressed))))
        }
        FORMAT_RAW => Ok(Value::String(Rc::new(bytes_to_string(&data)))),
        _ => Err(format!(
            "Lzma_compress: unknown format {} (expected 0=XZ, 1=ALONE, 2=RAW, 3=AUTO)",
            format
        )),
    }
}

pub(crate) fn native_lzma_decompress(args: &[Value]) -> Result<Value, String> {
    let data = match args.first() {
        Some(Value::String(s)) => input_bytes(s),
        Some(other) => {
            return Err(format!(
                "Lzma_decompress: expected a string argument, got {}",
                other.type_name()
            ))
        }
        None => return Err("Lzma_decompress: expected a string argument".to_string()),
    };
    let format = arg_format(args, 1, FORMAT_AUTO)?;

    let resolved = if format == FORMAT_AUTO {
        if is_xz(&data) {
            FORMAT_XZ
        } else if is_alone(&data) {
            FORMAT_ALONE
        } else {
            return Err(
                "Lzma_decompress: cannot auto-detect format (not XZ, LZMA-alone, or RAW)".to_string(),
            );
        }
    } else {
        format
    };

    match resolved {
        FORMAT_XZ => {
            let mut decoder = liblzma::read::XzDecoder::new(&data[..]);
            let mut decompressed = Vec::new();
            decoder
                .read_to_end(&mut decompressed)
                .map_err(|e| format!("Lzma_decompress: XZ decompression error: {}", e))?;
            Ok(output_value(decompressed))
        }
        FORMAT_ALONE => {
            let mut stream = liblzma::stream::Stream::new_lzma_decoder(u64::MAX)
                .map_err(|e| format!("Lzma_decompress: decoder init failed: {}", e))?;
            let decompressed = raw_process(&mut stream, &data, "Lzma_decompress")?;
            Ok(output_value(decompressed))
        }
        FORMAT_RAW => Ok(output_value(data)),
        _ => Err(format!(
            "Lzma_decompress: unknown format {} (expected 0=XZ, 1=ALONE, 2=RAW, 3=AUTO)",
            format
        )),
    }
}

#[cfg(test)]
mod lzma_native_tests {
    use super::*;

    fn s(text: &str) -> Value {
        Value::String(Rc::new(text.to_string()))
    }

    fn latin(bytes: &[u8]) -> Value {
        Value::String(Rc::new(bytes.iter().map(|&b| b as char).collect()))
    }

    fn as_latin(v: &Value) -> Vec<u8> {
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

    fn compress(data: &Value, format: i64, preset: i64) -> Value {
        native_lzma_compress(&[data.clone(), Value::Int(format as i32), Value::Int(preset as i32)])
            .expect("compress")
    }

    fn decompress(data: &Value, format: i64) -> Value {
        native_lzma_decompress(&[data.clone(), Value::Int(format as i32)]).expect("decompress")
    }

    #[test]
    fn xz_roundtrip_text() {
        let original = s("LZMA round-trip. Repeated data. Repeated data. Repeated data.");
        let back = decompress(&compress(&original, 0, 6), 3);
        assert_eq!(as_text(&back), as_text(&original));
    }

    #[test]
    fn xz_roundtrip_empty() {
        let back = decompress(&compress(&s(""), 0, 6), 3);
        assert_eq!(as_text(&back), "");
    }

    #[test]
    fn xz_roundtrip_binary() {
        let bytes: Vec<u8> = (0..=255u16).map(|b| b as u8).collect::<Vec<_>>().repeat(4);
        let back = decompress(&compress(&latin(&bytes), 0, 6), 3);
        assert_eq!(as_latin(&back), bytes);
    }

    #[test]
    fn xz_output_has_magic_and_shrinks() {
        let original = s(&"compressible ".repeat(200));
        let c = compress(&original, 0, 6);
        let bytes = as_latin(&c);
        assert_eq!(&bytes[..6], &[0xFD, 0x37, 0x7A, 0x58, 0x5A, 0x00]);
        assert!(bytes.len() < as_text(&original).len());
    }

    #[test]
    fn alone_roundtrip_and_magic() {
        let original = s("alone format payload 1234567890");
        let c = compress(&original, 1, 6);
        let bytes = as_latin(&c);
        assert_eq!(&bytes[..3], &[0x5D, 0x00, 0x00]);
        let back = decompress(&c, 3);
        assert_eq!(as_text(&back), as_text(&original));
    }

    #[test]
    fn raw_passthrough() {
        let original = s("raw payload stays as-is");
        let c = compress(&original, 2, 6);
        assert_eq!(as_text(&c), as_text(&original));
        let back = decompress(&c, 2);
        assert_eq!(as_text(&back), as_text(&original));
    }

    #[test]
    fn explicit_format_decode() {
        let original = s("explicit decode path");
        let cx = compress(&original, 0, 6);
        assert_eq!(as_text(&decompress(&cx, 0)), as_text(&original));
        let ca = compress(&original, 1, 6);
        assert_eq!(as_text(&decompress(&ca, 1)), as_text(&original));
    }

    #[test]
    fn preset_extreme_accepted() {
        let original = s("extreme preset still encodes");
        let c = compress(&original, 0, 0x8000_0006);
        assert_eq!(as_text(&decompress(&c, 3)), as_text(&original));
    }

    #[test]
    fn bad_preset_rejected() {
        assert!(native_lzma_compress(&[s("x"), Value::Int(0), Value::Int(10)]).is_err());
        assert!(native_lzma_compress(&[s("x"), Value::Int(0), Value::Int(-1)]).is_err());
    }

    #[test]
    fn unknown_format_rejected() {
        assert!(native_lzma_compress(&[s("x"), Value::Int(99), Value::Int(6)]).is_err());
        assert!(native_lzma_decompress(&[s("x"), Value::Int(99)]).is_err());
    }

    #[test]
    fn non_string_arg_rejected() {
        assert!(native_lzma_compress(&[Value::Int(1)]).is_err());
        assert!(native_lzma_decompress(&[Value::Null]).is_err());
        assert!(native_lzma_compress(&[]).is_err());
    }

    #[test]
    fn decompress_garbage_errors() {
        assert!(native_lzma_decompress(&[s("definitely not lzma data"), Value::Int(0)]).is_err());
    }

    #[test]
    fn auto_detect_unknown_errors() {
        assert!(native_lzma_decompress(&[s("plain text"), Value::Int(3)]).is_err());
    }

    #[test]
    fn xz_large_roundtrip() {
        let original = s(&"The quick brown fox jumps over the lazy dog. ".repeat(2000));
        let back = decompress(&compress(&original, 0, 6), 3);
        assert_eq!(as_text(&back), as_text(&original));
    }
}
