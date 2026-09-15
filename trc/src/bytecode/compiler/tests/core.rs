use super::{compile_program, su};
use super::super::Compiler;
use crate::ast;
use crate::bytecode::opcodes::OpCode;

    // -- test_compile_literal_int ------------------------------------------------

    #[test]
    fn test_compile_literal_int() {
        let compiled = compile_program(vec![ast::Declaration::VarDecl(ast::VarDecl {
            name: "x".to_string(),
            typ: None,
            init: Some(ast::Expr::Literal(ast::Literal::Int(42), su())),
            mutable: false,
            span: su(),
        })]);

        let main_chunk = &compiled.functions[0].chunk;
        // Should contain PUSH_I64 opcode somewhere.
        assert!(
            main_chunk.code.contains(&(OpCode::PUSH_I64 as u8)),
            "main chunk should contain PUSH_I64 for integer literal"
        );
    }

    // -- test_compile_literal_string ---------------------------------------------

    #[test]
    fn test_compile_literal_string() {
        let compiled = compile_program(vec![ast::Declaration::VarDecl(ast::VarDecl {
            name: "s".to_string(),
            typ: None,
            init: Some(ast::Expr::Literal(ast::Literal::String("hello".to_string()), su())),
            mutable: false,
            span: su(),
        })]);

        let main_chunk = &compiled.functions[0].chunk;
        assert!(
            main_chunk.code.contains(&(OpCode::PUSH_STRING as u8)),
            "main chunk should contain PUSH_STRING for string literal"
        );
        assert!(
            main_chunk.strings.contains(&"hello".to_string()),
            "string table should contain 'hello'"
        );
    }

    // -- test_compile_binary_add -------------------------------------------------

    #[test]
    fn test_compile_binary_add() {
        // Use variables so constant folding cannot eliminate the ADD opcode
        let compiled = compile_program(vec![
            ast::Declaration::VarDecl(ast::VarDecl {
                name: "a".to_string(),
                typ: None,
                init: Some(ast::Expr::Literal(ast::Literal::Int(1), su())),
                mutable: false,
                span: su(),
            }),
            ast::Declaration::VarDecl(ast::VarDecl {
                name: "b".to_string(),
                typ: None,
                init: Some(ast::Expr::Literal(ast::Literal::Int(2), su())),
                mutable: false,
                span: su(),
            }),
            ast::Declaration::VarDecl(ast::VarDecl {
                name: "x".to_string(),
                typ: None,
                init: Some(ast::Expr::Binary(
                    Box::new(ast::Expr::Identifier("a".to_string(), su())),
                    ast::Operator::Add,
                    Box::new(ast::Expr::Identifier("b".to_string(), su())),
                    su(),
                )),
                mutable: false,
                span: su(),
            }),
        ]);

        let main_chunk = &compiled.functions[0].chunk;
        assert!(
            main_chunk.code.contains(&(OpCode::ADD_I64 as u8)),
            "main chunk should contain ADD_I64 for integer addition"
        );
    }

    // -- test_class_locals_emit_invoke_operator ----------------------------------

    #[test]
    fn test_class_locals_emit_invoke_operator() {
        // Locals with declared class types must compile `+` to
        // INVOKE_OPERATOR so operator overloads dispatch at runtime
        // instead of falling through to integer addition.
        let compiled = compile_program(vec![
            ast::Declaration::VarDecl(ast::VarDecl {
                name: "a".to_string(),
                typ: Some(ast::Type::simple("Vec2")),
                init: None,
                mutable: false,
                span: su(),
            }),
            ast::Declaration::VarDecl(ast::VarDecl {
                name: "b".to_string(),
                typ: Some(ast::Type::simple("Vec2")),
                init: None,
                mutable: false,
                span: su(),
            }),
            ast::Declaration::VarDecl(ast::VarDecl {
                name: "c".to_string(),
                typ: Some(ast::Type::simple("Vec2")),
                init: Some(ast::Expr::Binary(
                    Box::new(ast::Expr::Identifier("a".to_string(), su())),
                    ast::Operator::Add,
                    Box::new(ast::Expr::Identifier("b".to_string(), su())),
                    su(),
                )),
                mutable: false,
                span: su(),
            }),
        ]);

        let main_chunk = &compiled.functions[0].chunk;
        assert!(
            main_chunk.code.contains(&(OpCode::INVOKE_OPERATOR as u8)),
            "class-typed `a + b` should compile to INVOKE_OPERATOR"
        );
    }

    // -- test_compile_var_decl_and_load ------------------------------------------

    #[test]
    fn test_compile_var_decl_and_load() {
        let compiled = compile_program(vec![
            ast::Declaration::VarDecl(ast::VarDecl {
                name: "x".to_string(),
                typ: None,
                init: Some(ast::Expr::Literal(ast::Literal::Int(10), su())),
                mutable: false,
                span: su(),
            }),
            ast::Declaration::VarDecl(ast::VarDecl {
                name: "y".to_string(),
                typ: None,
                init: Some(ast::Expr::Identifier("x".to_string(), su())),
                mutable: false,
                span: su(),
            }),
        ]);

        let main_chunk = &compiled.functions[0].chunk;
        assert!(
            main_chunk.code.contains(&(OpCode::STORE_LOCAL as u8)),
            "main chunk should contain STORE_LOCAL"
        );
        assert!(
            main_chunk.code.contains(&(OpCode::LOAD_LOCAL as u8)),
            "main chunk should contain LOAD_LOCAL"
        );
    }

    // -- test_compile_if_else ----------------------------------------------------

    #[test]
    fn test_compile_if_else() {
        let _compiled = compile_program(vec![ast::Declaration::VarDecl(ast::VarDecl {
            name: "x".to_string(),
            typ: None,
            init: Some(ast::Expr::Literal(ast::Literal::Int(0), su())),
            mutable: false,
            span: su(),
        })]);

        // Build an if-else as a statement in the main chunk.
        let _program = ast::Program {
            imports: vec![],
            declarations: vec![ast::Declaration::VarDecl(ast::VarDecl {
                name: "x".to_string(),
                typ: None,
                init: None,
                mutable: true,
                span: su(),
            })],
        };

        // We need to compile an if statement. Let's build it manually.
        let if_stmt = ast::Stmt::If(ast::IfStmt {
            condition: ast::Expr::Literal(ast::Literal::Bool(true), su()),
            then_branch: vec![ast::Stmt::Expr(ast::Expr::Literal(ast::Literal::Int(1), su()))],
            else_branch: Some(vec![ast::Stmt::Expr(ast::Expr::Literal(
                ast::Literal::Int(2),
                su(),
            ))]),
            span: su(),
        });

        let _program = ast::Program {
            imports: vec![],
            declarations: vec![],
        };

        // Since we can't easily add statements to the main chunk through
        // the public API (only declarations), let's test through the
        // compile_stmt method indirectly by using a function.
        let fn_decl = ast::FnDecl {
            access: ast::Access::Public,
            name: "test_if".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: None,
            body: vec![if_stmt],
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
            "if statement should emit JMP_IF_FALSE"
        );
        assert!(
            fn_chunk.code.contains(&(OpCode::JMP as u8)),
            "if-else should emit JMP to skip else branch"
        );
    }

    // -- test_compile_while_loop -------------------------------------------------

    #[test]
    fn test_compile_while_loop() {
        let while_stmt = ast::Stmt::While(ast::WhileStmt {
            condition: ast::Expr::Literal(ast::Literal::Bool(true), su()),
            body: vec![ast::Stmt::Break],
            span: su(),
        });

        let fn_decl = ast::FnDecl {
            access: ast::Access::Public,
            name: "test_while".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: None,
            body: vec![while_stmt],
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
            "while loop should emit JMP_IF_FALSE"
        );
        assert!(
            fn_chunk.code.contains(&(OpCode::JMP as u8)),
            "while loop should emit JMP back to start"
        );
    }

    // -- test_compile_do_while_loop ----------------------------------------------

    #[test]
    fn test_compile_do_while_loop() {
        let do_while_stmt = ast::Stmt::DoWhile(ast::DoWhileStmt {
            body: vec![ast::Stmt::Expr(ast::Expr::Literal(ast::Literal::Int(1), su()))],
            condition: ast::Expr::Literal(ast::Literal::Bool(true), su()),
            span: su(),
        });

        let fn_decl = ast::FnDecl {
            access: ast::Access::Public,
            name: "test_do_while".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: None,
            body: vec![do_while_stmt],
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
            fn_chunk.code.contains(&(OpCode::JMP_IF_TRUE as u8)),
            "do-while loop should emit JMP_IF_TRUE"
        );
    }

    // -- test_compile_function_call ----------------------------------------------

    #[test]
    fn test_compile_function_call() {
        let callee_fn = ast::FnDecl {
            access: ast::Access::Public,
            name: "add".to_string(),
            type_params: vec![],
            params: vec![
                ast::Param {
                    name: "a".to_string(),
                    typ: ast::Type::simple("long"),
                },
                ast::Param {
                    name: "b".to_string(),
                    typ: ast::Type::simple("long"),
                },
            ],
            return_type: Some(ast::Type::simple("long")),
            body: vec![ast::Stmt::Return(Some(ast::Expr::Binary(
                Box::new(ast::Expr::Identifier("a".to_string(), su())),
                ast::Operator::Add,
                Box::new(ast::Expr::Identifier("b".to_string(), su())),
                su(),
            )))],
            sugar: false,
            where_clause: vec![],
            span: su(),
        };

        let caller_fn = ast::FnDecl {
            access: ast::Access::Public,
            name: "main_fn".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: None,
            body: vec![ast::Stmt::Expr(ast::Expr::Call(
                Box::new(ast::Expr::Identifier("add".to_string(), su())),
                vec![
                    ast::Expr::Literal(ast::Literal::Int(1), su()),
                    ast::Expr::Literal(ast::Literal::Int(2), su()),
                ],
                su(),
            ))],
            sugar: false,
            where_clause: vec![],
            span: su(),
        };

        let program = ast::Program {
            imports: vec![],
            declarations: vec![
                ast::Declaration::Function(callee_fn),
                ast::Declaration::Function(caller_fn),
            ],
        };

        let mut compiler = Compiler::new();
        let compiled = compiler.compile(&program).expect("compilation should succeed");

        let caller_chunk = &compiled.functions[2].chunk;
        assert!(
            caller_chunk.code.contains(&(OpCode::CALL as u8)),
            "function call should emit CALL"
        );
    }

    // -- test_compile_class_new --------------------------------------------------

    #[test]
    fn test_compile_class_new() {
        let class_decl = ast::ClassDecl {
            name: "Point".to_string(),
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
            ],
            span: su(),
        };

        let fn_decl = ast::FnDecl {
            access: ast::Access::Public,
            name: "make_point".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: None,
            body: vec![ast::Stmt::Expr(ast::Expr::New(
                ast::Type::simple("Point"),
                vec![],
                su(),
            ))],
            sugar: false,
            where_clause: vec![],
            span: su(),
        };

        let program = ast::Program {
            imports: vec![],
            declarations: vec![
                ast::Declaration::Class(class_decl),
                ast::Declaration::Function(fn_decl),
            ],
        };

        let mut compiler = Compiler::new();
        let compiled = compiler.compile(&program).expect("compilation should succeed");

        assert_eq!(compiled.classes.len(), 1);
        assert_eq!(compiled.classes[0].name, "Point");

        let fn_chunk = &compiled.functions[1].chunk;
        assert!(
            fn_chunk.code.contains(&(OpCode::NEW as u8)),
            "new expression should emit NEW"
        );
    }

    // -- test_compile_enum -------------------------------------------------------

    #[test]
    fn test_compile_enum() {
        let enum_decl = ast::EnumDecl {
            name: "Shape".to_string(),
            type_params: vec![],
            variants: vec![
                ast::Variant {
                    name: "SCircle".to_string(),
                    fields: vec![ast::Param {
                        name: "radius".to_string(),
                        typ: ast::Type::simple("double"),
                    }],
                },
                ast::Variant {
                    name: "SRect".to_string(),
                    fields: vec![
                        ast::Param {
                            name: "w".to_string(),
                            typ: ast::Type::simple("double"),
                        },
                        ast::Param {
                            name: "h".to_string(),
                            typ: ast::Type::simple("double"),
                        },
                    ],
                },
            ],
            span: su(),
        };

        let fn_decl = ast::FnDecl {
            access: ast::Access::Public,
            name: "make_circle".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: None,
            body: vec![ast::Stmt::VarDecl(ast::VarDecl {
                name: "c".to_string(),
                typ: None,
                init: Some(ast::Expr::Call(
                    Box::new(ast::Expr::Identifier("SCircle".to_string(), su())),
                    vec![ast::Expr::Literal(ast::Literal::Float(3.0), su())],
                    su(),
                )),
                mutable: false,
                span: su(),
            })],
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

        assert_eq!(compiled.enums.len(), 1);
        assert_eq!(compiled.enums[0].name, "Shape");
        assert_eq!(compiled.enums[0].variants.len(), 2);
        assert_eq!(compiled.enums[0].variants[0].name, "SCircle");
        assert_eq!(compiled.enums[0].variants[0].field_count, 1);
        assert_eq!(compiled.enums[0].variants[1].name, "SRect");
        assert_eq!(compiled.enums[0].variants[1].field_count, 2);

        let fn_chunk = &compiled.functions[1].chunk;
        assert!(
            fn_chunk.code.contains(&(OpCode::ENUM_NEW as u8)),
            "enum variant constructor should emit ENUM_NEW"
        );
    }

    // -- test_compile_switch -----------------------------------------------------

    #[test]
    fn test_compile_switch() {
        let switch_stmt = ast::Stmt::Switch(ast::SwitchStmt {
            expr: ast::Expr::Identifier("x".to_string(), su()),
            cases: vec![
                ast::Case {
                    pattern: ast::Pattern::Literal(ast::Literal::Int(1)),
                    body: vec![ast::Stmt::Expr(ast::Expr::Literal(ast::Literal::Int(
                        10,
                    ), su()))],
                },
                ast::Case {
                    pattern: ast::Pattern::Literal(ast::Literal::Int(2)),
                    body: vec![ast::Stmt::Expr(ast::Expr::Literal(ast::Literal::Int(
                        20,
                    ), su()))],
                },
            ],
            default: Some(vec![ast::Stmt::Expr(ast::Expr::Literal(
                ast::Literal::Int(0),
                su(),
            ))]),
            span: su(),
        });

        let fn_decl = ast::FnDecl {
            access: ast::Access::Public,
            name: "test_switch".to_string(),
            type_params: vec![],
            params: vec![ast::Param {
                name: "x".to_string(),
                typ: ast::Type::simple("long"),
            }],
            return_type: None,
            body: vec![switch_stmt],
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
            fn_chunk.code.contains(&(OpCode::DUP as u8)),
            "switch should DUP the subject for each case"
        );
        assert!(
            fn_chunk.code.contains(&(OpCode::JMP_IF_FALSE as u8)),
            "switch cases should use JMP_IF_FALSE"
        );
    }
