use super::parse_src;
use crate::ast;

    // -----------------------------------------------------------------------
    // For-in loop test
    // -----------------------------------------------------------------------
    #[test]
    fn test_for_in_loop() {
        let src = r#"fn f(): void { for (item in list) { process(item); } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::For(fs) => {
                    assert_eq!(fs.var, "item");
                    assert!(matches!(fs.iterable, ast::Expr::Identifier(_, _)));
                }
                other => panic!("Expected For, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // While-let test
    // -----------------------------------------------------------------------
    #[test]
    fn test_while_let() {
        let src = r#"fn f(): void { while let line = file.readLine() { io::println(line); } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::WhileLet(wl) => {
                    assert_eq!(wl.var_name, "line");
                }
                other => panic!("Expected WhileLet, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // C-style for loop test
    // -----------------------------------------------------------------------
    #[test]
    fn test_c_style_for() {
        let src = r#"fn f(): void { for (let i = 0; i < n; i = i + 1) { break; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::CFor(cfor) => {
                    assert!(cfor.init.is_some());
                    assert!(cfor.condition.is_some());
                    assert!(cfor.increment.is_some());
                    // Verify init is a VarDecl
                    match cfor.init.as_ref().unwrap().as_ref() {
                        ast::Stmt::VarDecl(vd) => {
                            assert_eq!(vd.name, "i");
                            assert!(vd.mutable);
                        }
                        other => panic!("Expected VarDecl in init, got {:?}", other),
                    }
                }
                other => panic!("Expected CFor, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Switch on enum value test
    // -----------------------------------------------------------------------
    #[test]
    fn test_switch_on_enum() {
        let src = r#"fn f(): void { switch color { case Red => return; case Blue => { break; } } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Switch(ss) => {
                    assert_eq!(ss.cases.len(), 2);
                    assert_eq!(
                        ss.cases[0].pattern,
                        ast::Pattern::Constructor { name: "Red".to_string(), bindings: vec![] }
                    );
                    assert_eq!(
                        ss.cases[1].pattern,
                        ast::Pattern::Constructor { name: "Blue".to_string(), bindings: vec![] }
                    );
                }
                other => panic!("Expected Switch, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Where clause test
    // -----------------------------------------------------------------------
    #[test]
    fn test_where_clause() {
        let src = r#"fn foo<T>(x: T): void where T: Comparable<T> { return; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => {
                assert_eq!(fd.name, "foo");
                assert_eq!(fd.type_params.len(), 1);
                assert_eq!(fd.type_params[0].name, "T");
                // where clause should be parsed
                assert!(!fd.where_clause.is_empty(), "expected where clause to be parsed");
            }
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Range expression tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_exclusive_range() {
        let src = r#"fn f(): void { 1..10; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Range(start, end, _)) => {
                    assert!(matches!(start.as_ref(), ast::Expr::Literal(ast::Literal::Int(1), _)));
                    assert!(matches!(end.as_ref(), ast::Expr::Literal(ast::Literal::Int(10), _)));
                }
                other => panic!("Expected Range, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_inclusive_range() {
        let src = r#"fn f(): void { 1..=10; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::RangeInclusive(start, end, _)) => {
                    assert!(matches!(start.as_ref(), ast::Expr::Literal(ast::Literal::Int(1), _)));
                    assert!(matches!(end.as_ref(), ast::Expr::Literal(ast::Literal::Int(10), _)));
                }
                other => panic!("Expected RangeInclusive, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_range_with_expressions() {
        let src = r#"fn f(): void { a + 1..b * 2; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Range(start, end, _)) => {
                    assert!(matches!(start.as_ref(), ast::Expr::Binary(_, ast::Operator::Add, _, _)));
                    assert!(matches!(end.as_ref(), ast::Expr::Binary(_, ast::Operator::Mul, _, _)));
                }
                other => panic!("Expected Range with expressions, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_range_assignment() {
        let src = r#"fn f(): void { x = 1..10; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(_, value, _)) => {
                    assert!(matches!(value.as_ref(), ast::Expr::Range(_, _, _)),
                        "Expected Range on right side of assignment");
                }
                other => panic!("Expected Assign with Range, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Raw string literal expression tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_raw_string_expr() {
        let src = r#"fn f(): void { return r"hello"; }"#;
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
    fn test_raw_string_with_hash_expr() {
        let src = r##"fn f(): void { return r#"has "quotes""#; }"##;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Literal(ast::Literal::String(s), _))) => {
                    assert_eq!(s, "has \"quotes\"");
                }
                other => panic!("Expected Return(String), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Byte literal expression tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_byte_literal_expr() {
        let src = r#"fn f(): void { return b'x'; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Literal(ast::Literal::Int(120), _))) => {}
                other => panic!("Expected Return(Int(120)), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Numeric format expression tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_underscore_number_expr() {
        let src = r#"fn f(): int { return 1_000_000; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Literal(ast::Literal::Int(1000000), _))) => {}
                other => panic!("Expected Return(Int(1000000)), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_binary_expr() {
        let src = r#"fn f(): int { return 0b1010; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Literal(ast::Literal::Int(10), _))) => {}
                other => panic!("Expected Return(Int(10)), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_octal_expr() {
        let src = r#"fn f(): int { return 0o777; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Literal(ast::Literal::Int(511), _))) => {}
                other => panic!("Expected Return(Int(511)), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Compound assignment desugaring tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_compound_assign_plus_equal() {
        let src = r#"fn f(): void { x += 1; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(target, value, _)) => {
                    // target should be x
                    assert!(matches!(target.as_ref(), ast::Expr::Identifier(name, _) if name == "x"));
                    // value should be x + 1
                    match value.as_ref() {
                        ast::Expr::Binary(left, ast::Operator::Add, right, _) => {
                            assert!(matches!(left.as_ref(), ast::Expr::Identifier(name, _) if name == "x"));
                            assert!(matches!(right.as_ref(), ast::Expr::Literal(ast::Literal::Int(1), _)));
                        }
                        other => panic!("Expected Binary(Add), got {:?}", other),
                    }
                }
                other => panic!("Expected Assign, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_compound_assign_minus_equal() {
        let src = r#"fn f(): void { x -= 2; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(_, value, _)) => {
                    assert!(matches!(value.as_ref(), ast::Expr::Binary(_, ast::Operator::Sub, _, _)));
                }
                other => panic!("Expected Assign, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_compound_assign_star_equal() {
        let src = r#"fn f(): void { x *= 3; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(_, value, _)) => {
                    assert!(matches!(value.as_ref(), ast::Expr::Binary(_, ast::Operator::Mul, _, _)));
                }
                other => panic!("Expected Assign, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_compound_assign_slash_equal() {
        let src = r#"fn f(): void { x /= 4; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(_, value, _)) => {
                    assert!(matches!(value.as_ref(), ast::Expr::Binary(_, ast::Operator::Div, _, _)));
                }
                other => panic!("Expected Assign, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_compound_assign_percent_equal() {
        let src = r#"fn f(): void { x %= 5; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(_, value, _)) => {
                    assert!(matches!(value.as_ref(), ast::Expr::Binary(_, ast::Operator::Mod, _, _)));
                }
                other => panic!("Expected Assign, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_compound_assign_bitand_equal() {
        let src = r#"fn f(): void { x &= 3; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(_, value, _)) => {
                    assert!(matches!(value.as_ref(), ast::Expr::Binary(_, ast::Operator::BitAnd, _, _)));
                }
                other => panic!("Expected Assign, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_compound_assign_bitor_equal() {
        let src = r#"fn f(): void { x |= 3; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(_, value, _)) => {
                    assert!(matches!(value.as_ref(), ast::Expr::Binary(_, ast::Operator::BitOr, _, _)));
                }
                other => panic!("Expected Assign, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_compound_assign_bitxor_equal() {
        let src = r#"fn f(): void { x ^= 3; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(_, value, _)) => {
                    assert!(matches!(value.as_ref(), ast::Expr::Binary(_, ast::Operator::BitXor, _, _)));
                }
                other => panic!("Expected Assign, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_compound_assign_left_shift_equal() {
        let src = r#"fn f(): void { x <<= 2; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(_, value, _)) => {
                    assert!(matches!(value.as_ref(), ast::Expr::Binary(_, ast::Operator::BitShl, _, _)));
                }
                other => panic!("Expected Assign, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_compound_assign_right_shift_equal() {
        let src = r#"fn f(): void { x >>= 2; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(_, value, _)) => {
                    assert!(matches!(value.as_ref(), ast::Expr::Binary(_, ast::Operator::BitShr, _, _)));
                }
                other => panic!("Expected Assign, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_compound_assign_member_access() {
        // obj.field += 1 should desugar to obj.field = obj.field + 1
        let src = r#"fn f(): void { obj.field += 1; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(target, value, _)) => {
                    assert!(matches!(target.as_ref(), ast::Expr::MemberAccess(_, name, _) if name == "field"));
                    match value.as_ref() {
                        ast::Expr::Binary(left, ast::Operator::Add, right, _) => {
                            assert!(matches!(left.as_ref(), ast::Expr::MemberAccess(_, name, _) if name == "field"));
                            assert!(matches!(right.as_ref(), ast::Expr::Literal(ast::Literal::Int(1), _)));
                        }
                        other => panic!("Expected Binary(Add), got {:?}", other),
                    }
                }
                other => panic!("Expected Assign, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_compound_assign_index_access() {
        // arr[0] += 1 should desugar to arr[0] = arr[0] + 1
        let src = r#"fn f(): void { arr[0] += 1; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(target, value, _)) => {
                    assert!(matches!(target.as_ref(), ast::Expr::Index(_, _, _)));
                    match value.as_ref() {
                        ast::Expr::Binary(left, ast::Operator::Add, right, _) => {
                            assert!(matches!(left.as_ref(), ast::Expr::Index(_, _, _)));
                            assert!(matches!(right.as_ref(), ast::Expr::Literal(ast::Literal::Int(1), _)));
                        }
                        other => panic!("Expected Binary(Add), got {:?}", other),
                    }
                }
                other => panic!("Expected Assign, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Increment/decrement operator tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_prefix_increment() {
        // ++x desugars to x = x + 1
        let src = r#"fn f(): void { ++x; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(target, value, _)) => {
                    assert!(matches!(target.as_ref(), ast::Expr::Identifier(name, _) if name == "x"));
                    match value.as_ref() {
                        ast::Expr::Binary(left, ast::Operator::Add, right, _) => {
                            assert!(matches!(left.as_ref(), ast::Expr::Identifier(name, _) if name == "x"));
                            assert!(matches!(right.as_ref(), ast::Expr::Literal(ast::Literal::Int(1), _)));
                        }
                        other => panic!("Expected Binary(Add), got {:?}", other),
                    }
                }
                other => panic!("Expected Assign, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_prefix_decrement() {
        // --x desugars to x = x - 1
        let src = r#"fn f(): void { --x; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(target, value, _)) => {
                    assert!(matches!(target.as_ref(), ast::Expr::Identifier(name, _) if name == "x"));
                    match value.as_ref() {
                        ast::Expr::Binary(left, ast::Operator::Sub, right, _) => {
                            assert!(matches!(left.as_ref(), ast::Expr::Identifier(name, _) if name == "x"));
                            assert!(matches!(right.as_ref(), ast::Expr::Literal(ast::Literal::Int(1), _)));
                        }
                        other => panic!("Expected Binary(Sub), got {:?}", other),
                    }
                }
                other => panic!("Expected Assign, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_postfix_increment() {
        // x++ desugars to x = x + 1
        let src = r#"fn f(): void { x++; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(target, value, _)) => {
                    assert!(matches!(target.as_ref(), ast::Expr::Identifier(name, _) if name == "x"));
                    match value.as_ref() {
                        ast::Expr::Binary(left, ast::Operator::Add, right, _) => {
                            assert!(matches!(left.as_ref(), ast::Expr::Identifier(name, _) if name == "x"));
                            assert!(matches!(right.as_ref(), ast::Expr::Literal(ast::Literal::Int(1), _)));
                        }
                        other => panic!("Expected Binary(Add), got {:?}", other),
                    }
                }
                other => panic!("Expected Assign, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_postfix_decrement() {
        // x-- desugars to x = x - 1
        let src = r#"fn f(): void { x--; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(target, value, _)) => {
                    assert!(matches!(target.as_ref(), ast::Expr::Identifier(name, _) if name == "x"));
                    match value.as_ref() {
                        ast::Expr::Binary(left, ast::Operator::Sub, right, _) => {
                            assert!(matches!(left.as_ref(), ast::Expr::Identifier(name, _) if name == "x"));
                            assert!(matches!(right.as_ref(), ast::Expr::Literal(ast::Literal::Int(1), _)));
                        }
                        other => panic!("Expected Binary(Sub), got {:?}", other),
                    }
                }
                other => panic!("Expected Assign, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Ternary operator tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_ternary_simple() {
        let src = r#"fn f(): int { return true ? 1 : 2; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Ternary { condition, then_expr, else_expr, .. })) => {
                    assert!(matches!(condition.as_ref(), ast::Expr::Literal(ast::Literal::Bool(true), _)));
                    assert!(matches!(then_expr.as_ref(), ast::Expr::Literal(ast::Literal::Int(1), _)));
                    assert!(matches!(else_expr.as_ref(), ast::Expr::Literal(ast::Literal::Int(2), _)));
                }
                other => panic!("Expected Return(Ternary), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_ternary_with_expressions() {
        let src = r#"fn f(): int { return x > 0 ? x : -x; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Ternary { .. })) => {}
                other => panic!("Expected Return(Ternary), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_ternary_right_associative() {
        // a ? b : c ? d : e  should parse as  a ? b : (c ? d : e)
        let src = r#"fn f(): int { return a ? 1 : b ? 2 : 3; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Ternary { else_expr, .. })) => {
                    // else_expr should be another Ternary
                    assert!(matches!(else_expr.as_ref(), ast::Expr::Ternary { .. }),
                        "Expected nested Ternary in else branch");
                }
                other => panic!("Expected Return(Ternary), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_ternary_in_assignment() {
        let src = r#"fn f(): void { x = cond ? 1 : 0; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(_, value, _)) => {
                    assert!(matches!(value.as_ref(), ast::Expr::Ternary { .. }),
                        "Expected Ternary on right side of assignment");
                }
                other => panic!("Expected Assign with Ternary, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_error_propagation_still_works() {
        // x? at end of expression should still be error propagation
        let src = r#"fn f(): void { x?; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::ErrorPropagation(_, _)) => {}
                other => panic!("Expected ErrorPropagation, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // String interpolation tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_string_interpolation_simple() {
        let src = r#"fn f(): void { let s = "Hello, ${name}!"; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::VarDecl(vd) => {
                    // Should desugar to: "Hello, " + name + "!"
                    match &vd.init {
                        Some(ast::Expr::Binary(left, ast::Operator::Add, right, _)) => {
                            // left = "Hello, " + name
                            match left.as_ref() {
                                ast::Expr::Binary(l2, ast::Operator::Add, mid, _) => {
                                    assert!(matches!(l2.as_ref(), ast::Expr::Literal(ast::Literal::String(s), _) if s == "Hello, "));
                                    assert!(matches!(mid.as_ref(), ast::Expr::Identifier(name, _) if name == "name"));
                                }
                                other => panic!("Expected nested Binary(Add), got {:?}", other),
                            }
                            // right = "!"
                            assert!(matches!(right.as_ref(), ast::Expr::Literal(ast::Literal::String(s), _) if s == "!"));
                        }
                        other => panic!("Expected Binary(Add) chain, got {:?}", other),
                    }
                }
                other => panic!("Expected VarDecl, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_string_interpolation_no_markers() {
        // String without ${ should still be a plain StringLiteral
        let src = r#"fn f(): void { let s = "Hello, world!"; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::VarDecl(vd) => {
                    assert!(matches!(
                        &vd.init,
                        Some(ast::Expr::Literal(ast::Literal::String(s), _)) if s == "Hello, world!"
                    ));
                }
                other => panic!("Expected VarDecl, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_string_interpolation_multiple() {
        let src = r#"fn f(): void { let s = "${a} and ${b}"; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::VarDecl(vd) => {
                    // Should desugar to: a + " and " + b
                    match &vd.init {
                        Some(ast::Expr::Binary(left, ast::Operator::Add, right, _)) => {
                            // left = a + " and "
                            match left.as_ref() {
                                ast::Expr::Binary(l2, ast::Operator::Add, mid, _) => {
                                    assert!(matches!(l2.as_ref(), ast::Expr::Identifier(name, _) if name == "a"));
                                    assert!(matches!(mid.as_ref(), ast::Expr::Literal(ast::Literal::String(s), _) if s == " and "));
                                }
                                other => panic!("Expected nested Binary(Add), got {:?}", other),
                            }
                            assert!(matches!(right.as_ref(), ast::Expr::Identifier(name, _) if name == "b"));
                        }
                        other => panic!("Expected Binary(Add) chain, got {:?}", other),
                    }
                }
                other => panic!("Expected VarDecl, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }
