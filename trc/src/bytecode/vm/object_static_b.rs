// Static-call natives, part B: strings, structs, archives


use super::super::value::Value;
use super::Vm;
use std::collections::HashMap;
use std::rc::Rc;
use super::object::{parse_struct_format, simple_hmac, tar_build_archive, tar_parse_archive, value_to_f64, value_to_i64};

impl Vm {
    pub(super) fn exec_static_part_b(&mut self, class_name: &str, method_name: &str, arg_count: u8) -> Result<bool, String> {
        match (class_name, method_name) {
            // String::isBlank
            ("String" | "string", "isBlank") => {
                let val = self.pop();
                match &val {
                    Value::String(s) => self.push(Value::Bool(s.trim().is_empty())),
                    Value::Null => self.push(Value::Bool(true)),
                    _ => return Err(format!("String.isBlank: expected String, got {:?}", val)),
                }
            }
            // String::rfind
            ("String" | "string", "rfind") => {
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
                    _ => return Err(format!(
                        "String.rfind: expected (String, String), got ({:?}, {:?})",
                        val, sub
                    )),
                }
            }
            // String::compareTo
            ("String" | "string", "compareTo") => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::String(x), Value::String(y)) => {
                        self.push(Value::Int(x.cmp(y) as i32));
                    }
                    _ => return Err(format!(
                        "String.compareTo: expected (String, String), got ({:?}, {:?})",
                        a, b
                    )),
                }
            }
            // String::count
            ("String" | "string", "count") => {
                let sub = self.pop();
                let val = self.pop();
                match (&val, &sub) {
                    (Value::String(s), Value::String(needle)) => {
                        if needle.is_empty() {
                            self.push(Value::Int(0));
                        } else {
                            self.push(Value::Int(s.matches(needle.as_str()).count() as i32));
                        }
                    }
                    _ => return Err(format!(
                        "String.count: expected (String, String), got ({:?}, {:?})",
                        val, sub
                    )),
                }
            }
            // Integer::parse
            ("Integer" | "int", "parse") => {
                let val = self.pop();
                match &val {
                    Value::String(s) => match s.trim().parse::<i32>() {
                        Ok(n) => self.push(Value::Int(n)),
                        Err(_) => {
                            return Err(format!("Invalid integer: {}", s))
                        }
                    },
                    _ => return Err(format!("Integer.parse: expected String, got {:?}", val)),
                }
            }
            // Integer::MAX_VALUE / Integer::MIN_VALUE
            ("Integer" | "int", "MAX_VALUE") => {
                let arg_start = self.stack.len() - arg_count as usize;
                let _args: Vec<Value> = self.stack.drain(arg_start..).collect();
                self.push(Value::Int(i32::MAX));
            }
            ("Integer" | "int", "MIN_VALUE") => {
                let arg_start = self.stack.len() - arg_count as usize;
                let _args: Vec<Value> = self.stack.drain(arg_start..).collect();
                self.push(Value::Int(i32::MIN));
            }
            // Long::MAX_VALUE / Long::MIN_VALUE
            ("Long" | "long", "MAX_VALUE") => {
                let arg_start = self.stack.len() - arg_count as usize;
                let _args: Vec<Value> = self.stack.drain(arg_start..).collect();
                self.push(Value::Long(i64::MAX));
            }
            ("Long" | "long", "MIN_VALUE") => {
                let arg_start = self.stack.len() - arg_count as usize;
                let _args: Vec<Value> = self.stack.drain(arg_start..).collect();
                self.push(Value::Long(i64::MIN));
            }
            // Double::parseOr
            ("Double" | "double", "parseOr") => {
                let default_val = self.pop();
                let val = self.pop();
                let default = match &default_val {
                    Value::Double(d) => *d,
                    Value::Float(d) => *d as f64,
                    Value::Int(i) => *i as f64,
                    Value::Long(i) => *i as f64,
                    _ => 0.0,
                };
                match &val {
                    Value::String(s) => match s.trim().parse::<f64>() {
                        Ok(n) => self.push(Value::Double(n)),
                        Err(_) => self.push(Value::Double(default)),
                    },
                    _ => self.push(Value::Double(default)),
                }
            }
            // Boolean::logicalAnd
            ("Boolean" | "bool", "logicalAnd") => {
                let b = self.pop();
                let a = self.pop();
                let av = match a { Value::Bool(v) => v, _ => false };
                let bv = match b { Value::Bool(v) => v, _ => false };
                self.push(Value::Bool(av && bv));
            }
            // Boolean::logicalOr
            ("Boolean" | "bool", "logicalOr") => {
                let b = self.pop();
                let a = self.pop();
                let av = match a { Value::Bool(v) => v, _ => false };
                let bv = match b { Value::Bool(v) => v, _ => false };
                self.push(Value::Bool(av || bv));
            }
            // Random::nextInt (0 args => any int, 1 arg => 0..bound)
            ("Random", "nextInt") => {
                let bound = if arg_count > 0 {
                    let v = self.pop();
                    v.to_i64().unwrap_or(i64::MAX)
                } else {
                    i64::MAX
                };
                let n = if bound == i64::MAX {
                    rand::random::<i64>()
                } else if bound > 0 {
                    (rand::random::<u64>() % bound as u64) as i64
                } else {
                    0
                };
                self.push(Value::Int(n as i32));
            }
            // Random::nextDouble
            ("Random", "nextDouble") => {
                let arg_start = self.stack.len() - arg_count as usize;
                let _args: Vec<Value> = self.stack.drain(arg_start..).collect();
                self.push(Value::Double(rand::random::<f64>()));
            }
            // Random::nextLong is handled by the default arm below, which looks
            // up the registered Random_nextLong native. The native supports
            // both 0-arg (returns a random Long) and 2-arg (state0, state1)
            // calls - the latter returns [new_s0, new_s1, result] for
            // Xorshift128+ state advancement used by lib/tt/random/Random.tr.
            // Random::nextBoolean
            ("Random", "nextBoolean") => {
                let arg_start = self.stack.len() - arg_count as usize;
                let _args: Vec<Value> = self.stack.drain(arg_start..).collect();
                self.push(Value::Bool(rand::random::<bool>()));
            }
            // Random::nextFloat
            ("Random", "nextFloat") => {
                let arg_start = self.stack.len() - arg_count as usize;
                let _args: Vec<Value> = self.stack.drain(arg_start..).collect();
                self.push(Value::Float(rand::random::<f32>()));
            }
            // Subprocess::runFull — delegate to the real native so shell
            // builtins (echo, dir) and full result capture work.
            ("Subprocess", "runFull") => {
                let arg_start = self.stack.len() - arg_count as usize;
                let args: Vec<Value> = self.stack.drain(arg_start..).collect();
                let v = super::natives::subprocess::native_subprocess_run_full(&args)?;
                self.push(v);
            }
            // StructExt::pack(format, values: ArrayList<Variant>) -> string (packed bytes)
            ("StructExt", "pack") => {
                let arg_start = self.stack.len() - arg_count as usize;
                let args: Vec<Value> = self.stack.drain(arg_start..).collect();
                if args.len() < 2 {
                    return Err("StructExt.pack: expected (format, values)".to_string());
                }
                let format = match &args[0] {
                    Value::String(s) => s.as_str().to_string(),
                    _ => return Err("StructExt.pack: format must be String".to_string()),
                };
                let values: Vec<Value> = match &args[1] {
                    Value::ClassInstance { fields, .. } => match fields.borrow().get("_elements") {
                        Some(Value::Array { elements }) => elements.clone(),
                        _ => vec![],
                    },
                    Value::Array { elements } => elements.clone(),
                    _ => return Err("StructExt.pack: values must be ArrayList".to_string()),
                };
                let (little_endian, fields) = parse_struct_format(&format);
                let mut bytes: Vec<u8> = Vec::new();
                let mut vi = 0usize;
                for (fc, count) in &fields {
                    for _ in 0..*count {
                        if vi >= values.len() { break; }
                        let v = &values[vi];
                        vi += 1;
                        match *fc {
                            'b' | 'B' | '?' => {
                                let n = value_to_i64(v).unwrap_or(0) as i8;
                                bytes.push(n as u8);
                            }
                            'h' | 'H' => {
                                let n = value_to_i64(v).unwrap_or(0) as i16;
                                if little_endian {
                                    bytes.extend_from_slice(&n.to_le_bytes());
                                } else {
                                    bytes.extend_from_slice(&n.to_be_bytes());
                                }
                            }
                            'i' | 'I' | 'l' | 'L' => {
                                let n = value_to_i64(v).unwrap_or(0) as i32;
                                if little_endian {
                                    bytes.extend_from_slice(&n.to_le_bytes());
                                } else {
                                    bytes.extend_from_slice(&n.to_be_bytes());
                                }
                            }
                            'q' | 'Q' => {
                                let n = value_to_i64(v).unwrap_or(0);
                                if little_endian {
                                    bytes.extend_from_slice(&n.to_le_bytes());
                                } else {
                                    bytes.extend_from_slice(&n.to_be_bytes());
                                }
                            }
                            'f' => {
                                let f = value_to_f64(v).unwrap_or(0.0) as f32;
                                if little_endian {
                                    bytes.extend_from_slice(&f.to_le_bytes());
                                } else {
                                    bytes.extend_from_slice(&f.to_be_bytes());
                                }
                            }
                            'd' => {
                                let d = value_to_f64(v).unwrap_or(0.0);
                                if little_endian {
                                    bytes.extend_from_slice(&d.to_le_bytes());
                                } else {
                                    bytes.extend_from_slice(&d.to_be_bytes());
                                }
                            }
                            's' | 'p' => {
                                if let Value::String(s) = v {
                                    bytes.extend_from_slice(s.as_bytes());
                                }
                            }
                            _ => {}
                        }
                    }
                }
                self.push(Value::String(Rc::new(String::from_utf8_lossy(&bytes).to_string())));
            }
            // StructExt::unpack(format, data: string) -> ArrayList<Variant>
            ("StructExt", "unpack") => {
                let arg_start = self.stack.len() - arg_count as usize;
                let args: Vec<Value> = self.stack.drain(arg_start..).collect();
                if args.len() < 2 {
                    return Err("StructExt.unpack: expected (format, data)".to_string());
                }
                let format = match &args[0] {
                    Value::String(s) => s.as_str().to_string(),
                    _ => return Err("StructExt.unpack: format must be String".to_string()),
                };
                let data: Vec<u8> = match &args[1] {
                    Value::String(s) => s.as_bytes().to_vec(),
                    Value::Array { elements } => elements.iter().filter_map(|e| match e {
                        Value::Byte(b) => Some(*b as u8),
                        Value::Int(i) => Some(*i as u8),
                        _ => None,
                    }).collect(),
                    _ => return Err("StructExt.unpack: data must be String".to_string()),
                };
                let (little_endian, fields) = parse_struct_format(&format);
                let mut result: Vec<Value> = Vec::new();
                let mut pos = 0usize;
                for (fc, count) in &fields {
                    for _ in 0..*count {
                        match *fc {
                            'b' | 'B' | '?' => {
                                if pos < data.len() {
                                    let n = data[pos] as i8;
                                    result.push(Value::Int(n as i32));
                                    pos += 1;
                                }
                            }
                            'h' | 'H' => {
                                if pos + 2 <= data.len() {
                                    let n = if little_endian {
                                        i16::from_le_bytes([data[pos], data[pos+1]])
                                    } else {
                                        i16::from_be_bytes([data[pos], data[pos+1]])
                                    };
                                    result.push(Value::Int(n as i32));
                                    pos += 2;
                                }
                            }
                            'i' | 'I' | 'l' | 'L' => {
                                if pos + 4 <= data.len() {
                                    let n = if little_endian {
                                        i32::from_le_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]])
                                    } else {
                                        i32::from_be_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]])
                                    };
                                    result.push(Value::Int(n));
                                    pos += 4;
                                }
                            }
                            'q' | 'Q' => {
                                if pos + 8 <= data.len() {
                                    let mut arr = [0u8; 8];
                                    arr.copy_from_slice(&data[pos..pos+8]);
                                    let n = if little_endian {
                                        i64::from_le_bytes(arr)
                                    } else {
                                        i64::from_be_bytes(arr)
                                    };
                                    result.push(Value::Long(n));
                                    pos += 8;
                                }
                            }
                            'f' => {
                                if pos + 4 <= data.len() {
                                    let n = if little_endian {
                                        f32::from_le_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]])
                                    } else {
                                        f32::from_be_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]])
                                    };
                                    result.push(Value::Double(n as f64));
                                    pos += 4;
                                }
                            }
                            'd' => {
                                if pos + 8 <= data.len() {
                                    let mut arr = [0u8; 8];
                                    arr.copy_from_slice(&data[pos..pos+8]);
                                    let n = if little_endian {
                                        f64::from_le_bytes(arr)
                                    } else {
                                        f64::from_be_bytes(arr)
                                    };
                                    result.push(Value::Double(n));
                                    pos += 8;
                                }
                            }
                            's' | 'p' if pos < data.len() => {
                                result.push(Value::String(Rc::new((data[pos] as char).to_string())));
                                pos += 1;
                            }
                            _ => {}
                        }
                    }
                }
                let mut al_fields = HashMap::new();
                al_fields.insert("_elements".to_string(), Value::Array { elements: result });
                self.push(Value::ClassInstance {
                    class_name: "ArrayList".to_string(),
                    fields: Rc::new(std::cell::RefCell::new(al_fields)),
                    vtable: HashMap::new(),
                });
            }
            // StructExt::packInto(buffer, offset, packed: string) -> void
            ("StructExt", "packInto") => {
                let arg_start = self.stack.len() - arg_count as usize;
                let args: Vec<Value> = self.stack.drain(arg_start..).collect();
                if args.len() < 3 {
                    return Err("StructExt.packInto: expected (buffer, offset, packed)".to_string());
                }
                let offset = match &args[1] {
                    Value::Int(i) => *i as usize,
                    Value::Long(i) => *i as usize,
                    _ => return Err("StructExt.packInto: offset must be int".to_string()),
                };
                let packed: Vec<u8> = match &args[2] {
                    Value::String(s) => s.as_bytes().to_vec(),
                    _ => return Err("StructExt.packInto: packed must be String".to_string()),
                };
                match &args[0] {
                    Value::String(s) => {
                        let mut chars: Vec<char> = s.chars().collect();
                        for (i, b) in packed.iter().enumerate() {
                            let pos = offset + i;
                            if pos < chars.len() {
                                chars[pos] = *b as char;
                            } else {
                                chars.push(*b as char);
                            }
                        }
                        let new_s: String = chars.into_iter().collect();
                        // No way to mutate in place; push new string (callers ignore return).
                        self.push(Value::String(Rc::new(new_s)));
                    }
                    Value::ClassInstance { fields, .. } => {
                        let mut elements = match fields.borrow().get("_elements") {
                            Some(Value::Array { elements }) => elements.clone(),
                            _ => vec![],
                        };
                        for (i, b) in packed.iter().enumerate() {
                            let pos = offset + i;
                            if pos < elements.len() {
                                elements[pos] = Value::Int(*b as i32);
                            } else {
                                elements.push(Value::Int(*b as i32));
                            }
                        }
                        fields.borrow_mut().insert("_elements".to_string(), Value::Array { elements });
                        self.push(Value::Void);
                    }
                    _ => return Err("StructExt.packInto: buffer must be String or ArrayList".to_string()),
                }
            }
            // StructExt::unpackFrom(format, buffer: string, offset: int) -> ArrayList<Variant>
            ("StructExt", "unpackFrom") => {
                let arg_start = self.stack.len() - arg_count as usize;
                let args: Vec<Value> = self.stack.drain(arg_start..).collect();
                if args.len() < 3 {
                    return Err("StructExt.unpackFrom: expected (format, buffer, offset)".to_string());
                }
                let format = match &args[0] {
                    Value::String(s) => s.as_str().to_string(),
                    _ => return Err("StructExt.unpackFrom: format must be String".to_string()),
                };
                let data: Vec<u8> = match &args[1] {
                    Value::String(s) => s.as_bytes().to_vec(),
                    _ => return Err("StructExt.unpackFrom: buffer must be String".to_string()),
                };
                let offset = match &args[2] {
                    Value::Int(i) => *i as usize,
                    Value::Long(i) => *i as usize,
                    _ => return Err("StructExt.unpackFrom: offset must be int".to_string()),
                };
                let (little_endian, fields) = parse_struct_format(&format);
                let mut result: Vec<Value> = Vec::new();
                let mut pos = offset;
                for (fc, count) in &fields {
                    for _ in 0..*count {
                        match *fc {
                            'b' | 'B' | '?' => {
                                if pos < data.len() {
                                    let n = data[pos] as i8;
                                    result.push(Value::Int(n as i32));
                                    pos += 1;
                                }
                            }
                            'h' | 'H' => {
                                if pos + 2 <= data.len() {
                                    let n = if little_endian {
                                        i16::from_le_bytes([data[pos], data[pos+1]])
                                    } else {
                                        i16::from_be_bytes([data[pos], data[pos+1]])
                                    };
                                    result.push(Value::Int(n as i32));
                                    pos += 2;
                                }
                            }
                            'i' | 'I' | 'l' | 'L' => {
                                if pos + 4 <= data.len() {
                                    let n = if little_endian {
                                        i32::from_le_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]])
                                    } else {
                                        i32::from_be_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]])
                                    };
                                    result.push(Value::Int(n));
                                    pos += 4;
                                }
                            }
                            'q' | 'Q' => {
                                if pos + 8 <= data.len() {
                                    let mut arr = [0u8; 8];
                                    arr.copy_from_slice(&data[pos..pos+8]);
                                    let n = if little_endian {
                                        i64::from_le_bytes(arr)
                                    } else {
                                        i64::from_be_bytes(arr)
                                    };
                                    result.push(Value::Long(n));
                                    pos += 8;
                                }
                            }
                            'f' => {
                                if pos + 4 <= data.len() {
                                    let n = if little_endian {
                                        f32::from_le_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]])
                                    } else {
                                        f32::from_be_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]])
                                    };
                                    result.push(Value::Double(n as f64));
                                    pos += 4;
                                }
                            }
                            'd' => {
                                if pos + 8 <= data.len() {
                                    let mut arr = [0u8; 8];
                                    arr.copy_from_slice(&data[pos..pos+8]);
                                    let n = if little_endian {
                                        f64::from_le_bytes(arr)
                                    } else {
                                        f64::from_be_bytes(arr)
                                    };
                                    result.push(Value::Double(n));
                                    pos += 8;
                                }
                            }
                            's' | 'p' if pos < data.len() => {
                                result.push(Value::String(Rc::new((data[pos] as char).to_string())));
                                pos += 1;
                            }
                            _ => {}
                        }
                    }
                }
                let mut al_fields = HashMap::new();
                al_fields.insert("_elements".to_string(), Value::Array { elements: result });
                self.push(Value::ClassInstance {
                    class_name: "ArrayList".to_string(),
                    fields: Rc::new(std::cell::RefCell::new(al_fields)),
                    vtable: HashMap::new(),
                });
            }
            ("Tar", "build") => {
                let arg_start = self.stack.len() - arg_count as usize;
                let args: Vec<Value> = self.stack.drain(arg_start..).collect();
                let serialized = args.first().and_then(|v| if let Value::String(s) = v { Some(s.as_str()) } else { None }).unwrap_or("");
                let data = tar_build_archive(serialized)?;
                self.push(Value::String(Rc::new(data)));
            }
            ("Tar", "parse") => {
                let arg_start = self.stack.len() - arg_count as usize;
                let args: Vec<Value> = self.stack.drain(arg_start..).collect();
                let data = args.first().and_then(|v| if let Value::String(s) = v { Some(s.as_str()) } else { None }).unwrap_or("");
                let serialized = tar_parse_archive(data)?;
                self.push(Value::String(Rc::new(serialized)));
            }
            // Hmac::compute
            ("Hmac", "compute") => {
                let arg_start = self.stack.len() - arg_count as usize;
                let args: Vec<Value> = self.stack.drain(arg_start..).collect();
                let key = args.first().and_then(|v| if let Value::String(s) = v { Some(s.as_str()) } else { None }).unwrap_or("");
                let msg = args.get(1).and_then(|v| if let Value::String(s) = v { Some(s.as_str()) } else { None }).unwrap_or("");
                let algo = args.get(2).and_then(|v| if let Value::String(s) = v { Some(s.as_str()) } else { None }).unwrap_or("sha256");
                let digest = simple_hmac(key.as_bytes(), msg.as_bytes(), algo);
                self.push(Value::String(Rc::new(digest)));
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}
