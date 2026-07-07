use std::collections::HashMap;

use trc::bytecode::CompiledProgram;

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
pub fn serialize_compiled_program(program: &CompiledProgram) -> Vec<u8> {
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

    // Global count
    write_u32(&mut buf, program.global_count as u32);

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

pub fn deserialize_compiled_program(data: &[u8]) -> Result<CompiledProgram, String> {
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
        global_count: read_u32_at(data, &mut pos)? as usize,
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
