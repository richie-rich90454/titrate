// Titrate Alpha 0.2 – bytecode virtual machine: file natives
// Precision in every step – richie-rich90454, 2026

use super::super::super::value::Value;
use std::io::{BufRead, BufReader};
use std::cell::RefCell;
use std::rc::Rc;

pub(crate) fn native_file_read(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("File_readFile: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => {
            match std::fs::read_to_string(path.as_str()) {
                Ok(content) => Ok(Value::ResultOk(Box::new(Value::String(Rc::new(content))))),
                Err(e) => Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
                    format!("Failed to read file '{}': {}", path, e)
                ))))),
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
                Ok(()) => Ok(Value::ResultOk(Box::new(Value::Void))),
                Err(e) => Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
                    format!("Failed to write file '{}': {}", path, e)
                ))))),
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
                Err(e) => Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
                    format!("Failed to read file '{}': {}", path, e)
                ))))),
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
            let file_opt = file_rc.borrow();
            match file_opt.as_ref() {
                Some(file) => {
                    let mut reader = BufReader::new(file.try_clone().map_err(|e| format!("File_readLine: {}", e))?);
                    let mut line = String::new();
                    match reader.read_line(&mut line) {
                        Ok(0) => Ok(Value::ResultErr(Box::new(Value::String(Rc::new("EOF".to_string()))))),
                        Ok(_) => {
                            if line.ends_with('\n') { line.pop(); }
                            if line.ends_with('\r') { line.pop(); }
                            Ok(Value::ResultOk(Box::new(Value::String(Rc::new(line)))))
                        }
                        Err(e) => Ok(Value::ResultErr(Box::new(Value::String(Rc::new(format!("File_readLine: {}", e)))))),
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
        return Err("File_write: expected 2 arguments (FileHandle, content)".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::FileHandle(file_rc), Value::String(content)) => {
            let mut file_opt = file_rc.borrow_mut();
            match file_opt.as_mut() {
                Some(file) => {
                    use std::io::Write;
                    match file.write_all(content.as_bytes()) {
                        Ok(()) => Ok(Value::ResultOk(Box::new(Value::Void))),
                        Err(e) => Ok(Value::ResultErr(Box::new(Value::String(Rc::new(format!("File_write: {}", e)))))),
                    }
                }
                None => Ok(Value::ResultErr(Box::new(Value::String(Rc::new("FileHandle is closed".to_string()))))),
            }
        }
        _ => Err("File_write: expected (FileHandle, String)".to_string()),
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
        _ => Err("File_close: expected FileHandle argument".to_string()),
    }
}

pub(crate) fn native_file_seek(args: &[Value]) -> Result<Value, String> {
    Err("File_seek: not yet implemented".to_string())
}

pub(crate) fn native_file_tell(args: &[Value]) -> Result<Value, String> {
    Err("File_tell: not yet implemented".to_string())
}

pub(crate) fn native_file_read_bytes(args: &[Value]) -> Result<Value, String> {
    Err("File_readBytes: not yet implemented".to_string())
}

pub(crate) fn native_file_write_bytes(args: &[Value]) -> Result<Value, String> {
    Err("File_writeBytes: not yet implemented".to_string())
}

pub(crate) fn native_file_last_modified(args: &[Value]) -> Result<Value, String> {
    Err("File_lastModified: not yet implemented".to_string())
}
