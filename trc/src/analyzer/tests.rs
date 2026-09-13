fn program_with(decl: crate::ast::Declaration) -> crate::ast::Program {
    crate::ast::Program {
        imports: vec![],
        declarations: vec![decl],
    }
}

fn empty_program() -> crate::ast::Program {
    crate::ast::Program {
        imports: vec![],
        declarations: vec![],
    }
}

mod symbols_types;
mod programs_flow;
mod decls_inference;
mod patterns_errors;
