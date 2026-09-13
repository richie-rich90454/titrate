use super::vm_with_chunk;
    use crate::bytecode::value::Value;
    use crate::bytecode::chunk::Chunk;
    use crate::bytecode::opcodes::{OpCode, CastTarget};
    use crate::bytecode::frame::{FunctionDef, ClassDef};
    use crate::bytecode::vm::Vm;
    use std::collections::HashMap;
    use std::rc::Rc;

    // -- 1. test_vm_push_pop ---------------------------------------------------

    #[test]
    fn test_vm_push_pop() {
        let mut chunk = Chunk::new();
        // PUSH_I32 42
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&42i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1, 1, 1, 1]);
        // PUSH_NULL
        chunk.write_opcode(OpCode::PUSH_NULL, 1);
        // POP
        chunk.write_opcode(OpCode::POP, 1);
        // PUSH_BOOL 1
        chunk.write_opcode(OpCode::PUSH_BOOL, 1);
        chunk.write_u8(1, 1);
        // RET
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Bool(true)));
    }

    // -- 2. test_vm_arithmetic -------------------------------------------------

    #[test]
    fn test_vm_arithmetic() {
        let mut chunk = Chunk::new();

        // PUSH_I32 10
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&10i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1, 1, 1, 1]);
        // PUSH_I32 3
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&3i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1, 1, 1, 1]);
        // ADD_I32
        chunk.write_opcode(OpCode::ADD_I32, 1);
        // RET
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Int(13)));

        // Test SUB_I64
        let mut chunk = Chunk::new();
        // PUSH_I64 100
        chunk.write_opcode(OpCode::PUSH_I64, 1);
        chunk.code.extend_from_slice(&100i64.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 8]);
        // PUSH_I64 40
        chunk.write_opcode(OpCode::PUSH_I64, 1);
        chunk.code.extend_from_slice(&40i64.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 8]);
        // SUB_I64
        chunk.write_opcode(OpCode::SUB_I64, 1);
        // RET
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Long(60)));

        // Test MUL_I32
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&7i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&6i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::MUL_I32, 1);
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Int(42)));

        // Test DIV_I32
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&20i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&4i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::DIV_I32, 1);
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Int(5)));

        // Test division by zero
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
        assert!(vm.run().is_err());
    }

    // -- 3. test_vm_comparison -------------------------------------------------

    #[test]
    fn test_vm_comparison() {
        // EQ_I32: 5 == 5 → true
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&5i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&5i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::EQ_I32, 1);
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Bool(true)));

        // LT_I32: 3 < 5 → true
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&3i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&5i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::LT_I32, 1);
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Bool(true)));

        // GT_I64: 100 > 50 → true
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::PUSH_I64, 1);
        chunk.code.extend_from_slice(&100i64.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 8]);
        chunk.write_opcode(OpCode::PUSH_I64, 1);
        chunk.code.extend_from_slice(&50i64.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 8]);
        chunk.write_opcode(OpCode::GT_I64, 1);
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Bool(true)));
    }

    // -- helpers: push_value / run_i64_compare ---------------------------------

    fn push_value(chunk: &mut Chunk, v: &Value) {
        match v {
            Value::Bool(b) => {
                chunk.write_opcode(OpCode::PUSH_BOOL, 1);
                chunk.write_u8(if *b { 1 } else { 0 }, 1);
            }
            Value::Int(i) => {
                chunk.write_opcode(OpCode::PUSH_I32, 1);
                chunk.code.extend_from_slice(&i.to_be_bytes());
                chunk.source_lines.extend_from_slice(&[1; 4]);
            }
            Value::Long(l) => {
                chunk.write_opcode(OpCode::PUSH_I64, 1);
                chunk.code.extend_from_slice(&l.to_be_bytes());
                chunk.source_lines.extend_from_slice(&[1; 8]);
            }
            _ => panic!("push_value: unsupported test value {:?}", v),
        }
    }

    fn run_i64_compare(a: Value, b: Value, op: OpCode) -> Value {
        let mut chunk = Chunk::new();
        push_value(&mut chunk, &a);
        push_value(&mut chunk, &b);
        chunk.write_opcode(op, 1);
        chunk.write_opcode(OpCode::RET, 1);
        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        vm.stack.last().unwrap().clone()
    }

    // -- test_vm_i64_comparison_bool (Value::Bool in LT/GT/LE/GE_I64) ---------

    #[test]
    fn test_vm_i64_comparison_bool() {
        // LT_I64 with Bool operands (bool coerced to i64: false=0, true=1)
        assert_eq!(run_i64_compare(Value::Bool(false), Value::Bool(true), OpCode::LT_I64), Value::Bool(true));
        assert_eq!(run_i64_compare(Value::Bool(true), Value::Bool(false), OpCode::LT_I64), Value::Bool(false));
        assert_eq!(run_i64_compare(Value::Bool(false), Value::Int(5), OpCode::LT_I64), Value::Bool(true));
        assert_eq!(run_i64_compare(Value::Int(5), Value::Bool(true), OpCode::LT_I64), Value::Bool(false));
        assert_eq!(run_i64_compare(Value::Bool(true), Value::Int(1), OpCode::LT_I64), Value::Bool(false));
        assert_eq!(run_i64_compare(Value::Bool(false), Value::Long(100), OpCode::LT_I64), Value::Bool(true));
        assert_eq!(run_i64_compare(Value::Long(100), Value::Bool(false), OpCode::LT_I64), Value::Bool(false));
        // GT_I64 with Bool operands
        assert_eq!(run_i64_compare(Value::Bool(true), Value::Bool(false), OpCode::GT_I64), Value::Bool(true));
        assert_eq!(run_i64_compare(Value::Bool(false), Value::Bool(true), OpCode::GT_I64), Value::Bool(false));
        assert_eq!(run_i64_compare(Value::Bool(true), Value::Int(0), OpCode::GT_I64), Value::Bool(true));
        assert_eq!(run_i64_compare(Value::Int(0), Value::Bool(true), OpCode::GT_I64), Value::Bool(false));
        assert_eq!(run_i64_compare(Value::Long(100), Value::Bool(true), OpCode::GT_I64), Value::Bool(true));
        // LE_I64 with Bool operands
        assert_eq!(run_i64_compare(Value::Bool(true), Value::Bool(true), OpCode::LE_I64), Value::Bool(true));
        assert_eq!(run_i64_compare(Value::Bool(false), Value::Bool(true), OpCode::LE_I64), Value::Bool(true));
        assert_eq!(run_i64_compare(Value::Bool(true), Value::Int(0), OpCode::LE_I64), Value::Bool(false));
        assert_eq!(run_i64_compare(Value::Int(1), Value::Bool(true), OpCode::LE_I64), Value::Bool(true));
        // GE_I64 with Bool operands
        assert_eq!(run_i64_compare(Value::Bool(true), Value::Bool(true), OpCode::GE_I64), Value::Bool(true));
        assert_eq!(run_i64_compare(Value::Bool(false), Value::Bool(true), OpCode::GE_I64), Value::Bool(false));
        assert_eq!(run_i64_compare(Value::Bool(true), Value::Int(1), OpCode::GE_I64), Value::Bool(true));
        assert_eq!(run_i64_compare(Value::Int(2), Value::Bool(true), OpCode::GE_I64), Value::Bool(true));
        assert_eq!(run_i64_compare(Value::Bool(false), Value::Long(0), OpCode::GE_I64), Value::Bool(true));
    }

    // -- 4. test_vm_jumps ------------------------------------------------------

    #[test]
    fn test_vm_jumps() {
        // JMP: unconditional jump over a PUSH_NULL + POP
        let mut chunk = Chunk::new();
        // PUSH_I32 1
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&1i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        // JMP +2 (skip PUSH_NULL + POP, land on RET)
        // After reading JMP operand, IP points to byte after the i16 offset.
        // PUSH_NULL is 1 byte, POP is 1 byte, so offset=2 skips both.
        chunk.write_opcode(OpCode::JMP, 1);
        chunk.write_i16(2, 1);
        // PUSH_NULL (skipped)
        chunk.write_opcode(OpCode::PUSH_NULL, 1);
        // POP (skipped)
        chunk.write_opcode(OpCode::POP, 1);
        // RET
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Int(1)));

        // JMP_IF_FALSE: jump when value is false
        let mut chunk = Chunk::new();
        // PUSH_BOOL 0 (false)
        chunk.write_opcode(OpCode::PUSH_BOOL, 1);
        chunk.write_u8(0, 1);
        // JMP_IF_FALSE +2 (skip PUSH_I8 99 + its operand)
        chunk.write_opcode(OpCode::JMP_IF_FALSE, 1);
        chunk.write_i16(2, 1);
        // PUSH_I8 99 (skipped because false) — opcode + 1 byte operand = 2 bytes
        chunk.write_opcode(OpCode::PUSH_I8, 1);
        chunk.write_u8(99, 1);
        // PUSH_I32 42
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&42i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        // RET
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Int(42)));
    }

    // -- 5. test_vm_call_return ------------------------------------------------

    #[test]
    fn test_vm_call_return() {
        // Main: push 3, push 4, CALL func#1(2 args), RET
        // Func#1: LOAD_LOCAL 0, LOAD_LOCAL 1, ADD_I32, RET
        let mut main_chunk = Chunk::new();
        // PUSH_I32 3
        main_chunk.write_opcode(OpCode::PUSH_I32, 1);
        main_chunk.code.extend_from_slice(&3i32.to_be_bytes());
        main_chunk.source_lines.extend_from_slice(&[1; 4]);
        // PUSH_I32 4
        main_chunk.write_opcode(OpCode::PUSH_I32, 1);
        main_chunk.code.extend_from_slice(&4i32.to_be_bytes());
        main_chunk.source_lines.extend_from_slice(&[1; 4]);
        // CALL func_idx=1, arg_count=2
        main_chunk.write_opcode(OpCode::CALL, 1);
        main_chunk.write_u16(1, 1); // function index 1
        main_chunk.write_u8(2, 1);  // 2 args
        // RET
        main_chunk.write_opcode(OpCode::RET, 1);

        let mut add_chunk = Chunk::new();
        // LOAD_LOCAL 0 (first arg = 3)
        add_chunk.write_opcode(OpCode::LOAD_LOCAL, 1);
        add_chunk.write_u8(0, 1);
        // LOAD_LOCAL 1 (second arg = 4)
        add_chunk.write_opcode(OpCode::LOAD_LOCAL, 1);
        add_chunk.write_u8(1, 1);
        // ADD_I32
        add_chunk.write_opcode(OpCode::ADD_I32, 1);
        // RET
        add_chunk.write_opcode(OpCode::RET, 1);

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
            name: "add".to_string(),
            arity: 2,
            chunk: add_chunk,
            is_method: false,
            is_constructor: false,
            local_count: 2,
        param_types: Vec::new(),
        });

        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Int(7)));
    }

    // -- 6. test_vm_local_variables --------------------------------------------

    #[test]
    fn test_vm_local_variables() {
        let mut chunk = Chunk::new();
        // PUSH_I32 10
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&10i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        // STORE_LOCAL 0
        chunk.write_opcode(OpCode::STORE_LOCAL, 1);
        chunk.write_u8(0, 1);
        // PUSH_I32 20
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&20i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        // STORE_LOCAL 1
        chunk.write_opcode(OpCode::STORE_LOCAL, 1);
        chunk.write_u8(1, 1);
        // LOAD_LOCAL 0
        chunk.write_opcode(OpCode::LOAD_LOCAL, 1);
        chunk.write_u8(0, 1);
        // LOAD_LOCAL 1
        chunk.write_opcode(OpCode::LOAD_LOCAL, 1);
        chunk.write_u8(1, 1);
        // ADD_I32
        chunk.write_opcode(OpCode::ADD_I32, 1);
        // RET
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Int(30)));
    }

    // -- 7. test_vm_string_concat ----------------------------------------------

    #[test]
    fn test_vm_string_concat() {
        let mut chunk = Chunk::new();
        let hello_idx = chunk.add_string("Hello, ");
        let world_idx = chunk.add_string("world!");

        // PUSH_STRING "Hello, "
        chunk.write_opcode(OpCode::PUSH_STRING, 1);
        chunk.write_u16(hello_idx, 1);
        // PUSH_STRING "world!"
        chunk.write_opcode(OpCode::PUSH_STRING, 1);
        chunk.write_u16(world_idx, 1);
        // STR_CONCAT
        chunk.write_opcode(OpCode::STR_CONCAT, 1);
        // RET
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(
            vm.stack.last(),
            Some(&Value::String(Rc::new("Hello, world!".to_string())))
        );

        // STR_CONCAT_RIGHT: "x: " ++ 42
        let mut chunk2 = Chunk::new();
        let prefix_idx = chunk2.add_string("x: ");
        chunk2.write_opcode(OpCode::PUSH_STRING, 1);
        chunk2.write_u16(prefix_idx, 1);
        chunk2.write_opcode(OpCode::PUSH_I32, 1);
        chunk2.code.extend_from_slice(&42i32.to_be_bytes());
        chunk2.source_lines.extend_from_slice(&[1; 4]);
        chunk2.write_opcode(OpCode::STR_CONCAT_RIGHT, 1);
        chunk2.write_opcode(OpCode::RET, 1);

        let mut vm2 = vm_with_chunk(chunk2);
        vm2.run().unwrap();
        assert_eq!(
            vm2.stack.last(),
            Some(&Value::String(Rc::new("x: 42".to_string())))
        );

        // STR_CONCAT_LEFT: 42 ++ " items"
        let mut chunk3 = Chunk::new();
        let suffix_idx = chunk3.add_string(" items");
        chunk3.write_opcode(OpCode::PUSH_I32, 1);
        chunk3.code.extend_from_slice(&42i32.to_be_bytes());
        chunk3.source_lines.extend_from_slice(&[1; 4]);
        chunk3.write_opcode(OpCode::PUSH_STRING, 1);
        chunk3.write_u16(suffix_idx, 1);
        chunk3.write_opcode(OpCode::STR_CONCAT_LEFT, 1);
        chunk3.write_opcode(OpCode::RET, 1);

        let mut vm3 = vm_with_chunk(chunk3);
        vm3.run().unwrap();
        assert_eq!(
            vm3.stack.last(),
            Some(&Value::String(Rc::new("42 items".to_string())))
        );
    }

    // -- 8. test_vm_class_new_and_field_access ---------------------------------

    #[test]
    fn test_vm_class_new_and_field_access() {
        let mut chunk = Chunk::new();
        let x_idx = chunk.add_string("x");

        // NEW class_idx=0, arg_count=0 → pushes instance
        chunk.write_opcode(OpCode::NEW, 1);
        chunk.write_u16(0, 1);
        chunk.write_u8(0, 1);
        // Stack: [instance]

        // STORE_LOCAL 0 → store instance in local var 0
        chunk.write_opcode(OpCode::STORE_LOCAL, 1);
        chunk.write_u8(0, 1);
        // Stack: []

        // PUSH_I32 42 → [42]
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&42i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);

        // LOAD_LOCAL 0 → [42, instance]
        chunk.write_opcode(OpCode::LOAD_LOCAL, 1);
        chunk.write_u8(0, 1);

        // SET_FIELD "x" → pops instance (top), pops 42, sets x=42, pushes 42
        // Stack: [42]
        chunk.write_opcode(OpCode::SET_FIELD, 1);
        chunk.write_u16(x_idx, 1);

        // POP the 42 → []
        chunk.write_opcode(OpCode::POP, 1);

        // LOAD_LOCAL 0 → [instance]
        chunk.write_opcode(OpCode::LOAD_LOCAL, 1);
        chunk.write_u8(0, 1);

        // GET_FIELD "x" → pops instance, pushes field value
        // Stack: [42]
        chunk.write_opcode(OpCode::GET_FIELD, 1);
        chunk.write_u16(x_idx, 1);

        // RET
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = Vm::new();
        vm.add_function(FunctionDef {
            name: "main".to_string(),
            arity: 0,
            chunk,
            is_method: false,
            is_constructor: false,
            local_count: 1,
        param_types: Vec::new(),
        });
        vm.add_class(ClassDef {
            name: "Point".to_string(),
            parent: None,
            fields: vec![crate::bytecode::frame::FieldDef {
                name: "x".to_string(),
                has_init: false,
            }],
            methods: HashMap::new(),
            constructor: None,
            field_inits: vec![],
        });

        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Int(42)));
    }

    // -- 9. test_vm_enum -------------------------------------------------------

    #[test]
    fn test_vm_enum() {
        let mut chunk = Chunk::new();
        let color_idx = chunk.add_string("Color");
        let red_idx = chunk.add_string("Red");

        // PUSH_I32 255
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&255i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        // ENUM_NEW Color::Red(1 field)
        chunk.write_opcode(OpCode::ENUM_NEW, 1);
        chunk.write_u16(color_idx, 1);
        chunk.write_u16(red_idx, 1);
        chunk.write_u8(1, 1);
        // RET
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();

        match vm.stack.last() {
            Some(Value::EnumInstance { enum_name, variant, fields }) => {
                assert_eq!(enum_name, "Color");
                assert_eq!(variant, "Red");
                assert_eq!(fields.len(), 1);
                assert_eq!(fields[0], Value::Int(255));
            }
            _ => panic!("Expected EnumInstance"),
        }
    }

    // -- 10. test_vm_result_ok_err ---------------------------------------------

    #[test]
    fn test_vm_result_ok_err() {
        // ResultOk
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&42i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::RESULT_OK, 1);
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(
            vm.stack.last(),
            Some(&Value::ResultOk(Box::new(Value::Int(42))))
        );

        // ResultErr
        let mut chunk2 = Chunk::new();
        let err_idx = chunk2.add_string("fail");
        chunk2.write_opcode(OpCode::PUSH_STRING, 1);
        chunk2.write_u16(err_idx, 1);
        chunk2.write_opcode(OpCode::RESULT_ERR, 1);
        chunk2.write_opcode(OpCode::RET, 1);

        let mut vm2 = vm_with_chunk(chunk2);
        vm2.run().unwrap();
        assert_eq!(
            vm2.stack.last(),
            Some(&Value::ResultErr(Box::new(Value::String(Rc::new(
                "fail".to_string()
            )))))
        );

        // UNWRAP_OR_PROPAGATE on Ok
        let mut chunk3 = Chunk::new();
        chunk3.write_opcode(OpCode::PUSH_I32, 1);
        chunk3.code.extend_from_slice(&99i32.to_be_bytes());
        chunk3.source_lines.extend_from_slice(&[1; 4]);
        chunk3.write_opcode(OpCode::RESULT_OK, 1);
        chunk3.write_opcode(OpCode::UNWRAP_OR_PROPAGATE, 1);
        chunk3.write_opcode(OpCode::RET, 1);

        let mut vm3 = vm_with_chunk(chunk3);
        vm3.run().unwrap();
        assert_eq!(vm3.stack.last(), Some(&Value::Int(99)));
    }

    // -- 11. test_vm_cast ------------------------------------------------------

    #[test]
    fn test_vm_cast() {
        // Cast Int(42) to Long
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&42i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::CAST, 1);
        chunk.write_u8(CastTarget::Long as u8, 1);
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(vm.stack.last(), Some(&Value::Long(42)));

        // Cast Int(65) to Char
        let mut chunk2 = Chunk::new();
        chunk2.write_opcode(OpCode::PUSH_I32, 1);
        chunk2.code.extend_from_slice(&65i32.to_be_bytes());
        chunk2.source_lines.extend_from_slice(&[1; 4]);
        chunk2.write_opcode(OpCode::CAST, 1);
        chunk2.write_u8(CastTarget::Char as u8, 1);
        chunk2.write_opcode(OpCode::RET, 1);

        let mut vm2 = vm_with_chunk(chunk2);
        vm2.run().unwrap();
        assert_eq!(vm2.stack.last(), Some(&Value::Char('A')));

        // Cast Int(42) to String
        let mut chunk3 = Chunk::new();
        chunk3.write_opcode(OpCode::PUSH_I32, 1);
        chunk3.code.extend_from_slice(&42i32.to_be_bytes());
        chunk3.source_lines.extend_from_slice(&[1; 4]);
        chunk3.write_opcode(OpCode::CAST, 1);
        chunk3.write_u8(CastTarget::String as u8, 1);
        chunk3.write_opcode(OpCode::RET, 1);

        let mut vm3 = vm_with_chunk(chunk3);
        vm3.run().unwrap();
        assert_eq!(
            vm3.stack.last(),
            Some(&Value::String(Rc::new("42".to_string())))
        );

        // Cast Int(1) to Bool (truthy → true)
        let mut chunk4 = Chunk::new();
        chunk4.write_opcode(OpCode::PUSH_I32, 1);
        chunk4.code.extend_from_slice(&1i32.to_be_bytes());
        chunk4.source_lines.extend_from_slice(&[1; 4]);
        chunk4.write_opcode(OpCode::CAST, 1);
        chunk4.write_u8(CastTarget::Bool as u8, 1);
        chunk4.write_opcode(OpCode::RET, 1);

        let mut vm4 = vm_with_chunk(chunk4);
        vm4.run().unwrap();
        assert_eq!(vm4.stack.last(), Some(&Value::Bool(true)));

        // Cast Int(0) to Bool (falsy → false)
        let mut chunk5 = Chunk::new();
        chunk5.write_opcode(OpCode::PUSH_I32, 1);
        chunk5.code.extend_from_slice(&0i32.to_be_bytes());
        chunk5.source_lines.extend_from_slice(&[1; 4]);
        chunk5.write_opcode(OpCode::CAST, 1);
        chunk5.write_u8(CastTarget::Bool as u8, 1);
        chunk5.write_opcode(OpCode::RET, 1);

        let mut vm5 = vm_with_chunk(chunk5);
        vm5.run().unwrap();
        assert_eq!(vm5.stack.last(), Some(&Value::Bool(false)));
    }

    // -- 12. test_vm_native_fn -------------------------------------------------

    #[test]
    fn test_vm_native_fn() {
        // Call toString(42)
        let mut chunk = Chunk::new();
        // PUSH_I32 42
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&42i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        // CALL_NATIVE toString_idx=1, arg_count=1
        // toString is the second native registered (index 1)
        chunk.write_opcode(OpCode::CALL_NATIVE, 1);
        chunk.write_u16(1, 1); // native index 1 = toString
        chunk.write_u8(1, 1);  // 1 arg
        // RET
        chunk.write_opcode(OpCode::RET, 1);

        let mut vm = vm_with_chunk(chunk);
        vm.run().unwrap();
        assert_eq!(
            vm.stack.last(),
            Some(&Value::String(Rc::new("42".to_string())))
        );

        // Call Ok(42)
        let mut chunk2 = Chunk::new();
        chunk2.write_opcode(OpCode::PUSH_I32, 1);
        chunk2.code.extend_from_slice(&42i32.to_be_bytes());
        chunk2.source_lines.extend_from_slice(&[1; 4]);
        // Ok is native index 3
        chunk2.write_opcode(OpCode::CALL_NATIVE, 1);
        chunk2.write_u16(3, 1);
        chunk2.write_u8(1, 1);
        chunk2.write_opcode(OpCode::RET, 1);

        let mut vm2 = vm_with_chunk(chunk2);
        vm2.run().unwrap();
        assert_eq!(
            vm2.stack.last(),
            Some(&Value::ResultOk(Box::new(Value::Int(42))))
        );

        // Call Err("oops")
        let mut chunk3 = Chunk::new();
        let err_idx = chunk3.add_string("oops");
        chunk3.write_opcode(OpCode::PUSH_STRING, 1);
        chunk3.write_u16(err_idx, 1);
        // Err is native index 4
        chunk3.write_opcode(OpCode::CALL_NATIVE, 1);
        chunk3.write_u16(4, 1);
        chunk3.write_u8(1, 1);
        chunk3.write_opcode(OpCode::RET, 1);

        let mut vm3 = vm_with_chunk(chunk3);
        vm3.run().unwrap();
        assert_eq!(
            vm3.stack.last(),
            Some(&Value::ResultErr(Box::new(Value::String(Rc::new(
                "oops".to_string()
            )))))
        );
    }
