// Titrate Alpha 0.2 – bytecode virtual machine: system natives
// Precision in every step – richie-rich90454, 2026

use super::super::super::value::Value;
use std::rc::Rc;

pub(crate) fn native_sys_args(args: &[Value]) -> Result<Value, String> {
    // The CLI sets the program arguments (program name + user arguments,
    // excluding the script path) via `set_program_args` before running the VM.
    // Fall back to the process argv minus the executable and script path when
    // the VM is driven directly (e.g. from tests).
    let _ = args;
    let elements: Vec<Value> = if super::get_program_args().is_empty() {
        std::env::args()
            .skip(2)
            .map(|a| Value::String(Rc::new(a)))
            .collect()
    } else {
        super::get_program_args()
            .into_iter()
            .map(|a| Value::String(Rc::new(a)))
            .collect()
    };
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
        Value::String(path) => {
            let resolved = super::resolve_path(path.as_str());
            Ok(Value::Bool(resolved.exists()))
        }
        _ => Err("Fs_exists: expected String argument".to_string()),
    }
}

pub(crate) fn native_fs_is_file(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Fs_isFile: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => {
            let resolved = super::resolve_path(path.as_str());
            Ok(Value::Bool(resolved.is_file()))
        }
        _ => Err("Fs_isFile: expected String argument".to_string()),
    }
}

pub(crate) fn native_fs_is_dir(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Fs_isDir: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => {
            let resolved = super::resolve_path(path.as_str());
            Ok(Value::Bool(resolved.is_dir()))
        }
        _ => Err("Fs_isDir: expected String argument".to_string()),
    }
}

pub(crate) fn native_fs_size(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Fs_size: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => {
            let resolved = super::resolve_path(path.as_str());
            match std::fs::metadata(&resolved) {
                Ok(meta) => Ok(Value::Long(meta.len() as i64)),
                Err(e) => Err(format!("Fs_size: {}", e)),
            }
        }
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
        Some(Value::Int(s)) => *s,
        Some(Value::Long(s)) => *s as i32,
        _ => return Err("Signal_register: expected a signal number".to_string()),
    };

    // SIG_ERR is defined as ((_sig_func)-1), i.e. all-ones pointer.
    const SIG_ERR: usize = usize::MAX;

    // SAFETY: `signal` is an extern "C" function from the C standard library.
    // `signum` is an i32 obtained from a Titrate Int/Long value, and
    // `signal_handler` is an `extern "C" fn(i32)` stored in a static and used
    // only as an async-signal-safe flag-setter. The call matches the POSIX
    // `signal(2)` contract: it installs the handler atomically and returns the
    // previous handler (or SIG_ERR on failure, which we check below).
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
        Some(Value::Int(s)) => *s,
        Some(Value::Long(s)) => *s as i32,
        _ => return Err("Signal_raise: expected a signal number".to_string()),
    };

    // SAFETY: `raise` is an extern "C" function from the C standard library.
    // `signum` is an i32 derived from a Titrate Int/Long value. The call
    // satisfies the POSIX `raise(2)` contract — it sends the signal to the
    // calling process and returns 0 on success, non-zero on error (checked).
    let rc = unsafe { raise(signum) };
    if rc != 0 {
        return Err(format!("Signal_raise: failed to raise signal {}", signum));
    }

    Ok(Value::Int(0))
}

// ---------------------------------------------------------------------------
// Additional Os natives
// ---------------------------------------------------------------------------

