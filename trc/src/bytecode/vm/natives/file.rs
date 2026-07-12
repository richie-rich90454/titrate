// Titrate Alpha 0.2 – bytecode virtual machine: file natives
// Precision in every step – richie-rich90454, 2026

use super::super::super::value::Value;
use std::cell::RefCell;
use std::rc::Rc;

pub(crate) fn native_file_read(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("File_readFile: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => {
            match std::fs::read_to_string(path.as_str()) {
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
            match std::fs::write(path.as_str(), content.as_str()) {
                Ok(()) => Ok(Value::Bool(true)),
                Err(_) => Ok(Value::Bool(false)),
            }
        }
        _ => Err("File_writeFile: expected (String, String)".to_string()),
    }
}

pub(crate) fn native_file_read_lines(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("File_readLines: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => {
            match std::fs::read_to_string(path.as_str()) {
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
    let file = match mode {
        "r" | "rb" => std::fs::File::open(path),
        "w" | "wb" => std::fs::File::create(path),
        "a" | "ab" => std::fs::OpenOptions::new().append(true).open(path),
        "r+" => std::fs::OpenOptions::new().read(true).write(true).open(path),
        "w+" => std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(true).open(path),
        "a+" => std::fs::OpenOptions::new().read(true).append(true).open(path),
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
        return Err("File_readLine: expected 1 argument (FileHandle)".to_string());
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
        _ => Err("File_readLine: expected FileHandle argument".to_string()),
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
            match std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path.as_str())
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
        // do not maintain persistent handles, close is a no-op.
        Value::String(_) => Ok(Value::Void),
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
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .or_else(|_| std::fs::File::open(&path));
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
    let file = std::fs::File::open(&path);
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

    match std::fs::read(&path) {
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
            match std::fs::write(&path, &data) {
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
    match std::fs::metadata(&path) {
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
    match std::fs::File::open(&path) {
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
        return Err("File_flush: expected 1 argument (FileHandle)".to_string());
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
        _ => Err("File_flush: expected FileHandle argument".to_string()),
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
    match std::fs::metadata(&path) {
        Ok(meta) => Ok(Value::Long(meta.len() as i64)),
        Err(e) => Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
            format!("File_size: {}", e)
        ))))),
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

    match std::fs::OpenOptions::new().write(true).open(&path) {
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
    match std::fs::copy(&src, &dst) {
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
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(Value::Bool(true)),
        Err(_) => Ok(Value::Bool(false)),
    }
}
