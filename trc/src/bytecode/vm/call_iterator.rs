// Built-in Iterator methods


use super::super::value::Value;
use super::Vm;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

impl Vm {
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
}
