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
                param_types: Vec::new(),
            });
            self.frames.push(Frame::new(temp_func_idx, self.stack.len()));
            // Run the init chunk
            while self.frames.last().is_some_and(|f| f.function_index == temp_func_idx) {
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
            // Subprocess::runFull
            ("Subprocess", "runFull") => {
                let arg_start = self.stack.len() - arg_count as usize;
                let args: Vec<Value> = self.stack.drain(arg_start..).collect();
                let mut cmd_parts: Vec<String> = vec![];
                for a in &args {
                    cmd_parts.push(a.display_string());
                }
                if cmd_parts.is_empty() {
                    self.push(Value::Null);
                } else {
                    let program = cmd_parts[0].clone();
                    let cmd_args = &cmd_parts[1..];
                    let output = std::process::Command::new(&program)
                        .args(cmd_args)
                        .output();
                    match output {
                        Ok(out) => {
                            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                            let code = out.status.code().unwrap_or(-1);
                            let mut fields = HashMap::new();
                            fields.insert("stdout".to_string(), Value::String(Rc::new(stdout)));
                            fields.insert("stderr".to_string(), Value::String(Rc::new(stderr)));
                            fields.insert("exitCode".to_string(), Value::Int(code));
                            self.push(Value::ClassInstance {
                                class_name: "ProcessResult".to_string(),
                                fields: Rc::new(RefCell::new(fields)),
                                vtable: HashMap::new(),
                            });
                        }
                        Err(_) => self.push(Value::Null),
                    }
                }
            }
            // StructExt::pack
            ("StructExt", "pack") => {
                let arg_start = self.stack.len() - arg_count as usize;
                let args: Vec<Value> = self.stack.drain(arg_start..).collect();
                let mut bytes: Vec<u8> = vec![];
                for a in &args {
                    match a {
                        Value::Byte(b) => bytes.push(*b as u8),
                        Value::Int(i) => bytes.extend_from_slice(&i.to_be_bytes()),
                        Value::Long(i) => bytes.extend_from_slice(&i.to_be_bytes()),
                        Value::Short(s) => bytes.extend_from_slice(&s.to_be_bytes()),
                        Value::Double(d) => bytes.extend_from_slice(&d.to_be_bytes()),
                        Value::Float(f) => bytes.extend_from_slice(&f.to_be_bytes()),
                        Value::String(s) => bytes.extend_from_slice(s.as_bytes()),
                        Value::Bool(b) => bytes.push(if *b { 1 } else { 0 }),
                        _ => {}
                    }
                }
                self.push(Value::Array { elements: bytes.iter().map(|b| Value::Byte(*b as i8)).collect() });
            }
            // Lz4::compress / Zstd::compress / Tar::build (stub: return input as bytes)
            ("Lz4", "compress") => {
                let val = self.pop();
                let bytes = match &val {
                    Value::String(s) => s.as_bytes().to_vec(),
                    Value::Array { elements } => elements.iter().filter_map(|e| {
                        match e {
                            Value::Byte(b) => Some(*b as u8),
                            Value::Int(i) => Some(*i as u8),
                            _ => None,
                        }
                    }).collect(),
                    _ => vec![],
                };
                self.push(Value::Array { elements: bytes.iter().map(|b| Value::Byte(*b as i8)).collect() });
            }
            ("Lz4", "decompress") => {
                let val = self.pop();
                let bytes = match &val {
                    Value::Array { elements } => elements.iter().filter_map(|e| {
                        match e {
                            Value::Byte(b) => Some(*b as u8),
                            Value::Int(i) => Some(*i as u8),
                            _ => None,
                        }
                    }).collect(),
                    _ => vec![],
                };
                self.push(Value::String(Rc::new(String::from_utf8_lossy(&bytes).to_string())));
            }
            ("Zstd", "compress") => {
                let val = self.pop();
                let bytes = match &val {
                    Value::String(s) => s.as_bytes().to_vec(),
                    Value::Array { elements } => elements.iter().filter_map(|e| {
                        match e {
                            Value::Byte(b) => Some(*b as u8),
                            Value::Int(i) => Some(*i as u8),
                            _ => None,
                        }
                    }).collect(),
                    _ => vec![],
                };
                self.push(Value::Array { elements: bytes.iter().map(|b| Value::Byte(*b as i8)).collect() });
            }
            ("Zstd", "decompress") => {
                let val = self.pop();
                let bytes = match &val {
                    Value::Array { elements } => elements.iter().filter_map(|e| {
                        match e {
                            Value::Byte(b) => Some(*b as u8),
                            Value::Int(i) => Some(*i as u8),
                            _ => None,
                        }
                    }).collect(),
                    _ => vec![],
                };
                self.push(Value::String(Rc::new(String::from_utf8_lossy(&bytes).to_string())));
            }
            ("Tar", "build") => {
                let arg_start = self.stack.len() - arg_count as usize;
                let args: Vec<Value> = self.stack.drain(arg_start..).collect();
                let mut bytes: Vec<u8> = vec![];
                for a in &args {
                    if let Value::String(s) = a {
                        bytes.extend_from_slice(s.as_bytes());
                    }
                }
                self.push(Value::Array { elements: bytes.iter().map(|b| Value::Byte(*b as i8)).collect() });
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
            // Default: look up user-defined static method in class table
            _ => {
                // Try native function lookup (ClassName_method) FIRST.
                // The compiler converts calls like Hash_md5(x) into STATIC_CALL
                // with class_name="Hash", method_name="md5".  If a native
                // function named "Hash_md5" exists, it should be preferred over
                // a class method to avoid infinite recursion (the class method
                // itself calls Hash_md5, which would loop).
                let native_name = format!("{}_{}", class_name, method_name);
                if let Some(&native_idx) = self.native_names.get(&native_name) {
                    let arg_start = self.stack.len() - arg_count as usize;
                    let args: Vec<Value> = self.stack.drain(arg_start..).collect();
                    let func = self.natives[native_idx as usize];
                    let result = func(&args)?;

                    // Special handling for Thread_spawn: when invoked via
                    // STATIC_CALL ("Thread", "spawn"), the native_thread_spawn
                    // function only spawns a no-op OS thread. Because Value
                    // contains Rc<> (not Send-safe), the closure argument must
                    // be executed synchronously on the calling thread. The
                    // call_native_fn path already does this; mirror the same
                    // behaviour here so STATIC_CALL Thread.spawn also runs the
                    // task closure before returning.
                    if native_idx == self.thread_spawn_idx {
                        if let Some(closure) = args.first() {
                            if matches!(closure, Value::Closure { .. }) {
                                let _ = self.call_closure_with_args(closure, &[]);
                                let _ = self.pop();
                            }
                        }
                    }

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

                // Prefer module-level functions over instance methods when both
                // exist.  A STATIC_CALL like Shlex.split("hello world") should
                // resolve to the top-level `split` function in the Shlex module,
                // not the instance method `split` on the Shlex class (which
                // would require a receiver).  Search the function table by
                // mangled name first.
                // Module-level functions like String.toString are compiled with
                // mangled names such as "tt.lang.String.toString".  When the
                // compiler emits a STATIC_CALL with class_name="String" and
                // method_name="toString", we search for a function whose name
                // ends with ".String.toString".  When class_name is empty
                // (bare calls like quickSort()), search by method_name suffix.
                let suffix = if class_name.is_empty() {
                    format!(".{}", method_name)
                } else {
                    format!(".{}.{}", class_name, method_name)
                };
                let exact = format!("tt.lang.{}.{}", class_name, method_name);
                let mut found_idx: Option<u16> = None;
                // Prefer exact tt.lang match first (non-method functions only,
                // since instance methods share the same mangled name pattern
                // but require a `this` receiver). When multiple overloads share
                // the same mangled name, disambiguate by arity (and, if
                // available, by parameter type names) so the correct body is
                // invoked for the given call site.
                for (i, f) in self.functions.iter().enumerate() {
                    if f.name == exact && !f.is_method && f.arity == arg_count as usize {
                        found_idx = Some(i as u16);
                        break;
                    }
                }
                // Exact-name fallback ignoring arity (legacy path) — only used
                // when no arity-matched overload exists.
                if found_idx.is_none() {
                    for (i, f) in self.functions.iter().enumerate() {
                        if f.name == exact && !f.is_method {
                            found_idx = Some(i as u16);
                            break;
                        }
                    }
                }
                // Then fall back to suffix matching (again, non-method only).
                // Apply the same arity-based disambiguation.
                if found_idx.is_none() {
                    for (i, f) in self.functions.iter().enumerate() {
                        if f.name.ends_with(&suffix) && !f.is_method && f.arity == arg_count as usize {
                            found_idx = Some(i as u16);
                            break;
                        }
                    }
                }
                if found_idx.is_none() {
                    for (i, f) in self.functions.iter().enumerate() {
                        if f.name.ends_with(&suffix) && !f.is_method {
                            found_idx = Some(i as u16);
                            break;
                        }
                    }
                }
                if let Some(func_idx) = found_idx {
                    let base = self.stack.len() - arg_count as usize;
                    self.frames.push(Frame::new(func_idx, base));
                    return Ok(());
                }

                // No module-level function found.  Try to find the class and
                // its method as a fallback.  This allows calling instance
                // methods statically (with a null `this` placeholder) for
                // methods that don't actually use `this` (e.g., Hash.md5
                // which just delegates to a native function).
                // Class names in the VM are mangled (e.g., "tt.crypto.Hash"),
                // but STATIC_CALL operands use the simple name (e.g., "Hash").
                // Try exact match first, then suffix match.
                let class_def_idx = self.classes.iter().position(|c| c.name == class_name)
                    .or_else(|| {
                        let suffix = format!(".{}", class_name);
                        self.classes.iter().position(|c| c.name.ends_with(&suffix))
                    });
                if let Some(ci) = class_def_idx {
                    let cd = &self.classes[ci];
                    if let Some(indices) = cd.methods.get(&method_name) {
                        // Pick the overload whose arity matches the call site.
                        // If none matches, fall back to the first overload.
                        let func_idx = indices.iter().copied().find(|&idx| {
                            self.functions[idx as usize].arity == arg_count as usize
                        }).or_else(|| indices.first().copied());
                        if let Some(func_idx) = func_idx {
                            // Instance methods expect `this` in slot 0. Static calls
                            // don't have a receiver, so push a null placeholder before
                            // the arguments to serve as `this`. This allows calling
                            // instance methods that don't use `this` (e.g., Hash.md5
                            // which just delegates to a native function).
                            let arg_start = self.stack.len() - arg_count as usize;
                            self.stack.insert(arg_start, Value::Null);
                            let base = arg_start;
                            let frame = Frame::new(func_idx, base);
                            let local_count = self.functions[func_idx as usize].local_count;
                            let needed = base + local_count;
                            while self.stack.len() < needed {
                                self.stack.push(Value::Null);
                            }
                            self.frames.push(frame);
                            return Ok(());
                        }
                    }
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

// -----------------------------------------------------------------------
// Free helper functions for static calls
// -----------------------------------------------------------------------

/// Simple HMAC implementation using sha2.
/// Returns hex-encoded digest.
fn simple_hmac(key: &[u8], msg: &[u8], algo: &str) -> String {
    use sha2::{Sha256, Sha512, Digest};
    match algo {
        "sha512" => {
            let block_size = 128usize;
            let k = if key.len() > block_size {
                let mut h = Sha512::new();
                h.update(key);
                h.finalize().to_vec()
            } else {
                key.to_vec()
            };
            let mut k_padded = vec![0u8; block_size];
            k_padded[..k.len()].copy_from_slice(&k);
            let mut ipad = vec![0x36u8; block_size];
            let mut opad = vec![0x5cu8; block_size];
            for i in 0..block_size {
                ipad[i] ^= k_padded[i];
                opad[i] ^= k_padded[i];
            }
            let mut inner = Sha512::new();
            inner.update(&ipad);
            inner.update(msg);
            let inner_hash = inner.finalize();
            let mut outer = Sha512::new();
            outer.update(&opad);
            outer.update(inner_hash);
            let result = outer.finalize();
            hex_encode(&result)
        }
        _ => {
            let block_size = 64usize;
            let k = if key.len() > block_size {
                let mut h = Sha256::new();
                h.update(key);
                h.finalize().to_vec()
            } else {
                key.to_vec()
            };
            let mut k_padded = vec![0u8; block_size];
            k_padded[..k.len()].copy_from_slice(&k);
            let mut ipad = vec![0x36u8; block_size];
            let mut opad = vec![0x5cu8; block_size];
            for i in 0..block_size {
                ipad[i] ^= k_padded[i];
                opad[i] ^= k_padded[i];
            }
            let mut inner = Sha256::new();
            inner.update(&ipad);
            inner.update(msg);
            let inner_hash = inner.finalize();
            let mut outer = Sha256::new();
            outer.update(&opad);
            outer.update(inner_hash);
            let result = outer.finalize();
            hex_encode(&result)
        }
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
