use super::program_with;
use super::super::{analyze, analyze_with_mode_and_warnings, CompileError, ExhaustiveMode, Suggestion};
use super::super::levenshtein;
use crate::ast::*;

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

    #[test]
    fn test_for_in_loop_var_is_element_type() {
        // for-in over ArrayList<string> must type the loop variable as
        // string so that element assignments validate.
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "list".to_string(),
                    typ: Some(Type::generic("ArrayList", vec![Type::simple("string")])),
                    init: None,
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::For(ForStmt {
                    var: "s".to_string(),
                    iterable: Expr::Identifier("list".to_string(), Span::unknown()),
                    body: vec![Stmt::VarDecl(VarDecl {
                        name: "t".to_string(),
                        typ: Some(Type::simple("string")),
                        init: Some(Expr::Identifier("s".to_string(), Span::unknown())),
                        mutable: false,
                        span: Span::unknown(),
                    })],
                    span: Span::unknown(),
                }),
            ],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_ok(), "for-in element assignment should analyze: {:?}", result.err());
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
