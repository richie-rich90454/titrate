use super::*;
use crate::ast;

fn compile_program(declarations: Vec<ast::Declaration>) -> CompiledProgram {
    let program = ast::Program {
        imports: vec![],
        declarations,
    };
    let mut compiler = Compiler::new();
    compiler.compile(&program).expect("compilation should succeed")
}

fn su() -> ast::Span {
    ast::Span::unknown()
}

mod core;
mod generics_modules;
mod features_opts;
