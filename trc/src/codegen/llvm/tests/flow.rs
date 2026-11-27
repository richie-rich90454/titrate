use super::{analyzer, compile_program_to_ir, compile_to_ir, lexer, parser, Context, Declaration, LlvmBackend};

    // ---- Function tests (Task 1.5) ----

    #[test]
    fn function_declaration() {
        let ir = compile_program_to_ir(
            r#"
            fn helper(): void {
                io::println("hello from helper");
            }
            public fn main(): void {
                helper();
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("@helper"),
            "expected helper function definition, got:\n{}",
            ir
        );
    }

    #[test]
    fn function_call_from_main() {
        let ir = compile_program_to_ir(
            r#"
            fn greet(): void {
                io::println("hi");
            }
            public fn main(): void {
                greet();
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("call") && ir.contains("@greet"),
            "expected call to greet, got:\n{}",
            ir
        );
    }

    #[test]
    fn function_with_return_value() {
        let ir = compile_program_to_ir(
            r#"
            fn answer(): int {
                return 42;
            }
            public fn main(): void {
                let x: int = answer();
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("@answer"),
            "expected answer function, got:\n{}",
            ir
        );
        assert!(
            ir.contains("call") && ir.contains("@answer"),
            "expected call to answer, got:\n{}",
            ir
        );
    }

    #[test]
    fn function_with_parameters() {
        let ir = compile_program_to_ir(
            r#"
            fn add(a: int, b: int): int {
                return a + b;
            }
            public fn main(): void {
                let x: int = add(3, 4);
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(ir.contains("@add"), "expected add function, got:\n{}", ir);
        assert!(
            ir.contains("call") && ir.contains("@add"),
            "expected call to add, got:\n{}",
            ir
        );
    }

    #[test]
    fn recursive_function() {
        let ir = compile_program_to_ir(
            r#"
            fn fib(n: int): int {
                if (n <= 1) {
                    return n;
                }
                return fib(n - 1) + fib(n - 2);
            }
            public fn main(): void {
                let result: int = fib(10);
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(ir.contains("@fib"), "expected fib function, got:\n{}", ir);
        // The function should call itself (recursive call): at least 3
        // occurrences of @fib (1 define + 2 calls).
        let count = ir.matches("@fib").count();
        assert!(
            count >= 3,
            "expected at least 3 occurrences of @fib (define + 2 calls), got {}:\n{}",
            count,
            ir
        );
    }

    #[test]
    fn void_function_called_as_statement() {
        let ir = compile_program_to_ir(
            r#"
            fn printIt(x: int): void {
                io::println(x);
            }
            public fn main(): void {
                printIt(42);
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("@printIt"),
            "expected printIt function, got:\n{}",
            ir
        );
        assert!(
            ir.contains("call") && ir.contains("@printIt"),
            "expected call to printIt, got:\n{}",
            ir
        );
    }

    #[test]
    fn function_returning_double() {
        let ir = compile_program_to_ir(
            r#"
            fn pi(): double {
                return 3.14;
            }
            public fn main(): void {
                let p: double = pi();
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(ir.contains("@pi"), "expected pi function, got:\n{}", ir);
        assert!(
            ir.contains("call") && ir.contains("@pi"),
            "expected call to pi, got:\n{}",
            ir
        );
    }

    #[test]
    fn function_with_multiple_returns() {
        let ir = compile_program_to_ir(
            r#"
            fn classify(n: int): int {
                if (n > 0) {
                    return 1;
                }
                if (n < 0) {
                    return -1;
                }
                return 0;
            }
            public fn main(): void {
                let r: int = classify(5);
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("@classify"),
            "expected classify function, got:\n{}",
            ir
        );
        assert!(
            ir.contains("call") && ir.contains("@classify"),
            "expected call to classify, got:\n{}",
            ir
        );
    }

    // ---- Ownership / borrows / regions tests (Task 1.6) ----

    #[test]
    fn unsafe_block_compiles() {
        // An unsafe block should compile its body like a normal block.
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                unsafe {
                    let x: int = 42;
                    io::println(x);
                }
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("store i32 42"),
            "expected store i32 42 in unsafe block, got:\n{}",
            ir
        );
    }

    #[test]
    fn unsafe_block_with_io() {
        // An unsafe block can call io::println.
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                unsafe {
                    io::println("hello from unsafe");
                }
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("titrate_println"),
            "expected titrate_println call in unsafe block, got:\n{}",
            ir
        );
    }

    #[test]
    fn region_block_compiles_as_unsafe() {
        // `region name { ... }` is parsed as an UnsafeBlock.
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                region r {
                    let x: int = 10;
                    io::println(x);
                }
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("store i32 10"),
            "expected store i32 10 in region block, got:\n{}",
            ir
        );
    }

    #[test]
    fn borrow_of_int_variable() {
        // `&x` should produce a pointer to x's alloca.
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let x: int = 42;
                let r: &int = &x;
            }
        "#,
        )
        .expect("IR should succeed");
        // The borrow should not crash and should produce some pointer value.
        // We just check that the IR contains an alloca for x.
        assert!(
            ir.contains("alloca i32"),
            "expected alloca i32 for x, got:\n{}",
            ir
        );
    }

    #[test]
    fn mutable_borrow_of_double_variable() {
        // `&mut x` should produce a pointer to x's alloca.
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                var y: double = 3.14;
                let r: &mut double = &mut y;
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("alloca double"),
            "expected alloca double for y, got:\n{}",
            ir
        );
    }

    #[test]
    fn owned_deref_loads_value() {
        // `*x` should load the value from the Owned<T> pointer.
        // We use an identifier deref; the pointer is stored in an alloca.
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let x: Owned<int> = null;
                let v: int = *x;
            }
        "#,
        )
        .expect("IR should succeed");
        // The deref should produce a load instruction.
        assert!(
            ir.contains("load"),
            "expected load instruction for owned deref, got:\n{}",
            ir
        );
    }

    #[test]
    fn with_statement_compiles() {
        // `with (expr) { body }` should compile the body.
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let r: int = 42;
                with (r) {
                    io::println("inside with");
                }
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("titrate_println"),
            "expected titrate_println in with body, got:\n{}",
            ir
        );
    }

    #[test]
    fn with_let_binds_variable() {
        // `with (let f: T = expr) { body }` should bind f.
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                with (let f: int = 100) {
                    io::println(f);
                }
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("titrate_println_int"),
            "expected titrate_println_int for f, got:\n{}",
            ir
        );
    }

    #[test]
    fn nested_unsafe_blocks() {
        // Nested unsafe blocks should compile.
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                unsafe {
                    let x: int = 1;
                    unsafe {
                        let y: int = 2;
                        io::println(x + y);
                    }
                }
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("titrate_println_int"),
            "expected titrate_println_int, got:\n{}",
            ir
        );
    }

    #[test]
    fn titrate_malloc_declared() {
        // The titrate_malloc function should be declared as an external.
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let x: int = 1;
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("titrate_malloc"),
            "expected titrate_malloc declaration, got:\n{}",
            ir
        );
    }

    /// Compiling a class method must restore the caller's insert block.
    /// Otherwise methods emit into a caller function, leaving blocks with
    /// dangling cross-function uses that abort LLVM during module teardown.
    #[test]
    fn deleted_block_has_no_remaining_uses() {
        let source = r#"
            public class Leaf {
                public var value: int;
                public fn init(v: int) { this.value = v; }
                public fn get(): int { return this.value; }
            }
            public class Box {
                public fn make(v: int): Leaf { return new Leaf(v); }
            }
            public fn main(): void {
                let b = new Box();
                let l = b.make(1);
            }
        "#;
        let tokens = lexer::tokenize(source).expect("tokenize");
        let ast = parser::parse(tokens).expect("parse");
        let typed = analyzer::analyze(&ast).expect("analyze");
        let context = Context::create();
        let mut backend = LlvmBackend::new(&context, "test");
        backend.declare_natives();
        let host = backend.module.add_function(
            "host",
            context.void_type().fn_type(&[], false),
            None,
        );
        let host_block = context.append_basic_block(host, "host.entry");
        backend.builder.position_at_end(host_block);
        for decl in &typed.declarations {
            if let Declaration::Class(class_decl) = decl {
                backend.compile_class_decl(class_decl).expect("compile class");
            }
        }
        assert_eq!(
            backend.builder.get_insert_block(),
            Some(host_block),
            "class method emission must restore the caller's block"
        );
        backend.builder.position_at_end(host_block);
        backend.builder.build_return(None).expect("host return");
        let ir = backend.module.print_to_string().to_string();
        assert!(backend.module.verify().is_ok(), "IR must verify:\n{}", ir);
    }

    /// A closure body must not reuse its enclosing function's tail-call
    /// trampoline: its block and parameter slots belong to another function,
    /// which makes `module.verify()` hit LLVM's dominator-tree assertion.
    #[test]
    fn domtree_query_uses_correct_function() {
        let source = r#"
            public fn rec(n: int): int {
                if (n <= 0) { return 0; }
                let step = fn(t: int): int { return rec(t - 1); };
                return step(n - 1);
            }
            public fn main(): void {
                let v = rec(3);
            }
        "#;
        let ir = compile_program_to_ir(source).expect("IR should succeed");
        let start = ir
            .find("define internal i32 @_closure_0")
            .expect("closure trampoline");
        let rest = &ir[start..];
        let end = rest.find("\ndefine ").unwrap_or(rest.len());
        let closure_ir = &rest[..end];
        assert!(
            !closure_ir.contains("tco.body"),
            "closure must not branch to the enclosing function's TCO block:\n{}",
            closure_ir
        );
    }

    /// A `throw` inside a closure must not branch to the enclosing
    /// function's catch block; that block belongs to a different function.
    #[test]
    fn closure_catch_block_targets_own_function() {
        let source = r#"
            public fn guard(): void {
                try {
                    let fail = fn(): void { throw "boom"; };
                    fail();
                } catch (e: string) {
                    io::println(e);
                }
            }
            public fn main(): void { guard(); }
        "#;
        let ir = compile_program_to_ir(source).expect("IR should succeed");
        let mut sections: Vec<(String, Vec<&str>)> = Vec::new();
        for line in ir.lines() {
            if let Some(rest) = line.strip_prefix("define ") {
                if let Some(at) = rest.find('@') {
                    let name = rest[at + 1..]
                        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '.')
                        .next()
                        .unwrap_or("");
                    sections.push((name.to_string(), Vec::new()));
                }
            }
            if let Some((_, lines)) = sections.last_mut() {
                lines.push(line);
            }
        }
        for (name, lines) in &sections {
            let defs: std::collections::HashSet<&str> = lines
                .iter()
                .filter_map(|l| l.split_once(':').map(|(label, _)| label))
                .filter(|l| l.starts_with(|c: char| c.is_ascii_alphanumeric() || c == '_' || c == '.'))
                .collect();
            for line in lines {
                if let Some(target) = line.split("br label %").nth(1) {
                    let target = target.split_whitespace().next().unwrap_or("");
                    assert!(
                        defs.contains(target),
                        "function {} branches to undeclared block {}:\n{}",
                        name,
                        target,
                        lines.join("\n")
                    );
                }
            }
        }
    }
