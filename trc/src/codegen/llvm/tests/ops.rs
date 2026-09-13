use super::compile_to_ir;

    // ---- Operator tests (Task 1.3) ----

    #[test]
    fn int_addition() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let a: int = 1;
                let b: int = 2;
                a + b;
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(ir.contains("add"), "expected add instruction, got:\n{}", ir);
    }

    #[test]
    fn int_subtraction() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let a: int = 5;
                let b: int = 3;
                a - b;
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(ir.contains("sub"), "expected sub instruction, got:\n{}", ir);
    }

    #[test]
    fn int_multiplication() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let a: int = 3;
                let b: int = 4;
                a * b;
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(ir.contains("mul"), "expected mul instruction, got:\n{}", ir);
    }

    #[test]
    fn int_division() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let a: int = 10;
                let b: int = 2;
                a / b;
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("sdiv"),
            "expected sdiv instruction, got:\n{}",
            ir
        );
    }

    #[test]
    fn int_modulo() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let a: int = 10;
                let b: int = 3;
                a % b;
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("srem"),
            "expected srem instruction, got:\n{}",
            ir
        );
    }

    #[test]
    fn float_addition() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let a: double = 1.0;
                let b: double = 2.0;
                a + b;
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("fadd"),
            "expected fadd instruction, got:\n{}",
            ir
        );
    }

    #[test]
    fn float_subtraction() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let a: double = 5.0;
                let b: double = 3.0;
                a - b;
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("fsub"),
            "expected fsub instruction, got:\n{}",
            ir
        );
    }

    #[test]
    fn float_multiplication() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let a: double = 3.0;
                let b: double = 4.0;
                a * b;
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("fmul"),
            "expected fmul instruction, got:\n{}",
            ir
        );
    }

    #[test]
    fn float_division() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let a: double = 10.0;
                let b: double = 2.0;
                a / b;
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("fdiv"),
            "expected fdiv instruction, got:\n{}",
            ir
        );
    }

    #[test]
    fn int_equal_comparison() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let a: int = 1;
                let b: int = 2;
                a == b;
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(ir.contains("icmp eq"), "expected icmp eq, got:\n{}", ir);
    }

    #[test]
    fn int_not_equal_comparison() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let a: int = 1;
                let b: int = 2;
                a != b;
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(ir.contains("icmp ne"), "expected icmp ne, got:\n{}", ir);
    }

    #[test]
    fn int_less_than() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let a: int = 1;
                let b: int = 2;
                a < b;
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(ir.contains("icmp slt"), "expected icmp slt, got:\n{}", ir);
    }

    #[test]
    fn int_greater_than() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let a: int = 1;
                let b: int = 2;
                a > b;
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(ir.contains("icmp sgt"), "expected icmp sgt, got:\n{}", ir);
    }

    #[test]
    fn int_less_equal() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let a: int = 1;
                let b: int = 2;
                a <= b;
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(ir.contains("icmp sle"), "expected icmp sle, got:\n{}", ir);
    }

    #[test]
    fn int_greater_equal() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let a: int = 1;
                let b: int = 2;
                a >= b;
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(ir.contains("icmp sge"), "expected icmp sge, got:\n{}", ir);
    }

    #[test]
    fn logical_and_short_circuits() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let a: bool = true;
                let b: bool = false;
                a && b;
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("phi"),
            "expected phi for short-circuit &&, got:\n{}",
            ir
        );
    }

    #[test]
    fn logical_or_short_circuits() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let a: bool = true;
                let b: bool = false;
                a || b;
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("phi"),
            "expected phi for short-circuit ||, got:\n{}",
            ir
        );
    }

    #[test]
    fn logical_not() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let a: bool = true;
                !a;
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(ir.contains("xor"), "expected xor for !, got:\n{}", ir);
    }

    #[test]
    fn bitwise_and() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let a: int = 255;
                let b: int = 15;
                a & b;
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(ir.contains("and"), "expected and instruction, got:\n{}", ir);
    }

    #[test]
    fn bitwise_or() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let a: int = 240;
                let b: int = 15;
                a | b;
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(ir.contains("or"), "expected or instruction, got:\n{}", ir);
    }

    #[test]
    fn bitwise_xor() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let a: int = 255;
                let b: int = 15;
                a ^ b;
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(ir.contains("xor"), "expected xor instruction, got:\n{}", ir);
    }

    #[test]
    fn bitwise_not() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let a: int = 255;
                ~a;
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(ir.contains("xor"), "expected xor for ~, got:\n{}", ir);
    }

    #[test]
    fn bitwise_left_shift() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let a: int = 1;
                let b: int = 4;
                a << b;
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(ir.contains("shl"), "expected shl instruction, got:\n{}", ir);
    }

    #[test]
    fn bitwise_right_shift() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let a: int = 256;
                let b: int = 2;
                a >> b;
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("ashr"),
            "expected ashr instruction, got:\n{}",
            ir
        );
    }

    #[test]
    fn unary_negation_int() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let a: int = 5;
                -a;
            }
        "#,
        )
        .expect("IR should succeed");
        // -a is compiled as 0 - a
        assert!(
            ir.contains("sub"),
            "expected sub for negation, got:\n{}",
            ir
        );
    }

    #[test]
    fn unary_negation_float() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let a: double = 5.0;
                -a;
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("fneg"),
            "expected fneg instruction, got:\n{}",
            ir
        );
    }

    // ---- Control flow tests (Task 1.4) ----

    #[test]
    fn if_statement() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let a: int = 1;
                if (a > 0) {
                    io::println("positive");
                }
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("if.then"),
            "expected if.then block, got:\n{}",
            ir
        );
        assert!(ir.contains("if.end"), "expected if.end block, got:\n{}", ir);
        assert!(
            ir.contains("br i1"),
            "expected conditional branch, got:\n{}",
            ir
        );
    }

    #[test]
    fn if_else_statement() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let a: int = 1;
                if (a > 0) {
                    io::println("positive");
                } else {
                    io::println("non-positive");
                }
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("if.then"),
            "expected if.then block, got:\n{}",
            ir
        );
        assert!(
            ir.contains("if.else"),
            "expected if.else block, got:\n{}",
            ir
        );
        assert!(ir.contains("if.end"), "expected if.end block, got:\n{}", ir);
    }

    #[test]
    fn while_loop() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                var i: int = 0;
                while (i < 10) {
                    i = i + 1;
                }
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("while.cond"),
            "expected while.cond block, got:\n{}",
            ir
        );
        assert!(
            ir.contains("while.body"),
            "expected while.body block, got:\n{}",
            ir
        );
        assert!(
            ir.contains("while.end"),
            "expected while.end block, got:\n{}",
            ir
        );
    }

    #[test]
    fn do_while_loop() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                var i: int = 0;
                do {
                    i = i + 1;
                } while (i < 10);
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("do.body"),
            "expected do.body block, got:\n{}",
            ir
        );
        assert!(
            ir.contains("do.cond"),
            "expected do.cond block, got:\n{}",
            ir
        );
        assert!(ir.contains("do.end"), "expected do.end block, got:\n{}", ir);
    }

    #[test]
    fn for_in_range_loop() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                for (i in 0..10) {
                    io::println(i);
                }
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("for.cond"),
            "expected for.cond block, got:\n{}",
            ir
        );
        assert!(
            ir.contains("for.body"),
            "expected for.body block, got:\n{}",
            ir
        );
        assert!(
            ir.contains("for.inc"),
            "expected for.inc block, got:\n{}",
            ir
        );
        assert!(
            ir.contains("for.end"),
            "expected for.end block, got:\n{}",
            ir
        );
    }

    #[test]
    fn for_in_inclusive_range_loop() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                for (i in 0..=5) {
                    io::println(i);
                }
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("for.cond"),
            "expected for.cond block, got:\n{}",
            ir
        );
        assert!(
            ir.contains("for.body"),
            "expected for.body block, got:\n{}",
            ir
        );
    }

    #[test]
    fn c_style_for_loop() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                for (var i = 0; i < 10; i++) {
                    io::println(i);
                }
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("cfor.cond"),
            "expected cfor.cond block, got:\n{}",
            ir
        );
        assert!(
            ir.contains("cfor.body"),
            "expected cfor.body block, got:\n{}",
            ir
        );
        assert!(
            ir.contains("cfor.inc"),
            "expected cfor.inc block, got:\n{}",
            ir
        );
        assert!(
            ir.contains("cfor.end"),
            "expected cfor.end block, got:\n{}",
            ir
        );
    }

    #[test]
    fn switch_statement() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let x: int = 1;
                switch (x) {
                    case 0 => io::println("zero");
                    case 1 => io::println("one");
                    case _ => io::println("other");
                }
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("switch"),
            "expected switch instruction, got:\n{}",
            ir
        );
        assert!(
            ir.contains("switch.end"),
            "expected switch.end block, got:\n{}",
            ir
        );
    }

    #[test]
    fn ternary_expression() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let a: int = 1;
                let b: int = a > 0 ? 1 : 0;
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("tern.then"),
            "expected tern.then block, got:\n{}",
            ir
        );
        assert!(
            ir.contains("tern.else"),
            "expected tern.else block, got:\n{}",
            ir
        );
        assert!(
            ir.contains("tern.end"),
            "expected tern.end block, got:\n{}",
            ir
        );
        assert!(ir.contains("phi"), "expected phi for ternary, got:\n{}", ir);
    }

    #[test]
    fn break_in_while_loop() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                var i: int = 0;
                while (i < 100) {
                    if (i == 5) {
                        break;
                    }
                    i = i + 1;
                }
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("while.end"),
            "expected while.end block for break, got:\n{}",
            ir
        );
    }

    #[test]
    fn continue_in_while_loop() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                var i: int = 0;
                while (i < 10) {
                    i = i + 1;
                    if (i == 5) {
                        continue;
                    }
                }
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("while.cond"),
            "expected while.cond block for continue, got:\n{}",
            ir
        );
    }

    #[test]
    fn nested_if_else() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let a: int = 5;
                if (a > 0) {
                    if (a > 10) {
                        io::println("big");
                    } else {
                        io::println("small");
                    }
                } else {
                    io::println("negative");
                }
            }
        "#,
        )
        .expect("IR should succeed");
        // Should have two if.then blocks (one for outer, one for inner)
        let count = ir.matches("if.then").count();
        assert!(
            count >= 2,
            "expected at least 2 if.then blocks, got {}:\n{}",
            count,
            ir
        );
    }
