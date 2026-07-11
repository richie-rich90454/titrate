// Titrate Alpha 0.2 – bytecode virtual machine: string natives
// Precision in every step – richie-rich90454, 2026

use super::super::super::value::Value;
use std::rc::Rc;

pub(crate) fn native_string_trim_start(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("String_trimStart: expected 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => Ok(Value::String(Rc::new(s.trim_start().to_string()))),
        _ => Err("String_trimStart: expected String argument".to_string()),
    }
}

pub(crate) fn native_string_trim_end(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("String_trimEnd: expected 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => Ok(Value::String(Rc::new(s.trim_end().to_string()))),
        _ => Err("String_trimEnd: expected String argument".to_string()),
    }
}

pub(crate) fn native_string_starts_with(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("String_startsWith: expected 2 arguments (string, prefix)".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::String(s), Value::String(prefix)) => Ok(Value::Bool(s.starts_with(prefix.as_str()))),
        _ => Err("String_startsWith: expected (String, String)".to_string()),
    }
}

pub(crate) fn native_string_ends_with(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("String_endsWith: expected 2 arguments (string, suffix)".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::String(s), Value::String(suffix)) => Ok(Value::Bool(s.ends_with(suffix.as_str()))),
        _ => Err("String_endsWith: expected (String, String)".to_string()),
    }
}

pub(crate) fn native_string_pad_left(args: &[Value]) -> Result<Value, String> {
    if args.len() < 3 {
        return Err("String_padLeft: expected 3 arguments (string, width, char/string)".to_string());
    }
    match (&args[0], &args[1], &args[2]) {
        (Value::String(s), Value::Int(width), Value::Char(pad_char)) => {
            let char_count = s.chars().count();
            let pad_count = (*width as usize).saturating_sub(char_count);
            let padding: String = std::iter::repeat_n(*pad_char, pad_count).collect();
            let padded = format!("{}{}", padding, s.as_str());
            Ok(Value::String(Rc::new(padded)))
        }
        (Value::String(s), Value::Long(width), Value::Char(pad_char)) => {
            let char_count = s.chars().count();
            let pad_count = (*width as usize).saturating_sub(char_count);
            let padding: String = std::iter::repeat_n(*pad_char, pad_count).collect();
            let padded = format!("{}{}", padding, s.as_str());
            Ok(Value::String(Rc::new(padded)))
        }
        (Value::String(s), Value::Int(width), Value::String(pad_str)) => {
            let pad_char = pad_str.chars().next().unwrap_or(' ');
            let char_count = s.chars().count();
            let pad_count = (*width as usize).saturating_sub(char_count);
            let padding: String = std::iter::repeat_n(pad_char, pad_count).collect();
            let padded = format!("{}{}", padding, s.as_str());
            Ok(Value::String(Rc::new(padded)))
        }
        (Value::String(s), Value::Long(width), Value::String(pad_str)) => {
            let pad_char = pad_str.chars().next().unwrap_or(' ');
            let char_count = s.chars().count();
            let pad_count = (*width as usize).saturating_sub(char_count);
            let padding: String = std::iter::repeat_n(pad_char, pad_count).collect();
            let padded = format!("{}{}", padding, s.as_str());
            Ok(Value::String(Rc::new(padded)))
        }
        _ => Err("String_padLeft: expected (String, Int/Long, Char/String)".to_string()),
    }
}

pub(crate) fn native_string_pad_right(args: &[Value]) -> Result<Value, String> {
    if args.len() < 3 {
        return Err("String_padRight: expected 3 arguments (string, width, char/string)".to_string());
    }
    match (&args[0], &args[1], &args[2]) {
        (Value::String(s), Value::Int(width), Value::Char(pad_char)) => {
            let char_count = s.chars().count();
            let pad_count = (*width as usize).saturating_sub(char_count);
            let padding: String = std::iter::repeat_n(*pad_char, pad_count).collect();
            let padded = format!("{}{}", s.as_str(), padding);
            Ok(Value::String(Rc::new(padded)))
        }
        (Value::String(s), Value::Long(width), Value::Char(pad_char)) => {
            let char_count = s.chars().count();
            let pad_count = (*width as usize).saturating_sub(char_count);
            let padding: String = std::iter::repeat_n(*pad_char, pad_count).collect();
            let padded = format!("{}{}", s.as_str(), padding);
            Ok(Value::String(Rc::new(padded)))
        }
        (Value::String(s), Value::Int(width), Value::String(pad_str)) => {
            let pad_char = pad_str.chars().next().unwrap_or(' ');
            let char_count = s.chars().count();
            let pad_count = (*width as usize).saturating_sub(char_count);
            let padding: String = std::iter::repeat_n(pad_char, pad_count).collect();
            let padded = format!("{}{}", s.as_str(), padding);
            Ok(Value::String(Rc::new(padded)))
        }
        (Value::String(s), Value::Long(width), Value::String(pad_str)) => {
            let pad_char = pad_str.chars().next().unwrap_or(' ');
            let char_count = s.chars().count();
            let pad_count = (*width as usize).saturating_sub(char_count);
            let padding: String = std::iter::repeat_n(pad_char, pad_count).collect();
            let padded = format!("{}{}", s.as_str(), padding);
            Ok(Value::String(Rc::new(padded)))
        }
        _ => Err("String_padRight: expected (String, Int/Long, Char/String)".to_string()),
    }
}

pub(crate) fn native_string_to_uppercase(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::String(s)) => Ok(Value::String(Rc::new(s.to_uppercase()))),
        _ => Err("String_toUpperCase: expected a String argument".to_string()),
    }
}

pub(crate) fn native_string_to_lower_case(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::String(s)) => Ok(Value::String(Rc::new(s.to_lowercase()))),
        _ => Err("String_toLowerCase: expected a String argument".to_string()),
    }
}

pub(crate) fn native_string_replace(args: &[Value]) -> Result<Value, String> {
    if args.len() < 3 {
        return Err("String_replace: expected 3 arguments (input, target, replacement)".to_string());
    }
    let input = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("String_replace: expected String input".to_string()),
    };
    let target = match &args[1] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("String_replace: expected String target".to_string()),
    };
    let replacement = match &args[2] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("String_replace: expected String replacement".to_string()),
    };
    Ok(Value::String(Rc::new(input.replace(&target, &replacement))))
}

pub(crate) fn native_string_from_char_code(args: &[Value]) -> Result<Value, String> {
    let code = match args.first() {
        Some(Value::Int(c)) => *c as u32,
        Some(Value::Long(c)) => *c as u32,
        _ => return Err("String_fromCharCode: expected an Int code point".to_string()),
    };
    let s = char::from_u32(code).unwrap_or('\0').to_string();
    Ok(Value::String(Rc::new(s)))
}

pub(crate) fn native_string_char_at(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("String_charAt: expected 2 arguments (string, index)".to_string());
    }
    let s = match &args[0] {
        Value::String(s) => s.as_str(),
        _ => return Err("String_charAt: expected String argument".to_string()),
    };
    let idx = args[1].to_i64().unwrap_or(0);
    if idx < 0 {
        return Err(format!("String_charAt: index {} out of bounds", idx));
    }
    match s.chars().nth(idx as usize) {
        Some(c) => Ok(Value::String(Rc::new(c.to_string()))),
        None => Err(format!(
            "String_charAt: index {} out of bounds for string of length {}",
            idx,
            s.chars().count()
        )),
    }
}
