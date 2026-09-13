// Method invocation and overload dispatch


use super::super::frame::Frame;
use super::super::value::{Value, values_eq};
use super::Vm;
use std::collections::HashMap;
use std::io::Write;
use std::rc::Rc;

impl Vm {
    pub(super) fn invoke_method(&mut self, method_name_idx: u16, arg_count: u8) -> Result<(), String> {
        // Stack: [receiver, arg0, arg1, ...]
        // The receiver is already on the stack before the args.
        // Total items on stack for this call: 1 (receiver) + arg_count
        let method_name = {
            let frame = self.current_frame();
            let chunk = &self.functions[frame.function_index as usize].chunk;
            chunk.strings[method_name_idx as usize].clone()
        };

        // The receiver is at stack.len() - 1 - arg_count
        let receiver_idx = self.stack.len() - 1 - arg_count as usize;
        let receiver_raw = self.stack[receiver_idx].clone();

        // Handle Result methods BEFORE auto-unwrap, otherwise isOk/isErr
        // would be dispatched on the inner value and fail.
        if let Value::ResultOk(inner) = &receiver_raw {
            match method_name.as_str() {
                "unwrap" => {
                    self.stack.drain(receiver_idx..);
                    self.push((**inner).clone());
                    return Ok(());
                }
                "unwrapErr" => {
                    return Err(format!(
                        "called unwrapErr on an Ok value: {}",
                        inner.display_string()
                    ));
                }
                "isOk" => {
                    self.stack.drain(receiver_idx..);
                    self.push(Value::Bool(true));
                    return Ok(());
                }
                "isErr" => {
                    self.stack.drain(receiver_idx..);
                    self.push(Value::Bool(false));
                    return Ok(());
                }
                _ => {}
            }
        }
        if let Value::ResultErr(err_val) = &receiver_raw {
            match method_name.as_str() {
                "unwrap" => {
                    return Err(format!(
                        "called unwrap on an Err value: {}",
                        err_val.display_string()
                    ));
                }
                "unwrapErr" => {
                    let ev = (**err_val).clone();
                    self.stack.drain(receiver_idx..);
                    self.push(ev);
                    return Ok(());
                }
                "isOk" => {
                    self.stack.drain(receiver_idx..);
                    self.push(Value::Bool(false));
                    return Ok(());
                }
                "isErr" => {
                    self.stack.drain(receiver_idx..);
                    self.push(Value::Bool(true));
                    return Ok(());
                }
                _ => {}
            }
        }

        // Auto-unwrap ResultOk to allow method calls on unwrapped values
        let receiver = match &receiver_raw {
            Value::ResultOk(inner) => (**inner).clone(),
            v => v.clone(),
        };
        self.stack[receiver_idx] = receiver.clone();

        match &receiver {
            Value::ClassInstance {
                vtable, class_name, fields, ..
            } => {
                // Handle built-in ArrayList/HashMap methods
                match class_name.as_str() {
                    n if n.starts_with("ArrayList") => {
                        let result = self.call_arraylist_method(fields, &method_name, arg_count)?;
                        // Pop receiver + args, push result
                        let drain_start = receiver_idx;
                        self.stack.drain(drain_start..);
                        self.push(result);
                        return Ok(());
                    }
                    n if n.starts_with("HashMap") => {
                        let result = self.call_hashmap_method(fields, &method_name, arg_count)?;
                        let drain_start = receiver_idx;
                        self.stack.drain(drain_start..);
                        self.push(result);
                        return Ok(());
                    }
                    n if n.starts_with("Iterator") => {
                        let result = self.call_iterator_method(fields, &method_name, arg_count)?;
                        let drain_start = receiver_idx;
                        self.stack.drain(drain_start..);
                        self.push(result);
                        return Ok(());
                    }
                    _ => {}
                }

                // Look up method in vtable.
                // The vtable maps name → Vec<u16> (one fn_idx per arity) to
                // support overloaded methods. Collect all candidates whose
                // arity matches arg_count, then use type-based overload
                // resolution (pick_overload) to pick the best match. If no
                // arity matches exactly, fall back to the first overload.
                let func_idx = if let Some(indices) = vtable.get(&method_name) {
                    let arity_matches: Vec<u16> = indices.iter().copied()
                        .filter(|&idx| self.functions[idx as usize].arity == arg_count as usize)
                        .collect();
                    if !arity_matches.is_empty() {
                        self.pick_overload(&arity_matches, receiver_idx + 1, arg_count)
                    } else {
                        indices.first().copied()
                    }
                } else {
                    None
                };
                let func_idx = if let Some(idx) = func_idx {
                    idx
                } else {
                    // Walk up the class hierarchy to find the method
                    let mut search_class = class_name.clone();
                    let mut found_idx = None;
                    loop {
                        let class_defs = &self.classes;
                        let found = class_defs.iter().find(|c| c.name == search_class);
                        match found {
                            Some(cd) => {
                                if let Some(indices) = cd.methods.get(&method_name) {
                                    let arity_matches: Vec<u16> = indices.iter().copied()
                                        .filter(|&idx| self.functions[idx as usize].arity == arg_count as usize)
                                        .collect();
                                    let matched = if !arity_matches.is_empty() {
                                        self.pick_overload(&arity_matches, receiver_idx + 1, arg_count)
                                    } else {
                                        indices.first().copied()
                                    };
                                    if let Some(idx) = matched {
                                        found_idx = Some(idx);
                                        break;
                                    }
                                }
                                // Check parent class
                                if let Some(parent_idx) = cd.parent {
                                    search_class = self.classes[parent_idx as usize].name.clone();
                                } else {
                                    break;
                                }
                            }
                            None => break,
                        }
                    }
                    match found_idx {
                        Some(idx) => idx,
                        None => {
                            // Fallback: a class may store a callback as a field
                            // (e.g. obj.handler(args), obj._comparator(a, b)).
                            // If a field with the method name holds a Closure,
                            // replace the receiver on the stack with the closure
                            // and invoke it with the supplied arguments.
                            let fields_rc = fields.clone();
                            if let Some(field_val) = fields_rc.borrow().get(&method_name).cloned() {
                                if matches!(field_val, Value::Closure { .. }) {
                                    self.stack[receiver_idx] = field_val;
                                    return self.call_closure_from_stack(arg_count);
                                }
                            }
                            return Err(format!(
                                "No method '{}' on class '{}'",
                                method_name, class_name
                            ));
                        }
                    }
                };

                let base = receiver_idx;
                // Guard: empty chunk means the method was never compiled.
                if self.functions[func_idx as usize].chunk.code.is_empty() {
                    return Err(format!(
                        "INVOKE_VIRTUAL: method '{}' on class '{}' has no body (empty chunk)",
                        method_name, class_name
                    ));
                }
                self.frames.push(Frame::new(func_idx, base));
                // Pre-allocate stack slots for all local variables.
                // The method has `local_count` total slots, of which
                // 1 (this) + arg_count are already occupied by the receiver
                // and arguments. Fill the rest with Null so the working stack
                // starts past all locals (base + local_count).
                let local_count = self.functions[func_idx as usize].local_count;
                let needed = base + local_count;
                while self.stack.len() < needed {
                    self.stack.push(Value::Null);
                }
            }
            Value::String(s) => {
                // Handle string methods
                match method_name.as_str() {
                    "length" | "size" => {
                        self.stack.drain(receiver_idx..);
                        self.push(Value::Int(s.chars().count() as i32));
                    }
                    "toString" => {
                        self.stack.drain(receiver_idx..);
                        self.push(Value::String(s.clone()));
                    }
                    "trim" => {
                        self.stack.drain(receiver_idx..);
                        self.push(Value::String(Rc::new(s.trim().to_string())));
                    }
                    "split" => {
                        let delimiter = if arg_count > 0 {
                            self.stack.last().cloned().unwrap_or(Value::Void)
                        } else {
                            return Err("String.split requires 1 argument".to_string());
                        };
                        let delim_str = match &delimiter {
                            Value::String(d) => d.as_str().to_string(),
                            Value::Char(c) => c.to_string(),
                            _ => return Err("String.split requires a String or Char delimiter".to_string()),
                        };
                        let parts: Vec<Value> = s.split(&delim_str)
                            .map(|part| Value::String(Rc::new(part.to_string())))
                            .collect();
                        let mut fields = HashMap::new();
                        fields.insert("_elements".to_string(), Value::Array { elements: parts });
                        let result = Value::ClassInstance {
                            class_name: "ArrayList".to_string(),
                            fields: Rc::new(std::cell::RefCell::new(fields)),
                            vtable: HashMap::new(),
                        };
                        self.stack.drain(receiver_idx..);
                        self.push(result);
                    }
                    "isEmpty" => {
                        self.stack.drain(receiver_idx..);
                        self.push(Value::Bool(s.is_empty()));
                    }
                    "contains" => {
                        let substring = if arg_count > 0 {
                            self.stack.last().cloned().unwrap_or(Value::Void)
                        } else {
                            return Err("String.contains requires 1 argument".to_string());
                        };
                        match &substring {
                            Value::String(sub) => {
                                self.stack.drain(receiver_idx..);
                                self.push(Value::Bool(s.contains(sub.as_str())));
                            }
                            _ => return Err("String.contains requires a String argument".to_string()),
                        }
                    }
                    "startsWith" => {
                        let prefix = if arg_count > 0 {
                            self.stack.last().cloned().unwrap_or(Value::Void)
                        } else {
                            return Err("String.startsWith requires 1 argument".to_string());
                        };
                        match &prefix {
                            Value::String(p) => {
                                self.stack.drain(receiver_idx..);
                                self.push(Value::Bool(s.starts_with(p.as_str())));
                            }
                            _ => return Err("String.startsWith requires a String argument".to_string()),
                        }
                    }
                    "endsWith" => {
                        let suffix = if arg_count > 0 {
                            self.stack.last().cloned().unwrap_or(Value::Void)
                        } else {
                            return Err("String.endsWith requires 1 argument".to_string());
                        };
                        match &suffix {
                            Value::String(suf) => {
                                self.stack.drain(receiver_idx..);
                                self.push(Value::Bool(s.ends_with(suf.as_str())));
                            }
                            _ => return Err("String.endsWith requires a String argument".to_string()),
                        }
                    }
                    "substring" => {
                        if arg_count < 2 {
                            return Err("String.substring requires 2 arguments (start, end)".to_string());
                        }
                        let end_val = self.stack.last().cloned().unwrap_or(Value::Void);
                        let start_val = self.stack.get(self.stack.len() - 2).cloned().unwrap_or(Value::Void);
                        let start = match start_val {
                            Value::Int(i) => i as usize,
                            Value::Long(i) => i as usize,
                            _ => return Err("String.substring: start must be an integer".to_string()),
                        };
                        let end = match end_val {
                            Value::Int(i) => i as usize,
                            Value::Long(i) => i as usize,
                            _ => return Err("String.substring: end must be an integer".to_string()),
                        };
                        if start > end || end > s.chars().count() {
                            return Err(format!("String.substring: indices out of range ({}..{}) for string of length {}", start, end, s.chars().count()));
                        }
                        let sub: String = s.chars().skip(start).take(end - start).collect();
                        self.stack.drain(receiver_idx..);
                        self.push(Value::String(Rc::new(sub)));
                    }
                    "charAt" => {
                        let idx_val = if arg_count > 0 {
                            self.stack.last().cloned().unwrap_or(Value::Void)
                        } else {
                            return Err("String.charAt requires 1 argument".to_string());
                        };
                        let idx = match idx_val {
                            Value::Int(i) => i as usize,
                            Value::Long(i) => i as usize,
                            _ => return Err("String.charAt: index must be an integer".to_string()),
                        };
                        match s.chars().nth(idx) {
                            Some(c) => {
                                self.stack.drain(receiver_idx..);
                                self.push(Value::Char(c));
                            }
                            None => return Err(format!("String.charAt: index {} out of range", idx)),
                        }
                    }
                    _ => {
                        return Err(format!(
                            "No method '{}' on string",
                            method_name
                        ))
                    }
                }
            }
            Value::ResultOk(inner) => {
                match method_name.as_str() {
                    "unwrap" => {
                        self.stack.drain(receiver_idx..);
                        self.push((**inner).clone());
                    }
                    _ => {
                        return Err(format!(
                            "No method '{}' on Result",
                            method_name
                        ))
                    }
                }
            }
            Value::ResultErr(err_val) => {
                match method_name.as_str() {
                    "unwrap" => {
                        return Err(format!(
                            "called unwrap on an Err value: {}",
                            err_val.display_string()
                        ));
                    }
                    _ => {
                        return Err(format!(
                            "No method '{}' on Result",
                            method_name
                        ))
                    }
                }
            }
            Value::FileHandle(file_rc) => {
                match method_name.as_str() {
                    "readLine" => {
                        let result = {
                            let mut file_opt = file_rc.borrow_mut();
                            match file_opt.as_mut() {
                                Some(file) => {
                                    use std::io::Read;
                                    let mut line = String::new();
                                    let mut byte = [0u8; 1];
                                    loop {
                                        match file.read(&mut byte) {
                                            Ok(0) => break, // EOF
                                            Ok(_) => {
                                                let ch = byte[0] as char;
                                                if ch == '\n' {
                                                    break;
                                                }
                                                line.push(ch);
                                            }
                                            Err(e) => {
                                                line = format!("FileHandle.readLine: read error: {}", e);
                                                break;
                                            }
                                        }
                                    }
                                    if line.is_empty() {
                                        Value::ResultErr(Box::new(Value::String(Rc::new("EOF".to_string()))))
                                    } else {
                                        if line.ends_with('\r') { line.pop(); }
                                        Value::ResultOk(Box::new(Value::String(Rc::new(line))))
                                    }
                                }
                                None => Value::ResultErr(Box::new(Value::String(Rc::new("FileHandle is closed".to_string())))),
                            }
                        };
                        self.stack.drain(receiver_idx..);
                        self.push(result);
                    }
                    "write" => {
                        if arg_count == 0 {
                            return Err("FileHandle.write requires 1 argument (content)".to_string());
                        }
                        let content = self.stack.last().cloned().unwrap_or(Value::Void);
                        let result = {
                            let mut file_opt = file_rc.borrow_mut();
                            match file_opt.as_mut() {
                                Some(file) => {
                                    match &content {
                                        Value::String(s) => {
                                            match file.write_all(s.as_bytes()) {
                                                Ok(()) => Value::ResultOk(Box::new(Value::Void)),
                                                Err(e) => Value::ResultErr(Box::new(Value::String(Rc::new(format!("FileHandle.write: {}", e))))),
                                            }
                                        }
                                        _ => Value::ResultErr(Box::new(Value::String(Rc::new("FileHandle.write: expected String argument".to_string())))),
                                    }
                                }
                                None => Value::ResultErr(Box::new(Value::String(Rc::new("FileHandle is closed".to_string())))),
                            }
                        };
                        self.stack.drain(receiver_idx..);
                        self.push(result);
                    }
                    "close" => {
                        let mut file_opt = file_rc.borrow_mut();
                        *file_opt = None;
                        drop(file_opt);
                        self.stack.drain(receiver_idx..);
                        self.push(Value::Void);
                    }
                    _ => {
                        return Err(format!(
                            "No method '{}' on FileHandle",
                            method_name
                        ))
                    }
                }
            }
            Value::EnumInstance { variant, fields, .. } => {
                // Handle enum instance methods
                match method_name.as_str() {
                    "toString" => {
                        let s = if fields.is_empty() {
                            variant.clone()
                        } else {
                            let items: Vec<String> = fields.iter().map(|v| v.display_string()).collect();
                            format!("{}({})", variant, items.join(", "))
                        };
                        self.stack.drain(receiver_idx..);
                        self.push(Value::String(Rc::new(s)));
                    }
                    _ => {
                        return Err(format!(
                            "INVOKE_VIRTUAL: cannot invoke '{}' on enum instance '{}'",
                            method_name, variant
                        ))
                    }
                }
            }
            Value::Array { elements } => {
                // Handle Array methods (size, get, contains, isEmpty, indexOf, toString)
                let elements = elements.clone();
                let result = match method_name.as_str() {
                    "size" | "length" | "count" => Value::Int(elements.len() as i32),
                    "isEmpty" => Value::Bool(elements.is_empty()),
                    "get" => {
                        if arg_count < 1 {
                            return Err("Array.get requires 1 argument".to_string());
                        }
                        let idx_val = self.stack.last().cloned().unwrap_or(Value::Void);
                        let idx = match idx_val {
                            Value::Int(i) => i as usize,
                            Value::Long(i) => i as usize,
                            _ => return Err("Array.get: index must be an integer".to_string()),
                        };
                        if idx < elements.len() {
                            elements[idx].clone()
                        } else {
                            return Err(format!("Array index out of bounds: {}", idx));
                        }
                    }
                    "contains" => {
                        if arg_count < 1 {
                            return Err("Array.contains requires 1 argument".to_string());
                        }
                        let item = self.stack.last().cloned().unwrap_or(Value::Void);
                        Value::Bool(elements.iter().any(|e| values_eq(e, &item)))
                    }
                    "indexOf" => {
                        if arg_count < 1 {
                            return Err("Array.indexOf requires 1 argument".to_string());
                        }
                        let item = self.stack.last().cloned().unwrap_or(Value::Void);
                        let mut found = -1i32;
                        for (i, e) in elements.iter().enumerate() {
                            if values_eq(e, &item) {
                                found = i as i32;
                                break;
                            }
                        }
                        Value::Int(found)
                    }
                    "toString" => {
                        let items: Vec<String> = elements.iter().map(|v| v.display_string()).collect();
                        Value::String(Rc::new(format!("[{}]", items.join(", "))))
                    }
                    _ => {
                        return Err(format!(
                            "INVOKE_VIRTUAL: cannot invoke '{}' on Array",
                            method_name
                        ))
                    }
                };
                self.stack.drain(receiver_idx..);
                self.push(result);
            }
            Value::Tuple { elements } => {
                let elements = elements.clone();
                let result = match method_name.as_str() {
                    "size" | "length" => Value::Int(elements.len() as i32),
                    "toString" => {
                        let items: Vec<String> = elements.iter().map(|v| v.display_string()).collect();
                        Value::String(Rc::new(format!("({})", items.join(", "))))
                    }
                    _ => {
                        return Err(format!(
                            "INVOKE_VIRTUAL: cannot invoke '{}' on Tuple",
                            method_name
                        ))
                    }
                };
                self.stack.drain(receiver_idx..);
                self.push(result);
            }
            // Primitive type methods
            Value::Long(_) | Value::Int(_) | Value::Byte(_) | Value::Short(_)
            | Value::Double(_) | Value::Float(_) | Value::Half(_) | Value::Quad(_)
            | Value::Vast(_) | Value::Uvast(_) | Value::Bool(_) | Value::Char(_) => {
                let result = match method_name.as_str() {
                    "toString" => Value::String(Rc::new(receiver.display_string())),
                    "toDouble" => match &receiver {
                        Value::Long(v) => Value::Double(*v as f64),
                        Value::Int(v) => Value::Double(*v as f64),
                        Value::Byte(v) => Value::Double(*v as f64),
                        Value::Short(v) => Value::Double(*v as f64),
                        Value::Double(v) => Value::Double(*v),
                        Value::Float(v) => Value::Double(*v as f64),
                        Value::Half(v) => Value::Double(*v as f64),
                        Value::Quad(v) => Value::Double(*v),
                        Value::Bool(v) => Value::Double(if *v { 1.0 } else { 0.0 }),
                        Value::Char(v) => Value::Double(*v as u32 as f64),
                        Value::Vast(v) => Value::Double(*v as f64),
                        Value::Uvast(v) => Value::Double(*v as f64),
                        _ => Value::Double(0.0),
                    },
                    "toInt" => match &receiver {
                        Value::Long(v) => Value::Int(*v as i32),
                        Value::Int(v) => Value::Int(*v),
                        Value::Byte(v) => Value::Int(*v as i32),
                        Value::Short(v) => Value::Int(*v as i32),
                        Value::Double(v) => Value::Int(*v as i32),
                        Value::Float(v) => Value::Int(*v as i32),
                        Value::Bool(v) => Value::Int(if *v { 1 } else { 0 }),
                        Value::Char(v) => Value::Int(*v as u32 as i32),
                        Value::Vast(v) => Value::Int(*v as i32),
                        Value::Uvast(v) => Value::Int(*v as i32),
                        _ => Value::Int(0),
                    },
                    "toLong" => match &receiver {
                        Value::Long(v) => Value::Long(*v),
                        Value::Int(v) => Value::Long(*v as i64),
                        Value::Byte(v) => Value::Long(*v as i64),
                        Value::Short(v) => Value::Long(*v as i64),
                        Value::Double(v) => Value::Long(*v as i64),
                        Value::Float(v) => Value::Long(*v as i64),
                        Value::Bool(v) => Value::Long(if *v { 1 } else { 0 }),
                        Value::Char(v) => Value::Long(*v as u32 as i64),
                        Value::Vast(v) => Value::Long(*v as i64),
                        Value::Uvast(v) => Value::Long(*v as i64),
                        _ => Value::Long(0),
                    },
                    "toChar" => match &receiver {
                        Value::Int(v) => Value::Char(char::from_u32(*v as u32).unwrap_or('\0')),
                        Value::Long(v) => Value::Char(char::from_u32(*v as u32).unwrap_or('\0')),
                        Value::Char(v) => Value::Char(*v),
                        _ => Value::Char('\0'),
                    },
                    _ => {
                        return Err(format!(
                            "INVOKE_VIRTUAL: cannot invoke '{}' on {:?}",
                            method_name, receiver
                        ))
                    }
                };
                self.stack.drain(receiver_idx..);
                self.push(result);
            }
            Value::Null => {
                return Err(format!(
                    "INVOKE_VIRTUAL: cannot invoke '{}' on null",
                    method_name
                ))
            }
            _ => {
                return Err(format!(
                    "INVOKE_VIRTUAL: cannot invoke '{}' on {:?}",
                    method_name, receiver
                ))
            }
        }

        Ok(())
    }
}
