// Titrate Alpha 0.3 – bytecode virtual machine: mmap native functions
// Precision in every step – richie-rich90454, 2026
//
// Real memory-mapped file support using the `memmap2` crate.
// Mmap objects are stored in a global registry keyed by integer handle.

use super::super::super::value::Value;
use memmap2::MmapMut;
use std::fs::File;
use std::sync::{LazyLock, Mutex as StdMutex};
use std::collections::HashMap;

static MMAP_REGISTRY: LazyLock<StdMutex<HashMap<i64, MmapMut>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));
static MMAP_NEXT_HANDLE: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);

pub(crate) fn native_mmap_open(args: &[Value]) -> Result<Value, String> {
    let path = match args.first() {
        Some(Value::String(s)) => s.as_str().to_string(),
        _ => return Err("Mmap_open: expected a string path argument".to_string()),
    };
    let resolved = super::resolve_path(&path);

    // Open the file with read+write access so MmapMut::map_mut can map it
    // writable on Windows (which requires the underlying handle to have write
    // access). If read-write open fails (e.g. read-only file), fall back to
    // read-only and use a read-only mapping.
    let file_rw = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&resolved);
    let (file, can_write) = match file_rw {
        Ok(f) => (f, true),
        Err(_) => {
            let f = File::open(&resolved)
                .map_err(|e| format!("Mmap_open: failed to open '{}': {}", path, e))?;
            (f, false)
        }
    };

    let metadata = file.metadata()
        .map_err(|e| format!("Mmap_open: failed to get metadata: {}", e))?;

    if metadata.len() == 0 {
        return Err("Mmap_open: cannot mmap empty file".to_string());
    }

    // SAFETY: `MmapMut::map_mut` / `Mmap::map` are unsafe because they map the
    // file's bytes directly into process memory and return a view aliasing the
    // underlying file. The `file` handle was just opened and validated
    // (non-empty) above, and the returned mapping is stored in a registry that
    // keeps it alive for the lifetime of the mapping; memmap2 guarantees the
    // mapping is unmapped on drop. We do not hand out raw pointers to this
    // memory to arbitrary native code — only indexed byte access through
    // `Mmap_get`/`Mmap_set`, which bounds-check via `mmap.len()`.
    let mmap: MmapMut = if can_write {
        match unsafe { MmapMut::map_mut(&file) } {
            Ok(m) => m,
            Err(e) => return Err(format!("Mmap_open: failed to mmap '{}': {}", path, e)),
        }
    } else {
        // Read-only file: map read-only, then try to make mutable. If
        // make_mut fails (e.g. on Windows where the underlying file handle
        // lacks write access), fall back to a fresh MmapMut::map_mut on a
        // read-only mapping of the bytes copied into memory. Since we cannot
        // satisfy that without write access, return an error suggesting the
        // user make the file writable.
        let mmap_ro = unsafe { memmap2::Mmap::map(&file) }
            .map_err(|e| format!("Mmap_open: failed to mmap '{}': {}", path, e))?;
        match mmap_ro.make_mut() {
            Ok(m) => m,
            Err(_) => {
                return Err(format!(
                    "Mmap_open: file '{}' is read-only; cannot map writable", path
                ));
            }
        }
    };

    let handle = MMAP_NEXT_HANDLE.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let mut registry = MMAP_REGISTRY.lock().unwrap();
    registry.insert(handle, mmap);
    Ok(Value::Long(handle))
}

pub(crate) fn native_mmap_close(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Mmap_close: expected a handle argument".to_string()),
    };

    let mut registry = MMAP_REGISTRY.lock().unwrap();
    registry.remove(&handle);
    Ok(Value::Void)
}

pub(crate) fn native_mmap_get(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Mmap_get: expected 2 arguments (handle, offset)".to_string());
    }
    let handle = args[0].to_i64().unwrap_or(0);
    let offset = args[1].to_i64().unwrap_or(0) as usize;

    let registry = MMAP_REGISTRY.lock().unwrap();
    let mmap = registry.get(&handle)
        .ok_or_else(|| "Mmap_get: invalid mmap handle".to_string())?;

    if offset >= mmap.len() {
        return Err(format!("Mmap_get: offset {} out of bounds (len {})", offset, mmap.len()));
    }

    Ok(Value::Int(mmap[offset] as i32))
}

pub(crate) fn native_mmap_set(args: &[Value]) -> Result<Value, String> {
    if args.len() < 3 {
        return Err("Mmap_set: expected 3 arguments (handle, offset, value)".to_string());
    }
    let handle = args[0].to_i64().unwrap_or(0);
    let offset = args[1].to_i64().unwrap_or(0) as usize;
    let value = args[2].to_i64().unwrap_or(0) as u8;

    let mut registry = MMAP_REGISTRY.lock().unwrap();
    let mmap = registry.get_mut(&handle)
        .ok_or_else(|| "Mmap_set: invalid mmap handle".to_string())?;

    if offset >= mmap.len() {
        return Err(format!("Mmap_set: offset {} out of bounds (len {})", offset, mmap.len()));
    }

    mmap[offset] = value;
    Ok(Value::Void)
}

pub(crate) fn native_mmap_size(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Mmap_size: expected a handle argument".to_string()),
    };

    let registry = MMAP_REGISTRY.lock().unwrap();
    let mmap = registry.get(&handle)
        .ok_or_else(|| "Mmap_size: invalid mmap handle".to_string())?;

    Ok(Value::Int(mmap.len() as i32))
}

pub(crate) fn native_mmap_flush(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Mmap_flush: expected a handle argument".to_string()),
    };

    let mut registry = MMAP_REGISTRY.lock().unwrap();
    let mmap = registry.get_mut(&handle)
        .ok_or_else(|| "Mmap_flush: invalid mmap handle".to_string())?;

    mmap.flush()
        .map_err(|e| format!("Mmap_flush: failed to flush: {}", e))?;

    Ok(Value::Void)
}
