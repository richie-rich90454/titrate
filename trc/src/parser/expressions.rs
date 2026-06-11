use super::*;

// ---------------------------------------------------------------------------
// Expression parsing — precedence climbing
// ---------------------------------------------------------------------------

impl Parser {
    pub(super) fn parse_expression(&mut self) -> Result<ast::Expr, String> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<ast::Expr, String> {
        let expr = self.parse_ternary()?;

        if self.match_token(&lexer::Token::Equals) {
            let span = self.make_span();
            let value = self.parse_assignment()?; // right-associative
            // The left side must be an lvalue — we check at the AST level
            Ok(ast::Expr::Assign(Box::new(expr), Box::new(value), span))
        } else if let Some(op) = self.match_compound_assignment() {
            let span = self.make_span();
            let value = self.parse_assignment()?; // right-associative
            // Desugar: x += y  →  x = x + y
            let lhs_clone = expr.clone();
            let binary = ast::Expr::Binary(Box::new(lhs_clone), op, Box::new(value), span);
            Ok(ast::Expr::Assign(Box::new(expr), Box::new(binary), span))
        } else {
            Ok(expr)
        }
    }

    /// Parse ternary conditional expression: condition ? then_expr : else_expr
    /// Precedence: between assignment and range/logical OR.
    fn parse_ternary(&mut self) -> Result<ast::Expr, String> {
        let expr = self.parse_range()?;

        if self.match_token(&lexer::Token::Question) {
            let span = self.make_span();
            let then_expr = self.parse_ternary()?; // right-associative
            self.expect(&lexer::Token::Colon)?;
            let else_expr = self.parse_ternary()?; // right-associative
            Ok(ast::Expr::Ternary {
                condition: Box::new(expr),
                then_expr: Box::new(then_expr),
                else_expr: Box::new(else_expr),
                span,
            })
        } else {
            Ok(expr)
        }
    }

    /// Check if the current token is a compound assignment operator.
    /// If so, consume it and return the corresponding binary operator.
    fn match_compound_assignment(&mut self) -> Option<ast::Operator> {
        let op = if self.is_at(&lexer::Token::PlusEqual) {
            ast::Operator::Add
        } else if self.is_at(&lexer::Token::MinusEqual) {
            ast::Operator::Sub
        } else if self.is_at(&lexer::Token::StarEqual) {
            ast::Operator::Mul
        } else if self.is_at(&lexer::Token::SlashEqual) {
            ast::Operator::Div
        } else if self.is_at(&lexer::Token::PercentEqual) {
            ast::Operator::Mod
        } else if self.is_at(&lexer::Token::AmpersandEqual) {
            ast::Operator::BitAnd
        } else if self.is_at(&lexer::Token::PipeEqual) {
            ast::Operator::BitOr
        } else if self.is_at(&lexer::Token::CaretEqual) {
            ast::Operator::BitXor
        } else if self.is_at(&lexer::Token::LeftShiftEqual) {
            ast::Operator::BitShl
        } else if self.is_at(&lexer::Token::RightShiftEqual) {
            ast::Operator::BitShr
        } else {
            return None;
        };
        self.advance();
        Some(op)
    }

    fn parse_range(&mut self) -> Result<ast::Expr, String> {
        let expr = self.parse_or()?;

        if self.match_token(&lexer::Token::DotDot) {
            let span = self.make_span();
            let end = self.parse_or()?;
            return Ok(ast::Expr::Range(Box::new(expr), Box::new(end), span));
        }
        if self.match_token(&lexer::Token::DotDotEq) {
            let span = self.make_span();
            let end = self.parse_or()?;
            return Ok(ast::Expr::RangeInclusive(Box::new(expr), Box::new(end), span));
        }

        Ok(expr)
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
        // Prefix increment/decrement: ++x desugars to x = x + 1
        if self.match_token(&lexer::Token::PlusPlus) {
            let span = self.make_span();
            let expr = self.parse_unary()?;
            // Desugar ++x to x = x + 1
            let one = ast::Expr::Literal(ast::Literal::Int(1), span);
            let increment = ast::Expr::Binary(Box::new(expr.clone()), ast::Operator::Add, Box::new(one), span);
            return Ok(ast::Expr::Assign(Box::new(expr), Box::new(increment), span));
        }
        if self.match_token(&lexer::Token::MinusMinus) {
            let span = self.make_span();
            let expr = self.parse_unary()?;
            // Desugar --x to x = x - 1
            let one = ast::Expr::Literal(ast::Literal::Int(1), span);
            let decrement = ast::Expr::Binary(Box::new(expr.clone()), ast::Operator::Sub, Box::new(one), span);
            return Ok(ast::Expr::Assign(Box::new(expr), Box::new(decrement), span));
        }
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
            } else if self.is_at(&lexer::Token::Question) {
                // Disambiguate: if `?` is followed by something that could start
                // an expression, it's the beginning of a ternary — leave it for
                // parse_ternary(). Otherwise, consume as error propagation.
                if self.is_ternary_question() {
                    break;
                }
                self.advance(); // consume the `?`
                let span = self.make_span();
                expr = ast::Expr::ErrorPropagation(Box::new(expr), span);
            } else if self.match_token(&lexer::Token::As) {
                // Cast
                let span = self.make_span();
                let typ = self.parse_type()?;
                expr = ast::Expr::Cast(Box::new(expr), typ, span);
            } else if self.match_token(&lexer::Token::PlusPlus) {
                // Postfix increment: x++ desugars to x = x + 1
                let span = self.make_span();
                let one = ast::Expr::Literal(ast::Literal::Int(1), span);
                let increment = ast::Expr::Binary(Box::new(expr.clone()), ast::Operator::Add, Box::new(one), span);
                expr = ast::Expr::Assign(Box::new(expr), Box::new(increment), span);
            } else if self.match_token(&lexer::Token::MinusMinus) {
                // Postfix decrement: x-- desugars to x = x - 1
                let span = self.make_span();
                let one = ast::Expr::Literal(ast::Literal::Int(1), span);
                let decrement = ast::Expr::Binary(Box::new(expr.clone()), ast::Operator::Sub, Box::new(one), span);
                expr = ast::Expr::Assign(Box::new(expr), Box::new(decrement), span);
            } else {
                break;
            }
        }
        Ok(expr)
    }

    /// Parse closure parameters: `x`, `x: Type`, or `x, y: Type, z`
    /// Each param is (name, type). If no type annotation, defaults to "auto".
    fn parse_closure_params(&mut self) -> Result<Vec<(String, ast::Type)>, String> {
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
            let typ = if self.match_token(&lexer::Token::Colon) {
                self.parse_type()?
            } else {
                ast::Type::simple("auto")
            };
            params.push((name, typ));
            if !self.match_token(&lexer::Token::Comma) {
                break;
            }
        }
        Ok(params)
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
            lexer::Token::RawStringLiteral(_) => {
                let span = self.make_span();
                let t = self.advance();
                match t {
                    lexer::Token::RawStringLiteral(s) => {
                        Ok(ast::Expr::Literal(ast::Literal::String(s), span))
                    }
                    _ => Err("Expected raw string literal".to_string()),
                }
            }
            lexer::Token::ByteLiteral(_) => {
                let span = self.make_span();
                let t = self.advance();
                match t {
                    lexer::Token::ByteLiteral(v) => {
                        Ok(ast::Expr::Literal(ast::Literal::Int(v as i64), span))
                    }
                    _ => Err("Expected byte literal".to_string()),
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
                let span = self.make_span();
                self.advance();
                // Check for unit literal: ()
                if self.is_at(&lexer::Token::RightParen) {
                    self.advance();
                    return Ok(ast::Expr::Unit(span));
                }
                let first = self.parse_expression()?;
                // Check for tuple: (expr, expr, ...)
                if self.match_token(&lexer::Token::Comma) {
                    let mut elements = vec![first];
                    loop {
                        elements.push(self.parse_expression()?);
                        if !self.match_token(&lexer::Token::Comma) {
                            break;
                        }
                    }
                    self.expect(&lexer::Token::RightParen)?;
                    return Ok(ast::Expr::Tuple(elements, span));
                }
                self.expect(&lexer::Token::RightParen)?;
                Ok(first) // grouping: (expr)
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
            lexer::Token::Fn => {
                // Closure expression: fn(params) => expr  or  fn(params) { block }
                let span = self.make_span();
                self.advance(); // consume 'fn'
                self.expect(&lexer::Token::LeftParen)?;
                let params = self.parse_closure_params()?;
                self.expect(&lexer::Token::RightParen)?;

                let return_type = ast::Type::simple("void");

                if self.match_token(&lexer::Token::FatArrow) {
                    // Expression body: fn(x) => x * 2
                    let expr = self.parse_expression()?;
                    Ok(ast::Expr::Closure {
                        params,
                        return_type,
                        body: vec![],
                        expr: Some(Box::new(expr)),
                        captured_vars: vec![],
                        span,
                    })
                } else if self.is_at(&lexer::Token::LeftBrace) {
                    // Block body: fn(x) { return x + 1; }
                    let body = self.parse_block()?;
                    Ok(ast::Expr::Closure {
                        params,
                        return_type,
                        body,
                        expr: None,
                        captured_vars: vec![],
                        span,
                    })
                } else {
                    let (line, col) = self.span_here();
                    Err(format!(
                        "Expected '=>' or '{{' after closure parameters at {}:{}",
                        line, col
                    ))
                }
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
