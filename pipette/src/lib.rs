// Titrate build tool – pipette core library
// Precision in every step – richie-rich90454, 2026

pub mod config;
pub mod project;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use trc::analyzer;
use trc::bytecode::{CompiledProgram, Compiler, Vm};
use trc::lexer;
use trc::parser;

// ---------------------------------------------------------------------------
// Build
// ---------------------------------------------------------------------------

/// Build the project: read config, compile entry point + modules, write bytecode.
/// Returns the path to the build output.
pub fn build(project_dir: &Path) -> Result<PathBuf, String> {
    let cfg = project::load_config(project_dir)?;

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

    // Create build directory
    let build_dir = project_dir.join("build");
    fs::create_dir_all(&build_dir)
        .map_err(|e| format!("Failed to create build directory: {}", e))?;

    // Serialize and write the compiled program
    let output_path = build_dir.join("output.tbc");
    let data = serialize_compiled_program(&compiled);
    fs::write(&output_path, data)
        .map_err(|e| format!("Failed to write build output: {}", e))?;

    Ok(output_path)
}

// ---------------------------------------------------------------------------
// Run
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Test
// ---------------------------------------------------------------------------

/// Find and run test files (ending in `_test.tr`).
pub fn test(project_dir: &Path) -> Result<(), String> {
    let _cfg = project::load_config(project_dir)?;

    // Collect test files from src/ and any subdirectories
    let src_dir = project_dir.join("src");
    let mut test_files = Vec::new();
    collect_test_files(&src_dir, &mut test_files)?;

    if test_files.is_empty() {
        println!("No test files found (looking for *_test.tr in src/)");
        return Ok(());
    }

    let mut passed = 0;
    let mut failed = 0;

    for test_file in &test_files {
        let rel = test_file
            .strip_prefix(project_dir)
            .unwrap_or(test_file)
            .display()
            .to_string();

        print!("  testing {} ... ", rel);

        match run_test_file(test_file, project_dir) {
            Ok(()) => {
                println!("ok");
                passed += 1;
            }
            Err(e) => {
                println!("FAILED");
                println!("    {}", e);
                failed += 1;
            }
        }
    }

    println!(
        "\n{} test(s) passed, {} test(s) failed",
        passed, failed
    );

    if failed > 0 {
        Err(format!("{} test(s) failed", failed))
    } else {
        Ok(())
    }
}

fn collect_test_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }
    let entries = fs::read_dir(dir).map_err(|e| format!("Failed to read directory: {}", e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();
        if path.is_dir() {
            collect_test_files(&path, files)?;
        } else if let Some(name) = path.file_name() {
            let name = name.to_string_lossy();
            if name.ends_with("_test.tr") {
                files.push(path);
            }
        }
    }
    Ok(())
}

fn run_test_file(test_file: &Path, project_dir: &Path) -> Result<(), String> {
    let source = fs::read_to_string(test_file)
        .map_err(|e| format!("Failed to read test file: {}", e))?;

    let tokens = lexer::tokenize(&source)?;
    let ast = parser::parse(tokens)?;
    let typed_ast = analyzer::analyze(&ast).map_err(|errs| errs.join("\n"))?;

    let mut compiler = Compiler::new();
    let compiled = compiler.compile_with_modules(&typed_ast, project_dir)?;

    let mut vm = Vm::new();
    vm.load_program(compiled);
    vm.run()?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Watch
// ---------------------------------------------------------------------------

/// Watch for file changes and rebuild.
pub fn watch(project_dir: &Path) -> Result<(), String> {
    println!("Watching for changes... (Ctrl+C to stop)");

    // Initial build
    match build(project_dir) {
        Ok(_) => println!("Initial build succeeded."),
        Err(e) => eprintln!("Initial build failed: {}", e),
    }

    let src_dir = project_dir.join("src");
    let mut last_mtimes = get_mtimes(&src_dir);

    loop {
        std::thread::sleep(std::time::Duration::from_millis(500));

        let current_mtimes = get_mtimes(&src_dir);
        if current_mtimes != last_mtimes {
            println!("\nChange detected, rebuilding...");
            match build(project_dir) {
                Ok(_) => println!("Build succeeded."),
                Err(e) => eprintln!("Build failed: {}", e),
            }
            last_mtimes = current_mtimes;
        }
    }
}

fn get_mtimes(dir: &Path) -> HashMap<PathBuf, SystemTime> {
    let mut mtimes = HashMap::new();
    if !dir.exists() {
        return mtimes;
    }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                mtimes.extend(get_mtimes(&path));
            } else if path.extension().map_or(false, |ext| ext == "tr") {
                if let Ok(meta) = fs::metadata(&path) {
                    if let Ok(mtime) = meta.modified() {
                        mtimes.insert(path, mtime);
                    }
                }
            }
        }
    }
    mtimes
}

// ---------------------------------------------------------------------------
// Serialization helpers for CompiledProgram
// ---------------------------------------------------------------------------

/// Serialize a CompiledProgram to bytes.
/// Format:
///   [4 bytes: function count]
///   For each function:
///     [4 bytes: name length] [name bytes]
///     [4 bytes: arity]
///     [4 bytes: is_method (0/1)]
///     [4 bytes: is_constructor (0/1)]
///     [4 bytes: local_count]
///     [4 bytes: chunk code length] [code bytes]
///     [4 bytes: chunk string count]
///     For each string:
///       [4 bytes: string length] [string bytes]
///       [4 bytes: source_lines count] [source_lines bytes (u32 each)]
///   [4 bytes: class count]
///   For each class: (simplified – name only, no methods/fields for now)
///     [4 bytes: name length] [name bytes]
///   [4 bytes: enum count]
///   For each enum: (simplified – name only)
///     [4 bytes: name length] [name bytes]
///   [4 bytes: native_names count]
///   For each native name:
///     [4 bytes: name length] [name bytes]
fn serialize_compiled_program(program: &CompiledProgram) -> Vec<u8> {
    let mut buf = Vec::new();

    // Functions
    write_u32(&mut buf, program.functions.len() as u32);
    for func in &program.functions {
        write_str(&mut buf, &func.name);
        write_u32(&mut buf, func.arity as u32);
        write_u32(&mut buf, func.is_method as u32);
        write_u32(&mut buf, func.is_constructor as u32);
        write_u32(&mut buf, func.local_count as u32);

        // Chunk code
        write_u32(&mut buf, func.chunk.code.len() as u32);
        buf.extend_from_slice(&func.chunk.code);

        // Chunk constants (Vec<u64>)
        write_u32(&mut buf, func.chunk.constants.len() as u32);
        for &val in &func.chunk.constants {
            write_u64(&mut buf, val);
        }

        // Chunk strings
        write_u32(&mut buf, func.chunk.strings.len() as u32);
        for s in &func.chunk.strings {
            write_str(&mut buf, s);
        }

        // Chunk source_lines (Vec<u32>)
        write_u32(&mut buf, func.chunk.source_lines.len() as u32);
        for &line in &func.chunk.source_lines {
            write_u32(&mut buf, line);
        }
    }

    // Classes
    write_u32(&mut buf, program.classes.len() as u32);
    for class in &program.classes {
        write_str(&mut buf, &class.name);
        write_u32(&mut buf, class.parent.map(|p| p as u32).unwrap_or(u32::MAX));
        // Fields
        write_u32(&mut buf, class.fields.len() as u32);
        for field in &class.fields {
            write_str(&mut buf, &field.name);
            write_u32(&mut buf, field.has_init as u32);
        }
        // Methods
        write_u32(&mut buf, class.methods.len() as u32);
        for (name, &idx) in &class.methods {
            write_str(&mut buf, name);
            write_u32(&mut buf, idx as u32);
        }
        // Constructor
        write_u32(&mut buf, class.constructor.map(|c| c as u32).unwrap_or(u32::MAX));
        // Field inits
        write_u32(&mut buf, class.field_inits.len() as u32);
        for (name, chunk) in &class.field_inits {
            write_str(&mut buf, name);
            serialize_chunk(&mut buf, chunk);
        }
    }

    // Enums
    write_u32(&mut buf, program.enums.len() as u32);
    for en in &program.enums {
        write_str(&mut buf, &en.name);
        write_u32(&mut buf, en.variants.len() as u32);
        for variant in &en.variants {
            write_str(&mut buf, &variant.name);
            write_u32(&mut buf, variant.field_count as u32);
        }
    }

    // Native names
    write_u32(&mut buf, program.native_names.len() as u32);
    for name in &program.native_names {
        write_str(&mut buf, name);
    }

    buf
}

fn serialize_chunk(buf: &mut Vec<u8>, chunk: &trc::bytecode::Chunk) {
    write_u32(buf, chunk.code.len() as u32);
    buf.extend_from_slice(&chunk.code);

    write_u32(buf, chunk.constants.len() as u32);
    for &val in &chunk.constants {
        write_u64(buf, val);
    }

    write_u32(buf, chunk.strings.len() as u32);
    for s in &chunk.strings {
        write_str(buf, s);
    }

    write_u32(buf, chunk.source_lines.len() as u32);
    for &line in &chunk.source_lines {
        write_u32(buf, line);
    }
}

fn deserialize_chunk(data: &[u8], pos: &mut usize) -> Result<trc::bytecode::Chunk, String> {
    let code_len = read_u32_at(data, pos)? as usize;
    if *pos + code_len > data.len() {
        return Err("Unexpected end of data reading chunk code".to_string());
    }
    let code = data[*pos..*pos + code_len].to_vec();
    *pos += code_len;

    let const_count = read_u32_at(data, pos)? as usize;
    let mut constants = Vec::with_capacity(const_count);
    for _ in 0..const_count {
        constants.push(read_u64_at(data, pos)?);
    }

    let str_count = read_u32_at(data, pos)? as usize;
    let mut strings = Vec::with_capacity(str_count);
    for _ in 0..str_count {
        strings.push(read_str_at(data, pos)?);
    }

    let line_count = read_u32_at(data, pos)? as usize;
    let mut source_lines = Vec::with_capacity(line_count);
    for _ in 0..line_count {
        source_lines.push(read_u32_at(data, pos)?);
    }

    Ok(trc::bytecode::Chunk {
        code,
        constants,
        strings,
        source_lines,
    })
}

fn read_u32_at(data: &[u8], pos: &mut usize) -> Result<u32, String> {
    if *pos + 4 > data.len() {
        return Err("Unexpected end of data reading u32".to_string());
    }
    let val = u32::from_be_bytes([data[*pos], data[*pos + 1], data[*pos + 2], data[*pos + 3]]);
    *pos += 4;
    Ok(val)
}

fn read_u64_at(data: &[u8], pos: &mut usize) -> Result<u64, String> {
    if *pos + 8 > data.len() {
        return Err("Unexpected end of data reading u64".to_string());
    }
    let val = u64::from_be_bytes([
        data[*pos], data[*pos + 1], data[*pos + 2], data[*pos + 3],
        data[*pos + 4], data[*pos + 5], data[*pos + 6], data[*pos + 7],
    ]);
    *pos += 8;
    Ok(val)
}

fn read_str_at(data: &[u8], pos: &mut usize) -> Result<String, String> {
    let len = read_u32_at(data, pos)? as usize;
    if *pos + len > data.len() {
        return Err("Unexpected end of data reading string".to_string());
    }
    let s = String::from_utf8_lossy(&data[*pos..*pos + len]).to_string();
    *pos += len;
    Ok(s)
}

fn deserialize_compiled_program(data: &[u8]) -> Result<CompiledProgram, String> {
    let mut pos = 0;

    // Functions
    let func_count = read_u32_at(data, &mut pos)? as usize;
    let mut functions = Vec::with_capacity(func_count);
    for _ in 0..func_count {
        let name = read_str_at(data, &mut pos)?;
        let arity = read_u32_at(data, &mut pos)? as usize;
        let is_method = read_u32_at(data, &mut pos)? != 0;
        let is_constructor = read_u32_at(data, &mut pos)? != 0;
        let local_count = read_u32_at(data, &mut pos)? as usize;

        let chunk = deserialize_chunk(data, &mut pos)?;

        functions.push(trc::bytecode::frame::FunctionDef {
            name,
            arity,
            chunk,
            is_method,
            is_constructor,
            local_count,
        });
    }

    // Classes
    let class_count = read_u32_at(data, &mut pos)? as usize;
    let mut classes = Vec::with_capacity(class_count);
    for _ in 0..class_count {
        let name = read_str_at(data, &mut pos)?;
        let parent_val = read_u32_at(data, &mut pos)?;
        let parent = if parent_val == u32::MAX { None } else { Some(parent_val as u16) };

        let field_count = read_u32_at(data, &mut pos)? as usize;
        let mut fields = Vec::with_capacity(field_count);
        for _ in 0..field_count {
            let fname = read_str_at(data, &mut pos)?;
            let has_init = read_u32_at(data, &mut pos)? != 0;
            fields.push(trc::bytecode::frame::FieldDef {
                name: fname,
                has_init,
            });
        }

        let method_count = read_u32_at(data, &mut pos)? as usize;
        let mut methods = HashMap::new();
        for _ in 0..method_count {
            let mname = read_str_at(data, &mut pos)?;
            let midx = read_u32_at(data, &mut pos)? as u16;
            methods.insert(mname, midx);
        }

        let ctor_val = read_u32_at(data, &mut pos)?;
        let constructor = if ctor_val == u32::MAX { None } else { Some(ctor_val as u16) };

        let finit_count = read_u32_at(data, &mut pos)? as usize;
        let mut field_inits = Vec::with_capacity(finit_count);
        for _ in 0..finit_count {
            let finit_name = read_str_at(data, &mut pos)?;
            let chunk = deserialize_chunk(data, &mut pos)?;
            field_inits.push((finit_name, chunk));
        }

        classes.push(trc::bytecode::frame::ClassDef {
            name,
            parent,
            fields,
            methods,
            constructor,
            field_inits,
        });
    }

    // Enums
    let enum_count = read_u32_at(data, &mut pos)? as usize;
    let mut enums = Vec::with_capacity(enum_count);
    for _ in 0..enum_count {
        let name = read_str_at(data, &mut pos)?;
        let variant_count = read_u32_at(data, &mut pos)? as usize;
        let mut variants = Vec::with_capacity(variant_count);
        for _ in 0..variant_count {
            let vname = read_str_at(data, &mut pos)?;
            let fcount = read_u32_at(data, &mut pos)? as usize;
            variants.push(trc::bytecode::frame::VariantDef {
                name: vname,
                field_count: fcount,
            });
        }
        enums.push(trc::bytecode::frame::EnumDef { name, variants });
    }

    // Native names
    let native_count = read_u32_at(data, &mut pos)? as usize;
    let mut native_names = Vec::with_capacity(native_count);
    for _ in 0..native_count {
        native_names.push(read_str_at(data, &mut pos)?);
    }

    Ok(CompiledProgram {
        functions,
        classes,
        enums,
        native_names,
    })
}

fn write_u32(buf: &mut Vec<u8>, val: u32) {
    buf.extend_from_slice(&val.to_be_bytes());
}

fn write_u64(buf: &mut Vec<u8>, val: u64) {
    buf.extend_from_slice(&val.to_be_bytes());
}

fn write_str(buf: &mut Vec<u8>, s: &str) {
    write_u32(buf, s.len() as u32);
    buf.extend_from_slice(s.as_bytes());
}
