/// Scanner — the main tokenize() function body.

use super::token::{FloatSuffix, Token, try_consume_operator};
use super::SpannedToken;

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
                chars.next();
                column += 1;
                let mut s = String::new();
                let mut closed = false;
                while let Some(&c) = chars.peek() {
                    match c {
                        '"' => {
                            chars.next();
                            column += 1;
                            closed = true;
                            break;
                        }
                        '\\' => {
                            chars.next(); // consume backslash
                            column += 1;
                            let escaped = match chars.peek() {
                                Some('n') => { chars.next(); column += 1; '\n' }
                                Some('t') => { chars.next(); column += 1; '\t' }
                                Some('r') => { chars.next(); column += 1; '\r' }
                                Some('\\') => { chars.next(); column += 1; '\\' }
                                Some('"') => { chars.next(); column += 1; '"' }
                                Some('\'') => { chars.next(); column += 1; '\'' }
                                Some('0') => { chars.next(); column += 1; '\0' }
                                Some('b') => { chars.next(); column += 1; '\x08' }
                                Some('f') => { chars.next(); column += 1; '\x0c' }
                                Some('v') => { chars.next(); column += 1; '\x0b' }
                                Some('a') => { chars.next(); column += 1; '\x07' }
                                Some('x') => {
                                    // \xHH — hex escape (1-2 hex digits)
                                    chars.next();
                                    column += 1;
                                    let mut hex = String::new();
                                    for _ in 0..2 {
                                        if let Some(&h) = chars.peek() {
                                            if h.is_ascii_hexdigit() {
                                                hex.push(h);
                                                chars.next();
                                                column += 1;
                                            } else {
                                                break;
                                            }
                                        }
                                    }
                                    if hex.is_empty() {
                                        return Err(format!(
                                            "Invalid hex escape \\x at {}:{}",
                                            start_line, start_col
                                        ));
                                    }
                                    let val = u32::from_str_radix(&hex, 16)
                                        .map_err(|e| format!("Invalid hex escape \\x{} at {}:{}: {}", hex, start_line, start_col, e))?;
                                    char::from_u32(val).ok_or_else(|| format!(
                                        "Invalid hex escape \\x{} at {}:{}: not a valid Unicode scalar", hex, start_line, start_col
                                    ))?
                                }
                                Some('u') => {
                                    // \u{HHHHHH} — unicode escape
                                    chars.next();
                                    column += 1;
                                    if chars.peek() != Some(&'{') {
                                        return Err(format!(
                                            "Expected '{{' after \\u at {}:{}",
                                            start_line, start_col
                                        ));
                                    }
                                    chars.next();
                                    column += 1;
                                    let mut hex = String::new();
                                    while let Some(&h) = chars.peek() {
                                        if h == '}' {
                                            chars.next();
                                            column += 1;
                                            break;
                                        }
                                        if h.is_ascii_hexdigit() {
                                            hex.push(h);
                                            chars.next();
                                            column += 1;
                                        } else {
                                            return Err(format!(
                                                "Invalid unicode escape \\u at {}:{}",
                                                start_line, start_col
                                            ));
                                        }
                                    }
                                    if hex.is_empty() {
                                        return Err(format!(
                                            "Empty unicode escape \\u{{}} at {}:{}",
                                            start_line, start_col
                                        ));
                                    }
                                    let val = u32::from_str_radix(&hex, 16)
                                        .map_err(|e| format!("Invalid unicode escape \\u{{{}}} at {}:{}: {}", hex, start_line, start_col, e))?;
                                    char::from_u32(val).ok_or_else(|| format!(
                                        "Invalid unicode escape \\u{{{}}} at {}:{}: not a valid Unicode scalar", hex, start_line, start_col
                                    ))?
                                }
                                Some(&other) => {
                                    return Err(format!(
                                        "Unknown escape \\{} at {}:{}",
                                        other, start_line, start_col
                                    ));
                                }
                                None => {
                                    return Err(format!(
                                        "Unterminated string escape at {}:{}",
                                        start_line, start_col
                                    ));
                                }
                            };
                            s.push(escaped);
                        }
                        '\n' => {
                            return Err(format!(
                                "Unterminated string at {}:{}",
                                start_line, start_col
                            ));
                        }
                        _ => {
                            s.push(c);
                            chars.next();
                            column += 1;
                        }
                    }
                }
                if !closed {
                    return Err(format!(
                        "Unterminated string at {}:{}",
                        start_line, start_col
                    ));
                }
                tokens.push(SpannedToken {
                    token: Token::StringLiteral(s),
                    line: start_line,
                    column: start_col,
                });
            }

            // Char literal
            '\'' => {
                chars.next();
                column += 1;
                let ch_val = match chars.peek() {
                    Some('\\') => {
                        chars.next();
                        column += 1;
                        match chars.peek() {
                            Some('n') => '\n',
                            Some('t') => '\t',
                            Some('r') => '\r',
                            Some('\\') => '\\',
                            Some('\'') => '\'',
                            Some('"') => '"',
                            Some('0') => '\0',
                            Some('b') => '\x08',
                            Some('f') => '\x0c',
                            Some('v') => '\x0b',
                            Some('a') => '\x07',
                            Some(&other) => {
                                return Err(format!(
                                    "Unknown char escape \\{} at {}:{}",
                                    other, start_line, start_col
                                ));
                            }
                            None => {
                                return Err(format!(
                                    "Unterminated char literal at {}:{}",
                                    start_line, start_col
                                ));
                            }
                        }
                    }
                    Some(&c) => c,
                    None => {
                        return Err(format!(
                            "Unterminated char literal at {}:{}",
                            start_line, start_col
                        ));
                    }
                };
                chars.next();
                column += 1;
                match chars.peek() {
                    Some('\'') => {
                        chars.next();
                        column += 1;
                    }
                    _ => {
                        return Err(format!(
                            "Expected closing quote for char literal at {}:{}",
                            start_line, start_col
                        ));
                    }
                }
                tokens.push(SpannedToken {
                    token: Token::CharLiteral(ch_val),
                    line: start_line,
                    column: start_col,
                });
            }

            // Number literal (integer or float)
            '0'..='9' => {
                let mut num_str = String::new();
                let mut is_float = false;

                // Check for hex/oct/bin prefix
                if ch == '0' {
                    let next = chars.clone().nth(1);
                    if next == Some('x') || next == Some('X') {
                        chars.next();
                        chars.next();
                        column += 2;
                        let mut hex = String::new();
                        while let Some(&c) = chars.peek() {
                            match c {
                                '0'..='9' | 'a'..='f' | 'A'..='F' | '_' => {
                                    if c != '_' {
                                        hex.push(c);
                                    }
                                    chars.next();
                                    column += 1;
                                }
                                _ => break,
                            }
                        }
                        let val = i64::from_str_radix(&hex, 16)
                            .or_else(|_| u64::from_str_radix(&hex, 16).map(|v| v as i64))
                            .map_err(|e| format!("Invalid hex literal at {}:{}: {}", start_line, start_col, e))?;
                        // Consume optional 'L' or 'l' suffix (long literal)
                        if let Some(&suffix) = chars.peek() {
                            if suffix == 'L' || suffix == 'l' {
                                chars.next();
                                column += 1;
                            }
                        }
                        tokens.push(SpannedToken {
                            token: Token::IntLiteral(val),
                            line: start_line,
                            column: start_col,
                        });
                        continue;
                    }
                    if next == Some('o') || next == Some('O') {
                        chars.next();
                        chars.next();
                        column += 2;
                        let mut oct = String::new();
                        while let Some(&c) = chars.peek() {
                            match c {
                                '0'..='7' | '_' => {
                                    if c != '_' {
                                        oct.push(c);
                                    }
                                    chars.next();
                                    column += 1;
                                }
                                _ => break,
                            }
                        }
                        let val = i64::from_str_radix(&oct, 8)
                            .or_else(|_| u64::from_str_radix(&oct, 8).map(|v| v as i64))
                            .map_err(|e| format!("Invalid octal literal at {}:{}: {}", start_line, start_col, e))?;
                        tokens.push(SpannedToken {
                            token: Token::IntLiteral(val),
                            line: start_line,
                            column: start_col,
                        });
                        continue;
                    }
                    if next == Some('b') || next == Some('B') {
                        chars.next();
                        chars.next();
                        column += 2;
                        let mut bin = String::new();
                        while let Some(&c) = chars.peek() {
                            match c {
                                '0' | '1' | '_' => {
                                    if c != '_' {
                                        bin.push(c);
                                    }
                                    chars.next();
                                    column += 1;
                                }
                                _ => break,
                            }
                        }
                        let val = i64::from_str_radix(&bin, 2)
                            .or_else(|_| u64::from_str_radix(&bin, 2).map(|v| v as i64))
                            .map_err(|e| format!("Invalid binary literal at {}:{}: {}", start_line, start_col, e))?;
                        tokens.push(SpannedToken {
                            token: Token::IntLiteral(val),
                            line: start_line,
                            column: start_col,
                        });
                        continue;
                    }
                }

                // Decimal integer or float
                while let Some(&c) = chars.peek() {
                    match c {
                        '0'..='9' | '_' => {
                            if c != '_' {
                                num_str.push(c);
                            }
                            chars.next();
                            column += 1;
                        }
                        '.' => {
                            // Check if next after dot is a digit (float) or something else (dot operator)
                            let after_dot = chars.clone().nth(1);
                            if after_dot.map_or(false, |d| d.is_ascii_digit()) {
                                is_float = true;
                                num_str.push(c);
                                chars.next();
                                column += 1;
                            } else {
                                break;
                            }
                        }
                        _ => break,
                    }
                }

                if is_float {
                    // Continue consuming digits after dot
                    while let Some(&c) = chars.peek() {
                        match c {
                            '0'..='9' | '_' => {
                                if c != '_' {
                                    num_str.push(c);
                                }
                                chars.next();
                                column += 1;
                            }
                            _ => break,
                        }
                    }
                }

                // Check for scientific notation exponent (e.g., 1e5, 1.5e-3, 1E10)
                if let Some(&c) = chars.peek() {
                    if c == 'e' || c == 'E' {
                        // Look ahead to ensure there's a digit or sign after e/E
                        let mut lookahead = chars.clone();
                        lookahead.next(); // skip e/E
                        if let Some(&next) = lookahead.peek() {
                            if next.is_ascii_digit() || next == '+' || next == '-' {
                                is_float = true;
                                num_str.push(c);
                                chars.next();
                                column += 1;
                                // Check for optional sign
                                if let Some(&sign) = chars.peek() {
                                    if sign == '+' || sign == '-' {
                                        num_str.push(sign);
                                        chars.next();
                                        column += 1;
                                    }
                                }
                                // Consume exponent digits
                                while let Some(&d) = chars.peek() {
                                    if d.is_ascii_digit() {
                                        num_str.push(d);
                                        chars.next();
                                        column += 1;
                                    } else if d == '_' {
                                        chars.next();
                                        column += 1;
                                    } else {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }

                if is_float {
                    // Check for float suffix
                    let suffix = match chars.peek() {
                        Some('h') => {
                            chars.next();
                            column += 1;
                            Some(FloatSuffix::Half)
                        }
                        Some('q') => {
                            chars.next();
                            column += 1;
                            Some(FloatSuffix::Quad)
                        }
                        _ => None,
                    };

                    let val: f64 = num_str
                        .parse()
                        .map_err(|e| format!("Invalid float literal at {}:{}: {}", start_line, start_col, e))?;
                    tokens.push(SpannedToken {
                        token: Token::FloatLiteral { value: val, suffix },
                        line: start_line,
                        column: start_col,
                    });
                } else {
                    // Check for 'L' or 'l' suffix (long literal, e.g. 2147483647L)
                    if let Some(&suffix) = chars.peek() {
                        if suffix == 'L' || suffix == 'l' {
                            chars.next();
                            column += 1;
                        }
                    }
                    let val: i64 = num_str
                        .parse::<i64>()
                        .or_else(|_| num_str.parse::<u64>().map(|v| v as i64))
                        .map_err(|e| format!("Invalid integer literal at {}:{}: {}", start_line, start_col, e))?;
                    tokens.push(SpannedToken {
                        token: Token::IntLiteral(val),
                        line: start_line,
                        column: start_col,
                    });
                }
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
                        if fourth.map_or(true, |c| !c.is_alphanumeric() && c != '_') {
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

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::token::Token;

    #[allow(dead_code)]
    fn tok(token: Token) -> SpannedToken {
        SpannedToken {
            token,
            line: 0,
            column: 0,
        }
    }

    #[test]
    fn test_micro_test() {
        let src = r#"public fn main(): void { io::println("Hello"); }"#;
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();

        let main_id = Token::Identifier("main".to_string());
        let io_id = Token::Identifier("io".to_string());
        let println_id = Token::Identifier("println".to_string());
        let hello_str = Token::StringLiteral("Hello".to_string());

        let expected = vec![
            &Token::Public,
            &Token::Fn,
            &main_id,
            &Token::LeftParen,
            &Token::RightParen,
            &Token::Colon,
            &Token::Void,
            &Token::LeftBrace,
            &io_id,
            &Token::ColonColon,
            &println_id,
            &Token::LeftParen,
            &hello_str,
            &Token::RightParen,
            &Token::Semicolon,
            &Token::RightBrace,
            &Token::Eof,
        ];

        assert_eq!(token_kinds, expected);
    }

    #[test]
    fn test_integer_literals() {
        let src = "42 0xFF 0o77 0b1010";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let values: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(values[0], &Token::IntLiteral(42));
        assert_eq!(values[1], &Token::IntLiteral(255));
        assert_eq!(values[2], &Token::IntLiteral(63));
        assert_eq!(values[3], &Token::IntLiteral(10));
    }

    #[test]
    fn test_float_with_suffix() {
        let src = "1.5h 2.0q 3.14";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let values: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(values[0], &Token::FloatLiteral { value: 1.5, suffix: Some(FloatSuffix::Half) });
        assert_eq!(values[1], &Token::FloatLiteral { value: 2.0, suffix: Some(FloatSuffix::Quad) });
        assert_eq!(values[2], &Token::FloatLiteral { value: 3.14, suffix: None });
    }

    #[test]
    fn test_string_escapes() {
        let src = r#""hello\nworld\t""#;
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::StringLiteral("hello\nworld\t".to_string()));
    }

    #[test]
    fn test_char_literal() {
        let src = "'a' '\\n' '\\\\'";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::CharLiteral('a'));
        assert_eq!(tokens[1].token, Token::CharLiteral('\n'));
        assert_eq!(tokens[2].token, Token::CharLiteral('\\'));
    }

    #[test]
    fn test_comments() {
        let src = "42 // comment\n43 /* block */ 44";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let ints: Vec<&Token> = tokens.iter()
            .map(|st| &st.token)
            .filter(|t| matches!(t, Token::IntLiteral(_)))
            .collect();
        assert_eq!(ints, vec![&Token::IntLiteral(42), &Token::IntLiteral(43), &Token::IntLiteral(44)]);
    }

    #[test]
    fn test_operators() {
        let src = "== != <= >= << >> && || => -> :: &mut";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let ops: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert!(ops.contains(&&Token::EqualEqual));
        assert!(ops.contains(&&Token::NotEqual));
        assert!(ops.contains(&&Token::LessEqual));
        assert!(ops.contains(&&Token::GreaterEqual));
        assert!(ops.contains(&&Token::LeftShift));
        assert!(ops.contains(&&Token::RightShift));
        assert!(ops.contains(&&Token::AndAnd));
        assert!(ops.contains(&&Token::OrOr));
        assert!(ops.contains(&&Token::FatArrow));
        assert!(ops.contains(&&Token::Arrow));
        assert!(ops.contains(&&Token::ColonColon));
        assert!(ops.contains(&&Token::RefMut));
    }

    #[test]
    fn test_all_type_keywords() {
        let src = "void bool byte short int long vast uvast float double half quad char string size u8 u16 u32 u64";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let expected = vec![
            Token::Void, Token::Bool, Token::Byte, Token::Short,
            Token::Int, Token::Long, Token::Vast, Token::Uvast,
            Token::Float, Token::Double, Token::Half, Token::Quad,
            Token::Char, Token::String, Token::Size,
            Token::U8, Token::U16, Token::U32, Token::U64,
        ];
        let actual: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        for (i, exp) in expected.iter().enumerate() {
            assert_eq!(actual[i], exp, "Mismatch at position {}", i);
        }
    }

    #[test]
    fn test_error_on_unterminated_string() {
        let src = "\"hello";
        let result = tokenize(src);
        assert!(result.is_err());
    }

    #[test]
    fn test_unrecognised_char() {
        let src = "@";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert!(tokens.iter().any(|st| matches!(st.token, Token::Error(_))));
    }

    #[test]
    fn test_operator_identifier() {
        let src = "fn operator+(self, other: Vec2): Vec2";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert!(token_kinds.contains(&&Token::Identifier("operator+".to_string())),
            "Expected operator+ identifier, got: {:?}", token_kinds);
    }

    // -----------------------------------------------------------------------
    // Raw string literal tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_raw_string_simple() {
        let src = r##"r"hello world""##;
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::RawStringLiteral("hello world".to_string()));
    }

    #[test]
    fn test_raw_string_with_hash() {
        let src = r###"r#"hello "world""#"###;
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::RawStringLiteral("hello \"world\"".to_string()));
    }

    #[test]
    fn test_raw_string_no_escapes() {
        let src = r##"r"hello\nworld""##;
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::RawStringLiteral("hello\\nworld".to_string()));
    }

    #[test]
    fn test_raw_string_double_hash() {
        let src = r####"r##"contains #"# hash"##"####;
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::RawStringLiteral("contains #\"# hash".to_string()));
    }

    #[test]
    fn test_raw_string_empty() {
        let src = "r\"\"";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::RawStringLiteral("".to_string()));
    }

    #[test]
    fn test_raw_string_vs_identifier() {
        let src = "return result";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::Return);
        assert_eq!(tokens[1].token, Token::Identifier("result".to_string()));
    }

    // -----------------------------------------------------------------------
    // Byte literal tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_byte_literal_simple() {
        let src = "b'x'";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::ByteLiteral(b'x'));
    }

    #[test]
    fn test_byte_literal_escape() {
        let src = "b'\\n'";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::ByteLiteral(b'\n'));
    }

    #[test]
    fn test_byte_literal_backslash() {
        let src = "b'\\\\'";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::ByteLiteral(b'\\'));
    }

    #[test]
    fn test_byte_literal_hex_escape() {
        let src = "b'\\x41'";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::ByteLiteral(0x41));
    }

    #[test]
    fn test_byte_literal_vs_identifier() {
        let src = "bool byte";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::Bool);
        assert_eq!(tokens[1].token, Token::Byte);
    }

    // -----------------------------------------------------------------------
    // Underscore in numeric literals tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_underscore_decimal() {
        let src = "1_000_000";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::IntLiteral(1000000));
    }

    #[test]
    fn test_underscore_hex() {
        let src = "0xFF_FF";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::IntLiteral(0xFFFF));
    }

    #[test]
    fn test_underscore_binary() {
        let src = "0b1010_1100";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::IntLiteral(0b10101100));
    }

    #[test]
    fn test_underscore_octal() {
        let src = "0o777_000";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::IntLiteral(0o777000));
    }

    #[test]
    fn test_underscore_float() {
        let src = "1_000.5_0";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::FloatLiteral { value: 1000.50, suffix: None });
    }

    // -----------------------------------------------------------------------
    // Binary and octal literal tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_binary_literal() {
        let src = "0b1010";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::IntLiteral(10));
    }

    #[test]
    fn test_binary_literal_uppercase() {
        let src = "0B1100";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::IntLiteral(12));
    }

    #[test]
    fn test_octal_literal() {
        let src = "0o777";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::IntLiteral(511));
    }

    #[test]
    fn test_octal_literal_uppercase() {
        let src = "0O123";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::IntLiteral(83));
    }

    // -----------------------------------------------------------------------
    // Range token tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_dot_dot() {
        let src = "1..10";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert!(token_kinds.contains(&&Token::DotDot), "Expected DotDot token, got: {:?}", token_kinds);
        assert_eq!(token_kinds[0], &Token::IntLiteral(1));
        assert_eq!(token_kinds[1], &Token::DotDot);
        assert_eq!(token_kinds[2], &Token::IntLiteral(10));
    }

    #[test]
    fn test_dot_dot_eq() {
        let src = "1..=10";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert!(token_kinds.contains(&&Token::DotDotEq), "Expected DotDotEq token, got: {:?}", token_kinds);
        assert_eq!(token_kinds[0], &Token::IntLiteral(1));
        assert_eq!(token_kinds[1], &Token::DotDotEq);
        assert_eq!(token_kinds[2], &Token::IntLiteral(10));
    }

    #[test]
    fn test_dot_still_works() {
        let src = "obj.field";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[1], &Token::Dot);
    }

    #[test]
    fn test_float_not_range() {
        let src = "1.5";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::FloatLiteral { value: 1.5, suffix: None });
    }

    // -----------------------------------------------------------------------
    // Compound assignment operator tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_compound_assignment_plus_equal() {
        let src = "x += 1";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[0], &Token::Identifier("x".to_string()));
        assert_eq!(token_kinds[1], &Token::PlusEqual);
        assert_eq!(token_kinds[2], &Token::IntLiteral(1));
    }

    #[test]
    fn test_compound_assignment_minus_equal() {
        let src = "x -= 1";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[1], &Token::MinusEqual);
    }

    #[test]
    fn test_compound_assignment_star_equal() {
        let src = "x *= 2";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[1], &Token::StarEqual);
    }

    #[test]
    fn test_compound_assignment_slash_equal() {
        let src = "x /= 2";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[1], &Token::SlashEqual);
    }

    #[test]
    fn test_compound_assignment_percent_equal() {
        let src = "x %= 2";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[1], &Token::PercentEqual);
    }

    #[test]
    fn test_compound_assignment_ampersand_equal() {
        let src = "x &= 3";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[1], &Token::AmpersandEqual);
    }

    #[test]
    fn test_compound_assignment_pipe_equal() {
        let src = "x |= 3";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[1], &Token::PipeEqual);
    }

    #[test]
    fn test_compound_assignment_caret_equal() {
        let src = "x ^= 3";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[1], &Token::CaretEqual);
    }

    #[test]
    fn test_compound_assignment_left_shift_equal() {
        let src = "x <<= 2";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[1], &Token::LeftShiftEqual);
    }

    #[test]
    fn test_compound_assignment_right_shift_equal() {
        let src = "x >>= 2";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[1], &Token::RightShiftEqual);
    }

    #[test]
    fn test_compound_assignment_vs_regular() {
        let src = "x += 1 y + 1";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[1], &Token::PlusEqual);
        assert_eq!(token_kinds[4], &Token::Plus);
    }

    #[test]
    fn test_left_shift_vs_less_equal() {
        let src = "x <<= 2";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[1], &Token::LeftShiftEqual);
    }

    #[test]
    fn test_less_equal_not_shift_equal() {
        let src = "x <= 2";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[1], &Token::LessEqual);
    }

    #[test]
    fn test_all_compound_assignments() {
        let src = "+= -= *= /= %= &= |= ^= <<= >>=";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let ops: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert!(ops.contains(&&Token::PlusEqual));
        assert!(ops.contains(&&Token::MinusEqual));
        assert!(ops.contains(&&Token::StarEqual));
        assert!(ops.contains(&&Token::SlashEqual));
        assert!(ops.contains(&&Token::PercentEqual));
        assert!(ops.contains(&&Token::AmpersandEqual));
        assert!(ops.contains(&&Token::PipeEqual));
        assert!(ops.contains(&&Token::CaretEqual));
        assert!(ops.contains(&&Token::LeftShiftEqual));
        assert!(ops.contains(&&Token::RightShiftEqual));
    }

    // -----------------------------------------------------------------------
    // Increment/decrement operator tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_plus_plus() {
        let src = "++x";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[0], &Token::PlusPlus);
        assert_eq!(token_kinds[1], &Token::Identifier("x".to_string()));
    }

    #[test]
    fn test_minus_minus() {
        let src = "--x";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[0], &Token::MinusMinus);
        assert_eq!(token_kinds[1], &Token::Identifier("x".to_string()));
    }

    #[test]
    fn test_postfix_plus_plus() {
        let src = "x++";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[0], &Token::Identifier("x".to_string()));
        assert_eq!(token_kinds[1], &Token::PlusPlus);
    }

    #[test]
    fn test_postfix_minus_minus() {
        let src = "x--";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[0], &Token::Identifier("x".to_string()));
        assert_eq!(token_kinds[1], &Token::MinusMinus);
    }

    #[test]
    fn test_plus_plus_vs_plus_equal() {
        let src = "++x x++ x += 1";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[0], &Token::PlusPlus);
        assert_eq!(token_kinds[1], &Token::Identifier("x".to_string()));
        assert_eq!(token_kinds[2], &Token::Identifier("x".to_string()));
        assert_eq!(token_kinds[3], &Token::PlusPlus);
        assert_eq!(token_kinds[4], &Token::Identifier("x".to_string()));
        assert_eq!(token_kinds[5], &Token::PlusEqual);
    }

    #[test]
    fn test_minus_minus_vs_arrow() {
        let src = "--x x-- x -> y";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[0], &Token::MinusMinus);
        assert_eq!(token_kinds[1], &Token::Identifier("x".to_string()));
        assert_eq!(token_kinds[2], &Token::Identifier("x".to_string()));
        assert_eq!(token_kinds[3], &Token::MinusMinus);
        assert_eq!(token_kinds[4], &Token::Identifier("x".to_string()));
        assert_eq!(token_kinds[5], &Token::Arrow);
    }
}
