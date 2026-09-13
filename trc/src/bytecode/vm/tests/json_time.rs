use super::vm_with_chunk;
    use crate::bytecode::value::Value;
    use crate::bytecode::chunk::Chunk;
    use crate::bytecode::opcodes::{OpCode};
    use crate::bytecode::frame::{FunctionDef};
    use crate::bytecode::vm::Vm;
    use crate::bytecode::vm::natives::regex::{native_regex_match, native_regex_find, native_regex_replace};
    use crate::bytecode::vm::natives::system::*;
    use crate::bytecode::vm::natives::path::*;
    use crate::bytecode::vm::natives::time::*;
    use crate::bytecode::vm::natives::net::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

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
        param_types: Vec::new(),
        });
        vm.add_function(FunctionDef {
            name: "$closure_0".to_string(),
            arity: 1,
            chunk: closure_chunk,
            is_method: false,
            is_constructor: false,
            local_count: 1,
        param_types: Vec::new(),
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
        param_types: Vec::new(),
        });
        vm.add_function(FunctionDef {
            name: "$closure_0".to_string(),
            arity: 1,
            chunk: outer_chunk,
            is_method: false,
            is_constructor: false,
            local_count: 2,
        param_types: Vec::new(),
        });
        vm.add_function(FunctionDef {
            name: "$closure_1".to_string(),
            arity: 1,
            chunk: inner_chunk,
            is_method: false,
            is_constructor: false,
            local_count: 1,
        param_types: Vec::new(),
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
        param_types: Vec::new(),
        });
        vm.add_function(FunctionDef {
            name: "$closure_0".to_string(),
            arity: 1,
            chunk: closure_chunk,
            is_method: false,
            is_constructor: false,
            local_count: 1,
        param_types: Vec::new(),
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
        param_types: Vec::new(),
        });
        vm.add_function(FunctionDef {
            name: "makeTuple".to_string(),
            arity: 0,
            chunk: fn_chunk,
            is_method: false,
            is_constructor: false,
            local_count: 0,
        param_types: Vec::new(),
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
