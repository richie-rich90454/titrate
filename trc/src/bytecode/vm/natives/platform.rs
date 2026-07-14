// Titrate Alpha 0.2 – bytecode virtual machine: platform-specific natives
// Precision in every step – richie-rich90454, 2026
//
// Native support for the platform-specific stdlib modules:
//   - Windows: tt::sys::WinReg (registry), tt::sys::WinSound (beep/PCM)
//   - Unix:    tt::sys::Fcntl, tt::sys::Termios, tt::sys::Pty,
//              tt::sys::Syslog, tt::sys::Resource
//
// Each function is compiled on every platform so the lookup table can
// reference it unconditionally. On the native platform the function returns
// a stub success value (real implementations require the `winapi`/`windows`
// crate on Windows or unsafe libc FFI on Unix, neither of which is currently
// a dependency). On other platforms the function returns an error explaining
// the API is unavailable. The stub contract is sufficient for the stdlib
// modules to expose their full API surface.

use super::super::super::value::Value;

// ===========================================================================
// Windows Registry (WinReg_*)
// ===========================================================================

/// WinReg_OpenKey(parentHandle, subKey, [reserved], [access]) -> Long handle
pub(crate) fn native_winreg_open_key(args: &[Value]) -> Result<Value, String> {
    #[cfg(windows)]
    {
        let _ = args;
        Ok(Value::Long(1))
    }
    #[cfg(not(windows))]
    {
        let _ = args;
        Err("WinReg_OpenKey: Windows registry is only available on Windows".to_string())
    }
}

/// WinReg_CloseKey(handle) -> Null
pub(crate) fn native_winreg_close_key(args: &[Value]) -> Result<Value, String> {
    #[cfg(windows)]
    {
        let _ = args;
        Ok(Value::Null)
    }
    #[cfg(not(windows))]
    {
        let _ = args;
        Err("WinReg_CloseKey: Windows registry is only available on Windows".to_string())
    }
}

/// WinReg_QueryValue(handle, valueName) -> String
pub(crate) fn native_winreg_query_value(args: &[Value]) -> Result<Value, String> {
    #[cfg(windows)]
    {
        let _ = args;
        Ok(Value::String(std::rc::Rc::new(String::new())))
    }
    #[cfg(not(windows))]
    {
        let _ = args;
        Err("WinReg_QueryValue: Windows registry is only available on Windows".to_string())
    }
}

/// WinReg_SetValue(handle, subKey, [reserved], [type], value) -> Null
pub(crate) fn native_winreg_set_value(args: &[Value]) -> Result<Value, String> {
    #[cfg(windows)]
    {
        let _ = args;
        Ok(Value::Null)
    }
    #[cfg(not(windows))]
    {
        let _ = args;
        Err("WinReg_SetValue: Windows registry is only available on Windows".to_string())
    }
}

/// WinReg_EnumKey(handle, index) -> String
pub(crate) fn native_winreg_enum_key(args: &[Value]) -> Result<Value, String> {
    #[cfg(windows)]
    {
        let _ = args;
        Ok(Value::String(std::rc::Rc::new(String::new())))
    }
    #[cfg(not(windows))]
    {
        let _ = args;
        Err("WinReg_EnumKey: Windows registry is only available on Windows".to_string())
    }
}

/// WinReg_EnumValue(handle, index) -> String
pub(crate) fn native_winreg_enum_value(args: &[Value]) -> Result<Value, String> {
    #[cfg(windows)]
    {
        let _ = args;
        Ok(Value::String(std::rc::Rc::new(String::new())))
    }
    #[cfg(not(windows))]
    {
        let _ = args;
        Err("WinReg_EnumValue: Windows registry is only available on Windows".to_string())
    }
}

/// WinReg_CreateKey(parentHandle, subKey, [reserved], [access]) -> Long handle
pub(crate) fn native_winreg_create_key(args: &[Value]) -> Result<Value, String> {
    #[cfg(windows)]
    {
        let _ = args;
        Ok(Value::Long(1))
    }
    #[cfg(not(windows))]
    {
        let _ = args;
        Err("WinReg_CreateKey: Windows registry is only available on Windows".to_string())
    }
}

/// WinReg_DeleteKey(handle, subKey) -> Null
pub(crate) fn native_winreg_delete_key(args: &[Value]) -> Result<Value, String> {
    #[cfg(windows)]
    {
        let _ = args;
        Ok(Value::Null)
    }
    #[cfg(not(windows))]
    {
        let _ = args;
        Err("WinReg_DeleteKey: Windows registry is only available on Windows".to_string())
    }
}

/// WinReg_DeleteValue(handle, valueName) -> Null
pub(crate) fn native_winreg_delete_value(args: &[Value]) -> Result<Value, String> {
    #[cfg(windows)]
    {
        let _ = args;
        Ok(Value::Null)
    }
    #[cfg(not(windows))]
    {
        let _ = args;
        Err("WinReg_DeleteValue: Windows registry is only available on Windows".to_string())
    }
}

// ===========================================================================
// Windows Sound (WinSound_*)
// ===========================================================================

/// WinSound_Beep(frequency: Int, duration: Int) -> Bool
pub(crate) fn native_winsound_beep(args: &[Value]) -> Result<Value, String> {
    #[cfg(windows)]
    {
        let _ = args;
        Ok(Value::Bool(true))
    }
    #[cfg(not(windows))]
    {
        let _ = args;
        Err("WinSound_Beep: Windows sound is only available on Windows".to_string())
    }
}

/// WinSound_PlaySound(sound: String, flags: Int) -> Bool
pub(crate) fn native_winsound_play_sound(args: &[Value]) -> Result<Value, String> {
    #[cfg(windows)]
    {
        let _ = args;
        Ok(Value::Bool(true))
    }
    #[cfg(not(windows))]
    {
        let _ = args;
        Err("WinSound_PlaySound: Windows sound is only available on Windows".to_string())
    }
}

/// WinSound_MessageBeep(type: Int) -> Bool
pub(crate) fn native_winsound_message_beep(args: &[Value]) -> Result<Value, String> {
    #[cfg(windows)]
    {
        let _ = args;
        Ok(Value::Bool(true))
    }
    #[cfg(not(windows))]
    {
        let _ = args;
        Err("WinSound_MessageBeep: Windows sound is only available on Windows".to_string())
    }
}

// ===========================================================================
// Unix File Control (Fcntl_*)
// ===========================================================================

/// Fcntl_fcntl(fd: Int, cmd: Int, [arg]) -> Int
pub(crate) fn native_fcntl_fcntl(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Int(0))
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Fcntl_fcntl: Unix file control is only available on Unix".to_string())
    }
}

/// Fcntl_ioctl(fd: Int, request: Int, [arg]) -> Int
pub(crate) fn native_fcntl_ioctl(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Int(0))
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Fcntl_ioctl: Unix file control is only available on Unix".to_string())
    }
}

/// Fcntl_flock(fd: Int, operation: Int) -> Int
pub(crate) fn native_fcntl_flock(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Int(0))
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Fcntl_flock: Unix file control is only available on Unix".to_string())
    }
}

/// Fcntl_lockf(fd: Int, operation: Int, length: Long) -> Int
pub(crate) fn native_fcntl_lockf(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Int(0))
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Fcntl_lockf: Unix file control is only available on Unix".to_string())
    }
}

// ===========================================================================
// Unix Terminal Control (Termios_*)
// ===========================================================================

/// Termios_tcgetattr(fd: Int) -> Long handle
///
/// Returns a handle to a termios structure for the file descriptor. Stub
/// returns a fake handle; real implementation requires libc::tcgetattr.
pub(crate) fn native_termios_tcgetattr(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Long(1))
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Termios_tcgetattr: Unix terminal control is only available on Unix".to_string())
    }
}

/// Termios_tcsetattr(fd: Int, when: Int, handle: Long) -> Null
pub(crate) fn native_termios_tcsetattr(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Null)
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Termios_tcsetattr: Unix terminal control is only available on Unix".to_string())
    }
}

/// Termios_tcdrain(fd: Int) -> Null
pub(crate) fn native_termios_tcdrain(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Null)
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Termios_tcdrain: Unix terminal control is only available on Unix".to_string())
    }
}

/// Termios_tcflush(fd: Int, queue: Int) -> Null
pub(crate) fn native_termios_tcflush(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Null)
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Termios_tcflush: Unix terminal control is only available on Unix".to_string())
    }
}

/// Termios_tcsendbreak(fd: Int, duration: Int) -> Null
pub(crate) fn native_termios_tcsendbreak(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Null)
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Termios_tcsendbreak: Unix terminal control is only available on Unix".to_string())
    }
}

// ===========================================================================
// Unix Pseudo-Terminal (Pty_*)
// ===========================================================================

/// Pty_openpty() -> (masterFd: Long, slaveFd: Long)
///
/// Returns a tuple of (master_fd, slave_fd). Stub returns fake descriptors.
pub(crate) fn native_pty_openpty(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Tuple {
            elements: vec![Value::Long(3), Value::Long(4)],
        })
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Pty_openpty: Unix pseudo-terminal is only available on Unix".to_string())
    }
}

/// Pty_fork() -> Int pid
///
/// Stub returns a fake pid of 1 (parent side).
pub(crate) fn native_pty_fork(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Int(1))
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Pty_fork: Unix pseudo-terminal is only available on Unix".to_string())
    }
}

/// Pty_spawn(argv: Array) -> Int pid
///
/// Stub returns a fake pid of 1.
pub(crate) fn native_pty_spawn(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Int(1))
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Pty_spawn: Unix pseudo-terminal is only available on Unix".to_string())
    }
}

// ===========================================================================
// Unix Syslog (Syslog_*)
// ===========================================================================

/// Syslog_openlog(ident: String, option: Int, facility: Int) -> Null
pub(crate) fn native_syslog_openlog(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Null)
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Syslog_openlog: Unix syslog is only available on Unix".to_string())
    }
}

/// Syslog_syslog(priority: Int, message: String) -> Null
pub(crate) fn native_syslog_syslog(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Null)
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Syslog_syslog: Unix syslog is only available on Unix".to_string())
    }
}

/// Syslog_closelog() -> Null
pub(crate) fn native_syslog_closelog(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Null)
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Syslog_closelog: Unix syslog is only available on Unix".to_string())
    }
}

/// Syslog_setlogmask(mask: Int) -> Int
///
/// Returns the previous log mask. Stub returns 0.
pub(crate) fn native_syslog_setlogmask(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Int(0))
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Syslog_setlogmask: Unix syslog is only available on Unix".to_string())
    }
}

// ===========================================================================
// Unix Resource Limits (Resource_*)
// ===========================================================================

/// Resource_getrlimit(resource: Int) -> (soft: Long, hard: Long)
///
/// Returns a tuple of (soft_limit, hard_limit). Stub returns (-1, -1) to
/// represent RLIM_INFINITY.
pub(crate) fn native_resource_getrlimit(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Tuple {
            elements: vec![Value::Long(-1), Value::Long(-1)],
        })
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Resource_getrlimit: Unix resource limits are only available on Unix".to_string())
    }
}

/// Resource_setrlimit(resource: Int, soft: Long, hard: Long) -> Null
pub(crate) fn native_resource_setrlimit(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Null)
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Resource_setrlimit: Unix resource limits are only available on Unix".to_string())
    }
}

/// Resource_getrusage(who: Int) -> Long handle
///
/// Returns a handle to an rusage structure. Stub returns a fake handle.
pub(crate) fn native_resource_getrusage(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Long(1))
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Resource_getrusage: Unix resource limits are only available on Unix".to_string())
    }
}
