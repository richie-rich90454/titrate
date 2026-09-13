// Static-call natives, part A: io, numbers, and core strings


use super::super::value::Value;
use super::Vm;
use std::collections::HashMap;
use std::rc::Rc;

impl Vm {
    pub(super) fn exec_static_part_a(&mut self, class_name: &str, method_name: &str, arg_count: u8) -> Result<bool, String> {
        match (class_name, method_name) {
            ("io", "println") => {
                let arg_start = self.stack.len() - arg_count as usize;
                let args: Vec<Value> = self.stack.drain(arg_start..).collect();
                let output = if args.is_empty() {
                    String::new()
                } else {
                    args[0].display_string()
                };
                self.output.push(output);
                self.push(Value::Void);
            }
            // io::print (same as println but no newline)
            ("io", "print") => {
                let arg_start = self.stack.len() - arg_count as usize;
                let args: Vec<Value> = self.stack.drain(arg_start..).collect();
                let output = if args.is_empty() {
                    String::new()
                } else {
                    args[0].display_string()
                };
                // Append to last output line instead of pushing new line
                if let Some(last) = self.output.last_mut() {
                    last.push_str(&output);
                } else {
                    self.output.push(output);
                }
                self.push(Value::Void);
            }
            // io::readLine - read a line from stdin
            ("io", "readLine") => {
                let arg_start = self.stack.len() - arg_count as usize;
                let _args: Vec<Value> = self.stack.drain(arg_start..).collect();
                let mut input = String::new();
                if std::io::stdin().read_line(&mut input).is_ok() {
                    let trimmed = input.trim_end_matches('\n').trim_end_matches('\r').to_string();
                    self.push(Value::String(Rc::new(trimmed)));
                } else {
                    self.push(Value::String(Rc::new(String::new())));
                }
            }
            // io::readAll - read all of stdin
            ("io", "readAll") => {
                let arg_start = self.stack.len() - arg_count as usize;
                let _args: Vec<Value> = self.stack.drain(arg_start..).collect();
                let mut input = String::new();
                if std::io::Read::read_to_string(&mut std::io::stdin(), &mut input).is_ok() {
                    self.push(Value::String(Rc::new(input)));
                } else {
                    self.push(Value::String(Rc::new(String::new())));
                }
            }
            // io::stderr - switch to stderr mode (no-op in VM, just mark intent)
            ("io", "stderr") => {
                let arg_start = self.stack.len() - arg_count as usize;
                let _args: Vec<Value> = self.stack.drain(arg_start..).collect();
                self.push(Value::Void);
            }
            // io::eprintln - print to stderr
            ("io", "eprintln") => {
                let arg_start = self.stack.len() - arg_count as usize;
                let args: Vec<Value> = self.stack.drain(arg_start..).collect();
                let output = if args.is_empty() {
                    String::new()
                } else {
                    args[0].display_string()
                };
                eprintln!("{}", output);
                self.push(Value::Void);
            }
            // io::eprint - print to stderr without newline
            ("io", "eprint") => {
                let arg_start = self.stack.len() - arg_count as usize;
                let args: Vec<Value> = self.stack.drain(arg_start..).collect();
                let output = if args.is_empty() {
                    String::new()
                } else {
                    args[0].display_string()
                };
                eprint!("{}", output);
                self.push(Value::Void);
            }
            // Integer::toString
            ("Integer" | "int", "toString") => {
                let val = self.pop();
                let s = val.display_string();
                self.push(Value::String(Rc::new(s)));
            }
            // All numeric/wrapper type toString methods
            ("Double" | "double" | "Float" | "float" | "Long" | "long" |
             "Byte" | "byte" | "Short" | "short" | "Half" | "half" |
             "Quad" | "quad" | "Vast" | "vast" | "Uvast" | "uvast" |
             "Boolean" | "bool" | "Char" | "char" | "String_" | "string", "toString") => {
                let val = self.pop();
                let s = val.display_string();
                self.push(Value::String(Rc::new(s)));
            }
            // Integer::parseInt
            ("Integer" | "int", "parseInt") => {
                let val = self.pop();
                match &val {
                    Value::String(s) => match s.trim().parse::<i64>() {
                        Ok(n) => self.push(Value::Long(n)),
                        Err(_) => {
                            return Err(format!("Invalid integer: {}", s))
                        }
                    },
                    Value::Char(c) => {
                        let s: String = c.to_string();
                        match s.trim().parse::<i64>() {
                            Ok(n) => self.push(Value::Long(n)),
                            Err(_) => {
                                return Err(format!("Invalid integer: {}", s))
                            }
                        }
                    }
                    _ => {
                        return Err(format!(
                            "Integer.parseInt: expected String, got {:?}",
                            val
                        ))
                    }
                }
            }
            // Integer::parseOr - parse string to int, return default on failure
            ("Integer" | "int", "parseOr") => {
                let default_val = self.pop();
                let val = self.pop();
                let default = match &default_val {
                    Value::Int(n) => *n as i64,
                    Value::Long(n) => *n,
                    _ => 0,
                };
                match &val {
                    Value::String(s) => match s.trim().parse::<i64>() {
                        Ok(n) => self.push(Value::Long(n)),
                        Err(_) => self.push(Value::Long(default)),
                    },
                    _ => self.push(Value::Long(default)),
                }
            }
            // Integer::min / Integer::max
            ("Integer" | "int", "min") => {
                let b = self.pop();
                let a = self.pop();
                let av = a.to_i64().unwrap_or(0);
                let bv = b.to_i64().unwrap_or(0);
                self.push(Value::Int(av.min(bv) as i32));
            }
            ("Integer" | "int", "max") => {
                let b = self.pop();
                let a = self.pop();
                let av = a.to_i64().unwrap_or(0);
                let bv = b.to_i64().unwrap_or(0);
                self.push(Value::Int(av.max(bv) as i32));
            }
            // Integer::sum
            ("Integer" | "int", "sum") => {
                let b = self.pop();
                let a = self.pop();
                let av = a.to_i64().unwrap_or(0);
                let bv = b.to_i64().unwrap_or(0);
                self.push(Value::Int((av + bv) as i32));
            }
            // Long::parse
            ("Long" | "long", "parse") => {
                let val = self.pop();
                match &val {
                    Value::String(s) => match s.trim().parse::<i64>() {
                        Ok(n) => self.push(Value::Long(n)),
                        Err(_) => {
                            return Err(format!("Invalid long: {}", s))
                        }
                    },
                    _ => return Err(format!("Long.parse: expected String, got {:?}", val)),
                }
            }
            // Short::parse
            ("Short" | "short", "parse") => {
                let val = self.pop();
                match &val {
                    Value::String(s) => match s.trim().parse::<i16>() {
                        Ok(n) => self.push(Value::Short(n)),
                        Err(_) => {
                            return Err(format!("Invalid short: {}", s))
                        }
                    },
                    _ => return Err(format!("Short.parse: expected String, got {:?}", val)),
                }
            }
            // Byte::parse
            ("Byte" | "byte", "parse") => {
                let val = self.pop();
                match &val {
                    Value::String(s) => match s.trim().parse::<i8>() {
                        Ok(n) => self.push(Value::Byte(n)),
                        Err(_) => {
                            return Err(format!("Invalid byte: {}", s))
                        }
                    },
                    _ => return Err(format!("Byte.parse: expected String, got {:?}", val)),
                }
            }
            // Float::parse
            ("Float" | "float", "parse") => {
                let val = self.pop();
                match &val {
                    Value::String(s) => match s.trim().parse::<f32>() {
                        Ok(n) => self.push(Value::Float(n)),
                        Err(_) => {
                            return Err(format!("Invalid float: {}", s))
                        }
                    },
                    _ => return Err(format!("Float.parse: expected String, got {:?}", val)),
                }
            }
            // Half::parse
            ("Half" | "half", "parse") => {
                let val = self.pop();
                match &val {
                    Value::String(s) => match s.trim().parse::<f32>() {
                        Ok(n) => self.push(Value::Half(n)),
                        Err(_) => {
                            return Err(format!("Invalid half: {}", s))
                        }
                    },
                    _ => return Err(format!("Half.parse: expected String, got {:?}", val)),
                }
            }
            // Quad::parse
            ("Quad" | "quad", "parse") => {
                let val = self.pop();
                match &val {
                    Value::String(s) => match s.trim().parse::<f64>() {
                        Ok(n) => self.push(Value::Quad(n)),
                        Err(_) => {
                            return Err(format!("Invalid quad: {}", s))
                        }
                    },
                    _ => return Err(format!("Quad.parse: expected String, got {:?}", val)),
                }
            }
            // Vast::parse
            ("Vast" | "vast", "parse") => {
                let val = self.pop();
                match &val {
                    Value::String(s) => match s.trim().parse::<i128>() {
                        Ok(n) => self.push(Value::Vast(n)),
                        Err(_) => {
                            return Err(format!("Invalid vast: {}", s))
                        }
                    },
                    _ => return Err(format!("Vast.parse: expected String, got {:?}", val)),
                }
            }
            // Uvast::parse
            ("Uvast" | "uvast", "parse") => {
                let val = self.pop();
                match &val {
                    Value::String(s) => match s.trim().parse::<u128>() {
                        Ok(n) => self.push(Value::Uvast(n)),
                        Err(_) => {
                            return Err(format!("Invalid uvast: {}", s))
                        }
                    },
                    _ => return Err(format!("Uvast.parse: expected String, got {:?}", val)),
                }
            }
            // Boolean::parseBoolean
            ("Boolean" | "bool", "parseBoolean") => {
                let val = self.pop();
                match &val {
                    Value::String(s) => {
                        let b = s.as_str() == "true";
                        self.push(Value::Bool(b));
                    }
                    Value::Bool(b) => self.push(Value::Bool(*b)),
                    Value::Int(i) => self.push(Value::Bool(*i != 0)),
                    Value::Long(i) => self.push(Value::Bool(*i != 0)),
                    _ => self.push(Value::Bool(false)),
                }
            }
            // Double::parseDouble
            ("Double" | "double", "parseDouble") => {
                let val = self.pop();
                match &val {
                    Value::String(s) => match s.trim().parse::<f64>() {
                        Ok(n) => self.push(Value::Double(n)),
                        Err(_) => self.push(Value::Double(f64::NAN)),
                    },
                    _ => return Err(format!("Double.parseDouble: expected String, got {:?}", val)),
                }
            }
            // Double::NaN, Double::POSITIVE_INFINITY, Double::NEGATIVE_INFINITY
            ("Double" | "double", "NaN") => {
                let arg_start = self.stack.len() - arg_count as usize;
                let _args: Vec<Value> = self.stack.drain(arg_start..).collect();
                self.push(Value::Double(f64::NAN));
            }
            ("Double" | "double", "POSITIVE_INFINITY") => {
                let arg_start = self.stack.len() - arg_count as usize;
                let _args: Vec<Value> = self.stack.drain(arg_start..).collect();
                self.push(Value::Double(f64::INFINITY));
            }
            ("Double" | "double", "NEGATIVE_INFINITY") => {
                let arg_start = self.stack.len() - arg_count as usize;
                let _args: Vec<Value> = self.stack.drain(arg_start..).collect();
                self.push(Value::Double(f64::NEG_INFINITY));
            }
            // Double::isNaN
            ("Double" | "double", "isNaN") => {
                let val = self.pop();
                match &val {
                    Value::Double(d) => self.push(Value::Bool(d.is_nan())),
                    Value::Float(d) => self.push(Value::Bool(d.is_nan())),
                    Value::Half(d) => self.push(Value::Bool(d.is_nan())),
                    Value::Quad(d) => self.push(Value::Bool(d.is_nan())),
                    _ => self.push(Value::Bool(false)),
                }
            }
            // Double::isInfinite
            ("Double" | "double", "isInfinite") => {
                let val = self.pop();
                match &val {
                    Value::Double(d) => self.push(Value::Bool(d.is_infinite())),
                    Value::Float(d) => self.push(Value::Bool(d.is_infinite())),
                    Value::Half(d) => self.push(Value::Bool(d.is_infinite())),
                    Value::Quad(d) => self.push(Value::Bool(d.is_infinite())),
                    _ => self.push(Value::Bool(false)),
                }
            }
            // Double::isFinite
            ("Double" | "double", "isFinite") => {
                let val = self.pop();
                match &val {
                    Value::Double(d) => self.push(Value::Bool(d.is_finite())),
                    Value::Float(d) => self.push(Value::Bool(d.is_finite())),
                    Value::Half(d) => self.push(Value::Bool(d.is_finite())),
                    Value::Quad(d) => self.push(Value::Bool(d.is_finite())),
                    _ => self.push(Value::Bool(false)),
                }
            }
            // Double::compare
            ("Double" | "double", "compare") => {
                let b = self.pop();
                let a = self.pop();
                let av = a.to_f64().unwrap_or(0.0);
                let bv = b.to_f64().unwrap_or(0.0);
                if av < bv {
                    self.push(Value::Int(-1));
                } else if av > bv {
                    self.push(Value::Int(1));
                } else {
                    self.push(Value::Int(0));
                }
            }
            // String::reverse
            ("String" | "string", "reverse") => {
                let val = self.pop();
                match &val {
                    Value::String(s) => {
                        let reversed: String = s.chars().rev().collect();
                        self.push(Value::String(Rc::new(reversed)));
                    }
                    Value::Null => self.push(Value::Null),
                    _ => return Err(format!("String.reverse: expected String, got {:?}", val)),
                }
            }
            // System::currentTimeMillis
            ("System", "currentTimeMillis") => {
                let arg_start = self.stack.len() - arg_count as usize;
                let _args: Vec<Value> = self.stack.drain(arg_start..).collect();
                let millis = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0);
                self.push(Value::Long(millis));
            }
            // System::nanoTime
            ("System", "nanoTime") => {
                let arg_start = self.stack.len() - arg_count as usize;
                let _args: Vec<Value> = self.stack.drain(arg_start..).collect();
                let nanos = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos() as i64)
                    .unwrap_or(0);
                self.push(Value::Long(nanos));
            }
            // File::readText / File::writeText
            ("File", "readText") => {
                let path = self.pop();
                match &path {
                    Value::String(p) => {
                        let resolved = self.resolve_path(p);
                        match std::fs::read_to_string(&resolved) {
                            Ok(content) => self.push(Value::String(Rc::new(content))),
                            Err(_) => self.push(Value::String(Rc::new(String::new()))),
                        }
                    }
                    _ => return Err(format!("File.readText: expected String, got {:?}", path)),
                }
            }
            ("File", "writeText") => {
                let content = self.pop();
                let path = self.pop();
                match (&path, &content) {
                    (Value::String(p), Value::String(c)) => {
                        let resolved = self.resolve_path(p);
                        match std::fs::write(&resolved, c.as_bytes()) {
                            Ok(()) => self.push(Value::Bool(true)),
                            Err(_) => self.push(Value::Bool(false)),
                        }
                    }
                    _ => return Err(format!("File.writeText: expected (String, String), got ({:?}, {:?})", path, content)),
                }
            }
            // String::length
            ("String" | "string", "length") => {
                let val = self.pop();
                match &val {
                    Value::String(s) => self.push(Value::Int(s.chars().count() as i32)),
                    Value::Char(_) => self.push(Value::Int(1)),
                    Value::Null => self.push(Value::Int(0)),
                    _ => {
                        return Err(format!(
                            "String.length: expected String, got {:?}",
                            val
                        ))
                    }
                }
            }
            // String::charAt
            ("String" | "string", "charAt") => {
                let index = self.pop();
                let val = self.pop();
                match (&val, &index.to_i64()) {
                    (Value::String(s), Some(i)) => {
                        let idx = *i as usize;
                        if idx < s.chars().count() {
                            self.push(Value::Char(s.chars().nth(idx).unwrap()));
                        } else {
                            return Err(format!(
                                "String.charAt: index {} out of bounds",
                                idx
                            ));
                        }
                    }
                    (Value::Char(c), Some(0)) => {
                        self.push(Value::Char(*c));
                    }
                    (Value::Char(c), Some(i)) => {
                        let s: String = c.to_string();
                        let idx = *i as usize;
                        if idx < s.chars().count() {
                            self.push(Value::Char(s.chars().nth(idx).unwrap()));
                        } else {
                            return Err(format!(
                                "String.charAt: index {} out of bounds for char",
                                idx
                            ));
                        }
                    }
                    _ => {
                        return Err(format!(
                            "String.charAt: expected (String, Int), got ({:?}, {:?})",
                            val, index
                        ))
                    }
                }
            }
            // String::substring
            ("String" | "string", "substring") => {
                // Support both 2-arg (s, start) and 3-arg (s, start, end) forms.
                if arg_count >= 3 {
                    let end = self.pop();
                    let start = self.pop();
                    let val = self.pop();
                    let s_idx_opt = start.to_i64();
                    let e_idx_opt = end.to_i64();
                    match (&val, &s_idx_opt, &e_idx_opt) {
                        (Value::String(s), Some(si), Some(ei)) => {
                            let s_idx = *si as usize;
                            let e_idx = *ei as usize;
                            let substring: String = s.chars().skip(s_idx).take(e_idx.saturating_sub(s_idx)).collect();
                            self.push(Value::String(Rc::new(substring)));
                        }
                        _ => {
                            return Err("String.substring: type mismatch".to_string())
                        }
                    }
                } else {
                    let start = self.pop();
                    let val = self.pop();
                    let s_idx_opt = start.to_i64();
                    match (&val, &s_idx_opt) {
                        (Value::String(s), Some(si)) => {
                            let s_idx = *si as usize;
                            let substring: String = s.chars().skip(s_idx).collect();
                            self.push(Value::String(Rc::new(substring)));
                        }
                        _ => {
                            return Err("String.substring: type mismatch".to_string())
                        }
                    }
                }
            }
            // Array::new
            ("Array" | "array", "new") => {
                let size = self.pop();
                match size {
                    Value::Int(n) => {
                        let elements = vec![Value::Null; n as usize];
                        self.push(Value::Array { elements });
                    }
                    _ => return Err(format!("Array.new: expected Int size, got {:?}", size)),
                }
            }
            // File::readFile
            ("File", "readFile") => {
                let val = self.pop();
                match &val {
                    Value::String(path) => {
                        let resolved = self.resolve_path(path.as_str());
                        match std::fs::read_to_string(&resolved) {
                            Ok(content) => self.push(Value::String(Rc::new(content))),
                            Err(_) => self.push(Value::String(Rc::new(String::new()))),
                        }
                    }
                    _ => return Err(format!("File.readFile: expected String, got {:?}", val)),
                }
            }
            // File::writeFile
            ("File", "writeFile") => {
                let content = self.pop();
                let path = self.pop();
                match (&path, &content) {
                    (Value::String(p), Value::String(c)) => {
                        let resolved = self.resolve_path(p.as_str());
                        match std::fs::write(&resolved, c.as_str()) {
                            Ok(()) => self.push(Value::Bool(true)),
                            Err(_) => self.push(Value::Bool(false)),
                        }
                    }
                    _ => return Err("File.writeFile: expected (String, String)".to_string()),
                }
            }
            // File::readLines
            ("File", "readLines") => {
                let val = self.pop();
                match &val {
                    Value::String(path) => {
                        let resolved = self.resolve_path(path.as_str());
                        match std::fs::read_to_string(&resolved) {
                            Ok(content) => {
                                let lines: Vec<Value> = content.lines()
                                    .map(|line| Value::String(Rc::new(line.to_string())))
                                    .collect();
                                self.push(Value::Array { elements: lines });
                            }
                            Err(_) => self.push(Value::Array { elements: vec![] }),
                        }
                    }
                    _ => return Err(format!("File.readLines: expected String, got {:?}", val)),
                }
            }
            // File::open removed: fall through to user-defined open() in File.tr
            // String::split
            ("String", "split") => {
                let delim = self.pop();
                let s = self.pop();
                match (&s, &delim) {
                    (Value::String(str_val), Value::String(d)) => {
                        let parts: Vec<Value> = str_val.split(d.as_str())
                            .map(|part| Value::String(Rc::new(part.to_string())))
                            .collect();
                        let mut al_fields = HashMap::new();
                        al_fields.insert("_elements".to_string(), Value::Array { elements: parts });
                        self.push(Value::ClassInstance {
                            class_name: "ArrayList".to_string(),
                            fields: Rc::new(std::cell::RefCell::new(al_fields)),
                            vtable: HashMap::new(),
                        });
                    }
                    (Value::String(str_val), Value::Char(d)) => {
                        let parts: Vec<Value> = str_val.split(*d)
                            .map(|part| Value::String(Rc::new(part.to_string())))
                            .collect();
                        let mut al_fields = HashMap::new();
                        al_fields.insert("_elements".to_string(), Value::Array { elements: parts });
                        self.push(Value::ClassInstance {
                            class_name: "ArrayList".to_string(),
                            fields: Rc::new(std::cell::RefCell::new(al_fields)),
                            vtable: HashMap::new(),
                        });
                    }
                    _ => return Err("String.split: expected (String, String) or (String, Char)".to_string()),
                }
            }
            // String::indexOf
            ("String" | "string", "indexOf") => {
                // Support both 2-arg (s, sub) and 3-arg (s, sub, start) forms.
                let start_idx: i32 = if arg_count > 2 {
                    match self.pop() {
                        Value::Int(i) => i,
                        Value::Long(i) => i as i32,
                        v => return Err(format!("String.indexOf: start index must be int, got {:?}", v)),
                    }
                } else {
                    0
                };
                let sub = self.pop();
                let val = self.pop();
                match (&val, &sub) {
                    (Value::String(s), Value::String(needle)) => {
                        let char_start = if start_idx < 0 { 0 } else { start_idx as usize };
                        let byte_start = s.char_indices()
                            .nth(char_start)
                            .map(|(b, _)| b)
                            .unwrap_or(s.len());
                        if byte_start > s.len() {
                            self.push(Value::Int(-1));
                        } else {
                            match s[byte_start..].find(needle.as_str()) {
                                Some(byte_pos) => {
                                    let abs_byte = byte_start + byte_pos;
                                    let char_pos = s[..abs_byte].chars().count() as i32;
                                    self.push(Value::Int(char_pos));
                                }
                                None => self.push(Value::Int(-1)),
                            }
                        }
                    }
                    (Value::String(s), Value::Char(needle)) => {
                        let char_start = if start_idx < 0 { 0 } else { start_idx as usize };
                        let byte_start = s.char_indices()
                            .nth(char_start)
                            .map(|(b, _)| b)
                            .unwrap_or(s.len());
                        if byte_start > s.len() {
                            self.push(Value::Int(-1));
                        } else {
                            match s[byte_start..].find(*needle) {
                                Some(byte_pos) => {
                                    let abs_byte = byte_start + byte_pos;
                                    let char_pos = s[..abs_byte].chars().count() as i32;
                                    self.push(Value::Int(char_pos));
                                }
                                None => self.push(Value::Int(-1)),
                            }
                        }
                    }
                    _ => return Err(format!(
                        "String.indexOf: expected (String, String) or (String, Char), got ({:?}, {:?})",
                        val, sub
                    )),
                }
            }
            // String::lastIndexOf
            ("String" | "string", "lastIndexOf") => {
                let sub = self.pop();
                let val = self.pop();
                match (&val, &sub) {
                    (Value::String(s), Value::String(needle)) => {
                        match s.rfind(needle.as_str()) {
                            Some(byte_pos) => {
                                let char_pos = s[..byte_pos].chars().count() as i32;
                                self.push(Value::Int(char_pos));
                            }
                            None => self.push(Value::Int(-1)),
                        }
                    }
                    (Value::String(s), Value::Char(needle)) => {
                        match s.rfind(*needle) {
                            Some(byte_pos) => {
                                let char_pos = s[..byte_pos].chars().count() as i32;
                                self.push(Value::Int(char_pos));
                            }
                            None => self.push(Value::Int(-1)),
                        }
                    }
                    _ => return Err(format!(
                        "String.lastIndexOf: expected (String, String) or (String, Char), got ({:?}, {:?})",
                        val, sub
                    )),
                }
            }
            // String::contains
            ("String" | "string", "contains") => {
                let sub = self.pop();
                let val = self.pop();
                match (&val, &sub) {
                    (Value::String(s), Value::String(needle)) => {
                        self.push(Value::Bool(s.contains(needle.as_str())));
                    }
                    (Value::String(s), Value::Char(needle)) => {
                        self.push(Value::Bool(s.contains(*needle)));
                    }
                    _ => return Err(format!(
                        "String.contains: expected (String, String) or (String, Char), got ({:?}, {:?})",
                        val, sub
                    )),
                }
            }
            // String::toUpperCase
            ("String" | "string", "toUpperCase") => {
                let val = self.pop();
                match &val {
                    Value::String(s) => self.push(Value::String(Rc::new(s.to_uppercase()))),
                    Value::Char(c) => {
                        let upper: String = c.to_uppercase().collect();
                        if upper.chars().count() == 1 {
                            self.push(Value::Char(upper.chars().next().unwrap()));
                        } else {
                            self.push(Value::String(Rc::new(upper)));
                        }
                    }
                    Value::Null => self.push(Value::Null),
                    _ => return Err(format!("String.toUpperCase: expected String, got {:?}", val)),
                }
            }
            // String::toLowerCase
            ("String" | "string", "toLowerCase") => {
                let val = self.pop();
                match &val {
                    Value::String(s) => self.push(Value::String(Rc::new(s.to_lowercase()))),
                    Value::Char(c) => {
                        let lower: String = c.to_lowercase().collect();
                        if lower.chars().count() == 1 {
                            self.push(Value::Char(lower.chars().next().unwrap()));
                        } else {
                            self.push(Value::String(Rc::new(lower)));
                        }
                    }
                    Value::Null => self.push(Value::Null),
                    _ => return Err(format!("String.toLowerCase: expected String, got {:?}", val)),
                }
            }
            // String::trim
            ("String" | "string", "trim") => {
                let val = self.pop();
                match &val {
                    Value::String(s) => self.push(Value::String(Rc::new(s.trim().to_string()))),
                    Value::Null => self.push(Value::Null),
                    _ => return Err(format!("String.trim: expected String, got {:?}", val)),
                }
            }
            // String::startsWith
            ("String" | "string", "startsWith") => {
                let prefix = self.pop();
                let val = self.pop();
                match (&val, &prefix) {
                    (Value::String(s), Value::String(p)) => {
                        self.push(Value::Bool(s.starts_with(p.as_str())));
                    }
                    _ => return Err(format!(
                        "String.startsWith: expected (String, String), got ({:?}, {:?})",
                        val, prefix
                    )),
                }
            }
            // String::endsWith
            ("String" | "string", "endsWith") => {
                let suffix = self.pop();
                let val = self.pop();
                match (&val, &suffix) {
                    (Value::String(s), Value::String(suf)) => {
                        self.push(Value::Bool(s.ends_with(suf.as_str())));
                    }
                    _ => return Err(format!(
                        "String.endsWith: expected (String, String), got ({:?}, {:?})",
                        val, suffix
                    )),
                }
            }
            // String::replace
            ("String" | "string", "replace") => {
                let replacement = self.pop();
                let target = self.pop();
                let val = self.pop();
                match (&val, &target, &replacement) {
                    (Value::String(s), Value::String(t), Value::String(r)) => {
                        self.push(Value::String(Rc::new(s.replace(t.as_str(), r.as_str()))));
                    }
                    _ => return Err(format!(
                        "String.replace: expected (String, String, String), got ({:?}, {:?}, {:?})",
                        val, target, replacement
                    )),
                }
            }
            // String::isEmpty
            ("String" | "string", "isEmpty") => {
                let val = self.pop();
                match &val {
                    Value::String(s) => self.push(Value::Bool(s.is_empty())),
                    Value::Null => self.push(Value::Bool(true)),
                    _ => return Err(format!("String.isEmpty: expected String, got {:?}", val)),
                }
            }
            // String::concat
            ("String" | "string", "concat") => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::String(x), Value::String(y)) => {
                        self.push(Value::String(Rc::new(format!("{}{}", x, y))));
                    }
                    _ => return Err(format!(
                        "String.concat: expected (String, String), got ({:?}, {:?})",
                        a, b
                    )),
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}
