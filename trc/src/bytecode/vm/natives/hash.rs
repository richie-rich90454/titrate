// Titrate Alpha 0.2 – bytecode virtual machine: hash natives
// Precision in every step – richie-rich90454, 2026

use super::super::super::value::Value;
use std::rc::Rc;
use md5::{Digest, Md5};
use sha1::Sha1;
use sha2::{Sha256, Sha384, Sha512};
use sha3::Sha3_256;
use sha3::Sha3_384;
use sha3::Sha3_512;
use blake2::{Blake2b512, Blake2s256};

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
