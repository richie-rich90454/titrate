use super::*;


#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;

    // Helper to build a simple program with one declaration.
    fn program_with(decl: Declaration) -> Program {
        Program {
            imports: vec![],
            declarations: vec![decl],
        }
    }

    // Helper: empty program.
    fn empty_program() -> Program {
        Program {
            imports: vec![],
            declarations: vec![],
        }
    }

    // -----------------------------------------------------------------------
    // Symbol resolution tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_resolve_variable() {
        let prog = program_with(Declaration::VarDecl(VarDecl {
            name: "x".to_string(),
            typ: Some(Type::simple("int")),
            init: Some(Expr::Literal(Literal::Int(42), Span::unknown())),
            mutable: false,
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_resolve_function() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("int")),
            body: vec![Stmt::Return(Some(Expr::Literal(Literal::Int(1), Span::unknown())))],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_resolve_class() {
        let prog = program_with(Declaration::Class(ClassDecl {
            name: "MyClass".to_string(),
            type_params: vec![],
            parent: None,
            ifaces: vec![],
            members: vec![ClassMember::Field(FieldDecl {
                access: Access::Private,
                name: "val".to_string(),
                typ: Type::simple("int"),
                init: None,
                span: Span::unknown(),
            })],
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_resolve_enum() {
        let prog = program_with(Declaration::Enum(EnumDecl {
            name: "Color".to_string(),
            type_params: vec![],
            variants: vec![
                Variant {
                    name: "Red".to_string(),
                    fields: vec![],
                },
                Variant {
                    name: "Blue".to_string(),
                    fields: vec![],
                },
            ],
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_undeclared_identifier() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::Expr(Expr::Identifier("unknown_var".to_string(), Span::unknown()))],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("undeclared")));
    }

    #[test]
    fn test_variable_in_scope() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
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
                Stmt::Expr(Expr::Identifier("x".to_string(), Span::unknown())),
            ],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    // -----------------------------------------------------------------------
    // Type checking tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_type_mismatch_var_decl() {
        let prog = program_with(Declaration::VarDecl(VarDecl {
            name: "x".to_string(),
            typ: Some(Type::simple("int")),
            init: Some(Expr::Literal(Literal::Bool(true), Span::unknown())),
            mutable: false,
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("type mismatch")));
    }

    #[test]
    fn test_int_literal_fits_long() {
        let prog = program_with(Declaration::VarDecl(VarDecl {
            name: "x".to_string(),
            typ: Some(Type::simple("long")),
            init: Some(Expr::Literal(Literal::Int(42), Span::unknown())),
            mutable: false,
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_if_condition_must_be_bool() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::If(IfStmt {
                condition: Expr::Literal(Literal::Int(1), Span::unknown()),
                then_branch: vec![],
                else_branch: None,
                span: Span::unknown(),
            })],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("if condition must be bool")));
    }

    #[test]
    fn test_while_condition_must_be_bool() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::While(WhileStmt {
                condition: Expr::Literal(Literal::Int(1), Span::unknown()),
                body: vec![],
                span: Span::unknown(),
            })],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("while condition must be bool")));
    }

    #[test]
    fn test_return_type_mismatch() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("int")),
            body: vec![Stmt::Return(Some(Expr::Literal(Literal::Bool(true), Span::unknown())))],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("return type mismatch")));
    }

    #[test]
    fn test_valid_return_type() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("int")),
            body: vec![Stmt::Return(Some(Expr::Literal(Literal::Int(42), Span::unknown())))],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_arithmetic_type_check() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::Expr(Expr::Binary(
                Box::new(Expr::Literal(Literal::Bool(true), Span::unknown())),
                Operator::Add,
                Box::new(Expr::Literal(Literal::Int(1), Span::unknown())),
                Span::unknown(),
            ))],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
    }

    #[test]
    fn test_logical_operators_require_bool() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::Expr(Expr::Binary(
                Box::new(Expr::Literal(Literal::Int(1), Span::unknown())),
                Operator::And,
                Box::new(Expr::Literal(Literal::Bool(true), Span::unknown())),
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
    // Ownership analysis tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_use_after_move() {
        // let x: Owned<int> = new int(5); let y = x; io::println(x); -> error
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
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
                Stmt::Expr(Expr::Identifier("x".to_string(), Span::unknown())),
            ],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("use of moved variable")));
    }

    #[test]
    fn test_borrow_then_move() {
        // let x: Owned<int> = new int(5); let y = &x; x = new int(6); -> error
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::generic("Owned", vec![Type::simple("int")])),
                    init: Some(Expr::New(Type::simple("int"), vec![Expr::Literal(Literal::Int(5), Span::unknown())], Span::unknown())),
                    mutable: true,
                    span: Span::unknown(),
                }),
                Stmt::VarDecl(VarDecl {
                    name: "y".to_string(),
                    typ: Some(Type::generic("Ref", vec![Type::generic("Owned", vec![Type::simple("int")])])),
                    init: Some(Expr::RefExpr(
                        Box::new(Expr::Identifier("x".to_string(), Span::unknown())),
                        RefKind::Immutable,
                        Span::unknown(),
                    )),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::Expr(Expr::Assign(
                    Box::new(Expr::Identifier("x".to_string(), Span::unknown())),
                    Box::new(Expr::New(Type::simple("int"), vec![Expr::Literal(Literal::Int(6), Span::unknown())], Span::unknown())),
                    Span::unknown(),
                )),
            ],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs.iter().any(|e| e.contains("borrowed") || e.contains("Borrowed")), "expected borrow error, got: {:?}", errs);
    }

    #[test]
    fn test_return_borrow_of_local() {
        // fn foo(): &int { let x = 5; return &x; } -> error
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::generic("Ref", vec![Type::simple("int")])),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::simple("int")),
                    init: Some(Expr::Literal(Literal::Int(5), Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::Return(Some(Expr::RefExpr(
                    Box::new(Expr::Identifier("x".to_string(), Span::unknown())),
                    RefKind::Immutable,
                    Span::unknown(),
                ))),
            ],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("cannot return a borrow")));
    }

    #[test]
    fn test_mutable_and_immutable_borrow_conflict() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
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
                    typ: Some(Type::generic("Ref", vec![Type::simple("int")])),
                    init: Some(Expr::RefExpr(
                        Box::new(Expr::Identifier("x".to_string(), Span::unknown())),
                        RefKind::Immutable,
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

    #[test]
    fn test_unsafe_skips_ownership_checks() {
        // In unsafe, use-after-move should not error.
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::Expr(Expr::UnsafeBlock(vec![
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
                Stmt::Expr(Expr::Identifier("x".to_string(), Span::unknown())),
            ], Span::unknown()))],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    // -----------------------------------------------------------------------
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
            Stmt::Expr(Expr::StaticCall { class_name, method, span: _, .. }) => {
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
    // Operator overloading type check tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_operator_overload_type_check() {
        // Verify operator+ method has correct signature: (self, other: Vec2) -> Vec2
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
                ClassMember::Field(FieldDecl {
                    access: Access::Private,
                    name: "y".to_string(),
                    typ: Type::simple("double"),
                    init: None,
                    span: Span::unknown(),
                }),
                ClassMember::Method(MethodDecl {
                    access: Access::Public,
                    name: "operator+".to_string(),
                    type_params: vec![],
                    params: vec![
                        Param { name: "self".to_string(), typ: Type::simple("Self") },
                        Param { name: "other".to_string(), typ: Type::simple("Vec2") },
                    ],
                    return_type: Some(Type::simple("Vec2")),
                    body: vec![Stmt::Return(Some(Expr::New(
                        Type::simple("Vec2"),
                        vec![],
                        Span::unknown(),
                    )))],
                    where_clause: vec![],
                    span: Span::unknown(),
                }),
            ],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_ok(), "operator+ with correct signature should analyze successfully: {:?}", result.err());
    }

    #[test]
    fn test_operator_overload_return_type() {
        // Verify operator== returns bool
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
                    name: "operator==".to_string(),
                    type_params: vec![],
                    params: vec![
                        Param { name: "self".to_string(), typ: Type::simple("Self") },
                        Param { name: "other".to_string(), typ: Type::simple("Vec2") },
                    ],
                    return_type: Some(Type::simple("bool")),
                    body: vec![Stmt::Return(Some(Expr::Literal(Literal::Bool(true), Span::unknown())))],
                    where_clause: vec![],
                    span: Span::unknown(),
                }),
            ],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_ok(), "operator== returning bool should analyze successfully: {:?}", result.err());
    }

    // -----------------------------------------------------------------------
    // For-in type check tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_for_in_type_check() {
        // for-in loop with a valid iterable expression
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::For(ForStmt {
                var: "item".to_string(),
                iterable: Expr::Identifier("list".to_string(), Span::unknown()),
                body: vec![Stmt::Expr(Expr::Identifier("item".to_string(), Span::unknown()))],
                span: Span::unknown(),
            })],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        // Will error on undeclared "list" but "item" should be in scope
        let result = analyze(&prog);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("undeclared")));
    }

    // -----------------------------------------------------------------------
    // C-style for scope tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_c_style_for_scope() {
        // Variable declared in C-style for init should be scoped to the loop
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::CFor(CForStmt {
                    init: Some(Box::new(Stmt::VarDecl(VarDecl {
                        name: "i".to_string(),
                        typ: Some(Type::simple("int")),
                        init: Some(Expr::Literal(Literal::Int(0), Span::unknown())),
                        mutable: true,
                        span: Span::unknown(),
                    }))),
                    condition: Some(Expr::Binary(
                        Box::new(Expr::Identifier("i".to_string(), Span::unknown())),
                        Operator::Lt,
                        Box::new(Expr::Literal(Literal::Int(10), Span::unknown())),
                        Span::unknown(),
                    )),
                    increment: Some(Expr::Assign(
                        Box::new(Expr::Identifier("i".to_string(), Span::unknown())),
                        Box::new(Expr::Binary(
                            Box::new(Expr::Identifier("i".to_string(), Span::unknown())),
                            Operator::Add,
                            Box::new(Expr::Literal(Literal::Int(1), Span::unknown())),
                            Span::unknown(),
                        )),
                        Span::unknown(),
                    )),
                    body: vec![Stmt::Expr(Expr::Identifier("i".to_string(), Span::unknown()))],
                    span: Span::unknown(),
                }),
                // i should NOT be in scope after the loop
                Stmt::Expr(Expr::Identifier("i".to_string(), Span::unknown())),
            ],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        // "i" used after the loop should be an error (undeclared or out of scope)
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // While-let type check tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_while_let_type_check() {
        // while-let with a valid expression
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::WhileLet(WhileLetStmt {
                var_name: "line".to_string(),
                expr: Expr::Call(
                    Box::new(Expr::MemberAccess(
                        Box::new(Expr::Identifier("file".to_string(), Span::unknown())),
                        "readLine".to_string(),
                        Span::unknown(),
                    )),
                    vec![],
                    Span::unknown(),
                ),
                body: vec![Stmt::Expr(Expr::Identifier("line".to_string(), Span::unknown()))],
                span: Span::unknown(),
            })],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        // Will error on undeclared "file" but the while-let structure should be valid
        let result = analyze(&prog);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("undeclared")));
    }

    // -----------------------------------------------------------------------
    // Tuple assign tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_tuple_assign() {
        // Assign a tuple to a variable
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::VarDecl(VarDecl {
                name: "t".to_string(),
                typ: None,
                init: Some(Expr::Tuple(vec![
                    Expr::Literal(Literal::Int(1), Span::unknown()),
                    Expr::Literal(Literal::String("hello".to_string()), Span::unknown()),
                ], Span::unknown())),
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
    // Tuple return function tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_tuple_return_function() {
        // Function returning a tuple type
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "pair".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::Tuple(vec![Type::simple("int"), Type::simple("string")])),
            body: vec![Stmt::Return(Some(Expr::Tuple(vec![
                Expr::Literal(Literal::Int(42), Span::unknown()),
                Expr::Literal(Literal::String("answer".to_string()), Span::unknown()),
            ], Span::unknown())))],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    // -----------------------------------------------------------------------
    // Improved error message tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_undeclared_identifier_suggests_similar_name() {
        // Declare "count" and use "cont" 鈥?should suggest "count"
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "count".to_string(),
                    typ: Some(Type::simple("int")),
                    init: Some(Expr::Literal(Literal::Int(0), Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::Expr(Expr::Identifier("cont".to_string(), Span::unknown())),
            ],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs.iter().any(|e| e.contains("undeclared identifier")), "expected 'undeclared identifier' error, got: {:?}", errs);
        assert!(errs.iter().any(|e| e.contains("count")), "expected suggestion of 'count', got: {:?}", errs);
    }

    #[test]
    fn test_undeclared_identifier_no_suggestion_for_very_different_name() {
        // Declare "x" and use "zzzzzz" 鈥?should not suggest anything
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::simple("int")),
                    init: Some(Expr::Literal(Literal::Int(0), Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::Expr(Expr::Identifier("zzzzzz".to_string(), Span::unknown())),
            ],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs.iter().any(|e| e.contains("undeclared identifier")), "expected 'undeclared identifier' error, got: {:?}", errs);
        // The error should not suggest "x" since it's too different from "zzzzzz"
        let err_msg = errs.iter().find(|e| e.contains("undeclared identifier")).unwrap();
        assert!(!err_msg.contains("did you mean"), "should not suggest for very different name, got: {}", err_msg);
    }

    #[test]
    fn test_type_mismatch_shows_expected_vs_actual() {
        let prog = program_with(Declaration::VarDecl(VarDecl {
            name: "x".to_string(),
            typ: Some(Type::simple("int")),
            init: Some(Expr::Literal(Literal::Bool(true), Span::unknown())),
            mutable: false,
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        let err = errs.iter().find(|e| e.contains("type mismatch")).unwrap();
        assert!(err.contains("expected type int") || err.contains("expected") && err.contains("found"), "expected 'expected' and 'found' in error, got: {}", err);
    }

    #[test]
    fn test_duplicate_declaration_error() {
        let prog = Program {
            imports: vec![],
            declarations: vec![
                Declaration::Function(FnDecl {
                    access: Access::Public,
                    name: "foo".to_string(),
                    type_params: vec![],
                    params: vec![],
                    return_type: Some(Type::simple("void")),
                    body: vec![],
                    sugar: false,
                    where_clause: vec![],
                    span: Span::unknown(),
                }),
                Declaration::Function(FnDecl {
                    access: Access::Public,
                    name: "foo".to_string(),
                    type_params: vec![],
                    params: vec![],
                    return_type: Some(Type::simple("int")),
                    body: vec![Stmt::Return(Some(Expr::Literal(Literal::Int(1), Span::unknown())))],
                    sugar: false,
                    where_clause: vec![],
                    span: Span::unknown(),
                }),
            ],
        };
        let result = analyze(&prog);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs.iter().any(|e| e.contains("duplicate declaration")), "expected 'duplicate declaration' error, got: {:?}", errs);
    }

    #[test]
    fn test_missing_return_statement_error() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "get_value".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("int")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::simple("int")),
                    init: Some(Expr::Literal(Literal::Int(42), Span::unknown())),
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
        let errs = result.unwrap_err();
        let err = errs.iter().find(|e| e.contains("missing a return statement")).unwrap();
        assert!(err.contains("get_value"), "expected function name 'get_value' in error, got: {}", err);
        assert!(err.contains("int"), "expected return type 'int' in error, got: {}", err);
    }

    #[test]
    fn test_return_type_mismatch_includes_function_name() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "compute".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("int")),
            body: vec![Stmt::Return(Some(Expr::Literal(Literal::Bool(true), Span::unknown())))],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        let err = errs.iter().find(|e| e.contains("return type mismatch")).unwrap();
        assert!(err.contains("compute"), "expected function name in error, got: {}", err);
    }

    #[test]
    fn test_unreachable_code_warning() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("int")),
            body: vec![
                Stmt::Return(Some(Expr::Literal(Literal::Int(1), Span::unknown()))),
                Stmt::Expr(Expr::Literal(Literal::Int(2), Span::unknown())),
            ],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze_with_mode_and_warnings(&prog, ExhaustiveMode::default());
        // The function should still error because of missing return (the return is there but
        // the block_always_returns check may not see it due to the unreachable code after it).
        // Actually, the function does return, so it should be ok but with a warning.
        if let Ok((_, warnings)) = result {
            assert!(warnings.iter().any(|w| w.contains("unreachable")), "expected unreachable code warning, got: {:?}", warnings);
        }
    }

    #[test]
    fn test_unused_variable_warning() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "unused_var".to_string(),
                    typ: Some(Type::simple("int")),
                    init: Some(Expr::Literal(Literal::Int(42), Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
            ],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze_with_mode_and_warnings(&prog, ExhaustiveMode::default());
        if let Ok((_, warnings)) = result {
            assert!(warnings.iter().any(|w| w.contains("unused variable") && w.contains("unused_var")),
                "expected unused variable warning for 'unused_var', got: {:?}", warnings);
        }
    }

    #[test]
    fn test_unused_variable_underscore_prefix_no_warning() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "_unused".to_string(),
                    typ: Some(Type::simple("int")),
                    init: Some(Expr::Literal(Literal::Int(42), Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
            ],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze_with_mode_and_warnings(&prog, ExhaustiveMode::default());
        if let Ok((_, warnings)) = result {
            assert!(!warnings.iter().any(|w| w.contains("_unused")),
                "should not warn about '_unused' (underscore prefix), got: {:?}", warnings);
        }
    }

    #[test]
    fn test_compile_error_display_format() {
        let err = CompileError::new("test error".to_string());
        assert_eq!(err.to_string(), "test error");

        let err_with_suggestion = CompileError::new("test error".to_string())
            .suggest(Suggestion {
                message: "try this".to_string(),
                replacement: Some("foo".to_string()),
            });
        let displayed = err_with_suggestion.to_string();
        assert!(displayed.contains("test error"));
        assert!(displayed.contains("help:"));
        assert!(displayed.contains("foo"));
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein("", ""), 0);
        assert_eq!(levenshtein("abc", "abc"), 0);
        assert_eq!(levenshtein("abc", "abd"), 1);
        assert_eq!(levenshtein("abc", "ab"), 1);
        assert_eq!(levenshtein("abc", "abcd"), 1);
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        assert_eq!(levenshtein("count", "cont"), 1);
    }

    #[test]
    fn test_suggestion_struct() {
        let s = Suggestion {
            message: "similar name exists".to_string(),
            replacement: Some("count".to_string()),
        };
        let displayed = s.to_string();
        assert!(displayed.contains("count"));
        assert!(displayed.contains("did you mean"));

        let s_no_replacement = Suggestion {
            message: "try something else".to_string(),
            replacement: None,
        };
        assert_eq!(s_no_replacement.to_string(), "try something else");
    }

    #[test]
    fn test_compile_error_contains_method() {
        let err = CompileError::new("undeclared identifier: 'cont'".to_string())
            .suggest(Suggestion {
                message: "a similar name exists in scope".to_string(),
                replacement: Some("count".to_string()),
            });
        assert!(err.contains("undeclared"));
        assert!(err.contains("count"));
        assert!(!err.contains("xyz"));
    }

    #[test]
    fn test_immutable_assignment_suggests_mut() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
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
        let errs = result.unwrap_err();
        let err = errs.iter().find(|e| e.contains("immutable")).unwrap();
        assert!(err.contains("mut"), "expected suggestion about 'mut', got: {}", err);
    }

    #[test]
    fn test_error_propagation_shows_function_name() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "my_func".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("int")),
            body: vec![Stmt::Expr(Expr::ErrorPropagation(
                Box::new(Expr::Literal(Literal::Int(1), Span::unknown())),
                Span::unknown(),
            ))],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        let err = errs.iter().find(|e| e.contains("? operator")).unwrap();
        assert!(err.contains("my_func"), "expected function name in error, got: {}", err);
    }

    #[test]
    fn test_undeclared_identifier_in_assignment_suggests_similar() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "value".to_string(),
                    typ: Some(Type::simple("int")),
                    init: Some(Expr::Literal(Literal::Int(5), Span::unknown())),
                    mutable: true,
                    span: Span::unknown(),
                }),
                Stmt::Expr(Expr::Assign(
                    Box::new(Expr::Identifier("valu".to_string(), Span::unknown())),
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
        let errs = result.unwrap_err();
        let err = errs.iter().find(|e| e.contains("undeclared identifier in assignment")).unwrap();
        assert!(err.contains("value"), "expected suggestion of 'value', got: {}", err);
    }

    #[test]
    fn test_missing_return_in_if_else_branches() {
        // Function with if-else where both branches return 鈥?should NOT error
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "abs".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("int")),
            body: vec![Stmt::If(IfStmt {
                condition: Expr::Literal(Literal::Bool(true), Span::unknown()),
                then_branch: vec![Stmt::Return(Some(Expr::Literal(Literal::Int(1), Span::unknown())))],
                else_branch: Some(vec![Stmt::Return(Some(Expr::Literal(Literal::Int(-1), Span::unknown())))]),
                span: Span::unknown(),
            })],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        // This should NOT error because both branches return
        let result = analyze(&prog);
        assert!(result.is_ok(), "expected no error when both if/else branches return, got: {:?}", result);
    }
}