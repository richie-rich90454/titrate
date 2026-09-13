use super::{empty_program, program_with};
use super::super::analyze;
use crate::ast::*;

    // Valid program tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_valid_empty_program() {
        let prog = empty_program();
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_valid_function_with_params() {
        let prog = program_with(Declaration::Function(FnDecl {
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
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_valid_if_else() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::If(IfStmt {
                condition: Expr::Literal(Literal::Bool(true), Span::unknown()),
                then_branch: vec![],
                else_branch: Some(vec![]),
                span: Span::unknown(),
            })],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_valid_while_loop() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::While(WhileStmt {
                condition: Expr::Literal(Literal::Bool(false), Span::unknown()),
                body: vec![Stmt::Break],
                span: Span::unknown(),
            })],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_valid_string_concat() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::VarDecl(VarDecl {
                name: "s".to_string(),
                typ: Some(Type::simple("string")),
                init: Some(Expr::Binary(
                    Box::new(Expr::Literal(Literal::String("hello".to_string()), Span::unknown())),
                    Operator::Add,
                    Box::new(Expr::Literal(Literal::String(" world".to_string()), Span::unknown())),
                    Span::unknown(),
                )),
                mutable: false,
                span: Span::unknown(),
            })],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    // -----------------------------------------------------------------------
    // toString desugaring tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_tostring_desugaring_int() {
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
                    init: Some(Expr::Literal(Literal::Int(42), Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::Expr(Expr::Call(
                    Box::new(Expr::MemberAccess(
                        Box::new(Expr::Identifier("x".to_string(), Span::unknown())),
                        "toString".to_string(),
                        Span::unknown(),
                    )),
                    vec![],
                    Span::unknown(),
                )),
            ],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_ok());

        // Verify the expression was desugared.
        let analyzed = result.unwrap();
        let body = match &analyzed.declarations[0] {
            Declaration::Function(f) => &f.body,
            _ => panic!("expected function"),
        };
        match &body[1] {
            Stmt::Expr(Expr::StaticCall { class_name, method, args, span: _ }) => {
                assert_eq!(class_name, "Integer");
                assert_eq!(method, "toString");
                assert_eq!(args.len(), 1);
            }
            other => panic!("expected StaticCall, got {:?}", other),
        }
    }

    #[test]
    fn test_tostring_desugaring_bool() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "b".to_string(),
                    typ: Some(Type::simple("bool")),
                    init: Some(Expr::Literal(Literal::Bool(true), Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::Expr(Expr::Call(
                    Box::new(Expr::MemberAccess(
                        Box::new(Expr::Identifier("b".to_string(), Span::unknown())),
                        "toString".to_string(),
                        Span::unknown(),
                    )),
                    vec![],
                    Span::unknown(),
                )),
            ],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_ok());

        let analyzed = result.unwrap();
        let body = match &analyzed.declarations[0] {
            Declaration::Function(f) => &f.body,
            _ => panic!("expected function"),
        };
        match &body[1] {
            Stmt::Expr(Expr::StaticCall { class_name, method, .. }) => {
                assert_eq!(class_name, "Boolean");
                assert_eq!(method, "toString");
            }
            other => panic!("expected StaticCall, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Switch / enum tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_switch_enum_valid() {
        let prog = Program {
            imports: vec![],
            declarations: vec![
                Declaration::Enum(EnumDecl {
                    name: "Color".to_string(),
                    type_params: vec![],
                    variants: vec![
                        Variant { name: "Red".to_string(), fields: vec![] },
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
                                pattern: Pattern::Constructor {
                                    name: "Red".to_string(),
                                    bindings: vec![],
                                },
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
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_switch_enum_wrong_variant() {
        let prog = Program {
            imports: vec![],
            declarations: vec![
                Declaration::Enum(EnumDecl {
                    name: "Color".to_string(),
                    type_params: vec![],
                    variants: vec![
                        Variant { name: "Red".to_string(), fields: vec![] },
                    ],
                    span: Span::unknown(),
                }),
                Declaration::Enum(EnumDecl {
                    name: "Shape".to_string(),
                    type_params: vec![],
                    variants: vec![
                        Variant { name: "Circle".to_string(), fields: vec![] },
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
                                pattern: Pattern::Constructor {
                                    name: "Circle".to_string(),
                                    bindings: vec![],
                                },
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
        let result = analyze(&prog);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Error propagation tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_error_propagation_non_result_function() {
        // Use ? operator in a function that doesn't return Result.
        // We use a literal wrapped in ErrorPropagation to avoid undeclared identifier issues.
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("int")),
            body: vec![Stmt::Expr(Expr::ErrorPropagation(
                Box::new(Expr::Literal(Literal::Int(42), Span::unknown())),
                Span::unknown(),
            ))],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs.iter().any(|e| e.contains("? operator")), "expected '? operator' error, got: {:?}", errs);
    }

    #[test]
    fn test_error_propagation_in_result_function() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::generic("Result", vec![Type::simple("int"), Type::simple("string")])),
            body: vec![Stmt::Return(Some(Expr::ErrorPropagation(
                Box::new(Expr::Call(
                    Box::new(Expr::Identifier("bar".to_string(), Span::unknown())),
                    vec![],
                    Span::unknown(),
                )),
                Span::unknown(),
            )))],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        // This should pass because the function returns Result.
        // The function "bar" is undeclared, but that's a separate concern.
        // The ? operator check should pass.
        let result = analyze(&prog);
        // May error on undeclared "bar" but not on ? operator.
        if let Err(errs) = &result {
            assert!(!errs.iter().any(|e| e.contains("? operator")), "unexpected ? operator error: {:?}", errs);
        }
    }

    // -----------------------------------------------------------------------
    // Cast tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_valid_numeric_cast() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::VarDecl(VarDecl {
                name: "x".to_string(),
                typ: Some(Type::simple("long")),
                init: Some(Expr::Cast(
                    Box::new(Expr::Literal(Literal::Int(42), Span::unknown())),
                    Type::simple("long"),
                    Span::unknown(),
                )),
                mutable: false,
                span: Span::unknown(),
            })],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_invalid_cast() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::Expr(Expr::Cast(
                Box::new(Expr::Literal(Literal::String("hello".to_string()), Span::unknown())),
                Type::simple("int"),
                Span::unknown(),
            ))],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Immutability tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_assign_to_immutable() {
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
                Stmt::Expr(Expr::Assign(
                    Box::new(Expr::Identifier("x".to_string(), Span::unknown())),
                    Box::new(Expr::Literal(Literal::Int(10), Span::unknown())),
                    Span::unknown(),
                )),
            ],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("immutable")));
    }

    #[test]
    fn test_assign_to_mutable() {
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
                Stmt::Expr(Expr::Assign(
                    Box::new(Expr::Identifier("x".to_string(), Span::unknown())),
                    Box::new(Expr::Literal(Literal::Int(10), Span::unknown())),
                    Span::unknown(),
                )),
            ],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    // -----------------------------------------------------------------------
    // Bitwise operator tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_bitwise_on_integers() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::Expr(Expr::Binary(
                Box::new(Expr::Literal(Literal::Int(1), Span::unknown())),
                Operator::BitAnd,
                Box::new(Expr::Literal(Literal::Int(2), Span::unknown())),
                Span::unknown(),
            ))],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_bitwise_on_non_integers() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::Expr(Expr::Binary(
                Box::new(Expr::Literal(Literal::Bool(true), Span::unknown())),
                Operator::BitAnd,
                Box::new(Expr::Literal(Literal::Int(2), Span::unknown())),
                Span::unknown(),
            ))],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Unary operator tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_unary_neg_on_numeric() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::Expr(Expr::Unary(
                UnOp::Neg,
                Box::new(Expr::Literal(Literal::Int(5), Span::unknown())),
                Span::unknown(),
            ))],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_unary_not_on_bool() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::Expr(Expr::Unary(
                UnOp::Not,
                Box::new(Expr::Literal(Literal::Bool(true), Span::unknown())),
                Span::unknown(),
            ))],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_unary_not_on_non_bool() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::Expr(Expr::Unary(
                UnOp::Not,
                Box::new(Expr::Literal(Literal::Int(5), Span::unknown())),
                Span::unknown(),
            ))],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
    }

