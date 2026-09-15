use super::{make_fn_decl, make_param, make_program, println_call};
use crate::interpreter::{Interpreter, Value};
use crate::ast::*;

    // ---- Enum construction and pattern matching ----

    #[test]
    fn test_enum_construction() {
        let program = make_program(vec![
            Declaration::Enum(EnumDecl {
                name: "Color".to_string(),
                type_params: vec![],
                variants: vec![
                    Variant { name: "Red".to_string(), fields: vec![] },
                    Variant { name: "Green".to_string(), fields: vec![] },
                    Variant { name: "Blue".to_string(), fields: vec![] },
                ],
                span: Span::unknown(),
            }),
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "c".to_string(),
                    typ: Some(Type::simple("Color")),
                    init: Some(Expr::Call(
                        Box::new(Expr::Identifier("Red".to_string(), Span::unknown())),
                        vec![],
                        Span::unknown(),
                    )),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::Switch(SwitchStmt {
                    expr: Expr::Identifier("c".to_string(), Span::unknown()),
                    cases: vec![
                        Case {
                            pattern: Pattern::Constructor {
                                name: "Red".to_string(),
                                bindings: vec![],
                            },
                            body: vec![
                                println_call(Expr::Literal(Literal::String("red".to_string()), Span::unknown())),
                            ],
                        },
                        Case {
                            pattern: Pattern::Constructor {
                                name: "Green".to_string(),
                                bindings: vec![],
                            },
                            body: vec![
                                println_call(Expr::Literal(Literal::String("green".to_string()), Span::unknown())),
                            ],
                        },
                    ],
                    default: Some(vec![
                        println_call(Expr::Literal(Literal::String("other".to_string()), Span::unknown())),
                    ]),
                    span: Span::unknown(),
                }),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"red".to_string()));
    }

    #[test]
    fn test_enum_with_fields() {
        let program = make_program(vec![
            Declaration::Enum(EnumDecl {
                name: "Option".to_string(),
                type_params: vec![],
                variants: vec![
                    Variant {
                        name: "Some".to_string(),
                        fields: vec![make_param("value", "long")],
                    },
                    Variant {
                        name: "None".to_string(),
                        fields: vec![],
                    },
                ],
                span: Span::unknown(),
            }),
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "opt".to_string(),
                    typ: Some(Type::simple("Option")),
                    init: Some(Expr::Call(
                        Box::new(Expr::Identifier("Some".to_string(), Span::unknown())),
                        vec![Expr::Literal(Literal::Int(42), Span::unknown())],
                        Span::unknown(),
                    )),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::Switch(SwitchStmt {
                    expr: Expr::Identifier("opt".to_string(), Span::unknown()),
                    cases: vec![
                        Case {
                            pattern: Pattern::Constructor {
                                name: "Some".to_string(),
                                bindings: vec!["v".to_string()],
                            },
                            body: vec![
                                println_call(Expr::Identifier("v".to_string(), Span::unknown())),
                            ],
                        },
                    ],
                    default: Some(vec![
                        println_call(Expr::Literal(Literal::String("none".to_string()), Span::unknown())),
                    ]),
                    span: Span::unknown(),
                }),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"42".to_string()));
    }

    // ---- Result type and error propagation ----

    #[test]
    fn test_result_ok_display() {
        let val = Value::ResultOk(Box::new(Value::Long(42)));
        assert_eq!(val.display_string(), "Ok(42)");
    }

    #[test]
    fn test_result_err_display() {
        let val = Value::ResultErr(Box::new(Value::String("error".to_string())));
        assert_eq!(val.display_string(), "Err(error)");
    }

    #[test]
    fn test_error_propagation_ok() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "r".to_string(),
                    typ: Some(Type::generic("Result", vec![Type::simple("long"), Type::simple("string")])),
                    init: Some(Expr::Literal(Literal::Int(42), Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::VarDecl(VarDecl {
                    name: "val".to_string(),
                    typ: Some(Type::simple("long")),
                    init: Some(Expr::ErrorPropagation(Box::new(
                        Expr::Identifier("r".to_string(), Span::unknown()),
                    ), Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                println_call(Expr::Identifier("val".to_string(), Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"42".to_string()));
    }

    #[test]
    fn test_error_propagation_err() {
        let val = Value::ResultErr(Box::new(Value::String("error".to_string())));
        let result = match val {
            Value::ResultErr(e) => Value::ResultErr(e),
            Value::ResultOk(v) => *v,
            other => other,
        };
        match result {
            Value::ResultErr(e) => assert_eq!(*e, Value::String("error".to_string())),
            _ => panic!("Expected ResultErr"),
        }
    }

    // ---- ArrayList operations ----

    #[test]
    fn test_arraylist_size() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "list".to_string(),
                    typ: Some(Type::simple("ArrayList")),
                    init: Some(Expr::New(Type::simple("ArrayList"), vec![], Span::unknown())),
                    mutable: true,
                    span: Span::unknown(),
                }),
                println_call(Expr::Call(
                    Box::new(Expr::MemberAccess(
                        Box::new(Expr::Identifier("list".to_string(), Span::unknown())),
                        "size".to_string(),
                        Span::unknown(),
                    )),
                    vec![],
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"0".to_string()));
    }

    // ---- HashMap operations ----

    #[test]
    fn test_hashmap_get_missing() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "map".to_string(),
                    typ: Some(Type::simple("HashMap")),
                    init: Some(Expr::New(Type::simple("HashMap"), vec![], Span::unknown())),
                    mutable: true,
                    span: Span::unknown(),
                }),
                println_call(Expr::Call(
                    Box::new(Expr::MemberAccess(
                        Box::new(Expr::Identifier("map".to_string(), Span::unknown())),
                        "get".to_string(),
                        Span::unknown(),
                    )),
                    vec![Expr::Literal(Literal::String("key".to_string()), Span::unknown())],
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"null".to_string()));
    }

    // ---- Type casting with as ----

    #[test]
    fn test_cast_int_to_long() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::simple("long")),
                    init: Some(Expr::Cast(
                        Box::new(Expr::Literal(Literal::Int(42), Span::unknown())),
                        Type::simple("long"),
                        Span::unknown(),
                    )),
                    mutable: false,
                    span: Span::unknown(),
                }),
                println_call(Expr::Identifier("x".to_string(), Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"42".to_string()));
    }

    #[test]
    fn test_cast_int_to_double() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Cast(
                    Box::new(Expr::Literal(Literal::Int(42), Span::unknown())),
                    Type::simple("double"),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"42".to_string()));
    }

    #[test]
    fn test_cast_long_to_byte() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Cast(
                    Box::new(Expr::Literal(Literal::Int(300), Span::unknown())),
                    Type::simple("byte"),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        // 300 as i8 = 44 (wrapping)
        assert_eq!(output.last(), Some(&"44".to_string()));
    }

    #[test]
    fn test_cast_to_string() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Cast(
                    Box::new(Expr::Literal(Literal::Int(42), Span::unknown())),
                    Type::simple("string"),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"42".to_string()));
    }

    // ---- Unary operators ----

    #[test]
    fn test_unary_neg() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Unary(UnOp::Neg, Box::new(Expr::Literal(Literal::Int(5), Span::unknown())), Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"-5".to_string()));
    }

    #[test]
    fn test_unary_not() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Unary(UnOp::Not, Box::new(Expr::Literal(Literal::Bool(true), Span::unknown())), Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"false".to_string()));
    }

    #[test]
    fn test_bitwise_not() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Unary(UnOp::BitNot, Box::new(Expr::Literal(Literal::Int(0), Span::unknown())), Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"-1".to_string()));
    }

    // ---- Bitwise operators ----

    #[test]
    fn test_bitwise_and() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Binary(
                    Box::new(Expr::Literal(Literal::Int(0b1100), Span::unknown())),
                    Operator::BitAnd,
                    Box::new(Expr::Literal(Literal::Int(0b1010), Span::unknown())),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"8".to_string()));
    }

    #[test]
    fn test_bitwise_or() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Binary(
                    Box::new(Expr::Literal(Literal::Int(0b1100), Span::unknown())),
                    Operator::BitOr,
                    Box::new(Expr::Literal(Literal::Int(0b1010), Span::unknown())),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"14".to_string()));
    }

    #[test]
    fn test_shift_left() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Binary(
                    Box::new(Expr::Literal(Literal::Int(1), Span::unknown())),
                    Operator::BitShl,
                    Box::new(Expr::Literal(Literal::Int(4), Span::unknown())),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"16".to_string()));
    }

    #[test]
    fn test_shift_right() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Binary(
                    Box::new(Expr::Literal(Literal::Int(16), Span::unknown())),
                    Operator::BitShr,
                    Box::new(Expr::Literal(Literal::Int(2), Span::unknown())),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"4".to_string()));
    }

    // ---- Logical operators ----

    #[test]
    fn test_logical_and() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Binary(
                    Box::new(Expr::Literal(Literal::Bool(true), Span::unknown())),
                    Operator::And,
                    Box::new(Expr::Literal(Literal::Bool(false), Span::unknown())),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"false".to_string()));
    }

    #[test]
    fn test_logical_or() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Binary(
                    Box::new(Expr::Literal(Literal::Bool(true), Span::unknown())),
                    Operator::Or,
                    Box::new(Expr::Literal(Literal::Bool(false), Span::unknown())),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"true".to_string()));
    }

    // ---- For loop ----

    #[test]
    fn test_for_loop() {
        // Test for-loop with an ArrayList as iterable
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "list".to_string(),
                    typ: Some(Type::simple("ArrayList")),
                    init: Some(Expr::New(Type::simple("ArrayList"), vec![], Span::unknown())),
                    mutable: true,
                    span: Span::unknown(),
                }),
                Stmt::Expr(Expr::Call(
                    Box::new(Expr::MemberAccess(
                        Box::new(Expr::Identifier("list".to_string(), Span::unknown())),
                        "add".to_string(),
                        Span::unknown(),
                    )),
                    vec![Expr::Literal(Literal::Int(1), Span::unknown())],
                    Span::unknown(),
                )),
                Stmt::Expr(Expr::Call(
                    Box::new(Expr::MemberAccess(
                        Box::new(Expr::Identifier("list".to_string(), Span::unknown())),
                        "add".to_string(),
                        Span::unknown(),
                    )),
                    vec![Expr::Literal(Literal::Int(2), Span::unknown())],
                    Span::unknown(),
                )),
                Stmt::Expr(Expr::Call(
                    Box::new(Expr::MemberAccess(
                        Box::new(Expr::Identifier("list".to_string(), Span::unknown())),
                        "add".to_string(),
                        Span::unknown(),
                    )),
                    vec![Expr::Literal(Literal::Int(3), Span::unknown())],
                    Span::unknown(),
                )),
                Stmt::VarDecl(VarDecl {
                    name: "sum".to_string(),
                    typ: Some(Type::simple("long")),
                    init: Some(Expr::Literal(Literal::Int(0), Span::unknown())),
                    mutable: true,
                    span: Span::unknown(),
                }),
                Stmt::For(ForStmt {
                    var: "i".to_string(),
                    iterable: Expr::Identifier("list".to_string(), Span::unknown()),
                    body: vec![
                        Stmt::Expr(Expr::Assign(
                            Box::new(Expr::Identifier("sum".to_string(), Span::unknown())),
                            Box::new(Expr::Binary(
                                Box::new(Expr::Identifier("sum".to_string(), Span::unknown())),
                                Operator::Add,
                                Box::new(Expr::Identifier("i".to_string(), Span::unknown())),
                                Span::unknown(),
                            )),
                            Span::unknown(),
                        )),
                    ],
                    span: Span::unknown(),
                }),
                println_call(Expr::Identifier("sum".to_string(), Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"6".to_string()));
    }

    // ---- Static call ----

    #[test]
    fn test_static_call_println() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::Expr(Expr::StaticCall {
                    class_name: "io".to_string(),
                    method: "println".to_string(),
                    args: vec![Expr::Literal(Literal::String("hello".to_string()), Span::unknown())],
                    span: Span::unknown(),
                }),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"hello".to_string()));
    }

    #[test]
    fn test_static_call_to_string() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::StaticCall {
                    class_name: "Integer".to_string(),
                    method: "toString".to_string(),
                    args: vec![Expr::Literal(Literal::Int(42), Span::unknown())],
                    span: Span::unknown(),
                }),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"42".to_string()));
    }

    #[test]
    fn test_static_call_parse_int() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::StaticCall {
                    class_name: "Integer".to_string(),
                    method: "parseInt".to_string(),
                    args: vec![Expr::Literal(Literal::String("123".to_string()), Span::unknown())],
                    span: Span::unknown(),
                }),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"123".to_string()));
    }

    // ---- Value equality ----

    #[test]
    fn test_value_equality() {
        assert_eq!(Value::Int(42), Value::Int(42));
        assert_ne!(Value::Int(42), Value::Int(43));
        assert_eq!(Value::Bool(true), Value::Bool(true));
        assert_eq!(Value::String("hello".to_string()), Value::String("hello".to_string()));
        assert_eq!(Value::Null, Value::Null);
    }

