//! Host-runtime contract shared by the WASM backend and its test runner.
//!
//! Generated modules import these functions from the `env` module. A host must
//! provide them (the test harness in `tests/wasm_*.rs` does) and export a
//! linear memory named `memory` plus a mutable i32 global named `heap_ptr`.

use wasm_encoder::ValType;

/// Index of each host import in the WASM function index space (imports come
/// first). Keeping this list in one place prevents the encoder and the runner
/// from drifting apart.
pub const IMPORTS: &[(&str, &[ValType], Option<ValType>)] = &[
    ("print", &[ValType::I32], None),
    ("print_raw", &[ValType::I32], None),
    ("print_i64", &[ValType::I64], None),
    ("print_f64", &[ValType::F64], None),
    ("print_bool", &[ValType::I32], None),
    ("print_char", &[ValType::I32], None),
    ("concat", &[ValType::I32, ValType::I32], Some(ValType::I32)),
    ("str_eq", &[ValType::I32, ValType::I32], Some(ValType::I32)),
    ("str_len", &[ValType::I32], Some(ValType::I32)),
    ("str_char_at", &[ValType::I32, ValType::I32], Some(ValType::I32)),
    (
        "str_substring",
        &[ValType::I32, ValType::I32, ValType::I32],
        Some(ValType::I32),
    ),
    ("str_index_of", &[ValType::I32, ValType::I32], Some(ValType::I32)),
    ("str_upper", &[ValType::I32], Some(ValType::I32)),
    ("str_lower", &[ValType::I32], Some(ValType::I32)),
    ("str_trim", &[ValType::I32], Some(ValType::I32)),
    (
        "str_starts_with",
        &[ValType::I32, ValType::I32],
        Some(ValType::I32),
    ),
    (
        "str_ends_with",
        &[ValType::I32, ValType::I32],
        Some(ValType::I32),
    ),
    ("to_string_i64", &[ValType::I64], Some(ValType::I32)),
    ("to_string_f64", &[ValType::F64], Some(ValType::I32)),
    ("to_string_bool", &[ValType::I32], Some(ValType::I32)),
    ("to_string_char", &[ValType::I32], Some(ValType::I32)),
    ("parse_int", &[ValType::I32], Some(ValType::I64)),
    ("parse_double", &[ValType::I32], Some(ValType::F64)),
    ("fmod", &[ValType::F64, ValType::F64], Some(ValType::F64)),
    ("math_sin", &[ValType::F64], Some(ValType::F64)),
    ("math_cos", &[ValType::F64], Some(ValType::F64)),
    ("math_tan", &[ValType::F64], Some(ValType::F64)),
    ("math_asin", &[ValType::F64], Some(ValType::F64)),
    ("math_acos", &[ValType::F64], Some(ValType::F64)),
    ("math_atan", &[ValType::F64], Some(ValType::F64)),
    ("math_atan2", &[ValType::F64, ValType::F64], Some(ValType::F64)),
    ("math_exp", &[ValType::F64], Some(ValType::F64)),
    ("math_ln", &[ValType::F64], Some(ValType::F64)),
    ("math_log2", &[ValType::F64], Some(ValType::F64)),
    ("math_log10", &[ValType::F64], Some(ValType::F64)),
    ("math_pow", &[ValType::F64, ValType::F64], Some(ValType::F64)),
    ("math_cbrt", &[ValType::F64], Some(ValType::F64)),
    ("math_hypot", &[ValType::F64, ValType::F64], Some(ValType::F64)),
    ("math_abs", &[ValType::F64], Some(ValType::F64)),
    ("math_floor", &[ValType::F64], Some(ValType::F64)),
    ("math_ceil", &[ValType::F64], Some(ValType::F64)),
    ("math_round", &[ValType::F64], Some(ValType::F64)),
    ("math_sqrt", &[ValType::F64], Some(ValType::F64)),
    // Class-instance support: objects are host-side field maps addressed by
    // i32 handles. Field names are passed as WASM string pointers.
    ("obj_new", &[], Some(ValType::I32)),
    ("field_set_i64", &[ValType::I32, ValType::I32, ValType::I64], None),
    ("field_set_f64", &[ValType::I32, ValType::I32, ValType::F64], None),
    ("field_set_bool", &[ValType::I32, ValType::I32, ValType::I32], None),
    ("field_set_str", &[ValType::I32, ValType::I32, ValType::I32], None),
    ("field_set_obj", &[ValType::I32, ValType::I32, ValType::I32], None),
    ("field_get_i64", &[ValType::I32, ValType::I32], Some(ValType::I64)),
    ("field_get_f64", &[ValType::I32, ValType::I32], Some(ValType::F64)),
    ("field_get_bool", &[ValType::I32, ValType::I32], Some(ValType::I32)),
    ("field_get_str", &[ValType::I32, ValType::I32], Some(ValType::I32)),
    ("field_get_obj", &[ValType::I32, ValType::I32], Some(ValType::I32)),
];

/// Number of host imports; defined functions start at this index.
pub const IMPORT_COUNT: u32 = IMPORTS.len() as u32;

/// Resolve a host import name to its function index.
pub fn import_index(name: &str) -> Option<u32> {
    IMPORTS
        .iter()
        .position(|(n, _, _)| *n == name)
        .map(|i| i as u32)
}
