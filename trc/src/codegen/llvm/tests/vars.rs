use super::{compile_program_to_ir, compile_to_ir};

    #[test]
    fn generic_class_monomorphization() {
        // Box<int> and Box<string> specialize on demand; each gets a
        // mangled implementation callable from main.
        let ir = compile_program_to_ir(
            r#"
            public class Box<T> {
                public T val;
                public fn init(v: T) { this.val = v; }
                public fn get(): T { return this.val; }
            }
            public fn main(): void {
                let b = new Box<int>(42);
                io::println(b.get());
                let s = new Box<string>("hi");
                io::println(s.get());
            }
        "#,
        )
        .expect("generic class IR should succeed");
        assert!(
            ir.contains("Box__int_get"),
            "expected Box__int_get specialization, got:\n{}",
            ir
        );
        assert!(
            ir.contains("Box__string_get"),
            "expected Box__string_get specialization, got:\n{}",
            ir
        );
    }

    #[test]
    fn generic_function_deduction() {
        // ident(3, 4) deduces T=int and emits a mangled specialization.
        let ir = compile_program_to_ir(
            r#"
            public fn ident<T>(x: T): T { return x; }
            public fn main(): void {
                io::println(ident(41));
            }
        "#,
        )
        .expect("generic function IR should succeed");
        assert!(
            ir.contains("ident__int"),
            "expected ident__int specialization, got:\n{}",
            ir
        );
    }

    #[test]
    fn generic_child_of_generic_parent() {
        // Mid<string> links to the Base__string specialization; the
        // inherited get resolves through the parent chain.
        let ir = compile_program_to_ir(
            r#"
            public class Base<T> {
                public T val;
                public fn init(v: T) { this.val = v; }
                public fn get(): T { return this.val; }
            }
            public class Mid<T> extends Base<T> {
                public fn init(v: T) { super.init(v); }
            }
            public fn main(): void {
                let m = new Mid<string>("hello");
                io::println(m.get());
            }
        "#,
        )
        .expect("generic inheritance IR should succeed");
        assert!(
            ir.contains("Base__string_get"),
            "expected Base__string_get specialization, got:\n{}",
            ir
        );
    }

    #[test]
    fn int_variable_declaration() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let x: int = 42;
            }
        "#,
        )
        .expect("IR generation should succeed");
        assert!(
            ir.contains("alloca i32"),
            "expected i32 alloca, got:\n{}",
            ir
        );
        assert!(
            ir.contains("store i32 42"),
            "expected store i32 42, got:\n{}",
            ir
        );
    }

    #[test]
    fn long_variable_declaration() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let x: long = 1000000000000;
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("alloca i64"),
            "expected i64 alloca, got:\n{}",
            ir
        );
        assert!(ir.contains("store i64"), "expected store i64, got:\n{}", ir);
    }

    #[test]
    fn double_variable_declaration() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let pi: double = 3.14;
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("alloca double"),
            "expected double alloca, got:\n{}",
            ir
        );
        assert!(
            ir.contains("store double"),
            "expected store double, got:\n{}",
            ir
        );
    }

    #[test]
    fn float_variable_declaration() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let f: float = 1.0;
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("alloca float"),
            "expected float alloca, got:\n{}",
            ir
        );
    }

    #[test]
    fn bool_variable_declaration() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let b: bool = true;
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(ir.contains("alloca i1"), "expected i1 alloca, got:\n{}", ir);
    }

    #[test]
    fn char_variable_declaration() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let c: char = 'A';
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("alloca i32"),
            "expected i32 alloca for char, got:\n{}",
            ir
        );
        assert!(
            ir.contains("store i32 65"),
            "expected store i32 65, got:\n{}",
            ir
        );
    }

    #[test]
    fn byte_variable_declaration() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let b: byte = 100;
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(ir.contains("alloca i8"), "expected i8 alloca, got:\n{}", ir);
    }

    #[test]
    fn string_variable_still_works() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                let s: string = "hello";
            }
        "#,
        )
        .expect("IR should succeed");
        // String is stored as a { i64, ptr } struct.
        assert!(ir.contains("alloca"), "expected alloca, got:\n{}", ir);
    }

    #[test]
    fn println_int() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                io::println(42);
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("titrate_println_int"),
            "expected titrate_println_int call, got:\n{}",
            ir
        );
    }

    #[test]
    fn println_double() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                io::println(3.14);
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("titrate_println_double"),
            "expected titrate_println_double call, got:\n{}",
            ir
        );
    }

    #[test]
    fn println_bool() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                io::println(true);
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("titrate_println_bool"),
            "expected titrate_println_bool call, got:\n{}",
            ir
        );
    }

    #[test]
    fn println_char() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                io::println('A');
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("titrate_println_char"),
            "expected titrate_println_char call, got:\n{}",
            ir
        );
    }

    #[test]
    fn println_string_still_works() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                io::println("Hello!");
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("titrate_println"),
            "expected titrate_println call, got:\n{}",
            ir
        );
    }

    #[test]
    fn int_assignment() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                var x: int = 1;
                x = 2;
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("store i32 2"),
            "expected store i32 2, got:\n{}",
            ir
        );
    }

    #[test]
    fn double_assignment() {
        let ir = compile_to_ir(
            r#"
            public fn main(): void {
                var x: double = 1.0;
                x = 2.0;
            }
        "#,
        )
        .expect("IR should succeed");
        assert!(
            ir.contains("store double"),
            "expected store double, got:\n{}",
            ir
        );
    }
