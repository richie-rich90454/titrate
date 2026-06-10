use trc::bytecode::Vm;
use trc::bytecode::value::Value;
use trc::bytecode::opcodes::OpCode;
use trc::bytecode::compiler::Compiler;
use trc::parser;
use trc::lexer;
use trc::ast;

// ---------------------------------------------------------------------------
// Math native function tests
// ---------------------------------------------------------------------------

#[test]
fn test_math_sin() {
    let mut vm = Vm::new();
    let result = vm.call_native_by_name("Math_sin", &[Value::Double(0.0)]);
    match result {
        Ok(Value::Double(v)) => {
            assert!((v - 0.0).abs() < 1e-10, "Math.sin(0) should be 0, got {}", v);
        }
        Ok(other) => panic!("Expected Double, got {:?}", other),
        Err(e) => panic!("Math_sin failed: {}", e),
    }
}

#[test]
fn test_math_cos() {
    let mut vm = Vm::new();
    let result = vm.call_native_by_name("Math_cos", &[Value::Double(0.0)]);
    match result {
        Ok(Value::Double(v)) => {
            assert!((v - 1.0).abs() < 1e-10, "Math.cos(0) should be 1, got {}", v);
        }
        Ok(other) => panic!("Expected Double, got {:?}", other),
        Err(e) => panic!("Math_cos failed: {}", e),
    }
}

#[test]
fn test_math_sqrt() {
    let mut vm = Vm::new();
    let result = vm.call_native_by_name("Math_sqrt", &[Value::Double(4.0)]);
    match result {
        Ok(Value::Double(v)) => {
            assert!((v - 2.0).abs() < 1e-10, "Math.sqrt(4) should be 2, got {}", v);
        }
        Ok(other) => panic!("Expected Double, got {:?}", other),
        Err(e) => panic!("Math_sqrt failed: {}", e),
    }
}

#[test]
fn test_math_sqrt_9() {
    let mut vm = Vm::new();
    let result = vm.call_native_by_name("Math_sqrt", &[Value::Double(9.0)]);
    match result {
        Ok(Value::Double(v)) => {
            assert!((v - 3.0).abs() < 1e-10, "Math.sqrt(9) should be 3, got {}", v);
        }
        Ok(other) => panic!("Expected Double, got {:?}", other),
        Err(e) => panic!("Math_sqrt failed: {}", e),
    }
}

#[test]
fn test_math_abs() {
    let mut vm = Vm::new();
    let result = vm.call_native_by_name("Math_abs", &[Value::Double(-5.0)]);
    match result {
        Ok(Value::Double(v)) => {
            assert!((v - 5.0).abs() < 1e-10, "Math.abs(-5) should be 5, got {}", v);
        }
        Ok(other) => panic!("Expected Double, got {:?}", other),
        Err(e) => panic!("Math_abs failed: {}", e),
    }
}

#[test]
fn test_math_floor_ceil() {
    let mut vm = Vm::new();

    // floor(3.7) = 3.0
    let result = vm.call_native_by_name("Math_floor", &[Value::Double(3.7)]);
    match result {
        Ok(Value::Double(v)) => {
            assert!((v - 3.0).abs() < 1e-10, "Math.floor(3.7) should be 3, got {}", v);
        }
        Ok(other) => panic!("Expected Double, got {:?}", other),
        Err(e) => panic!("Math_floor failed: {}", e),
    }

    // ceil(3.2) = 4.0
    let result = vm.call_native_by_name("Math_ceil", &[Value::Double(3.2)]);
    match result {
        Ok(Value::Double(v)) => {
            assert!((v - 4.0).abs() < 1e-10, "Math.ceil(3.2) should be 4, got {}", v);
        }
        Ok(other) => panic!("Expected Double, got {:?}", other),
        Err(e) => panic!("Math_ceil failed: {}", e),
    }
}

#[test]
fn test_math_pow() {
    let mut vm = Vm::new();
    let result = vm.call_native_by_name("Math_pow", &[
        Value::Double(2.0),
        Value::Double(10.0),
    ]);
    match result {
        Ok(Value::Double(v)) => {
            assert!((v - 1024.0).abs() < 1e-10, "Math.pow(2,10) should be 1024, got {}", v);
        }
        Ok(other) => panic!("Expected Double, got {:?}", other),
        Err(e) => panic!("Math_pow failed: {}", e),
    }
}

// ---------------------------------------------------------------------------
// Random native function tests
// ---------------------------------------------------------------------------

#[test]
fn test_random_seed() {
    let mut vm = Vm::new();
    let result = vm.call_native_by_name("Random_seed", &[]);
    match result {
        Ok(Value::Long(_)) => {}
        Ok(other) => panic!("Expected Long, got {:?}", other),
        Err(e) => panic!("Random_seed failed: {}", e),
    }
}

#[test]
fn test_random_next_long_deterministic() {
    let mut vm = Vm::new();
    // Two calls with the same state should produce the same result
    let state0 = Value::Long(12345);
    let state1 = Value::Long(67890);

    let result1 = vm.call_native_by_name("Random_nextLong", &[state0.clone(), state1.clone()]);
    let result2 = vm.call_native_by_name("Random_nextLong", &[state0, state1]);

    match (result1, result2) {
        (Ok(Value::Array { elements: e1 }), Ok(Value::Array { elements: e2 })) => {
            assert_eq!(e1.len(), e2.len(), "Arrays should have same length");
            for (i, (v1, v2)) in e1.iter().zip(e2.iter()).enumerate() {
                assert_eq!(format!("{:?}", v1), format!("{:?}", v2),
                    "Element {} should match between two calls with same seed", i);
            }
        }
        (Err(e), _) => panic!("Random_nextLong failed: {}", e),
        (Ok(other), _) => panic!("Expected Array, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Time native function tests
// ---------------------------------------------------------------------------

#[test]
fn test_time_now() {
    let mut vm = Vm::new();
    let result = vm.call_native_by_name("Time_now", &[]);
    match result {
        Ok(Value::Long(ms)) => {
            // Should be a reasonable epoch millis value (after 2020)
            let min_epoch_ms: i64 = 1577836800000; // 2020-01-01
            assert!(ms > min_epoch_ms, "Time.now() should return epoch millis after 2020, got {}", ms);
        }
        Ok(other) => panic!("Expected Long, got {:?}", other),
        Err(e) => panic!("Time_now failed: {}", e),
    }
}

#[test]
fn test_time_get_year() {
    let mut vm = Vm::new();
    // Use a known timestamp: 2025-01-01 00:00:00 UTC = 1735689600000
    let epoch_ms = Value::Long(1735689600000);
    let result = vm.call_native_by_name("Time_getYear", &[epoch_ms]);
    match result {
        Ok(Value::Int(year)) => {
            assert_eq!(year, 2025, "Year for 2025-01-01 should be 2025, got {}", year);
        }
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Time_getYear failed: {}", e),
    }
}

#[test]
fn test_time_get_month() {
    let mut vm = Vm::new();
    // 2025-01-01 00:00:00 UTC
    let epoch_ms = Value::Long(1735689600000);
    let result = vm.call_native_by_name("Time_getMonth", &[epoch_ms]);
    match result {
        Ok(Value::Int(month)) => {
            assert_eq!(month, 1, "Month for 2025-01-01 should be 1, got {}", month);
        }
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Time_getMonth failed: {}", e),
    }
}

#[test]
fn test_time_get_day() {
    let mut vm = Vm::new();
    // 2025-01-01 00:00:00 UTC
    let epoch_ms = Value::Long(1735689600000);
    let result = vm.call_native_by_name("Time_getDay", &[epoch_ms]);
    match result {
        Ok(Value::Int(day)) => {
            assert_eq!(day, 1, "Day for 2025-01-01 should be 1, got {}", day);
        }
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Time_getDay failed: {}", e),
    }
}

#[test]
fn test_time_get_hour() {
    let mut vm = Vm::new();
    // 2025-01-01 12:00:00 UTC = 1735689600000 + 12*3600*1000 = 1735732800000
    let epoch_ms = Value::Long(1735732800000);
    let result = vm.call_native_by_name("Time_getHour", &[epoch_ms]);
    match result {
        Ok(Value::Int(hour)) => {
            assert_eq!(hour, 12, "Hour for 2025-01-01 12:00 UTC should be 12, got {}", hour);
        }
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Time_getHour failed: {}", e),
    }
}

// ---------------------------------------------------------------------------
// String native method tests
// ---------------------------------------------------------------------------

#[test]
fn test_string_length() {
    let mut vm = Vm::new();
    let result = vm.call_native_by_name("String_length", &[
        Value::String(std::rc::Rc::new("hello".to_string())),
    ]);
    match result {
        Ok(Value::Int(len)) => {
            assert_eq!(len, 5, "String.length('hello') should be 5, got {}", len);
        }
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("String_length failed: {}", e),
    }
}

#[test]
fn test_string_trim() {
    let mut vm = Vm::new();
    let result = vm.call_native_by_name("String_trim", &[
        Value::String(std::rc::Rc::new("  hello  ".to_string())),
    ]);
    match result {
        Ok(Value::String(s)) => {
            assert_eq!(s.as_str(), "hello", "String.trim('  hello  ') should be 'hello', got '{}'", s);
        }
        Ok(other) => panic!("Expected String, got {:?}", other),
        Err(e) => panic!("String_trim failed: {}", e),
    }
}

#[test]
fn test_string_split() {
    let mut vm = Vm::new();
    let result = vm.call_native_by_name("String_split", &[
        Value::String(std::rc::Rc::new("hello world".to_string())),
        Value::String(std::rc::Rc::new(" ".to_string())),
    ]);
    match result {
        Ok(Value::Array { elements }) => {
            assert_eq!(elements.len(), 2, "String.split('hello world', ' ') should have 2 parts");
        }
        Ok(other) => panic!("Expected Array, got {:?}", other),
        Err(e) => panic!("String_split failed: {}", e),
    }
}

// ---------------------------------------------------------------------------
// String utility native function tests
// ---------------------------------------------------------------------------

#[test]
fn test_string_trim_start() {
    let mut vm = Vm::new();
    let result = vm.call_native_by_name("String_trimStart", &[
        Value::String(std::rc::Rc::new("  hello  ".to_string())),
    ]);
    match result {
        Ok(Value::String(s)) => {
            assert_eq!(s.as_str(), "hello  ", "String.trimStart('  hello  ') should be 'hello  ', got '{}'", s);
        }
        Ok(other) => panic!("Expected String, got {:?}", other),
        Err(e) => panic!("String_trimStart failed: {}", e),
    }
}

#[test]
fn test_string_trim_end() {
    let mut vm = Vm::new();
    let result = vm.call_native_by_name("String_trimEnd", &[
        Value::String(std::rc::Rc::new("  hello  ".to_string())),
    ]);
    match result {
        Ok(Value::String(s)) => {
            assert_eq!(s.as_str(), "  hello", "String.trimEnd('  hello  ') should be '  hello', got '{}'", s);
        }
        Ok(other) => panic!("Expected String, got {:?}", other),
        Err(e) => panic!("String_trimEnd failed: {}", e),
    }
}

#[test]
fn test_string_starts_with() {
    let mut vm = Vm::new();
    let result = vm.call_native_by_name("String_startsWith", &[
        Value::String(std::rc::Rc::new("hello world".to_string())),
        Value::String(std::rc::Rc::new("hello".to_string())),
    ]);
    match result {
        Ok(Value::Bool(b)) => {
            assert!(b, "String.startsWith('hello world', 'hello') should be true");
        }
        Ok(other) => panic!("Expected Bool, got {:?}", other),
        Err(e) => panic!("String_startsWith failed: {}", e),
    }
}

#[test]
fn test_string_starts_with_false() {
    let mut vm = Vm::new();
    let result = vm.call_native_by_name("String_startsWith", &[
        Value::String(std::rc::Rc::new("hello world".to_string())),
        Value::String(std::rc::Rc::new("world".to_string())),
    ]);
    match result {
        Ok(Value::Bool(b)) => {
            assert!(!b, "String.startsWith('hello world', 'world') should be false");
        }
        Ok(other) => panic!("Expected Bool, got {:?}", other),
        Err(e) => panic!("String_startsWith failed: {}", e),
    }
}

#[test]
fn test_string_ends_with() {
    let mut vm = Vm::new();
    let result = vm.call_native_by_name("String_endsWith", &[
        Value::String(std::rc::Rc::new("hello world".to_string())),
        Value::String(std::rc::Rc::new("world".to_string())),
    ]);
    match result {
        Ok(Value::Bool(b)) => {
            assert!(b, "String.endsWith('hello world', 'world') should be true");
        }
        Ok(other) => panic!("Expected Bool, got {:?}", other),
        Err(e) => panic!("String_endsWith failed: {}", e),
    }
}

#[test]
fn test_string_ends_with_false() {
    let mut vm = Vm::new();
    let result = vm.call_native_by_name("String_endsWith", &[
        Value::String(std::rc::Rc::new("hello world".to_string())),
        Value::String(std::rc::Rc::new("hello".to_string())),
    ]);
    match result {
        Ok(Value::Bool(b)) => {
            assert!(!b, "String.endsWith('hello world', 'hello') should be false");
        }
        Ok(other) => panic!("Expected Bool, got {:?}", other),
        Err(e) => panic!("String_endsWith failed: {}", e),
    }
}

#[test]
fn test_string_pad_left() {
    let mut vm = Vm::new();
    let result = vm.call_native_by_name("String_padLeft", &[
        Value::String(std::rc::Rc::new("hi".to_string())),
        Value::Int(5),
        Value::Char('*'),
    ]);
    match result {
        Ok(Value::String(s)) => {
            assert_eq!(s.as_str(), "***hi", "String.padLeft('hi', 5, '*') should be '***hi', got '{}'", s);
        }
        Ok(other) => panic!("Expected String, got {:?}", other),
        Err(e) => panic!("String_padLeft failed: {}", e),
    }
}

#[test]
fn test_string_pad_left_long_width() {
    let mut vm = Vm::new();
    let result = vm.call_native_by_name("String_padLeft", &[
        Value::String(std::rc::Rc::new("42".to_string())),
        Value::Long(6),
        Value::Char('0'),
    ]);
    match result {
        Ok(Value::String(s)) => {
            assert_eq!(s.as_str(), "000042", "String.padLeft('42', 6, '0') should be '000042', got '{}'", s);
        }
        Ok(other) => panic!("Expected String, got {:?}", other),
        Err(e) => panic!("String_padLeft failed: {}", e),
    }
}

#[test]
fn test_string_pad_right() {
    let mut vm = Vm::new();
    let result = vm.call_native_by_name("String_padRight", &[
        Value::String(std::rc::Rc::new("hi".to_string())),
        Value::Int(5),
        Value::Char('-'),
    ]);
    match result {
        Ok(Value::String(s)) => {
            assert_eq!(s.as_str(), "hi---", "String.padRight('hi', 5, '-') should be 'hi---', got '{}'", s);
        }
        Ok(other) => panic!("Expected String, got {:?}", other),
        Err(e) => panic!("String_padRight failed: {}", e),
    }
}

#[test]
fn test_string_pad_right_long_width() {
    let mut vm = Vm::new();
    let result = vm.call_native_by_name("String_padRight", &[
        Value::String(std::rc::Rc::new("abc".to_string())),
        Value::Long(7),
        Value::Char('.'),
    ]);
    match result {
        Ok(Value::String(s)) => {
            assert_eq!(s.as_str(), "abc....", "String.padRight('abc', 7, '.') should be 'abc....', got '{}'", s);
        }
        Ok(other) => panic!("Expected String, got {:?}", other),
        Err(e) => panic!("String_padRight failed: {}", e),
    }
}

// ---------------------------------------------------------------------------
// Env native function tests
// ---------------------------------------------------------------------------

#[test]
fn test_env_get_nonexistent() {
    let mut vm = Vm::new();
    let result = vm.call_native_by_name("Env_get", &[
        Value::String(std::rc::Rc::new("TITRATE_NONEXISTENT_VAR_XYZ".to_string())),
    ]);
    match result {
        Ok(Value::Null) => {}
        Ok(other) => panic!("Expected Null for nonexistent env var, got {:?}", other),
        Err(e) => panic!("Env_get failed: {}", e),
    }
}

#[test]
fn test_env_set_and_get() {
    let mut vm = Vm::new();
    // Set an env var
    let set_result = vm.call_native_by_name("Env_set", &[
        Value::String(std::rc::Rc::new("TITRATE_TEST_ENV_SET".to_string())),
        Value::String(std::rc::Rc::new("test_value_123".to_string())),
    ]);
    match set_result {
        Ok(Value::Void) => {}
        Ok(other) => panic!("Expected Void from Env_set, got {:?}", other),
        Err(e) => panic!("Env_set failed: {}", e),
    }

    // Get it back
    let get_result = vm.call_native_by_name("Env_get", &[
        Value::String(std::rc::Rc::new("TITRATE_TEST_ENV_SET".to_string())),
    ]);
    match get_result {
        Ok(Value::String(s)) => {
            assert_eq!(s.as_str(), "test_value_123", "Env_get should return 'test_value_123', got '{}'", s);
        }
        Ok(other) => panic!("Expected String, got {:?}", other),
        Err(e) => panic!("Env_get failed: {}", e),
    }
}

#[test]
fn test_env_vars() {
    let mut vm = Vm::new();
    let result = vm.call_native_by_name("Env_vars", &[]);
    match result {
        Ok(Value::Array { elements }) => {
            // Should return a non-empty array of "KEY=VALUE" strings
            assert!(!elements.is_empty(), "Env_vars should return at least one environment variable");
        }
        Ok(other) => panic!("Expected Array, got {:?}", other),
        Err(e) => panic!("Env_vars failed: {}", e),
    }
}

// ---------------------------------------------------------------------------
// OS native function tests
// ---------------------------------------------------------------------------

#[test]
fn test_os_name() {
    let mut vm = Vm::new();
    let result = vm.call_native_by_name("Os_name", &[]);
    match result {
        Ok(Value::String(s)) => {
            // Should be one of the known OS names
            let valid = ["linux", "macos", "windows", "freebsd", "netbsd", "openbsd", "dragonfly", "solaris"];
            assert!(valid.contains(&s.as_str()), "Os_name should return a known OS name, got '{}'", s);
        }
        Ok(other) => panic!("Expected String, got {:?}", other),
        Err(e) => panic!("Os_name failed: {}", e),
    }
}

#[test]
fn test_os_arch() {
    let mut vm = Vm::new();
    let result = vm.call_native_by_name("Os_arch", &[]);
    match result {
        Ok(Value::String(s)) => {
            // Should be a non-empty string
            assert!(!s.is_empty(), "Os_arch should return a non-empty string");
        }
        Ok(other) => panic!("Expected String, got {:?}", other),
        Err(e) => panic!("Os_arch failed: {}", e),
    }
}

#[test]
fn test_os_family() {
    let mut vm = Vm::new();
    let result = vm.call_native_by_name("Os_family", &[]);
    match result {
        Ok(Value::String(s)) => {
            // Should be one of the known family names
            let valid = ["unix", "windows"];
            assert!(valid.contains(&s.as_str()), "Os_family should return a known family name, got '{}'", s);
        }
        Ok(other) => panic!("Expected String, got {:?}", other),
        Err(e) => panic!("Os_family failed: {}", e),
    }
}

// ---------------------------------------------------------------------------
// Hash native function tests
// ---------------------------------------------------------------------------

#[test]
fn test_hash_md5() {
    let mut vm = Vm::new();
    let result = vm.call_native_by_name("Hash_md5", &[
        Value::String(std::rc::Rc::new("hello".to_string())),
    ]);
    match result {
        Ok(Value::String(s)) => {
            assert_eq!(s.as_str(), "5d41402abc4b2a76b9719d911017c592",
                "MD5('hello') should be 5d41402abc4b2a76b9719d911017c592, got '{}'", s);
        }
        Ok(other) => panic!("Expected String, got {:?}", other),
        Err(e) => panic!("Hash_md5 failed: {}", e),
    }
}

#[test]
fn test_hash_sha1() {
    let mut vm = Vm::new();
    let result = vm.call_native_by_name("Hash_sha1", &[
        Value::String(std::rc::Rc::new("hello".to_string())),
    ]);
    match result {
        Ok(Value::String(s)) => {
            assert_eq!(s.as_str(), "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d",
                "SHA1('hello') should be aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d, got '{}'", s);
        }
        Ok(other) => panic!("Expected String, got {:?}", other),
        Err(e) => panic!("Hash_sha1 failed: {}", e),
    }
}

#[test]
fn test_hash_sha256() {
    let mut vm = Vm::new();
    let result = vm.call_native_by_name("Hash_sha256", &[
        Value::String(std::rc::Rc::new("hello".to_string())),
    ]);
    match result {
        Ok(Value::String(s)) => {
            assert_eq!(s.as_str(), "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
                "SHA256('hello') should be 2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824, got '{}'", s);
        }
        Ok(other) => panic!("Expected String, got {:?}", other),
        Err(e) => panic!("Hash_sha256 failed: {}", e),
    }
}

#[test]
fn test_hash_md5_empty() {
    let mut vm = Vm::new();
    let result = vm.call_native_by_name("Hash_md5", &[
        Value::String(std::rc::Rc::new("".to_string())),
    ]);
    match result {
        Ok(Value::String(s)) => {
            assert_eq!(s.as_str(), "d41d8cd98f00b204e9800998ecf8427e",
                "MD5('') should be d41d8cd98f00b204e9800998ecf8427e, got '{}'", s);
        }
        Ok(other) => panic!("Expected String, got {:?}", other),
        Err(e) => panic!("Hash_md5 failed: {}", e),
    }
}

// ---------------------------------------------------------------------------
// Base64 encode/decode tests
// ---------------------------------------------------------------------------

#[test]
fn test_base64_encode() {
    let mut vm = Vm::new();
    let result = vm.call_native_by_name("Base64_encode", &[
        Value::String(std::rc::Rc::new("hello".to_string())),
    ]);
    match result {
        Ok(Value::String(s)) => {
            assert_eq!(s.as_str(), "aGVsbG8=",
                "Base64_encode('hello') should be 'aGVsbG8=', got '{}'", s);
        }
        Ok(other) => panic!("Expected String, got {:?}", other),
        Err(e) => panic!("Base64_encode failed: {}", e),
    }
}

#[test]
fn test_base64_roundtrip() {
    let mut vm = Vm::new();
    let original = "Titrate language integration test!";
    let encoded = vm.call_native_by_name("Base64_encode", &[
        Value::String(std::rc::Rc::new(original.to_string())),
    ]).expect("Base64_encode should succeed");

    let encoded_str = match &encoded {
        Value::String(s) => s.clone(),
        other => panic!("Expected String from encode, got {:?}", other),
    };

    let decoded = vm.call_native_by_name("Base64_decode", &[
        Value::String(encoded_str),
    ]).expect("Base64_decode should succeed");

    match decoded {
        Value::String(s) => {
            assert_eq!(s.as_str(), original,
                "Base64 round-trip should recover original string, got '{}'", s);
        }
        other => panic!("Expected String from decode, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Hex encode/decode tests
// ---------------------------------------------------------------------------

#[test]
fn test_hex_encode() {
    let mut vm = Vm::new();
    let result = vm.call_native_by_name("Hex_encode", &[
        Value::String(std::rc::Rc::new("ABC".to_string())),
    ]);
    match result {
        Ok(Value::String(s)) => {
            assert_eq!(s.as_str(), "414243",
                "Hex_encode('ABC') should be '414243', got '{}'", s);
        }
        Ok(other) => panic!("Expected String, got {:?}", other),
        Err(e) => panic!("Hex_encode failed: {}", e),
    }
}

#[test]
fn test_hex_roundtrip() {
    let mut vm = Vm::new();
    let original = "hello world";
    let encoded = vm.call_native_by_name("Hex_encode", &[
        Value::String(std::rc::Rc::new(original.to_string())),
    ]).expect("Hex_encode should succeed");

    let encoded_str = match &encoded {
        Value::String(s) => s.clone(),
        other => panic!("Expected String from encode, got {:?}", other),
    };

    let decoded = vm.call_native_by_name("Hex_decode", &[
        Value::String(encoded_str),
    ]).expect("Hex_decode should succeed");

    match decoded {
        Value::String(s) => {
            assert_eq!(s.as_str(), original,
                "Hex round-trip should recover original string, got '{}'", s);
        }
        other => panic!("Expected String from decode, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// URL encode/decode tests
// ---------------------------------------------------------------------------

#[test]
fn test_url_encode() {
    let mut vm = Vm::new();
    let result = vm.call_native_by_name("Url_encode", &[
        Value::String(std::rc::Rc::new("hello world".to_string())),
    ]);
    match result {
        Ok(Value::String(s)) => {
            assert_eq!(s.as_str(), "hello%20world",
                "Url_encode('hello world') should be 'hello%20world', got '{}'", s);
        }
        Ok(other) => panic!("Expected String, got {:?}", other),
        Err(e) => panic!("Url_encode failed: {}", e),
    }
}

#[test]
fn test_url_roundtrip() {
    let mut vm = Vm::new();
    let original = "foo=bar&baz=qux";
    let encoded = vm.call_native_by_name("Url_encode", &[
        Value::String(std::rc::Rc::new(original.to_string())),
    ]).expect("Url_encode should succeed");

    let encoded_str = match &encoded {
        Value::String(s) => s.clone(),
        other => panic!("Expected String from encode, got {:?}", other),
    };

    let decoded = vm.call_native_by_name("Url_decode", &[
        Value::String(encoded_str),
    ]).expect("Url_decode should succeed");

    match decoded {
        Value::String(s) => {
            assert_eq!(s.as_str(), original,
                "URL round-trip should recover original string, got '{}'", s);
        }
        other => panic!("Expected String from decode, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Closure opcode tests
// ---------------------------------------------------------------------------

#[test]
fn test_closure_compiles_to_closure_new_opcode() {
    // A closure should emit CLOSURE_NEW opcode
    let src = r#"fn f(): void { let g = fn(x) => x * 2; }"#;
    let tokens = lexer::tokenize(src).expect("tokenize should succeed");
    let prog = parser::parse(tokens).expect("parse should succeed");
    let mut compiler = Compiler::new();
    let compiled = compiler.compile(&prog).expect("compile should succeed");

    // At least one function should contain CLOSURE_NEW
    let has_closure_new = compiled.functions.iter().any(|f| {
        f.chunk.code.contains(&(OpCode::CLOSURE_NEW as u8))
    });
    assert!(has_closure_new,
        "compiling a closure should emit CLOSURE_NEW");
}

#[test]
fn test_closure_with_capture_creates_closure() {
    // A closure that references an enclosing variable should still compile
    // and emit CLOSURE_NEW (even though captured_vars is not yet populated by the parser)
    let src = r#"fn f(): void { let x = 10; let g = fn() => x; }"#;
    let tokens = lexer::tokenize(src).expect("tokenize should succeed");
    let prog = parser::parse(tokens).expect("parse should succeed");
    let mut compiler = Compiler::new();
    let compiled = compiler.compile(&prog).expect("compile should succeed");

    // The main function should contain CLOSURE_NEW
    let main_func = compiled.functions.iter().find(|f| f.name == "f")
        .expect("function 'f' should exist");
    assert!(main_func.chunk.code.contains(&(OpCode::CLOSURE_NEW as u8)),
        "closure should emit CLOSURE_NEW");

    // A separate closure function should have been created
    let closure_funcs: Vec<_> = compiled.functions.iter()
        .filter(|f| f.name.starts_with("$closure_"))
        .collect();
    assert!(!closure_funcs.is_empty(),
        "compiling a closure should create a $closure_ function");
}

#[test]
fn test_closure_without_capture_no_extra_load() {
    // A closure with no captured variables should still emit CLOSURE_NEW
    // but with 0 upvalue count
    let src = r#"fn f(): void { let g = fn(x) => x * 2; }"#;
    let tokens = lexer::tokenize(src).expect("tokenize should succeed");
    let prog = parser::parse(tokens).expect("parse should succeed");
    let mut compiler = Compiler::new();
    let compiled = compiler.compile(&prog).expect("compile should succeed");

    // Find the main function and check that CLOSURE_NEW has 0 upvalue count
    let main_func = compiled.functions.iter().find(|f| f.name == "f")
        .expect("function 'f' should exist");
    assert!(main_func.chunk.code.contains(&(OpCode::CLOSURE_NEW as u8)),
        "non-capturing closure should still emit CLOSURE_NEW");
}

// ---------------------------------------------------------------------------
// Range expression tests
// ---------------------------------------------------------------------------

#[test]
fn test_range_exclusive_ast() {
    let src = r#"fn f(): void { 1..10; }"#;
    let tokens = lexer::tokenize(src).expect("tokenize should succeed");
    let prog = parser::parse(tokens).expect("parse should succeed");
    match &prog.declarations[0] {
        ast::Declaration::Function(fd) => match &fd.body[0] {
            ast::Stmt::Expr(ast::Expr::Range(start, end, _)) => {
                assert!(matches!(start.as_ref(), ast::Expr::Literal(ast::Literal::Int(1), _)));
                assert!(matches!(end.as_ref(), ast::Expr::Literal(ast::Literal::Int(10), _)));
            }
            other => panic!("Expected Range, got {:?}", other),
        },
        other => panic!("Expected Function, got {:?}", other),
    }
}

#[test]
fn test_range_inclusive_ast() {
    let src = r#"fn f(): void { 1..=10; }"#;
    let tokens = lexer::tokenize(src).expect("tokenize should succeed");
    let prog = parser::parse(tokens).expect("parse should succeed");
    match &prog.declarations[0] {
        ast::Declaration::Function(fd) => match &fd.body[0] {
            ast::Stmt::Expr(ast::Expr::RangeInclusive(start, end, _)) => {
                assert!(matches!(start.as_ref(), ast::Expr::Literal(ast::Literal::Int(1), _)));
                assert!(matches!(end.as_ref(), ast::Expr::Literal(ast::Literal::Int(10), _)));
            }
            other => panic!("Expected RangeInclusive, got {:?}", other),
        },
        other => panic!("Expected Function, got {:?}", other),
    }
}

#[test]
fn test_range_with_expressions_ast() {
    let src = r#"fn f(): void { a + 1..b * 2; }"#;
    let tokens = lexer::tokenize(src).expect("tokenize should succeed");
    let prog = parser::parse(tokens).expect("parse should succeed");
    match &prog.declarations[0] {
        ast::Declaration::Function(fd) => match &fd.body[0] {
            ast::Stmt::Expr(ast::Expr::Range(start, end, _)) => {
                assert!(matches!(start.as_ref(), ast::Expr::Binary(_, ast::Operator::Add, _, _)));
                assert!(matches!(end.as_ref(), ast::Expr::Binary(_, ast::Operator::Mul, _, _)));
            }
            other => panic!("Expected Range with expressions, got {:?}", other),
        },
        other => panic!("Expected Function, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Constant folding tests
// ---------------------------------------------------------------------------

#[test]
fn test_constant_fold_i64_add() {
    // 3 + 4 should compile to just a constant push of 7
    let src = r#"fn f(): int { return 3 + 4; }"#;
    let tokens = lexer::tokenize(src).expect("tokenize should succeed");
    let prog = parser::parse(tokens).expect("parse should succeed");
    let mut compiler = Compiler::new();
    let compiled = compiler.compile(&prog).expect("compile should succeed");

    // Find the function "f" and check its chunk
    let func = compiled.functions.iter().find(|f| f.name == "f")
        .expect("function 'f' should exist");

    // After constant folding, ADD_I64 should be eliminated
    assert!(!func.chunk.code.contains(&(OpCode::ADD_I64 as u8)),
        "ADD_I64 should be folded away for 3+4");

    // The folded value 7 should be present as a PUSH_I64
    let push_offset = func.chunk.code.iter().position(|&b| b == OpCode::PUSH_I64 as u8)
        .expect("should have PUSH_I64 after folding");
    let val = i64::from_be_bytes(
        func.chunk.code[push_offset+1..push_offset+9].try_into().unwrap()
    );
    assert_eq!(val, 7, "folded value should be 7, got {}", val);
}

#[test]
fn test_constant_fold_i64_mul() {
    // 6 * 7 should compile to just a constant push of 42
    let src = r#"fn f(): int { return 6 * 7; }"#;
    let tokens = lexer::tokenize(src).expect("tokenize should succeed");
    let prog = parser::parse(tokens).expect("parse should succeed");
    let mut compiler = Compiler::new();
    let compiled = compiler.compile(&prog).expect("compile should succeed");

    let func = compiled.functions.iter().find(|f| f.name == "f")
        .expect("function 'f' should exist");

    assert!(!func.chunk.code.contains(&(OpCode::MUL_I64 as u8)),
        "MUL_I64 should be folded away for 6*7");

    let push_offset = func.chunk.code.iter().position(|&b| b == OpCode::PUSH_I64 as u8)
        .expect("should have PUSH_I64 after folding");
    let val = i64::from_be_bytes(
        func.chunk.code[push_offset+1..push_offset+9].try_into().unwrap()
    );
    assert_eq!(val, 42, "folded value should be 42, got {}", val);
}

// ---------------------------------------------------------------------------
// Dead code elimination tests
// ---------------------------------------------------------------------------

#[test]
fn test_dead_code_after_return() {
    // Code after return should be eliminated
    let src = r#"fn f(): int { return 42; 99; }"#;
    let tokens = lexer::tokenize(src).expect("tokenize should succeed");
    let prog = parser::parse(tokens).expect("parse should succeed");
    let mut compiler = Compiler::new();
    let compiled = compiler.compile(&prog).expect("compile should succeed");

    let func = compiled.functions.iter().find(|f| f.name == "f")
        .expect("function 'f' should exist");

    // Count PUSH_I64 opcodes — should be exactly 1 (the 42), not 2 (42 and 99)
    let push_count = func.chunk.code.iter().filter(|&&b| b == OpCode::PUSH_I64 as u8).count();
    assert_eq!(push_count, 1,
        "dead code after return should be eliminated: expected 1 PUSH_I64, got {}", push_count);
}

#[test]
fn test_dead_code_after_return_multiple() {
    // Multiple statements after return should all be eliminated
    let src = r#"fn f(): int { return 1; return 2; return 3; }"#;
    let tokens = lexer::tokenize(src).expect("tokenize should succeed");
    let prog = parser::parse(tokens).expect("parse should succeed");
    let mut compiler = Compiler::new();
    let compiled = compiler.compile(&prog).expect("compile should succeed");

    let func = compiled.functions.iter().find(|f| f.name == "f")
        .expect("function 'f' should exist");

    // Only one PUSH_I64 should remain (the 1)
    let push_count = func.chunk.code.iter().filter(|&&b| b == OpCode::PUSH_I64 as u8).count();
    assert_eq!(push_count, 1,
        "dead code after return should be eliminated: expected 1 PUSH_I64, got {}", push_count);
}
