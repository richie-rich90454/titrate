// Titrate Alpha 0.2 – bytecode virtual machine: object instantiation
// Precision in every step – richie-rich90454, 2026

use super::super::frame::{Frame, FunctionDef};
use super::super::chunk::Chunk;
use super::super::value::Value;
use super::natives::lookup_builtin_native;
use super::Vm;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

impl Vm {
    pub(super) fn exec_new(&mut self, class_idx: u16, arg_count: u8) -> Result<(), String> {
        let ci = class_idx as usize;
        if ci >= self.classes.len() {
            return Err(format!("NEW: class index {} out of range", class_idx));
        }

        // Clone all needed data upfront to avoid borrow conflicts
        let class_name = self.classes[ci].name.clone();

        // Handle built-in pseudo-classes
        match class_name.as_str() {
            n if n.starts_with("ArrayList") => {
                let mut fields = HashMap::new();
                fields.insert("_elements".to_string(), Value::Array { elements: vec![] });
                let instance = Value::ClassInstance {
                    class_name: class_name.clone(),
                    fields: Rc::new(std::cell::RefCell::new(fields)),
                    vtable: HashMap::new(),
                };
                self.push(instance);
                return Ok(());
            }
            n if n.starts_with("HashMap") => {
                let mut fields = HashMap::new();
                fields.insert("_keys".to_string(), Value::Array { elements: vec![] });
                fields.insert("_values".to_string(), Value::Array { elements: vec![] });
                let instance = Value::ClassInstance {
                    class_name: class_name.clone(),
                    fields: Rc::new(std::cell::RefCell::new(fields)),
                    vtable: HashMap::new(),
                };
                self.push(instance);
                return Ok(());
            }
            _ => {}
        }

        // Find the constructor matching the arg count.
        // The class may have multiple constructors (overloaded by arity).
        // We search the function table for a function named "<class_name>.<init>"
        // with arity matching arg_count.  If none matches, fall back to the
        // class's default constructor field.
        let ctor_pattern = format!("{}.<init>", class_name);
        let constructor = self.functions.iter().enumerate()
            .find(|(_, f)| f.name == ctor_pattern && f.arity == arg_count as usize)
            .map(|(i, _)| i as u16)
            .or(self.classes[ci].constructor);

        let field_inits: Vec<(String, Chunk)> = self.classes[ci].field_inits.clone();
        let field_names: Vec<String> = self.classes[ci].fields.iter().map(|f| f.name.clone()).collect();
        let vtable = self.classes[ci].methods.clone();

        // Build default fields (all Null initially)
        let mut fields = HashMap::new();
        for name in &field_names {
            fields.insert(name.clone(), Value::Null);
        }

        let instance = Value::ClassInstance {
            class_name,
            fields: Rc::new(std::cell::RefCell::new(fields)),
            vtable,
        };

        // Push instance onto the stack
        self.push(instance.clone());

        // Run field initializers
        // Each field_init is a (name, Chunk) pair that computes the initial value.
        // We execute each chunk and set the field.
        for (field_name, init_chunk) in field_inits {
            // Execute the init chunk by creating a temporary function/frame
            let temp_func_idx = self.functions.len() as u16;
            self.functions.push(FunctionDef {
                name: format!("<init_{}>", field_name),
                arity: 0,
                chunk: init_chunk,
                is_method: false,
                is_constructor: false,
                local_count: 0,
            });
            self.frames.push(Frame::new(temp_func_idx, self.stack.len()));
            // Run the init chunk
            while self.frames.last().map_or(false, |f| f.function_index == temp_func_idx) {
                self.step()?;
            }
            // The init chunk should have left a value on the stack
            let init_val = self.pop();
            // Set the field on the instance
            if let Value::ClassInstance { fields, .. } = &instance {
                fields.borrow_mut().insert(field_name, init_val);
            }
            // Remove the temporary function
            self.functions.pop();
        }

        // If class has a constructor, call it
        if let Some(ctor_idx) = constructor {
            // The stack is: [..., arg0, arg1, ..., instance]
            // We need:      [..., instance, arg0, arg1, ...]
            // Pop the instance, then insert it before the arguments.
            let instance_val = self.pop();
            // Use the actual number of args passed (from the NEW opcode)
            // rather than the constructor's arity, since they should match
            // but arg_count is the authoritative source.
            let num_args = arg_count as usize;
            let arg_start = self.stack.len() - num_args;
            self.stack.insert(arg_start, instance_val.clone());
            // Now base points to the instance (which is "this")
            let base = arg_start;
            // Pre-allocate local slots for the constructor
            let local_count = self.functions[ctor_idx as usize].local_count;
            let needed = base + local_count;
            while self.stack.len() < needed {
                self.stack.push(Value::Null);
            }
            self.frames.push(Frame::new(ctor_idx, base));
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Static calls
    // -----------------------------------------------------------------------

    pub(super) fn exec_static_call(
        &mut self,
        class_name_idx: u16,
        method_name_idx: u16,
        arg_count: u8,
    ) -> Result<(), String> {
        let (class_name, method_name) = {
            let frame = self.current_frame();
            let chunk = &self.functions[frame.function_index as usize].chunk;
            (
                chunk.strings[class_name_idx as usize].clone(),
                chunk.strings[method_name_idx as usize].clone(),
            )
        };

        match (class_name.as_str(), method_name.as_str()) {
            // io::println
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
                let _ = self.pop(); // pop any args
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
                let _ = self.pop(); // pop any args
                let mut input = String::new();
                if std::io::Read::read_to_string(&mut std::io::stdin(), &mut input).is_ok() {
                    self.push(Value::String(Rc::new(input)));
                } else {
                    self.push(Value::String(Rc::new(String::new())));
                }
            }
            // io::stderr - switch to stderr mode (no-op in VM, just mark intent)
            ("io", "stderr") => {
                let _ = self.pop(); // pop any args
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
                    Value::String(s) => match s.parse::<i64>() {
                        Ok(n) => self.push(Value::ResultOk(Box::new(Value::Long(n)))),
                        Err(_) => {
                            self.push(Value::ResultErr(Box::new(Value::String(Rc::new(
                                format!("Invalid integer: {}", s),
                            )))))
                        }
                    },
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
                        Ok(n) => self.push(Value::ResultOk(Box::new(Value::Long(n)))),
                        Err(_) => self.push(Value::ResultErr(Box::new(Value::String(Rc::new(
                            format!("Invalid long: {}", s),
                        ))))),
                    },
                    _ => return Err(format!("Long.parse: expected String, got {:?}", val)),
                }
            }
            // Short::parse
            ("Short" | "short", "parse") => {
                let val = self.pop();
                match &val {
                    Value::String(s) => match s.trim().parse::<i16>() {
                        Ok(n) => self.push(Value::ResultOk(Box::new(Value::Short(n)))),
                        Err(_) => self.push(Value::ResultErr(Box::new(Value::String(Rc::new(
                            format!("Invalid short: {}", s),
                        ))))),
                    },
                    _ => return Err(format!("Short.parse: expected String, got {:?}", val)),
                }
            }
            // Byte::parse
            ("Byte" | "byte", "parse") => {
                let val = self.pop();
                match &val {
                    Value::String(s) => match s.trim().parse::<i8>() {
                        Ok(n) => self.push(Value::ResultOk(Box::new(Value::Byte(n)))),
                        Err(_) => self.push(Value::ResultErr(Box::new(Value::String(Rc::new(
                            format!("Invalid byte: {}", s),
                        ))))),
                    },
                    _ => return Err(format!("Byte.parse: expected String, got {:?}", val)),
                }
            }
            // Float::parse
            ("Float" | "float", "parse") => {
                let val = self.pop();
                match &val {
                    Value::String(s) => match s.trim().parse::<f32>() {
                        Ok(n) => self.push(Value::ResultOk(Box::new(Value::Float(n)))),
                        Err(_) => self.push(Value::ResultErr(Box::new(Value::String(Rc::new(
                            format!("Invalid float: {}", s),
                        ))))),
                    },
                    _ => return Err(format!("Float.parse: expected String, got {:?}", val)),
                }
            }
            // Half::parse
            ("Half" | "half", "parse") => {
                let val = self.pop();
                match &val {
                    Value::String(s) => match s.trim().parse::<f32>() {
                        Ok(n) => self.push(Value::ResultOk(Box::new(Value::Half(n)))),
                        Err(_) => self.push(Value::ResultErr(Box::new(Value::String(Rc::new(
                            format!("Invalid half: {}", s),
                        ))))),
                    },
                    _ => return Err(format!("Half.parse: expected String, got {:?}", val)),
                }
            }
            // Quad::parse
            ("Quad" | "quad", "parse") => {
                let val = self.pop();
                match &val {
                    Value::String(s) => match s.trim().parse::<f64>() {
                        Ok(n) => self.push(Value::ResultOk(Box::new(Value::Quad(n)))),
                        Err(_) => self.push(Value::ResultErr(Box::new(Value::String(Rc::new(
                            format!("Invalid quad: {}", s),
                        ))))),
                    },
                    _ => return Err(format!("Quad.parse: expected String, got {:?}", val)),
                }
            }
            // Vast::parse
            ("Vast" | "vast", "parse") => {
                let val = self.pop();
                match &val {
                    Value::String(s) => match s.trim().parse::<i128>() {
                        Ok(n) => self.push(Value::ResultOk(Box::new(Value::Vast(n)))),
                        Err(_) => self.push(Value::ResultErr(Box::new(Value::String(Rc::new(
                            format!("Invalid vast: {}", s),
                        ))))),
                    },
                    _ => return Err(format!("Vast.parse: expected String, got {:?}", val)),
                }
            }
            // Uvast::parse
            ("Uvast" | "uvast", "parse") => {
                let val = self.pop();
                match &val {
                    Value::String(s) => match s.trim().parse::<u128>() {
                        Ok(n) => self.push(Value::ResultOk(Box::new(Value::Uvast(n)))),
                        Err(_) => self.push(Value::ResultErr(Box::new(Value::String(Rc::new(
                            format!("Invalid uvast: {}", s),
                        ))))),
                    },
                    _ => return Err(format!("Uvast.parse: expected String, got {:?}", val)),
                }
            }
            // Boolean::parseBoolean
            ("Boolean" | "bool", "parseBoolean") => {
                let val = self.pop();
                match &val {
                    Value::String(s) => {
                        let b = matches!(s.trim().to_lowercase().as_str(), "true" | "1" | "yes" | "on");
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
                        Ok(n) => self.push(Value::ResultOk(Box::new(Value::Double(n)))),
                        Err(_) => self.push(Value::ResultErr(Box::new(Value::String(Rc::new(
                            format!("Invalid double: {}", s),
                        ))))),
                    },
                    _ => return Err(format!("Double.parseDouble: expected String, got {:?}", val)),
                }
            }
            // Double::NaN, Double::POSITIVE_INFINITY, Double::NEGATIVE_INFINITY
            ("Double" | "double", "NaN") => {
                let _ = self.pop(); // pop any args
                self.push(Value::Double(f64::NAN));
            }
            ("Double" | "double", "POSITIVE_INFINITY") => {
                let _ = self.pop();
                self.push(Value::Double(f64::INFINITY));
            }
            ("Double" | "double", "NEGATIVE_INFINITY") => {
                let _ = self.pop();
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
                    _ => return Err(format!("String.reverse: expected String, got {:?}", val)),
                }
            }
            // System::currentTimeMillis
            ("System", "currentTimeMillis") => {
                let _ = self.pop();
                let millis = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0);
                self.push(Value::Long(millis));
            }
            // System::nanoTime
            ("System", "nanoTime") => {
                let _ = self.pop();
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
                            Ok(content) => self.push(Value::ResultOk(Box::new(Value::String(Rc::new(content))))),
                            Err(e) => self.push(Value::ResultErr(Box::new(Value::String(Rc::new(
                                format!("Failed to read file '{}': {}", p, e),
                            ))))),
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
                            Ok(()) => self.push(Value::ResultOk(Box::new(Value::Void))),
                            Err(e) => self.push(Value::ResultErr(Box::new(Value::String(Rc::new(
                                format!("Failed to write file '{}': {}", p, e),
                            ))))),
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
                        return Err(format!(
                            "String.substring: type mismatch"
                        ))
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
                            Ok(content) => self.push(Value::ResultOk(Box::new(Value::String(Rc::new(content))))),
                            Err(e) => self.push(Value::ResultErr(Box::new(Value::String(Rc::new(
                                format!("Failed to read file: {}", e)
                            ))))),
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
                            Ok(()) => self.push(Value::ResultOk(Box::new(Value::Void))),
                            Err(e) => self.push(Value::ResultErr(Box::new(Value::String(Rc::new(
                                format!("Failed to write file: {}", e)
                            ))))),
                        }
                    }
                    _ => return Err(format!("File.writeFile: expected (String, String)")),
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
                            Err(e) => self.push(Value::ResultErr(Box::new(Value::String(Rc::new(
                                format!("Failed to read file: {}", e)
                            ))))),
                        }
                    }
                    _ => return Err(format!("File.readLines: expected String, got {:?}", val)),
                }
            }
            // File::open - opens a file and returns Result<FileHandle, string>
            ("File", "open") => {
                let mode = if arg_count > 1 {
                    self.pop()
                } else {
                    Value::String(Rc::new("r".to_string()))
                };
                let path = self.pop();
                match (&path, &mode) {
                    (Value::String(p), Value::String(m)) => {
                        let resolved = self.resolve_path(p.as_str());
                        let file = match m.as_str() {
                            "r" | "rb" => std::fs::File::open(&resolved),
                            "w" | "wb" => std::fs::File::create(&resolved),
                            "a" | "ab" => std::fs::OpenOptions::new().append(true).open(&resolved),
                            "r+" => std::fs::OpenOptions::new().read(true).write(true).open(&resolved),
                            "w+" => std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(true).open(&resolved),
                            "a+" => std::fs::OpenOptions::new().read(true).append(true).open(&resolved),
                            _ => {
                                self.push(Value::ResultErr(Box::new(Value::String(Rc::new(
                                    format!("File.open: unsupported mode '{}'", m)
                                )))));
                                return Ok(());
                            }
                        };
                        match file {
                            Ok(f) => self.push(Value::ResultOk(Box::new(Value::FileHandle(
                                Rc::new(RefCell::new(Some(f)))
                            )))),
                            Err(e) => self.push(Value::ResultErr(Box::new(Value::String(Rc::new(
                                format!("Failed to open file '{}': {}", p, e)
                            ))))),
                        }
                    }
                    _ => return Err(format!("File.open: expected (String, String), got ({:?}, {:?})", path, mode)),
                }
            }
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
                    _ => return Err(format!("String.split: expected (String, String) or (String, Char)")),
                }
            }
            // String::indexOf
            ("String" | "string", "indexOf") => {
                let sub = self.pop();
                let val = self.pop();
                match (&val, &sub) {
                    (Value::String(s), Value::String(needle)) => {
                        match s.find(needle.as_str()) {
                            Some(byte_pos) => {
                                let char_pos = s[..byte_pos].chars().count() as i32;
                                self.push(Value::Int(char_pos));
                            }
                            None => self.push(Value::Int(-1)),
                        }
                    }
                    (Value::String(s), Value::Char(needle)) => {
                        match s.find(*needle) {
                            Some(byte_pos) => {
                                let char_pos = s[..byte_pos].chars().count() as i32;
                                self.push(Value::Int(char_pos));
                            }
                            None => self.push(Value::Int(-1)),
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
                    _ => return Err(format!("String.toUpperCase: expected String, got {:?}", val)),
                }
            }
            // String::toLowerCase
            ("String" | "string", "toLowerCase") => {
                let val = self.pop();
                match &val {
                    Value::String(s) => self.push(Value::String(Rc::new(s.to_lowercase()))),
                    _ => return Err(format!("String.toLowerCase: expected String, got {:?}", val)),
                }
            }
            // String::trim
            ("String" | "string", "trim") => {
                let val = self.pop();
                match &val {
                    Value::String(s) => self.push(Value::String(Rc::new(s.trim().to_string()))),
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
            // Default: look up user-defined static method in class table
            _ => {
                // Try to find the class and its static method
                let class_def = self.classes.iter().find(|c| c.name == class_name);
                if let Some(cd) = class_def {
                    if let Some(&func_idx) = cd.methods.get(&method_name) {
                        let base = self.stack.len() - arg_count as usize;
                        self.frames.push(Frame::new(func_idx, base));
                        return Ok(());
                    }
                }

                // Fallback: try native function lookup (ClassName_method)
                let native_name = format!("{}_{}", class_name, method_name);
                if let Some(&native_idx) = self.native_names.get(&native_name) {
                    let arg_start = self.stack.len() - arg_count as usize;
                    let args: Vec<Value> = self.stack.drain(arg_start..).collect();
                    let func = self.natives[native_idx as usize];
                    let result = func(&args)?;
                    self.push(result);
                    return Ok(());
                }

                // Also try lookup_builtin_native in case it hasn't been registered yet
                if let Some(func) = lookup_builtin_native(&native_name) {
                    let arg_start = self.stack.len() - arg_count as usize;
                    let args: Vec<Value> = self.stack.drain(arg_start..).collect();
                    let result = func(&args)?;
                    self.push(result);
                    return Ok(());
                }

                return Err(format!(
                    "Unknown static call: {}.{}",
                    class_name, method_name
                ));
            }
        }

        Ok(())
    }
}
