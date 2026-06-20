// Phase 2: Parser — converts a token stream into an AST.
// All desugaring is performed here so the downstream passes see a clean tree.

mod declarations;
mod expressions;
mod patterns;
mod statements;
mod types;

use crate::{ast, lexer};

// ---------------------------------------------------------------------------
// Parser struct
// ---------------------------------------------------------------------------

pub(crate) struct Parser {
    pub(super) tokens: Vec<lexer::SpannedToken>,
    pub(super) pos: usize,
}

impl Parser {
    pub(super) fn new(tokens: Vec<lexer::SpannedToken>) -> Self {
        Parser { tokens, pos: 0 }
    }

    /// Return a reference to the current token, or Eof if past the end.
    pub(super) fn peek(&self) -> &lexer::Token {
        match self.tokens.get(self.pos) {
            Some(st) => &st.token,
            None => &lexer::Token::Eof,
        }
    }

    /// Consume and return the current token, advancing the position.
    pub(super) fn advance(&mut self) -> lexer::Token {
        match self.tokens.get(self.pos) {
            Some(st) => {
                let tok = st.token.clone();
                self.pos += 1;
                tok
            }
            None => lexer::Token::Eof,
        }
    }

    /// If the current token matches `expected`, consume it and return Ok;
    /// otherwise return an error.
    pub(super) fn expect(&mut self, expected: &lexer::Token) -> Result<lexer::Token, String> {
        let current = self.peek().clone();
        if tokens_match(&current, expected) {
            Ok(self.advance())
        } else {
            Err(format!("Expected {}, found {}", expected, current))
        }
    }

    /// Expect a `>` token to close a generic type parameter list.
    /// Handles `>>` (RightShift) and `>>>` (TripleGreater) produced by the
    /// lexer when multiple `>` characters are adjacent (e.g.
    /// `ArrayList<ArrayList<double>>` or `ArrayList<ArrayList<ArrayList<double>>>`).
    /// The multi-char token is consumed and a single Greater is pushed back so
    /// nested generics close correctly.
    pub(super) fn expect_close_angle(&mut self) -> Result<(), String> {
        let (token, line, column) = match self.tokens.get(self.pos) {
            Some(st) => (st.token.clone(), st.line, st.column),
            None => (lexer::Token::Eof, 0, 0),
        };
        match token {
            lexer::Token::Greater => {
                self.pos += 1;
                Ok(())
            }
            lexer::Token::RightShift => {
                self.pos += 1;
                self.tokens.insert(
                    self.pos,
                    lexer::SpannedToken {
                        token: lexer::Token::Greater,
                        line,
                        column: column + 1,
                    },
                );
                Ok(())
            }
            lexer::Token::TripleGreater => {
                self.pos += 1;
                self.tokens.insert(
                    self.pos,
                    lexer::SpannedToken {
                        token: lexer::Token::RightShift,
                        line,
                        column: column + 1,
                    },
                );
                Ok(())
            }
            _ => Err(format!("Expected >, found {}", token)),
        }
    }

    /// Expect a name: either an identifier or a type keyword used as a name
    /// (e.g. `size` in `fn size(): int`). Type keywords are reserved words
    /// like `int`, `double`, `size`, etc. that are commonly used as method
    /// names in the standard library.
    pub(super) fn expect_name(&mut self) -> Result<String, String> {
        let tok = self.advance();
        match tok {
            lexer::Token::Identifier(s) => Ok(s),
            t if is_type_keyword(&t) => type_keyword_name(&t)
                .ok_or_else(|| format!("Expected name, found {}", t)),
            // `where` can be used as a function name (e.g. `fn where(...)`)
            lexer::Token::Where => Ok("where".to_string()),
            _ => Err(format!("Expected name, found {}", tok)),
        }
    }

    /// If the current token matches `expected`, consume it and return true;
    /// otherwise return false (no consumption).
    pub(super) fn match_token(&mut self, expected: &lexer::Token) -> bool {
        if self.is_at(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Check whether the current token matches `expected` without consuming.
    pub(super) fn is_at(&self, expected: &lexer::Token) -> bool {
        tokens_match(self.peek(), expected)
    }

    /// Check whether we have consumed all tokens.
    pub(super) fn is_eof(&self) -> bool {
        matches!(self.peek(), lexer::Token::Eof)
    }

    /// Peek at the token at position `self.pos + offset` without consuming.
    pub(super) fn peek_at(&self, offset: usize) -> &lexer::Token {
        match self.tokens.get(self.pos + offset) {
            Some(st) => &st.token,
            None => &lexer::Token::Eof,
        }
    }

    /// Check whether the current `?` token is the start of a ternary expression
    /// rather than error propagation. Returns true if the token after `?` looks
    /// like it could start an expression (i.e., it's a ternary `?`).
    pub(super) fn is_ternary_question(&self) -> bool {
        debug_assert!(self.is_at(&lexer::Token::Question));
        let next = self.peek_at(1).clone();
        matches!(next,
            lexer::Token::IntLiteral(_)
            | lexer::Token::FloatLiteral { .. }
            | lexer::Token::StringLiteral(_)
            | lexer::Token::RawStringLiteral(_)
            | lexer::Token::CharLiteral(_)
            | lexer::Token::ByteLiteral(_)
            | lexer::Token::BoolLiteral(_)
            | lexer::Token::NullLiteral
            | lexer::Token::Identifier(_)
            | lexer::Token::LeftParen
            | lexer::Token::LeftBrace
            | lexer::Token::LeftBracket
            | lexer::Token::Not
            | lexer::Token::Minus
            | lexer::Token::Tilde
            | lexer::Token::Star
            | lexer::Token::Ampersand
            | lexer::Token::RefMut
            | lexer::Token::PlusPlus
            | lexer::Token::MinusMinus
            | lexer::Token::New
            | lexer::Token::This
            | lexer::Token::Super
            | lexer::Token::Owned
            | lexer::Token::Unsafe
        )
    }

    /// Helper: get the current line/column for error messages.
    pub(super) fn span_here(&self) -> (usize, usize) {
        match self.tokens.get(self.pos) {
            Some(st) => (st.line, st.column),
            None => (0, 0),
        }
    }

    /// Create an ast::Span from the current position.
    pub(super) fn make_span(&self) -> ast::Span {
        let (line, col) = self.span_here();
        ast::Span::new(line as u32, col as u32)
    }

    /// Parse optional type parameters: `<T, U: Display, ...>`
    /// If the next token is `<`, parse a comma-separated list of identifiers
    /// (each optionally followed by `:` and an interface name) until `>`.
    /// Otherwise return an empty vec.
    pub(super) fn parse_type_params(&mut self) -> Result<Vec<ast::TypeParam>, String> {
        if self.match_token(&lexer::Token::Less) {
            let mut params = Vec::new();
            loop {
                let tok = self.expect(&lexer::Token::Identifier(String::new()))?;
                let name = match tok {
                    lexer::Token::Identifier(s) => s,
                    _ => return Err(format!("Expected type parameter name, found {}", tok)),
                };
                // Optional constraint: `T: Display`
                let constraint = if self.match_token(&lexer::Token::Colon) {
                    let ty = self.parse_type()?;
                    Some(ty)
                } else {
                    None
                };
                params.push(ast::TypeParam { name, constraint });
                if !self.match_token(&lexer::Token::Comma) {
                    break;
                }
            }
            self.expect_close_angle()?;
            Ok(params)
        } else {
            Ok(vec![])
        }
    }

    /// Parse optional where clause: `where T: Comparable<T>, U: Display`
    /// Returns a list of TypeParam entries representing the constraints.
    pub(super) fn parse_where_clause(&mut self) -> Result<Vec<ast::TypeParam>, String> {
        if self.match_token(&lexer::Token::Where) {
            let mut constraints = Vec::new();
            loop {
                let tok = self.expect(&lexer::Token::Identifier(String::new()))?;
                let name = match tok {
                    lexer::Token::Identifier(s) => s,
                    _ => return Err(format!("Expected type parameter name in where clause, found {}", tok)),
                };
                self.expect(&lexer::Token::Colon)?;
                let constraint = self.parse_type()?;
                constraints.push(ast::TypeParam { name, constraint: Some(constraint) });
                if !self.match_token(&lexer::Token::Comma) {
                    break;
                }
            }
            Ok(constraints)
        } else {
            Ok(vec![])
        }
    }

    /// If the current token is an operator that can be overloaded,
    /// return its string representation (e.g. "+", "==", "<<").
    /// Returns None if the current token is not an overloadable operator.
    pub(super) fn operator_token_to_str(&self) -> Option<&'static str> {
        match self.peek() {
            lexer::Token::Plus => Some("+"),
            lexer::Token::Minus => Some("-"),
            lexer::Token::Star => Some("*"),
            lexer::Token::Slash => Some("/"),
            lexer::Token::Percent => Some("%"),
            lexer::Token::EqualEqual => Some("=="),
            lexer::Token::NotEqual => Some("!="),
            lexer::Token::Less => Some("<"),
            lexer::Token::Greater => Some(">"),
            lexer::Token::LessEqual => Some("<="),
            lexer::Token::GreaterEqual => Some(">="),
            lexer::Token::Ampersand => Some("&"),
            lexer::Token::Pipe => Some("|"),
            lexer::Token::Caret => Some("^"),
            lexer::Token::LeftShift => Some("<<"),
            lexer::Token::RightShift => Some(">>"),
            lexer::Token::TripleGreater => Some(">>>"),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Token comparison (structural equality, ignoring value in literals)
// ---------------------------------------------------------------------------

pub(super) fn tokens_match(a: &lexer::Token, b: &lexer::Token) -> bool {
    // Always compare by variant discriminant only, so that
    // expect(Identifier(String::new())) matches any Identifier, etc.
    std::mem::discriminant(a) == std::mem::discriminant(b)
}

// ---------------------------------------------------------------------------
// Helpers for checking token categories
// ---------------------------------------------------------------------------

/// Tokens that represent a type keyword (used in sugar declarations).
pub(super) fn is_type_keyword(tok: &lexer::Token) -> bool {
    matches!(
        tok,
        lexer::Token::Void
            | lexer::Token::Bool
            | lexer::Token::Byte
            | lexer::Token::Short
            | lexer::Token::Int
            | lexer::Token::Long
            | lexer::Token::Vast
            | lexer::Token::Uvast
            | lexer::Token::Float
            | lexer::Token::Double
            | lexer::Token::Half
            | lexer::Token::Quad
            | lexer::Token::Char
            | lexer::Token::String
            | lexer::Token::Size
            | lexer::Token::U8
            | lexer::Token::U16
            | lexer::Token::U32
            | lexer::Token::U64
    )
}

/// Convert a type keyword token into its string name.
pub(super) fn type_keyword_name(tok: &lexer::Token) -> Option<String> {
    match tok {
        lexer::Token::Void => Some("void".to_string()),
        lexer::Token::Bool => Some("bool".to_string()),
        lexer::Token::Byte => Some("byte".to_string()),
        lexer::Token::Short => Some("short".to_string()),
        lexer::Token::Int => Some("int".to_string()),
        lexer::Token::Long => Some("long".to_string()),
        lexer::Token::Vast => Some("vast".to_string()),
        lexer::Token::Uvast => Some("uvast".to_string()),
        lexer::Token::Float => Some("float".to_string()),
        lexer::Token::Double => Some("double".to_string()),
        lexer::Token::Half => Some("half".to_string()),
        lexer::Token::Quad => Some("quad".to_string()),
        lexer::Token::Char => Some("char".to_string()),
        lexer::Token::String => Some("string".to_string()),
        lexer::Token::Size => Some("size".to_string()),
        lexer::Token::U8 => Some("u8".to_string()),
        lexer::Token::U16 => Some("u16".to_string()),
        lexer::Token::U32 => Some("u32".to_string()),
        lexer::Token::U64 => Some("u64".to_string()),
        _ => None,
    }
}

/// Convert any token that can serve as a member/field name into a String.
/// This includes identifiers, type keywords (e.g. `size`, `int`), and
/// special keywords like `Ok`, `Err`, `toString`, etc.
pub(super) fn token_as_name(tok: &lexer::Token) -> Option<String> {
    match tok {
        lexer::Token::Identifier(s) => Some(s.clone()),
        // Type keywords can appear as method names (e.g. list.size())
        t => type_keyword_name(t).or_else(|| match t {
            lexer::Token::Ok => Some("Ok".to_string()),
            lexer::Token::Err => Some("Err".to_string()),
            lexer::Token::Result => Some("Result".to_string()),
            lexer::Token::Owned => Some("Owned".to_string()),
            lexer::Token::New => Some("new".to_string()),
            lexer::Token::This => Some("this".to_string()),
            lexer::Token::Super => Some("super".to_string()),
            lexer::Token::NullLiteral => Some("null".to_string()),
            // `fn` can be used as a parameter name (e.g. `forEach(fn: fn(T): void)`)
            lexer::Token::Fn => Some("fn".to_string()),
            // `var` can be used as a variable name (e.g. `let var: double = ...` for variance)
            lexer::Token::Var => Some("var".to_string()),
            // `where` can be used as a function name (e.g. `where(...)`)
            lexer::Token::Where => Some("where".to_string()),
            _ => None,
        }),
    }
}

/// Check if a token can begin a type (keyword or identifier or type-like keywords).
pub(super) fn is_type_start(tok: &lexer::Token) -> bool {
    is_type_keyword(tok)
        || matches!(tok, lexer::Token::Identifier(_))
        || matches!(tok, lexer::Token::Owned | lexer::Token::Result | lexer::Token::Fn)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn parse(tokens: Vec<lexer::SpannedToken>) -> Result<ast::Program, String> {
    let mut parser = Parser::new(tokens);
    parser.parse_program()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer;

    fn parse_src(src: &str) -> Result<ast::Program, String> {
        let tokens = lexer::tokenize(src)?;
        parse(tokens)
    }

    // -----------------------------------------------------------------------
    // Micro-test
    // -----------------------------------------------------------------------
    #[test]
    fn test_micro_test() {
        let src = r#"int x = 5; public void main() { io::println("Hello"); }"#;
        let prog = parse_src(src).expect("parse should succeed");

        // x is VarDecl with type Named{name:"int"}, init Literal(Int(5)), mutable=true
        assert_eq!(prog.declarations.len(), 2);

        match &prog.declarations[0] {
            ast::Declaration::VarDecl(vd) => {
                assert_eq!(vd.name, "x");
                assert_eq!(vd.typ, Some(ast::Type::simple("int")));
                assert_eq!(vd.init, Some(ast::Expr::Literal(ast::Literal::Int(5), ast::Span::new(1, 9))));
                assert!(vd.mutable);
            }
            other => panic!("Expected VarDecl, got {:?}", other),
        }

        // main has return_type Named{name:"void"}, params empty
        match &prog.declarations[1] {
            ast::Declaration::Function(fd) => {
                assert_eq!(fd.name, "main");
                assert_eq!(fd.return_type, Some(ast::Type::simple("void")));
                assert!(fd.params.is_empty());
                assert!(fd.sugar);
            }
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Statement tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_block_stmt() {
        let src = r#"fn f(): void { { let x = 1; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => {
                assert_eq!(fd.body.len(), 1);
                match &fd.body[0] {
                    ast::Stmt::Block(block) => {
                        assert_eq!(block.len(), 1);
                    }
                    other => panic!("Expected Block, got {:?}", other),
                }
            }
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_if_stmt() {
        let src = r#"fn f(): void { if (true) { let x = 1; } else { let y = 2; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => {
                assert_eq!(fd.body.len(), 1);
                match &fd.body[0] {
                    ast::Stmt::If(if_stmt) => {
                        assert!(matches!(if_stmt.condition, ast::Expr::Literal(ast::Literal::Bool(true), _)));
                        assert_eq!(if_stmt.then_branch.len(), 1);
                        assert!(if_stmt.else_branch.is_some());
                    }
                    other => panic!("Expected If, got {:?}", other),
                }
            }
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_if_no_parens() {
        let src = r#"fn f(): void { if true { let x = 1; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::If(_) => {}
                other => panic!("Expected If, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_while_stmt() {
        let src = r#"fn f(): void { while (true) { break; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::While(ws) => {
                    assert!(matches!(ws.condition, ast::Expr::Literal(ast::Literal::Bool(true), _)));
                    assert_eq!(ws.body.len(), 1);
                    assert!(matches!(&ws.body[0], ast::Stmt::Break));
                }
                other => panic!("Expected While, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_do_while_stmt() {
        let src = r#"fn f(): void { do { x = x + 1; } while (x < 10); }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::DoWhile(dw) => {
                    assert!(matches!(dw.condition, ast::Expr::Binary(_, ast::Operator::Lt, _, _)));
                    assert_eq!(dw.body.len(), 1);
                }
                other => panic!("Expected DoWhile, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_do_while_no_parens() {
        let src = r#"fn f(): void { do { break; } while true; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::DoWhile(dw) => {
                    assert!(matches!(dw.condition, ast::Expr::Literal(ast::Literal::Bool(true), _)));
                }
                other => panic!("Expected DoWhile, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_for_stmt() {
        let src = r#"fn f(): void { for var i in items { continue; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::For(fs) => {
                    assert_eq!(fs.var, "i");
                    assert!(matches!(fs.iterable, ast::Expr::Identifier(_, _)));
                    assert_eq!(fs.body.len(), 1);
                    assert!(matches!(&fs.body[0], ast::Stmt::Continue));
                }
                other => panic!("Expected For, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_return_stmt() {
        let src = r#"fn f(): int { return 42; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Literal(ast::Literal::Int(42), _))) => {}
                other => panic!("Expected Return(Some(42)), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_return_void() {
        let src = r#"fn f(): void { return; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(None) => {}
                other => panic!("Expected Return(None), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_switch_stmt() {
        let src = r#"fn f(): void { switch x { case 1 => return; case 2 => { break; } default => { continue; } } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Switch(ss) => {
                    assert_eq!(ss.cases.len(), 2);
                    assert!(ss.default.is_some());
                }
                other => panic!("Expected Switch, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_let_stmt() {
        let src = r#"fn f(): void { let x: int = 5; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::VarDecl(vd) => {
                    assert_eq!(vd.name, "x");
                    assert_eq!(vd.typ, Some(ast::Type::simple("int")));
                    assert!(!vd.mutable);
                }
                other => panic!("Expected VarDecl, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_var_desugar() {
        let src = r#"fn f(): void { var x = 5; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::VarDecl(vd) => {
                    assert_eq!(vd.name, "x");
                    assert!(vd.mutable);
                }
                other => panic!("Expected VarDecl, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_const_stmt() {
        let src = r#"fn f(): void { const X: int = 42; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::ConstDecl(vd) => {
                    assert_eq!(vd.name, "X");
                    assert!(!vd.mutable);
                }
                other => panic!("Expected ConstDecl, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_typed_var_desugar() {
        let src = r#"fn f(): void { int x = 5; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::VarDecl(vd) => {
                    assert_eq!(vd.name, "x");
                    assert_eq!(vd.typ, Some(ast::Type::simple("int")));
                    assert!(vd.mutable);
                }
                other => panic!("Expected VarDecl, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_stmt() {
        let src = r#"fn f(): void { foo(); }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Call(_, args, _)) => {
                    assert!(args.is_empty());
                }
                other => panic!("Expected Expr(Call), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Expression tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_int_literal() {
        let src = r#"fn f(): int { return 42; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Literal(ast::Literal::Int(42), _))) => {}
                other => panic!("Expected Return(Int(42)), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_float_literal() {
        let src = r#"fn f(): double { return 3.14; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Literal(ast::Literal::Float(_), _))) => {}
                other => panic!("Expected Return(Float), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_string_literal() {
        let src = r#"fn f(): void { return "hello"; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Literal(ast::Literal::String(s), _))) => {
                    assert_eq!(s, "hello");
                }
                other => panic!("Expected Return(String), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_bool_literal() {
        let src = r#"fn f(): bool { return true; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Literal(ast::Literal::Bool(true), _))) => {}
                other => panic!("Expected Return(Bool(true)), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_null_literal() {
        let src = r#"fn f(): void { return null; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Literal(ast::Literal::Null, _))) => {}
                other => panic!("Expected Return(Null), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_identifier_expr() {
        let src = r#"fn f(): int { return x; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Identifier(s, _))) => {
                    assert_eq!(s, "x");
                }
                other => panic!("Expected Return(Identifier), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_this_expr() {
        let src = r#"fn f(): void { return this; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::This(_))) => {}
                other => panic!("Expected Return(This), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_super_expr() {
        let src = r#"fn f(): void { return super; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Super(_))) => {}
                other => panic!("Expected Return(Super), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_paren_expr() {
        let src = r#"fn f(): int { return (1 + 2); }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Binary(_, ast::Operator::Add, _, _))) => {}
                other => panic!("Expected Return(Binary Add), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_new_expr() {
        let src = r#"fn f(): void { new Foo(1, 2); }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::New(typ, args, _)) => {
                    assert_eq!(typ, &ast::Type::simple("Foo"));
                    assert_eq!(args.len(), 2);
                }
                other => panic!("Expected New, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_ok_expr() {
        let src = r#"fn f(): void { return Ok(42); }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Call(callee, args, _))) => {
                    assert!(matches!(callee.as_ref(), ast::Expr::Identifier(s, _) if s == "Ok"));
                    assert_eq!(args.len(), 1);
                }
                other => panic!("Expected Return(Call(Ok, ...)), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_err_expr() {
        let src = r#"fn f(): void { return Err("oops"); }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Call(callee, args, _))) => {
                    assert!(matches!(callee.as_ref(), ast::Expr::Identifier(s, _) if s == "Err"));
                    assert_eq!(args.len(), 1);
                }
                other => panic!("Expected Return(Call(Err, ...)), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_unsafe_block() {
        let src = r#"fn f(): void { unsafe { let x = 1; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::UnsafeBlock(block, _)) => {
                    assert_eq!(block.len(), 1);
                }
                other => panic!("Expected UnsafeBlock, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_region_block() {
        let src = r#"fn f(): void { region r { let x = 1; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::UnsafeBlock(block, _)) => {
                    assert_eq!(block.len(), 1);
                }
                other => panic!("Expected UnsafeBlock (region), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Postfix expression tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_call_expr() {
        let src = r#"fn f(): void { foo(1, 2); }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Call(_, args, _)) => {
                    assert_eq!(args.len(), 2);
                }
                other => panic!("Expected Call, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_member_access() {
        let src = r#"fn f(): void { obj.field; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::MemberAccess(_, name, _)) => {
                    assert_eq!(name, "field");
                }
                other => panic!("Expected MemberAccess, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_namespace_access() {
        let src = r#"fn f(): void { io::println("hi"); }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Call(callee, _, _)) => {
                    assert!(matches!(callee.as_ref(), ast::Expr::MemberAccess(_, name, _) if name == "println"));
                }
                other => panic!("Expected Call(MemberAccess), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_index_expr() {
        let src = r#"fn f(): void { arr[0]; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Index(_, _, _)) => {}
                other => panic!("Expected Index, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_error_propagation() {
        let src = r#"fn f(): void { foo?; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::ErrorPropagation(_, _)) => {}
                other => panic!("Expected ErrorPropagation, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_cast_expr() {
        let src = r#"fn f(): void { x as int; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Cast(_, typ, _)) => {
                    assert_eq!(typ, &ast::Type::simple("int"));
                }
                other => panic!("Expected Cast, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Operator precedence tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_addition_precedence() {
        let src = r#"fn f(): int { return 1 + 2 * 3; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Binary(_, ast::Operator::Add, right, _))) => {
                    // Right side should be 2 * 3
                    assert!(matches!(right.as_ref(), ast::Expr::Binary(_, ast::Operator::Mul, _, _)));
                }
                other => panic!("Expected Add with Mul on right, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_assignment_right_assoc() {
        let src = r#"fn f(): void { x = y = 5; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(_, right, _)) => {
                    // Right side should be y = 5
                    assert!(matches!(right.as_ref(), ast::Expr::Assign(_, _, _)));
                }
                other => panic!("Expected Assign with Assign on right, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_unary_neg() {
        let src = r#"fn f(): int { return -5; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Unary(ast::UnOp::Neg, _, _))) => {}
                other => panic!("Expected Unary Neg, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_unary_not() {
        let src = r#"fn f(): bool { return !true; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Unary(ast::UnOp::Not, _, _))) => {}
                other => panic!("Expected Unary Not, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_unary_bitnot() {
        let src = r#"fn f(): int { return ~5; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Unary(ast::UnOp::BitNot, _, _))) => {}
                other => panic!("Expected Unary BitNot, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_ref_expr() {
        let src = r#"fn f(): void { let p = &x; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::VarDecl(vd) => match &vd.init {
                    Some(ast::Expr::RefExpr(_, ast::RefKind::Immutable, _)) => {}
                    other => panic!("Expected RefExpr Immutable, got {:?}", other),
                },
                other => panic!("Expected VarDecl, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_ref_mut_expr() {
        let src = r#"fn f(): void { let p = &mut x; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::VarDecl(vd) => match &vd.init {
                    Some(ast::Expr::RefExpr(_, ast::RefKind::Mutable, _)) => {}
                    other => panic!("Expected RefExpr Mutable, got {:?}", other),
                },
                other => panic!("Expected VarDecl, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Desugaring tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_sugar_fn_desugar() {
        let src = r#"public int add(int a, int b) { return a + b; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => {
                assert_eq!(fd.name, "add");
                assert_eq!(fd.return_type, Some(ast::Type::simple("int")));
                assert_eq!(fd.params.len(), 2);
                assert_eq!(fd.params[0].name, "a");
                assert_eq!(fd.params[0].typ, ast::Type::simple("int"));
                assert_eq!(fd.params[1].name, "b");
                assert_eq!(fd.params[1].typ, ast::Type::simple("int"));
                assert!(fd.sugar);
                assert_eq!(fd.access, ast::Access::Public);
            }
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_var_desugar_top_level() {
        let src = r#"var x = 5;"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::VarDecl(vd) => {
                assert_eq!(vd.name, "x");
                assert!(vd.mutable);
                assert_eq!(vd.init, Some(ast::Expr::Literal(ast::Literal::Int(5), ast::Span::new(1, 9))));
            }
            other => panic!("Expected VarDecl, got {:?}", other),
        }
    }

    #[test]
    fn test_typed_var_desugar_top_level() {
        let src = r#"int x = 5;"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::VarDecl(vd) => {
                assert_eq!(vd.name, "x");
                assert_eq!(vd.typ, Some(ast::Type::simple("int")));
                assert!(vd.mutable);
            }
            other => panic!("Expected VarDecl, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Pattern tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_literal_pattern() {
        let src = r#"fn f(): void { switch x { case 42 => return; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Switch(ss) => {
                    assert_eq!(ss.cases.len(), 1);
                    assert_eq!(ss.cases[0].pattern, ast::Pattern::Literal(ast::Literal::Int(42)));
                }
                other => panic!("Expected Switch, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_wildcard_pattern() {
        let src = r#"fn f(): void { switch x { case _ => return; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Switch(ss) => {
                    assert_eq!(ss.cases[0].pattern, ast::Pattern::Wildcard);
                }
                other => panic!("Expected Switch, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_constructor_pattern() {
        let src = r#"fn f(): void { switch x { case Some(v) => return; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Switch(ss) => {
                    assert_eq!(
                        ss.cases[0].pattern,
                        ast::Pattern::Constructor {
                            name: "Some".to_string(),
                            bindings: vec!["v".to_string()]
                        }
                    );
                }
                other => panic!("Expected Switch, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Class tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_class_with_constructor() {
        let src = r#"class Circle {
            double radius;
            public Circle(double r) { this.radius = r; }
            fn area(): double { return 3.14 * this.radius * this.radius; }
        }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Class(cd) => {
                assert_eq!(cd.name, "Circle");
                assert_eq!(cd.members.len(), 3);
                // Field
                match &cd.members[0] {
                    ast::ClassMember::Field(f) => {
                        assert_eq!(f.name, "radius");
                    }
                    other => panic!("Expected Field, got {:?}", other),
                }
                // Constructor
                match &cd.members[1] {
                    ast::ClassMember::Constructor(m) => {
                        assert_eq!(m.name, "new");
                        assert_eq!(m.params.len(), 1);
                    }
                    other => panic!("Expected Constructor, got {:?}", other),
                }
                // Method
                match &cd.members[2] {
                    ast::ClassMember::Method(m) => {
                        assert_eq!(m.name, "area");
                    }
                    other => panic!("Expected Method, got {:?}", other),
                }
            }
            other => panic!("Expected Class, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Enum tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_enum_with_variants() {
        let src = r#"enum Color { Red, Green, Blue }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Enum(ed) => {
                assert_eq!(ed.name, "Color");
                assert_eq!(ed.variants.len(), 3);
                assert_eq!(ed.variants[0].name, "Red");
                assert!(ed.variants[0].fields.is_empty());
            }
            other => panic!("Expected Enum, got {:?}", other),
        }
    }

    #[test]
    fn test_enum_with_fields() {
        let src = r#"enum Shape { Circle(double), Rectangle(int, int) }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Enum(ed) => {
                assert_eq!(ed.variants[0].name, "Circle");
                assert_eq!(ed.variants[0].fields.len(), 1);
                assert_eq!(ed.variants[1].name, "Rectangle");
                assert_eq!(ed.variants[1].fields.len(), 2);
            }
            other => panic!("Expected Enum, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Import tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_import() {
        let src = r#"import io::println;"#;
        let prog = parse_src(src).expect("parse should succeed");
        assert_eq!(prog.imports.len(), 1);
        assert_eq!(prog.imports[0].path, vec!["io", "println"]);
    }

    // -----------------------------------------------------------------------
    // Type parsing tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_generic_type() {
        let src = r#"fn f(): Owned<int> { return x; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => {
                assert_eq!(
                    fd.return_type,
                    Some(ast::Type::generic("Owned", vec![ast::Type::simple("int")]))
                );
            }
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_multi_param_generic_type() {
        let src = r#"fn f(): Result<int, string> { return x; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => {
                assert_eq!(
                    fd.return_type,
                    Some(ast::Type::generic(
                        "Result",
                        vec![ast::Type::simple("int"), ast::Type::simple("string")]
                    ))
                );
            }
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // String concatenation
    // -----------------------------------------------------------------------
    #[test]
    fn test_string_concat() {
        let src = r#"fn f(): void { return "hello" + " world"; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Binary(_, ast::Operator::Add, _, _))) => {}
                other => panic!("Expected Binary Add, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Error cases
    // -----------------------------------------------------------------------
    #[test]
    fn test_missing_semicolon() {
        let src = r#"fn f(): void { let x = 5 }"#;
        let result = parse_src(src);
        assert!(result.is_err(), "Expected parse error for missing semicolon");
    }

    #[test]
    fn test_mismatched_parens() {
        let src = r#"fn f(): void { foo(; }"#;
        let result = parse_src(src);
        assert!(result.is_err(), "Expected parse error for mismatched parens");
    }

    #[test]
    fn test_missing_brace() {
        let src = r#"fn f(): void { let x = 5;"#;
        let result = parse_src(src);
        assert!(result.is_err(), "Expected parse error for missing brace");
    }

    #[test]
    fn test_invalid_token_in_expr() {
        let src = r#"fn f(): void { return @; }"#;
        let result = parse_src(src);
        // The lexer produces an Error token, the parser should fail on it
        assert!(result.is_err(), "Expected parse error for invalid token");
    }

    // -----------------------------------------------------------------------
    // Interface test
    // -----------------------------------------------------------------------
    #[test]
    fn test_interface() {
        let src = r#"interface Printable {
            fn format(): string;
        }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Interface(id) => {
                assert_eq!(id.name, "Printable");
                assert_eq!(id.methods.len(), 1);
                assert_eq!(id.methods[0].name, "format");
            }
            other => panic!("Expected Interface, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Canonical fn test
    // -----------------------------------------------------------------------
    #[test]
    fn test_canonical_fn() {
        let src = r#"fn add(a: int, b: int): int { return a + b; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => {
                assert_eq!(fd.name, "add");
                assert_eq!(fd.params.len(), 2);
                assert_eq!(fd.params[0].name, "a");
                assert_eq!(fd.params[0].typ, ast::Type::simple("int"));
                assert_eq!(fd.return_type, Some(ast::Type::simple("int")));
                assert!(!fd.sugar);
            }
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Owned expression
    // -----------------------------------------------------------------------
    #[test]
    fn test_owned_expr() {
        let src = r#"fn f(): void { Owned<int>(x); }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::OwnedDeref(inner, _)) => {
                    assert!(matches!(inner.as_ref(), ast::Expr::Identifier(s, _) if s == "x"));
                }
                other => panic!("Expected OwnedDeref, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Chained postfix
    // -----------------------------------------------------------------------
    #[test]
    fn test_chained_postfix() {
        let src = r#"fn f(): void { obj.method().field[0]; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Index(_, _, _)) => {}
                other => panic!("Expected Index, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Bitwise operators
    // -----------------------------------------------------------------------
    #[test]
    fn test_bitwise_ops() {
        let src = r#"fn f(): int { return a | b & c ^ d << e >> f; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                // Left-to-right association: the outermost is the last operator applied
                ast::Stmt::Return(Some(ast::Expr::Binary(_, ast::Operator::BitShr, _, _))) => {}
                other => panic!("Expected BitShr (outermost with left-to-right), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Logical operators
    // -----------------------------------------------------------------------
    #[test]
    fn test_logical_ops() {
        let src = r#"fn f(): bool { return a && b || c; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Binary(_, ast::Operator::Or, _, _))) => {}
                other => panic!("Expected Or, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Else if
    // -----------------------------------------------------------------------
    #[test]
    fn test_else_if() {
        let src = r#"fn f(): void { if (a) { x; } else if (b) { y; } else { z; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::If(if_stmt) => {
                    assert!(if_stmt.else_branch.is_some());
                    let else_block = if_stmt.else_branch.as_ref().unwrap();
                    assert_eq!(else_block.len(), 1);
                    // else if becomes a nested If
                    match &else_block[0] {
                        ast::Stmt::If(nested) => {
                            assert!(nested.else_branch.is_some());
                        }
                        other => panic!("Expected nested If, got {:?}", other),
                    }
                }
                other => panic!("Expected If, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Class with extends and implements
    // -----------------------------------------------------------------------
    #[test]
    fn test_class_extends_implements() {
        let src = r#"class Dog extends Animal implements Printable {
            fn speak(): void { bark(); }
        }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Class(cd) => {
                assert_eq!(cd.name, "Dog");
                assert_eq!(cd.parent, Some(ast::Type::simple("Animal")));
                assert_eq!(cd.ifaces.len(), 1);
                assert_eq!(cd.ifaces[0], ast::Type::simple("Printable"));
            }
            other => panic!("Expected Class, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Multiple imports
    // -----------------------------------------------------------------------
    #[test]
    fn test_multiple_imports() {
        let src = r#"import io::println;
import math::sqrt;"#;
        let prog = parse_src(src).expect("parse should succeed");
        assert_eq!(prog.imports.len(), 2);
        assert_eq!(prog.imports[0].path, vec!["io", "println"]);
        assert_eq!(prog.imports[1].path, vec!["math", "sqrt"]);
    }

    // -----------------------------------------------------------------------
    // Empty program
    // -----------------------------------------------------------------------
    #[test]
    fn test_empty_program() {
        let src = "";
        let prog = parse_src(src).expect("parse should succeed");
        assert!(prog.imports.is_empty());
        assert!(prog.declarations.is_empty());
    }

    // -----------------------------------------------------------------------
    // Char literal expression
    // -----------------------------------------------------------------------
    #[test]
    fn test_char_expr() {
        let src = r#"fn f(): char { return 'a'; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Literal(ast::Literal::Char('a'), _))) => {}
                other => panic!("Expected Return(Char('a')), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Const at top level
    // -----------------------------------------------------------------------
    #[test]
    fn test_top_level_const() {
        let src = r#"const PI: double = 3.14;"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::ConstDecl(vd) => {
                assert_eq!(vd.name, "PI");
                assert!(!vd.mutable);
            }
            other => panic!("Expected ConstDecl, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Let at top level
    // -----------------------------------------------------------------------
    #[test]
    fn test_top_level_let() {
        let src = r#"let x: int = 10;"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::VarDecl(vd) => {
                assert_eq!(vd.name, "x");
                assert!(!vd.mutable);
            }
            other => panic!("Expected VarDecl, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Equality operators
    // -----------------------------------------------------------------------
    #[test]
    fn test_equality_ops() {
        let src = r#"fn f(): bool { return a == b != c; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Binary(_, ast::Operator::Ne, _, _))) => {}
                other => panic!("Expected Ne, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Comparison operators
    // -----------------------------------------------------------------------
    #[test]
    fn test_comparison_ops() {
        let src = r#"fn f(): bool { return a < b > c <= d >= e; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Binary(_, ast::Operator::Ge, _, _))) => {}
                other => panic!("Expected Ge, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // StaticCall — ClassName.method(args) via :: syntax
    // -----------------------------------------------------------------------
    #[test]
    fn test_static_call_via_coloncolon() {
        let src = r#"fn f(): void { Math::sqrt(4.0); }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Call(callee, _, _)) => {
                    // Math::sqrt becomes MemberAccess(Identifier("Math"), "sqrt")
                    assert!(matches!(callee.as_ref(), ast::Expr::MemberAccess(_, name, _) if name == "sqrt"));
                }
                other => panic!("Expected Call, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // For loop without var keyword
    // -----------------------------------------------------------------------
    #[test]
    fn test_for_without_var() {
        let src = r#"fn f(): void { for item in list { process(item); } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::For(fs) => {
                    assert_eq!(fs.var, "item");
                }
                other => panic!("Expected For, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // For with parens
    // -----------------------------------------------------------------------
    #[test]
    fn test_for_with_parens() {
        let src = r#"fn f(): void { for (item in list) { process(item); } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::For(fs) => {
                    assert_eq!(fs.var, "item");
                }
                other => panic!("Expected For, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_for_var_with_parens() {
        let src = r#"fn f(): void { for (var i in items) { continue; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::For(fs) => {
                    assert_eq!(fs.var, "i");
                }
                other => panic!("Expected For, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // While without parens
    // -----------------------------------------------------------------------
    #[test]
    fn test_while_no_parens() {
        let src = r#"fn f(): void { while true { break; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::While(_) => {}
                other => panic!("Expected While, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // String pattern in switch
    // -----------------------------------------------------------------------
    #[test]
    fn test_string_pattern() {
        let src = r#"fn f(): void { switch x { case "hello" => return; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Switch(ss) => {
                    assert_eq!(
                        ss.cases[0].pattern,
                        ast::Pattern::Literal(ast::Literal::String("hello".to_string()))
                    );
                }
                other => panic!("Expected Switch, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Bool pattern in switch
    // -----------------------------------------------------------------------
    #[test]
    fn test_bool_pattern() {
        let src = r#"fn f(): void { switch x { case true => return; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Switch(ss) => {
                    assert_eq!(
                        ss.cases[0].pattern,
                        ast::Pattern::Literal(ast::Literal::Bool(true))
                    );
                }
                other => panic!("Expected Switch, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Constructor pattern without bindings
    // -----------------------------------------------------------------------
    #[test]
    fn test_constructor_pattern_no_bindings() {
        let src = r#"fn f(): void { switch x { case None => return; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Switch(ss) => {
                    assert_eq!(
                        ss.cases[0].pattern,
                        ast::Pattern::Constructor {
                            name: "None".to_string(),
                            bindings: vec![]
                        }
                    );
                }
                other => panic!("Expected Switch, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Private access modifier
    // -----------------------------------------------------------------------
    #[test]
    fn test_private_fn() {
        let src = r#"private fn helper(): void { return; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => {
                assert_eq!(fd.access, ast::Access::Private);
            }
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Default access is private
    // -----------------------------------------------------------------------
    #[test]
    fn test_default_access() {
        let src = r#"fn f(): void { return; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => {
                assert_eq!(fd.access, ast::Access::Private);
            }
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Interface with extends
    // -----------------------------------------------------------------------
    #[test]
    fn test_interface_extends() {
        let src = r#"interface Serializable extends Base {
            fn serialize(): string;
        }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Interface(id) => {
                assert_eq!(id.name, "Serializable");
                assert_eq!(id.parents.len(), 1);
                assert_eq!(id.parents[0], ast::Type::simple("Base"));
            }
            other => panic!("Expected Interface, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Type parameters on declarations
    // -----------------------------------------------------------------------
    #[test]
    fn test_fn_type_params() {
        let src = r#"fn identity<T>(x: T): T { return x; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => {
                assert_eq!(fd.name, "identity");
                assert_eq!(fd.type_params, vec![ast::TypeParam { name: "T".to_string(), constraint: None }]);
            }
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_class_type_params() {
        let src = r#"class Container<T> { T value; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Class(cd) => {
                assert_eq!(cd.name, "Container");
                assert_eq!(cd.type_params, vec![ast::TypeParam { name: "T".to_string(), constraint: None }]);
            }
            other => panic!("Expected Class, got {:?}", other),
        }
    }

    #[test]
    fn test_interface_type_params() {
        let src = r#"interface Comparable<T> { fn compare(other: T): int; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Interface(id) => {
                assert_eq!(id.name, "Comparable");
                assert_eq!(id.type_params, vec![ast::TypeParam { name: "T".to_string(), constraint: None }]);
            }
            other => panic!("Expected Interface, got {:?}", other),
        }
    }

    #[test]
    fn test_enum_type_params() {
        let src = r#"enum Option<T> { Some(T), None }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Enum(ed) => {
                assert_eq!(ed.name, "Option");
                assert_eq!(ed.type_params, vec![ast::TypeParam { name: "T".to_string(), constraint: None }]);
            }
            other => panic!("Expected Enum, got {:?}", other),
        }
    }

    #[test]
    fn test_method_type_params() {
        let src = r#"class Foo { fn bar<U>(x: U): void { return; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Class(cd) => {
                assert_eq!(cd.members.len(), 1);
                match &cd.members[0] {
                    ast::ClassMember::Method(m) => {
                        assert_eq!(m.name, "bar");
                        assert_eq!(m.type_params, vec![ast::TypeParam { name: "U".to_string(), constraint: None }]);
                    }
                    other => panic!("Expected Method, got {:?}", other),
                }
            }
            other => panic!("Expected Class, got {:?}", other),
        }
    }

    #[test]
    fn test_multi_type_params() {
        let src = r#"class Map<K, V> { }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Class(cd) => {
                assert_eq!(cd.name, "Map");
                assert_eq!(cd.type_params, vec![ast::TypeParam { name: "K".to_string(), constraint: None }, ast::TypeParam { name: "V".to_string(), constraint: None }]);
            }
            other => panic!("Expected Class, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // C-style for loop tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_c_for_basic() {
        let src = r#"fn f(): void { for (let i = 0; i < 10; i = i + 1) { break; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::CFor(cfor) => {
                    // init should be a VarDecl for i
                    assert!(cfor.init.is_some());
                    match cfor.init.as_ref().unwrap().as_ref() {
                        ast::Stmt::VarDecl(vd) => {
                            assert_eq!(vd.name, "i");
                            assert!(!vd.mutable);
                        }
                        other => panic!("Expected VarDecl in init, got {:?}", other),
                    }
                    // condition should be a binary expr
                    assert!(cfor.condition.is_some());
                    assert!(matches!(cfor.condition.as_ref().unwrap(), ast::Expr::Binary(_, ast::Operator::Lt, _, _)));
                    // increment should be an assignment
                    assert!(cfor.increment.is_some());
                    assert!(matches!(cfor.increment.as_ref().unwrap(), ast::Expr::Assign(_, _, _)));
                    assert_eq!(cfor.body.len(), 1);
                }
                other => panic!("Expected CFor, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_c_for_with_var() {
        let src = r#"fn f(): void { for (var j = low; j < high; j = j + 1) { continue; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::CFor(cfor) => {
                    assert!(cfor.init.is_some());
                    match cfor.init.as_ref().unwrap().as_ref() {
                        ast::Stmt::VarDecl(vd) => {
                            assert_eq!(vd.name, "j");
                            assert!(vd.mutable);
                        }
                        other => panic!("Expected VarDecl (mutable) in init, got {:?}", other),
                    }
                }
                other => panic!("Expected CFor, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_c_for_with_method_call() {
        let src = r#"fn f(): void { for (let i = 0; i < numbers.size(); i = i + 1) { break; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::CFor(cfor) => {
                    // condition should include a method call
                    assert!(cfor.condition.is_some());
                }
                other => panic!("Expected CFor, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_c_for_no_init() {
        let src = r#"fn f(): void { for (; i < 10; i = i + 1) { break; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::CFor(cfor) => {
                    assert!(cfor.init.is_none());
                    assert!(cfor.condition.is_some());
                    assert!(cfor.increment.is_some());
                }
                other => panic!("Expected CFor, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_c_for_no_condition() {
        let src = r#"fn f(): void { for (let i = 0; ; i = i + 1) { break; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::CFor(cfor) => {
                    assert!(cfor.init.is_some());
                    assert!(cfor.condition.is_none());
                    assert!(cfor.increment.is_some());
                }
                other => panic!("Expected CFor, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_c_for_no_increment() {
        let src = r#"fn f(): void { for (let i = 0; i < 10; ) { break; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::CFor(cfor) => {
                    assert!(cfor.init.is_some());
                    assert!(cfor.condition.is_some());
                    assert!(cfor.increment.is_none());
                }
                other => panic!("Expected CFor, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_for_in_still_works_without_parens() {
        let src = r#"fn f(): void { for item in list { process(item); } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::For(fs) => {
                    assert_eq!(fs.var, "item");
                }
                other => panic!("Expected For (for-in), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_for_in_still_works_with_parens() {
        let src = r#"fn f(): void { for (item in list) { process(item); } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::For(fs) => {
                    assert_eq!(fs.var, "item");
                }
                other => panic!("Expected For (for-in), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_for_in_var_still_works_with_parens() {
        let src = r#"fn f(): void { for (var i in items) { continue; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::For(fs) => {
                    assert_eq!(fs.var, "i");
                }
                other => panic!("Expected For (for-in), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Operator overloading tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_operator_overload_method() {
        let src = r#"class Vec2 {
    double x;
    double y;

    fn operator+(self, other: Vec2): Vec2 {
        return new Vec2(this.x + other.x, this.y + other.y);
    }
}"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Class(cd) => {
                assert_eq!(cd.name, "Vec2");
                // Should have 2 fields and 1 method
                let methods: Vec<_> = cd.members.iter().filter_map(|m| match m {
                    ast::ClassMember::Method(m) => Some(m.name.clone()),
                    _ => None,
                }).collect();
                assert!(methods.contains(&"operator+".to_string()),
                    "Expected operator+ method, found methods: {:?}", methods);
            }
            other => panic!("Expected Class, got {:?}", other),
        }
    }

    #[test]
    fn test_operator_overload_comparison() {
        let src = r#"class Vec2 {
    double x;
    double y;

    fn operator==(self, other: Vec2): bool {
        return this.x == other.x && this.y == other.y;
    }

    fn operator!=(self, other: Vec2): bool {
        return !(this == other);
    }
}"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Class(cd) => {
                assert_eq!(cd.name, "Vec2");
                let methods: Vec<_> = cd.members.iter().filter_map(|m| match m {
                    ast::ClassMember::Method(m) => Some(m.name.clone()),
                    _ => None,
                }).collect();
                assert!(methods.contains(&"operator==".to_string()),
                    "Expected operator== method, found methods: {:?}", methods);
                assert!(methods.contains(&"operator!=".to_string()),
                    "Expected operator!= method, found methods: {:?}", methods);
            }
            other => panic!("Expected Class, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Tuple tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_tuple_two_elements() {
        let src = r#"fn f(): void { let x = (1, "hello"); }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => {
                assert_eq!(fd.body.len(), 1);
                match &fd.body[0] {
                    ast::Stmt::VarDecl(vd) => {
                        match &vd.init {
                            Some(ast::Expr::Tuple(elements, _)) => {
                                assert_eq!(elements.len(), 2);
                            }
                            other => panic!("Expected Tuple expression, got {:?}", other),
                        }
                    }
                    other => panic!("Expected VarDecl, got {:?}", other),
                }
            }
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_tuple_three_elements() {
        let src = r#"fn f(): void { let x = (1, 2, 3); }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => {
                assert_eq!(fd.body.len(), 1);
                match &fd.body[0] {
                    ast::Stmt::VarDecl(vd) => {
                        match &vd.init {
                            Some(ast::Expr::Tuple(elements, _)) => {
                                assert_eq!(elements.len(), 3);
                            }
                            other => panic!("Expected Tuple expression, got {:?}", other),
                        }
                    }
                    other => panic!("Expected VarDecl, got {:?}", other),
                }
            }
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_tuple_type_annotation() {
        let src = r#"fn f(): void { let x: (int, string) = (1, "a"); }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => {
                assert_eq!(fd.body.len(), 1);
                match &fd.body[0] {
                    ast::Stmt::VarDecl(vd) => {
                        match &vd.typ {
                            Some(ast::Type::Tuple(types)) => {
                                assert_eq!(types.len(), 2);
                            }
                            other => panic!("Expected Tuple type, got {:?}", other),
                        }
                    }
                    other => panic!("Expected VarDecl, got {:?}", other),
                }
            }
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_tuple_destructuring() {
        let src = r#"fn f(): void { let (a, b) = pair; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => {
                assert_eq!(fd.body.len(), 1);
                match &fd.body[0] {
                    ast::Stmt::TupleDestructure { names, mutable, .. } => {
                        assert_eq!(names.len(), 2);
                        assert_eq!(names[0], "a");
                        assert_eq!(names[1], "b");
                        assert!(!mutable);
                    }
                    other => panic!("Expected TupleDestructure, got {:?}", other),
                }
            }
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Closure tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_closure_expression_body() {
        let src = r#"fn f(): void { let g = fn(x) => x * 2; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => {
                assert_eq!(fd.body.len(), 1);
                match &fd.body[0] {
                    ast::Stmt::VarDecl(vd) => {
                        assert_eq!(vd.name, "g");
                        match &vd.init {
                            Some(ast::Expr::Closure { params, expr, body, .. }) => {
                                assert_eq!(params.len(), 1);
                                assert_eq!(params[0].0, "x");
                                assert!(expr.is_some(), "closure should have expression body");
                                assert!(body.is_empty(), "closure with => should have empty block body");
                            }
                            other => panic!("Expected Closure, got {:?}", other),
                        }
                    }
                    other => panic!("Expected VarDecl, got {:?}", other),
                }
            }
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_closure_block_body() {
        let src = r#"fn f(): void { let g = fn(x) { return x * 2; }; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => {
                assert_eq!(fd.body.len(), 1);
                match &fd.body[0] {
                    ast::Stmt::VarDecl(vd) => {
                        assert_eq!(vd.name, "g");
                        match &vd.init {
                            Some(ast::Expr::Closure { params, expr, body, .. }) => {
                                assert_eq!(params.len(), 1);
                                assert_eq!(params[0].0, "x");
                                assert!(expr.is_none(), "closure with block body should not have expression");
                                assert_eq!(body.len(), 1, "closure block body should have 1 statement");
                            }
                            other => panic!("Expected Closure, got {:?}", other),
                        }
                    }
                    other => panic!("Expected VarDecl, got {:?}", other),
                }
            }
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_closure_capture() {
        let src = r#"fn f(): void { let y = 10; let g = fn(x) => x + y; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => {
                assert_eq!(fd.body.len(), 2);
                // Second statement is the closure var decl
                match &fd.body[1] {
                    ast::Stmt::VarDecl(vd) => {
                        assert_eq!(vd.name, "g");
                        match &vd.init {
                            Some(ast::Expr::Closure { params, .. }) => {
                                assert_eq!(params.len(), 1);
                                assert_eq!(params[0].0, "x");
                            }
                            other => panic!("Expected Closure, got {:?}", other),
                        }
                    }
                    other => panic!("Expected VarDecl, got {:?}", other),
                }
            }
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Self parameter tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_self_parameter() {
        let src = r#"class Foo { fn method(self, x: int): void { } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Class(cd) => {
                assert_eq!(cd.name, "Foo");
                match &cd.members[0] {
                    ast::ClassMember::Method(m) => {
                        assert_eq!(m.name, "method");
                        assert_eq!(m.params.len(), 2);
                        assert_eq!(m.params[0].name, "self");
                        assert_eq!(m.params[0].typ, ast::Type::simple("Self"));
                        assert_eq!(m.params[1].name, "x");
                        assert_eq!(m.params[1].typ, ast::Type::simple("int"));
                    }
                    other => panic!("Expected Method, got {:?}", other),
                }
            }
            other => panic!("Expected Class, got {:?}", other),
        }
    }

    #[test]
    fn test_self_with_type() {
        let src = r#"class Vec2 { fn method(self: Vec2, x: int): void { } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Class(cd) => {
                assert_eq!(cd.name, "Vec2");
                match &cd.members[0] {
                    ast::ClassMember::Method(m) => {
                        assert_eq!(m.name, "method");
                        assert_eq!(m.params.len(), 2);
                        assert_eq!(m.params[0].name, "self");
                        assert_eq!(m.params[0].typ, ast::Type::simple("Vec2"));
                        assert_eq!(m.params[1].name, "x");
                        assert_eq!(m.params[1].typ, ast::Type::simple("int"));
                    }
                    other => panic!("Expected Method, got {:?}", other),
                }
            }
            other => panic!("Expected Class, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Stdlib import tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_import_tt_math() {
        let src = r#"import tt::math::Math;"#;
        let prog = parse_src(src).expect("parse should succeed");
        assert_eq!(prog.imports.len(), 1);
        assert_eq!(prog.imports[0].path, vec!["tt", "math", "Math"]);
    }

    #[test]
    fn test_import_tt_chem() {
        let src = r#"import tt::chem::Molecule;"#;
        let prog = parse_src(src).expect("parse should succeed");
        assert_eq!(prog.imports.len(), 1);
        assert_eq!(prog.imports[0].path, vec!["tt", "chem", "Molecule"]);
    }

    #[test]
    fn test_static_call_math() {
        let src = r#"fn f(): void { Math::sqrt(4.0); }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Call(callee, args, _)) => {
                    assert!(matches!(callee.as_ref(), ast::Expr::MemberAccess(_, name, _) if name == "sqrt"));
                    assert_eq!(args.len(), 1);
                }
                other => panic!("Expected Call, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_class_with_operator_overload_all() {
        let src = r#"class Number {
    double value;

    fn operator+(self, other: Number): Number {
        return new Number();
    }

    fn operator-(self, other: Number): Number {
        return new Number();
    }

    fn operator*(self, other: Number): Number {
        return new Number();
    }

    fn operator/(self, other: Number): Number {
        return new Number();
    }
}"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Class(cd) => {
                assert_eq!(cd.name, "Number");
                let methods: Vec<_> = cd.members.iter().filter_map(|m| match m {
                    ast::ClassMember::Method(m) => Some(m.name.clone()),
                    _ => None,
                }).collect();
                assert!(methods.contains(&"operator+".to_string()),
                    "Expected operator+ method, found: {:?}", methods);
                assert!(methods.contains(&"operator-".to_string()),
                    "Expected operator- method, found: {:?}", methods);
                assert!(methods.contains(&"operator*".to_string()),
                    "Expected operator* method, found: {:?}", methods);
                assert!(methods.contains(&"operator/".to_string()),
                    "Expected operator/ method, found: {:?}", methods);
            }
            other => panic!("Expected Class, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Import glob test
    // -----------------------------------------------------------------------
    #[test]
    fn test_import_glob() {
        let src = r#"import tt::util::*;"#;
        let prog = parse_src(src).expect("parse should succeed");
        assert_eq!(prog.imports.len(), 1);
        assert_eq!(prog.imports[0].path, vec!["tt", "util"]);
        assert!(prog.imports[0].glob);
    }

    // -----------------------------------------------------------------------
    // Import multiple (selective) test
    // -----------------------------------------------------------------------
    #[test]
    fn test_import_multiple() {
        // Multiple imports from the same module using separate import statements
        let src = r#"import tt::math::Math;
import tt::math::NDArray;"#;
        let prog = parse_src(src).expect("parse should succeed");
        assert_eq!(prog.imports.len(), 2);
        assert_eq!(prog.imports[0].path, vec!["tt", "math", "Math"]);
        assert_eq!(prog.imports[1].path, vec!["tt", "math", "NDArray"]);
    }

    // -----------------------------------------------------------------------
    // Dot-notation import tests (syntax sugar for ::)
    // -----------------------------------------------------------------------
    #[test]
    fn test_import_dot_notation() {
        let src = r#"import tt.math.Math;"#;
        let prog = parse_src(src).expect("parse should succeed");
        assert_eq!(prog.imports.len(), 1);
        assert_eq!(prog.imports[0].path, vec!["tt", "math", "Math"]);
    }

    #[test]
    fn test_import_dot_notation_deep() {
        let src = r#"import tt.math.linalg.Matrix;"#;
        let prog = parse_src(src).expect("parse should succeed");
        assert_eq!(prog.imports.len(), 1);
        assert_eq!(prog.imports[0].path, vec!["tt", "math", "linalg", "Matrix"]);
    }

    #[test]
    fn test_import_dot_notation_glob() {
        let src = r#"import tt.util.*;"#;
        let prog = parse_src(src).expect("parse should succeed");
        assert_eq!(prog.imports.len(), 1);
        assert_eq!(prog.imports[0].path, vec!["tt", "util"]);
        assert!(prog.imports[0].glob);
    }

    #[test]
    fn test_import_mixed_notation() {
        // Both notations should work in the same file
        let src = r#"import tt::math::Math;
import tt.chem.Molecule;"#;
        let prog = parse_src(src).expect("parse should succeed");
        assert_eq!(prog.imports.len(), 2);
        assert_eq!(prog.imports[0].path, vec!["tt", "math", "Math"]);
        assert_eq!(prog.imports[1].path, vec!["tt", "chem", "Molecule"]);
    }

    #[test]
    fn test_import_dot_notation_does_not_affect_expressions() {
        // Ensure that `a.b` in expressions is still parsed as member access, not import path
        let src = r#"fn f(): void { let x = obj.field; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::VarDecl(vd) => match &vd.init {
                    Some(ast::Expr::MemberAccess(obj, name, _)) => {
                        assert!(matches!(obj.as_ref(), ast::Expr::Identifier(ident, _) if ident == "obj"));
                        assert_eq!(name, "field");
                    }
                    other => panic!("Expected MemberAccess, got {:?}", other),
                },
                other => panic!("Expected VarDecl, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // For-in loop test
    // -----------------------------------------------------------------------
    #[test]
    fn test_for_in_loop() {
        let src = r#"fn f(): void { for (item in list) { process(item); } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::For(fs) => {
                    assert_eq!(fs.var, "item");
                    assert!(matches!(fs.iterable, ast::Expr::Identifier(_, _)));
                }
                other => panic!("Expected For, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // While-let test
    // -----------------------------------------------------------------------
    #[test]
    fn test_while_let() {
        let src = r#"fn f(): void { while let line = file.readLine() { io::println(line); } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::WhileLet(wl) => {
                    assert_eq!(wl.var_name, "line");
                }
                other => panic!("Expected WhileLet, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // C-style for loop test
    // -----------------------------------------------------------------------
    #[test]
    fn test_c_style_for() {
        let src = r#"fn f(): void { for (let i = 0; i < n; i = i + 1) { break; } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::CFor(cfor) => {
                    assert!(cfor.init.is_some());
                    assert!(cfor.condition.is_some());
                    assert!(cfor.increment.is_some());
                    // Verify init is a VarDecl
                    match cfor.init.as_ref().unwrap().as_ref() {
                        ast::Stmt::VarDecl(vd) => {
                            assert_eq!(vd.name, "i");
                            assert!(!vd.mutable);
                        }
                        other => panic!("Expected VarDecl in init, got {:?}", other),
                    }
                }
                other => panic!("Expected CFor, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Switch on enum value test
    // -----------------------------------------------------------------------
    #[test]
    fn test_switch_on_enum() {
        let src = r#"fn f(): void { switch color { case Red => return; case Blue => { break; } } }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Switch(ss) => {
                    assert_eq!(ss.cases.len(), 2);
                    assert_eq!(
                        ss.cases[0].pattern,
                        ast::Pattern::Constructor { name: "Red".to_string(), bindings: vec![] }
                    );
                    assert_eq!(
                        ss.cases[1].pattern,
                        ast::Pattern::Constructor { name: "Blue".to_string(), bindings: vec![] }
                    );
                }
                other => panic!("Expected Switch, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Where clause test
    // -----------------------------------------------------------------------
    #[test]
    fn test_where_clause() {
        let src = r#"fn foo<T>(x: T): void where T: Comparable<T> { return; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => {
                assert_eq!(fd.name, "foo");
                assert_eq!(fd.type_params.len(), 1);
                assert_eq!(fd.type_params[0].name, "T");
                // where clause should be parsed
                assert!(!fd.where_clause.is_empty(), "expected where clause to be parsed");
            }
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Range expression tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_exclusive_range() {
        let src = r#"fn f(): void { 1..10; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Range(start, end, _)) => {
                    assert!(matches!(start.as_ref(), ast::Expr::Literal(ast::Literal::Int(1), _)));
                    assert!(matches!(end.as_ref(), ast::Expr::Literal(ast::Literal::Int(10), _)));
                }
                other => panic!("Expected Range, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_inclusive_range() {
        let src = r#"fn f(): void { 1..=10; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::RangeInclusive(start, end, _)) => {
                    assert!(matches!(start.as_ref(), ast::Expr::Literal(ast::Literal::Int(1), _)));
                    assert!(matches!(end.as_ref(), ast::Expr::Literal(ast::Literal::Int(10), _)));
                }
                other => panic!("Expected RangeInclusive, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_range_with_expressions() {
        let src = r#"fn f(): void { a + 1..b * 2; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Range(start, end, _)) => {
                    assert!(matches!(start.as_ref(), ast::Expr::Binary(_, ast::Operator::Add, _, _)));
                    assert!(matches!(end.as_ref(), ast::Expr::Binary(_, ast::Operator::Mul, _, _)));
                }
                other => panic!("Expected Range with expressions, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_range_assignment() {
        let src = r#"fn f(): void { x = 1..10; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(_, value, _)) => {
                    assert!(matches!(value.as_ref(), ast::Expr::Range(_, _, _)),
                        "Expected Range on right side of assignment");
                }
                other => panic!("Expected Assign with Range, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Raw string literal expression tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_raw_string_expr() {
        let src = r#"fn f(): void { return r"hello"; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Literal(ast::Literal::String(s), _))) => {
                    assert_eq!(s, "hello");
                }
                other => panic!("Expected Return(String), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_raw_string_with_hash_expr() {
        let src = r##"fn f(): void { return r#"has "quotes""#; }"##;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Literal(ast::Literal::String(s), _))) => {
                    assert_eq!(s, "has \"quotes\"");
                }
                other => panic!("Expected Return(String), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Byte literal expression tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_byte_literal_expr() {
        let src = r#"fn f(): void { return b'x'; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Literal(ast::Literal::Int(120), _))) => {}
                other => panic!("Expected Return(Int(120)), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Numeric format expression tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_underscore_number_expr() {
        let src = r#"fn f(): int { return 1_000_000; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Literal(ast::Literal::Int(1000000), _))) => {}
                other => panic!("Expected Return(Int(1000000)), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_binary_expr() {
        let src = r#"fn f(): int { return 0b1010; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Literal(ast::Literal::Int(10), _))) => {}
                other => panic!("Expected Return(Int(10)), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_octal_expr() {
        let src = r#"fn f(): int { return 0o777; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Literal(ast::Literal::Int(511), _))) => {}
                other => panic!("Expected Return(Int(511)), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Compound assignment desugaring tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_compound_assign_plus_equal() {
        let src = r#"fn f(): void { x += 1; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(target, value, _)) => {
                    // target should be x
                    assert!(matches!(target.as_ref(), ast::Expr::Identifier(name, _) if name == "x"));
                    // value should be x + 1
                    match value.as_ref() {
                        ast::Expr::Binary(left, ast::Operator::Add, right, _) => {
                            assert!(matches!(left.as_ref(), ast::Expr::Identifier(name, _) if name == "x"));
                            assert!(matches!(right.as_ref(), ast::Expr::Literal(ast::Literal::Int(1), _)));
                        }
                        other => panic!("Expected Binary(Add), got {:?}", other),
                    }
                }
                other => panic!("Expected Assign, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_compound_assign_minus_equal() {
        let src = r#"fn f(): void { x -= 2; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(_, value, _)) => {
                    assert!(matches!(value.as_ref(), ast::Expr::Binary(_, ast::Operator::Sub, _, _)));
                }
                other => panic!("Expected Assign, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_compound_assign_star_equal() {
        let src = r#"fn f(): void { x *= 3; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(_, value, _)) => {
                    assert!(matches!(value.as_ref(), ast::Expr::Binary(_, ast::Operator::Mul, _, _)));
                }
                other => panic!("Expected Assign, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_compound_assign_slash_equal() {
        let src = r#"fn f(): void { x /= 4; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(_, value, _)) => {
                    assert!(matches!(value.as_ref(), ast::Expr::Binary(_, ast::Operator::Div, _, _)));
                }
                other => panic!("Expected Assign, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_compound_assign_percent_equal() {
        let src = r#"fn f(): void { x %= 5; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(_, value, _)) => {
                    assert!(matches!(value.as_ref(), ast::Expr::Binary(_, ast::Operator::Mod, _, _)));
                }
                other => panic!("Expected Assign, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_compound_assign_bitand_equal() {
        let src = r#"fn f(): void { x &= 3; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(_, value, _)) => {
                    assert!(matches!(value.as_ref(), ast::Expr::Binary(_, ast::Operator::BitAnd, _, _)));
                }
                other => panic!("Expected Assign, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_compound_assign_bitor_equal() {
        let src = r#"fn f(): void { x |= 3; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(_, value, _)) => {
                    assert!(matches!(value.as_ref(), ast::Expr::Binary(_, ast::Operator::BitOr, _, _)));
                }
                other => panic!("Expected Assign, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_compound_assign_bitxor_equal() {
        let src = r#"fn f(): void { x ^= 3; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(_, value, _)) => {
                    assert!(matches!(value.as_ref(), ast::Expr::Binary(_, ast::Operator::BitXor, _, _)));
                }
                other => panic!("Expected Assign, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_compound_assign_left_shift_equal() {
        let src = r#"fn f(): void { x <<= 2; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(_, value, _)) => {
                    assert!(matches!(value.as_ref(), ast::Expr::Binary(_, ast::Operator::BitShl, _, _)));
                }
                other => panic!("Expected Assign, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_compound_assign_right_shift_equal() {
        let src = r#"fn f(): void { x >>= 2; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(_, value, _)) => {
                    assert!(matches!(value.as_ref(), ast::Expr::Binary(_, ast::Operator::BitShr, _, _)));
                }
                other => panic!("Expected Assign, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_compound_assign_member_access() {
        // obj.field += 1 should desugar to obj.field = obj.field + 1
        let src = r#"fn f(): void { obj.field += 1; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(target, value, _)) => {
                    assert!(matches!(target.as_ref(), ast::Expr::MemberAccess(_, name, _) if name == "field"));
                    match value.as_ref() {
                        ast::Expr::Binary(left, ast::Operator::Add, right, _) => {
                            assert!(matches!(left.as_ref(), ast::Expr::MemberAccess(_, name, _) if name == "field"));
                            assert!(matches!(right.as_ref(), ast::Expr::Literal(ast::Literal::Int(1), _)));
                        }
                        other => panic!("Expected Binary(Add), got {:?}", other),
                    }
                }
                other => panic!("Expected Assign, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_compound_assign_index_access() {
        // arr[0] += 1 should desugar to arr[0] = arr[0] + 1
        let src = r#"fn f(): void { arr[0] += 1; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(target, value, _)) => {
                    assert!(matches!(target.as_ref(), ast::Expr::Index(_, _, _)));
                    match value.as_ref() {
                        ast::Expr::Binary(left, ast::Operator::Add, right, _) => {
                            assert!(matches!(left.as_ref(), ast::Expr::Index(_, _, _)));
                            assert!(matches!(right.as_ref(), ast::Expr::Literal(ast::Literal::Int(1), _)));
                        }
                        other => panic!("Expected Binary(Add), got {:?}", other),
                    }
                }
                other => panic!("Expected Assign, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Increment/decrement operator tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_prefix_increment() {
        // ++x desugars to x = x + 1
        let src = r#"fn f(): void { ++x; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(target, value, _)) => {
                    assert!(matches!(target.as_ref(), ast::Expr::Identifier(name, _) if name == "x"));
                    match value.as_ref() {
                        ast::Expr::Binary(left, ast::Operator::Add, right, _) => {
                            assert!(matches!(left.as_ref(), ast::Expr::Identifier(name, _) if name == "x"));
                            assert!(matches!(right.as_ref(), ast::Expr::Literal(ast::Literal::Int(1), _)));
                        }
                        other => panic!("Expected Binary(Add), got {:?}", other),
                    }
                }
                other => panic!("Expected Assign, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_prefix_decrement() {
        // --x desugars to x = x - 1
        let src = r#"fn f(): void { --x; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(target, value, _)) => {
                    assert!(matches!(target.as_ref(), ast::Expr::Identifier(name, _) if name == "x"));
                    match value.as_ref() {
                        ast::Expr::Binary(left, ast::Operator::Sub, right, _) => {
                            assert!(matches!(left.as_ref(), ast::Expr::Identifier(name, _) if name == "x"));
                            assert!(matches!(right.as_ref(), ast::Expr::Literal(ast::Literal::Int(1), _)));
                        }
                        other => panic!("Expected Binary(Sub), got {:?}", other),
                    }
                }
                other => panic!("Expected Assign, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_postfix_increment() {
        // x++ desugars to x = x + 1
        let src = r#"fn f(): void { x++; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(target, value, _)) => {
                    assert!(matches!(target.as_ref(), ast::Expr::Identifier(name, _) if name == "x"));
                    match value.as_ref() {
                        ast::Expr::Binary(left, ast::Operator::Add, right, _) => {
                            assert!(matches!(left.as_ref(), ast::Expr::Identifier(name, _) if name == "x"));
                            assert!(matches!(right.as_ref(), ast::Expr::Literal(ast::Literal::Int(1), _)));
                        }
                        other => panic!("Expected Binary(Add), got {:?}", other),
                    }
                }
                other => panic!("Expected Assign, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_postfix_decrement() {
        // x-- desugars to x = x - 1
        let src = r#"fn f(): void { x--; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(target, value, _)) => {
                    assert!(matches!(target.as_ref(), ast::Expr::Identifier(name, _) if name == "x"));
                    match value.as_ref() {
                        ast::Expr::Binary(left, ast::Operator::Sub, right, _) => {
                            assert!(matches!(left.as_ref(), ast::Expr::Identifier(name, _) if name == "x"));
                            assert!(matches!(right.as_ref(), ast::Expr::Literal(ast::Literal::Int(1), _)));
                        }
                        other => panic!("Expected Binary(Sub), got {:?}", other),
                    }
                }
                other => panic!("Expected Assign, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Ternary operator tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_ternary_simple() {
        let src = r#"fn f(): int { return true ? 1 : 2; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Ternary { condition, then_expr, else_expr, .. })) => {
                    assert!(matches!(condition.as_ref(), ast::Expr::Literal(ast::Literal::Bool(true), _)));
                    assert!(matches!(then_expr.as_ref(), ast::Expr::Literal(ast::Literal::Int(1), _)));
                    assert!(matches!(else_expr.as_ref(), ast::Expr::Literal(ast::Literal::Int(2), _)));
                }
                other => panic!("Expected Return(Ternary), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_ternary_with_expressions() {
        let src = r#"fn f(): int { return x > 0 ? x : -x; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Ternary { .. })) => {}
                other => panic!("Expected Return(Ternary), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_ternary_right_associative() {
        // a ? b : c ? d : e  should parse as  a ? b : (c ? d : e)
        let src = r#"fn f(): int { return a ? 1 : b ? 2 : 3; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Return(Some(ast::Expr::Ternary { else_expr, .. })) => {
                    // else_expr should be another Ternary
                    assert!(matches!(else_expr.as_ref(), ast::Expr::Ternary { .. }),
                        "Expected nested Ternary in else branch");
                }
                other => panic!("Expected Return(Ternary), got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_ternary_in_assignment() {
        let src = r#"fn f(): void { x = cond ? 1 : 0; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::Assign(_, value, _)) => {
                    assert!(matches!(value.as_ref(), ast::Expr::Ternary { .. }),
                        "Expected Ternary on right side of assignment");
                }
                other => panic!("Expected Assign with Ternary, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }

    #[test]
    fn test_error_propagation_still_works() {
        // x? at end of expression should still be error propagation
        let src = r#"fn f(): void { x?; }"#;
        let prog = parse_src(src).expect("parse should succeed");
        match &prog.declarations[0] {
            ast::Declaration::Function(fd) => match &fd.body[0] {
                ast::Stmt::Expr(ast::Expr::ErrorPropagation(_, _)) => {}
                other => panic!("Expected ErrorPropagation, got {:?}", other),
            },
            other => panic!("Expected Function, got {:?}", other),
        }
    }
}
