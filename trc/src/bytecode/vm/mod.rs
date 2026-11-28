// Titrate Alpha 0.2 – bytecode virtual machine
// Precision in every step – richie-rich90454, 2026

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::runtime_policy::RuntimePolicy;

use super::frame::{ClassDef, EnumDef, ExceptionHandler, Frame, FunctionDef};
use super::chunk::Chunk;
use super::value::{NativeFn, Value};

mod step;
mod step_stack_arith;
mod step_cmp_logic;
mod step_flow;
mod call;
mod call_method;
mod call_arraylist;
mod call_hashmap;
mod call_iterator;
mod operators;
mod object;
mod object_static_a;
mod object_static_b;
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
    /// Reverse mapping so native dispatch can check the policy by name.
    pub(super) native_name_by_idx: Vec<String>,
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
    /// Cached native index for Thread_spawn (set after registration).
    /// Used by call_native_fn to execute the closure argument synchronously
    /// because Value contains Rc<> which is not Send-safe across threads.
    pub(super) thread_spawn_idx: u16,
    /// Step counter for debugging infinite loops
    pub(super) step_count: u64,
    /// Maximum steps per program run (0 = unlimited).
    pub(super) max_steps: u64,
    /// Unrecoverable VM error that must bypass try/catch routing.
    pub(super) fatal_error: Option<String>,
    /// Module-level global variables
    pub(super) globals: Vec<Value>,
    /// Exception handler stack for try/catch
    pub(super) exception_handlers: Vec<ExceptionHandler>,
    /// Capability and resource policy for this run (trusted by default).
    pub(super) policy: RuntimePolicy,
    /// Confinement state for natives dispatched by this VM. Shared with the
    /// registered native closures so policy changes affect live registrations.
    pub(super) native_context: Rc<RefCell<natives::NativeContext>>,
    /// Context-aware implementations for natives that touch paths or allocate.
    pub(super) native_ctx_fns: HashMap<u16, natives::ContextNativeFn>,
}

const DEFAULT_MAX_STEPS: u64 = 10_000_000;

fn max_steps_from_env() -> u64 {
    match std::env::var("TITRATE_MAX_STEPS") {
        Ok(v) => v.parse::<u64>().unwrap_or(DEFAULT_MAX_STEPS),
        Err(_) => DEFAULT_MAX_STEPS,
    }
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
            native_name_by_idx: Vec::new(),
            heap: Vec::new(),
            region_stack: Vec::new(),
            output: Vec::new(),
            working_dir: None,
            max_call_depth: 10000,
            thread_spawn_idx: 0xFFFF,
            step_count: 0,
            max_steps: max_steps_from_env(),
            fatal_error: None,
            globals: Vec::new(),
            exception_handlers: Vec::new(),
            policy: RuntimePolicy::trusted(),
            native_context: Rc::new(RefCell::new(natives::NativeContext::trusted())),
            native_ctx_fns: HashMap::new(),
        };

        // Register built-in native functions
        vm.register_native("println", natives::builtins::native_println);
        vm.register_native("toString", natives::builtins::native_to_string);
        vm.register_native("parseInt", natives::builtins::native_parse_int);
        vm.register_native("Ok", natives::builtins::native_ok);
        vm.register_native("Err", natives::builtins::native_err);
        vm.register_native("File_readFile", natives::file::native_file_read);
        vm.register_native("File_writeFile", natives::file::native_file_write);
        vm.register_native("File_append", natives::file::native_file_append);
        vm.register_native("File_readChunk", natives::file::native_file_read_chunk);
        vm.register_native("File_readLines", natives::file::native_file_read_lines);
        vm.register_native("File_readLine", natives::file::native_file_read_line);
        vm.register_native("File_write", natives::file::native_file_write_content);
        vm.register_native("File_close", natives::file::native_file_close);
        vm.register_native("File_seek", natives::file::native_file_seek);
        vm.register_native("File_tell", natives::file::native_file_tell);
        vm.register_native("File_readBytes", natives::file::native_file_read_bytes);
        vm.register_native("File_writeBytes", natives::file::native_file_write_bytes);
        vm.register_native("File_lastModified", natives::file::native_file_last_modified);
        vm.register_native("File_flush", natives::file::native_file_flush);
        vm.register_native("File_size", natives::file::native_file_size);
        vm.register_native("File_truncate", natives::file::native_file_truncate);
        vm.register_native("File_copy", natives::file::native_file_copy);
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
        vm.register_native("Path_isSymlink", natives::path::native_path_is_symlink);

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
        vm.register_native("Hash_sha384", natives::hash::native_hash_sha384);
        vm.register_native("Hash_sha512", natives::hash::native_hash_sha512);
        vm.register_native("Hash_sha3_256", natives::hash::native_hash_sha3_256);
        vm.register_native("Hash_sha3_384", natives::hash::native_hash_sha3_384);
        vm.register_native("Hash_sha3_512", natives::hash::native_hash_sha3_512);
        vm.register_native("Hash_blake2b", natives::hash::native_hash_blake2b);
        vm.register_native("Hash_blake2s", natives::hash::native_hash_blake2s);
        vm.register_native("Hash_crc32", natives::hash::native_hash_crc32);
        vm.register_native("Hasher_new", natives::hash::native_hasher_new);
        vm.register_native("Hasher_update", natives::hash::native_hasher_update);
        vm.register_native("Hasher_digest", natives::hash::native_hasher_digest);
        vm.register_native("Hasher_hexDigest", natives::hash::native_hasher_hex_digest);
        vm.register_native("Hasher_reset", natives::hash::native_hasher_reset);
        vm.register_native("Hmac_compareDigest", natives::hash::native_hmac_compare_digest);

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
        vm.register_native("Math_fma", natives::math::native_math_fma);

        // Additional Regex natives
        vm.register_native("Regex_groupCount", natives::regex::native_regex_group_count);
        vm.register_native("Regex_findGroups", natives::regex::native_regex_find_groups);
        vm.register_native("Regex_findWithFlags", natives::regex::native_regex_find_with_flags);
        vm.register_native("Regex_matchWithFlags", natives::regex::native_regex_match_with_flags);

        // Additional Directory natives
        vm.register_native("Dir_walk", natives::directory::native_dir_walk);
        vm.register_native("Dir_copy", natives::directory::native_dir_copy);
        vm.register_native("Dir_move", natives::directory::native_dir_move);

        // Additional Time natives
        vm.register_native("Time_dayOfWeek", natives::time::native_time_day_of_week);
        vm.register_native("Time_dayOfYear", natives::time::native_time_day_of_year);
        vm.register_native("Time_monotonic", natives::time::native_time_monotonic);
        vm.register_native("Time_perfCounter", natives::time::native_time_perf_counter);
        vm.register_native("Time_epochSeconds", natives::time::native_time_epoch_seconds);

        // Double and Long parsing natives
        vm.register_native("Double_parseDouble", natives::builtins::native_double_parse_double);
        vm.register_native("Long_parseLong", natives::builtins::native_long_parse_long);

        // Subprocess natives
        vm.register_native("Subprocess_run", natives::subprocess::native_subprocess_run);
        vm.register_native("Subprocess_exec", natives::subprocess::native_subprocess_exec);

        // Tempfile natives
        vm.register_native("Tempfile_create", natives::tempfile::native_tempfile_create);

        // Thread natives
        vm.thread_spawn_idx = vm.register_native("Thread_spawn", natives::thread::native_thread_spawn);
        vm.register_native("Thread_spawnRunnable", natives::thread::native_thread_spawn_runnable);
        vm.register_native("Thread_join", natives::thread::native_thread_join);
        vm.register_native("Thread_sleep", natives::thread::native_thread_sleep);
        vm.register_native("Thread_yield", natives::thread::native_thread_yield);
        vm.register_native("Thread_getId", natives::thread::native_thread_get_id);
        vm.register_native("Thread_currentId", natives::thread::native_thread_current_id);
        vm.register_native("Thread_detach", natives::thread::native_thread_detach);

        // Mutex natives
        vm.register_native("Mutex_new", natives::mutex::native_mutex_new);
        vm.register_native("Mutex_lock", natives::mutex::native_mutex_lock);
        vm.register_native("Mutex_unlock", natives::mutex::native_mutex_unlock);
        vm.register_native("Mutex_tryLock", natives::mutex::native_mutex_try_lock);
        vm.register_native("RecursiveMutex_new", natives::mutex::native_recursive_mutex_new);
        vm.register_native("RecursiveMutex_lock", natives::mutex::native_recursive_mutex_lock);
        vm.register_native("RecursiveMutex_unlock", natives::mutex::native_recursive_mutex_unlock);
        vm.register_native("RecursiveMutex_tryLock", natives::mutex::native_recursive_mutex_try_lock);

        // Condvar natives
        vm.register_native("CondVar_new", natives::condvar::native_cv_new);
        vm.register_native("CondVar_wait", natives::condvar::native_cv_wait);
        vm.register_native("CondVar_waitFor", natives::condvar::native_cv_wait_for);
        vm.register_native("CondVar_notifyOne", natives::condvar::native_cv_notify_one);
        vm.register_native("CondVar_notifyAll", natives::condvar::native_cv_notify_all);

        // Semaphore natives
        vm.register_native("Semaphore_new", natives::semaphore::native_sem_new);
        vm.register_native("Semaphore_acquire", natives::semaphore::native_sem_acquire);
        vm.register_native("Semaphore_release", natives::semaphore::native_sem_release);
        vm.register_native("Semaphore_tryAcquire", natives::semaphore::native_sem_try_acquire);
        vm.register_native("Semaphore_availablePermits", natives::semaphore::native_sem_available_permits);

        // AtomicInt natives
        vm.register_native("AtomicInt_new", natives::atomic::native_atomic_int_new);
        vm.register_native("AtomicInt_get", natives::atomic::native_atomic_int_get);
        vm.register_native("AtomicInt_set", natives::atomic::native_atomic_int_set);
        vm.register_native("AtomicInt_fetchAdd", natives::atomic::native_atomic_int_fetch_add);
        vm.register_native("AtomicInt_fetchSub", natives::atomic::native_atomic_int_fetch_sub);
        vm.register_native("AtomicInt_compareAndSwap", natives::atomic::native_atomic_int_compare_and_swap);
        vm.register_native("AtomicInt_fetchOr", natives::atomic::native_atomic_int_fetch_or);
        vm.register_native("AtomicInt_fetchAnd", natives::atomic::native_atomic_int_fetch_and);
        vm.register_native("AtomicInt_fetchXor", natives::atomic::native_atomic_int_fetch_xor);
        vm.register_native("AtomicInt_exchange", natives::atomic::native_atomic_int_exchange);

        // AtomicBool natives
        vm.register_native("AtomicBool_new", natives::atomic::native_atomic_bool_new);
        vm.register_native("AtomicBool_get", natives::atomic::native_atomic_bool_get);
        vm.register_native("AtomicBool_set", natives::atomic::native_atomic_bool_set);
        vm.register_native("AtomicBool_compareAndSwap", natives::atomic::native_atomic_bool_compare_and_swap);

        // AtomicLong natives
        vm.register_native("AtomicLong_new", natives::atomic::native_atomic_long_new);
        vm.register_native("AtomicLong_get", natives::atomic::native_atomic_long_get);
        vm.register_native("AtomicLong_set", natives::atomic::native_atomic_long_set);
        vm.register_native("AtomicLong_fetchAdd", natives::atomic::native_atomic_long_fetch_add);
        vm.register_native("AtomicLong_fetchSub", natives::atomic::native_atomic_long_fetch_sub);
        vm.register_native("AtomicLong_compareAndSwap", natives::atomic::native_atomic_long_compare_and_swap);

        // AtomicRef natives
        vm.register_native("AtomicRef_new", natives::atomic::native_atomic_ref_new);
        vm.register_native("AtomicRef_get", natives::atomic::native_atomic_ref_get);
        vm.register_native("AtomicRef_set", natives::atomic::native_atomic_ref_set);
        vm.register_native("AtomicRef_compareAndSwap", natives::atomic::native_atomic_ref_compare_and_swap);

        // TCP Socket natives
        vm.register_native("Socket_new", natives::socket::native_socket_new);
        vm.register_native("Socket_connect", natives::socket::native_socket_connect);
        vm.register_native("Socket_bind", natives::socket::native_socket_bind);
        vm.register_native("Socket_listen", natives::socket::native_socket_listen);
        vm.register_native("Socket_accept", natives::socket::native_socket_accept);
        vm.register_native("Socket_send", natives::socket::native_socket_send);
        vm.register_native("Socket_recv", natives::socket::native_socket_recv);
        vm.register_native("Socket_close", natives::socket::native_socket_close);
        vm.register_native("Socket_setTimeout", natives::socket::native_socket_set_timeout);
        vm.register_native("Socket_setNoDelay", natives::socket::native_socket_set_no_delay);

        // UDP Socket natives
        vm.register_native("UdpSocket_new", natives::socket::native_udp_socket_new);
        vm.register_native("UdpSocket_bind", natives::socket::native_udp_socket_bind);
        vm.register_native("UdpSocket_sendTo", natives::socket::native_udp_socket_send_to);
        vm.register_native("UdpSocket_recvFrom", natives::socket::native_udp_socket_recv_from);
        vm.register_native("UdpSocket_close", natives::socket::native_udp_socket_close);
        vm.register_native("UdpSocket_setTimeout", natives::socket::native_udp_socket_set_timeout);
        vm.register_native("UdpSocket_lastSenderHost", natives::socket::native_udp_socket_last_sender_host);
        vm.register_native("UdpSocket_lastSenderPort", natives::socket::native_udp_socket_last_sender_port);
        vm.register_native("Socket_getAddrInfo", natives::socket::native_socket_get_addr_info);

        // SSL natives
        vm.register_native("Ssl_contextNew", natives::ssl::native_ssl_context_new);
        vm.register_native("Ssl_connect", natives::ssl::native_ssl_connect);
        vm.register_native("Ssl_send", natives::ssl::native_ssl_send);
        vm.register_native("Ssl_recv", natives::ssl::native_ssl_recv);
        vm.register_native("Ssl_close", natives::ssl::native_ssl_close);
        vm.register_native("Ssl_peerCertificate", natives::ssl::native_ssl_peer_certificate);
        vm.register_native("Ssl_contextClose", natives::ssl::native_ssl_context_close);
        vm.register_native("Ssl_getPeerCertHash", natives::ssl::native_ssl_get_peer_cert_hash);

        // SQLite natives (stubs)
        vm.register_native("Sqlite_open", natives::sqlite::native_sqlite_open);
        vm.register_native("Sqlite_execute", natives::sqlite::native_sqlite_execute);
        vm.register_native("Sqlite_query", natives::sqlite::native_sqlite_query);
        vm.register_native("Sqlite_close", natives::sqlite::native_sqlite_close);
        vm.register_native("Sqlite_lastInsertId", natives::sqlite::native_sqlite_last_insert_id);
        vm.register_native("Sqlite_nextRow", natives::sqlite::native_sqlite_next_row);
        vm.register_native("Sqlite_getInt", natives::sqlite::native_sqlite_get_int);
        vm.register_native("Sqlite_getString", natives::sqlite::native_sqlite_get_string);
        vm.register_native("Sqlite_getDouble", natives::sqlite::native_sqlite_get_double);
        vm.register_native("Sqlite_columnCount", natives::sqlite::native_sqlite_column_count);
        vm.register_native("Sqlite_columnName", natives::sqlite::native_sqlite_column_name);
        vm.register_native("Sqlite_closeResult", natives::sqlite::native_sqlite_close_result);

        // Mmap natives (stubs)
        vm.register_native("Mmap_open", natives::mmap::native_mmap_open);
        vm.register_native("Mmap_close", natives::mmap::native_mmap_close);
        vm.register_native("Mmap_get", natives::mmap::native_mmap_get);
        vm.register_native("Mmap_set", natives::mmap::native_mmap_set);
        vm.register_native("Mmap_size", natives::mmap::native_mmap_size);
        vm.register_native("Mmap_flush", natives::mmap::native_mmap_flush);

        // Signal natives (stubs)
        vm.register_native("Signal_register", natives::system::native_signal_register);
        vm.register_native("Signal_raise", natives::system::native_signal_raise);

        // Zlib natives (stubs)
        vm.register_native("Zlib_compress", natives::zlib::native_zlib_compress);
        vm.register_native("Zlib_decompress", natives::zlib::native_zlib_decompress);
        vm.register_native("Gzip_compress", natives::zlib::native_gzip_compress);
        vm.register_native("Gzip_decompress", natives::zlib::native_gzip_decompress);

        // Lzma natives (stubs — return Err so Lzma.tr falls back to pass-through)
        vm.register_native("Lzma_compress", natives::lzma::native_lzma_compress);
        vm.register_native("Lzma_decompress", natives::lzma::native_lzma_decompress);

        // Zip archive natives
        vm.register_native("ZipFile_open", natives::zip::native_zipfile_open);
        vm.register_native("ZipFile_entryCount", natives::zip::native_zipfile_entry_count);
        vm.register_native("ZipFile_entryName", natives::zip::native_zipfile_entry_name);
        vm.register_native("ZipFile_readEntry", natives::zip::native_zipfile_read_entry);
        vm.register_native("ZipFile_extractAll", natives::zip::native_zipfile_extract_all);
        vm.register_native("ZipFile_close", natives::zip::native_zipfile_close);
        vm.register_native("ZipWriter_open", natives::zip::native_zipwriter_open);
        vm.register_native("ZipWriter_addEntry", natives::zip::native_zipwriter_add_entry);
        vm.register_native("ZipWriter_close", natives::zip::native_zipwriter_close);

        // Additional Os natives
        vm.register_native("Os_cpuCount", natives::system_os::native_os_cpu_count);
        vm.register_native("Os_userName", natives::system_os::native_os_user_name);
        vm.register_native("Os_hostName", natives::system_os::native_os_host_name);
        vm.register_native("Os_urandom", natives::system_os::native_os_urandom);
        vm.register_native("Os_chmod", natives::system_os::native_os_chmod);
        vm.register_native("Os_makedirs", natives::system_os::native_os_makedirs);
        vm.register_native("Os_symlink", natives::system_os::native_os_symlink);
        vm.register_native("Os_readlink", natives::system_os::native_os_readlink);
        vm.register_native("Os_kill", natives::system_os::native_os_kill);
        vm.register_native("Os_environ", natives::system_os::native_os_environ);
        vm.register_native("Os_umask", natives::system_os::native_os_umask);
        vm.register_native("Os_scandir", natives::system_os::native_os_scandir);
        vm.register_native("Os_environMap", natives::system_os::native_os_environ_map);

        // Appended last: native indices are positional in compiled bytecode.
        vm.register_native("Graphics_windowCreate", natives::graphics_audio::native_graphics_window_create);
        vm.register_native("Graphics_windowIsAvailable", natives::graphics_audio::native_graphics_window_is_available);
        vm.register_native("Graphics_inputPush", natives::graphics_audio::native_graphics_input_push);
        vm.register_native("Graphics_inputPoll", natives::graphics_audio::native_graphics_input_poll);
        vm.register_native("Graphics_bufferCreate", natives::graphics_audio::native_graphics_buffer_create);
        vm.register_native("Graphics_bufferDestroy", natives::graphics_audio::native_graphics_buffer_destroy);
        vm.register_native("Graphics_bufferSet", natives::graphics_audio::native_graphics_buffer_set);
        vm.register_native("Graphics_bufferGet", natives::graphics_audio::native_graphics_buffer_get);
        vm.register_native("Graphics_bufferClear", natives::graphics_audio::native_graphics_buffer_clear);
        vm.register_native("Graphics_bufferWidth", natives::graphics_audio::native_graphics_buffer_width);
        vm.register_native("Graphics_bufferHeight", natives::graphics_audio::native_graphics_buffer_height);
        vm.register_native("AudioDevice_isAvailable", natives::graphics_audio::native_audio_device_is_available);
        vm.register_native("AudioDevice_status", natives::graphics_audio::native_audio_device_status);
        vm.register_native("AudioDevice_play", natives::graphics_audio::native_audio_device_play);
        vm.register_native("Synth_sine", natives::graphics_audio::native_synth_sine);
        vm.register_native("Synth_square", natives::graphics_audio::native_synth_square);
        vm.register_native("Synth_saw", natives::graphics_audio::native_synth_saw);
        vm.register_native("Synth_noise", natives::graphics_audio::native_synth_noise);
        vm.register_native("Synth_mix", natives::graphics_audio::native_synth_mix);
        vm.register_native("IO_readLine", natives::builtins::native_io_read_line);
        vm.register_native("IO_readAll", natives::builtins::native_io_read_all);
        vm.register_native("StructExt_pack", natives::structext::native_structext_pack);
        vm.register_native("StructExt_unpack", natives::structext::native_structext_unpack);
        vm.register_native("StructExt_packInto", natives::structext::native_structext_pack_into);
        vm.register_native("StructExt_unpackFrom", natives::structext::native_structext_unpack_from);

        // Appended to avoid shifting existing native indices: call_native_by_name
        // and STATIC_CALL both resolve this through native_names.
        vm.register_native("File_exists", natives::file::native_file_exists);

        vm
    }

    // -----------------------------------------------------------------------
    // Native function registration
    // -----------------------------------------------------------------------

    pub fn register_native(&mut self, name: &str, func: NativeFn) -> u16 {
        let idx = self.natives.len() as u16;
        self.natives.push(func);
        self.native_names.insert(name.to_string(), idx);
        self.native_name_by_idx.push(name.to_string());
        if let Some(ctx_fn) = natives::context_native_by_name(name) {
            self.native_ctx_fns.insert(idx, ctx_fn);
        }
        idx
    }

    /// Dispatch a registered native with this VM's confinement context.
    pub(super) fn invoke_native(&self, idx: u16, args: &[Value]) -> Result<Value, String> {
        // Every dispatch path (CALL_NATIVE, STATIC_CALL, direct lookup) funnels
        // through here, so a relative path escape is denied once, with E5001.
        // Absolute paths keep the legacy re-rooting done by NativeContext.
        if !self.policy.is_trusted() && !args.is_empty() {
            if let Some(name) = self.native_name_by_idx.get(idx as usize) {
                if name.starts_with("File_") || name.starts_with("Dir_") || name.starts_with("Path_") {
                    if let Some(Value::String(path)) = args.first() {
                        let requested = std::path::Path::new(path.as_str());
                        if !requested.is_absolute() {
                            self.policy
                                .enforce_path(name, requested)
                                .map_err(|denial| denial.to_string())?;
                        }
                    }
                }
            }
        }
        if let Some(ctx_fn) = self.native_ctx_fns.get(&idx) {
            let ctx = self.native_context.borrow();
            ctx_fn(&ctx, args)
        } else {
            self.natives[idx as usize](args)
        }
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
        self.step_count = 0;
        self.fatal_error = None;
        self.functions = program.functions;
        self.classes = program.classes;
        self.enums = program.enums;
        // Initialize global variable slots with Null.
        self.globals = vec![Value::Null; program.global_count];
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
        self.working_dir = Some(dir.clone());
        natives::set_native_working_dir(Some(dir));
    }

    /// Set the maximum call depth to prevent stack overflow.
    pub fn set_max_call_depth(&mut self, depth: usize) {
        self.max_call_depth = depth;
    }

    /// Set the maximum number of steps per run (0 = unlimited).
    pub fn set_max_steps(&mut self, max_steps: u64) {
        self.max_steps = max_steps;
    }

    /// Append a line to captured output, enforcing the output byte budget.
    pub(super) fn push_output(&mut self, line: String) -> Result<(), String> {
        let budget = self.policy.budgets().max_output_bytes;
        let used: usize = self.output.iter().map(|l| l.len()).sum();
        if used.saturating_add(line.len()) > budget {
            let msg = format!("RuntimePolicy: output budget of {} bytes exceeded", budget);
            self.fatal_error = Some(msg.clone());
            return Err(msg);
        }
        self.output.push(line);
        Ok(())
    }

    pub fn set_policy(&mut self, policy: RuntimePolicy) {
        *self.native_context.borrow_mut() = natives::NativeContext::from_policy(&policy);
        // Legacy one-argument helpers are still exposed publicly; keep them in
        // sync for direct callers. VM-dispatched natives use native_context.
        natives::set_sandbox_root(policy.sandbox_root().map(|p| p.to_path_buf()));
        natives::set_allocation_budget(
            policy.budgets().max_allocation.min(natives::MAX_IO_BYTES),
        );
        // An env var may disable the step cap in trusted mode, but an
        // untrusted program must always get the policy's finite budget.
        self.max_steps = if policy.is_trusted() {
            self.max_steps.min(policy.budgets().max_steps)
        } else {
            policy.budgets().max_steps.max(1)
        };
        self.policy = policy;
    }

    /// Drop exception handlers whose owning frame is gone. A live handler's
    /// frame is always on the frame stack, so a handler recorded at a deeper
    /// frame depth is stale (e.g. leaked by `break`/`continue` out of a
    /// `try`, or a closure returning across try scope). Unwinding to one
    /// would jump to a foreign catch_ip in the wrong frame and corrupt the
    /// VM, so discard them before selecting a handler.
    pub(super) fn pop_stale_handlers(&mut self) {
        while self
            .exception_handlers
            .last()
            .is_some_and(|h| h.frame_depth > self.frames.len())
        {
            self.exception_handlers.pop();
        }
    }

    /// Call a registered native function by name with the given arguments.
    /// Useful for testing native functions directly.
    pub fn call_native_by_name(&mut self, name: &str, args: &[Value]) -> Result<Value, String> {
        if let Err(denial) = self.policy.enforce_native(name) {
            return Err(denial.to_string());
        }
        let idx = self.native_names.get(name).copied()
            .ok_or_else(|| format!("Unknown native function '{}'", name))?;
        self.invoke_native(idx, args)
    }

    /// Resolve a file path, applying the same confinement as the natives so
    /// File object methods cannot bypass the sandbox root.
    fn resolve_path(&self, path: &str) -> std::path::PathBuf {
        let p = std::path::Path::new(path);
        let resolved = if p.is_absolute() {
            p.to_path_buf()
        } else if let Some(ref dir) = self.working_dir {
            dir.join(path)
        } else {
            p.to_path_buf()
        };
        match self.policy.sandbox_root() {
            Some(root) => natives::confine_to_root(resolved, root),
            None => resolved,
        }
    }

    // -----------------------------------------------------------------------
    // Stack helpers
    // -----------------------------------------------------------------------

    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    fn pop(&mut self) -> Value {
        match self.stack.pop() {
            Some(v) => v,
            None => {
                self.fatal_error = Some("VM stack underflow: pop from an empty stack".to_string());
                Value::Null
            }
        }
    }

    fn peek(&mut self, distance: usize) -> Value {
        let idx = distance.checked_add(1).and_then(|d| self.stack.len().checked_sub(d));
        match idx.and_then(|i| self.stack.get(i)) {
            Some(v) => v.clone(),
            None => {
                self.fatal_error = Some(format!(
                    "VM stack underflow: peek at distance {} on a stack of length {}",
                    distance,
                    self.stack.len()
                ));
                Value::Null
            }
        }
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

    fn read_bytes<const N: usize>(&mut self, name: &str) -> Result<[u8; N], String> {
        let frame = self.current_frame();
        let ip = frame.ip;
        let func_idx = frame.function_index as usize;
        let code = self
            .functions
            .get(func_idx)
            .map(|f| f.chunk.code.as_slice())
            .ok_or_else(|| format!("{}: function index {} out of range", name, func_idx))?;
        let end = ip
            .checked_add(N)
            .ok_or_else(|| format!("{}: instruction pointer overflow at ip={}", name, ip))?;
        let slice = code.get(ip..end).ok_or_else(|| {
            format!(
                "{}: out of bounds ip={} code_len={} func_idx={}",
                name, ip, code.len(), func_idx
            )
        })?;
        let mut bytes = [0u8; N];
        bytes.copy_from_slice(slice);
        self.current_frame_mut().ip = end;
        Ok(bytes)
    }

    fn read_u8(&mut self) -> Result<u8, String> {
        Ok(self.read_bytes::<1>("read_u8")?[0])
    }

    fn read_u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_be_bytes(self.read_bytes::<2>("read_u16")?))
    }

    fn read_u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_be_bytes(self.read_bytes::<4>("read_u32")?))
    }

    fn read_i16(&mut self) -> Result<i16, String> {
        Ok(i16::from_be_bytes(self.read_bytes::<2>("read_i16")?))
    }

    fn read_i32(&mut self) -> Result<i32, String> {
        Ok(i32::from_be_bytes(self.read_bytes::<4>("read_i32")?))
    }

    fn read_i64(&mut self) -> Result<i64, String> {
        Ok(i64::from_be_bytes(self.read_bytes::<8>("read_i64")?))
    }

    fn read_f32(&mut self) -> Result<f32, String> {
        Ok(f32::from_be_bytes(self.read_bytes::<4>("read_f32")?))
    }

    fn read_f64(&mut self) -> Result<f64, String> {
        Ok(f64::from_be_bytes(self.read_bytes::<8>("read_f64")?))
    }

    fn chunk_string(&self, idx: u16) -> Result<String, String> {
        let frame = self.current_frame();
        let chunk = &self.functions[frame.function_index as usize].chunk;
        chunk
            .strings
            .get(idx as usize)
            .cloned()
            .ok_or_else(|| format!("String index {} out of bounds (count {})", idx, chunk.strings.len()))
    }

    // -----------------------------------------------------------------------
    // Main execution loop
    // -----------------------------------------------------------------------

    pub fn run(&mut self) -> Result<(), String> {
        // Entry point is always function index 0 (main).
        if self.functions.is_empty() {
            return Err("No main function to execute".to_string());
        }

        self.step_count = 0;
        self.fatal_error = None;
        let base = 0;
        self.frames.push(Frame::new(0, base));

        while !self.frames.is_empty() {
            if let Err(e) = self.step() {
                // Fatal errors (step limit, stack/operand corruption) must never
                // be swallowed by user-level try/catch.
                if let Some(fatal) = self.fatal_error.take() {
                    return Err(fatal);
                }
                // A VM error (division by zero, type mismatch, etc.) occurred.
                // If an exception handler is active, treat it like a thrown
                // string value so user-level try/catch can recover.
                self.pop_stale_handlers();
                if let Some(handler) = self.exception_handlers.last().cloned() {
                    while self.frames.len() > handler.frame_depth {
                        self.frames.pop();
                    }
                    if self.frames.is_empty() {
                        return Err(e);
                    }
                    while self.stack.len() > handler.stack_depth {
                        self.stack.pop();
                    }
                    self.push(Value::String(Rc::new(e)));
                    self.current_frame_mut().ip = handler.catch_ip;
                    self.exception_handlers.pop();
                } else {
                    return Err(e);
                }
            }
            if let Some(fatal) = self.fatal_error.take() {
                return Err(fatal);
            }
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
            if let Some(fatal) = self.fatal_error.take() {
                return Err(fatal);
            }
        }

        Ok(())
    }
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod policy_isolation_tests {
    use super::*;
    use crate::runtime_policy::{ResourceBudgets, RuntimePolicy};
    use std::path::PathBuf;

    #[test]
    fn two_vms_on_one_thread_keep_independent_sandbox_roots() {
        let base = std::env::temp_dir().join(format!("titrate_vm_policy_{}", std::process::id()));
        let first_root = base.join("first");
        let second_root = base.join("second");
        std::fs::create_dir_all(&first_root).expect("create first sandbox");
        std::fs::create_dir_all(&second_root).expect("create second sandbox");
        std::fs::write(first_root.join("probe.txt"), "first").expect("write first probe");
        std::fs::write(second_root.join("probe.txt"), "second").expect("write second probe");

        let mut first = Vm::new();
        first.set_policy(RuntimePolicy::untrusted(first_root.clone()));

        // Constructing another VM on this thread must not clear the first
        // VM's confinement.
        let mut second = Vm::new();
        second.set_policy(RuntimePolicy::untrusted(second_root.clone()));

        let first_resolved = natives::resolve_path_ctx(&first.native_context.borrow(), "/probe.txt");
        let second_resolved = natives::resolve_path_ctx(&second.native_context.borrow(), "/probe.txt");
        assert!(first_resolved.starts_with(&first_root), "{:?}", first_resolved);
        assert!(second_resolved.starts_with(&second_root), "{:?}", second_resolved);
        assert_ne!(first_resolved, second_resolved);

        let probe = Value::String(Rc::new("/probe.txt".to_string()));
        assert_eq!(
            first.call_native_by_name("File_exists", &[probe.clone()]).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            second.call_native_by_name("File_exists", &[probe]).unwrap(),
            Value::Bool(true)
        );

        let trusted = Vm::new();
        assert_eq!(
            natives::resolve_path_ctx(&trusted.native_context.borrow(), "/probe.txt"),
            PathBuf::from("/probe.txt")
        );

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn allocation_budgets_are_per_vm() {
        let root = std::env::temp_dir().join(format!("titrate_vm_budget_{}", std::process::id()));
        let mut tight = Vm::new();
        tight.set_policy(
            RuntimePolicy::untrusted(root.clone()).with_budgets(ResourceBudgets {
                max_steps: 1_000,
                max_output_bytes: 8,
                max_allocation: 4,
            }),
        );
        let mut loose = Vm::new();
        loose.set_policy(RuntimePolicy::untrusted(root.clone()));

        assert!(natives::checked_len_ctx(&tight.native_context.borrow(), 8, "test").is_err());
        assert!(natives::checked_len_ctx(&tight.native_context.borrow(), 4, "test").is_ok());
        assert!(natives::checked_len_ctx(&loose.native_context.borrow(), 8, "test").is_ok());
    }
}
