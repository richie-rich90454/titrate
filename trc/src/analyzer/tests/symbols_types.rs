use super::program_with;
use super::super::analyze;
use crate::ast::*;

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
