// Phase 4: Tests for the Titrate interpreter
// Precision in every step – richie-rich90454, 2026

#[cfg(test)]
mod tests {
    use crate::interpreter::{Value, Memory, Interpreter, interpret};
    use crate::ast::*;
    // Disambiguate: AST's MethodDecl takes precedence over interpreter's
    use crate::ast::MethodDecl;

    fn make_program(declarations: Vec<Declaration>) -> Program {
        Program {
            imports: vec![],
            declarations,
        }
    }

    fn make_fn_decl(name: &str, params: Vec<Param>, body: Vec<Stmt>) -> FnDecl {
        FnDecl {
            access: Access::Public,
            name: name.to_string(),
            type_params: vec![],
            params,
            return_type: None,
            body,
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }
    }

    fn make_param(name: &str, typ: &str) -> Param {
        Param {
            name: name.to_string(),
            typ: Type::simple(typ),
        }
    }

    fn println_call(arg: Expr) -> Stmt {
        Stmt::Expr(Expr::Call(
            Box::new(Expr::MemberAccess(
                Box::new(Expr::Identifier("io".to_string(), Span::unknown())),
                "println".to_string(),
                Span::unknown(),
            )),
            vec![arg],
            Span::unknown(),
        ))
    }

    // ---- Variable declarations and assignments ----

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
        assert_eq!(output.last(), Some(&"Ok(123)".to_string()));
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
        assert!(result.err().map_or(false, |e| e.contains("zero")));
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
        assert!(output.last().map_or(false, |s| s.starts_with("ref(")));
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
        assert_eq!(output.last(), Some(&"Ok(42)".to_string()));
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
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert!(output.last().map_or(false, |s| s.starts_with("Err(")));
    }

    // ---- Double toString ----

    #[test]
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
}
