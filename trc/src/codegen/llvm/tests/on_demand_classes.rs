use super::*;
use crate::ast::Declaration;

/// Mirror `compile_program`'s class passes: skeletons first, then declaration
/// order. This is the ordering that used to hand a skeleton `ClassInfo` to a
/// `new B()` inside an earlier class's method.
fn compile_with_class_passes(source: &str) -> Result<String, String> {
    let tokens = lexer::tokenize(source).map_err(|e| format!("tokenize: {}", e))?;
    let ast = parser::parse(tokens).map_err(|e| format!("parse: {}", e))?;
    let typed = analyzer::analyze(&ast).map_err(|e| format!("analyze: {:?}", e))?;
    let context = Context::create();
    let mut backend = LlvmBackend::new(&context, "test");
    backend.declare_natives();

    let classes: Vec<_> = typed
        .declarations
        .iter()
        .filter_map(|d| match d {
            Declaration::Class(c) if c.type_params.is_empty() => Some(c.clone()),
            _ => None,
        })
        .collect();
    for c in &classes {
        backend.class_decls.insert(c.name.clone(), c.clone());
        backend.register_class_skeleton(c)?;
    }
    for c in &classes {
        backend.compile_class_decl(c)?;
    }

    for d in &typed.declarations {
        if let Declaration::Function(f) = d {
            if f.name != "main" && f.type_params.is_empty() {
                backend
                    .function_decls
                    .insert(f.name.clone(), f.clone());
                backend.compile_function(f)?;
            }
        }
    }
    let main = typed
        .declarations
        .iter()
        .find_map(|d| match d {
            Declaration::Function(f) if f.name == "main" => Some(f),
            _ => None,
        })
        .ok_or("no main")?;
    backend.compile_main(main)?;
    backend
        .module
        .verify()
        .map_err(|e| format!("verify: {}", e))?;
    Ok(backend.module.print_to_string().to_string())
}

/// A `new B()` inside a method of an earlier-declared class A must get B's
/// real vtable and constructor, not its skeleton. Otherwise B's first virtual
/// call dereferences a null vtable (native access violation).
#[test]
fn new_inside_earlier_class_method_installs_vtable_and_calls_ctor() {
    let source = r#"
        public class A {
            private B b;
            public fn init() {
                this.b = new B();
            }
            public fn ping(): int {
                return this.b.value();
            }
        }

        public class B {
            private int v;
            public fn init() {
                this.v = 42;
            }
            public fn value(): int {
                return this.v;
            }
        }

        public fn main(): void {
            let a = new A();
            io::println(Integer.toString(a.ping()));
        }
    "#;
    let ir = compile_with_class_passes(source).expect("compile");
    let a_ctor = ir[ir.find("define void @A_new(").expect("A_new")..]
        .split("\n\ndefine ")
        .next()
        .unwrap_or("");
    assert!(
        a_ctor.contains("store ptr @_vtable_B"),
        "new B() used a skeleton (no vtable store):\n{}",
        a_ctor
    );
    assert!(
        a_ctor.contains("call void @B_new("),
        "new B() skipped its constructor:\n{}",
        a_ctor
    );
}

/// Re-entrant on-demand compilation (A's ctor requests B, B's ctor requests A)
/// must terminate and leave the builder at the caller's insert block.
#[test]
fn on_demand_class_compile_preserves_insert_point() {
    let source = r#"
        public class A {
            private B b;
            public fn init() {
                this.b = new B();
            }
        }

        public class B {
            private A a;
            public fn init() {
                this.a = new A();
            }
        }

        public fn main(): void {
            let a = new A();
            if (a == null) { io::println("null"); }
        }
    "#;
    let ir = compile_with_class_passes(source).expect("compile");
    // The module must still be well formed after the nested compilation.
    assert!(ir.contains("define void @A_new("), "{}", ir);
    assert!(ir.contains("define void @B_new("), "{}", ir);
}
