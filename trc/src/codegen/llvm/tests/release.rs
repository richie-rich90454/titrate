use super::{compile_program_to_ir, compile_program_to_ir_release};
use super::super::LlvmBackend;
use crate::ast::Declaration;
use inkwell::context::Context;
use inkwell::AddressSpace;

    // ---- Release-mode optimization tests (Task 4.1) ----

    #[test]
    fn release_mode_emits_alwaysinline_for_small_functions() {
        // A small internal function (< 10 statements) should get the
        // `alwaysinline` attribute when compiled in release mode.
        let ir = compile_program_to_ir_release(
            r#"
            fn helper(x: int): int {
                return x * 2;
            }
            public fn main(): void {
                let y: int = helper(21);
                io::println(y);
            }
        "#,
        )
        .expect("release IR should succeed");
        assert!(
            ir.contains("alwaysinline"),
            "expected alwaysinline attribute in release IR, got:\n{}",
            ir,
        );
    }

    #[test]
    fn release_mode_avoids_fastcc_for_small_functions() {
        // Small internal functions get `alwaysinline` but must stay on the
        // default calling convention. A `fastcc` + `alwaysinline` function
        // whose return value flows through the native-call marshalling alloca
        // makes LLVM's optimizer emit `store to poison` (undefined behavior),
        // so release binaries would crash.
        let ir = compile_program_to_ir_release(
            r#"
            fn tiny(a: int, b: int): int {
                return a + b;
            }
            public fn main(): void {
                let y: int = tiny(1, 2);
                io::println(y);
            }
        "#,
        )
        .expect("release IR should succeed");
        assert!(
            ir.contains("alwaysinline"),
            "expected alwaysinline in release IR, got:\n{}",
            ir,
        );
        assert!(
            !ir.contains("fastcc"),
            "expected no fastcc calling convention in release IR, got:\n{}",
            ir,
        );
    }

    #[test]
    fn release_mode_emits_loop_vectorize_metadata() {
        // A for-in range loop should get !llvm.loop metadata with
        // vectorization hints when compiled in release mode.
        let ir = compile_program_to_ir_release(
            r#"
            public fn main(): void {
                for (i in 0..100) {
                    io::println(i);
                }
            }
        "#,
        )
        .expect("release IR should succeed");
        assert!(
            ir.contains("llvm.loop"),
            "expected llvm.loop metadata in release IR, got:\n{}",
            ir,
        );
        assert!(
            ir.contains("llvm.loop.vectorize.enable"),
            "expected vectorize.enable hint, got:\n{}",
            ir,
        );
    }

    #[test]
    fn debug_mode_does_not_emit_optimization_hints() {
        // In debug mode (release_mode = false), no alwaysinline or
        // llvm.loop metadata should be emitted.
        let ir = compile_program_to_ir(
            r#"
            fn helper(x: int): int {
                return x * 2;
            }
            public fn main(): void {
                for (i in 0..10) {
                    let y: int = helper(i);
                    io::println(y);
                }
            }
        "#,
        )
        .expect("debug IR should succeed");
        assert!(
            !ir.contains("alwaysinline"),
            "debug IR should NOT contain alwaysinline, got:\n{}",
            ir,
        );
        assert!(
            !ir.contains("llvm.loop"),
            "debug IR should NOT contain llvm.loop metadata, got:\n{}",
            ir,
        );
    }

    #[test]
    fn memset_intrinsic_declared_for_class_allocation() {
        // The emit_memset_zero helper should declare and call the
        // llvm.memset.p0.i64 intrinsic. We test the helper directly
        // because full class instantiation requires more type inference
        // than the test harness provides.
        use crate::analyzer;
        use crate::lexer;
        use crate::parser;
        let source = r#"
            public fn main(): void {
                let x: int = 1;
            }
        "#;
        let tokens = lexer::tokenize(source).expect("tokenize");
        let ast = parser::parse(tokens).expect("parse");
        let typed_ast = analyzer::analyze(&ast).expect("analyze");
        let context = Context::create();
        let mut backend = LlvmBackend::new(&context, "test");
        backend.declare_natives();
        let main_decl = typed_ast
            .declarations
            .iter()
            .find_map(|d| match d {
                Declaration::Function(f) if f.name == "main" => Some(f),
                _ => None,
            })
            .expect("no main");
        backend.compile_main(main_decl).expect("compile main");

        // Now call emit_memset_zero from within a function body.
        let fn_type = context.void_type().fn_type(&[], false);
        let test_fn = backend.module.add_function("test_memset", fn_type, None);
        let entry = context.append_basic_block(test_fn, "entry");
        backend.builder.position_at_end(entry);
        let i8_ptr = context.ptr_type(AddressSpace::default());
        let dummy_ptr = i8_ptr.const_null();
        let size = context.i64_type().const_int(64, false);
        backend
            .emit_memset_zero(dummy_ptr, size)
            .expect("emit_memset_zero");

        let ir = backend.module.print_to_string().to_string();
        assert!(
            ir.contains("llvm.memset.p0.i64"),
            "expected llvm.memset.p0.i64 declaration/call, got:\n{}",
            ir,
        );
    }
