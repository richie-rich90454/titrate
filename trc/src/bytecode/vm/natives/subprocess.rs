// Titrate Alpha 0.2 – bytecode virtual machine: subprocess natives
// Precision in every step – richie-rich90454, 2026

use super::super::super::value::Value;
use std::collections::HashMap;
use std::rc::Rc;

pub(crate) fn native_subprocess_run(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Subprocess_run: expected at least 1 argument (command)".to_string());
    }
    let program = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Subprocess_run: expected String command".to_string()),
    };
    // When a single command string is passed, route it through the platform
    // shell so shell builtins (echo, dir, type, etc.) and shell syntax work.
    // When extra arguments are passed, treat the first as a program path and
    // the rest as argv (no shell interpretation).
    let mut cmd = if args.len() == 1 {
        let (shell, shell_arg) = if cfg!(target_os = "windows") {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };
        let mut c = std::process::Command::new(shell);
        c.arg(shell_arg).arg(&program);
        c
    } else {
        let mut c = std::process::Command::new(&program);
        for arg in &args[1..] {
            match arg {
                Value::String(s) => { c.arg(s.as_str()); }
                other => { c.arg(format!("{:?}", other)); }
            }
        }
        c
    };
    let status = cmd.status()
        .map_err(|e| format!("Subprocess_run: failed to execute '{}': {}", program, e))?;
    Ok(Value::Int(status.code().unwrap_or(-1)))
}

pub(crate) fn native_subprocess_exec(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Subprocess_exec: expected at least 1 argument (command)".to_string());
    }
    let program = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Subprocess_exec: expected String command".to_string()),
    };
    // When a single command string is passed, route it through the platform
    // shell so shell builtins (echo, dir, type, etc.) and shell syntax work.
    // When extra arguments are passed, treat the first as a program path and
    // the rest as argv (no shell interpretation).
    let mut cmd = if args.len() == 1 {
        let (shell, shell_arg) = if cfg!(target_os = "windows") {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };
        let mut c = std::process::Command::new(shell);
        c.arg(shell_arg).arg(&program);
        c
    } else {
        let mut c = std::process::Command::new(&program);
        for arg in &args[1..] {
            match arg {
                Value::String(s) => { c.arg(s.as_str()); }
                other => { c.arg(format!("{:?}", other)); }
            }
        }
        c
    };
    let output = cmd.output()
        .map_err(|e| format!("Subprocess_exec: failed to execute '{}': {}", program, e))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(Value::String(Rc::new(stdout)))
}

/// Pipe input data to a command's stdin and return captured stdout.
/// Args: (command: String, [args...], input: String) — the last string arg is
/// treated as stdin input when it is preceded by at least one command arg.
/// Simpler contract: (command: String, input: String).
pub(crate) fn native_subprocess_popen_write(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Subprocess_popenWrite: expected at least 2 arguments (command, input)".to_string());
    }
    let program = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Subprocess_popenWrite: expected String command".to_string()),
    };
    let input = match &args[1] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Subprocess_popenWrite: expected String input".to_string()),
    };
    // On Windows, use cmd /c for shell-like behaviour; on Unix, use sh -c.
    let (shell, shell_arg) = if cfg!(target_os = "windows") {
        ("cmd", "/C")
    } else {
        ("sh", "-c")
    };
    let mut cmd = std::process::Command::new(shell);
    cmd.arg(shell_arg).arg(&program);
    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn()
        .map_err(|e| format!("Subprocess_popenWrite: failed to spawn '{}': {}", program, e))?;

    use std::io::Write;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(input.as_bytes());
    }
    let output = child.wait_with_output()
        .map_err(|e| format!("Subprocess_popenWrite: failed to collect output: {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(Value::String(Rc::new(stdout)))
}

// ---------------------------------------------------------------------------
// Full-result execution backing tt::lang::Subprocess (Popen)
// ---------------------------------------------------------------------------

/// Build a shell command for a command line so shell builtins (echo, dir,
/// type) and shell syntax work on every platform.
fn shell_command(program: &str) -> std::process::Command {
    let (shell, shell_arg) = if cfg!(target_os = "windows") {
        ("cmd", "/C")
    } else {
        ("sh", "-c")
    };
    let mut cmd = std::process::Command::new(shell);
    cmd.arg(shell_arg).arg(program);
    cmd
}

/// Construct the tt::lang::Subprocess.SubprocessResult value the stdlib
/// Popen class reads (fields stdout/stderr/returnCode/pid).
fn subprocess_result(stdout: String, stderr: String, code: i32, pid: i64) -> Value {
    let mut fields = HashMap::new();
    fields.insert("stdout".to_string(), Value::String(Rc::new(stdout)));
    fields.insert("stderr".to_string(), Value::String(Rc::new(stderr)));
    fields.insert("returnCode".to_string(), Value::Int(code));
    fields.insert("pid".to_string(), Value::Int(pid as i32));
    Value::ClassInstance {
        class_name: "SubprocessResult".to_string(),
        fields: Rc::new(std::cell::RefCell::new(fields)),
        vtable: HashMap::new(),
    }
}

/// Wait for `child` up to `timeout_ms` and collect piped stdout/stderr.
/// A negative timeout waits forever; an expired timeout is an explicit
/// error (Popen-level terminate()/kill() own lifecycle control, and the
/// worker thread reaps the child so nothing leaks).
fn wait_with_timeout_ms(
    child: std::process::Child,
    pid: i64,
    timeout_ms: i64,
    what: &str,
) -> Result<Value, String> {
    use std::sync::mpsc;
    use std::time::Duration;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let out = child.wait_with_output();
        let _ = tx.send(out);
    });
    let output = if timeout_ms < 0 {
        rx.recv()
            .map_err(|e| format!("{}: worker thread failed: {}", what, e))?
    } else {
        match rx.recv_timeout(Duration::from_millis(timeout_ms.max(0) as u64)) {
            Ok(out) => out,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                return Err(format!(
                    "{}: timed out after {} ms (pid {})",
                    what, timeout_ms, pid
                ));
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(format!("{}: worker thread failed", what));
            }
        }
    };
    let output = output.map_err(|e| format!("{}: failed to collect output: {}", what, e))?;
    Ok(subprocess_result(
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.code().unwrap_or(-1),
        pid,
    ))
}

/// Subprocess_runFull(commandLine: String) -> SubprocessResult
pub(crate) fn native_subprocess_run_full(args: &[Value]) -> Result<Value, String> {
    let program = match args.first() {
        Some(Value::String(s)) => s.as_str().to_string(),
        _ => return Err("Subprocess_runFull: expected a String command line".to_string()),
    };
    let mut cmd = shell_command(&program);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    let child = cmd
        .spawn()
        .map_err(|e| format!("Subprocess_runFull: failed to spawn '{}': {}", program, e))?;
    let pid = child.id() as i64;
    let output = child
        .wait_with_output()
        .map_err(|e| format!("Subprocess_runFull: failed to collect output: {}", e))?;
    Ok(subprocess_result(
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.code().unwrap_or(-1),
        pid,
    ))
}

/// Subprocess_runWithInput(commandLine: String, input: String, timeoutMs: Long)
/// -> SubprocessResult
pub(crate) fn native_subprocess_run_with_input(args: &[Value]) -> Result<Value, String> {
    if args.len() < 3 {
        return Err(
            "Subprocess_runWithInput: expected (commandLine, input, timeoutMs)".to_string(),
        );
    }
    let program = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Subprocess_runWithInput: expected a String command line".to_string()),
    };
    let input = match &args[1] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Subprocess_runWithInput: expected a String input".to_string()),
    };
    let timeout_ms = match &args[2] {
        Value::Long(l) => *l,
        Value::Int(i) => *i as i64,
        _ => return Err("Subprocess_runWithInput: expected an Int/Long timeout".to_string()),
    };
    let mut cmd = shell_command(&program);
    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Subprocess_runWithInput: failed to spawn '{}': {}", program, e))?;
    let pid = child.id() as i64;
    use std::io::Write;
    if let Some(mut stdin) = child.stdin.take() {
        // A broken pipe just means the child exited early; its output is
        // still collected below.
        let _ = stdin.write_all(input.as_bytes());
    }
    wait_with_timeout_ms(child, pid, timeout_ms, "Subprocess_runWithInput")
}

/// Subprocess_runWithTimeout(commandLine: String, timeoutMs: Long)
/// -> SubprocessResult
pub(crate) fn native_subprocess_run_with_timeout(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Subprocess_runWithTimeout: expected (commandLine, timeoutMs)".to_string());
    }
    let program = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => {
            return Err("Subprocess_runWithTimeout: expected a String command line".to_string())
        }
    };
    let timeout_ms = match &args[1] {
        Value::Long(l) => *l,
        Value::Int(i) => *i as i64,
        _ => {
            return Err("Subprocess_runWithTimeout: expected an Int/Long timeout".to_string())
        }
    };
    let mut cmd = shell_command(&program);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    let child = cmd
        .spawn()
        .map_err(|e| format!("Subprocess_runWithTimeout: failed to spawn '{}': {}", program, e))?;
    let pid = child.id() as i64;
    wait_with_timeout_ms(child, pid, timeout_ms, "Subprocess_runWithTimeout")
}
