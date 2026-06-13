// Titrate Alpha 0.2 – bytecode virtual machine: system natives
// Precision in every step – richie-rich90454, 2026

use super::super::super::value::Value;
use std::rc::Rc;

pub(crate) fn native_sys_args(args: &[Value]) -> Result<Value, String> {
    // The VM doesn't have direct access to std::env::args() in a clean way,
    // but we can return an empty array as placeholder. A real implementation
    // would need the args to be passed into the VM at startup.
    let _ = args;
    let elements: Vec<Value> = std::env::args()
        .map(|a| Value::String(Rc::new(a)))
        .collect();
    Ok(Value::Array { elements })
}

pub(crate) fn native_sys_env(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Sys_env: expected 1 argument (key)".to_string());
    }
    match &args[0] {
        Value::String(key) => match std::env::var(key.as_str()) {
            Ok(val) => Ok(Value::String(Rc::new(val))),
            Err(_) => Ok(Value::String(Rc::new(String::new()))),
        },
        _ => Err("Sys_env: expected String argument".to_string()),
    }
}

pub(crate) fn native_sys_set_env(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Sys_setEnv: expected 2 arguments (key, value)".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::String(key), Value::String(val)) => {
            std::env::set_var(key.as_str(), val.as_str());
            Ok(Value::Void)
        }
        _ => Err("Sys_setEnv: expected (String, String)".to_string()),
    }
}

pub(crate) fn native_sys_exit(args: &[Value]) -> Result<Value, String> {
    let code = if args.is_empty() {
        0i64
    } else {
        args[0].to_i64().unwrap_or(0)
    };
    std::process::exit(code as i32);
}

pub(crate) fn native_sys_working_dir(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    match std::env::current_dir() {
        Ok(path) => Ok(Value::String(Rc::new(path.to_string_lossy().to_string()))),
        Err(e) => Err(format!("Sys_workingDir: {}", e)),
    }
}

pub(crate) fn native_sys_sleep(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Sys_sleep: expected 1 argument (milliseconds)".to_string());
    }
    let ms = args[0].to_i64().unwrap_or(0);
    std::thread::sleep(std::time::Duration::from_millis(ms as u64));
    Ok(Value::Void)
}

pub(crate) fn native_env_get(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Env_get: expected 1 argument (name)".to_string());
    }
    match &args[0] {
        Value::String(name) => match std::env::var(name.as_str()) {
            Ok(val) => Ok(Value::String(Rc::new(val))),
            Err(_) => Ok(Value::Null),
        },
        _ => Err("Env_get: expected String argument".to_string()),
    }
}

pub(crate) fn native_env_set(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Env_set: expected 2 arguments (name, value)".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::String(name), Value::String(val)) => {
            std::env::set_var(name.as_str(), val.as_str());
            Ok(Value::Void)
        }
        _ => Err("Env_set: expected (String, String)".to_string()),
    }
}

pub(crate) fn native_env_vars(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let elements: Vec<Value> = std::env::vars()
        .map(|(k, v)| Value::String(Rc::new(format!("{}={}", k, v))))
        .collect();
    Ok(Value::Array { elements })
}

pub(crate) fn native_fs_exists(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Fs_exists: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => Ok(Value::Bool(std::path::Path::new(path.as_str()).exists())),
        _ => Err("Fs_exists: expected String argument".to_string()),
    }
}

pub(crate) fn native_fs_is_file(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Fs_isFile: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => Ok(Value::Bool(std::path::Path::new(path.as_str()).is_file())),
        _ => Err("Fs_isFile: expected String argument".to_string()),
    }
}

pub(crate) fn native_fs_is_dir(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Fs_isDir: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => Ok(Value::Bool(std::path::Path::new(path.as_str()).is_dir())),
        _ => Err("Fs_isDir: expected String argument".to_string()),
    }
}

pub(crate) fn native_fs_size(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Fs_size: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => match std::fs::metadata(path.as_str()) {
            Ok(meta) => Ok(Value::Long(meta.len() as i64)),
            Err(e) => Err(format!("Fs_size: {}", e)),
        },
        _ => Err("Fs_size: expected String argument".to_string()),
    }
}

pub(crate) fn native_process_id(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Ok(Value::Long(std::process::id() as i64))
}

pub(crate) fn native_process_args(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let elements: Vec<Value> = std::env::args()
        .map(|a| Value::String(Rc::new(a)))
        .collect();
    Ok(Value::Array { elements })
}

pub(crate) fn native_os_name(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Ok(Value::String(Rc::new(std::env::consts::OS.to_string())))
}

pub(crate) fn native_os_arch(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Ok(Value::String(Rc::new(std::env::consts::ARCH.to_string())))
}

pub(crate) fn native_os_family(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Ok(Value::String(Rc::new(std::env::consts::FAMILY.to_string())))
}

// ---------------------------------------------------------------------------
// Signal natives (stubs – signal handling not supported on Windows)
// ---------------------------------------------------------------------------

pub(crate) fn native_signal_register(_args: &[Value]) -> Result<Value, String> {
    Err("Signal handling not supported on this platform".to_string())
}

pub(crate) fn native_signal_raise(_args: &[Value]) -> Result<Value, String> {
    Err("Signal handling not supported on this platform".to_string())
}

// ---------------------------------------------------------------------------
// Additional Os natives
// ---------------------------------------------------------------------------

pub(crate) fn native_os_cpu_count(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let count = std::thread::available_parallelism()
        .map(|n| n.get() as i64)
        .unwrap_or(1);
    Ok(Value::Long(count))
}

pub(crate) fn native_os_user_name(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    // Try USERNAME (Windows) then USER (Unix)
    let name = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "unknown".to_string());
    Ok(Value::String(Rc::new(name)))
}

pub(crate) fn native_os_host_name(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    // Try COMPUTERNAME (Windows) then HOSTNAME (Unix)
    let name = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown".to_string());
    Ok(Value::String(Rc::new(name)))
}

pub(crate) fn native_os_urandom(args: &[Value]) -> Result<Value, String> {
    let n = match args.first() {
        Some(Value::Long(v)) => *v as usize,
        Some(Value::Int(v)) => *v as usize,
        _ => return Err("Os_urandom: expected an Int/Long byte count".to_string()),
    };
    let mut buf = vec![0u8; n];
    // Use the rand crate that's already a dependency
    use rand::RngCore;
    rand::thread_rng().fill_bytes(&mut buf);
    let hex: String = buf.iter().map(|b| format!("{:02x}", b)).collect();
    Ok(Value::String(Rc::new(hex)))
}

pub(crate) fn native_os_chmod(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Os_chmod: expected 2 arguments (path, mode)".to_string());
    }
    let path = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Os_chmod: expected String path".to_string()),
    };
    let mode = args[1].to_i64().unwrap_or(0) as u32;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        match std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)) {
            Ok(()) => Ok(Value::ResultOk(Box::new(Value::String(Rc::new(path))))),
            Err(e) => Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
                format!("Os_chmod: {}", e)
            ))))),
        }
    }

    #[cfg(not(unix))]
    {
        let _ = mode;
        // On Windows, chmod is not supported; return an error
        Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
            "Os_chmod: not supported on this platform".to_string()
        )))))
    }
}

pub(crate) fn native_os_makedirs(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Os_makedirs: expected 1 argument (path)".to_string());
    }
    let path = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Os_makedirs: expected String path".to_string()),
    };
    match std::fs::create_dir_all(&path) {
        Ok(()) => Ok(Value::ResultOk(Box::new(Value::String(Rc::new(path))))),
        Err(e) => Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
            format!("Os_makedirs: {}", e)
        ))))),
    }
}

pub(crate) fn native_os_symlink(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Os_symlink: expected 2 arguments (original, link)".to_string());
    }
    let original = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Os_symlink: expected String original".to_string()),
    };
    let link = match &args[1] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Os_symlink: expected String link".to_string()),
    };

    #[cfg(unix)]
    {
        match std::os::unix::fs::symlink(&original, &link) {
            Ok(()) => Ok(Value::ResultOk(Box::new(Value::String(Rc::new(link))))),
            Err(e) => Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
                format!("Os_symlink: {}", e)
            ))))),
        }
    }

    #[cfg(not(unix))]
    {
        // On Windows, symlink support requires privileges; use std::os::windows::fs::symlink_file/dir
        // For simplicity, try symlink_file as a best-effort
        #[cfg(windows)]
        {
            match std::os::windows::fs::symlink_file(&original, &link) {
                Ok(()) => Ok(Value::ResultOk(Box::new(Value::String(Rc::new(link))))),
                Err(e) => Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
                    format!("Os_symlink: {}", e)
                ))))),
            }
        }
        #[cfg(not(windows))]
        {
            let _ = (original, link);
            Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
                "Os_symlink: not supported on this platform".to_string()
            )))))
        }
    }
}

pub(crate) fn native_os_readlink(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Os_readlink: expected 1 argument (path)".to_string());
    }
    let path = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Os_readlink: expected String path".to_string()),
    };
    match std::fs::read_link(&path) {
        Ok(target) => Ok(Value::ResultOk(Box::new(Value::String(Rc::new(
            target.to_string_lossy().to_string()
        ))))),
        Err(e) => Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
            format!("Os_readlink: {}", e)
        ))))),
    }
}
