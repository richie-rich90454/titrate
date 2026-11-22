// Titrate Alpha 0.2 – bytecode virtual machine: directory natives
// Precision in every step – richie-rich90454, 2026

use super::super::super::value::Value;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::rc::Rc;

const MAX_WALK_DEPTH: usize = 64;
const MAX_WALK_ENTRIES: usize = 100_000;

/// Recurse through real directories only: symlinks are skipped and canonical
/// paths are tracked so a link cycle cannot make the walk loop forever.
fn walk_dir(
    dir: &Path,
    results: &mut Vec<Value>,
    visited: &mut HashSet<PathBuf>,
    depth: usize,
) -> Result<(), String> {
    if depth > MAX_WALK_DEPTH {
        return Err("Dir_walk: maximum nesting depth exceeded".to_string());
    }
    let canonical = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    if !visited.insert(canonical) {
        return Ok(());
    }
    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("Dir_walk: cannot read '{}': {}", dir.display(), e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("Dir_walk: {}", e))?;
        let path = entry.path();
        results.push(Value::String(Rc::new(path.to_string_lossy().to_string())));
        if results.len() > MAX_WALK_ENTRIES {
            return Err("Dir_walk: too many entries".to_string());
        }
        let meta = match std::fs::symlink_metadata(&path) {
            Ok(meta) => meta,
            Err(_) => continue,
        };
        if meta.file_type().is_symlink() {
            continue;
        }
        if meta.is_dir() {
            walk_dir(&path, results, visited, depth + 1)?;
        }
    }
    Ok(())
}

pub(crate) fn native_dir_list(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Dir_list: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => {
            let resolved = super::resolve_path(path.as_str());
            let entries: Vec<Value> = std::fs::read_dir(&resolved)
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
        Value::String(path) => {
            let resolved = super::resolve_path(path.as_str());
            match std::fs::create_dir_all(&resolved) {
                Ok(()) => Ok(Value::Bool(true)),
                Err(_) => Ok(Value::Bool(false)),
            }
        }
        _ => Err("Dir_create: expected String argument".to_string()),
    }
}

pub(crate) fn native_dir_remove(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Dir_remove: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => {
            let resolved = super::resolve_path(path.as_str());
            // Try remove_file first (handles regular files), then fall back
            // to remove_dir (handles empty directories). This makes Dir_remove
            // work for both files and directories, matching the comment in
            // File.tr's delete() method.
            if std::fs::remove_file(&resolved).is_ok() || std::fs::remove_dir(&resolved).is_ok() {
                Ok(Value::Bool(true))
            } else {
                Ok(Value::Bool(false))
            }
        }
        _ => Err("Dir_remove: expected String argument".to_string()),
    }
}

pub(crate) fn native_dir_remove_tree(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Dir_removeTree: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => {
            let resolved = super::resolve_path(path.as_str());
            match std::fs::remove_dir_all(&resolved) {
                Ok(()) => Ok(Value::Bool(true)),
                Err(_) => Ok(Value::Bool(false)),
            }
        }
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
    let resolved = super::resolve_path(&path);
    let mut results = Vec::new();
    walk_dir(&resolved, &mut results, &mut HashSet::new(), 0)?;
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
    let src_resolved = super::resolve_path(&src);
    let dst_resolved = super::resolve_path(&dst);
    fn copy_recursive(
        src: &Path,
        dst: &Path,
        visited: &mut HashSet<PathBuf>,
        depth: usize,
    ) -> Result<(), String> {
        if depth > MAX_WALK_DEPTH {
            return Err("Dir_copy: maximum nesting depth exceeded".to_string());
        }
        let canonical = std::fs::canonicalize(src).unwrap_or_else(|_| src.to_path_buf());
        if !visited.insert(canonical) {
            return Ok(());
        }
        std::fs::create_dir_all(dst)
            .map_err(|e| format!("Dir_copy: cannot create '{}': {}", dst.display(), e))?;
        for entry in std::fs::read_dir(src)
            .map_err(|e| format!("Dir_copy: cannot read '{}': {}", src.display(), e))?
        {
            let entry = entry.map_err(|e| format!("Dir_copy: {}", e))?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            let meta = match std::fs::symlink_metadata(&src_path) {
                Ok(meta) => meta,
                Err(_) => continue,
            };
            if meta.file_type().is_symlink() {
                continue;
            }
            if meta.is_dir() {
                copy_recursive(&src_path, &dst_path, visited, depth + 1)?;
            } else {
                std::fs::copy(&src_path, &dst_path)
                    .map_err(|e| format!("Dir_copy: cannot copy '{}': {}", src_path.display(), e))?;
            }
        }
        Ok(())
    }
    copy_recursive(&src_resolved, &dst_resolved, &mut HashSet::new(), 0)?;
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
    let src_resolved = super::resolve_path(&src);
    let dst_resolved = super::resolve_path(&dst);
    std::fs::rename(&src_resolved, &dst_resolved)
        .map_err(|e| format!("Dir_move: cannot move '{}' to '{}': {}", src, dst, e))?;
    Ok(Value::Void)
}
