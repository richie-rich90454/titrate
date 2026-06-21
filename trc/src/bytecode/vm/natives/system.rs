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
            Err(_) => Ok(Value::Null),
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

pub(crate) fn native_sys_set_working_dir(args: &[Value]) -> Result<Value, String> {
    let path = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("Sys_setWorkingDir: expected a string path".to_string()),
    };
    match std::env::set_current_dir(path) {
        Ok(()) => Ok(Value::Void),
        Err(e) => Err(format!("Sys_setWorkingDir: {}", e)),
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
// Signal natives – real OS signal handling via the C standard library
// ---------------------------------------------------------------------------

// Flags set by the signal handler; index 0 is unused (signal 0 doesn't exist).
static SIGNAL_RECEIVED: [std::sync::atomic::AtomicBool; 32] =
    [const { std::sync::atomic::AtomicBool::new(false) }; 32];

/// Async-signal-safe handler: just records that a signal was received.
extern "C" fn signal_handler(sig: i32) {
    if sig > 0 && (sig as usize) < SIGNAL_RECEIVED.len() {
        SIGNAL_RECEIVED[sig as usize].store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

// C standard library signal functions (linked on both Unix and Windows).
extern "C" {
    fn signal(sig: i32, handler: extern "C" fn(i32)) -> usize;
    fn raise(sig: i32) -> i32;
}

/// Install a signal handler that records received signals. The handler does
/// not invoke Titrate code directly (signal handlers must be async-signal-
/// safe); use Signal_wasReceived to poll for delivery.
pub(crate) fn native_signal_register(args: &[Value]) -> Result<Value, String> {
    let signum = match args.first() {
        Some(Value::Int(s)) => *s as i32,
        Some(Value::Long(s)) => *s as i32,
        _ => return Err("Signal_register: expected a signal number".to_string()),
    };

    // SIG_ERR is defined as ((_sig_func)-1), i.e. all-ones pointer.
    const SIG_ERR: usize = usize::MAX;

    let prev = unsafe { signal(signum, signal_handler) };
    if prev == SIG_ERR {
        return Err(format!(
            "Signal_register: failed to register handler for signal {}",
            signum
        ));
    }

    Ok(Value::Int(0))
}

/// Send a signal to the current process via the C raise() function.
pub(crate) fn native_signal_raise(args: &[Value]) -> Result<Value, String> {
    let signum = match args.first() {
        Some(Value::Int(s)) => *s as i32,
        Some(Value::Long(s)) => *s as i32,
        _ => return Err("Signal_raise: expected a signal number".to_string()),
    };

    let rc = unsafe { raise(signum) };
    if rc != 0 {
        return Err(format!("Signal_raise: failed to raise signal {}", signum));
    }

    Ok(Value::Int(0))
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
    let _path = match &args[0] {
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

pub(crate) fn native_os_kill(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Os_kill: expected 2 arguments (pid, signal)".to_string());
    }
    let pid = args[0].to_i64().unwrap_or(0);
    let _sig = args[1].to_i64().unwrap_or(0);

    #[cfg(unix)]
    {
        // Use libc::kill directly (libc is already a dependency)
        let ret = unsafe { libc::kill(pid as libc::pid_t, _sig as libc::c_int) };
        if ret == 0 {
            Ok(Value::Void)
        } else {
            Err(format!("Os_kill: failed to send signal {} to pid {}", _sig, pid))
        }
    }

    #[cfg(not(unix))]
    {
        // On Windows, terminate the process using taskkill
        let output = std::process::Command::new("taskkill")
            .args(["/PID", &format!("{}", pid), "/F"])
            .output();
        match output {
            Ok(_) => Ok(Value::Void),
            Err(e) => Err(format!("Os_kill: {}", e)),
        }
    }
}

pub(crate) fn native_os_environ(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    // Return all environment variables as a formatted string (KEY=VALUE\n...)
    let env_str: String = std::env::vars()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<String>>()
        .join("\n");
    Ok(Value::String(Rc::new(env_str)))
}

pub(crate) fn native_os_umask(args: &[Value]) -> Result<Value, String> {
    let mask = match args.first() {
        Some(Value::Long(v)) => *v as u32,
        Some(Value::Int(v)) => *v as u32,
        _ => return Err("Os_umask: expected an Int/Long mask value".to_string()),
    };

    #[cfg(unix)]
    {
        let old_mask = unsafe { libc::umask(mask as libc::mode_t) };
        Ok(Value::Long(old_mask as i64))
    }

    #[cfg(not(unix))]
    {
        // Windows: use the C runtime _umask function.
        extern "C" {
            fn _umask(mode: i32) -> i32;
        }
        let old = unsafe { _umask(mask as i32) };
        Ok(Value::Long(old as i64))
    }
}

pub(crate) fn native_os_scandir(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Os_scandir: expected 1 argument (path)".to_string());
    }
    let path = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Os_scandir: expected String path".to_string()),
    };
    let dir = match std::fs::read_dir(&path) {
        Ok(d) => d,
        Err(e) => return Err(format!("Os_scandir: {}", e)),
    };
    let mut entries: Vec<Value> = Vec::new();
    for entry in dir {
        match entry {
            Ok(e) => {
                let name = e.file_name().to_string_lossy().to_string();
                let is_file = e.file_type().map(|ft| ft.is_file()).unwrap_or(false);
                let is_dir = e.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
                let is_symlink = e.file_type().map(|ft| ft.is_symlink()).unwrap_or(false);
                // Return as an array of [name, isFile, isDir, isSymlink] tuples
                entries.push(Value::Array {
                    elements: vec![
                        Value::String(Rc::new(name)),
                        Value::Bool(is_file),
                        Value::Bool(is_dir),
                        Value::Bool(is_symlink),
                    ],
                });
            }
            Err(_) => continue,
        }
    }
    Ok(Value::Array { elements: entries })
}

pub(crate) fn native_os_environ_map(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    // Return all environment variables as an array of [key, value] pairs
    let pairs: Vec<Value> = std::env::vars()
        .map(|(k, v)| Value::Array {
            elements: vec![
                Value::String(Rc::new(k)),
                Value::String(Rc::new(v)),
            ],
        })
        .collect();
    Ok(Value::Array { elements: pairs })
}

// ---------------------------------------------------------------------------
// Additional Os native stubs
// ---------------------------------------------------------------------------

pub(crate) fn native_sys_change_dir(args: &[Value]) -> Result<Value, String> {
    let path = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("Sys_changeDir: expected a String path".to_string()),
    };
    std::env::set_current_dir(path)
        .map_err(|e| format!("Sys_changeDir: {}", e))?;
    Ok(Value::Void)
}

pub(crate) fn native_os_getppid(_args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        // On Unix, get the parent process ID via libc
        let ppid = unsafe { libc::getppid() };
        Ok(Value::Int(ppid as i32))
    }
    #[cfg(not(unix))]
    {
        // Windows: use the toolhelp32 API to find the parent process ID.
        #[repr(C)]
        struct ProcessEntry32W {
            dw_size: u32,
            cnt_usage: u32,
            th32_process_id: u32,
            th32_default_heap_id: usize,
            th32_module_id: u32,
            cnt_threads: u32,
            th32_parent_process_id: u32,
            pc_pri_class: i32,
            dw_flags: u32,
            sz_exe_file: [u16; 260],
        }

        const TH32CS_SNAPPROCESS: u32 = 0x00000002;
        const INVALID_HANDLE_VALUE: isize = -1;

        extern "system" {
            fn CreateToolhelp32Snapshot(flags: u32, pid: u32) -> isize;
            fn Process32FirstW(snapshot: isize, entry: *mut ProcessEntry32W) -> i32;
            fn Process32NextW(snapshot: isize, entry: *mut ProcessEntry32W) -> i32;
            fn CloseHandle(handle: isize) -> i32;
            fn GetCurrentProcessId() -> u32;
        }

        let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
        if snapshot == INVALID_HANDLE_VALUE {
            return Ok(Value::Int(0));
        }

        let mut entry: ProcessEntry32W = unsafe { std::mem::zeroed() };
        entry.dw_size = std::mem::size_of::<ProcessEntry32W>() as u32;

        let my_pid = unsafe { GetCurrentProcessId() };
        let mut ppid: i32 = 0;

        if unsafe { Process32FirstW(snapshot, &mut entry) } != 0 {
            loop {
                if entry.th32_process_id == my_pid {
                    ppid = entry.th32_parent_process_id as i32;
                    break;
                }
                if unsafe { Process32NextW(snapshot, &mut entry) } == 0 {
                    break;
                }
            }
        }

        unsafe { CloseHandle(snapshot) };
        Ok(Value::Int(ppid))
    }
}

pub(crate) fn native_os_strerror(args: &[Value]) -> Result<Value, String> {
    let code = match args.first() {
        Some(Value::Int(c)) => *c,
        _ => return Err("Os_strerror: expected an Int error code".to_string()),
    };
    #[cfg(unix)]
    {
        let ptr = unsafe { libc::strerror(code as libc::c_int) };
        if ptr.is_null() {
            Ok(Value::String(Rc::new(format!("error {}", code))))
        } else {
            let msg = unsafe { std::ffi::CStr::from_ptr(ptr) }
                .to_string_lossy()
                .to_string();
            Ok(Value::String(Rc::new(msg)))
        }
    }
    #[cfg(not(unix))]
    {
        Ok(Value::String(Rc::new(format!("error {}", code))))
    }
}

pub(crate) fn native_os_removedirs(args: &[Value]) -> Result<Value, String> {
    let path = match args.first() {
        Some(Value::String(s)) => s.as_str().to_string(),
        _ => return Err("Os_removedirs: expected a String path".to_string()),
    };
    // Remove directory and all parent directories that become empty
    let mut current = std::path::PathBuf::from(&path);
    loop {
        match std::fs::remove_dir(&current) {
            Ok(()) => {
                if !current.pop() { break; }
            }
            Err(_) => break,
        }
    }
    Ok(Value::Void)
}

pub(crate) fn native_os_renames(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Os_renames: expected old and new paths".to_string());
    }
    let old = match &args[0] {
        Value::String(s) => s.as_str(),
        _ => return Err("Os_renames: old path must be a String".to_string()),
    };
    let new = match &args[1] {
        Value::String(s) => s.as_str(),
        _ => return Err("Os_renames: new path must be a String".to_string()),
    };
    // Create parent directories of new path
    if let Some(parent) = std::path::Path::new(new).parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::rename(old, new)
        .map_err(|e| format!("Os_renames: {}", e))?;
    Ok(Value::Void)
}

pub(crate) fn native_os_replace(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Os_replace: expected src and dst paths".to_string());
    }
    let src = match &args[0] {
        Value::String(s) => s.as_str(),
        _ => return Err("Os_replace: src must be a String".to_string()),
    };
    let dst = match &args[1] {
        Value::String(s) => s.as_str(),
        _ => return Err("Os_replace: dst must be a String".to_string()),
    };
    std::fs::rename(src, dst)
        .map_err(|e| format!("Os_replace: {}", e))?;
    Ok(Value::Void)
}

pub(crate) fn native_os_link(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Os_link: expected src and dst paths".to_string());
    }
    let src = match &args[0] {
        Value::String(s) => s.as_str(),
        _ => return Err("Os_link: src must be a String".to_string()),
    };
    let dst = match &args[1] {
        Value::String(s) => s.as_str(),
        _ => return Err("Os_link: dst must be a String".to_string()),
    };
    std::fs::hard_link(src, dst)
        .map_err(|e| format!("Os_link: {}", e))?;
    Ok(Value::Void)
}

pub(crate) fn native_os_utime(args: &[Value]) -> Result<Value, String> {
    let path = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("Os_utime: expected a String path".to_string()),
    };
    let mtime_secs = match args.get(2) {
        Some(Value::Long(t)) => *t,
        Some(Value::Int(t)) => *t as i64,
        _ => return Err("Os_utime: expected mtime as Long (seconds since epoch)".to_string()),
    };
    let mtime = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(mtime_secs as u64);
    let file = std::fs::File::open(path)
        .map_err(|e| format!("Os_utime: cannot open '{}': {}", path, e))?;
    file.set_modified(mtime)
        .map_err(|e| format!("Os_utime: cannot set mtime on '{}': {}", path, e))?;
    Ok(Value::Void)
}

pub(crate) fn native_os_lstat(args: &[Value]) -> Result<Value, String> {
    // Return stat info as an array: [size, isFile, isDir, isSymlink]
    let path = match args.first() {
        Some(Value::String(s)) => s.as_str().to_string(),
        _ => return Err("Os_lstat: expected a String path".to_string()),
    };
    let metadata = std::fs::symlink_metadata(&path)
        .map_err(|e| format!("Os_lstat: {}", e))?;
    Ok(Value::Array {
        elements: vec![
            Value::Long(metadata.len() as i64),
            Value::Bool(metadata.is_file()),
            Value::Bool(metadata.is_dir()),
            Value::Bool(metadata.is_symlink()),
        ],
    })
}

pub(crate) fn native_os_access(args: &[Value]) -> Result<Value, String> {
    let path = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("Os_access: expected a String path".to_string()),
    };
    let mode = match args.get(1) {
        Some(Value::Int(m)) => *m,
        Some(Value::Long(m)) => *m as i32,
        _ => 0, // F_OK = 0 (existence check)
    };
    let p = std::path::Path::new(path);
    #[cfg(unix)]
    {
        // Use libc::access for real permission checking on Unix
        use std::ffi::CString;
        let c_path = match CString::new(path) {
            Ok(s) => s,
            Err(_) => return Ok(Value::Bool(false)),
        };
        let c_mode = match mode {
            0 => libc::F_OK,
            1 => libc::X_OK,
            2 => libc::W_OK,
            4 => libc::R_OK,
            _ => libc::F_OK,
        };
        let ret = unsafe { libc::access(c_path.as_ptr(), c_mode) };
        Ok(Value::Bool(ret == 0))
    }
    #[cfg(not(unix))]
    {
        let result = match mode {
            0 => p.exists(), // F_OK
            1 => p.exists(), // X_OK (simplified on Windows)
            2 => { // W_OK — try opening for append
                std::fs::OpenOptions::new().append(true).open(p).is_ok()
            }
            4 => { // R_OK — try opening for read
                std::fs::File::open(p).is_ok()
            }
            _ => p.exists(),
        };
        Ok(Value::Bool(result))
    }
}

pub(crate) fn native_os_system(args: &[Value]) -> Result<Value, String> {
    let cmd = match args.first() {
        Some(Value::String(s)) => s.as_str().to_string(),
        _ => return Err("Os_system: expected a String command".to_string()),
    };
    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("cmd").args(["/C", &cmd]).status();
    #[cfg(not(target_os = "windows"))]
    let result = std::process::Command::new("sh").args(["-c", &cmd]).status();
    match result {
        Ok(status) => Ok(Value::Int(status.code().unwrap_or(-1))),
        Err(_) => Ok(Value::Int(-1)),
    }
}

pub(crate) fn native_os_uname(_args: &[Value]) -> Result<Value, String> {
    let info = format!(
        "{}|{}|{}|{}",
        std::env::consts::OS,
        std::env::consts::ARCH,
        env!("CARGO_PKG_VERSION"),
        "unknown"
    );
    Ok(Value::String(Rc::new(info)))
}

pub(crate) fn native_env_unset(args: &[Value]) -> Result<Value, String> {
    let key = match args.first() {
        Some(Value::String(s)) => s.as_str().to_string(),
        _ => return Err("Env_unset: expected a String key".to_string()),
    };
    std::env::remove_var(&key);
    Ok(Value::Void)
}

pub(crate) fn native_os_release(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::String(Rc::new(env!("CARGO_PKG_VERSION").to_string())))
}

pub(crate) fn native_os_version(_args: &[Value]) -> Result<Value, String> {
    let info = format!("{}|{}", std::env::consts::OS, std::env::consts::ARCH);
    Ok(Value::String(Rc::new(info)))
}

pub(crate) fn native_titrate_version(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::String(Rc::new(env!("CARGO_PKG_VERSION").to_string())))
}

pub(crate) fn native_gc_collect(_args: &[Value]) -> Result<Value, String> {
    // GC collection hint - in a GC language this is typically a no-op or suggestion
    Ok(Value::Void)
}

pub(crate) fn native_fs_total_space(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Fs_totalSpace: expected 1 argument (path)".to_string());
    }
    let path = match &args[0] {
        Value::String(s) => s.as_str(),
        _ => return Err("Fs_totalSpace: expected String argument".to_string()),
    };
    #[cfg(unix)]
    {
        let c_path = std::ffi::CString::new(path)
            .map_err(|e| format!("Fs_totalSpace: invalid path: {}", e))?;
        let mut statvfs_buf: libc::statvfs = unsafe { std::mem::zeroed() };
        let ret = unsafe { libc::statvfs(c_path.as_ptr(), &mut statvfs_buf) };
        if ret != 0 {
            return Ok(Value::Long(0));
        }
        let total = statvfs_buf.f_blocks as u64 * statvfs_buf.f_frsize as u64;
        Ok(Value::Long(total as i64))
    }
    #[cfg(not(unix))]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        extern "system" {
            fn GetDiskFreeSpaceExW(
                lpDirectoryName: *const u16,
                lpFreeBytesAvailable: *mut u64,
                lpTotalNumberOfBytes: *mut u64,
                lpTotalNumberOfFreeBytes: *mut u64,
            ) -> i32;
        }

        let wide: Vec<u16> = OsStr::new(path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let mut free_bytes_available: u64 = 0;
        let mut total_bytes: u64 = 0;
        let mut total_free_bytes: u64 = 0;

        let ret = unsafe {
            GetDiskFreeSpaceExW(
                wide.as_ptr(),
                &mut free_bytes_available,
                &mut total_bytes,
                &mut total_free_bytes,
            )
        };

        if ret == 0 {
            return Ok(Value::Long(0));
        }
        Ok(Value::Long(total_bytes as i64))
    }
}

pub(crate) fn native_fs_free_space(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Fs_freeSpace: expected 1 argument (path)".to_string());
    }
    let path = match &args[0] {
        Value::String(s) => s.as_str(),
        _ => return Err("Fs_freeSpace: expected String argument".to_string()),
    };
    #[cfg(unix)]
    {
        let c_path = std::ffi::CString::new(path)
            .map_err(|e| format!("Fs_freeSpace: invalid path: {}", e))?;
        let mut statvfs_buf: libc::statvfs = unsafe { std::mem::zeroed() };
        let ret = unsafe { libc::statvfs(c_path.as_ptr(), &mut statvfs_buf) };
        if ret != 0 {
            return Ok(Value::Long(0));
        }
        let free = statvfs_buf.f_bavail as u64 * statvfs_buf.f_frsize as u64;
        Ok(Value::Long(free as i64))
    }
    #[cfg(not(unix))]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        extern "system" {
            fn GetDiskFreeSpaceExW(
                lpDirectoryName: *const u16,
                lpFreeBytesAvailable: *mut u64,
                lpTotalNumberOfBytes: *mut u64,
                lpTotalNumberOfFreeBytes: *mut u64,
            ) -> i32;
        }

        let wide: Vec<u16> = OsStr::new(path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let mut free_bytes_available: u64 = 0;
        let mut total_bytes: u64 = 0;
        let mut total_free_bytes: u64 = 0;

        let ret = unsafe {
            GetDiskFreeSpaceExW(
                wide.as_ptr(),
                &mut free_bytes_available,
                &mut total_bytes,
                &mut total_free_bytes,
            )
        };

        if ret == 0 {
            return Ok(Value::Long(0));
        }
        Ok(Value::Long(total_free_bytes as i64))
    }
}
