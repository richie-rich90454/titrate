use super::vm_with_chunk;
    use crate::bytecode::value::Value;
    use crate::bytecode::chunk::Chunk;
    use crate::bytecode::opcodes::{OpCode};
    use crate::bytecode::frame::{FunctionDef};
    use crate::bytecode::vm::Vm;
    use crate::bytecode::vm::natives::string::*;
    use std::rc::Rc;

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
        param_types: Vec::new(),
        });

        vm.run().unwrap();

        // The top of the stack should be a Closure value
        match vm.stack.last() {
            Some(Value::Closure { func_idx, upvalues }) => {
                assert_eq!(*func_idx, 1, "closure should reference function index 1");
                assert_eq!(upvalues.len(), 2, "closure should have 2 upvalues");
                assert_eq!(*upvalues[0].borrow(), Value::Int(10), "first upvalue should be Int(10)");
                assert_eq!(*upvalues[1].borrow(), Value::Int(20), "second upvalue should be Int(20)");
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
        param_types: Vec::new(),
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
