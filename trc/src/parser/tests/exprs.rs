use super::parse_src;
use crate::ast;

    // -----------------------------------------------------------------------
    // Desugaring tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_sugar_fn_desugar() {
        let src = r#"public int add(int a, int b) { return a + b; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => {
                assert_eq!(fd.name, "add");
                assert_eq!(fd.return_type, Some(ast::Type::simple("int")));
                assert_eq!(fd.params.len(), 2);
                assert_eq!(fd.params[0].name, "a");
                assert_eq!(fd.params[0].typ, ast::Type::simple("int"));
                assert_eq!(fd.params[1].name, "b");
                assert_eq!(fd.params[1].typ, ast::Type::simple("int"));
                assert!(fd.sugar);
                assert_eq!(fd.access, ast::Access::Public);
            }
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_var_desugar_top_level() {
        let src = r#"var x = 5;"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::VarDecl(vd) => {
                assert_eq!(vd.name, "x");
                assert!(vd.mutable);
                assert_eq!(vd.init, Some(ast::Expr::Literal(ast::Literal::Int(5), ast::Span::new(1, 9))));
            }
            other => panic!("Expected VarDecl, got {:?}", other),
        }
    }

    #[test]
    fn test_typed_var_desugar_top_level() {
        let src = r#"int x = 5;"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::VarDecl(vd) => {
                assert_eq!(vd.name, "x");
                assert_eq!(vd.typ, Some(ast::Type::simple("int")));
                assert!(vd.mutable);
            }
            other => panic!("Expected VarDecl, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Pattern tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_literal_pattern() {
        let src = r#"fn f(): void { switch x { case 42 => return; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Switch(ss) => {
                    assert_eq!(ss.cases.len(), 1);
                    assert_eq!(ss.cases[0].pattern, ast::Pattern::Literal(ast::Literal::Int(42)));
                }
                other => panic!("Expected Switch, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_wildcard_pattern() {
        let src = r#"fn f(): void { switch x { case _ => return; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Switch(ss) => {
                    assert_eq!(ss.cases[0].pattern, ast::Pattern::Wildcard);
                }
                other => panic!("Expected Switch, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_constructor_pattern() {
        let src = r#"fn f(): void { switch x { case Some(v) => return; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Switch(ss) => {
                    assert_eq!(
                        ss.cases[0].pattern,
                        ast::Pattern::Constructor {
                            name: "Some".to_string(),
                            bindings: vec!["v".to_string()]
                        }
                    );
                }
                other => panic!("Expected Switch, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Class tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_class_with_constructor() {
        let src = r#"class Circle {
            double radius;
            public Circle(double r) { this.radius = r; }
            fn area(): double { return 3.14 * this.radius * this.radius; }
        }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Class(cd) => {
                assert_eq!(cd.name, "Circle");
                assert_eq!(cd.members.len(), 3);
                // Field
                match &cd.members[0] {
                    ast::ClassMember::Field(f) => {
                        assert_eq!(f.name, "radius");
                    }
                    other => panic!("Expected Field, got {:?}", other),
                }
                // Constructor
                match &cd.members[1] {
                    ast::ClassMember::Constructor(m) => {
                        assert_eq!(m.name, "new");
                        assert_eq!(m.params.len(), 1);
                    }
                    other => panic!("Expected Constructor, got {:?}", other),
                }
                // Method
                match &cd.members[2] {
                    ast::ClassMember::Method(m) => {
                        assert_eq!(m.name, "area");
                    }
                    other => panic!("Expected Method, got {:?}", other),
                }
            }
            other => panic!("Expected Class, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Enum tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_enum_with_variants() {
        let src = r#"enum Color { Red, Green, Blue }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Enum(ed) => {
                assert_eq!(ed.name, "Color");
                assert_eq!(ed.variants.len(), 3);
                assert_eq!(ed.variants[0].name, "Red");
                assert!(ed.variants[0].fields.is_empty());
            }
            other => panic!("Expected Enum, got {:?}", other),
        }
    }

    #[test]
    fn test_enum_with_fields() {
        let src = r#"enum Shape { Circle(double), Rectangle(int, int) }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Enum(ed) => {
                assert_eq!(ed.variants[0].name, "Circle");
                assert_eq!(ed.variants[0].fields.len(), 1);
                assert_eq!(ed.variants[1].name, "Rectangle");
                assert_eq!(ed.variants[1].fields.len(), 2);
            }
            other => panic!("Expected Enum, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Import tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_import() {
        let src = r#"import io::println;"#;
        let prog = parse_src(src).expect("parse should succeed");
        assert_eq!(prog.imports.len(), 1);
        assert_eq!(prog.imports[0].path, vec!["io", "println"]);
    }

    // -----------------------------------------------------------------------
    // Type parsing tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_generic_type() {
        let src = r#"fn f(): Owned<int> { return x; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => {
                assert_eq!(
                    fd.return_type,
                    Some(ast::Type::generic("Owned", vec![ast::Type::simple("int")]))
                );
            }
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_multi_param_generic_type() {
        let src = r#"fn f(): Result<int, string> { return x; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => {
                assert_eq!(
                    fd.return_type,
                    Some(ast::Type::generic(
                        "Result",
                        vec![ast::Type::simple("int"), ast::Type::simple("string")]
                    ))
                );
            }
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // String concatenation
    // -----------------------------------------------------------------------
    #[test]
    fn test_string_concat() {
        let src = r#"fn f(): void { return "hello" + " world"; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Binary(_, ast::Operator::Add, _, _))) => {}
                other => panic!("Expected Binary Add, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Error cases
    // -----------------------------------------------------------------------
    #[test]
    fn test_missing_semicolon() {
        let src = r#"fn f(): void { let x = 5 }"#;
        let result = parse_src(src);
        assert!(result.is_err(), "Expected parse error for missing semicolon");
    }

    #[test]
    fn test_mismatched_parens() {
        let src = r#"fn f(): void { foo(; }"#;
        let result = parse_src(src);
        assert!(result.is_err(), "Expected parse error for mismatched parens");
    }

    #[test]
    fn test_missing_brace() {
        let src = r#"fn f(): void { let x = 5;"#;
        let result = parse_src(src);
        assert!(result.is_err(), "Expected parse error for missing brace");
    }

    #[test]
    fn test_invalid_token_in_expr() {
        let src = r#"fn f(): void { return @; }"#;
        let result = parse_src(src);
        // The lexer produces an Error token, the parser should fail on it
        assert!(result.is_err(), "Expected parse error for invalid token");
    }

    // -----------------------------------------------------------------------
    // Interface test
    // -----------------------------------------------------------------------
    #[test]
    fn test_interface() {
        let src = r#"interface Printable {
            fn format(): string;
        }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Interface(id) => {
                assert_eq!(id.name, "Printable");
                assert_eq!(id.methods.len(), 1);
                assert_eq!(id.methods[0].name, "format");
            }
            other => panic!("Expected Interface, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Canonical fn test
    // -----------------------------------------------------------------------
    #[test]
    fn test_canonical_fn() {
        let src = r#"fn add(a: int, b: int): int { return a + b; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => {
                assert_eq!(fd.name, "add");
                assert_eq!(fd.params.len(), 2);
                assert_eq!(fd.params[0].name, "a");
                assert_eq!(fd.params[0].typ, ast::Type::simple("int"));
                assert_eq!(fd.return_type, Some(ast::Type::simple("int")));
                assert!(!fd.sugar);
            }
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Owned expression
    // -----------------------------------------------------------------------
    #[test]
    fn test_owned_expr() {
        let src = r#"fn f(): void { Owned<int>(x); }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::OwnedDeref(inner, _)) => {
                    assert!(matches!(inner.as_ref(), ast::Expr::Identifier(s, _) if s == "x"));
                }
                other => panic!("Expected OwnedDeref, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Chained postfix
    // -----------------------------------------------------------------------
    #[test]
    fn test_chained_postfix() {
        let src = r#"fn f(): void { obj.method().field[0]; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Index(_, _, _)) => {}
                other => panic!("Expected Index, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Bitwise operators
    // -----------------------------------------------------------------------
    #[test]
    fn test_bitwise_ops() {
        let src = r#"fn f(): int { return a | b & c ^ d << e >> f; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                // Left-to-right association: the outermost is the last operator applied
                ast::Stmt::Return(Some(ast::Expr::Binary(_, ast::Operator::BitShr, _, _))) => {}
                other => panic!("Expected BitShr (outermost with left-to-right), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Logical operators
    // -----------------------------------------------------------------------
    #[test]
    fn test_logical_ops() {
        let src = r#"fn f(): bool { return a && b || c; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Binary(_, ast::Operator::Or, _, _))) => {}
                other => panic!("Expected Or, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Else if
    // -----------------------------------------------------------------------
    #[test]
    fn test_else_if() {
        let src = r#"fn f(): void { if (a) { x; } else if (b) { y; } else { z; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::If(if_stmt) => {
                    assert!(if_stmt.else_branch.is_some());
                    let else_block = if_stmt.else_branch.as_ref().unwrap();
                    assert_eq!(else_block.len(), 1);
                    // else if becomes a nested If
                    match &else_block[0] {
                        ast::Stmt::If(nested) => {
                            assert!(nested.else_branch.is_some());
                        }
                        other => panic!("Expected nested If, got {:?}", other),
                    }
                }
                other => panic!("Expected If, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Class with extends and implements
    // -----------------------------------------------------------------------
    #[test]
    fn test_class_extends_implements() {
        let src = r#"class Dog extends Animal implements Printable {
            fn speak(): void { bark(); }
        }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Class(cd) => {
                assert_eq!(cd.name, "Dog");
                assert_eq!(cd.parent, Some(ast::Type::simple("Animal")));
                assert_eq!(cd.ifaces.len(), 1);
                assert_eq!(cd.ifaces[0], ast::Type::simple("Printable"));
            }
            other => panic!("Expected Class, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Multiple imports
    // -----------------------------------------------------------------------
    #[test]
    fn test_multiple_imports() {
        let src = r#"import io::println;
import math::sqrt;"#;
        let prog = parse_src(src).expect("parse should succeed");
        assert_eq!(prog.imports.len(), 2);
        assert_eq!(prog.imports[0].path, vec!["io", "println"]);
        assert_eq!(prog.imports[1].path, vec!["math", "sqrt"]);
    }

    // -----------------------------------------------------------------------
    // Empty program
    // -----------------------------------------------------------------------
    #[test]
    fn test_empty_program() {
        let src = "";
        let prog = parse_src(src).expect("parse should succeed");
        assert!(prog.imports.is_empty());
        assert!(prog.declarations.is_empty());
    }

    // -----------------------------------------------------------------------
    // Char literal expression
    // -----------------------------------------------------------------------
    #[test]
    fn test_char_expr() {
        let src = r#"fn f(): char { return 'a'; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Literal(ast::Literal::Char('a'), _))) => {}
                other => panic!("Expected Return(Char('a')), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Const at top level
    // -----------------------------------------------------------------------
    #[test]
    fn test_top_level_const() {
        let src = r#"const PI: double = 3.14;"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::ConstDecl(vd) => {
                assert_eq!(vd.name, "PI");
                assert!(!vd.mutable);
            }
            other => panic!("Expected ConstDecl, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Let at top level
    // -----------------------------------------------------------------------
    #[test]
    fn test_top_level_let() {
        let src = r#"let x: int = 10;"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::VarDecl(vd) => {
                assert_eq!(vd.name, "x");
                assert!(vd.mutable);
            }
            other => panic!("Expected VarDecl, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Equality operators
    // -----------------------------------------------------------------------
    #[test]
    fn test_equality_ops() {
        let src = r#"fn f(): bool { return a == b != c; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Binary(_, ast::Operator::Ne, _, _))) => {}
                other => panic!("Expected Ne, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Comparison operators
    // -----------------------------------------------------------------------
    #[test]
    fn test_comparison_ops() {
        let src = r#"fn f(): bool { return a < b > c <= d >= e; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Binary(_, ast::Operator::Ge, _, _))) => {}
                other => panic!("Expected Ge, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // StaticCall — ClassName.method(args) via :: syntax
    // -----------------------------------------------------------------------
    #[test]
    fn test_static_call_via_coloncolon() {
        let src = r#"fn f(): void { Math::sqrt(4.0); }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Call(callee, _, _)) => {
                    // Math::sqrt becomes MemberAccess(Identifier("Math"), "sqrt")
                    assert!(matches!(callee.as_ref(), ast::Expr::MemberAccess(_, name, _) if name == "sqrt"));
                }
                other => panic!("Expected Call, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // For loop without var keyword
    // -----------------------------------------------------------------------
    #[test]
    fn test_for_without_var() {
        let src = r#"fn f(): void { for item in list { process(item); } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::For(fs) => {
                    assert_eq!(fs.var, "item");
                }
                other => panic!("Expected For, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // For with parens
    // -----------------------------------------------------------------------
    #[test]
    fn test_for_with_parens() {
        let src = r#"fn f(): void { for (item in list) { process(item); } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::For(fs) => {
                    assert_eq!(fs.var, "item");
                }
                other => panic!("Expected For, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_for_var_with_parens() {
        let src = r#"fn f(): void { for (var i in items) { continue; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::For(fs) => {
                    assert_eq!(fs.var, "i");
                }
                other => panic!("Expected For, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // While without parens
    // -----------------------------------------------------------------------
    #[test]
    fn test_while_no_parens() {
        let src = r#"fn f(): void { while true { break; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::While(_) => {}
                other => panic!("Expected While, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // String pattern in switch
    // -----------------------------------------------------------------------
    #[test]
    fn test_string_pattern() {
        let src = r#"fn f(): void { switch x { case "hello" => return; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Switch(ss) => {
                    assert_eq!(
                        ss.cases[0].pattern,
                        ast::Pattern::Literal(ast::Literal::String("hello".to_string()))
                    );
                }
                other => panic!("Expected Switch, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Bool pattern in switch
    // -----------------------------------------------------------------------
    #[test]
    fn test_bool_pattern() {
        let src = r#"fn f(): void { switch x { case true => return; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Switch(ss) => {
                    assert_eq!(
                        ss.cases[0].pattern,
                        ast::Pattern::Literal(ast::Literal::Bool(true))
                    );
                }
                other => panic!("Expected Switch, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Constructor pattern without bindings
    // -----------------------------------------------------------------------
    #[test]
    fn test_constructor_pattern_no_bindings() {
        let src = r#"fn f(): void { switch x { case None => return; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Switch(ss) => {
                    assert_eq!(
                        ss.cases[0].pattern,
                        ast::Pattern::Constructor {
                            name: "None".to_string(),
                            bindings: vec![]
                        }
                    );
                }
                other => panic!("Expected Switch, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Private access modifier
    // -----------------------------------------------------------------------
    #[test]
    fn test_private_fn() {
        let src = r#"private fn helper(): void { return; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => {
                assert_eq!(fd.access, ast::Access::Private);
            }
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Default access is private
    // -----------------------------------------------------------------------
    #[test]
    fn test_default_access() {
        let src = r#"fn f(): void { return; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => {
                assert_eq!(fd.access, ast::Access::Private);
            }
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Interface with extends
    // -----------------------------------------------------------------------
    #[test]
    fn test_interface_extends() {
        let src = r#"interface Serializable extends Base {
            fn serialize(): string;
        }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Interface(id) => {
                assert_eq!(id.name, "Serializable");
                assert_eq!(id.parents.len(), 1);
                assert_eq!(id.parents[0], ast::Type::simple("Base"));
            }
            other => panic!("Expected Interface, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Type parameters on declarations
    // -----------------------------------------------------------------------
    #[test]
    fn test_fn_type_params() {
        let src = r#"fn identity<T>(x: T): T { return x; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => {
                assert_eq!(fd.name, "identity");
                assert_eq!(fd.type_params, vec![ast::TypeParam { name: "T".to_string(), constraint: None }]);
            }
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_class_type_params() {
        let src = r#"class Container<T> { T value; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Class(cd) => {
                assert_eq!(cd.name, "Container");
                assert_eq!(cd.type_params, vec![ast::TypeParam { name: "T".to_string(), constraint: None }]);
            }
            other => panic!("Expected Class, got {:?}", other),
        }
    }

    #[test]
    fn test_interface_type_params() {
        let src = r#"interface Comparable<T> { fn compare(other: T): int; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Interface(id) => {
                assert_eq!(id.name, "Comparable");
                assert_eq!(id.type_params, vec![ast::TypeParam { name: "T".to_string(), constraint: None }]);
            }
            other => panic!("Expected Interface, got {:?}", other),
        }
    }

    #[test]
    fn test_enum_type_params() {
        let src = r#"enum Option<T> { Some(T), None }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Enum(ed) => {
                assert_eq!(ed.name, "Option");
                assert_eq!(ed.type_params, vec![ast::TypeParam { name: "T".to_string(), constraint: None }]);
            }
            other => panic!("Expected Enum, got {:?}", other),
        }
    }

    #[test]
    fn test_method_type_params() {
        let src = r#"class Foo { fn bar<U>(x: U): void { return; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Class(cd) => {
                assert_eq!(cd.members.len(), 1);
                match &cd.members[0] {
                    ast::ClassMember::Method(m) => {
                        assert_eq!(m.name, "bar");
                        assert_eq!(m.type_params, vec![ast::TypeParam { name: "U".to_string(), constraint: None }]);
                    }
                    other => panic!("Expected Method, got {:?}", other),
                }
            }
            other => panic!("Expected Class, got {:?}", other),
        }
    }

    #[test]
    fn test_multi_type_params() {
        let src = r#"class Map<K, V> { }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Class(cd) => {
                assert_eq!(cd.name, "Map");
                assert_eq!(cd.type_params, vec![ast::TypeParam { name: "K".to_string(), constraint: None }, ast::TypeParam { name: "V".to_string(), constraint: None }]);
            }
            other => panic!("Expected Class, got {:?}", other),
        }
    }

