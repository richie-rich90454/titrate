use super::{compile_program_to_ir, compile_to_ir};

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
