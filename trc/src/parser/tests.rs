use super::parse;
use super::ast;
use crate::lexer;

pub(super) fn parse_src(src: &str) -> Result<ast::Program, String> {
    let tokens = lexer::tokenize(src)?;
    parse(tokens)
}

mod stmts;
mod exprs;
mod items;
mod ops;
