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
    pub(super) depth: usize,
}

/// Maximum recursion depth for the recursive descent parser.
///
/// This limit prevents stack overflows on pathologically deep input (e.g.
/// 10 000 nested parentheses). A legitimate program never approaches this
/// depth — typical code has fewer than 10 levels of expression nesting.
/// When exceeded, the parser returns a structured error instead of
/// crashing the process with STATUS_STACK_OVERFLOW.
const MAX_RECURSION_DEPTH: usize = 64;

impl Parser {
    pub(super) fn new(tokens: Vec<lexer::SpannedToken>) -> Self {
        Parser { tokens, pos: 0, depth: 0 }
    }

    /// Increment the recursion depth counter and enforce the maximum limit.
    ///
    /// Call this at the entry of every recursive parsing function
    /// (`parse_expression`, `parse_stmt`).  When the limit is exceeded, an
    /// `Err` is returned so the parser unwinds cleanly instead of
    /// overflowing the stack.
    pub(super) fn enter_recursion(&mut self) -> Result<(), String> {
        if self.depth >= MAX_RECURSION_DEPTH {
            let (line, col) = self.span_here();
            return Err(format!(
                "Maximum recursion depth {} exceeded at {}:{}",
                MAX_RECURSION_DEPTH, line, col
            ));
        }
        self.depth += 1;
        Ok(())
    }

    /// Decrement the recursion depth counter.
    ///
    /// Always call this in a matching pair with `enter_recursion`, after the
    /// recursive work completes (whether it succeeded or failed).
    pub(super) fn leave_recursion(&mut self) {
        self.depth -= 1;
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
            Err(self.err(format!("Expected {}, found {}", expected, current)))
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
            _ => Err(self.err(format!("Expected >, found {}", token))),
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
                .ok_or_else(|| self.err(format!("Expected name, found {}", t))),
            // `where` can be used as a function name (e.g. `fn where(...)`)
            lexer::Token::Where => Ok("where".to_string()),
            _ => Err(self.err(format!("Expected name, found {}", tok))),
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

    /// Build an error message annotated with the current line:column.
    /// Centralises the `"<msg> at L:C"` format so parser errors consistently
    /// carry source location for the diagnostic renderer.
    pub(super) fn err(&self, msg: impl Into<String>) -> String {
        let (line, col) = self.span_here();
        format!("{} at {}:{}", msg.into(), line, col)
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
                    _ => return Err(self.err(format!("Expected type parameter name, found {}", tok))),
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
                    _ => return Err(self.err(format!("Expected type parameter name in where clause, found {}", tok))),
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

#[cfg(test)]
mod tests;
