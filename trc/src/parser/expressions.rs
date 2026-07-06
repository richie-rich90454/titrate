use super::*;

// ---------------------------------------------------------------------------
// Expression parsing — precedence climbing
// ---------------------------------------------------------------------------

impl Parser {
    pub(super) fn parse_expression(&mut self) -> Result<ast::Expr, String> {
        self.enter_recursion()?;
        let result = self.parse_assignment();
        self.leave_recursion();
        result
    }

    fn parse_assignment(&mut self) -> Result<ast::Expr, String> {
        let expr = self.parse_ternary()?;

        if self.match_token(&lexer::Token::Equals) {
            let span = self.make_span();
            self.enter_recursion()?;
            let value = self.parse_assignment()?;
            self.leave_recursion();
            // The left side must be an lvalue — we check at the AST level
            Ok(ast::Expr::Assign(Box::new(expr), Box::new(value), span))
        } else if let Some(op) = self.match_compound_assignment() {
            let span = self.make_span();
            self.enter_recursion()?;
            let value = self.parse_assignment()?;
            self.leave_recursion();
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
            self.enter_recursion()?;
            let then_expr = self.parse_ternary()?;
            self.leave_recursion();
            self.expect(&lexer::Token::Colon)?;
            self.enter_recursion()?;
            let else_expr = self.parse_ternary()?;
            self.leave_recursion();
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
            } else if self.match_token(&lexer::Token::TripleGreater) {
                ast::Operator::BitUshr
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
        // Collect prefix operators iteratively to avoid deep recursion on
        // inputs like `!!!!...!!!!x`. Each operator would otherwise add a
        // stack frame, causing STATUS_STACK_OVERFLOW on pathologically long
        // prefix chains.
        enum PrefixOp {
            Increment,
            Decrement,
            Not,
            Neg,
            BitNot,
            Deref,
            RefImmutable,
            RefMutable,
        }
        let mut ops: Vec<(PrefixOp, ast::Span)> = Vec::new();
        loop {
            if self.match_token(&lexer::Token::PlusPlus) {
                ops.push((PrefixOp::Increment, self.make_span()));
            } else if self.match_token(&lexer::Token::MinusMinus) {
                ops.push((PrefixOp::Decrement, self.make_span()));
            } else if self.match_token(&lexer::Token::Not) {
                ops.push((PrefixOp::Not, self.make_span()));
            } else if self.match_token(&lexer::Token::Minus) {
                ops.push((PrefixOp::Neg, self.make_span()));
            } else if self.match_token(&lexer::Token::Tilde) {
                ops.push((PrefixOp::BitNot, self.make_span()));
            } else if self.match_token(&lexer::Token::Star) {
                ops.push((PrefixOp::Deref, self.make_span()));
            } else if self.match_token(&lexer::Token::Ampersand) {
                if self.match_token(&lexer::Token::RefMut) {
                    ops.push((PrefixOp::RefMutable, self.make_span()));
                } else {
                    ops.push((PrefixOp::RefImmutable, self.make_span()));
                }
            } else if self.match_token(&lexer::Token::RefMut) {
                ops.push((PrefixOp::RefMutable, self.make_span()));
            } else {
                break;
            }
        }
        // Parse the operand (postfix/primary expression).
        let mut expr = self.parse_postfix()?;
        // Apply prefix operators in reverse order (rightmost first).
        for (op, span) in ops.into_iter().rev() {
            expr = match op {
                PrefixOp::Increment => {
                    let one = ast::Expr::Literal(ast::Literal::Int(1), span);
                    let increment = ast::Expr::Binary(
                        Box::new(expr.clone()),
                        ast::Operator::Add,
                        Box::new(one),
                        span,
                    );
                    ast::Expr::Assign(Box::new(expr), Box::new(increment), span)
                }
                PrefixOp::Decrement => {
                    let one = ast::Expr::Literal(ast::Literal::Int(1), span);
                    let decrement = ast::Expr::Binary(
                        Box::new(expr.clone()),
                        ast::Operator::Sub,
                        Box::new(one),
                        span,
                    );
                    ast::Expr::Assign(Box::new(expr), Box::new(decrement), span)
                }
                PrefixOp::Not => ast::Expr::Unary(ast::UnOp::Not, Box::new(expr), span),
                PrefixOp::Neg => ast::Expr::Unary(ast::UnOp::Neg, Box::new(expr), span),
                PrefixOp::BitNot => ast::Expr::Unary(ast::UnOp::BitNot, Box::new(expr), span),
                PrefixOp::Deref => ast::Expr::OwnedDeref(Box::new(expr), span),
                PrefixOp::RefImmutable => {
                    ast::Expr::RefExpr(Box::new(expr), ast::RefKind::Immutable, span)
                }
                PrefixOp::RefMutable => {
                    ast::Expr::RefExpr(Box::new(expr), ast::RefKind::Mutable, span)
                }
            };
        }
        Ok(expr)
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
                // Member access — field name can be an identifier, a keyword used
                // as a name, or a numeric literal (tuple field access: t.0, t.1).
                let span = self.make_span();
                let name_tok = self.advance();
                let name = match &name_tok {
                    lexer::Token::IntLiteral(v) => v.to_string(),
                    _ => token_as_name(&name_tok)
                        .ok_or_else(|| format!("Expected member name, found {}", name_tok))?,
                };
                expr = ast::Expr::MemberAccess(Box::new(expr), name, span);
            } else if self.match_token(&lexer::Token::ColonColon) {
                // :: namespace access → treat as member access
                let span = self.make_span();
                let name_tok = self.advance();
                let name = token_as_name(&name_tok)
                    .ok_or_else(|| format!("Expected namespace member, found {}", name_tok))?;
                expr = ast::Expr::MemberAccess(Box::new(expr), name, span);
            } else if self.is_at(&lexer::Token::Less) {
                // Could be generic type arguments: Identifier<Type, ...>::method(...)
                // or a less-than comparison: a < b
                // Use backtracking: try to parse <Type, ...> and check if followed by :: or (
                let saved = self.pos;
                self.advance(); // consume '<'
                let mut success = true;
                loop {
                    if self.parse_type().is_err() {
                        success = false;
                        break;
                    }
                    if self.match_token(&lexer::Token::Comma) {
                        continue;
                    }
                    if self.expect_close_angle().is_ok() {
                        break;
                    } else {
                        success = false;
                        break;
                    }
                }
                if success && (self.is_at(&lexer::Token::ColonColon) || self.is_at(&lexer::Token::LeftParen) || self.is_at(&lexer::Token::Dot)) {
                    // Generic type arguments parsed successfully — type args are ignored
                    // at runtime (dynamic dispatch), so just continue the loop.
                    continue;
                } else {
                    // Not generic type arguments — backtrack and treat as less-than
                    self.pos = saved;
                    break;
                }
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
            } else if self.match_token(&lexer::Token::Is) {
                // Type check: expr is Type
                let span = self.make_span();
                let typ = self.parse_type()?;
                expr = ast::Expr::Is(Box::new(expr), typ, span);
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
                _ => return Err(self.err(format!("Expected parameter name, found {}", name_tok))),
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
                        // Check for string interpolation: ${expr}
                        if s.contains("${") {
                            Self::parse_string_interpolation(&s, span)
                        } else {
                            Ok(ast::Expr::Literal(ast::Literal::String(s), span))
                        }
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
            // `var` can be used as a variable name (e.g. `var + this.epsilon`)
            lexer::Token::Var => {
                let span = self.make_span();
                self.advance();
                Ok(ast::Expr::Identifier("var".to_string(), span))
            }
            // `where` can be used as a function name (e.g. `where(...)`)
            lexer::Token::Where => {
                let span = self.make_span();
                self.advance();
                Ok(ast::Expr::Identifier("where".to_string(), span))
            }
            // `Result` can be used as a type/identifier in expressions
            // (e.g. `Result<T, E>::ok(...)`)
            lexer::Token::Result => {
                let span = self.make_span();
                self.advance();
                Ok(ast::Expr::Identifier("Result".to_string(), span))
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
                // Could be a closure expression: fn(params) => expr  or  fn(params) { block }
                // Or a call to a variable named `fn`: fn(elem)
                // Use backtracking to disambiguate.
                let span = self.make_span();
                let saved = self.pos;
                self.advance(); // consume 'fn'

                if self.match_token(&lexer::Token::LeftParen) {
                    // Try to parse as closure
                    let closure_result = self.parse_closure_params();
                    if let Ok(params) = closure_result {
                        if self.match_token(&lexer::Token::RightParen) {
                            // Check for optional return type: fn(params): ReturnType
                            let return_type = if self.match_token(&lexer::Token::Colon) {
                                self.parse_type()?
                            } else {
                                ast::Type::simple("void")
                            };
                            if self.match_token(&lexer::Token::FatArrow) {
                                // Expression body: fn(x) => x * 2
                                let expr = self.parse_expression()?;
                                return Ok(ast::Expr::Closure {
                                    params,
                                    return_type,
                                    body: vec![],
                                    expr: Some(Box::new(expr)),
                                    captured_vars: vec![],
                                    span,
                                });
                            } else if self.is_at(&lexer::Token::LeftBrace) {
                                // Block body: fn(x) { return x + 1; }
                                let body = self.parse_block()?;
                                return Ok(ast::Expr::Closure {
                                    params,
                                    return_type,
                                    body,
                                    expr: None,
                                    captured_vars: vec![],
                                    span,
                                });
                            }
                        }
                    }
                    // Not a closure — backtrack to just after `fn` was consumed
                    // so parse_postfix can handle `(args)` as a function call.
                    self.pos = saved + 1;
                } else {
                    // No `(` after `fn` — `fn` is just an identifier
                }

                // Treat `fn` as an identifier (e.g., a parameter named `fn`)
                Ok(ast::Expr::Identifier("fn".to_string(), span))
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
            // Type keywords can be used as variable/parameter names
            // (e.g. `size` in `this.size = size;`)
            t if is_type_keyword(&t) => {
                let span = self.make_span();
                self.advance();
                let name = type_keyword_name(&t)
                    .ok_or_else(|| format!("Expected expression, found {}", t))?;
                Ok(ast::Expr::Identifier(name, span))
            }
            _ => {
                Err(self.err(format!("Expected expression, found {}", self.peek())))
            }
        }
    }

    /// Parse a string literal containing `${expr}` interpolation markers.
    ///
    /// Desugars `"Hello, ${name}! You are ${age} years old."` into:
    /// `"Hello, " + name + "! You are " + age + " years old."`
    ///
    /// This reuses the existing `Binary(Add)` / `STR_CONCAT*` infrastructure
    /// so that any type with a `display_string()` representation works.
    fn parse_string_interpolation(s: &str, span: ast::Span) -> Result<ast::Expr, String> {
        enum Part {
            Literal(String),
            Expression(String),
        }

        let mut parts: Vec<Part> = Vec::new();
        let mut chars = s.chars().peekable();
        let mut current_literal = String::new();

        while let Some(c) = chars.next() {
            if c == '$' && chars.peek() == Some(&'{') {
                chars.next(); // consume '{'
                if !current_literal.is_empty() {
                    parts.push(Part::Literal(std::mem::take(&mut current_literal)));
                }
                let mut expr_str = String::new();
                let mut depth: i32 = 1;
                while let Some(ec) = chars.next() {
                    if ec == '{' {
                        depth += 1;
                        expr_str.push(ec);
                    } else if ec == '}' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                        expr_str.push(ec);
                    } else {
                        expr_str.push(ec);
                    }
                }
                if depth != 0 {
                    return Err(format!("String interpolation: unterminated ${{ in \"{}\" at {}:{}", s, span.line, span.column));
                }
                if expr_str.trim().is_empty() {
                    return Err(format!("String interpolation: empty expression in \"{}\" at {}:{}", s, span.line, span.column));
                }
                parts.push(Part::Expression(expr_str));
            } else {
                current_literal.push(c);
            }
        }
        if !current_literal.is_empty() {
            parts.push(Part::Literal(current_literal));
        }

        // Build a left-associative Binary(Add) chain.
        let mut result: Option<ast::Expr> = None;
        for part in parts {
            let expr = match part {
                Part::Literal(text) => {
                    ast::Expr::Literal(ast::Literal::String(text), span)
                }
                Part::Expression(expr_str) => {
                    let tokens = lexer::tokenize(&expr_str)
                        .map_err(|e| format!("String interpolation: {}", e))?;
                    let mut sub_parser = Parser::new(tokens);
                    sub_parser
                        .parse_expression()
                        .map_err(|e| format!("String interpolation: {}", e))?
                }
            };
            result = Some(match result {
                None => expr,
                Some(left) => ast::Expr::Binary(
                    Box::new(left),
                    ast::Operator::Add,
                    Box::new(expr),
                    span,
                ),
            });
        }

        result.ok_or_else(|| "String interpolation: empty string".to_string())
    }
}
