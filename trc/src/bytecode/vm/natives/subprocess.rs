// Titrate Alpha 0.2 – bytecode virtual machine: subprocess natives
// Precision in every step – richie-rich90454, 2026

use super::super::super::value::Value;
use std::rc::Rc;

pub(crate) fn native_subprocess_run(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Subprocess_run: expected at least 1 argument (command)".to_string());
    }
    let program = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Subprocess_run: expected String command".to_string()),
    };
    let mut cmd = std::process::Command::new(&program);
    // Additional arguments as strings
    for arg in &args[1..] {
        match arg {
            Value::String(s) => { cmd.arg(s.as_str()); }
            other => { cmd.arg(format!("{:?}", other)); }
        }
    }
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
    let mut cmd = std::process::Command::new(&program);
    for arg in &args[1..] {
        match arg {
            Value::String(s) => { cmd.arg(s.as_str()); }
            other => { cmd.arg(format!("{:?}", other)); }
        }
    }
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
