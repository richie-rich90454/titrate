// Built-in ArrayList methods


use super::super::value::Value;
use super::Vm;
use std::collections::HashMap;
use std::rc::Rc;
use super::call::{generate_combinations, sift_down};

impl Vm {
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
            "map" => {
                if arg_count < 1 {
                    return Err("ArrayList.map requires 1 argument (closure)".to_string());
                }
                let closure = self.stack.last().cloned().unwrap_or(Value::Void);
                let elements = match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let mut result = Vec::with_capacity(elements.len());
                for elem in &elements {
                    self.call_closure_with_args(&closure, std::slice::from_ref(elem))?;
                    result.push(self.pop());
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
                    // Sort strings lexicographically; other types fall back
                    // to numeric comparison via to_f64(). Mixing types yields
                    // Equal so the order is stable for homogeneous lists.
                    match (a, b) {
                        (Value::String(x), Value::String(y)) => {
                            x.as_str().cmp(y.as_str())
                        }
                        (Value::Char(x), Value::Char(y)) => x.cmp(y),
                        _ => {
                            let av = a.to_f64().unwrap_or(0.0);
                            let bv = b.to_f64().unwrap_or(0.0);
                            av.partial_cmp(&bv).unwrap_or(std::cmp::Ordering::Equal)
                        }
                    }
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
}
