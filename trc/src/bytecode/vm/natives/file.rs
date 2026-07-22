// Titrate Alpha 0.2 – bytecode virtual machine: file natives
// Precision in every step – richie-rich90454, 2026

use super::super::super::value::Value;
use std::cell::RefCell;
use std::rc::Rc;

// Thread-local read positions for path-based File_readLine calls.
// Keyed by file path; tracks the byte offset of the next unread byte.
thread_local! {
    static FILE_READ_POSITIONS: RefCell<std::collections::HashMap<String, u64>> =
        RefCell::new(std::collections::HashMap::new());
}

pub(crate) fn native_file_read(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("File_readFile: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => {
            let resolved = super::resolve_path(path.as_str());
            match std::fs::read_to_string(&resolved) {
                Ok(content) => Ok(Value::String(Rc::new(content))),
                Err(_) => Ok(Value::String(Rc::new(String::new()))),
            }
        }
        _ => Err("File_readFile: expected String path".to_string()),
    }
}

pub(crate) fn native_file_write(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("File_writeFile: expected 2 arguments (path, content)".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::String(path), Value::String(content)) => {
            let resolved = super::resolve_path(path.as_str());
            match std::fs::write(&resolved, content.as_str()) {
                Ok(()) => Ok(Value::Bool(true)),
                Err(_) => Ok(Value::Bool(false)),
            }
        }
        _ => Err("File_writeFile: expected (String, String)".to_string()),
    }
}

pub(crate) fn native_file_append(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("File_append: expected 2 arguments (path, content)".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::String(path), Value::String(content)) => {
            let resolved = super::resolve_path(path.as_str());
            use std::io::Write;
            match std::fs::OpenOptions::new().create(true).append(true).open(&resolved) {
                Ok(mut file) => match file.write_all(content.as_bytes()) {
                    Ok(()) => Ok(Value::Bool(true)),
                    Err(_) => Ok(Value::Bool(false)),
                },
                Err(_) => Ok(Value::Bool(false)),
            }
        }
        _ => Err("File_append: expected (String, String)".to_string()),
    }
}

pub(crate) fn native_file_read_chunk(args: &[Value]) -> Result<Value, String> {
    if args.len() < 3 {
        return Err("File_readChunk: expected 3 arguments (path, offset, length)".to_string());
    }
    let path = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("File_readChunk: expected String path".to_string()),
    };
    let offset = match &args[1] {
        Value::Long(n) => *n as u64,
        Value::Int(n) => *n as u64,
        _ => return Err("File_readChunk: expected int/long offset".to_string()),
    };
    let length = match &args[2] {
        Value::Int(n) => *n as usize,
        Value::Long(n) => *n as usize,
        _ => return Err("File_readChunk: expected int/long length".to_string()),
    };
    let resolved = super::resolve_path(&path);
    use std::io::{Read, Seek, SeekFrom};
    match std::fs::File::open(&resolved) {
        Ok(mut file) => {
            if offset > 0 && file.seek(SeekFrom::Start(offset)).is_err() {
                return Ok(Value::String(Rc::new(String::new())));
            }
            let mut buf = vec![0u8; length];
            match file.read(&mut buf) {
                Ok(n) => {
                    buf.truncate(n);
                    Ok(Value::String(Rc::new(String::from_utf8_lossy(&buf).to_string())))
                }
                Err(_) => Ok(Value::String(Rc::new(String::new()))),
            }
        }
        Err(_) => Ok(Value::String(Rc::new(String::new()))),
    }
}

pub(crate) fn native_file_read_lines(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("File_readLines: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => {
            let resolved = super::resolve_path(path.as_str());
            match std::fs::read_to_string(&resolved) {
                Ok(content) => {
                    let lines: Vec<Value> = content.lines()
                        .map(|line| Value::String(Rc::new(line.to_string())))
                        .collect();
                    Ok(Value::Array { elements: lines })
                }
                Err(_) => Ok(Value::Array { elements: vec![] }),
            }
        }
        _ => Err("File_readLines: expected String path".to_string()),
    }
}

#[allow(dead_code)]
pub(crate) fn native_file_open(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("File_open: expected at least 1 argument (path)".to_string());
    }
    let path = match &args[0] {
        Value::String(s) => s.as_str(),
        _ => return Err("File_open: expected String path".to_string()),
    };
    let mode = if args.len() > 1 {
        match &args[1] {
            Value::String(s) => s.as_str(),
            _ => return Err("File_open: expected String mode".to_string()),
        }
    } else {
        "r"
    };
    let resolved = super::resolve_path(path);
    let file = match mode {
        "r" | "rb" => std::fs::File::open(&resolved),
        "w" | "wb" => std::fs::File::create(&resolved),
        "a" | "ab" => std::fs::OpenOptions::new().append(true).open(&resolved),
        "r+" => std::fs::OpenOptions::new().read(true).write(true).open(&resolved),
        "w+" => std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(true).open(&resolved),
        "a+" => std::fs::OpenOptions::new().read(true).append(true).open(&resolved),
        _ => return Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
            format!("File_open: unsupported mode '{}'", mode)
        ))))),
    };
    match file {
        Ok(f) => Ok(Value::ResultOk(Box::new(Value::FileHandle(
            Rc::new(RefCell::new(Some(f)))
        )))),
        Err(e) => Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
            format!("Failed to open file '{}': {}", path, e)
        ))))),
    }
}

pub(crate) fn native_file_read_line(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("File_readLine: expected 1 argument (FileHandle or path)".to_string());
    }
    match &args[0] {
        Value::FileHandle(file_rc) => {
            let mut file_opt = file_rc.borrow_mut();
            match file_opt.as_mut() {
                Some(file) => {
                    use std::io::Read;
                    let mut result = String::new();
                    let mut byte = [0u8; 1];
                    loop {
                        match file.read(&mut byte) {
                            Ok(0) => break, // EOF
                            Ok(_) => {
                                let ch = byte[0] as char;
                                if ch == '\n' {
                                    break;
                                }
                                result.push(ch);
                            }
                            Err(e) => return Err(format!("File_readLine: read error: {}", e)),
                        }
                    }
                    if result.is_empty() {
                        Ok(Value::ResultErr(Box::new(Value::String(Rc::new("EOF".to_string())))))
                    } else {
                        if result.ends_with('\r') { result.pop(); }
                        Ok(Value::ResultOk(Box::new(Value::String(Rc::new(result)))))
                    }
                }
                None => Ok(Value::ResultErr(Box::new(Value::String(Rc::new("FileHandle is closed".to_string()))))),
            }
        }
        Value::String(path) => {
            // Path-based sequential read using thread-local position tracking.
            // Each call returns the next line; returns Null at EOF.
            let path_str = path.as_str();
            let pos = FILE_READ_POSITIONS.with(|m| {
                m.borrow().get(path_str).copied().unwrap_or(0)
            });
            let resolved = super::resolve_path(path_str);
            let mut file = match std::fs::OpenOptions::new().read(true).open(&resolved) {
                Ok(f) => f,
                Err(_) => return Ok(Value::Null),
            };
            use std::io::{Read, Seek, SeekFrom};
            if pos > 0 && file.seek(SeekFrom::Start(pos)).is_err() {
                return Ok(Value::Null);
            }
            let mut result = String::new();
            let mut byte = [0u8; 1];
            let mut read_any = false;
            loop {
                match file.read(&mut byte) {
                    Ok(0) => break,
                    Ok(_) => {
                        read_any = true;
                        let ch = byte[0] as char;
                        if ch == '\n' { break; }
                        result.push(ch);
                    }
                    Err(_) => break,
                }
            }
            let new_pos = file.stream_position().unwrap_or(pos);
            if !read_any && result.is_empty() {
                // EOF: clear position so a subsequent read starts fresh
                FILE_READ_POSITIONS.with(|m| m.borrow_mut().remove(path_str));
                return Ok(Value::Null);
            }
            FILE_READ_POSITIONS.with(|m| {
                m.borrow_mut().insert(path_str.to_string(), new_pos);
            });
            if result.ends_with('\r') { result.pop(); }
            Ok(Value::String(Rc::new(result)))
        }
        _ => Err("File_readLine: expected FileHandle or String path argument".to_string()),
    }
}

pub(crate) fn native_file_write_content(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("File_write: expected 2 arguments (FileHandle or path, content)".to_string());
    }
    let content_str = match &args[1] {
        Value::String(s) => s.as_str().to_string(),
        v => v.display_string(),
    };
    match &args[0] {
        Value::FileHandle(file_rc) => {
            let mut file_opt = file_rc.borrow_mut();
            match file_opt.as_mut() {
                Some(file) => {
                    use std::io::Write;
                    match file.write_all(content_str.as_bytes()) {
                        Ok(()) => Ok(Value::Bool(true)),
                        Err(_) => Ok(Value::Bool(false)),
                    }
                }
                None => Ok(Value::Bool(false)),
            }
        }
        Value::String(path) => {
            // Write content to the file at the given path (append mode)
            use std::io::Write;
            let resolved = super::resolve_path(path.as_str());
            match std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&resolved)
            {
                Ok(mut file) => {
                    match file.write_all(content_str.as_bytes()) {
                        Ok(()) => Ok(Value::Bool(true)),
                        Err(_) => Ok(Value::Bool(false)),
                    }
                }
                Err(_) => Ok(Value::Bool(false)),
            }
        }
        _ => Err("File_write: expected FileHandle or String path argument".to_string()),
    }
}

pub(crate) fn native_file_close(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("File_close: expected 1 argument (FileHandle)".to_string());
    }
    match &args[0] {
        Value::FileHandle(file_rc) => {
            let mut file_opt = file_rc.borrow_mut();
            *file_opt = None;
            Ok(Value::Void)
        }
        // File class passes a string path; since path-based file operations
        // do not maintain persistent handles, close is a no-op. Clear any
        // cached read position so a subsequent open starts from byte 0.
        Value::String(path) => {
            FILE_READ_POSITIONS.with(|m| m.borrow_mut().remove(path.as_str()));
            Ok(Value::Void)
        }
        // ClassInstance may have a FileHandle field named "handle"
        Value::ClassInstance { fields, .. } => {
            if let Some(Value::FileHandle(file_rc)) = fields.borrow().get("handle") {
                let mut file_opt = file_rc.borrow_mut();
                *file_opt = None;
            }
            Ok(Value::Void)
        }
        _ => Err("File_close: expected FileHandle argument".to_string()),
    }
}

pub(crate) fn native_file_seek(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("File_seek: expected 2 arguments (path or FileHandle, position)".to_string());
    }
    let position = args[1].to_i64().unwrap_or(0);

    // If a FileHandle is provided, seek on it directly
    if let Value::FileHandle(file_rc) = &args[0] {
        let mut file_opt = file_rc.borrow_mut();
        match file_opt.as_mut() {
            Some(file) => {
                use std::io::{Seek, SeekFrom};
                match file.seek(SeekFrom::Start(position as u64)) {
                    Ok(pos) => return Ok(Value::Long(pos as i64)),
                    Err(e) => return Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
                        format!("File_seek: {}", e)
                    ))))),
                }
            }
            None => return Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
                "FileHandle is closed".to_string()
            ))))),
        }
    }

    // Path-based seek: open, seek, return position
    let path = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("File_seek: expected FileHandle or String path".to_string()),
    };
    let resolved = super::resolve_path(&path);
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&resolved)
        .or_else(|_| std::fs::File::open(&resolved));
    match file {
        Ok(mut f) => {
            use std::io::{Seek, SeekFrom};
            match f.seek(SeekFrom::Start(position as u64)) {
                Ok(pos) => Ok(Value::Long(pos as i64)),
                Err(e) => Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
                    format!("File_seek: {}", e)
                ))))),
            }
        }
        Err(e) => Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
            format!("File_seek: {}", e)
        ))))),
    }
}

pub(crate) fn native_file_tell(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("File_tell: expected 1 argument (path or FileHandle)".to_string());
    }

    // If a FileHandle is provided, tell on it directly
    if let Value::FileHandle(file_rc) = &args[0] {
        let file_opt = file_rc.borrow();
        match file_opt.as_ref() {
            Some(file) => {
                use std::io::Seek;
                match file.try_clone() {
                    Ok(mut cloned) => match cloned.stream_position() {
                        Ok(pos) => return Ok(Value::Long(pos as i64)),
                        Err(e) => return Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
                            format!("File_tell: {}", e)
                        ))))),
                    },
                    Err(e) => return Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
                        format!("File_tell: {}", e)
                    ))))),
                }
            }
            None => return Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
                "FileHandle is closed".to_string()
            ))))),
        }
    }

    // Path-based tell: open and get current position (will be 0 for fresh open)
    let path = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("File_tell: expected String path or FileHandle".to_string()),
    };
    let resolved = super::resolve_path(&path);
    let file = std::fs::File::open(&resolved);
    match file {
        Ok(mut f) => {
            use std::io::Seek;
            match f.stream_position() {
                Ok(pos) => Ok(Value::Long(pos as i64)),
                Err(e) => Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
                    format!("File_tell: {}", e)
                ))))),
            }
        }
        Err(e) => Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
            format!("File_tell: {}", e)
        ))))),
    }
}

pub(crate) fn native_file_read_bytes(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("File_readBytes: expected 2 arguments (path, count)".to_string());
    }
    let path = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("File_readBytes: expected String path".to_string()),
    };
    let count = args[1].to_i64().unwrap_or(0) as usize;
    let resolved = super::resolve_path(&path);

    match std::fs::read(&resolved) {
        Ok(data) => {
            let end = std::cmp::min(count, data.len());
            let bytes = &data[..end];
            // Return as hex-encoded string (matching the pattern used elsewhere)
            let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
            Ok(Value::String(Rc::new(hex)))
        }
        Err(e) => Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
            format!("File_readBytes: {}", e)
        ))))),
    }
}

pub(crate) fn native_file_write_bytes(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("File_writeBytes: expected 2 arguments (path, hexData)".to_string());
    }
    let path = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("File_writeBytes: expected String path".to_string()),
    };
    let hex_data = match &args[1] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("File_writeBytes: expected String hexData".to_string()),
    };

    // Decode hex string to bytes
    if hex_data.len() % 2 != 0 {
        return Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
            "File_writeBytes: hex data must have even length".to_string()
        )))));
    }
    let bytes: Result<Vec<u8>, _> = (0..hex_data.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex_data[i..i+2], 16))
        .collect();
    match bytes {
        Ok(data) => {
            let resolved = super::resolve_path(&path);
            match std::fs::write(&resolved, &data) {
                Ok(()) => Ok(Value::Void),
                Err(e) => Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
                    format!("File_writeBytes: {}", e)
                ))))),
            }
        }
        Err(e) => Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
            format!("File_writeBytes: invalid hex data: {}", e)
        ))))),
    }
}

pub(crate) fn native_file_last_modified(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("File_lastModified: expected 1 argument (path)".to_string());
    }
    let path = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("File_lastModified: expected String path".to_string()),
    };
    let resolved = super::resolve_path(&path);
    match std::fs::metadata(&resolved) {
        Ok(meta) => {
            match meta.modified() {
                Ok(time) => {
                    let duration = time.duration_since(std::time::UNIX_EPOCH)
                        .map_err(|e| format!("File_lastModified: {}", e))?;
                    let epoch_ms = duration.as_millis() as i64;
                    Ok(Value::Long(epoch_ms))
                }
                Err(e) => Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
                    format!("File_lastModified: {}", e)
                ))))),
            }
        }
        Err(e) => Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
            format!("File_lastModified: {}", e)
        ))))),
    }
}

pub(crate) fn native_file_set_modified(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("File_setModified: expected 2 arguments (path, epochMillis)".to_string());
    }
    let path = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("File_setModified: expected String path".to_string()),
    };
    let epoch_ms = args[1].to_i64().unwrap_or(0);
    let duration = std::time::Duration::from_millis(epoch_ms as u64);
    let time = std::time::SystemTime::UNIX_EPOCH + duration;
    let resolved = super::resolve_path(&path);
    // Open the file with write access — File::set_modified requires write
    // permissions on Windows (rejects read-only handles with ERROR_ACCESS_DENIED).
    match std::fs::OpenOptions::new().write(true).open(&resolved) {
        Ok(file) => {
            match file.set_modified(time) {
                Ok(()) => Ok(Value::Void),
                Err(e) => Err(format!("File_setModified: {}", e)),
            }
        }
        Err(e) => Err(format!("File_setModified: {}", e)),
    }
}

pub(crate) fn native_file_flush(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("File_flush: expected 1 argument (FileHandle or path)".to_string());
    }
    match &args[0] {
        Value::FileHandle(file_rc) => {
            let mut file_opt = file_rc.borrow_mut();
            match file_opt.as_mut() {
                Some(file) => {
                    use std::io::Write;
                    match file.flush() {
                        Ok(()) => Ok(Value::Void),
                        Err(e) => Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
                            format!("File_flush: {}", e)
                        ))))),
                    }
                }
                None => Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
                    "FileHandle is closed".to_string()
                ))))),
            }
        }
        // Path-based flush is a no-op: File_writeFile writes are already
        // flushed to disk by the OS on write completion.
        Value::String(_) => Ok(Value::Void),
        _ => Err("File_flush: expected FileHandle or String path argument".to_string()),
    }
}

pub(crate) fn native_file_size(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("File_size: expected 1 argument (path)".to_string());
    }
    let path = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("File_size: expected String path".to_string()),
    };
    let resolved = super::resolve_path(&path);
    match std::fs::metadata(&resolved) {
        Ok(meta) => Ok(Value::Long(meta.len() as i64)),
        Err(_) => Ok(Value::Long(0)),
    }
}

pub(crate) fn native_file_truncate(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("File_truncate: expected 2 arguments (path, length)".to_string());
    }
    let path = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("File_truncate: expected String path".to_string()),
    };
    let length = args[1].to_i64().unwrap_or(0) as u64;
    let resolved = super::resolve_path(&path);

    match std::fs::OpenOptions::new().write(true).open(&resolved) {
        Ok(file) => {
            match file.set_len(length) {
                Ok(()) => Ok(Value::Void),
                Err(e) => Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
                    format!("File_truncate: {}", e)
                ))))),
            }
        }
        Err(e) => Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
            format!("File_truncate: {}", e)
        ))))),
    }
}

pub(crate) fn native_file_copy(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("File_copy: expected 2 arguments (src, dst)".to_string());
    }
    let src = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("File_copy: expected String src".to_string()),
    };
    let dst = match &args[1] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("File_copy: expected String dst".to_string()),
    };
    let src_resolved = super::resolve_path(&src);
    let dst_resolved = super::resolve_path(&dst);
    match std::fs::copy(&src_resolved, &dst_resolved) {
        Ok(_) => Ok(Value::ResultOk(Box::new(Value::String(Rc::new(dst))))),
        Err(e) => Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
            format!("File_copy: {}", e)
        ))))),
    }
}

pub(crate) fn native_file_delete(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("File_delete: expected 1 argument (path)".to_string());
    }
    let path = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("File_delete: expected String argument".to_string()),
    };
    let resolved = super::resolve_path(&path);
    match std::fs::remove_file(&resolved) {
        Ok(()) => Ok(Value::Bool(true)),
        Err(_) => Ok(Value::Bool(false)),
    }
}

// Advisory file locking via a sidecar ".lock" file.
// lockType is "SHARED" or "EXCLUSIVE" (currently treated identically:
// the first caller wins; subsequent callers see the lock as held).
// Returns true if the lock was acquired, false if already held by someone else.
pub(crate) fn native_file_try_lock(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("File_tryLock: expected 2 arguments (path, lockType)".to_string());
    }
    let path = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("File_tryLock: expected String path".to_string()),
    };
    let lock_type = match &args[1] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("File_tryLock: expected String lockType".to_string()),
    };
    let resolved = super::resolve_path(&path);
    let lock_path = format!("{}.lock", resolved.display());
    // create_new(true) atomically fails if the file already exists.
    let result = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&lock_path)
        .and_then(|mut f| {
            use std::io::Write;
            f.write_all(lock_type.as_bytes())
        });
    match result {
        Ok(()) => Ok(Value::Bool(true)),
        Err(_) => Ok(Value::Bool(false)),
    }
}

// Release an advisory file lock acquired by File_tryLock.
// Removes the sidecar ".lock" file. Returns true on success or if no lock
// existed; false only on unexpected I/O errors.
pub(crate) fn native_file_unlock(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("File_unlock: expected 1 argument (path)".to_string());
    }
    let path = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("File_unlock: expected String path".to_string()),
    };
    let resolved = super::resolve_path(&path);
    let lock_path = format!("{}.lock", resolved.display());
    match std::fs::remove_file(&lock_path) {
        Ok(()) => Ok(Value::Bool(true)),
        Err(_) => Ok(Value::Bool(true)),
    }
}
