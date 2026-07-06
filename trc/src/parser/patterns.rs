use super::*;

// ---------------------------------------------------------------------------
// Pattern parsing
// ---------------------------------------------------------------------------

impl Parser {
    pub(super) fn parse_pattern(&mut self) -> Result<ast::Pattern, String> {
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
                                _ => return Err(self.err(format!("Expected binding name, found {}", b))),
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
                    _ => return Err(self.err("Expected identifier")),
                };
                if self.match_token(&lexer::Token::LeftParen) {
                    let mut bindings = Vec::new();
                    if !self.is_at(&lexer::Token::RightParen) {
                        loop {
                            let b = self.expect(&lexer::Token::Identifier(String::new()))?;
                            match b {
                                lexer::Token::Identifier(s) => bindings.push(s),
                                _ => return Err(self.err(format!("Expected binding name, found {}", b))),
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
                Err(self.err(format!("Expected pattern, found {}", self.peek())))
            }
        }
    }
}
