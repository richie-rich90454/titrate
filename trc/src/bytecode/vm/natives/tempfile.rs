// Titrate Alpha 0.2 – bytecode virtual machine: tempfile natives
// Precision in every step – richie-rich90454, 2026

use super::super::super::value::Value;
use std::rc::Rc;

pub(crate) static TEMPFILE_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub(crate) fn native_tempfile_create(args: &[Value]) -> Result<Value, String> {
    let prefix = match args.first() {
        Some(Value::String(s)) => s.as_str().to_string(),
        _ => "titrate_".to_string(),
    };
    let is_dir = args.get(1)
        .map(|v| matches!(v, Value::Bool(true)))
        .unwrap_or(false);
    let pid = std::process::id();
    let counter = TEMPFILE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let suffix = format!("{}_{}", pid, counter);
    if is_dir {
        let dir = std::env::temp_dir().join(format!("{}{}", prefix, suffix));
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("Tempfile_create: cannot create directory '{}': {}", dir.display(), e))?;
        Ok(Value::String(Rc::new(dir.to_string_lossy().to_string())))
    } else {
        let path = std::env::temp_dir().join(format!("{}{}", prefix, suffix));
        std::fs::File::create(&path)
            .map_err(|e| format!("Tempfile_create: cannot create file '{}': {}", path.display(), e))?;
        Ok(Value::String(Rc::new(path.to_string_lossy().to_string())))
    }
}
