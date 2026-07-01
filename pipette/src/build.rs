use std::fs;
use std::path::{Path, PathBuf};

use trc::analyzer;
use trc::bytecode::Compiler;
use trc::lexer;
use trc::parser;

use crate::deps::resolve_dependencies;
use crate::project;
use crate::serialize::serialize_compiled_program;
use crate::BuildProfile;

/// Build the project: read config, compile entry point + modules, write bytecode.
/// Returns the path to the build output.
pub fn build(project_dir: &Path) -> Result<PathBuf, String> {
    build_with_profile(project_dir, BuildProfile::Debug)
}

/// Build the project with the specified profile.
pub fn build_with_profile(project_dir: &Path, profile: BuildProfile) -> Result<PathBuf, String> {
    let cfg = project::load_config(project_dir)?;

    // Check that all dependencies are available
    resolve_dependencies(&cfg)?;

    // Read the entry point source
    let entry_path = project_dir.join(&cfg.package.entry);
    let source = fs::read_to_string(&entry_path).map_err(|e| {
        format!(
            "Failed to read entry point '{}': {}",
            entry_path.display(),
            e
        )
    })?;

    // Tokenize
    let tokens = lexer::tokenize(&source)?;

    // Parse
    let ast = parser::parse(tokens)?;

    // Semantic analysis
    let typed_ast = analyzer::analyze(&ast).map_err(|errs| errs.join("\n"))?;

    // Compile with module resolution (lib/ directory as search path)
    let mut compiler = Compiler::new();
    let compiled = compiler.compile_with_modules(&typed_ast, project_dir)?;

    // Create build directory (profile-specific subdirectory)
    let build_dir = project_dir.join("build").join(profile.to_string());
    fs::create_dir_all(&build_dir)
        .map_err(|e| format!("Failed to create build directory: {}", e))?;

    // Serialize and write the compiled program
    let output_path = build_dir.join("output.tbc");
    let data = serialize_compiled_program(&compiled);
    fs::write(&output_path, data)
        .map_err(|e| format!("Failed to write build output: {}", e))?;

    match profile {
        BuildProfile::Debug => {
            println!("Built {} (debug)", cfg.package.name);
        }
        BuildProfile::Release => {
            println!("Built {} (release – optimized)", cfg.package.name);
        }
    }

    Ok(output_path)
}

/// Parse the flags that may follow the `build` subcommand.
///
/// Returns `(native, release)` indicating whether each flag was present.
pub fn parse_build_flags(args: &[String]) -> (bool, bool) {
    let native = args.iter().any(|a| a == "--native");
    let release = args.iter().any(|a| a == "--release");
    (native, release)
}

/// Build the project to a native executable by invoking `trc --native`.
///
/// Drives the LLVM backend via the `trc` binary (located on `PATH` or under
/// the workspace `target/` directory) and places the resulting executable at
/// `build/native/<package_name>[.exe]`.
pub fn build_native(_project_dir: &Path) -> Result<PathBuf, String> {
    // Implemented in a follow-up commit.
    Err("--native build is not yet implemented".to_string())
}
