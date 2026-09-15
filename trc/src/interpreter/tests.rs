// Phase 4: Tests for the Titrate interpreter
use crate::ast::*;

    fn make_program(declarations: Vec<Declaration>) -> Program {
        Program {
            imports: vec![],
            declarations,
        }
    }

    fn make_fn_decl(name: &str, params: Vec<Param>, body: Vec<Stmt>) -> FnDecl {
        FnDecl {
            access: Access::Public,
            name: name.to_string(),
            type_params: vec![],
            params,
            return_type: None,
            body,
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }
    }

    fn make_param(name: &str, typ: &str) -> Param {
        Param {
            name: name.to_string(),
            typ: Type::simple(typ),
        }
    }

    fn println_call(arg: Expr) -> Stmt {
        Stmt::Expr(Expr::Call(
            Box::new(Expr::MemberAccess(
                Box::new(Expr::Identifier("io".to_string(), Span::unknown())),
                "println".to_string(),
                Span::unknown(),
            )),
            vec![arg],
            Span::unknown(),
        ))
    }

    // ---- Variable declarations and assignments ----

mod basics;
mod types_ops;
mod advanced;
