// Native implementations for the POSIX OS primitives (Fcntl, Termios, Pty,
// Resource). On Unix these delegate to libc syscalls. On Windows the POSIX
// syscall does not exist (and the winapi features enabled for this crate do
// not expose a substitute), so the calls are forwarded to the original
// platform.rs stubs, which return explicit "unsupported" errors.
//
// The forward calls are intentional: they keep the platform.rs natives
// referenced so the crate builds without dead-code warnings.

use super::super::super::value::Value;

#[cfg(unix)]
fn arg_i64(args: &[Value], idx: usize, what: &str) -> Result<i64, String> {
    match args.get(idx).and_then(|v| v.to_i64()) {
        Some(v) => Ok(v),
        None => Err(format!("{}: expected an Int/Long argument {}", what, idx)),
    }
}

/// Wrap values in an ArrayList-shaped instance so Titrate can use .get(i).
#[cfg(unix)]
fn int_list(elements: Vec<Value>) -> Value {
    let mut fields = std::collections::HashMap::new();
    fields.insert("_elements".to_string(), Value::Array { elements });
    Value::ClassInstance {
        class_name: "ArrayList".to_string(),
        fields: std::rc::Rc::new(std::cell::RefCell::new(fields)),
        vtable: std::collections::HashMap::new(),
    }
}

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

/// Resource_getrlimit(resource) -> [soft, hard]
pub(crate) fn native_resource_getrlimit(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let resource = arg_i64(args, 0, "Resource_getrlimit")?;
        let mut rlim: libc::rlimit = unsafe { std::mem::zeroed() };
        let rc = unsafe { libc::getrlimit(resource as _, &mut rlim) };
        if rc != 0 {
            return Ok(int_list(vec![Value::Long(-1), Value::Long(-1)]));
        }
        let soft = if rlim.rlim_cur == libc::RLIM_INFINITY {
            -1
        } else {
            rlim.rlim_cur as i64
        };
        let hard = if rlim.rlim_max == libc::RLIM_INFINITY {
            -1
        } else {
            rlim.rlim_max as i64
        };
        Ok(int_list(vec![Value::Long(soft), Value::Long(hard)]))
    }
    #[cfg(not(unix))]
    {
        super::platform::native_resource_getrlimit(args)
    }
}

/// Resource_setrlimit(resource, soft, hard) -> Int (0 on success, -1 on error)
pub(crate) fn native_resource_setrlimit(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let resource = arg_i64(args, 0, "Resource_setrlimit")?;
        let soft = arg_i64(args, 1, "Resource_setrlimit")?;
        let hard = arg_i64(args, 2, "Resource_setrlimit")?;
        let mut rlim: libc::rlimit = unsafe { std::mem::zeroed() };
        rlim.rlim_cur = if soft < 0 {
            libc::RLIM_INFINITY
        } else {
            soft as libc::rlim_t
        };
        rlim.rlim_max = if hard < 0 {
            libc::RLIM_INFINITY
        } else {
            hard as libc::rlim_t
        };
        let rc = unsafe { libc::setrlimit(resource as _, &rlim) };
        Ok(Value::Int(if rc == 0 { 0 } else { -1 }))
    }
    #[cfg(not(unix))]
    {
        super::platform::native_resource_setrlimit(args)
    }
}

/// Resource_getrusage(who) -> list of 16 fields:
/// (utime, stime, maxrss, ixrss, idrss, isrss, minflt, majflt, nswap,
///  inblock, oublock, msgsnd, msgrcv, nsignals, nvcsw, nivcsw)
pub(crate) fn native_resource_getrusage(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let who = arg_i64(args, 0, "Resource_getrusage")? as libc::c_int;
        let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
        let zeros = || -> Value {
            int_list(vec![
                Value::Double(0.0), Value::Double(0.0),
                Value::Long(0), Value::Long(0), Value::Long(0), Value::Long(0),
                Value::Long(0), Value::Long(0), Value::Long(0),
                Value::Long(0), Value::Long(0),
                Value::Long(0), Value::Long(0), Value::Long(0),
                Value::Long(0), Value::Long(0),
            ])
        };
        if unsafe { libc::getrusage(who, &mut usage) } != 0 {
            return Ok(zeros());
        }
        let l = |v: libc::c_long| Value::Long(v);
        Ok(int_list(vec![
            Value::Double(usage.ru_utime.tv_sec as f64 + usage.ru_utime.tv_usec as f64 / 1_000_000.0),
            Value::Double(usage.ru_stime.tv_sec as f64 + usage.ru_stime.tv_usec as f64 / 1_000_000.0),
            l(usage.ru_maxrss), l(usage.ru_ixrss), l(usage.ru_idrss), l(usage.ru_isrss),
            l(usage.ru_minflt), l(usage.ru_majflt), l(usage.ru_nswap),
            l(usage.ru_inblock), l(usage.ru_oublock),
            l(usage.ru_msgsnd), l(usage.ru_msgrcv), l(usage.ru_nsignals),
            l(usage.ru_nvcsw), l(usage.ru_nivcsw),
        ]))
    }
    #[cfg(not(unix))]
    {
        super::platform::native_resource_getrusage(args)
    }
}

// ---------------------------------------------------------------------------
// Fcntl
// ---------------------------------------------------------------------------

/// Fcntl_fcntl(fd, cmd, [arg]) -> Int (raw syscall result)
pub(crate) fn native_fcntl_fcntl(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let fd = arg_i64(args, 0, "Fcntl_fcntl")? as libc::c_int;
        let cmd = arg_i64(args, 1, "Fcntl_fcntl")? as libc::c_int;
        let arg = args.get(2).and_then(|v| v.to_i64()).unwrap_or(0) as libc::c_int;
        let rc = unsafe { libc::fcntl(fd, cmd, arg) };
        Ok(Value::Int(rc))
    }
    #[cfg(not(unix))]
    {
        super::platform::native_fcntl_fcntl(args)
    }
}

/// Fcntl_ioctl(fd, request, [arg]) -> Int (raw syscall result)
pub(crate) fn native_fcntl_ioctl(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let fd = arg_i64(args, 0, "Fcntl_ioctl")? as libc::c_int;
        let request = arg_i64(args, 1, "Fcntl_ioctl")?;
        let arg = args.get(2).and_then(|v| v.to_i64()).unwrap_or(0) as libc::c_int;
        let rc = unsafe { libc::ioctl(fd, request as _, arg) };
        Ok(Value::Int(rc))
    }
    #[cfg(not(unix))]
    {
        super::platform::native_fcntl_ioctl(args)
    }
}

/// Fcntl_flock(fd, operation) -> Int (0 on success, -1 on error)
pub(crate) fn native_fcntl_flock(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let fd = arg_i64(args, 0, "Fcntl_flock")? as libc::c_int;
        let operation = arg_i64(args, 1, "Fcntl_flock")? as libc::c_int;
        let rc = unsafe { libc::flock(fd, operation) };
        Ok(Value::Int(rc))
    }
    #[cfg(not(unix))]
    {
        super::platform::native_fcntl_flock(args)
    }
}

/// Fcntl_lockf(fd, operation, length, start, whence) -> Int
/// `operation` uses the flock-style flags (LOCK_EX/SH/UN/NB). lockf(2) has
/// its own command set, so the flags are mapped to F_ULOCK/F_LOCK/F_TLOCK.
pub(crate) fn native_fcntl_lockf(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let fd = arg_i64(args, 0, "Fcntl_lockf")? as libc::c_int;
        let operation = arg_i64(args, 1, "Fcntl_lockf")? as libc::c_int;
        let length = arg_i64(args, 2, "Fcntl_lockf")?;
        let start = args.get(3).and_then(|v| v.to_i64()).unwrap_or(0);
        let whence = args.get(4).and_then(|v| v.to_i64()).unwrap_or(0) as libc::c_int;
        if (start != 0 || whence != 0)
            && unsafe { libc::lseek(fd, start as libc::off_t, whence) } < 0
        {
            return Ok(Value::Int(-1));
        }
        let cmd = if operation & 8 != 0 {
            0 // F_ULOCK
        } else if operation & 4 != 0 {
            2 // F_TLOCK
        } else {
            1 // F_LOCK
        };
        let rc = unsafe { libc::lockf(fd, cmd, length as libc::off_t) };
        Ok(Value::Int(rc))
    }
    #[cfg(not(unix))]
    {
        super::platform::native_fcntl_lockf(args)
    }
}

// ---------------------------------------------------------------------------
// Termios helpers
// ---------------------------------------------------------------------------

#[cfg(unix)]
fn int_field(fields: &std::collections::HashMap<String, Value>, name: &str) -> libc::tcflag_t {
    fields
        .get(name)
        .and_then(|v| v.to_i64())
        .unwrap_or(0) as libc::tcflag_t
}

/// Read the 17-entry `cc` ArrayList stored on a TermiosAttr ClassInstance.
#[cfg(unix)]
fn cc_field(fields: &std::collections::HashMap<String, Value>) -> Vec<u8> {
    let mut out = vec![0u8; 17];
    if let Some(Value::ClassInstance { fields: al, .. }) = fields.get("cc") {
        if let Some(Value::Array { elements }) = al.borrow().get("_elements") {
            for (i, v) in elements.iter().take(17).enumerate() {
                out[i] = v.to_i64().unwrap_or(0) as u8;
            }
        }
    }
    out
}

/// Build an ArrayList-shaped ClassInstance carrying the raw termios words.
#[cfg(unix)]
fn termios_words(t: &libc::termios) -> Value {
    let mut words: Vec<Value> = Vec::with_capacity(23);
    words.push(Value::Int(t.c_iflag as i32));
    words.push(Value::Int(t.c_oflag as i32));
    words.push(Value::Int(t.c_cflag as i32));
    words.push(Value::Int(t.c_lflag as i32));
    words.push(Value::Int(unsafe { libc::cfgetispeed(t) } as i32));
    words.push(Value::Int(unsafe { libc::cfgetospeed(t) } as i32));
    for i in 0..17 {
        words.push(Value::Int(t.c_cc[i] as i32));
    }
    int_list(words)
}

/// Termios_tcgetattr(fd) -> ArrayList of 23 raw integer words.
pub(crate) fn native_termios_tcgetattr(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let fd = arg_i64(args, 0, "Termios_tcgetattr")? as libc::c_int;
        let mut t: libc::termios = unsafe { std::mem::zeroed() };
        if unsafe { libc::tcgetattr(fd, &mut t) } != 0 {
            return Ok(termios_words(&t));
        }
        Ok(termios_words(&t))
    }
    #[cfg(not(unix))]
    {
        super::platform::native_termios_tcgetattr(args)
    }
}

/// Termios_tcsetattr(fd, when, attr: TermiosAttr) -> Int
pub(crate) fn native_termios_tcsetattr(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let fd = arg_i64(args, 0, "Termios_tcsetattr")? as libc::c_int;
        let when = arg_i64(args, 1, "Termios_tcsetattr")? as libc::c_int;
        let (iflag, oflag, cflag, lflag, ispeed, ospeed, cc) = match args.get(2) {
            Some(Value::ClassInstance { fields, .. }) => {
                let f = fields.borrow();
                (
                    int_field(&f, "iflag"),
                    int_field(&f, "oflag"),
                    int_field(&f, "cflag"),
                    int_field(&f, "lflag"),
                    int_field(&f, "ispeed") as libc::speed_t,
                    int_field(&f, "ospeed") as libc::speed_t,
                    cc_field(&f),
                )
            }
            _ => return Err("Termios_tcsetattr: expected a TermiosAttr argument".to_string()),
        };
        // Start from the live state so fields not exposed by TermiosAttr
        // (line discipline, platform-specific cc slots) are preserved.
        let mut t: libc::termios = unsafe { std::mem::zeroed() };
        let _ = unsafe { libc::tcgetattr(fd, &mut t) };
        t.c_iflag = iflag;
        t.c_oflag = oflag;
        t.c_cflag = cflag;
        t.c_lflag = lflag;
        for (i, b) in cc.iter().enumerate().take(libc::NCCS) {
            t.c_cc[i] = *b;
        }
        unsafe {
            libc::cfsetispeed(&mut t, ispeed);
            libc::cfsetospeed(&mut t, ospeed);
        }
        let rc = unsafe { libc::tcsetattr(fd, when, &t) };
        Ok(Value::Int(if rc == 0 { 0 } else { -1 }))
    }
    #[cfg(not(unix))]
    {
        super::platform::native_termios_tcsetattr(args)
    }
}

macro_rules! termios_int_native {
    ($name:ident, $platform:ident, $label:literal, $call:expr) => {
        pub(crate) fn $name(args: &[Value]) -> Result<Value, String> {
            #[cfg(unix)]
            {
                let fd = arg_i64(args, 0, $label)? as libc::c_int;
                let extra = args.get(1).and_then(|v| v.to_i64()).unwrap_or(0) as libc::c_int;
                let rc = unsafe { $call(fd, extra) };
                Ok(Value::Int(rc))
            }
            #[cfg(not(unix))]
            {
                super::platform::$platform(args)
            }
        }
    };
}

termios_int_native!(native_termios_tcdrain, native_termios_tcdrain, "Termios_tcdrain",
    |fd: libc::c_int, _extra: libc::c_int| libc::tcdrain(fd));
termios_int_native!(native_termios_tcflush, native_termios_tcflush, "Termios_tcflush",
    |fd: libc::c_int, extra: libc::c_int| libc::tcflush(fd, extra));
termios_int_native!(native_termios_tcsendbreak, native_termios_tcsendbreak, "Termios_tcsendbreak",
    |fd: libc::c_int, extra: libc::c_int| libc::tcsendbreak(fd, extra));

// ---------------------------------------------------------------------------
// Pty
// ---------------------------------------------------------------------------

/// Pty_openpty() -> [masterFd, slaveFd]; both are -1 on failure.
pub(crate) fn native_pty_openpty(_args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let fail = || int_list(vec![Value::Long(-1), Value::Long(-1)]);
        let master = unsafe { libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY) };
        if master < 0 {
            return Ok(fail());
        }
        if unsafe { libc::grantpt(master) } != 0 || unsafe { libc::unlockpt(master) } != 0 {
            unsafe { libc::close(master) };
            return Ok(fail());
        }
        let name = unsafe { libc::ptsname(master) };
        if name.is_null() {
            unsafe { libc::close(master) };
            return Ok(fail());
        }
        let slave = unsafe {
            libc::open(name, libc::O_RDWR | libc::O_NOCTTY)
        };
        if slave < 0 {
            unsafe { libc::close(master) };
            return Ok(fail());
        }
        Ok(int_list(vec![
            Value::Long(master as i64),
            Value::Long(slave as i64),
        ]))
    }
    #[cfg(not(unix))]
    {
        super::platform::native_pty_openpty(args)
    }
}

/// Pty_close(fd) -> Int (0 on success, -1 on error).
pub(crate) fn native_pty_close(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let fd = arg_i64(args, 0, "Pty_close")? as libc::c_int;
        let rc = unsafe { libc::close(fd) };
        Ok(Value::Int(if rc == 0 { 0 } else { -1 }))
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Ok(Value::Int(0))
    }
}

/// Termios_tcgetpgrp(fd) -> foreground process group id, or -1.
/// Registered under a new name; there is no platform.rs stub to delegate to.
pub(crate) fn native_termios_tcgetpgrp(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let fd = arg_i64(args, 0, "Termios_tcgetpgrp")? as libc::c_int;
        Ok(Value::Int(unsafe { libc::tcgetpgrp(fd) }))
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Ok(Value::Int(-1))
    }
}

/// Termios_tcsetpgrp(fd, pgid) -> Int (0 on success, -1 on error).
pub(crate) fn native_termios_tcsetpgrp(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let fd = arg_i64(args, 0, "Termios_tcsetpgrp")? as libc::c_int;
        let pgid = arg_i64(args, 1, "Termios_tcsetpgrp")? as libc::pid_t;
        let rc = unsafe { libc::tcsetpgrp(fd, pgid) };
        Ok(Value::Int(if rc == 0 { 0 } else { -1 }))
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Ok(Value::Int(-1))
    }
}

// ---------------------------------------------------------------------------
// Fork (Os_fork)
// ---------------------------------------------------------------------------

/// Count OS threads from the kernel's own accounting. Forking a multithreaded
/// process can deadlock: libc locks held by other threads at fork time are
/// copied into the child already locked, so the child hangs on its first
/// allocation. Linux exposes the count in /proc; where it cannot be verified,
/// return None so the caller refuses rather than risk a deadlock.
#[cfg(unix)]
fn os_thread_count() -> Option<usize> {
    #[cfg(target_os = "linux")]
    {
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if let Some(rest) = line.strip_prefix("Threads:") {
                    if let Ok(n) = rest.trim().parse::<usize>() {
                        return Some(n);
                    }
                }
            }
        }
        Some(1)
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

/// Os_fork() -> Int
///
/// POSIX fork: returns the child's pid in the parent, 0 in the child, and -1 on
/// failure or refusal. The child resumes the same bytecode program at the call
/// site.
///
/// Inherited: bytecode/call stack, heap, all file descriptors (sharing open
/// file descriptions and offsets with the parent), environment and signal
/// dispositions. Not inherited: other threads (only the calling thread
/// survives), and in-process native-handle registry entries whose OS resource
/// lives on another thread. Each process has its own fd table, so closing a
/// duplicated descriptor in one does not close it in the other.
pub(crate) fn native_os_fork(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let vm_threads = super::thread::active_vm_threads();
    #[cfg(unix)]
    {
        if vm_threads > 0 || os_thread_count().is_none_or(|n| n > 1) {
            eprintln!(
                "Os_fork: refusing to fork with {} live VM thread(s) or an \
                 unverifiable process thread count; forking a multithreaded \
                 process can deadlock (returning -1)",
                vm_threads
            );
            return Ok(Value::Int(-1));
        }
        // SAFETY: the guard above guarantees this process is single-threaded,
        // so no libc lock can be held by another thread across the fork.
        let pid = unsafe { libc::fork() };
        Ok(Value::Int(pid))
    }
    #[cfg(not(unix))]
    {
        let _ = vm_threads;
        // Windows has no fork(2): CreateProcess creates a new image-based
        // process, not a snapshot of the caller's address space, so POSIX fork
        // semantics cannot be reproduced here.
        eprintln!("Os_fork: POSIX fork is not available on Windows (returning -1)");
        Ok(Value::Int(-1))
    }
}

// The platform.rs stubs remain registered as native entry points even though
// the OS primitives now dispatch here. Keep them referenced on Unix so the
// crate stays warning-free there as well as on Windows (where the functions
// above delegate to them directly).
#[cfg(unix)]
#[allow(dead_code)]
fn reference_platform_stubs() {
    use super::platform;
    type Native = fn(&[Value]) -> Result<Value, String>;
    let _ = platform::native_fcntl_fcntl as Native;
    let _ = platform::native_fcntl_ioctl as Native;
    let _ = platform::native_fcntl_flock as Native;
    let _ = platform::native_fcntl_lockf as Native;
    let _ = platform::native_termios_tcgetattr as Native;
    let _ = platform::native_termios_tcsetattr as Native;
    let _ = platform::native_termios_tcdrain as Native;
    let _ = platform::native_termios_tcflush as Native;
    let _ = platform::native_termios_tcsendbreak as Native;
    let _ = platform::native_pty_openpty as Native;
    let _ = platform::native_resource_getrlimit as Native;
    let _ = platform::native_resource_setrlimit as Native;
    let _ = platform::native_resource_getrusage as Native;
}
