use super::*;

// ---------------------------------------------------------------------------
// Program
// ---------------------------------------------------------------------------

impl Parser {
    pub(super) fn parse_program(&mut self) -> Result<ast::Program, String> {
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
    pub(super) fn parse_import(&mut self) -> Result<ast::Import, String> {
        let span = self.make_span();
        self.expect(&lexer::Token::Import)?;
        let mut path = Vec::new();
        let mut glob = false;

        // First segment must be an identifier or keyword-as-name (e.g. Result)
        let first_tok = self.advance();
        let first = token_as_name(&first_tok)
            .ok_or_else(|| format!("Expected identifier in import path, found {}", first_tok))?;
        path.push(first);

        // Consume :: or . segments (both are valid import path separators)
        while self.match_token(&lexer::Token::ColonColon) || self.match_token(&lexer::Token::Dot) {
            // Check for glob star: :: * or . *
            if self.is_at(&lexer::Token::Star) {
                self.advance();
                glob = true;
                break;
            }
            let seg_tok = self.advance();
            let seg = token_as_name(&seg_tok)
                .ok_or_else(|| format!("Expected identifier in import path, found {}", seg_tok))?;
            path.push(seg);
        }

        self.expect(&lexer::Token::Semicolon)?;
        Ok(ast::Import { path, glob, span })
    }
}

// ---------------------------------------------------------------------------
// Declaration
// ---------------------------------------------------------------------------

impl Parser {
    pub(super) fn parse_declaration(&mut self) -> Result<ast::Declaration, String> {
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
    pub(super) fn parse_sugar_decl(&mut self, access: ast::Access) -> Result<ast::Declaration, String> {
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
            let where_clause = self.parse_where_clause()?;
            let body = self.parse_block()?;
            Ok(ast::Declaration::Function(ast::FnDecl {
                access,
                name,
                type_params: vec![],
                params,
                return_type: Some(return_type),
                body,
                sugar: true,
                where_clause,
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
    pub(super) fn parse_fn_decl(
        &mut self,
        access: ast::Access,
        sugar: bool,
    ) -> Result<ast::Declaration, String> {
        let span = self.make_span();
        let name = self.expect_name()?;

        let type_params = self.parse_type_params()?;

        self.expect(&lexer::Token::LeftParen)?;
        let params = self.parse_params()?;
        self.expect(&lexer::Token::RightParen)?;

        let return_type = if self.match_token(&lexer::Token::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };

        let where_clause = self.parse_where_clause()?;

        let body = self.parse_block()?;

        Ok(ast::Declaration::Function(ast::FnDecl {
            access,
            name,
            type_params,
            params,
            return_type,
            body,
            sugar,
            where_clause,
            span,
        }))
    }

    /// Parse canonical params: `name: Type, name: Type, ...`
    pub(super) fn parse_params(&mut self) -> Result<Vec<ast::Param>, String> {
        let mut params = Vec::new();
        if self.is_at(&lexer::Token::RightParen) {
            return Ok(params);
        }
        loop {
            let name_tok = self.advance();
            let name = token_as_name(&name_tok)
                .ok_or_else(|| format!("Expected parameter name, found {}", name_tok))?;
            // Support `self` without type annotation (type defaults to "Self")
            if name == "self" && !self.is_at(&lexer::Token::Colon) {
                params.push(ast::Param { name, typ: ast::Type::simple("Self") });
            } else {
                self.expect(&lexer::Token::Colon)?;
                let typ = self.parse_type()?;
                params.push(ast::Param { name, typ });
            }
            if !self.match_token(&lexer::Token::Comma) {
                break;
            }
        }
        Ok(params)
    }

    /// Parse sugar params: `Type name, Type name, ...`
    pub(super) fn parse_sugar_params(&mut self) -> Result<Vec<ast::Param>, String> {
        let mut params = Vec::new();
        if self.is_at(&lexer::Token::RightParen) {
            return Ok(params);
        }
        loop {
            let typ = self.parse_type()?;
            let name_tok = self.advance();
            let name = token_as_name(&name_tok)
                .ok_or_else(|| format!("Expected parameter name, found {}", name_tok))?;
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
    pub(super) fn parse_class_decl(&mut self, _access: ast::Access) -> Result<ast::Declaration, String> {
        let span = self.make_span();
        let name_tok = self.advance();
        let name = token_as_name(&name_tok)
            .ok_or_else(|| format!("Expected class name, found {}", name_tok))?;

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

    pub(super) fn parse_class_members(&mut self, class_name: &str) -> Result<Vec<ast::ClassMember>, String> {
        let mut members = Vec::new();
        while !self.is_at(&lexer::Token::RightBrace) && !self.is_eof() {
            members.push(self.parse_class_member(class_name)?);
        }
        Ok(members)
    }

    pub(super) fn parse_class_member(&mut self, class_name: &str) -> Result<ast::ClassMember, String> {
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
                        where_clause: vec![],
                        span,
                    }));
                } else {
                    // Not a constructor, backtrack
                    self.pos = saved;
                }
            }
        }

        // fn method(params): type { body }
        // But not if `fn` is followed by `(` (function type for a field, e.g. `fn(E): R mapper;`)
        if self.is_at(&lexer::Token::Fn) {
            let next = self.tokens.get(self.pos + 1).map(|st| &st.token);
            if next != Some(&lexer::Token::LeftParen) {
                self.advance();
                let span = self.make_span();
                let mut name = self.expect_name()?;
                // Support operator overloading: `fn operator+(...)` → name = "operator+"
                if name == "operator" {
                    let op_str = self.operator_token_to_str();
                    if let Some(s) = op_str {
                        name = format!("operator{}", s);
                        self.advance(); // consume the operator token
                    }
                }
                let type_params = self.parse_type_params()?;
                self.expect(&lexer::Token::LeftParen)?;
                let params = self.parse_params()?;
                self.expect(&lexer::Token::RightParen)?;
                let return_type = if self.match_token(&lexer::Token::Colon) {
                    Some(self.parse_type()?)
                } else {
                    None
                };
                let where_clause = self.parse_where_clause()?;
                let body = self.parse_block()?;
                // fn init(params) is the constructor
                if name == "init" {
                    return Ok(ast::ClassMember::Constructor(ast::MethodDecl {
                        access,
                        name: "new".to_string(),
                        type_params,
                        params,
                        return_type,
                        body,
                        where_clause,
                        span,
                    }));
                }
                return Ok(ast::ClassMember::Method(ast::MethodDecl {
                    access,
                    name,
                    type_params,
                    params,
                    return_type,
                    body,
                    where_clause,
                    span,
                }));
            }
        }

        // Titrate-style field declarations: var name: Type; let name: Type = expr; const name: Type = expr;
        if matches!(self.peek(), lexer::Token::Var | lexer::Token::Let | lexer::Token::Const) {
            self.advance(); // consume var/let/const
            let span = self.make_span();
            let name = self.expect_name()?;
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
            return Ok(ast::ClassMember::Field(ast::FieldDecl {
                access,
                name,
                typ: typ.unwrap_or_else(|| ast::Type::simple("void")),
                init,
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

            // Check if next is a name (identifier or type keyword like `size`)
            // that could be a method name or field name
            if let Some(name) = token_as_name(self.peek()) {
                self.advance();

                if self.is_at(&lexer::Token::LeftParen) {
                    // Sugar method: Type name(params) { body }
                    let span = self.make_span();
                    self.advance(); // consume '('
                    let params = self.parse_sugar_params()?;
                    self.expect(&lexer::Token::RightParen)?;
                    let where_clause = self.parse_where_clause()?;
                    let body = self.parse_block()?;
                    return Ok(ast::ClassMember::Method(ast::MethodDecl {
                        access,
                        name,
                        type_params: vec![],
                        params,
                        return_type: Some(return_type),
                        body,
                        where_clause,
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
        let name = match token_as_name(self.peek()) {
            Some(n) => {
                self.advance();
                n
            }
            None => return Err(format!("Expected field name, found {}", self.peek())),
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
    pub(super) fn parse_interface_decl(&mut self, _access: ast::Access) -> Result<ast::Declaration, String> {
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

    pub(super) fn parse_method_sig(&mut self) -> Result<ast::MethodSig, String> {
        // [public|private] fn name(params): type;
        // or with default body: [public|private] fn name(params): type { body }
        // Allow optional access modifier before fn
        let _ = self.match_token(&lexer::Token::Public) || self.match_token(&lexer::Token::Private);
        self.expect(&lexer::Token::Fn)?;
        let name = self.expect_name()?;
        let _type_params = self.parse_type_params()?;
        self.expect(&lexer::Token::LeftParen)?;
        let params = self.parse_params()?;
        self.expect(&lexer::Token::RightParen)?;
        let return_type = if self.match_token(&lexer::Token::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        // Check for default method body (interface default methods)
        let body = if self.is_at(&lexer::Token::LeftBrace) {
            Some(self.parse_block()?)
        } else {
            self.expect(&lexer::Token::Semicolon)?;
            None
        };
        Ok(ast::MethodSig {
            name,
            params,
            return_type,
            body,
        })
    }
}

// ---------------------------------------------------------------------------
// Enum declaration
// ---------------------------------------------------------------------------

impl Parser {
    pub(super) fn parse_enum_decl(&mut self, _access: ast::Access) -> Result<ast::Declaration, String> {
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

    pub(super) fn parse_variant(&mut self) -> Result<ast::Variant, String> {
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
    pub(super) fn parse_variant_fields(&mut self) -> Result<Vec<ast::Param>, String> {
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
    pub(super) fn is_variant_named_field(&self) -> bool {
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
