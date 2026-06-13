// Titrate Alpha 0.2 – bytecode virtual machine: hash natives
// Precision in every step – richie-rich90454, 2026

use super::super::super::value::Value;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{LazyLock, Mutex as StdMutex};
use md5::{Digest, Md5};
use sha1::Sha1;
use sha2::{Sha256, Sha384, Sha512};
use sha3::Sha3_256;
use sha3::Sha3_384;
use sha3::Sha3_512;
use blake2::{Blake2b512, Blake2s256};
use crc32fast::Hasher as Crc32Hasher;

// ---------------------------------------------------------------------------
// One-shot hash functions
// ---------------------------------------------------------------------------

pub(crate) fn native_hash_md5(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::String(s)) => {
            let mut hasher = Md5::new();
            hasher.update(s.as_bytes());
            let result = hasher.finalize();
            Ok(Value::String(Rc::new(format!("{:x}", result))))
        }
        _ => Err("Hash_md5: expected a String argument".to_string()),
    }
}

pub(crate) fn native_hash_sha1(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::String(s)) => {
            let mut hasher = Sha1::new();
            hasher.update(s.as_bytes());
            let result = hasher.finalize();
            Ok(Value::String(Rc::new(format!("{:x}", result))))
        }
        _ => Err("Hash_sha1: expected a String argument".to_string()),
    }
}

pub(crate) fn native_hash_sha256(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::String(s)) => {
            let mut hasher = Sha256::new();
            hasher.update(s.as_bytes());
            let result = hasher.finalize();
            Ok(Value::String(Rc::new(format!("{:x}", result))))
        }
        _ => Err("Hash_sha256: expected a String argument".to_string()),
    }
}

pub(crate) fn native_hash_sha384(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::String(s)) => {
            let mut hasher = Sha384::new();
            hasher.update(s.as_bytes());
            let result = hasher.finalize();
            Ok(Value::String(Rc::new(format!("{:x}", result))))
        }
        _ => Err("Hash_sha384: expected a String argument".to_string()),
    }
}

pub(crate) fn native_hash_sha512(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::String(s)) => {
            let mut hasher = Sha512::new();
            hasher.update(s.as_bytes());
            let result = hasher.finalize();
            Ok(Value::String(Rc::new(format!("{:x}", result))))
        }
        _ => Err("Hash_sha512: expected a String argument".to_string()),
    }
}

pub(crate) fn native_hash_sha3_256(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::String(s)) => {
            let mut hasher = Sha3_256::new();
            hasher.update(s.as_bytes());
            let result = hasher.finalize();
            Ok(Value::String(Rc::new(format!("{:x}", result))))
        }
        _ => Err("Hash_sha3_256: expected a String argument".to_string()),
    }
}

pub(crate) fn native_hash_sha3_384(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::String(s)) => {
            let mut hasher = Sha3_384::new();
            hasher.update(s.as_bytes());
            let result = hasher.finalize();
            Ok(Value::String(Rc::new(format!("{:x}", result))))
        }
        _ => Err("Hash_sha3_384: expected a String argument".to_string()),
    }
}

pub(crate) fn native_hash_sha3_512(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::String(s)) => {
            let mut hasher = Sha3_512::new();
            hasher.update(s.as_bytes());
            let result = hasher.finalize();
            Ok(Value::String(Rc::new(format!("{:x}", result))))
        }
        _ => Err("Hash_sha3_512: expected a String argument".to_string()),
    }
}

pub(crate) fn native_hash_blake2b(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::String(s)) => {
            let mut hasher = Blake2b512::new();
            hasher.update(s.as_bytes());
            let result = hasher.finalize();
            Ok(Value::String(Rc::new(format!("{:x}", result))))
        }
        _ => Err("Hash_blake2b: expected a String argument".to_string()),
    }
}

pub(crate) fn native_hash_blake2s(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::String(s)) => {
            let mut hasher = Blake2s256::new();
            hasher.update(s.as_bytes());
            let result = hasher.finalize();
            Ok(Value::String(Rc::new(format!("{:x}", result))))
        }
        _ => Err("Hash_blake2s: expected a String argument".to_string()),
    }
}

pub(crate) fn native_hash_crc32(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::String(s)) => {
            let mut hasher = Crc32Hasher::new();
            hasher.update(s.as_bytes());
            let checksum = hasher.finalize();
            Ok(Value::String(Rc::new(format!("{:08x}", checksum))))
        }
        _ => Err("Hash_crc32: expected a String argument".to_string()),
    }
}

/// Constant-time comparison of two hex digest strings to prevent timing attacks.
pub(crate) fn native_hmac_compare_digest(args: &[Value]) -> Result<Value, String> {
    let a = match args.first() {
        Some(Value::String(s)) => s.as_bytes(),
        _ => return Err("Hmac_compareDigest: expected two String arguments".to_string()),
    };
    let b = match args.get(1) {
        Some(Value::String(s)) => s.as_bytes(),
        _ => return Err("Hmac_compareDigest: expected two String arguments".to_string()),
    };
    // Constant-time comparison: always compare all bytes
    if a.len() != b.len() {
        // Still do a comparison of the same length to avoid length-based timing
        let len = a.len().min(b.len());
        let mut result: u8 = (a.len() != b.len()) as u8;
        for i in 0..len {
            result |= a[i] ^ b[i];
        }
        Ok(Value::Bool(false))
    } else {
        let mut result: u8 = 0;
        for i in 0..a.len() {
            result |= a[i] ^ b[i];
        }
        Ok(Value::Bool(result == 0))
    }
}

// ---------------------------------------------------------------------------
// Incremental Hasher – handle-based registry with enum dispatch
// ---------------------------------------------------------------------------

/// Enum wrapping the different hasher state machines.
/// Since `Digest` is not object-safe, we dispatch manually.
enum HasherState {
    Md5(Md5),
    Sha1(Sha1),
    Sha256(Sha256),
    Sha384(Sha384),
    Sha512(Sha512),
    Sha3_256(Sha3_256),
    Sha3_384(Sha3_384),
    Sha3_512(Sha3_512),
    Blake2b(Blake2b512),
    Blake2s(Blake2s256),
}

impl HasherState {
    fn algorithm_name(&self) -> &'static str {
        match self {
            HasherState::Md5(_) => "md5",
            HasherState::Sha1(_) => "sha1",
            HasherState::Sha256(_) => "sha256",
            HasherState::Sha384(_) => "sha384",
            HasherState::Sha512(_) => "sha512",
            HasherState::Sha3_256(_) => "sha3-256",
            HasherState::Sha3_384(_) => "sha3-384",
            HasherState::Sha3_512(_) => "sha3-512",
            HasherState::Blake2b(_) => "blake2b",
            HasherState::Blake2s(_) => "blake2s",
        }
    }

    fn update(&mut self, data: &[u8]) {
        match self {
            HasherState::Md5(h) => h.update(data),
            HasherState::Sha1(h) => h.update(data),
            HasherState::Sha256(h) => h.update(data),
            HasherState::Sha384(h) => h.update(data),
            HasherState::Sha512(h) => h.update(data),
            HasherState::Sha3_256(h) => h.update(data),
            HasherState::Sha3_384(h) => h.update(data),
            HasherState::Sha3_512(h) => h.update(data),
            HasherState::Blake2b(h) => h.update(data),
            HasherState::Blake2s(h) => h.update(data),
        }
    }

    /// Finalize and return hex digest, then reset the hasher to initial state.
    fn hex_digest_and_reset(&mut self) -> String {
        match self {
            HasherState::Md5(h) => {
                let result = h.finalize_reset();
                format!("{:x}", result)
            }
            HasherState::Sha1(h) => {
                let result = h.finalize_reset();
                format!("{:x}", result)
            }
            HasherState::Sha256(h) => {
                let result = h.finalize_reset();
                format!("{:x}", result)
            }
            HasherState::Sha384(h) => {
                let result = h.finalize_reset();
                format!("{:x}", result)
            }
            HasherState::Sha512(h) => {
                let result = h.finalize_reset();
                format!("{:x}", result)
            }
            HasherState::Sha3_256(h) => {
                let result = h.finalize_reset();
                format!("{:x}", result)
            }
            HasherState::Sha3_384(h) => {
                let result = h.finalize_reset();
                format!("{:x}", result)
            }
            HasherState::Sha3_512(h) => {
                let result = h.finalize_reset();
                format!("{:x}", result)
            }
            HasherState::Blake2b(h) => {
                let result = h.finalize_reset();
                format!("{:x}", result)
            }
            HasherState::Blake2s(h) => {
                let result = h.finalize_reset();
                format!("{:x}", result)
            }
        }
    }

    /// Finalize and return raw bytes as a string, then reset the hasher.
    fn digest_and_reset(&mut self) -> Vec<u8> {
        match self {
            HasherState::Md5(h) => h.finalize_reset().to_vec(),
            HasherState::Sha1(h) => h.finalize_reset().to_vec(),
            HasherState::Sha256(h) => h.finalize_reset().to_vec(),
            HasherState::Sha384(h) => h.finalize_reset().to_vec(),
            HasherState::Sha512(h) => h.finalize_reset().to_vec(),
            HasherState::Sha3_256(h) => h.finalize_reset().to_vec(),
            HasherState::Sha3_384(h) => h.finalize_reset().to_vec(),
            HasherState::Sha3_512(h) => h.finalize_reset().to_vec(),
            HasherState::Blake2b(h) => h.finalize_reset().to_vec(),
            HasherState::Blake2s(h) => h.finalize_reset().to_vec(),
        }
    }

    /// Reset the hasher to initial state without finalizing.
    fn reset(&mut self) {
        match self {
            HasherState::Md5(h) => h.reset(),
            HasherState::Sha1(h) => h.reset(),
            HasherState::Sha256(h) => h.reset(),
            HasherState::Sha384(h) => h.reset(),
            HasherState::Sha512(h) => h.reset(),
            HasherState::Sha3_256(h) => h.reset(),
            HasherState::Sha3_384(h) => h.reset(),
            HasherState::Sha3_512(h) => h.reset(),
            HasherState::Blake2b(h) => h.reset(),
            HasherState::Blake2s(h) => h.reset(),
        }
    }
}

fn new_hasher_state(algorithm: &str) -> Result<HasherState, String> {
    match algorithm {
        "md5" => Ok(HasherState::Md5(Md5::new())),
        "sha1" => Ok(HasherState::Sha1(Sha1::new())),
        "sha256" => Ok(HasherState::Sha256(Sha256::new())),
        "sha384" => Ok(HasherState::Sha384(Sha384::new())),
        "sha512" => Ok(HasherState::Sha512(Sha512::new())),
        "sha3-256" => Ok(HasherState::Sha3_256(Sha3_256::new())),
        "sha3-384" => Ok(HasherState::Sha3_384(Sha3_384::new())),
        "sha3-512" => Ok(HasherState::Sha3_512(Sha3_512::new())),
        "blake2b" => Ok(HasherState::Blake2b(Blake2b512::new())),
        "blake2s" => Ok(HasherState::Blake2s(Blake2s256::new())),
        _ => Err(format!(
            "Hasher_new: unsupported algorithm '{}'. Supported: md5, sha1, sha256, sha384, sha512, sha3-256, sha3-384, sha3-512, blake2b, blake2s",
            algorithm
        )),
    }
}

static HASHER_REGISTRY: LazyLock<StdMutex<HashMap<i64, HasherState>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));
static HASHER_NEXT_HANDLE: AtomicI64 = AtomicI64::new(1);

pub(crate) fn native_hasher_new(args: &[Value]) -> Result<Value, String> {
    let algorithm = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("Hasher_new: expected a String algorithm argument".to_string()),
    };
    let state = new_hasher_state(algorithm)?;
    let handle = HASHER_NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
    let mut registry = HASHER_REGISTRY.lock().unwrap();
    registry.insert(handle, state);
    Ok(Value::Long(handle))
}

pub(crate) fn native_hasher_update(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Hasher_update: expected an Int/Long handle as first argument".to_string()),
    };
    let data = match args.get(1) {
        Some(Value::String(s)) => s.as_bytes().to_vec(),
        _ => return Err("Hasher_update: expected a String data argument".to_string()),
    };
    let mut registry = HASHER_REGISTRY.lock().unwrap();
    let hasher = registry
        .get_mut(&handle)
        .ok_or_else(|| "Hasher_update: invalid handle".to_string())?;
    hasher.update(&data);
    Ok(Value::Null)
}

pub(crate) fn native_hasher_digest(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Hasher_digest: expected an Int/Long handle".to_string()),
    };
    let mut registry = HASHER_REGISTRY.lock().unwrap();
    let hasher = registry
        .get_mut(&handle)
        .ok_or_else(|| "Hasher_digest: invalid handle".to_string())?;
    let bytes = hasher.digest_and_reset();
    // Return raw bytes as a string (each byte as a char)
    let s: String = bytes.iter().map(|&b| b as char).collect();
    Ok(Value::String(Rc::new(s)))
}

pub(crate) fn native_hasher_hex_digest(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Hasher_hexDigest: expected an Int/Long handle".to_string()),
    };
    let mut registry = HASHER_REGISTRY.lock().unwrap();
    let hasher = registry
        .get_mut(&handle)
        .ok_or_else(|| "Hasher_hexDigest: invalid handle".to_string())?;
    let hex = hasher.hex_digest_and_reset();
    Ok(Value::String(Rc::new(hex)))
}

pub(crate) fn native_hasher_reset(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Hasher_reset: expected an Int/Long handle".to_string()),
    };
    let mut registry = HASHER_REGISTRY.lock().unwrap();
    let hasher = registry
        .get_mut(&handle)
        .ok_or_else(|| "Hasher_reset: invalid handle".to_string())?;
    hasher.reset();
    Ok(Value::Null)
}
