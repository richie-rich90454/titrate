/// Scanner — the main tokenize() function body.
use super::token::{Token, try_consume_operator};
use super::SpannedToken;
use super::scan_literals::{scan_char, scan_number, scan_string};

/// Convert source code into a list of tokens.
/// Returns an error on the first unrecognised character that cannot be
/// represented as an Error token.
pub fn tokenize(src: &str) -> Result<Vec<SpannedToken>, String> {
    let mut tokens = Vec::new();
    let mut chars = src.chars().peekable();
    let mut line = 1;
    let mut column = 1;

    // Skip UTF-8 BOM if present at the start of the file (there may be multiple)
    while chars.peek() == Some(&'\u{FEFF}') {
        chars.next();
    }

    // Tokenizing one drop at a time...
    while let Some(&ch) = chars.peek() {
        let start_line = line;
        let start_col = column;

        match ch {
            // Whitespace
            ' ' | '\t' | '\r' => {
                chars.next();
                column += 1;
            }
            '\n' => {
                chars.next();
                line += 1;
                column = 1;
            }

            // Line comment
            '/' if chars.clone().nth(1) == Some('/') => {
                chars.next();
                chars.next();
                column += 2;
                while let Some(&c) = chars.peek() {
                    if c == '\n' {
                        break;
                    }
                    chars.next();
                    column += 1;
                }
            }

            // Block comment (non-nestable)
            '/' if chars.clone().nth(1) == Some('*') => {
                chars.next();
                chars.next();
                column += 2;
                while let Some(&c) = chars.peek() {
                    if c == '*' && chars.clone().nth(1) == Some('/') {
                        chars.next();
                        chars.next();
                        column += 2;
                        break;
                    }
                    if c == '\n' {
                        line += 1;
                        column = 1;
                    } else {
                        column += 1;
                    }
                    chars.next();
                }
            }

            // String literal
            '"' => {
                scan_string(&mut chars, &mut line, &mut column, start_line, start_col, &mut tokens)?;
            }

            // Char literal
            '\'' => {
                scan_char(&mut chars, &mut line, &mut column, start_line, start_col, &mut tokens)?;
            }

            // Number literal (integer or float)
            '0'..='9' => {
                scan_number(&mut chars, &mut line, &mut column, start_line, start_col, &mut tokens, ch)?;
            }

            // Identifiers and keywords
            'a'..='z' | 'A'..='Z' | '_' => {
                // Check for raw string literal: r"..." or r#"..."#
                if ch == 'r' {
                    let next = chars.clone().nth(1);
                    if next == Some('"') || next == Some('#') {
                        chars.next(); // consume 'r'
                        column += 1;

                        // Count the number of '#' delimiters
                        let mut hash_count = 0;
                        while chars.peek() == Some(&'#') {
                            chars.next();
                            column += 1;
                            hash_count += 1;
                        }

                        // Expect opening '"'
                        if chars.peek() != Some(&'"') {
                            return Err(format!(
                                "Expected opening '\"' in raw string at {}:{}",
                                start_line, start_col
                            ));
                        }
                        chars.next(); // consume '"'
                        column += 1;

                        // Build the closing pattern: '"' + hash_count '#'
                        let closing: String = format!("\"{}", "#".repeat(hash_count));
                        let mut s = String::new();
                        let mut closed = false;
                        while let Some(&c) = chars.peek() {
                            // Check if we've reached the closing delimiter
                            if c == '"' {
                                let ahead: String = chars.clone().take(closing.len()).collect();
                                if ahead == closing {
                                    // Consume the closing delimiter
                                    for _ in 0..closing.len() {
                                        chars.next();
                                        column += 1;
                                    }
                                    closed = true;
                                    break;
                                }
                            }
                            if c == '\n' {
                                line += 1;
                                column = 1;
                            } else {
                                column += 1;
                            }
                            s.push(c);
                            chars.next();
                        }
                        if !closed {
                            return Err(format!(
                                "Unterminated raw string at {}:{}",
                                start_line, start_col
                            ));
                        }
                        tokens.push(SpannedToken {
                            token: Token::RawStringLiteral(s),
                            line: start_line,
                            column: start_col,
                        });
                        continue;
                    }
                }

                // Check for byte literal: b'x'
                if ch == 'b' {
                    let next = chars.clone().nth(1);
                    if next == Some('\'') {
                        chars.next(); // consume 'b'
                        column += 1;
                        chars.next(); // consume opening '\''
                        column += 1;

                        let byte_val = match chars.peek() {
                            Some('\\') => {
                                chars.next(); // consume '\'
                                column += 1;
                                match chars.peek() {
                                    Some('n') => { chars.next(); column += 1; b'\n' }
                                    Some('t') => { chars.next(); column += 1; b'\t' }
                                    Some('r') => { chars.next(); column += 1; b'\r' }
                                    Some('\\') => { chars.next(); column += 1; b'\\' }
                                    Some('\'') => { chars.next(); column += 1; b'\'' }
                                    Some('"') => { chars.next(); column += 1; b'"' }
                                    Some('0') => { chars.next(); column += 1; b'\0' }
                                    Some('x') => {
                                        chars.next(); // consume 'x'
                                        column += 1;
                                        let hex1 = chars.peek().and_then(|c| c.to_digit(16));
                                        let hex2 = chars.clone().nth(1).and_then(|c| c.to_digit(16));
                                        match (hex1, hex2) {
                                            (Some(h1), Some(h2)) => {
                                                chars.next(); column += 1;
                                                chars.next(); column += 1;
                                                (h1 * 16 + h2) as u8
                                            }
                                            _ => {
                                                return Err(format!(
                                                    "Invalid hex escape in byte literal at {}:{}",
                                                    start_line, start_col
                                                ));
                                            }
                                        }
                                    }
                                    Some(&other) => {
                                        return Err(format!(
                                            "Unknown byte escape \\{} at {}:{}",
                                            other, start_line, start_col
                                        ));
                                    }
                                    None => {
                                        return Err(format!(
                                            "Unterminated byte literal at {}:{}",
                                            start_line, start_col
                                        ));
                                    }
                                }
                            }
                            Some(&c) if c.is_ascii() && c != '\'' => {
                                chars.next();
                                column += 1;
                                c as u8
                            }
                            Some(_) => {
                                return Err(format!(
                                    "Invalid byte literal character at {}:{}",
                                    start_line, start_col
                                ));
                            }
                            None => {
                                return Err(format!(
                                    "Unterminated byte literal at {}:{}",
                                    start_line, start_col
                                ));
                            }
                        };

                        match chars.peek() {
                            Some('\'') => {
                                chars.next();
                                column += 1;
                            }
                            _ => {
                                return Err(format!(
                                    "Expected closing quote for byte literal at {}:{}",
                                    start_line, start_col
                                ));
                            }
                        }
                        tokens.push(SpannedToken {
                            token: Token::ByteLiteral(byte_val),
                            line: start_line,
                            column: start_col,
                        });
                        continue;
                    }
                }

                let mut ident = String::new();
                while let Some(&c) = chars.peek() {
                    match c {
                        'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => {
                            ident.push(c);
                            chars.next();
                            column += 1;
                        }
                        _ => break,
                    }
                }

                // Operator overloading: if the identifier is "operator" and the
                // next character(s) form a valid operator, consume them as part
                // of the identifier (e.g. "operator+" → Identifier("operator+")).
                if ident == "operator" {
                    if let Some(op_str) = try_consume_operator(&mut chars, &mut column) {
                        ident.push_str(&op_str);
                    }
                }

                let tok = match ident.as_str() {
                    "public" => Token::Public,
                    "private" => Token::Private,
                    "fn" => Token::Fn,
                    "class" => Token::Class,
                    "interface" => Token::Interface,
                    "enum" => Token::Enum,
                    "extends" => Token::Extends,
                    "implements" => Token::Implements,
                    "let" => Token::Let,
                    "var" => Token::Var,
                    "const" => Token::Const,
                    "do" => Token::Do,
                    "if" => Token::If,
                    "else" => Token::Else,
                    "while" => Token::While,
                    "for" => Token::For,
                    "return" => Token::Return,
                    "break" => Token::Break,
                    "continue" => Token::Continue,
                    "switch" => Token::Switch,
                    "case" => Token::Case,
                    "default" => Token::Default,
                    "with" => Token::With,
                    "throw" => Token::Throw,
                    "try" => Token::Try,
                    "catch" => Token::Catch,
                    "finally" => Token::Finally,
                    "true" => Token::BoolLiteral(true),
                    "false" => Token::BoolLiteral(false),
                    "null" => Token::NullLiteral,
                    "new" => Token::New,
                    "this" => Token::This,
                    "super" => Token::Super,
                    "Result" => Token::Result,
                    "Ok" => Token::Ok,
                    "Err" => Token::Err,
                    "Owned" => Token::Owned,
                    "region" => Token::Region,
                    "unsafe" => Token::Unsafe,
                    "as" => Token::As,
                    "is" => Token::Is,
                    "type" => Token::Type,
                    "import" => Token::Import,
                    "module" => Token::Module,
                    "where" => Token::Where,
                    "void" => Token::Void,
                    "bool" => Token::Bool,
                    "byte" => Token::Byte,
                    "short" => Token::Short,
                    "int" => Token::Int,
                    "long" => Token::Long,
                    "vast" => Token::Vast,
                    "uvast" => Token::Uvast,
                    "float" => Token::Float,
                    "double" => Token::Double,
                    "half" => Token::Half,
                    "quad" => Token::Quad,
                    "char" => Token::Char,
                    "string" => Token::String,
                    "size" => Token::Size,
                    "u8" => Token::U8,
                    "u16" => Token::U16,
                    "u32" => Token::U32,
                    "u64" => Token::U64,
                    _ => Token::Identifier(ident),
                };
                tokens.push(SpannedToken {
                    token: tok,
                    line: start_line,
                    column: start_col,
                });
            }

            // Operators and punctuation
            '+' => {
                chars.next();
                column += 1;
                if chars.peek() == Some(&'+') {
                    chars.next();
                    column += 1;
                    tokens.push(SpannedToken {
                        token: Token::PlusPlus,
                        line: start_line,
                        column: start_col,
                    });
                } else if chars.peek() == Some(&'=') {
                    chars.next();
                    column += 1;
                    tokens.push(SpannedToken {
                        token: Token::PlusEqual,
                        line: start_line,
                        column: start_col,
                    });
                } else {
                    tokens.push(SpannedToken {
                        token: Token::Plus,
                        line: start_line,
                        column: start_col,
                    });
                }
            }
            '-' => {
                chars.next();
                column += 1;
                if chars.peek() == Some(&'>') {
                    chars.next();
                    column += 1;
                    tokens.push(SpannedToken {
                        token: Token::Arrow,
                        line: start_line,
                        column: start_col,
                    });
                } else if chars.peek() == Some(&'-') {
                    chars.next();
                    column += 1;
                    tokens.push(SpannedToken {
                        token: Token::MinusMinus,
                        line: start_line,
                        column: start_col,
                    });
                } else if chars.peek() == Some(&'=') {
                    chars.next();
                    column += 1;
                    tokens.push(SpannedToken {
                        token: Token::MinusEqual,
                        line: start_line,
                        column: start_col,
                    });
                } else {
                    tokens.push(SpannedToken {
                        token: Token::Minus,
                        line: start_line,
                        column: start_col,
                    });
                }
            }
            '*' => {
                chars.next();
                column += 1;
                if chars.peek() == Some(&'=') {
                    chars.next();
                    column += 1;
                    tokens.push(SpannedToken {
                        token: Token::StarEqual,
                        line: start_line,
                        column: start_col,
                    });
                } else {
                    tokens.push(SpannedToken {
                        token: Token::Star,
                        line: start_line,
                        column: start_col,
                    });
                }
            }
            '/' => {
                // Already handled comments above; this is the division operator
                chars.next();
                column += 1;
                if chars.peek() == Some(&'=') {
                    chars.next();
                    column += 1;
                    tokens.push(SpannedToken {
                        token: Token::SlashEqual,
                        line: start_line,
                        column: start_col,
                    });
                } else {
                    tokens.push(SpannedToken {
                        token: Token::Slash,
                        line: start_line,
                        column: start_col,
                    });
                }
            }
            '%' => {
                chars.next();
                column += 1;
                if chars.peek() == Some(&'=') {
                    chars.next();
                    column += 1;
                    tokens.push(SpannedToken {
                        token: Token::PercentEqual,
                        line: start_line,
                        column: start_col,
                    });
                } else {
                    tokens.push(SpannedToken {
                        token: Token::Percent,
                        line: start_line,
                        column: start_col,
                    });
                }
            }
            '=' => {
                chars.next();
                column += 1;
                if chars.peek() == Some(&'=') {
                    chars.next();
                    column += 1;
                    tokens.push(SpannedToken {
                        token: Token::EqualEqual,
                        line: start_line,
                        column: start_col,
                    });
                } else if chars.peek() == Some(&'>') {
                    chars.next();
                    column += 1;
                    tokens.push(SpannedToken {
                        token: Token::FatArrow,
                        line: start_line,
                        column: start_col,
                    });
                } else {
                    tokens.push(SpannedToken {
                        token: Token::Equals,
                        line: start_line,
                        column: start_col,
                    });
                }
            }
            '!' => {
                chars.next();
                column += 1;
                if chars.peek() == Some(&'=') {
                    chars.next();
                    column += 1;
                    tokens.push(SpannedToken {
                        token: Token::NotEqual,
                        line: start_line,
                        column: start_col,
                    });
                } else {
                    tokens.push(SpannedToken {
                        token: Token::Not,
                        line: start_line,
                        column: start_col,
                    });
                }
            }
            '<' => {
                chars.next();
                column += 1;
                if chars.peek() == Some(&'=') {
                    chars.next();
                    column += 1;
                    tokens.push(SpannedToken {
                        token: Token::LessEqual,
                        line: start_line,
                        column: start_col,
                    });
                } else if chars.peek() == Some(&'<') {
                    chars.next();
                    column += 1;
                    if chars.peek() == Some(&'=') {
                        chars.next();
                        column += 1;
                        tokens.push(SpannedToken {
                            token: Token::LeftShiftEqual,
                            line: start_line,
                            column: start_col,
                        });
                    } else {
                        tokens.push(SpannedToken {
                            token: Token::LeftShift,
                            line: start_line,
                            column: start_col,
                        });
                    }
                } else {
                    tokens.push(SpannedToken {
                        token: Token::Less,
                        line: start_line,
                        column: start_col,
                    });
                }
            }
            '>' => {
                chars.next();
                column += 1;
                if chars.peek() == Some(&'=') {
                    chars.next();
                    column += 1;
                    tokens.push(SpannedToken {
                        token: Token::GreaterEqual,
                        line: start_line,
                        column: start_col,
                    });
                } else if chars.peek() == Some(&'>') {
                    chars.next();
                    column += 1;
                    if chars.peek() == Some(&'>') {
                        // >>> or >>>=
                        chars.next();
                        column += 1;
                        if chars.peek() == Some(&'=') {
                            chars.next();
                            column += 1;
                            tokens.push(SpannedToken {
                                token: Token::TripleGreaterEqual,
                                line: start_line,
                                column: start_col,
                            });
                        } else {
                            tokens.push(SpannedToken {
                                token: Token::TripleGreater,
                                line: start_line,
                                column: start_col,
                            });
                        }
                    } else if chars.peek() == Some(&'=') {
                        chars.next();
                        column += 1;
                        tokens.push(SpannedToken {
                            token: Token::RightShiftEqual,
                            line: start_line,
                            column: start_col,
                        });
                    } else {
                        tokens.push(SpannedToken {
                            token: Token::RightShift,
                            line: start_line,
                            column: start_col,
                        });
                    }
                } else {
                    tokens.push(SpannedToken {
                        token: Token::Greater,
                        line: start_line,
                        column: start_col,
                    });
                }
            }
            '&' => {
                chars.next();
                column += 1;
                if chars.peek() == Some(&'&') {
                    chars.next();
                    column += 1;
                    tokens.push(SpannedToken {
                        token: Token::AndAnd,
                        line: start_line,
                        column: start_col,
                    });
                } else if chars.peek() == Some(&'=') {
                    chars.next();
                    column += 1;
                    tokens.push(SpannedToken {
                        token: Token::AmpersandEqual,
                        line: start_line,
                        column: start_col,
                    });
                } else {
                    // Check for &mut
                    let rest: String = chars.clone().take(3).collect();
                    if rest.starts_with("mut") {
                        // Make sure "mut" is not a prefix of a longer identifier
                        let fourth = chars.clone().nth(3);
                        if fourth.is_none_or(|c| !c.is_alphanumeric() && c != '_') {
                            chars.next(); chars.next(); chars.next(); // "mut"
                            column += 3;
                            tokens.push(SpannedToken {
                                token: Token::RefMut,
                                line: start_line,
                                column: start_col,
                            });
                        } else {
                            tokens.push(SpannedToken {
                                token: Token::Ampersand,
                                line: start_line,
                                column: start_col,
                            });
                        }
                    } else {
                        tokens.push(SpannedToken {
                            token: Token::Ampersand,
                            line: start_line,
                            column: start_col,
                        });
                    }
                }
            }
            '|' => {
                chars.next();
                column += 1;
                if chars.peek() == Some(&'|') {
                    chars.next();
                    column += 1;
                    tokens.push(SpannedToken {
                        token: Token::OrOr,
                        line: start_line,
                        column: start_col,
                    });
                } else if chars.peek() == Some(&'=') {
                    chars.next();
                    column += 1;
                    tokens.push(SpannedToken {
                        token: Token::PipeEqual,
                        line: start_line,
                        column: start_col,
                    });
                } else {
                    tokens.push(SpannedToken {
                        token: Token::Pipe,
                        line: start_line,
                        column: start_col,
                    });
                }
            }
            '^' => {
                chars.next();
                column += 1;
                if chars.peek() == Some(&'=') {
                    chars.next();
                    column += 1;
                    tokens.push(SpannedToken {
                        token: Token::CaretEqual,
                        line: start_line,
                        column: start_col,
                    });
                } else {
                    tokens.push(SpannedToken {
                        token: Token::Caret,
                        line: start_line,
                        column: start_col,
                    });
                }
            }
            '~' => {
                chars.next();
                column += 1;
                tokens.push(SpannedToken {
                    token: Token::Tilde,
                    line: start_line,
                    column: start_col,
                });
            }
            ':' => {
                chars.next();
                column += 1;
                if chars.peek() == Some(&':') {
                    chars.next();
                    column += 1;
                    tokens.push(SpannedToken {
                        token: Token::ColonColon,
                        line: start_line,
                        column: start_col,
                    });
                } else {
                    tokens.push(SpannedToken {
                        token: Token::Colon,
                        line: start_line,
                        column: start_col,
                    });
                }
            }
            '?' => {
                chars.next();
                column += 1;
                tokens.push(SpannedToken {
                    token: Token::Question,
                    line: start_line,
                    column: start_col,
                });
            }
            '.' => {
                chars.next();
                column += 1;
                if chars.peek() == Some(&'.') {
                    chars.next();
                    column += 1;
                    if chars.peek() == Some(&'=') {
                        chars.next();
                        column += 1;
                        tokens.push(SpannedToken {
                            token: Token::DotDotEq,
                            line: start_line,
                            column: start_col,
                        });
                    } else {
                        tokens.push(SpannedToken {
                            token: Token::DotDot,
                            line: start_line,
                            column: start_col,
                        });
                    }
                } else {
                    tokens.push(SpannedToken {
                        token: Token::Dot,
                        line: start_line,
                        column: start_col,
                    });
                }
            }
            ',' => {
                chars.next();
                column += 1;
                tokens.push(SpannedToken {
                    token: Token::Comma,
                    line: start_line,
                    column: start_col,
                });
            }
            ';' => {
                chars.next();
                column += 1;
                tokens.push(SpannedToken {
                    token: Token::Semicolon,
                    line: start_line,
                    column: start_col,
                });
            }
            '(' => {
                chars.next();
                column += 1;
                tokens.push(SpannedToken {
                    token: Token::LeftParen,
                    line: start_line,
                    column: start_col,
                });
            }
            ')' => {
                chars.next();
                column += 1;
                tokens.push(SpannedToken {
                    token: Token::RightParen,
                    line: start_line,
                    column: start_col,
                });
            }
            '{' => {
                chars.next();
                column += 1;
                tokens.push(SpannedToken {
                    token: Token::LeftBrace,
                    line: start_line,
                    column: start_col,
                });
            }
            '}' => {
                chars.next();
                column += 1;
                tokens.push(SpannedToken {
                    token: Token::RightBrace,
                    line: start_line,
                    column: start_col,
                });
            }
            '[' => {
                chars.next();
                column += 1;
                tokens.push(SpannedToken {
                    token: Token::LeftBracket,
                    line: start_line,
                    column: start_col,
                });
            }
            ']' => {
                chars.next();
                column += 1;
                tokens.push(SpannedToken {
                    token: Token::RightBracket,
                    line: start_line,
                    column: start_col,
                });
            }
            _ => {
                let c = ch;
                chars.next();
                column += 1;
                tokens.push(SpannedToken {
                    token: Token::Error(format!("{}", c)),
                    line: start_line,
                    column: start_col,
                });
            }
        }
    }

    tokens.push(SpannedToken {
        token: Token::Eof,
        line,
        column,
    });

    Ok(tokens)
}

