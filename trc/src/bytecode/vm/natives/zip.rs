// Titrate Alpha 0.3 – bytecode virtual machine: ZIP archive native functions
// Precision in every step – richie-rich90454, 2026
//
// Real ZIP archive support using the `zip` crate.
// ZipArchive and ZipWriter objects are stored in global registries keyed by handle.

use super::super::super::value::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::sync::{LazyLock, Mutex as StdMutex};
use zip::ZipArchive;
use zip::write::ZipWriter;
use zip::write::SimpleFileOptions;

static ZIP_READER_REGISTRY: LazyLock<StdMutex<HashMap<i64, ZipArchive<File>>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));
static ZIP_WRITER_REGISTRY: LazyLock<StdMutex<HashMap<i64, ZipWriter<File>>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));
static ZIP_NEXT_HANDLE: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);

// ---------------------------------------------------------------------------
// ZIP reader natives
// ---------------------------------------------------------------------------

pub(crate) fn native_zipfile_open(args: &[Value]) -> Result<Value, String> {
    let path = match args.first() {
        Some(Value::String(s)) => s.as_str().to_string(),
        _ => return Err("ZipFile_open: expected a string path argument".to_string()),
    };
    let resolved = super::resolve_path(&path);

    let file = File::open(&resolved)
        .map_err(|e| format!("ZipFile_open: failed to open '{}': {}", path, e))?;

    let archive = ZipArchive::new(file)
        .map_err(|e| format!("ZipFile_open: failed to read zip archive: {}", e))?;

    let handle = ZIP_NEXT_HANDLE.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let mut registry = ZIP_READER_REGISTRY.lock().unwrap();
    registry.insert(handle, archive);
    Ok(Value::Long(handle))
}

pub(crate) fn native_zipfile_entry_count(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("ZipFile_entryCount: expected a handle argument".to_string()),
    };

    let mut registry = ZIP_READER_REGISTRY.lock().unwrap();
    let archive = registry.get_mut(&handle)
        .ok_or_else(|| "ZipFile_entryCount: invalid zip handle".to_string())?;

    Ok(Value::Int(archive.len() as i32))
}

pub(crate) fn native_zipfile_entry_name(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("ZipFile_entryName: expected 2 arguments (handle, index)".to_string());
    }
    let handle = args[0].to_i64().unwrap_or(0);
    let index = args[1].to_i64().unwrap_or(0) as usize;

    let mut registry = ZIP_READER_REGISTRY.lock().unwrap();
    let archive = registry.get_mut(&handle)
        .ok_or_else(|| "ZipFile_entryName: invalid zip handle".to_string())?;

    let entry = archive.by_index(index)
        .map_err(|e| format!("ZipFile_entryName: invalid index {}: {}", index, e))?;

    Ok(Value::String(std::rc::Rc::new(entry.name().to_string())))
}

pub(crate) fn native_zipfile_read_entry(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("ZipFile_readEntry: expected 2 arguments (handle, name)".to_string());
    }
    let handle = args[0].to_i64().unwrap_or(0);
    let name = match &args[1] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("ZipFile_readEntry: expected a string name argument".to_string()),
    };

    let mut registry = ZIP_READER_REGISTRY.lock().unwrap();
    let archive = registry.get_mut(&handle)
        .ok_or_else(|| "ZipFile_readEntry: invalid zip handle".to_string())?;

    let mut entry = archive.by_name(&name)
        .map_err(|e| format!("ZipFile_readEntry: entry '{}' not found: {}", name, e))?;

    let mut buf = Vec::new();
    entry.read_to_end(&mut buf)
        .map_err(|e| format!("ZipFile_readEntry: failed to read entry: {}", e))?;

    let content = String::from_utf8(buf)
        .map_err(|e| format!("ZipFile_readEntry: entry contains invalid UTF-8: {}", e))?;

    Ok(Value::String(std::rc::Rc::new(content)))
}

pub(crate) fn native_zipfile_extract_all(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("ZipFile_extractAll: expected 2 arguments (handle, destDir)".to_string());
    }
    let handle = args[0].to_i64().unwrap_or(0);
    let dest_dir = match &args[1] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("ZipFile_extractAll: expected a string destDir argument".to_string()),
    };

    let mut registry = ZIP_READER_REGISTRY.lock().unwrap();
    let archive = registry.get_mut(&handle)
        .ok_or_else(|| "ZipFile_extractAll: invalid zip handle".to_string())?;

    let dest_dir_resolved = super::resolve_path(&dest_dir);
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
            .map_err(|e| format!("ZipFile_extractAll: failed to get entry {}: {}", i, e))?;

        let outpath = dest_dir_resolved.join(file.name());

        if file.is_dir() {
            std::fs::create_dir_all(&outpath)
                .map_err(|e| format!("ZipFile_extractAll: failed to create dir '{}': {}", outpath.display(), e))?;
        } else {
            if let Some(p) = outpath.parent() {
                std::fs::create_dir_all(p)
                    .map_err(|e| format!("ZipFile_extractAll: failed to create parent dir: {}", e))?;
            }
            let mut outfile = File::create(&outpath)
                .map_err(|e| format!("ZipFile_extractAll: failed to create file '{}': {}", outpath.display(), e))?;
            std::io::copy(&mut file, &mut outfile)
                .map_err(|e| format!("ZipFile_extractAll: failed to write file: {}", e))?;
        }
    }

    Ok(Value::Void)
}

pub(crate) fn native_zipfile_close(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("ZipFile_close: expected a handle argument".to_string()),
    };

    let mut registry = ZIP_READER_REGISTRY.lock().unwrap();
    registry.remove(&handle);
    Ok(Value::Void)
}

// ---------------------------------------------------------------------------
// ZIP writer natives
// ---------------------------------------------------------------------------

pub(crate) fn native_zipwriter_open(args: &[Value]) -> Result<Value, String> {
    let path = match args.first() {
        Some(Value::String(s)) => s.as_str().to_string(),
        _ => return Err("ZipWriter_open: expected a string path argument".to_string()),
    };
    let resolved = super::resolve_path(&path);

    let file = File::create(&resolved)
        .map_err(|e| format!("ZipWriter_open: failed to create '{}': {}", path, e))?;

    let writer = ZipWriter::new(file);

    let handle = ZIP_NEXT_HANDLE.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let mut registry = ZIP_WRITER_REGISTRY.lock().unwrap();
    registry.insert(handle, writer);
    Ok(Value::Long(handle))
}

pub(crate) fn native_zipwriter_add_entry(args: &[Value]) -> Result<Value, String> {
    if args.len() < 3 {
        return Err("ZipWriter_addEntry: expected 3 arguments (handle, name, data)".to_string());
    }
    let handle = args[0].to_i64().unwrap_or(0);
    let name = match &args[1] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("ZipWriter_addEntry: expected a string name argument".to_string()),
    };
    let data = match &args[2] {
        Value::String(s) => s.as_bytes().to_vec(),
        _ => return Err("ZipWriter_addEntry: expected a string data argument".to_string()),
    };

    let mut registry = ZIP_WRITER_REGISTRY.lock().unwrap();
    let writer = registry.get_mut(&handle)
        .ok_or_else(|| "ZipWriter_addEntry: invalid zip writer handle".to_string())?;

    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    writer.start_file(&name, options)
        .map_err(|e| format!("ZipWriter_addEntry: failed to start file '{}': {}", name, e))?;

    writer.write_all(&data)
        .map_err(|e| format!("ZipWriter_addEntry: failed to write data: {}", e))?;

    Ok(Value::Void)
}

pub(crate) fn native_zipwriter_close(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("ZipWriter_close: expected a handle argument".to_string()),
    };

    let mut registry = ZIP_WRITER_REGISTRY.lock().unwrap();
    if let Some(writer) = registry.remove(&handle) {
        writer.finish()
            .map_err(|e| format!("ZipWriter_close: failed to finish zip archive: {}", e))?;
    }
    Ok(Value::Void)
}
