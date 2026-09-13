use super::*;
use crate::analyzer;
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

mod vars;
mod ops;
mod flow;
mod release;
