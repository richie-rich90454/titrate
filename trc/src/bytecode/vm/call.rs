// Titrate Alpha 0.2 – bytecode virtual machine: call handling
// Precision in every step – richie-rich90454, 2026

use super::super::frame::Frame;
use super::super::value::Value;
use super::Vm;
use std::collections::HashMap;
use std::io::Write;
use std::rc::Rc;

impl Vm {
    pub(super) fn call_function(&mut self, func_idx: u16, arg_count: u8) -> Result<(), String> {
        if self.frames.len() >= self.max_call_depth {
            return Err("Stack overflow: maximum call depth exceeded".to_string());
        }

        let fi = func_idx as usize;
        if fi >= self.functions.len() {
            return Err(format!("CALL: function index {} out of range", func_idx));
        }

        let arity = self.functions[fi].arity;
        if (arg_count as usize) != arity {
            return Err(format!(
                "CALL: function {} expects {} args, got {}",
                self.functions[fi].name, arity, arg_count
            ));
        }

        let base = self.stack.len() - arg_count as usize;

        // Check if there's a Closure value on the stack with this function index.
        // Search the stack for a matching closure to get its upvalues.
        let upvalues = self.find_closure_upvalues(func_idx);

        if let Some(uvs) = upvalues {
            self.frames.push(Frame::new_with_upvalues(func_idx, base, uvs));
        } else {
            self.frames.push(Frame::new(func_idx, base));
        }

        // Pre-allocate stack slots for all local variables.
        // The function has `local_count` total slots, of which `arity` are
        // already occupied by arguments. Fill the rest with Null.
        let local_count = self.functions[fi].local_count;
        let needed = base + local_count;
        while self.stack.len() < needed {
            self.stack.push(Value::Null);
        }

        Ok(())
    }

    /// Search the stack for a Closure value with the given function index
    /// and return its upvalues if found.
    pub(super) fn find_closure_upvalues(&self, func_idx: u16) -> Option<Vec<Value>> {
        for val in self.stack.iter().rev() {
            if let Value::Closure { func_idx: idx, upvalues } = val {
                if *idx == func_idx as usize {
                    return Some(upvalues.clone());
                }
            }
        }
        None
    }

    pub(super) fn call_native_fn(&mut self, native_idx: u16, arg_count: u8) -> Result<(), String> {
        let ni = native_idx as usize;
        if ni >= self.natives.len() {
            return Err(format!(
                "CALL_NATIVE: native index {} out of range",
                native_idx
            ));
        }

        let arg_start = self.stack.len() - arg_count as usize;
        let args: Vec<Value> = self.stack.drain(arg_start..).collect();

        // Special handling for println (native index 0): capture output
        if ni == 0 {
            // println
            let output = if args.is_empty() {
                String::new()
            } else {
                args[0].display_string()
            };
            self.output.push(output);
            self.push(Value::Void);
            return Ok(());
        }

        let func = self.natives[ni];
        let result = func(&args)?;
        self.push(result);
        Ok(())
    }

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
        let receiver = self.stack[receiver_idx].clone();

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
                    _ => {}
                }

                // Look up method in vtable
                let func_idx = if let Some(idx) = vtable.get(&method_name) {
                    *idx
                } else {
                    // Walk up the class hierarchy to find the method
                    let mut search_class = class_name.clone();
                    let mut found_idx = None;
                    loop {
                        let class_defs = &self.classes;
                        let found = class_defs.iter().find(|c| c.name == search_class);
                        match found {
                            Some(cd) => {
                                if let Some(idx) = cd.methods.get(&method_name) {
                                    found_idx = Some(*idx);
                                    break;
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
                            return Err(format!(
                                "No method '{}' on class '{}'",
                                method_name, class_name
                            ));
                        }
                    }
                };

                let base = receiver_idx;
                self.frames.push(Frame::new(func_idx, base));
            }
            Value::String(s) => {
                // Handle string methods
                match method_name.as_str() {
                    "length" => {
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
            _ => {
                return Err(format!(
                    "INVOKE_VIRTUAL: cannot invoke '{}' on {:?}",
                    method_name, receiver
                ))
            }
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Built-in ArrayList methods
    // -----------------------------------------------------------------------

    pub(super) fn call_arraylist_method(
        &mut self,
        fields: &std::rc::Rc<std::cell::RefCell<HashMap<String, Value>>>,
        method: &str,
        arg_count: u8,
    ) -> Result<Value, String> {
        match method {
            "add" => {
                let item = if arg_count > 0 {
                    self.stack.last().cloned().unwrap_or(Value::Void)
                } else {
                    return Err("ArrayList.add requires 1 argument".to_string());
                };
                let mut elements = match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                elements.push(item);
                fields.borrow_mut().insert("_elements".to_string(), Value::Array { elements });
                Ok(Value::Void)
            }
            "get" => {
                let idx_val = if arg_count > 0 {
                    self.stack.last().cloned().unwrap_or(Value::Void)
                } else {
                    return Err("ArrayList.get requires 1 argument".to_string());
                };
                let idx = match idx_val {
                    Value::Int(i) => i as usize,
                    Value::Long(i) => i as usize,
                    _ => return Err("ArrayList.get requires an integer index".to_string()),
                };
                match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => {
                        if idx < elements.len() {
                            Ok(elements[idx].clone())
                        } else {
                            Err(format!("ArrayList index out of bounds: {}", idx))
                        }
                    }
                    _ => Err("ArrayList has no elements".to_string()),
                }
            }
            "size" => {
                match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => Ok(Value::Int(elements.len() as i32)),
                    _ => Ok(Value::Int(0)),
                }
            }
            "set" => {
                if arg_count < 2 {
                    return Err("ArrayList.set requires 2 arguments (index, value)".to_string());
                }
                let value = self.pop();
                let idx_val = self.pop();
                let idx = match idx_val {
                    Value::Int(i) => i as usize,
                    Value::Long(i) => i as usize,
                    _ => return Err("ArrayList.set requires an integer index".to_string()),
                };
                match fields.borrow_mut().get_mut("_elements") {
                    Some(Value::Array { elements }) => {
                        if idx < elements.len() {
                            elements[idx] = value;
                            Ok(Value::Void)
                        } else {
                            Err(format!("ArrayList index out of bounds: {}", idx))
                        }
                    }
                    _ => Err("ArrayList has no elements".to_string()),
                }
            }
            "remove" => {
                if arg_count < 1 {
                    return Err("ArrayList.remove requires 1 argument (index)".to_string());
                }
                let idx_val = self.pop();
                let idx = match idx_val {
                    Value::Int(i) => i as usize,
                    Value::Long(i) => i as usize,
                    _ => return Err("ArrayList.remove requires an integer index".to_string()),
                };
                match fields.borrow_mut().get_mut("_elements") {
                    Some(Value::Array { elements }) => {
                        if idx < elements.len() {
                            Ok(elements.remove(idx))
                        } else {
                            Err(format!("ArrayList index out of bounds: {}", idx))
                        }
                    }
                    _ => Err("ArrayList has no elements".to_string()),
                }
            }
            "sort" => Ok(Value::Void),
            "length" => {
                match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => Ok(Value::Int(elements.len() as i32)),
                    _ => Ok(Value::Int(0)),
                }
            }
            "forEach" => {
                if arg_count < 1 {
                    return Err("ArrayList.forEach requires 1 argument (closure)".to_string());
                }
                let closure = self.stack.last().cloned().unwrap_or(Value::Void);
                let elements = match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                for elem in &elements {
                    self.call_closure_with_args(&closure, &[elem.clone()])?;
                }
                Ok(Value::Void)
            }
            "toString" => {
                let elements = match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let items: Vec<String> = elements.iter().map(|e| e.display_string()).collect();
                Ok(Value::String(Rc::new(format!("[{}]", items.join(", ")))))
            }
            _ => Err(format!("Unknown ArrayList method '{}'", method)),
        }
    }

    // -----------------------------------------------------------------------
    // Built-in HashMap methods
    // -----------------------------------------------------------------------

    pub(super) fn call_hashmap_method(
        &mut self,
        fields: &std::rc::Rc<std::cell::RefCell<HashMap<String, Value>>>,
        method: &str,
        arg_count: u8,
    ) -> Result<Value, String> {
        match method {
            "put" => {
                if arg_count < 2 {
                    return Err("HashMap.put requires 2 arguments".to_string());
                }
                let stack_len = self.stack.len();
                let key = self.stack[stack_len - 2].clone();
                let value = self.stack[stack_len - 1].clone();
                let mut keys = match fields.borrow().get("_keys") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let mut values = match fields.borrow().get("_values") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let mut found = false;
                for (i, k) in keys.iter().enumerate() {
                    if *k == key {
                        values[i] = value.clone();
                        found = true;
                        break;
                    }
                }
                if !found {
                    keys.push(key);
                    values.push(value);
                }
                fields.borrow_mut().insert("_keys".to_string(), Value::Array { elements: keys });
                fields.borrow_mut().insert("_values".to_string(), Value::Array { elements: values });
                Ok(Value::Void)
            }
            "get" => {
                if arg_count < 1 {
                    return Err("HashMap.get requires 1 argument".to_string());
                }
                let key = self.stack.last().cloned().unwrap_or(Value::Void);
                let keys = match fields.borrow().get("_keys") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => return Ok(Value::Null),
                };
                let values = match fields.borrow().get("_values") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => return Ok(Value::Null),
                };
                for (i, k) in keys.iter().enumerate() {
                    if *k == key {
                        return Ok(values.get(i).cloned().unwrap_or(Value::Null));
                    }
                }
                Ok(Value::Null)
            }
            "containsKey" => {
                if arg_count < 1 {
                    return Err("HashMap.containsKey requires 1 argument".to_string());
                }
                let key = self.stack.last().cloned().unwrap_or(Value::Void);
                let keys = match fields.borrow().get("_keys") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => return Ok(Value::Bool(false)),
                };
                Ok(Value::Bool(keys.iter().any(|k| *k == key)))
            }
            "keys" => {
                let keys = match fields.borrow().get("_keys") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let mut al_fields = HashMap::new();
                al_fields.insert("_elements".to_string(), Value::Array { elements: keys });
                Ok(Value::ClassInstance {
                    class_name: "ArrayList".to_string(),
                    fields: Rc::new(std::cell::RefCell::new(al_fields)),
                    vtable: HashMap::new(),
                })
            }
            "values" => {
                let values = match fields.borrow().get("_values") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let mut al_fields = HashMap::new();
                al_fields.insert("_elements".to_string(), Value::Array { elements: values });
                Ok(Value::ClassInstance {
                    class_name: "ArrayList".to_string(),
                    fields: Rc::new(std::cell::RefCell::new(al_fields)),
                    vtable: HashMap::new(),
                })
            }
            "entries" => {
                let keys = match fields.borrow().get("_keys") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let values = match fields.borrow().get("_values") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let entries: Vec<Value> = keys.iter().zip(values.iter()).map(|(k, v)| {
                    Value::Tuple { elements: vec![k.clone(), v.clone()] }
                }).collect();
                let mut al_fields = HashMap::new();
                al_fields.insert("_elements".to_string(), Value::Array { elements: entries });
                Ok(Value::ClassInstance {
                    class_name: "ArrayList".to_string(),
                    fields: Rc::new(std::cell::RefCell::new(al_fields)),
                    vtable: HashMap::new(),
                })
            }
            "size" => {
                let keys = match fields.borrow().get("_keys") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                Ok(Value::Int(keys.len() as i32))
            }
            "remove" => {
                if arg_count < 1 {
                    return Err("HashMap.remove requires 1 argument".to_string());
                }
                let key = self.stack.last().cloned().unwrap_or(Value::Void);
                let mut keys = match fields.borrow().get("_keys") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => return Ok(Value::Null),
                };
                let mut values = match fields.borrow().get("_values") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => return Ok(Value::Null),
                };
                let mut found_idx = None;
                for (i, k) in keys.iter().enumerate() {
                    if *k == key {
                        found_idx = Some(i);
                        break;
                    }
                }
                match found_idx {
                    Some(i) => {
                        let old_val = values.remove(i);
                        keys.remove(i);
                        fields.borrow_mut().insert("_keys".to_string(), Value::Array { elements: keys });
                        fields.borrow_mut().insert("_values".to_string(), Value::Array { elements: values });
                        Ok(old_val)
                    }
                    None => Ok(Value::Null),
                }
            }
            "toString" => {
                let keys = match fields.borrow().get("_keys") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let values = match fields.borrow().get("_values") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let items: Vec<String> = keys.iter().zip(values.iter())
                    .map(|(k, v)| format!("{}: {}", k.display_string(), v.display_string()))
                    .collect();
                Ok(Value::String(Rc::new(format!("{{{}}}", items.join(", ")))))
            }
            _ => Err(format!("Unknown HashMap method '{}'", method)),
        }
    }

    // -----------------------------------------------------------------------
    // INVOKE_OPERATOR – operator overloading with fallback
    // -----------------------------------------------------------------------

    pub(super) fn invoke_operator(&mut self, method_name_idx: u16, arg_count: u8) -> Result<(), String> {
        // Stack: [left, right, ...]  (left is below right)
        // arg_count should be 1 (the right operand).
        let method_name = {
            let frame = self.current_frame();
            let chunk = &self.functions[frame.function_index as usize].chunk;
            chunk.strings[method_name_idx as usize].clone()
        };

        // The receiver (left operand) is at stack.len() - 1 - arg_count
        let receiver_idx = self.stack.len() - 1 - arg_count as usize;
        let receiver = self.stack[receiver_idx].clone();

        if let Value::ClassInstance { vtable, class_name, .. } = &receiver {
            // Check if the operator method exists in the vtable or class hierarchy
            let has_operator = if vtable.contains_key(&method_name) {
                true
            } else {
                // Walk up the class hierarchy
                let mut search_class = class_name.clone();
                loop {
                    let class_defs = &self.classes;
                    let found = class_defs.iter().find(|c| c.name == search_class);
                    match found {
                        Some(cd) => {
                            if cd.methods.contains_key(&method_name) {
                                break true;
                            }
                            if let Some(parent_idx) = cd.parent {
                                search_class = self.classes[parent_idx as usize].name.clone();
                            } else {
                                break false;
                            }
                        }
                        None => break false,
                    }
                }
            };

            if has_operator {
                // Delegate to invoke_method which handles the full method call
                return self.invoke_method(method_name_idx, arg_count);
            }
        }

        // No operator method found — fall back to built-in operator behavior.
        // Pop right and left from the stack, apply the built-in operator, push result.
        let right = self.pop();
        let left = self.pop();
        let result = self.apply_builtin_operator(&left, &right, &method_name)?;
        self.push(result);
        Ok(())
    }

    /// Call a closure value with the given arguments.
    /// Used by built-in methods like ArrayList.forEach.
    pub(super) fn call_closure_with_args(&mut self, closure: &Value, args: &[Value]) -> Result<(), String> {
        match closure {
            Value::Closure { func_idx, .. } => {
                let fi = *func_idx;
                if fi >= self.functions.len() {
                    return Err("forEach: invalid closure".to_string());
                }
                let arity = self.functions[fi].arity;
                let base = self.stack.len();
                // Push args
                for arg in args {
                    self.push(arg.clone());
                }
                // Pad if needed
                for _ in args.len()..arity as usize {
                    self.push(Value::Null);
                }
                self.frames.push(Frame::new(fi as u16, base));
                // Execute the closure frame
                while self.frames.len() > 1 {
                    self.step()?;
                }
                // Pop the return value
                let _ = self.pop();
                Ok(())
            }
            _ => Err("forEach: expected a closure".to_string()),
        }
    }

    /// Apply a built-in operator when no operator overload method is found.
    pub(super) fn apply_builtin_operator(&self, left: &Value, right: &Value, method_name: &str) -> Result<Value, String> {
        match method_name {
            "operator+" => self.builtin_add(left, right),
            "operator-" => self.builtin_sub(left, right),
            "operator*" => self.builtin_mul(left, right),
            "operator/" => self.builtin_div(left, right),
            "operator%" => self.builtin_mod(left, right),
            "operator==" => Ok(Value::Bool(left == right)),
            "operator!=" => Ok(Value::Bool(left != right)),
            "operator<" => self.builtin_cmp(left, right, |a, b| a < b, |a, b| a < b),
            "operator>" => self.builtin_cmp(left, right, |a, b| a > b, |a, b| a > b),
            "operator<=" => self.builtin_cmp(left, right, |a, b| a <= b, |a, b| a <= b),
            "operator>=" => self.builtin_cmp(left, right, |a, b| a >= b, |a, b| a >= b),
            "operator&" => self.builtin_bitwise(left, right, |a, b| a & b, |a, b| a & b),
            "operator|" => self.builtin_bitwise(left, right, |a, b| a | b, |a, b| a | b),
            "operator^" => self.builtin_bitwise(left, right, |a, b| a ^ b, |a, b| a ^ b),
            "operator<<" => self.builtin_shift(left, right, false),
            "operator>>" => self.builtin_shift(left, right, true),
            _ => Err(format!("Unknown operator method '{}'", method_name)),
        }
    }
}
