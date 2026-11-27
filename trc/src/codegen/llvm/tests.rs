use super::*;
use crate::analyzer;
use crate::ast::Declaration;
use crate::lexer;
use crate::parser;

    /// Compile a source string to LLVM IR and return the IR string.
    fn compile_to_ir(source: &str) -> Result<String, String> {
        let tokens = lexer::tokenize(source).map_err(|e| format!("tokenize: {}", e))?;
        let ast = parser::parse(tokens).map_err(|e| format!("parse: {}", e))?;
        let typed_ast = analyzer::analyze(&ast).map_err(|e| format!("analyze: {:?}", e))?;
        let context = Context::create();
        let mut backend = LlvmBackend::new(&context, "test");
        backend.declare_natives();
        // Find and compile main.
        let main_decl = typed_ast
            .declarations
            .iter()
            .find_map(|d| match d {
                Declaration::Function(f) if f.name == "main" => Some(f),
                _ => None,
            })
            .ok_or("no main")?;
        backend
            .compile_main(main_decl)
            .map_err(|e| format!("compile: {}", e))?;
        Ok(backend.module.print_to_string().to_string())
    }

    /// A missing-key sentinel for a `HashMap<string, long>` must be i64, like
    /// the loaded value, or LLVM's `select` operand assertion fires.
    #[test]
    fn map_get_long_select_operands_have_matching_widths() {
        let source = r#"
            public fn main(): void {
                var m = new HashMap<string, long>();
                m.put("k", 7L);
                let v: long = m.get("k");
                io::println(Integer.toString(v));
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
            .expect("main");
        backend.compile_main(main_decl).expect("compile main");
        let ir = backend.module.print_to_string().to_string();
        assert!(ir.contains("map.get.value"), "{}", ir);
        assert!(backend.module.verify().is_ok(), "{}", ir);
    }

    /// Compile a full program (including non-main functions) to LLVM IR.
    fn compile_program_to_ir(source: &str) -> Result<String, String> {
        let tokens = lexer::tokenize(source).map_err(|e| format!("tokenize: {}", e))?;
        let ast = parser::parse(tokens).map_err(|e| format!("parse: {}", e))?;
        let typed_ast = analyzer::analyze(&ast).map_err(|e| format!("analyze: {:?}", e))?;
        let context = Context::create();
        let mut backend = LlvmBackend::new(&context, "test");
        backend.declare_natives();
        // Register all function declarations first (for recursion).
        // Generic functions are stashed separately for on-demand
        // monomorphization.
        for decl in &typed_ast.declarations {
            if let Declaration::Function(f) = decl {
                if f.type_params.is_empty() {
                    backend.function_decls.insert(f.name.clone(), f.clone());
                } else {
                    backend
                        .generic_fn_decls
                        .entry(f.name.clone())
                        .or_default()
                        .push(f.clone());
                }
            }
        }
        // Register class declarations (mirrors compile_program) so
        // generic instantiation can find template declarations.
        for decl in &typed_ast.declarations {
            if let Declaration::Class(class_decl) = decl {
                backend
                    .class_decls
                    .insert(class_decl.name.clone(), class_decl.clone());
            }
        }
        // Compile all non-generic, non-main functions first.
        for decl in &typed_ast.declarations {
            if let Declaration::Function(f) = decl {
                if f.name != "main" && f.type_params.is_empty() {
                    backend
                        .compile_function(f)
                        .map_err(|e| format!("compile function '{}': {}", f.name, e))?;
                }
            }
        }
        // Find and compile main.
        let main_decl = typed_ast
            .declarations
            .iter()
            .find_map(|d| match d {
                Declaration::Function(f) if f.name == "main" => Some(f),
                _ => None,
            })
            .ok_or("no main")?;
        backend
            .compile_main(main_decl)
            .map_err(|e| format!("compile main: {}", e))?;
        Ok(backend.module.print_to_string().to_string())
    }

    /// release-mode flag set, so that optimization hints are emitted.
    fn compile_program_to_ir_release(source: &str) -> Result<String, String> {
        let tokens = lexer::tokenize(source).map_err(|e| format!("tokenize: {}", e))?;
        let ast = parser::parse(tokens).map_err(|e| format!("parse: {}", e))?;
        let typed_ast = analyzer::analyze(&ast).map_err(|e| format!("analyze: {:?}", e))?;
        let context = Context::create();
        let mut backend = LlvmBackend::new(&context, "test");
        backend.release_mode = true;
        backend.declare_natives();
        // Compile class declarations first (mirrors compile_program_to_ir_text).
        for decl in &typed_ast.declarations {
            if let Declaration::Class(class_decl) = decl {
                backend.compile_class_decl(class_decl)?;
            }
        }
        // Register all function declarations first (for recursion).
        // Generic functions are stashed separately for on-demand
        // monomorphization.
        for decl in &typed_ast.declarations {
            if let Declaration::Function(f) = decl {
                if f.type_params.is_empty() {
                    backend.function_decls.insert(f.name.clone(), f.clone());
                } else {
                    backend
                        .generic_fn_decls
                        .entry(f.name.clone())
                        .or_default()
                        .push(f.clone());
                }
            }
        }
        // Compile all non-generic, non-main functions first.
        for decl in &typed_ast.declarations {
            if let Declaration::Function(f) = decl {
                if f.name != "main" && f.type_params.is_empty() {
                    backend
                        .compile_function(f)
                        .map_err(|e| format!("compile function '{}': {}", f.name, e))?;
                }
            }
        }
        // Find and compile main.
        let main_decl = typed_ast
            .declarations
            .iter()
            .find_map(|d| match d {
                Declaration::Function(f) if f.name == "main" => Some(f),
                _ => None,
            })
            .ok_or("no main")?;
        backend
            .compile_main(main_decl)
            .map_err(|e| format!("compile main: {}", e))?;
        Ok(backend.module.print_to_string().to_string())
    }

    /// Pointer operand of the last `..., ptr %name` in an IR line.
    fn ptr_operand(line: &str) -> &str {
        let i = line.rfind(", ptr %").expect("ptr operand") + 7;
        line[i..].split_whitespace().next().unwrap_or("")
    }

    /// A container local initialized from a call must own one stable slot:
    /// rebinding it inside a loop writes to the slot the loop condition reads.
    #[test]
    fn loop_condition_uses_native_container_len() {
        let source = r#"
            public fn makeOne(): ArrayList<int> {
                let l: ArrayList<int> = new ArrayList<int>();
                l.add(1);
                return l;
            }
            public fn makeEmpty(): ArrayList<int> {
                let l: ArrayList<int> = new ArrayList<int>();
                return l;
            }
            public fn main(): void {
                let r: ArrayList<int> = makeOne();
                var guard: int = 0;
                while (r.size() > 0) {
                    r = makeEmpty();
                    guard = guard + 1;
                }
            }
        "#;
        let ir = compile_program_to_ir(source).expect("compile loop");
        let main = &ir[ir.find("define i32 @main(").expect("main")..];
        let cond = main.split("while.cond:").nth(1).expect("while.cond");
        let cond_block = cond.split("\n\n").next().unwrap_or(cond);
        let cond_line = cond_block
            .lines()
            .find(|l| l.contains("= load { i64, ptr }, ptr %"))
            .expect("container len load in while.cond");
        let cond_slot = ptr_operand(cond_line);

        let body = main.split("while.body:").nth(1).expect("while.body");
        let body_block = body.split("\n\n").next().unwrap_or(body);
        let store_line = body_block
            .lines()
            .find(|l| l.trim_start().starts_with("store { i64, ptr }"))
            .expect("container rebinding in while.body");
        let store_slot = ptr_operand(store_line);

        assert_eq!(cond_slot, store_slot, "stale container slot:\n{}", ir);
        assert!(!cond_slot.contains("container.arg.tmp"), "{}", ir);
    }

    /// Array element access is emitted inside loop bodies; its scratch slots
    /// must live in the entry block or a long loop overflows the native stack.
    #[test]
    fn loop_body_allocas_are_hoisted_to_entry() {
        let source = r#"
            public fn sum(list: ArrayList<double>): double {
                var total: double = 0.0;
                var i: int = 0;
                while (i < list.size()) {
                    total = total + list.get(i);
                    i = i + 1;
                }
                return total;
            }
            public fn main(): void {
                let l: ArrayList<double> = new ArrayList<double>();
                l.add(1.0);
                sum(l);
            }
        "#;
        let ir = compile_program_to_ir(source).expect("compile loops");
        assert!(ir.contains("titrate_array_get_double"), "{}", ir);

        let mut in_function = false;
        let mut in_entry = false;
        for line in ir.lines() {
            if line.starts_with("define ") {
                in_function = true;
                in_entry = false;
                continue;
            }
            if line.starts_with('}') {
                in_function = false;
                in_entry = false;
                continue;
            }
            if !in_function {
                continue;
            }
            let trimmed = line.trim_start();
            if trimmed.starts_with("entry:") {
                in_entry = true;
                continue;
            }
            assert!(!line.contains("alloca") || in_entry, "alloca outside entry: {}\n{}", line, ir);
            if in_entry
                && (trimmed.starts_with("br ")
                    || trimmed.starts_with("ret ")
                    || trimmed.starts_with("switch ")
                    || trimmed.starts_with("unreachable"))
            {
                in_entry = false;
            }
        }
    }

    /// A fresh container initializer gets a private slot; a declaration that
    /// names an existing container keeps aliasing it.
    #[test]
    fn container_decl_from_call_gets_private_slot_but_alias_shares() {
        let source = r#"
            public fn make(): ArrayList<int> {
                let l: ArrayList<int> = new ArrayList<int>();
                return l;
            }
            public fn main(): void {
                let a: ArrayList<int> = make();
                let b: ArrayList<int> = a;
                b.add(1);
                a.size();
            }
        "#;
        let ir = compile_program_to_ir(source).expect("compile declarations");
        let main = &ir[ir.find("define i32 @main(").expect("main")..];
        assert!(main.contains("%a = alloca { i64, ptr }"), "{}", main);
        assert!(!main.contains("%b = alloca"), "{}", main);
        assert!(!main.contains("container.arg.tmp"), "{}", main);
    }

mod vars;
mod ops;
mod flow;
mod release;
mod on_demand_classes;
