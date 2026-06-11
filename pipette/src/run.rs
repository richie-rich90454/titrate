use std::fs;
use std::path::Path;

use trc::bytecode::Vm;

use crate::build::build;
use crate::serialize::deserialize_compiled_program;

/// Build the project and then execute it.
pub fn run(project_dir: &Path) -> Result<(), String> {
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
