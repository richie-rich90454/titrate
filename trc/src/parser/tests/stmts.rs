use super::parse_src;
use crate::ast;

    // Micro-test
    // -----------------------------------------------------------------------
    #[test]
    fn test_micro_test() {
        let src = r#"int x = 5; public void main() { io::println("Hello"); }"#;
        let prog = parse_src(src).expect("parse should succeed");

        // x is VarDecl with type Named{name:"int"}, init Literal(Int(5)), mutable=true
        assert_eq!(prog.declarations.len(), 2);

        match &prog.declarations[0] {
            ast::Declaration::VarDecl(vd) => {
                assert_eq!(vd.name, "x");
                assert_eq!(vd.typ, Some(ast::Type::simple("int")));
                assert_eq!(vd.init, Some(ast::Expr::Literal(ast::Literal::Int(5), ast::Span::new(1, 9))));
                assert!(vd.mutable);
            }
            other => panic!("Expected VarDecl, got {:?}", other),
        }

        // main has return_type Named{name:"void"}, params empty
        match &prog.declarations[1] {
            ast::Declaration::Function(fd) => {
                assert_eq!(fd.name, "main");
                assert_eq!(fd.return_type, Some(ast::Type::simple("void")));
                assert!(fd.params.is_empty());
                assert!(fd.sugar);
            }
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Statement tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_block_stmt() {
        let src = r#"fn f(): void { { let x = 1; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => {
                assert_eq!(fd.body.len(), 1);
                match &fd.body[0] {
                    ast::Stmt::Block(block) => {
                        assert_eq!(block.len(), 1);
                    }
                    other => panic!("Expected Block, got {:?}", other),
                }
            }
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_if_stmt() {
        let src = r#"fn f(): void { if (true) { let x = 1; } else { let y = 2; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => {
                assert_eq!(fd.body.len(), 1);
                match &fd.body[0] {
                    ast::Stmt::If(if_stmt) => {
                        assert!(matches!(if_stmt.condition, ast::Expr::Literal(ast::Literal::Bool(true), _)));
                        assert_eq!(if_stmt.then_branch.len(), 1);
                        assert!(if_stmt.else_branch.is_some());
                    }
                    other => panic!("Expected If, got {:?}", other),
                }
            }
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_if_no_parens() {
        let src = r#"fn f(): void { if true { let x = 1; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::If(_) => {}
                other => panic!("Expected If, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_while_stmt() {
        let src = r#"fn f(): void { while (true) { break; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::While(ws) => {
                    assert!(matches!(ws.condition, ast::Expr::Literal(ast::Literal::Bool(true), _)));
                    assert_eq!(ws.body.len(), 1);
                    assert!(matches!(&ws.body[0], ast::Stmt::Break));
                }
                other => panic!("Expected While, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_do_while_stmt() {
        let src = r#"fn f(): void { do { x = x + 1; } while (x < 10); }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::DoWhile(dw) => {
                    assert!(matches!(dw.condition, ast::Expr::Binary(_, ast::Operator::Lt, _, _)));
                    assert_eq!(dw.body.len(), 1);
                }
                other => panic!("Expected DoWhile, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_do_while_no_parens() {
        let src = r#"fn f(): void { do { break; } while true; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::DoWhile(dw) => {
                    assert!(matches!(dw.condition, ast::Expr::Literal(ast::Literal::Bool(true), _)));
                }
                other => panic!("Expected DoWhile, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_for_stmt() {
        let src = r#"fn f(): void { for var i in items { continue; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::For(fs) => {
                    assert_eq!(fs.var, "i");
                    assert!(matches!(fs.iterable, ast::Expr::Identifier(_, _)));
                    assert_eq!(fs.body.len(), 1);
                    assert!(matches!(&fs.body[0], ast::Stmt::Continue));
                }
                other => panic!("Expected For, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_return_stmt() {
        let src = r#"fn f(): int { return 42; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Literal(ast::Literal::Int(42), _))) => {}
                other => panic!("Expected Return(Some(42)), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_return_void() {
        let src = r#"fn f(): void { return; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(None) => {}
                other => panic!("Expected Return(None), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_switch_stmt() {
        let src = r#"fn f(): void { switch x { case 1 => return; case 2 => { break; } default => { continue; } } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Switch(ss) => {
                    assert_eq!(ss.cases.len(), 2);
                    assert!(ss.default.is_some());
                }
                other => panic!("Expected Switch, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_let_stmt() {
        let src = r#"fn f(): void { let x: int = 5; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::VarDecl(vd) => {
                    assert_eq!(vd.name, "x");
                    assert_eq!(vd.typ, Some(ast::Type::simple("int")));
                    assert!(vd.mutable);
                }
                other => panic!("Expected VarDecl, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_var_desugar() {
        let src = r#"fn f(): void { var x = 5; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::VarDecl(vd) => {
                    assert_eq!(vd.name, "x");
                    assert!(vd.mutable);
                }
                other => panic!("Expected VarDecl, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_const_stmt() {
        let src = r#"fn f(): void { const X: int = 42; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::ConstDecl(vd) => {
                    assert_eq!(vd.name, "X");
                    assert!(!vd.mutable);
                }
                other => panic!("Expected ConstDecl, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_typed_var_desugar() {
        let src = r#"fn f(): void { int x = 5; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::VarDecl(vd) => {
                    assert_eq!(vd.name, "x");
                    assert_eq!(vd.typ, Some(ast::Type::simple("int")));
                    assert!(vd.mutable);
                }
                other => panic!("Expected VarDecl, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_stmt() {
        let src = r#"fn f(): void { foo(); }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Call(_, args, _)) => {
                    assert!(args.is_empty());
                }
                other => panic!("Expected Expr(Call), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Expression tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_int_literal() {
        let src = r#"fn f(): int { return 42; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Literal(ast::Literal::Int(42), _))) => {}
                other => panic!("Expected Return(Int(42)), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_float_literal() {
        let src = r#"fn f(): double { return 3.14; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Literal(ast::Literal::Float(_), _))) => {}
                other => panic!("Expected Return(Float), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_string_literal() {
        let src = r#"fn f(): void { return "hello"; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Literal(ast::Literal::String(s), _))) => {
                    assert_eq!(s, "hello");
                }
                other => panic!("Expected Return(String), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_bool_literal() {
        let src = r#"fn f(): bool { return true; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Literal(ast::Literal::Bool(true), _))) => {}
                other => panic!("Expected Return(Bool(true)), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_null_literal() {
        let src = r#"fn f(): void { return null; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Literal(ast::Literal::Null, _))) => {}
                other => panic!("Expected Return(Null), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_identifier_expr() {
        let src = r#"fn f(): int { return x; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Identifier(s, _))) => {
                    assert_eq!(s, "x");
                }
                other => panic!("Expected Return(Identifier), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_this_expr() {
        let src = r#"fn f(): void { return this; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::This(_))) => {}
                other => panic!("Expected Return(This), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_super_expr() {
        let src = r#"fn f(): void { return super; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Super(_))) => {}
                other => panic!("Expected Return(Super), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_paren_expr() {
        let src = r#"fn f(): int { return (1 + 2); }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Binary(_, ast::Operator::Add, _, _))) => {}
                other => panic!("Expected Return(Binary Add), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_new_expr() {
        let src = r#"fn f(): void { new Foo(1, 2); }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::New(typ, args, _)) => {
                    assert_eq!(typ, &ast::Type::simple("Foo"));
                    assert_eq!(args.len(), 2);
                }
                other => panic!("Expected New, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_ok_expr() {
        let src = r#"fn f(): void { return Ok(42); }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Call(callee, args, _))) => {
                    assert!(matches!(callee.as_ref(), ast::Expr::Identifier(s, _) if s == "Ok"));
                    assert_eq!(args.len(), 1);
                }
                other => panic!("Expected Return(Call(Ok, ...)), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_err_expr() {
        let src = r#"fn f(): void { return Err("oops"); }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Call(callee, args, _))) => {
                    assert!(matches!(callee.as_ref(), ast::Expr::Identifier(s, _) if s == "Err"));
                    assert_eq!(args.len(), 1);
                }
                other => panic!("Expected Return(Call(Err, ...)), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_unsafe_block() {
        let src = r#"fn f(): void { unsafe { let x = 1; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::UnsafeBlock(block, _)) => {
                    assert_eq!(block.len(), 1);
                }
                other => panic!("Expected UnsafeBlock, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_region_block() {
        let src = r#"fn f(): void { region r { let x = 1; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::UnsafeBlock(block, _)) => {
                    assert_eq!(block.len(), 1);
                }
                other => panic!("Expected UnsafeBlock (region), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Postfix expression tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_call_expr() {
        let src = r#"fn f(): void { foo(1, 2); }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Call(_, args, _)) => {
                    assert_eq!(args.len(), 2);
                }
                other => panic!("Expected Call, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_member_access() {
        let src = r#"fn f(): void { obj.field; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::MemberAccess(_, name, _)) => {
                    assert_eq!(name, "field");
                }
                other => panic!("Expected MemberAccess, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_namespace_access() {
        let src = r#"fn f(): void { io::println("hi"); }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Call(callee, _, _)) => {
                    assert!(matches!(callee.as_ref(), ast::Expr::MemberAccess(_, name, _) if name == "println"));
                }
                other => panic!("Expected Call(MemberAccess), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_index_expr() {
        let src = r#"fn f(): void { arr[0]; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Index(_, _, _)) => {}
                other => panic!("Expected Index, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_error_propagation() {
        let src = r#"fn f(): void { foo?; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::ErrorPropagation(_, _)) => {}
                other => panic!("Expected ErrorPropagation, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_cast_expr() {
        let src = r#"fn f(): void { x as int; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Cast(_, typ, _)) => {
                    assert_eq!(typ, &ast::Type::simple("int"));
                }
                other => panic!("Expected Cast, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Operator precedence tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_addition_precedence() {
        let src = r#"fn f(): int { return 1 + 2 * 3; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Binary(_, ast::Operator::Add, right, _))) => {
                    // Right side should be 2 * 3
                    assert!(matches!(right.as_ref(), ast::Expr::Binary(_, ast::Operator::Mul, _, _)));
                }
                other => panic!("Expected Add with Mul on right, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_assignment_right_assoc() {
        let src = r#"fn f(): void { x = y = 5; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(_, right, _)) => {
                    // Right side should be y = 5
                    assert!(matches!(right.as_ref(), ast::Expr::Assign(_, _, _)));
                }
                other => panic!("Expected Assign with Assign on right, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_unary_neg() {
        let src = r#"fn f(): int { return -5; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Unary(ast::UnOp::Neg, _, _))) => {}
                other => panic!("Expected Unary Neg, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_unary_not() {
        let src = r#"fn f(): bool { return !true; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Unary(ast::UnOp::Not, _, _))) => {}
                other => panic!("Expected Unary Not, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_unary_bitnot() {
        let src = r#"fn f(): int { return ~5; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Unary(ast::UnOp::BitNot, _, _))) => {}
                other => panic!("Expected Unary BitNot, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_ref_expr() {
        let src = r#"fn f(): void { let p = &x; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::VarDecl(vd) => match &vd.init {
                    Some(ast::Expr::RefExpr(_, ast::RefKind::Immutable, _)) => {}
                    other => panic!("Expected RefExpr Immutable, got {:?}", other),
                },
                other => panic!("Expected VarDecl, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_ref_mut_expr() {
        let src = r#"fn f(): void { let p = &mut x; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::VarDecl(vd) => match &vd.init {
                    Some(ast::Expr::RefExpr(_, ast::RefKind::Mutable, _)) => {}
                    other => panic!("Expected RefExpr Mutable, got {:?}", other),
                },
                other => panic!("Expected VarDecl, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

