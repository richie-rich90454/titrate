use super::su;
use super::super::Compiler;
use crate::ast;
use crate::bytecode::chunk::Chunk;
use crate::bytecode::opcodes::OpCode;

    // -- test_compile_closure -----------------------------------------------------

    #[test]
    fn test_compile_closure() {
        let closure_expr = ast::Expr::Closure {
            params: vec![("x".to_string(), ast::Type::simple("long"))],
            return_type: ast::Type::simple("long"),
            body: vec![],
            expr: Some(Box::new(ast::Expr::Binary(
                Box::new(ast::Expr::Identifier("x".to_string(), su())),
                ast::Operator::Add,
                Box::new(ast::Expr::Literal(ast::Literal::Int(1), su())),
                su(),
            ))),
            captured_vars: vec![],
            span: su(),
        };

        let fn_decl = ast::FnDecl {
            access: ast::Access::Public,
            name: "test_closure".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: None,
            body: vec![ast::Stmt::VarDecl(ast::VarDecl {
                name: "f".to_string(),
                typ: None,
                init: Some(closure_expr),
                mutable: false,
                span: su(),
            })],
            sugar: false,
            where_clause: vec![],
            span: su(),
        };

        let program = ast::Program {
            imports: vec![],
            declarations: vec![ast::Declaration::Function(fn_decl)],
        };

        let mut compiler = Compiler::new();
        let compiled = compiler.compile(&program).expect("compilation should succeed");

        let fn_chunk = &compiled.functions[1].chunk;
        assert!(
            fn_chunk.code.contains(&(OpCode::CLOSURE_NEW as u8)),
            "closure expression should emit CLOSURE_NEW"
        );
    }

    // -- test_compile_tuple -------------------------------------------------------

    #[test]
    fn test_compile_tuple() {
        let tuple_expr = ast::Expr::Tuple(vec![
            ast::Expr::Literal(ast::Literal::Int(1), su()),
            ast::Expr::Literal(ast::Literal::Int(2), su()),
            ast::Expr::Literal(ast::Literal::Int(3), su()),
        ], su());

        let fn_decl = ast::FnDecl {
            access: ast::Access::Public,
            name: "test_tuple".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: None,
            body: vec![ast::Stmt::VarDecl(ast::VarDecl {
                name: "t".to_string(),
                typ: None,
                init: Some(tuple_expr),
                mutable: false,
                span: su(),
            })],
            sugar: false,
            where_clause: vec![],
            span: su(),
        };

        let program = ast::Program {
            imports: vec![],
            declarations: vec![ast::Declaration::Function(fn_decl)],
        };

        let mut compiler = Compiler::new();
        let compiled = compiler.compile(&program).expect("compilation should succeed");

        let fn_chunk = &compiled.functions[1].chunk;
        assert!(
            fn_chunk.code.contains(&(OpCode::TUPLE_NEW as u8)),
            "tuple expression should emit TUPLE_NEW"
        );
    }

    // -- test_compile_operator_overload -------------------------------------------

    #[test]
    fn test_compile_operator_overload() {
        let class_decl = ast::ClassDecl {
            name: "Vec2".to_string(),
            type_params: vec![],
            parent: None,
            ifaces: vec![],
            members: vec![
                ast::ClassMember::Field(ast::FieldDecl {
                    access: ast::Access::Public,
                    name: "x".to_string(),
                    typ: ast::Type::simple("long"),
                    init: Some(ast::Expr::Literal(ast::Literal::Int(0), su())),
                    span: su(),
                }),
                ast::ClassMember::Field(ast::FieldDecl {
                    access: ast::Access::Public,
                    name: "y".to_string(),
                    typ: ast::Type::simple("long"),
                    init: Some(ast::Expr::Literal(ast::Literal::Int(0), su())),
                    span: su(),
                }),
                ast::ClassMember::Method(ast::MethodDecl {
                    access: ast::Access::Public,
                    name: "operator+".to_string(),
                    type_params: vec![],
                    params: vec![ast::Param {
                        name: "other".to_string(),
                        typ: ast::Type::simple("Vec2"),
                    }],
                    return_type: Some(ast::Type::simple("Vec2")),
                    body: vec![ast::Stmt::Return(Some(ast::Expr::New(
                        ast::Type::simple("Vec2"),
                        vec![],
                        su(),
                    )))],
                    where_clause: vec![],
                    span: su(),
                }),
            ],
            span: su(),
        };

        let program = ast::Program {
            imports: vec![],
            declarations: vec![ast::Declaration::Class(class_decl)],
        };

        let mut compiler = Compiler::new();
        let compiled = compiler.compile(&program).expect("compilation should succeed");

        assert_eq!(compiled.classes.len(), 1);
        assert_eq!(compiled.classes[0].name, "Vec2");
        // The operator+ method should be registered in the class methods
        assert!(
            compiled.classes[0].methods.contains_key("operator+"),
            "Vec2 class should have operator+ method"
        );
    }

    // -- test_compile_for_in -----------------------------------------------------

    #[test]
    fn test_compile_for_in() {
        let list_decl = ast::Stmt::VarDecl(ast::VarDecl {
            name: "list".to_string(),
            typ: Some(ast::Type::generic(
                "ArrayList",
                vec![ast::Type::simple("string")],
            )),
            init: None,
            mutable: false,
            span: su(),
        });
        let for_stmt = ast::Stmt::For(ast::ForStmt {
            var: "item".to_string(),
            iterable: ast::Expr::Identifier("list".to_string(), su()),
            body: vec![ast::Stmt::Expr(ast::Expr::Identifier("item".to_string(), su()))],
            span: su(),
        });

        let fn_decl = ast::FnDecl {
            access: ast::Access::Public,
            name: "test_for_in".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: None,
            body: vec![list_decl, for_stmt],
            sugar: false,
            where_clause: vec![],
            span: su(),
        };

        let program = ast::Program {
            imports: vec![],
            declarations: vec![ast::Declaration::Function(fn_decl)],
        };

        let mut compiler = Compiler::new();
        let compiled = compiler.compile(&program).expect("compilation should succeed");

        let fn_chunk = &compiled.functions[1].chunk;
        // For-in loop should contain JMP_IF_FALSE for the loop condition
        assert!(
            fn_chunk.code.contains(&(OpCode::JMP_IF_FALSE as u8)),
            "for-in loop should emit JMP_IF_FALSE"
        );
        // And a JMP back to the start
        assert!(
            fn_chunk.code.contains(&(OpCode::JMP as u8)),
            "for-in loop should emit JMP back to start"
        );
    }

    // -- test_compile_c_style_for ------------------------------------------------

    #[test]
    fn test_compile_c_style_for() {
        let cfor_stmt = ast::Stmt::CFor(ast::CForStmt {
            init: Some(Box::new(ast::Stmt::VarDecl(ast::VarDecl {
                name: "i".to_string(),
                typ: None,
                init: Some(ast::Expr::Literal(ast::Literal::Int(0), su())),
                mutable: true,
                span: su(),
            }))),
            condition: Some(ast::Expr::Binary(
                Box::new(ast::Expr::Identifier("i".to_string(), su())),
                ast::Operator::Lt,
                Box::new(ast::Expr::Literal(ast::Literal::Int(10), su())),
                su(),
            )),
            increment: Some(ast::Expr::Assign(
                Box::new(ast::Expr::Identifier("i".to_string(), su())),
                Box::new(ast::Expr::Binary(
                    Box::new(ast::Expr::Identifier("i".to_string(), su())),
                    ast::Operator::Add,
                    Box::new(ast::Expr::Literal(ast::Literal::Int(1), su())),
                    su(),
                )),
                su(),
            )),
            body: vec![ast::Stmt::Break],
            span: su(),
        });

        let fn_decl = ast::FnDecl {
            access: ast::Access::Public,
            name: "test_c_for".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: None,
            body: vec![cfor_stmt],
            sugar: false,
            where_clause: vec![],
            span: su(),
        };

        let program = ast::Program {
            imports: vec![],
            declarations: vec![ast::Declaration::Function(fn_decl)],
        };

        let mut compiler = Compiler::new();
        let compiled = compiler.compile(&program).expect("compilation should succeed");

        let fn_chunk = &compiled.functions[1].chunk;
        assert!(
            fn_chunk.code.contains(&(OpCode::JMP_IF_FALSE as u8)),
            "C-style for should emit JMP_IF_FALSE"
        );
        assert!(
            fn_chunk.code.contains(&(OpCode::JMP as u8)),
            "C-style for should emit JMP"
        );
    }

    // -- test_compile_while_let --------------------------------------------------

    #[test]
    fn test_compile_while_let() {
        let file_decl = ast::Stmt::VarDecl(ast::VarDecl {
            name: "file".to_string(),
            typ: Some(ast::Type::simple("File")),
            init: None,
            mutable: false,
            span: su(),
        });
        let while_let_stmt = ast::Stmt::WhileLet(ast::WhileLetStmt {
            var_name: "line".to_string(),
            expr: ast::Expr::Call(
                Box::new(ast::Expr::MemberAccess(
                    Box::new(ast::Expr::Identifier("file".to_string(), su())),
                    "readLine".to_string(),
                    su(),
                )),
                vec![],
                su(),
            ),
            body: vec![ast::Stmt::Break],
            span: su(),
        });

        let fn_decl = ast::FnDecl {
            access: ast::Access::Public,
            name: "test_while_let".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: None,
            body: vec![file_decl, while_let_stmt],
            sugar: false,
            where_clause: vec![],
            span: su(),
        };

        let program = ast::Program {
            imports: vec![],
            declarations: vec![ast::Declaration::Function(fn_decl)],
        };

        let mut compiler = Compiler::new();
        let compiled = compiler.compile(&program).expect("compilation should succeed");

        let fn_chunk = &compiled.functions[1].chunk;
        assert!(
            fn_chunk.code.contains(&(OpCode::JMP_IF_FALSE as u8)),
            "while-let should emit JMP_IF_FALSE"
        );
        assert!(
            fn_chunk.code.contains(&(OpCode::JMP as u8)),
            "while-let should emit JMP back to start"
        );
    }

    // -- test_compile_switch_enum ------------------------------------------------

    #[test]
    fn test_compile_switch_enum() {
        let enum_decl = ast::EnumDecl {
            name: "Color".to_string(),
            type_params: vec![],
            variants: vec![
                ast::Variant {
                    name: "Red".to_string(),
                    fields: vec![],
                },
                ast::Variant {
                    name: "Blue".to_string(),
                    fields: vec![],
                },
            ],
            span: su(),
        };

        let switch_stmt = ast::Stmt::Switch(ast::SwitchStmt {
            expr: ast::Expr::Identifier("color".to_string(), su()),
            cases: vec![
                ast::Case {
                    pattern: ast::Pattern::Constructor {
                        name: "Red".to_string(),
                        bindings: vec![],
                    },
                    body: vec![ast::Stmt::Expr(ast::Expr::Literal(ast::Literal::Int(1), su()))],
                },
                ast::Case {
                    pattern: ast::Pattern::Constructor {
                        name: "Blue".to_string(),
                        bindings: vec![],
                    },
                    body: vec![ast::Stmt::Expr(ast::Expr::Literal(ast::Literal::Int(2), su()))],
                },
            ],
            default: None,
            span: su(),
        });

        let fn_decl = ast::FnDecl {
            access: ast::Access::Public,
            name: "test_switch_enum".to_string(),
            type_params: vec![],
            params: vec![ast::Param {
                name: "color".to_string(),
                typ: ast::Type::simple("Color"),
            }],
            return_type: None,
            body: vec![switch_stmt],
            sugar: false,
            where_clause: vec![],
            span: su(),
        };

        let program = ast::Program {
            imports: vec![],
            declarations: vec![
                ast::Declaration::Enum(enum_decl),
                ast::Declaration::Function(fn_decl),
            ],
        };

        let mut compiler = Compiler::new();
        let compiled = compiler.compile(&program).expect("compilation should succeed");

        let fn_chunk = &compiled.functions[1].chunk;
        assert!(
            fn_chunk.code.contains(&(OpCode::DUP as u8)),
            "switch on enum should DUP the subject"
        );
        assert!(
            fn_chunk.code.contains(&(OpCode::JMP_IF_FALSE as u8)),
            "switch on enum should use JMP_IF_FALSE"
        );
    }

    // -- test_compile_where_clause -----------------------------------------------

    #[test]
    fn test_compile_where_clause() {
        // Compile a non-generic function with an empty where clause
        // (where clause is for type checking; compilation should succeed)
        let fn_decl = ast::FnDecl {
            access: ast::Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![ast::Param {
                name: "x".to_string(),
                typ: ast::Type::simple("int"),
            }],
            return_type: Some(ast::Type::simple("void")),
            body: vec![ast::Stmt::Return(None)],
            sugar: false,
            where_clause: vec![],
            span: su(),
        };

        let program = ast::Program {
            imports: vec![],
            declarations: vec![ast::Declaration::Function(fn_decl)],
        };

        let mut compiler = Compiler::new();
        let compiled = compiler.compile(&program).expect("compilation should succeed");

        // The function should be compiled
        assert!(
            compiled.functions.len() >= 2,
            "compiled program should contain main and foo functions"
        );

        // Also test that a generic function with where clause can be registered
        let generic_fn = ast::FnDecl {
            access: ast::Access::Public,
            name: "bar".to_string(),
            type_params: vec![ast::TypeParam {
                name: "T".to_string(),
                constraint: Some(ast::Type::simple("Comparable")),
            }],
            params: vec![ast::Param {
                name: "x".to_string(),
                typ: ast::Type::simple("T"),
            }],
            return_type: Some(ast::Type::simple("void")),
            body: vec![ast::Stmt::Return(None)],
            sugar: false,
            where_clause: vec![ast::TypeParam {
                name: "T".to_string(),
                constraint: Some(ast::Type::simple("Comparable")),
            }],
            span: su(),
        };

        let mut compiler2 = Compiler::new();
        // Register the generic function (should not fail on registration)
        compiler2.register_function(&generic_fn);
        // Generic functions are stored in generic_function_map, not function_map
        assert!(
            compiler2.generic_function_map.contains_key("bar"),
            "generic function should be registered in generic_function_map"
        );
        assert_eq!(
            compiler2.generic_functions.len(),
            1,
            "generic_functions should contain one entry"
        );
    }

    // =========================================================================
    // Constant folding tests
    // =========================================================================

    /// Helper: build a chunk with PUSH_I64 + PUSH_I64 + binary-op, run
    /// constant folding, and return the resulting chunk.
    fn fold_i64_chunk(va: i64, vb: i64, op: OpCode) -> Chunk {
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::PUSH_I64, 1);
        chunk.code.extend_from_slice(&va.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 8]);
        chunk.write_opcode(OpCode::PUSH_I64, 1);
        chunk.code.extend_from_slice(&vb.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 8]);
        chunk.write_opcode(op, 1);
        chunk.write_opcode(OpCode::RET, 1);
        Compiler::fold_constants_chunk(&mut chunk);
        chunk
    }

    fn fold_i32_chunk(va: i32, vb: i32, op: OpCode) -> Chunk {
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&va.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&vb.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(op, 1);
        chunk.write_opcode(OpCode::RET, 1);
        Compiler::fold_constants_chunk(&mut chunk);
        chunk
    }

    #[test]
    fn test_fold_i64_add() {
        let chunk = fold_i64_chunk(3, 4, OpCode::ADD_I64);
        // After folding: PUSH_I64 7, RET  (no ADD_I64)
        assert!(!chunk.code.contains(&(OpCode::ADD_I64 as u8)),
            "ADD_I64 should be folded away");
        assert!(chunk.code.contains(&(OpCode::PUSH_I64 as u8)),
            "should still have PUSH_I64");
        // Find the PUSH_I64 and verify the value
        let push_offset = chunk.code.iter().position(|&b| b == OpCode::PUSH_I64 as u8).unwrap();
        let val = i64::from_be_bytes(chunk.code[push_offset+1..push_offset+9].try_into().unwrap());
        assert_eq!(val, 7);
    }

    #[test]
    fn test_fold_i64_sub() {
        let chunk = fold_i64_chunk(10, 3, OpCode::SUB_I64);
        assert!(!chunk.code.contains(&(OpCode::SUB_I64 as u8)));
        let push_offset = chunk.code.iter().position(|&b| b == OpCode::PUSH_I64 as u8).unwrap();
        let val = i64::from_be_bytes(chunk.code[push_offset+1..push_offset+9].try_into().unwrap());
        assert_eq!(val, 7);
    }

    #[test]
    fn test_fold_i64_mul() {
        let chunk = fold_i64_chunk(6, 7, OpCode::MUL_I64);
        assert!(!chunk.code.contains(&(OpCode::MUL_I64 as u8)));
        let push_offset = chunk.code.iter().position(|&b| b == OpCode::PUSH_I64 as u8).unwrap();
        let val = i64::from_be_bytes(chunk.code[push_offset+1..push_offset+9].try_into().unwrap());
        assert_eq!(val, 42);
    }

    #[test]
    fn test_fold_i32_add() {
        let chunk = fold_i32_chunk(20, 22, OpCode::ADD_I32);
        assert!(!chunk.code.contains(&(OpCode::ADD_I32 as u8)));
        let push_offset = chunk.code.iter().position(|&b| b == OpCode::PUSH_I32 as u8).unwrap();
        let val = i32::from_be_bytes(chunk.code[push_offset+1..push_offset+5].try_into().unwrap());
        assert_eq!(val, 42);
    }

    #[test]
    fn test_fold_i32_sub() {
        let chunk = fold_i32_chunk(100, 58, OpCode::SUB_I32);
        assert!(!chunk.code.contains(&(OpCode::SUB_I32 as u8)));
        let push_offset = chunk.code.iter().position(|&b| b == OpCode::PUSH_I32 as u8).unwrap();
        let val = i32::from_be_bytes(chunk.code[push_offset+1..push_offset+5].try_into().unwrap());
        assert_eq!(val, 42);
    }

    #[test]
    fn test_fold_i32_mul() {
        let chunk = fold_i32_chunk(6, 7, OpCode::MUL_I32);
        assert!(!chunk.code.contains(&(OpCode::MUL_I32 as u8)));
        let push_offset = chunk.code.iter().position(|&b| b == OpCode::PUSH_I32 as u8).unwrap();
        let val = i32::from_be_bytes(chunk.code[push_offset+1..push_offset+5].try_into().unwrap());
        assert_eq!(val, 42);
    }

    #[test]
    fn test_fold_string_concat() {
        let mut chunk = Chunk::new();
        let idx_a = chunk.add_string("hello");
        let idx_b = chunk.add_string(" world");
        chunk.write_opcode(OpCode::PUSH_STRING, 1);
        chunk.write_u16(idx_a, 1);
        chunk.write_opcode(OpCode::PUSH_STRING, 1);
        chunk.write_u16(idx_b, 1);
        chunk.write_opcode(OpCode::STR_CONCAT, 1);
        chunk.write_opcode(OpCode::RET, 1);

        Compiler::fold_constants_chunk(&mut chunk);

        // After folding: PUSH_STRING <new_idx>, RET  (no STR_CONCAT)
        assert!(!chunk.code.contains(&(OpCode::STR_CONCAT as u8)),
            "STR_CONCAT should be folded away");
        // The combined string "hello world" should be in the string table
        assert!(chunk.strings.iter().any(|s| s == "hello world"),
            "string table should contain 'hello world'");
    }

    #[test]
    fn test_fold_does_not_affect_non_constant() {
        // PUSH_I64 5, LOAD_LOCAL 0, ADD_I64 — should NOT fold
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::PUSH_I64, 1);
        chunk.code.extend_from_slice(&5i64.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 8]);
        chunk.write_opcode(OpCode::LOAD_LOCAL, 1);
        chunk.write_u8(0, 1);
        chunk.write_opcode(OpCode::ADD_I64, 1);
        chunk.write_opcode(OpCode::RET, 1);

        Compiler::fold_constants_chunk(&mut chunk);

        assert!(chunk.code.contains(&(OpCode::ADD_I64 as u8)),
            "ADD_I64 should NOT be folded when operands aren't both constants");
        assert!(chunk.code.contains(&(OpCode::LOAD_LOCAL as u8)),
            "LOAD_LOCAL should remain");
    }

    // =========================================================================
    // Dead code elimination tests
    // =========================================================================

    #[test]
    fn test_dead_code_after_ret() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&42i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::RET, 1);
        // Dead code: these should be eliminated
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&99i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::POP, 1);
        chunk.write_opcode(OpCode::RET, 1);

        Compiler::eliminate_dead_code_chunk(&mut chunk);

        // The dead PUSH_I32 99 and POP should be gone
        assert!(!chunk.code.contains(&(OpCode::POP as u8)),
            "POP after RET should be eliminated");
        // Only one PUSH_I32 should remain (the 42)
        let push_count = chunk.code.iter().filter(|&&b| b == OpCode::PUSH_I32 as u8).count();
        assert_eq!(push_count, 1, "only one PUSH_I32 should remain after dead code elimination");
    }

    #[test]
    fn test_dead_code_after_jmp() {
        let mut chunk = Chunk::new();
        // PUSH_BOOL true
        chunk.write_opcode(OpCode::PUSH_BOOL, 1);
        chunk.write_u8(1, 1);
        // JMP +3 (skip over the dead code)
        chunk.write_opcode(OpCode::JMP, 1);
        chunk.write_i16(3, 1);
        // Dead code:
        chunk.write_opcode(OpCode::PUSH_NULL, 1);
        chunk.write_opcode(OpCode::POP, 1);
        // Target of jump:
        chunk.write_opcode(OpCode::RET, 1);

        Compiler::eliminate_dead_code_chunk(&mut chunk);

        assert!(!chunk.code.contains(&(OpCode::PUSH_NULL as u8)),
            "PUSH_NULL after unconditional JMP should be eliminated");
        assert!(!chunk.code.contains(&(OpCode::POP as u8)),
            "POP after unconditional JMP should be eliminated");
    }

    #[test]
    fn test_dead_code_preserves_jump_targets() {
        // Code: JMP to label, dead code, label: RET
        // The jump target (label) must be preserved even though it follows
        // an unconditional JMP.
        let mut chunk = Chunk::new();
        // JMP +3 (jump over the dead PUSH_NULL+POP to the RET)
        chunk.write_opcode(OpCode::JMP, 1);
        chunk.write_i16(3, 1);
        // Dead code
        chunk.write_opcode(OpCode::PUSH_NULL, 1);
        chunk.write_opcode(OpCode::POP, 1);
        // Jump target: RET
        chunk.write_opcode(OpCode::RET, 1);

        Compiler::eliminate_dead_code_chunk(&mut chunk);

        // RET must still be present (it's a jump target)
        assert!(chunk.code.contains(&(OpCode::RET as u8)),
            "RET jump target must be preserved");
    }

    #[test]
    fn test_remove_unused_strings() {
        let mut chunk = Chunk::new();
        let used_idx = chunk.add_string("used");
        let _unused_idx = chunk.add_string("unused");
        chunk.write_opcode(OpCode::PUSH_STRING, 1);
        chunk.write_u16(used_idx, 1);
        chunk.write_opcode(OpCode::RET, 1);

        Compiler::remove_unused_strings(&mut chunk);

        assert_eq!(chunk.strings.len(), 1, "unused string should be removed");
        assert_eq!(chunk.strings[0], "used");
        // The PUSH_STRING operand should be remapped to index 0
        let push_offset = chunk.code.iter().position(|&b| b == OpCode::PUSH_STRING as u8).unwrap();
        let new_idx = u16::from_be_bytes([chunk.code[push_offset+1], chunk.code[push_offset+2]]);
        assert_eq!(new_idx, 0, "string index should be remapped to 0");
    }

    #[test]
    fn test_too_many_local_variables() {
        // Create a function with 256 parameters, which exceeds the 255 local variable limit
        let params: Vec<ast::Param> = (0..256)
            .map(|i| ast::Param {
                name: format!("v{}", i),
                typ: ast::Type::simple("long"),
            })
            .collect();

        let declarations = vec![ast::Declaration::Function(ast::FnDecl {
            access: ast::Access::Public,
            name: "too_many_locals".to_string(),
            type_params: vec![],
            params,
            return_type: None,
            body: vec![],
            sugar: false,
            where_clause: vec![],
            span: su(),
        })];

        let program = ast::Program {
            imports: vec![],
            declarations,
        };
        let mut compiler = Compiler::new();
        let result = compiler.compile(&program);
        match result {
            Err(msg) => assert_eq!(msg, "Too many local variables in function (max 255)",
                "Expected local variable overflow error, got: {}", msg),
            Ok(_) => panic!("Expected compilation error for too many local variables"),
        }
    }
