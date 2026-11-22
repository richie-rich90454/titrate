//! Host-runtime contract shared by the WASM backend and its test runner.
//!
//! Generated modules import these functions from the `env` module. A host must
//! provide them (the test harness in `tests/wasm_*.rs` does) and export a
//! linear memory named `memory` plus a mutable i32 global named `heap_ptr`.

use wasm_encoder::ValType;

use ValType::{F64, I32, I64};

macro_rules! imports {
    ($( ($name:expr, [$($pt:expr),*], $ret:expr) ),* $(,)?) => {
        pub const IMPORTS: &[(&str, &[ValType], Option<ValType>)] = &[
            $( ($name, &[$($pt),*], $ret) ),*
        ];
    };
}

/// Value-kind tags used by the generic "boxed value" storage imports.
/// `0` = i64, `1` = f64, `2` = bool, `3` = string pointer, `4` = object handle.
pub const TAG_I64: i32 = 0;
pub const TAG_F64: i32 = 1;
pub const TAG_BOOL: i32 = 2;
pub const TAG_STR: i32 = 3;
pub const TAG_OBJ: i32 = 4;

imports! {
    // --- output -------------------------------------------------------------
    ("print", [I32], None),
    ("print_raw", [I32], None),
    ("print_i64", [I64], None),
    ("print_f64", [F64], None),
    ("print_bool", [I32], None),
    ("print_char", [I32], None),

    // --- strings ------------------------------------------------------------
    ("concat", [I32, I32], Some(I32)),
    ("str_eq", [I32, I32], Some(I32)),
    ("str_len", [I32], Some(I32)),
    ("str_char_at", [I32, I32], Some(I32)),
    ("str_substring", [I32, I32, I32], Some(I32)),
    ("str_index_of", [I32, I32], Some(I32)),
    ("str_upper", [I32], Some(I32)),
    ("str_lower", [I32], Some(I32)),
    ("str_trim", [I32], Some(I32)),
    ("str_starts_with", [I32, I32], Some(I32)),
    ("str_ends_with", [I32, I32], Some(I32)),
    ("to_string_i64", [I64], Some(I32)),
    ("to_string_f64", [F64], Some(I32)),
    ("to_string_bool", [I32], Some(I32)),
    ("to_string_char", [I32], Some(I32)),
    ("parse_int", [I32], Some(I64)),
    ("parse_double", [I32], Some(F64)),
    ("fmod", [F64, F64], Some(F64)),

    // --- math ---------------------------------------------------------------
    ("math_sin", [F64], Some(F64)),
    ("math_cos", [F64], Some(F64)),
    ("math_tan", [F64], Some(F64)),
    ("math_asin", [F64], Some(F64)),
    ("math_acos", [F64], Some(F64)),
    ("math_atan", [F64], Some(F64)),
    ("math_atan2", [F64, F64], Some(F64)),
    ("math_exp", [F64], Some(F64)),
    ("math_ln", [F64], Some(F64)),
    ("math_log2", [F64], Some(F64)),
    ("math_log10", [F64], Some(F64)),
    ("math_pow", [F64, F64], Some(F64)),
    ("math_cbrt", [F64], Some(F64)),
    ("math_hypot", [F64, F64], Some(F64)),
    ("math_abs", [F64], Some(F64)),
    ("math_floor", [F64], Some(F64)),
    ("math_ceil", [F64], Some(F64)),
    ("math_round", [F64], Some(F64)),
    ("math_sqrt", [F64], Some(F64)),

    // --- objects ------------------------------------------------------------
    ("obj_new", [], Some(I32)),
    ("obj_named_new", [I32], Some(I32)),
    ("field_set_i64", [I32, I32, I64], None),
    ("field_set_f64", [I32, I32, F64], None),
    ("field_set_bool", [I32, I32, I32], None),
    ("field_set_str", [I32, I32, I32], None),
    ("field_set_obj", [I32, I32, I32], None),
    ("field_get_i64", [I32, I32], Some(I64)),
    ("field_get_f64", [I32, I32], Some(F64)),
    ("field_get_bool", [I32, I32], Some(I32)),
    ("field_get_str", [I32, I32], Some(I32)),
    ("field_get_obj", [I32, I32], Some(I32)),
    ("obj_class", [I32], Some(I32)),
    ("obj_display_named", [I32, I32], Some(I32)),

    // --- ArrayList ----------------------------------------------------------
    ("list_new", [], Some(I32)),
    ("list_add", [I32, I32, I64, F64, I32], None),
    ("list_set", [I32, I64, I32, I64, F64, I32], None),
    ("list_get_i64", [I32, I64], Some(I64)),
    ("list_get_f64", [I32, I64], Some(F64)),
    ("list_get_bool", [I32, I64], Some(I32)),
    ("list_get_str", [I32, I64], Some(I32)),
    ("list_get_obj", [I32, I64], Some(I32)),
    ("list_size", [I32], Some(I64)),
    ("list_contains", [I32, I32, I64, F64, I32], Some(I32)),
    ("list_index_of", [I32, I32, I64, F64, I32], Some(I64)),
    ("list_display", [I32], Some(I32)),
    ("list_clear", [I32], None),
    ("list_is_empty", [I32], Some(I32)),
    ("list_remove_i64", [I32, I64], Some(I64)),
    ("list_remove_f64", [I32, I64], Some(F64)),
    ("list_remove_bool", [I32, I64], Some(I32)),
    ("list_remove_str", [I32, I64], Some(I32)),
    ("list_remove_obj", [I32, I64], Some(I32)),
    ("list_pop_i64", [I32], Some(I64)),
    ("list_pop_f64", [I32], Some(F64)),
    ("list_pop_bool", [I32], Some(I32)),
    ("list_pop_str", [I32], Some(I32)),
    ("list_pop_obj", [I32], Some(I32)),
    ("list_add_all", [I32, I32], None),
    ("list_last_index_of", [I32, I32, I64, F64, I32], Some(I64)),
    ("list_max_i64", [I32], Some(I64)),
    ("list_max_f64", [I32], Some(F64)),
    ("list_min_i64", [I32], Some(I64)),
    ("list_min_f64", [I32], Some(F64)),
    ("list_reverse", [I32], None),

    // --- dynamic values ------------------------------------------------------
    ("null_new", [], Some(I32)),
    ("box_value", [I32, I64, F64, I32], Some(I32)),
    ("dyn_to_string", [I32], Some(I32)),

    // --- HashMap (all primitive keys) ----------------------------------------
    ("map_new", [], Some(I32)),
    ("map_put_str", [I32, I32, I32, I64, F64, I32], None),
    ("map_put_i64", [I32, I64, I32, I64, F64, I32], None),
    ("map_put_f64", [I32, F64, I32, I64, F64, I32], None),
    ("map_put_bool", [I32, I32, I32, I64, F64, I32], None),
    ("map_get_str_i64", [I32, I32], Some(I64)),
    ("map_get_str_f64", [I32, I32], Some(F64)),
    ("map_get_str_bool", [I32, I32], Some(I32)),
    ("map_get_str_str", [I32, I32], Some(I32)),
    ("map_get_str_obj", [I32, I32], Some(I32)),
    ("map_get_i64_i64", [I32, I64], Some(I64)),
    ("map_get_i64_f64", [I32, I64], Some(F64)),
    ("map_get_i64_bool", [I32, I64], Some(I32)),
    ("map_get_i64_str", [I32, I64], Some(I32)),
    ("map_get_i64_obj", [I32, I64], Some(I32)),
    ("map_get_f64_i64", [I32, F64], Some(I64)),
    ("map_get_f64_f64", [I32, F64], Some(F64)),
    ("map_get_f64_bool", [I32, F64], Some(I32)),
    ("map_get_f64_str", [I32, F64], Some(I32)),
    ("map_get_f64_obj", [I32, F64], Some(I32)),
    ("map_get_bool_i64", [I32, I32], Some(I64)),
    ("map_get_bool_f64", [I32, I32], Some(F64)),
    ("map_get_bool_bool", [I32, I32], Some(I32)),
    ("map_get_bool_str", [I32, I32], Some(I32)),
    ("map_get_bool_obj", [I32, I32], Some(I32)),
    ("map_size", [I32], Some(I64)),
    ("map_contains_str", [I32, I32], Some(I32)),
    ("map_contains_i64", [I32, I64], Some(I32)),
    ("map_contains_f64", [I32, F64], Some(I32)),
    ("map_contains_bool", [I32, I32], Some(I32)),
    ("map_display", [I32], Some(I32)),
    ("map_keys", [I32], Some(I32)),
    ("map_values", [I32], Some(I32)),
    ("map_contains_value_i64", [I32, I32, I64, F64, I32], Some(I32)),
    ("map_contains_value_f64", [I32, I32, I64, F64, I32], Some(I32)),
    ("map_contains_value_bool", [I32, I32, I64, F64, I32], Some(I32)),
    ("map_contains_value_str", [I32, I32, I64, F64, I32], Some(I32)),
    ("map_contains_value_obj", [I32, I32, I64, F64, I32], Some(I32)),
    ("map_clear", [I32], None),
    ("map_remove_str", [I32, I32], None),
    ("map_remove_i64", [I32, I64], None),
    ("map_remove_f64", [I32, F64], None),
    ("map_remove_bool", [I32, I32], None),

    // --- enums --------------------------------------------------------------
    ("enum_new", [I32, I32, I64], Some(I32)),
    ("enum_set", [I32, I64, I32, I64, F64, I32], None),
    ("enum_get_i64", [I32, I64], Some(I64)),
    ("enum_get_f64", [I32, I64], Some(F64)),
    ("enum_get_bool", [I32, I64], Some(I32)),
    ("enum_get_str", [I32, I64], Some(I32)),
    ("enum_get_obj", [I32, I64], Some(I32)),
    ("enum_variant", [I32], Some(I32)),
    ("enum_display", [I32], Some(I32)),

    // --- Result -------------------------------------------------------------
    ("result_new", [I32, I32, I64, F64, I32], Some(I32)),
    ("result_is_ok", [I32], Some(I32)),
    ("result_unwrap_i64", [I32], Some(I64)),
    ("result_unwrap_f64", [I32], Some(F64)),
    ("result_unwrap_bool", [I32], Some(I32)),
    ("result_unwrap_str", [I32], Some(I32)),
    ("result_unwrap_obj", [I32], Some(I32)),
    ("result_display", [I32], Some(I32)),

    // --- tuples -------------------------------------------------------------
    ("tuple_new", [I64], Some(I32)),
    ("tuple_set", [I32, I64, I32, I64, F64, I32], None),
    ("tuple_get_i64", [I32, I64], Some(I64)),
    ("tuple_get_f64", [I32, I64], Some(F64)),
    ("tuple_get_bool", [I32, I64], Some(I32)),
    ("tuple_get_str", [I32, I64], Some(I32)),
    ("tuple_get_obj", [I32, I64], Some(I32)),
    ("tuple_display", [I32], Some(I32)),

    // --- closures -----------------------------------------------------------
    ("closure_new", [I64], Some(I32)),

    // --- error handling -----------------------------------------------------
    ("err_set", [I32, I64, F64, I32], None),
    ("err_get_i64", [], Some(I64)),
    ("err_get_f64", [], Some(F64)),
    ("err_get_bool", [], Some(I32)),
    ("err_get_str", [], Some(I32)),
    ("err_get_obj", [], Some(I32)),
    ("err_flag", [], Some(I32)),
    ("err_clear", [], None),

    // --- file ---------------------------------------------------------------
    ("file_read", [I32], Some(I32)),
    ("file_write", [I32, I32], None),
    ("file_append", [I32, I32], None),
    ("file_exists", [I32], Some(I32)),
    ("file_delete", [I32], None),
    ("file_size", [I32], Some(I64)),
    ("file_read_lines", [I32], Some(I32)),
    ("file_copy", [I32, I32], None),
    ("file_rename", [I32, I32], Some(I32)),

    // --- JSON ----------------------------------------------------------------
    ("json_parse", [I32], Some(I32)),
    ("json_stringify_i64", [I64], Some(I32)),
    ("json_stringify_f64", [F64], Some(I32)),
    ("json_stringify_bool", [I32], Some(I32)),
    ("json_stringify_str", [I32], Some(I32)),
    ("json_stringify_handle", [I32], Some(I32)),

    // --- regex --------------------------------------------------------------
    ("regex_match", [I32, I32], Some(I32)),
    ("regex_find", [I32, I32], Some(I32)),
    ("regex_replace", [I32, I32, I32], Some(I32)),
    ("regex_group_count", [I32], Some(I64)),
    ("regex_find_groups", [I32, I32], Some(I32)),
    ("regex_sub_n", [I32, I32, I32, I64], Some(I32)),
    ("regex_full_match", [I32, I32], Some(I32)),
    ("regex_split", [I32, I32], Some(I32)),

    // --- OS / environment ----------------------------------------------------
    ("os_name", [], Some(I32)),
    ("os_arch", [], Some(I32)),
    ("os_family", [], Some(I32)),
    ("os_cpu_count", [], Some(I64)),
    ("os_getcwd", [], Some(I32)),
    ("os_getenv", [I32], Some(I32)),
    ("os_user_name", [], Some(I32)),
    ("os_host_name", [], Some(I32)),
    ("os_urandom", [I64], Some(I32)),
    ("os_chmod", [I32, I32], Some(I32)),
    ("os_makedirs", [I32], None),
    ("os_getpid", [], Some(I64)),

    // --- networking (deterministic host emulation) ---------------------------
    ("socket_new", [], Some(I64)),
    ("socket_connect", [I64, I32, I64], None),
    ("socket_send", [I64, I32], Some(I64)),
    ("socket_recv", [I64, I64], Some(I32)),
    ("socket_close", [I64], None),
    ("socket_set_timeout", [I64, I64], None),
    ("net_connect", [I32, I64], Some(I64)),
    ("net_send", [I64, I32], Some(I64)),
    ("net_receive", [I64, I64], Some(I32)),
    ("net_close", [I64], None),
    ("net_set_timeout", [I64, I64], None),
    ("net_get_local_address", [I64], Some(I32)),
    ("net_get_remote_address", [I64], Some(I32)),
    ("udp_socket_new", [], Some(I64)),
    ("udp_socket_close", [I64], None),
    ("udp_socket_set_timeout", [I64, I64], None),
    ("http_get", [I32], Some(I32)),
    ("http_post", [I32, I32, I32], Some(I32)),
    ("http_put", [I32, I32, I32], Some(I32)),
    ("http_delete", [I32], Some(I32)),
    ("http_patch", [I32, I32, I32], Some(I32)),
    ("http_head", [I32], Some(I32)),
    ("http_set_timeout", [I64], None),
    ("http_set_follow_redirects", [I64], None),

    // --- iterators ----------------------------------------------------------
    ("iterator_new", [I32], Some(I32)),
    ("iterator_from_list", [I32], Some(I32)),
    ("iterator_collect", [I32], Some(I32)),

    // --- unsupported-at-runtime diagnostic ----------------------------------
    ("wasm_unsupported", [I32], None),
}

/// Number of host imports; defined functions start at this index.
pub const IMPORT_COUNT: u32 = IMPORTS.len() as u32;

/// Resolve a host import name to its function index.
pub fn import_index(name: &str) -> Option<u32> {
    IMPORTS
        .iter()
        .position(|(n, _, _)| *n == name)
        .map(|i| i as u32)
}
