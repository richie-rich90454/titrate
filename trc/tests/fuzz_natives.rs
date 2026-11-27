//! Deterministic fuzz harness for native VM calls.
//!
//! A sampled set of pure, side-effect-free natives is called with randomly
//! generated `Value` arguments, most of which have the wrong type. Every call
//! must return a structured result or a structured error — never panic.
//!
//! The generator is a linear congruential generator with a fixed seed, so a
//! failure is reproducible from the iteration number alone. No external
//! crates and no host commands are used.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

use trc::bytecode::{value::Value, Vm};

/// 64-bit linear congruential generator (Numerical Recipes constants).
struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Lcg {
            state: if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed },
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        // Low bits of an LCG are poorly distributed; use the high half.
        self.state >> 32
    }

    /// Inclusive range.
    fn range(&mut self, lo: usize, hi: usize) -> usize {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u64() as usize % (hi - lo + 1))
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.range(0, items.len() - 1)]
    }

    fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

/// Fifty pure natives. Deliberately excludes anything that can block, spawn
/// work, exit the process, or write to the filesystem (sleep, network,
/// subprocess, file mutation, and signal natives are out of scope here).
const NATIVES: &[&str] = &[
    // Math: transcendental, powers, rounding, constants.
    "Math_sin",
    "Math_cos",
    "Math_tan",
    "Math_asin",
    "Math_acos",
    "Math_atan",
    "Math_atan2",
    "Math_ln",
    "Math_log2",
    "Math_log10",
    "Math_exp",
    "Math_pow",
    "Math_sqrt",
    "Math_cbrt",
    "Math_abs",
    "Math_absInt",
    "Math_floor",
    "Math_ceil",
    "Math_round",
    "Math_fma",
    "Math_inf",
    "Math_nan",
    "Math_negInf",
    "Math_maxDouble",
    "Math_minDouble",
    "Math_maxInt",
    "Math_minInt",
    "Math_ulp",
    // Strings.
    "String_split",
    "String_trim",
    "String_length",
    "String_trimStart",
    "String_trimEnd",
    "String_startsWith",
    "String_endsWith",
    "String_toUpperCase",
    "String_toLowerCase",
    "String_replace",
    // Hashing.
    "Hash_md5",
    "Hash_sha1",
    "Hash_sha256",
    "Hash_crc32",
    // Encoding.
    "Base64_encode",
    "Base64_decode",
    "Hex_encode",
    "Hex_decode",
    // Structured text.
    "Regex_match",
    "Regex_find",
    "Json_parse",
    "Json_stringify",
];

const STRINGS: &[&str] = &[
    "",
    "a",
    "abc",
    "hello world",
    "42",
    "3.14",
    "true",
    "x/y",
    "ff",
    "a,b,c",
    "(?!",
];
const CHARS: &[char] = &['a', 'Z', '0', ' ', '\n', 'x'];
const SCALAR_VARIANTS: usize = 17;

fn random_scalar(rng: &mut Lcg) -> Value {
    match rng.range(0, SCALAR_VARIANTS) {
        0 => Value::Void,
        1 => Value::Null,
        2 => Value::Bool(rng.bool()),
        3 => Value::Byte(rng.range(0, 24) as i8 - 12),
        4 => Value::Short(rng.range(0, 200) as i16 - 100),
        5 => Value::Int(rng.range(0, 200) as i32 - 100),
        6 => Value::Long(rng.range(0, 200) as i64 - 100),
        7 => Value::Vast(rng.range(0, 200) as i128 - 100),
        8 => Value::Uvast(rng.range(0, 200) as u128),
        9 => Value::Float(rng.range(0, 200) as f32 - 100.0),
        10 => Value::Double(rng.range(0, 200) as f64 - 100.0),
        11 => Value::Half(rng.range(0, 200) as f32 - 100.0),
        12 => Value::Quad(rng.range(0, 200) as f64 - 100.0),
        13 => Value::Char(*rng.pick(CHARS)),
        14 => Value::String(Rc::new((*rng.pick(STRINGS)).to_string())),
        15 => Value::EnumVariant {
            enum_name: "FuzzEnum".to_string(),
            variant: "FuzzVariant".to_string(),
            field_count: rng.range(0, 3),
        },
        _ => Value::Moved,
    }
}

fn random_value(rng: &mut Lcg) -> Value {
    match rng.range(0, 10) {
        0..=6 => random_scalar(rng),
        7 => Value::Array {
            elements: (0..rng.range(0, 3)).map(|_| random_scalar(rng)).collect(),
        },
        8 => Value::Tuple {
            elements: (0..rng.range(0, 3)).map(|_| random_scalar(rng)).collect(),
        },
        9 if rng.bool() => Value::ResultOk(Box::new(random_scalar(rng))),
        _ => Value::ResultErr(Box::new(random_scalar(rng))),
    }
}

const ITERATIONS: usize = 5_000;

#[test]
fn sampled_natives_are_registered() {
    assert_eq!(NATIVES.len(), 50, "the sample must contain exactly 50 natives");
    let mut vm = Vm::new();
    for &name in NATIVES {
        if let Err(message) = vm.call_native_by_name(name, &[]) {
            assert!(
                !message.contains("Unknown native function"),
                "sampled native '{}' is not registered: {}",
                name,
                message
            );
        }
    }
}

#[test]
fn natives_wrong_typed_args_never_panic() {
    let mut rng = Lcg::new(0x4E41_5449_5645_5331);
    let mut vm = Vm::new();
    let mut panics = 0usize;

    for iteration in 0..ITERATIONS {
        let name = *rng.pick(NATIVES);
        let arg_count = rng.range(0, 5);
        let args: Vec<Value> = (0..arg_count).map(|_| random_value(&mut rng)).collect();

        let outcome = catch_unwind(AssertUnwindSafe(|| vm.call_native_by_name(name, &args)));
        if outcome.is_err() {
            panics += 1;
            if panics <= 3 {
                eprintln!(
                    "Panic on iteration {} calling '{}' with {} arguments",
                    iteration, name, arg_count
                );
            }
        }
    }

    assert_eq!(
        panics, 0,
        "native fuzz reported {} panics out of {} calls",
        panics, ITERATIONS
    );
}
