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

    let file = File::open(&resolved)
        .map_err(|e| format!("Mmap_open: failed to open '{}': {}", path, e))?;

    let metadata = file.metadata()
        .map_err(|e| format!("Mmap_open: failed to get metadata: {}", e))?;

    if metadata.len() == 0 {
        return Err("Mmap_open: cannot mmap empty file".to_string());
    }

    // Try read-write first; fall back to read-only
    // SAFETY: `MmapMut::map_mut` is unsafe because it maps the file's bytes
    // directly into process memory and returns a view aliasing the underlying
    // file. The `file` handle was just opened and validated (non-empty) above,
    // and the returned `MmapMut` is stored in a registry that keeps it alive
    // for the lifetime of the mapping; memmap2 guarantees the mapping is
    // unmapped on drop. We do not hand out raw pointers to this memory to
    // arbitrary native code — only indexed byte access through `Mmap_get`/`
    // `Mmap_set`, which bounds-check via `mmap.len()`.
    let mmap = match unsafe { MmapMut::map_mut(&file) } {
        Ok(m) => m,
        Err(_) => {
            // Read-only fallback
            // SAFETY: Same justification as above — `file` is a valid, non-empty
            // open handle, and the resulting mapping is owned by the returned
            // `Mmap` which is immediately converted to `MmapMut` via
            // `make_mut()` (which itself returns `Result` and propagates OS
            // errors). The mapping is kept alive in the registry.
            let mmap_ro = unsafe { memmap2::Mmap::map(&file) }
                .map_err(|e| format!("Mmap_open: failed to mmap '{}': {}", path, e))?;
            // Convert read-only to mutable (will fail if file is read-only on some platforms)
            mmap_ro.make_mut()
                .map_err(|e| format!("Mmap_open: failed to make mmap mutable: {}", e))?
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
