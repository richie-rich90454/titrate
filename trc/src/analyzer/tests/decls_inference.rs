use super::program_with;
use super::super::{analyze, analyze_with_mode, analyze_with_mode_and_warnings, ExhaustiveMode};
use crate::ast::*;

    // -----------------------------------------------------------------------
    // Interface test
    // -----------------------------------------------------------------------

    #[test]
    fn test_interface_declaration() {
        let prog = program_with(Declaration::Interface(InterfaceDecl {
            name: "Printable".to_string(),
            type_params: vec![],
            parents: vec![],
            methods: vec![MethodSig {
                name: "toString".to_string(),
                params: vec![],
                return_type: Some(Type::simple("string")),
                body: None,
            }],
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    // -----------------------------------------------------------------------
    // Multiple declarations interaction test
    // -----------------------------------------------------------------------

    #[test]
    fn test_function_calling_function() {
        let prog = Program {
            imports: vec![],
            declarations: vec![
                Declaration::Function(FnDecl {
                    access: Access::Public,
                    name: "helper".to_string(),
                    type_params: vec![],
                    params: vec![],
                    return_type: Some(Type::simple("int")),
                    body: vec![Stmt::Return(Some(Expr::Literal(Literal::Int(1), Span::unknown())))],
                    sugar: false,
                    where_clause: vec![],
                    span: Span::unknown(),
                }),
                Declaration::Function(FnDecl {
                    access: Access::Public,
                    name: "main".to_string(),
                    type_params: vec![],
                    params: vec![],
                    return_type: Some(Type::simple("void")),
                    body: vec![Stmt::Expr(Expr::Call(
                        Box::new(Expr::Identifier("helper".to_string(), Span::unknown())),
                        vec![],
                        Span::unknown(),
                    ))],
                    sugar: false,
                    where_clause: vec![],
                    span: Span::unknown(),
                }),
            ],
        };
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_function_wrong_arg_count() {
        let prog = Program {
            imports: vec![],
            declarations: vec![
                Declaration::Function(FnDecl {
                    access: Access::Public,
                    name: "add".to_string(),
                    type_params: vec![],
                    params: vec![
                        Param { name: "a".to_string(), typ: Type::simple("int") },
                        Param { name: "b".to_string(), typ: Type::simple("int") },
                    ],
                    return_type: Some(Type::simple("int")),
                    body: vec![Stmt::Return(Some(Expr::Binary(
                        Box::new(Expr::Identifier("a".to_string(), Span::unknown())),
                        Operator::Add,
                        Box::new(Expr::Identifier("b".to_string(), Span::unknown())),
                        Span::unknown(),
                    )))],
                    sugar: false,
                    where_clause: vec![],
                    span: Span::unknown(),
                }),
                Declaration::Function(FnDecl {
                    access: Access::Public,
                    name: "main".to_string(),
                    type_params: vec![],
                    params: vec![],
                    return_type: Some(Type::simple("void")),
                    body: vec![Stmt::Expr(Expr::Call(
                        Box::new(Expr::Identifier("add".to_string(), Span::unknown())),
                        vec![Expr::Literal(Literal::Int(1), Span::unknown())],
                        Span::unknown(),
                    ))],
                    sugar: false,
                    where_clause: vec![],
                    span: Span::unknown(),
                }),
            ],
        };
        let result = analyze(&prog);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("arguments")));
    }

    // -----------------------------------------------------------------------
    // Owned deref test
    // -----------------------------------------------------------------------

    #[test]
    fn test_owned_deref() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::generic("Owned", vec![Type::simple("int")])),
                    init: Some(Expr::New(Type::simple("int"), vec![Expr::Literal(Literal::Int(5), Span::unknown())], Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::VarDecl(VarDecl {
                    name: "y".to_string(),
                    typ: Some(Type::simple("int")),
                    init: Some(Expr::OwnedDeref(Box::new(Expr::Identifier("x".to_string(), Span::unknown())), Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
            ],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_owned_deref_on_non_owned() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::simple("int")),
                    init: Some(Expr::Literal(Literal::Int(5), Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::Expr(Expr::OwnedDeref(Box::new(Expr::Identifier("x".to_string(), Span::unknown())), Span::unknown())),
            ],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("Owned")));
    }

    // -----------------------------------------------------------------------
    // Const declaration test
    // -----------------------------------------------------------------------

    #[test]
    // 3.14159 is the const-decl fixture input, not an approximation of PI.
    #[allow(clippy::approx_constant)]
    fn test_const_decl() {
        let prog = program_with(Declaration::ConstDecl(VarDecl {
            name: "PI".to_string(),
            typ: Some(Type::simple("double")),
            init: Some(Expr::Literal(Literal::Float(3.14159), Span::unknown())),
            mutable: false,
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    // -----------------------------------------------------------------------
    // For loop test
    // -----------------------------------------------------------------------

    #[test]
    fn test_for_loop() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::For(ForStmt {
                var: "i".to_string(),
                iterable: Expr::Identifier("range".to_string(), Span::unknown()),
                body: vec![Stmt::Expr(Expr::Identifier("i".to_string(), Span::unknown()))],
                span: Span::unknown(),
            })],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        // Will error on undeclared "range" but "i" should be in scope.
        let result = analyze(&prog);
        // "range" is undeclared, so this should error.
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Type inference test
    // -----------------------------------------------------------------------

    #[test]
    fn test_type_inference_no_declared_type() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: None,
                    init: Some(Expr::Literal(Literal::Int(42), Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                // x should be inferred as int.
                Stmt::Expr(Expr::Identifier("x".to_string(), Span::unknown())),
            ],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_ok());
        // Verify type was inferred.
        let analyzed = result.unwrap();
        match &analyzed.declarations[0] {
            Declaration::Function(f) => {
                match &f.body[0] {
                    Stmt::VarDecl(v) => {
                        assert_eq!(v.typ, Some(Type::simple("int")));
                    }
                    _ => panic!("expected var decl"),
                }
            }
            _ => panic!("expected function"),
        }
    }

    // -----------------------------------------------------------------------
    // Double mutable borrow test
    // -----------------------------------------------------------------------

    #[test]
    fn test_double_mutable_borrow() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::simple("int")),
                    init: Some(Expr::Literal(Literal::Int(5), Span::unknown())),
                    mutable: true,
                    span: Span::unknown(),
                }),
                Stmt::VarDecl(VarDecl {
                    name: "y".to_string(),
                    typ: Some(Type::generic("RefMut", vec![Type::simple("int")])),
                    init: Some(Expr::RefExpr(
                        Box::new(Expr::Identifier("x".to_string(), Span::unknown())),
                        RefKind::Mutable,
                        Span::unknown(),
                    )),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::VarDecl(VarDecl {
                    name: "z".to_string(),
                    typ: Some(Type::generic("RefMut", vec![Type::simple("int")])),
                    init: Some(Expr::RefExpr(
                        Box::new(Expr::Identifier("x".to_string(), Span::unknown())),
                        RefKind::Mutable,
                        Span::unknown(),
                    )),
                    mutable: false,
                    span: Span::unknown(),
                }),
            ],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Borrow of moved variable test
    // -----------------------------------------------------------------------

    #[test]
    fn test_borrow_after_move() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::generic("Owned", vec![Type::simple("int")])),
                    init: Some(Expr::New(Type::simple("int"), vec![Expr::Literal(Literal::Int(5), Span::unknown())], Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::VarDecl(VarDecl {
                    name: "y".to_string(),
                    typ: Some(Type::generic("Owned", vec![Type::simple("int")])),
                    init: Some(Expr::Identifier("x".to_string(), Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::VarDecl(VarDecl {
                    name: "z".to_string(),
                    typ: Some(Type::generic("Ref", vec![Type::generic("Owned", vec![Type::simple("int")])])),
                    init: Some(Expr::RefExpr(
                        Box::new(Expr::Identifier("x".to_string(), Span::unknown())),
                        RefKind::Immutable,
                        Span::unknown(),
                    )),
                    mutable: false,
                    span: Span::unknown(),
                }),
            ],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Exhaustiveness checking tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_exhaustive_switch_all_variants_covered() {
        // Switch covering all enum variants 鈫?no warning, no error
        let prog = Program {
            imports: vec![],
            declarations: vec![
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
                Declaration::Function(FnDecl {
                    access: Access::Public,
                    name: "test".to_string(),
                    type_params: vec![],
                    params: vec![Param { name: "c".to_string(), typ: Type::simple("Color") }],
                    return_type: Some(Type::simple("void")),
                    body: vec![Stmt::Switch(SwitchStmt {
                        expr: Expr::Identifier("c".to_string(), Span::unknown()),
                        cases: vec![
                            Case {
                                pattern: Pattern::Constructor { name: "Red".to_string(), bindings: vec![] },
                                body: vec![],
                            },
                            Case {
                                pattern: Pattern::Constructor { name: "Green".to_string(), bindings: vec![] },
                                body: vec![],
                            },
                            Case {
                                pattern: Pattern::Constructor { name: "Blue".to_string(), bindings: vec![] },
                                body: vec![],
                            },
                        ],
                        default: None,
                        span: Span::unknown(),
                    })],
                    sugar: false,
                    where_clause: vec![],
                    span: Span::unknown(),
                }),
            ],
        };
        let result = analyze_with_mode_and_warnings(&prog, ExhaustiveMode::Warning);
        assert!(result.is_ok());
        let (_, warnings) = result.unwrap();
        assert!(warnings.is_empty(), "expected no warnings for exhaustive switch, got: {:?}", warnings);
    }

    #[test]
    fn test_non_exhaustive_switch_produces_warning() {
        // Switch missing some variants 鈫?warning in Warning mode
        let prog = Program {
            imports: vec![],
            declarations: vec![
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
                Declaration::Function(FnDecl {
                    access: Access::Public,
                    name: "test".to_string(),
                    type_params: vec![],
                    params: vec![Param { name: "c".to_string(), typ: Type::simple("Color") }],
                    return_type: Some(Type::simple("void")),
                    body: vec![Stmt::Switch(SwitchStmt {
                        expr: Expr::Identifier("c".to_string(), Span::unknown()),
                        cases: vec![
                            Case {
                                pattern: Pattern::Constructor { name: "Red".to_string(), bindings: vec![] },
                                body: vec![],
                            },
                        ],
                        default: None,
                        span: Span::unknown(),
                    })],
                    sugar: false,
                    where_clause: vec![],
                    span: Span::unknown(),
                }),
            ],
        };
        let result = analyze_with_mode_and_warnings(&prog, ExhaustiveMode::Warning);
        assert!(result.is_ok(), "Warning mode should not produce an error");
        let (_, warnings) = result.unwrap();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("non-exhaustive pattern match"));
        assert!(warnings[0].contains("Green"));
        assert!(warnings[0].contains("Blue"));
    }

    #[test]
    fn test_non_exhaustive_switch_error_mode() {
        // Switch missing some variants 鈫?error in Error mode
        let prog = Program {
            imports: vec![],
            declarations: vec![
                Declaration::Enum(EnumDecl {
                    name: "Color".to_string(),
                    type_params: vec![],
                    variants: vec![
                        Variant { name: "Red".to_string(), fields: vec![] },
                        Variant { name: "Green".to_string(), fields: vec![] },
                    ],
                    span: Span::unknown(),
                }),
                Declaration::Function(FnDecl {
                    access: Access::Public,
                    name: "test".to_string(),
                    type_params: vec![],
                    params: vec![Param { name: "c".to_string(), typ: Type::simple("Color") }],
                    return_type: Some(Type::simple("void")),
                    body: vec![Stmt::Switch(SwitchStmt {
                        expr: Expr::Identifier("c".to_string(), Span::unknown()),
                        cases: vec![
                            Case {
                                pattern: Pattern::Constructor { name: "Red".to_string(), bindings: vec![] },
                                body: vec![],
                            },
                        ],
                        default: None,
                        span: Span::unknown(),
                    })],
                    sugar: false,
                    where_clause: vec![],
                    span: Span::unknown(),
                }),
            ],
        };
        let result = analyze_with_mode(&prog, ExhaustiveMode::Error);
        assert!(result.is_err(), "Error mode should produce an error for non-exhaustive switch");
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("non-exhaustive pattern match")));
    }

    #[test]
    fn test_switch_with_default_no_warning() {
        // Switch with default case 鈫?no exhaustiveness warning even if variants are missing
        let prog = Program {
            imports: vec![],
            declarations: vec![
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
                Declaration::Function(FnDecl {
                    access: Access::Public,
                    name: "test".to_string(),
                    type_params: vec![],
                    params: vec![Param { name: "c".to_string(), typ: Type::simple("Color") }],
                    return_type: Some(Type::simple("void")),
                    body: vec![Stmt::Switch(SwitchStmt {
                        expr: Expr::Identifier("c".to_string(), Span::unknown()),
                        cases: vec![
                            Case {
                                pattern: Pattern::Constructor { name: "Red".to_string(), bindings: vec![] },
                                body: vec![],
                            },
                        ],
                        default: Some(vec![]),
                        span: Span::unknown(),
                    })],
                    sugar: false,
                    where_clause: vec![],
                    span: Span::unknown(),
                }),
            ],
        };
        let result = analyze_with_mode_and_warnings(&prog, ExhaustiveMode::Warning);
        assert!(result.is_ok());
        let (_, warnings) = result.unwrap();
        assert!(warnings.is_empty(), "expected no warnings for switch with default, got: {:?}", warnings);
    }

    // -----------------------------------------------------------------------
    // Closure type inference tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_closure_type_inference() {
        // Verify that a closure assigned to a variable gets inferred as "function" type
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "f".to_string(),
                    typ: None,
                    init: Some(Expr::Closure {
                        params: vec![("x".to_string(), Type::simple("int"))],
                        return_type: Type::simple("int"),
                        body: vec![],
                        expr: Some(Box::new(Expr::Binary(
                            Box::new(Expr::Identifier("x".to_string(), Span::unknown())),
                            Operator::Mul,
                            Box::new(Expr::Literal(Literal::Int(2), Span::unknown())),
                            Span::unknown(),
                        ))),
                        captured_vars: vec![],
                        span: Span::unknown(),
                    }),
                    mutable: false,
                    span: Span::unknown(),
                }),
            ],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_ok(), "closure type inference should succeed");
        // Verify the inferred type
        let analyzed = result.unwrap();
        match &analyzed.declarations[0] {
            Declaration::Function(f) => {
                match &f.body[0] {
                    Stmt::VarDecl(v) => {
                        assert_eq!(v.typ, Some(Type::simple("function")),
                            "closure variable should be inferred as 'function' type");
                    }
                    _ => panic!("expected var decl"),
                }
            }
            _ => panic!("expected function"),
        }
    }

    // -----------------------------------------------------------------------
    // Tuple type inference tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_tuple_type_inference() {
        // Verify that a tuple variable gets inferred with a Tuple type
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "t".to_string(),
                    typ: None,
                    init: Some(Expr::Tuple(vec![
                        Expr::Literal(Literal::Int(1), Span::unknown()),
                        Expr::Literal(Literal::String("hello".to_string()), Span::unknown()),
                    ], Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
            ],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_ok(), "tuple type inference should succeed");
        let analyzed = result.unwrap();
        match &analyzed.declarations[0] {
            Declaration::Function(f) => {
                match &f.body[0] {
                    Stmt::VarDecl(v) => {
                        match &v.typ {
                            Some(Type::Tuple(types)) => {
                                assert_eq!(types.len(), 2, "tuple should have 2 element types");
                            }
                            other => panic!("expected Tuple type, got {:?}", other),
                        }
                    }
                    _ => panic!("expected var decl"),
                }
            }
            _ => panic!("expected function"),
        }
    }

    // -----------------------------------------------------------------------
    // Self parameter type test
    // -----------------------------------------------------------------------

    #[test]
    fn test_self_parameter_type() {
        // Verify that a method with `self` parameter (type Self) is valid
        let prog = program_with(Declaration::Class(ClassDecl {
            name: "Vec2".to_string(),
            type_params: vec![],
            parent: None,
            ifaces: vec![],
            members: vec![
                ClassMember::Field(FieldDecl {
                    access: Access::Public,
                    name: "x".to_string(),
                    typ: Type::simple("double"),
                    init: None,
                    span: Span::unknown(),
                }),
                ClassMember::Method(MethodDecl {
                    access: Access::Public,
                    name: "magnitude".to_string(),
                    type_params: vec![],
                    params: vec![
                        Param { name: "self".to_string(), typ: Type::simple("Self") },
                    ],
                    return_type: Some(Type::simple("double")),
                    body: vec![Stmt::Return(Some(Expr::MemberAccess(
                        Box::new(Expr::This(Span::unknown())),
                        "x".to_string(),
                        Span::unknown(),
                    )))],
                    where_clause: vec![],
                    span: Span::unknown(),
                }),
            ],
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok(), "method with self parameter should analyze successfully");
    }

    // -----------------------------------------------------------------------
