use std::fs;
use std::path::Path;

use trc::bytecode::Vm;

use crate::build::build;
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
        // Implemented in a follow-up commit.
        return Err("--native run is not yet implemented".to_string());
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
