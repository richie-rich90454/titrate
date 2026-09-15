// Titrate Alpha 0.4 – bytecode virtual machine: zstd native functions
// Precision in every step – richie-rich90454, 2026
//
// Real Zstandard compression via the `zstd` crate (bundled C library sources
// are compiled in by default; there is no separate `bundled` feature in
// zstd 0.13 — the default build already vendors the upstream sources).
// Binary data is exchanged via Latin-1 encoded strings (each byte → one char),
// preserving all 256 byte values without UTF-8 corruption (same convention as
// zlib.rs). Decompressed output is returned as UTF-8 text when valid, else as
// Latin-1, so both text and binary round-trips reproduce the input exactly.

use super::super::super::value::Value;
use std::io::{Cursor, Read};
use std::rc::Rc;

const ZSTD_MAGIC: u32 = 0xFD2F_B528;
const ZSTD_BLOCKSIZE_MAX: u64 = 128 * 1024;

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

/// Small integers as Int, larger ones as Long (both fit Titrate numerics).
fn int_or_long(v: u64) -> Value {
    if v <= i32::MAX as u64 {
        Value::Int(v as i32)
    } else {
        Value::Long(v as i64)
    }
}

fn arg_string(args: &[Value], what: &str) -> Result<Vec<u8>, String> {
    match args.first() {
        Some(Value::String(s)) => Ok(input_bytes(s)),
        Some(other) => Err(format!("{}: expected a string argument, got {}", what, other.type_name())),
        None => Err(format!("{}: expected a string argument", what)),
    }
}

fn arg_data(args: &[Value], idx: usize, what: &str) -> Result<Vec<u8>, String> {
    match args.get(idx) {
        Some(Value::String(s)) => Ok(input_bytes(s)),
        Some(other) => Err(format!("{}: argument {} must be a string, got {}", what, idx, other.type_name())),
        None => Err(format!("{}: expected a string argument {}", what, idx)),
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

/// Extract Titrate string samples (Array or ArrayList ClassInstance) for
/// dictionary training.
fn arg_samples(args: &[Value], idx: usize, what: &str) -> Result<Vec<Vec<u8>>, String> {
    let items: Vec<Value> = match args.get(idx) {
        Some(Value::Array { elements }) => elements.clone(),
        Some(Value::ClassInstance { fields, .. }) => {
            let borrowed = fields.borrow();
            match borrowed.get("_elements") {
                Some(Value::Array { elements }) => elements.clone(),
                _ => return Err(format!("{}: samples ArrayList has no _elements field", what)),
            }
        }
        Some(other) => {
            return Err(format!(
                "{}: samples must be an ArrayList<string>, got {}",
                what,
                other.type_name()
            ))
        }
        None => return Err(format!("{}: expected a samples argument {}", what, idx)),
    };
    items
        .iter()
        .map(|v| match v {
            Value::String(s) => Ok(input_bytes(s)),
            other => Err(format!("{}: samples must be strings, got {}", what, other.type_name())),
        })
        .collect()
}

fn check_level(level: i64, what: &str) -> Result<i32, String> {
    if (1..=22).contains(&level) {
        Ok(level as i32)
    } else {
        Err(format!("{}: level must be 1..=22 (got {})", what, level))
    }
}

pub(crate) fn native_zstd_compress(args: &[Value]) -> Result<Value, String> {
    let data = arg_string(args, "Zstd_compress")?;
    let level = check_level(arg_int(args, 1, "Zstd_compress")?, "Zstd_compress")?;
    let compressed =
        zstd::bulk::compress(&data, level).map_err(|e| format!("Zstd_compress: {}", e))?;
    Ok(Value::String(Rc::new(bytes_to_string(&compressed))))
}

pub(crate) fn native_zstd_decompress(args: &[Value]) -> Result<Value, String> {
    let data = arg_string(args, "Zstd_decompress")?;
    let decompressed = zstd::stream::decode_all(Cursor::new(&data))
        .map_err(|e| format!("Zstd_decompress: {}", e))?;
    Ok(output_value(decompressed))
}

pub(crate) fn native_zstd_compress_with_dict(args: &[Value]) -> Result<Value, String> {
    let data = arg_string(args, "Zstd_compressWithDict")?;
    let dict = arg_data(args, 1, "Zstd_compressWithDict")?;
    let mut ctx = zstd::bulk::Compressor::with_dictionary(3, &dict)
        .map_err(|e| format!("Zstd_compressWithDict: {}", e))?;
    let compressed = ctx.compress(&data).map_err(|e| format!("Zstd_compressWithDict: {}", e))?;
    Ok(Value::String(Rc::new(bytes_to_string(&compressed))))
}

pub(crate) fn native_zstd_decompress_with_dict(args: &[Value]) -> Result<Value, String> {
    let data = arg_string(args, "Zstd_decompressWithDict")?;
    let dict = arg_data(args, 1, "Zstd_decompressWithDict")?;
    let mut decoder = zstd::stream::Decoder::with_dictionary(Cursor::new(&data), &dict)
        .map_err(|e| format!("Zstd_decompressWithDict: {}", e))?;
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .map_err(|e| format!("Zstd_decompressWithDict: {}", e))?;
    Ok(output_value(decompressed))
}

pub(crate) fn native_zstd_compress_bound(args: &[Value]) -> Result<Value, String> {
    let n = arg_int(args, 0, "Zstd_compressBound")?;
    if n < 0 {
        return Err("Zstd_compressBound: input size must be >= 0".to_string());
    }
    Ok(int_or_long(zstd::zstd_safe::compress_bound(n as usize) as u64))
}

pub(crate) fn native_zstd_get_frame_content_size(args: &[Value]) -> Result<Value, String> {
    let data = arg_string(args, "Zstd_getFrameContentSize")?;
    match zstd::zstd_safe::get_frame_content_size(&data) {
        Ok(Some(n)) => Ok(Value::Long(n as i64)),
        Ok(None) => Ok(Value::Long(-1)),
        Err(e) => Err(format!("Zstd_getFrameContentSize: {}", e)),
    }
}

/// Parsed Zstd frame header (RFC 8878 §3.1.1).
struct FrameHeader {
    window_size: u64,
    checksum: bool,
    header_size: usize,
}

fn parse_frame_header(data: &[u8]) -> Result<FrameHeader, String> {
    if data.len() < 5 {
        return Err("Zstd frame info: not a Zstd frame (too short)".to_string());
    }
    let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    if magic != ZSTD_MAGIC {
        return Err("Zstd frame info: not a Zstd frame (bad magic number)".to_string());
    }
    let fhd = data[4];
    if fhd & 0x08 != 0 {
        return Err("Zstd frame info: reserved header bit is set".to_string());
    }
    let fcs_flag = (fhd >> 6) & 0x03;
    let single_segment = fhd & 0x20 != 0;
    let checksum = fhd & 0x04 != 0;
    let did_flag = fhd & 0x03;
    let mut pos = 5usize;

    let window_size = if single_segment {
        None
    } else {
        if data.len() <= pos {
            return Err("Zstd frame info: truncated window descriptor".to_string());
        }
        let wd = data[pos];
        pos += 1;
        let base = 1u64 << (10 + (wd >> 3) as u32);
        Some(base + (base / 8) * (u64::from(wd & 0x07)))
    };

    let did_len = [0, 1, 2, 4][did_flag as usize];
    if data.len() < pos + did_len {
        return Err("Zstd frame info: truncated dictionary id".to_string());
    }
    pos += did_len;

    let fcs_len = match (single_segment, fcs_flag) {
        (true, _) => 1,
        (false, 0) => 0,
        (false, 1) => 2,
        (false, 2) => 4,
        (false, _) => 8,
    };
    if data.len() < pos + fcs_len {
        return Err("Zstd frame info: truncated content size".to_string());
    }
    let content_size = match fcs_len {
        0 => None,
        1 => Some(u64::from(data[pos])),
        2 => Some(u64::from(u16::from_le_bytes([data[pos], data[pos + 1]])) + 256),
        4 => Some(u64::from(u32::from_le_bytes([
            data[pos],
            data[pos + 1],
            data[pos + 2],
            data[pos + 3],
        ]))),
        _ => Some(u64::from_le_bytes([
            data[pos],
            data[pos + 1],
            data[pos + 2],
            data[pos + 3],
            data[pos + 4],
            data[pos + 5],
            data[pos + 6],
            data[pos + 7],
        ])),
    };
    pos += fcs_len;

    Ok(FrameHeader {
        window_size: window_size.or(content_size).unwrap_or(0),
        checksum,
        header_size: pos + 4,
    })
}

pub(crate) fn native_zstd_frame_header_size(args: &[Value]) -> Result<Value, String> {
    let data = arg_string(args, "Zstd_frameHeaderSize")?;
    Ok(Value::Int(parse_frame_header(&data)?.header_size as i32))
}

pub(crate) fn native_zstd_version_number(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::Int(zstd::zstd_safe::version_number() as i32))
}

pub(crate) fn native_zstd_is_frame(args: &[Value]) -> Result<Value, String> {
    let data = arg_string(args, "Zstd_isFrame")?;
    if data.len() < 4 {
        return Ok(Value::Bool(false));
    }
    Ok(Value::Bool(
        u32::from_le_bytes([data[0], data[1], data[2], data[3]]) == ZSTD_MAGIC,
    ))
}

pub(crate) fn native_zstd_min_compression_level(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::Int(*zstd::compression_level_range().start()))
}

pub(crate) fn native_zstd_max_compression_level(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::Int(*zstd::compression_level_range().end()))
}

pub(crate) fn native_zstd_train_dictionary(args: &[Value]) -> Result<Value, String> {
    let samples = arg_samples(args, 0, "Zstd_trainDictionary")?;
    let dict_size = arg_int(args, 1, "Zstd_trainDictionary")?;
    if dict_size <= 0 {
        return Err("Zstd_trainDictionary: dictSize must be > 0".to_string());
    }
    let refs: Vec<&[u8]> = samples.iter().map(Vec::as_slice).collect();
    let dict = zstd::dict::from_samples(&refs, dict_size as usize)
        .map_err(|e| format!("Zstd_trainDictionary: {}", e))?;
    Ok(Value::String(Rc::new(bytes_to_string(&dict))))
}

pub(crate) fn native_zstd_get_dict_id(args: &[Value]) -> Result<Value, String> {
    let data = arg_string(args, "Zstd_getDictId")?;
    Ok(Value::Int(
        zstd::zstd_safe::get_dict_id_from_frame(&data).map_or(0, |id| id.get() as i32),
    ))
}

pub(crate) fn native_zstd_frame_block_size_max(args: &[Value]) -> Result<Value, String> {
    let data = arg_string(args, "Zstd_frameBlockSizeMax")?;
    let window = parse_frame_header(&data)?.window_size;
    Ok(int_or_long(window.min(ZSTD_BLOCKSIZE_MAX)))
}

pub(crate) fn native_zstd_frame_window_size(args: &[Value]) -> Result<Value, String> {
    let data = arg_string(args, "Zstd_frameWindowSize")?;
    Ok(int_or_long(parse_frame_header(&data)?.window_size))
}

pub(crate) fn native_zstd_frame_checksum_flag(args: &[Value]) -> Result<Value, String> {
    let data = arg_string(args, "Zstd_frameChecksumFlag")?;
    Ok(Value::Int(i32::from(parse_frame_header(&data)?.checksum)))
}

pub(crate) fn native_zstd_compress_with_params(args: &[Value]) -> Result<Value, String> {
    // Called as (data, level, windowLog, chainLog, searchLog, minMatch) from
    // Zstd.tr; tolerate a 5-arg form (data, windowLog, chainLog, searchLog,
    // minMatch) with the default level.
    let data = arg_string(args, "Zstd_compressWithParams")?;
    let (level, rest) = if args.len() >= 6 {
        (check_level(arg_int(args, 1, "Zstd_compressWithParams")?, "Zstd_compressWithParams")?, 2)
    } else {
        (3, 1)
    };
    let get = |i: usize| arg_int(args, rest + i, "Zstd_compressWithParams");
    let (window_log, chain_log, search_log, min_match) = (get(0)?, get(1)?, get(2)?, get(3)?);
    let mut ctx =
        zstd::bulk::Compressor::new(level).map_err(|e| format!("Zstd_compressWithParams: {}", e))?;
    let mut set = |p| ctx.set_parameter(p).map_err(|e| format!("Zstd_compressWithParams: {}", e));
    set(zstd::zstd_safe::CParameter::WindowLog(window_log as u32))?;
    set(zstd::zstd_safe::CParameter::ChainLog(chain_log as u32))?;
    set(zstd::zstd_safe::CParameter::SearchLog(search_log as u32))?;
    set(zstd::zstd_safe::CParameter::MinMatch(min_match as u32))?;
    let compressed = ctx.compress(&data).map_err(|e| format!("Zstd_compressWithParams: {}", e))?;
    Ok(Value::String(Rc::new(bytes_to_string(&compressed))))
}

/// Split concatenated frames; Err names the offending frame index.
fn split_frames(data: &[u8]) -> Result<Vec<&[u8]>, String> {
    let mut frames = Vec::new();
    let mut pos = 0usize;
    while pos < data.len() {
        let size = zstd::zstd_safe::find_frame_compressed_size(&data[pos..])
            .map_err(|e| format!("Zstd frame {}: {}", frames.len(), e))?;
        if size == 0 {
            return Err(format!("Zstd frame {}: zero-length frame", frames.len()));
        }
        frames.push(&data[pos..pos + size]);
        pos += size;
    }
    if frames.is_empty() {
        return Err("Zstd: no frames found".to_string());
    }
    Ok(frames)
}

pub(crate) fn native_zstd_decompress_frame(args: &[Value]) -> Result<Value, String> {
    let data = arg_string(args, "Zstd_decompressFrame")?;
    let index = arg_int(args, 1, "Zstd_decompressFrame")?;
    if index < 0 {
        return Err("Zstd_decompressFrame: frameIndex must be >= 0".to_string());
    }
    let frames = split_frames(&data)?;
    let frame = frames.get(index as usize).ok_or_else(|| {
        format!("Zstd_decompressFrame: frameIndex {} out of range ({} frames)", index, frames.len())
    })?;
    let decompressed = zstd::stream::decode_all(Cursor::new(frame))
        .map_err(|e| format!("Zstd_decompressFrame: {}", e))?;
    Ok(output_value(decompressed))
}

pub(crate) fn native_zstd_count_frames(args: &[Value]) -> Result<Value, String> {
    let data = arg_string(args, "Zstd_countFrames")?;
    Ok(Value::Int(split_frames(&data)?.len() as i32))
}

#[cfg(test)]
mod zstd_native_tests {
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

    fn rt(data: &Value, level: i64) -> Vec<u8> {
        let c = native_zstd_compress(&[data.clone(), Value::Int(level as i32)]).expect("compress");
        let d = native_zstd_decompress(&[c]).expect("decompress");
        as_bytes(&d)
    }

    #[test]
    fn roundtrip_levels() {
        let p = s("zstd levels. Repeat! Repeat! Repeat! Repeat!");
        for level in [1, 3, 22] {
            assert_eq!(rt(&p, level), as_bytes(&p), "level {}", level);
        }
    }

    #[test]
    fn roundtrip_empty() {
        assert_eq!(rt(&s(""), 3), b"");
    }

    #[test]
    fn roundtrip_binary() {
        let p: Vec<u8> = (0..=255u16).map(|b| b as u8).collect::<Vec<_>>().repeat(5);
        assert_eq!(rt(&latin(&p), 3), p);
    }

    #[test]
    fn frame_has_magic() {
        let c = native_zstd_compress(&[s("magic check"), Value::Int(3)]).expect("compress");
        let bytes = as_bytes(&c);
        assert_eq!(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]), 0xFD2F_B528);
    }

    #[test]
    fn level_validation() {
        assert!(native_zstd_compress(&[s("x"), Value::Int(0)]).is_err());
        assert!(native_zstd_compress(&[s("x"), Value::Int(23)]).is_err());
        assert!(native_zstd_compress(&[s("x"), Value::Int(-2)]).is_err());
        assert!(native_zstd_compress(&[s("x")]).is_err());
    }

    #[test]
    fn decompress_garbage_errors() {
        assert!(native_zstd_decompress(&[s("not zstd")]).is_err());
        assert!(native_zstd_decompress(&[]).is_err());
    }

    #[test]
    fn is_frame() {
        let c = native_zstd_compress(&[s("frame?"), Value::Int(3)]).expect("compress");
        assert_eq!(native_zstd_is_frame(&[c]).expect("is"), Value::Bool(true));
        assert_eq!(native_zstd_is_frame(&[s("nope")]).expect("is"), Value::Bool(false));
        assert_eq!(native_zstd_is_frame(&[s("ab")]).expect("is"), Value::Bool(false));
    }

    #[test]
    fn frame_content_size_matches() {
        let payload = "sized payload 0123456789".repeat(10);
        let c = native_zstd_compress(&[s(&payload), Value::Int(3)]).expect("compress");
        let n = as_i64(&native_zstd_get_frame_content_size(&[c]).expect("fcs"));
        assert_eq!(n, payload.len() as i64);
    }

    #[test]
    fn frame_header_fields_sane() {
        let c = native_zstd_compress(&[s("header fields"), Value::Int(3)]).expect("compress");
        let hs = as_i64(&native_zstd_frame_header_size(std::slice::from_ref(&c)).expect("hs"));
        assert!((2..=18).contains(&hs));
        let ws = as_i64(&native_zstd_frame_window_size(std::slice::from_ref(&c)).expect("ws"));
        assert!(ws > 0);
        let bm = as_i64(&native_zstd_frame_block_size_max(std::slice::from_ref(&c)).expect("bm"));
        assert!(bm > 0 && bm <= 128 * 1024);
        let cf = as_i64(&native_zstd_frame_checksum_flag(&[c]).expect("cf"));
        assert!(cf == 0 || cf == 1);
        assert!(native_zstd_frame_header_size(&[s("junk")]).is_err());
    }

    #[test]
    fn version_and_levels() {
        assert!(as_i64(&native_zstd_version_number(&[]).expect("v")) > 0);
        let min = as_i64(&native_zstd_min_compression_level(&[]).expect("min"));
        let max = as_i64(&native_zstd_max_compression_level(&[]).expect("max"));
        assert!(min < 1 && max >= 22);
    }

    #[test]
    fn compress_bound_covers_input() {
        let b = as_i64(&native_zstd_compress_bound(&[Value::Int(1000)]).expect("bound"));
        assert!(b >= 1000);
        assert!(native_zstd_compress_bound(&[Value::Int(-1)]).is_err());
    }

    #[test]
    fn dict_roundtrip() {
        let samples: Vec<Vec<u8>> = (0..20).map(|i| format!("sample row {} with common prefix data", i).into_bytes()).collect();
        let dict = {
            let refs: Vec<&[u8]> = samples.iter().map(Vec::as_slice).collect();
            zstd::dict::from_samples(&refs, 4096).expect("train")
        };
        let payload = "sample row 7 with common prefix data and more";
        let c = native_zstd_compress_with_dict(&[s(payload), latin(&dict)]).expect("c");
        let d = native_zstd_decompress_with_dict(&[c, latin(&dict)]).expect("d");
        assert_eq!(as_text(&d), payload);
    }

    #[test]
    fn train_dictionary_flow() {
        let run = || {
            let mut items = Vec::new();
            for i in 0..30 {
                items.push(Value::String(Rc::new(format!("training sample number {} padding padding", i))));
            }
            let mut fields = std::collections::HashMap::new();
            fields.insert("_elements".to_string(), Value::Array { elements: items });
            let list = Value::ClassInstance {
                class_name: "ArrayList".to_string(),
                fields: Rc::new(std::cell::RefCell::new(fields)),
                vtable: std::collections::HashMap::new(),
            };
            native_zstd_train_dictionary(&[list, Value::Int(8192)])
        };
        let dict = run().expect("train");
        let payload = "training sample number 3 padding padding plus tail";
        let c = native_zstd_compress_with_dict(&[s(payload), dict.clone()]).expect("c");
        assert!(native_zstd_get_dict_id(std::slice::from_ref(&c)).is_ok());
        let d = native_zstd_decompress_with_dict(&[c.clone(), dict]).expect("d");
        assert_eq!(as_text(&d), payload);
        assert!(native_zstd_train_dictionary(&[s("x"), Value::Int(0)]).is_err());
    }

    #[test]
    fn params_roundtrip() {
        let p = s("params payload ".repeat(50).as_str());
        let c6 = native_zstd_compress_with_params(&[
            p.clone(),
            Value::Int(3),
            Value::Int(15),
            Value::Int(13),
            Value::Int(11),
            Value::Int(4),
        ])
        .expect("params6");
        assert_eq!(as_text(&native_zstd_decompress(&[c6]).expect("d")), as_text(&p));
        let c5 = native_zstd_compress_with_params(&[
            p.clone(),
            Value::Int(15),
            Value::Int(13),
            Value::Int(11),
            Value::Int(4),
        ])
        .expect("params5");
        assert_eq!(as_text(&native_zstd_decompress(&[c5]).expect("d")), as_text(&p));
    }

    #[test]
    fn multi_frame_split_and_count() {
        let a = native_zstd_compress(&[s("frame one"), Value::Int(3)]).expect("a");
        let b = native_zstd_compress(&[s("frame two"), Value::Int(3)]).expect("b");
        let mut both = as_bytes(&a);
        both.extend_from_slice(&as_bytes(&b));
        let joined = latin(&both);
        assert_eq!(
            native_zstd_count_frames(std::slice::from_ref(&joined)).expect("count"),
            Value::Int(2)
        );
        assert_eq!(
            as_text(&native_zstd_decompress_frame(&[joined.clone(), Value::Int(0)]).expect("f0")),
            "frame one"
        );
        assert_eq!(
            as_text(&native_zstd_decompress_frame(&[joined.clone(), Value::Int(1)]).expect("f1")),
            "frame two"
        );
        assert!(native_zstd_decompress_frame(&[joined.clone(), Value::Int(2)]).is_err());
        assert!(native_zstd_decompress_frame(&[joined, Value::Int(-1)]).is_err());
        assert!(native_zstd_count_frames(&[s("junk")]).is_err());
    }

    #[test]
    fn wrong_dict_fails() {
        // A frame trained with a dictionary cannot be opened with a
        // truncated (hence invalid) dictionary.
        let samples: Vec<Vec<u8>> = (0..20).map(|i| format!("dict row {} common content here", i).into_bytes()).collect();
        let refs: Vec<&[u8]> = samples.iter().map(Vec::as_slice).collect();
        let dict = zstd::dict::from_samples(&refs, 4096).expect("train");
        let payload = "dict row 3 common content here plus tail";
        let c = native_zstd_compress_with_dict(&[s(payload), latin(&dict)]).expect("c");
        assert!(native_zstd_decompress_with_dict(&[c.clone(), latin(&dict[..8])]).is_err());
        assert!(native_zstd_decompress_with_dict(&[c, s("not-a-dict")]).is_err());
    }
}
