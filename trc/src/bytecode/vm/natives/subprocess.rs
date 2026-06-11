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
