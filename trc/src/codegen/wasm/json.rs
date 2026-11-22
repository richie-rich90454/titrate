//! Minimal JSON literal parsing for the WASM backend.
//!
//! The bytecode VM represents parsed JSON with dynamic collections. WebAssembly
//! needs a static element type, so only string-literal JSON with homogeneous
//! collection values is lowered; other inputs produce an explicit `wasm:`
//! error instead of a silent miscompile.

#[derive(Clone, Debug)]
pub(crate) enum JsonLit {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Array(Vec<JsonLit>),
    Object(Vec<(String, JsonLit)>),
}

pub(crate) fn parse(src: &str) -> Result<JsonLit, String> {
    let mut p = Parser { bytes: src.as_bytes(), pos: 0 };
    let v = p.value()?;
    p.ws();
    if p.pos != p.bytes.len() {
        return Err("trailing characters in JSON literal".to_string());
    }
    Ok(v)
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn ws(&mut self) {
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.ws();
        self.bytes.get(self.pos).copied()
    }

    fn eat(&mut self, c: u8) -> Result<(), String> {
        if self.peek() == Some(c) {
            self.pos += 1;
            Ok(())
        } else {
            Err(format!("expected '{}' in JSON literal", c as char))
        }
    }

    fn value(&mut self) -> Result<JsonLit, String> {
        match self.peek().ok_or("unexpected end of JSON literal")? {
            b'{' => self.object(),
            b'[' => self.array(),
            b'"' => Ok(JsonLit::Str(self.string()?)),
            b't' => {
                self.keyword(b"true")?;
                Ok(JsonLit::Bool(true))
            }
            b'f' => {
                self.keyword(b"false")?;
                Ok(JsonLit::Bool(false))
            }
            b'n' => {
                self.keyword(b"null")?;
                Ok(JsonLit::Null)
            }
            b'-' | b'0'..=b'9' => self.number(),
            other => Err(format!("unexpected '{}' in JSON literal", other as char)),
        }
    }

    fn keyword(&mut self, kw: &[u8]) -> Result<(), String> {
        if self.bytes.get(self.pos..self.pos + kw.len()) == Some(kw) {
            self.pos += kw.len();
            Ok(())
        } else {
            Err("invalid JSON keyword".to_string())
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.eat(b'"')?;
        let mut out = String::new();
        loop {
            let c = *self.bytes.get(self.pos).ok_or("unterminated JSON string")?;
            self.pos += 1;
            match c {
                b'"' => return Ok(out),
                b'\\' => {
                    let e = *self.bytes.get(self.pos).ok_or("unterminated JSON escape")?;
                    self.pos += 1;
                    match e {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let hex = self
                                .bytes
                                .get(self.pos..self.pos + 4)
                                .ok_or("short unicode escape")?;
                            let s = std::str::from_utf8(hex).map_err(|_| "bad unicode escape")?;
                            let n = u32::from_str_radix(s, 16).map_err(|_| "bad unicode escape")?;
                            out.push(char::from_u32(n).ok_or("bad unicode scalar")?);
                            self.pos += 4;
                        }
                        _ => return Err("invalid JSON escape".to_string()),
                    }
                }
                _ => out.push(c as char),
            }
        }
    }

    fn number(&mut self) -> Result<JsonLit, String> {
        let start = self.pos;
        let mut float = false;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        while let Some(c) = self.bytes.get(self.pos).copied() {
            match c {
                b'0'..=b'9' => self.pos += 1,
                b'.' | b'e' | b'E' | b'+' | b'-' => {
                    float = true;
                    self.pos += 1;
                }
                _ => break,
            }
        }
        let s = std::str::from_utf8(&self.bytes[start..self.pos]).map_err(|_| "bad number")?;
        if float {
            s.parse::<f64>().map(JsonLit::Float).map_err(|_| "bad number".to_string())
        } else {
            s.parse::<i64>().map(JsonLit::Int).map_err(|_| "bad number".to_string())
        }
    }

    fn array(&mut self) -> Result<JsonLit, String> {
        self.eat(b'[')?;
        let mut out = Vec::new();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(JsonLit::Array(out));
        }
        loop {
            out.push(self.value()?);
            match self.peek().ok_or("unterminated JSON array")? {
                b',' => {
                    self.pos += 1;
                }
                b']' => {
                    self.pos += 1;
                    return Ok(JsonLit::Array(out));
                }
                _ => return Err("expected ',' or ']' in JSON array".to_string()),
            }
        }
    }

    fn object(&mut self) -> Result<JsonLit, String> {
        self.eat(b'{')?;
        let mut out = Vec::new();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(JsonLit::Object(out));
        }
        loop {
            let key = self.string()?;
            self.eat(b':')?;
            let val = self.value()?;
            out.push((key, val));
            match self.peek().ok_or("unterminated JSON object")? {
                b',' => {
                    self.pos += 1;
                }
                b'}' => {
                    self.pos += 1;
                    return Ok(JsonLit::Object(out));
                }
                _ => return Err("expected ',' or '}' in JSON object".to_string()),
            }
        }
    }
}
