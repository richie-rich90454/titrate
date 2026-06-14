// Titrate Alpha 0.2 – bytecode virtual machine: directory natives
// Precision in every step – richie-rich90454, 2026

use super::super::super::value::Value;
use std::rc::Rc;

pub(crate) fn native_dir_list(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Dir_list: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => {
            let entries: Vec<Value> = std::fs::read_dir(path.as_str())
                .map_err(|e| format!("Dir_list: {}", e))?
                .filter_map(|e| e.ok())
                .map(|e| Value::String(Rc::new(e.file_name().to_string_lossy().to_string())))
                .collect();
            Ok(Value::Array { elements: entries })
        }
        _ => Err("Dir_list: expected String argument".to_string()),
    }
}

pub(crate) fn native_dir_create(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Dir_create: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => match std::fs::create_dir_all(path.as_str()) {
            Ok(()) => Ok(Value::Bool(true)),
            Err(_) => Ok(Value::Bool(false)),
        },
        _ => Err("Dir_create: expected String argument".to_string()),
    }
}

pub(crate) fn native_dir_remove(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Dir_remove: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => match std::fs::remove_dir_all(path.as_str()) {
            Ok(()) => Ok(Value::Bool(true)),
            Err(_) => Ok(Value::Bool(false)),
        },
        _ => Err("Dir_remove: expected String argument".to_string()),
    }
}

pub(crate) fn native_dir_remove_tree(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Dir_removeTree: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => match std::fs::remove_dir_all(path.as_str()) {
            Ok(()) => Ok(Value::Bool(true)),
            Err(_) => Ok(Value::Bool(false)),
        },
        _ => Err("Dir_removeTree: expected String argument".to_string()),
    }
}

pub(crate) fn native_dir_walk(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Dir_walk: expected 1 argument (path)".to_string());
    }
    let path = match args.first() {
        Some(Value::String(s)) => s.as_str().to_string(),
        _ => return Err("Dir_walk: expected String path".to_string()),
    };
    let mut results = Vec::new();
    fn walk_dir(dir: &std::path::Path, results: &mut Vec<Value>) -> Result<(), String> {
        let entries = std::fs::read_dir(dir)
            .map_err(|e| format!("Dir_walk: cannot read '{}': {}", dir.display(), e))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("Dir_walk: {}", e))?;
            let path = entry.path();
            results.push(Value::String(Rc::new(path.to_string_lossy().to_string())));
            if path.is_dir() {
                walk_dir(&path, results)?;
            }
        }
        Ok(())
    }
    walk_dir(std::path::Path::new(&path), &mut results)?;
    Ok(Value::Array { elements: results })
}

pub(crate) fn native_dir_copy(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Dir_copy: expected 2 arguments (src, dst)".to_string());
    }
    let src = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Dir_copy: expected String source path".to_string()),
    };
    let dst = match &args[1] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Dir_copy: expected String destination path".to_string()),
    };
    fn copy_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<(), String> {
        std::fs::create_dir_all(dst)
            .map_err(|e| format!("Dir_copy: cannot create '{}': {}", dst.display(), e))?;
        for entry in std::fs::read_dir(src)
            .map_err(|e| format!("Dir_copy: cannot read '{}': {}", src.display(), e))?
        {
            let entry = entry.map_err(|e| format!("Dir_copy: {}", e))?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            if src_path.is_dir() {
                copy_recursive(&src_path, &dst_path)?;
            } else {
                std::fs::copy(&src_path, &dst_path)
                    .map_err(|e| format!("Dir_copy: cannot copy '{}': {}", src_path.display(), e))?;
            }
        }
        Ok(())
    }
    copy_recursive(std::path::Path::new(&src), std::path::Path::new(&dst))?;
    Ok(Value::Void)
}

pub(crate) fn native_dir_move(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Dir_move: expected 2 arguments (src, dst)".to_string());
    }
    let src = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Dir_move: expected String source path".to_string()),
    };
    let dst = match &args[1] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Dir_move: expected String destination path".to_string()),
    };
    std::fs::rename(&src, &dst)
        .map_err(|e| format!("Dir_move: cannot move '{}' to '{}': {}", src, dst, e))?;
    Ok(Value::Void)
}
