// Titrate Alpha 0.2 – bytecode virtual machine: json natives
// Precision in every step – richie-rich90454, 2026

use super::super::super::value::Value;
use std::rc::Rc;
use std::collections::HashMap;

pub(crate) fn native_json_parse(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Json_parse: expected 1 argument (json string)".to_string());
    }
    let json_str = match &args[0] {
        Value::String(s) => s.as_str().trim().to_string(),
        _ => return Err("Json_parse: expected String argument".to_string()),
    };
    json_parse_value(&json_str).map(|(v, _)| v)
}

pub(crate) fn native_json_stringify(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Json_stringify: expected 1 argument (value)".to_string());
    }
    Ok(Value::String(Rc::new(json_stringify_value(&args[0]))))
}

pub(crate) fn json_stringify_value(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Int(n) => n.to_string(),
        Value::Long(n) => n.to_string(),
        Value::Double(f) => {
            if f.is_nan() || f.is_infinite() {
                "null".to_string()
            } else {
                let s = format!("{}", f);
                // Ensure decimal point for whole-number doubles
                if !s.contains('.') && !s.contains('e') && !s.contains('E') {
                    format!("{}.0", s)
                } else {
                    s
                }
            }
        }
        Value::Float(f) => {
            if f.is_nan() || f.is_infinite() {
                "null".to_string()
            } else {
                let s = format!("{}", f);
                if !s.contains('.') && !s.contains('e') && !s.contains('E') {
                    format!("{}.0", s)
                } else {
                    s
                }
            }
        }
        Value::String(s) => {
            let escaped = s
                .replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n")
                .replace('\r', "\\r")
                .replace('\t', "\\t");
            format!("\"{}\"", escaped)
        }
        Value::Array { elements } => {
            let items: Vec<String> = elements.iter().map(json_stringify_value).collect();
            format!("[{}]", items.join(", "))
        }
        Value::ClassInstance { fields, .. } => {
            let borrowed = fields.borrow();
            // Check if this is a HashMap representation (has _keys and _values)
            if let (Some(Value::Array { elements: keys }), Some(Value::Array { elements: values })) =
                (borrowed.get("_keys"), borrowed.get("_values"))
            {
                let items: Vec<String> = keys.iter().zip(values.iter())
                    .map(|(k, v)| format!("{}: {}", json_stringify_value(k), json_stringify_value(v)))
                    .collect();
                format!("{{{}}}", items.join(", "))
            } else {
                // Generic class instance: serialize all fields
                let items: Vec<String> = borrowed.iter()
                    .map(|(k, v)| format!("\"{}\": {}", k, json_stringify_value(v)))
                    .collect();
                format!("{{{}}}", items.join(", "))
            }
        }
        Value::Void => "null".to_string(),
        // Fallback for other types
        _ => format!("\"{}\"", value_to_string(value)),
    }
}

pub(crate) fn value_to_string(value: &Value) -> String {
    match value {
        Value::Void => "void".to_string(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Byte(v) => format!("{}b", v),
        Value::Short(v) => format!("{}s", v),
        Value::Int(v) => v.to_string(),
        Value::Long(v) => format!("{}L", v),
        Value::Vast(v) => format!("{}V", v),
        Value::Uvast(v) => format!("{}U", v),
        Value::Float(v) => format!("{}f", v),
        Value::Double(v) => format!("{}d", v),
        Value::Half(v) => format!("{}h", v),
        Value::Quad(v) => format!("{}q", v),
        Value::Char(c) => c.to_string(),
        Value::String(s) => s.to_string(),
        Value::Array { elements } => {
            let items: Vec<String> = elements.iter().map(value_to_string).collect();
            format!("[{}]", items.join(", "))
        }
        Value::ClassInstance { class_name, fields, .. } => {
            let borrowed = fields.borrow();
            let items: Vec<String> = borrowed.iter()
                .map(|(k, v)| format!("{}: {}", k, value_to_string(v)))
                .collect();
            format!("{}({})", class_name, items.join(", "))
        }
        _ => format!("{:?}", value),
    }
}

/// Simple recursive-descent JSON parser.
/// Returns (Value, remaining_string) on success.
pub(crate) fn json_parse_value(input: &str) -> Result<(Value, &str), String> {
    let input = input.trim_start();
    if input.is_empty() {
        return Err("Json_parse: unexpected end of input".to_string());
    }
    if let Some(stripped) = input.strip_prefix("null") {
        return Ok((Value::Null, stripped));
    }
    if let Some(stripped) = input.strip_prefix("true") {
        return Ok((Value::Bool(true), stripped));
    }
    if let Some(stripped) = input.strip_prefix("false") {
        return Ok((Value::Bool(false), stripped));
    }
    if input.starts_with('"') {
        return json_parse_string(input);
    }
    if input.starts_with('[') {
        return json_parse_array(input);
    }
    if input.starts_with('{') {
        return json_parse_object(input);
    }
    // Number
    json_parse_number(input)
}

pub(crate) fn json_parse_string(input: &str) -> Result<(Value, &str), String> {
    let bytes = input.as_bytes();
    if bytes.is_empty() || bytes[0] != b'"' {
        return Err("Json_parse: expected '\"'".to_string());
    }
    let mut i = 1;
    let mut result = String::new();
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => {
                if i + 1 >= bytes.len() {
                    return Err("Json_parse: unexpected end of string escape".to_string());
                }
                i += 1;
                match bytes[i] {
                    b'"' => result.push('"'),
                    b'\\' => result.push('\\'),
                    b'/' => result.push('/'),
                    b'n' => result.push('\n'),
                    b'r' => result.push('\r'),
                    b't' => result.push('\t'),
                    b'u' => {
                        // Parse 4 hex digits for Unicode escape
                        if i + 4 >= bytes.len() {
                            return Err("JSON: incomplete unicode escape".to_string());
                        }
                        let hex: String = bytes[i+1..i+5].iter().map(|&b| b as char).collect();
                        match u32::from_str_radix(&hex, 16) {
                            Ok(code_point) => {
                                if let Some(ch) = char::from_u32(code_point) {
                                    result.push(ch);
                                } else {
                                    return Err("JSON: invalid unicode code point".to_string());
                                }
                                i += 4;
                            }
                            Err(_) => return Err("JSON: invalid unicode escape".to_string()),
                        }
                    }
                    _ => result.push(bytes[i] as char),
                }
                i += 1;
            }
            b'"' => {
                return Ok((Value::String(Rc::new(result)), &input[i + 1..]));
            }
            b => {
                result.push(b as char);
                i += 1;
            }
        }
    }
    Err("Json_parse: unterminated string".to_string())
}

pub(crate) fn json_parse_number(input: &str) -> Result<(Value, &str), String> {
    let mut i = 0;
    let bytes = input.as_bytes();
    let start = 0;
    if i < bytes.len() && (bytes[i] == b'-' || bytes[i] == b'+') {
        i += 1;
    }
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    let is_float = if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        true
    } else {
        false
    };
    // Handle exponent
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        i += 1;
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
    }
    let num_str = &input[start..i];
    if is_float {
        match num_str.parse::<f64>() {
            Ok(f) => Ok((Value::Double(f), &input[i..])),
            Err(_) => Err(format!("Json_parse: invalid number '{}'", num_str)),
        }
    } else {
        match num_str.parse::<i64>() {
            Ok(n) => Ok((Value::Long(n), &input[i..])),
            Err(_) => Err(format!("Json_parse: invalid number '{}'", num_str)),
        }
    }
}

pub(crate) fn json_parse_array(input: &str) -> Result<(Value, &str), String> {
    let mut rest = input[1..].trim_start(); // skip '['
    let mut elements = Vec::new();
    if let Some(stripped) = rest.strip_prefix(']') {
        return Ok((Value::Array { elements }, stripped));
    }
    loop {
        let (val, remaining) = json_parse_value(rest)?;
        elements.push(val);
        rest = remaining.trim_start();
        if let Some(stripped) = rest.strip_prefix(']') {
            return Ok((Value::Array { elements }, stripped));
        }
        if !rest.starts_with(',') {
            return Err("Json_parse: expected ',' or ']' in array".to_string());
        }
        rest = rest[1..].trim_start();
    }
}

pub(crate) fn json_parse_object(input: &str) -> Result<(Value, &str), String> {
    let mut rest = input[1..].trim_start(); // skip '{'
    let mut keys = Vec::new();
    let mut values = Vec::new();
    if let Some(stripped) = rest.strip_prefix('}') {
        let mut fields = HashMap::new();
        fields.insert("_keys".to_string(), Value::Array { elements: keys });
        fields.insert("_values".to_string(), Value::Array { elements: values });
        return Ok((Value::ClassInstance {
            class_name: "HashMap".to_string(),
            fields: Rc::new(std::cell::RefCell::new(fields)),
            vtable: HashMap::new(),
        }, stripped));
    }
    loop {
        // Parse key (must be a string)
        let (key_val, remaining) = json_parse_value(rest)?;
        let key_str = match &key_val {
            Value::String(s) => s.as_str().to_string(),
            _ => return Err("Json_parse: object key must be a string".to_string()),
        };
        rest = remaining.trim_start();
        if !rest.starts_with(':') {
            return Err("Json_parse: expected ':' in object".to_string());
        }
        rest = rest[1..].trim_start();
        // Parse value
        let (val, remaining) = json_parse_value(rest)?;
        keys.push(Value::String(Rc::new(key_str)));
        values.push(val);
        rest = remaining.trim_start();
        if let Some(stripped) = rest.strip_prefix('}') {
            let mut fields = HashMap::new();
            fields.insert("_keys".to_string(), Value::Array { elements: keys });
            fields.insert("_values".to_string(), Value::Array { elements: values });
            return Ok((Value::ClassInstance {
                class_name: "HashMap".to_string(),
                fields: Rc::new(std::cell::RefCell::new(fields)),
                vtable: HashMap::new(),
            }, stripped));
        }
        if !rest.starts_with(',') {
            return Err("Json_parse: expected ',' or '}' in object".to_string());
        }
        rest = rest[1..].trim_start();
    }
}
