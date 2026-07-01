use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

use trc::bytecode::Vm;

use crate::build::{build, build_native};
use crate::serialize::deserialize_compiled_program;

/// Parse the flags that may follow the `run` subcommand.
///
/// Returns `true` when `--native` was requested.
pub fn parse_run_flags(args: &[String]) -> bool {
    args.iter().any(|a| a == "--native")
}

/// Build the project and then execute it.
///
/// When `native` is `true`, the project is compiled to a native executable
/// (via `trc --native`) and spawned as a child process. Otherwise the
/// bytecode VM is used.
pub fn run(project_dir: &Path, native: bool) -> Result<(), String> {
    if native {
        // Build the native executable, then spawn it with inherited stdio
        // and forward the child's exit code.
        let exe = build_native(project_dir)?;
        let status = Command::new(&exe)
            .current_dir(project_dir)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .map_err(|e| {
                format!(
                    "Failed to spawn native executable '{}': {}",
                    exe.display(),
                    e
                )
            })?;
        std::process::exit(status.code().unwrap_or(1));
    }

    build(project_dir)?;

    // Load and execute
    let build_path = project_dir.join("build").join("output.tbc");
    let data = fs::read(&build_path).map_err(|e| format!("Failed to read build output: {}", e))?;
    let compiled = deserialize_compiled_program(&data)?;

    let mut vm = Vm::new();
    vm.load_program(compiled);
    vm.run()?;

    // Print captured output
    for line in &vm.output {
        println!("{}", line);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_run_flags_detects_native() {
        let args = vec!["--native".to_string()];
        assert!(parse_run_flags(&args));
    }

    #[test]
    fn parse_run_flags_empty() {
        let args: Vec<String> = vec![];
        assert!(!parse_run_flags(&args));
    }

    #[test]
    fn parse_run_flags_release_is_not_native() {
        // --release is accepted by `run` but does not request native execution.
        let args = vec!["--release".to_string()];
        assert!(!parse_run_flags(&args));
    }

    #[test]
    fn parse_run_flags_native_among_other_args() {
        let args = vec![
            "--foo".to_string(),
            "--native".to_string(),
            "bar".to_string(),
        ];
        assert!(parse_run_flags(&args));
    }
}
