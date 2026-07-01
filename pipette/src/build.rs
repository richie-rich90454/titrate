use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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
/// `build/native/<package_name>[.exe]`. The native build always passes
/// `--release` to `trc` so the LLVM lowering is optimized.
pub fn build_native(project_dir: &Path) -> Result<PathBuf, String> {
    let cfg = project::load_config(project_dir)?;

    // Ensure declared dependencies are available before compiling.
    resolve_dependencies(&cfg)?;

    let entry = &cfg.package.entry;

    // Locate the `trc` compiler: PATH first, then workspace target/.
    let trc_path = find_trc_binary().ok_or_else(|| {
        "could not locate the `trc` compiler; build it with `cargo build -p trc` \
         or ensure it is on PATH"
            .to_string()
    })?;

    // `trc --native --release <entry>` lowers to LLVM IR, links with
    // libtitrate_native, and writes `<stem>_native[.exe]` beside the source.
    // stdio is inherited (the default) so compile errors stream to the user.
    let status = Command::new(&trc_path)
        .arg("--native")
        .arg("--release")
        .arg(entry)
        .current_dir(project_dir)
        .status()
        .map_err(|e| format!("Failed to invoke trc '{}': {}", trc_path.display(), e))?;
    if !status.success() {
        return Err(format!("trc exited with status {}", status));
    }

    // trc produced `<entry_dir>/<stem>_native[.exe]` next to the source.
    #[cfg(windows)]
    let exe_suffix = ".exe";
    #[cfg(not(windows))]
    let exe_suffix = "";

    let entry_path = Path::new(entry);
    let entry_dir = entry_path.parent().unwrap_or_else(|| Path::new(""));
    let stem = entry_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| format!("invalid entry file name: '{}'", entry))?;
    let produced_exe = project_dir
        .join(entry_dir)
        .join(format!("{}_native{}", stem, exe_suffix));

    if !produced_exe.is_file() {
        return Err(format!(
            "trc did not produce the expected executable at '{}'",
            produced_exe.display()
        ));
    }

    // Move the executable into build/native/<package_name>[.exe].
    let native_dir = project_dir.join("build").join("native");
    fs::create_dir_all(&native_dir)
        .map_err(|e| format!("Failed to create native build directory: {}", e))?;
    let target_exe = native_dir.join(format!("{}{}", cfg.package.name, exe_suffix));

    // Try a rename first; fall back to copy+remove for cross-volume moves.
    if fs::rename(&produced_exe, &target_exe).is_err() {
        fs::copy(&produced_exe, &target_exe).map_err(|e| {
            format!(
                "Failed to copy native binary to '{}': {}",
                target_exe.display(),
                e
            )
        })?;
        let _ = fs::remove_file(&produced_exe);
    }

    println!("Built {} (native)", cfg.package.name);
    println!("Native binary at {}", target_exe.display());
    Ok(target_exe)
}

/// Locate the `trc` compiler binary.
///
/// Search order:
/// 1. `trc` (or `trc.exe` on Windows) on the `PATH` environment variable.
/// 2. `<workspace_root>/target/release/trc[.exe]` (preferred, matches --release).
/// 3. `<workspace_root>/target/debug/trc[.exe]`.
///
/// The `target/` lookup walks up from the current directory, mirroring the
/// existing lookup used by `pipette bench`.
fn find_trc_binary() -> Option<PathBuf> {
    if let Some(p) = find_in_path() {
        return Some(p);
    }
    let mut dir = env::current_dir().ok()?;
    for _ in 0..10 {
        for profile in &["release", "debug"] {
            let exe_name = if cfg!(windows) { "trc.exe" } else { "trc" };
            let candidate = dir.join("target").join(profile).join(exe_name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

/// Search the `PATH` environment variable for the `trc` executable.
fn find_in_path() -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    let exe_name = if cfg!(windows) { "trc.exe" } else { "trc" };
    for dir in env::split_paths(&path) {
        let candidate = dir.join(exe_name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}
