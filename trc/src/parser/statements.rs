use super::*;

// ---------------------------------------------------------------------------
// Variable declarations
// ---------------------------------------------------------------------------

impl Parser {
    /// Parse `let x: type = expr;` or `let x = expr;`
    pub(super) fn parse_let_decl(&mut self, mutable: bool) -> Result<ast::VarDecl, String> {
        let span = self.make_span();
        let name_tok = self.advance();
        let name = token_as_name(&name_tok)
            .ok_or_else(|| format!("Expected variable name, found {}", name_tok))?;

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
    pub(super) fn parse_var_decl(&mut self) -> Result<ast::VarDecl, String> {
        let span = self.make_span();
        let name_tok = self.advance();
        let name = token_as_name(&name_tok)
            .ok_or_else(|| format!("Expected variable name, found {}", name_tok))?;

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
    pub(super) fn parse_const_decl(&mut self) -> Result<ast::VarDecl, String> {
        let span = self.make_span();
        let name_tok = self.advance();
        let name = token_as_name(&name_tok)
            .ok_or_else(|| format!("Expected constant name, found {}", name_tok))?;

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

    /// Parse tuple destructuring: `let (a, b, ...) = expr;` or `var (a, b, ...) = expr;`
    /// Also supports optional type annotations: `let (a: Type, b: Type) = expr;`
    /// and overall type: `let (a, b): Type = expr;`
    pub(super) fn parse_tuple_destructure(&mut self, mutable: bool) -> Result<ast::Stmt, String> {
        let span = self.make_span();
        self.expect(&lexer::Token::LeftParen)?;
        let mut names = Vec::new();
        loop {
            let name_tok = self.advance();
            let name = token_as_name(&name_tok)
                .ok_or_else(|| format!("Expected identifier in tuple destructuring, found {}", name_tok))?;
            names.push(name);
            // Skip optional per-element type annotation: `name: Type`
            if self.match_token(&lexer::Token::Colon) {
                let _ = self.parse_type()?;
            }
            if !self.match_token(&lexer::Token::Comma) {
                break;
            }
        }
        self.expect(&lexer::Token::RightParen)?;
        // Skip optional overall type annotation: `(a, b): Type`
        if self.match_token(&lexer::Token::Colon) {
            let _ = self.parse_type()?;
        }
        self.expect(&lexer::Token::Equals)?;
        let expr = self.parse_expression()?;
        self.expect(&lexer::Token::Semicolon)?;
        Ok(ast::Stmt::TupleDestructure { names, expr, mutable, span })
    }
}

// ---------------------------------------------------------------------------
// Statements
// ---------------------------------------------------------------------------

impl Parser {
    pub(super) fn parse_stmt(&mut self) -> Result<ast::Stmt, String> {
        match self.peek().clone() {
            lexer::Token::LeftBrace => {
                let block = self.parse_block()?;
                Ok(ast::Stmt::Block(block))
            }
            lexer::Token::If => {
                self.advance();
                self.parse_if_stmt()
            }
            lexer::Token::Do => {
                self.advance();
                self.parse_do_while_stmt()
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
            lexer::Token::With => {
                self.advance();
                self.parse_with_stmt()
            }
            lexer::Token::Throw => {
                self.advance();
                let span = self.make_span();
                let expr = self.parse_expression()?;
                self.expect(&lexer::Token::Semicolon)?;
                Ok(ast::Stmt::Throw(expr, span))
            }
            lexer::Token::Try => {
                self.advance();
                self.parse_try_catch_stmt()
            }
            lexer::Token::Let => {
                self.advance();
                // Check for tuple destructuring: let (a, b, ...) = expr;
                if self.is_at(&lexer::Token::LeftParen) {
                    self.parse_tuple_destructure(false)
                } else {
                    let vd = self.parse_let_decl(false)?;
                    Ok(ast::Stmt::VarDecl(vd))
                }
            }
            lexer::Token::Var => {
                self.advance();
                // Check for tuple destructuring: var (a, b, ...) = expr;
                if self.is_at(&lexer::Token::LeftParen) {
                    self.parse_tuple_destructure(true)
                } else {
                    let vd = self.parse_var_decl()?;
                    Ok(ast::Stmt::VarDecl(vd))
                }
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

                let name_tok = self.advance();
                let name = token_as_name(&name_tok)
                    .ok_or_else(|| format!("Expected variable name, found {}", name_tok))?;

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

    pub(super) fn parse_block(&mut self) -> Result<ast::Block, String> {
        self.expect(&lexer::Token::LeftBrace)?;
        let mut stmts = Vec::new();
        while !self.is_at(&lexer::Token::RightBrace) && !self.is_eof() {
            stmts.push(self.parse_stmt()?);
        }
        self.expect(&lexer::Token::RightBrace)?;
        Ok(stmts)
    }

    pub(super) fn parse_if_stmt(&mut self) -> Result<ast::Stmt, String> {
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

    pub(super) fn parse_while_stmt(&mut self) -> Result<ast::Stmt, String> {
        let span = self.make_span();

        // Check for while-let: while let name = expr { body }
        if self.is_at(&lexer::Token::Let) {
            self.advance(); // consume 'let'
            let name_tok = self.expect(&lexer::Token::Identifier(String::new()))?;
            let var_name = match name_tok {
                lexer::Token::Identifier(s) => s,
                _ => return Err(format!("Expected identifier after 'while let', found {}", name_tok)),
            };
            self.expect(&lexer::Token::Equals)?;
            let expr = if self.match_token(&lexer::Token::LeftParen) {
                let e = self.parse_expression()?;
                self.expect(&lexer::Token::RightParen)?;
                e
            } else {
                self.parse_expression()?
            };
            let body = self.parse_block()?;
            return Ok(ast::Stmt::WhileLet(ast::WhileLetStmt {
                var_name,
                expr,
                body,
                span,
            }));
        }

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

    pub(super) fn parse_do_while_stmt(&mut self) -> Result<ast::Stmt, String> {
        let span = self.make_span();

        // Parse body: do { ... }
        let body = self.parse_block()?;

        // Expect 'while'
        let while_tok = self.expect(&lexer::Token::While)?;
        if !matches!(while_tok, lexer::Token::While) {
            return Err(format!("Expected 'while' after do block, found {}", while_tok));
        }

        // Parse condition: while (expr)
        let condition = if self.match_token(&lexer::Token::LeftParen) {
            let expr = self.parse_expression()?;
            self.expect(&lexer::Token::RightParen)?;
            expr
        } else {
            self.parse_expression()?
        };

        // Optional trailing semicolon
        let _ = self.match_token(&lexer::Token::Semicolon);

        Ok(ast::Stmt::DoWhile(ast::DoWhileStmt { body, condition, span }))
    }

    pub(super) fn parse_for_stmt(&mut self) -> Result<ast::Stmt, String> {
        let span = self.make_span();
        let has_parens = self.match_token(&lexer::Token::LeftParen);

        if has_parens && self.is_c_style_for() {
            return self.parse_c_for_stmt(span);
        }

        // for-in loop: for ([var] name in expr) { body }  or  for [var] name in expr { body }
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

    /// Lookahead to determine if a `for (` is a C-style for loop.
    /// C-style: `for (let j = ...; ...)` or `for (var j = ...; ...)` or `for (expr; ...)`
    /// For-in: `for (name in ...)` or `for (var name in ...)`
    pub(super) fn is_c_style_for(&self) -> bool {
        let mut pos = self.pos;

        // Check for `let` or `var` after `(`
        if let Some(st) = self.tokens.get(pos) {
            if matches!(st.token, lexer::Token::Let | lexer::Token::Var) {
                pos += 1;
                // After let/var, check if next is Identifier followed by `=` (C-style) or `in` (for-in)
                if let Some(st2) = self.tokens.get(pos) {
                    if let lexer::Token::Identifier(_) = &st2.token {
                        pos += 1;
                        if let Some(st3) = self.tokens.get(pos) {
                            // `=` means C-style, `in` means for-in
                            return matches!(st3.token, lexer::Token::Equals);
                        }
                    }
                }
                // If we can't determine, assume not C-style
                return false;
            }
        }

        // Otherwise, check if the tokens before the first `;` look like an expression
        // (not `identifier in`). We scan for a semicolon at the same nesting level.
        let mut depth = 0;
        let mut found_semi = false;
        let mut found_in = false;
        let mut scan_pos = pos;
        while let Some(st) = self.tokens.get(scan_pos) {
            match &st.token {
                lexer::Token::LeftParen | lexer::Token::LeftBracket | lexer::Token::LeftBrace => {
                    depth += 1;
                }
                lexer::Token::RightParen | lexer::Token::RightBracket | lexer::Token::RightBrace => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                }
                lexer::Token::Semicolon if depth == 0 => {
                    found_semi = true;
                    break;
                }
                lexer::Token::Identifier(ref s) if s == "in" && depth == 0 => {
                    // Check if this `in` is preceded by an identifier (for-in pattern)
                    // and followed by an expression (not a semicolon)
                    found_in = true;
                    break;
                }
                _ => {}
            }
            scan_pos += 1;
        }

        found_semi && !found_in
    }

    /// Parse a C-style for loop: `for (init; cond; incr) { body }`
    pub(super) fn parse_c_for_stmt(&mut self, span: ast::Span) -> Result<ast::Stmt, String> {
        // We've already consumed `for (`
        // Parse init
        let init = if self.is_at(&lexer::Token::Semicolon) {
            self.advance(); // consume `;`
            None
        } else if self.is_at(&lexer::Token::Let) {
            self.advance();
            let vd = self.parse_let_decl(false)?;
            Some(Box::new(ast::Stmt::VarDecl(vd)))
        } else if self.is_at(&lexer::Token::Var) {
            self.advance();
            let vd = self.parse_var_decl()?;
            Some(Box::new(ast::Stmt::VarDecl(vd)))
        } else {
            // Expression statement (without semicolon consumed by parse_stmt)
            let expr = self.parse_expression()?;
            self.expect(&lexer::Token::Semicolon)?;
            Some(Box::new(ast::Stmt::Expr(expr)))
        };

        // Parse condition
        let condition = if self.is_at(&lexer::Token::Semicolon) {
            None
        } else {
            Some(self.parse_expression()?)
        };
        self.expect(&lexer::Token::Semicolon)?;

        // Parse increment
        let increment = if self.is_at(&lexer::Token::RightParen) {
            None
        } else {
            Some(self.parse_expression()?)
        };
        self.expect(&lexer::Token::RightParen)?;

        let body = self.parse_block()?;

        Ok(ast::Stmt::CFor(ast::CForStmt {
            init,
            condition,
            increment,
            body,
            span,
        }))
    }

    pub(super) fn parse_return_stmt(&mut self) -> Result<ast::Stmt, String> {
        if self.match_token(&lexer::Token::Semicolon) {
            Ok(ast::Stmt::Return(None))
        } else {
            let expr = self.parse_expression()?;
            self.expect(&lexer::Token::Semicolon)?;
            Ok(ast::Stmt::Return(Some(expr)))
        }
    }

    pub(super) fn parse_switch_stmt(&mut self) -> Result<ast::Stmt, String> {
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
    pub(super) fn parse_case_body(&mut self) -> Result<ast::Block, String> {
        if self.is_at(&lexer::Token::LeftBrace) {
            self.parse_block()
        } else {
            let stmt = self.parse_stmt()?;
            Ok(vec![stmt])
        }
    }

    /// Parse a with-statement: `with (resource) { body }` or `with (let f: T = expr) { body }`
    pub(super) fn parse_with_stmt(&mut self) -> Result<ast::Stmt, String> {
        let span = self.make_span();
        self.expect(&lexer::Token::LeftParen)?;

        // Check for `let name [: Type] = expr` form
        let (resource_expr, var_name, var_type) = if self.match_token(&lexer::Token::Let) {
            let name_tok = self.expect(&lexer::Token::Identifier(String::new()))?;
            let name = match name_tok {
                lexer::Token::Identifier(s) => s,
                _ => return Err(format!("Expected identifier after 'with let', found {}", name_tok)),
            };
            let typ = if self.match_token(&lexer::Token::Colon) {
                Some(self.parse_type()?)
            } else {
                None
            };
            self.expect(&lexer::Token::Equals)?;
            let expr = self.parse_expression()?;
            (expr, Some(name), typ)
        } else {
            // Simple form: with (expr) { body }
            let expr = self.parse_expression()?;
            (expr, None, None)
        };

        self.expect(&lexer::Token::RightParen)?;
        let body = self.parse_block()?;

        Ok(ast::Stmt::With(ast::WithStmt {
            resource_expr,
            var_name,
            var_type,
            body,
            span,
        }))
    }

    /// Parse `try { ... } catch (var: Type) { ... }` or `try { ... } finally { ... }`
    pub(super) fn parse_try_catch_stmt(&mut self) -> Result<ast::Stmt, String> {
        let span = self.make_span();
        let try_block = self.parse_block()?;

        // Check for catch clause
        if self.match_token(&lexer::Token::Catch) {
            self.expect(&lexer::Token::LeftParen)?;
            let name_tok = self.expect(&lexer::Token::Identifier(String::new()))?;
            let catch_var = match name_tok {
                lexer::Token::Identifier(s) => s,
                _ => return Err(format!("Expected identifier in catch, found {}", name_tok)),
            };
            let catch_var_type = if self.match_token(&lexer::Token::Colon) {
                Some(self.parse_type()?)
            } else {
                None
            };
            self.expect(&lexer::Token::RightParen)?;
            let catch_block = self.parse_block()?;

            // Check for optional finally
            if self.match_token(&lexer::Token::Finally) {
                let _finally_block = self.parse_block()?;
            }

            Ok(ast::Stmt::TryCatch {
                try_block,
                catch_var,
                catch_var_type,
                catch_block,
                span,
            })
        } else if self.match_token(&lexer::Token::Finally) {
            // try { } finally { } — treat as try/catch with empty catch
            let finally_block = self.parse_block()?;
            Ok(ast::Stmt::TryCatch {
                try_block,
                catch_var: "_".to_string(),
                catch_var_type: None,
                catch_block: finally_block,
                span,
            })
        } else {
            Err("Expected 'catch' or 'finally' after try block".to_string())
        }
    }
}
