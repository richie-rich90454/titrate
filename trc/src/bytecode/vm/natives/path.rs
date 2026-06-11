// Titrate Alpha 0.2 – bytecode virtual machine: path natives
// Precision in every step – richie-rich90454, 2026

use super::super::super::value::Value;
use std::rc::Rc;

pub(crate) fn native_path_join(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Path_join: expected 2 arguments (path, other)".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::String(path), Value::String(other)) => {
            let joined = std::path::Path::new(path.as_str())
                .join(other.as_str())
                .to_string_lossy()
                .to_string();
            Ok(Value::String(Rc::new(joined)))
        }
        _ => Err("Path_join: expected (String, String)".to_string()),
    }
}

pub(crate) fn native_path_exists(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Path_exists: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => Ok(Value::Bool(std::path::Path::new(path.as_str()).exists())),
        _ => Err("Path_exists: expected String argument".to_string()),
    }
}

pub(crate) fn native_path_is_file(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Path_isFile: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => Ok(Value::Bool(std::path::Path::new(path.as_str()).is_file())),
        _ => Err("Path_isFile: expected String argument".to_string()),
    }
}

pub(crate) fn native_path_is_dir(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Path_isDir: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => Ok(Value::Bool(std::path::Path::new(path.as_str()).is_dir())),
        _ => Err("Path_isDir: expected String argument".to_string()),
    }
}

pub(crate) fn native_path_basename(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Path_basename: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => {
            let name = std::path::Path::new(path.as_str())
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            Ok(Value::String(Rc::new(name)))
        }
        _ => Err("Path_basename: expected String argument".to_string()),
    }
}

pub(crate) fn native_path_dirname(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Path_dirname: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => {
            let dir = std::path::Path::new(path.as_str())
                .parent()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            Ok(Value::String(Rc::new(dir)))
        }
        _ => Err("Path_dirname: expected String argument".to_string()),
    }
}

pub(crate) fn native_path_extension(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Path_extension: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => {
            let ext = std::path::Path::new(path.as_str())
                .extension()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            Ok(Value::String(Rc::new(ext)))
        }
        _ => Err("Path_extension: expected String argument".to_string()),
    }
}
