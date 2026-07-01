use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;

use trc::lexer;
use trc::parser;
use trc::analyzer;
use trc::bytecode;

/// Parsed command-line arguments.
struct Args {
    file: Option<String>,
    native: bool,
    release: bool,
    /// Emit LLVM IR to a `.ll` file beside the source (`<stem>.ll`).
    emit_ir: bool,
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
    };
    for arg in args.iter().skip(1) {
        match arg.as_str() {
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
fn has_native_lib(dir: &PathBuf) -> bool {
    dir.join("titrate_native.lib").is_file() || dir.join("libtitrate_native.a").is_file()
}

#[cfg(not(windows))]
fn has_native_lib(dir: &PathBuf) -> bool {
    dir.join("libtitrate_native.a").is_file()
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

    // 3. Link.
    // (Link flags from Titrate.toml [native] are wired up in a later change;
    // for now pass empty slices so existing native builds are unchanged.)
    llvm::linker::link(&obj_path, &exe_path, &native_lib_dir, &[], &[])?;

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

fn main() {
    let args: Vec<String> = env::args().collect();
    let parsed = parse_args(&args);

    let path = match parsed.file {
        Some(p) => p,
        None => {
            eprintln!("Usage: trc <file.tr> [--native] [--release] [--emit-ir]");
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

    match bytecode::execute(&typed_ast) {
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
