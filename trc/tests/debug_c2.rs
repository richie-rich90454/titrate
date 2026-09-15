use trc::bytecode::opcodes::OpCode;
use std::convert::TryFrom;
use trc::lexer;
use trc::parser;
use trc::bytecode;

fn dump_chunk(code: &[u8], label: &str) {
    println!("=== {} ({} bytes) ===", label, code.len());
    let mut i = 0;
    while i < code.len() {
        let op_byte = code[i];
        let op = OpCode::try_from(op_byte).ok();
        let (name, operand_len) = match op {
            Some(op) => {
                let n = op.operand_size();
                (format!("{:?}", op), n)
            }
            None => (format!("UNKNOWN({})", op_byte), 0),
        };
        let end = (i + 1 + operand_len).min(code.len());
        let operands = &code[i+1..end];
        let operand_hex: Vec<String> = operands.iter().map(|b| format!("{:02x}", b)).collect();
        let extra = interpret_operands(op, operands);
        println!("  {:4} {:3} {:<22} {} {}", i, op_byte, name, operand_hex.join(" "), extra);
        i += 1 + operand_len;
    }
}

fn interpret_operands(op: Option<OpCode>, operands: &[u8]) -> String {
    let Some(op) = op else { return String::new(); };
    match op {
        OpCode::PUSH_I8 => format!("= {}", operands[0] as i8),
        OpCode::PUSH_I16 => {
            let v = i16::from_be_bytes([operands[0], operands[1]]);
            format!("= {}", v)
        }
        OpCode::PUSH_I32 => {
            let v = i32::from_be_bytes([operands[0], operands[1], operands[2], operands[3]]);
            format!("= {}", v)
        }
        OpCode::PUSH_I64 => {
            let mut arr = [0u8; 8];
            arr.copy_from_slice(operands);
            format!("= {}", i64::from_be_bytes(arr))
        }
        OpCode::PUSH_F32 => {
            let v = f32::from_be_bytes([operands[0], operands[1], operands[2], operands[3]]);
            format!("= {}", v)
        }
        OpCode::PUSH_F64 => {
            let mut arr = [0u8; 8];
            arr.copy_from_slice(operands);
            format!("= {}", f64::from_be_bytes(arr))
        }
        OpCode::PUSH_BOOL => format!("= {}", operands[0] != 0),
        OpCode::PUSH_STRING => {
            let idx = u16::from_be_bytes([operands[0], operands[1]]);
            format!("= str[{}]", idx)
        }
        OpCode::LOAD_LOCAL | OpCode::STORE_LOCAL => format!("= slot {}", operands[0]),
        OpCode::JMP | OpCode::JMP_IF_FALSE | OpCode::JMP_IF_TRUE => {
            let off = i16::from_be_bytes([operands[0], operands[1]]);
            format!("= +{}", off)
        }
        OpCode::CALL | OpCode::CALL_NATIVE | OpCode::CALL_SUPER => {
            let idx = u16::from_be_bytes([operands[0], operands[1]]);
            let argc = operands[2];
            format!("= fn{} args={}", idx, argc)
        }
        _ => String::new(),
    }
}

#[test]
fn debug_dump_comparison_bytecode() {
    let source = r#"
public fn compute(): double {
    return 5.0;
}
public fn main(): void {
    let totalEnergy: double = compute();
    let energyFinite: bool = totalEnergy > -10000000000.0 && totalEnergy < 10000000000.0;
    io::println(Boolean.toString(energyFinite));
}
"#;
    let tokens = lexer::tokenize(source).expect("tokenize");
    let ast = parser::parse(tokens).expect("parse");
    let mut compiler = bytecode::Compiler::new();
    let compiled = compiler.compile(&ast).expect("compile");
    for (idx, func) in compiled.functions.iter().enumerate() {
        println!("\n--- Function {} : {} (arity={}, locals={}) ---", idx, func.name, func.arity, func.local_count);
        dump_chunk(&func.chunk.code, &format!("fn[{}] {}", idx, func.name));
    }
}

#[test]
fn debug_dump_mega_test_03_main() {
    use std::fs;
    use std::path::PathBuf;
    let base_dir = PathBuf::from("../mega_test_03/src");
    let source = fs::read_to_string(base_dir.join("main.tr")).expect("read");
    let tokens = lexer::tokenize(&source).expect("tokenize");
    let ast = parser::parse(tokens).expect("parse");
    let mut compiler = bytecode::Compiler::new();
    let compiled = compiler.compile_with_modules(&ast, &base_dir).expect("compile");
    for (idx, func) in compiled.functions.iter().enumerate() {
        if func.name == "main" {
            println!("\n--- Function {} : {} (arity={}, locals={}) ---", idx, func.name, func.arity, func.local_count);
            dump_chunk(&func.chunk.code, &format!("fn[{}] {}", idx, func.name));
        }
    }
    // Also dump string table of main
    for func in compiled.functions.iter() {
        if func.name == "main" {
            println!("\n--- String table for main ---");
            for (i, s) in func.chunk.strings.iter().enumerate() {
                println!("  str[{}] = {:?}", i, s);
            }
        }
    }
}
