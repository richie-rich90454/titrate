// Titrate Alpha 0.2 – bytecode virtual machine: core natives
// Precision in every step – richie-rich90454, 2026

use super::super::super::value::Value;
use std::rc::Rc;

pub(crate) fn native_println(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        // This shouldn't happen in normal use, but handle gracefully
        return Ok(Value::Void);
    }
    // Note: the actual output capture is done by the VM's output field.
    // For the native function, we just return Void. The VM's CALL_NATIVE
    // handler for println should capture output. However, since native
    // functions are called generically, we need a different approach.
    // The println native will be handled specially in call_native_fn.
    Ok(Value::Void)
}

pub(crate) fn native_to_string(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("toString: expected 1 argument".to_string());
    }
    Ok(Value::String(Rc::new(args[0].display_string())))
}

pub(crate) fn native_parse_int(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("parseInt: expected 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => match s.parse::<i64>() {
            Ok(n) => Ok(Value::ResultOk(Box::new(Value::Long(n)))),
            Err(_) => Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
                format!("Invalid integer: {}", s),
            ))))),
        },
        _ => Err(format!("parseInt: expected String, got {:?}", args[0])),
    }
}

pub(crate) fn native_ok(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Ok: expected 1 argument".to_string());
    }
    Ok(Value::ResultOk(Box::new(args[0].clone())))
}

pub(crate) fn native_err(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Err: expected 1 argument".to_string());
    }
    Ok(Value::ResultErr(Box::new(args[0].clone())))
}

pub(crate) fn native_string_split(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("String_split: expected 2 arguments (string, delimiter)".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::String(s), Value::String(delim)) => {
            let parts: Vec<Value> = s.split(delim.as_str())
                .map(|part| Value::String(Rc::new(part.to_string())))
                .collect();
            Ok(Value::Array { elements: parts })
        }
        (Value::String(s), Value::Char(delim)) => {
            let parts: Vec<Value> = s.split(*delim)
                .map(|part| Value::String(Rc::new(part.to_string())))
                .collect();
            Ok(Value::Array { elements: parts })
        }
        _ => Err("String_split: expected (String, String) or (String, Char)".to_string()),
    }
}

pub(crate) fn native_integer_parse_or(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Integer_parseOr: expected 2 arguments (string, default)".to_string());
    }
    match &args[0] {
        Value::String(s) => match s.parse::<i32>() {
            Ok(n) => Ok(Value::Int(n)),
            Err(_) => Ok(args[1].clone()),
        },
        _ => Ok(args[1].clone()),
    }
}

pub(crate) fn native_string_trim(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("String_trim: expected 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => Ok(Value::String(Rc::new(s.trim().to_string()))),
        _ => Err("String_trim: expected String argument".to_string()),
    }
}

pub(crate) fn native_string_length(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("String_length: expected 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => Ok(Value::Int(s.chars().count() as i32)),
        _ => Err("String_length: expected String argument".to_string()),
    }
}

pub(crate) fn native_double_parse_double(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Double_parseDouble: expected 1 argument (string)".to_string());
    }
    let s = match &args[0] {
        Value::String(s) => s.as_str().trim().to_string(),
        _ => return Err("Double_parseDouble: expected String argument".to_string()),
    };
    s.parse::<f64>()
        .map(Value::Double)
        .map_err(|e| format!("Double_parseDouble: cannot parse '{}': {}", s, e))
}

pub(crate) fn native_long_parse_long(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Long_parseLong: expected 1 argument (string)".to_string());
    }
    let s = match &args[0] {
        Value::String(s) => s.as_str().trim().to_string(),
        _ => return Err("Long_parseLong: expected String argument".to_string()),
    };
    s.parse::<i64>()
        .map(Value::Long)
        .map_err(|e| format!("Long_parseLong: cannot parse '{}': {}", s, e))
}

pub(crate) fn native_type_name_of(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("TypeName_of: expected 1 argument".to_string());
    }
    let name = match &args[0] {
        Value::ClassInstance { class_name, .. } => class_name.clone(),
        Value::EnumInstance { enum_name, .. } => enum_name.clone(),
        other => other.type_name().to_string(),
    };
    Ok(Value::String(Rc::new(name)))
}
