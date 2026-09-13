use super::vm_with_chunk;
    use crate::bytecode::value::Value;
    use crate::bytecode::chunk::Chunk;
    use crate::bytecode::opcodes::{OpCode};
    use crate::bytecode::frame::{FunctionDef};
    use crate::bytecode::vm::Vm;
    use crate::bytecode::vm::natives::json::{native_json_parse, native_json_stringify};
    use crate::bytecode::vm::natives::net::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    // -- New native function tests -------------------------------------------

    #[test]
    fn test_string_to_uppercase() {
        let mut vm = Vm::new();
        let result = vm.call_native_by_name("String_toUpperCase", &[
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
        let up = vm.call_native_by_name("Math_nextUp", std::slice::from_ref(&one)).unwrap();
        let down = vm.call_native_by_name("Math_nextDown", std::slice::from_ref(&one)).unwrap();
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
            Value::Double(d) => assert!((0.0..1.0).contains(&d),
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
        // 2024-01-01 00:00:00 UTC is a Monday (0) — pass epoch_ms
        let result = vm.call_native_by_name("Time_dayOfWeek", &[
            Value::Long(1704067200000),
        ]).unwrap();
        match result {
            Value::Int(d) => assert_eq!(d, 0, "2024-01-01 should be Monday (0), got {}", d),
            other => panic!("Expected Int, got {:?}", other),
        }
    }

    #[test]
    fn test_time_day_of_year() {
        let mut vm = Vm::new();
        // 2024-01-01 is day 1 of the year — pass epoch_ms
        let result = vm.call_native_by_name("Time_dayOfYear", &[
            Value::Long(1704067200000),
        ]).unwrap();
        match result {
            Value::Int(d) => assert_eq!(d, 1, "2024-01-01 should be day 1, got {}", d),
            other => panic!("Expected Int, got {:?}", other),
        }
    }

    #[test]
    // 3.14159 is the fixture input string, not an approximation of PI.
    #[allow(clippy::approx_constant)]
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
        // Test error case - returns NaN per parseOr contract
        let result = vm.call_native_by_name("Double_parseDouble", &[
            Value::String(Rc::new("not_a_number".to_string())),
        ]).unwrap();
        match result {
            Value::Double(d) => assert!(d.is_nan(), "Parsing 'not_a_number' should return NaN, got {}", d),
            other => panic!("Expected Double NaN, got {:?}", other),
        }
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

    // -- test_http_put -----------------------------------------------------------

    #[test]
    fn test_http_put() {
        let result = native_http_put(&[
            Value::String(Rc::new("http://example.com/resource".to_string())),
            Value::String(Rc::new("{\"key\":\"value\"}".to_string())),
            Value::String(Rc::new("application/json".to_string())),
        ]);
        match result {
            Ok(Value::String(s)) => {
                assert!(!s.is_empty(), "HTTP PUT response should not be empty");
            }
            Ok(other) => panic!("Expected String, got {:?}", other),
            Err(_) => {
                eprintln!("Skipping test_http_put: network unavailable");
            }
        }
    }

    #[test]
    fn test_http_put_wrong_args() {
        let result = native_http_put(&[
            Value::String(Rc::new("http://example.com/".to_string())),
        ]);
        assert!(result.is_err(), "Http_put with < 3 args should return error");
    }

    // -- test_http_delete -------------------------------------------------------

    #[test]
    fn test_http_delete() {
        let result = native_http_delete(&[
            Value::String(Rc::new("http://example.com/resource".to_string())),
        ]);
        match result {
            Ok(Value::String(s)) => {
                assert!(!s.is_empty(), "HTTP DELETE response should not be empty");
            }
            Ok(other) => panic!("Expected String, got {:?}", other),
            Err(_) => {
                eprintln!("Skipping test_http_delete: network unavailable");
            }
        }
    }

    #[test]
    fn test_http_delete_wrong_args() {
        let result = native_http_delete(&[]);
        assert!(result.is_err(), "Http_delete with no args should return error");
    }

    // -- test_http_patch --------------------------------------------------------

    #[test]
    fn test_http_patch() {
        let result = native_http_patch(&[
            Value::String(Rc::new("http://example.com/resource".to_string())),
            Value::String(Rc::new("{\"key\":\"patched\"}".to_string())),
            Value::String(Rc::new("application/json".to_string())),
        ]);
        match result {
            Ok(Value::String(s)) => {
                assert!(!s.is_empty(), "HTTP PATCH response should not be empty");
            }
            Ok(other) => panic!("Expected String, got {:?}", other),
            Err(_) => {
                eprintln!("Skipping test_http_patch: network unavailable");
            }
        }
    }

    #[test]
    fn test_http_patch_wrong_args() {
        let result = native_http_patch(&[
            Value::String(Rc::new("http://example.com/".to_string())),
        ]);
        assert!(result.is_err(), "Http_patch with < 3 args should return error");
    }

    // -- test_http_head ---------------------------------------------------------

    #[test]
    fn test_http_head() {
        let result = native_http_head(&[
            Value::String(Rc::new("http://example.com/".to_string())),
        ]);
        match result {
            Ok(Value::String(_s)) => {
                // HEAD response body is typically empty, but the function returns a String
            }
            Ok(other) => panic!("Expected String, got {:?}", other),
            Err(_) => {
                eprintln!("Skipping test_http_head: network unavailable");
            }
        }
    }

    #[test]
    fn test_http_head_wrong_args() {
        let result = native_http_head(&[]);
        assert!(result.is_err(), "Http_head with no args should return error");
    }

    // -- test_json_stringify ----------------------------------------------------

    #[test]
    fn test_json_stringify_null() {
        let result = native_json_stringify(&[Value::Null]).unwrap();
        assert_eq!(result, Value::String(Rc::new("null".to_string())));
    }

    #[test]
    fn test_json_stringify_bool() {
        let result_true = native_json_stringify(&[Value::Bool(true)]).unwrap();
        assert_eq!(result_true, Value::String(Rc::new("true".to_string())));

        let result_false = native_json_stringify(&[Value::Bool(false)]).unwrap();
        assert_eq!(result_false, Value::String(Rc::new("false".to_string())));
    }

    #[test]
    fn test_json_stringify_int() {
        let result = native_json_stringify(&[Value::Int(42)]).unwrap();
        assert_eq!(result, Value::String(Rc::new("42".to_string())));
    }

    #[test]
    fn test_json_stringify_long() {
        let result = native_json_stringify(&[Value::Long(123456789)]).unwrap();
        assert_eq!(result, Value::String(Rc::new("123456789".to_string())));
    }

    #[test]
    // 3.14 is intentional round-trip fixture data, not an approximation.
    #[allow(clippy::approx_constant)]
    fn test_json_stringify_double() {
        let result = native_json_stringify(&[Value::Double(3.14)]).unwrap();
        match result {
            Value::String(s) => {
                let parsed: f64 = s.parse().unwrap();
                assert!((parsed - 3.14).abs() < 0.001, "Expected ~3.14, got {}", s);
            }
            other => panic!("Expected String, got {:?}", other),
        }
    }

    #[test]
    fn test_json_stringify_string() {
        let result = native_json_stringify(&[Value::String(Rc::new("hello".to_string()))]).unwrap();
        assert_eq!(result, Value::String(Rc::new("\"hello\"".to_string())));
    }

    #[test]
    fn test_json_stringify_string_with_escapes() {
        let result = native_json_stringify(&[Value::String(Rc::new("line1\nline2\ttab".to_string()))]).unwrap();
        assert_eq!(result, Value::String(Rc::new("\"line1\\nline2\\ttab\"".to_string())));
    }

    #[test]
    fn test_json_stringify_array() {
        let result = native_json_stringify(&[Value::Array {
            elements: vec![Value::Long(1), Value::Long(2), Value::Long(3)],
        }]).unwrap();
        assert_eq!(result, Value::String(Rc::new("[1, 2, 3]".to_string())));
    }

    #[test]
    fn test_json_stringify_hashmap() {
        let mut fields = HashMap::new();
        fields.insert("_keys".to_string(), Value::Array {
            elements: vec![Value::String(Rc::new("name".to_string()))],
        });
        fields.insert("_values".to_string(), Value::Array {
            elements: vec![Value::String(Rc::new("Alice".to_string()))],
        });
        let hashmap = Value::ClassInstance {
            class_name: "HashMap".to_string(),
            fields: Rc::new(RefCell::new(fields)),
            vtable: HashMap::new(),
        };
        let result = native_json_stringify(&[hashmap]).unwrap();
        match result {
            Value::String(s) => {
                assert!(s.contains("\"name\""), "Should contain key 'name', got {}", s);
                assert!(s.contains("\"Alice\""), "Should contain value 'Alice', got {}", s);
            }
            other => panic!("Expected String, got {:?}", other),
        }
    }

    #[test]
    fn test_json_stringify_class_instance() {
        let mut fields = HashMap::new();
        fields.insert("x".to_string(), Value::Int(10));
        fields.insert("y".to_string(), Value::Int(20));
        let instance = Value::ClassInstance {
            class_name: "Point".to_string(),
            fields: Rc::new(RefCell::new(fields)),
            vtable: HashMap::new(),
        };
        let result = native_json_stringify(&[instance]).unwrap();
        match result {
            Value::String(s) => {
                assert!(s.contains("\"x\": 10"), "Should contain field x, got {}", s);
                assert!(s.contains("\"y\": 20"), "Should contain field y, got {}", s);
            }
            other => panic!("Expected String, got {:?}", other),
        }
    }

    #[test]
    fn test_json_stringify_no_args() {
        let result = native_json_stringify(&[]);
        assert!(result.is_err(), "Json_stringify with no args should return error");
    }

    #[test]
    fn test_json_stringify_roundtrip() {
        // Parse then stringify should produce equivalent JSON
        let original = "{\"key\": \"value\"}";
        let parsed = native_json_parse(&[Value::String(Rc::new(original.to_string()))]).unwrap();
        let stringified = native_json_stringify(&[parsed]).unwrap();
        match stringified {
            Value::String(s) => {
                assert!(s.contains("\"key\""), "Roundtrip should contain key, got {}", s);
                assert!(s.contains("\"value\""), "Roundtrip should contain value, got {}", s);
            }
            other => panic!("Expected String, got {:?}", other),
        }
    }

    // -- test_vm_stack_overflow ---------------------------------------------------

    #[test]
    fn test_vm_stack_overflow() {
        // Build a recursive function: fn recurse() { recurse() }
        // Function 0 (main): calls recurse (function 1)
        // Function 1 (recurse): calls itself with no args

        // Build the recurse function chunk
        let mut recurse_chunk = Chunk::new();
        // CALL function 1 (recurse itself), 0 args
        recurse_chunk.write_opcode(OpCode::CALL, 1);
        recurse_chunk.write_u16(1, 1); // func_idx = 1
        recurse_chunk.write_u8(0, 1);  // arg_count = 0
        // RET
        recurse_chunk.write_opcode(OpCode::RET, 1);

        // Build the main function chunk
        let mut main_chunk = Chunk::new();
        // CALL function 1 (recurse), 0 args
        main_chunk.write_opcode(OpCode::CALL, 1);
        main_chunk.write_u16(1, 1); // func_idx = 1
        main_chunk.write_u8(0, 1);  // arg_count = 0
        // RET
        main_chunk.write_opcode(OpCode::RET, 1);

        let mut vm = Vm::new();
        // Add main as function 0
        vm.add_function(FunctionDef {
            name: "main".to_string(),
            arity: 0,
            chunk: main_chunk,
            is_method: false,
            is_constructor: false,
            local_count: 0,
        param_types: Vec::new(),
        });
        // Add recurse as function 1
        vm.add_function(FunctionDef {
            name: "recurse".to_string(),
            arity: 0,
            chunk: recurse_chunk,
            is_method: false,
            is_constructor: false,
            local_count: 0,
        param_types: Vec::new(),
        });

        // Set a low max call depth for testing
        vm.set_max_call_depth(10);

        let result = vm.run();
        match result {
            Err(msg) => assert!(msg.starts_with("Stack overflow: maximum call depth exceeded"),
                "Expected stack overflow error, got: {}", msg),
            Ok(()) => panic!("Expected stack overflow error but execution succeeded"),
        }
    }

    // -- C.1: opcode Value variant coverage & structured error tests -----------

    #[test]
    fn test_array_set_with_int_index() {
        let mut chunk = Chunk::new();
        // Stack order for ARRAY_SET: [value, array, index]
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&99i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&10i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&20i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&30i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::ARRAY_NEW, 1);
        chunk.write_u16(3, 1);
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&1i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::ARRAY_SET, 1);
        chunk.write_opcode(OpCode::RET, 1);
        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        match vm.stack.last() {
            Some(Value::Array { elements }) => {
                assert_eq!(elements.len(), 3);
                assert_eq!(elements[1], Value::Int(99));
            }
            other => panic!("Expected Array, got {:?}", other),
        }
    }

    #[test]
    fn test_array_set_with_long_index() {
        let mut chunk = Chunk::new();
        // Stack order for ARRAY_SET: [value, array, index]
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&77i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&10i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&20i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::ARRAY_NEW, 1);
        chunk.write_u16(2, 1);
        chunk.write_opcode(OpCode::PUSH_I64, 1);
        chunk.code.extend_from_slice(&0i64.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 8]);
        chunk.write_opcode(OpCode::ARRAY_SET, 1);
        chunk.write_opcode(OpCode::RET, 1);
        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        match vm.stack.last() {
            Some(Value::Array { elements }) => {
                assert_eq!(elements.len(), 2);
                assert_eq!(elements[0], Value::Int(77));
            }
            other => panic!("Expected Array, got {:?}", other),
        }
    }

    #[test]
    fn test_array_set_invalid_index_type() {
        let mut chunk = Chunk::new();
        // Stack order for ARRAY_SET: [value, array, index]
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&42i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&10i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::ARRAY_NEW, 1);
        chunk.write_u16(1, 1);
        chunk.write_opcode(OpCode::PUSH_NULL, 1);
        chunk.write_opcode(OpCode::ARRAY_SET, 1);
        chunk.write_opcode(OpCode::RET, 1);
        let mut vm = vm_with_chunk(chunk);
        let result = vm.run();
        assert!(result.is_err(), "ARRAY_SET with Null index should error");
        let err = result.unwrap_err();
        assert!(err.contains("ARRAY_SET"), "Error should mention ARRAY_SET, got: {}", err);
    }

    #[test]
    fn test_array_set_out_of_bounds() {
        let mut chunk = Chunk::new();
        // Stack order for ARRAY_SET: [value, array, index]
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&99i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&10i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::ARRAY_NEW, 1);
        chunk.write_u16(1, 1);
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&5i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::ARRAY_SET, 1);
        chunk.write_opcode(OpCode::RET, 1);
        let mut vm = vm_with_chunk(chunk);
        let result = vm.run();
        assert!(result.is_err(), "ARRAY_SET out of bounds should error");
        let err = result.unwrap_err();
        assert!(err.contains("out of bounds"), "Error should mention out of bounds, got: {}", err);
    }

    #[test]
    fn test_add_i32_type_mismatch() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&10i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::PUSH_NULL, 1);
        chunk.write_opcode(OpCode::ADD_I32, 1);
        chunk.write_opcode(OpCode::RET, 1);
        let mut vm = vm_with_chunk(chunk);
        let result = vm.run();
        assert!(result.is_err(), "ADD_I32 with Null should error");
        let err = result.unwrap_err();
        assert!(err.contains("ADD_I32"), "Error should mention ADD_I32, got: {}", err);
    }

    #[test]
    fn test_add_f64_type_mismatch() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::PUSH_F64, 1);
        chunk.code.extend_from_slice(&1.5f64.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 8]);
        chunk.write_opcode(OpCode::PUSH_BOOL, 1);
        chunk.write_u8(1, 1);
        chunk.write_opcode(OpCode::ADD_F64, 1);
        chunk.write_opcode(OpCode::RET, 1);
        let mut vm = vm_with_chunk(chunk);
        let result = vm.run();
        assert!(result.is_err(), "ADD_F64 with Bool should error");
        let err = result.unwrap_err();
        assert!(err.contains("ADD_F64"), "Error should mention ADD_F64, got: {}", err);
    }

    #[test]
    fn test_lt_f64_type_mismatch() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::PUSH_F64, 1);
        chunk.code.extend_from_slice(&1.5f64.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 8]);
        chunk.write_opcode(OpCode::PUSH_STRING, 1);
        chunk.write_u16(0, 1);
        chunk.strings.push("hello".to_string());
        chunk.write_opcode(OpCode::LT_F64, 1);
        chunk.write_opcode(OpCode::RET, 1);
        let mut vm = vm_with_chunk(chunk);
        let result = vm.run();
        assert!(result.is_err(), "LT_F64 with String should error");
        let err = result.unwrap_err();
        assert!(err.contains("LT_F64"), "Error should mention LT_F64, got: {}", err);
    }

    #[test]
    fn test_div_i32_by_zero() {
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
        let result = vm.run();
        assert!(result.is_err(), "DIV_I32 by zero should error");
        let err = result.unwrap_err();
        assert!(err.contains("Division by zero"), "Error should mention division by zero, got: {}", err);
    }

    #[test]
    fn test_deref_non_ref() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&42i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::DEREF, 1);
        chunk.write_opcode(OpCode::RET, 1);
        let mut vm = vm_with_chunk(chunk);
        let result = vm.run();
        assert!(result.is_err(), "DEREF on non-Ref should error");
        let err = result.unwrap_err();
        assert!(err.contains("DEREF"), "Error should mention DEREF, got: {}", err);
    }

    #[test]
    fn test_unbox_non_owned() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&42i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::UNBOX_VALUE, 1);
        chunk.write_opcode(OpCode::RET, 1);
        let mut vm = vm_with_chunk(chunk);
        let result = vm.run();
        assert!(result.is_err(), "UNBOX_VALUE on non-Owned should error");
        let err = result.unwrap_err();
        assert!(err.contains("UNBOX_VALUE"), "Error should mention UNBOX_VALUE, got: {}", err);
    }

    #[test]
    fn test_array_len_on_non_array() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&42i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::ARRAY_LEN, 1);
        chunk.write_opcode(OpCode::RET, 1);
        let mut vm = vm_with_chunk(chunk);
        let result = vm.run();
        assert!(result.is_err(), "ARRAY_LEN on non-Array should error");
        let err = result.unwrap_err();
        assert!(err.contains("ARRAY_LEN"), "Error should mention ARRAY_LEN, got: {}", err);
    }

    #[test]
    fn test_unwrap_or_propagate_on_non_result() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&42i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::UNWRAP_OR_PROPAGATE, 1);
        chunk.write_opcode(OpCode::RET, 1);
        let mut vm = vm_with_chunk(chunk);
        let result = vm.run();
        assert!(result.is_err(), "UNWRAP_OR_PROPAGATE on non-Result should error");
        let err = result.unwrap_err();
        assert!(err.contains("UNWRAP_OR_PROPAGATE"), "Error should mention UNWRAP_OR_PROPAGATE, got: {}", err);
    }

    #[test]
    fn test_eq_string_type_mismatch() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::PUSH_STRING, 1);
        chunk.write_u16(0, 1);
        chunk.strings.push("hello".to_string());
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&42i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::EQ_STRING, 1);
        chunk.write_opcode(OpCode::RET, 1);
        let mut vm = vm_with_chunk(chunk);
        let result = vm.run();
        assert!(result.is_err(), "EQ_STRING with Int should error");
        let err = result.unwrap_err();
        assert!(err.contains("EQ_STRING"), "Error should mention EQ_STRING, got: {}", err);
    }

    #[test]
    fn test_neg_i32_type_mismatch() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::PUSH_NULL, 1);
        chunk.write_opcode(OpCode::NEG_I32, 1);
        chunk.write_opcode(OpCode::RET, 1);
        let mut vm = vm_with_chunk(chunk);
        let result = vm.run();
        assert!(result.is_err(), "NEG_I32 on Null should error");
        let err = result.unwrap_err();
        assert!(err.contains("NEG_I32"), "Error should mention NEG_I32, got: {}", err);
    }

    #[test]
    fn test_str_concat_type_mismatch() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::PUSH_STRING, 1);
        chunk.write_u16(0, 1);
        chunk.strings.push("hello".to_string());
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&42i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::STR_CONCAT, 1);
        chunk.write_opcode(OpCode::RET, 1);
        let mut vm = vm_with_chunk(chunk);
        let result = vm.run();
        assert!(result.is_ok(), "STR_CONCAT with Int should now succeed (mixed-type concat), got: {:?}", result.err());
    }
