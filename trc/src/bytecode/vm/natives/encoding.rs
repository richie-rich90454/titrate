// Titrate Alpha 0.2 – bytecode virtual machine: encoding natives
// Precision in every step – richie-rich90454, 2026

use super::super::super::value::Value;
use std::rc::Rc;
use base64::{Engine as _, engine::general_purpose};
use percent_encoding::{utf8_percent_encode, percent_decode_str, NON_ALPHANUMERIC};

pub(crate) fn native_base64_encode(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::String(s)) => {
            let encoded = general_purpose::STANDARD.encode(s.as_bytes());
            Ok(Value::String(Rc::new(encoded)))
        }
        _ => Err("Base64_encode: expected a String argument".to_string()),
    }
}

pub(crate) fn native_base64_decode(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::String(s)) => {
            general_purpose::STANDARD
                .decode(s.as_str())
                .map(|bytes| Value::String(Rc::new(String::from_utf8_lossy(&bytes).to_string())))
                .map_err(|e| format!("Base64_decode: {}", e))
        }
        _ => Err("Base64_decode: expected a String argument".to_string()),
    }
}

pub(crate) fn native_hex_encode(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::String(s)) => {
            let encoded: String = s.as_bytes().iter().map(|b| format!("{:02x}", b)).collect();
            Ok(Value::String(Rc::new(encoded)))
        }
        _ => Err("Hex_encode: expected a String argument".to_string()),
    }
}

pub(crate) fn native_hex_decode(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::String(s)) => {
            let bytes: Result<Vec<u8>, _> = (0..s.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
                .collect();
            bytes
                .map(|b| Value::String(Rc::new(String::from_utf8_lossy(&b).to_string())))
                .map_err(|e| format!("Hex_decode: {}", e))
        }
        _ => Err("Hex_decode: expected a String argument".to_string()),
    }
}

pub(crate) fn native_url_encode(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::String(s)) => {
            let encoded = utf8_percent_encode(s, NON_ALPHANUMERIC).to_string();
            Ok(Value::String(Rc::new(encoded)))
        }
        _ => Err("Url_encode: expected a String argument".to_string()),
    }
}

pub(crate) fn native_url_decode(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::String(s)) => {
            percent_decode_str(s)
                .decode_utf8()
                .map(|cow| Value::String(Rc::new(cow.to_string())))
                .map_err(|e| format!("Url_decode: {}", e))
        }
        _ => Err("Url_decode: expected a String argument".to_string()),
    }
}
