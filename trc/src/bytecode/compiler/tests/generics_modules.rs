use super::su;
use super::super::{resolver, Compiler, Symbol};
use std::collections::HashMap;
use crate::ast;

    // -- test_mangle_name --------------------------------------------------------

    #[test]
    fn test_mangle_name() {
        assert_eq!(Compiler::mangle_name("Box", &[]), "Box");
        assert_eq!(
            Compiler::mangle_name("Box", &[ast::Type::simple("int")]),
            "Box__int"
        );
        assert_eq!(
            Compiler::mangle_name("HashMap", &[ast::Type::simple("string"), ast::Type::simple("int")]),
            "HashMap__string__int"
        );
        assert_eq!(
            Compiler::mangle_name("Box", &[ast::Type::generic("Owned", vec![ast::Type::simple("int")])]),
            "Box__Owned__int"
        );
    }

    // -- test_substitute_type ----------------------------------------------------

    #[test]
    fn test_substitute_type() {
        let mut type_args = HashMap::new();
        type_args.insert("T".to_string(), ast::Type::simple("int"));

        // Simple type parameter substitution: T → int
        let ty = ast::Type::simple("T");
        let result = Compiler::substitute_type(&ty, &type_args);
        assert_eq!(result, ast::Type::simple("int"));

        // Non-parameterized type is unchanged: string → string
        let ty = ast::Type::simple("string");
        let result = Compiler::substitute_type(&ty, &type_args);
        assert_eq!(result, ast::Type::simple("string"));

        // Nested type parameter substitution: Owned<T> → Owned<int>
        let ty = ast::Type::generic("Owned", vec![ast::Type::simple("T")]);
        let result = Compiler::substitute_type(&ty, &type_args);
        assert_eq!(result, ast::Type::generic("Owned", vec![ast::Type::simple("int")]));

        // Multiple type parameters
        let mut type_args2 = HashMap::new();
        type_args2.insert("K".to_string(), ast::Type::simple("string"));
        type_args2.insert("V".to_string(), ast::Type::simple("int"));

        let ty = ast::Type::generic("Map", vec![ast::Type::simple("K"), ast::Type::simple("V")]);
        let result = Compiler::substitute_type(&ty, &type_args2);
        assert_eq!(result, ast::Type::generic("Map", vec![ast::Type::simple("string"), ast::Type::simple("int")]));
    }

    // -- test_instantiate_generic_function ----------------------------------------

    #[test]
    fn test_instantiate_generic_function() {
        let generic_fn = ast::FnDecl {
            access: ast::Access::Public,
            name: "id".to_string(),
            type_params: vec![ast::TypeParam { name: "T".to_string(), constraint: None }],
            params: vec![ast::Param {
                name: "x".to_string(),
                typ: ast::Type::simple("T"),
            }],
            return_type: Some(ast::Type::simple("T")),
            body: vec![ast::Stmt::Return(Some(ast::Expr::Identifier("x".to_string(), su())))],
            sugar: false,
            where_clause: vec![],
            span: su(),
        };

        let caller_fn = ast::FnDecl {
            access: ast::Access::Public,
            name: "caller".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: None,
            body: vec![ast::Stmt::Expr(ast::Expr::Call(
                Box::new(ast::Expr::Identifier("id__int_a1".to_string(), su())),
                vec![ast::Expr::Literal(ast::Literal::Int(42), su())],
                su(),
            ))],
            sugar: false,
            where_clause: vec![],
            span: su(),
        };

        let program = ast::Program {
            imports: vec![],
            declarations: vec![
                ast::Declaration::Function(generic_fn),
                ast::Declaration::Function(caller_fn),
            ],
        };

        let mut compiler = Compiler::new();

        // Register declarations first (first pass).
        for decl in &program.declarations {
            if let ast::Declaration::Function(fn_decl) = decl {
                compiler.register_function(fn_decl);
            }
        }

        // Now instantiate the generic function.
        let fn_idx = compiler.instantiate_generic_function("id", &[ast::Type::simple("int")], 1)
            .expect("instantiation should succeed");
        assert!(fn_idx > 0, "instantiated function should have a valid index");

        // Verify the mangled name is in function_map.
        assert!(
            compiler.function_map.contains_key("id__int_a1"),
            "function_map should contain mangled name 'id__int_a1'"
        );

        // Verify mono_cache.
        assert!(
            compiler.mono_cache.contains_key("id__int_a1"),
            "mono_cache should contain 'id__int_a1'"
        );

        // Second instantiation should return the same index (cache hit).
        let fn_idx2 = compiler.instantiate_generic_function("id", &[ast::Type::simple("int")], 1)
            .expect("second instantiation should succeed");
        assert_eq!(fn_idx, fn_idx2, "cached instantiation should return same index");

        // Now compile the full program.
        let compiled = compiler.compile(&program).expect("compilation should succeed");

        // The specialized function should exist.
        let found = compiled.functions.iter().any(|f| f.name == "id__int_a1");
        assert!(found, "compiled program should contain function 'id__int_a1'");
    }

    // -- test_instantiate_generic_class -------------------------------------------

    #[test]
    fn test_instantiate_generic_class() {
        let generic_class = ast::ClassDecl {
            name: "Box".to_string(),
            type_params: vec![ast::TypeParam { name: "T".to_string(), constraint: None }],
            parent: None,
            ifaces: vec![],
            members: vec![
                ast::ClassMember::Field(ast::FieldDecl {
                    access: ast::Access::Public,
                    name: "value".to_string(),
                    typ: ast::Type::simple("T"),
                    init: None,
                    span: su(),
                }),
                ast::ClassMember::Method(ast::MethodDecl {
                    access: ast::Access::Public,
                    name: "getValue".to_string(),
                    type_params: vec![],
                    params: vec![],
                    return_type: Some(ast::Type::simple("T")),
                    body: vec![ast::Stmt::Return(Some(ast::Expr::MemberAccess(
                        Box::new(ast::Expr::This(su())),
                        "value".to_string(),
                        su(),
                    )))],
                    where_clause: vec![],
                    span: su(),
                }),
            ],
            span: su(),
        };

        let fn_decl = ast::FnDecl {
            access: ast::Access::Public,
            name: "make_box".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: None,
            body: vec![ast::Stmt::Expr(ast::Expr::New(
                ast::Type::generic("Box", vec![ast::Type::simple("int")]),
                vec![],
                su(),
            ))],
            sugar: false,
            where_clause: vec![],
            span: su(),
        };

        let program = ast::Program {
            imports: vec![],
            declarations: vec![
                ast::Declaration::Class(generic_class),
                ast::Declaration::Function(fn_decl),
            ],
        };

        let mut compiler = Compiler::new();
        let compiled = compiler.compile(&program).expect("compilation should succeed");

        // The specialized class should exist with mangled name.
        let found_class = compiled.classes.iter().any(|c| c.name == "Box__int");
        assert!(found_class, "compiled program should contain class 'Box__int'");

        // The specialized method should exist.
        let found_method = compiled.functions.iter().any(|f| f.name == "Box__int.getValue");
        assert!(found_method, "compiled program should contain method 'Box__int.getValue'");
    }

    // -- test_generic_builtin_class ----------------------------------------------

    #[test]
    fn test_generic_builtin_class() {
        let fn_decl = ast::FnDecl {
            access: ast::Access::Public,
            name: "test_builtin".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: None,
            body: vec![
                ast::Stmt::Expr(ast::Expr::New(
                    ast::Type::generic("ArrayList", vec![ast::Type::simple("int")]),
                    vec![],
                    su(),
                )),
                ast::Stmt::Expr(ast::Expr::New(
                    ast::Type::generic("HashMap", vec![ast::Type::simple("string"), ast::Type::simple("int")]),
                    vec![],
                    su(),
                )),
            ],
            sugar: false,
            where_clause: vec![],
            span: su(),
        };

        let program = ast::Program {
            imports: vec![],
            declarations: vec![ast::Declaration::Function(fn_decl)],
        };

        let mut compiler = Compiler::new();
        let compiled = compiler.compile(&program).expect("compilation should succeed");

        // ArrayList<int> should create class "ArrayList__int"
        let found_al = compiled.classes.iter().any(|c| c.name == "ArrayList__int");
        assert!(found_al, "compiled program should contain class 'ArrayList__int'");

        // HashMap<string, int> should create class "HashMap__string__int"
        let found_hm = compiled.classes.iter().any(|c| c.name == "HashMap__string__int");
        assert!(found_hm, "compiled program should contain class 'HashMap__string__int'");
    }

    // -- test_generic_function_auto_instantiate_without_type_args ---------------------

    #[test]
    fn test_generic_function_auto_instantiate_without_type_args() {
        let generic_fn = ast::FnDecl {
            access: ast::Access::Public,
            name: "id".to_string(),
            type_params: vec![ast::TypeParam { name: "T".to_string(), constraint: None }],
            params: vec![ast::Param {
                name: "x".to_string(),
                typ: ast::Type::simple("T"),
            }],
            return_type: Some(ast::Type::simple("T")),
            body: vec![ast::Stmt::Return(Some(ast::Expr::Identifier("x".to_string(), su())))],
            sugar: false,
            where_clause: vec![],
            span: su(),
        };

        // Calling id(42) without type arguments should auto-instantiate with Variant.
        let caller_fn = ast::FnDecl {
            access: ast::Access::Public,
            name: "caller".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: None,
            body: vec![ast::Stmt::Expr(ast::Expr::Call(
                Box::new(ast::Expr::Identifier("id".to_string(), su())),
                vec![ast::Expr::Literal(ast::Literal::Int(42), su())],
                su(),
            ))],
            sugar: false,
            where_clause: vec![],
            span: su(),
        };

        let program = ast::Program {
            imports: vec![],
            declarations: vec![
                ast::Declaration::Function(generic_fn),
                ast::Declaration::Function(caller_fn),
            ],
        };

        let mut compiler = Compiler::new();
        let result = compiler.compile(&program);
        assert!(result.is_ok(), "calling generic function without type args should auto-instantiate");
    }

    // -- Module system tests -----------------------------------------------------

    // -- test_module_resolve_path ------------------------------------------------

    #[test]
    fn test_module_resolve_path() {
        let mut resolver = resolver::ModuleResolver::new();
        let root = std::env::temp_dir();

        // Create a temporary directory structure: root/tt/lang/Integer.tr
        let dir = root.join("tt").join("lang");
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("Integer.tr");
        std::fs::write(&file_path, "fn main() {}").unwrap();

        let import_path: Vec<String> = vec!["tt".to_string(), "lang".to_string(), "Integer".to_string()];
        let resolved = resolver.resolve(&import_path, &root).expect("should resolve");
        assert_eq!(resolved, file_path);

        // Cleanup
        std::fs::remove_dir_all(root.join("tt")).ok();
    }

    // -- test_module_resolve_cached ----------------------------------------------

    #[test]
    fn test_module_resolve_cached() {
        let mut resolver = resolver::ModuleResolver::new();
        let root = std::env::temp_dir();

        let dir = root.join("cache_test").join("sub");
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("Module.tr");
        std::fs::write(&file_path, "fn main() {}").unwrap();

        let import_path: Vec<String> = vec!["cache_test".to_string(), "sub".to_string(), "Module".to_string()];

        // First resolve: file system lookup.
        let resolved1 = resolver.resolve(&import_path, &root).expect("should resolve");
        // Second resolve: should come from cache even if file is deleted.
        std::fs::remove_file(&file_path).unwrap();
        let resolved2 = resolver.resolve(&import_path, &root).expect("should resolve from cache");
        assert_eq!(resolved1, resolved2);

        // Cleanup
        std::fs::remove_dir_all(root.join("cache_test")).ok();
    }

    // -- test_module_resolve_not_found -------------------------------------------

    #[test]
    fn test_module_resolve_not_found() {
        let mut resolver = resolver::ModuleResolver::new();
        let root = std::env::temp_dir();

        let import_path: Vec<String> = vec!["nonexistent".to_string(), "Module".to_string()];
        let result = resolver.resolve(&import_path, &root);
        assert!(result.is_err(), "should fail for nonexistent module");
        let err = result.err().unwrap();
        assert!(err.contains("Cannot resolve module"), "error should mention resolution failure");
    }

    // -- test_module_resolve_glob ------------------------------------------------

    #[test]
    fn test_module_resolve_glob() {
        let mut resolver = resolver::ModuleResolver::new();
        let root = std::env::temp_dir();

        let dir = root.join("glob_test").join("io");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Reader.tr"), "fn main() {}").unwrap();
        std::fs::write(dir.join("Writer.tr"), "fn main() {}").unwrap();

        let import_path: Vec<String> = vec!["glob_test".to_string(), "io".to_string()];
        let files = resolver.resolve_glob(&import_path, &root).expect("should resolve glob");
        assert_eq!(files.len(), 2);
        assert!(files[0].file_name().unwrap().to_str().unwrap().contains("Reader"));
        assert!(files[1].file_name().unwrap().to_str().unwrap().contains("Writer"));

        // Cleanup
        std::fs::remove_dir_all(root.join("glob_test")).ok();
    }

    // -- test_symbol_table -------------------------------------------------------

    #[test]
    fn test_symbol_table() {
        let mut compiler = Compiler::new();

        // Manually insert symbols into the symbol table.
        compiler.symbol_table.insert("imported_fn".to_string(), Symbol::Function(5));
        compiler.symbol_table.insert("MyClass".to_string(), Symbol::Class(2));
        compiler.symbol_table.insert("Color".to_string(), Symbol::Enum(0));

        assert!(matches!(compiler.symbol_table.get("imported_fn"), Some(Symbol::Function(5))));
        assert!(matches!(compiler.symbol_table.get("MyClass"), Some(Symbol::Class(2))));
        assert!(matches!(compiler.symbol_table.get("Color"), Some(Symbol::Enum(0))));
        assert!(!compiler.symbol_table.contains_key("nonexistent"));
    }

    // -- test_module_load --------------------------------------------------------

    #[test]
    fn test_module_load() {
        let root = std::env::temp_dir();
        let dir = root.join("load_test");
        std::fs::create_dir_all(&dir).unwrap();

        let source = r#"fn greet() { 1 + 2; }"#;
        let file_path = dir.join("Greeter.tr");
        std::fs::write(&file_path, source).unwrap();

        let mut compiler = Compiler::new();
        let idx = compiler.load_module(&file_path, "load_test.Greeter").expect("should load module");
        assert_eq!(idx, 0);
        assert_eq!(compiler.modules.len(), 1);
        assert_eq!(compiler.modules[0].name, "load_test.Greeter");
        assert!(compiler.modules[0].program.is_some());
        assert!(!compiler.modules[0].compiled);

        // Loading again should return the same index (cached).
        let idx2 = compiler.load_module(&file_path, "load_test.Greeter").expect("should load cached module");
        assert_eq!(idx, idx2);
        assert_eq!(compiler.modules.len(), 1);

        // Cleanup
        std::fs::remove_dir_all(&dir).ok();
    }

    // -- test_module_compile_with_modules ----------------------------------------

    #[test]
    fn test_module_compile_with_modules() {
        let root = std::env::temp_dir();
        let dir = root.join("compile_mod_test");
        std::fs::create_dir_all(&dir).unwrap();

        // Create a library module: Helper.tr with a public function.
        let helper_source = r#"public fn add(a: long, b: long): long { return a + b; }"#;
        std::fs::write(dir.join("Helper.tr"), helper_source).unwrap();

        // Create a main program that imports Helper.
        let main_program = ast::Program {
            imports: vec![ast::Import {
                path: vec!["compile_mod_test".to_string(), "Helper".to_string()],
                glob: false,
                span: su(),
            }],
            declarations: vec![ast::Declaration::Function(ast::FnDecl {
                access: ast::Access::Public,
                name: "main".to_string(),
                type_params: vec![],
                params: vec![],
                return_type: None,
                body: vec![ast::Stmt::Expr(ast::Expr::Call(
                    Box::new(ast::Expr::Identifier("add".to_string(), su())),
                    vec![
                        ast::Expr::Literal(ast::Literal::Int(1), su()),
                        ast::Expr::Literal(ast::Literal::Int(2), su()),
                    ],
                    su(),
                ))],
                sugar: false,
                where_clause: vec![],
                span: su(),
            })],
        };

        let mut compiler = Compiler::new();
        let result = compiler.compile_with_modules(&main_program, &root);
        assert!(result.is_ok(), "compile_with_modules should succeed: {:?}", result.err());

        let compiled = result.unwrap();
        // Should have the Helper.add function (mangled as "compile_mod_test.Helper.add") and the main function.
        let has_helper_add = compiled.functions.iter().any(|f| f.name == "compile_mod_test.Helper.add");
        assert!(has_helper_add, "compiled program should contain 'compile_mod_test.Helper.add' function");

        // Cleanup
        std::fs::remove_dir_all(&dir).ok();
    }

    // -- test_module_visibility --------------------------------------------------

    #[test]
    fn test_module_visibility() {
        let mut compiler = Compiler::new();

        // Create a module with a public and a private function.
        let module_program = ast::Program {
            imports: vec![],
            declarations: vec![
                ast::Declaration::Function(ast::FnDecl {
                    access: ast::Access::Public,
                    name: "visible_fn".to_string(),
                    type_params: vec![],
                    params: vec![],
                    return_type: None,
                    body: vec![],
                    sugar: false,
                    where_clause: vec![],
                    span: su(),
                }),
                ast::Declaration::Function(ast::FnDecl {
                    access: ast::Access::Private,
                    name: "hidden_fn".to_string(),
                    type_params: vec![],
                    params: vec![],
                    return_type: None,
                    body: vec![],
                    sugar: false,
                    where_clause: vec![],
                    span: su(),
                }),
            ],
        };

        // Register the module's declarations.
        compiler.register_module_declarations(&module_program, "mymod").unwrap();

        // The public function should be in function_map with mangled name.
        assert!(compiler.function_map.contains_key("mymod.visible_fn"),
            "public function should be registered with mangled name");

        // The private function should also be registered with mangled name
        // so that it can be called from within the same module.
        assert!(compiler.function_map.contains_key("mymod.hidden_fn"),
            "private function should be registered with mangled name for intra-module calls");
    }

    // -- test_module_circular_import ---------------------------------------------

    #[test]
    fn test_module_circular_import_detection() {
        let root = std::env::temp_dir();
        let dir = root.join("circular_test");
        std::fs::create_dir_all(&dir).unwrap();

        // Create two modules that import each other.
        let a_source = r#"import circular_test::B; fn a() {}"#;
        let b_source = r#"import circular_test::A; fn b() {}"#;

        std::fs::write(dir.join("A.tr"), a_source).unwrap();
        std::fs::write(dir.join("B.tr"), b_source).unwrap();

        let main_program = ast::Program {
            imports: vec![ast::Import {
                path: vec!["circular_test".to_string(), "A".to_string()],
                glob: false,
                span: su(),
            }],
            declarations: vec![],
        };

        let mut compiler = Compiler::new();
        let result = compiler.compile_with_modules(&main_program, &root);
        // Circular imports are now handled gracefully by breaking the cycle.
        // The compiler should succeed because declarations are registered
        // in a first pass before compiling function bodies.
        assert!(result.is_ok(), "circular import should be handled gracefully: {:?}", result.err());

        // Cleanup
        std::fs::remove_dir_all(&dir).ok();
    }

    // -- test_module_glob_import -------------------------------------------------

    #[test]
    fn test_module_glob_import() {
        let root = std::env::temp_dir();
        let dir = root.join("glob_import_test").join("utils");
        std::fs::create_dir_all(&dir).unwrap();

        // Create two utility modules.
        std::fs::write(dir.join("Math.tr"), "public fn sqrt(x: double): double { return x; }").unwrap();
        std::fs::write(dir.join("Str.tr"), "public fn trim(s: string): string { return s; }").unwrap();

        let main_program = ast::Program {
            imports: vec![ast::Import {
                path: vec!["glob_import_test".to_string(), "utils".to_string()],
                glob: true,
                span: su(),
            }],
            declarations: vec![],
        };

        let mut compiler = Compiler::new();
        let result = compiler.compile_with_modules(&main_program, &root);
        assert!(result.is_ok(), "glob import should succeed: {:?}", result.err());

        // Both modules should be loaded.
        assert!(compiler.modules.len() >= 2, "should have loaded at least 2 modules from glob");

        // Cleanup
        std::fs::remove_dir_all(root.join("glob_import_test")).ok();
    }

    // -- test_module_compile_imported_class ----------------------------------------

    #[test]
    fn test_module_compile_imported_class() {
        let root = std::env::temp_dir();
        let dir = root.join("class_import_test");
        std::fs::create_dir_all(&dir).unwrap();

        // Create a module with a public class.
        let helper_source = r#"public class Point { public long x; public long y; public fn getX(): long { return this.x; } }"#;
        std::fs::write(dir.join("Geometry.tr"), helper_source).unwrap();

        let main_program = ast::Program {
            imports: vec![ast::Import {
                path: vec!["class_import_test".to_string(), "Geometry".to_string()],
                glob: false,
                span: su(),
            }],
            declarations: vec![],
        };

        let mut compiler = Compiler::new();
        let result = compiler.compile_with_modules(&main_program, &root);
        assert!(result.is_ok(), "importing a class module should succeed: {:?}", result.err());

        let compiled = result.unwrap();
        let has_point = compiled.classes.iter().any(|c| c.name == "class_import_test.Geometry.Point");
        assert!(has_point, "compiled program should contain 'class_import_test.Geometry.Point' class");

        // Cleanup
        std::fs::remove_dir_all(&dir).ok();
    }
