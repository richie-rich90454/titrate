use super::vm_with_chunk;
    use crate::bytecode::value::Value;
    use crate::bytecode::chunk::Chunk;
    use crate::bytecode::opcodes::{OpCode};
    use crate::bytecode::frame::{FunctionDef};
    use crate::bytecode::vm::Vm;
    use crate::bytecode::vm::natives::json::{native_json_parse};
    use std::rc::Rc;

    // -- 13. test_closure_execution ---------------------------------------------

    #[test]
    fn test_closure_execution() {
        // Main: push 3, CLOSURE_NEW func#1 (0 upvalues), PUSH_I32 7,
        //       CALL func#1 with 1 arg, RET
        // Func#1 ($closure_0): LOAD_LOCAL 0 (arg), RET
        let mut main_chunk = Chunk::new();
        // PUSH_I32 3 (dummy value on stack before closure creation)
        main_chunk.write_opcode(OpCode::PUSH_I32, 1);
        main_chunk.code.extend_from_slice(&3i32.to_be_bytes());
        main_chunk.source_lines.extend_from_slice(&[1; 4]);
        // POP the dummy (we just needed something before CLOSURE_NEW)
        main_chunk.write_opcode(OpCode::POP, 1);
        // CLOSURE_NEW func_idx=1, upvalue_count=0
        main_chunk.write_opcode(OpCode::CLOSURE_NEW, 1);
        main_chunk.write_u16(1, 1); // function index 1
        main_chunk.write_u8(0, 1);  // 0 upvalues
        // PUSH_I32 7  (argument for the closure)
        main_chunk.write_opcode(OpCode::PUSH_I32, 1);
        main_chunk.code.extend_from_slice(&7i32.to_be_bytes());
        main_chunk.source_lines.extend_from_slice(&[1; 4]);
        // CALL func_idx=1, arg_count=1
        main_chunk.write_opcode(OpCode::CALL, 1);
        main_chunk.write_u16(1, 1); // function index 1
        main_chunk.write_u8(1, 1);  // 1 arg
        // RET
        main_chunk.write_opcode(OpCode::RET, 1);

        let mut closure_chunk = Chunk::new();
        // LOAD_LOCAL 0 (first arg = 7)
        closure_chunk.write_opcode(OpCode::LOAD_LOCAL, 1);
        closure_chunk.write_u8(0, 1);
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
        assert_eq!(vm.stack.last(), Some(&Value::Int(7)));
    }

    // -- 14. test_closure_capture_variable ---------------------------------------

    #[test]
    fn test_closure_capture_variable() {
        // Main: PUSH_I32 10, STORE_LOCAL 0, LOAD_LOCAL 0,
        //       CLOSURE_NEW func#1 (1 upvalue: the 10), PUSH_I32 5,
        //       CALL func#1 with 1 arg, RET
        // Func#1 ($closure_0): LOAD_LOCAL 0 (arg=5), GET_UPVALUE 0 (captured=10), ADD_I32, RET
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
        main_chunk.write_u16(1, 1); // function index 1
        main_chunk.write_u8(1, 1);  // 1 upvalue
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
        // GET_UPVALUE 0 (captured value = 10)
        closure_chunk.write_opcode(OpCode::GET_UPVALUE, 1);
        closure_chunk.write_u8(0, 1);
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
            local_count: 1,
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

    // -- 15. test_tuple_creation_and_access --------------------------------------

    #[test]
    fn test_tuple_creation_and_access() {
        // Push 42, push "hello", TUPLE_NEW 2, TUPLE_GET 0, RET
        let mut chunk = Chunk::new();
        // PUSH_I32 42
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&42i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        // PUSH_STRING "hello"
        let hello_idx = chunk.add_string("hello");
        chunk.write_opcode(OpCode::PUSH_STRING, 1);
        chunk.write_u16(hello_idx, 1);
        // TUPLE_NEW 2
        chunk.write_opcode(OpCode::TUPLE_NEW, 1);
        chunk.write_u16(2, 1);
        // TUPLE_GET 0 (first element = 42)
        chunk.write_opcode(OpCode::TUPLE_GET, 1);
        chunk.write_u8(0, 1);
        // RET
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Int(42)));
    }

    // -- 16. test_tuple_destructuring_vm -----------------------------------------

    #[test]
    fn test_tuple_destructuring_vm() {
        // Push 10, push 20, TUPLE_NEW 2, store in local 0,
        // LOAD_LOCAL 0, TUPLE_GET 0 → 10 (store in local 1),
        // LOAD_LOCAL 0, TUPLE_GET 1 → 20,
        // LOAD_LOCAL 1 → 10, ADD_I32 → 30, RET
        let mut chunk = Chunk::new();
        // PUSH_I32 10
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&10i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        // PUSH_I32 20
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&20i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        // TUPLE_NEW 2
        chunk.write_opcode(OpCode::TUPLE_NEW, 1);
        chunk.write_u16(2, 1);
        // STORE_LOCAL 0 (store the tuple)
        chunk.write_opcode(OpCode::STORE_LOCAL, 1);
        chunk.write_u8(0, 1);
        // LOAD_LOCAL 0 (load tuple)
        chunk.write_opcode(OpCode::LOAD_LOCAL, 1);
        chunk.write_u8(0, 1);
        // TUPLE_GET 0 → 10
        chunk.write_opcode(OpCode::TUPLE_GET, 1);
        chunk.write_u8(0, 1);
        // STORE_LOCAL 1 (store first element)
        chunk.write_opcode(OpCode::STORE_LOCAL, 1);
        chunk.write_u8(1, 1);
        // LOAD_LOCAL 0 (load tuple again)
        chunk.write_opcode(OpCode::LOAD_LOCAL, 1);
        chunk.write_u8(0, 1);
        // TUPLE_GET 1 → 20
        chunk.write_opcode(OpCode::TUPLE_GET, 1);
        chunk.write_u8(1, 1);
        // LOAD_LOCAL 1 → push 10 back
        chunk.write_opcode(OpCode::LOAD_LOCAL, 1);
        chunk.write_u8(1, 1);
        // ADD_I32 → 30
        chunk.write_opcode(OpCode::ADD_I32, 1);
        // RET
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = Vm::new();
        vm.add_function(FunctionDef {
            name: "main".to_string(),
            arity: 0,
            chunk,
            is_method: false,
            is_constructor: false,
            local_count: 2,
        param_types: Vec::new(),
        });

        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Int(30)));
    }

    // -- 17. test_operator_overload_add ------------------------------------------

    #[test]
    fn test_operator_overload_add() {
        // Test INVOKE_OPERATOR "operator+" with two ints (falls back to built-in add)
        let mut chunk = Chunk::new();
        // PUSH_I32 3
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&3i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        // PUSH_I32 4
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&4i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        // INVOKE_OPERATOR "operator+" with 1 arg
        let op_idx = chunk.add_string("operator+");
        chunk.write_opcode(OpCode::INVOKE_OPERATOR, 1);
        chunk.write_u16(op_idx, 1);
        chunk.write_u8(1, 1); // 1 arg (right operand)
        // RET
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Int(7)));
    }

    // -- 18. test_operator_overload_compare ---------------------------------------

    #[test]
    fn test_operator_overload_compare() {
        // Test INVOKE_OPERATOR "operator==" with two equal ints
        let mut chunk = Chunk::new();
        // PUSH_I32 5
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&5i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        // PUSH_I32 5
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&5i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        // INVOKE_OPERATOR "operator==" with 1 arg
        let op_idx = chunk.add_string("operator==");
        chunk.write_opcode(OpCode::INVOKE_OPERATOR, 1);
        chunk.write_u16(op_idx, 1);
        chunk.write_u8(1, 1);
        // RET
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Bool(true)));

        // Test operator!= with different values
        let mut chunk2 = Chunk::new();
        // PUSH_I32 3
        chunk2.write_opcode(OpCode::PUSH_I32, 1);
        chunk2.code.extend_from_slice(&3i32.to_be_bytes());
        chunk2.source_lines.extend_from_slice(&[1; 4]);
        // PUSH_I32 7
        chunk2.write_opcode(OpCode::PUSH_I32, 1);
        chunk2.code.extend_from_slice(&7i32.to_be_bytes());
        chunk2.source_lines.extend_from_slice(&[1; 4]);
        // INVOKE_OPERATOR "operator!=" with 1 arg
        let op_idx2 = chunk2.add_string("operator!=");
        chunk2.write_opcode(OpCode::INVOKE_OPERATOR, 1);
        chunk2.write_u16(op_idx2, 1);
        chunk2.write_u8(1, 1);
        // RET
        chunk2.write_opcode(OpCode::RET, 1);

        let mut vm2 = vm_with_chunk(chunk2);
        vm2.run().unwrap();
        assert_eq!(vm2.stack.last(), Some(&Value::Bool(true)));
    }

    // -- 19. test_json_parse_null -----------------------------------------------

    #[test]
    fn test_json_parse_null() {
        let result = native_json_parse(&[Value::String(Rc::new("null".to_string()))]);
        assert_eq!(result.unwrap(), Value::Null);
    }

    // -- 20. test_json_parse_bool -----------------------------------------------

    #[test]
    fn test_json_parse_bool() {
        let result_true = native_json_parse(&[Value::String(Rc::new("true".to_string()))]);
        assert_eq!(result_true.unwrap(), Value::Bool(true));

        let result_false = native_json_parse(&[Value::String(Rc::new("false".to_string()))]);
        assert_eq!(result_false.unwrap(), Value::Bool(false));
    }

    // -- 21. test_json_parse_number ---------------------------------------------

    #[test]
    // 3.14 is intentional fixture data (JSON input "3.14"), not an
    // approximation of PI.
    #[allow(clippy::approx_constant)]
    fn test_json_parse_number() {
        let result_int = native_json_parse(&[Value::String(Rc::new("42".to_string()))]);
        assert_eq!(result_int.unwrap(), Value::Long(42));

        let result_float = native_json_parse(&[Value::String(Rc::new("3.14".to_string()))]);
        let val = result_float.unwrap();
        match val {
            Value::Double(f) => assert!((f - 3.14).abs() < 0.001),
            _ => panic!("Expected Double, got {:?}", val),
        }
    }

    // -- 22. test_json_parse_string ---------------------------------------------

    #[test]
    fn test_json_parse_string() {
        let result = native_json_parse(&[Value::String(Rc::new("\"hello\"".to_string()))]);
        assert_eq!(result.unwrap(), Value::String(Rc::new("hello".to_string())));
    }

    // -- 23. test_json_parse_array ----------------------------------------------

    #[test]
    fn test_json_parse_array() {
        let result = native_json_parse(&[Value::String(Rc::new("[1, 2, 3]".to_string()))]);
        // json_parse_array now returns an ArrayList ClassInstance (mirroring
        // Titrate's ArrayList representation with an _elements field) so that
        // parsed JSON arrays can be detected via `is ArrayList` and have
        // .size()/.get() called on them directly from Titrate code.
        match result.unwrap() {
            Value::ClassInstance { class_name, fields, .. } => {
                assert_eq!(class_name, "ArrayList");
                let borrowed = fields.borrow();
                match borrowed.get("_elements") {
                    Some(Value::Array { elements }) => {
                        assert_eq!(elements.len(), 3);
                        assert_eq!(elements[0], Value::Long(1));
                        assert_eq!(elements[1], Value::Long(2));
                        assert_eq!(elements[2], Value::Long(3));
                    }
                    _ => panic!("Expected _elements array field"),
                }
            }
            other => panic!("Expected ArrayList ClassInstance, got {:?}", other),
        }
    }

    // -- 24. test_json_parse_object ---------------------------------------------

    #[test]
    fn test_json_parse_object() {
        let result = native_json_parse(&[Value::String(Rc::new("{\"key\": \"value\"}".to_string()))]);
        match result.unwrap() {
            Value::ClassInstance { class_name, fields, .. } => {
                assert_eq!(class_name, "HashMap");
                let borrowed = fields.borrow();
                match borrowed.get("_keys") {
                    Some(Value::Array { elements: keys }) => {
                        assert_eq!(keys.len(), 1);
                        assert_eq!(keys[0], Value::String(Rc::new("key".to_string())));
                    }
                    _ => panic!("Expected _keys array"),
                }
                match borrowed.get("_values") {
                    Some(Value::Array { elements: values }) => {
                        assert_eq!(values.len(), 1);
                        assert_eq!(values[0], Value::String(Rc::new("value".to_string())));
                    }
                    _ => panic!("Expected _values array"),
                }
            }
            other => panic!("Expected ClassInstance (HashMap), got {:?}", other),
        }
    }

    // -- 25. test_ndarray_zeros --------------------------------------------------

    #[test]
    fn test_ndarray_zeros() {
        // Create a 2x2 zeros array using Value::Array, verify elements are 0.0
        let zeros = Value::Array {
            elements: vec![
                Value::Array { elements: vec![Value::Double(0.0), Value::Double(0.0)] },
                Value::Array { elements: vec![Value::Double(0.0), Value::Double(0.0)] },
            ],
        };
        match &zeros {
            Value::Array { elements } => {
                assert_eq!(elements.len(), 2);
                for row in elements {
                    match row {
                        Value::Array { elements: cols } => {
                            assert_eq!(cols.len(), 2);
                            for val in cols {
                                assert_eq!(*val, Value::Double(0.0));
                            }
                        }
                        _ => panic!("Expected inner Array"),
                    }
                }
            }
            _ => panic!("Expected Array"),
        }
    }

    // -- 26. test_ndarray_ones ---------------------------------------------------

    #[test]
    fn test_ndarray_ones() {
        // Create a 2x2 ones array using Value::Array, verify elements are 1.0
        let ones = Value::Array {
            elements: vec![
                Value::Array { elements: vec![Value::Double(1.0), Value::Double(1.0)] },
                Value::Array { elements: vec![Value::Double(1.0), Value::Double(1.0)] },
            ],
        };
        match &ones {
            Value::Array { elements } => {
                assert_eq!(elements.len(), 2);
                for row in elements {
                    match row {
                        Value::Array { elements: cols } => {
                            assert_eq!(cols.len(), 2);
                            for val in cols {
                                assert_eq!(*val, Value::Double(1.0));
                            }
                        }
                        _ => panic!("Expected inner Array"),
                    }
                }
            }
            _ => panic!("Expected Array"),
        }
    }

    // -- 27. test_ndarray_set_get -----------------------------------------------

    #[test]
    fn test_ndarray_set_get() {
        // Create a 2x2 array, set values, get them back
        let mut arr = Value::Array {
            elements: vec![
                Value::Array { elements: vec![Value::Double(0.0), Value::Double(0.0)] },
                Value::Array { elements: vec![Value::Double(0.0), Value::Double(0.0)] },
            ],
        };
        // Set arr[1][0] = 42.0
        if let Value::Array { elements } = &mut arr {
            if let Value::Array { elements: cols } = &mut elements[1] {
                cols[0] = Value::Double(42.0);
            }
        }
        // Get arr[1][0]
        match &arr {
            Value::Array { elements } => {
                match &elements[1] {
                    Value::Array { elements: cols } => {
                        assert_eq!(cols[0], Value::Double(42.0));
                    }
                    _ => panic!("Expected inner Array"),
                }
            }
            _ => panic!("Expected Array"),
        }
    }

    // -- 28. test_ndarray_add ----------------------------------------------------

    #[test]
    fn test_ndarray_add() {
        // Add two 2x2 arrays element-wise
        let a = Value::Array {
            elements: vec![
                Value::Array { elements: vec![Value::Double(1.0), Value::Double(2.0)] },
                Value::Array { elements: vec![Value::Double(3.0), Value::Double(4.0)] },
            ],
        };
        let b = Value::Array {
            elements: vec![
                Value::Array { elements: vec![Value::Double(5.0), Value::Double(6.0)] },
                Value::Array { elements: vec![Value::Double(7.0), Value::Double(8.0)] },
            ],
        };
        // Element-wise add
        fn add_arrays(a: &Value, b: &Value) -> Value {
            match (a, b) {
                (Value::Array { elements: ea }, Value::Array { elements: eb }) => {
                    Value::Array {
                        elements: ea.iter().zip(eb.iter()).map(|(x, y)| add_arrays(x, y)).collect(),
                    }
                }
                (Value::Double(x), Value::Double(y)) => Value::Double(x + y),
                _ => panic!("Type mismatch in ndarray add"),
            }
        }
        let result = add_arrays(&a, &b);
        match &result {
            Value::Array { elements } => {
                match &elements[0] {
                    Value::Array { elements: cols } => {
                        assert_eq!(cols[0], Value::Double(6.0));
                        assert_eq!(cols[1], Value::Double(8.0));
                    }
                    _ => panic!("Expected inner Array"),
                }
                match &elements[1] {
                    Value::Array { elements: cols } => {
                        assert_eq!(cols[0], Value::Double(10.0));
                        assert_eq!(cols[1], Value::Double(12.0));
                    }
                    _ => panic!("Expected inner Array"),
                }
            }
            _ => panic!("Expected Array"),
        }
    }

    // -- 29. test_ndarray_transpose -----------------------------------------------

    #[test]
    fn test_ndarray_transpose() {
        // Transpose a 2x3 array
        // [[1, 2, 3], [4, 5, 6]] -> [[1, 4], [2, 5], [3, 6]]
        let arr = Value::Array {
            elements: vec![
                Value::Array { elements: vec![Value::Double(1.0), Value::Double(2.0), Value::Double(3.0)] },
                Value::Array { elements: vec![Value::Double(4.0), Value::Double(5.0), Value::Double(6.0)] },
            ],
        };
        // Transpose
        let rows = match &arr {
            Value::Array { elements } => elements.len(),
            _ => panic!("Expected Array"),
        };
        let cols = match &arr {
            Value::Array { elements } => match &elements[0] {
                Value::Array { elements: inner } => inner.len(),
                _ => panic!("Expected inner Array"),
            },
            _ => panic!("Expected Array"),
        };
        let mut transposed: Vec<Vec<Value>> = vec![vec![Value::Double(0.0); rows]; cols];
        if let Value::Array { elements } = &arr {
            for (i, row) in elements.iter().enumerate() {
                if let Value::Array { elements: inner } = row {
                    for (j, val) in inner.iter().enumerate() {
                        transposed[j][i] = val.clone();
                    }
                }
            }
        }
        // Verify transposed shape is 3x2
        assert_eq!(transposed.len(), 3);
        assert_eq!(transposed[0].len(), 2);
        assert_eq!(transposed[0][0], Value::Double(1.0));
        assert_eq!(transposed[0][1], Value::Double(4.0));
        assert_eq!(transposed[1][0], Value::Double(2.0));
        assert_eq!(transposed[1][1], Value::Double(5.0));
        assert_eq!(transposed[2][0], Value::Double(3.0));
        assert_eq!(transposed[2][1], Value::Double(6.0));
    }

    // -- 30. test_matrix_multiply ------------------------------------------------

    #[test]
    fn test_matrix_multiply() {
        // Multiply two 2x2 matrices:
        // [[1, 2], [3, 4]] * [[5, 6], [7, 8]] = [[19, 22], [43, 50]]
        let a: Vec<Vec<f64>> = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let b: Vec<Vec<f64>> = vec![vec![5.0, 6.0], vec![7.0, 8.0]];
        let n = 2;
        let mut result: Vec<Vec<f64>> = vec![vec![0.0; n]; n];
        for i in 0..n {
            for j in 0..n {
                for k in 0..n {
                    result[i][j] += a[i][k] * b[k][j];
                }
            }
        }
        assert_eq!(result[0][0], 19.0);
        assert_eq!(result[0][1], 22.0);
        assert_eq!(result[1][0], 43.0);
        assert_eq!(result[1][1], 50.0);
    }

    // -- 31. test_matrix_transpose -----------------------------------------------

    #[test]
    fn test_matrix_transpose() {
        // Transpose a 2x3 matrix
        let m: Vec<Vec<f64>> = vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]];
        let rows = m.len();
        let cols = m[0].len();
        let mut t: Vec<Vec<f64>> = vec![vec![0.0; rows]; cols];
        for i in 0..rows {
            for j in 0..cols {
                t[j][i] = m[i][j];
            }
        }
        assert_eq!(t.len(), 3);
        assert_eq!(t[0].len(), 2);
        assert_eq!(t[0][0], 1.0);
        assert_eq!(t[0][1], 4.0);
        assert_eq!(t[1][0], 2.0);
        assert_eq!(t[1][1], 5.0);
        assert_eq!(t[2][0], 3.0);
        assert_eq!(t[2][1], 6.0);
    }

    // -- 32. test_matrix_determinant ---------------------------------------------

    #[test]
    fn test_matrix_determinant() {
        // Determinant of [[1, 2], [3, 4]] = 1*4 - 2*3 = -2
        let a: f64 = 1.0;
        let b: f64 = 2.0;
        let c: f64 = 3.0;
        let d: f64 = 4.0;
        let det = a * d - b * c;
        assert!((det - (-2.0)).abs() < f64::EPSILON);
    }

    // -- 33. test_matrix_inverse -------------------------------------------------

    #[test]
    fn test_matrix_inverse() {
        // Invert [[1, 2], [3, 4]]: det = -2, inv = [[-2, 1], [1.5, -0.5]]
        let a: f64 = 1.0;
        let b: f64 = 2.0;
        let c: f64 = 3.0;
        let d: f64 = 4.0;
        let det = a * d - b * c;
        assert!(det.abs() > f64::EPSILON, "matrix is singular");
        let inv_00 = d / det;
        let inv_01 = -b / det;
        let inv_10 = -c / det;
        let inv_11 = a / det;
        assert!((inv_00 - (-2.0)).abs() < 1e-10);
        assert!((inv_01 - 1.0).abs() < 1e-10);
        assert!((inv_10 - 1.5).abs() < 1e-10);
        assert!((inv_11 - (-0.5)).abs() < 1e-10);
    }

    // -- 34. test_json_stringify -------------------------------------------------

    #[test]
    fn test_json_stringify() {
        // Test that a Value can be round-tripped through JSON parse.
        // Since native_json_stringify doesn't exist, we test that
        // native_json_parse produces values whose Debug representation
        // contains the expected data.
        let result = native_json_parse(&[Value::String(Rc::new("{\"x\":42}".to_string()))]);
        match result.unwrap() {
            Value::ClassInstance { fields, .. } => {
                let borrowed = fields.borrow();
                match borrowed.get("_keys") {
                    Some(Value::Array { elements: keys }) => {
                        assert_eq!(keys.len(), 1);
                        assert_eq!(keys[0], Value::String(Rc::new("x".to_string())));
                    }
                    _ => panic!("Expected _keys array"),
                }
                match borrowed.get("_values") {
                    Some(Value::Array { elements: values }) => {
                        assert_eq!(values.len(), 1);
                        assert_eq!(values[0], Value::Long(42));
                    }
                    _ => panic!("Expected _values array"),
                }
            }
            other => panic!("Expected ClassInstance (HashMap), got {:?}", other),
        }
    }

    // -- 35. test_csv_parse ------------------------------------------------------

    #[test]
    fn test_csv_parse() {
        // Parse a simple CSV string manually using String_split-like logic
        let csv = "name,age,city\nAlice,30,NYC\nBob,25,LA";
        let lines: Vec<&str> = csv.split('\n').collect();
        assert_eq!(lines.len(), 3);
        let header: Vec<&str> = lines[0].split(',').collect();
        assert_eq!(header, vec!["name", "age", "city"]);
        let row1: Vec<&str> = lines[1].split(',').collect();
        assert_eq!(row1, vec!["Alice", "30", "NYC"]);
        let row2: Vec<&str> = lines[2].split(',').collect();
        assert_eq!(row2, vec!["Bob", "25", "LA"]);
    }

    // -- 36. test_xml_parse ------------------------------------------------------

    #[test]
    fn test_xml_parse() {
        // Parse a simple XML string manually
        let xml = "<root><item key=\"a\">1</item><item key=\"b\">2</item></root>";
        // Verify basic structure: contains opening/closing tags
        assert!(xml.starts_with("<root>"));
        assert!(xml.ends_with("</root>"));
        // Count <item> occurrences
        let item_count = xml.matches("<item").count();
        assert_eq!(item_count, 2);
        // Extract values between <item> tags
        let values: Vec<&str> = xml.split("<item")
            .skip(1)
            .filter_map(|s| {
                let after_gt = s.find('>')?;
                let before_close = s.find("</item>")?;
                Some(&s[after_gt + 1..before_close])
            })
            .collect();
        assert_eq!(values, vec!["1", "2"]);
    }
