// String, char, and number literal scanning

use super::token::{FloatSuffix, Token};
use super::SpannedToken;

pub(super) fn scan_string(chars: &mut std::iter::Peekable<std::str::Chars<'_>>, _line: &mut usize, column: &mut usize, start_line: usize, start_col: usize, tokens: &mut Vec<SpannedToken>) -> Result<(), String> {
                chars.next();
                *column += 1;
                let mut s = String::new();
                let mut closed = false;
                while let Some(&c) = chars.peek() {
                    match c {
                        '"' => {
                            chars.next();
                            *column += 1;
                            closed = true;
                            break;
                        }
                        '\\' => {
                            chars.next(); // consume backslash
                            *column += 1;
                            let escaped = match chars.peek() {
                                Some('n') => { chars.next(); *column += 1; '\n' }
                                Some('t') => { chars.next(); *column += 1; '\t' }
                                Some('r') => { chars.next(); *column += 1; '\r' }
                                Some('\\') => { chars.next(); *column += 1; '\\' }
                                Some('"') => { chars.next(); *column += 1; '"' }
                                Some('\'') => { chars.next(); *column += 1; '\'' }
                                Some('0') => { chars.next(); *column += 1; '\0' }
                                Some('b') => { chars.next(); *column += 1; '\x08' }
                                Some('f') => { chars.next(); *column += 1; '\x0c' }
                                Some('v') => { chars.next(); *column += 1; '\x0b' }
                                Some('a') => { chars.next(); *column += 1; '\x07' }
                                Some('x') => {
                                    // \xHH — hex escape (1-2 hex digits)
                                    chars.next();
                                    *column += 1;
                                    let mut hex = String::new();
                                    for _ in 0..2 {
                                        if let Some(&h) = chars.peek() {
                                            if h.is_ascii_hexdigit() {
                                                hex.push(h);
                                                chars.next();
                                                *column += 1;
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
                                    // \u{HHHHHH} — unicode escape with braces
                                    // \uXXXX     — unicode escape without braces (4 hex digits)
                                    chars.next();
                                    *column += 1;
                                    if chars.peek() == Some(&'{') {
                                        chars.next();
                                        *column += 1;
                                        let mut hex = String::new();
                                        while let Some(&h) = chars.peek() {
                                            if h == '}' {
                                                chars.next();
                                                *column += 1;
                                                break;
                                            }
                                            if h.is_ascii_hexdigit() {
                                                hex.push(h);
                                                chars.next();
                                                *column += 1;
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
                                    } else {
                                        // \uXXXX — exactly 4 hex digits without braces
                                        let mut hex = String::new();
                                        for _ in 0..4 {
                                            if let Some(&h) = chars.peek() {
                                                if h.is_ascii_hexdigit() {
                                                    hex.push(h);
                                                    chars.next();
                                                    *column += 1;
                                                } else {
                                                    return Err(format!(
                                                        "Invalid unicode escape \\u{} at {}:{}: expected 4 hex digits",
                                                        hex, start_line, start_col
                                                    ));
                                                }
                                            } else {
                                                return Err(format!(
                                                    "Unterminated unicode escape \\u{} at {}:{}",
                                                    hex, start_line, start_col
                                                ));
                                            }
                                        }
                                        let val = u32::from_str_radix(&hex, 16)
                                            .map_err(|e| format!("Invalid unicode escape \\u{} at {}:{}: {}", hex, start_line, start_col, e))?;
                                        char::from_u32(val).ok_or_else(|| format!(
                                            "Invalid unicode escape \\u{} at {}:{}: not a valid Unicode scalar", hex, start_line, start_col
                                        ))?
                                    }
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
                            *column += 1;
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
                Ok(())
}

pub(super) fn scan_char(
chars: &mut std::iter::Peekable<std::str::Chars<'_>>, _line: &mut usize, column: &mut usize, start_line: usize, start_col: usize, tokens: &mut Vec<SpannedToken>
) -> Result<(), String> {
                chars.next();
                *column += 1;
                let ch_val = match chars.peek() {
                    Some('\\') => {
                        chars.next();
                        *column += 1;
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
                *column += 1;
                match chars.peek() {
                    Some('\'') => {
                        chars.next();
                        *column += 1;
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
                Ok(())
}

pub(super) fn scan_number(
chars: &mut std::iter::Peekable<std::str::Chars<'_>>, _line: &mut usize, column: &mut usize, start_line: usize, start_col: usize, tokens: &mut Vec<SpannedToken>
, ch: char) -> Result<(), String> {
                let mut num_str = String::new();
                let mut is_float = false;

                // Check for hex/oct/bin prefix
                if ch == '0' {
                    let next = chars.clone().nth(1);
                    if next == Some('x') || next == Some('X') {
                        chars.next();
                        chars.next();
                        *column += 2;
                        let mut hex = String::new();
                        while let Some(&c) = chars.peek() {
                            match c {
                                '0'..='9' | 'a'..='f' | 'A'..='F' | '_' => {
                                    if c != '_' {
                                        hex.push(c);
                                    }
                                    chars.next();
                                    *column += 1;
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
                                *column += 1;
                            }
                        }
                        tokens.push(SpannedToken {
                            token: Token::IntLiteral(val),
                            line: start_line,
                            column: start_col,
                        });
                        return Ok(());
                    }
                    if next == Some('o') || next == Some('O') {
                        chars.next();
                        chars.next();
                        *column += 2;
                        let mut oct = String::new();
                        while let Some(&c) = chars.peek() {
                            match c {
                                '0'..='7' | '_' => {
                                    if c != '_' {
                                        oct.push(c);
                                    }
                                    chars.next();
                                    *column += 1;
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
                        return Ok(());
                    }
                    if next == Some('b') || next == Some('B') {
                        chars.next();
                        chars.next();
                        *column += 2;
                        let mut bin = String::new();
                        while let Some(&c) = chars.peek() {
                            match c {
                                '0' | '1' | '_' => {
                                    if c != '_' {
                                        bin.push(c);
                                    }
                                    chars.next();
                                    *column += 1;
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
                        return Ok(());
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
                            *column += 1;
                        }
                        '.' => {
                            // Check if next after dot is a digit (float) or something else (dot operator)
                            let after_dot = chars.clone().nth(1);
                            if after_dot.is_some_and(|d| d.is_ascii_digit()) {
                                is_float = true;
                                num_str.push(c);
                                chars.next();
                                *column += 1;
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
                                *column += 1;
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
                                *column += 1;
                                // Check for optional sign
                                if let Some(&sign) = chars.peek() {
                                    if sign == '+' || sign == '-' {
                                        num_str.push(sign);
                                        chars.next();
                                        *column += 1;
                                    }
                                }
                                // Consume exponent digits
                                while let Some(&d) = chars.peek() {
                                    if d.is_ascii_digit() {
                                        num_str.push(d);
                                        chars.next();
                                        *column += 1;
                                    } else if d == '_' {
                                        chars.next();
                                        *column += 1;
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
                            *column += 1;
                            Some(FloatSuffix::Half)
                        }
                        Some('q') => {
                            chars.next();
                            *column += 1;
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
                            *column += 1;
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
                Ok(())
}
