// Built-in HashMap methods


use super::super::value::Value;
use super::Vm;
use std::collections::HashMap;
use std::rc::Rc;

impl Vm {
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
                values.push(value.clone());
                fields.borrow_mut().insert("_keys".to_string(), Value::Array { elements: keys });
                fields.borrow_mut().insert("_values".to_string(), Value::Array { elements: values });
                Ok(value)
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
            "containsValue" => {
                if arg_count < 1 {
                    return Err("HashMap.containsValue requires 1 argument".to_string());
                }
                let target = self.stack.last().cloned().unwrap_or(Value::Void);
                let values = match fields.borrow().get("_values") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => return Ok(Value::Bool(false)),
                };
                Ok(Value::Bool(values.contains(&target)))
            }
            "clone" => {
                let keys = match fields.borrow().get("_keys") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let values = match fields.borrow().get("_values") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let mut new_fields = HashMap::new();
                new_fields.insert("_keys".to_string(), Value::Array { elements: keys });
                new_fields.insert("_values".to_string(), Value::Array { elements: values });
                Ok(Value::ClassInstance {
                    class_name: "HashMap".to_string(),
                    fields: Rc::new(std::cell::RefCell::new(new_fields)),
                    vtable: HashMap::new(),
                })
            }
            "update" => {
                if arg_count < 1 {
                    return Err("HashMap.update requires 1 argument (other map)".to_string());
                }
                let other = self.stack.last().cloned().unwrap_or(Value::Void);
                let (other_keys, other_values) = match &other {
                    Value::ClassInstance { fields: of, .. } => {
                        let ok = match of.borrow().get("_keys") {
                            Some(Value::Array { elements }) => elements.clone(),
                            _ => vec![],
                        };
                        let ov = match of.borrow().get("_values") {
                            Some(Value::Array { elements }) => elements.clone(),
                            _ => vec![],
                        };
                        (ok, ov)
                    }
                    _ => return Ok(Value::Void),
                };
                let mut keys = match fields.borrow().get("_keys") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let mut values = match fields.borrow().get("_values") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                for (i, k) in other_keys.iter().enumerate() {
                    let v = other_values.get(i).cloned().unwrap_or(Value::Null);
                    let mut found = false;
                    for (j, ek) in keys.iter().enumerate() {
                        if *ek == *k {
                            values[j] = v.clone();
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        keys.push(k.clone());
                        values.push(v);
                    }
                }
                fields.borrow_mut().insert("_keys".to_string(), Value::Array { elements: keys });
                fields.borrow_mut().insert("_values".to_string(), Value::Array { elements: values });
                Ok(Value::Void)
            }
            "pop" => {
                if arg_count == 1 {
                    let key = self.stack.last().cloned().unwrap_or(Value::Void);
                    let mut keys = match fields.borrow().get("_keys") {
                        Some(Value::Array { elements }) => elements.clone(),
                        _ => return Ok(Value::Null),
                    };
                    let mut values = match fields.borrow().get("_values") {
                        Some(Value::Array { elements }) => elements.clone(),
                        _ => return Ok(Value::Null),
                    };
                    for (i, k) in keys.iter().enumerate() {
                        if *k == key {
                            let old_val = values.remove(i);
                            keys.remove(i);
                            fields.borrow_mut().insert("_keys".to_string(), Value::Array { elements: keys });
                            fields.borrow_mut().insert("_values".to_string(), Value::Array { elements: values });
                            return Ok(old_val);
                        }
                    }
                    Ok(Value::Null)
                } else if arg_count >= 2 {
                    let default_val = self.stack[self.stack.len() - 1].clone();
                    let key = self.stack[self.stack.len() - 2].clone();
                    let mut keys = match fields.borrow().get("_keys") {
                        Some(Value::Array { elements }) => elements.clone(),
                        _ => return Ok(default_val),
                    };
                    let mut values = match fields.borrow().get("_values") {
                        Some(Value::Array { elements }) => elements.clone(),
                        _ => return Ok(default_val),
                    };
                    for (i, k) in keys.iter().enumerate() {
                        if *k == key {
                            let old_val = values.remove(i);
                            keys.remove(i);
                            fields.borrow_mut().insert("_keys".to_string(), Value::Array { elements: keys });
                            fields.borrow_mut().insert("_values".to_string(), Value::Array { elements: values });
                            return Ok(old_val);
                        }
                    }
                    Ok(default_val)
                } else {
                    Err("HashMap.pop requires at least 1 argument".to_string())
                }
            }
            "popItem" => {
                let mut keys = match fields.borrow().get("_keys") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => return Ok(Value::Null),
                };
                if keys.is_empty() {
                    return Ok(Value::Null);
                }
                let mut values = match fields.borrow().get("_values") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => return Ok(Value::Null),
                };
                let key = keys.remove(0);
                let val = values.remove(0);
                fields.borrow_mut().insert("_keys".to_string(), Value::Array { elements: keys });
                fields.borrow_mut().insert("_values".to_string(), Value::Array { elements: values });
                Ok(Value::Tuple { elements: vec![key, val] })
            }
            "computeIfAbsent" => {
                if arg_count < 2 {
                    return Err("HashMap.computeIfAbsent requires 2 arguments".to_string());
                }
                let closure = self.stack[self.stack.len() - 1].clone();
                let key = self.stack[self.stack.len() - 2].clone();
                let keys = match fields.borrow().get("_keys") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let values = match fields.borrow().get("_values") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                for (i, k) in keys.iter().enumerate() {
                    if *k == key {
                        return Ok(values.get(i).cloned().unwrap_or(Value::Null));
                    }
                }
                self.call_closure_with_args(&closure, std::slice::from_ref(&key))?;
                let new_val = self.pop();
                let mut keys = keys;
                let mut values = values;
                keys.push(key);
                values.push(new_val.clone());
                fields.borrow_mut().insert("_keys".to_string(), Value::Array { elements: keys });
                fields.borrow_mut().insert("_values".to_string(), Value::Array { elements: values });
                Ok(new_val)
            }
            "merge" => {
                if arg_count < 3 {
                    return Err("HashMap.merge requires 3 arguments".to_string());
                }
                let closure = self.stack[self.stack.len() - 1].clone();
                let value = self.stack[self.stack.len() - 2].clone();
                let key = self.stack[self.stack.len() - 3].clone();
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
                        let old_val = values.get(i).cloned().unwrap_or(Value::Null);
                        let args = vec![old_val, value.clone()];
                        self.call_closure_with_args(&closure, &args)?;
                        let new_val = self.pop();
                        values[i] = new_val.clone();
                        fields.borrow_mut().insert("_values".to_string(), Value::Array { elements: values });
                        return Ok(new_val);
                    }
                }
                keys.push(key);
                values.push(value.clone());
                fields.borrow_mut().insert("_keys".to_string(), Value::Array { elements: keys });
                fields.borrow_mut().insert("_values".to_string(), Value::Array { elements: values });
                Ok(value)
            }
            "forEach" => {
                if arg_count < 1 {
                    return Err("HashMap.forEach requires 1 argument (closure)".to_string());
                }
                let closure = self.stack.last().cloned().unwrap_or(Value::Void);
                let keys = match fields.borrow().get("_keys") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let values = match fields.borrow().get("_values") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                for (k, v) in keys.iter().zip(values.iter()) {
                    let args = vec![k.clone(), v.clone()];
                    self.call_closure_with_args(&closure, &args)?;
                    let _ = self.pop();
                }
                Ok(Value::Void)
            }
            "map" => {
                if arg_count < 1 {
                    return Err("HashMap.map requires 1 argument (closure)".to_string());
                }
                let closure = self.stack.last().cloned().unwrap_or(Value::Void);
                let keys = match fields.borrow().get("_keys") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let values = match fields.borrow().get("_values") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let mut new_keys: Vec<Value> = Vec::new();
                let mut new_values: Vec<Value> = Vec::new();
                for (k, v) in keys.iter().zip(values.iter()) {
                    let args = vec![k.clone(), v.clone()];
                    self.call_closure_with_args(&closure, &args)?;
                    let result = self.pop();
                    if let Value::Tuple { elements } = result {
                        if elements.len() >= 2 {
                            new_keys.push(elements[0].clone());
                            new_values.push(elements[1].clone());
                        }
                    }
                }
                let mut new_fields = HashMap::new();
                new_fields.insert("_keys".to_string(), Value::Array { elements: new_keys });
                new_fields.insert("_values".to_string(), Value::Array { elements: new_values });
                Ok(Value::ClassInstance {
                    class_name: "HashMap".to_string(),
                    fields: Rc::new(std::cell::RefCell::new(new_fields)),
                    vtable: HashMap::new(),
                })
            }
            "filter" => {
                if arg_count < 1 {
                    return Err("HashMap.filter requires 1 argument (closure)".to_string());
                }
                let closure = self.stack.last().cloned().unwrap_or(Value::Void);
                let keys = match fields.borrow().get("_keys") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let values = match fields.borrow().get("_values") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let mut new_keys: Vec<Value> = Vec::new();
                let mut new_values: Vec<Value> = Vec::new();
                for (k, v) in keys.iter().zip(values.iter()) {
                    let args = vec![k.clone(), v.clone()];
                    self.call_closure_with_args(&closure, &args)?;
                    let result = self.pop();
                    if result.is_truthy() {
                        new_keys.push(k.clone());
                        new_values.push(v.clone());
                    }
                }
                let mut new_fields = HashMap::new();
                new_fields.insert("_keys".to_string(), Value::Array { elements: new_keys });
                new_fields.insert("_values".to_string(), Value::Array { elements: new_values });
                Ok(Value::ClassInstance {
                    class_name: "HashMap".to_string(),
                    fields: Rc::new(std::cell::RefCell::new(new_fields)),
                    vtable: HashMap::new(),
                })
            }
            "loadFactor" => {
                let keys = match fields.borrow().get("_keys") {
                    Some(Value::Array { elements }) => elements.len(),
                    _ => 0,
                };
                if keys == 0 {
                    Ok(Value::Double(0.0))
                } else {
                    let s = keys as f64;
                    Ok(Value::Double(s / (s * 2.0 + 1.0)))
                }
            }
            "rehash" => {
                // No-op: the VM HashMap uses flat arrays, no hash table to rehash.
                Ok(Value::Void)
            }
            "reserve" => {
                // No-op: the VM HashMap grows dynamically.
                Ok(Value::Void)
            }
            "equalRange" => {
                if arg_count < 1 {
                    return Err("HashMap.equalRange requires 1 argument".to_string());
                }
                let key = self.stack.last().cloned().unwrap_or(Value::Void);
                let keys = match fields.borrow().get("_keys") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let values = match fields.borrow().get("_values") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let mut result: Vec<Value> = Vec::new();
                for (i, k) in keys.iter().enumerate() {
                    if *k == key {
                        if let Some(v) = values.get(i) {
                            result.push(v.clone());
                        }
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
            "iterator" => {
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
            _ => Err(format!("Unknown HashMap method '{}'", method)),
        }
    }
}
