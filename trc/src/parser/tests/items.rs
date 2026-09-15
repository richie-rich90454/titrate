use super::parse_src;
use crate::ast;

    // -----------------------------------------------------------------------
    // C-style for loop tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_c_for_basic() {
        let src = r#"fn f(): void { for (let i = 0; i < 10; i = i + 1) { break; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::CFor(cfor) => {
                    // init should be a VarDecl for i
                    assert!(cfor.init.is_some());
                    match cfor.init.as_ref().unwrap().as_ref() {
                        ast::Stmt::VarDecl(vd) => {
                            assert_eq!(vd.name, "i");
                            assert!(vd.mutable);
                        }
                        other => panic!("Expected VarDecl in init, got {:?}", other),
                    }
                    // condition should be a binary expr
                    assert!(cfor.condition.is_some());
                    assert!(matches!(cfor.condition.as_ref().unwrap(), ast::Expr::Binary(_, ast::Operator::Lt, _, _)));
                    // increment should be an assignment
                    assert!(cfor.increment.is_some());
                    assert!(matches!(cfor.increment.as_ref().unwrap(), ast::Expr::Assign(_, _, _)));
                    assert_eq!(cfor.body.len(), 1);
                }
                other => panic!("Expected CFor, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_c_for_with_var() {
        let src = r#"fn f(): void { for (var j = low; j < high; j = j + 1) { continue; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::CFor(cfor) => {
                    assert!(cfor.init.is_some());
                    match cfor.init.as_ref().unwrap().as_ref() {
                        ast::Stmt::VarDecl(vd) => {
                            assert_eq!(vd.name, "j");
                            assert!(vd.mutable);
                        }
                        other => panic!("Expected VarDecl (mutable) in init, got {:?}", other),
                    }
                }
                other => panic!("Expected CFor, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_c_for_with_method_call() {
        let src = r#"fn f(): void { for (let i = 0; i < numbers.size(); i = i + 1) { break; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::CFor(cfor) => {
                    // condition should include a method call
                    assert!(cfor.condition.is_some());
                }
                other => panic!("Expected CFor, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_c_for_no_init() {
        let src = r#"fn f(): void { for (; i < 10; i = i + 1) { break; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::CFor(cfor) => {
                    assert!(cfor.init.is_none());
                    assert!(cfor.condition.is_some());
                    assert!(cfor.increment.is_some());
                }
                other => panic!("Expected CFor, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_c_for_no_condition() {
        let src = r#"fn f(): void { for (let i = 0; ; i = i + 1) { break; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::CFor(cfor) => {
                    assert!(cfor.init.is_some());
                    assert!(cfor.condition.is_none());
                    assert!(cfor.increment.is_some());
                }
                other => panic!("Expected CFor, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_c_for_no_increment() {
        let src = r#"fn f(): void { for (let i = 0; i < 10; ) { break; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::CFor(cfor) => {
                    assert!(cfor.init.is_some());
                    assert!(cfor.condition.is_some());
                    assert!(cfor.increment.is_none());
                }
                other => panic!("Expected CFor, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_for_in_still_works_without_parens() {
        let src = r#"fn f(): void { for item in list { process(item); } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::For(fs) => {
                    assert_eq!(fs.var, "item");
                }
                other => panic!("Expected For (for-in), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_for_in_still_works_with_parens() {
        let src = r#"fn f(): void { for (item in list) { process(item); } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::For(fs) => {
                    assert_eq!(fs.var, "item");
                }
                other => panic!("Expected For (for-in), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_for_in_var_still_works_with_parens() {
        let src = r#"fn f(): void { for (var i in items) { continue; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::For(fs) => {
                    assert_eq!(fs.var, "i");
                }
                other => panic!("Expected For (for-in), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Operator overloading tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_operator_overload_method() {
        let src = r#"class Vec2 {
    double x;
    double y;

    fn operator+(self, other: Vec2): Vec2 {
        return new Vec2(this.x + other.x, this.y + other.y);
    }
}"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Class(cd) => {
                assert_eq!(cd.name, "Vec2");
                // Should have 2 fields and 1 method
                let methods: Vec<_> = cd.members.iter().filter_map(|m| match m {
                    ast::ClassMember::Method(m) => Some(m.name.clone()),
                    _ => None,
                }).collect();
                assert!(methods.contains(&"operator+".to_string()),
                    "Expected operator+ method, found methods: {:?}", methods);
            }
            other => panic!("Expected Class, got {:?}", other),
        }
    }

    #[test]
    fn test_operator_overload_comparison() {
        let src = r#"class Vec2 {
    double x;
    double y;

    fn operator==(self, other: Vec2): bool {
        return this.x == other.x && this.y == other.y;
    }

    fn operator!=(self, other: Vec2): bool {
        return !(this == other);
    }
}"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Class(cd) => {
                assert_eq!(cd.name, "Vec2");
                let methods: Vec<_> = cd.members.iter().filter_map(|m| match m {
                    ast::ClassMember::Method(m) => Some(m.name.clone()),
                    _ => None,
                }).collect();
                assert!(methods.contains(&"operator==".to_string()),
                    "Expected operator== method, found methods: {:?}", methods);
                assert!(methods.contains(&"operator!=".to_string()),
                    "Expected operator!= method, found methods: {:?}", methods);
            }
            other => panic!("Expected Class, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Tuple tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_tuple_two_elements() {
        let src = r#"fn f(): void { let x = (1, "hello"); }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => {
                assert_eq!(fd.body.len(), 1);
                match &fd.body[0] {
                    ast::Stmt::VarDecl(vd) => {
                        match &vd.init {
                            Some(ast::Expr::Tuple(elements, _)) => {
                                assert_eq!(elements.len(), 2);
                            }
                            other => panic!("Expected Tuple expression, got {:?}", other),
                        }
                    }
                    other => panic!("Expected VarDecl, got {:?}", other),
                }
            }
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_tuple_three_elements() {
        let src = r#"fn f(): void { let x = (1, 2, 3); }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => {
                assert_eq!(fd.body.len(), 1);
                match &fd.body[0] {
                    ast::Stmt::VarDecl(vd) => {
                        match &vd.init {
                            Some(ast::Expr::Tuple(elements, _)) => {
                                assert_eq!(elements.len(), 3);
                            }
                            other => panic!("Expected Tuple expression, got {:?}", other),
                        }
                    }
                    other => panic!("Expected VarDecl, got {:?}", other),
                }
            }
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_tuple_type_annotation() {
        let src = r#"fn f(): void { let x: (int, string) = (1, "a"); }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => {
                assert_eq!(fd.body.len(), 1);
                match &fd.body[0] {
                    ast::Stmt::VarDecl(vd) => {
                        match &vd.typ {
                            Some(ast::Type::Tuple(types)) => {
                                assert_eq!(types.len(), 2);
                            }
                            other => panic!("Expected Tuple type, got {:?}", other),
                        }
                    }
                    other => panic!("Expected VarDecl, got {:?}", other),
                }
            }
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_tuple_destructuring() {
        let src = r#"fn f(): void { let (a, b) = pair; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => {
                assert_eq!(fd.body.len(), 1);
                match &fd.body[0] {
                    ast::Stmt::TupleDestructure { names, mutable, .. } => {
                        assert_eq!(names.len(), 2);
                        assert_eq!(names[0], "a");
                        assert_eq!(names[1], "b");
                        assert!(mutable);
                    }
                    other => panic!("Expected TupleDestructure, got {:?}", other),
                }
            }
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Closure tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_closure_expression_body() {
        let src = r#"fn f(): void { let g = fn(x) => x * 2; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => {
                assert_eq!(fd.body.len(), 1);
                match &fd.body[0] {
                    ast::Stmt::VarDecl(vd) => {
                        assert_eq!(vd.name, "g");
                        match &vd.init {
                            Some(ast::Expr::Closure { params, expr, body, .. }) => {
                                assert_eq!(params.len(), 1);
                                assert_eq!(params[0].0, "x");
                                assert!(expr.is_some(), "closure should have expression body");
                                assert!(body.is_empty(), "closure with => should have empty block body");
                            }
                            other => panic!("Expected Closure, got {:?}", other),
                        }
                    }
                    other => panic!("Expected VarDecl, got {:?}", other),
                }
            }
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_closure_block_body() {
        let src = r#"fn f(): void { let g = fn(x) { return x * 2; }; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => {
                assert_eq!(fd.body.len(), 1);
                match &fd.body[0] {
                    ast::Stmt::VarDecl(vd) => {
                        assert_eq!(vd.name, "g");
                        match &vd.init {
                            Some(ast::Expr::Closure { params, expr, body, .. }) => {
                                assert_eq!(params.len(), 1);
                                assert_eq!(params[0].0, "x");
                                assert!(expr.is_none(), "closure with block body should not have expression");
                                assert_eq!(body.len(), 1, "closure block body should have 1 statement");
                            }
                            other => panic!("Expected Closure, got {:?}", other),
                        }
                    }
                    other => panic!("Expected VarDecl, got {:?}", other),
                }
            }
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_closure_capture() {
        let src = r#"fn f(): void { let y = 10; let g = fn(x) => x + y; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => {
                assert_eq!(fd.body.len(), 2);
                // Second statement is the closure var decl
                match &fd.body[1] {
                    ast::Stmt::VarDecl(vd) => {
                        assert_eq!(vd.name, "g");
                        match &vd.init {
                            Some(ast::Expr::Closure { params, .. }) => {
                                assert_eq!(params.len(), 1);
                                assert_eq!(params[0].0, "x");
                            }
                            other => panic!("Expected Closure, got {:?}", other),
                        }
                    }
                    other => panic!("Expected VarDecl, got {:?}", other),
                }
            }
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Self parameter tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_self_parameter() {
        let src = r#"class Foo { fn method(self, x: int): void { } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Class(cd) => {
                assert_eq!(cd.name, "Foo");
                match &cd.members[0] {
                    ast::ClassMember::Method(m) => {
                        assert_eq!(m.name, "method");
                        assert_eq!(m.params.len(), 2);
                        assert_eq!(m.params[0].name, "self");
                        assert_eq!(m.params[0].typ, ast::Type::simple("Self"));
                        assert_eq!(m.params[1].name, "x");
                        assert_eq!(m.params[1].typ, ast::Type::simple("int"));
                    }
                    other => panic!("Expected Method, got {:?}", other),
                }
            }
            other => panic!("Expected Class, got {:?}", other),
        }
    }

    #[test]
    fn test_self_with_type() {
        let src = r#"class Vec2 { fn method(self: Vec2, x: int): void { } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Class(cd) => {
                assert_eq!(cd.name, "Vec2");
                match &cd.members[0] {
                    ast::ClassMember::Method(m) => {
                        assert_eq!(m.name, "method");
                        assert_eq!(m.params.len(), 2);
                        assert_eq!(m.params[0].name, "self");
                        assert_eq!(m.params[0].typ, ast::Type::simple("Vec2"));
                        assert_eq!(m.params[1].name, "x");
                        assert_eq!(m.params[1].typ, ast::Type::simple("int"));
                    }
                    other => panic!("Expected Method, got {:?}", other),
                }
            }
            other => panic!("Expected Class, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Stdlib import tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_import_tt_math() {
        let src = r#"import tt::math::Math;"#;
        let prog = parse_src(src).expect("parse should succeed");
        assert_eq!(prog.imports.len(), 1);
        assert_eq!(prog.imports[0].path, vec!["tt", "math", "Math"]);
    }

    #[test]
    fn test_import_tt_chem() {
        let src = r#"import tt::chem::Molecule;"#;
        let prog = parse_src(src).expect("parse should succeed");
        assert_eq!(prog.imports.len(), 1);
        assert_eq!(prog.imports[0].path, vec!["tt", "chem", "Molecule"]);
    }

    #[test]
    fn test_static_call_math() {
        let src = r#"fn f(): void { Math::sqrt(4.0); }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Call(callee, args, _)) => {
                    assert!(matches!(callee.as_ref(), ast::Expr::MemberAccess(_, name, _) if name == "sqrt"));
                    assert_eq!(args.len(), 1);
                }
                other => panic!("Expected Call, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_class_with_operator_overload_all() {
        let src = r#"class Number {
    double value;

    fn operator+(self, other: Number): Number {
        return new Number();
    }

    fn operator-(self, other: Number): Number {
        return new Number();
    }

    fn operator*(self, other: Number): Number {
        return new Number();
    }

    fn operator/(self, other: Number): Number {
        return new Number();
    }
}"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Class(cd) => {
                assert_eq!(cd.name, "Number");
                let methods: Vec<_> = cd.members.iter().filter_map(|m| match m {
                    ast::ClassMember::Method(m) => Some(m.name.clone()),
                    _ => None,
                }).collect();
                assert!(methods.contains(&"operator+".to_string()),
                    "Expected operator+ method, found: {:?}", methods);
                assert!(methods.contains(&"operator-".to_string()),
                    "Expected operator- method, found: {:?}", methods);
                assert!(methods.contains(&"operator*".to_string()),
                    "Expected operator* method, found: {:?}", methods);
                assert!(methods.contains(&"operator/".to_string()),
                    "Expected operator/ method, found: {:?}", methods);
            }
            other => panic!("Expected Class, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Import glob test
    // -----------------------------------------------------------------------
    #[test]
    fn test_import_glob() {
        let src = r#"import tt::util::*;"#;
        let prog = parse_src(src).expect("parse should succeed");
        assert_eq!(prog.imports.len(), 1);
        assert_eq!(prog.imports[0].path, vec!["tt", "util"]);
        assert!(prog.imports[0].glob);
    }

    // -----------------------------------------------------------------------
    // Import multiple (selective) test
    // -----------------------------------------------------------------------
    #[test]
    fn test_import_multiple() {
        // Multiple imports from the same module using separate import statements
        let src = r#"import tt::math::Math;
import tt::math::NDArray;"#;
        let prog = parse_src(src).expect("parse should succeed");
        assert_eq!(prog.imports.len(), 2);
        assert_eq!(prog.imports[0].path, vec!["tt", "math", "Math"]);
        assert_eq!(prog.imports[1].path, vec!["tt", "math", "NDArray"]);
    }

    // -----------------------------------------------------------------------
    // Dot-notation import tests (syntax sugar for ::)
    // -----------------------------------------------------------------------
    #[test]
    fn test_import_dot_notation() {
        let src = r#"import tt.math.Math;"#;
        let prog = parse_src(src).expect("parse should succeed");
        assert_eq!(prog.imports.len(), 1);
        assert_eq!(prog.imports[0].path, vec!["tt", "math", "Math"]);
    }

    #[test]
    fn test_import_dot_notation_deep() {
        let src = r#"import tt.math.linalg.Matrix;"#;
        let prog = parse_src(src).expect("parse should succeed");
        assert_eq!(prog.imports.len(), 1);
        assert_eq!(prog.imports[0].path, vec!["tt", "math", "linalg", "Matrix"]);
    }

    #[test]
    fn test_import_dot_notation_glob() {
        let src = r#"import tt.util.*;"#;
        let prog = parse_src(src).expect("parse should succeed");
        assert_eq!(prog.imports.len(), 1);
        assert_eq!(prog.imports[0].path, vec!["tt", "util"]);
        assert!(prog.imports[0].glob);
    }

    #[test]
    fn test_import_mixed_notation() {
        // Both notations should work in the same file
        let src = r#"import tt::math::Math;
import tt.chem.Molecule;"#;
        let prog = parse_src(src).expect("parse should succeed");
        assert_eq!(prog.imports.len(), 2);
        assert_eq!(prog.imports[0].path, vec!["tt", "math", "Math"]);
        assert_eq!(prog.imports[1].path, vec!["tt", "chem", "Molecule"]);
    }

    #[test]
    fn test_import_dot_notation_does_not_affect_expressions() {
        // Ensure that `a.b` in expressions is still parsed as member access, not import path
        let src = r#"fn f(): void { let x = obj.field; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::VarDecl(vd) => match &vd.init {
                    Some(ast::Expr::MemberAccess(obj, name, _)) => {
                        assert!(matches!(obj.as_ref(), ast::Expr::Identifier(ident, _) if ident == "obj"));
                        assert_eq!(name, "field");
                    }
                    other => panic!("Expected MemberAccess, got {:?}", other),
                },
                other => panic!("Expected VarDecl, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }
