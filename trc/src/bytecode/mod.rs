// Titrate Alpha 0.2 – crafted by richie-rich90454, 2026

pub mod compiler;
pub mod frame;
pub mod opcodes;
pub mod chunk;
pub mod value;
mod value_impl;
pub mod vm;

pub use compiler::{CompiledProgram, Compiler};
pub use opcodes::{CastTarget, OpCode, TypeTag};
pub use chunk::Chunk;
pub use vm::Vm;

use crate::ast;

/// Compile and run a Titrate program through the bytecode VM.
/// Returns captured output lines.
pub fn execute(program: &ast::Program) -> Result<Vec<String>, String> {
    execute_with_root(program, std::path::Path::new("."))
}

/// Compile and run with a root directory for module resolution.
pub fn execute_with_root(program: &ast::Program, root_dir: &std::path::Path) -> Result<Vec<String>, String> {
    // A file with only declarations and no entry point has nothing to
    // execute. Report it honestly instead of running an empty chunk,
    // which would fail far from the cause inside the VM.
    let has_main = program
        .declarations
        .iter()
        .any(|d| matches!(d, ast::Declaration::Function(f) if f.name == "main"));
    if !has_main {
        return Err("no entry point: file defines no 'main' function".to_string());
    }

    let mut compiler = Compiler::new();
    let compiled = if program.imports.is_empty() {
        compiler.compile(program)?
    } else {
        compiler.compile_with_modules(program, root_dir)?
    };

    let mut vm = Vm::new();
    vm.load_program(compiled);
    vm.run()?;
    Ok(vm.output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;

    fn fn_decl(name: &str) -> ast::Declaration {
        ast::Declaration::Function(FnDecl {
            access: Access::Public,
            name: name.to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        })
    }

    #[test]
    fn execute_rejects_program_without_main() {
        let program = ast::Program {
            imports: vec![],
            declarations: vec![fn_decl("helper")],
        };
        let err = execute(&program).expect_err("program without main should not execute");
        assert!(
            err.contains("no entry point"),
            "expected entry point error, got: {}",
            err
        );
    }

    #[test]
    fn execute_runs_empty_main() {
        let program = ast::Program {
            imports: vec![],
            declarations: vec![fn_decl("main")],
        };
        let output = execute(&program).expect("empty main should run");
        assert!(output.is_empty());
    }
}
