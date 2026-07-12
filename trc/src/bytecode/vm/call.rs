// Titrate Alpha 0.2 – bytecode virtual machine: call handling
// Precision in every step – richie-rich90454, 2026

use super::super::frame::Frame;
use super::super::value::{Value, values_eq};
use super::Vm;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write;
use std::rc::Rc;

impl Vm {
    pub(super) fn call_function(&mut self, func_idx: u16, arg_count: u8) -> Result<(), String> {
        if self.frames.len() >= self.max_call_depth {
            let name = self.functions.get(func_idx as usize).map(|f| f.name.as_str()).unwrap_or("?");
            return Err(format!("Stack overflow: maximum call depth exceeded calling {}", name));
        }

        let fi = func_idx as usize;
        if fi >= self.functions.len() {
            return Err(format!("CALL: function index {} out of range", func_idx));
        }

        let arity = self.functions[fi].arity;
        let fname = &self.functions[fi].name;
        // Special case: Variant.of(tag, value) may be called with 1 arg
        // (just the value). Insert a default tag "variant" before the value.
        if (arg_count as usize) != arity {
            if fname.ends_with("Variant.of") && arg_count == 1 && arity == 2 {
                let insert_pos = self.stack.len() - 1;
                self.stack.insert(insert_pos, Value::String(Rc::new("variant".to_string())));
            } else {
                // Try to find an overload with matching arity.
                // The compiler may register multiple functions with the same
                // name but different arities (overloading). Search the function
                // table for a function with the same name but matching arity.
                let mut found: Option<u16> = None;
                for (i, f) in self.functions.iter().enumerate() {
                    if f.name == fname.as_str() && f.arity == arg_count as usize {
                        found = Some(i as u16);
                        break;
                    }
                }
                // If exact name match failed, try mangled-name prefix matching.
                // Functions overloaded by parameter type are mangled as
                // "module.function__ParamType" (e.g. pformat__Variant,
                // pformat__string). Strip the "__..." suffix and look for a
                // sibling with matching arity.
                if found.is_none() {
                    if let Some(stripped) = fname.rfind("__").map(|idx| &fname[..idx]) {
                        for (i, f) in self.functions.iter().enumerate() {
                            let f_stripped = f.name.rfind("__").map(|idx| &f.name[..idx]).unwrap_or(&f.name);
                            if f_stripped == stripped && f.arity == arg_count as usize {
                                found = Some(i as u16);
                                break;
                            }
                        }
                    }
                }
                if let Some(alt_idx) = found {
                    return self.call_function(alt_idx, arg_count);
                }
                return Err(format!(
                    "CALL: function {} expects {} args, got {}",
                    fname, arity, arg_count
                ));
            }
        }

        let base = self.stack.len() - arity;

        // Guard: if the target function has an empty chunk, it was never
        // compiled (e.g. a generic function that was registered but not
        // instantiated, or a module that failed to compile).  Return a
        // meaningful error instead of panicking on index-out-of-bounds.
        if self.functions[fi].chunk.code.is_empty() {
            return Err(format!(
                "CALL: function '{}' has no body (empty chunk)",
                fname
            ));
        }

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
    pub(super) fn find_closure_upvalues(&self, func_idx: u16) -> Option<Vec<Rc<RefCell<Value>>>> {
        for val in self.stack.iter().rev() {
            if let Value::Closure { func_idx: idx, upvalues } = val {
                if *idx == func_idx as usize {
                    return Some(upvalues.clone());
                }
            }
        }
        None
    }

    /// Call a closure that sits on the stack below the arguments.
    /// Stack layout before call: [closure, arg0, arg1, ..., argN-1]
    /// The closure value is popped, then a frame is set up with the args.
    pub(super) fn call_closure_from_stack(&mut self, arg_count: u8) -> Result<(), String> {
        if self.frames.len() >= self.max_call_depth {
            return Err("Stack overflow: maximum call depth exceeded calling closure".to_string());
        }
        let ac = arg_count as usize;
        if self.stack.len() < ac + 1 {
            return Err("CALL_CLOSURE: stack underflow".to_string());
        }
        // The closure is just below the arguments.
        let closure_idx = self.stack.len() - ac - 1;
        let closure = self.stack[closure_idx].clone();
        let (func_idx, upvalues) = match &closure {
            Value::Closure { func_idx, upvalues } => (*func_idx, upvalues.clone()),
            _ => {
                return Err(format!(
                    "CALL_CLOSURE: expected closure on stack, got {:?}",
                    closure
                ))
            }
        };
        let fi = func_idx;
        if fi >= self.functions.len() {
            return Err(format!("CALL_CLOSURE: function index {} out of range", fi));
        }
        let arity = self.functions[fi].arity;
        // Guard: empty chunk means the function was never compiled.
        if self.functions[fi].chunk.code.is_empty() {
            return Err(format!(
                "CALL_CLOSURE: function '{}' has no body (empty chunk)",
                self.functions[fi].name
            ));
        }
        if ac != arity {
            return Err(format!(
                "CALL_CLOSURE: function {} expects {} args, got {}",
                self.functions[fi].name, arity, ac
            ));
        }
        // Remove the closure from the stack, leaving args in place.
        self.stack.remove(closure_idx);
        let base = self.stack.len() - ac;
        // Set up the frame with upvalues from the closure.
        if upvalues.is_empty() {
            self.frames.push(Frame::new(fi as u16, base));
        } else {
            self.frames.push(Frame::new_with_upvalues(fi as u16, base, upvalues));
        }
        // Pre-allocate local slots.
        let local_count = self.functions[fi].local_count;
        let needed = base + local_count;
        while self.stack.len() < needed {
            self.stack.push(Value::Null);
        }
        Ok(())
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

        // Special handling for Thread_spawn: execute the closure argument
        // synchronously because Value contains Rc<> which is not Send-safe.
        // The native_thread_spawn function already spawned a no-op thread and
        // returned a handle; we run the closure here on the calling thread.
        if ni as u16 == self.thread_spawn_idx {
            if let Some(closure) = args.first() {
                if matches!(closure, Value::Closure { .. }) {
                    // Execute the closure synchronously (ignoring its return value).
                    let _ = self.call_closure_with_args(closure, &[]);
                    let _ = self.pop();
                }
            }
        }

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
        let receiver_raw = self.stack[receiver_idx].clone();
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
                if arg_count == 0 {
                    return Err("ArrayList.add requires at least 1 argument".to_string());
                }
                let mut elements = match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                if arg_count == 1 {
                    let item = self.stack.last().cloned().unwrap_or(Value::Void);
                    elements.push(item);
                } else {
                    let item = self.stack.last().cloned().unwrap_or(Value::Void);
                    let idx_val = self.stack.get(self.stack.len() - 2).cloned().unwrap_or(Value::Void);
                    let idx = match idx_val {
                        Value::Int(i) => i as usize,
                        Value::Long(i) => i as usize,
                        _ => return Err("ArrayList.add with index requires integer index".to_string()),
                    };
                    if idx <= elements.len() {
                        elements.insert(idx, item);
                    } else {
                        elements.push(item);
                    }
                }
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
                    self.call_closure_with_args(&closure, std::slice::from_ref(elem))?;
                    let _ = self.pop();
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
            "indexOf" => {
                if arg_count < 1 {
                    return Err("ArrayList.indexOf requires 1 argument".to_string());
                }
                let target = self.stack.last().cloned().unwrap_or(Value::Void);
                let elements = match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                for (i, e) in elements.iter().enumerate() {
                    if *e == target {
                        return Ok(Value::Int(i as i32));
                    }
                }
                Ok(Value::Int(-1))
            }
            "lastIndexOf" => {
                if arg_count < 1 {
                    return Err("ArrayList.lastIndexOf requires 1 argument".to_string());
                }
                let target = self.stack.last().cloned().unwrap_or(Value::Void);
                let elements = match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                for (i, e) in elements.iter().enumerate().rev() {
                    if *e == target {
                        return Ok(Value::Int(i as i32));
                    }
                }
                Ok(Value::Int(-1))
            }
            "filter" => {
                if arg_count < 1 {
                    return Err("ArrayList.filter requires 1 argument (closure)".to_string());
                }
                let closure = self.stack.last().cloned().unwrap_or(Value::Void);
                let elements = match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let mut result = Vec::new();
                for elem in &elements {
                    self.call_closure_with_args(&closure, std::slice::from_ref(elem))?;
                    let keep = match self.pop() {
                        Value::Bool(b) => b,
                        _ => false,
                    };
                    if keep {
                        result.push(elem.clone());
                    }
                }
                let mut al_fields = HashMap::new();
                al_fields.insert("_elements".to_string(), Value::Array { elements: result });
                Ok(Value::ClassInstance {
                    class_name: "ArrayList".to_string(),
                    fields: Rc::new(std::cell::RefCell::new(al_fields)),
                    vtable: HashMap::new(),
                })
            }
            "contains" => {
                if arg_count < 1 {
                    return Err("ArrayList.contains requires 1 argument".to_string());
                }
                let target = self.stack.last().cloned().unwrap_or(Value::Void);
                let elements = match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                Ok(Value::Bool(elements.contains(&target)))
            }
            "clear" => {
                fields.borrow_mut().insert("_elements".to_string(), Value::Array { elements: vec![] });
                Ok(Value::Void)
            }
            "removeAt" => {
                if arg_count < 1 {
                    return Err("ArrayList.removeAt requires 1 argument".to_string());
                }
                let idx_val = self.stack.last().cloned().unwrap_or(Value::Void);
                let idx = match idx_val {
                    Value::Int(i) => i as usize,
                    Value::Long(i) => i as usize,
                    _ => return Err("ArrayList.removeAt requires an integer index".to_string()),
                };
                let mut elements = match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                if idx < elements.len() {
                    let removed = elements.remove(idx);
                    fields.borrow_mut().insert("_elements".to_string(), Value::Array { elements });
                    Ok(removed)
                } else {
                    Err(format!("ArrayList removeAt: index {} out of bounds (len {})", idx, elements.len()))
                }
            }
            "max" => {
                let elements = match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                if elements.is_empty() {
                    return Ok(Value::Null);
                }
                let mut best = elements[0].clone();
                for e in &elements[1..] {
                    let better = match (&best, e) {
                        (Value::Int(a), Value::Int(b)) => b > a,
                        (Value::Long(a), Value::Long(b)) => b > a,
                        (Value::Double(a), Value::Double(b)) => b > a,
                        (Value::Float(a), Value::Float(b)) => b > a,
                        _ => false,
                    };
                    if better {
                        best = e.clone();
                    }
                }
                Ok(best)
            }
            "min" => {
                let elements = match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                if elements.is_empty() {
                    return Ok(Value::Null);
                }
                let mut best = elements[0].clone();
                for e in &elements[1..] {
                    let better = match (&best, e) {
                        (Value::Int(a), Value::Int(b)) => b < a,
                        (Value::Long(a), Value::Long(b)) => b < a,
                        (Value::Double(a), Value::Double(b)) => b < a,
                        (Value::Float(a), Value::Float(b)) => b < a,
                        _ => false,
                    };
                    if better {
                        best = e.clone();
                    }
                }
                Ok(best)
            }
            "addAll" => {
                if arg_count < 1 {
                    return Err("ArrayList.addAll requires 1 argument".to_string());
                }
                let other = self.stack.last().cloned().unwrap_or(Value::Void);
                let mut elements = match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                match &other {
                    Value::ClassInstance { class_name, fields: other_fields, .. } if class_name.starts_with("ArrayList") => {
                        if let Some(Value::Array { elements: other_elements }) = other_fields.borrow().get("_elements") {
                            elements.extend(other_elements.iter().cloned());
                        }
                    }
                    Value::Array { elements: other_elements } => {
                        elements.extend(other_elements.iter().cloned());
                    }
                    _ => {}
                }
                fields.borrow_mut().insert("_elements".to_string(), Value::Array { elements });
                Ok(Value::Void)
            }
            "toArray" => {
                let elements = match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                Ok(Value::Array { elements })
            }
            "subList" => {
                if arg_count < 2 {
                    return Err("ArrayList.subList requires 2 arguments".to_string());
                }
                let end_val = self.stack.last().cloned().unwrap_or(Value::Void);
                let start_val = self.stack.get(self.stack.len() - 2).cloned().unwrap_or(Value::Void);
                let start = start_val.to_i64().unwrap_or(0) as usize;
                let end = end_val.to_i64().unwrap_or(0) as usize;
                let elements = match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let start = start.min(elements.len());
                let end = end.min(elements.len());
                let sub = if start <= end {
                    elements[start..end].to_vec()
                } else {
                    vec![]
                };
                let mut al_fields = HashMap::new();
                al_fields.insert("_elements".to_string(), Value::Array { elements: sub });
                Ok(Value::ClassInstance {
                    class_name: "ArrayList".to_string(),
                    fields: Rc::new(std::cell::RefCell::new(al_fields)),
                    vtable: HashMap::new(),
                })
            }
            "clone" | "shallowCopy" => {
                let elements = match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let mut al_fields = HashMap::new();
                al_fields.insert("_elements".to_string(), Value::Array { elements });
                Ok(Value::ClassInstance {
                    class_name: "ArrayList".to_string(),
                    fields: Rc::new(std::cell::RefCell::new(al_fields)),
                    vtable: HashMap::new(),
                })
            }
            "reverse" => {
                let mut elements = match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                elements.reverse();
                fields.borrow_mut().insert("_elements".to_string(), Value::Array { elements });
                Ok(Value::Void)
            }
            "sort" => {
                let mut elements = match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                elements.sort_by(|a, b| {
                    let av = a.to_f64().unwrap_or(0.0);
                    let bv = b.to_f64().unwrap_or(0.0);
                    av.partial_cmp(&bv).unwrap_or(std::cmp::Ordering::Equal)
                });
                fields.borrow_mut().insert("_elements".to_string(), Value::Array { elements });
                Ok(Value::Void)
            }
            "isEmpty" => {
                let len = match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => elements.len(),
                    _ => 0,
                };
                Ok(Value::Bool(len == 0))
            }
            "pop" => {
                let mut elements = match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                if elements.is_empty() {
                    Ok(Value::Null)
                } else {
                    let val = elements.pop().unwrap();
                    fields.borrow_mut().insert("_elements".to_string(), Value::Array { elements });
                    Ok(val)
                }
            }
            "reduce" => {
                if arg_count < 1 {
                    return Err("ArrayList.reduce requires 1 argument (closure)".to_string());
                }
                let closure = self.stack.last().cloned().unwrap_or(Value::Void);
                let elements = match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                if elements.is_empty() {
                    return Ok(Value::Null);
                }
                let mut acc = elements[0].clone();
                for elem in &elements[1..] {
                    self.call_closure_with_args(&closure, &[acc.clone(), elem.clone()])?;
                    acc = self.pop();
                }
                Ok(acc)
            }
            "iterator" => {
                let elements = match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let mut iter_fields = HashMap::new();
                iter_fields.insert("_elements".to_string(), Value::Array { elements });
                iter_fields.insert("_index".to_string(), Value::Int(0));
                Ok(Value::ClassInstance {
                    class_name: "Iterator".to_string(),
                    fields: Rc::new(std::cell::RefCell::new(iter_fields)),
                    vtable: HashMap::new(),
                })
            }
            "combinations" => {
                if arg_count < 1 {
                    return Err("ArrayList.combinations requires 1 argument".to_string());
                }
                let k_val = self.stack.last().cloned().unwrap_or(Value::Int(0));
                let k = k_val.to_i64().unwrap_or(0) as usize;
                let elements = match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let result = generate_combinations(&elements, k);
                let mut al_fields = HashMap::new();
                al_fields.insert("_elements".to_string(), Value::Array { elements: result });
                Ok(Value::ClassInstance {
                    class_name: "ArrayList".to_string(),
                    fields: Rc::new(std::cell::RefCell::new(al_fields)),
                    vtable: HashMap::new(),
                })
            }
            "heapify" => {
                let mut elements = match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                if elements.len() > 1 {
                    for i in (0..elements.len() / 2).rev() {
                        sift_down(&mut elements, i);
                    }
                }
                fields.borrow_mut().insert("_elements".to_string(), Value::Array { elements });
                Ok(Value::Void)
            }
            "abs" => {
                let elements = match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let result: Vec<Value> = elements.iter().map(|e| {
                    match e {
                        Value::Int(n) => Value::Int(n.abs()),
                        Value::Long(n) => Value::Long(n.abs()),
                        Value::Double(d) => Value::Double(d.abs()),
                        Value::Float(d) => Value::Float(d.abs()),
                        _ => e.clone(),
                    }
                }).collect();
                let mut al_fields = HashMap::new();
                al_fields.insert("_elements".to_string(), Value::Array { elements: result });
                Ok(Value::ClassInstance {
                    class_name: "ArrayList".to_string(),
                    fields: Rc::new(std::cell::RefCell::new(al_fields)),
                    vtable: HashMap::new(),
                })
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
                Ok(Value::Bool(keys.contains(&key)))
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
            "isEmpty" => {
                let keys = match fields.borrow().get("_keys") {
                    Some(Value::Array { elements }) => elements.len(),
                    _ => 0,
                };
                Ok(Value::Bool(keys == 0))
            }
            "clear" => {
                fields.borrow_mut().insert("_keys".to_string(), Value::Array { elements: vec![] });
                fields.borrow_mut().insert("_values".to_string(), Value::Array { elements: vec![] });
                Ok(Value::Void)
            }
            "getOrDefault" => {
                if arg_count < 2 {
                    return Err("HashMap.getOrDefault requires 2 arguments".to_string());
                }
                let default_val = self.pop();
                let key = self.pop();
                let keys = match fields.borrow().get("_keys") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => return Ok(default_val),
                };
                let values = match fields.borrow().get("_values") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => return Ok(default_val),
                };
                for (i, k) in keys.iter().enumerate() {
                    if *k == key {
                        return Ok(values.get(i).cloned().unwrap_or(default_val));
                    }
                }
                Ok(default_val)
            }
            "setDefault" => {
                if arg_count < 2 {
                    return Err("HashMap.setDefault requires 2 arguments".to_string());
                }
                let default_val = self.pop();
                let key = self.pop();
                let mut keys = match fields.borrow().get("_keys") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let mut values = match fields.borrow().get("_values") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                for (i, k) in keys.iter().enumerate() {
                    if *k == key {
                        return Ok(values.get(i).cloned().unwrap_or(Value::Null));
                    }
                }
                keys.push(key);
                values.push(default_val.clone());
                fields.borrow_mut().insert("_keys".to_string(), Value::Array { elements: keys });
                fields.borrow_mut().insert("_values".to_string(), Value::Array { elements: values });
                Ok(default_val)
            }
            "putIfAbsent" => {
                if arg_count < 2 {
                    return Err("HashMap.putIfAbsent requires 2 arguments".to_string());
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
                for (i, k) in keys.iter().enumerate() {
                    if *k == key {
                        return Ok(values.get(i).cloned().unwrap_or(Value::Null));
                    }
                }
                keys.push(key);
                values.push(value);
                fields.borrow_mut().insert("_keys".to_string(), Value::Array { elements: keys });
                fields.borrow_mut().insert("_values".to_string(), Value::Array { elements: values });
                Ok(Value::Null)
            }
            "replace" => {
                if arg_count < 2 {
                    return Err("HashMap.replace requires 2 arguments".to_string());
                }
                let stack_len = self.stack.len();
                let key = self.stack[stack_len - 2].clone();
                let value = self.stack[stack_len - 1].clone();
                let keys = match fields.borrow().get("_keys") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let mut values = match fields.borrow().get("_values") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                for (i, k) in keys.iter().enumerate() {
                    if *k == key {
                        let old = values.get(i).cloned().unwrap_or(Value::Null);
                        values[i] = value;
                        fields.borrow_mut().insert("_values".to_string(), Value::Array { elements: values });
                        return Ok(old);
                    }
                }
                Ok(Value::Null)
            }
            _ => Err(format!("Unknown HashMap method '{}'", method)),
        }
    }

    // -----------------------------------------------------------------------
    // Built-in Iterator methods
    // -----------------------------------------------------------------------

    pub(super) fn call_iterator_method(
        &mut self,
        fields: &Rc<RefCell<HashMap<String, Value>>>,
        method: &str,
        arg_count: u8,
    ) -> Result<Value, String> {
        let remaining = {
            let fb = fields.borrow();
            let elements = match fb.get("_elements") {
                Some(Value::Array { elements }) => elements.clone(),
                _ => vec![],
            };
            let idx = match fb.get("_index") {
                Some(Value::Int(i)) => *i as usize,
                Some(Value::Long(i)) => *i as usize,
                _ => 0,
            };
            elements.get(idx..).unwrap_or(&[]).to_vec()
        };

        match method {
            "hasNext" => Ok(Value::Bool(!remaining.is_empty())),
            "next" => {
                if remaining.is_empty() {
                    return Ok(Value::Null);
                }
                let mut fb = fields.borrow_mut();
                let idx = match fb.get("_index") {
                    Some(Value::Int(i)) => *i as usize,
                    Some(Value::Long(i)) => *i as usize,
                    _ => 0,
                };
                fb.insert("_index".to_string(), Value::Int((idx + 1) as i32));
                Ok(remaining[0].clone())
            }
            "reset" => {
                fields.borrow_mut().insert("_index".to_string(), Value::Int(0));
                Ok(Value::Void)
            }
            "forEach" | "forEachRemaining" => {
                if arg_count < 1 {
                    return Err(format!("Iterator.{} requires 1 argument (closure)", method));
                }
                let closure = self.stack.last().cloned().unwrap_or(Value::Void);
                for elem in &remaining {
                    self.call_closure_with_args(&closure, std::slice::from_ref(elem))?;
                    let _ = self.pop();
                }
                Ok(Value::Void)
            }
            "count" => Ok(Value::Int(remaining.len() as i32)),
            "collect" => {
                let mut al_fields = HashMap::new();
                al_fields.insert("_elements".to_string(), Value::Array { elements: remaining });
                Ok(Value::ClassInstance {
                    class_name: "ArrayList".to_string(),
                    fields: Rc::new(RefCell::new(al_fields)),
                    vtable: HashMap::new(),
                })
            }
            "reduce" => {
                if arg_count < 2 {
                    return Err("Iterator.reduce requires 2 arguments (init, closure)".to_string());
                }
                let stack_len = self.stack.len();
                let closure = self.stack[stack_len - 1].clone();
                let mut acc = self.stack[stack_len - 2].clone();
                for elem in &remaining {
                    self.call_closure_with_args(&closure, &[acc.clone(), elem.clone()])?;
                    acc = self.pop();
                }
                Ok(acc)
            }
            "map" => {
                if arg_count < 1 {
                    return Err("Iterator.map requires 1 argument (closure)".to_string());
                }
                let closure = self.stack.last().cloned().unwrap_or(Value::Void);
                let mut result = Vec::new();
                for elem in &remaining {
                    self.call_closure_with_args(&closure, std::slice::from_ref(elem))?;
                    result.push(self.pop());
                }
                let mut iter_fields = HashMap::new();
                iter_fields.insert("_elements".to_string(), Value::Array { elements: result });
                iter_fields.insert("_index".to_string(), Value::Int(0));
                Ok(Value::ClassInstance {
                    class_name: "Iterator".to_string(),
                    fields: Rc::new(RefCell::new(iter_fields)),
                    vtable: HashMap::new(),
                })
            }
            "filter" => {
                if arg_count < 1 {
                    return Err("Iterator.filter requires 1 argument (closure)".to_string());
                }
                let closure = self.stack.last().cloned().unwrap_or(Value::Void);
                let mut result = Vec::new();
                for elem in &remaining {
                    self.call_closure_with_args(&closure, std::slice::from_ref(elem))?;
                    let keep = match self.pop() {
                        Value::Bool(b) => b,
                        _ => false,
                    };
                    if keep {
                        result.push(elem.clone());
                    }
                }
                let mut iter_fields = HashMap::new();
                iter_fields.insert("_elements".to_string(), Value::Array { elements: result });
                iter_fields.insert("_index".to_string(), Value::Int(0));
                Ok(Value::ClassInstance {
                    class_name: "Iterator".to_string(),
                    fields: Rc::new(RefCell::new(iter_fields)),
                    vtable: HashMap::new(),
                })
            }
            "take" => {
                if arg_count < 1 {
                    return Err("Iterator.take requires 1 argument".to_string());
                }
                let n = self.stack.last().cloned().unwrap_or(Value::Int(0)).to_i64().unwrap_or(0) as usize;
                let taken: Vec<Value> = remaining.iter().take(n).cloned().collect();
                let mut iter_fields = HashMap::new();
                iter_fields.insert("_elements".to_string(), Value::Array { elements: taken });
                iter_fields.insert("_index".to_string(), Value::Int(0));
                Ok(Value::ClassInstance {
                    class_name: "Iterator".to_string(),
                    fields: Rc::new(RefCell::new(iter_fields)),
                    vtable: HashMap::new(),
                })
            }
            "skip" => {
                if arg_count < 1 {
                    return Err("Iterator.skip requires 1 argument".to_string());
                }
                let n = self.stack.last().cloned().unwrap_or(Value::Int(0)).to_i64().unwrap_or(0) as usize;
                let skipped: Vec<Value> = remaining.iter().skip(n).cloned().collect();
                let mut iter_fields = HashMap::new();
                iter_fields.insert("_elements".to_string(), Value::Array { elements: skipped });
                iter_fields.insert("_index".to_string(), Value::Int(0));
                Ok(Value::ClassInstance {
                    class_name: "Iterator".to_string(),
                    fields: Rc::new(RefCell::new(iter_fields)),
                    vtable: HashMap::new(),
                })
            }
            "chain" => {
                if arg_count < 1 {
                    return Err("Iterator.chain requires 1 argument".to_string());
                }
                let other = self.stack.last().cloned().unwrap_or(Value::Null);
                let mut combined = remaining.clone();
                if let Value::ClassInstance { fields: other_fields, .. } = other {
                    let ob = other_fields.borrow();
                    if let Some(Value::Array { elements }) = ob.get("_elements") {
                        let idx = match ob.get("_index") {
                            Some(Value::Int(i)) => *i as usize,
                            Some(Value::Long(i)) => *i as usize,
                            _ => 0,
                        };
                        combined.extend(elements.iter().skip(idx).cloned());
                    }
                }
                let mut iter_fields = HashMap::new();
                iter_fields.insert("_elements".to_string(), Value::Array { elements: combined });
                iter_fields.insert("_index".to_string(), Value::Int(0));
                Ok(Value::ClassInstance {
                    class_name: "Iterator".to_string(),
                    fields: Rc::new(RefCell::new(iter_fields)),
                    vtable: HashMap::new(),
                })
            }
            "any" => {
                if arg_count < 1 {
                    return Err("Iterator.any requires 1 argument (closure)".to_string());
                }
                let closure = self.stack.last().cloned().unwrap_or(Value::Void);
                for elem in &remaining {
                    self.call_closure_with_args(&closure, std::slice::from_ref(elem))?;
                    if let Value::Bool(true) = self.pop() {
                        return Ok(Value::Bool(true));
                    }
                }
                Ok(Value::Bool(false))
            }
            "all" => {
                if arg_count < 1 {
                    return Err("Iterator.all requires 1 argument (closure)".to_string());
                }
                let closure = self.stack.last().cloned().unwrap_or(Value::Void);
                for elem in &remaining {
                    self.call_closure_with_args(&closure, std::slice::from_ref(elem))?;
                    if let Value::Bool(false) = self.pop() {
                        return Ok(Value::Bool(false));
                    }
                }
                Ok(Value::Bool(true))
            }
            "find" => {
                if arg_count < 1 {
                    return Err("Iterator.find requires 1 argument (closure)".to_string());
                }
                let closure = self.stack.last().cloned().unwrap_or(Value::Void);
                for elem in &remaining {
                    self.call_closure_with_args(&closure, std::slice::from_ref(elem))?;
                    if let Value::Bool(true) = self.pop() {
                        return Ok(elem.clone());
                    }
                }
                Ok(Value::Null)
            }
            "position" => {
                if arg_count < 1 {
                    return Err("Iterator.position requires 1 argument (closure)".to_string());
                }
                let closure = self.stack.last().cloned().unwrap_or(Value::Void);
                for (idx, elem) in remaining.iter().enumerate() {
                    self.call_closure_with_args(&closure, std::slice::from_ref(elem))?;
                    if let Value::Bool(true) = self.pop() {
                        return Ok(Value::Int(idx as i32));
                    }
                }
                Ok(Value::Int(-1))
            }
            "max" => {
                if remaining.is_empty() {
                    return Ok(Value::Null);
                }
                let mut best = remaining[0].clone();
                for e in &remaining[1..] {
                    let better = match (&best, e) {
                        (Value::Int(a), Value::Int(b)) => b > a,
                        (Value::Long(a), Value::Long(b)) => b > a,
                        (Value::Double(a), Value::Double(b)) => b > a,
                        (Value::Float(a), Value::Float(b)) => b > a,
                        _ => false,
                    };
                    if better {
                        best = e.clone();
                    }
                }
                Ok(best)
            }
            "min" => {
                if remaining.is_empty() {
                    return Ok(Value::Null);
                }
                let mut best = remaining[0].clone();
                for e in &remaining[1..] {
                    let better = match (&best, e) {
                        (Value::Int(a), Value::Int(b)) => b < a,
                        (Value::Long(a), Value::Long(b)) => b < a,
                        (Value::Double(a), Value::Double(b)) => b < a,
                        (Value::Float(a), Value::Float(b)) => b < a,
                        _ => false,
                    };
                    if better {
                        best = e.clone();
                    }
                }
                Ok(best)
            }
            "nth" => {
                if arg_count < 1 {
                    return Err("Iterator.nth requires 1 argument".to_string());
                }
                let n = self.stack.last().cloned().unwrap_or(Value::Int(0)).to_i64().unwrap_or(0) as usize;
                if n >= remaining.len() {
                    return Ok(Value::Null);
                }
                let val = remaining[n].clone();
                let current_idx = match fields.borrow().get("_index") {
                    Some(Value::Int(i)) => *i as usize,
                    Some(Value::Long(i)) => *i as usize,
                    _ => 0,
                };
                fields.borrow_mut().insert("_index".to_string(), Value::Int((current_idx + n + 1) as i32));
                Ok(val)
            }
            "last" => {
                Ok(remaining.last().cloned().unwrap_or(Value::Null))
            }
            "sum" => {
                let mut total = 0.0;
                for e in &remaining {
                    total += e.to_f64().unwrap_or(0.0);
                }
                Ok(Value::Double(total))
            }
            "product" => {
                let mut total = 1.0;
                for e in &remaining {
                    total *= e.to_f64().unwrap_or(0.0);
                }
                Ok(Value::Double(total))
            }
            _ => Err(format!("Unknown Iterator method '{}'", method)),
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
    /// The closure's return value is left on the stack; callers should pop it
    /// if they don't need it.
    pub(super) fn call_closure_with_args(&mut self, closure: &Value, args: &[Value]) -> Result<(), String> {
        match closure {
            Value::Closure { func_idx, upvalues } => {
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
                for _ in args.len()..arity {
                    self.push(Value::Null);
                }
                if upvalues.is_empty() {
                    self.frames.push(Frame::new(fi as u16, base));
                } else {
                    self.frames.push(Frame::new_with_upvalues(fi as u16, base, upvalues.clone()));
                }
                // Execute the closure frame
                while self.frames.len() > 1 {
                    self.step()?;
                }
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

// -----------------------------------------------------------------------
// Free helper functions for ArrayList methods
// -----------------------------------------------------------------------

/// Generate all k-sized combinations of elements from the input slice.
/// Returns a Vec<Value> where each element is an ArrayList of k elements.
fn generate_combinations(elements: &[Value], k: usize) -> Vec<Value> {
    let n = elements.len();
    if k == 0 || k > n {
        return vec![];
    }
    let mut result = vec![];
    let mut indices: Vec<usize> = (0..k).collect();
    loop {
        let combo: Vec<Value> = indices.iter().map(|&i| elements[i].clone()).collect();
        let mut al_fields = HashMap::new();
        al_fields.insert("_elements".to_string(), Value::Array { elements: combo });
        result.push(Value::ClassInstance {
            class_name: "ArrayList".to_string(),
            fields: Rc::new(std::cell::RefCell::new(al_fields)),
            vtable: HashMap::new(),
        });
        // Find the rightmost index that can be incremented
        let mut i = k as isize - 1;
        while i >= 0 {
            if indices[i as usize] < n - k + (i as usize) {
                indices[i as usize] += 1;
                // Reset all indices to the right
                for j in ((i + 1) as usize)..k {
                    indices[j] = indices[j - 1] + 1;
                }
                break;
            }
            i -= 1;
        }
        if i < 0 {
            break;
        }
    }
    result
}

/// Sift down for heapify (min-heap based on to_f64 comparison).
fn sift_down(elements: &mut [Value], start: usize) {
    let n = elements.len();
    let mut root = start;
    loop {
        let mut smallest = root;
        let left = 2 * root + 1;
        let right = 2 * root + 2;
        if left < n {
            let lv = elements[left].to_f64().unwrap_or(0.0);
            let sv = elements[smallest].to_f64().unwrap_or(0.0);
            if lv < sv {
                smallest = left;
            }
        }
        if right < n {
            let rv = elements[right].to_f64().unwrap_or(0.0);
            let sv = elements[smallest].to_f64().unwrap_or(0.0);
            if rv < sv {
                smallest = right;
            }
        }
        if smallest == root {
            break;
        }
        elements.swap(root, smallest);
        root = smallest;
    }
}
