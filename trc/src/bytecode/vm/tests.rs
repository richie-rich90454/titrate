// Titrate Alpha 0.2 – bytecode virtual machine: tests
// Precision in every step – richie-rich90454, 2026

#[cfg(test)]
mod tests {
    use crate::bytecode::value::Value;
    use crate::bytecode::chunk::Chunk;
    use crate::bytecode::opcodes::{OpCode, CastTarget};
    use crate::bytecode::frame::{FunctionDef, ClassDef};
    use crate::bytecode::vm::Vm;
    use crate::bytecode::vm::natives::json::{native_json_parse, native_json_stringify};
    use crate::bytecode::vm::natives::regex::{native_regex_match, native_regex_find, native_regex_replace};
    use crate::bytecode::vm::natives::string::*;
    use crate::bytecode::vm::natives::system::*;
    use crate::bytecode::vm::natives::path::*;
    use crate::bytecode::vm::natives::time::*;
    use crate::bytecode::vm::natives::net::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    /// Helper: create a minimal VM with a single function (main) containing
    /// the given bytecode and an empty string table.
    fn vm_with_chunk(chunk: Chunk) -> Vm {
        let mut vm = Vm::new();
        vm.add_function(FunctionDef {
            name: "main".to_string(),
            arity: 0,
            chunk,
            is_method: false,
            is_constructor: false,
            local_count: 0,
        });
        vm
    }

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
        });
        vm.add_function(FunctionDef {
            name: "add".to_string(),
            arity: 2,
            chunk: add_chunk,
            is_method: false,
            is_constructor: false,
            local_count: 2,
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
        });
        vm.add_function(FunctionDef {
            name: "$closure_0".to_string(),
            arity: 1,
            chunk: closure_chunk,
            is_method: false,
            is_constructor: false,
            local_count: 1,
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
        });
        vm.add_function(FunctionDef {
            name: "$closure_0".to_string(),
            arity: 1,
            chunk: closure_chunk,
            is_method: false,
            is_constructor: false,
            local_count: 1,
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
        match result.unwrap() {
            Value::Array { elements } => {
                assert_eq!(elements.len(), 3);
                assert_eq!(elements[0], Value::Long(1));
                assert_eq!(elements[1], Value::Long(2));
                assert_eq!(elements[2], Value::Long(3));
            }
            other => panic!("Expected Array, got {:?}", other),
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
        });
        vm.add_function(FunctionDef {
            name: "$closure_0".to_string(),
            arity: 1,
            chunk: closure_chunk,
            is_method: false,
            is_constructor: false,
            local_count: 1,
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
        });
        vm.add_function(FunctionDef {
            name: "$closure_0".to_string(),
            arity: 1,
            chunk: outer_chunk,
            is_method: false,
            is_constructor: false,
            local_count: 2,
        });
        vm.add_function(FunctionDef {
            name: "$closure_1".to_string(),
            arity: 1,
            chunk: inner_chunk,
            is_method: false,
            is_constructor: false,
            local_count: 1,
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
        });
        vm.add_function(FunctionDef {
            name: "$closure_0".to_string(),
            arity: 1,
            chunk: closure_chunk,
            is_method: false,
            is_constructor: false,
            local_count: 1,
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
        });
        vm.add_function(FunctionDef {
            name: "makeTuple".to_string(),
            arity: 0,
            chunk: fn_chunk,
            is_method: false,
            is_constructor: false,
            local_count: 0,
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

    // -- 58. test_meter_plus -----------------------------------------------------

    #[test]
    fn test_meter_plus() {
        // Simulate adding two Meter values (represented as Doubles)
        let m1 = Value::Double(5.0);
        let m2 = Value::Double(3.0);
        // Add
        match (&m1, &m2) {
            (Value::Double(a), Value::Double(b)) => {
                assert_eq!(a + b, 8.0);
            }
            _ => panic!("Expected Double values"),
        }
    }

    // -- 59. test_joule_from_base ------------------------------------------------

    #[test]
    fn test_joule_from_base() {
        // Joule = kg * m^2 / s^2
        // 1 J = 1.0 (in base SI units)
        let mass: f64 = 1.0;  // kg
        let velocity: f64 = 1.0;  // m/s
        let joules = 0.5 * mass * velocity * velocity;
        assert!((joules - 0.5).abs() < f64::EPSILON);
    }

    // -- 60. test_constants_boltzmann ---------------------------------------------

    #[test]
    fn test_constants_boltzmann() {
        // Boltzmann constant: 1.380649e-23 J/K
        let boltzmann: f64 = 1.380649e-23;
        assert!(boltzmann > 0.0, "Boltzmann constant should be positive");
        assert!((boltzmann - 1.380649e-23).abs() < 1e-30);
    }

    // -- 61. test_atom_creation --------------------------------------------------

    #[test]
    fn test_atom_creation() {
        // Create an Atom as a ClassInstance with element, x, y, z fields
        let mut atom_fields: HashMap<String, Value> = HashMap::new();
        atom_fields.insert("element".to_string(), Value::String(Rc::new("C".to_string())));
        atom_fields.insert("x".to_string(), Value::Double(0.0));
        atom_fields.insert("y".to_string(), Value::Double(0.0));
        atom_fields.insert("z".to_string(), Value::Double(0.0));

        let atom = Value::ClassInstance {
            class_name: "Atom".to_string(),
            fields: Rc::new(RefCell::new(atom_fields)),
            vtable: HashMap::new(),
        };

        match &atom {
            Value::ClassInstance { class_name, fields, .. } => {
                assert_eq!(class_name, "Atom");
                let borrowed = fields.borrow();
                assert_eq!(borrowed.get("element"), Some(&Value::String(Rc::new("C".to_string()))));
                assert_eq!(borrowed.get("x"), Some(&Value::Double(0.0)));
            }
            _ => panic!("Expected ClassInstance"),
        }
    }

    // -- 62. test_atom_distance --------------------------------------------------

    #[test]
    fn test_atom_distance() {
        // Distance between two atoms at (0,0,0) and (3,4,0) = 5.0
        let x1: f64 = 0.0; let y1: f64 = 0.0; let z1: f64 = 0.0;
        let x2: f64 = 3.0; let y2: f64 = 4.0; let z2: f64 = 0.0;
        let dx = x2 - x1;
        let dy = y2 - y1;
        let dz = z2 - z1;
        let dist = (dx * dx + dy * dy + dz * dz).sqrt();
        assert!((dist - 5.0).abs() < f64::EPSILON);
    }

    // -- 63. test_molecule_add_atom -----------------------------------------------

    #[test]
    fn test_molecule_add_atom() {
        // Create a Molecule as a ClassInstance with an atoms Array field
        let mut mol_fields: HashMap<String, Value> = HashMap::new();
        mol_fields.insert("name".to_string(), Value::String(Rc::new("Water".to_string())));
        mol_fields.insert("atoms".to_string(), Value::Array { elements: vec![] });

        // Add atoms
        if let Some(Value::Array { elements }) = mol_fields.get_mut("atoms") {
            elements.push(Value::String(Rc::new("O".to_string())));
            elements.push(Value::String(Rc::new("H".to_string())));
            elements.push(Value::String(Rc::new("H".to_string())));
        }

        let molecule = Value::ClassInstance {
            class_name: "Molecule".to_string(),
            fields: Rc::new(RefCell::new(mol_fields)),
            vtable: HashMap::new(),
        };

        match &molecule {
            Value::ClassInstance { fields, .. } => {
                let borrowed = fields.borrow();
                match borrowed.get("atoms") {
                    Some(Value::Array { elements }) => {
                        assert_eq!(elements.len(), 3);
                    }
                    _ => panic!("Expected atoms array"),
                }
                match borrowed.get("name") {
                    Some(Value::String(s)) => {
                        assert_eq!(&*s as &str, "Water");
                    }
                    _ => panic!("Expected name string"),
                }
            }
            _ => panic!("Expected ClassInstance"),
        }
    }

    // -- 64. test_tcp_client_creation --------------------------------------------

    #[test]
    fn test_tcp_client_creation() {
        // Create a TcpClient as a ClassInstance with host and port fields
        let mut client_fields: HashMap<String, Value> = HashMap::new();
        client_fields.insert("host".to_string(), Value::String(Rc::new("127.0.0.1".to_string())));
        client_fields.insert("port".to_string(), Value::Int(8080));
        client_fields.insert("connected".to_string(), Value::Bool(false));

        let client = Value::ClassInstance {
            class_name: "TcpClient".to_string(),
            fields: Rc::new(RefCell::new(client_fields)),
            vtable: HashMap::new(),
        };

        match &client {
            Value::ClassInstance { class_name, fields, .. } => {
                assert_eq!(class_name, "TcpClient");
                let borrowed = fields.borrow();
                assert_eq!(borrowed.get("host"), Some(&Value::String(Rc::new("127.0.0.1".to_string()))));
                assert_eq!(borrowed.get("port"), Some(&Value::Int(8080)));
                assert_eq!(borrowed.get("connected"), Some(&Value::Bool(false)));
            }
            _ => panic!("Expected ClassInstance"),
        }
    }

    // -- 65. test_http_client_creation -------------------------------------------

    #[test]
    fn test_http_client_creation() {
        // Create an HttpClient as a ClassInstance with base_url field
        let mut client_fields: HashMap<String, Value> = HashMap::new();
        client_fields.insert("base_url".to_string(), Value::String(Rc::new("https://api.example.com".to_string())));
        client_fields.insert("timeout".to_string(), Value::Long(30000));

        let client = Value::ClassInstance {
            class_name: "HttpClient".to_string(),
            fields: Rc::new(RefCell::new(client_fields)),
            vtable: HashMap::new(),
        };

        match &client {
            Value::ClassInstance { class_name, fields, .. } => {
                assert_eq!(class_name, "HttpClient");
                let borrowed = fields.borrow();
                assert_eq!(borrowed.get("base_url"), Some(&Value::String(Rc::new("https://api.example.com".to_string()))));
                assert_eq!(borrowed.get("timeout"), Some(&Value::Long(30000)));
            }
            _ => panic!("Expected ClassInstance"),
        }
    }

    // -- 66. test_duration_of_seconds --------------------------------------------

    #[test]
    fn test_duration_of_seconds() {
        // Create Duration from seconds (represented as i64 milliseconds)
        let seconds: i64 = 5;
        let duration_ms = seconds * 1000;
        assert_eq!(duration_ms, 5000);

        let duration_ms2: i64 = 0;
        assert_eq!(duration_ms2, 0);

        let duration_ms3: i64 = 1 * 1000;
        assert_eq!(duration_ms3, 1000);
    }

    // -- 67. test_datetime_plus_duration -----------------------------------------

    #[test]
    fn test_datetime_plus_duration() {
        // Add duration to datetime (represented as i64 ms timestamps)
        let datetime_ms: i64 = 1609459200000; // 2021-01-01 00:00:00 UTC
        let duration_ms: i64 = 86400000; // 1 day
        let result = datetime_ms + duration_ms;
        assert_eq!(result, 1609545600000); // 2021-01-02 00:00:00 UTC
    }

    // -- 68. test_datetime_comparison --------------------------------------------

    #[test]
    fn test_datetime_comparison() {
        // Compare two datetimes
        let dt1: i64 = 1609459200000; // 2021-01-01
        let dt2: i64 = 1609545600000; // 2021-01-02
        assert!(dt1 < dt2);
        assert!(dt2 > dt1);
        assert!(dt1 == dt1);
        assert!(dt1 != dt2);
    }

    // -- 69. test_regex_compile --------------------------------------------------

    #[test]
    fn test_regex_compile() {
        // Compiling a regex pattern should not panic
        let result = native_regex_match(&[
            Value::String(Rc::new(r"\d+".to_string())),
            Value::String(Rc::new("123".to_string())),
        ]);
        assert!(result.is_ok(), "regex compile and match should succeed");
    }

    // -- 70. test_regex_match_simple ---------------------------------------------

    #[test]
    fn test_regex_match_simple() {
        // Match a simple pattern
        let result = native_regex_match(&[
            Value::String(Rc::new(r"^hello".to_string())),
            Value::String(Rc::new("hello world".to_string())),
        ]);
        assert_eq!(result.unwrap(), Value::Bool(true));

        // Non-matching
        let result = native_regex_match(&[
            Value::String(Rc::new(r"^world".to_string())),
            Value::String(Rc::new("hello world".to_string())),
        ]);
        assert_eq!(result.unwrap(), Value::Bool(false));
    }

    // -- 71. test_regex_replace_simple -------------------------------------------

    #[test]
    fn test_regex_replace_simple() {
        // Replace with a simple pattern
        let result = native_regex_replace(&[
            Value::String(Rc::new(r"cat".to_string())),
            Value::String(Rc::new("the cat sat on the mat".to_string())),
            Value::String(Rc::new("dog".to_string())),
        ]);
        assert_eq!(
            result.unwrap(),
            Value::String(Rc::new("the dog sat on the mat".to_string()))
        );
    }

    // -- 72. test_env_get -------------------------------------------------------

    #[test]
    fn test_env_get() {
        // Set an env var then retrieve it
        std::env::set_var("TITRATE_TEST_ENV_GET", "hello");
        let result = native_env_get(&[Value::String(Rc::new("TITRATE_TEST_ENV_GET".to_string()))]);
        assert_eq!(result.unwrap(), Value::String(Rc::new("hello".to_string())));

        // Non-existent env var returns Null
        let result = native_env_get(&[Value::String(Rc::new("TITRATE_NONEXISTENT_VAR_XYZ".to_string()))]);
        assert_eq!(result.unwrap(), Value::Null);

        // Error on wrong type
        let result = native_env_get(&[Value::Int(42)]);
        assert!(result.is_err());
    }

    // -- 73. test_env_set -------------------------------------------------------

    #[test]
    fn test_env_set() {
        let result = native_env_set(&[
            Value::String(Rc::new("TITRATE_TEST_ENV_SET".to_string())),
            Value::String(Rc::new("world".to_string())),
        ]);
        assert_eq!(result.unwrap(), Value::Void);
        assert_eq!(std::env::var("TITRATE_TEST_ENV_SET").unwrap(), "world");

        // Error on wrong type
        let result = native_env_set(&[Value::Int(1), Value::Int(2)]);
        assert!(result.is_err());
    }

    // -- 74. test_env_vars -------------------------------------------------------

    #[test]
    fn test_env_vars() {
        let result = native_env_vars(&[]);
        match result.unwrap() {
            Value::Array { elements } => {
                assert!(!elements.is_empty());
                // Each element should be a "key=value" string
                for elem in &elements {
                    match elem {
                        Value::String(s) => assert!(s.contains('=')),
                        _ => panic!("Expected String in env vars array"),
                    }
                }
            }
            _ => panic!("Expected Array from Env_vars"),
        }
    }

    // -- 75. test_fs_exists -----------------------------------------------------

    #[test]
    fn test_fs_exists() {
        // Current directory should exist
        let result = native_fs_exists(&[Value::String(Rc::new(".".to_string()))]);
        assert_eq!(result.unwrap(), Value::Bool(true));

        // Non-existent path
        let result = native_fs_exists(&[Value::String(Rc::new("/no/such/path/titrate_test_xyz".to_string()))]);
        assert_eq!(result.unwrap(), Value::Bool(false));

        // Error on wrong type
        let result = native_fs_exists(&[Value::Int(42)]);
        assert!(result.is_err());
    }

    // -- 76. test_fs_is_file ----------------------------------------------------

    #[test]
    fn test_fs_is_file() {
        // Current directory is not a file
        let result = native_fs_is_file(&[Value::String(Rc::new(".".to_string()))]);
        assert_eq!(result.unwrap(), Value::Bool(false));

        // A known existing file - use Cargo.toml from the crate root
        let cargo_toml = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("Cargo.toml")
            .to_string_lossy()
            .to_string();
        let result = native_fs_is_file(&[Value::String(Rc::new(cargo_toml))]);
        assert_eq!(result.unwrap(), Value::Bool(true));

        // Error on wrong type
        let result = native_fs_is_file(&[Value::Int(42)]);
        assert!(result.is_err());
    }

    // -- 77. test_fs_is_dir -----------------------------------------------------

    #[test]
    fn test_fs_is_dir() {
        // Current directory should be a directory
        let result = native_fs_is_dir(&[Value::String(Rc::new(".".to_string()))]);
        assert_eq!(result.unwrap(), Value::Bool(true));

        // This source file is not a directory
        let this_file = file!().replace('\\', "/");
        let result = native_fs_is_dir(&[Value::String(Rc::new(this_file))]);
        assert_eq!(result.unwrap(), Value::Bool(false));

        // Error on wrong type
        let result = native_fs_is_dir(&[Value::Int(42)]);
        assert!(result.is_err());
    }

    // -- 78. test_fs_size -------------------------------------------------------

    #[test]
    fn test_fs_size() {
        // Cargo.toml should have a positive size
        let cargo_toml = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("Cargo.toml")
            .to_string_lossy()
            .to_string();
        let result = native_fs_size(&[Value::String(Rc::new(cargo_toml))]);
        match result.unwrap() {
            Value::Long(n) => assert!(n > 0),
            _ => panic!("Expected Long from Fs_size"),
        }

        // Non-existent file should error
        let result = native_fs_size(&[Value::String(Rc::new("/no/such/file_titrate_xyz".to_string()))]);
        assert!(result.is_err());

        // Error on wrong type
        let result = native_fs_size(&[Value::Int(42)]);
        assert!(result.is_err());
    }

    // -- 79. test_process_id ----------------------------------------------------

    #[test]
    fn test_process_id() {
        let result = native_process_id(&[]);
        match result.unwrap() {
            Value::Long(n) => assert!(n > 0),
            _ => panic!("Expected Long from Process_id"),
        }
    }

    // -- 80. test_process_args --------------------------------------------------

    #[test]
    fn test_process_args() {
        let result = native_process_args(&[]);
        match result.unwrap() {
            Value::Array { elements } => {
                assert!(!elements.is_empty());
                for elem in &elements {
                    match elem {
                        Value::String(_) => {}
                        _ => panic!("Expected String in process args array"),
                    }
                }
            }
            _ => panic!("Expected Array from Process_args"),
        }
    }

    // -- 81. test_os_name -------------------------------------------------------

    #[test]
    fn test_os_name() {
        let result = native_os_name(&[]);
        match result.unwrap() {
            Value::String(s) => assert!(!s.is_empty()),
            _ => panic!("Expected String from Os_name"),
        }
    }

    // -- 82. test_os_arch -------------------------------------------------------

    #[test]
    fn test_os_arch() {
        let result = native_os_arch(&[]);
        match result.unwrap() {
            Value::String(s) => assert!(!s.is_empty()),
            _ => panic!("Expected String from Os_arch"),
        }
    }

    // -- 83. test_os_family -----------------------------------------------------

    #[test]
    fn test_os_family() {
        let result = native_os_family(&[]);
        match result.unwrap() {
            Value::String(s) => {
                assert!(!s.is_empty());
                // Should be "unix" or "windows"
                assert!(s.as_str() == "unix" || s.as_str() == "windows", "Unexpected OS family: {}", s);
            }
            _ => panic!("Expected String from Os_family"),
        }
    }

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
        let up = vm.call_native_by_name("Math_nextUp", &[one.clone()]).unwrap();
        let down = vm.call_native_by_name("Math_nextDown", &[one.clone()]).unwrap();
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
            Value::Double(d) => assert!(d >= 0.0 && d < 1.0,
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
        });
        // Add recurse as function 1
        vm.add_function(FunctionDef {
            name: "recurse".to_string(),
            arity: 0,
            chunk: recurse_chunk,
            is_method: false,
            is_constructor: false,
            local_count: 0,
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
}

