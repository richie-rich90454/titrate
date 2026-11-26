// Titrate Alpha 0.2 – bytecode virtual machine: call handling
// Precision in every step – richie-rich90454, 2026

use super::super::frame::Frame;
use super::super::value::Value;
use super::Vm;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

impl Vm {
    /// Resolve the actual function index for a call, applying the arity fixes
    /// (Variant.of default tag, overload lookup) shared by CALL and TAIL_CALL.
    /// May insert a value into the stack for the Variant.of special case.
    fn resolve_call_target(&mut self, func_idx: u16, arg_count: u8) -> Result<usize, String> {
        let mut fi = func_idx as usize;
        loop {
            if fi >= self.functions.len() {
                return Err(format!("CALL: function index {} out of range", fi));
            }

            let arity = self.functions[fi].arity;
            let fname = self.functions[fi].name.as_str();
            if (arg_count as usize) != arity {
                if fname.ends_with("Variant.of") && arg_count == 1 && arity == 2 {
                    let insert_pos = self.stack.len() - 1;
                    self.stack.insert(insert_pos, Value::String(Rc::new("variant".to_string())));
                } else {
                    let mut found: Option<u16> = None;
                    for (i, f) in self.functions.iter().enumerate() {
                        if f.name == fname && f.arity == arg_count as usize {
                            found = Some(i as u16);
                            break;
                        }
                    }
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
                        fi = alt_idx as usize;
                        continue;
                    }
                    return Err(format!(
                        "CALL: function {} expects {} args, got {}",
                        fname, arity, arg_count
                    ));
                }
            }

            if self.functions[fi].chunk.code.is_empty() {
                return Err(format!(
                    "CALL: function '{}' has no body (empty chunk)",
                    fname
                ));
            }
            return Ok(fi);
        }
    }

    pub(super) fn call_function(&mut self, func_idx: u16, arg_count: u8) -> Result<(), String> {
        if self.frames.len() >= self.max_call_depth {
            let name = self.functions.get(func_idx as usize).map(|f| f.name.as_str()).unwrap_or("?");
            return Err(format!("Stack overflow: maximum call depth exceeded calling {}", name));
        }

        let fi = self.resolve_call_target(func_idx, arg_count)?;
        let arity = self.functions[fi].arity;
        let base = self.stack.len() - arity;

        let upvalues = self.find_closure_upvalues(fi as u16);
        if let Some(uvs) = upvalues {
            self.frames.push(Frame::new_with_upvalues(fi as u16, base, uvs));
        } else {
            self.frames.push(Frame::new(fi as u16, base));
        }

        // Pre-allocate stack slots for all local variables.
        let local_count = self.functions[fi].local_count;
        let needed = base + local_count;
        while self.stack.len() < needed {
            self.stack.push(Value::Null);
        }

        Ok(())
    }

    /// Tail call: execute `func_idx` in the current frame instead of pushing a
    /// new one. The compiled arguments sit on top of the stack; they are copied
    /// into the reused frame's parameter slots, the caller's locals are dropped,
    /// and the frame's IP is reset to the callee entry. The frame count stays
    /// constant, so deep tail recursion cannot hit `max_call_depth`.
    pub(super) fn tail_call_function(&mut self, func_idx: u16, arg_count: u8) -> Result<(), String> {
        let fi = self.resolve_call_target(func_idx, arg_count)?;
        let arity = self.functions[fi].arity;
        let base = self.current_frame().base;

        // Resolve upvalues before moving/truncating the stack, matching CALL.
        let upvalues = self.find_closure_upvalues(fi as u16);

        let src = self.stack.len() - arity;
        let args = self.stack[src..src + arity].to_vec();
        self.stack.truncate(base);
        self.stack.extend(args);

        {
            let frame = self.current_frame_mut();
            frame.function_index = fi as u16;
            frame.ip = 0;
            frame.upvalues = upvalues;
        }

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
        let (func_idx, upvalues) = match &self.stack[closure_idx] {
            Value::Closure { func_idx, upvalues } => (*func_idx, upvalues.clone()),
            _ => {
                return Err(format!(
                    "CALL_CLOSURE: expected closure on stack, got {:?}",
                    self.stack[closure_idx]
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

        // One capability gate for all native dispatch; no wrapper edits needed.
        let denied = {
            let name = &self.native_name_by_idx[ni];
            !self.policy.allows_native(name)
        };
        if denied {
            let msg = format!(
                "RuntimePolicy: native '{}' is denied in untrusted mode",
                self.native_name_by_idx[ni].as_str()
            );
            self.fatal_error = Some(msg.clone());
            return Err(msg);
        }

        let arg_start = self.stack.len() - arg_count as usize;

        // Special handling for println (native index 0): capture output
        if ni == 0 {
            let output = if arg_count == 0 {
                String::new()
            } else {
                self.stack[arg_start].display_string()
            };
            self.stack.truncate(arg_start);
            self.push_output(output)?;
            self.push(Value::Void);
            return Ok(());
        }

        let args: Vec<Value> = self.stack.drain(arg_start..).collect();
        let result = self.invoke_native(native_idx, &args)?;

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


    // -----------------------------------------------------------------------
    // Overload resolution: pick the fn_idx whose param_types best match
    // the runtime argument types on the stack.
    // -----------------------------------------------------------------------

    /// Score how well a parameter type name matches a runtime Value.
    /// Higher score = better match. 0 = no match.
    fn score_param_type_match(param_type: &str, value: &Value) -> i32 {
        // Normalize param type: strip module prefix (e.g. "tt.units.Base.Meter" → "Meter")
        let simple_param = param_type.rsplit('.').next().unwrap_or(param_type);
        match value {
            Value::Double(_) => match simple_param {
                "double" => 100,
                "float" | "half" | "quad" => 50, // widening conversions
                "int" | "long" | "byte" | "short" | "vast" | "uvast" => 30,
                _ => 0,
            },
            Value::Float(_) => match simple_param {
                "float" => 100,
                "double" | "half" | "quad" => 50,
                "int" | "long" | "byte" | "short" => 30,
                _ => 0,
            },
            Value::Half(_) => match simple_param {
                "half" => 100,
                "double" | "float" | "quad" => 50,
                _ => 0,
            },
            Value::Quad(_) => match simple_param {
                "quad" => 100,
                "double" | "float" | "half" => 50,
                _ => 0,
            },
            Value::Int(_) => match simple_param {
                "int" => 100,
                "long" => 80,
                "byte" | "short" => 50,
                "double" | "float" => 30, // narrowing
                _ => 0,
            },
            Value::Long(_) => match simple_param {
                "long" => 100,
                "int" => 50,
                "vast" => 80,
                "double" | "float" => 30,
                _ => 0,
            },
            Value::Byte(_) => match simple_param {
                "byte" => 100,
                "short" | "int" | "long" => 50,
                _ => 0,
            },
            Value::Short(_) => match simple_param {
                "short" => 100,
                "int" | "long" => 50,
                _ => 0,
            },
            Value::Vast(_) => match simple_param { "vast" => 100, _ => 0 },
            Value::Uvast(_) => match simple_param { "uvast" => 100, _ => 0 },
            Value::Bool(_) => match simple_param { "bool" => 100, _ => 0 },
            Value::Char(_) => match simple_param {
                "char" => 100,
                "int" | "long" | "byte" | "short" => 30,
                _ => 0,
            },
            Value::String(_) => match simple_param { "string" | "String" => 100, _ => 0 },
            Value::ClassInstance { class_name, .. } => {
                let simple_class = class_name.rsplit('.').next().unwrap_or(class_name.as_str());
                if simple_class == simple_param {
                    100
                } else if simple_param == "Object" || simple_param == "Variant" {
                    50
                } else {
                    0
                }
            }
            Value::EnumInstance { .. } | Value::EnumVariant { .. } => {
                if simple_param == "Variant" || simple_param == "Object" {
                    50
                } else {
                    0
                }
            }
            Value::Array { .. } => match simple_param {
                "ArrayList" | "Array" | "Variant" | "Object" => 50,
                _ => 0,
            },
            Value::Tuple { .. } => match simple_param {
                "Tuple" | "Variant" | "Object" => 50,
                _ => 0,
            },
            Value::Null => match simple_param {
                "Object" | "Variant" => 50,
                _ => 0,
            },
            _ => 0,
        }
    }

    /// Given a list of candidate function indices (all with matching arity),
    /// pick the one whose param_types best match the runtime args on the stack.
    /// `arg_base` is the stack index of the first argument (i.e., receiver_idx + 1).
    pub(super) fn pick_overload(&self, candidates: &[u16], arg_base: usize, arg_count: u8) -> Option<u16> {
        if candidates.is_empty() {
            return None;
        }
        if candidates.len() == 1 {
            return Some(candidates[0]);
        }
        let mut best_idx = candidates[0];
        let mut best_score: i64 = -1;
        for &cand in candidates {
            let func = &self.functions[cand as usize];
            // Skip candidates with wrong arity (safety check).
            if func.arity != arg_count as usize {
                continue;
            }
            let mut total: i64 = 0;
            for i in 0..(arg_count as usize) {
                if arg_base + i >= self.stack.len() {
                    total = -1;
                    break;
                }
                let val = &self.stack[arg_base + i];
                let ptype = func.param_types.get(i).map(|s| s.as_str()).unwrap_or("");
                total += Self::score_param_type_match(ptype, val) as i64;
            }
            if total > best_score {
                best_score = total;
                best_idx = cand;
            }
        }
        Some(best_idx)
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
                // Record the frame count BEFORE pushing the closure frame so
                // we stop as soon as the closure returns. Using `> 1` would
                // incorrectly continue executing the *parent* frame (the one
                // whose opcode handler invoked us) until IT returns, which
                // corrupts the stack and panics with slice out-of-bounds.
                let target_frame_count = self.frames.len();
                if upvalues.is_empty() {
                    self.frames.push(Frame::new(fi as u16, base));
                } else {
                    self.frames.push(Frame::new_with_upvalues(fi as u16, base, upvalues.clone()));
                }
                // Execute the closure frame (and any frames it pushes) until
                // we're back to the frame count that existed before the call.
                while self.frames.len() > target_frame_count {
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
pub(super) fn generate_combinations(elements: &[Value], k: usize) -> Vec<Value> {
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
pub(super) fn sift_down(elements: &mut [Value], start: usize) {
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
