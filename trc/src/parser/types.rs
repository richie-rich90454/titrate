use super::*;

// ---------------------------------------------------------------------------
// Type parsing
// ---------------------------------------------------------------------------

impl Parser {
    pub(super) fn parse_type(&mut self) -> Result<ast::Type, String> {
        self.enter_recursion()?;
        let result = self.parse_type_inner();
        self.leave_recursion();
        result
    }

    fn parse_type_inner(&mut self) -> Result<ast::Type, String> {
        // Check for &mut T or &T reference types
        if self.match_token(&lexer::Token::RefMut) {
            let inner = self.parse_type()?;
            return Ok(ast::Type::MutRef(Box::new(inner)));
        }
        if self.match_token(&lexer::Token::Ampersand) {
            // Check if next token is 'mut' (could be &mut without the combined RefMut token)
            if self.match_token(&lexer::Token::RefMut) {
                let inner = self.parse_type()?;
                return Ok(ast::Type::MutRef(Box::new(inner)));
            }
            let inner = self.parse_type()?;
            return Ok(ast::Type::Ref(Box::new(inner)));
        }

        // Check for function type: fn(params): return_type
        // The type information is not deeply used by the compiler (function
        // values are dynamically dispatched), so we parse the syntax and
        // return a simple "fn" named type.
        // Supports both unnamed params: fn(K, V): void
        // and named params: fn(a: K, b: K): int
        if self.match_token(&lexer::Token::Fn) {
            self.expect(&lexer::Token::LeftParen)?;
            if !self.is_at(&lexer::Token::RightParen) {
                loop {
                    let _ = self.parse_type()?;
                    // If next is ':', the previous type was actually a param name;
                    // parse the actual type after the colon.
                    if self.match_token(&lexer::Token::Colon) {
                        let _ = self.parse_type()?;
                    }
                    if !self.match_token(&lexer::Token::Comma) {
                        break;
                    }
                }
            }
            self.expect(&lexer::Token::RightParen)?;
            if self.match_token(&lexer::Token::Colon) {
                let _ = self.parse_type()?;
            }
            return Ok(ast::Type::simple("fn"));
        }

        // Check for tuple type: (T1, T2, ...)
        if self.match_token(&lexer::Token::LeftParen) {
            // Empty parens = void (unit type)
            if self.is_at(&lexer::Token::RightParen) {
                self.advance();
                return Ok(ast::Type::simple("void"));
            }
            let first = self.parse_type()?;
            if self.match_token(&lexer::Token::Comma) {
                let mut types = vec![first];
                loop {
                    types.push(self.parse_type()?);
                    if !self.match_token(&lexer::Token::Comma) {
                        break;
                    }
                }
                self.expect(&lexer::Token::RightParen)?;
                return Ok(ast::Type::Tuple(types));
            }
            self.expect(&lexer::Token::RightParen)?;
            // Single type in parens is just grouping
            return Ok(first);
        }

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
            let mut name = match id_tok {
                lexer::Token::Identifier(s) => s,
                _ => return Err(self.err(format!("Expected type name, found {}", id_tok))),
            };
            // Handle qualified names: Module.ClassName or Module::ClassName
            loop {
                if self.match_token(&lexer::Token::Dot) {
                    let next = self.expect(&lexer::Token::Identifier(String::new()))?;
                    if let lexer::Token::Identifier(s) = next {
                        name.push('.');
                        name.push_str(&s);
                    } else {
                        return Err(self.err(format!("Expected identifier after '.', found {}", next)));
                    }
                } else if self.match_token(&lexer::Token::ColonColon) {
                    let next = self.expect(&lexer::Token::Identifier(String::new()))?;
                    if let lexer::Token::Identifier(s) = next {
                        name.push_str("::");
                        name.push_str(&s);
                    } else {
                        return Err(self.err(format!("Expected identifier after '::', found {}", next)));
                    }
                } else {
                    break;
                }
            }
            name
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
            self.expect_close_angle()?;
            ps
        } else {
            Vec::new()
        };

        Ok(ast::Type::Named { name, params })
    }
}
