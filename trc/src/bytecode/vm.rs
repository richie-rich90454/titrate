// Titrate Alpha 0.2 – bytecode virtual machine
// Precision in every step – richie-rich90454, 2026

use std::char;
use std::collections::HashMap;
use std::rc::Rc;

use super::frame::{ClassDef, EnumDef, Frame, FunctionDef};
use super::opcodes::{CastTarget, Chunk, OpCode, TypeTag};
use super::value::{NativeFn, Value};

// ---------------------------------------------------------------------------
// Virtual machine
// ---------------------------------------------------------------------------

pub struct Vm {
    /// Value stack
    stack: Vec<Value>,
    /// Call frame stack
    frames: Vec<Frame>,
    /// Function table (index 0 = top-level/main chunk)
    functions: Vec<FunctionDef>,
    /// Class table
    classes: Vec<ClassDef>,
    /// Enum table
    enums: Vec<EnumDef>,
    /// Native function table
    natives: Vec<NativeFn>,
    /// Native function name → index mapping
    native_names: HashMap<String, u16>,
    /// Heap memory for references/regions
    heap: Vec<Value>,
    /// Region stack for scoped allocation
    region_stack: Vec<Vec<usize>>,
    /// Captured output
    pub output: Vec<String>,
    /// Working directory for resolving relative file paths
    working_dir: Option<std::path::PathBuf>,
}

impl Vm {
    // -----------------------------------------------------------------------
    // Construction
    // -----------------------------------------------------------------------

    pub fn new() -> Self {
        let mut vm = Vm {
            stack: Vec::new(),
            frames: Vec::new(),
            functions: Vec::new(),
            classes: Vec::new(),
            enums: Vec::new(),
            natives: Vec::new(),
            native_names: HashMap::new(),
            heap: Vec::new(),
            region_stack: Vec::new(),
            output: Vec::new(),
            working_dir: None,
        };

        // Register built-in native functions
        vm.register_native("println", native_println);
        vm.register_native("toString", native_to_string);
        vm.register_native("parseInt", native_parse_int);
        vm.register_native("Ok", native_ok);
        vm.register_native("Err", native_err);
        vm.register_native("File_readFile", native_file_read);
        vm.register_native("File_writeFile", native_file_write);
        vm.register_native("File_readLines", native_file_read_lines);
        vm.register_native("String_split", native_string_split);

        vm
    }

    // -----------------------------------------------------------------------
    // Native function registration
    // -----------------------------------------------------------------------

    pub fn register_native(&mut self, name: &str, func: NativeFn) -> u16 {
        let idx = self.natives.len() as u16;
        self.natives.push(func);
        self.native_names.insert(name.to_string(), idx);
        idx
    }

    // -----------------------------------------------------------------------
    // Public API for loading compiled code
    // -----------------------------------------------------------------------

    pub fn add_function(&mut self, def: FunctionDef) -> u16 {
        let idx = self.functions.len() as u16;
        self.functions.push(def);
        idx
    }

    pub fn add_class(&mut self, def: ClassDef) -> u16 {
        let idx = self.classes.len() as u16;
        self.classes.push(def);
        idx
    }

    pub fn add_enum(&mut self, def: EnumDef) -> u16 {
        let idx = self.enums.len() as u16;
        self.enums.push(def);
        idx
    }

    /// Load a compiled program into the VM, replacing any previously loaded code.
    pub fn load_program(&mut self, program: super::compiler::CompiledProgram) {
        self.functions = program.functions;
        self.classes = program.classes;
        self.enums = program.enums;
        // Register any native functions that the compiler discovered we need.
        for name in &program.native_names {
            if !self.native_names.contains_key(name) {
                if let Some(func) = lookup_builtin_native(name) {
                    self.register_native(name, func);
                }
            }
        }
    }

    /// Set the working directory for resolving relative file paths.
    pub fn set_working_dir(&mut self, dir: std::path::PathBuf) {
        self.working_dir = Some(dir);
    }

    /// Resolve a file path: if relative, prepend the working directory.
    fn resolve_path(&self, path: &str) -> std::path::PathBuf {
        let p = std::path::Path::new(path);
        if p.is_absolute() {
            p.to_path_buf()
        } else if let Some(ref dir) = self.working_dir {
            dir.join(path)
        } else {
            p.to_path_buf()
        }
    }

    // -----------------------------------------------------------------------
    // Stack helpers
    // -----------------------------------------------------------------------

    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    fn pop(&mut self) -> Value {
        self.stack.pop().expect("VM stack underflow")
    }

    fn peek(&self, distance: usize) -> Value {
        let idx = self.stack.len() - 1 - distance;
        self.stack[idx].clone()
    }

    // -----------------------------------------------------------------------
    // Frame / chunk helpers
    // -----------------------------------------------------------------------

    fn current_frame(&self) -> &Frame {
        self.frames.last().expect("No call frame")
    }

    fn current_frame_mut(&mut self) -> &mut Frame {
        self.frames.last_mut().expect("No call frame")
    }

    #[allow(dead_code)]
    fn current_chunk(&self) -> &Chunk {
        let fi = self.current_frame().function_index as usize;
        &self.functions[fi].chunk
    }

    // -----------------------------------------------------------------------
    // Byte reading helpers (advance IP)
    // -----------------------------------------------------------------------

    fn read_u8(&mut self) -> u8 {
        let frame = self.current_frame();
        let ip = frame.ip;
        let chunk = &self.functions[frame.function_index as usize].chunk;
        let val = chunk.code[ip];
        self.current_frame_mut().ip += 1;
        val
    }

    fn read_u16(&mut self) -> u16 {
        let frame = self.current_frame();
        let ip = frame.ip;
        let chunk = &self.functions[frame.function_index as usize].chunk;
        let hi = chunk.code[ip] as u16;
        let lo = chunk.code[ip + 1] as u16;
        let val = (hi << 8) | lo;
        self.current_frame_mut().ip += 2;
        val
    }

    fn read_i16(&mut self) -> i16 {
        let frame = self.current_frame();
        let ip = frame.ip;
        let chunk = &self.functions[frame.function_index as usize].chunk;
        let val = i16::from_be_bytes([chunk.code[ip], chunk.code[ip + 1]]);
        self.current_frame_mut().ip += 2;
        val
    }

    fn read_i32(&mut self) -> i32 {
        let frame = self.current_frame();
        let ip = frame.ip;
        let chunk = &self.functions[frame.function_index as usize].chunk;
        let val = i32::from_be_bytes([
            chunk.code[ip],
            chunk.code[ip + 1],
            chunk.code[ip + 2],
            chunk.code[ip + 3],
        ]);
        self.current_frame_mut().ip += 4;
        val
    }

    fn read_i64(&mut self) -> i64 {
        let frame = self.current_frame();
        let ip = frame.ip;
        let chunk = &self.functions[frame.function_index as usize].chunk;
        let val = i64::from_be_bytes([
            chunk.code[ip],
            chunk.code[ip + 1],
            chunk.code[ip + 2],
            chunk.code[ip + 3],
            chunk.code[ip + 4],
            chunk.code[ip + 5],
            chunk.code[ip + 6],
            chunk.code[ip + 7],
        ]);
        self.current_frame_mut().ip += 8;
        val
    }

    fn read_f32(&mut self) -> f32 {
        let frame = self.current_frame();
        let ip = frame.ip;
        let chunk = &self.functions[frame.function_index as usize].chunk;
        let val = f32::from_be_bytes([
            chunk.code[ip],
            chunk.code[ip + 1],
            chunk.code[ip + 2],
            chunk.code[ip + 3],
        ]);
        self.current_frame_mut().ip += 4;
        val
    }

    fn read_f64(&mut self) -> f64 {
        let frame = self.current_frame();
        let ip = frame.ip;
        let chunk = &self.functions[frame.function_index as usize].chunk;
        let val = f64::from_be_bytes([
            chunk.code[ip],
            chunk.code[ip + 1],
            chunk.code[ip + 2],
            chunk.code[ip + 3],
            chunk.code[ip + 4],
            chunk.code[ip + 5],
            chunk.code[ip + 6],
            chunk.code[ip + 7],
        ]);
        self.current_frame_mut().ip += 8;
        val
    }

    // -----------------------------------------------------------------------
    // Main execution loop
    // -----------------------------------------------------------------------

    pub fn run(&mut self) -> Result<(), String> {
        // Entry point is always function index 0 (main).
        if self.functions.is_empty() {
            return Err("No main function to execute".to_string());
        }

        let base = 0;
        self.frames.push(Frame::new(0, base));

        while !self.frames.is_empty() {
            self.step()?;
        }

        Ok(())
    }

    fn step(&mut self) -> Result<(), String> {
        let op_byte = self.read_u8();
        let op = OpCode::try_from(op_byte)
            .map_err(|v| format!("Unknown opcode: {}", v))?;

        match op {
            // -- Constants -------------------------------------------------------
            OpCode::PUSH_I8 => {
                let val = self.read_u8() as i8;
                self.push(Value::Byte(val));
            }
            OpCode::PUSH_I16 => {
                let val = self.read_i16();
                self.push(Value::Short(val));
            }
            OpCode::PUSH_I32 => {
                let val = self.read_i32();
                self.push(Value::Int(val));
            }
            OpCode::PUSH_I64 => {
                let val = self.read_i64();
                self.push(Value::Long(val));
            }
            OpCode::PUSH_F32 => {
                let val = self.read_f32();
                self.push(Value::Float(val));
            }
            OpCode::PUSH_F64 => {
                let val = self.read_f64();
                self.push(Value::Double(val));
            }
            OpCode::PUSH_BOOL => {
                let val = self.read_u8();
                self.push(Value::Bool(val != 0));
            }
            OpCode::PUSH_CHAR => {
                let code_point = {
                    let frame = self.current_frame();
                    let ip = frame.ip;
                    let chunk = &self.functions[frame.function_index as usize].chunk;
                    let val = u32::from_be_bytes([
                        chunk.code[ip],
                        chunk.code[ip + 1],
                        chunk.code[ip + 2],
                        chunk.code[ip + 3],
                    ]);
                    self.current_frame_mut().ip += 4;
                    val
                };
                let c = char::from_u32(code_point)
                    .ok_or_else(|| format!("Invalid Unicode code point: {}", code_point))?;
                self.push(Value::Char(c));
            }
            OpCode::PUSH_STRING => {
                let idx = self.read_u16() as usize;
                let frame = self.current_frame();
                let chunk = &self.functions[frame.function_index as usize].chunk;
                let s = chunk.strings[idx].clone();
                self.push(Value::String(Rc::new(s)));
            }
            OpCode::PUSH_NULL => {
                self.push(Value::Null);
            }
            OpCode::PUSH_VOID => {
                self.push(Value::Void);
            }

            // -- Stack -----------------------------------------------------------
            OpCode::POP => {
                self.pop();
            }
            OpCode::DUP => {
                let val = self.peek(0);
                self.push(val);
            }
            OpCode::SWAP => {
                let len = self.stack.len();
                self.stack.swap(len - 1, len - 2);
            }

            // -- Arithmetic ------------------------------------------------------
            OpCode::ADD_I32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x.wrapping_add(*y))),
                    _ => return Err(format!("ADD_I32: type mismatch {:?} + {:?}", a, b)),
                }
            }
            OpCode::ADD_I64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => self.push(Value::Long(x.wrapping_add(*y))),
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x.wrapping_add(*y))),
                    (Value::Int(x), Value::Long(y)) => self.push(Value::Long((*x as i64).wrapping_add(*y))),
                    (Value::Long(x), Value::Int(y)) => self.push(Value::Long(x.wrapping_add(*y as i64))),
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Double(x + y)),
                    (Value::Double(x), Value::Long(y)) => self.push(Value::Double(x + (*y as f64))),
                    (Value::Long(x), Value::Double(y)) => self.push(Value::Double((*x as f64) + y)),
                    (Value::Double(x), Value::Int(y)) => self.push(Value::Double(x + (*y as f64))),
                    (Value::Int(x), Value::Double(y)) => self.push(Value::Double((*x as f64) + y)),
                    (Value::Float(x), Value::Float(y)) => self.push(Value::Float(x + y)),
                    _ => return Err(format!("ADD_I64: type mismatch {:?} + {:?}", a, b)),
                }
            }
            OpCode::ADD_F32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Float(x), Value::Float(y)) => self.push(Value::Float(x + y)),
                    _ => return Err(format!("ADD_F32: type mismatch {:?} + {:?}", a, b)),
                }
            }
            OpCode::ADD_F64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Double(x + y)),
                    _ => return Err(format!("ADD_F64: type mismatch {:?} + {:?}", a, b)),
                }
            }
            OpCode::SUB_I32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x.wrapping_sub(*y))),
                    _ => return Err(format!("SUB_I32: type mismatch {:?} - {:?}", a, b)),
                }
            }
            OpCode::SUB_I64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => self.push(Value::Long(x.wrapping_sub(*y))),
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x.wrapping_sub(*y))),
                    (Value::Int(x), Value::Long(y)) => self.push(Value::Long((*x as i64).wrapping_sub(*y))),
                    (Value::Long(x), Value::Int(y)) => self.push(Value::Long(x.wrapping_sub(*y as i64))),
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Double(x - y)),
                    (Value::Double(x), Value::Long(y)) => self.push(Value::Double(x - (*y as f64))),
                    (Value::Long(x), Value::Double(y)) => self.push(Value::Double((*x as f64) - y)),
                    (Value::Double(x), Value::Int(y)) => self.push(Value::Double(x - (*y as f64))),
                    (Value::Int(x), Value::Double(y)) => self.push(Value::Double((*x as f64) - y)),
                    (Value::Float(x), Value::Float(y)) => self.push(Value::Float(x - y)),
                    _ => return Err(format!("SUB_I64: type mismatch {:?} - {:?}", a, b)),
                }
            }
            OpCode::SUB_F32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Float(x), Value::Float(y)) => self.push(Value::Float(x - y)),
                    _ => return Err(format!("SUB_F32: type mismatch {:?} - {:?}", a, b)),
                }
            }
            OpCode::SUB_F64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Double(x - y)),
                    _ => return Err(format!("SUB_F64: type mismatch {:?} - {:?}", a, b)),
                }
            }
            OpCode::MUL_I32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x.wrapping_mul(*y))),
                    _ => return Err(format!("MUL_I32: type mismatch {:?} * {:?}", a, b)),
                }
            }
            OpCode::MUL_I64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => self.push(Value::Long(x.wrapping_mul(*y))),
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x.wrapping_mul(*y))),
                    (Value::Int(x), Value::Long(y)) => self.push(Value::Long((*x as i64).wrapping_mul(*y))),
                    (Value::Long(x), Value::Int(y)) => self.push(Value::Long(x.wrapping_mul(*y as i64))),
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Double(x * y)),
                    (Value::Double(x), Value::Long(y)) => self.push(Value::Double(x * (*y as f64))),
                    (Value::Long(x), Value::Double(y)) => self.push(Value::Double((*x as f64) * y)),
                    (Value::Double(x), Value::Int(y)) => self.push(Value::Double(x * (*y as f64))),
                    (Value::Int(x), Value::Double(y)) => self.push(Value::Double((*x as f64) * y)),
                    (Value::Float(x), Value::Float(y)) => self.push(Value::Float(x * y)),
                    _ => return Err(format!("MUL_I64: type mismatch {:?} * {:?}", a, b)),
                }
            }
            OpCode::MUL_F32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Float(x), Value::Float(y)) => self.push(Value::Float(x * y)),
                    _ => return Err(format!("MUL_F32: type mismatch {:?} * {:?}", a, b)),
                }
            }
            OpCode::MUL_F64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Double(x * y)),
                    _ => return Err(format!("MUL_F64: type mismatch {:?} * {:?}", a, b)),
                }
            }
            OpCode::DIV_I32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Int(_), Value::Int(0)) => {
                        return Err("Division by zero (int)".to_string());
                    }
                    (Value::Int(x), Value::Int(y)) => {
                        self.push(Value::Int(x.wrapping_div(*y)));
                    }
                    _ => return Err(format!("DIV_I32: type mismatch {:?} / {:?}", a, b)),
                }
            }
            OpCode::DIV_I64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Long(_), Value::Long(0)) => {
                        return Err("Division by zero (long)".to_string());
                    }
                    (Value::Long(x), Value::Long(y)) => {
                        self.push(Value::Long(x.wrapping_div(*y)));
                    }
                    (Value::Int(_), Value::Int(0)) => {
                        return Err("Division by zero (int)".to_string());
                    }
                    (Value::Int(x), Value::Int(y)) => {
                        self.push(Value::Int(x.wrapping_div(*y)));
                    }
                    (Value::Int(_), Value::Long(0)) | (Value::Long(_), Value::Int(0)) => {
                        return Err("Division by zero".to_string());
                    }
                    (Value::Int(x), Value::Long(y)) => {
                        self.push(Value::Long((*x as i64).wrapping_div(*y)));
                    }
                    (Value::Long(x), Value::Int(y)) => {
                        self.push(Value::Long(x.wrapping_div(*y as i64)));
                    }
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Double(x / y)),
                    (Value::Double(x), Value::Long(y)) => self.push(Value::Double(x / (*y as f64))),
                    (Value::Long(x), Value::Double(y)) => self.push(Value::Double((*x as f64) / y)),
                    (Value::Double(x), Value::Int(y)) => self.push(Value::Double(x / (*y as f64))),
                    (Value::Int(x), Value::Double(y)) => self.push(Value::Double((*x as f64) / y)),
                    _ => return Err(format!("DIV_I64: type mismatch {:?} / {:?}", a, b)),
                }
            }
            OpCode::DIV_F32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Float(x), Value::Float(y)) => self.push(Value::Float(x / y)),
                    _ => return Err(format!("DIV_F32: type mismatch {:?} / {:?}", a, b)),
                }
            }
            OpCode::DIV_F64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Double(x / y)),
                    _ => return Err(format!("DIV_F64: type mismatch {:?} / {:?}", a, b)),
                }
            }
            OpCode::MOD_I32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Int(_), Value::Int(0)) => {
                        return Err("Remainder division by zero (int)".to_string());
                    }
                    (Value::Int(x), Value::Int(y)) => {
                        self.push(Value::Int(x.wrapping_rem(*y)));
                    }
                    _ => return Err(format!("MOD_I32: type mismatch {:?} % {:?}", a, b)),
                }
            }
            OpCode::MOD_I64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Long(_), Value::Long(0)) => {
                        return Err("Remainder division by zero (long)".to_string());
                    }
                    (Value::Long(x), Value::Long(y)) => {
                        self.push(Value::Long(x.wrapping_rem(*y)));
                    }
                    _ => return Err(format!("MOD_I64: type mismatch {:?} % {:?}", a, b)),
                }
            }
            OpCode::MOD_F32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Float(x), Value::Float(y)) => self.push(Value::Float(x % y)),
                    _ => return Err(format!("MOD_F32: type mismatch {:?} % {:?}", a, b)),
                }
            }
            OpCode::MOD_F64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Double(x % y)),
                    _ => return Err(format!("MOD_F64: type mismatch {:?} % {:?}", a, b)),
                }
            }
            OpCode::NEG_I32 => {
                let a = self.pop();
                match a {
                    Value::Int(x) => self.push(Value::Int(x.wrapping_neg())),
                    _ => return Err(format!("NEG_I32: type mismatch {:?}", a)),
                }
            }
            OpCode::NEG_I64 => {
                let a = self.pop();
                match a {
                    Value::Long(x) => self.push(Value::Long(x.wrapping_neg())),
                    _ => return Err(format!("NEG_I64: type mismatch {:?}", a)),
                }
            }
            OpCode::NEG_F32 => {
                let a = self.pop();
                match a {
                    Value::Float(x) => self.push(Value::Float(-x)),
                    _ => return Err(format!("NEG_F32: type mismatch {:?}", a)),
                }
            }
            OpCode::NEG_F64 => {
                let a = self.pop();
                match a {
                    Value::Double(x) => self.push(Value::Double(-x)),
                    _ => return Err(format!("NEG_F64: type mismatch {:?}", a)),
                }
            }

            // -- Bitwise ---------------------------------------------------------
            OpCode::BITAND_I32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x & y)),
                    _ => return Err(format!("BITAND_I32: type mismatch {:?}", a)),
                }
            }
            OpCode::BITAND_I64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => self.push(Value::Long(x & y)),
                    _ => return Err(format!("BITAND_I64: type mismatch {:?}", a)),
                }
            }
            OpCode::BITOR_I32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x | y)),
                    _ => return Err(format!("BITOR_I32: type mismatch {:?}", a)),
                }
            }
            OpCode::BITOR_I64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => self.push(Value::Long(x | y)),
                    _ => return Err(format!("BITOR_I64: type mismatch {:?}", a)),
                }
            }
            OpCode::BITXOR_I32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x ^ y)),
                    _ => return Err(format!("BITXOR_I32: type mismatch {:?}", a)),
                }
            }
            OpCode::BITXOR_I64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => self.push(Value::Long(x ^ y)),
                    _ => return Err(format!("BITXOR_I64: type mismatch {:?}", a)),
                }
            }
            OpCode::SHL_I32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x.wrapping_shl(*y as u32))),
                    _ => return Err(format!("SHL_I32: type mismatch {:?}", a)),
                }
            }
            OpCode::SHL_I64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => self.push(Value::Long(x.wrapping_shl(*y as u32))),
                    _ => return Err(format!("SHL_I64: type mismatch {:?}", a)),
                }
            }
            OpCode::SHR_I32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x.wrapping_shr(*y as u32))),
                    _ => return Err(format!("SHR_I32: type mismatch {:?}", a)),
                }
            }
            OpCode::SHR_I64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => self.push(Value::Long(x.wrapping_shr(*y as u32))),
                    _ => return Err(format!("SHR_I64: type mismatch {:?}", a)),
                }
            }
            OpCode::BITNOT_I32 => {
                let a = self.pop();
                match a {
                    Value::Int(x) => self.push(Value::Int(!x)),
                    _ => return Err(format!("BITNOT_I32: type mismatch {:?}", a)),
                }
            }
            OpCode::BITNOT_I64 => {
                let a = self.pop();
                match a {
                    Value::Long(x) => self.push(Value::Long(!x)),
                    _ => return Err(format!("BITNOT_I64: type mismatch {:?}", a)),
                }
            }

            // -- Comparison ------------------------------------------------------
            OpCode::EQ_I32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x == y)),
                    _ => return Err(format!("EQ_I32: type mismatch {:?}", a)),
                }
            }
            OpCode::EQ_I64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => self.push(Value::Bool(x == y)),
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x == y)),
                    (Value::Int(x), Value::Long(y)) => self.push(Value::Bool((*x as i64) == *y)),
                    (Value::Long(x), Value::Int(y)) => self.push(Value::Bool(*x == (*y as i64))),
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Bool(x == y)),
                    _ => return Err(format!("EQ_I64: type mismatch {:?} == {:?}", a, b)),
                }
            }
            OpCode::EQ_F32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Float(x), Value::Float(y)) => {
                        self.push(Value::Bool(x.to_bits() == y.to_bits()))
                    }
                    _ => return Err(format!("EQ_F32: type mismatch {:?}", a)),
                }
            }
            OpCode::EQ_F64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Double(x), Value::Double(y)) => {
                        self.push(Value::Bool(x.to_bits() == y.to_bits()))
                    }
                    _ => return Err(format!("EQ_F64: type mismatch {:?}", a)),
                }
            }
            OpCode::EQ_BOOL => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Bool(x), Value::Bool(y)) => self.push(Value::Bool(x == y)),
                    _ => return Err(format!("EQ_BOOL: type mismatch {:?}", a)),
                }
            }
            OpCode::EQ_CHAR => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Char(x), Value::Char(y)) => self.push(Value::Bool(x == y)),
                    _ => return Err(format!("EQ_CHAR: type mismatch {:?}", a)),
                }
            }
            OpCode::EQ_STRING => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::String(x), Value::String(y)) => self.push(Value::Bool(x == y)),
                    (Value::Null, Value::String(_)) | (Value::String(_), Value::Null) => self.push(Value::Bool(false)),
                    (Value::Null, Value::Null) => self.push(Value::Bool(true)),
                    _ => return Err(format!("EQ_STRING: type mismatch {:?}", a)),
                }
            }
            OpCode::NE_I32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x != y)),
                    _ => return Err(format!("NE_I32: type mismatch {:?}", a)),
                }
            }
            OpCode::NE_I64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => self.push(Value::Bool(x != y)),
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x != y)),
                    (Value::Int(x), Value::Long(y)) => self.push(Value::Bool((*x as i64) != *y)),
                    (Value::Long(x), Value::Int(y)) => self.push(Value::Bool(*x != (*y as i64))),
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Bool(x != y)),
                    _ => return Err(format!("NE_I64: type mismatch {:?} != {:?}", a, b)),
                }
            }
            OpCode::NE_F32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Float(x), Value::Float(y)) => {
                        self.push(Value::Bool(x.to_bits() != y.to_bits()))
                    }
                    _ => return Err(format!("NE_F32: type mismatch {:?}", a)),
                }
            }
            OpCode::NE_F64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Double(x), Value::Double(y)) => {
                        self.push(Value::Bool(x.to_bits() != y.to_bits()))
                    }
                    _ => return Err(format!("NE_F64: type mismatch {:?}", a)),
                }
            }
            OpCode::LT_I32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x < y)),
                    _ => return Err(format!("LT_I32: type mismatch {:?}", a)),
                }
            }
            OpCode::LT_I64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => self.push(Value::Bool(x < y)),
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x < y)),
                    (Value::Int(x), Value::Long(y)) => self.push(Value::Bool((*x as i64) < *y)),
                    (Value::Long(x), Value::Int(y)) => self.push(Value::Bool(*x < (*y as i64))),
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Bool(x < y)),
                    (Value::Double(x), Value::Long(y)) => self.push(Value::Bool(x < &(*y as f64))),
                    (Value::Long(x), Value::Double(y)) => self.push(Value::Bool(&(*x as f64) < y)),
                    (Value::Double(x), Value::Int(y)) => self.push(Value::Bool(x < &(*y as f64))),
                    (Value::Int(x), Value::Double(y)) => self.push(Value::Bool(&(*x as f64) < y)),
                    _ => return Err(format!("LT_I64: type mismatch {:?} < {:?}", a, b)),
                }
            }
            OpCode::LT_F32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Float(x), Value::Float(y)) => self.push(Value::Bool(x < y)),
                    _ => return Err(format!("LT_F32: type mismatch {:?}", a)),
                }
            }
            OpCode::LT_F64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Bool(x < y)),
                    _ => return Err(format!("LT_F64: type mismatch {:?}", a)),
                }
            }
            OpCode::LE_I32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x <= y)),
                    _ => return Err(format!("LE_I32: type mismatch {:?}", a)),
                }
            }
            OpCode::LE_I64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => self.push(Value::Bool(x <= y)),
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x <= y)),
                    (Value::Int(x), Value::Long(y)) => self.push(Value::Bool((*x as i64) <= *y)),
                    (Value::Long(x), Value::Int(y)) => self.push(Value::Bool(*x <= (*y as i64))),
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Bool(x <= y)),
                    (Value::Double(x), Value::Long(y)) => self.push(Value::Bool(x <= &(*y as f64))),
                    (Value::Long(x), Value::Double(y)) => self.push(Value::Bool(&(*x as f64) <= y)),
                    (Value::Double(x), Value::Int(y)) => self.push(Value::Bool(x <= &(*y as f64))),
                    (Value::Int(x), Value::Double(y)) => self.push(Value::Bool(&(*x as f64) <= y)),
                    _ => return Err(format!("LE_I64: type mismatch {:?} <= {:?}", a, b)),
                }
            }
            OpCode::LE_F32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Float(x), Value::Float(y)) => self.push(Value::Bool(x <= y)),
                    _ => return Err(format!("LE_F32: type mismatch {:?}", a)),
                }
            }
            OpCode::LE_F64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Bool(x <= y)),
                    _ => return Err(format!("LE_F64: type mismatch {:?}", a)),
                }
            }
            OpCode::GT_I32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x > y)),
                    _ => return Err(format!("GT_I32: type mismatch {:?}", a)),
                }
            }
            OpCode::GT_I64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => self.push(Value::Bool(x > y)),
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x > y)),
                    (Value::Int(x), Value::Long(y)) => self.push(Value::Bool((*x as i64) > *y)),
                    (Value::Long(x), Value::Int(y)) => self.push(Value::Bool(*x > (*y as i64))),
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Bool(x > y)),
                    (Value::Double(x), Value::Long(y)) => self.push(Value::Bool(x > &(*y as f64))),
                    (Value::Long(x), Value::Double(y)) => self.push(Value::Bool(&(*x as f64) > y)),
                    (Value::Double(x), Value::Int(y)) => self.push(Value::Bool(x > &(*y as f64))),
                    (Value::Int(x), Value::Double(y)) => self.push(Value::Bool(&(*x as f64) > y)),
                    _ => return Err(format!("GT_I64: type mismatch {:?} > {:?}", a, b)),
                }
            }
            OpCode::GT_F32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Float(x), Value::Float(y)) => self.push(Value::Bool(x > y)),
                    _ => return Err(format!("GT_F32: type mismatch {:?}", a)),
                }
            }
            OpCode::GT_F64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Bool(x > y)),
                    _ => return Err(format!("GT_F64: type mismatch {:?}", a)),
                }
            }
            OpCode::GE_I32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x >= y)),
                    _ => return Err(format!("GE_I32: type mismatch {:?}", a)),
                }
            }
            OpCode::GE_I64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => self.push(Value::Bool(x >= y)),
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x >= y)),
                    (Value::Int(x), Value::Long(y)) => self.push(Value::Bool((*x as i64) >= *y)),
                    (Value::Long(x), Value::Int(y)) => self.push(Value::Bool(*x >= (*y as i64))),
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Bool(x >= y)),
                    (Value::Double(x), Value::Long(y)) => self.push(Value::Bool(x >= &(*y as f64))),
                    (Value::Long(x), Value::Double(y)) => self.push(Value::Bool(&(*x as f64) >= y)),
                    (Value::Double(x), Value::Int(y)) => self.push(Value::Bool(x >= &(*y as f64))),
                    (Value::Int(x), Value::Double(y)) => self.push(Value::Bool(&(*x as f64) >= y)),
                    _ => return Err(format!("GE_I64: type mismatch {:?} >= {:?}", a, b)),
                }
            }
            OpCode::GE_F32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Float(x), Value::Float(y)) => self.push(Value::Bool(x >= y)),
                    _ => return Err(format!("GE_F32: type mismatch {:?}", a)),
                }
            }
            OpCode::GE_F64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Bool(x >= y)),
                    _ => return Err(format!("GE_F64: type mismatch {:?}", a)),
                }
            }

            // -- Logic -----------------------------------------------------------
            OpCode::AND => {
                let b = self.pop();
                let a = self.pop();
                self.push(Value::Bool(a.is_truthy() && b.is_truthy()));
            }
            OpCode::OR => {
                let b = self.pop();
                let a = self.pop();
                self.push(Value::Bool(a.is_truthy() || b.is_truthy()));
            }
            OpCode::NOT => {
                let a = self.pop();
                self.push(Value::Bool(!a.is_truthy()));
            }

            // -- String ----------------------------------------------------------
            OpCode::STR_CONCAT => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::String(x), Value::String(y)) => {
                        let mut result = (**x).clone();
                        result.push_str(&**y);
                        self.push(Value::String(Rc::new(result)));
                    }
                    _ => return Err(format!("STR_CONCAT: type mismatch {:?}, {:?}", a, b)),
                }
            }
            OpCode::STR_CONCAT_RIGHT => {
                let b = self.pop();
                let a = self.pop();
                match &a {
                    Value::String(x) => {
                        let result = format!("{}{}", &**x, b.display_string());
                        self.push(Value::String(Rc::new(result)));
                    }
                    _ => return Err(format!("STR_CONCAT_RIGHT: left must be String, got {:?}", a)),
                }
            }
            OpCode::STR_CONCAT_LEFT => {
                let b = self.pop();
                let a = self.pop();
                match &b {
                    Value::String(y) => {
                        let result = format!("{}{}", a.display_string(), &**y);
                        self.push(Value::String(Rc::new(result)));
                    }
                    _ => return Err(format!("STR_CONCAT_LEFT: right must be String, got {:?}", b)),
                }
            }

            // -- Control flow ----------------------------------------------------
            OpCode::JMP => {
                let offset = self.read_i16();
                // offset is relative to the IP *after* reading the operand
                let new_ip = self.current_frame().ip as isize + offset as isize;
                self.current_frame_mut().ip = new_ip as usize;
            }
            OpCode::JMP_IF_FALSE => {
                let offset = self.read_i16();
                let val = self.pop();
                if !val.is_truthy() {
                    let new_ip = self.current_frame().ip as isize + offset as isize;
                    self.current_frame_mut().ip = new_ip as usize;
                }
            }
            OpCode::JMP_IF_TRUE => {
                let offset = self.read_i16();
                let val = self.pop();
                if val.is_truthy() {
                    let new_ip = self.current_frame().ip as isize + offset as isize;
                    self.current_frame_mut().ip = new_ip as usize;
                }
            }
            OpCode::CALL => {
                let func_idx = self.read_u16();
                let arg_count = self.read_u8();
                self.call_function(func_idx, arg_count)?;
            }
            OpCode::RET => {
                let return_value = self.pop();
                let frame = self.frames.pop().expect("No frame to return from");
                // If returning from a constructor, the result is the instance ("this")
                let result = if self.functions[frame.function_index as usize].is_constructor {
                    self.stack[frame.base].clone()
                } else {
                    return_value
                };
                // If this was the last frame, push the return value and we're done
                if self.frames.is_empty() {
                    self.push(result);
                } else {
                    // Trim the callee's locals off the stack
                    self.stack.truncate(frame.base);
                    self.push(result);
                }
            }
            OpCode::CALL_NATIVE => {
                let native_idx = self.read_u16();
                let arg_count = self.read_u8();
                self.call_native_fn(native_idx, arg_count)?;
            }

            // -- Variables -------------------------------------------------------
            OpCode::LOAD_LOCAL => {
                let slot = self.read_u8();
                let base = self.current_frame().base;
                let idx = base + slot as usize;
                if idx < self.stack.len() {
                    let val = self.stack[idx].clone();
                    self.push(val);
                } else {
                    // Slot was pre-allocated but popped by end_scope or similar.
                    // Push Null as the default value.
                    self.push(Value::Null);
                }
            }
            OpCode::STORE_LOCAL => {
                let slot = self.read_u8();
                let val = self.stack.pop().unwrap_or(Value::Null);
                let base = self.current_frame().base;
                let idx = base + slot as usize;
                // Ensure the stack is large enough to hold this slot.
                // This handles the case where pre-allocated slots were popped
                // by end_scope or other operations.
                while self.stack.len() <= idx {
                    self.stack.push(Value::Null);
                }
                self.stack[idx] = val;
            }
            OpCode::LOAD_UPVALUE => {
                let _slot = self.read_u8();
                // Upvalues are not yet implemented; treat as a no-op placeholder
                return Err("LOAD_UPVALUE: closures not yet implemented".to_string());
            }
            OpCode::STORE_UPVALUE => {
                let _slot = self.read_u8();
                return Err("STORE_UPVALUE: closures not yet implemented".to_string());
            }

            // -- Objects ---------------------------------------------------------
            OpCode::NEW => {
                let class_idx = self.read_u16();
                self.exec_new(class_idx)?;
            }
            OpCode::INVOKE_VIRTUAL => {
                let method_name_idx = self.read_u16();
                let arg_count = self.read_u8();
                self.invoke_method(method_name_idx, arg_count)?;
            }
            OpCode::GET_FIELD => {
                let field_name_idx = self.read_u16();
                let field_name = {
                    let frame = self.current_frame();
                    let chunk = &self.functions[frame.function_index as usize].chunk;
                    chunk.strings[field_name_idx as usize].clone()
                };
                let instance = self.pop();
                match &instance {
                    Value::ClassInstance { fields, .. } => {
                        let val = fields
                            .borrow()
                            .get(&field_name)
                            .cloned()
                            .unwrap_or(Value::Null);
                        self.push(val);
                    }
                    _ => {
                        return Err(format!(
                            "GET_FIELD: cannot get field '{}' on {:?}",
                            field_name, instance
                        ))
                    }
                }
            }
            OpCode::SET_FIELD => {
                let field_name_idx = self.read_u16();
                let field_name = {
                    let frame = self.current_frame();
                    let chunk = &self.functions[frame.function_index as usize].chunk;
                    chunk.strings[field_name_idx as usize].clone()
                };
                let instance = self.pop();
                let value = self.pop();
                match &instance {
                    Value::ClassInstance { fields, .. } => {
                        fields.borrow_mut().insert(field_name, value.clone());
                        self.push(value);
                    }
                    _ => {
                        return Err(format!(
                            "SET_FIELD: cannot set field '{}' on {:?}",
                            field_name, instance
                        ))
                    }
                }
            }

            // -- Arrays ----------------------------------------------------------
            OpCode::ARRAY_NEW => {
                let size = self.read_u16() as usize;
                let mut elements = Vec::with_capacity(size);
                for _ in 0..size {
                    elements.push(Value::Null);
                }
                // Pop `size` values from the stack and fill in reverse order
                for i in (0..size).rev() {
                    elements[i] = self.pop();
                }
                self.push(Value::Array { elements });
            }
            OpCode::ARRAY_GET => {
                let index = self.pop();
                let array = self.pop();
                match (&array, &index) {
                    (Value::String(s), Value::Int(i)) => {
                        let idx = *i as usize;
                        match s.chars().nth(idx) {
                            Some(ch) => self.push(Value::Char(ch)),
                            None => return Err(format!("String index out of bounds: {}", idx)),
                        }
                    }
                    (Value::Array { elements }, Value::Int(i)) => {
                        let idx = *i as usize;
                        if idx < elements.len() {
                            self.push(elements[idx].clone());
                        } else {
                            return Err(format!("Array index out of bounds: {}", idx));
                        }
                    }
                    (Value::Array { elements }, Value::Long(i)) => {
                        let idx = *i as usize;
                        if idx < elements.len() {
                            self.push(elements[idx].clone());
                        } else {
                            return Err(format!("Array index out of bounds: {}", idx));
                        }
                    }
                    // ArrayList support: get from _elements field
                    (Value::ClassInstance { class_name, fields, .. }, Value::Int(i))
                        if class_name.starts_with("ArrayList") =>
                    {
                        let idx = *i as usize;
                        match fields.borrow().get("_elements") {
                            Some(Value::Array { elements }) => {
                                if idx < elements.len() {
                                    self.push(elements[idx].clone());
                                } else {
                                    return Err(format!("ArrayList index out of bounds: {}", idx));
                                }
                            }
                            _ => return Err("ArrayList has no elements".to_string()),
                        }
                    }
                    (Value::ClassInstance { class_name, fields, .. }, Value::Long(i))
                        if class_name.starts_with("ArrayList") =>
                    {
                        let idx = *i as usize;
                        match fields.borrow().get("_elements") {
                            Some(Value::Array { elements }) => {
                                if idx < elements.len() {
                                    self.push(elements[idx].clone());
                                } else {
                                    return Err(format!("ArrayList index out of bounds: {}", idx));
                                }
                            }
                            _ => return Err("ArrayList has no elements".to_string()),
                        }
                    }
                    _ => {
                        return Err(format!(
                            "ARRAY_GET: invalid index type on array: {:?}[{:?}]",
                            array, index
                        ))
                    }
                }
            }
            OpCode::ARRAY_SET => {
                let index = self.pop();
                let array = self.pop();
                let value = self.pop();
                match (&array, &index) {
                    (Value::Array { .. }, Value::Int(i)) => {
                        let idx = *i as usize;
                        let mut elements = match array {
                            Value::Array { elements } => elements,
                            _ => unreachable!(),
                        };
                        if idx < elements.len() {
                            elements[idx] = value.clone();
                            self.push(Value::Array { elements });
                        } else {
                            return Err(format!("Array index out of bounds: {}", idx));
                        }
                    }
                    (Value::Array { .. }, Value::Long(i)) => {
                        let idx = *i as usize;
                        let mut elements = match array {
                            Value::Array { elements } => elements,
                            _ => unreachable!(),
                        };
                        if idx < elements.len() {
                            elements[idx] = value.clone();
                            self.push(Value::Array { elements });
                        } else {
                            return Err(format!("Array index out of bounds: {}", idx));
                        }
                    }
                    _ => {
                        return Err(format!(
                            "ARRAY_SET: invalid index type on array: {:?}[{:?}]",
                            array, index
                        ))
                    }
                }
            }
            OpCode::ARRAY_LEN => {
                let array = self.pop();
                match array {
                    Value::Array { elements } => {
                        self.push(Value::Long(elements.len() as i64));
                    }
                    Value::ClassInstance { class_name, fields, .. } if class_name.starts_with("ArrayList") => {
                        match fields.borrow().get("_elements") {
                            Some(Value::Array { elements }) => {
                                self.push(Value::Long(elements.len() as i64));
                            }
                            _ => self.push(Value::Long(0)),
                        }
                    }
                    _ => return Err(format!("ARRAY_LEN: not an array: {:?}", array)),
                }
            }

            // -- Ownership -------------------------------------------------------
            OpCode::BOX_VALUE => {
                let val = self.pop();
                self.push(Value::Owned(Box::new(val)));
            }
            OpCode::UNBOX_VALUE => {
                let val = self.pop();
                match val {
                    Value::Owned(inner) => self.push(*inner),
                    _ => return Err(format!("UNBOX_VALUE: not an Owned value: {:?}", val)),
                }
            }
            OpCode::REGION_ALLOC => {
                let val = self.pop();
                let idx = self.heap.len();
                self.heap.push(val);
                if let Some(region) = self.region_stack.last_mut() {
                    region.push(idx);
                }
                self.push(Value::Ref(idx));
            }
            OpCode::FREE_REGION => {
                // Pop the current region and mark its heap slots as freed.
                if let Some(indices) = self.region_stack.pop() {
                    for idx in indices {
                        if idx < self.heap.len() {
                            self.heap[idx] = Value::Null;
                        }
                    }
                }
            }
            OpCode::REF_IMMUTABLE => {
                let val = self.pop();
                let idx = self.heap.len();
                self.heap.push(val);
                self.push(Value::Ref(idx));
            }
            OpCode::REF_MUTABLE => {
                let val = self.pop();
                let idx = self.heap.len();
                self.heap.push(val);
                self.push(Value::Ref(idx));
            }
            OpCode::DEREF => {
                let val = self.pop();
                match val {
                    Value::Ref(idx) => {
                        let heap_val = self.heap.get(idx).cloned().ok_or_else(|| {
                            format!("DEREF: invalid heap reference {}", idx)
                        })?;
                        self.push(heap_val);
                    }
                    _ => return Err(format!("DEREF: not a Ref: {:?}", val)),
                }
            }

            // -- Enum ------------------------------------------------------------
            OpCode::ENUM_NEW => {
                let enum_name_idx = self.read_u16();
                let variant_name_idx = self.read_u16();
                let field_count = self.read_u8() as usize;
                // Look up strings from the current chunk
                let (enum_name, variant) = {
                    let frame = self.current_frame();
                    let chunk = &self.functions[frame.function_index as usize].chunk;
                    (
                        chunk.strings[enum_name_idx as usize].clone(),
                        chunk.strings[variant_name_idx as usize].clone(),
                    )
                };
                let mut fields = Vec::with_capacity(field_count);
                for _ in 0..field_count {
                    fields.push(self.pop());
                }
                fields.reverse();
                self.push(Value::EnumInstance {
                    enum_name,
                    variant,
                    fields,
                });
            }

            // -- Result ----------------------------------------------------------
            OpCode::RESULT_OK => {
                let val = self.pop();
                self.push(Value::ResultOk(Box::new(val)));
            }
            OpCode::RESULT_ERR => {
                let val = self.pop();
                self.push(Value::ResultErr(Box::new(val)));
            }
            OpCode::UNWRAP_OR_PROPAGATE => {
                let val = self.pop();
                match &val {
                    Value::ResultErr(_) => {
                        // Propagate: return from the current function with the Err
                        let frame = self.frames.pop().expect("No frame to return from");
                        if self.frames.is_empty() {
                            self.push(val);
                        } else {
                            self.stack.truncate(frame.base);
                            self.push(val);
                            // Signal that we need to keep propagating up
                            // We do this by returning from this step and letting
                            // the outer frame encounter the ResultErr on its stack
                            // and continue propagation.
                            // Actually, we should keep popping frames until we find
                            // a handler. The simplest approach: return the Err as
                            // the function's return value. The caller's code will
                            // have another UNWRAP_OR_PROPAGATE if needed.
                            return Ok(());
                        }
                    }
                    Value::ResultOk(inner) => {
                        self.push(*inner.clone());
                    }
                    _ => {
                        return Err(format!(
                            "UNWRAP_OR_PROPAGATE: not a Result: {:?}",
                            val
                        ))
                    }
                }
            }

            // -- Cast ------------------------------------------------------------
            OpCode::CAST => {
                let target_byte = self.read_u8();
                let target = CastTarget::try_from(target_byte)
                    .map_err(|v| format!("Unknown cast target: {}", v))?;
                let val = self.pop();
                let result = self.eval_cast(&val, target)?;
                self.push(result);
            }

            // -- Iteration -------------------------------------------------------
            OpCode::ITER_NEXT => {
                let offset = self.read_i16();
                // The iterator state is stored as a local variable.
                // We expect the top of stack to hold the iterator value.
                // If it's exhausted (Null or past end), jump by offset.
                // Otherwise push the next element.
                // For simplicity, the compiler manages the iterator counter
                // as a local. The VM just checks: if the value on top is
                // a "sentinel" (Null), jump. Otherwise the value is already
                // the next element.
                let val = self.peek(0);
                if val == Value::Null {
                    // Exhausted – consume the iterator marker and jump
                    self.pop();
                    let new_ip = self.current_frame().ip as isize + offset as isize;
                    self.current_frame_mut().ip = new_ip as usize;
                }
                // If not Null, the value stays on the stack as the iteration value
            }

            // -- Pattern matching ------------------------------------------------
            OpCode::MATCH_ENUM => {
                let variant_name_idx = self.read_u16();
                let _offset = self.read_i16(); // consumed but not used for jumping
                let val = self.pop();
                match &val {
                    Value::EnumInstance { variant, fields, .. } => {
                        let expected_variant = {
                            let frame = self.current_frame();
                            let chunk = &self.functions[frame.function_index as usize].chunk;
                            chunk.strings[variant_name_idx as usize].clone()
                        };
                        if variant == &expected_variant {
                            // Match: push the fields as individual values, then true
                            for f in fields {
                                self.push(f.clone());
                            }
                            self.push(Value::Bool(true));
                        } else {
                            // No match: push false
                            self.push(Value::Bool(false));
                        }
                    }
                    _ => {
                        // Not an enum instance: push false
                        self.push(Value::Bool(false));
                    }
                }
            }
            OpCode::MATCH_OK => {
                let _offset = self.read_i16(); // consumed but not used for jumping
                let val = self.pop();
                match &val {
                    Value::ResultOk(inner) => {
                        self.push(*inner.clone());
                        self.push(Value::Bool(true));
                    }
                    _ => {
                        // Not ResultOk: push false
                        self.push(Value::Bool(false));
                    }
                }
            }
            OpCode::MATCH_ERR => {
                let _offset = self.read_i16(); // consumed but not used for jumping
                let val = self.pop();
                match &val {
                    Value::ResultErr(inner) => {
                        self.push(*inner.clone());
                        self.push(Value::Bool(true));
                    }
                    _ => {
                        // Not ResultErr: push false
                        self.push(Value::Bool(false));
                    }
                }
            }

            // -- Static calls ----------------------------------------------------
            OpCode::STATIC_CALL => {
                let class_name_idx = self.read_u16();
                let method_name_idx = self.read_u16();
                let arg_count = self.read_u8();
                self.exec_static_call(class_name_idx, method_name_idx, arg_count)?;
            }

            // -- Type narrowing --------------------------------------------------
            OpCode::TYPE_CHECK => {
                let tag_byte = self.read_u8();
                let tag = TypeTag::try_from(tag_byte)
                    .map_err(|v| format!("Unknown type tag: {}", v))?;
                let val = self.pop();
                let matches = self.check_type_tag(&val, tag);
                self.push(Value::Bool(matches));
            }

            // -- Super constructor call -----------------------------------------
            OpCode::CALL_SUPER => {
                // Operands: func_idx (u16), user_arg_count (u8)
                // Stack: [this, user_arg0, user_arg1, ...]
                // This is like CALL but the base is set to `this` position
                // and arity check uses user_arg_count (not including `this`).
                let func_idx = self.read_u16();
                let user_arg_count = self.read_u8() as usize;
                let fi = func_idx as usize;
                if fi >= self.functions.len() {
                    return Err(format!("CALL_SUPER: function index {} out of range", func_idx));
                }
                // The total items on stack for this call: 1 (this) + user_arg_count
                // Base points to `this`
                let base = self.stack.len() - 1 - user_arg_count;
                self.frames.push(Frame::new(func_idx, base));
                // Pre-allocate local slots
                let local_count = self.functions[fi].local_count;
                let needed = base + local_count;
                while self.stack.len() < needed {
                    self.stack.push(Value::Null);
                }
            }
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Function calls
    // -----------------------------------------------------------------------

    fn call_function(&mut self, func_idx: u16, arg_count: u8) -> Result<(), String> {
        let fi = func_idx as usize;
        if fi >= self.functions.len() {
            return Err(format!("CALL: function index {} out of range", func_idx));
        }

        let arity = self.functions[fi].arity;
        if (arg_count as usize) != arity {
            return Err(format!(
                "CALL: function {} expects {} args, got {}",
                self.functions[fi].name, arity, arg_count
            ));
        }

        let base = self.stack.len() - arg_count as usize;
        self.frames.push(Frame::new(func_idx, base));

        // Pre-allocate stack slots for all local variables.
        // The function has `local_count` total slots, of which `arity` are
        // already occupied by arguments. Fill the rest with Null.
        let local_count = self.functions[fi].local_count;
        let needed = base + local_count;
        while self.stack.len() < needed {
            self.stack.push(Value::Null);
        }

        Ok(())
    }

    fn call_native_fn(&mut self, native_idx: u16, arg_count: u8) -> Result<(), String> {
        let ni = native_idx as usize;
        if ni >= self.natives.len() {
            return Err(format!(
                "CALL_NATIVE: native index {} out of range",
                native_idx
            ));
        }

        let arg_start = self.stack.len() - arg_count as usize;
        let args: Vec<Value> = self.stack.drain(arg_start..).collect();

        // Special handling for println (native index 0): capture output
        if ni == 0 {
            // println
            let output = if args.is_empty() {
                String::new()
            } else {
                args[0].display_string()
            };
            self.output.push(output);
            self.push(Value::Void);
            return Ok(());
        }

        let func = self.natives[ni];
        let result = func(&args)?;
        self.push(result);
        Ok(())
    }

    fn invoke_method(&mut self, method_name_idx: u16, arg_count: u8) -> Result<(), String> {
        // Stack: [receiver, arg0, arg1, ...]
        // The receiver is already on the stack before the args.
        // Total items on stack for this call: 1 (receiver) + arg_count
        let method_name = {
            let frame = self.current_frame();
            let chunk = &self.functions[frame.function_index as usize].chunk;
            chunk.strings[method_name_idx as usize].clone()
        };

        // The receiver is at stack.len() - 1 - arg_count
        let receiver_idx = self.stack.len() - 1 - arg_count as usize;
        let receiver = self.stack[receiver_idx].clone();

        match &receiver {
            Value::ClassInstance {
                vtable, class_name, fields, ..
            } => {
                // Handle built-in ArrayList/HashMap methods
                match class_name.as_str() {
                    n if n.starts_with("ArrayList") => {
                        let result = self.call_arraylist_method(fields, &method_name, arg_count)?;
                        // Pop receiver + args, push result
                        let drain_start = receiver_idx;
                        self.stack.drain(drain_start..);
                        self.push(result);
                        return Ok(());
                    }
                    n if n.starts_with("HashMap") => {
                        let result = self.call_hashmap_method(fields, &method_name, arg_count)?;
                        let drain_start = receiver_idx;
                        self.stack.drain(drain_start..);
                        self.push(result);
                        return Ok(());
                    }
                    _ => {}
                }

                // Look up method in vtable
                let func_idx = if let Some(idx) = vtable.get(&method_name) {
                    *idx
                } else {
                    // Walk up the class hierarchy to find the method
                    let mut search_class = class_name.clone();
                    let mut found_idx = None;
                    loop {
                        let class_defs = &self.classes;
                        let found = class_defs.iter().find(|c| c.name == search_class);
                        match found {
                            Some(cd) => {
                                if let Some(idx) = cd.methods.get(&method_name) {
                                    found_idx = Some(*idx);
                                    break;
                                }
                                // Check parent class
                                if let Some(parent_idx) = cd.parent {
                                    search_class = self.classes[parent_idx as usize].name.clone();
                                } else {
                                    break;
                                }
                            }
                            None => break,
                        }
                    }
                    match found_idx {
                        Some(idx) => idx,
                        None => {
                            return Err(format!(
                                "No method '{}' on class '{}'",
                                method_name, class_name
                            ));
                        }
                    }
                };

                let base = receiver_idx;
                self.frames.push(Frame::new(func_idx, base));
            }
            Value::String(s) => {
                // Handle string methods
                match method_name.as_str() {
                    "length" => {
                        self.stack.drain(receiver_idx..);
                        self.push(Value::Int(s.len() as i32));
                    }
                    "toString" => {
                        self.stack.drain(receiver_idx..);
                        self.push(Value::String(s.clone()));
                    }
                    _ => {
                        return Err(format!(
                            "No method '{}' on string",
                            method_name
                        ))
                    }
                }
            }
            _ => {
                return Err(format!(
                    "INVOKE_VIRTUAL: cannot invoke '{}' on {:?}",
                    method_name, receiver
                ))
            }
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Built-in ArrayList methods
    // -----------------------------------------------------------------------

    fn call_arraylist_method(
        &mut self,
        fields: &std::rc::Rc<std::cell::RefCell<HashMap<String, Value>>>,
        method: &str,
        arg_count: u8,
    ) -> Result<Value, String> {
        match method {
            "add" => {
                let item = if arg_count > 0 {
                    self.stack.last().cloned().unwrap_or(Value::Void)
                } else {
                    return Err("ArrayList.add requires 1 argument".to_string());
                };
                let mut elements = match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                elements.push(item);
                fields.borrow_mut().insert("_elements".to_string(), Value::Array { elements });
                Ok(Value::Void)
            }
            "get" => {
                let idx_val = if arg_count > 0 {
                    self.stack.last().cloned().unwrap_or(Value::Void)
                } else {
                    return Err("ArrayList.get requires 1 argument".to_string());
                };
                let idx = match idx_val {
                    Value::Int(i) => i as usize,
                    Value::Long(i) => i as usize,
                    _ => return Err("ArrayList.get requires an integer index".to_string()),
                };
                match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => {
                        if idx < elements.len() {
                            Ok(elements[idx].clone())
                        } else {
                            Err(format!("ArrayList index out of bounds: {}", idx))
                        }
                    }
                    _ => Err("ArrayList has no elements".to_string()),
                }
            }
            "size" => {
                match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => Ok(Value::Int(elements.len() as i32)),
                    _ => Ok(Value::Int(0)),
                }
            }
            "set" => {
                if arg_count < 2 {
                    return Err("ArrayList.set requires 2 arguments (index, value)".to_string());
                }
                let value = self.pop();
                let idx_val = self.pop();
                let idx = match idx_val {
                    Value::Int(i) => i as usize,
                    Value::Long(i) => i as usize,
                    _ => return Err("ArrayList.set requires an integer index".to_string()),
                };
                match fields.borrow_mut().get_mut("_elements") {
                    Some(Value::Array { elements }) => {
                        if idx < elements.len() {
                            elements[idx] = value;
                            Ok(Value::Void)
                        } else {
                            Err(format!("ArrayList index out of bounds: {}", idx))
                        }
                    }
                    _ => Err("ArrayList has no elements".to_string()),
                }
            }
            "remove" => {
                if arg_count < 1 {
                    return Err("ArrayList.remove requires 1 argument (index)".to_string());
                }
                let idx_val = self.pop();
                let idx = match idx_val {
                    Value::Int(i) => i as usize,
                    Value::Long(i) => i as usize,
                    _ => return Err("ArrayList.remove requires an integer index".to_string()),
                };
                match fields.borrow_mut().get_mut("_elements") {
                    Some(Value::Array { elements }) => {
                        if idx < elements.len() {
                            Ok(elements.remove(idx))
                        } else {
                            Err(format!("ArrayList index out of bounds: {}", idx))
                        }
                    }
                    _ => Err("ArrayList has no elements".to_string()),
                }
            }
            "sort" => Ok(Value::Void),
            _ => Err(format!("Unknown ArrayList method '{}'", method)),
        }
    }

    // -----------------------------------------------------------------------
    // Built-in HashMap methods
    // -----------------------------------------------------------------------

    fn call_hashmap_method(
        &mut self,
        fields: &std::rc::Rc<std::cell::RefCell<HashMap<String, Value>>>,
        method: &str,
        arg_count: u8,
    ) -> Result<Value, String> {
        match method {
            "put" => {
                if arg_count < 2 {
                    return Err("HashMap.put requires 2 arguments".to_string());
                }
                let stack_len = self.stack.len();
                let key = self.stack[stack_len - 2].clone();
                let value = self.stack[stack_len - 1].clone();
                let mut keys = match fields.borrow().get("_keys") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let mut values = match fields.borrow().get("_values") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let mut found = false;
                for (i, k) in keys.iter().enumerate() {
                    if *k == key {
                        values[i] = value.clone();
                        found = true;
                        break;
                    }
                }
                if !found {
                    keys.push(key);
                    values.push(value);
                }
                fields.borrow_mut().insert("_keys".to_string(), Value::Array { elements: keys });
                fields.borrow_mut().insert("_values".to_string(), Value::Array { elements: values });
                Ok(Value::Void)
            }
            "get" => {
                if arg_count < 1 {
                    return Err("HashMap.get requires 1 argument".to_string());
                }
                let key = self.stack.last().cloned().unwrap_or(Value::Void);
                let keys = match fields.borrow().get("_keys") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => return Ok(Value::Null),
                };
                let values = match fields.borrow().get("_values") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => return Ok(Value::Null),
                };
                for (i, k) in keys.iter().enumerate() {
                    if *k == key {
                        return Ok(values.get(i).cloned().unwrap_or(Value::Null));
                    }
                }
                Ok(Value::Null)
            }
            _ => Err(format!("Unknown HashMap method '{}'", method)),
        }
    }

    // -----------------------------------------------------------------------
    // NEW (object instantiation)
    // -----------------------------------------------------------------------

    fn exec_new(&mut self, class_idx: u16) -> Result<(), String> {
        let ci = class_idx as usize;
        if ci >= self.classes.len() {
            return Err(format!("NEW: class index {} out of range", class_idx));
        }

        // Clone all needed data upfront to avoid borrow conflicts
        let class_name = self.classes[ci].name.clone();

        // Handle built-in pseudo-classes
        match class_name.as_str() {
            n if n.starts_with("ArrayList") => {
                let mut fields = HashMap::new();
                fields.insert("_elements".to_string(), Value::Array { elements: vec![] });
                let instance = Value::ClassInstance {
                    class_name: class_name.clone(),
                    fields: Rc::new(std::cell::RefCell::new(fields)),
                    vtable: HashMap::new(),
                };
                self.push(instance);
                return Ok(());
            }
            n if n.starts_with("HashMap") => {
                let mut fields = HashMap::new();
                fields.insert("_keys".to_string(), Value::Array { elements: vec![] });
                fields.insert("_values".to_string(), Value::Array { elements: vec![] });
                let instance = Value::ClassInstance {
                    class_name: class_name.clone(),
                    fields: Rc::new(std::cell::RefCell::new(fields)),
                    vtable: HashMap::new(),
                };
                self.push(instance);
                return Ok(());
            }
            _ => {}
        }

        let constructor = self.classes[ci].constructor;
        let field_inits: Vec<(String, Chunk)> = self.classes[ci].field_inits.clone();
        let field_names: Vec<String> = self.classes[ci].fields.iter().map(|f| f.name.clone()).collect();
        let vtable = self.classes[ci].methods.clone();

        // Build default fields (all Null initially)
        let mut fields = HashMap::new();
        for name in &field_names {
            fields.insert(name.clone(), Value::Null);
        }

        let instance = Value::ClassInstance {
            class_name,
            fields: Rc::new(std::cell::RefCell::new(fields)),
            vtable,
        };

        // Push instance onto the stack
        self.push(instance.clone());

        // Run field initializers
        // Each field_init is a (name, Chunk) pair that computes the initial value.
        // We execute each chunk and set the field.
        for (field_name, init_chunk) in field_inits {
            // Execute the init chunk by creating a temporary function/frame
            let temp_func_idx = self.functions.len() as u16;
            self.functions.push(FunctionDef {
                name: format!("<init_{}>", field_name),
                arity: 0,
                chunk: init_chunk,
                is_method: false,
                is_constructor: false,
                local_count: 0,
            });
            self.frames.push(Frame::new(temp_func_idx, self.stack.len()));
            // Run the init chunk
            while self.frames.last().map_or(false, |f| f.function_index == temp_func_idx) {
                self.step()?;
            }
            // The init chunk should have left a value on the stack
            let init_val = self.pop();
            // Set the field on the instance
            if let Value::ClassInstance { fields, .. } = &instance {
                fields.borrow_mut().insert(field_name, init_val);
            }
            // Remove the temporary function
            self.functions.pop();
        }

        // If class has a constructor, call it
        if let Some(ctor_idx) = constructor {
            let ctor_arity = self.functions[ctor_idx as usize].arity;
            // The stack is: [..., arg0, arg1, ..., instance]
            // We need:      [..., instance, arg0, arg1, ...]
            // Pop the instance, then insert it before the arguments.
            let instance_val = self.pop();
            let arg_start = self.stack.len() - ctor_arity;
            self.stack.insert(arg_start, instance_val.clone());
            // Now base points to the instance (which is "this")
            let base = arg_start;
            // Pre-allocate local slots for the constructor
            let local_count = self.functions[ctor_idx as usize].local_count;
            let needed = base + local_count;
            while self.stack.len() < needed {
                self.stack.push(Value::Null);
            }
            self.frames.push(Frame::new(ctor_idx, base));
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Static calls
    // -----------------------------------------------------------------------

    fn exec_static_call(
        &mut self,
        class_name_idx: u16,
        method_name_idx: u16,
        arg_count: u8,
    ) -> Result<(), String> {
        let (class_name, method_name) = {
            let frame = self.current_frame();
            let chunk = &self.functions[frame.function_index as usize].chunk;
            (
                chunk.strings[class_name_idx as usize].clone(),
                chunk.strings[method_name_idx as usize].clone(),
            )
        };

        match (class_name.as_str(), method_name.as_str()) {
            // io::println
            ("io", "println") => {
                let arg_start = self.stack.len() - arg_count as usize;
                let args: Vec<Value> = self.stack.drain(arg_start..).collect();
                let output = if args.is_empty() {
                    String::new()
                } else {
                    args[0].display_string()
                };
                self.output.push(output);
                self.push(Value::Void);
            }
            // Integer::toString
            ("Integer" | "int", "toString") => {
                let val = self.pop();
                let s = val.display_string();
                self.push(Value::String(Rc::new(s)));
            }
            // All numeric/wrapper type toString methods
            ("Double" | "double" | "Float" | "float" | "Long" | "long" |
             "Byte" | "byte" | "Short" | "short" | "Half" | "half" |
             "Quad" | "quad" | "Vast" | "vast" | "Uvast" | "uvast" |
             "Boolean" | "bool" | "Char" | "char" | "String_" | "string", "toString") => {
                let val = self.pop();
                let s = val.display_string();
                self.push(Value::String(Rc::new(s)));
            }
            // Integer::parseInt
            ("Integer" | "int", "parseInt") => {
                let val = self.pop();
                match &val {
                    Value::String(s) => match s.parse::<i64>() {
                        Ok(n) => self.push(Value::ResultOk(Box::new(Value::Long(n)))),
                        Err(_) => {
                            self.push(Value::ResultErr(Box::new(Value::String(Rc::new(
                                format!("Invalid integer: {}", s),
                            )))))
                        }
                    },
                    _ => {
                        return Err(format!(
                            "Integer.parseInt: expected String, got {:?}",
                            val
                        ))
                    }
                }
            }
            // Integer::parseOr - parse string to int, return default on failure
            ("Integer" | "int", "parseOr") => {
                let default_val = self.pop();
                let val = self.pop();
                let default = match &default_val {
                    Value::Int(n) => *n as i64,
                    Value::Long(n) => *n,
                    _ => 0,
                };
                match &val {
                    Value::String(s) => match s.trim().parse::<i64>() {
                        Ok(n) => self.push(Value::Long(n)),
                        Err(_) => self.push(Value::Long(default)),
                    },
                    _ => self.push(Value::Long(default)),
                }
            }
            // String::length
            ("String" | "string", "length") => {
                let val = self.pop();
                match &val {
                    Value::String(s) => self.push(Value::Int(s.len() as i32)),
                    _ => {
                        return Err(format!(
                            "String.length: expected String, got {:?}",
                            val
                        ))
                    }
                }
            }
            // String::charAt
            ("String" | "string", "charAt") => {
                let index = self.pop();
                let val = self.pop();
                match (&val, &index) {
                    (Value::String(s), Value::Int(i)) => {
                        let idx = *i as usize;
                        if idx < s.len() {
                            self.push(Value::Char(s.chars().nth(idx).unwrap()));
                        } else {
                            return Err(format!(
                                "String.charAt: index {} out of bounds",
                                idx
                            ));
                        }
                    }
                    _ => {
                        return Err(format!(
                            "String.charAt: expected (String, Int), got ({:?}, {:?})",
                            val, index
                        ))
                    }
                }
            }
            // String::substring
            ("String" | "string", "substring") => {
                let end = self.pop();
                let start = self.pop();
                let val = self.pop();
                match (&val, &start, &end) {
                    (Value::String(s), Value::Int(si), Value::Int(ei)) => {
                        let s_idx = *si as usize;
                        let e_idx = *ei as usize;
                        let substring: String = s.chars().skip(s_idx).take(e_idx - s_idx).collect();
                        self.push(Value::String(Rc::new(substring)));
                    }
                    _ => {
                        return Err(format!(
                            "String.substring: type mismatch"
                        ))
                    }
                }
            }
            // Array::new
            ("Array" | "array", "new") => {
                let size = self.pop();
                match size {
                    Value::Int(n) => {
                        let elements = vec![Value::Null; n as usize];
                        self.push(Value::Array { elements });
                    }
                    _ => return Err(format!("Array.new: expected Int size, got {:?}", size)),
                }
            }
            // File::readFile
            ("File", "readFile") => {
                let val = self.pop();
                match &val {
                    Value::String(path) => {
                        let resolved = self.resolve_path(path.as_str());
                        match std::fs::read_to_string(&resolved) {
                            Ok(content) => self.push(Value::ResultOk(Box::new(Value::String(Rc::new(content))))),
                            Err(e) => self.push(Value::ResultErr(Box::new(Value::String(Rc::new(
                                format!("Failed to read file: {}", e)
                            ))))),
                        }
                    }
                    _ => return Err(format!("File.readFile: expected String, got {:?}", val)),
                }
            }
            // File::writeFile
            ("File", "writeFile") => {
                let content = self.pop();
                let path = self.pop();
                match (&path, &content) {
                    (Value::String(p), Value::String(c)) => {
                        let resolved = self.resolve_path(p.as_str());
                        match std::fs::write(&resolved, c.as_str()) {
                            Ok(()) => self.push(Value::ResultOk(Box::new(Value::Void))),
                            Err(e) => self.push(Value::ResultErr(Box::new(Value::String(Rc::new(
                                format!("Failed to write file: {}", e)
                            ))))),
                        }
                    }
                    _ => return Err(format!("File.writeFile: expected (String, String)")),
                }
            }
            // File::readLines
            ("File", "readLines") => {
                let val = self.pop();
                match &val {
                    Value::String(path) => {
                        let resolved = self.resolve_path(path.as_str());
                        match std::fs::read_to_string(&resolved) {
                            Ok(content) => {
                                let lines: Vec<Value> = content.lines()
                                    .map(|line| Value::String(Rc::new(line.to_string())))
                                    .collect();
                                self.push(Value::Array { elements: lines });
                            }
                            Err(e) => self.push(Value::ResultErr(Box::new(Value::String(Rc::new(
                                format!("Failed to read file: {}", e)
                            ))))),
                        }
                    }
                    _ => return Err(format!("File.readLines: expected String, got {:?}", val)),
                }
            }
            // String::split
            ("String", "split") => {
                let delim = self.pop();
                let s = self.pop();
                match (&s, &delim) {
                    (Value::String(str_val), Value::String(d)) => {
                        let parts: Vec<Value> = str_val.split(d.as_str())
                            .map(|part| Value::String(Rc::new(part.to_string())))
                            .collect();
                        self.push(Value::Array { elements: parts });
                    }
                    (Value::String(str_val), Value::Char(d)) => {
                        let parts: Vec<Value> = str_val.split(*d)
                            .map(|part| Value::String(Rc::new(part.to_string())))
                            .collect();
                        self.push(Value::Array { elements: parts });
                    }
                    _ => return Err(format!("String.split: expected (String, String) or (String, Char)")),
                }
            }
            // Default: look up user-defined static method in class table
            _ => {
                // Try to find the class and its static method
                let class_def = self.classes.iter().find(|c| c.name == class_name);
                if let Some(cd) = class_def {
                    if let Some(&func_idx) = cd.methods.get(&method_name) {
                        let base = self.stack.len() - arg_count as usize;
                        self.frames.push(Frame::new(func_idx, base));
                        return Ok(());
                    }
                }
                return Err(format!(
                    "Unknown static call: {}.{}",
                    class_name, method_name
                ));
            }
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Type checking
    // -----------------------------------------------------------------------

    fn check_type_tag(&self, val: &Value, tag: TypeTag) -> bool {
        match tag {
            TypeTag::I8 => matches!(val, Value::Byte(_)),
            TypeTag::I16 => matches!(val, Value::Short(_)),
            TypeTag::I32 => matches!(val, Value::Int(_)),
            TypeTag::I64 => matches!(val, Value::Long(_)),
            TypeTag::I128 => matches!(val, Value::Vast(_)),
            TypeTag::U128 => matches!(val, Value::Uvast(_)),
            TypeTag::F32 => matches!(val, Value::Float(_)),
            TypeTag::F64 => matches!(val, Value::Double(_)),
            TypeTag::Bool => matches!(val, Value::Bool(_)),
            TypeTag::Char => matches!(val, Value::Char(_)),
            TypeTag::String => matches!(val, Value::String(_)),
            TypeTag::Null => matches!(val, Value::Null),
            TypeTag::Void => matches!(val, Value::Void),
            TypeTag::Class => matches!(val, Value::ClassInstance { .. }),
            TypeTag::Enum => matches!(val, Value::EnumInstance { .. }),
            TypeTag::Array => matches!(val, Value::Array { .. }),
            TypeTag::Ref => matches!(val, Value::Ref(_)),
            TypeTag::Owned => matches!(val, Value::Owned(_)),
            TypeTag::Result => {
                matches!(val, Value::ResultOk(_) | Value::ResultErr(_))
            }
            TypeTag::Function => matches!(val, Value::Function(_)),
        }
    }

    // -----------------------------------------------------------------------
    // Cast – mirrors the old tree-walking interpreter exactly
    // -----------------------------------------------------------------------

    fn eval_cast(&self, val: &Value, target: CastTarget) -> Result<Value, String> {
        match target {
            CastTarget::Byte => {
                let v = val.to_i64().ok_or_else(|| format!("Cannot cast {:?} to byte", val))?;
                Ok(Value::Byte(v as i8))
            }
            CastTarget::Short => {
                let v = val.to_i64().ok_or_else(|| format!("Cannot cast {:?} to short", val))?;
                Ok(Value::Short(v as i16))
            }
            CastTarget::Int => {
                let v = val.to_i64().ok_or_else(|| format!("Cannot cast {:?} to int", val))?;
                Ok(Value::Int(v as i32))
            }
            CastTarget::Long => {
                let v = val.to_i64().ok_or_else(|| format!("Cannot cast {:?} to long", val))?;
                Ok(Value::Long(v))
            }
            CastTarget::Vast => {
                let v = val.to_i64().ok_or_else(|| format!("Cannot cast {:?} to vast", val))?;
                Ok(Value::Vast(v as i128))
            }
            CastTarget::Uvast => {
                let v = val.to_u128().ok_or_else(|| format!("Cannot cast {:?} to uvast", val))?;
                Ok(Value::Uvast(v))
            }
            CastTarget::Float => {
                let v = val.to_f64().ok_or_else(|| format!("Cannot cast {:?} to float", val))?;
                Ok(Value::Float(v as f32))
            }
            CastTarget::Double => {
                let v = val.to_f64().ok_or_else(|| format!("Cannot cast {:?} to double", val))?;
                Ok(Value::Double(v))
            }
            CastTarget::Half => {
                let v = val.to_f64().ok_or_else(|| format!("Cannot cast {:?} to half", val))?;
                Ok(Value::Half(v as f32))
            }
            CastTarget::Quad => {
                let v = val.to_f64().ok_or_else(|| format!("Cannot cast {:?} to quad", val))?;
                Ok(Value::Quad(v))
            }
            CastTarget::Char => {
                let v = val.to_i64().ok_or_else(|| format!("Cannot cast {:?} to char", val))?;
                Ok(Value::Char(v as u8 as char))
            }
            CastTarget::String => {
                Ok(Value::String(Rc::new(val.display_string())))
            }
            CastTarget::Bool => {
                Ok(Value::Bool(val.is_truthy()))
            }
        }
    }
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// CastTarget conversion from u8
// ---------------------------------------------------------------------------

impl TryFrom<u8> for CastTarget {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(CastTarget::Byte),
            1 => Ok(CastTarget::Short),
            2 => Ok(CastTarget::Int),
            3 => Ok(CastTarget::Long),
            4 => Ok(CastTarget::Vast),
            5 => Ok(CastTarget::Uvast),
            6 => Ok(CastTarget::Float),
            7 => Ok(CastTarget::Double),
            8 => Ok(CastTarget::Half),
            9 => Ok(CastTarget::Quad),
            10 => Ok(CastTarget::Char),
            11 => Ok(CastTarget::String),
            12 => Ok(CastTarget::Bool),
            _ => Err(value),
        }
    }
}

// ---------------------------------------------------------------------------
// TypeTag conversion from u8
// ---------------------------------------------------------------------------

impl TryFrom<u8> for TypeTag {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(TypeTag::I8),
            1 => Ok(TypeTag::I16),
            2 => Ok(TypeTag::I32),
            3 => Ok(TypeTag::I64),
            4 => Ok(TypeTag::I128),
            5 => Ok(TypeTag::U128),
            6 => Ok(TypeTag::F32),
            7 => Ok(TypeTag::F64),
            8 => Ok(TypeTag::Bool),
            9 => Ok(TypeTag::Char),
            10 => Ok(TypeTag::String),
            11 => Ok(TypeTag::Null),
            12 => Ok(TypeTag::Void),
            13 => Ok(TypeTag::Class),
            14 => Ok(TypeTag::Enum),
            15 => Ok(TypeTag::Array),
            16 => Ok(TypeTag::Ref),
            17 => Ok(TypeTag::Owned),
            18 => Ok(TypeTag::Result),
            19 => Ok(TypeTag::Function),
            _ => Err(value),
        }
    }
}

// ---------------------------------------------------------------------------
// Built-in native functions
// ---------------------------------------------------------------------------

fn native_println(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        // This shouldn't happen in normal use, but handle gracefully
        return Ok(Value::Void);
    }
    // Note: the actual output capture is done by the VM's output field.
    // For the native function, we just return Void. The VM's CALL_NATIVE
    // handler for println should capture output. However, since native
    // functions are called generically, we need a different approach.
    // The println native will be handled specially in call_native_fn.
    Ok(Value::Void)
}

fn native_to_string(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("toString: expected 1 argument".to_string());
    }
    Ok(Value::String(Rc::new(args[0].display_string())))
}

fn native_parse_int(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("parseInt: expected 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => match s.parse::<i64>() {
            Ok(n) => Ok(Value::ResultOk(Box::new(Value::Long(n)))),
            Err(_) => Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
                format!("Invalid integer: {}", s),
            ))))),
        },
        _ => Err(format!("parseInt: expected String, got {:?}", args[0])),
    }
}

fn native_ok(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Ok: expected 1 argument".to_string());
    }
    Ok(Value::ResultOk(Box::new(args[0].clone())))
}

fn native_err(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Err: expected 1 argument".to_string());
    }
    Ok(Value::ResultErr(Box::new(args[0].clone())))
}

fn native_file_read(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("File_readFile: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => {
            match std::fs::read_to_string(path.as_str()) {
                Ok(content) => Ok(Value::ResultOk(Box::new(Value::String(Rc::new(content))))),
                Err(e) => Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
                    format!("Failed to read file '{}': {}", path, e)
                ))))),
            }
        }
        _ => Err("File_readFile: expected String path".to_string()),
    }
}

fn native_file_write(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("File_writeFile: expected 2 arguments (path, content)".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::String(path), Value::String(content)) => {
            match std::fs::write(path.as_str(), content.as_str()) {
                Ok(()) => Ok(Value::ResultOk(Box::new(Value::Void))),
                Err(e) => Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
                    format!("Failed to write file '{}': {}", path, e)
                ))))),
            }
        }
        _ => Err("File_writeFile: expected (String, String)".to_string()),
    }
}

fn native_file_read_lines(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("File_readLines: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => {
            match std::fs::read_to_string(path.as_str()) {
                Ok(content) => {
                    let lines: Vec<Value> = content.lines()
                        .map(|line| Value::String(Rc::new(line.to_string())))
                        .collect();
                    Ok(Value::Array { elements: lines })
                }
                Err(e) => Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
                    format!("Failed to read file '{}': {}", path, e)
                ))))),
            }
        }
        _ => Err("File_readLines: expected String path".to_string()),
    }
}

fn native_string_split(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("String_split: expected 2 arguments (string, delimiter)".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::String(s), Value::String(delim)) => {
            let parts: Vec<Value> = s.split(delim.as_str())
                .map(|part| Value::String(Rc::new(part.to_string())))
                .collect();
            Ok(Value::Array { elements: parts })
        }
        (Value::String(s), Value::Char(delim)) => {
            let parts: Vec<Value> = s.split(*delim)
                .map(|part| Value::String(Rc::new(part.to_string())))
                .collect();
            Ok(Value::Array { elements: parts })
        }
        _ => Err("String_split: expected (String, String) or (String, Char)".to_string()),
    }
}

/// Look up a built-in native function by name. Returns `None` for unknown names.
fn lookup_builtin_native(name: &str) -> Option<NativeFn> {
    match name {
        "println" => Some(native_println),
        "toString" => Some(native_to_string),
        "parseInt" => Some(native_parse_int),
        "Ok" => Some(native_ok),
        "Err" => Some(native_err),
        "File_readFile" => Some(native_file_read),
        "File_writeFile" => Some(native_file_write),
        "File_readLines" => Some(native_file_read_lines),
        "String_split" => Some(native_string_split),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Override call_native_fn to handle println specially (output capture)
// ---------------------------------------------------------------------------

// We need to override the generic call_native_fn to capture println output.
// The approach: in call_native_fn, check if the native is println and handle
// output capture. We already defined call_native_fn above, so let's refactor
// it to handle println specially.
//
// Actually, looking at the code above, call_native_fn calls the function
// generically. For println, we need to capture output. Let's modify the
// approach: the native_println function returns a special marker, and
// call_native_fn checks for it. Or better: handle println as a special case
// in call_native_fn.
//
// The cleanest approach is to re-implement call_native_fn to check the
// native index for println. But we already defined it above. Let's use a
// different approach: make native_println capture output through the VM.
// Since NativeFn is a function pointer, it can't access VM state.
//
// Solution: In call_native_fn, check if native_idx == 0 (println is
// registered first) and handle output capture directly.

// We'll override the call_native_fn method. Since Rust doesn't allow
// method overriding in the same impl block, we need to restructure.
// The method is already defined above, so we need to modify it.
// Let's use a different approach: modify the existing call_native_fn
// to handle println specially by checking the native index.

// Actually, the simplest fix is to have the println native function
// just return Void, and have the VM check for println in call_native_fn
// before calling the generic native. But since we already defined
// call_native_fn above, we can't redefine it.
//
// The best approach: modify the existing call_native_fn to check for
// the println native (index 0) and handle output capture.
// But we can't edit what we haven't written yet - we're writing the file.
// Let me just write the file correctly from the start.

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Helper: create a minimal VM with a single function (main) containing
    /// the given bytecode and an empty string table.
    fn vm_with_chunk(chunk: Chunk) -> Vm {
        let mut vm = Vm::new();
        vm.add_function(FunctionDef {
            name: "main".to_string(),
            arity: 0,
            chunk,
            is_method: false,
            is_constructor: false,
            local_count: 0,
        });
        vm
    }

    // -- 1. test_vm_push_pop ---------------------------------------------------

    #[test]
    fn test_vm_push_pop() {
        let mut chunk = Chunk::new();
        // PUSH_I32 42
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&42i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1, 1, 1, 1]);
        // PUSH_NULL
        chunk.write_opcode(OpCode::PUSH_NULL, 1);
        // POP
        chunk.write_opcode(OpCode::POP, 1);
        // PUSH_BOOL 1
        chunk.write_opcode(OpCode::PUSH_BOOL, 1);
        chunk.write_u8(1, 1);
        // RET
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Bool(true)));
    }

    // -- 2. test_vm_arithmetic -------------------------------------------------

    #[test]
    fn test_vm_arithmetic() {
        let mut chunk = Chunk::new();

        // PUSH_I32 10
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&10i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1, 1, 1, 1]);
        // PUSH_I32 3
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&3i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1, 1, 1, 1]);
        // ADD_I32
        chunk.write_opcode(OpCode::ADD_I32, 1);
        // RET
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Int(13)));

        // Test SUB_I64
        let mut chunk = Chunk::new();
        // PUSH_I64 100
        chunk.write_opcode(OpCode::PUSH_I64, 1);
        chunk.code.extend_from_slice(&100i64.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 8]);
        // PUSH_I64 40
        chunk.write_opcode(OpCode::PUSH_I64, 1);
        chunk.code.extend_from_slice(&40i64.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 8]);
        // SUB_I64
        chunk.write_opcode(OpCode::SUB_I64, 1);
        // RET
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Long(60)));

        // Test MUL_I32
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&7i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&6i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::MUL_I32, 1);
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Int(42)));

        // Test DIV_I32
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&20i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&4i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::DIV_I32, 1);
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Int(5)));

        // Test division by zero
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&10i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&0i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::DIV_I32, 1);
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        assert!(vm.run().is_err());
    }

    // -- 3. test_vm_comparison -------------------------------------------------

    #[test]
    fn test_vm_comparison() {
        // EQ_I32: 5 == 5 → true
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&5i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&5i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::EQ_I32, 1);
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Bool(true)));

        // LT_I32: 3 < 5 → true
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&3i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&5i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::LT_I32, 1);
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Bool(true)));

        // GT_I64: 100 > 50 → true
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::PUSH_I64, 1);
        chunk.code.extend_from_slice(&100i64.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 8]);
        chunk.write_opcode(OpCode::PUSH_I64, 1);
        chunk.code.extend_from_slice(&50i64.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 8]);
        chunk.write_opcode(OpCode::GT_I64, 1);
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Bool(true)));
    }

    // -- 4. test_vm_jumps ------------------------------------------------------

    #[test]
    fn test_vm_jumps() {
        // JMP: unconditional jump over a PUSH_NULL + POP
        let mut chunk = Chunk::new();
        // PUSH_I32 1
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&1i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        // JMP +2 (skip PUSH_NULL + POP, land on RET)
        // After reading JMP operand, IP points to byte after the i16 offset.
        // PUSH_NULL is 1 byte, POP is 1 byte, so offset=2 skips both.
        chunk.write_opcode(OpCode::JMP, 1);
        chunk.write_i16(2, 1);
        // PUSH_NULL (skipped)
        chunk.write_opcode(OpCode::PUSH_NULL, 1);
        // POP (skipped)
        chunk.write_opcode(OpCode::POP, 1);
        // RET
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Int(1)));

        // JMP_IF_FALSE: jump when value is false
        let mut chunk = Chunk::new();
        // PUSH_BOOL 0 (false)
        chunk.write_opcode(OpCode::PUSH_BOOL, 1);
        chunk.write_u8(0, 1);
        // JMP_IF_FALSE +2 (skip PUSH_I8 99 + its operand)
        chunk.write_opcode(OpCode::JMP_IF_FALSE, 1);
        chunk.write_i16(2, 1);
        // PUSH_I8 99 (skipped because false) — opcode + 1 byte operand = 2 bytes
        chunk.write_opcode(OpCode::PUSH_I8, 1);
        chunk.write_u8(99, 1);
        // PUSH_I32 42
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&42i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        // RET
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Int(42)));
    }

    // -- 5. test_vm_call_return ------------------------------------------------

    #[test]
    fn test_vm_call_return() {
        // Main: push 3, push 4, CALL func#1(2 args), RET
        // Func#1: LOAD_LOCAL 0, LOAD_LOCAL 1, ADD_I32, RET
        let mut main_chunk = Chunk::new();
        // PUSH_I32 3
        main_chunk.write_opcode(OpCode::PUSH_I32, 1);
        main_chunk.code.extend_from_slice(&3i32.to_be_bytes());
        main_chunk.source_lines.extend_from_slice(&[1; 4]);
        // PUSH_I32 4
        main_chunk.write_opcode(OpCode::PUSH_I32, 1);
        main_chunk.code.extend_from_slice(&4i32.to_be_bytes());
        main_chunk.source_lines.extend_from_slice(&[1; 4]);
        // CALL func_idx=1, arg_count=2
        main_chunk.write_opcode(OpCode::CALL, 1);
        main_chunk.write_u16(1, 1); // function index 1
        main_chunk.write_u8(2, 1);  // 2 args
        // RET
        main_chunk.write_opcode(OpCode::RET, 1);

        let mut add_chunk = Chunk::new();
        // LOAD_LOCAL 0 (first arg = 3)
        add_chunk.write_opcode(OpCode::LOAD_LOCAL, 1);
        add_chunk.write_u8(0, 1);
        // LOAD_LOCAL 1 (second arg = 4)
        add_chunk.write_opcode(OpCode::LOAD_LOCAL, 1);
        add_chunk.write_u8(1, 1);
        // ADD_I32
        add_chunk.write_opcode(OpCode::ADD_I32, 1);
        // RET
        add_chunk.write_opcode(OpCode::RET, 1);

        let mut vm = Vm::new();
        vm.add_function(FunctionDef {
            name: "main".to_string(),
            arity: 0,
            chunk: main_chunk,
            is_method: false,
            is_constructor: false,
            local_count: 0,
        });
        vm.add_function(FunctionDef {
            name: "add".to_string(),
            arity: 2,
            chunk: add_chunk,
            is_method: false,
            is_constructor: false,
            local_count: 2,
        });

        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Int(7)));
    }

    // -- 6. test_vm_local_variables --------------------------------------------

    #[test]
    fn test_vm_local_variables() {
        let mut chunk = Chunk::new();
        // PUSH_I32 10
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&10i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        // STORE_LOCAL 0
        chunk.write_opcode(OpCode::STORE_LOCAL, 1);
        chunk.write_u8(0, 1);
        // PUSH_I32 20
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&20i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        // STORE_LOCAL 1
        chunk.write_opcode(OpCode::STORE_LOCAL, 1);
        chunk.write_u8(1, 1);
        // LOAD_LOCAL 0
        chunk.write_opcode(OpCode::LOAD_LOCAL, 1);
        chunk.write_u8(0, 1);
        // LOAD_LOCAL 1
        chunk.write_opcode(OpCode::LOAD_LOCAL, 1);
        chunk.write_u8(1, 1);
        // ADD_I32
        chunk.write_opcode(OpCode::ADD_I32, 1);
        // RET
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Int(30)));
    }

    // -- 7. test_vm_string_concat ----------------------------------------------

    #[test]
    fn test_vm_string_concat() {
        let mut chunk = Chunk::new();
        let hello_idx = chunk.add_string("Hello, ");
        let world_idx = chunk.add_string("world!");

        // PUSH_STRING "Hello, "
        chunk.write_opcode(OpCode::PUSH_STRING, 1);
        chunk.write_u16(hello_idx, 1);
        // PUSH_STRING "world!"
        chunk.write_opcode(OpCode::PUSH_STRING, 1);
        chunk.write_u16(world_idx, 1);
        // STR_CONCAT
        chunk.write_opcode(OpCode::STR_CONCAT, 1);
        // RET
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(
            vm.stack.last(),
            Some(&Value::String(Rc::new("Hello, world!".to_string())))
        );

        // STR_CONCAT_RIGHT: "x: " ++ 42
        let mut chunk2 = Chunk::new();
        let prefix_idx = chunk2.add_string("x: ");
        chunk2.write_opcode(OpCode::PUSH_STRING, 1);
        chunk2.write_u16(prefix_idx, 1);
        chunk2.write_opcode(OpCode::PUSH_I32, 1);
        chunk2.code.extend_from_slice(&42i32.to_be_bytes());
        chunk2.source_lines.extend_from_slice(&[1; 4]);
        chunk2.write_opcode(OpCode::STR_CONCAT_RIGHT, 1);
        chunk2.write_opcode(OpCode::RET, 1);

        let mut vm2 = vm_with_chunk(chunk2);
        vm2.run().unwrap();
        assert_eq!(
            vm2.stack.last(),
            Some(&Value::String(Rc::new("x: 42".to_string())))
        );

        // STR_CONCAT_LEFT: 42 ++ " items"
        let mut chunk3 = Chunk::new();
        let suffix_idx = chunk3.add_string(" items");
        chunk3.write_opcode(OpCode::PUSH_I32, 1);
        chunk3.code.extend_from_slice(&42i32.to_be_bytes());
        chunk3.source_lines.extend_from_slice(&[1; 4]);
        chunk3.write_opcode(OpCode::PUSH_STRING, 1);
        chunk3.write_u16(suffix_idx, 1);
        chunk3.write_opcode(OpCode::STR_CONCAT_LEFT, 1);
        chunk3.write_opcode(OpCode::RET, 1);

        let mut vm3 = vm_with_chunk(chunk3);
        vm3.run().unwrap();
        assert_eq!(
            vm3.stack.last(),
            Some(&Value::String(Rc::new("42 items".to_string())))
        );
    }

    // -- 8. test_vm_class_new_and_field_access ---------------------------------

    #[test]
    fn test_vm_class_new_and_field_access() {
        let mut chunk = Chunk::new();
        let x_idx = chunk.add_string("x");

        // NEW class_idx=0 → pushes instance
        chunk.write_opcode(OpCode::NEW, 1);
        chunk.write_u16(0, 1);
        // Stack: [instance]

        // STORE_LOCAL 0 → store instance in local var 0
        chunk.write_opcode(OpCode::STORE_LOCAL, 1);
        chunk.write_u8(0, 1);
        // Stack: []

        // PUSH_I32 42 → [42]
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&42i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);

        // LOAD_LOCAL 0 → [42, instance]
        chunk.write_opcode(OpCode::LOAD_LOCAL, 1);
        chunk.write_u8(0, 1);

        // SET_FIELD "x" → pops instance (top), pops 42, sets x=42, pushes 42
        // Stack: [42]
        chunk.write_opcode(OpCode::SET_FIELD, 1);
        chunk.write_u16(x_idx, 1);

        // POP the 42 → []
        chunk.write_opcode(OpCode::POP, 1);

        // LOAD_LOCAL 0 → [instance]
        chunk.write_opcode(OpCode::LOAD_LOCAL, 1);
        chunk.write_u8(0, 1);

        // GET_FIELD "x" → pops instance, pushes field value
        // Stack: [42]
        chunk.write_opcode(OpCode::GET_FIELD, 1);
        chunk.write_u16(x_idx, 1);

        // RET
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = Vm::new();
        vm.add_function(FunctionDef {
            name: "main".to_string(),
            arity: 0,
            chunk,
            is_method: false,
            is_constructor: false,
            local_count: 1,
        });
        vm.add_class(ClassDef {
            name: "Point".to_string(),
            parent: None,
            fields: vec![super::super::frame::FieldDef {
                name: "x".to_string(),
                has_init: false,
            }],
            methods: HashMap::new(),
            constructor: None,
            field_inits: vec![],
        });

        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Int(42)));
    }

    // -- 9. test_vm_enum -------------------------------------------------------

    #[test]
    fn test_vm_enum() {
        let mut chunk = Chunk::new();
        let color_idx = chunk.add_string("Color");
        let red_idx = chunk.add_string("Red");

        // PUSH_I32 255
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&255i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        // ENUM_NEW Color::Red(1 field)
        chunk.write_opcode(OpCode::ENUM_NEW, 1);
        chunk.write_u16(color_idx, 1);
        chunk.write_u16(red_idx, 1);
        chunk.write_u8(1, 1);
        // RET
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();

        match vm.stack.last() {
            Some(Value::EnumInstance { enum_name, variant, fields }) => {
                assert_eq!(enum_name, "Color");
                assert_eq!(variant, "Red");
                assert_eq!(fields.len(), 1);
                assert_eq!(fields[0], Value::Int(255));
            }
            _ => panic!("Expected EnumInstance"),
        }
    }

    // -- 10. test_vm_result_ok_err ---------------------------------------------

    #[test]
    fn test_vm_result_ok_err() {
        // ResultOk
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&42i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::RESULT_OK, 1);
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(
            vm.stack.last(),
            Some(&Value::ResultOk(Box::new(Value::Int(42))))
        );

        // ResultErr
        let mut chunk2 = Chunk::new();
        let err_idx = chunk2.add_string("fail");
        chunk2.write_opcode(OpCode::PUSH_STRING, 1);
        chunk2.write_u16(err_idx, 1);
        chunk2.write_opcode(OpCode::RESULT_ERR, 1);
        chunk2.write_opcode(OpCode::RET, 1);

        let mut vm2 = vm_with_chunk(chunk2);
        vm2.run().unwrap();
        assert_eq!(
            vm2.stack.last(),
            Some(&Value::ResultErr(Box::new(Value::String(Rc::new(
                "fail".to_string()
            )))))
        );

        // UNWRAP_OR_PROPAGATE on Ok
        let mut chunk3 = Chunk::new();
        chunk3.write_opcode(OpCode::PUSH_I32, 1);
        chunk3.code.extend_from_slice(&99i32.to_be_bytes());
        chunk3.source_lines.extend_from_slice(&[1; 4]);
        chunk3.write_opcode(OpCode::RESULT_OK, 1);
        chunk3.write_opcode(OpCode::UNWRAP_OR_PROPAGATE, 1);
        chunk3.write_opcode(OpCode::RET, 1);

        let mut vm3 = vm_with_chunk(chunk3);
        vm3.run().unwrap();
        assert_eq!(vm3.stack.last(), Some(&Value::Int(99)));
    }

    // -- 11. test_vm_cast ------------------------------------------------------

    #[test]
    fn test_vm_cast() {
        // Cast Int(42) to Long
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&42i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::CAST, 1);
        chunk.write_u8(CastTarget::Long as u8, 1);
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Long(42)));

        // Cast Int(65) to Char
        let mut chunk2 = Chunk::new();
        chunk2.write_opcode(OpCode::PUSH_I32, 1);
        chunk2.code.extend_from_slice(&65i32.to_be_bytes());
        chunk2.source_lines.extend_from_slice(&[1; 4]);
        chunk2.write_opcode(OpCode::CAST, 1);
        chunk2.write_u8(CastTarget::Char as u8, 1);
        chunk2.write_opcode(OpCode::RET, 1);

        let mut vm2 = vm_with_chunk(chunk2);
        vm2.run().unwrap();
        assert_eq!(vm2.stack.last(), Some(&Value::Char('A')));

        // Cast Int(42) to String
        let mut chunk3 = Chunk::new();
        chunk3.write_opcode(OpCode::PUSH_I32, 1);
        chunk3.code.extend_from_slice(&42i32.to_be_bytes());
        chunk3.source_lines.extend_from_slice(&[1; 4]);
        chunk3.write_opcode(OpCode::CAST, 1);
        chunk3.write_u8(CastTarget::String as u8, 1);
        chunk3.write_opcode(OpCode::RET, 1);

        let mut vm3 = vm_with_chunk(chunk3);
        vm3.run().unwrap();
        assert_eq!(
            vm3.stack.last(),
            Some(&Value::String(Rc::new("42".to_string())))
        );

        // Cast Int(1) to Bool (truthy → true)
        let mut chunk4 = Chunk::new();
        chunk4.write_opcode(OpCode::PUSH_I32, 1);
        chunk4.code.extend_from_slice(&1i32.to_be_bytes());
        chunk4.source_lines.extend_from_slice(&[1; 4]);
        chunk4.write_opcode(OpCode::CAST, 1);
        chunk4.write_u8(CastTarget::Bool as u8, 1);
        chunk4.write_opcode(OpCode::RET, 1);

        let mut vm4 = vm_with_chunk(chunk4);
        vm4.run().unwrap();
        assert_eq!(vm4.stack.last(), Some(&Value::Bool(true)));

        // Cast Int(0) to Bool (falsy → false)
        let mut chunk5 = Chunk::new();
        chunk5.write_opcode(OpCode::PUSH_I32, 1);
        chunk5.code.extend_from_slice(&0i32.to_be_bytes());
        chunk5.source_lines.extend_from_slice(&[1; 4]);
        chunk5.write_opcode(OpCode::CAST, 1);
        chunk5.write_u8(CastTarget::Bool as u8, 1);
        chunk5.write_opcode(OpCode::RET, 1);

        let mut vm5 = vm_with_chunk(chunk5);
        vm5.run().unwrap();
        assert_eq!(vm5.stack.last(), Some(&Value::Bool(false)));
    }

    // -- 12. test_vm_native_fn -------------------------------------------------

    #[test]
    fn test_vm_native_fn() {
        // Call toString(42)
        let mut chunk = Chunk::new();
        // PUSH_I32 42
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&42i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        // CALL_NATIVE toString_idx=1, arg_count=1
        // toString is the second native registered (index 1)
        chunk.write_opcode(OpCode::CALL_NATIVE, 1);
        chunk.write_u16(1, 1); // native index 1 = toString
        chunk.write_u8(1, 1);  // 1 arg
        // RET
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(
            vm.stack.last(),
            Some(&Value::String(Rc::new("42".to_string())))
        );

        // Call Ok(42)
        let mut chunk2 = Chunk::new();
        chunk2.write_opcode(OpCode::PUSH_I32, 1);
        chunk2.code.extend_from_slice(&42i32.to_be_bytes());
        chunk2.source_lines.extend_from_slice(&[1; 4]);
        // Ok is native index 3
        chunk2.write_opcode(OpCode::CALL_NATIVE, 1);
        chunk2.write_u16(3, 1);
        chunk2.write_u8(1, 1);
        chunk2.write_opcode(OpCode::RET, 1);

        let mut vm2 = vm_with_chunk(chunk2);
        vm2.run().unwrap();
        assert_eq!(
            vm2.stack.last(),
            Some(&Value::ResultOk(Box::new(Value::Int(42))))
        );

        // Call Err("oops")
        let mut chunk3 = Chunk::new();
        let err_idx = chunk3.add_string("oops");
        chunk3.write_opcode(OpCode::PUSH_STRING, 1);
        chunk3.write_u16(err_idx, 1);
        // Err is native index 4
        chunk3.write_opcode(OpCode::CALL_NATIVE, 1);
        chunk3.write_u16(4, 1);
        chunk3.write_u8(1, 1);
        chunk3.write_opcode(OpCode::RET, 1);

        let mut vm3 = vm_with_chunk(chunk3);
        vm3.run().unwrap();
        assert_eq!(
            vm3.stack.last(),
            Some(&Value::ResultErr(Box::new(Value::String(Rc::new(
                "oops".to_string()
            )))))
        );
    }
}
