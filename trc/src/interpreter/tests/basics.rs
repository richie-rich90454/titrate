use super::{make_fn_decl, make_param, make_program, println_call};
use crate::interpreter::Interpreter;
use crate::ast::*;

    #[test]
    fn test_var_decl_and_read() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::simple("int")),
                    init: Some(Expr::Literal(Literal::Int(42), Span::unknown())),
                    mutable: true,
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
    // 3.14159 is the const-decl fixture input, not an approximation of PI.
    #[allow(clippy::approx_constant)]
    fn test_const_decl() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::ConstDecl(VarDecl {
                    name: "PI".to_string(),
                    typ: Some(Type::simple("double")),
                    init: Some(Expr::Literal(Literal::Float(3.14159), Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                println_call(Expr::Identifier("PI".to_string(), Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"3.14159".to_string()));
    }

    #[test]
    fn test_var_assignment() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::simple("int")),
                    init: Some(Expr::Literal(Literal::Int(1), Span::unknown())),
                    mutable: true,
                    span: Span::unknown(),
                }),
                Stmt::Expr(Expr::Assign(
                    Box::new(Expr::Identifier("x".to_string(), Span::unknown())),
                    Box::new(Expr::Literal(Literal::Int(2), Span::unknown())),
                    Span::unknown(),
                )),
                println_call(Expr::Identifier("x".to_string(), Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"2".to_string()));
    }

    // ---- Arithmetic operators ----

    #[test]
    fn test_arithmetic() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "result".to_string(),
                    typ: Some(Type::simple("long")),
                    init: Some(Expr::Binary(
                        Box::new(Expr::Binary(
                            Box::new(Expr::Literal(Literal::Int(2), Span::unknown())),
                            Operator::Add,
                            Box::new(Expr::Literal(Literal::Int(3), Span::unknown())),
                            Span::unknown(),
                        )),
                        Operator::Mul,
                        Box::new(Expr::Literal(Literal::Int(4), Span::unknown())),
                        Span::unknown(),
                    )),
                    mutable: false,
                    span: Span::unknown(),
                }),
                println_call(Expr::Identifier("result".to_string(), Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"20".to_string()));
    }

    #[test]
    fn test_division() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Binary(
                    Box::new(Expr::Literal(Literal::Int(10), Span::unknown())),
                    Operator::Div,
                    Box::new(Expr::Literal(Literal::Int(3), Span::unknown())),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"3".to_string()));
    }

    #[test]
    fn test_modulo() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Binary(
                    Box::new(Expr::Literal(Literal::Int(10), Span::unknown())),
                    Operator::Mod,
                    Box::new(Expr::Literal(Literal::Int(3), Span::unknown())),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"1".to_string()));
    }

    // ---- Comparison operators ----

    #[test]
    fn test_comparison() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Binary(
                    Box::new(Expr::Literal(Literal::Int(5), Span::unknown())),
                    Operator::Gt,
                    Box::new(Expr::Literal(Literal::Int(3), Span::unknown())),
                    Span::unknown(),
                )),
                println_call(Expr::Binary(
                    Box::new(Expr::Literal(Literal::Int(3), Span::unknown())),
                    Operator::Eq,
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

    // ---- String concatenation ----

    #[test]
    fn test_string_concat() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Binary(
                    Box::new(Expr::Literal(Literal::String("Hello".to_string()), Span::unknown())),
                    Operator::Add,
                    Box::new(Expr::Literal(Literal::String(" World".to_string()), Span::unknown())),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"Hello World".to_string()));
    }

    #[test]
    fn test_string_number_concat() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Binary(
                    Box::new(Expr::Literal(Literal::String("Value: ".to_string()), Span::unknown())),
                    Operator::Add,
                    Box::new(Expr::Literal(Literal::Int(42), Span::unknown())),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"Value: 42".to_string()));
    }

    // ---- If/else ----

    #[test]
    fn test_if_else() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::simple("long")),
                    init: Some(Expr::Literal(Literal::Int(10), Span::unknown())),
                    mutable: true,
                    span: Span::unknown(),
                }),
                Stmt::If(IfStmt {
                    condition: Expr::Binary(
                        Box::new(Expr::Identifier("x".to_string(), Span::unknown())),
                        Operator::Gt,
                        Box::new(Expr::Literal(Literal::Int(5), Span::unknown())),
                        Span::unknown(),
                    ),
                    then_branch: vec![
                        println_call(Expr::Literal(Literal::String("big".to_string()), Span::unknown())),
                    ],
                    else_branch: Some(vec![
                        println_call(Expr::Literal(Literal::String("small".to_string()), Span::unknown())),
                    ]),
                    span: Span::unknown(),
                }),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"big".to_string()));
    }

    #[test]
    fn test_if_false() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::If(IfStmt {
                    condition: Expr::Literal(Literal::Bool(false), Span::unknown()),
                    then_branch: vec![
                        println_call(Expr::Literal(Literal::String("yes".to_string()), Span::unknown())),
                    ],
                    else_branch: Some(vec![
                        println_call(Expr::Literal(Literal::String("no".to_string()), Span::unknown())),
                    ]),
                    span: Span::unknown(),
                }),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"no".to_string()));
    }

    // ---- While loops ----

    #[test]
    fn test_while_loop() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "i".to_string(),
                    typ: Some(Type::simple("long")),
                    init: Some(Expr::Literal(Literal::Int(0), Span::unknown())),
                    mutable: true,
                    span: Span::unknown(),
                }),
                Stmt::While(WhileStmt {
                    condition: Expr::Binary(
                        Box::new(Expr::Identifier("i".to_string(), Span::unknown())),
                        Operator::Lt,
                        Box::new(Expr::Literal(Literal::Int(3), Span::unknown())),
                        Span::unknown(),
                    ),
                    body: vec![
                        Stmt::Expr(Expr::Assign(
                            Box::new(Expr::Identifier("i".to_string(), Span::unknown())),
                            Box::new(Expr::Binary(
                                Box::new(Expr::Identifier("i".to_string(), Span::unknown())),
                                Operator::Add,
                                Box::new(Expr::Literal(Literal::Int(1), Span::unknown())),
                                Span::unknown(),
                            )),
                            Span::unknown(),
                        )),
                    ],
                    span: Span::unknown(),
                }),
                println_call(Expr::Identifier("i".to_string(), Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"3".to_string()));
    }

    #[test]
    fn test_while_break() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "i".to_string(),
                    typ: Some(Type::simple("long")),
                    init: Some(Expr::Literal(Literal::Int(0), Span::unknown())),
                    mutable: true,
                    span: Span::unknown(),
                }),
                Stmt::While(WhileStmt {
                    condition: Expr::Literal(Literal::Bool(true), Span::unknown()),
                    body: vec![
                        Stmt::Expr(Expr::Assign(
                            Box::new(Expr::Identifier("i".to_string(), Span::unknown())),
                            Box::new(Expr::Binary(
                                Box::new(Expr::Identifier("i".to_string(), Span::unknown())),
                                Operator::Add,
                                Box::new(Expr::Literal(Literal::Int(1), Span::unknown())),
                                Span::unknown(),
                            )),
                            Span::unknown(),
                        )),
                        Stmt::If(IfStmt {
                            condition: Expr::Binary(
                                Box::new(Expr::Identifier("i".to_string(), Span::unknown())),
                                Operator::Eq,
                                Box::new(Expr::Literal(Literal::Int(3), Span::unknown())),
                                Span::unknown(),
                            ),
                            then_branch: vec![Stmt::Break],
                            else_branch: None,
                            span: Span::unknown(),
                        }),
                    ],
                    span: Span::unknown(),
                }),
                println_call(Expr::Identifier("i".to_string(), Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"3".to_string()));
    }

    #[test]
    fn test_while_continue() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "i".to_string(),
                    typ: Some(Type::simple("long")),
                    init: Some(Expr::Literal(Literal::Int(0), Span::unknown())),
                    mutable: true,
                    span: Span::unknown(),
                }),
                Stmt::VarDecl(VarDecl {
                    name: "sum".to_string(),
                    typ: Some(Type::simple("long")),
                    init: Some(Expr::Literal(Literal::Int(0), Span::unknown())),
                    mutable: true,
                    span: Span::unknown(),
                }),
                Stmt::While(WhileStmt {
                    condition: Expr::Binary(
                        Box::new(Expr::Identifier("i".to_string(), Span::unknown())),
                        Operator::Lt,
                        Box::new(Expr::Literal(Literal::Int(5), Span::unknown())),
                        Span::unknown(),
                    ),
                    body: vec![
                        Stmt::Expr(Expr::Assign(
                            Box::new(Expr::Identifier("i".to_string(), Span::unknown())),
                            Box::new(Expr::Binary(
                                Box::new(Expr::Identifier("i".to_string(), Span::unknown())),
                                Operator::Add,
                                Box::new(Expr::Literal(Literal::Int(1), Span::unknown())),
                                Span::unknown(),
                            )),
                            Span::unknown(),
                        )),
                        Stmt::If(IfStmt {
                            condition: Expr::Binary(
                                Box::new(Expr::Identifier("i".to_string(), Span::unknown())),
                                Operator::Mod,
                                Box::new(Expr::Literal(Literal::Int(2), Span::unknown())),
                                Span::unknown(),
                            ),
                            then_branch: vec![Stmt::Continue],
                            else_branch: None,
                            span: Span::unknown(),
                        }),
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
        // sum of even numbers 2+4 = 6
        assert_eq!(output.last(), Some(&"6".to_string()));
    }

    // ---- Function definitions and calls ----

    #[test]
    fn test_function_call() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("add", vec![
                make_param("a", "long"),
                make_param("b", "long"),
            ], vec![
                Stmt::Return(Some(Expr::Binary(
                    Box::new(Expr::Identifier("a".to_string(), Span::unknown())),
                    Operator::Add,
                    Box::new(Expr::Identifier("b".to_string(), Span::unknown())),
                    Span::unknown(),
                ))),
            ])),
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Call(
                    Box::new(Expr::Identifier("add".to_string(), Span::unknown())),
                    vec![
                        Expr::Literal(Literal::Int(3), Span::unknown()),
                        Expr::Literal(Literal::Int(4), Span::unknown()),
                    ],
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"7".to_string()));
    }

    #[test]
    fn test_recursive_function() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("fib", vec![
                make_param("n", "long"),
            ], vec![
                Stmt::If(IfStmt {
                    condition: Expr::Binary(
                        Box::new(Expr::Identifier("n".to_string(), Span::unknown())),
                        Operator::Le,
                        Box::new(Expr::Literal(Literal::Int(1), Span::unknown())),
                        Span::unknown(),
                    ),
                    then_branch: vec![Stmt::Return(Some(Expr::Identifier("n".to_string(), Span::unknown())))],
                    else_branch: None,
                    span: Span::unknown(),
                }),
                Stmt::Return(Some(Expr::Binary(
                    Box::new(Expr::Call(
                        Box::new(Expr::Identifier("fib".to_string(), Span::unknown())),
                        vec![Expr::Binary(
                            Box::new(Expr::Identifier("n".to_string(), Span::unknown())),
                            Operator::Sub,
                            Box::new(Expr::Literal(Literal::Int(1), Span::unknown())),
                            Span::unknown(),
                        )],
                        Span::unknown(),
                    )),
                    Operator::Add,
                    Box::new(Expr::Call(
                        Box::new(Expr::Identifier("fib".to_string(), Span::unknown())),
                        vec![Expr::Binary(
                            Box::new(Expr::Identifier("n".to_string(), Span::unknown())),
                            Operator::Sub,
                            Box::new(Expr::Literal(Literal::Int(2), Span::unknown())),
                            Span::unknown(),
                        )],
                        Span::unknown(),
                    )),
                    Span::unknown(),
                ))),
            ])),
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Call(
                    Box::new(Expr::Identifier("fib".to_string(), Span::unknown())),
                    vec![Expr::Literal(Literal::Int(10), Span::unknown())],
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"55".to_string()));
    }

    // ---- Class instantiation and method calls ----

    #[test]
    fn test_class_instantiation() {
        let program = make_program(vec![
            Declaration::Class(ClassDecl {
                name: "Point".to_string(),
                type_params: vec![],
                parent: None,
                ifaces: vec![],
                members: vec![
                    ClassMember::Field(FieldDecl {
                        access: Access::Public,
                        name: "x".to_string(),
                        typ: Type::simple("long"),
                        init: Some(Expr::Literal(Literal::Int(0), Span::unknown())),
                        span: Span::unknown(),
                    }),
                    ClassMember::Field(FieldDecl {
                        access: Access::Public,
                        name: "y".to_string(),
                        typ: Type::simple("long"),
                        init: Some(Expr::Literal(Literal::Int(0), Span::unknown())),
                        span: Span::unknown(),
                    }),
                ],
                span: Span::unknown(),
            }),
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "p".to_string(),
                    typ: Some(Type::simple("Point")),
                    init: Some(Expr::New(Type::simple("Point"), vec![], Span::unknown())),
                    mutable: true,
                    span: Span::unknown(),
                }),
                println_call(Expr::MemberAccess(
                    Box::new(Expr::Identifier("p".to_string(), Span::unknown())),
                    "x".to_string(),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"0".to_string()));
    }

    #[test]
    fn test_class_with_constructor() {
        let program = make_program(vec![
            Declaration::Class(ClassDecl {
                name: "Point".to_string(),
                type_params: vec![],
                parent: None,
                ifaces: vec![],
                members: vec![
                    ClassMember::Field(FieldDecl {
                        access: Access::Public,
                        name: "x".to_string(),
                        typ: Type::simple("long"),
                        init: None,
                        span: Span::unknown(),
                    }),
                    ClassMember::Field(FieldDecl {
                        access: Access::Public,
                        name: "y".to_string(),
                        typ: Type::simple("long"),
                        init: None,
                        span: Span::unknown(),
                    }),
                    ClassMember::Constructor(MethodDecl {
                        access: Access::Public,
                        name: "new".to_string(),
                        type_params: vec![],
                        params: vec![
                            make_param("x", "long"),
                            make_param("y", "long"),
                        ],
                        return_type: None,
                        body: vec![
                            Stmt::Expr(Expr::Assign(
                                Box::new(Expr::MemberAccess(
                                    Box::new(Expr::This(Span::unknown())),
                                    "x".to_string(),
                                    Span::unknown(),
                                )),
                                Box::new(Expr::Identifier("x".to_string(), Span::unknown())),
                                Span::unknown(),
                            )),
                            Stmt::Expr(Expr::Assign(
                                Box::new(Expr::MemberAccess(
                                    Box::new(Expr::This(Span::unknown())),
                                    "y".to_string(),
                                    Span::unknown(),
                                )),
                                Box::new(Expr::Identifier("y".to_string(), Span::unknown())),
                                Span::unknown(),
                            )),
                        ],
                        where_clause: vec![],
                        span: Span::unknown(),
                    }),
                ],
                span: Span::unknown(),
            }),
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "p".to_string(),
                    typ: Some(Type::simple("Point")),
                    init: Some(Expr::New(Type::simple("Point"), vec![
                        Expr::Literal(Literal::Int(3), Span::unknown()),
                        Expr::Literal(Literal::Int(4), Span::unknown()),
                    ], Span::unknown())),
                    mutable: true,
                    span: Span::unknown(),
                }),
                println_call(Expr::MemberAccess(
                    Box::new(Expr::Identifier("p".to_string(), Span::unknown())),
                    "x".to_string(),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"3".to_string()));
    }

    #[test]
    fn test_class_method() {
        let program = make_program(vec![
            Declaration::Class(ClassDecl {
                name: "Counter".to_string(),
                type_params: vec![],
                parent: None,
                ifaces: vec![],
                members: vec![
                    ClassMember::Field(FieldDecl {
                        access: Access::Public,
                        name: "count".to_string(),
                        typ: Type::simple("long"),
                        init: Some(Expr::Literal(Literal::Int(0), Span::unknown())),
                        span: Span::unknown(),
                    }),
                    ClassMember::Method(MethodDecl {
                        access: Access::Public,
                        name: "increment".to_string(),
                        type_params: vec![],
                        params: vec![],
                        return_type: None,
                        body: vec![
                            Stmt::Expr(Expr::Assign(
                                Box::new(Expr::MemberAccess(
                                    Box::new(Expr::This(Span::unknown())),
                                    "count".to_string(),
                                    Span::unknown(),
                                )),
                                Box::new(Expr::Binary(
                                    Box::new(Expr::MemberAccess(
                                        Box::new(Expr::This(Span::unknown())),
                                        "count".to_string(),
                                        Span::unknown(),
                                    )),
                                    Operator::Add,
                                    Box::new(Expr::Literal(Literal::Int(1), Span::unknown())),
                                    Span::unknown(),
                                )),
                                Span::unknown(),
                            )),
                        ],
                        where_clause: vec![],
                        span: Span::unknown(),
                    }),
                    ClassMember::Method(MethodDecl {
                        access: Access::Public,
                        name: "getCount".to_string(),
                        type_params: vec![],
                        params: vec![],
                        return_type: Some(Type::simple("long")),
                        body: vec![
                            Stmt::Return(Some(Expr::MemberAccess(
                                Box::new(Expr::This(Span::unknown())),
                                "count".to_string(),
                                Span::unknown(),
                            ))),
                        ],
                        where_clause: vec![],
                        span: Span::unknown(),
                    }),
                ],
                span: Span::unknown(),
            }),
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "c".to_string(),
                    typ: Some(Type::simple("Counter")),
                    init: Some(Expr::New(Type::simple("Counter"), vec![], Span::unknown())),
                    mutable: true,
                    span: Span::unknown(),
                }),
                Stmt::Expr(Expr::Call(
                    Box::new(Expr::MemberAccess(
                        Box::new(Expr::Identifier("c".to_string(), Span::unknown())),
                        "increment".to_string(),
                        Span::unknown(),
                    )),
                    vec![],
                    Span::unknown(),
                )),
                Stmt::Expr(Expr::Call(
                    Box::new(Expr::MemberAccess(
                        Box::new(Expr::Identifier("c".to_string(), Span::unknown())),
                        "increment".to_string(),
                        Span::unknown(),
                    )),
                    vec![],
                    Span::unknown(),
                )),
                println_call(Expr::Call(
                    Box::new(Expr::MemberAccess(
                        Box::new(Expr::Identifier("c".to_string(), Span::unknown())),
                        "getCount".to_string(),
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
        assert_eq!(output.last(), Some(&"2".to_string()));
    }

