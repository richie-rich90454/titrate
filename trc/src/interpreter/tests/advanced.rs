use super::{make_fn_decl, make_program, println_call};
use crate::interpreter::{Interpreter, Memory, Value, interpret};
use crate::ast::*;

    // ---- Division by zero ----

    #[test]
    fn test_division_by_zero() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::Expr(Expr::Binary(
                    Box::new(Expr::Literal(Literal::Int(1), Span::unknown())),
                    Operator::Div,
                    Box::new(Expr::Literal(Literal::Int(0), Span::unknown())),
                    Span::unknown(),
                )),
            ])),
        ]);
        let result = interpret(&program);
        assert!(result.is_err());
        assert!(result.err().is_some_and(|e| e.contains("zero")));
    }

    // ---- Undefined variable ----

    #[test]
    fn test_undefined_variable() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::Expr(Expr::Identifier("unknown".to_string(), Span::unknown())),
            ])),
        ]);
        let result = interpret(&program);
        assert!(result.is_err());
    }

    // ---- Null literal ----

    #[test]
    fn test_null_literal() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Literal(Literal::Null, Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"null".to_string()));
    }

    // ---- Char literal ----

    #[test]
    fn test_char_literal() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Literal(Literal::Char('A'), Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"A".to_string()));
    }

    // ---- Float arithmetic ----

    #[test]
    fn test_float_arithmetic() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Binary(
                    Box::new(Expr::Literal(Literal::Float(1.5), Span::unknown())),
                    Operator::Add,
                    Box::new(Expr::Literal(Literal::Float(2.5), Span::unknown())),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"4".to_string()));
    }

    // ---- Block scoping ----

    #[test]
    fn test_block_scoping() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::simple("long")),
                    init: Some(Expr::Literal(Literal::Int(1), Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::Block(vec![
                    Stmt::VarDecl(VarDecl {
                        name: "x".to_string(),
                        typ: Some(Type::simple("long")),
                        init: Some(Expr::Literal(Literal::Int(2), Span::unknown())),
                        mutable: false,
                        span: Span::unknown(),
                    }),
                    println_call(Expr::Identifier("x".to_string(), Span::unknown())),
                ]),
                println_call(Expr::Identifier("x".to_string(), Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output[0], "2");
        assert_eq!(output[1], "1");
    }

    // ---- No main function ----

    #[test]
    fn test_no_main() {
        let program = make_program(vec![]);
        let result = interpret(&program);
        assert!(result.is_ok());
    }

    // ---- Return void ----

    #[test]
    fn test_return_void() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::Return(None),
            ])),
        ]);
        let result = interpret(&program);
        assert!(result.is_ok());
    }

    // ---- Ne operator ----

    #[test]
    fn test_ne_operator() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Binary(
                    Box::new(Expr::Literal(Literal::Int(1), Span::unknown())),
                    Operator::Ne,
                    Box::new(Expr::Literal(Literal::Int(2), Span::unknown())),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"true".to_string()));
    }

    // ---- Le and Ge operators ----

    #[test]
    fn test_le_ge_operators() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Binary(
                    Box::new(Expr::Literal(Literal::Int(3), Span::unknown())),
                    Operator::Le,
                    Box::new(Expr::Literal(Literal::Int(3), Span::unknown())),
                    Span::unknown(),
                )),
                println_call(Expr::Binary(
                    Box::new(Expr::Literal(Literal::Int(5), Span::unknown())),
                    Operator::Ge,
                    Box::new(Expr::Literal(Literal::Int(3), Span::unknown())),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output[0], "true");
        assert_eq!(output[1], "true");
    }

    // ---- Bool literal ----

    #[test]
    fn test_bool_literal() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Literal(Literal::Bool(true), Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"true".to_string()));
    }

    // ---- Owned type ----

    #[test]
    fn test_owned_deref() {
        let owned = Value::Owned(Box::new(Value::Long(42)));
        match owned {
            Value::Owned(inner) => assert_eq!(*inner, Value::Long(42)),
            _ => panic!("Expected Owned"),
        }
    }

    // ---- Moved sentinel ----

    #[test]
    fn test_moved_value() {
        let moved = Value::Moved;
        assert!(!moved.is_truthy());
        assert_eq!(moved.display_string(), "<moved>");
    }

    // ---- Memory operations ----

    #[test]
    fn test_memory_alloc_and_read() {
        let mut mem = Memory::new();
        let idx = mem.alloc(Value::Long(42));
        let val = mem.read(idx);
        assert!(val.is_ok());
        assert_eq!(val.ok(), Some(Value::Long(42)));
    }

    #[test]
    fn test_memory_write() {
        let mut mem = Memory::new();
        let idx = mem.alloc(Value::Long(42));
        let write_result = mem.write(idx, Value::Long(100));
        assert!(write_result.is_ok());
        let val = mem.read(idx);
        assert_eq!(val.ok(), Some(Value::Long(100)));
    }

    #[test]
    fn test_memory_out_of_bounds() {
        let mem = Memory::new();
        let val = mem.read(999);
        assert!(val.is_err());
    }

    #[test]
    fn test_region_alloc_and_pop() {
        let mut mem = Memory::new();
        mem.push_region();
        let idx = mem.region_alloc(Value::Long(42));
        let val = mem.read(idx);
        assert_eq!(val.ok(), Some(Value::Long(42)));
        mem.pop_region();
        let val_after = mem.read(idx);
        assert_eq!(val_after.ok(), Some(Value::Void));
    }

    // ---- Raw memory ----

    #[test]
    fn test_raw_memory() {
        let mut mem = Memory::new();
        let offset = mem.raw_alloc(&[1, 2, 3, 4]);
        let data = mem.raw_read(offset, 4);
        assert!(data.is_ok());
        assert_eq!(data.ok(), Some(vec![1, 2, 3, 4]));
    }

    // ---- RefExpr ----

    #[test]
    fn test_ref_expr() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "r".to_string(),
                    typ: Some(Type::simple("ref")),
                    init: Some(Expr::RefExpr(
                        Box::new(Expr::Literal(Literal::Int(42), Span::unknown())),
                        RefKind::Immutable,
                        Span::unknown(),
                    )),
                    mutable: false,
                    span: Span::unknown(),
                }),
                println_call(Expr::Identifier("r".to_string(), Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        // Should print ref(0) since it's a reference to memory slot 0
        assert!(output.last().is_some_and(|s| s.starts_with("ref(")));
    }

    // ---- Unsafe block ----

    #[test]
    fn test_unsafe_block() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::Expr(Expr::UnsafeBlock(vec![
                    Stmt::Expr(Expr::Call(
                        Box::new(Expr::MemberAccess(
                            Box::new(Expr::Identifier("io".to_string(), Span::unknown())),
                            "println".to_string(),
                            Span::unknown(),
                        )),
                        vec![Expr::Literal(Literal::String("unsafe".to_string()), Span::unknown())],
                        Span::unknown(),
                    )),
                ], Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"unsafe".to_string()));
    }

    // ---- Wildcard pattern in switch ----

    #[test]
    fn test_wildcard_pattern() {
        let program = make_program(vec![
            Declaration::Enum(EnumDecl {
                name: "Color".to_string(),
                type_params: vec![],
                variants: vec![
                    Variant { name: "Red".to_string(), fields: vec![] },
                    Variant { name: "Blue".to_string(), fields: vec![] },
                ],
                span: Span::unknown(),
            }),
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::Switch(SwitchStmt {
                    expr: Expr::Call(
                        Box::new(Expr::Identifier("Blue".to_string(), Span::unknown())),
                        vec![],
                        Span::unknown(),
                    ),
                    cases: vec![
                        Case {
                            pattern: Pattern::Wildcard,
                            body: vec![
                                println_call(Expr::Literal(Literal::String("matched".to_string()), Span::unknown())),
                            ],
                        },
                    ],
                    default: None,
                    span: Span::unknown(),
                }),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"matched".to_string()));
    }

    // ---- Literal pattern in switch ----

    #[test]
    fn test_literal_pattern() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::Switch(SwitchStmt {
                    expr: Expr::Literal(Literal::Int(42), Span::unknown()),
                    cases: vec![
                        Case {
                            pattern: Pattern::Literal(Literal::Int(42)),
                            body: vec![
                                println_call(Expr::Literal(Literal::String("found".to_string()), Span::unknown())),
                            ],
                        },
                    ],
                    default: Some(vec![
                        println_call(Expr::Literal(Literal::String("not found".to_string()), Span::unknown())),
                    ]),
                    span: Span::unknown(),
                }),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"found".to_string()));
    }

    // ---- Array indexing via ArrayList ----

    #[test]
    fn test_array_indexing() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "arr".to_string(),
                    typ: Some(Type::simple("ArrayList")),
                    init: Some(Expr::New(Type::simple("ArrayList"), vec![], Span::unknown())),
                    mutable: true,
                    span: Span::unknown(),
                }),
                Stmt::Expr(Expr::Call(
                    Box::new(Expr::MemberAccess(
                        Box::new(Expr::Identifier("arr".to_string(), Span::unknown())),
                        "add".to_string(),
                        Span::unknown(),
                    )),
                    vec![Expr::Literal(Literal::Int(10), Span::unknown())],
                    Span::unknown(),
                )),
                Stmt::Expr(Expr::Call(
                    Box::new(Expr::MemberAccess(
                        Box::new(Expr::Identifier("arr".to_string(), Span::unknown())),
                        "add".to_string(),
                        Span::unknown(),
                    )),
                    vec![Expr::Literal(Literal::Int(20), Span::unknown())],
                    Span::unknown(),
                )),
                Stmt::Expr(Expr::Call(
                    Box::new(Expr::MemberAccess(
                        Box::new(Expr::Identifier("arr".to_string(), Span::unknown())),
                        "add".to_string(),
                        Span::unknown(),
                    )),
                    vec![Expr::Literal(Literal::Int(30), Span::unknown())],
                    Span::unknown(),
                )),
                println_call(Expr::Call(
                    Box::new(Expr::MemberAccess(
                        Box::new(Expr::Identifier("arr".to_string(), Span::unknown())),
                        "get".to_string(),
                        Span::unknown(),
                    )),
                    vec![Expr::Literal(Literal::Int(1), Span::unknown())],
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"20".to_string()));
    }

    // ---- String length member ----

    #[test]
    fn test_string_length() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "s".to_string(),
                    typ: Some(Type::simple("string")),
                    init: Some(Expr::Literal(Literal::String("hello".to_string()), Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                println_call(Expr::MemberAccess(
                    Box::new(Expr::Identifier("s".to_string(), Span::unknown())),
                    "length".to_string(),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"5".to_string()));
    }

    // ---- Cast int to float ----

    #[test]
    fn test_cast_int_to_float() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Cast(
                    Box::new(Expr::Literal(Literal::Int(7), Span::unknown())),
                    Type::simple("float"),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"7".to_string()));
    }

    // ---- Cast double to int ----

    #[test]
    fn test_cast_double_to_int() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Cast(
                    Box::new(Expr::Literal(Literal::Float(3.9), Span::unknown())),
                    Type::simple("int"),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"3".to_string()));
    }

    // ---- Error propagation with ResultErr propagates ----

    #[test]
    fn test_error_propagation_returns_err() {
        // Test that error propagation on a ResultErr value returns an error
        // We construct a ResultErr value via the interpreter's built-in handling
        // Since Expr::ResultErr doesn't exist in the AST, we test error propagation
        // by creating a function that returns a ResultErr through the interpreter
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "val".to_string(),
                    typ: Some(Type::generic("Result", vec![Type::simple("long"), Type::simple("string")])),
                    init: Some(Expr::Call(
                        Box::new(Expr::Identifier("makeErr".to_string(), Span::unknown())),
                        vec![],
                        Span::unknown(),
                    )),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::VarDecl(VarDecl {
                    name: "unwrapped".to_string(),
                    typ: Some(Type::simple("long")),
                    init: Some(Expr::ErrorPropagation(Box::new(
                        Expr::Identifier("val".to_string(), Span::unknown()),
                    ), Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                println_call(Expr::Literal(Literal::String("should not reach".to_string()), Span::unknown())),
            ])),
            Declaration::Function(FnDecl {
                access: Access::Public,
                name: "makeErr".to_string(),
                type_params: vec![],
                params: vec![],
                return_type: Some(Type::generic("Result", vec![Type::simple("long"), Type::simple("string")])),
                body: vec![
                    Stmt::Return(Some(Expr::Call(
                        Box::new(Expr::StaticCall {
                            class_name: "Result".to_string(),
                            method: "err".to_string(),
                            args: vec![Expr::Literal(Literal::String("bad".to_string()), Span::unknown())],
                            span: Span::unknown(),
                        }),
                        vec![],
                        Span::unknown(),
                    ))),
                ],
                sugar: false,
                where_clause: vec![],
                span: Span::unknown(),
            }),
        ]);
        let interp = Interpreter::new();
        let result = interp.run(&program);
        // The error propagation should cause the function to return an error
        assert!(result.is_err() || interp.output.borrow().is_empty());
    }

    // ---- parseInt success ----

    #[test]
    fn test_parse_int_success() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::StaticCall {
                    class_name: "Integer".to_string(),
                    method: "parseInt".to_string(),
                    args: vec![Expr::Literal(Literal::String("42".to_string()), Span::unknown())],
                    span: Span::unknown(),
                }),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"42".to_string()));
    }

    // ---- parseInt failure ----

    #[test]
    fn test_parse_int_failure() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::StaticCall {
                    class_name: "Integer".to_string(),
                    method: "parseInt".to_string(),
                    args: vec![Expr::Literal(Literal::String("not_a_number".to_string()), Span::unknown())],
                    span: Span::unknown(),
                }),
            ])),
        ]);
        let interp = Interpreter::new();
        let result = interp.run(&program);
        assert!(result.is_err());
    }

    // ---- Double toString ----

    #[test]
    // 3.14 is the to_string fixture input, not an approximation of PI.
    #[allow(clippy::approx_constant)]
    fn test_double_to_string() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::StaticCall {
                    class_name: "Double".to_string(),
                    method: "toString".to_string(),
                    args: vec![Expr::Literal(Literal::Float(3.14), Span::unknown())],
                    span: Span::unknown(),
                }),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"3.14".to_string()));
    }

    // ---- Closure tests ----

    #[test]
    fn test_interpret_closure() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "f".to_string(),
                    typ: None,
                    init: Some(Expr::Closure {
                        params: vec![("x".to_string(), Type::simple("long"))],
                        return_type: Type::simple("long"),
                        body: vec![],
                        expr: Some(Box::new(Expr::Binary(
                            Box::new(Expr::Identifier("x".to_string(), Span::unknown())),
                            Operator::Add,
                            Box::new(Expr::Literal(Literal::Int(1), Span::unknown())),
                            Span::unknown(),
                        ))),
                        captured_vars: vec![],
                        span: Span::unknown(),
                    }),
                    mutable: false,
                    span: Span::unknown(),
                }),
                println_call(Expr::Call(
                    Box::new(Expr::Identifier("f".to_string(), Span::unknown())),
                    vec![Expr::Literal(Literal::Int(41), Span::unknown())],
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"42".to_string()));
    }

    // ---- Tuple tests ----

    #[test]
    fn test_interpret_tuple() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "t".to_string(),
                    typ: None,
                    init: Some(Expr::Tuple(vec![
                        Expr::Literal(Literal::Int(10), Span::unknown()),
                        Expr::Literal(Literal::Int(20), Span::unknown()),
                    ], Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                println_call(Expr::Identifier("t".to_string(), Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"(10, 20)".to_string()));
    }

    #[test]
    fn test_interpret_tuple_destructure() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::TupleDestructure {
                    names: vec!["a".to_string(), "b".to_string()],
                    expr: Expr::Tuple(vec![
                        Expr::Literal(Literal::Int(10), Span::unknown()),
                        Expr::Literal(Literal::Int(20), Span::unknown()),
                    ], Span::unknown()),
                    mutable: false,
                    span: Span::unknown(),
                },
                println_call(Expr::Binary(
                    Box::new(Expr::Identifier("a".to_string(), Span::unknown())),
                    Operator::Add,
                    Box::new(Expr::Identifier("b".to_string(), Span::unknown())),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"30".to_string()));
    }
