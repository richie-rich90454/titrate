//! TitrateValue byte-buffer serialization harness.
//!
//! Mirrors the C-ABI layout documented in HANDOFF section 7:
//! `{ i32 tag, i32 pad, [u8; 16] payload }` = 24 bytes. Every decode failure
//! carries a `deserialize:` reason so a malformed buffer can never be mistaken
//! for a valid native argument.

use crate::common::Lcg;

pub const SIZE: usize = 24;
pub const TAG_VOID: i32 = 0;
pub const TAG_NULL: i32 = 1;
pub const TAG_BOOL: i32 = 2;
pub const TAG_BYTE: i32 = 3;
pub const TAG_SHORT: i32 = 4;
pub const TAG_INT: i32 = 5;
pub const TAG_LONG: i32 = 6;
pub const TAG_VAST: i32 = 7;
pub const TAG_UVAST: i32 = 8;
pub const TAG_FLOAT: i32 = 9;
pub const TAG_DOUBLE: i32 = 10;
pub const TAG_CHAR: i32 = 11;
pub const TAG_STRING: i32 = 12;
pub const TAG_ARRAY: i32 = 13;
pub const TAG_CLASS_INSTANCE: i32 = 14;
pub const TAG_RESULT_OK: i32 = 15;
pub const TAG_RESULT_ERR: i32 = 16;
pub const TAG_ENUM_INSTANCE: i32 = 17;
pub const TAG_TUPLE: i32 = 18;
pub const TAG_HALF: i32 = 19;
pub const TAG_QUAD: i32 = 20;
pub const TAG_HANDLE: i32 = 21;
pub const TAG_CLOSURE: i32 = 22;

/// Same bound the production bridge uses for a serialized argument.
const MAX_ABI_LEN: i64 = 1 << 28;
const MAX_ARRAY_COUNT: i64 = 1_000_000;
const MIN_DATA_POINTER: i64 = 0x10000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decoded {
    Void,
    Null,
    Bool(bool),
    Int(i32),
    Long(i64),
    Bits(u64),
    Str { len: i64, has_data: bool },
    Array { len: i64, has_data: bool },
    Handle(i64),
}

fn payload_word(payload: &[u8; 16], start: usize) -> i64 {
    let bytes: [u8; 8] = payload[start..start + 8]
        .try_into()
        .expect("8-byte payload word");
    i64::from_le_bytes(bytes)
}

/// Encode one tagged value into a caller-owned 24-byte buffer.
pub fn encode(tag: i32, payload: &[u8; 16]) -> [u8; SIZE] {
    let mut buf = [0u8; SIZE];
    buf[..4].copy_from_slice(&tag.to_le_bytes());
    buf[8..].copy_from_slice(payload);
    buf
}

/// A length/pointer payload used by strings and arrays.
pub fn length_pointer_payload(len: i64, ptr: i64) -> [u8; 16] {
    let mut payload = [0u8; 16];
    payload[..8].copy_from_slice(&len.to_le_bytes());
    payload[8..].copy_from_slice(&ptr.to_le_bytes());
    payload
}

/// Decode and validate a serialized argument. Errors are descriptive by
/// construction: every path names the field that failed.
pub fn decode(buf: &[u8]) -> Result<Decoded, String> {
    if buf.len() != SIZE {
        return Err(format!(
            "deserialize: expected a {}-byte TitrateValue, got {} bytes",
            SIZE,
            buf.len()
        ));
    }
    let tag = i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
    if buf[4..8] != [0u8; 4] {
        return Err("deserialize: nonzero TitrateValue padding".to_string());
    }
    let mut payload = [0u8; 16];
    payload.copy_from_slice(&buf[8..]);

    let decoded = match tag {
        TAG_VOID => Decoded::Void,
        TAG_NULL => Decoded::Null,
        TAG_BOOL => Decoded::Bool(payload[0] != 0),
        TAG_BYTE | TAG_SHORT | TAG_INT => Decoded::Int(payload_word(&payload, 0) as i32),
        TAG_LONG => Decoded::Long(payload_word(&payload, 0)),
        TAG_FLOAT | TAG_DOUBLE | TAG_HALF | TAG_QUAD => {
            let raw: [u8; 8] = payload[..8].try_into().expect("8-byte payload word");
            Decoded::Bits(u64::from_le_bytes(raw))
        }
        TAG_CHAR => Decoded::Int(payload_word(&payload, 0) as i32),
        TAG_STRING => {
            let len = payload_word(&payload, 0);
            let ptr = payload_word(&payload, 8);
            check_length("string", len, MAX_ABI_LEN)?;
            check_pointer("string", len, ptr)?;
            Decoded::Str {
                len,
                has_data: ptr != 0,
            }
        }
        TAG_ARRAY | TAG_TUPLE => {
            let len = payload_word(&payload, 0);
            let ptr = payload_word(&payload, 8);
            check_length("array", len, MAX_ARRAY_COUNT)?;
            check_pointer("array", len, ptr)?;
            Decoded::Array {
                len,
                has_data: ptr != 0,
            }
        }
        TAG_VAST
        | TAG_UVAST
        | TAG_CLASS_INSTANCE
        | TAG_RESULT_OK
        | TAG_RESULT_ERR
        | TAG_ENUM_INSTANCE
        | TAG_HANDLE
        | TAG_CLOSURE => Decoded::Handle(payload_word(&payload, 0)),
        _ => return Err(format!("deserialize: unknown TitrateValue tag {}", tag)),
    };
    Ok(decoded)
}

fn check_length(kind: &str, len: i64, limit: i64) -> Result<(), String> {
    if len < 0 {
        return Err(format!("deserialize: negative {} length {}", kind, len));
    }
    if len > limit {
        return Err(format!("deserialize: {} length {} exceeds {}", kind, len, limit));
    }
    Ok(())
}

fn check_pointer(kind: &str, len: i64, ptr: i64) -> Result<(), String> {
    if len > 0 && ptr == 0 {
        return Err(format!("deserialize: {} length {} has a null data pointer", kind, len));
    }
    if len > 0 && ptr < MIN_DATA_POINTER {
        return Err(format!(
            "deserialize: {} data pointer {} is below the ABI minimum",
            kind, ptr
        ));
    }
    Ok(())
}

/// True only for a non-empty, prefix-tagged decoder error.
pub fn is_descriptive_error(error: &str) -> bool {
    error.starts_with("deserialize:") && error.len() > "deserialize:".len()
}

/// Deterministic mixed generator: scalar tags, valid containers, corrupt
/// headers, and unknown tags all occur without an external fuzz engine.
pub fn random_buffer(rng: &mut Lcg) -> [u8; SIZE] {
    let mut payload = [0u8; 16];
    rng.fill(&mut payload);
    let tag = match rng.range(0, 9) {
        0 => TAG_VOID,
        1 => TAG_NULL,
        2 => TAG_BOOL,
        3 => TAG_INT,
        4 => TAG_LONG,
        5 => TAG_DOUBLE,
        6 => TAG_STRING,
        7 => TAG_ARRAY,
        8 => TAG_HANDLE,
        _ => i32::MAX,
    };
    if tag == TAG_STRING && rng.boolean() {
        let len = rng.range(0, 16) as i64;
        let ptr = if len == 0 { 0 } else { MIN_DATA_POINTER + len };
        payload = length_pointer_payload(len, ptr);
    }
    if tag == TAG_ARRAY && rng.boolean() {
        let len = rng.range(0, 4) as i64;
        let ptr = MIN_DATA_POINTER + 0x1000;
        payload = length_pointer_payload(len, ptr);
    }
    let mut buf = encode(tag, &payload);
    if rng.range(0, 6) == 0 {
        // Corrupt the pad word to exercise the structural check.
        buf[5] = 0xFF;
    }
    buf
}

/// Fixed malformed inputs with the reason each one must produce.
pub fn malformed_cases() -> Vec<(&'static str, Vec<u8>)> {
    let unknown = encode(i32::MAX, &[0u8; 16]).to_vec();
    let mut padded = encode(TAG_VOID, &[0u8; 16]);
    padded[6] = 1;
    vec![
        ("truncated buffer", vec![0u8; SIZE - 1]),
        ("unknown tag", unknown),
        ("nonzero padding", padded.to_vec()),
        (
            "negative string length",
            encode(TAG_STRING, &length_pointer_payload(-1, MIN_DATA_POINTER)).to_vec(),
        ),
        (
            "null string data pointer",
            encode(TAG_STRING, &length_pointer_payload(8, 0)).to_vec(),
        ),
        (
            "oversized array",
            encode(TAG_ARRAY, &length_pointer_payload(MAX_ARRAY_COUNT + 1, MIN_DATA_POINTER)).to_vec(),
        ),
        (
            "negative array count",
            encode(TAG_ARRAY, &length_pointer_payload(-1, MIN_DATA_POINTER)).to_vec(),
        ),
    ]
}
