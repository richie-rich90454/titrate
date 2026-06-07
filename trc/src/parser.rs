// Phase 2: Parser — converts a token stream into an AST.
// All desugaring is performed here so the downstream passes see a clean tree.

use crate::{ast, lexer};

// ---------------------------------------------------------------------------
// Parser struct
// ---------------------------------------------------------------------------

struct Parser {
    tokens: Vec<lexer::SpannedToken>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<lexer::SpannedToken>) -> Self {
        Parser { tokens, pos: 0 }
    }

    /// Return a reference to the current token, or Eof if past the end.
    fn peek(&self) -> &lexer::Token {
        match self.tokens.get(self.pos) {
            Some(st) => &st.token,
            None => &lexer::Token::Eof,
        }
    }

    /// Consume and return the current token, advancing the position.
    fn advance(&mut self) -> lexer::Token {
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
    fn expect(&mut self, expected: &lexer::Token) -> Result<lexer::Token, String> {
        let current = self.peek().clone();
        if tokens_match(&current, expected) {
            Ok(self.advance())
        } else {
            Err(format!("Expected {}, found {}", expected, current))
        }
    }

    /// If the current token matches `expected`, consume it and return true;
    /// otherwise return false (no consumption).
    fn match_token(&mut self, expected: &lexer::Token) -> bool {
        if self.is_at(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Check whether the current token matches `expected` without consuming.
    fn is_at(&self, expected: &lexer::Token) -> bool {
        tokens_match(self.peek(), expected)
    }

    /// Check whether we have consumed all tokens.
    fn is_eof(&self) -> bool {
        matches!(self.peek(), lexer::Token::Eof)
    }

    /// Helper: get the current line/column for error messages.
    fn span_here(&self) -> (usize, usize) {
        match self.tokens.get(self.pos) {
            Some(st) => (st.line, st.column),
            None => (0, 0),
        }
    }

    /// Create an ast::Span from the current position.
    fn make_span(&self) -> ast::Span {
        let (line, col) = self.span_here();
        ast::Span::new(line as u32, col as u32)
    }

    /// Parse optional type parameters: `<T, U: Display, ...>`
    /// If the next token is `<`, parse a comma-separated list of identifiers
    /// (each optionally followed by `:` and an interface name) until `>`.
    /// Otherwise return an empty vec.
    fn parse_type_params(&mut self) -> Result<Vec<ast::TypeParam>, String> {
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
            self.expect(&lexer::Token::Greater)?;
            Ok(params)
        } else {
            Ok(vec![])
        }
    }
}

// ---------------------------------------------------------------------------
// Token comparison (structural equality, ignoring value in literals)
// ---------------------------------------------------------------------------

fn tokens_match(a: &lexer::Token, b: &lexer::Token) -> bool {
    // Always compare by variant discriminant only, so that
    // expect(Identifier(String::new())) matches any Identifier, etc.
    std::mem::discriminant(a) == std::mem::discriminant(b)
}

// ---------------------------------------------------------------------------
// Helpers for checking token categories
// ---------------------------------------------------------------------------

/// Tokens that represent a type keyword (used in sugar declarations).
fn is_type_keyword(tok: &lexer::Token) -> bool {
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
fn type_keyword_name(tok: &lexer::Token) -> Option<String> {
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
fn token_as_name(tok: &lexer::Token) -> Option<String> {
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
            _ => None,
        }),
    }
}

/// Check if a token can begin a type (keyword or identifier or type-like keywords).
fn is_type_start(tok: &lexer::Token) -> bool {
    is_type_keyword(tok)
        || matches!(tok, lexer::Token::Identifier(_))
        || matches!(tok, lexer::Token::Owned | lexer::Token::Result)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn parse(tokens: Vec<lexer::SpannedToken>) -> Result<ast::Program, String> {
    let mut parser = Parser::new(tokens);
    parser.parse_program()
}

// ---------------------------------------------------------------------------
// Program
// ---------------------------------------------------------------------------

impl Parser {
    fn parse_program(&mut self) -> Result<ast::Program, String> {
        let mut imports = Vec::new();
        let mut declarations = Vec::new();

        // Parse imports
        while self.is_at(&lexer::Token::Import) {
            imports.push(self.parse_import()?);
        }

        // Parse declarations until EOF
        while !self.is_eof() {
            declarations.push(self.parse_declaration()?);
        }

        Ok(ast::Program {
            imports,
            declarations,
        })
    }
}

// ---------------------------------------------------------------------------
// Import
// ---------------------------------------------------------------------------

impl Parser {
    fn parse_import(&mut self) -> Result<ast::Import, String> {
        let span = self.make_span();
        self.expect(&lexer::Token::Import)?;
        let mut path = Vec::new();

        // First segment must be an identifier
        let first = self.expect(&lexer::Token::Identifier(String::new()))?;
        match first {
            lexer::Token::Identifier(name) => path.push(name),
            _ => return Err(format!("Expected identifier in import path, found {}", first)),
        }

        // Consume :: segments
        while self.match_token(&lexer::Token::ColonColon) {
            let seg = self.expect(&lexer::Token::Identifier(String::new()))?;
            match seg {
                lexer::Token::Identifier(name) => path.push(name),
                _ => return Err(format!("Expected identifier in import path, found {}", seg)),
            }
        }

        self.expect(&lexer::Token::Semicolon)?;
        Ok(ast::Import { path, span })
    }
}

// ---------------------------------------------------------------------------
// Declaration
// ---------------------------------------------------------------------------

impl Parser {
    fn parse_declaration(&mut self) -> Result<ast::Declaration, String> {
        // Check for access modifier
        let access = if self.match_token(&lexer::Token::Public) {
            ast::Access::Public
        } else if self.match_token(&lexer::Token::Private) {
            ast::Access::Private
        } else {
            ast::Access::Private
        };

        match self.peek().clone() {
            lexer::Token::Fn => {
                self.advance();
                self.parse_fn_decl(access, false)
            }
            lexer::Token::Class => {
                self.advance();
                self.parse_class_decl(access)
            }
            lexer::Token::Interface => {
                self.advance();
                self.parse_interface_decl(access)
            }
            lexer::Token::Enum => {
                self.advance();
                self.parse_enum_decl(access)
            }
            lexer::Token::Let => {
                self.advance();
                let vd = self.parse_let_decl(false)?;
                Ok(ast::Declaration::VarDecl(vd))
            }
            lexer::Token::Var => {
                self.advance();
                let vd = self.parse_var_decl()?;
                Ok(ast::Declaration::VarDecl(vd))
            }
            lexer::Token::Const => {
                self.advance();
                let vd = self.parse_const_decl()?;
                Ok(ast::Declaration::ConstDecl(vd))
            }
            // Sugar: Type name(...) { ... } — type keyword before name means sugar fn
            tok if is_type_keyword(&tok) => {
                // Could be a sugar function or a typed variable declaration
                // Lookahead: Type Identifier ( => sugar fn
                //            Type Identifier = => typed var decl
                self.parse_sugar_decl(access)
            }
            _ => {
                let (line, col) = self.span_here();
                Err(format!(
                    "Expected declaration at {}:{}, found {}",
                    line, col, self.peek()
                ))
            }
        }
    }

    /// Parse sugar declarations where a type keyword precedes the name.
    /// `public void main() { ... }` → sugar fn
    /// `int x = 5;` → typed var decl
    fn parse_sugar_decl(&mut self, access: ast::Access) -> Result<ast::Declaration, String> {
        let span = self.make_span();
        let type_tok = self.advance();
        let type_name = type_keyword_name(&type_tok)
            .ok_or_else(|| format!("Expected type keyword, found {}", type_tok))?;
        let return_type = ast::Type::simple(&type_name);

        // Next must be an identifier
        let name_tok = self.expect(&lexer::Token::Identifier(String::new()))?;
        let name = match name_tok {
            lexer::Token::Identifier(s) => s,
            _ => return Err(format!("Expected identifier, found {}", name_tok)),
        };

        // If next is '(' it's a sugar function; otherwise it's a typed var decl
        if self.is_at(&lexer::Token::LeftParen) {
            // Sugar function declaration
            self.advance(); // consume '('
            let params = self.parse_sugar_params()?;
            self.expect(&lexer::Token::RightParen)?;
            let body = self.parse_block()?;
            Ok(ast::Declaration::Function(ast::FnDecl {
                access,
                name,
                type_params: vec![],
                params,
                return_type: Some(return_type),
                body,
                sugar: true,
                span,
            }))
        } else {
            // Typed variable declaration: Type name = expr;
            let mutable = true;
            let init = if self.match_token(&lexer::Token::Equals) {
                Some(self.parse_expression()?)
            } else {
                None
            };
            self.expect(&lexer::Token::Semicolon)?;
            Ok(ast::Declaration::VarDecl(ast::VarDecl {
                name,
                typ: Some(return_type),
                init,
                mutable,
                span,
            }))
        }
    }
}

// ---------------------------------------------------------------------------
// Function declaration (canonical: fn name(params): type { body })
// ---------------------------------------------------------------------------

impl Parser {
    fn parse_fn_decl(
        &mut self,
        access: ast::Access,
        sugar: bool,
    ) -> Result<ast::Declaration, String> {
        let span = self.make_span();
        let name_tok = self.expect(&lexer::Token::Identifier(String::new()))?;
        let name = match name_tok {
            lexer::Token::Identifier(s) => s,
            _ => return Err(format!("Expected function name, found {}", name_tok)),
        };

        let type_params = self.parse_type_params()?;

        self.expect(&lexer::Token::LeftParen)?;
        let params = self.parse_params()?;
        self.expect(&lexer::Token::RightParen)?;

        let return_type = if self.match_token(&lexer::Token::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };

        let body = self.parse_block()?;

        Ok(ast::Declaration::Function(ast::FnDecl {
            access,
            name,
            type_params,
            params,
            return_type,
            body,
            sugar,
            span,
        }))
    }

    /// Parse canonical params: `name: Type, name: Type, ...`
    fn parse_params(&mut self) -> Result<Vec<ast::Param>, String> {
        let mut params = Vec::new();
        if self.is_at(&lexer::Token::RightParen) {
            return Ok(params);
        }
        loop {
            let name_tok = self.expect(&lexer::Token::Identifier(String::new()))?;
            let name = match name_tok {
                lexer::Token::Identifier(s) => s,
                _ => return Err(format!("Expected parameter name, found {}", name_tok)),
            };
            self.expect(&lexer::Token::Colon)?;
            let typ = self.parse_type()?;
            params.push(ast::Param { name, typ });
            if !self.match_token(&lexer::Token::Comma) {
                break;
            }
        }
        Ok(params)
    }

    /// Parse sugar params: `Type name, Type name, ...`
    fn parse_sugar_params(&mut self) -> Result<Vec<ast::Param>, String> {
        let mut params = Vec::new();
        if self.is_at(&lexer::Token::RightParen) {
            return Ok(params);
        }
        loop {
            let typ = self.parse_type()?;
            let name_tok = self.expect(&lexer::Token::Identifier(String::new()))?;
            let name = match name_tok {
                lexer::Token::Identifier(s) => s,
                _ => return Err(format!("Expected parameter name, found {}", name_tok)),
            };
            params.push(ast::Param { name, typ });
            if !self.match_token(&lexer::Token::Comma) {
                break;
            }
        }
        Ok(params)
    }
}

// ---------------------------------------------------------------------------
// Class declaration
// ---------------------------------------------------------------------------

impl Parser {
    fn parse_class_decl(&mut self, _access: ast::Access) -> Result<ast::Declaration, String> {
        let span = self.make_span();
        let name_tok = self.expect(&lexer::Token::Identifier(String::new()))?;
        let name = match name_tok {
            lexer::Token::Identifier(s) => s,
            _ => return Err(format!("Expected class name, found {}", name_tok)),
        };

        let type_params = self.parse_type_params()?;

        let parent = if self.match_token(&lexer::Token::Extends) {
            Some(self.parse_type()?)
        } else {
            None
        };

        let mut ifaces = Vec::new();
        if self.match_token(&lexer::Token::Implements) {
            loop {
                ifaces.push(self.parse_type()?);
                if !self.match_token(&lexer::Token::Comma) {
                    break;
                }
            }
        }

        self.expect(&lexer::Token::LeftBrace)?;
        let members = self.parse_class_members(&name)?;
        self.expect(&lexer::Token::RightBrace)?;

        Ok(ast::Declaration::Class(ast::ClassDecl {
            name,
            type_params,
            parent,
            ifaces,
            members,
            span,
        }))
    }

    fn parse_class_members(&mut self, class_name: &str) -> Result<Vec<ast::ClassMember>, String> {
        let mut members = Vec::new();
        while !self.is_at(&lexer::Token::RightBrace) && !self.is_eof() {
            members.push(self.parse_class_member(class_name)?);
        }
        Ok(members)
    }

    fn parse_class_member(&mut self, class_name: &str) -> Result<ast::ClassMember, String> {
        let access = if self.match_token(&lexer::Token::Public) {
            ast::Access::Public
        } else if self.match_token(&lexer::Token::Private) {
            ast::Access::Private
        } else {
            ast::Access::Private
        };

        // Check for constructor sugar: `ClassName(params) { body }`
        if let lexer::Token::Identifier(ref n) = self.peek().clone() {
            if n == class_name {
                // Lookahead: is next token '('?
                let saved = self.pos;
                let _tok = self.advance();
                if self.is_at(&lexer::Token::LeftParen) {
                    let span = self.make_span();
                    self.advance(); // consume '('
                    let params = self.parse_sugar_params()?;
                    self.expect(&lexer::Token::RightParen)?;
                    let body = self.parse_block()?;
                    return Ok(ast::ClassMember::Constructor(ast::MethodDecl {
                        access,
                        name: "new".to_string(),
                        type_params: vec![],
                        params,
                        return_type: None,
                        body,
                        span,
                    }));
                } else {
                    // Not a constructor, backtrack
                    self.pos = saved;
                }
            }
        }

        // fn method(params): type { body }
        if self.is_at(&lexer::Token::Fn) {
            self.advance();
            let span = self.make_span();
            let name_tok = self.expect(&lexer::Token::Identifier(String::new()))?;
            let name = match name_tok {
                lexer::Token::Identifier(s) => s,
                _ => return Err(format!("Expected method name, found {}", name_tok)),
            };
            let type_params = self.parse_type_params()?;
            self.expect(&lexer::Token::LeftParen)?;
            let params = self.parse_params()?;
            self.expect(&lexer::Token::RightParen)?;
            let return_type = if self.match_token(&lexer::Token::Colon) {
                Some(self.parse_type()?)
            } else {
                None
            };
            let body = self.parse_block()?;
            return Ok(ast::ClassMember::Method(ast::MethodDecl {
                access,
                name,
                type_params,
                params,
                return_type,
                body,
                span,
            }));
        }

        // Sugar method or typed field: Type name...
        // Need lookahead to distinguish:
        //   Type name(params) { body }  → sugar method
        //   Type name; or Type name = expr;  → typed field
        //   name: Type [= expr];  → field with colon syntax
        if is_type_start(self.peek()) {
            // Save position for lookahead
            let saved = self.pos;
            let return_type = self.parse_type()?;

            // Check if next is an identifier (could be method name or field name)
            if let lexer::Token::Identifier(_) = self.peek() {
                let name_tok = self.advance();
                let name = match name_tok {
                    lexer::Token::Identifier(s) => s,
                    _ => return Err(format!("Expected name, found {}", name_tok)),
                };

                if self.is_at(&lexer::Token::LeftParen) {
                    // Sugar method: Type name(params) { body }
                    let span = self.make_span();
                    self.advance(); // consume '('
                    let params = self.parse_sugar_params()?;
                    self.expect(&lexer::Token::RightParen)?;
                    let body = self.parse_block()?;
                    return Ok(ast::ClassMember::Method(ast::MethodDecl {
                        access,
                        name,
                        type_params: vec![],
                        params,
                        return_type: Some(return_type),
                        body,
                        span,
                    }));
                } else {
                    // Typed field: Type name [= expr];
                    let span = self.make_span();
                    let init = if self.match_token(&lexer::Token::Equals) {
                        Some(self.parse_expression()?)
                    } else {
                        None
                    };
                    self.expect(&lexer::Token::Semicolon)?;
                    return Ok(ast::ClassMember::Field(ast::FieldDecl {
                        access,
                        name,
                        typ: return_type,
                        init,
                        span,
                    }));
                }
            } else {
                // The type was actually a field name followed by colon — backtrack
                self.pos = saved;
            }
        }

        // Field: name: Type or name: Type = expr
        let span = self.make_span();
        let name_tok = self.expect(&lexer::Token::Identifier(String::new()))?;
        let name = match name_tok {
            lexer::Token::Identifier(s) => s,
            _ => return Err(format!("Expected field name, found {}", name_tok)),
        };
        self.expect(&lexer::Token::Colon)?;
        let typ = self.parse_type()?;
        let init = if self.match_token(&lexer::Token::Equals) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        self.expect(&lexer::Token::Semicolon)?;

        Ok(ast::ClassMember::Field(ast::FieldDecl {
            access,
            name,
            typ,
            init,
            span,
        }))
    }
}

// ---------------------------------------------------------------------------
// Interface declaration
// ---------------------------------------------------------------------------

impl Parser {
    fn parse_interface_decl(&mut self, _access: ast::Access) -> Result<ast::Declaration, String> {
        let span = self.make_span();
        let name_tok = self.expect(&lexer::Token::Identifier(String::new()))?;
        let name = match name_tok {
            lexer::Token::Identifier(s) => s,
            _ => return Err(format!("Expected interface name, found {}", name_tok)),
        };

        let type_params = self.parse_type_params()?;

        let mut parents = Vec::new();
        if self.match_token(&lexer::Token::Extends) {
            loop {
                parents.push(self.parse_type()?);
                if !self.match_token(&lexer::Token::Comma) {
                    break;
                }
            }
        }

        self.expect(&lexer::Token::LeftBrace)?;
        let mut methods = Vec::new();
        while !self.is_at(&lexer::Token::RightBrace) && !self.is_eof() {
            methods.push(self.parse_method_sig()?);
        }
        self.expect(&lexer::Token::RightBrace)?;

        Ok(ast::Declaration::Interface(ast::InterfaceDecl {
            name,
            type_params,
            parents,
            methods,
            span,
        }))
    }

    fn parse_method_sig(&mut self) -> Result<ast::MethodSig, String> {
        // fn name(params): type;
        self.expect(&lexer::Token::Fn)?;
        let name_tok = self.expect(&lexer::Token::Identifier(String::new()))?;
        let name = match name_tok {
            lexer::Token::Identifier(s) => s,
            _ => return Err(format!("Expected method name, found {}", name_tok)),
        };
        self.expect(&lexer::Token::LeftParen)?;
        let params = self.parse_params()?;
        self.expect(&lexer::Token::RightParen)?;
        let return_type = if self.match_token(&lexer::Token::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect(&lexer::Token::Semicolon)?;
        Ok(ast::MethodSig {
            name,
            params,
            return_type,
        })
    }
}

// ---------------------------------------------------------------------------
// Enum declaration
// ---------------------------------------------------------------------------

impl Parser {
    fn parse_enum_decl(&mut self, _access: ast::Access) -> Result<ast::Declaration, String> {
        let span = self.make_span();
        let name_tok = self.expect(&lexer::Token::Identifier(String::new()))?;
        let name = match name_tok {
            lexer::Token::Identifier(s) => s,
            _ => return Err(format!("Expected enum name, found {}", name_tok)),
        };

        let type_params = self.parse_type_params()?;

        self.expect(&lexer::Token::LeftBrace)?;
        let mut variants = Vec::new();
        while !self.is_at(&lexer::Token::RightBrace) && !self.is_eof() {
            variants.push(self.parse_variant()?);
            let _ = self.match_token(&lexer::Token::Comma);
        }
        self.expect(&lexer::Token::RightBrace)?;

        Ok(ast::Declaration::Enum(ast::EnumDecl {
            name,
            type_params,
            variants,
            span,
        }))
    }

    fn parse_variant(&mut self) -> Result<ast::Variant, String> {
        let name_tok = self.expect(&lexer::Token::Identifier(String::new()))?;
        let name = match name_tok {
            lexer::Token::Identifier(s) => s,
            _ => return Err(format!("Expected variant name, found {}", name_tok)),
        };

        let fields = if self.match_token(&lexer::Token::LeftParen) {
            let f = self.parse_variant_fields()?;
            self.expect(&lexer::Token::RightParen)?;
            f
        } else {
            Vec::new()
        };

        Ok(ast::Variant { name, fields })
    }

    /// Parse variant fields — supports both `name: Type` and bare `Type` forms.
    fn parse_variant_fields(&mut self) -> Result<Vec<ast::Param>, String> {
        let mut fields = Vec::new();
        if self.is_at(&lexer::Token::RightParen) {
            return Ok(fields);
        }
        let mut idx = 0;
        loop {
            // Try to determine if this is `name: Type` or just `Type`
            // If the current token is a type keyword or type-like keyword, and the next
            // token is a colon, then it's `name: Type`. Otherwise it's just `Type`.
            // If the current token is an identifier, it could be either:
            //   - `name: Type` (if followed by colon)
            //   - A type name (if followed by comma or right paren or less)
            if self.is_variant_named_field() {
                // name: Type
                let name_tok = self.expect(&lexer::Token::Identifier(String::new()))?;
                let name = match name_tok {
                    lexer::Token::Identifier(s) => s,
                    _ => return Err(format!("Expected field name, found {}", name_tok)),
                };
                self.expect(&lexer::Token::Colon)?;
                let typ = self.parse_type()?;
                fields.push(ast::Param { name, typ });
            } else {
                // Just a type — generate a synthetic name
                let typ = self.parse_type()?;
                let name = format!("_{}", idx);
                idx += 1;
                fields.push(ast::Param { name, typ });
            }
            if !self.match_token(&lexer::Token::Comma) {
                break;
            }
        }
        Ok(fields)
    }

    /// Check if the current position looks like a named field `name: Type`
    /// by peeking ahead for a colon after the current token.
    fn is_variant_named_field(&self) -> bool {
        // Look ahead: if we see Identifier followed by Colon, it's a named field
        if let Some(st) = self.tokens.get(self.pos) {
            if let lexer::Token::Identifier(_) = &st.token {
                // Check if the next token is a colon
                if let Some(next) = self.tokens.get(self.pos + 1) {
                    return matches!(next.token, lexer::Token::Colon);
                }
            }
        }
        false
    }
}

// ---------------------------------------------------------------------------
// Variable declarations
// ---------------------------------------------------------------------------

impl Parser {
    /// Parse `let x: type = expr;` or `let x = expr;`
    fn parse_let_decl(&mut self, mutable: bool) -> Result<ast::VarDecl, String> {
        let span = self.make_span();
        let name_tok = self.expect(&lexer::Token::Identifier(String::new()))?;
        let name = match name_tok {
            lexer::Token::Identifier(s) => s,
            _ => return Err(format!("Expected variable name, found {}", name_tok)),
        };

        let typ = if self.match_token(&lexer::Token::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };

        let init = if self.match_token(&lexer::Token::Equals) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.expect(&lexer::Token::Semicolon)?;
        Ok(ast::VarDecl {
            name,
            typ,
            init,
            mutable,
            span,
        })
    }

    /// Parse `var x = expr;` — desugar to let with mutable=true
    fn parse_var_decl(&mut self) -> Result<ast::VarDecl, String> {
        let span = self.make_span();
        let name_tok = self.expect(&lexer::Token::Identifier(String::new()))?;
        let name = match name_tok {
            lexer::Token::Identifier(s) => s,
            _ => return Err(format!("Expected variable name, found {}", name_tok)),
        };

        let typ = if self.match_token(&lexer::Token::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };

        let init = if self.match_token(&lexer::Token::Equals) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.expect(&lexer::Token::Semicolon)?;
        Ok(ast::VarDecl {
            name,
            typ,
            init,
            mutable: true,
            span,
        })
    }

    /// Parse `const X: type = expr;`
    fn parse_const_decl(&mut self) -> Result<ast::VarDecl, String> {
        let span = self.make_span();
        let name_tok = self.expect(&lexer::Token::Identifier(String::new()))?;
        let name = match name_tok {
            lexer::Token::Identifier(s) => s,
            _ => return Err(format!("Expected constant name, found {}", name_tok)),
        };

        let typ = if self.match_token(&lexer::Token::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };

        self.expect(&lexer::Token::Equals)?;
        let init = self.parse_expression()?;
        self.expect(&lexer::Token::Semicolon)?;
        Ok(ast::VarDecl {
            name,
            typ,
            init: Some(init),
            mutable: false,
            span,
        })
    }
}

// ---------------------------------------------------------------------------
// Statements
// ---------------------------------------------------------------------------

impl Parser {
    fn parse_stmt(&mut self) -> Result<ast::Stmt, String> {
        match self.peek().clone() {
            lexer::Token::LeftBrace => {
                let block = self.parse_block()?;
                Ok(ast::Stmt::Block(block))
            }
            lexer::Token::If => {
                self.advance();
                self.parse_if_stmt()
            }
            lexer::Token::While => {
                self.advance();
                self.parse_while_stmt()
            }
            lexer::Token::For => {
                self.advance();
                self.parse_for_stmt()
            }
            lexer::Token::Return => {
                self.advance();
                self.parse_return_stmt()
            }
            lexer::Token::Break => {
                self.advance();
                self.expect(&lexer::Token::Semicolon)?;
                Ok(ast::Stmt::Break)
            }
            lexer::Token::Continue => {
                self.advance();
                self.expect(&lexer::Token::Semicolon)?;
                Ok(ast::Stmt::Continue)
            }
            lexer::Token::Switch => {
                self.advance();
                self.parse_switch_stmt()
            }
            lexer::Token::Let => {
                self.advance();
                let vd = self.parse_let_decl(false)?;
                Ok(ast::Stmt::VarDecl(vd))
            }
            lexer::Token::Var => {
                self.advance();
                let vd = self.parse_var_decl()?;
                Ok(ast::Stmt::VarDecl(vd))
            }
            lexer::Token::Const => {
                self.advance();
                let vd = self.parse_const_decl()?;
                Ok(ast::Stmt::ConstDecl(vd))
            }
            // unsafe { ... } as a statement — no semicolon needed
            lexer::Token::Unsafe => {
                self.advance();
                let span = self.make_span();
                let block = self.parse_block()?;
                Ok(ast::Stmt::Expr(ast::Expr::UnsafeBlock(block, span)))
            }
            // region name { ... } as a statement — no semicolon needed
            lexer::Token::Region => {
                self.advance();
                let span = self.make_span();
                let _name_tok = self.expect(&lexer::Token::Identifier(String::new()))?;
                let block = self.parse_block()?;
                Ok(ast::Stmt::Expr(ast::Expr::UnsafeBlock(block, span)))
            }
            // Type name = expr; → desugar to let with type and mutable=true
            tok if is_type_keyword(&tok) => {
                let span = self.make_span();
                let type_tok = self.advance();
                let type_name = type_keyword_name(&type_tok)
                    .ok_or_else(|| format!("Expected type keyword, found {}", type_tok))?;
                let typ = ast::Type::simple(&type_name);

                let name_tok = self.expect(&lexer::Token::Identifier(String::new()))?;
                let name = match name_tok {
                    lexer::Token::Identifier(s) => s,
                    _ => return Err(format!("Expected variable name, found {}", name_tok)),
                };

                let init = if self.match_token(&lexer::Token::Equals) {
                    Some(self.parse_expression()?)
                } else {
                    None
                };
                self.expect(&lexer::Token::Semicolon)?;
                Ok(ast::Stmt::VarDecl(ast::VarDecl {
                    name,
                    typ: Some(typ),
                    init,
                    mutable: true,
                    span,
                }))
            }
            _ => {
                // Expression statement
                let expr = self.parse_expression()?;
                self.expect(&lexer::Token::Semicolon)?;
                Ok(ast::Stmt::Expr(expr))
            }
        }
    }

    fn parse_block(&mut self) -> Result<ast::Block, String> {
        self.expect(&lexer::Token::LeftBrace)?;
        let mut stmts = Vec::new();
        while !self.is_at(&lexer::Token::RightBrace) && !self.is_eof() {
            stmts.push(self.parse_stmt()?);
        }
        self.expect(&lexer::Token::RightBrace)?;
        Ok(stmts)
    }

    fn parse_if_stmt(&mut self) -> Result<ast::Stmt, String> {
        let span = self.make_span();
        // Condition may or may not be in parens
        let condition = if self.match_token(&lexer::Token::LeftParen) {
            let expr = self.parse_expression()?;
            self.expect(&lexer::Token::RightParen)?;
            expr
        } else {
            self.parse_expression()?
        };

        let then_branch = self.parse_block()?;

        let else_branch = if self.match_token(&lexer::Token::Else) {
            if self.is_at(&lexer::Token::If) {
                // else if
                self.advance();
                let stmt = self.parse_if_stmt()?;
                Some(vec![stmt])
            } else {
                Some(self.parse_block()?)
            }
        } else {
            None
        };

        Ok(ast::Stmt::If(ast::IfStmt {
            condition,
            then_branch,
            else_branch,
            span,
        }))
    }

    fn parse_while_stmt(&mut self) -> Result<ast::Stmt, String> {
        let span = self.make_span();
        let condition = if self.match_token(&lexer::Token::LeftParen) {
            let expr = self.parse_expression()?;
            self.expect(&lexer::Token::RightParen)?;
            expr
        } else {
            self.parse_expression()?
        };

        let body = self.parse_block()?;
        Ok(ast::Stmt::While(ast::WhileStmt { condition, body, span }))
    }

    fn parse_for_stmt(&mut self) -> Result<ast::Stmt, String> {
        let span = self.make_span();
        // for ([var] name in expr) { body }  or  for [var] name in expr { body }
        let has_parens = self.match_token(&lexer::Token::LeftParen);
        let _var_kw = self.match_token(&lexer::Token::Var); // optional 'var'
        let name_tok = self.expect(&lexer::Token::Identifier(String::new()))?;
        let var = match name_tok {
            lexer::Token::Identifier(s) => s,
            _ => return Err(format!("Expected loop variable, found {}", name_tok)),
        };
        // Expect 'in' as an identifier
        let in_tok = self.expect(&lexer::Token::Identifier(String::new()))?;
        match &in_tok {
            lexer::Token::Identifier(s) if s == "in" => {}
            _ => return Err(format!("Expected 'in' in for loop, found {}", in_tok)),
        }
        let iterable = self.parse_expression()?;
        if has_parens {
            self.expect(&lexer::Token::RightParen)?;
        }
        let body = self.parse_block()?;
        Ok(ast::Stmt::For(ast::ForStmt {
            var,
            iterable,
            body,
            span,
        }))
    }

    fn parse_return_stmt(&mut self) -> Result<ast::Stmt, String> {
        if self.match_token(&lexer::Token::Semicolon) {
            Ok(ast::Stmt::Return(None))
        } else {
            let expr = self.parse_expression()?;
            self.expect(&lexer::Token::Semicolon)?;
            Ok(ast::Stmt::Return(Some(expr)))
        }
    }

    fn parse_switch_stmt(&mut self) -> Result<ast::Stmt, String> {
        let span = self.make_span();
        let expr = self.parse_expression()?;
        self.expect(&lexer::Token::LeftBrace)?;

        let mut cases = Vec::new();
        let mut default = None;

        while !self.is_at(&lexer::Token::RightBrace) && !self.is_eof() {
            if self.match_token(&lexer::Token::Case) {
                let pattern = self.parse_pattern()?;
                self.expect(&lexer::Token::FatArrow)?;
                let body = self.parse_case_body()?;
                cases.push(ast::Case { pattern, body });
            } else if self.match_token(&lexer::Token::Default) {
                self.expect(&lexer::Token::FatArrow)?;
                let body = self.parse_case_body()?;
                default = Some(body);
            } else {
                let (line, col) = self.span_here();
                return Err(format!(
                    "Expected 'case' or 'default' in switch at {}:{}, found {}",
                    line, col, self.peek()
                ));
            }
        }

        self.expect(&lexer::Token::RightBrace)?;
        Ok(ast::Stmt::Switch(ast::SwitchStmt {
            expr,
            cases,
            default,
            span,
        }))
    }

    /// Parse the body of a switch case — either a single statement or a block.
    fn parse_case_body(&mut self) -> Result<ast::Block, String> {
        if self.is_at(&lexer::Token::LeftBrace) {
            self.parse_block()
        } else {
            let stmt = self.parse_stmt()?;
            Ok(vec![stmt])
        }
    }
}

// ---------------------------------------------------------------------------
// Pattern parsing
// ---------------------------------------------------------------------------

impl Parser {
    fn parse_pattern(&mut self) -> Result<ast::Pattern, String> {
        match self.peek().clone() {
            lexer::Token::IntLiteral(_) => {
                let tok = self.advance();
                match tok {
                    lexer::Token::IntLiteral(v) => {
                        Ok(ast::Pattern::Literal(ast::Literal::Int(v)))
                    }
                    _ => Err("Expected int literal".to_string()),
                }
            }
            lexer::Token::FloatLiteral { .. } => {
                let tok = self.advance();
                match tok {
                    lexer::Token::FloatLiteral { value, .. } => {
                        Ok(ast::Pattern::Literal(ast::Literal::Float(value)))
                    }
                    _ => Err("Expected float literal".to_string()),
                }
            }
            lexer::Token::StringLiteral(_) => {
                let tok = self.advance();
                match tok {
                    lexer::Token::StringLiteral(s) => {
                        Ok(ast::Pattern::Literal(ast::Literal::String(s)))
                    }
                    _ => Err("Expected string literal".to_string()),
                }
            }
            lexer::Token::CharLiteral(_) => {
                let tok = self.advance();
                match tok {
                    lexer::Token::CharLiteral(c) => {
                        Ok(ast::Pattern::Literal(ast::Literal::Char(c)))
                    }
                    _ => Err("Expected char literal".to_string()),
                }
            }
            lexer::Token::BoolLiteral(_) => {
                let tok = self.advance();
                match tok {
                    lexer::Token::BoolLiteral(b) => {
                        Ok(ast::Pattern::Literal(ast::Literal::Bool(b)))
                    }
                    _ => Err("Expected bool literal".to_string()),
                }
            }
            lexer::Token::NullLiteral => {
                self.advance();
                Ok(ast::Pattern::Literal(ast::Literal::Null))
            }
            lexer::Token::Identifier(ref s) if s == "_" => {
                self.advance();
                Ok(ast::Pattern::Wildcard)
            }
            lexer::Token::Ok | lexer::Token::Err => {
                // Ok(v) and Err(e) are Result variant patterns
                let name = match self.advance() {
                    lexer::Token::Ok => "Ok".to_string(),
                    lexer::Token::Err => "Err".to_string(),
                    _ => return Err("Expected Ok or Err".to_string()),
                };
                if self.match_token(&lexer::Token::LeftParen) {
                    let mut bindings = Vec::new();
                    if !self.is_at(&lexer::Token::RightParen) {
                        loop {
                            // Allow wildcard _ as binding
                            let b = self.advance();
                            match b {
                                lexer::Token::Identifier(ref s) if s == "_" => bindings.push("_".to_string()),
                                lexer::Token::Identifier(s) => bindings.push(s),
                                _ => return Err(format!("Expected binding name, found {}", b)),
                            }
                            if !self.match_token(&lexer::Token::Comma) {
                                break;
                            }
                        }
                    }
                    self.expect(&lexer::Token::RightParen)?;
                    Ok(ast::Pattern::Constructor { name, bindings })
                } else {
                    Ok(ast::Pattern::Constructor { name, bindings: Vec::new() })
                }
            }
            lexer::Token::Identifier(_) => {
                let tok = self.advance();
                let name = match tok {
                    lexer::Token::Identifier(s) => s,
                    _ => return Err("Expected identifier".to_string()),
                };
                if self.match_token(&lexer::Token::LeftParen) {
                    let mut bindings = Vec::new();
                    if !self.is_at(&lexer::Token::RightParen) {
                        loop {
                            let b = self.expect(&lexer::Token::Identifier(String::new()))?;
                            match b {
                                lexer::Token::Identifier(s) => bindings.push(s),
                                _ => return Err(format!("Expected binding name, found {}", b)),
                            }
                            if !self.match_token(&lexer::Token::Comma) {
                                break;
                            }
                        }
                    }
                    self.expect(&lexer::Token::RightParen)?;
                    Ok(ast::Pattern::Constructor { name, bindings })
                } else {
                    Ok(ast::Pattern::Constructor {
                        name,
                        bindings: Vec::new(),
                    })
                }
            }
            _ => {
                let (line, col) = self.span_here();
                Err(format!(
                    "Expected pattern at {}:{}, found {}",
                    line, col, self.peek()
                ))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Type parsing
// ---------------------------------------------------------------------------

impl Parser {
    fn parse_type(&mut self) -> Result<ast::Type, String> {
        let tok = self.peek().clone();
        let name = if is_type_keyword(&tok) {
            self.advance();
            type_keyword_name(&tok)
                .ok_or_else(|| format!("Expected type keyword, found {}", tok))?
        } else if matches!(tok, lexer::Token::Owned) {
            self.advance();
            "Owned".to_string()
        } else if matches!(tok, lexer::Token::Result) {
            self.advance();
            "Result".to_string()
        } else {
            let id_tok = self.expect(&lexer::Token::Identifier(String::new()))?;
            match id_tok {
                lexer::Token::Identifier(s) => s,
                _ => return Err(format!("Expected type name, found {}", id_tok)),
            }
        };

        // Check for generic parameters <Type, Type, ...>
        let params = if self.match_token(&lexer::Token::Less) {
            let mut ps = Vec::new();
            loop {
                ps.push(self.parse_type()?);
                if !self.match_token(&lexer::Token::Comma) {
                    break;
                }
            }
            self.expect(&lexer::Token::Greater)?;
            ps
        } else {
            Vec::new()
        };

        Ok(ast::Type::Named { name, params })
    }
}

// ---------------------------------------------------------------------------
// Expression parsing — precedence climbing
// ---------------------------------------------------------------------------

impl Parser {
    fn parse_expression(&mut self) -> Result<ast::Expr, String> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<ast::Expr, String> {
        let expr = self.parse_or()?;

        if self.match_token(&lexer::Token::Equals) {
            let span = self.make_span();
            let value = self.parse_assignment()?; // right-associative
            // The left side must be an lvalue — we check at the AST level
            Ok(ast::Expr::Assign(Box::new(expr), Box::new(value), span))
        } else {
            Ok(expr)
        }
    }

    fn parse_or(&mut self) -> Result<ast::Expr, String> {
        let mut left = self.parse_and()?;
        while self.match_token(&lexer::Token::OrOr) {
            let span = self.make_span();
            let right = self.parse_and()?;
            left = ast::Expr::Binary(Box::new(left), ast::Operator::Or, Box::new(right), span);
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<ast::Expr, String> {
        let mut left = self.parse_equality()?;
        while self.match_token(&lexer::Token::AndAnd) {
            let span = self.make_span();
            let right = self.parse_equality()?;
            left = ast::Expr::Binary(Box::new(left), ast::Operator::And, Box::new(right), span);
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<ast::Expr, String> {
        let mut left = self.parse_comparison()?;
        loop {
            let op = if self.match_token(&lexer::Token::EqualEqual) {
                ast::Operator::Eq
            } else if self.match_token(&lexer::Token::NotEqual) {
                ast::Operator::Ne
            } else {
                break;
            };
            let span = self.make_span();
            let right = self.parse_comparison()?;
            left = ast::Expr::Binary(Box::new(left), op, Box::new(right), span);
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<ast::Expr, String> {
        let mut left = self.parse_bitwise()?;
        loop {
            let op = if self.match_token(&lexer::Token::Less) {
                ast::Operator::Lt
            } else if self.match_token(&lexer::Token::Greater) {
                ast::Operator::Gt
            } else if self.match_token(&lexer::Token::LessEqual) {
                ast::Operator::Le
            } else if self.match_token(&lexer::Token::GreaterEqual) {
                ast::Operator::Ge
            } else {
                break;
            };
            let span = self.make_span();
            let right = self.parse_bitwise()?;
            left = ast::Expr::Binary(Box::new(left), op, Box::new(right), span);
        }
        Ok(left)
    }

    fn parse_bitwise(&mut self) -> Result<ast::Expr, String> {
        let mut left = self.parse_addition()?;
        loop {
            let op = if self.match_token(&lexer::Token::Pipe) {
                ast::Operator::BitOr
            } else if self.match_token(&lexer::Token::Caret) {
                ast::Operator::BitXor
            } else if self.match_token(&lexer::Token::Ampersand) {
                ast::Operator::BitAnd
            } else if self.match_token(&lexer::Token::LeftShift) {
                ast::Operator::BitShl
            } else if self.match_token(&lexer::Token::RightShift) {
                ast::Operator::BitShr
            } else {
                break;
            };
            let span = self.make_span();
            let right = self.parse_addition()?;
            left = ast::Expr::Binary(Box::new(left), op, Box::new(right), span);
        }
        Ok(left)
    }

    fn parse_addition(&mut self) -> Result<ast::Expr, String> {
        let mut left = self.parse_multiplication()?;
        loop {
            let op = if self.match_token(&lexer::Token::Plus) {
                ast::Operator::Add
            } else if self.match_token(&lexer::Token::Minus) {
                ast::Operator::Sub
            } else {
                break;
            };
            let span = self.make_span();
            let right = self.parse_multiplication()?;
            left = ast::Expr::Binary(Box::new(left), op, Box::new(right), span);
        }
        Ok(left)
    }

    fn parse_multiplication(&mut self) -> Result<ast::Expr, String> {
        let mut left = self.parse_unary()?;
        loop {
            let op = if self.match_token(&lexer::Token::Star) {
                ast::Operator::Mul
            } else if self.match_token(&lexer::Token::Slash) {
                ast::Operator::Div
            } else if self.match_token(&lexer::Token::Percent) {
                ast::Operator::Mod
            } else {
                break;
            };
            let span = self.make_span();
            let right = self.parse_unary()?;
            left = ast::Expr::Binary(Box::new(left), op, Box::new(right), span);
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<ast::Expr, String> {
        if self.match_token(&lexer::Token::Not) {
            let span = self.make_span();
            let expr = self.parse_unary()?;
            return Ok(ast::Expr::Unary(ast::UnOp::Not, Box::new(expr), span));
        }
        if self.match_token(&lexer::Token::Minus) {
            let span = self.make_span();
            let expr = self.parse_unary()?;
            return Ok(ast::Expr::Unary(ast::UnOp::Neg, Box::new(expr), span));
        }
        if self.match_token(&lexer::Token::Tilde) {
            let span = self.make_span();
            let expr = self.parse_unary()?;
            return Ok(ast::Expr::Unary(ast::UnOp::BitNot, Box::new(expr), span));
        }
        if self.match_token(&lexer::Token::Star) {
            // *expr — dereference (OwnedDeref or raw pointer deref)
            let span = self.make_span();
            let expr = self.parse_unary()?;
            return Ok(ast::Expr::OwnedDeref(Box::new(expr), span));
        }
        if self.match_token(&lexer::Token::Ampersand) {
            // &expr or &mut expr
            let span = self.make_span();
            if self.match_token(&lexer::Token::RefMut) {
                // This shouldn't happen since &mut is lexed as RefMut, but handle just in case
                let expr = self.parse_unary()?;
                return Ok(ast::Expr::RefExpr(Box::new(expr), ast::RefKind::Mutable, span));
            }
            let expr = self.parse_unary()?;
            return Ok(ast::Expr::RefExpr(Box::new(expr), ast::RefKind::Immutable, span));
        }
        if self.match_token(&lexer::Token::RefMut) {
            // &mut expr
            let span = self.make_span();
            let expr = self.parse_unary()?;
            return Ok(ast::Expr::RefExpr(Box::new(expr), ast::RefKind::Mutable, span));
        }
        self.parse_postfix()
    }

    // Heuristic for desugar: richie-rich90454's magic
    fn parse_postfix(&mut self) -> Result<ast::Expr, String> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.match_token(&lexer::Token::LeftParen) {
                // Function call
                let span = self.make_span();
                let args = self.parse_args()?;
                self.expect(&lexer::Token::RightParen)?;
                expr = ast::Expr::Call(Box::new(expr), args, span);
            } else if self.match_token(&lexer::Token::Dot) {
                // Member access — field name can be an identifier or a keyword used as a name
                let span = self.make_span();
                let name_tok = self.advance();
                let name = token_as_name(&name_tok)
                    .ok_or_else(|| format!("Expected member name, found {}", name_tok))?;
                expr = ast::Expr::MemberAccess(Box::new(expr), name, span);
            } else if self.match_token(&lexer::Token::ColonColon) {
                // :: namespace access → treat as member access
                let span = self.make_span();
                let name_tok = self.advance();
                let name = token_as_name(&name_tok)
                    .ok_or_else(|| format!("Expected namespace member, found {}", name_tok))?;
                expr = ast::Expr::MemberAccess(Box::new(expr), name, span);
            } else if self.match_token(&lexer::Token::LeftBracket) {
                // Index access
                let span = self.make_span();
                let index = self.parse_expression()?;
                self.expect(&lexer::Token::RightBracket)?;
                expr = ast::Expr::Index(Box::new(expr), Box::new(index), span);
            } else if self.match_token(&lexer::Token::Question) {
                // Error propagation
                let span = self.make_span();
                expr = ast::Expr::ErrorPropagation(Box::new(expr), span);
            } else if self.match_token(&lexer::Token::As) {
                // Cast
                let span = self.make_span();
                let typ = self.parse_type()?;
                expr = ast::Expr::Cast(Box::new(expr), typ, span);
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_args(&mut self) -> Result<Vec<ast::Expr>, String> {
        let mut args = Vec::new();
        if self.is_at(&lexer::Token::RightParen) {
            return Ok(args);
        }
        loop {
            args.push(self.parse_expression()?);
            if !self.match_token(&lexer::Token::Comma) {
                break;
            }
        }
        Ok(args)
    }

    fn parse_primary(&mut self) -> Result<ast::Expr, String> {
        let tok = self.peek().clone();

        match tok {
            lexer::Token::IntLiteral(_) => {
                let span = self.make_span();
                let t = self.advance();
                match t {
                    lexer::Token::IntLiteral(v) => {
                        Ok(ast::Expr::Literal(ast::Literal::Int(v), span))
                    }
                    _ => Err("Expected int literal".to_string()),
                }
            }
            lexer::Token::FloatLiteral { .. } => {
                let span = self.make_span();
                let t = self.advance();
                match t {
                    lexer::Token::FloatLiteral { value, .. } => {
                        Ok(ast::Expr::Literal(ast::Literal::Float(value), span))
                    }
                    _ => Err("Expected float literal".to_string()),
                }
            }
            lexer::Token::StringLiteral(_) => {
                let span = self.make_span();
                let t = self.advance();
                match t {
                    lexer::Token::StringLiteral(s) => {
                        Ok(ast::Expr::Literal(ast::Literal::String(s), span))
                    }
                    _ => Err("Expected string literal".to_string()),
                }
            }
            lexer::Token::CharLiteral(_) => {
                let span = self.make_span();
                let t = self.advance();
                match t {
                    lexer::Token::CharLiteral(c) => {
                        Ok(ast::Expr::Literal(ast::Literal::Char(c), span))
                    }
                    _ => Err("Expected char literal".to_string()),
                }
            }
            lexer::Token::BoolLiteral(_) => {
                let span = self.make_span();
                let t = self.advance();
                match t {
                    lexer::Token::BoolLiteral(b) => {
                        Ok(ast::Expr::Literal(ast::Literal::Bool(b), span))
                    }
                    _ => Err("Expected bool literal".to_string()),
                }
            }
            lexer::Token::NullLiteral => {
                let span = self.make_span();
                self.advance();
                Ok(ast::Expr::Literal(ast::Literal::Null, span))
            }
            lexer::Token::Identifier(_) => {
                let span = self.make_span();
                let t = self.advance();
                match t {
                    lexer::Token::Identifier(s) => {
                        Ok(ast::Expr::Identifier(s, span))
                    }
                    _ => Err("Expected identifier".to_string()),
                }
            }
            lexer::Token::This => {
                let span = self.make_span();
                self.advance();
                Ok(ast::Expr::This(span))
            }
            lexer::Token::Super => {
                let span = self.make_span();
                self.advance();
                Ok(ast::Expr::Super(span))
            }
            lexer::Token::LeftParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(&lexer::Token::RightParen)?;
                Ok(expr)
            }
            lexer::Token::New => {
                let span = self.make_span();
                self.advance();
                let typ = self.parse_type()?;
                self.expect(&lexer::Token::LeftParen)?;
                let args = self.parse_args()?;
                self.expect(&lexer::Token::RightParen)?;
                Ok(ast::Expr::New(typ, args, span))
            }
            lexer::Token::Ok => {
                let span = self.make_span();
                self.advance();
                self.expect(&lexer::Token::LeftParen)?;
                let expr = self.parse_expression()?;
                self.expect(&lexer::Token::RightParen)?;
                Ok(ast::Expr::Call(
                    Box::new(ast::Expr::Identifier("Ok".to_string(), span)),
                    vec![expr],
                    span,
                ))
            }
            lexer::Token::Err => {
                let span = self.make_span();
                self.advance();
                self.expect(&lexer::Token::LeftParen)?;
                let expr = self.parse_expression()?;
                self.expect(&lexer::Token::RightParen)?;
                Ok(ast::Expr::Call(
                    Box::new(ast::Expr::Identifier("Err".to_string(), span)),
                    vec![expr],
                    span,
                ))
            }
            lexer::Token::Unsafe => {
                let span = self.make_span();
                self.advance();
                let block = self.parse_block()?;
                Ok(ast::Expr::UnsafeBlock(block, span))
            }
            lexer::Token::Region => {
                let span = self.make_span();
                self.advance();
                // region name { block }
                let _name_tok = self.expect(&lexer::Token::Identifier(String::new()))?;
                let block = self.parse_block()?;
                // Represent region as an unsafe block for now; the interpreter
                // handles region.alloc calls specially.
                Ok(ast::Expr::UnsafeBlock(block, span))
            }
            lexer::Token::Owned => {
                // Owned<type>(expr) — let parse_type() consume Owned<int>
                let span = self.make_span();
                let _typ = self.parse_type()?;
                self.expect(&lexer::Token::LeftParen)?;
                let expr = self.parse_expression()?;
                self.expect(&lexer::Token::RightParen)?;
                Ok(ast::Expr::OwnedDeref(Box::new(expr), span))
            }
            _ => {
                let (line, col) = self.span_here();
                Err(format!(
                    "Expected expression at {}:{}, found {}",
                    line, col, self.peek()
                ))
            }
        }
    }
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
}
