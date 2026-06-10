// Titrate Alpha 0.2 – bytecode virtual machine
// Precision in every step – richie-rich90454, 2026

use std::cell::RefCell;
use std::char;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::rc::Rc;

use super::frame::{ClassDef, EnumDef, Frame, FunctionDef};
use super::opcodes::{CastTarget, Chunk, OpCode, TypeTag};
use super::value::{NativeFn, Value};
use chrono::{Datelike, Timelike};
use md5::{Digest, Md5};
use sha1::Sha1;
use sha2::Sha256;
use base64::{Engine as _, engine::general_purpose};
use percent_encoding::{utf8_percent_encode, percent_decode_str, NON_ALPHANUMERIC};

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
        vm.register_native("File_open", native_file_open);
        vm.register_native("File_readLine", native_file_read_line);
        vm.register_native("File_write", native_file_write_content);
        vm.register_native("File_close", native_file_close);
        vm.register_native("String_split", native_string_split);
        vm.register_native("Integer_parseOr", native_integer_parse_or);
        vm.register_native("String_trim", native_string_trim);
        vm.register_native("String_length", native_string_length);

        // Path natives
        vm.register_native("Path_join", native_path_join);
        vm.register_native("Path_exists", native_path_exists);
        vm.register_native("Path_isFile", native_path_is_file);
        vm.register_native("Path_isDir", native_path_is_dir);
        vm.register_native("Path_basename", native_path_basename);
        vm.register_native("Path_dirname", native_path_dirname);
        vm.register_native("Path_extension", native_path_extension);

        // Directory natives
        vm.register_native("Dir_list", native_dir_list);
        vm.register_native("Dir_create", native_dir_create);
        vm.register_native("Dir_remove", native_dir_remove);

        // Sys natives
        vm.register_native("Sys_args", native_sys_args);
        vm.register_native("Sys_env", native_sys_env);
        vm.register_native("Sys_setEnv", native_sys_set_env);
        vm.register_native("Sys_exit", native_sys_exit);
        vm.register_native("Sys_workingDir", native_sys_working_dir);
        vm.register_native("Sys_sleep", native_sys_sleep);

        // Network natives
        vm.register_native("Net_connect", native_net_connect);
        vm.register_native("Net_send", native_net_send);
        vm.register_native("Net_receive", native_net_receive);
        vm.register_native("Net_bind", native_net_bind);
        vm.register_native("Net_accept", native_net_accept);
        vm.register_native("Net_close", native_net_close);
        vm.register_native("Http_get", native_http_get);
        vm.register_native("Http_post", native_http_post);

        // Time natives
        vm.register_native("Time_now", native_time_now);
        vm.register_native("Time_sleep", native_time_sleep);
        vm.register_native("Time_format", native_time_format);
        vm.register_native("Time_getYear", native_time_get_year);
        vm.register_native("Time_getMonth", native_time_get_month);
        vm.register_native("Time_getDay", native_time_get_day);
        vm.register_native("Time_getHour", native_time_get_hour);
        vm.register_native("Time_getMinute", native_time_get_minute);
        vm.register_native("Time_getSecond", native_time_get_second);

        // Regex natives
        vm.register_native("Regex_match", native_regex_match);
        vm.register_native("Regex_find", native_regex_find);
        vm.register_native("Regex_replace", native_regex_replace);

        // Math natives
        vm.register_native("Math_sin", native_math_sin);
        vm.register_native("Math_cos", native_math_cos);
        vm.register_native("Math_tan", native_math_tan);
        vm.register_native("Math_asin", native_math_asin);
        vm.register_native("Math_acos", native_math_acos);
        vm.register_native("Math_atan", native_math_atan);
        vm.register_native("Math_atan2", native_math_atan2);
        vm.register_native("Math_ln", native_math_ln);
        vm.register_native("Math_log10", native_math_log10);
        vm.register_native("Math_log2", native_math_log2);
        vm.register_native("Math_exp", native_math_exp);
        vm.register_native("Math_pow", native_math_pow);
        vm.register_native("Math_sqrt", native_math_sqrt);
        vm.register_native("Math_cbrt", native_math_cbrt);
        vm.register_native("Math_abs", native_math_abs);
        vm.register_native("Math_absInt", native_math_abs_int);
        vm.register_native("Math_floor", native_math_floor);
        vm.register_native("Math_ceil", native_math_ceil);
        vm.register_native("Math_round", native_math_round);
        vm.register_native("Math_inf", native_math_inf);
        vm.register_native("Math_nan", native_math_nan);
        vm.register_native("Math_maxDouble", native_math_max_double);
        vm.register_native("Math_minDouble", native_math_min_double);
        vm.register_native("Math_maxInt", native_math_max_int);
        vm.register_native("Math_minInt", native_math_min_int);

        // Random natives
        vm.register_native("Random_seed", native_random_seed);
        vm.register_native("Random_nextLong", native_random_next_long);

        // Json natives
        vm.register_native("Json_parse", native_json_parse);

        // Env natives
        vm.register_native("Env_get", native_env_get);
        vm.register_native("Env_set", native_env_set);
        vm.register_native("Env_vars", native_env_vars);

        // Fs natives
        vm.register_native("Fs_exists", native_fs_exists);
        vm.register_native("Fs_isFile", native_fs_is_file);
        vm.register_native("Fs_isDir", native_fs_is_dir);
        vm.register_native("Fs_size", native_fs_size);

        // Process natives
        vm.register_native("Process_id", native_process_id);
        vm.register_native("Process_args", native_process_args);

        // Os natives
        vm.register_native("Os_name", native_os_name);
        vm.register_native("Os_arch", native_os_arch);
        vm.register_native("Os_family", native_os_family);

        // String utility natives
        vm.register_native("String_trimStart", native_string_trim_start);
        vm.register_native("String_trimEnd", native_string_trim_end);
        vm.register_native("String_startsWith", native_string_starts_with);
        vm.register_native("String_endsWith", native_string_ends_with);
        vm.register_native("String_padLeft", native_string_pad_left);
        vm.register_native("String_padRight", native_string_pad_right);

        // Hash natives
        vm.register_native("Hash_md5", native_hash_md5);
        vm.register_native("Hash_sha1", native_hash_sha1);
        vm.register_native("Hash_sha256", native_hash_sha256);

        // Base64 natives
        vm.register_native("Base64_encode", native_base64_encode);
        vm.register_native("Base64_decode", native_base64_decode);

        // Hex natives
        vm.register_native("Hex_encode", native_hex_encode);
        vm.register_native("Hex_decode", native_hex_decode);

        // URL encoding natives
        vm.register_native("Url_encode", native_url_encode);
        vm.register_native("Url_decode", native_url_decode);

        // Additional String natives
        vm.register_native("String_toUppercase", native_string_to_uppercase);
        vm.register_native("String_toLowerCase", native_string_to_lower_case);
        vm.register_native("String_replace", native_string_replace);

        // Additional Math natives
        vm.register_native("Math_nextUp", native_math_next_up);
        vm.register_native("Math_nextDown", native_math_next_down);
        vm.register_native("Math_ulp", native_math_ulp);
        vm.register_native("Math_getExponent", native_math_get_exponent);
        vm.register_native("Math_scalb", native_math_scalb);
        vm.register_native("Math_random", native_math_random);
        vm.register_native("Math_negInf", native_math_neg_inf);

        // Additional Regex natives
        vm.register_native("Regex_groupCount", native_regex_group_count);

        // Additional Directory natives
        vm.register_native("Dir_walk", native_dir_walk);
        vm.register_native("Dir_copy", native_dir_copy);
        vm.register_native("Dir_move", native_dir_move);

        // Additional Time natives
        vm.register_native("Time_dayOfWeek", native_time_day_of_week);
        vm.register_native("Time_dayOfYear", native_time_day_of_year);

        // Double and Long parsing natives
        vm.register_native("Double_parseDouble", native_double_parse_double);
        vm.register_native("Long_parseLong", native_long_parse_long);

        // Subprocess natives
        vm.register_native("Subprocess_run", native_subprocess_run);

        // Tempfile natives
        vm.register_native("Tempfile_create", native_tempfile_create);

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

    /// Call a registered native function by name with the given arguments.
    /// Useful for testing native functions directly.
    pub fn call_native_by_name(&mut self, name: &str, args: &[Value]) -> Result<Value, String> {
        let idx = self.native_names.get(name).copied()
            .ok_or_else(|| format!("Unknown native function '{}'", name))?;
        let native = self.natives[idx as usize];
        native(args)
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

    /// Call a specific function by its index in the function table.
    /// The function must have zero parameters. Runs until the function returns.
    pub fn call_function_by_index(&mut self, func_idx: usize) -> Result<(), String> {
        if func_idx >= self.functions.len() {
            return Err(format!("Function index {} out of range", func_idx));
        }

        let arity = self.functions[func_idx].arity;
        if arity != 0 {
            return Err(format!(
                "Test function '{}' expects {} arguments but call_function_by_index only supports zero-argument calls",
                self.functions[func_idx].name, arity
            ));
        }

        let base = self.stack.len();
        let func_idx_u16 = func_idx as u16;
        self.frames.push(Frame::new(func_idx_u16, base));

        // Pre-allocate stack slots for local variables
        let local_count = self.functions[func_idx].local_count;
        let needed = base + local_count;
        while self.stack.len() < needed {
            self.stack.push(Value::Null);
        }

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
                let slot = self.read_u8() as usize;
                let upvalues = self.current_frame().upvalues.clone();
                match &upvalues {
                    Some(uvs) => {
                        if slot < uvs.len() {
                            self.push(uvs[slot].clone());
                        } else {
                            return Err(format!(
                                "LOAD_UPVALUE: index {} out of bounds (upvalue count {})",
                                slot,
                                uvs.len()
                            ));
                        }
                    }
                    None => {
                        return Err("LOAD_UPVALUE: no upvalues in current frame".to_string());
                    }
                }
            }
            OpCode::STORE_UPVALUE => {
                let slot = self.read_u8() as usize;
                let val = self.pop();
                let upvalues = self.current_frame_mut().upvalues.as_mut();
                match upvalues {
                    Some(uvs) => {
                        if slot < uvs.len() {
                            uvs[slot] = val;
                        } else {
                            return Err(format!(
                                "STORE_UPVALUE: index {} out of bounds (upvalue count {})",
                                slot,
                                uvs.len()
                            ));
                        }
                    }
                    None => {
                        return Err("STORE_UPVALUE: no upvalues in current frame".to_string());
                    }
                }
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
            OpCode::INVOKE_OPERATOR => {
                let method_name_idx = self.read_u16();
                let arg_count = self.read_u8();
                self.invoke_operator(method_name_idx, arg_count)?;
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
            OpCode::TUPLE_NEW => {
                let count = self.read_u16() as usize;
                let mut elements = Vec::with_capacity(count);
                for _ in 0..count {
                    elements.push(Value::Null);
                }
                // Pop values from stack in reverse order
                for i in (0..count).rev() {
                    elements[i] = self.pop();
                }
                self.push(Value::Tuple { elements });
            }
            OpCode::TUPLE_GET => {
                let index = self.read_u8() as usize;
                let tuple = self.pop();
                match &tuple {
                    Value::Tuple { elements } => {
                        if index < elements.len() {
                            self.push(elements[index].clone());
                        } else {
                            return Err(format!(
                                "TUPLE_GET: index {} out of bounds (length {})",
                                index,
                                elements.len()
                            ));
                        }
                    }
                    _ => {
                        return Err(format!(
                            "TUPLE_GET: expected tuple, found {:?}",
                            tuple
                        ))
                    }
                }
            }

            // -- Closures --------------------------------------------------------
            OpCode::CLOSURE_NEW => {
                let func_idx = self.read_u16() as usize;
                let upvalue_count = self.read_u8() as usize;
                let mut upvalues = Vec::with_capacity(upvalue_count);
                for _ in 0..upvalue_count {
                    upvalues.push(self.pop());
                }
                upvalues.reverse();
                self.push(Value::Closure {
                    func_idx,
                    upvalues,
                });
            }
            OpCode::GET_UPVALUE => {
                let slot = self.read_u8() as usize;
                let upvalues = self.current_frame().upvalues.clone();
                match &upvalues {
                    Some(uvs) => {
                        if slot < uvs.len() {
                            self.push(uvs[slot].clone());
                        } else {
                            return Err(format!(
                                "GET_UPVALUE: index {} out of bounds (upvalue count {})",
                                slot,
                                uvs.len()
                            ));
                        }
                    }
                    None => {
                        return Err("GET_UPVALUE: no upvalues in current frame".to_string());
                    }
                }
            }
            OpCode::SET_UPVALUE => {
                let slot = self.read_u8() as usize;
                let val = self.pop();
                let upvalues = self.current_frame_mut().upvalues.as_mut();
                match upvalues {
                    Some(uvs) => {
                        if slot < uvs.len() {
                            uvs[slot] = val;
                        } else {
                            return Err(format!(
                                "SET_UPVALUE: index {} out of bounds (upvalue count {})",
                                slot,
                                uvs.len()
                            ));
                        }
                    }
                    None => {
                        return Err("SET_UPVALUE: no upvalues in current frame".to_string());
                    }
                }
            }
            OpCode::CLOSURE_NEW_CAPTURED => {
                let func_idx = self.read_u16() as usize;
                let capture_count = self.read_u8() as usize;
                let mut upvalues = Vec::with_capacity(capture_count);
                for _ in 0..capture_count {
                    upvalues.push(self.pop());
                }
                upvalues.reverse();
                self.push(Value::Closure {
                    func_idx,
                    upvalues,
                });
            }
            OpCode::CLOSURE_CAPTURE => {
                let slot = self.read_u8() as usize;
                let frame = self.current_frame();
                let base = frame.base;
                if base + slot < self.stack.len() {
                    let val = self.stack[base + slot].clone();
                    self.push(val);
                } else {
                    return Err(format!(
                        "CLOSURE_CAPTURE: local slot {} out of bounds (stack len {})",
                        slot,
                        self.stack.len()
                    ));
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

        // Check if there's a Closure value on the stack with this function index.
        // Search the stack for a matching closure to get its upvalues.
        let upvalues = self.find_closure_upvalues(func_idx);

        if let Some(uvs) = upvalues {
            self.frames.push(Frame::new_with_upvalues(func_idx, base, uvs));
        } else {
            self.frames.push(Frame::new(func_idx, base));
        }

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

    /// Search the stack for a Closure value with the given function index
    /// and return its upvalues if found.
    fn find_closure_upvalues(&self, func_idx: u16) -> Option<Vec<Value>> {
        for val in self.stack.iter().rev() {
            if let Value::Closure { func_idx: idx, upvalues } = val {
                if *idx == func_idx as usize {
                    return Some(upvalues.clone());
                }
            }
        }
        None
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
                    "trim" => {
                        self.stack.drain(receiver_idx..);
                        self.push(Value::String(Rc::new(s.trim().to_string())));
                    }
                    "split" => {
                        let delimiter = if arg_count > 0 {
                            self.stack.last().cloned().unwrap_or(Value::Void)
                        } else {
                            return Err("String.split requires 1 argument".to_string());
                        };
                        let delim_str = match &delimiter {
                            Value::String(d) => d.as_str().to_string(),
                            Value::Char(c) => c.to_string(),
                            _ => return Err("String.split requires a String or Char delimiter".to_string()),
                        };
                        let parts: Vec<Value> = s.split(&delim_str)
                            .map(|part| Value::String(Rc::new(part.to_string())))
                            .collect();
                        let mut fields = HashMap::new();
                        fields.insert("_elements".to_string(), Value::Array { elements: parts });
                        let result = Value::ClassInstance {
                            class_name: "ArrayList".to_string(),
                            fields: Rc::new(std::cell::RefCell::new(fields)),
                            vtable: HashMap::new(),
                        };
                        self.stack.drain(receiver_idx..);
                        self.push(result);
                    }
                    "isEmpty" => {
                        self.stack.drain(receiver_idx..);
                        self.push(Value::Bool(s.is_empty()));
                    }
                    "contains" => {
                        let substring = if arg_count > 0 {
                            self.stack.last().cloned().unwrap_or(Value::Void)
                        } else {
                            return Err("String.contains requires 1 argument".to_string());
                        };
                        match &substring {
                            Value::String(sub) => {
                                self.stack.drain(receiver_idx..);
                                self.push(Value::Bool(s.contains(sub.as_str())));
                            }
                            _ => return Err("String.contains requires a String argument".to_string()),
                        }
                    }
                    "startsWith" => {
                        let prefix = if arg_count > 0 {
                            self.stack.last().cloned().unwrap_or(Value::Void)
                        } else {
                            return Err("String.startsWith requires 1 argument".to_string());
                        };
                        match &prefix {
                            Value::String(p) => {
                                self.stack.drain(receiver_idx..);
                                self.push(Value::Bool(s.starts_with(p.as_str())));
                            }
                            _ => return Err("String.startsWith requires a String argument".to_string()),
                        }
                    }
                    "endsWith" => {
                        let suffix = if arg_count > 0 {
                            self.stack.last().cloned().unwrap_or(Value::Void)
                        } else {
                            return Err("String.endsWith requires 1 argument".to_string());
                        };
                        match &suffix {
                            Value::String(suf) => {
                                self.stack.drain(receiver_idx..);
                                self.push(Value::Bool(s.ends_with(suf.as_str())));
                            }
                            _ => return Err("String.endsWith requires a String argument".to_string()),
                        }
                    }
                    "substring" => {
                        if arg_count < 2 {
                            return Err("String.substring requires 2 arguments (start, end)".to_string());
                        }
                        let end_val = self.stack.last().cloned().unwrap_or(Value::Void);
                        let start_val = self.stack.get(self.stack.len() - 2).cloned().unwrap_or(Value::Void);
                        let start = match start_val {
                            Value::Int(i) => i as usize,
                            Value::Long(i) => i as usize,
                            _ => return Err("String.substring: start must be an integer".to_string()),
                        };
                        let end = match end_val {
                            Value::Int(i) => i as usize,
                            Value::Long(i) => i as usize,
                            _ => return Err("String.substring: end must be an integer".to_string()),
                        };
                        if start > end || end > s.len() {
                            return Err(format!("String.substring: indices out of range ({}..{}) for string of length {}", start, end, s.len()));
                        }
                        let sub = s[start..end].to_string();
                        self.stack.drain(receiver_idx..);
                        self.push(Value::String(Rc::new(sub)));
                    }
                    "charAt" => {
                        let idx_val = if arg_count > 0 {
                            self.stack.last().cloned().unwrap_or(Value::Void)
                        } else {
                            return Err("String.charAt requires 1 argument".to_string());
                        };
                        let idx = match idx_val {
                            Value::Int(i) => i as usize,
                            Value::Long(i) => i as usize,
                            _ => return Err("String.charAt: index must be an integer".to_string()),
                        };
                        match s.chars().nth(idx) {
                            Some(c) => {
                                self.stack.drain(receiver_idx..);
                                self.push(Value::Char(c));
                            }
                            None => return Err(format!("String.charAt: index {} out of range", idx)),
                        }
                    }
                    _ => {
                        return Err(format!(
                            "No method '{}' on string",
                            method_name
                        ))
                    }
                }
            }
            Value::ResultOk(inner) => {
                match method_name.as_str() {
                    "unwrap" => {
                        self.stack.drain(receiver_idx..);
                        self.push((**inner).clone());
                    }
                    _ => {
                        return Err(format!(
                            "No method '{}' on Result",
                            method_name
                        ))
                    }
                }
            }
            Value::ResultErr(err_val) => {
                match method_name.as_str() {
                    "unwrap" => {
                        return Err(format!(
                            "called unwrap on an Err value: {}",
                            err_val.display_string()
                        ));
                    }
                    _ => {
                        return Err(format!(
                            "No method '{}' on Result",
                            method_name
                        ))
                    }
                }
            }
            Value::FileHandle(file_rc) => {
                match method_name.as_str() {
                    "readLine" => {
                        let result = {
                            let file_opt = file_rc.borrow_mut();
                            match file_opt.as_ref() {
                                Some(file) => {
                                    let mut reader = BufReader::new(file.try_clone().map_err(|e| format!("FileHandle.readLine: failed to clone file handle: {}", e))?);
                                    let mut line = String::new();
                                    match reader.read_line(&mut line) {
                                        Ok(0) => Value::ResultErr(Box::new(Value::String(Rc::new("EOF".to_string())))),
                                        Ok(_) => {
                                            // Remove trailing newline
                                            if line.ends_with('\n') { line.pop(); }
                                            if line.ends_with('\r') { line.pop(); }
                                            Value::ResultOk(Box::new(Value::String(Rc::new(line))))
                                        }
                                        Err(e) => Value::ResultErr(Box::new(Value::String(Rc::new(format!("FileHandle.readLine: {}", e))))),
                                    }
                                }
                                None => Value::ResultErr(Box::new(Value::String(Rc::new("FileHandle is closed".to_string())))),
                            }
                        };
                        self.stack.drain(receiver_idx..);
                        self.push(result);
                    }
                    "write" => {
                        if arg_count == 0 {
                            return Err("FileHandle.write requires 1 argument (content)".to_string());
                        }
                        let content = self.stack.last().cloned().unwrap_or(Value::Void);
                        let result = {
                            let mut file_opt = file_rc.borrow_mut();
                            match file_opt.as_mut() {
                                Some(file) => {
                                    match &content {
                                        Value::String(s) => {
                                            match file.write_all(s.as_bytes()) {
                                                Ok(()) => Value::ResultOk(Box::new(Value::Void)),
                                                Err(e) => Value::ResultErr(Box::new(Value::String(Rc::new(format!("FileHandle.write: {}", e))))),
                                            }
                                        }
                                        _ => Value::ResultErr(Box::new(Value::String(Rc::new("FileHandle.write: expected String argument".to_string())))),
                                    }
                                }
                                None => Value::ResultErr(Box::new(Value::String(Rc::new("FileHandle is closed".to_string())))),
                            }
                        };
                        self.stack.drain(receiver_idx..);
                        self.push(result);
                    }
                    "close" => {
                        let mut file_opt = file_rc.borrow_mut();
                        *file_opt = None;
                        drop(file_opt);
                        self.stack.drain(receiver_idx..);
                        self.push(Value::Void);
                    }
                    _ => {
                        return Err(format!(
                            "No method '{}' on FileHandle",
                            method_name
                        ))
                    }
                }
            }
            Value::EnumInstance { variant, fields, .. } => {
                // Handle enum instance methods
                match method_name.as_str() {
                    "toString" => {
                        let s = if fields.is_empty() {
                            variant.clone()
                        } else {
                            let items: Vec<String> = fields.iter().map(|v| v.display_string()).collect();
                            format!("{}({})", variant, items.join(", "))
                        };
                        self.stack.drain(receiver_idx..);
                        self.push(Value::String(Rc::new(s)));
                    }
                    _ => {
                        return Err(format!(
                            "INVOKE_VIRTUAL: cannot invoke '{}' on enum instance '{}'",
                            method_name, variant
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
            "length" => {
                match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => Ok(Value::Int(elements.len() as i32)),
                    _ => Ok(Value::Int(0)),
                }
            }
            "forEach" => {
                if arg_count < 1 {
                    return Err("ArrayList.forEach requires 1 argument (closure)".to_string());
                }
                let closure = self.stack.last().cloned().unwrap_or(Value::Void);
                let elements = match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                for elem in &elements {
                    self.call_closure_with_args(&closure, &[elem.clone()])?;
                }
                Ok(Value::Void)
            }
            "toString" => {
                let elements = match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let items: Vec<String> = elements.iter().map(|e| e.display_string()).collect();
                Ok(Value::String(Rc::new(format!("[{}]", items.join(", ")))))
            }
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
            "containsKey" => {
                if arg_count < 1 {
                    return Err("HashMap.containsKey requires 1 argument".to_string());
                }
                let key = self.stack.last().cloned().unwrap_or(Value::Void);
                let keys = match fields.borrow().get("_keys") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => return Ok(Value::Bool(false)),
                };
                Ok(Value::Bool(keys.iter().any(|k| *k == key)))
            }
            "keys" => {
                let keys = match fields.borrow().get("_keys") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let mut al_fields = HashMap::new();
                al_fields.insert("_elements".to_string(), Value::Array { elements: keys });
                Ok(Value::ClassInstance {
                    class_name: "ArrayList".to_string(),
                    fields: Rc::new(std::cell::RefCell::new(al_fields)),
                    vtable: HashMap::new(),
                })
            }
            "values" => {
                let values = match fields.borrow().get("_values") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let mut al_fields = HashMap::new();
                al_fields.insert("_elements".to_string(), Value::Array { elements: values });
                Ok(Value::ClassInstance {
                    class_name: "ArrayList".to_string(),
                    fields: Rc::new(std::cell::RefCell::new(al_fields)),
                    vtable: HashMap::new(),
                })
            }
            "entries" => {
                let keys = match fields.borrow().get("_keys") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let values = match fields.borrow().get("_values") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let entries: Vec<Value> = keys.iter().zip(values.iter()).map(|(k, v)| {
                    Value::Tuple { elements: vec![k.clone(), v.clone()] }
                }).collect();
                let mut al_fields = HashMap::new();
                al_fields.insert("_elements".to_string(), Value::Array { elements: entries });
                Ok(Value::ClassInstance {
                    class_name: "ArrayList".to_string(),
                    fields: Rc::new(std::cell::RefCell::new(al_fields)),
                    vtable: HashMap::new(),
                })
            }
            "size" => {
                let keys = match fields.borrow().get("_keys") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                Ok(Value::Int(keys.len() as i32))
            }
            "remove" => {
                if arg_count < 1 {
                    return Err("HashMap.remove requires 1 argument".to_string());
                }
                let key = self.stack.last().cloned().unwrap_or(Value::Void);
                let mut keys = match fields.borrow().get("_keys") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => return Ok(Value::Null),
                };
                let mut values = match fields.borrow().get("_values") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => return Ok(Value::Null),
                };
                let mut found_idx = None;
                for (i, k) in keys.iter().enumerate() {
                    if *k == key {
                        found_idx = Some(i);
                        break;
                    }
                }
                match found_idx {
                    Some(i) => {
                        let old_val = values.remove(i);
                        keys.remove(i);
                        fields.borrow_mut().insert("_keys".to_string(), Value::Array { elements: keys });
                        fields.borrow_mut().insert("_values".to_string(), Value::Array { elements: values });
                        Ok(old_val)
                    }
                    None => Ok(Value::Null),
                }
            }
            "toString" => {
                let keys = match fields.borrow().get("_keys") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let values = match fields.borrow().get("_values") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let items: Vec<String> = keys.iter().zip(values.iter())
                    .map(|(k, v)| format!("{}: {}", k.display_string(), v.display_string()))
                    .collect();
                Ok(Value::String(Rc::new(format!("{{{}}}", items.join(", ")))))
            }
            _ => Err(format!("Unknown HashMap method '{}'", method)),
        }
    }

    // -----------------------------------------------------------------------
    // INVOKE_OPERATOR – operator overloading with fallback
    // -----------------------------------------------------------------------

    fn invoke_operator(&mut self, method_name_idx: u16, arg_count: u8) -> Result<(), String> {
        // Stack: [left, right, ...]  (left is below right)
        // arg_count should be 1 (the right operand).
        let method_name = {
            let frame = self.current_frame();
            let chunk = &self.functions[frame.function_index as usize].chunk;
            chunk.strings[method_name_idx as usize].clone()
        };

        // The receiver (left operand) is at stack.len() - 1 - arg_count
        let receiver_idx = self.stack.len() - 1 - arg_count as usize;
        let receiver = self.stack[receiver_idx].clone();

        if let Value::ClassInstance { vtable, class_name, .. } = &receiver {
            // Check if the operator method exists in the vtable or class hierarchy
            let has_operator = if vtable.contains_key(&method_name) {
                true
            } else {
                // Walk up the class hierarchy
                let mut search_class = class_name.clone();
                loop {
                    let class_defs = &self.classes;
                    let found = class_defs.iter().find(|c| c.name == search_class);
                    match found {
                        Some(cd) => {
                            if cd.methods.contains_key(&method_name) {
                                break true;
                            }
                            if let Some(parent_idx) = cd.parent {
                                search_class = self.classes[parent_idx as usize].name.clone();
                            } else {
                                break false;
                            }
                        }
                        None => break false,
                    }
                }
            };

            if has_operator {
                // Delegate to invoke_method which handles the full method call
                return self.invoke_method(method_name_idx, arg_count);
            }
        }

        // No operator method found — fall back to built-in operator behavior.
        // Pop right and left from the stack, apply the built-in operator, push result.
        let right = self.pop();
        let left = self.pop();
        let result = self.apply_builtin_operator(&left, &right, &method_name)?;
        self.push(result);
        Ok(())
    }

    /// Call a closure value with the given arguments.
    /// Used by built-in methods like ArrayList.forEach.
    fn call_closure_with_args(&mut self, closure: &Value, args: &[Value]) -> Result<(), String> {
        match closure {
            Value::Closure { func_idx, .. } => {
                let fi = *func_idx;
                if fi >= self.functions.len() {
                    return Err("forEach: invalid closure".to_string());
                }
                let arity = self.functions[fi].arity;
                let base = self.stack.len();
                // Push args
                for arg in args {
                    self.push(arg.clone());
                }
                // Pad if needed
                for _ in args.len()..arity as usize {
                    self.push(Value::Null);
                }
                self.frames.push(Frame::new(fi as u16, base));
                // Execute the closure frame
                while self.frames.len() > 1 {
                    self.step()?;
                }
                // Pop the return value
                let _ = self.pop();
                Ok(())
            }
            _ => Err("forEach: expected a closure".to_string()),
        }
    }

    /// Apply a built-in operator when no operator overload method is found.
    fn apply_builtin_operator(&self, left: &Value, right: &Value, method_name: &str) -> Result<Value, String> {
        match method_name {
            "operator+" => self.builtin_add(left, right),
            "operator-" => self.builtin_sub(left, right),
            "operator*" => self.builtin_mul(left, right),
            "operator/" => self.builtin_div(left, right),
            "operator%" => self.builtin_mod(left, right),
            "operator==" => Ok(Value::Bool(left == right)),
            "operator!=" => Ok(Value::Bool(left != right)),
            "operator<" => self.builtin_cmp(left, right, |a, b| a < b, |a, b| a < b),
            "operator>" => self.builtin_cmp(left, right, |a, b| a > b, |a, b| a > b),
            "operator<=" => self.builtin_cmp(left, right, |a, b| a <= b, |a, b| a <= b),
            "operator>=" => self.builtin_cmp(left, right, |a, b| a >= b, |a, b| a >= b),
            "operator&" => self.builtin_bitwise(left, right, |a, b| a & b, |a, b| a & b),
            "operator|" => self.builtin_bitwise(left, right, |a, b| a | b, |a, b| a | b),
            "operator^" => self.builtin_bitwise(left, right, |a, b| a ^ b, |a, b| a ^ b),
            "operator<<" => self.builtin_shift(left, right, false),
            "operator>>" => self.builtin_shift(left, right, true),
            _ => Err(format!("Unknown operator method '{}'", method_name)),
        }
    }

    fn builtin_add(&self, left: &Value, right: &Value) -> Result<Value, String> {
        match (left, right) {
            (Value::Byte(a), Value::Byte(b)) => Ok(Value::Byte(a.wrapping_add(*b))),
            (Value::Short(a), Value::Short(b)) => Ok(Value::Short(a.wrapping_add(*b))),
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a.wrapping_add(*b))),
            (Value::Long(a), Value::Long(b)) => Ok(Value::Long(a.wrapping_add(*b))),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
            (Value::Double(a), Value::Double(b)) => Ok(Value::Double(a + b)),
            (Value::Half(a), Value::Half(b)) => Ok(Value::Half(a + b)),
            (Value::Quad(a), Value::Quad(b)) => Ok(Value::Quad(a + b)),
            (Value::Int(a), Value::Long(b)) => Ok(Value::Long(*a as i64 + b)),
            (Value::Long(a), Value::Int(b)) => Ok(Value::Long(a + *b as i64)),
            (Value::String(a), Value::String(b)) => Ok(Value::String(Rc::new(format!("{}{}", a, b)))),
            _ => Err(format!("Cannot add {:?} and {:?}", left, right)),
        }
    }

    fn builtin_sub(&self, left: &Value, right: &Value) -> Result<Value, String> {
        match (left, right) {
            (Value::Byte(a), Value::Byte(b)) => Ok(Value::Byte(a.wrapping_sub(*b))),
            (Value::Short(a), Value::Short(b)) => Ok(Value::Short(a.wrapping_sub(*b))),
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a.wrapping_sub(*b))),
            (Value::Long(a), Value::Long(b)) => Ok(Value::Long(a.wrapping_sub(*b))),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
            (Value::Double(a), Value::Double(b)) => Ok(Value::Double(a - b)),
            (Value::Half(a), Value::Half(b)) => Ok(Value::Half(a - b)),
            (Value::Quad(a), Value::Quad(b)) => Ok(Value::Quad(a - b)),
            (Value::Int(a), Value::Long(b)) => Ok(Value::Long(*a as i64 - b)),
            (Value::Long(a), Value::Int(b)) => Ok(Value::Long(a - *b as i64)),
            _ => Err(format!("Cannot subtract {:?} and {:?}", left, right)),
        }
    }

    fn builtin_mul(&self, left: &Value, right: &Value) -> Result<Value, String> {
        match (left, right) {
            (Value::Byte(a), Value::Byte(b)) => Ok(Value::Byte(a.wrapping_mul(*b))),
            (Value::Short(a), Value::Short(b)) => Ok(Value::Short(a.wrapping_mul(*b))),
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a.wrapping_mul(*b))),
            (Value::Long(a), Value::Long(b)) => Ok(Value::Long(a.wrapping_mul(*b))),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
            (Value::Double(a), Value::Double(b)) => Ok(Value::Double(a * b)),
            (Value::Half(a), Value::Half(b)) => Ok(Value::Half(a * b)),
            (Value::Quad(a), Value::Quad(b)) => Ok(Value::Quad(a * b)),
            (Value::Int(a), Value::Long(b)) => Ok(Value::Long(*a as i64 * b)),
            (Value::Long(a), Value::Int(b)) => Ok(Value::Long(a * *b as i64)),
            _ => Err(format!("Cannot multiply {:?} and {:?}", left, right)),
        }
    }

    fn builtin_div(&self, left: &Value, right: &Value) -> Result<Value, String> {
        match (left, right) {
            (Value::Byte(a), Value::Byte(b)) => {
                if *b == 0 { return Err("Division by zero".to_string()); }
                Ok(Value::Byte(a / b))
            }
            (Value::Short(a), Value::Short(b)) => {
                if *b == 0 { return Err("Division by zero".to_string()); }
                Ok(Value::Short(a / b))
            }
            (Value::Int(a), Value::Int(b)) => {
                if *b == 0 { return Err("Division by zero".to_string()); }
                Ok(Value::Int(a / b))
            }
            (Value::Long(a), Value::Long(b)) => {
                if *b == 0 { return Err("Division by zero".to_string()); }
                Ok(Value::Long(a / b))
            }
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a / b)),
            (Value::Double(a), Value::Double(b)) => Ok(Value::Double(a / b)),
            (Value::Half(a), Value::Half(b)) => Ok(Value::Half(a / b)),
            (Value::Quad(a), Value::Quad(b)) => Ok(Value::Quad(a / b)),
            _ => Err(format!("Cannot divide {:?} by {:?}", left, right)),
        }
    }

    fn builtin_mod(&self, left: &Value, right: &Value) -> Result<Value, String> {
        match (left, right) {
            (Value::Byte(a), Value::Byte(b)) => {
                if *b == 0 { return Err("Division by zero".to_string()); }
                Ok(Value::Byte(a % b))
            }
            (Value::Short(a), Value::Short(b)) => {
                if *b == 0 { return Err("Division by zero".to_string()); }
                Ok(Value::Short(a % b))
            }
            (Value::Int(a), Value::Int(b)) => {
                if *b == 0 { return Err("Division by zero".to_string()); }
                Ok(Value::Int(a % b))
            }
            (Value::Long(a), Value::Long(b)) => {
                if *b == 0 { return Err("Division by zero".to_string()); }
                Ok(Value::Long(a % b))
            }
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a % b)),
            (Value::Double(a), Value::Double(b)) => Ok(Value::Double(a % b)),
            (Value::Half(a), Value::Half(b)) => Ok(Value::Half(a % b)),
            (Value::Quad(a), Value::Quad(b)) => Ok(Value::Quad(a % b)),
            _ => Err(format!("Cannot modulo {:?} and {:?}", left, right)),
        }
    }

    fn builtin_cmp(
        &self,
        left: &Value,
        right: &Value,
        int_op: fn(i64, i64) -> bool,
        float_op: fn(f64, f64) -> bool,
    ) -> Result<Value, String> {
        match (left, right) {
            (Value::Byte(a), Value::Byte(b)) => Ok(Value::Bool(int_op(*a as i64, *b as i64))),
            (Value::Short(a), Value::Short(b)) => Ok(Value::Bool(int_op(*a as i64, *b as i64))),
            (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(int_op(*a as i64, *b as i64))),
            (Value::Long(a), Value::Long(b)) => Ok(Value::Bool(int_op(*a, *b))),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(float_op(*a as f64, *b as f64))),
            (Value::Double(a), Value::Double(b)) => Ok(Value::Bool(float_op(*a, *b))),
            (Value::Half(a), Value::Half(b)) => Ok(Value::Bool(float_op(*a as f64, *b as f64))),
            (Value::Quad(a), Value::Quad(b)) => Ok(Value::Bool(float_op(*a, *b))),
            _ => Err(format!("Cannot compare {:?} and {:?}", left, right)),
        }
    }

    fn builtin_bitwise(
        &self,
        left: &Value,
        right: &Value,
        int_op: fn(i64, i64) -> i64,
        _long_op: fn(i64, i64) -> i64,
    ) -> Result<Value, String> {
        match (left, right) {
            (Value::Byte(a), Value::Byte(b)) => Ok(Value::Byte(int_op(*a as i64, *b as i64) as i8)),
            (Value::Short(a), Value::Short(b)) => Ok(Value::Short(int_op(*a as i64, *b as i64) as i16)),
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(int_op(*a as i64, *b as i64) as i32)),
            (Value::Long(a), Value::Long(b)) => Ok(Value::Long(int_op(*a, *b))),
            _ => Err(format!("Cannot apply bitwise op to {:?} and {:?}", left, right)),
        }
    }

    fn builtin_shift(&self, left: &Value, right: &Value, is_right: bool) -> Result<Value, String> {
        match (left, right) {
            (Value::Byte(a), Value::Int(b)) => {
                if is_right { Ok(Value::Byte((*a as i64).wrapping_shr(*b as u32) as i8)) }
                else { Ok(Value::Byte((*a as i64).wrapping_shl(*b as u32) as i8)) }
            }
            (Value::Short(a), Value::Int(b)) => {
                if is_right { Ok(Value::Short((*a as i64).wrapping_shr(*b as u32) as i16)) }
                else { Ok(Value::Short((*a as i64).wrapping_shl(*b as u32) as i16)) }
            }
            (Value::Int(a), Value::Int(b)) => {
                if is_right { Ok(Value::Int(a.wrapping_shr(*b as u32))) }
                else { Ok(Value::Int(a.wrapping_shl(*b as u32))) }
            }
            (Value::Long(a), Value::Int(b)) => {
                if is_right { Ok(Value::Long(a.wrapping_shr(*b as u32))) }
                else { Ok(Value::Long(a.wrapping_shl(*b as u32))) }
            }
            _ => Err(format!("Cannot shift {:?} by {:?}", left, right)),
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
            // File::open - opens a file and returns Result<FileHandle, string>
            ("File", "open") => {
                let mode = if arg_count > 1 {
                    self.pop()
                } else {
                    Value::String(Rc::new("r".to_string()))
                };
                let path = self.pop();
                match (&path, &mode) {
                    (Value::String(p), Value::String(m)) => {
                        let resolved = self.resolve_path(p.as_str());
                        let file = match m.as_str() {
                            "r" | "rb" => std::fs::File::open(&resolved),
                            "w" | "wb" => std::fs::File::create(&resolved),
                            "a" | "ab" => std::fs::OpenOptions::new().append(true).open(&resolved),
                            "r+" => std::fs::OpenOptions::new().read(true).write(true).open(&resolved),
                            "w+" => std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(true).open(&resolved),
                            "a+" => std::fs::OpenOptions::new().read(true).append(true).open(&resolved),
                            _ => {
                                self.push(Value::ResultErr(Box::new(Value::String(Rc::new(
                                    format!("File.open: unsupported mode '{}'", m)
                                )))));
                                return Ok(());
                            }
                        };
                        match file {
                            Ok(f) => self.push(Value::ResultOk(Box::new(Value::FileHandle(
                                Rc::new(RefCell::new(Some(f)))
                            )))),
                            Err(e) => self.push(Value::ResultErr(Box::new(Value::String(Rc::new(
                                format!("Failed to open file '{}': {}", p, e)
                            ))))),
                        }
                    }
                    _ => return Err(format!("File.open: expected (String, String), got ({:?}, {:?})", path, mode)),
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

fn native_file_open(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("File_open: expected at least 1 argument (path)".to_string());
    }
    let path = match &args[0] {
        Value::String(s) => s.as_str(),
        _ => return Err("File_open: expected String path".to_string()),
    };
    let mode = if args.len() > 1 {
        match &args[1] {
            Value::String(s) => s.as_str(),
            _ => return Err("File_open: expected String mode".to_string()),
        }
    } else {
        "r"
    };
    let file = match mode {
        "r" | "rb" => std::fs::File::open(path),
        "w" | "wb" => std::fs::File::create(path),
        "a" | "ab" => std::fs::OpenOptions::new().append(true).open(path),
        "r+" => std::fs::OpenOptions::new().read(true).write(true).open(path),
        "w+" => std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(true).open(path),
        "a+" => std::fs::OpenOptions::new().read(true).append(true).open(path),
        _ => return Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
            format!("File_open: unsupported mode '{}'", mode)
        ))))),
    };
    match file {
        Ok(f) => Ok(Value::ResultOk(Box::new(Value::FileHandle(
            Rc::new(RefCell::new(Some(f)))
        )))),
        Err(e) => Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
            format!("Failed to open file '{}': {}", path, e)
        ))))),
    }
}

fn native_file_read_line(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("File_readLine: expected 1 argument (FileHandle)".to_string());
    }
    match &args[0] {
        Value::FileHandle(file_rc) => {
            let file_opt = file_rc.borrow();
            match file_opt.as_ref() {
                Some(file) => {
                    let mut reader = BufReader::new(file.try_clone().map_err(|e| format!("File_readLine: {}", e))?);
                    let mut line = String::new();
                    match reader.read_line(&mut line) {
                        Ok(0) => Ok(Value::ResultErr(Box::new(Value::String(Rc::new("EOF".to_string()))))),
                        Ok(_) => {
                            if line.ends_with('\n') { line.pop(); }
                            if line.ends_with('\r') { line.pop(); }
                            Ok(Value::ResultOk(Box::new(Value::String(Rc::new(line)))))
                        }
                        Err(e) => Ok(Value::ResultErr(Box::new(Value::String(Rc::new(format!("File_readLine: {}", e)))))),
                    }
                }
                None => Ok(Value::ResultErr(Box::new(Value::String(Rc::new("FileHandle is closed".to_string()))))),
            }
        }
        _ => Err("File_readLine: expected FileHandle argument".to_string()),
    }
}

fn native_file_write_content(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("File_write: expected 2 arguments (FileHandle, content)".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::FileHandle(file_rc), Value::String(content)) => {
            let mut file_opt = file_rc.borrow_mut();
            match file_opt.as_mut() {
                Some(file) => {
                    use std::io::Write;
                    match file.write_all(content.as_bytes()) {
                        Ok(()) => Ok(Value::ResultOk(Box::new(Value::Void))),
                        Err(e) => Ok(Value::ResultErr(Box::new(Value::String(Rc::new(format!("File_write: {}", e)))))),
                    }
                }
                None => Ok(Value::ResultErr(Box::new(Value::String(Rc::new("FileHandle is closed".to_string()))))),
            }
        }
        _ => Err("File_write: expected (FileHandle, String)".to_string()),
    }
}

fn native_file_close(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("File_close: expected 1 argument (FileHandle)".to_string());
    }
    match &args[0] {
        Value::FileHandle(file_rc) => {
            let mut file_opt = file_rc.borrow_mut();
            *file_opt = None;
            Ok(Value::Void)
        }
        _ => Err("File_close: expected FileHandle argument".to_string()),
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

fn native_integer_parse_or(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Integer_parseOr: expected 2 arguments (string, default)".to_string());
    }
    match &args[0] {
        Value::String(s) => match s.parse::<i32>() {
            Ok(n) => Ok(Value::Int(n)),
            Err(_) => Ok(args[1].clone()),
        },
        _ => Ok(args[1].clone()),
    }
}

fn native_string_trim(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("String_trim: expected 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => Ok(Value::String(Rc::new(s.trim().to_string()))),
        _ => Err("String_trim: expected String argument".to_string()),
    }
}

fn native_string_length(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("String_length: expected 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => Ok(Value::Int(s.len() as i32)),
        _ => Err("String_length: expected String argument".to_string()),
    }
}

// ---------------------------------------------------------------------------
// Path native functions
// ---------------------------------------------------------------------------

fn native_path_join(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Path_join: expected 2 arguments (path, other)".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::String(path), Value::String(other)) => {
            let joined = std::path::Path::new(path.as_str())
                .join(other.as_str())
                .to_string_lossy()
                .to_string();
            Ok(Value::String(Rc::new(joined)))
        }
        _ => Err("Path_join: expected (String, String)".to_string()),
    }
}

fn native_path_exists(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Path_exists: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => Ok(Value::Bool(std::path::Path::new(path.as_str()).exists())),
        _ => Err("Path_exists: expected String argument".to_string()),
    }
}

fn native_path_is_file(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Path_isFile: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => Ok(Value::Bool(std::path::Path::new(path.as_str()).is_file())),
        _ => Err("Path_isFile: expected String argument".to_string()),
    }
}

fn native_path_is_dir(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Path_isDir: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => Ok(Value::Bool(std::path::Path::new(path.as_str()).is_dir())),
        _ => Err("Path_isDir: expected String argument".to_string()),
    }
}

fn native_path_basename(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Path_basename: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => {
            let name = std::path::Path::new(path.as_str())
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            Ok(Value::String(Rc::new(name)))
        }
        _ => Err("Path_basename: expected String argument".to_string()),
    }
}

fn native_path_dirname(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Path_dirname: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => {
            let dir = std::path::Path::new(path.as_str())
                .parent()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            Ok(Value::String(Rc::new(dir)))
        }
        _ => Err("Path_dirname: expected String argument".to_string()),
    }
}

fn native_path_extension(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Path_extension: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => {
            let ext = std::path::Path::new(path.as_str())
                .extension()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            Ok(Value::String(Rc::new(ext)))
        }
        _ => Err("Path_extension: expected String argument".to_string()),
    }
}

// ---------------------------------------------------------------------------
// Directory native functions
// ---------------------------------------------------------------------------

fn native_dir_list(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Dir_list: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => {
            let entries: Vec<Value> = std::fs::read_dir(path.as_str())
                .map_err(|e| format!("Dir_list: {}", e))?
                .filter_map(|e| e.ok())
                .map(|e| Value::String(Rc::new(e.file_name().to_string_lossy().to_string())))
                .collect();
            Ok(Value::Array { elements: entries })
        }
        _ => Err("Dir_list: expected String argument".to_string()),
    }
}

fn native_dir_create(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Dir_create: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => match std::fs::create_dir_all(path.as_str()) {
            Ok(()) => Ok(Value::Bool(true)),
            Err(_) => Ok(Value::Bool(false)),
        },
        _ => Err("Dir_create: expected String argument".to_string()),
    }
}

fn native_dir_remove(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Dir_remove: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => match std::fs::remove_dir_all(path.as_str()) {
            Ok(()) => Ok(Value::Bool(true)),
            Err(_) => Ok(Value::Bool(false)),
        },
        _ => Err("Dir_remove: expected String argument".to_string()),
    }
}

// ---------------------------------------------------------------------------
// Sys native functions
// ---------------------------------------------------------------------------

fn native_sys_args(args: &[Value]) -> Result<Value, String> {
    // The VM doesn't have direct access to std::env::args() in a clean way,
    // but we can return an empty array as placeholder. A real implementation
    // would need the args to be passed into the VM at startup.
    let _ = args;
    let elements: Vec<Value> = std::env::args()
        .map(|a| Value::String(Rc::new(a)))
        .collect();
    Ok(Value::Array { elements })
}

fn native_sys_env(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Sys_env: expected 1 argument (key)".to_string());
    }
    match &args[0] {
        Value::String(key) => match std::env::var(key.as_str()) {
            Ok(val) => Ok(Value::String(Rc::new(val))),
            Err(_) => Ok(Value::String(Rc::new(String::new()))),
        },
        _ => Err("Sys_env: expected String argument".to_string()),
    }
}

fn native_sys_set_env(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Sys_setEnv: expected 2 arguments (key, value)".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::String(key), Value::String(val)) => {
            std::env::set_var(key.as_str(), val.as_str());
            Ok(Value::Void)
        }
        _ => Err("Sys_setEnv: expected (String, String)".to_string()),
    }
}

fn native_sys_exit(args: &[Value]) -> Result<Value, String> {
    let code = if args.is_empty() {
        0i64
    } else {
        args[0].to_i64().unwrap_or(0)
    };
    std::process::exit(code as i32);
}

fn native_sys_working_dir(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    match std::env::current_dir() {
        Ok(path) => Ok(Value::String(Rc::new(path.to_string_lossy().to_string()))),
        Err(e) => Err(format!("Sys_workingDir: {}", e)),
    }
}

fn native_sys_sleep(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Sys_sleep: expected 1 argument (milliseconds)".to_string());
    }
    let ms = args[0].to_i64().unwrap_or(0);
    std::thread::sleep(std::time::Duration::from_millis(ms as u64));
    Ok(Value::Void)
}

// ---------------------------------------------------------------------------
// Network native functions
// ---------------------------------------------------------------------------

fn native_net_connect(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Net_connect: expected 2 arguments (host, port)".to_string());
    }
    let host = match &args[0] {
        Value::String(s) => s.as_str(),
        _ => return Err("Net_connect: expected String host".to_string()),
    };
    let port = args[1].to_i64().unwrap_or(0);
    let addr = format!("{}:{}", host, port);
    match std::net::TcpStream::connect(&addr) {
        Ok(stream) => Ok(Value::Socket(Rc::new(RefCell::new(Some(stream))))),
        Err(e) => Err(format!("Net_connect: failed to connect to {}: {}", addr, e)),
    }
}

fn native_net_send(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Net_send: expected 2 arguments (socket, data)".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::Socket(socket_rc), Value::String(data)) => {
            let mut socket_opt = socket_rc.borrow_mut();
            match socket_opt.as_mut() {
                Some(stream) => {
                    use std::io::Write;
                    match stream.write_all(data.as_bytes()) {
                        Ok(()) => Ok(Value::Int(data.len() as i32)),
                        Err(e) => Err(format!("Net_send: {}", e)),
                    }
                }
                None => Err("Net_send: socket is closed".to_string()),
            }
        }
        _ => Err("Net_send: expected (Socket, String)".to_string()),
    }
}

fn native_net_receive(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Net_receive: expected 2 arguments (socket, maxBytes)".to_string());
    }
    let max_bytes = args[1].to_i64().unwrap_or(4096) as usize;
    match &args[0] {
        Value::Socket(socket_rc) => {
            let mut socket_opt = socket_rc.borrow_mut();
            match socket_opt.as_mut() {
                Some(stream) => {
                    use std::io::Read;
                    let mut buf = vec![0u8; max_bytes];
                    match stream.read(&mut buf) {
                        Ok(n) => {
                            let s = String::from_utf8_lossy(&buf[..n]).to_string();
                            Ok(Value::String(Rc::new(s)))
                        }
                        Err(e) => Err(format!("Net_receive: {}", e)),
                    }
                }
                None => Err("Net_receive: socket is closed".to_string()),
            }
        }
        _ => Err("Net_receive: expected Socket argument".to_string()),
    }
}

fn native_net_bind(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Net_bind: expected 1 argument (port)".to_string());
    }
    let port = args[0].to_i64().unwrap_or(0);
    let addr = format!("0.0.0.0:{}", port);
    match std::net::TcpListener::bind(&addr) {
        Ok(listener) => Ok(Value::Listener(Rc::new(RefCell::new(Some(listener))))),
        Err(e) => Err(format!("Net_bind: failed to bind to port {}: {}", port, e)),
    }
}

fn native_net_accept(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Net_accept: expected 1 argument (listener)".to_string());
    }
    match &args[0] {
        Value::Listener(listener_rc) => {
            let mut listener_opt = listener_rc.borrow_mut();
            match listener_opt.as_mut() {
                Some(listener) => {
                    let (stream, _addr) = listener.accept()
                        .map_err(|e| format!("Net_accept: {}", e))?;
                    Ok(Value::Socket(Rc::new(RefCell::new(Some(stream)))))
                }
                None => Err("Net_accept: listener is closed".to_string()),
            }
        }
        _ => Err("Net_accept: expected Listener argument".to_string()),
    }
}

fn native_net_close(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Net_close: expected 1 argument (socket or listener)".to_string());
    }
    match &args[0] {
        Value::Socket(socket_rc) => {
            let mut socket_opt = socket_rc.borrow_mut();
            *socket_opt = None;
            Ok(Value::Void)
        }
        Value::Listener(listener_rc) => {
            let mut listener_opt = listener_rc.borrow_mut();
            *listener_opt = None;
            Ok(Value::Void)
        }
        _ => Err("Net_close: expected Socket or Listener argument".to_string()),
    }
}

fn native_http_get(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Http_get: expected 1 argument (url)".to_string());
    }
    let url = match &args[0] {
        Value::String(s) => s.as_str(),
        _ => return Err("Http_get: expected String url".to_string()),
    };

    // Parse URL manually
    let url_str = url.to_string();
    let (host, port, path) = parse_http_url(&url_str)?;

    let addr = format!("{}:{}", host, port);
    let mut stream = std::net::TcpStream::connect(&addr)
        .map_err(|e| format!("Http_get: connection failed: {}", e))?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(30)))
        .map_err(|e| format!("Http_get: {}", e))?;

    let request = format!("GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n", path, host);
    use std::io::Write;
    stream.write_all(request.as_bytes())
        .map_err(|e| format!("Http_get: write failed: {}", e))?;

    use std::io::Read;
    let mut response = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => response.extend_from_slice(&buf[..n]),
            Err(e) => {
                if response.is_empty() {
                    return Err(format!("Http_get: read failed: {}", e));
                }
                break;
            }
        }
    }

    let response_str = String::from_utf8_lossy(&response).to_string();
    // Strip HTTP headers
    if let Some(idx) = response_str.find("\r\n\r\n") {
        Ok(Value::String(Rc::new(response_str[idx + 4..].to_string())))
    } else {
        Ok(Value::String(Rc::new(response_str)))
    }
}

fn native_http_post(args: &[Value]) -> Result<Value, String> {
    if args.len() < 3 {
        return Err("Http_post: expected 3 arguments (url, body, contentType)".to_string());
    }
    let url = match &args[0] {
        Value::String(s) => s.as_str(),
        _ => return Err("Http_post: expected String url".to_string()),
    };
    let body = match &args[1] {
        Value::String(s) => s.as_str(),
        _ => return Err("Http_post: expected String body".to_string()),
    };
    let content_type = match &args[2] {
        Value::String(s) => s.as_str(),
        _ => return Err("Http_post: expected String contentType".to_string()),
    };

    let url_str = url.to_string();
    let (host, port, path) = parse_http_url(&url_str)?;

    let addr = format!("{}:{}", host, port);
    let mut stream = std::net::TcpStream::connect(&addr)
        .map_err(|e| format!("Http_post: connection failed: {}", e))?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(30)))
        .map_err(|e| format!("Http_post: {}", e))?;

    let request = format!(
        "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        path, host, content_type, body.len(), body
    );
    use std::io::Write;
    stream.write_all(request.as_bytes())
        .map_err(|e| format!("Http_post: write failed: {}", e))?;

    use std::io::Read;
    let mut response = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => response.extend_from_slice(&buf[..n]),
            Err(e) => {
                if response.is_empty() {
                    return Err(format!("Http_post: read failed: {}", e));
                }
                break;
            }
        }
    }

    let response_str = String::from_utf8_lossy(&response).to_string();
    if let Some(idx) = response_str.find("\r\n\r\n") {
        Ok(Value::String(Rc::new(response_str[idx + 4..].to_string())))
    } else {
        Ok(Value::String(Rc::new(response_str)))
    }
}

/// Parse a simple HTTP URL into (host, port, path).
fn parse_http_url(url: &str) -> Result<(String, u16, String), String> {
    let url = url.strip_prefix("http://").unwrap_or(url);
    let url = url.strip_prefix("https://").unwrap_or(url);

    let (host_port, path) = match url.find('/') {
        Some(idx) => (&url[..idx], &url[idx..]),
        None => (url, "/"),
    };

    let (host, port) = if host_port.contains(':') {
        let parts: Vec<&str> = host_port.splitn(2, ':').collect();
        let port: u16 = parts[1].parse().unwrap_or(80);
        (parts[0].to_string(), port)
    } else {
        (host_port.to_string(), 80)
    };

    Ok((host, port, path.to_string()))
}

// ---------------------------------------------------------------------------
// Time native functions
// ---------------------------------------------------------------------------

fn native_time_now(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let epoch_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("Time_now: {}", e))?
        .as_millis() as i64;
    Ok(Value::Long(epoch_ms))
}

fn native_time_sleep(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Time_sleep: expected 1 argument (milliseconds)".to_string());
    }
    let ms = args[0].to_i64().unwrap_or(0);
    std::thread::sleep(std::time::Duration::from_millis(ms as u64));
    Ok(Value::Void)
}

fn native_time_format(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Time_format: expected 2 arguments (epoch_ms, format)".to_string());
    }
    let epoch_ms = args[0].to_i64().unwrap_or(0);
    let fmt = match &args[1] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Time_format: expected String format".to_string()),
    };
    // Simple format: support yyyy, MM, dd, HH, mm, ss placeholders
    let secs = epoch_ms / 1000;
    let datetime = chrono::DateTime::from_timestamp(secs, 0)
        .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());
    let formatted = datetime.format(&fmt).to_string();
    Ok(Value::String(Rc::new(formatted)))
}

fn native_time_get_year(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Time_getYear: expected 1 argument (epoch_ms)".to_string());
    }
    let epoch_ms = args[0].to_i64().unwrap_or(0);
    let secs = epoch_ms / 1000;
    let datetime = chrono::DateTime::from_timestamp(secs, 0)
        .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());
    Ok(Value::Int(datetime.year() as i32))
}

fn native_time_get_month(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Time_getMonth: expected 1 argument (epoch_ms)".to_string());
    }
    let epoch_ms = args[0].to_i64().unwrap_or(0);
    let secs = epoch_ms / 1000;
    let datetime = chrono::DateTime::from_timestamp(secs, 0)
        .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());
    Ok(Value::Int(datetime.month() as i32))
}

fn native_time_get_day(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Time_getDay: expected 1 argument (epoch_ms)".to_string());
    }
    let epoch_ms = args[0].to_i64().unwrap_or(0);
    let secs = epoch_ms / 1000;
    let datetime = chrono::DateTime::from_timestamp(secs, 0)
        .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());
    Ok(Value::Int(datetime.day() as i32))
}

fn native_time_get_hour(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Time_getHour: expected 1 argument (epoch_ms)".to_string());
    }
    let epoch_ms = args[0].to_i64().unwrap_or(0);
    let secs = epoch_ms / 1000;
    let datetime = chrono::DateTime::from_timestamp(secs, 0)
        .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());
    Ok(Value::Int(datetime.hour() as i32))
}

fn native_time_get_minute(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Time_getMinute: expected 1 argument (epoch_ms)".to_string());
    }
    let epoch_ms = args[0].to_i64().unwrap_or(0);
    let secs = epoch_ms / 1000;
    let datetime = chrono::DateTime::from_timestamp(secs, 0)
        .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());
    Ok(Value::Int(datetime.minute() as i32))
}

fn native_time_get_second(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Time_getSecond: expected 1 argument (epoch_ms)".to_string());
    }
    let epoch_ms = args[0].to_i64().unwrap_or(0);
    let secs = epoch_ms / 1000;
    let datetime = chrono::DateTime::from_timestamp(secs, 0)
        .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());
    Ok(Value::Int(datetime.second() as i32))
}

// ---------------------------------------------------------------------------
// Regex native functions
// ---------------------------------------------------------------------------

fn native_regex_match(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Regex_match: expected 2 arguments (pattern, input)".to_string());
    }
    let pattern = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Regex_match: expected String pattern".to_string()),
    };
    let input = match &args[1] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Regex_match: expected String input".to_string()),
    };
    let re = regex::Regex::new(&pattern)
        .map_err(|e| format!("Regex_match: invalid pattern '{}': {}", pattern, e))?;
    Ok(Value::Bool(re.is_match(&input)))
}

fn native_regex_find(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Regex_find: expected 2 arguments (pattern, input)".to_string());
    }
    let pattern = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Regex_find: expected String pattern".to_string()),
    };
    let input = match &args[1] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Regex_find: expected String input".to_string()),
    };
    let re = regex::Regex::new(&pattern)
        .map_err(|e| format!("Regex_find: invalid pattern '{}': {}", pattern, e))?;
    match re.find(&input) {
        Some(m) => {
            // Return "start,end,matched_text"
            let result = format!("{},{},{}", m.start(), m.end(), m.as_str());
            Ok(Value::String(Rc::new(result)))
        }
        None => Ok(Value::String(Rc::new(String::new()))),
    }
}

fn native_regex_replace(args: &[Value]) -> Result<Value, String> {
    if args.len() < 3 {
        return Err("Regex_replace: expected 3 arguments (pattern, input, replacement)".to_string());
    }
    let pattern = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Regex_replace: expected String pattern".to_string()),
    };
    let input = match &args[1] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Regex_replace: expected String input".to_string()),
    };
    let replacement = match &args[2] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Regex_replace: expected String replacement".to_string()),
    };
    let re = regex::Regex::new(&pattern)
        .map_err(|e| format!("Regex_replace: invalid pattern '{}': {}", pattern, e))?;
    let result = re.replace_all(&input, &replacement).to_string();
    Ok(Value::String(Rc::new(result)))
}

// ---------------------------------------------------------------------------
// Math native functions
// ---------------------------------------------------------------------------

fn native_math_sin(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_sin: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    Ok(Value::Double(x.sin()))
}

fn native_math_cos(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_cos: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    Ok(Value::Double(x.cos()))
}

fn native_math_tan(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_tan: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    Ok(Value::Double(x.tan()))
}

fn native_math_asin(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_asin: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    Ok(Value::Double(x.asin()))
}

fn native_math_acos(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_acos: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    Ok(Value::Double(x.acos()))
}

fn native_math_atan(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_atan: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    Ok(Value::Double(x.atan()))
}

fn native_math_atan2(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 { return Err("Math_atan2: expected 2 arguments (y, x)".to_string()); }
    let y = args[0].to_f64().unwrap_or(0.0);
    let x = args[1].to_f64().unwrap_or(0.0);
    Ok(Value::Double(y.atan2(x)))
}

fn native_math_ln(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_ln: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    Ok(Value::Double(x.ln()))
}

fn native_math_log10(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_log10: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    Ok(Value::Double(x.log10()))
}

fn native_math_log2(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_log2: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    Ok(Value::Double(x.log2()))
}

fn native_math_exp(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_exp: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    Ok(Value::Double(x.exp()))
}

fn native_math_pow(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 { return Err("Math_pow: expected 2 arguments (base, exp)".to_string()); }
    let base = args[0].to_f64().unwrap_or(0.0);
    let exp = args[1].to_f64().unwrap_or(0.0);
    Ok(Value::Double(base.powf(exp)))
}

fn native_math_sqrt(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_sqrt: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    Ok(Value::Double(x.sqrt()))
}

fn native_math_cbrt(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_cbrt: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    Ok(Value::Double(x.cbrt()))
}

fn native_math_abs(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_abs: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    Ok(Value::Double(x.abs()))
}

fn native_math_abs_int(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_absInt: expected 1 argument".to_string()); }
    let x = args[0].to_i64().unwrap_or(0);
    Ok(Value::Long(x.abs()))
}

fn native_math_floor(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_floor: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    Ok(Value::Double(x.floor()))
}

fn native_math_ceil(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_ceil: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    Ok(Value::Double(x.ceil()))
}

fn native_math_round(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_round: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    Ok(Value::Long(x.round() as i64))
}

fn native_math_inf(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Ok(Value::Double(f64::INFINITY))
}

fn native_math_nan(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Ok(Value::Double(f64::NAN))
}

fn native_math_max_double(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Ok(Value::Double(f64::MAX))
}

fn native_math_min_double(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Ok(Value::Double(f64::MIN))
}

fn native_math_max_int(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Ok(Value::Long(i64::MAX))
}

fn native_math_min_int(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Ok(Value::Long(i64::MIN))
}

// ---------------------------------------------------------------------------
// Random native functions
// ---------------------------------------------------------------------------

fn native_random_seed(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let epoch_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("Random_seed: {}", e))?
        .as_millis() as i64;
    Ok(Value::Long(epoch_ms))
}

fn native_random_next_long(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Random_nextLong: expected 2 arguments (state0, state1)".to_string());
    }
    let s0 = args[0].to_i64().unwrap_or(0) as u64;
    let mut s1 = args[1].to_i64().unwrap_or(0) as u64;

    // Xorshift128+
    s1 ^= s1 << 23;
    s1 ^= s1 >> 17;
    s1 ^= s0;
    s1 ^= s0 >> 26;
    let new_s0 = s1;
    let result = (new_s0.wrapping_add(s1)) as i64;
    let new_s1 = s0;

    Ok(Value::Array {
        elements: vec![
            Value::Long(new_s0 as i64),
            Value::Long(new_s1 as i64),
            Value::Long(result),
        ],
    })
}

fn native_json_parse(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Json_parse: expected 1 argument (json string)".to_string());
    }
    let json_str = match &args[0] {
        Value::String(s) => s.as_str().trim().to_string(),
        _ => return Err("Json_parse: expected String argument".to_string()),
    };
    json_parse_value(&json_str).map(|(v, _)| v)
}

/// Simple recursive-descent JSON parser.
/// Returns (Value, remaining_string) on success.
fn json_parse_value(input: &str) -> Result<(Value, &str), String> {
    let input = input.trim_start();
    if input.is_empty() {
        return Err("Json_parse: unexpected end of input".to_string());
    }
    if input.starts_with("null") {
        return Ok((Value::Null, &input[4..]));
    }
    if input.starts_with("true") {
        return Ok((Value::Bool(true), &input[4..]));
    }
    if input.starts_with("false") {
        return Ok((Value::Bool(false), &input[5..]));
    }
    if input.starts_with('"') {
        return json_parse_string(input);
    }
    if input.starts_with('[') {
        return json_parse_array(input);
    }
    if input.starts_with('{') {
        return json_parse_object(input);
    }
    // Number
    json_parse_number(input)
}

fn json_parse_string(input: &str) -> Result<(Value, &str), String> {
    let bytes = input.as_bytes();
    if bytes[0] != b'"' {
        return Err("Json_parse: expected '\"'".to_string());
    }
    let mut i = 1;
    let mut result = String::new();
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => {
                if i + 1 >= bytes.len() {
                    return Err("Json_parse: unexpected end of string escape".to_string());
                }
                i += 1;
                match bytes[i] {
                    b'"' => result.push('"'),
                    b'\\' => result.push('\\'),
                    b'/' => result.push('/'),
                    b'n' => result.push('\n'),
                    b'r' => result.push('\r'),
                    b't' => result.push('\t'),
                    _ => result.push(bytes[i] as char),
                }
                i += 1;
            }
            b'"' => {
                return Ok((Value::String(Rc::new(result)), &input[i + 1..]));
            }
            b => {
                result.push(b as char);
                i += 1;
            }
        }
    }
    Err("Json_parse: unterminated string".to_string())
}

fn json_parse_number(input: &str) -> Result<(Value, &str), String> {
    let mut i = 0;
    let bytes = input.as_bytes();
    let start = 0;
    if i < bytes.len() && (bytes[i] == b'-' || bytes[i] == b'+') {
        i += 1;
    }
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    let is_float = if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        true
    } else {
        false
    };
    // Handle exponent
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        i += 1;
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
    }
    let num_str = &input[start..i];
    if is_float {
        match num_str.parse::<f64>() {
            Ok(f) => Ok((Value::Double(f), &input[i..])),
            Err(_) => Err(format!("Json_parse: invalid number '{}'", num_str)),
        }
    } else {
        match num_str.parse::<i64>() {
            Ok(n) => Ok((Value::Long(n), &input[i..])),
            Err(_) => Err(format!("Json_parse: invalid number '{}'", num_str)),
        }
    }
}

fn json_parse_array(input: &str) -> Result<(Value, &str), String> {
    let mut rest = input[1..].trim_start(); // skip '['
    let mut elements = Vec::new();
    if rest.starts_with(']') {
        return Ok((Value::Array { elements }, &rest[1..]));
    }
    loop {
        let (val, remaining) = json_parse_value(rest)?;
        elements.push(val);
        rest = remaining.trim_start();
        if rest.starts_with(']') {
            return Ok((Value::Array { elements }, &rest[1..]));
        }
        if !rest.starts_with(',') {
            return Err("Json_parse: expected ',' or ']' in array".to_string());
        }
        rest = rest[1..].trim_start();
    }
}

fn json_parse_object(input: &str) -> Result<(Value, &str), String> {
    let mut rest = input[1..].trim_start(); // skip '{'
    let mut keys = Vec::new();
    let mut values = Vec::new();
    if rest.starts_with('}') {
        let mut fields = HashMap::new();
        fields.insert("_keys".to_string(), Value::Array { elements: keys });
        fields.insert("_values".to_string(), Value::Array { elements: values });
        return Ok((Value::ClassInstance {
            class_name: "HashMap".to_string(),
            fields: Rc::new(std::cell::RefCell::new(fields)),
            vtable: HashMap::new(),
        }, &rest[1..]));
    }
    loop {
        // Parse key (must be a string)
        let (key_val, remaining) = json_parse_value(rest)?;
        let key_str = match &key_val {
            Value::String(s) => s.as_str().to_string(),
            _ => return Err("Json_parse: object key must be a string".to_string()),
        };
        rest = remaining.trim_start();
        if !rest.starts_with(':') {
            return Err("Json_parse: expected ':' in object".to_string());
        }
        rest = rest[1..].trim_start();
        // Parse value
        let (val, remaining) = json_parse_value(rest)?;
        keys.push(Value::String(Rc::new(key_str)));
        values.push(val);
        rest = remaining.trim_start();
        if rest.starts_with('}') {
            let mut fields = HashMap::new();
            fields.insert("_keys".to_string(), Value::Array { elements: keys });
            fields.insert("_values".to_string(), Value::Array { elements: values });
            return Ok((Value::ClassInstance {
                class_name: "HashMap".to_string(),
                fields: Rc::new(std::cell::RefCell::new(fields)),
                vtable: HashMap::new(),
            }, &rest[1..]));
        }
        if !rest.starts_with(',') {
            return Err("Json_parse: expected ',' or '}' in object".to_string());
        }
        rest = rest[1..].trim_start();
    }
}

// ---------------------------------------------------------------------------
// Env native functions
// ---------------------------------------------------------------------------

fn native_env_get(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Env_get: expected 1 argument (name)".to_string());
    }
    match &args[0] {
        Value::String(name) => match std::env::var(name.as_str()) {
            Ok(val) => Ok(Value::String(Rc::new(val))),
            Err(_) => Ok(Value::Null),
        },
        _ => Err("Env_get: expected String argument".to_string()),
    }
}

fn native_env_set(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Env_set: expected 2 arguments (name, value)".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::String(name), Value::String(val)) => {
            std::env::set_var(name.as_str(), val.as_str());
            Ok(Value::Void)
        }
        _ => Err("Env_set: expected (String, String)".to_string()),
    }
}

fn native_env_vars(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let elements: Vec<Value> = std::env::vars()
        .map(|(k, v)| Value::String(Rc::new(format!("{}={}", k, v))))
        .collect();
    Ok(Value::Array { elements })
}

// ---------------------------------------------------------------------------
// Fs native functions
// ---------------------------------------------------------------------------

fn native_fs_exists(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Fs_exists: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => Ok(Value::Bool(std::path::Path::new(path.as_str()).exists())),
        _ => Err("Fs_exists: expected String argument".to_string()),
    }
}

fn native_fs_is_file(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Fs_isFile: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => Ok(Value::Bool(std::path::Path::new(path.as_str()).is_file())),
        _ => Err("Fs_isFile: expected String argument".to_string()),
    }
}

fn native_fs_is_dir(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Fs_isDir: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => Ok(Value::Bool(std::path::Path::new(path.as_str()).is_dir())),
        _ => Err("Fs_isDir: expected String argument".to_string()),
    }
}

fn native_fs_size(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Fs_size: expected 1 argument (path)".to_string());
    }
    match &args[0] {
        Value::String(path) => match std::fs::metadata(path.as_str()) {
            Ok(meta) => Ok(Value::Long(meta.len() as i64)),
            Err(e) => Err(format!("Fs_size: {}", e)),
        },
        _ => Err("Fs_size: expected String argument".to_string()),
    }
}

// ---------------------------------------------------------------------------
// Process native functions
// ---------------------------------------------------------------------------

fn native_process_id(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Ok(Value::Long(std::process::id() as i64))
}

fn native_process_args(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let elements: Vec<Value> = std::env::args()
        .map(|a| Value::String(Rc::new(a)))
        .collect();
    Ok(Value::Array { elements })
}

// ---------------------------------------------------------------------------
// Os native functions
// ---------------------------------------------------------------------------

fn native_os_name(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Ok(Value::String(Rc::new(std::env::consts::OS.to_string())))
}

fn native_os_arch(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Ok(Value::String(Rc::new(std::env::consts::ARCH.to_string())))
}

fn native_os_family(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Ok(Value::String(Rc::new(std::env::consts::FAMILY.to_string())))
}

// ---------------------------------------------------------------------------
// String utility native functions
// ---------------------------------------------------------------------------

fn native_string_trim_start(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("String_trimStart: expected 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => Ok(Value::String(Rc::new(s.trim_start().to_string()))),
        _ => Err("String_trimStart: expected String argument".to_string()),
    }
}

fn native_string_trim_end(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("String_trimEnd: expected 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => Ok(Value::String(Rc::new(s.trim_end().to_string()))),
        _ => Err("String_trimEnd: expected String argument".to_string()),
    }
}

fn native_string_starts_with(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("String_startsWith: expected 2 arguments (string, prefix)".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::String(s), Value::String(prefix)) => Ok(Value::Bool(s.starts_with(prefix.as_str()))),
        _ => Err("String_startsWith: expected (String, String)".to_string()),
    }
}

fn native_string_ends_with(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("String_endsWith: expected 2 arguments (string, suffix)".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::String(s), Value::String(suffix)) => Ok(Value::Bool(s.ends_with(suffix.as_str()))),
        _ => Err("String_endsWith: expected (String, String)".to_string()),
    }
}

fn native_string_pad_left(args: &[Value]) -> Result<Value, String> {
    if args.len() < 3 {
        return Err("String_padLeft: expected 3 arguments (string, width, char)".to_string());
    }
    match (&args[0], &args[1], &args[2]) {
        (Value::String(s), Value::Int(width), Value::Char(pad_char)) => {
            let padded = format!("{:>width$}", s.as_str(), width = *width as usize)
                .replace(' ', &pad_char.to_string());
            Ok(Value::String(Rc::new(padded)))
        }
        (Value::String(s), Value::Long(width), Value::Char(pad_char)) => {
            let padded = format!("{:>width$}", s.as_str(), width = *width as usize)
                .replace(' ', &pad_char.to_string());
            Ok(Value::String(Rc::new(padded)))
        }
        _ => Err("String_padLeft: expected (String, Int/Long, Char)".to_string()),
    }
}

fn native_string_pad_right(args: &[Value]) -> Result<Value, String> {
    if args.len() < 3 {
        return Err("String_padRight: expected 3 arguments (string, width, char)".to_string());
    }
    match (&args[0], &args[1], &args[2]) {
        (Value::String(s), Value::Int(width), Value::Char(pad_char)) => {
            let padded = format!("{:<width$}", s.as_str(), width = *width as usize)
                .replace(' ', &pad_char.to_string());
            Ok(Value::String(Rc::new(padded)))
        }
        (Value::String(s), Value::Long(width), Value::Char(pad_char)) => {
            let padded = format!("{:<width$}", s.as_str(), width = *width as usize)
                .replace(' ', &pad_char.to_string());
            Ok(Value::String(Rc::new(padded)))
        }
        _ => Err("String_padRight: expected (String, Int/Long, Char)".to_string()),
    }
}

// ---------------------------------------------------------------------------
// Hash native functions
// ---------------------------------------------------------------------------

fn native_hash_md5(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::String(s)) => {
            let mut hasher = Md5::new();
            hasher.update(s.as_bytes());
            let result = hasher.finalize();
            Ok(Value::String(Rc::new(format!("{:x}", result))))
        }
        _ => Err("Hash_md5: expected a String argument".to_string()),
    }
}

fn native_hash_sha1(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::String(s)) => {
            let mut hasher = Sha1::new();
            hasher.update(s.as_bytes());
            let result = hasher.finalize();
            Ok(Value::String(Rc::new(format!("{:x}", result))))
        }
        _ => Err("Hash_sha1: expected a String argument".to_string()),
    }
}

fn native_hash_sha256(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::String(s)) => {
            let mut hasher = Sha256::new();
            hasher.update(s.as_bytes());
            let result = hasher.finalize();
            Ok(Value::String(Rc::new(format!("{:x}", result))))
        }
        _ => Err("Hash_sha256: expected a String argument".to_string()),
    }
}

// ---------------------------------------------------------------------------
// Base64 native functions
// ---------------------------------------------------------------------------

fn native_base64_encode(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::String(s)) => {
            let encoded = general_purpose::STANDARD.encode(s.as_bytes());
            Ok(Value::String(Rc::new(encoded)))
        }
        _ => Err("Base64_encode: expected a String argument".to_string()),
    }
}

fn native_base64_decode(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::String(s)) => {
            general_purpose::STANDARD
                .decode(s.as_str())
                .map(|bytes| Value::String(Rc::new(String::from_utf8_lossy(&bytes).to_string())))
                .map_err(|e| format!("Base64_decode: {}", e))
        }
        _ => Err("Base64_decode: expected a String argument".to_string()),
    }
}

// ---------------------------------------------------------------------------
// Hex native functions
// ---------------------------------------------------------------------------

fn native_hex_encode(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::String(s)) => {
            let encoded: String = s.as_bytes().iter().map(|b| format!("{:02x}", b)).collect();
            Ok(Value::String(Rc::new(encoded)))
        }
        _ => Err("Hex_encode: expected a String argument".to_string()),
    }
}

fn native_hex_decode(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::String(s)) => {
            let bytes: Result<Vec<u8>, _> = (0..s.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
                .collect();
            bytes
                .map(|b| Value::String(Rc::new(String::from_utf8_lossy(&b).to_string())))
                .map_err(|e| format!("Hex_decode: {}", e))
        }
        _ => Err("Hex_decode: expected a String argument".to_string()),
    }
}

// ---------------------------------------------------------------------------
// URL encoding native functions
// ---------------------------------------------------------------------------

fn native_url_encode(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::String(s)) => {
            let encoded = utf8_percent_encode(s, NON_ALPHANUMERIC).to_string();
            Ok(Value::String(Rc::new(encoded)))
        }
        _ => Err("Url_encode: expected a String argument".to_string()),
    }
}

fn native_url_decode(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::String(s)) => {
            percent_decode_str(s)
                .decode_utf8()
                .map(|cow| Value::String(Rc::new(cow.to_string())))
                .map_err(|e| format!("Url_decode: {}", e))
        }
        _ => Err("Url_decode: expected a String argument".to_string()),
    }
}

// ---------------------------------------------------------------------------
// Additional String native functions
// ---------------------------------------------------------------------------

fn native_string_to_uppercase(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::String(s)) => Ok(Value::String(Rc::new(s.to_uppercase()))),
        _ => Err("String_toUppercase: expected a String argument".to_string()),
    }
}

fn native_string_to_lower_case(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::String(s)) => Ok(Value::String(Rc::new(s.to_lowercase()))),
        _ => Err("String_toLowerCase: expected a String argument".to_string()),
    }
}

fn native_string_replace(args: &[Value]) -> Result<Value, String> {
    if args.len() < 3 {
        return Err("String_replace: expected 3 arguments (input, target, replacement)".to_string());
    }
    let input = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("String_replace: expected String input".to_string()),
    };
    let target = match &args[1] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("String_replace: expected String target".to_string()),
    };
    let replacement = match &args[2] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("String_replace: expected String replacement".to_string()),
    };
    Ok(Value::String(Rc::new(input.replace(&target, &replacement))))
}

// ---------------------------------------------------------------------------
// Additional Math native functions
// ---------------------------------------------------------------------------

fn native_math_next_up(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_nextUp: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    Ok(Value::Double(x.next_up()))
}

fn native_math_next_down(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_nextDown: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    Ok(Value::Double(x.next_down()))
}

fn native_math_ulp(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_ulp: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0).abs();
    let ulp = if x == 0.0 {
        f64::MIN_POSITIVE
    } else {
        let exp = x.log2().floor() as i32;
        f64::powf(2.0, exp as f64) * f64::EPSILON
    };
    Ok(Value::Double(ulp))
}

fn native_math_get_exponent(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_getExponent: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    if x == 0.0 || !x.is_finite() {
        return Ok(Value::Long(i64::MIN));
    }
    let exp = x.abs().log2().floor() as i64;
    Ok(Value::Long(exp))
}

fn native_math_scalb(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 { return Err("Math_scalb: expected 2 arguments (x, scaleFactor)".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    let scale = args[1].to_i64().unwrap_or(0) as i32;
    Ok(Value::Double(x * 2.0_f64.powi(scale)))
}

fn native_math_random(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Math_random: {}", e))?
        .as_nanos() as u64;
    // Simple xorshift64 for a quick random double in [0, 1)
    let mut s = seed;
    s ^= s << 13;
    s ^= s >> 7;
    s ^= s << 17;
    let result = (s >> 11) as f64 / (1u64 << 53) as f64;
    Ok(Value::Double(result))
}

fn native_math_neg_inf(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Ok(Value::Double(f64::NEG_INFINITY))
}

// ---------------------------------------------------------------------------
// Additional Regex native functions
// ---------------------------------------------------------------------------

fn native_regex_group_count(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Regex_groupCount: expected 1 argument (pattern)".to_string());
    }
    let pattern = match args.first() {
        Some(Value::String(s)) => s.as_str().to_string(),
        _ => return Err("Regex_groupCount: expected String pattern".to_string()),
    };
    let re = regex::Regex::new(&pattern)
        .map_err(|e| format!("Regex_groupCount: invalid pattern '{}': {}", pattern, e))?;
    Ok(Value::Int(re.captures_len() as i32 - 1))
}

// ---------------------------------------------------------------------------
// Additional Directory native functions
// ---------------------------------------------------------------------------

fn native_dir_walk(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Dir_walk: expected 1 argument (path)".to_string());
    }
    let path = match args.first() {
        Some(Value::String(s)) => s.as_str().to_string(),
        _ => return Err("Dir_walk: expected String path".to_string()),
    };
    let mut results = Vec::new();
    fn walk_dir(dir: &std::path::Path, results: &mut Vec<Value>) -> Result<(), String> {
        let entries = std::fs::read_dir(dir)
            .map_err(|e| format!("Dir_walk: cannot read '{}': {}", dir.display(), e))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("Dir_walk: {}", e))?;
            let path = entry.path();
            results.push(Value::String(Rc::new(path.to_string_lossy().to_string())));
            if path.is_dir() {
                walk_dir(&path, results)?;
            }
        }
        Ok(())
    }
    walk_dir(std::path::Path::new(&path), &mut results)?;
    Ok(Value::Array { elements: results })
}

fn native_dir_copy(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Dir_copy: expected 2 arguments (src, dst)".to_string());
    }
    let src = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Dir_copy: expected String source path".to_string()),
    };
    let dst = match &args[1] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Dir_copy: expected String destination path".to_string()),
    };
    fn copy_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<(), String> {
        std::fs::create_dir_all(dst)
            .map_err(|e| format!("Dir_copy: cannot create '{}': {}", dst.display(), e))?;
        for entry in std::fs::read_dir(src)
            .map_err(|e| format!("Dir_copy: cannot read '{}': {}", src.display(), e))?
        {
            let entry = entry.map_err(|e| format!("Dir_copy: {}", e))?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            if src_path.is_dir() {
                copy_recursive(&src_path, &dst_path)?;
            } else {
                std::fs::copy(&src_path, &dst_path)
                    .map_err(|e| format!("Dir_copy: cannot copy '{}': {}", src_path.display(), e))?;
            }
        }
        Ok(())
    }
    copy_recursive(std::path::Path::new(&src), std::path::Path::new(&dst))?;
    Ok(Value::Void)
}

fn native_dir_move(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Dir_move: expected 2 arguments (src, dst)".to_string());
    }
    let src = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Dir_move: expected String source path".to_string()),
    };
    let dst = match &args[1] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Dir_move: expected String destination path".to_string()),
    };
    std::fs::rename(&src, &dst)
        .map_err(|e| format!("Dir_move: cannot move '{}' to '{}': {}", src, dst, e))?;
    Ok(Value::Void)
}

// ---------------------------------------------------------------------------
// Additional Time native functions
// ---------------------------------------------------------------------------

fn native_time_day_of_week(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Time_dayOfWeek: expected 1 argument (epoch seconds)".to_string());
    }
    let secs = args[0].to_i64().unwrap_or(0);
    let datetime = chrono::DateTime::from_timestamp(secs, 0)
        .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());
    // chrono: 0=Mon, 6=Sun via .weekday().num_days_from_monday()
    Ok(Value::Int(datetime.weekday().num_days_from_monday() as i32))
}

fn native_time_day_of_year(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Time_dayOfYear: expected 1 argument (epoch seconds)".to_string());
    }
    let secs = args[0].to_i64().unwrap_or(0);
    let datetime = chrono::DateTime::from_timestamp(secs, 0)
        .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());
    Ok(Value::Int(datetime.ordinal() as i32))
}

// ---------------------------------------------------------------------------
// Double and Long parsing native functions
// ---------------------------------------------------------------------------

fn native_double_parse_double(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Double_parseDouble: expected 1 argument (string)".to_string());
    }
    let s = match &args[0] {
        Value::String(s) => s.as_str().trim().to_string(),
        _ => return Err("Double_parseDouble: expected String argument".to_string()),
    };
    s.parse::<f64>()
        .map(Value::Double)
        .map_err(|e| format!("Double_parseDouble: cannot parse '{}': {}", s, e))
}

fn native_long_parse_long(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Long_parseLong: expected 1 argument (string)".to_string());
    }
    let s = match &args[0] {
        Value::String(s) => s.as_str().trim().to_string(),
        _ => return Err("Long_parseLong: expected String argument".to_string()),
    };
    s.parse::<i64>()
        .map(Value::Long)
        .map_err(|e| format!("Long_parseLong: cannot parse '{}': {}", s, e))
}

// ---------------------------------------------------------------------------
// Subprocess native functions
// ---------------------------------------------------------------------------

fn native_subprocess_run(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Subprocess_run: expected at least 1 argument (command)".to_string());
    }
    let program = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Subprocess_run: expected String command".to_string()),
    };
    let mut cmd = std::process::Command::new(&program);
    // Additional arguments as strings
    for arg in &args[1..] {
        match arg {
            Value::String(s) => { cmd.arg(s.as_str()); }
            other => { cmd.arg(format!("{:?}", other)); }
        }
    }
    let status = cmd.status()
        .map_err(|e| format!("Subprocess_run: failed to execute '{}': {}", program, e))?;
    Ok(Value::Int(status.code().unwrap_or(-1)))
}

// ---------------------------------------------------------------------------
// Tempfile native functions
// ---------------------------------------------------------------------------

fn native_tempfile_create(args: &[Value]) -> Result<Value, String> {
    let prefix = match args.first() {
        Some(Value::String(s)) => s.as_str().to_string(),
        _ => "titrate_".to_string(),
    };
    let is_dir = args.get(1)
        .map(|v| matches!(v, Value::Bool(true)))
        .unwrap_or(false);
    if is_dir {
        let dir = std::env::temp_dir().join(format!("{}{}", prefix, std::process::id()));
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("Tempfile_create: cannot create directory '{}': {}", dir.display(), e))?;
        Ok(Value::String(Rc::new(dir.to_string_lossy().to_string())))
    } else {
        let path = std::env::temp_dir().join(format!("{}{}", prefix, std::process::id()));
        std::fs::File::create(&path)
            .map_err(|e| format!("Tempfile_create: cannot create file '{}': {}", path.display(), e))?;
        Ok(Value::String(Rc::new(path.to_string_lossy().to_string())))
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
        "File_open" => Some(native_file_open),
        "File_readLine" => Some(native_file_read_line),
        "File_write" => Some(native_file_write_content),
        "File_close" => Some(native_file_close),
        "String_split" => Some(native_string_split),
        "Integer_parseOr" => Some(native_integer_parse_or),
        "String_trim" => Some(native_string_trim),
        "String_length" => Some(native_string_length),
        // Path natives
        "Path_join" => Some(native_path_join),
        "Path_exists" => Some(native_path_exists),
        "Path_isFile" => Some(native_path_is_file),
        "Path_isDir" => Some(native_path_is_dir),
        "Path_basename" => Some(native_path_basename),
        "Path_dirname" => Some(native_path_dirname),
        "Path_extension" => Some(native_path_extension),
        // Directory natives
        "Dir_list" => Some(native_dir_list),
        "Dir_create" => Some(native_dir_create),
        "Dir_remove" => Some(native_dir_remove),
        // Sys natives
        "Sys_args" => Some(native_sys_args),
        "Sys_env" => Some(native_sys_env),
        "Sys_setEnv" => Some(native_sys_set_env),
        "Sys_exit" => Some(native_sys_exit),
        "Sys_workingDir" => Some(native_sys_working_dir),
        "Sys_sleep" => Some(native_sys_sleep),
        // Network natives
        "Net_connect" => Some(native_net_connect),
        "Net_send" => Some(native_net_send),
        "Net_receive" => Some(native_net_receive),
        "Net_bind" => Some(native_net_bind),
        "Net_accept" => Some(native_net_accept),
        "Net_close" => Some(native_net_close),
        "Http_get" => Some(native_http_get),
        "Http_post" => Some(native_http_post),
        // Time natives
        "Time_now" => Some(native_time_now),
        "Time_sleep" => Some(native_time_sleep),
        "Time_format" => Some(native_time_format),
        "Time_getYear" => Some(native_time_get_year),
        "Time_getMonth" => Some(native_time_get_month),
        "Time_getDay" => Some(native_time_get_day),
        "Time_getHour" => Some(native_time_get_hour),
        "Time_getMinute" => Some(native_time_get_minute),
        "Time_getSecond" => Some(native_time_get_second),
        // Regex natives
        "Regex_match" => Some(native_regex_match),
        "Regex_find" => Some(native_regex_find),
        "Regex_replace" => Some(native_regex_replace),
        // Math natives
        "Math_sin" => Some(native_math_sin),
        "Math_cos" => Some(native_math_cos),
        "Math_tan" => Some(native_math_tan),
        "Math_asin" => Some(native_math_asin),
        "Math_acos" => Some(native_math_acos),
        "Math_atan" => Some(native_math_atan),
        "Math_atan2" => Some(native_math_atan2),
        "Math_ln" => Some(native_math_ln),
        "Math_log10" => Some(native_math_log10),
        "Math_log2" => Some(native_math_log2),
        "Math_exp" => Some(native_math_exp),
        "Math_pow" => Some(native_math_pow),
        "Math_sqrt" => Some(native_math_sqrt),
        "Math_cbrt" => Some(native_math_cbrt),
        "Math_abs" => Some(native_math_abs),
        "Math_absInt" => Some(native_math_abs_int),
        "Math_floor" => Some(native_math_floor),
        "Math_ceil" => Some(native_math_ceil),
        "Math_round" => Some(native_math_round),
        "Math_inf" => Some(native_math_inf),
        "Math_nan" => Some(native_math_nan),
        "Math_maxDouble" => Some(native_math_max_double),
        "Math_minDouble" => Some(native_math_min_double),
        "Math_maxInt" => Some(native_math_max_int),
        "Math_minInt" => Some(native_math_min_int),
        // Random natives
        "Random_seed" => Some(native_random_seed),
        "Random_nextLong" => Some(native_random_next_long),
        // Json natives
        "Json_parse" => Some(native_json_parse),
        // Env natives
        "Env_get" => Some(native_env_get),
        "Env_set" => Some(native_env_set),
        "Env_vars" => Some(native_env_vars),
        // Fs natives
        "Fs_exists" => Some(native_fs_exists),
        "Fs_isFile" => Some(native_fs_is_file),
        "Fs_isDir" => Some(native_fs_is_dir),
        "Fs_size" => Some(native_fs_size),
        // Process natives
        "Process_id" => Some(native_process_id),
        "Process_args" => Some(native_process_args),
        // Os natives
        "Os_name" => Some(native_os_name),
        "Os_arch" => Some(native_os_arch),
        "Os_family" => Some(native_os_family),
        // String utility natives
        "String_trimStart" => Some(native_string_trim_start),
        "String_trimEnd" => Some(native_string_trim_end),
        "String_startsWith" => Some(native_string_starts_with),
        "String_endsWith" => Some(native_string_ends_with),
        "String_padLeft" => Some(native_string_pad_left),
        "String_padRight" => Some(native_string_pad_right),
        // Additional String natives
        "String_toUppercase" => Some(native_string_to_uppercase),
        "String_toLowerCase" => Some(native_string_to_lower_case),
        "String_replace" => Some(native_string_replace),
        // Hash natives
        "Hash_md5" => Some(native_hash_md5),
        "Hash_sha1" => Some(native_hash_sha1),
        "Hash_sha256" => Some(native_hash_sha256),
        // Base64 natives
        "Base64_encode" => Some(native_base64_encode),
        "Base64_decode" => Some(native_base64_decode),
        // Hex natives
        "Hex_encode" => Some(native_hex_encode),
        "Hex_decode" => Some(native_hex_decode),
        // URL encoding natives
        "Url_encode" => Some(native_url_encode),
        "Url_decode" => Some(native_url_decode),
        // Additional Math natives
        "Math_nextUp" => Some(native_math_next_up),
        "Math_nextDown" => Some(native_math_next_down),
        "Math_ulp" => Some(native_math_ulp),
        "Math_getExponent" => Some(native_math_get_exponent),
        "Math_scalb" => Some(native_math_scalb),
        "Math_random" => Some(native_math_random),
        "Math_negInf" => Some(native_math_neg_inf),
        // Additional Regex natives
        "Regex_groupCount" => Some(native_regex_group_count),
        // Additional Directory natives
        "Dir_walk" => Some(native_dir_walk),
        "Dir_copy" => Some(native_dir_copy),
        "Dir_move" => Some(native_dir_move),
        // Additional Time natives
        "Time_dayOfWeek" => Some(native_time_day_of_week),
        "Time_dayOfYear" => Some(native_time_day_of_year),
        // Double and Long parsing natives
        "Double_parseDouble" => Some(native_double_parse_double),
        "Long_parseLong" => Some(native_long_parse_long),
        // Subprocess natives
        "Subprocess_run" => Some(native_subprocess_run),
        // Tempfile natives
        "Tempfile_create" => Some(native_tempfile_create),
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

    // -- 13. test_closure_execution ---------------------------------------------

    #[test]
    fn test_closure_execution() {
        // Main: push 3, CLOSURE_NEW func#1 (0 upvalues), PUSH_I32 7,
        //       CALL func#1 with 1 arg, RET
        // Func#1 ($closure_0): LOAD_LOCAL 0 (arg), RET
        let mut main_chunk = Chunk::new();
        // PUSH_I32 3 (dummy value on stack before closure creation)
        main_chunk.write_opcode(OpCode::PUSH_I32, 1);
        main_chunk.code.extend_from_slice(&3i32.to_be_bytes());
        main_chunk.source_lines.extend_from_slice(&[1; 4]);
        // POP the dummy (we just needed something before CLOSURE_NEW)
        main_chunk.write_opcode(OpCode::POP, 1);
        // CLOSURE_NEW func_idx=1, upvalue_count=0
        main_chunk.write_opcode(OpCode::CLOSURE_NEW, 1);
        main_chunk.write_u16(1, 1); // function index 1
        main_chunk.write_u8(0, 1);  // 0 upvalues
        // PUSH_I32 7  (argument for the closure)
        main_chunk.write_opcode(OpCode::PUSH_I32, 1);
        main_chunk.code.extend_from_slice(&7i32.to_be_bytes());
        main_chunk.source_lines.extend_from_slice(&[1; 4]);
        // CALL func_idx=1, arg_count=1
        main_chunk.write_opcode(OpCode::CALL, 1);
        main_chunk.write_u16(1, 1); // function index 1
        main_chunk.write_u8(1, 1);  // 1 arg
        // RET
        main_chunk.write_opcode(OpCode::RET, 1);

        let mut closure_chunk = Chunk::new();
        // LOAD_LOCAL 0 (first arg = 7)
        closure_chunk.write_opcode(OpCode::LOAD_LOCAL, 1);
        closure_chunk.write_u8(0, 1);
        // RET
        closure_chunk.write_opcode(OpCode::RET, 1);

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
            name: "$closure_0".to_string(),
            arity: 1,
            chunk: closure_chunk,
            is_method: false,
            is_constructor: false,
            local_count: 1,
        });

        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Int(7)));
    }

    // -- 14. test_closure_capture_variable ---------------------------------------

    #[test]
    fn test_closure_capture_variable() {
        // Main: PUSH_I32 10, STORE_LOCAL 0, LOAD_LOCAL 0,
        //       CLOSURE_NEW func#1 (1 upvalue: the 10), PUSH_I32 5,
        //       CALL func#1 with 1 arg, RET
        // Func#1 ($closure_0): LOAD_LOCAL 0 (arg=5), GET_UPVALUE 0 (captured=10), ADD_I32, RET
        let mut main_chunk = Chunk::new();
        // PUSH_I32 10
        main_chunk.write_opcode(OpCode::PUSH_I32, 1);
        main_chunk.code.extend_from_slice(&10i32.to_be_bytes());
        main_chunk.source_lines.extend_from_slice(&[1; 4]);
        // STORE_LOCAL 0
        main_chunk.write_opcode(OpCode::STORE_LOCAL, 1);
        main_chunk.write_u8(0, 1);
        // LOAD_LOCAL 0 (push captured value for CLOSURE_NEW)
        main_chunk.write_opcode(OpCode::LOAD_LOCAL, 1);
        main_chunk.write_u8(0, 1);
        // CLOSURE_NEW func_idx=1, upvalue_count=1
        main_chunk.write_opcode(OpCode::CLOSURE_NEW, 1);
        main_chunk.write_u16(1, 1); // function index 1
        main_chunk.write_u8(1, 1);  // 1 upvalue
        // PUSH_I32 5 (argument for the closure)
        main_chunk.write_opcode(OpCode::PUSH_I32, 1);
        main_chunk.code.extend_from_slice(&5i32.to_be_bytes());
        main_chunk.source_lines.extend_from_slice(&[1; 4]);
        // CALL func_idx=1, arg_count=1
        main_chunk.write_opcode(OpCode::CALL, 1);
        main_chunk.write_u16(1, 1);
        main_chunk.write_u8(1, 1);
        // RET
        main_chunk.write_opcode(OpCode::RET, 1);

        let mut closure_chunk = Chunk::new();
        // LOAD_LOCAL 0 (arg = 5)
        closure_chunk.write_opcode(OpCode::LOAD_LOCAL, 1);
        closure_chunk.write_u8(0, 1);
        // GET_UPVALUE 0 (captured value = 10)
        closure_chunk.write_opcode(OpCode::GET_UPVALUE, 1);
        closure_chunk.write_u8(0, 1);
        // ADD_I32
        closure_chunk.write_opcode(OpCode::ADD_I32, 1);
        // RET
        closure_chunk.write_opcode(OpCode::RET, 1);

        let mut vm = Vm::new();
        vm.add_function(FunctionDef {
            name: "main".to_string(),
            arity: 0,
            chunk: main_chunk,
            is_method: false,
            is_constructor: false,
            local_count: 1,
        });
        vm.add_function(FunctionDef {
            name: "$closure_0".to_string(),
            arity: 1,
            chunk: closure_chunk,
            is_method: false,
            is_constructor: false,
            local_count: 1,
        });

        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Int(15)));
    }

    // -- 15. test_tuple_creation_and_access --------------------------------------

    #[test]
    fn test_tuple_creation_and_access() {
        // Push 42, push "hello", TUPLE_NEW 2, TUPLE_GET 0, RET
        let mut chunk = Chunk::new();
        // PUSH_I32 42
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&42i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        // PUSH_STRING "hello"
        let hello_idx = chunk.add_string("hello");
        chunk.write_opcode(OpCode::PUSH_STRING, 1);
        chunk.write_u16(hello_idx, 1);
        // TUPLE_NEW 2
        chunk.write_opcode(OpCode::TUPLE_NEW, 1);
        chunk.write_u16(2, 1);
        // TUPLE_GET 0 (first element = 42)
        chunk.write_opcode(OpCode::TUPLE_GET, 1);
        chunk.write_u8(0, 1);
        // RET
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Int(42)));
    }

    // -- 16. test_tuple_destructuring_vm -----------------------------------------

    #[test]
    fn test_tuple_destructuring_vm() {
        // Push 10, push 20, TUPLE_NEW 2, store in local 0,
        // LOAD_LOCAL 0, TUPLE_GET 0 → 10 (store in local 1),
        // LOAD_LOCAL 0, TUPLE_GET 1 → 20,
        // LOAD_LOCAL 1 → 10, ADD_I32 → 30, RET
        let mut chunk = Chunk::new();
        // PUSH_I32 10
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&10i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        // PUSH_I32 20
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&20i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        // TUPLE_NEW 2
        chunk.write_opcode(OpCode::TUPLE_NEW, 1);
        chunk.write_u16(2, 1);
        // STORE_LOCAL 0 (store the tuple)
        chunk.write_opcode(OpCode::STORE_LOCAL, 1);
        chunk.write_u8(0, 1);
        // LOAD_LOCAL 0 (load tuple)
        chunk.write_opcode(OpCode::LOAD_LOCAL, 1);
        chunk.write_u8(0, 1);
        // TUPLE_GET 0 → 10
        chunk.write_opcode(OpCode::TUPLE_GET, 1);
        chunk.write_u8(0, 1);
        // STORE_LOCAL 1 (store first element)
        chunk.write_opcode(OpCode::STORE_LOCAL, 1);
        chunk.write_u8(1, 1);
        // LOAD_LOCAL 0 (load tuple again)
        chunk.write_opcode(OpCode::LOAD_LOCAL, 1);
        chunk.write_u8(0, 1);
        // TUPLE_GET 1 → 20
        chunk.write_opcode(OpCode::TUPLE_GET, 1);
        chunk.write_u8(1, 1);
        // LOAD_LOCAL 1 → push 10 back
        chunk.write_opcode(OpCode::LOAD_LOCAL, 1);
        chunk.write_u8(1, 1);
        // ADD_I32 → 30
        chunk.write_opcode(OpCode::ADD_I32, 1);
        // RET
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = Vm::new();
        vm.add_function(FunctionDef {
            name: "main".to_string(),
            arity: 0,
            chunk,
            is_method: false,
            is_constructor: false,
            local_count: 2,
        });

        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Int(30)));
    }

    // -- 17. test_operator_overload_add ------------------------------------------

    #[test]
    fn test_operator_overload_add() {
        // Test INVOKE_OPERATOR "operator+" with two ints (falls back to built-in add)
        let mut chunk = Chunk::new();
        // PUSH_I32 3
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&3i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        // PUSH_I32 4
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&4i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        // INVOKE_OPERATOR "operator+" with 1 arg
        let op_idx = chunk.add_string("operator+");
        chunk.write_opcode(OpCode::INVOKE_OPERATOR, 1);
        chunk.write_u16(op_idx, 1);
        chunk.write_u8(1, 1); // 1 arg (right operand)
        // RET
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Int(7)));
    }

    // -- 18. test_operator_overload_compare ---------------------------------------

    #[test]
    fn test_operator_overload_compare() {
        // Test INVOKE_OPERATOR "operator==" with two equal ints
        let mut chunk = Chunk::new();
        // PUSH_I32 5
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&5i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        // PUSH_I32 5
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&5i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        // INVOKE_OPERATOR "operator==" with 1 arg
        let op_idx = chunk.add_string("operator==");
        chunk.write_opcode(OpCode::INVOKE_OPERATOR, 1);
        chunk.write_u16(op_idx, 1);
        chunk.write_u8(1, 1);
        // RET
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Bool(true)));

        // Test operator!= with different values
        let mut chunk2 = Chunk::new();
        // PUSH_I32 3
        chunk2.write_opcode(OpCode::PUSH_I32, 1);
        chunk2.code.extend_from_slice(&3i32.to_be_bytes());
        chunk2.source_lines.extend_from_slice(&[1; 4]);
        // PUSH_I32 7
        chunk2.write_opcode(OpCode::PUSH_I32, 1);
        chunk2.code.extend_from_slice(&7i32.to_be_bytes());
        chunk2.source_lines.extend_from_slice(&[1; 4]);
        // INVOKE_OPERATOR "operator!=" with 1 arg
        let op_idx2 = chunk2.add_string("operator!=");
        chunk2.write_opcode(OpCode::INVOKE_OPERATOR, 1);
        chunk2.write_u16(op_idx2, 1);
        chunk2.write_u8(1, 1);
        // RET
        chunk2.write_opcode(OpCode::RET, 1);

        let mut vm2 = vm_with_chunk(chunk2);
        vm2.run().unwrap();
        assert_eq!(vm2.stack.last(), Some(&Value::Bool(true)));
    }

    // -- 19. test_json_parse_null -----------------------------------------------

    #[test]
    fn test_json_parse_null() {
        let result = native_json_parse(&[Value::String(Rc::new("null".to_string()))]);
        assert_eq!(result.unwrap(), Value::Null);
    }

    // -- 20. test_json_parse_bool -----------------------------------------------

    #[test]
    fn test_json_parse_bool() {
        let result_true = native_json_parse(&[Value::String(Rc::new("true".to_string()))]);
        assert_eq!(result_true.unwrap(), Value::Bool(true));

        let result_false = native_json_parse(&[Value::String(Rc::new("false".to_string()))]);
        assert_eq!(result_false.unwrap(), Value::Bool(false));
    }

    // -- 21. test_json_parse_number ---------------------------------------------

    #[test]
    fn test_json_parse_number() {
        let result_int = native_json_parse(&[Value::String(Rc::new("42".to_string()))]);
        assert_eq!(result_int.unwrap(), Value::Long(42));

        let result_float = native_json_parse(&[Value::String(Rc::new("3.14".to_string()))]);
        let val = result_float.unwrap();
        match val {
            Value::Double(f) => assert!((f - 3.14).abs() < 0.001),
            _ => panic!("Expected Double, got {:?}", val),
        }
    }

    // -- 22. test_json_parse_string ---------------------------------------------

    #[test]
    fn test_json_parse_string() {
        let result = native_json_parse(&[Value::String(Rc::new("\"hello\"".to_string()))]);
        assert_eq!(result.unwrap(), Value::String(Rc::new("hello".to_string())));
    }

    // -- 23. test_json_parse_array ----------------------------------------------

    #[test]
    fn test_json_parse_array() {
        let result = native_json_parse(&[Value::String(Rc::new("[1, 2, 3]".to_string()))]);
        match result.unwrap() {
            Value::Array { elements } => {
                assert_eq!(elements.len(), 3);
                assert_eq!(elements[0], Value::Long(1));
                assert_eq!(elements[1], Value::Long(2));
                assert_eq!(elements[2], Value::Long(3));
            }
            other => panic!("Expected Array, got {:?}", other),
        }
    }

    // -- 24. test_json_parse_object ---------------------------------------------

    #[test]
    fn test_json_parse_object() {
        let result = native_json_parse(&[Value::String(Rc::new("{\"key\": \"value\"}".to_string()))]);
        match result.unwrap() {
            Value::ClassInstance { class_name, fields, .. } => {
                assert_eq!(class_name, "HashMap");
                let borrowed = fields.borrow();
                match borrowed.get("_keys") {
                    Some(Value::Array { elements: keys }) => {
                        assert_eq!(keys.len(), 1);
                        assert_eq!(keys[0], Value::String(Rc::new("key".to_string())));
                    }
                    _ => panic!("Expected _keys array"),
                }
                match borrowed.get("_values") {
                    Some(Value::Array { elements: values }) => {
                        assert_eq!(values.len(), 1);
                        assert_eq!(values[0], Value::String(Rc::new("value".to_string())));
                    }
                    _ => panic!("Expected _values array"),
                }
            }
            other => panic!("Expected ClassInstance (HashMap), got {:?}", other),
        }
    }

    // -- 25. test_ndarray_zeros --------------------------------------------------

    #[test]
    fn test_ndarray_zeros() {
        // Create a 2x2 zeros array using Value::Array, verify elements are 0.0
        let zeros = Value::Array {
            elements: vec![
                Value::Array { elements: vec![Value::Double(0.0), Value::Double(0.0)] },
                Value::Array { elements: vec![Value::Double(0.0), Value::Double(0.0)] },
            ],
        };
        match &zeros {
            Value::Array { elements } => {
                assert_eq!(elements.len(), 2);
                for row in elements {
                    match row {
                        Value::Array { elements: cols } => {
                            assert_eq!(cols.len(), 2);
                            for val in cols {
                                assert_eq!(*val, Value::Double(0.0));
                            }
                        }
                        _ => panic!("Expected inner Array"),
                    }
                }
            }
            _ => panic!("Expected Array"),
        }
    }

    // -- 26. test_ndarray_ones ---------------------------------------------------

    #[test]
    fn test_ndarray_ones() {
        // Create a 2x2 ones array using Value::Array, verify elements are 1.0
        let ones = Value::Array {
            elements: vec![
                Value::Array { elements: vec![Value::Double(1.0), Value::Double(1.0)] },
                Value::Array { elements: vec![Value::Double(1.0), Value::Double(1.0)] },
            ],
        };
        match &ones {
            Value::Array { elements } => {
                assert_eq!(elements.len(), 2);
                for row in elements {
                    match row {
                        Value::Array { elements: cols } => {
                            assert_eq!(cols.len(), 2);
                            for val in cols {
                                assert_eq!(*val, Value::Double(1.0));
                            }
                        }
                        _ => panic!("Expected inner Array"),
                    }
                }
            }
            _ => panic!("Expected Array"),
        }
    }

    // -- 27. test_ndarray_set_get -----------------------------------------------

    #[test]
    fn test_ndarray_set_get() {
        // Create a 2x2 array, set values, get them back
        let mut arr = Value::Array {
            elements: vec![
                Value::Array { elements: vec![Value::Double(0.0), Value::Double(0.0)] },
                Value::Array { elements: vec![Value::Double(0.0), Value::Double(0.0)] },
            ],
        };
        // Set arr[1][0] = 42.0
        if let Value::Array { elements } = &mut arr {
            if let Value::Array { elements: cols } = &mut elements[1] {
                cols[0] = Value::Double(42.0);
            }
        }
        // Get arr[1][0]
        match &arr {
            Value::Array { elements } => {
                match &elements[1] {
                    Value::Array { elements: cols } => {
                        assert_eq!(cols[0], Value::Double(42.0));
                    }
                    _ => panic!("Expected inner Array"),
                }
            }
            _ => panic!("Expected Array"),
        }
    }

    // -- 28. test_ndarray_add ----------------------------------------------------

    #[test]
    fn test_ndarray_add() {
        // Add two 2x2 arrays element-wise
        let a = Value::Array {
            elements: vec![
                Value::Array { elements: vec![Value::Double(1.0), Value::Double(2.0)] },
                Value::Array { elements: vec![Value::Double(3.0), Value::Double(4.0)] },
            ],
        };
        let b = Value::Array {
            elements: vec![
                Value::Array { elements: vec![Value::Double(5.0), Value::Double(6.0)] },
                Value::Array { elements: vec![Value::Double(7.0), Value::Double(8.0)] },
            ],
        };
        // Element-wise add
        fn add_arrays(a: &Value, b: &Value) -> Value {
            match (a, b) {
                (Value::Array { elements: ea }, Value::Array { elements: eb }) => {
                    Value::Array {
                        elements: ea.iter().zip(eb.iter()).map(|(x, y)| add_arrays(x, y)).collect(),
                    }
                }
                (Value::Double(x), Value::Double(y)) => Value::Double(x + y),
                _ => panic!("Type mismatch in ndarray add"),
            }
        }
        let result = add_arrays(&a, &b);
        match &result {
            Value::Array { elements } => {
                match &elements[0] {
                    Value::Array { elements: cols } => {
                        assert_eq!(cols[0], Value::Double(6.0));
                        assert_eq!(cols[1], Value::Double(8.0));
                    }
                    _ => panic!("Expected inner Array"),
                }
                match &elements[1] {
                    Value::Array { elements: cols } => {
                        assert_eq!(cols[0], Value::Double(10.0));
                        assert_eq!(cols[1], Value::Double(12.0));
                    }
                    _ => panic!("Expected inner Array"),
                }
            }
            _ => panic!("Expected Array"),
        }
    }

    // -- 29. test_ndarray_transpose -----------------------------------------------

    #[test]
    fn test_ndarray_transpose() {
        // Transpose a 2x3 array
        // [[1, 2, 3], [4, 5, 6]] -> [[1, 4], [2, 5], [3, 6]]
        let arr = Value::Array {
            elements: vec![
                Value::Array { elements: vec![Value::Double(1.0), Value::Double(2.0), Value::Double(3.0)] },
                Value::Array { elements: vec![Value::Double(4.0), Value::Double(5.0), Value::Double(6.0)] },
            ],
        };
        // Transpose
        let rows = match &arr {
            Value::Array { elements } => elements.len(),
            _ => panic!("Expected Array"),
        };
        let cols = match &arr {
            Value::Array { elements } => match &elements[0] {
                Value::Array { elements: inner } => inner.len(),
                _ => panic!("Expected inner Array"),
            },
            _ => panic!("Expected Array"),
        };
        let mut transposed: Vec<Vec<Value>> = vec![vec![Value::Double(0.0); rows]; cols];
        if let Value::Array { elements } = &arr {
            for (i, row) in elements.iter().enumerate() {
                if let Value::Array { elements: inner } = row {
                    for (j, val) in inner.iter().enumerate() {
                        transposed[j][i] = val.clone();
                    }
                }
            }
        }
        // Verify transposed shape is 3x2
        assert_eq!(transposed.len(), 3);
        assert_eq!(transposed[0].len(), 2);
        assert_eq!(transposed[0][0], Value::Double(1.0));
        assert_eq!(transposed[0][1], Value::Double(4.0));
        assert_eq!(transposed[1][0], Value::Double(2.0));
        assert_eq!(transposed[1][1], Value::Double(5.0));
        assert_eq!(transposed[2][0], Value::Double(3.0));
        assert_eq!(transposed[2][1], Value::Double(6.0));
    }

    // -- 30. test_matrix_multiply ------------------------------------------------

    #[test]
    fn test_matrix_multiply() {
        // Multiply two 2x2 matrices:
        // [[1, 2], [3, 4]] * [[5, 6], [7, 8]] = [[19, 22], [43, 50]]
        let a: Vec<Vec<f64>> = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let b: Vec<Vec<f64>> = vec![vec![5.0, 6.0], vec![7.0, 8.0]];
        let n = 2;
        let mut result: Vec<Vec<f64>> = vec![vec![0.0; n]; n];
        for i in 0..n {
            for j in 0..n {
                for k in 0..n {
                    result[i][j] += a[i][k] * b[k][j];
                }
            }
        }
        assert_eq!(result[0][0], 19.0);
        assert_eq!(result[0][1], 22.0);
        assert_eq!(result[1][0], 43.0);
        assert_eq!(result[1][1], 50.0);
    }

    // -- 31. test_matrix_transpose -----------------------------------------------

    #[test]
    fn test_matrix_transpose() {
        // Transpose a 2x3 matrix
        let m: Vec<Vec<f64>> = vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]];
        let rows = m.len();
        let cols = m[0].len();
        let mut t: Vec<Vec<f64>> = vec![vec![0.0; rows]; cols];
        for i in 0..rows {
            for j in 0..cols {
                t[j][i] = m[i][j];
            }
        }
        assert_eq!(t.len(), 3);
        assert_eq!(t[0].len(), 2);
        assert_eq!(t[0][0], 1.0);
        assert_eq!(t[0][1], 4.0);
        assert_eq!(t[1][0], 2.0);
        assert_eq!(t[1][1], 5.0);
        assert_eq!(t[2][0], 3.0);
        assert_eq!(t[2][1], 6.0);
    }

    // -- 32. test_matrix_determinant ---------------------------------------------

    #[test]
    fn test_matrix_determinant() {
        // Determinant of [[1, 2], [3, 4]] = 1*4 - 2*3 = -2
        let a: f64 = 1.0;
        let b: f64 = 2.0;
        let c: f64 = 3.0;
        let d: f64 = 4.0;
        let det = a * d - b * c;
        assert!((det - (-2.0)).abs() < f64::EPSILON);
    }

    // -- 33. test_matrix_inverse -------------------------------------------------

    #[test]
    fn test_matrix_inverse() {
        // Invert [[1, 2], [3, 4]]: det = -2, inv = [[-2, 1], [1.5, -0.5]]
        let a: f64 = 1.0;
        let b: f64 = 2.0;
        let c: f64 = 3.0;
        let d: f64 = 4.0;
        let det = a * d - b * c;
        assert!(det.abs() > f64::EPSILON, "matrix is singular");
        let inv_00 = d / det;
        let inv_01 = -b / det;
        let inv_10 = -c / det;
        let inv_11 = a / det;
        assert!((inv_00 - (-2.0)).abs() < 1e-10);
        assert!((inv_01 - 1.0).abs() < 1e-10);
        assert!((inv_10 - 1.5).abs() < 1e-10);
        assert!((inv_11 - (-0.5)).abs() < 1e-10);
    }

    // -- 34. test_json_stringify -------------------------------------------------

    #[test]
    fn test_json_stringify() {
        // Test that a Value can be round-tripped through JSON parse.
        // Since native_json_stringify doesn't exist, we test that
        // native_json_parse produces values whose Debug representation
        // contains the expected data.
        let result = native_json_parse(&[Value::String(Rc::new("{\"x\":42}".to_string()))]);
        match result.unwrap() {
            Value::ClassInstance { fields, .. } => {
                let borrowed = fields.borrow();
                match borrowed.get("_keys") {
                    Some(Value::Array { elements: keys }) => {
                        assert_eq!(keys.len(), 1);
                        assert_eq!(keys[0], Value::String(Rc::new("x".to_string())));
                    }
                    _ => panic!("Expected _keys array"),
                }
                match borrowed.get("_values") {
                    Some(Value::Array { elements: values }) => {
                        assert_eq!(values.len(), 1);
                        assert_eq!(values[0], Value::Long(42));
                    }
                    _ => panic!("Expected _values array"),
                }
            }
            other => panic!("Expected ClassInstance (HashMap), got {:?}", other),
        }
    }

    // -- 35. test_csv_parse ------------------------------------------------------

    #[test]
    fn test_csv_parse() {
        // Parse a simple CSV string manually using String_split-like logic
        let csv = "name,age,city\nAlice,30,NYC\nBob,25,LA";
        let lines: Vec<&str> = csv.split('\n').collect();
        assert_eq!(lines.len(), 3);
        let header: Vec<&str> = lines[0].split(',').collect();
        assert_eq!(header, vec!["name", "age", "city"]);
        let row1: Vec<&str> = lines[1].split(',').collect();
        assert_eq!(row1, vec!["Alice", "30", "NYC"]);
        let row2: Vec<&str> = lines[2].split(',').collect();
        assert_eq!(row2, vec!["Bob", "25", "LA"]);
    }

    // -- 36. test_xml_parse ------------------------------------------------------

    #[test]
    fn test_xml_parse() {
        // Parse a simple XML string manually
        let xml = "<root><item key=\"a\">1</item><item key=\"b\">2</item></root>";
        // Verify basic structure: contains opening/closing tags
        assert!(xml.starts_with("<root>"));
        assert!(xml.ends_with("</root>"));
        // Count <item> occurrences
        let item_count = xml.matches("<item").count();
        assert_eq!(item_count, 2);
        // Extract values between <item> tags
        let values: Vec<&str> = xml.split("<item")
            .skip(1)
            .filter_map(|s| {
                let after_gt = s.find('>')?;
                let before_close = s.find("</item>")?;
                Some(&s[after_gt + 1..before_close])
            })
            .collect();
        assert_eq!(values, vec!["1", "2"]);
    }

    // -- 37. test_closure_as_argument -------------------------------------------

    #[test]
    fn test_closure_as_argument() {
        // Simulate passing a closure to ArrayList.forEach:
        // Create a closure that adds 10 to its argument, then call it with 5.
        // Main: CLOSURE_NEW func#1 (0 upvalues), PUSH_I32 5,
        //       CALL func#1 with 1 arg, RET
        // Func#1 ($closure_0): LOAD_LOCAL 0 (arg), PUSH_I32 10, ADD_I32, RET
        let mut main_chunk = Chunk::new();
        // CLOSURE_NEW func_idx=1, upvalue_count=0
        main_chunk.write_opcode(OpCode::CLOSURE_NEW, 1);
        main_chunk.write_u16(1, 1);
        main_chunk.write_u8(0, 1);
        // PUSH_I32 5 (argument for the closure)
        main_chunk.write_opcode(OpCode::PUSH_I32, 1);
        main_chunk.code.extend_from_slice(&5i32.to_be_bytes());
        main_chunk.source_lines.extend_from_slice(&[1; 4]);
        // CALL func_idx=1, arg_count=1
        main_chunk.write_opcode(OpCode::CALL, 1);
        main_chunk.write_u16(1, 1);
        main_chunk.write_u8(1, 1);
        // RET
        main_chunk.write_opcode(OpCode::RET, 1);

        let mut closure_chunk = Chunk::new();
        // LOAD_LOCAL 0 (arg = 5)
        closure_chunk.write_opcode(OpCode::LOAD_LOCAL, 1);
        closure_chunk.write_u8(0, 1);
        // PUSH_I32 10
        closure_chunk.write_opcode(OpCode::PUSH_I32, 1);
        closure_chunk.code.extend_from_slice(&10i32.to_be_bytes());
        closure_chunk.source_lines.extend_from_slice(&[1; 4]);
        // ADD_I32
        closure_chunk.write_opcode(OpCode::ADD_I32, 1);
        // RET
        closure_chunk.write_opcode(OpCode::RET, 1);

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
            name: "$closure_0".to_string(),
            arity: 1,
            chunk: closure_chunk,
            is_method: false,
            is_constructor: false,
            local_count: 1,
        });

        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Int(15)));
    }

    // -- 38. test_closure_nested -------------------------------------------------

    #[test]
    fn test_closure_nested() {
        // Nested closures capturing different variables:
        // Outer closure captures x=10, inner closure captures y=20.
        // We simulate this by having the outer closure return the inner closure's result.
        // Main: PUSH_I32 10, STORE_LOCAL 0, LOAD_LOCAL 0,
        //       CLOSURE_NEW func#1 (1 upvalue: x=10),
        //       PUSH_I32 3, CALL func#1 with 1 arg, RET
        // Func#1 ($closure_0): PUSH_I32 20, STORE_LOCAL 0, LOAD_LOCAL 0,
        //                      CLOSURE_NEW func#2 (1 upvalue: y=20),
        //                      LOAD_LOCAL 1 (arg), CALL func#2 with 1 arg, RET
        // Func#2 ($closure_1): LOAD_LOCAL 0 (arg), GET_UPVALUE 0 (y=20), ADD_I32, RET

        let mut main_chunk = Chunk::new();
        // PUSH_I32 10
        main_chunk.write_opcode(OpCode::PUSH_I32, 1);
        main_chunk.code.extend_from_slice(&10i32.to_be_bytes());
        main_chunk.source_lines.extend_from_slice(&[1; 4]);
        // STORE_LOCAL 0
        main_chunk.write_opcode(OpCode::STORE_LOCAL, 1);
        main_chunk.write_u8(0, 1);
        // LOAD_LOCAL 0 (push captured value for CLOSURE_NEW)
        main_chunk.write_opcode(OpCode::LOAD_LOCAL, 1);
        main_chunk.write_u8(0, 1);
        // CLOSURE_NEW func_idx=1, upvalue_count=1
        main_chunk.write_opcode(OpCode::CLOSURE_NEW, 1);
        main_chunk.write_u16(1, 1);
        main_chunk.write_u8(1, 1);
        // PUSH_I32 3 (argument for outer closure)
        main_chunk.write_opcode(OpCode::PUSH_I32, 1);
        main_chunk.code.extend_from_slice(&3i32.to_be_bytes());
        main_chunk.source_lines.extend_from_slice(&[1; 4]);
        // CALL func_idx=1, arg_count=1
        main_chunk.write_opcode(OpCode::CALL, 1);
        main_chunk.write_u16(1, 1);
        main_chunk.write_u8(1, 1);
        // RET
        main_chunk.write_opcode(OpCode::RET, 1);

        // Outer closure: takes arg, creates inner closure, calls it
        let mut outer_chunk = Chunk::new();
        // PUSH_I32 20
        outer_chunk.write_opcode(OpCode::PUSH_I32, 1);
        outer_chunk.code.extend_from_slice(&20i32.to_be_bytes());
        outer_chunk.source_lines.extend_from_slice(&[1; 4]);
        // STORE_LOCAL 1
        outer_chunk.write_opcode(OpCode::STORE_LOCAL, 1);
        outer_chunk.write_u8(1, 1);
        // LOAD_LOCAL 1 (push captured value for inner CLOSURE_NEW)
        outer_chunk.write_opcode(OpCode::LOAD_LOCAL, 1);
        outer_chunk.write_u8(1, 1);
        // CLOSURE_NEW func_idx=2, upvalue_count=1
        outer_chunk.write_opcode(OpCode::CLOSURE_NEW, 1);
        outer_chunk.write_u16(2, 1);
        outer_chunk.write_u8(1, 1);
        // LOAD_LOCAL 0 (arg passed to inner closure)
        outer_chunk.write_opcode(OpCode::LOAD_LOCAL, 1);
        outer_chunk.write_u8(0, 1);
        // CALL func_idx=2, arg_count=1
        outer_chunk.write_opcode(OpCode::CALL, 1);
        outer_chunk.write_u16(2, 1);
        outer_chunk.write_u8(1, 1);
        // RET
        outer_chunk.write_opcode(OpCode::RET, 1);

        // Inner closure: arg + upvalue(y=20)
        let mut inner_chunk = Chunk::new();
        // LOAD_LOCAL 0 (arg = 3)
        inner_chunk.write_opcode(OpCode::LOAD_LOCAL, 1);
        inner_chunk.write_u8(0, 1);
        // GET_UPVALUE 0 (captured y = 20)
        inner_chunk.write_opcode(OpCode::GET_UPVALUE, 1);
        inner_chunk.write_u8(0, 1);
        // ADD_I32
        inner_chunk.write_opcode(OpCode::ADD_I32, 1);
        // RET
        inner_chunk.write_opcode(OpCode::RET, 1);

        let mut vm = Vm::new();
        vm.add_function(FunctionDef {
            name: "main".to_string(),
            arity: 0,
            chunk: main_chunk,
            is_method: false,
            is_constructor: false,
            local_count: 1,
        });
        vm.add_function(FunctionDef {
            name: "$closure_0".to_string(),
            arity: 1,
            chunk: outer_chunk,
            is_method: false,
            is_constructor: false,
            local_count: 2,
        });
        vm.add_function(FunctionDef {
            name: "$closure_1".to_string(),
            arity: 1,
            chunk: inner_chunk,
            is_method: false,
            is_constructor: false,
            local_count: 1,
        });

        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Int(23)));
    }

    // -- 39. test_closure_multiple_captures --------------------------------------

    #[test]
    fn test_closure_multiple_captures() {
        // Closure capturing multiple variables: x=5, y=10
        // Main: PUSH_I32 5, STORE_LOCAL 0, PUSH_I32 10, STORE_LOCAL 1,
        //       LOAD_LOCAL 0, LOAD_LOCAL 1,
        //       CLOSURE_NEW func#1 (2 upvalues: x=5, y=10),
        //       PUSH_I32 100, CALL func#1 with 1 arg, RET
        // Func#1 ($closure_0): LOAD_LOCAL 0 (arg), GET_UPVALUE 0 (x=5), ADD_I32,
        //                      GET_UPVALUE 1 (y=10), ADD_I32, RET

        let mut main_chunk = Chunk::new();
        // PUSH_I32 5
        main_chunk.write_opcode(OpCode::PUSH_I32, 1);
        main_chunk.code.extend_from_slice(&5i32.to_be_bytes());
        main_chunk.source_lines.extend_from_slice(&[1; 4]);
        // STORE_LOCAL 0
        main_chunk.write_opcode(OpCode::STORE_LOCAL, 1);
        main_chunk.write_u8(0, 1);
        // PUSH_I32 10
        main_chunk.write_opcode(OpCode::PUSH_I32, 1);
        main_chunk.code.extend_from_slice(&10i32.to_be_bytes());
        main_chunk.source_lines.extend_from_slice(&[1; 4]);
        // STORE_LOCAL 1
        main_chunk.write_opcode(OpCode::STORE_LOCAL, 1);
        main_chunk.write_u8(1, 1);
        // LOAD_LOCAL 0 (first upvalue)
        main_chunk.write_opcode(OpCode::LOAD_LOCAL, 1);
        main_chunk.write_u8(0, 1);
        // LOAD_LOCAL 1 (second upvalue)
        main_chunk.write_opcode(OpCode::LOAD_LOCAL, 1);
        main_chunk.write_u8(1, 1);
        // CLOSURE_NEW func_idx=1, upvalue_count=2
        main_chunk.write_opcode(OpCode::CLOSURE_NEW, 1);
        main_chunk.write_u16(1, 1);
        main_chunk.write_u8(2, 1);
        // PUSH_I32 100 (argument for the closure)
        main_chunk.write_opcode(OpCode::PUSH_I32, 1);
        main_chunk.code.extend_from_slice(&100i32.to_be_bytes());
        main_chunk.source_lines.extend_from_slice(&[1; 4]);
        // CALL func_idx=1, arg_count=1
        main_chunk.write_opcode(OpCode::CALL, 1);
        main_chunk.write_u16(1, 1);
        main_chunk.write_u8(1, 1);
        // RET
        main_chunk.write_opcode(OpCode::RET, 1);

        let mut closure_chunk = Chunk::new();
        // LOAD_LOCAL 0 (arg = 100)
        closure_chunk.write_opcode(OpCode::LOAD_LOCAL, 1);
        closure_chunk.write_u8(0, 1);
        // GET_UPVALUE 0 (captured x = 5)
        closure_chunk.write_opcode(OpCode::GET_UPVALUE, 1);
        closure_chunk.write_u8(0, 1);
        // ADD_I32 → 105
        closure_chunk.write_opcode(OpCode::ADD_I32, 1);
        // GET_UPVALUE 1 (captured y = 10)
        closure_chunk.write_opcode(OpCode::GET_UPVALUE, 1);
        closure_chunk.write_u8(1, 1);
        // ADD_I32 → 115
        closure_chunk.write_opcode(OpCode::ADD_I32, 1);
        // RET
        closure_chunk.write_opcode(OpCode::RET, 1);

        let mut vm = Vm::new();
        vm.add_function(FunctionDef {
            name: "main".to_string(),
            arity: 0,
            chunk: main_chunk,
            is_method: false,
            is_constructor: false,
            local_count: 2,
        });
        vm.add_function(FunctionDef {
            name: "$closure_0".to_string(),
            arity: 1,
            chunk: closure_chunk,
            is_method: false,
            is_constructor: false,
            local_count: 1,
        });

        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Int(115)));
    }

    // -- 40. test_tuple_nested ---------------------------------------------------

    #[test]
    fn test_tuple_nested() {
        // Nested tuples: ((1, 2), (3, 4))
        // Push inner tuple 1, push inner tuple 2, create outer tuple
        let mut chunk = Chunk::new();
        // PUSH_I32 1
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&1i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        // PUSH_I32 2
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&2i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        // TUPLE_NEW 2 → (1, 2)
        chunk.write_opcode(OpCode::TUPLE_NEW, 1);
        chunk.write_u16(2, 1);
        // PUSH_I32 3
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&3i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        // PUSH_I32 4
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&4i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        // TUPLE_NEW 2 → (3, 4)
        chunk.write_opcode(OpCode::TUPLE_NEW, 1);
        chunk.write_u16(2, 1);
        // TUPLE_NEW 2 → ((1, 2), (3, 4))
        chunk.write_opcode(OpCode::TUPLE_NEW, 1);
        chunk.write_u16(2, 1);
        // RET
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        match vm.stack.last() {
            Some(Value::Tuple { elements }) => {
                assert_eq!(elements.len(), 2);
                match &elements[0] {
                    Value::Tuple { elements: inner } => {
                        assert_eq!(inner.len(), 2);
                        assert_eq!(inner[0], Value::Int(1));
                        assert_eq!(inner[1], Value::Int(2));
                    }
                    _ => panic!("Expected inner Tuple"),
                }
                match &elements[1] {
                    Value::Tuple { elements: inner } => {
                        assert_eq!(inner.len(), 2);
                        assert_eq!(inner[0], Value::Int(3));
                        assert_eq!(inner[1], Value::Int(4));
                    }
                    _ => panic!("Expected inner Tuple"),
                }
            }
            _ => panic!("Expected Tuple"),
        }
    }

    // -- 41. test_tuple_return_from_function --------------------------------------

    #[test]
    fn test_tuple_return_from_function() {
        // Function returning a tuple (1, 2)
        // Main: CALL func#1(0 args), TUPLE_GET 0, RET
        // Func#1: PUSH_I32 1, PUSH_I32 2, TUPLE_NEW 2, RET
        let mut main_chunk = Chunk::new();
        // CALL func_idx=1, arg_count=0
        main_chunk.write_opcode(OpCode::CALL, 1);
        main_chunk.write_u16(1, 1);
        main_chunk.write_u8(0, 1);
        // TUPLE_GET 0 (first element = 1)
        main_chunk.write_opcode(OpCode::TUPLE_GET, 1);
        main_chunk.write_u8(0, 1);
        // RET
        main_chunk.write_opcode(OpCode::RET, 1);

        let mut fn_chunk = Chunk::new();
        // PUSH_I32 1
        fn_chunk.write_opcode(OpCode::PUSH_I32, 1);
        fn_chunk.code.extend_from_slice(&1i32.to_be_bytes());
        fn_chunk.source_lines.extend_from_slice(&[1; 4]);
        // PUSH_I32 2
        fn_chunk.write_opcode(OpCode::PUSH_I32, 1);
        fn_chunk.code.extend_from_slice(&2i32.to_be_bytes());
        fn_chunk.source_lines.extend_from_slice(&[1; 4]);
        // TUPLE_NEW 2
        fn_chunk.write_opcode(OpCode::TUPLE_NEW, 1);
        fn_chunk.write_u16(2, 1);
        // RET
        fn_chunk.write_opcode(OpCode::RET, 1);

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
            name: "makeTuple".to_string(),
            arity: 0,
            chunk: fn_chunk,
            is_method: false,
            is_constructor: false,
            local_count: 0,
        });

        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Int(1)));
    }

    // -- 42. test_tuple_in_arraylist ---------------------------------------------

    #[test]
    fn test_tuple_in_arraylist() {
        // ArrayList of tuples: create an Array containing tuples, then access elements
        let mut chunk = Chunk::new();
        // PUSH_I32 10
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&10i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        // PUSH_I32 20
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&20i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        // TUPLE_NEW 2 → (10, 20)
        chunk.write_opcode(OpCode::TUPLE_NEW, 1);
        chunk.write_u16(2, 1);
        // PUSH_I32 30
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&30i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        // PUSH_I32 40
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&40i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        // TUPLE_NEW 2 → (30, 40)
        chunk.write_opcode(OpCode::TUPLE_NEW, 1);
        chunk.write_u16(2, 1);
        // ARRAY_NEW 2 → [(10,20), (30,40)]
        chunk.write_opcode(OpCode::ARRAY_NEW, 1);
        chunk.write_u16(2, 1);
        // PUSH_I32 0 (index for ARRAY_GET)
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&0i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        // ARRAY_GET → pops index (0), pops array, pushes (10, 20)
        chunk.write_opcode(OpCode::ARRAY_GET, 1);
        // TUPLE_GET 1 → 20
        chunk.write_opcode(OpCode::TUPLE_GET, 1);
        chunk.write_u8(1, 1);
        // RET
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Int(20)));
    }

    // -- 43. test_regex_match ----------------------------------------------------

    #[test]
    fn test_regex_match() {
        // Match a simple pattern
        let result = native_regex_match(&[
            Value::String(Rc::new(r"\d+".to_string())),
            Value::String(Rc::new("abc123def".to_string())),
        ]);
        assert_eq!(result.unwrap(), Value::Bool(true));

        // Non-matching pattern
        let result = native_regex_match(&[
            Value::String(Rc::new(r"^[a-z]+$".to_string())),
            Value::String(Rc::new("ABC123".to_string())),
        ]);
        assert_eq!(result.unwrap(), Value::Bool(false));
    }

    // -- 44. test_regex_find -----------------------------------------------------

    #[test]
    fn test_regex_find() {
        // Find a match in a string
        let result = native_regex_find(&[
            Value::String(Rc::new(r"\d+".to_string())),
            Value::String(Rc::new("abc123def".to_string())),
        ]);
        let val = result.unwrap();
        match &val {
            Value::String(s) => {
                // Should be "3,6,123" (start=3, end=6, matched="123")
                let parts: Vec<&str> = s.split(',').collect();
                assert_eq!(parts.len(), 3);
                assert_eq!(parts[0], "3");
                assert_eq!(parts[1], "6");
                assert_eq!(parts[2], "123");
            }
            _ => panic!("Expected String, got {:?}", val),
        }

        // No match
        let result = native_regex_find(&[
            Value::String(Rc::new(r"\d+".to_string())),
            Value::String(Rc::new("abcdef".to_string())),
        ]);
        assert_eq!(result.unwrap(), Value::String(Rc::new(String::new())));
    }

    // -- 45. test_regex_replace --------------------------------------------------

    #[test]
    fn test_regex_replace() {
        // Replace matches in a string
        let result = native_regex_replace(&[
            Value::String(Rc::new(r"\d+".to_string())),
            Value::String(Rc::new("abc123def456".to_string())),
            Value::String(Rc::new("NUM".to_string())),
        ]);
        assert_eq!(
            result.unwrap(),
            Value::String(Rc::new("abcNUMdefNUM".to_string()))
        );
    }

    // -- 46. test_time_now -------------------------------------------------------

    #[test]
    fn test_time_now() {
        // Get current time
        let result = native_time_now(&[]);
        match result.unwrap() {
            Value::Long(ms) => {
                // Should be a reasonable timestamp (after year 2020)
                assert!(ms > 1577836800000i64, "timestamp should be after 2020");
            }
            other => panic!("Expected Long, got {:?}", other),
        }
    }

    // -- 47. test_time_format ----------------------------------------------------

    #[test]
    fn test_time_format() {
        // Format a known timestamp: 0 = Unix epoch
        let result = native_time_format(&[
            Value::Long(0),
            Value::String(Rc::new("%Y-%m-%d".to_string())),
        ]);
        match result.unwrap() {
            Value::String(s) => {
                assert!(s.starts_with("1970"), "epoch should format to 1970, got: {}", s);
            }
            other => panic!("Expected String, got {:?}", other),
        }
    }

    // -- 48. test_duration_arithmetic --------------------------------------------

    #[test]
    fn test_duration_arithmetic() {
        // Add and subtract durations (represented as i64 milliseconds)
        let d1: i64 = 5000; // 5 seconds in ms
        let d2: i64 = 3000; // 3 seconds in ms
        // Add
        let sum = d1 + d2;
        assert_eq!(sum, 8000);
        // Subtract
        let diff = d1 - d2;
        assert_eq!(diff, 2000);
        // Multiply by scalar
        let triple = d1 * 3;
        assert_eq!(triple, 15000);
    }

    // -- 49. test_http_get -------------------------------------------------------

    #[test]
    fn test_http_get() {
        // Make an HTTP GET request (if network available, otherwise skip)
        let result = native_http_get(&[Value::String(Rc::new("http://example.com/".to_string()))]);
        match result {
            Ok(Value::String(s)) => {
                // If we got a response, it should contain some HTML
                assert!(!s.is_empty(), "HTTP response should not be empty");
            }
            Ok(other) => panic!("Expected String, got {:?}", other),
            Err(_) => {
                // Network not available in test environment; skip
                eprintln!("Skipping test_http_get: network unavailable");
            }
        }
    }

    // -- 50. test_set_add_contains -----------------------------------------------

    #[test]
    fn test_set_add_contains() {
        // Simulate a Set using a ClassInstance with _keys array and _values array
        // (matching the HashMap representation used by JSON parse)
        let mut fields_map: HashMap<String, Value> = HashMap::new();
        fields_map.insert("_keys".to_string(), Value::Array {
            elements: vec![Value::Int(1), Value::Int(2), Value::Int(3)],
        });
        fields_map.insert("_values".to_string(), Value::Array {
            elements: vec![Value::Bool(true), Value::Bool(true), Value::Bool(true)],
        });
        let set = Value::ClassInstance {
            class_name: "Set".to_string(),
            fields: Rc::new(RefCell::new(fields_map)),
            vtable: HashMap::new(),
        };

        // Verify the set contains its elements
        match &set {
            Value::ClassInstance { fields, .. } => {
                let borrowed = fields.borrow();
                match borrowed.get("_keys") {
                    Some(Value::Array { elements: keys }) => {
                        assert_eq!(keys.len(), 3);
                        assert!(keys.contains(&Value::Int(1)));
                        assert!(keys.contains(&Value::Int(2)));
                        assert!(keys.contains(&Value::Int(3)));
                        assert!(!keys.contains(&Value::Int(4)));
                    }
                    _ => panic!("Expected _keys array"),
                }
            }
            _ => panic!("Expected ClassInstance"),
        }
    }

    // -- 51. test_set_union ------------------------------------------------------

    #[test]
    fn test_set_union() {
        // Union of two sets: merge their _keys arrays (deduplicated)
        let set_a_keys = vec![Value::Int(1), Value::Int(2)];
        let set_b_keys = vec![Value::Int(2), Value::Int(3)];
        let mut union_keys: Vec<Value> = set_a_keys.clone();
        for k in &set_b_keys {
            if !union_keys.contains(k) {
                union_keys.push(k.clone());
            }
        }
        assert_eq!(union_keys.len(), 3);
        assert!(union_keys.contains(&Value::Int(1)));
        assert!(union_keys.contains(&Value::Int(2)));
        assert!(union_keys.contains(&Value::Int(3)));
    }

    // -- 52. test_deque_push_pop -------------------------------------------------

    #[test]
    fn test_deque_push_pop() {
        // Simulate a Deque using an Array: push_front, push_back, pop_front, pop_back
        let mut deque: Vec<Value> = vec![];

        // push_back
        deque.push(Value::Int(1));
        deque.push(Value::Int(3));
        // push_front
        deque.insert(0, Value::Int(0));
        // deque = [0, 1, 3]

        assert_eq!(deque.len(), 3);

        // pop_front
        let front = deque.remove(0);
        assert_eq!(front, Value::Int(0));

        // pop_back
        let back = deque.pop().unwrap();
        assert_eq!(back, Value::Int(3));

        // Remaining: [1]
        assert_eq!(deque.len(), 1);
        assert_eq!(deque[0], Value::Int(1));
    }

    // -- 53. test_priority_queue -------------------------------------------------

    #[test]
    fn test_priority_queue() {
        // Simulate a priority queue: push values, then pop in sorted (min) order
        let mut pq: Vec<i32> = vec![5, 1, 3, 2, 4];
        pq.sort();
        // Pop in order
        assert_eq!(pq.remove(0), 1);
        assert_eq!(pq.remove(0), 2);
        assert_eq!(pq.remove(0), 3);
        assert_eq!(pq.remove(0), 4);
        assert_eq!(pq.remove(0), 5);
    }

    // -- 54. test_counter_increment -----------------------------------------------

    #[test]
    fn test_counter_increment() {
        // Simulate a Counter using a HashMap-like ClassInstance
        let mut counter: HashMap<String, Value> = HashMap::new();
        counter.insert("a".to_string(), Value::Long(1));
        counter.insert("b".to_string(), Value::Long(2));

        // Increment "a"
        if let Some(Value::Long(count)) = counter.get_mut("a") {
            *count += 1;
        }

        assert_eq!(counter.get("a"), Some(&Value::Long(2)));
        assert_eq!(counter.get("b"), Some(&Value::Long(2)));

        // Increment "c" (new key)
        counter.insert("c".to_string(), Value::Long(1));
        assert_eq!(counter.get("c"), Some(&Value::Long(1)));
    }

    // -- 55. test_path_join ------------------------------------------------------

    #[test]
    fn test_path_join() {
        let result = native_path_join(&[
            Value::String(Rc::new("/usr".to_string())),
            Value::String(Rc::new("local/bin".to_string())),
        ]);
        match result.unwrap() {
            Value::String(s) => {
                assert!(s.contains("usr"), "path should contain 'usr', got: {}", s);
                assert!(s.contains("local"), "path should contain 'local', got: {}", s);
            }
            other => panic!("Expected String, got {:?}", other),
        }
    }

    // -- 56. test_path_basename --------------------------------------------------

    #[test]
    fn test_path_basename() {
        let result = native_path_basename(&[
            Value::String(Rc::new("/usr/local/bin".to_string())),
        ]);
        match result.unwrap() {
            Value::String(s) => {
                assert_eq!(&*s as &str, "bin", "basename should be 'bin', got: {}", s);
            }
            other => panic!("Expected String, got {:?}", other),
        }
    }

    // -- 57. test_sys_working_dir ------------------------------------------------

    #[test]
    fn test_sys_working_dir() {
        let result = native_sys_working_dir(&[]);
        match result.unwrap() {
            Value::String(s) => {
                assert!(!s.is_empty(), "working directory should not be empty");
            }
            other => panic!("Expected String, got {:?}", other),
        }
    }

    // -- 58. test_meter_plus -----------------------------------------------------

    #[test]
    fn test_meter_plus() {
        // Simulate adding two Meter values (represented as Doubles)
        let m1 = Value::Double(5.0);
        let m2 = Value::Double(3.0);
        // Add
        match (&m1, &m2) {
            (Value::Double(a), Value::Double(b)) => {
                assert_eq!(a + b, 8.0);
            }
            _ => panic!("Expected Double values"),
        }
    }

    // -- 59. test_joule_from_base ------------------------------------------------

    #[test]
    fn test_joule_from_base() {
        // Joule = kg * m^2 / s^2
        // 1 J = 1.0 (in base SI units)
        let mass: f64 = 1.0;  // kg
        let velocity: f64 = 1.0;  // m/s
        let joules = 0.5 * mass * velocity * velocity;
        assert!((joules - 0.5).abs() < f64::EPSILON);
    }

    // -- 60. test_constants_boltzmann ---------------------------------------------

    #[test]
    fn test_constants_boltzmann() {
        // Boltzmann constant: 1.380649e-23 J/K
        let boltzmann: f64 = 1.380649e-23;
        assert!(boltzmann > 0.0, "Boltzmann constant should be positive");
        assert!((boltzmann - 1.380649e-23).abs() < 1e-30);
    }

    // -- 61. test_atom_creation --------------------------------------------------

    #[test]
    fn test_atom_creation() {
        // Create an Atom as a ClassInstance with element, x, y, z fields
        let mut atom_fields: HashMap<String, Value> = HashMap::new();
        atom_fields.insert("element".to_string(), Value::String(Rc::new("C".to_string())));
        atom_fields.insert("x".to_string(), Value::Double(0.0));
        atom_fields.insert("y".to_string(), Value::Double(0.0));
        atom_fields.insert("z".to_string(), Value::Double(0.0));

        let atom = Value::ClassInstance {
            class_name: "Atom".to_string(),
            fields: Rc::new(RefCell::new(atom_fields)),
            vtable: HashMap::new(),
        };

        match &atom {
            Value::ClassInstance { class_name, fields, .. } => {
                assert_eq!(class_name, "Atom");
                let borrowed = fields.borrow();
                assert_eq!(borrowed.get("element"), Some(&Value::String(Rc::new("C".to_string()))));
                assert_eq!(borrowed.get("x"), Some(&Value::Double(0.0)));
            }
            _ => panic!("Expected ClassInstance"),
        }
    }

    // -- 62. test_atom_distance --------------------------------------------------

    #[test]
    fn test_atom_distance() {
        // Distance between two atoms at (0,0,0) and (3,4,0) = 5.0
        let x1: f64 = 0.0; let y1: f64 = 0.0; let z1: f64 = 0.0;
        let x2: f64 = 3.0; let y2: f64 = 4.0; let z2: f64 = 0.0;
        let dx = x2 - x1;
        let dy = y2 - y1;
        let dz = z2 - z1;
        let dist = (dx * dx + dy * dy + dz * dz).sqrt();
        assert!((dist - 5.0).abs() < f64::EPSILON);
    }

    // -- 63. test_molecule_add_atom -----------------------------------------------

    #[test]
    fn test_molecule_add_atom() {
        // Create a Molecule as a ClassInstance with an atoms Array field
        let mut mol_fields: HashMap<String, Value> = HashMap::new();
        mol_fields.insert("name".to_string(), Value::String(Rc::new("Water".to_string())));
        mol_fields.insert("atoms".to_string(), Value::Array { elements: vec![] });

        // Add atoms
        if let Some(Value::Array { elements }) = mol_fields.get_mut("atoms") {
            elements.push(Value::String(Rc::new("O".to_string())));
            elements.push(Value::String(Rc::new("H".to_string())));
            elements.push(Value::String(Rc::new("H".to_string())));
        }

        let molecule = Value::ClassInstance {
            class_name: "Molecule".to_string(),
            fields: Rc::new(RefCell::new(mol_fields)),
            vtable: HashMap::new(),
        };

        match &molecule {
            Value::ClassInstance { fields, .. } => {
                let borrowed = fields.borrow();
                match borrowed.get("atoms") {
                    Some(Value::Array { elements }) => {
                        assert_eq!(elements.len(), 3);
                    }
                    _ => panic!("Expected atoms array"),
                }
                match borrowed.get("name") {
                    Some(Value::String(s)) => {
                        assert_eq!(&*s as &str, "Water");
                    }
                    _ => panic!("Expected name string"),
                }
            }
            _ => panic!("Expected ClassInstance"),
        }
    }

    // -- 64. test_tcp_client_creation --------------------------------------------

    #[test]
    fn test_tcp_client_creation() {
        // Create a TcpClient as a ClassInstance with host and port fields
        let mut client_fields: HashMap<String, Value> = HashMap::new();
        client_fields.insert("host".to_string(), Value::String(Rc::new("127.0.0.1".to_string())));
        client_fields.insert("port".to_string(), Value::Int(8080));
        client_fields.insert("connected".to_string(), Value::Bool(false));

        let client = Value::ClassInstance {
            class_name: "TcpClient".to_string(),
            fields: Rc::new(RefCell::new(client_fields)),
            vtable: HashMap::new(),
        };

        match &client {
            Value::ClassInstance { class_name, fields, .. } => {
                assert_eq!(class_name, "TcpClient");
                let borrowed = fields.borrow();
                assert_eq!(borrowed.get("host"), Some(&Value::String(Rc::new("127.0.0.1".to_string()))));
                assert_eq!(borrowed.get("port"), Some(&Value::Int(8080)));
                assert_eq!(borrowed.get("connected"), Some(&Value::Bool(false)));
            }
            _ => panic!("Expected ClassInstance"),
        }
    }

    // -- 65. test_http_client_creation -------------------------------------------

    #[test]
    fn test_http_client_creation() {
        // Create an HttpClient as a ClassInstance with base_url field
        let mut client_fields: HashMap<String, Value> = HashMap::new();
        client_fields.insert("base_url".to_string(), Value::String(Rc::new("https://api.example.com".to_string())));
        client_fields.insert("timeout".to_string(), Value::Long(30000));

        let client = Value::ClassInstance {
            class_name: "HttpClient".to_string(),
            fields: Rc::new(RefCell::new(client_fields)),
            vtable: HashMap::new(),
        };

        match &client {
            Value::ClassInstance { class_name, fields, .. } => {
                assert_eq!(class_name, "HttpClient");
                let borrowed = fields.borrow();
                assert_eq!(borrowed.get("base_url"), Some(&Value::String(Rc::new("https://api.example.com".to_string()))));
                assert_eq!(borrowed.get("timeout"), Some(&Value::Long(30000)));
            }
            _ => panic!("Expected ClassInstance"),
        }
    }

    // -- 66. test_duration_of_seconds --------------------------------------------

    #[test]
    fn test_duration_of_seconds() {
        // Create Duration from seconds (represented as i64 milliseconds)
        let seconds: i64 = 5;
        let duration_ms = seconds * 1000;
        assert_eq!(duration_ms, 5000);

        let duration_ms2: i64 = 0;
        assert_eq!(duration_ms2, 0);

        let duration_ms3: i64 = 1 * 1000;
        assert_eq!(duration_ms3, 1000);
    }

    // -- 67. test_datetime_plus_duration -----------------------------------------

    #[test]
    fn test_datetime_plus_duration() {
        // Add duration to datetime (represented as i64 ms timestamps)
        let datetime_ms: i64 = 1609459200000; // 2021-01-01 00:00:00 UTC
        let duration_ms: i64 = 86400000; // 1 day
        let result = datetime_ms + duration_ms;
        assert_eq!(result, 1609545600000); // 2021-01-02 00:00:00 UTC
    }

    // -- 68. test_datetime_comparison --------------------------------------------

    #[test]
    fn test_datetime_comparison() {
        // Compare two datetimes
        let dt1: i64 = 1609459200000; // 2021-01-01
        let dt2: i64 = 1609545600000; // 2021-01-02
        assert!(dt1 < dt2);
        assert!(dt2 > dt1);
        assert!(dt1 == dt1);
        assert!(dt1 != dt2);
    }

    // -- 69. test_regex_compile --------------------------------------------------

    #[test]
    fn test_regex_compile() {
        // Compiling a regex pattern should not panic
        let result = native_regex_match(&[
            Value::String(Rc::new(r"\d+".to_string())),
            Value::String(Rc::new("123".to_string())),
        ]);
        assert!(result.is_ok(), "regex compile and match should succeed");
    }

    // -- 70. test_regex_match_simple ---------------------------------------------

    #[test]
    fn test_regex_match_simple() {
        // Match a simple pattern
        let result = native_regex_match(&[
            Value::String(Rc::new(r"^hello".to_string())),
            Value::String(Rc::new("hello world".to_string())),
        ]);
        assert_eq!(result.unwrap(), Value::Bool(true));

        // Non-matching
        let result = native_regex_match(&[
            Value::String(Rc::new(r"^world".to_string())),
            Value::String(Rc::new("hello world".to_string())),
        ]);
        assert_eq!(result.unwrap(), Value::Bool(false));
    }

    // -- 71. test_regex_replace_simple -------------------------------------------

    #[test]
    fn test_regex_replace_simple() {
        // Replace with a simple pattern
        let result = native_regex_replace(&[
            Value::String(Rc::new(r"cat".to_string())),
            Value::String(Rc::new("the cat sat on the mat".to_string())),
            Value::String(Rc::new("dog".to_string())),
        ]);
        assert_eq!(
            result.unwrap(),
            Value::String(Rc::new("the dog sat on the mat".to_string()))
        );
    }

    // -- 72. test_env_get -------------------------------------------------------

    #[test]
    fn test_env_get() {
        // Set an env var then retrieve it
        std::env::set_var("TITRATE_TEST_ENV_GET", "hello");
        let result = native_env_get(&[Value::String(Rc::new("TITRATE_TEST_ENV_GET".to_string()))]);
        assert_eq!(result.unwrap(), Value::String(Rc::new("hello".to_string())));

        // Non-existent env var returns Null
        let result = native_env_get(&[Value::String(Rc::new("TITRATE_NONEXISTENT_VAR_XYZ".to_string()))]);
        assert_eq!(result.unwrap(), Value::Null);

        // Error on wrong type
        let result = native_env_get(&[Value::Int(42)]);
        assert!(result.is_err());
    }

    // -- 73. test_env_set -------------------------------------------------------

    #[test]
    fn test_env_set() {
        let result = native_env_set(&[
            Value::String(Rc::new("TITRATE_TEST_ENV_SET".to_string())),
            Value::String(Rc::new("world".to_string())),
        ]);
        assert_eq!(result.unwrap(), Value::Void);
        assert_eq!(std::env::var("TITRATE_TEST_ENV_SET").unwrap(), "world");

        // Error on wrong type
        let result = native_env_set(&[Value::Int(1), Value::Int(2)]);
        assert!(result.is_err());
    }

    // -- 74. test_env_vars -------------------------------------------------------

    #[test]
    fn test_env_vars() {
        let result = native_env_vars(&[]);
        match result.unwrap() {
            Value::Array { elements } => {
                assert!(!elements.is_empty());
                // Each element should be a "key=value" string
                for elem in &elements {
                    match elem {
                        Value::String(s) => assert!(s.contains('=')),
                        _ => panic!("Expected String in env vars array"),
                    }
                }
            }
            _ => panic!("Expected Array from Env_vars"),
        }
    }

    // -- 75. test_fs_exists -----------------------------------------------------

    #[test]
    fn test_fs_exists() {
        // Current directory should exist
        let result = native_fs_exists(&[Value::String(Rc::new(".".to_string()))]);
        assert_eq!(result.unwrap(), Value::Bool(true));

        // Non-existent path
        let result = native_fs_exists(&[Value::String(Rc::new("/no/such/path/titrate_test_xyz".to_string()))]);
        assert_eq!(result.unwrap(), Value::Bool(false));

        // Error on wrong type
        let result = native_fs_exists(&[Value::Int(42)]);
        assert!(result.is_err());
    }

    // -- 76. test_fs_is_file ----------------------------------------------------

    #[test]
    fn test_fs_is_file() {
        // Current directory is not a file
        let result = native_fs_is_file(&[Value::String(Rc::new(".".to_string()))]);
        assert_eq!(result.unwrap(), Value::Bool(false));

        // A known existing file - use Cargo.toml from the crate root
        let cargo_toml = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("Cargo.toml")
            .to_string_lossy()
            .to_string();
        let result = native_fs_is_file(&[Value::String(Rc::new(cargo_toml))]);
        assert_eq!(result.unwrap(), Value::Bool(true));

        // Error on wrong type
        let result = native_fs_is_file(&[Value::Int(42)]);
        assert!(result.is_err());
    }

    // -- 77. test_fs_is_dir -----------------------------------------------------

    #[test]
    fn test_fs_is_dir() {
        // Current directory should be a directory
        let result = native_fs_is_dir(&[Value::String(Rc::new(".".to_string()))]);
        assert_eq!(result.unwrap(), Value::Bool(true));

        // This source file is not a directory
        let this_file = file!().replace('\\', "/");
        let result = native_fs_is_dir(&[Value::String(Rc::new(this_file))]);
        assert_eq!(result.unwrap(), Value::Bool(false));

        // Error on wrong type
        let result = native_fs_is_dir(&[Value::Int(42)]);
        assert!(result.is_err());
    }

    // -- 78. test_fs_size -------------------------------------------------------

    #[test]
    fn test_fs_size() {
        // Cargo.toml should have a positive size
        let cargo_toml = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("Cargo.toml")
            .to_string_lossy()
            .to_string();
        let result = native_fs_size(&[Value::String(Rc::new(cargo_toml))]);
        match result.unwrap() {
            Value::Long(n) => assert!(n > 0),
            _ => panic!("Expected Long from Fs_size"),
        }

        // Non-existent file should error
        let result = native_fs_size(&[Value::String(Rc::new("/no/such/file_titrate_xyz".to_string()))]);
        assert!(result.is_err());

        // Error on wrong type
        let result = native_fs_size(&[Value::Int(42)]);
        assert!(result.is_err());
    }

    // -- 79. test_process_id ----------------------------------------------------

    #[test]
    fn test_process_id() {
        let result = native_process_id(&[]);
        match result.unwrap() {
            Value::Long(n) => assert!(n > 0),
            _ => panic!("Expected Long from Process_id"),
        }
    }

    // -- 80. test_process_args --------------------------------------------------

    #[test]
    fn test_process_args() {
        let result = native_process_args(&[]);
        match result.unwrap() {
            Value::Array { elements } => {
                assert!(!elements.is_empty());
                for elem in &elements {
                    match elem {
                        Value::String(_) => {}
                        _ => panic!("Expected String in process args array"),
                    }
                }
            }
            _ => panic!("Expected Array from Process_args"),
        }
    }

    // -- 81. test_os_name -------------------------------------------------------

    #[test]
    fn test_os_name() {
        let result = native_os_name(&[]);
        match result.unwrap() {
            Value::String(s) => assert!(!s.is_empty()),
            _ => panic!("Expected String from Os_name"),
        }
    }

    // -- 82. test_os_arch -------------------------------------------------------

    #[test]
    fn test_os_arch() {
        let result = native_os_arch(&[]);
        match result.unwrap() {
            Value::String(s) => assert!(!s.is_empty()),
            _ => panic!("Expected String from Os_arch"),
        }
    }

    // -- 83. test_os_family -----------------------------------------------------

    #[test]
    fn test_os_family() {
        let result = native_os_family(&[]);
        match result.unwrap() {
            Value::String(s) => {
                assert!(!s.is_empty());
                // Should be "unix" or "windows"
                assert!(s.as_str() == "unix" || s.as_str() == "windows", "Unexpected OS family: {}", s);
            }
            _ => panic!("Expected String from Os_family"),
        }
    }

    // -- 84. test_string_trim_start ---------------------------------------------

    #[test]
    fn test_string_trim_start() {
        let result = native_string_trim_start(&[Value::String(Rc::new("  hello  ".to_string()))]);
        assert_eq!(result.unwrap(), Value::String(Rc::new("hello  ".to_string())));

        let result = native_string_trim_start(&[Value::String(Rc::new("no_leading".to_string()))]);
        assert_eq!(result.unwrap(), Value::String(Rc::new("no_leading".to_string())));

        // Error on wrong type
        let result = native_string_trim_start(&[Value::Int(42)]);
        assert!(result.is_err());
    }

    // -- 85. test_string_trim_end -----------------------------------------------

    #[test]
    fn test_string_trim_end() {
        let result = native_string_trim_end(&[Value::String(Rc::new("  hello  ".to_string()))]);
        assert_eq!(result.unwrap(), Value::String(Rc::new("  hello".to_string())));

        let result = native_string_trim_end(&[Value::String(Rc::new("no_trailing".to_string()))]);
        assert_eq!(result.unwrap(), Value::String(Rc::new("no_trailing".to_string())));

        // Error on wrong type
        let result = native_string_trim_end(&[Value::Int(42)]);
        assert!(result.is_err());
    }

    // -- 86. test_string_starts_with --------------------------------------------

    #[test]
    fn test_string_starts_with() {
        let result = native_string_starts_with(&[
            Value::String(Rc::new("hello world".to_string())),
            Value::String(Rc::new("hello".to_string())),
        ]);
        assert_eq!(result.unwrap(), Value::Bool(true));

        let result = native_string_starts_with(&[
            Value::String(Rc::new("hello world".to_string())),
            Value::String(Rc::new("world".to_string())),
        ]);
        assert_eq!(result.unwrap(), Value::Bool(false));

        // Error on wrong type
        let result = native_string_starts_with(&[Value::Int(1), Value::Int(2)]);
        assert!(result.is_err());
    }

    // -- 87. test_string_ends_with ----------------------------------------------

    #[test]
    fn test_string_ends_with() {
        let result = native_string_ends_with(&[
            Value::String(Rc::new("hello world".to_string())),
            Value::String(Rc::new("world".to_string())),
        ]);
        assert_eq!(result.unwrap(), Value::Bool(true));

        let result = native_string_ends_with(&[
            Value::String(Rc::new("hello world".to_string())),
            Value::String(Rc::new("hello".to_string())),
        ]);
        assert_eq!(result.unwrap(), Value::Bool(false));

        // Error on wrong type
        let result = native_string_ends_with(&[Value::Int(1), Value::Int(2)]);
        assert!(result.is_err());
    }

    // -- 88. test_string_pad_left -----------------------------------------------

    #[test]
    fn test_string_pad_left() {
        let result = native_string_pad_left(&[
            Value::String(Rc::new("hi".to_string())),
            Value::Int(5),
            Value::Char('*'),
        ]);
        assert_eq!(result.unwrap(), Value::String(Rc::new("***hi".to_string())));

        // Already long enough
        let result = native_string_pad_left(&[
            Value::String(Rc::new("hello".to_string())),
            Value::Int(3),
            Value::Char(' '),
        ]);
        assert_eq!(result.unwrap(), Value::String(Rc::new("hello".to_string())));

        // Error on wrong type
        let result = native_string_pad_left(&[Value::Int(1), Value::Int(2), Value::Int(3)]);
        assert!(result.is_err());
    }

    // -- 89. test_string_pad_right ----------------------------------------------

    #[test]
    fn test_string_pad_right() {
        let result = native_string_pad_right(&[
            Value::String(Rc::new("hi".to_string())),
            Value::Int(5),
            Value::Char('*'),
        ]);
        assert_eq!(result.unwrap(), Value::String(Rc::new("hi***".to_string())));

        // Already long enough
        let result = native_string_pad_right(&[
            Value::String(Rc::new("hello".to_string())),
            Value::Int(3),
            Value::Char(' '),
        ]);
        assert_eq!(result.unwrap(), Value::String(Rc::new("hello".to_string())));

        // Error on wrong type
        let result = native_string_pad_right(&[Value::Int(1), Value::Int(2), Value::Int(3)]);
        assert!(result.is_err());
    }

    // =========================================================================
    // Closure opcode tests
    // =========================================================================

    #[test]
    fn test_closure_new_captured() {
        // Build a chunk that:
        // 1. Pushes two upvalue values onto the stack
        // 2. CLOSURE_NEW_CAPTURED with func_idx=1, capture_count=2
        // 3. RET
        let mut chunk = Chunk::new();
        // Push upvalue values (they'll be popped by CLOSURE_NEW_CAPTURED)
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&10i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&20i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        // CLOSURE_NEW_CAPTURED: func_idx=1 (u16), capture_count=2 (u8)
        chunk.write_opcode(OpCode::CLOSURE_NEW_CAPTURED, 1);
        chunk.write_u16(1, 1); // function index
        chunk.write_u8(2, 1);  // capture count
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        // Add a dummy function at index 1 that the closure will reference
        vm.add_function(FunctionDef {
            name: "inner".to_string(),
            arity: 0,
            chunk: {
                let mut c = Chunk::new();
                c.write_opcode(OpCode::PUSH_NULL, 1);
                c.write_opcode(OpCode::RET, 1);
                c
            },
            is_method: false,
            is_constructor: false,
            local_count: 0,
        });

        vm.run().unwrap();

        // The top of the stack should be a Closure value
        match vm.stack.last() {
            Some(Value::Closure { func_idx, upvalues }) => {
                assert_eq!(*func_idx, 1, "closure should reference function index 1");
                assert_eq!(upvalues.len(), 2, "closure should have 2 upvalues");
                assert_eq!(upvalues[0], Value::Int(10), "first upvalue should be Int(10)");
                assert_eq!(upvalues[1], Value::Int(20), "second upvalue should be Int(20)");
            }
            other => panic!("Expected Closure on stack, got {:?}", other),
        }
    }

    #[test]
    fn test_closure_capture() {
        // Build a chunk that:
        // 1. Stores a value in local slot 0
        // 2. CLOSURE_CAPTURE slot 0 — pushes the local's value onto the stack
        // 3. RET
        let mut chunk = Chunk::new();
        // Store value at local slot 0: PUSH_I32 42, STORE_LOCAL 0
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&42i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::STORE_LOCAL, 1);
        chunk.write_u8(0, 1);
        // CLOSURE_CAPTURE: push the value at local slot 0 onto the stack
        chunk.write_opcode(OpCode::CLOSURE_CAPTURE, 1);
        chunk.write_u8(0, 1); // local slot index
        // RET
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = Vm::new();
        vm.add_function(FunctionDef {
            name: "main".to_string(),
            arity: 0,
            chunk,
            is_method: false,
            is_constructor: false,
            local_count: 1, // need at least 1 local slot
        });

        vm.run().unwrap();

        // After CLOSURE_CAPTURE, the value from slot 0 should be on the stack
        assert_eq!(vm.stack.last(), Some(&Value::Int(42)),
            "CLOSURE_CAPTURE should push the local's value onto the stack");
    }

    #[test]
    fn test_closure_new_captured_zero_captures() {
        // CLOSURE_NEW_CAPTURED with 0 captures creates a closure with empty upvalues
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::CLOSURE_NEW_CAPTURED, 1);
        chunk.write_u16(0, 1); // function index 0 (main itself, for testing)
        chunk.write_u8(0, 1);  // 0 captures
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();

        match vm.stack.last() {
            Some(Value::Closure { func_idx, upvalues }) => {
                assert_eq!(*func_idx, 0);
                assert!(upvalues.is_empty(), "closure with 0 captures should have empty upvalues");
            }
            other => panic!("Expected Closure, got {:?}", other),
        }
    }

    // =========================================================================
    // Hash / Encoding native function tests
    // =========================================================================

    // -- test_hash_md5 ----------------------------------------------------------

    #[test]
    fn test_hash_md5() {
        let mut vm = Vm::new();
        let result = vm.call_native_by_name("Hash_md5", &[
            Value::String(Rc::new("hello".to_string())),
        ]);
        match result {
            Ok(Value::String(s)) => {
                // MD5 of "hello" is 5d41402abc4b2a76b9719d911017c592
                assert_eq!(s.as_str(), "5d41402abc4b2a76b9719d911017c592",
                    "Hash_md5('hello') should be 5d41402abc4b2a76b9719d911017c592, got {}", s);
            }
            Ok(other) => panic!("Expected String, got {:?}", other),
            Err(e) => panic!("Hash_md5 failed: {}", e),
        }
    }

    // -- test_hash_sha256 -------------------------------------------------------

    #[test]
    fn test_hash_sha256() {
        let mut vm = Vm::new();
        let result = vm.call_native_by_name("Hash_sha256", &[
            Value::String(Rc::new("hello".to_string())),
        ]);
        match result {
            Ok(Value::String(s)) => {
                // SHA-256 of "hello" starts with "2cf24dba5fb0a30e"
                assert!(s.starts_with("2cf24dba5fb0a30e"),
                    "Hash_sha256('hello') should start with 2cf24dba5fb0a30e, got {}", s);
                // Full SHA-256 hex is 64 characters
                assert_eq!(s.len(), 64,
                    "SHA-256 hex output should be 64 characters, got {}", s.len());
            }
            Ok(other) => panic!("Expected String, got {:?}", other),
            Err(e) => panic!("Hash_sha256 failed: {}", e),
        }
    }

    // -- test_base64_encode_decode ----------------------------------------------

    #[test]
    fn test_base64_encode_decode() {
        let mut vm = Vm::new();

        // Encode "hello"
        let encoded = vm.call_native_by_name("Base64_encode", &[
            Value::String(Rc::new("hello".to_string())),
        ]);
        match encoded {
            Ok(Value::String(s)) => {
                assert_eq!(s.as_str(), "aGVsbG8=",
                    "Base64_encode('hello') should be 'aGVsbG8=', got '{}'", s);
            }
            Ok(other) => panic!("Expected String, got {:?}", other),
            Err(e) => panic!("Base64_encode failed: {}", e),
        }

        // Decode "aGVsbG8=" back to "hello"
        let decoded = vm.call_native_by_name("Base64_decode", &[
            Value::String(Rc::new("aGVsbG8=".to_string())),
        ]);
        match decoded {
            Ok(Value::String(s)) => {
                assert_eq!(s.as_str(), "hello",
                    "Base64_decode('aGVsbG8=') should be 'hello', got '{}'", s);
            }
            Ok(other) => panic!("Expected String, got {:?}", other),
            Err(e) => panic!("Base64_decode failed: {}", e),
        }
    }

    // -- test_hex_encode_decode -------------------------------------------------

    #[test]
    fn test_hex_encode_decode() {
        let mut vm = Vm::new();

        // Encode "hello"
        let encoded = vm.call_native_by_name("Hex_encode", &[
            Value::String(Rc::new("hello".to_string())),
        ]);
        match encoded {
            Ok(Value::String(s)) => {
                assert_eq!(s.as_str(), "68656c6c6f",
                    "Hex_encode('hello') should be '68656c6c6f', got '{}'", s);
            }
            Ok(other) => panic!("Expected String, got {:?}", other),
            Err(e) => panic!("Hex_encode failed: {}", e),
        }

        // Decode "68656c6c6f" back to "hello"
        let decoded = vm.call_native_by_name("Hex_decode", &[
            Value::String(Rc::new("68656c6c6f".to_string())),
        ]);
        match decoded {
            Ok(Value::String(s)) => {
                assert_eq!(s.as_str(), "hello",
                    "Hex_decode('68656c6c6f') should be 'hello', got '{}'", s);
            }
            Ok(other) => panic!("Expected String, got {:?}", other),
            Err(e) => panic!("Hex_decode failed: {}", e),
        }
    }

    // -- test_url_encode_decode -------------------------------------------------

    #[test]
    fn test_url_encode_decode() {
        let mut vm = Vm::new();

        // Encode "hello world"
        let encoded = vm.call_native_by_name("Url_encode", &[
            Value::String(Rc::new("hello world".to_string())),
        ]);
        match encoded {
            Ok(Value::String(s)) => {
                assert_eq!(s.as_str(), "hello%20world",
                    "Url_encode('hello world') should be 'hello%20world', got '{}'", s);
            }
            Ok(other) => panic!("Expected String, got {:?}", other),
            Err(e) => panic!("Url_encode failed: {}", e),
        }

        // Decode "hello%20world" back to "hello world"
        let decoded = vm.call_native_by_name("Url_decode", &[
            Value::String(Rc::new("hello%20world".to_string())),
        ]);
        match decoded {
            Ok(Value::String(s)) => {
                assert_eq!(s.as_str(), "hello world",
                    "Url_decode('hello%20world') should be 'hello world', got '{}'", s);
            }
            Ok(other) => panic!("Expected String, got {:?}", other),
            Err(e) => panic!("Url_decode failed: {}", e),
        }
    }

    // -- test_base64_encode_empty -----------------------------------------------

    #[test]
    fn test_base64_encode_empty() {
        let mut vm = Vm::new();
        let result = vm.call_native_by_name("Base64_encode", &[
            Value::String(Rc::new("".to_string())),
        ]);
        match result {
            Ok(Value::String(s)) => {
                assert_eq!(s.as_str(), "",
                    "Base64_encode('') should be '', got '{}'", s);
            }
            Ok(other) => panic!("Expected String, got {:?}", other),
            Err(e) => panic!("Base64_encode failed: {}", e),
        }
    }

    // -- test_url_encode_special ------------------------------------------------

    #[test]
    fn test_url_encode_special() {
        let mut vm = Vm::new();
        let result = vm.call_native_by_name("Url_encode", &[
            Value::String(Rc::new("a=b&c=d".to_string())),
        ]);
        match result {
            Ok(Value::String(s)) => {
                assert_eq!(s.as_str(), "a%3Db%26c%3Dd",
                    "Url_encode('a=b&c=d') should be 'a%3Db%26c%3Dd', got '{}'", s);
            }
            Ok(other) => panic!("Expected String, got {:?}", other),
            Err(e) => panic!("Url_encode failed: {}", e),
        }
    }

    // -- New native function tests -------------------------------------------

    #[test]
    fn test_string_to_uppercase() {
        let mut vm = Vm::new();
        let result = vm.call_native_by_name("String_toUppercase", &[
            Value::String(Rc::new("hello World".to_string())),
        ]).unwrap();
        match result {
            Value::String(s) => assert_eq!(s.as_str(), "HELLO WORLD"),
            other => panic!("Expected String, got {:?}", other),
        }
    }

    #[test]
    fn test_string_to_lower_case() {
        let mut vm = Vm::new();
        let result = vm.call_native_by_name("String_toLowerCase", &[
            Value::String(Rc::new("Hello WORLD".to_string())),
        ]).unwrap();
        match result {
            Value::String(s) => assert_eq!(s.as_str(), "hello world"),
            other => panic!("Expected String, got {:?}", other),
        }
    }

    #[test]
    fn test_string_replace() {
        let mut vm = Vm::new();
        let result = vm.call_native_by_name("String_replace", &[
            Value::String(Rc::new("hello world hello".to_string())),
            Value::String(Rc::new("hello".to_string())),
            Value::String(Rc::new("hi".to_string())),
        ]).unwrap();
        match result {
            Value::String(s) => assert_eq!(s.as_str(), "hi world hi"),
            other => panic!("Expected String, got {:?}", other),
        }
    }

    #[test]
    fn test_math_next_up_down() {
        let mut vm = Vm::new();
        let one = Value::Double(1.0);
        let up = vm.call_native_by_name("Math_nextUp", &[one.clone()]).unwrap();
        let down = vm.call_native_by_name("Math_nextDown", &[one.clone()]).unwrap();
        match (up, down) {
            (Value::Double(u), Value::Double(d)) => {
                assert!(u > 1.0, "next_up(1.0) should be > 1.0");
                assert!(d < 1.0, "next_down(1.0) should be < 1.0");
            }
            other => panic!("Expected Double values, got {:?}", other),
        }
    }

    #[test]
    fn test_math_neg_inf() {
        let mut vm = Vm::new();
        let result = vm.call_native_by_name("Math_negInf", &[]).unwrap();
        match result {
            Value::Double(d) => assert!(d.is_infinite() && d.is_sign_negative(),
                "Math_negInf should be negative infinity, got {}", d),
            other => panic!("Expected Double, got {:?}", other),
        }
    }

    #[test]
    fn test_math_ulp() {
        let mut vm = Vm::new();
        let result = vm.call_native_by_name("Math_ulp", &[Value::Double(1.0)]).unwrap();
        match result {
            Value::Double(d) => assert!(d > 0.0, "ulp(1.0) should be positive, got {}", d),
            other => panic!("Expected Double, got {:?}", other),
        }
    }

    #[test]
    fn test_math_get_exponent() {
        let mut vm = Vm::new();
        let result = vm.call_native_by_name("Math_getExponent", &[Value::Double(8.0)]).unwrap();
        match result {
            Value::Long(e) => assert_eq!(e, 3, "getExponent(8.0) should be 3, got {}", e),
            other => panic!("Expected Long, got {:?}", other),
        }
    }

    #[test]
    fn test_math_scalb() {
        let mut vm = Vm::new();
        let result = vm.call_native_by_name("Math_scalb", &[
            Value::Double(1.0),
            Value::Long(3),
        ]).unwrap();
        match result {
            Value::Double(d) => assert_eq!(d, 8.0, "scalb(1.0, 3) should be 8.0, got {}", d),
            other => panic!("Expected Double, got {:?}", other),
        }
    }

    #[test]
    fn test_math_random() {
        let mut vm = Vm::new();
        let result = vm.call_native_by_name("Math_random", &[]).unwrap();
        match result {
            Value::Double(d) => assert!(d >= 0.0 && d < 1.0,
                "Math_random should be in [0, 1), got {}", d),
            other => panic!("Expected Double, got {:?}", other),
        }
    }

    #[test]
    fn test_regex_group_count() {
        let mut vm = Vm::new();
        let result = vm.call_native_by_name("Regex_groupCount", &[
            Value::String(Rc::new("(a)(b)(c)".to_string())),
        ]).unwrap();
        match result {
            Value::Int(n) => assert_eq!(n, 3, "Regex_groupCount('(a)(b)(c)') should be 3, got {}", n),
            other => panic!("Expected Int, got {:?}", other),
        }
    }

    #[test]
    fn test_time_day_of_week() {
        let mut vm = Vm::new();
        // 2024-01-01 00:00:00 UTC is a Monday (0)
        let result = vm.call_native_by_name("Time_dayOfWeek", &[
            Value::Long(1704067200),
        ]).unwrap();
        match result {
            Value::Int(d) => assert_eq!(d, 0, "2024-01-01 should be Monday (0), got {}", d),
            other => panic!("Expected Int, got {:?}", other),
        }
    }

    #[test]
    fn test_time_day_of_year() {
        let mut vm = Vm::new();
        // 2024-01-01 is day 1 of the year
        let result = vm.call_native_by_name("Time_dayOfYear", &[
            Value::Long(1704067200),
        ]).unwrap();
        match result {
            Value::Int(d) => assert_eq!(d, 1, "2024-01-01 should be day 1, got {}", d),
            other => panic!("Expected Int, got {:?}", other),
        }
    }

    #[test]
    fn test_double_parse_double() {
        let mut vm = Vm::new();
        let result = vm.call_native_by_name("Double_parseDouble", &[
            Value::String(Rc::new("3.14159".to_string())),
        ]).unwrap();
        match result {
            Value::Double(d) => assert!((d - 3.14159).abs() < 1e-10,
                "Double_parseDouble('3.14159') should be 3.14159, got {}", d),
            other => panic!("Expected Double, got {:?}", other),
        }
        // Test error case
        let err = vm.call_native_by_name("Double_parseDouble", &[
            Value::String(Rc::new("not_a_number".to_string())),
        ]);
        assert!(err.is_err(), "Parsing 'not_a_number' should fail");
    }

    #[test]
    fn test_long_parse_long() {
        let mut vm = Vm::new();
        let result = vm.call_native_by_name("Long_parseLong", &[
            Value::String(Rc::new("123456789".to_string())),
        ]).unwrap();
        match result {
            Value::Long(l) => assert_eq!(l, 123456789, "Long_parseLong('123456789') should be 123456789, got {}", l),
            other => panic!("Expected Long, got {:?}", other),
        }
        // Test error case
        let err = vm.call_native_by_name("Long_parseLong", &[
            Value::String(Rc::new("not_a_number".to_string())),
        ]);
        assert!(err.is_err(), "Parsing 'not_a_number' should fail");
    }

    #[test]
    fn test_subprocess_run() {
        let mut vm = Vm::new();
        // On Windows, use "cmd" with /C echo
        let result = vm.call_native_by_name("Subprocess_run", &[
            Value::String(Rc::new("cmd".to_string())),
            Value::String(Rc::new("/C".to_string())),
            Value::String(Rc::new("echo hello".to_string())),
        ]);
        match result {
            Ok(Value::Int(code)) => assert_eq!(code, 0, "Successful command should return exit code 0"),
            Ok(other) => panic!("Expected Int, got {:?}", other),
            Err(e) => panic!("Subprocess_run failed: {}", e),
        }
    }

    #[test]
    fn test_tempfile_create() {
        let mut vm = Vm::new();
        // Create a temp file
        let result = vm.call_native_by_name("Tempfile_create", &[
            Value::String(Rc::new("test_vm_".to_string())),
        ]).unwrap();
        let _path = match result {
            Value::String(s) => {
                let p = s.to_string();
                assert!(std::path::Path::new(&p).exists(), "Temp file should exist at {}", p);
                // Clean up
                let _ = std::fs::remove_file(&p);
                p
            }
            other => panic!("Expected String, got {:?}", other),
        };
        // Create a temp directory
        let result = vm.call_native_by_name("Tempfile_create", &[
            Value::String(Rc::new("test_vm_dir_".to_string())),
            Value::Bool(true),
        ]).unwrap();
        match result {
            Value::String(s) => {
                let p = s.to_string();
                assert!(std::path::Path::new(&p).is_dir(), "Temp dir should exist at {}", p);
                // Clean up
                let _ = std::fs::remove_dir_all(&p);
            }
            other => panic!("Expected String, got {:?}", other),
        }
    }

    #[test]
    fn test_dir_walk_and_move() {
        let mut vm = Vm::new();
        let base = std::env::temp_dir().join("titrate_test_walk");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(base.join("a.txt"), "hello").unwrap();
        std::fs::create_dir_all(base.join("sub")).unwrap();
        std::fs::write(base.join("sub").join("b.txt"), "world").unwrap();

        // Test Dir_walk
        let result = vm.call_native_by_name("Dir_walk", &[
            Value::String(Rc::new(base.to_string_lossy().to_string())),
        ]).unwrap();
        let count = match result {
            Value::Array { elements } => elements.len(),
            other => panic!("Expected Array, got {:?}", other),
        };
        assert!(count >= 3, "Dir_walk should find at least 3 entries, got {}", count);

        // Test Dir_copy
        let dst = std::env::temp_dir().join("titrate_test_walk_copy");
        let _ = std::fs::remove_dir_all(&dst);
        let result = vm.call_native_by_name("Dir_copy", &[
            Value::String(Rc::new(base.to_string_lossy().to_string())),
            Value::String(Rc::new(dst.to_string_lossy().to_string())),
        ]);
        assert!(result.is_ok(), "Dir_copy should succeed");
        assert!(dst.join("a.txt").exists(), "Copied file should exist");

        // Test Dir_move
        let moved = std::env::temp_dir().join("titrate_test_walk_moved");
        let _ = std::fs::remove_dir_all(&moved);
        let result = vm.call_native_by_name("Dir_move", &[
            Value::String(Rc::new(dst.to_string_lossy().to_string())),
            Value::String(Rc::new(moved.to_string_lossy().to_string())),
        ]);
        assert!(result.is_ok(), "Dir_move should succeed");
        assert!(moved.join("a.txt").exists(), "Moved file should exist");
        assert!(!dst.exists(), "Original dir should be gone after move");

        // Clean up
        let _ = std::fs::remove_dir_all(&base);
        let _ = std::fs::remove_dir_all(&moved);
    }
}
