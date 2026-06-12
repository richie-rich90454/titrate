// Titrate Alpha 0.2 – bytecode virtual machine
// Precision in every step – richie-rich90454, 2026

use std::collections::HashMap;

use super::frame::{ClassDef, EnumDef, Frame, FunctionDef};
use super::chunk::Chunk;
use super::value::{NativeFn, Value};

mod step;
mod call;
mod operators;
mod object;
mod cast;
pub mod natives;
#[cfg(test)]
mod tests;

// Re-export lookup_builtin_native so mod.rs can call it from load_program
pub(crate) use natives::lookup_builtin_native;

// ---------------------------------------------------------------------------
// Virtual machine
// ---------------------------------------------------------------------------

pub struct Vm {
    /// Value stack
    pub(super) stack: Vec<Value>,
    /// Call frame stack
    pub(super) frames: Vec<Frame>,
    /// Function table (index 0 = top-level/main chunk)
    pub(super) functions: Vec<FunctionDef>,
    /// Class table
    pub(super) classes: Vec<ClassDef>,
    /// Enum table
    pub(super) enums: Vec<EnumDef>,
    /// Native function table
    pub(super) natives: Vec<NativeFn>,
    /// Native function name → index mapping
    pub(super) native_names: HashMap<String, u16>,
    /// Heap memory for references/regions
    pub(super) heap: Vec<Value>,
    /// Region stack for scoped allocation
    pub(super) region_stack: Vec<Vec<usize>>,
    /// Captured output
    pub output: Vec<String>,
    /// Working directory for resolving relative file paths
    pub(super) working_dir: Option<std::path::PathBuf>,
    /// Maximum call depth to prevent stack overflow (default 10000)
    pub(super) max_call_depth: usize,
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
            max_call_depth: 10000,
        };

        // Register built-in native functions
        vm.register_native("println", natives::builtins::native_println);
        vm.register_native("toString", natives::builtins::native_to_string);
        vm.register_native("parseInt", natives::builtins::native_parse_int);
        vm.register_native("Ok", natives::builtins::native_ok);
        vm.register_native("Err", natives::builtins::native_err);
        vm.register_native("File_readFile", natives::file::native_file_read);
        vm.register_native("File_writeFile", natives::file::native_file_write);
        vm.register_native("File_readLines", natives::file::native_file_read_lines);
        vm.register_native("File_open", natives::file::native_file_open);
        vm.register_native("File_readLine", natives::file::native_file_read_line);
        vm.register_native("File_write", natives::file::native_file_write_content);
        vm.register_native("File_close", natives::file::native_file_close);
        vm.register_native("String_split", natives::builtins::native_string_split);
        vm.register_native("Integer_parseOr", natives::builtins::native_integer_parse_or);
        vm.register_native("String_trim", natives::builtins::native_string_trim);
        vm.register_native("String_length", natives::builtins::native_string_length);

        // Path natives
        vm.register_native("Path_join", natives::path::native_path_join);
        vm.register_native("Path_exists", natives::path::native_path_exists);
        vm.register_native("Path_isFile", natives::path::native_path_is_file);
        vm.register_native("Path_isDir", natives::path::native_path_is_dir);
        vm.register_native("Path_basename", natives::path::native_path_basename);
        vm.register_native("Path_dirname", natives::path::native_path_dirname);
        vm.register_native("Path_extension", natives::path::native_path_extension);

        // Directory natives
        vm.register_native("Dir_list", natives::directory::native_dir_list);
        vm.register_native("Dir_create", natives::directory::native_dir_create);
        vm.register_native("Dir_remove", natives::directory::native_dir_remove);

        // Sys natives
        vm.register_native("Sys_args", natives::system::native_sys_args);
        vm.register_native("Sys_env", natives::system::native_sys_env);
        vm.register_native("Sys_setEnv", natives::system::native_sys_set_env);
        vm.register_native("Sys_exit", natives::system::native_sys_exit);
        vm.register_native("Sys_workingDir", natives::system::native_sys_working_dir);
        vm.register_native("Sys_sleep", natives::system::native_sys_sleep);

        // Network natives
        vm.register_native("Net_connect", natives::net::native_net_connect);
        vm.register_native("Net_send", natives::net::native_net_send);
        vm.register_native("Net_receive", natives::net::native_net_receive);
        vm.register_native("Net_bind", natives::net::native_net_bind);
        vm.register_native("Net_accept", natives::net::native_net_accept);
        vm.register_native("Net_close", natives::net::native_net_close);
        vm.register_native("Http_get", natives::net::native_http_get);
        vm.register_native("Http_post", natives::net::native_http_post);
        vm.register_native("Http_put", natives::net::native_http_put);
        vm.register_native("Http_delete", natives::net::native_http_delete);
        vm.register_native("Http_patch", natives::net::native_http_patch);
        vm.register_native("Http_head", natives::net::native_http_head);

        // Time natives
        vm.register_native("Time_now", natives::time::native_time_now);
        vm.register_native("Time_sleep", natives::time::native_time_sleep);
        vm.register_native("Time_format", natives::time::native_time_format);
        vm.register_native("Time_getYear", natives::time::native_time_get_year);
        vm.register_native("Time_getMonth", natives::time::native_time_get_month);
        vm.register_native("Time_getDay", natives::time::native_time_get_day);
        vm.register_native("Time_getHour", natives::time::native_time_get_hour);
        vm.register_native("Time_getMinute", natives::time::native_time_get_minute);
        vm.register_native("Time_getSecond", natives::time::native_time_get_second);

        // Regex natives
        vm.register_native("Regex_match", natives::regex::native_regex_match);
        vm.register_native("Regex_find", natives::regex::native_regex_find);
        vm.register_native("Regex_replace", natives::regex::native_regex_replace);

        // Math natives
        vm.register_native("Math_sin", natives::math::native_math_sin);
        vm.register_native("Math_cos", natives::math::native_math_cos);
        vm.register_native("Math_tan", natives::math::native_math_tan);
        vm.register_native("Math_asin", natives::math::native_math_asin);
        vm.register_native("Math_acos", natives::math::native_math_acos);
        vm.register_native("Math_atan", natives::math::native_math_atan);
        vm.register_native("Math_atan2", natives::math::native_math_atan2);
        vm.register_native("Math_ln", natives::math::native_math_ln);
        vm.register_native("Math_log10", natives::math::native_math_log10);
        vm.register_native("Math_log2", natives::math::native_math_log2);
        vm.register_native("Math_exp", natives::math::native_math_exp);
        vm.register_native("Math_pow", natives::math::native_math_pow);
        vm.register_native("Math_sqrt", natives::math::native_math_sqrt);
        vm.register_native("Math_cbrt", natives::math::native_math_cbrt);
        vm.register_native("Math_abs", natives::math::native_math_abs);
        vm.register_native("Math_absInt", natives::math::native_math_abs_int);
        vm.register_native("Math_floor", natives::math::native_math_floor);
        vm.register_native("Math_ceil", natives::math::native_math_ceil);
        vm.register_native("Math_round", natives::math::native_math_round);
        vm.register_native("Math_inf", natives::math::native_math_inf);
        vm.register_native("Math_nan", natives::math::native_math_nan);
        vm.register_native("Math_maxDouble", natives::math::native_math_max_double);
        vm.register_native("Math_minDouble", natives::math::native_math_min_double);
        vm.register_native("Math_maxInt", natives::math::native_math_max_int);
        vm.register_native("Math_minInt", natives::math::native_math_min_int);

        // Random natives
        vm.register_native("Random_seed", natives::random::native_random_seed);
        vm.register_native("Random_nextLong", natives::random::native_random_next_long);

        // Json natives
        vm.register_native("Json_parse", natives::json::native_json_parse);
        vm.register_native("Json_stringify", natives::json::native_json_stringify);

        // Env natives
        vm.register_native("Env_get", natives::system::native_env_get);
        vm.register_native("Env_set", natives::system::native_env_set);
        vm.register_native("Env_vars", natives::system::native_env_vars);

        // Fs natives
        vm.register_native("Fs_exists", natives::system::native_fs_exists);
        vm.register_native("Fs_isFile", natives::system::native_fs_is_file);
        vm.register_native("Fs_isDir", natives::system::native_fs_is_dir);
        vm.register_native("Fs_size", natives::system::native_fs_size);

        // Process natives
        vm.register_native("Process_id", natives::system::native_process_id);
        vm.register_native("Process_args", natives::system::native_process_args);

        // Os natives
        vm.register_native("Os_name", natives::system::native_os_name);
        vm.register_native("Os_arch", natives::system::native_os_arch);
        vm.register_native("Os_family", natives::system::native_os_family);

        // String utility natives
        vm.register_native("String_trimStart", natives::string::native_string_trim_start);
        vm.register_native("String_trimEnd", natives::string::native_string_trim_end);
        vm.register_native("String_startsWith", natives::string::native_string_starts_with);
        vm.register_native("String_endsWith", natives::string::native_string_ends_with);
        vm.register_native("String_padLeft", natives::string::native_string_pad_left);
        vm.register_native("String_padRight", natives::string::native_string_pad_right);

        // Hash natives
        vm.register_native("Hash_md5", natives::hash::native_hash_md5);
        vm.register_native("Hash_sha1", natives::hash::native_hash_sha1);
        vm.register_native("Hash_sha256", natives::hash::native_hash_sha256);

        // Base64 natives
        vm.register_native("Base64_encode", natives::encoding::native_base64_encode);
        vm.register_native("Base64_decode", natives::encoding::native_base64_decode);

        // Hex natives
        vm.register_native("Hex_encode", natives::encoding::native_hex_encode);
        vm.register_native("Hex_decode", natives::encoding::native_hex_decode);

        // URL encoding natives
        vm.register_native("Url_encode", natives::encoding::native_url_encode);
        vm.register_native("Url_decode", natives::encoding::native_url_decode);

        // Additional String natives
        vm.register_native("String_toUpperCase", natives::string::native_string_to_uppercase);
        vm.register_native("String_toLowerCase", natives::string::native_string_to_lower_case);
        vm.register_native("String_replace", natives::string::native_string_replace);

        // Additional Math natives
        vm.register_native("Math_nextUp", natives::math::native_math_next_up);
        vm.register_native("Math_nextDown", natives::math::native_math_next_down);
        vm.register_native("Math_ulp", natives::math::native_math_ulp);
        vm.register_native("Math_getExponent", natives::math::native_math_get_exponent);
        vm.register_native("Math_scalb", natives::math::native_math_scalb);
        vm.register_native("Math_random", natives::math::native_math_random);
        vm.register_native("Math_negInf", natives::math::native_math_neg_inf);

        // Additional Regex natives
        vm.register_native("Regex_groupCount", natives::regex::native_regex_group_count);

        // Additional Directory natives
        vm.register_native("Dir_walk", natives::directory::native_dir_walk);
        vm.register_native("Dir_copy", natives::directory::native_dir_copy);
        vm.register_native("Dir_move", natives::directory::native_dir_move);

        // Additional Time natives
        vm.register_native("Time_dayOfWeek", natives::time::native_time_day_of_week);
        vm.register_native("Time_dayOfYear", natives::time::native_time_day_of_year);

        // Double and Long parsing natives
        vm.register_native("Double_parseDouble", natives::builtins::native_double_parse_double);
        vm.register_native("Long_parseLong", natives::builtins::native_long_parse_long);

        // Subprocess natives
        vm.register_native("Subprocess_run", natives::subprocess::native_subprocess_run);
        vm.register_native("Subprocess_exec", natives::subprocess::native_subprocess_exec);

        // Tempfile natives
        vm.register_native("Tempfile_create", natives::tempfile::native_tempfile_create);

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

    /// Set the maximum call depth to prevent stack overflow.
    pub fn set_max_call_depth(&mut self, depth: usize) {
        self.max_call_depth = depth;
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
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}
