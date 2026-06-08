use trc::bytecode::Vm;
use trc::bytecode::value::Value;

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
