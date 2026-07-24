use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use trc::lexer;
use trc::parser;
use trc::analyzer;
use trc::bytecode;
use trc::errors;

/// Parsed command-line arguments.
struct Args {
    file: Option<String>,
    native: bool,
    release: bool,
    /// Emit LLVM IR to a `.ll` file beside the source (`<stem>.ll`).
    emit_ir: bool,
    help: bool,
}

/// Parse the command-line arguments. Recognised flags:
///   --native   – emit a native executable via the LLVM backend
///   --release  – enable LLVM optimizations (implies --native-style output)
///   --emit-ir  – write the LLVM IR to a `.ll` file beside the source
/// Any other argument that does not start with `--` is treated as the input
/// `.tr` file path.
fn parse_args(args: &[String]) -> Args {
    let mut parsed = Args {
        file: None,
        native: false,
        release: false,
        emit_ir: false,
        help: false,
    };
    for arg in args.iter().skip(1) {
        match arg.as_str() {
            "--help" | "-h" => parsed.help = true,
            "--native" => parsed.native = true,
            "--release" => parsed.release = true,
            "--emit-ir" => parsed.emit_ir = true,
            s if s.starts_with("--") => {
                eprintln!("warning: unknown flag '{}'", s);
            }
            s => {
                if parsed.file.is_none() {
                    parsed.file = Some(s.to_string());
                } else {
                    eprintln!("warning: ignoring extra argument '{}'", s);
                }
            }
        }
    }
    parsed
}

/// Locate the directory containing the built `titrate_native` static library.
///
/// We check the `TITRATE_WORKSPACE_ROOT` env var (set by `trc/build.rs`) first,
/// then fall back to walking up from the current executable to find a
/// `target/` directory. Returns the directory that actually contains the
/// library file, or `None` if it cannot be found.
fn find_native_lib_dir(release: bool) -> Option<PathBuf> {
    let profile = if release { "release" } else { "debug" };

    // 1. Workspace root from build.rs env var.
    if let Ok(root) = env::var("TITRATE_WORKSPACE_ROOT") {
        let candidate = PathBuf::from(root).join("target").join(profile);
        if has_native_lib(&candidate) {
            return Some(candidate);
        }
    }

    // 2. Walk up from CARGO_MANIFEST_DIR (set when running via cargo).
    if let Ok(manifest) = env::var("CARGO_MANIFEST_DIR") {
        let candidate = PathBuf::from(manifest)
            .join("target")
            .join(profile);
        if has_native_lib(&candidate) {
            return Some(candidate);
        }
    }

    // 3. Walk up from the current directory looking for a `target` folder.
    let mut dir = env::current_dir().ok()?;
    for _ in 0..10 {
        let candidate = dir.join("target").join(profile);
        if has_native_lib(&candidate) {
            return Some(candidate);
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

#[cfg(windows)]
fn has_native_lib(dir: &Path) -> bool {
    dir.join("titrate_native.lib").is_file() || dir.join("libtitrate_native.a").is_file()
}

#[cfg(not(windows))]
fn has_native_lib(dir: &Path) -> bool {
    dir.join("libtitrate_native.a").is_file()
}

/// Load extra link libraries and raw linker flags for the native build.
///
/// Looks up the `[native]` section of a `Titrate.toml` found by walking up
/// from the current directory. If no manifest is present (or it has no
/// `[native]` section), falls back to the `TITRATE_LINK_LIBS` and
/// `TITRATE_LINK_ARGS` environment variables (comma-separated), which
/// `pipette build --native` sets when it invokes `trc`.
fn load_native_link_flags() -> (Vec<String>, Vec<String>) {
    if let Some(flags) = load_native_from_toml() {
        return flags;
    }
    load_native_from_env()
}

/// Read `[native]` from the nearest `Titrate.toml` ancestor of the CWD.
fn load_native_from_toml() -> Option<(Vec<String>, Vec<String>)> {
    let toml_path = find_titrate_toml()?;
    let contents = fs::read_to_string(&toml_path).ok()?;
    parse_native_section(&contents)
}

/// Walk up from the current directory looking for `Titrate.toml`.
fn find_titrate_toml() -> Option<PathBuf> {
    let mut dir = env::current_dir().ok()?;
    loop {
        let candidate = dir.join("Titrate.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Parse only the `[native]` section of a `Titrate.toml` manifest.
///
/// Returns `Some((link_libs, link_args))` when a `[native]` section is
/// present (fields default to empty arrays if omitted within the section),
/// or `None` when the section is absent.
fn parse_native_section(contents: &str) -> Option<(Vec<String>, Vec<String>)> {
    let mut in_native = false;
    let mut found_section = false;
    let mut link_libs = Vec::new();
    let mut link_args = Vec::new();
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_native = trimmed[1..trimmed.len() - 1].trim() == "native";
            if in_native {
                found_section = true;
            }
            continue;
        }
        if in_native {
            if let Some(eq_pos) = trimmed.find('=') {
                let key = trimmed[..eq_pos].trim();
                let value = trimmed[eq_pos + 1..].trim();
                match key {
                    "link_libs" => link_libs = parse_string_array(value),
                    "link_args" => link_args = parse_string_array(value),
                    _ => {}
                }
            }
        }
    }
    if found_section {
        Some((link_libs, link_args))
    } else {
        None
    }
}

/// Parse a TOML array of strings (e.g. `["ssl", "crypto"]`) into a `Vec<String>`.
fn parse_string_array(value: &str) -> Vec<String> {
    let trimmed = value.trim();
    if !(trimmed.starts_with('[') && trimmed.ends_with(']')) {
        return Vec::new();
    }
    let inner = &trimmed[1..trimmed.len() - 1];
    if inner.trim().is_empty() {
        return Vec::new();
    }
    inner
        .split(',')
        .map(|s| s.trim())
        .map(|s| {
            if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
                s[1..s.len() - 1].to_string()
            } else {
                s.to_string()
            }
        })
        .filter(|s| !s.is_empty())
        .collect()
}

/// Fall back to env vars when no `Titrate.toml` `[native]` section is found.
fn load_native_from_env() -> (Vec<String>, Vec<String>) {
    let libs = parse_csv_env("TITRATE_LINK_LIBS");
    let args = parse_csv_env("TITRATE_LINK_ARGS");
    (libs, args)
}

fn parse_csv_env(name: &str) -> Vec<String> {
    env::var(name)
        .ok()
        .map(|s| {
            s.split(',')
                .map(|x| x.trim().to_string())
                .filter(|x| !x.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// Run the native (LLVM) backend: compile the typed AST to an object file,
/// link it with `libtitrate_native`, and write the final executable next to
/// the source file.
fn run_native(
    typed_ast: &trc::ast::Program,
    source_path: &str,
    release: bool,
    emit_ir: bool,
) -> Result<(), String> {
    use trc::codegen::llvm;

    // Place build artifacts next to the source file.
    let source = PathBuf::from(source_path);
    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("invalid source file name")?;
    let dir = source
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    #[cfg(windows)]
    let exe_ext = "exe";
    #[cfg(not(windows))]
    let exe_ext = "";

    let exe_name = if exe_ext.is_empty() {
        format!("{}_native", stem)
    } else {
        format!("{}_native.{}", stem, exe_ext)
    };
    let exe_path = dir.join(&exe_name);

    let obj_name = format!("{}_native.o", stem);
    let obj_path = dir.join(&obj_name);

    // 1. Lower to LLVM IR and emit the object file. When --emit-ir is set,
    //    the IR is also written to `<stem>.ll` here (before the linker runs).
    let ir_path = dir.join(format!("{}.ll", stem));
    if emit_ir {
        // Single codegen pass: object file + IR. The IR is written before
        // the linker is invoked, so a link failure still leaves the .ll file.
        llvm::compile_with_ir(typed_ast, &obj_path, &ir_path, release)?;
        println!("LLVM IR written to {}", ir_path.display());
    } else {
        llvm::compile(typed_ast, &obj_path, release)?;
    }

    // 2. Locate the titrate_native static library.
    let native_lib_dir = find_native_lib_dir(release).ok_or_else(|| {
        "could not locate titrate_native static library; build it first with \
         `cargo build -p titrate_native`"
            .to_string()
    })?;

    // 3. Link. Pull any extra link libs/args from the [native] section of a
    //    nearby Titrate.toml (or the TITRATE_LINK_LIBS/TITRATE_LINK_ARGS env
    //    vars set by `pipette build --native` when no manifest is found).
    let (extra_link_libs, extra_link_args) = load_native_link_flags();
    llvm::linker::link(
        &obj_path,
        &exe_path,
        &native_lib_dir,
        &extra_link_libs,
        &extra_link_args,
    )?;

    // 4. Clean up the intermediate object file.
    let _ = fs::remove_file(&obj_path);

    println!("Native binary written to {}", exe_path.display());
    Ok(())
}

/// Emit LLVM IR only: lower the typed AST to LLVM IR and write it to a
/// `<stem>.ll` file beside the source. No object file is produced and the
/// linker is never invoked. This is the `--emit-ir` (without `--native`) path.
fn run_emit_ir(
    typed_ast: &trc::ast::Program,
    source_path: &str,
) -> Result<(), String> {
    use trc::codegen::llvm;

    let source = PathBuf::from(source_path);
    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("invalid source file name")?;
    let dir = source
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let ir_path = dir.join(format!("{}.ll", stem));

    llvm::compile_ir(typed_ast, &ir_path)?;
    println!("LLVM IR written to {}", ir_path.display());
    Ok(())
}

/// Render a lexer or parser error in the canonical `error[E0XXX]: <message>`
/// format with source location and caret.
///
/// `message` may carry a trailing ` at L:C` suffix (produced by the parser's
/// `err()` helper or the lexer's `at {}:{}` format); the suffix is stripped
/// before rendering and the location is used to build the `-->` and caret
/// lines.
fn _report_error(code: &str, message: &str, file: &str, source: &str) {
    let (clean_msg, line_opt, col_opt) = match errors::parse_location_suffix(message) {
        Some((l, c)) => (errors::strip_location_suffix(message), Some(l), Some(c)),
        None => (message.to_string(), None, None),
    };
    let source_line = line_opt.and_then(|l| source.lines().nth(l.saturating_sub(1)));
    let rendered = errors::render_error(code, &clean_msg, file, line_opt, col_opt, source_line, 1);
    eprint!("{}", rendered);
}

/// Render a semantic (analyzer) error in the canonical format.
///
/// Semantic errors are returned as strings by `analyzer::analyze`; each string
/// is the `Display` output of a `CompileError`, which may include one or more
/// `\n  help: <suggestion>` lines. This function splits the primary message
/// from the suggestions, classifies it into a stable code, and renders the
/// full diagnostic.
fn _report_semantic_error(err: &str, file: &str, source: &str) {
    let mut parts = err.splitn(2, "\n  help: ");
    let message = parts.next().unwrap_or(err);
    let suggestions: Vec<&str> = match parts.next() {
        Some(rest) => rest.split("\n  help: ").collect(),
        None => Vec::new(),
    };
    let code = errors::classify_semantic_error(message);
    let (clean_msg, line_opt, col_opt) = match errors::parse_location_suffix(message) {
        Some((l, c)) => (errors::strip_location_suffix(message), Some(l), Some(c)),
        None => (message.to_string(), None, None),
    };
    let source_line = line_opt.and_then(|l| source.lines().nth(l.saturating_sub(1)));
    let mut rendered = errors::render_error(code, &clean_msg, file, line_opt, col_opt, source_line, 1);
    for s in &suggestions {
        rendered.push_str(&format!("  help: {}\n", s));
    }
    eprint!("{}", rendered);
}

fn print_usage() {
    eprintln!("titrate compiler (trc) v{}", env!("CARGO_PKG_VERSION"));
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("    trc <file.tr> [OPTIONS]");
    eprintln!();
    eprintln!("OPTIONS:");
    eprintln!("    --native      Emit a native executable via the LLVM backend");
    eprintln!("    --release     Enable LLVM optimizations (implies --native)");
    eprintln!("    --emit-ir     Write LLVM IR to a .ll file beside the source");
    eprintln!("    --help, -h    Show this help message");
    eprintln!();
    eprintln!("MODES:");
    eprintln!("    Default        Run the file using the bytecode VM");
    eprintln!("    --native       Compile to a native executable (requires LLVM)");
    eprintln!("    --emit-ir      Emit LLVM IR without compiling or linking");
    eprintln!("    --emit-ir --native   Emit IR and compile to native");
    eprintln!();
    eprintln!("EXAMPLES:");
    eprintln!("    trc hello.tr                   Run hello.tr with the VM");
    eprintln!("    trc hello.tr --native          Compile hello.tr to a native executable");
    eprintln!("    trc hello.tr --release         Compile with optimizations");
    eprintln!("    trc hello.tr --emit-ir         Emit LLVM IR to hello.ll");
    eprintln!();
    eprintln!("OUTPUT:");
    eprintln!("    VM mode:      Prints program output to stdout");
    eprintln!("    --native:     Creates <stem>_native[.exe] next to the source file");
    eprintln!("    --emit-ir:    Creates <stem>.ll next to the source file");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let parsed = parse_args(&args);

    if parsed.help {
        print_usage();
        process::exit(0);
    }

    let path = match parsed.file {
        Some(p) => p,
        None => {
            print_usage();
            process::exit(1);
        }
    };

    // Let the titration begin – richie-rich90454
    let source = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading '{}': {}", path, e);
            process::exit(1);
        }
    };

    let tokens = match lexer::tokenize(&source) {
        Ok(tokens) => tokens,
        Err(e) => {
            eprintln!("Lexer error: {}", e);
            process::exit(1);
        }
    };

    let ast = match parser::parse(tokens) {
        Ok(ast) => ast,
        Err(e) => {
            eprintln!("Parse error: {}", e);
            process::exit(1);
        }
    };

    let typed_ast = match analyzer::analyze(&ast) {
        Ok(ast) => ast,
        Err(errs) => {
            for e in &errs {
                eprintln!("Semantic error: {}", e);
            }
            process::exit(1);
        }
    };

    // --emit-ir without --native: write the .ll file and exit (no object, no linker).
    if parsed.emit_ir && !parsed.native {
        if let Err(e) = run_emit_ir(&typed_ast, &path) {
            eprintln!("Native backend error: {}", e);
            process::exit(1);
        }
        return;
    }

    if parsed.native {
        if let Err(e) = run_native(&typed_ast, &path, parsed.release, parsed.emit_ir) {
            eprintln!("Native backend error: {}", e);
            process::exit(1);
        }
        return;
    }

    // Determine root directory for module resolution.
    // Walk up from the source file until we find a directory containing 'lib/'.
    let source_path = std::path::Path::new(&path).canonicalize().ok()
        .unwrap_or_else(|| std::path::PathBuf::from(&path));
    let mut root_dir = source_path.parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    while !root_dir.join("lib").is_dir() && root_dir.parent().is_some() {
        root_dir = root_dir.parent().unwrap().to_path_buf();
    }

    match bytecode::execute_with_root(&typed_ast, &root_dir) {
        Ok(output) => {
            for line in &output {
                println!("{}", line);
            }
        }
        Err(e) => {
            eprintln!("Runtime error: {}", e);
            process::exit(1);
        }
    }
}
