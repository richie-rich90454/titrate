// Phase 4: Method dispatch for the Titrate interpreter
// Precision in every step – richie-rich90454, 2026

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::ast;

use super::{ControlFlow, Env, Interpreter, MethodDecl, Value};

impl Interpreter {
    pub(super) fn call_method(
        &self,
        instance: &Value,
        method_name: &str,
        args: &[Value],
    ) -> Result<Value, String> {
        match instance {
            Value::ClassInstance { class_name, fields, vtable } => {
                // Check vtable first
                if let Some(method) = vtable.get(method_name) {
                    return self.call_user_method(instance, method, args);
                }

                // Check class definition - clone method to release borrow
                let method_decl = {
                    let class_defs = self.class_defs.borrow();
                    class_defs.get(class_name).and_then(|cd| cd.methods.get(method_name).cloned())
                };
                if let Some(method) = method_decl {
                    return self.call_user_method(instance, &method, args);
                }

                // Handle builtin methods for ArrayList and HashMap
                match class_name.as_str() {
                    "ArrayList" => self.call_arraylist_method(fields, method_name, args),
                    "HashMap" => self.call_hashmap_method(fields, method_name, args),
                    _ => Err(format!("No method '{}' on class '{}'", method_name, class_name)),
                }
            }
            Value::BuiltinObject(name) => {
                // Handle method calls on builtin objects like io.println, Integer.toString
                match name.as_str() {
                    "io" => {
                        match method_name {
                            "println" => {
                                let output = args.first().map(|v| v.display_string()).unwrap_or_default();
                                self.output.borrow_mut().push(output);
                                Ok(Value::Void)
                            }
                            "print" => {
                                let output = args.first().map(|v| v.display_string()).unwrap_or_default();
                                self.output.borrow_mut().push(output);
                                Ok(Value::Void)
                            }
                            _ => Err(format!("No method '{}' on io", method_name)),
                        }
                    }
                    "Integer" => {
                        match method_name {
                            "toString" => self.call_builtin_fn("toString", args),
                            "parseInt" => self.call_builtin_fn("parseInt", args),
                            _ => Err(format!("No method '{}' on Integer", method_name)),
                        }
                    }
                    "Double" | "Float" | "Long" | "Byte" | "Short" | "Half" | "Quad" | "Vast" | "Uvast" | "String_" => {
                        match method_name {
                            "toString" => self.call_builtin_fn("toString", args),
                            _ => Err(format!("No method '{}' on {}", method_name, name)),
                        }
                    }
                    "Boolean" => {
                        match method_name {
                            "toString" => self.call_builtin_fn("toString", args),
                            _ => Err(format!("No method '{}' on Boolean", method_name)),
                        }
                    }
                    "Char" => {
                        match method_name {
                            "toString" => self.call_builtin_fn("toString", args),
                            _ => Err(format!("No method '{}' on Char", method_name)),
                        }
                    }
                    _ => Err(format!("Cannot call method '{}' on builtin object '{}'", method_name, name)),
                }
            }
            Value::String(s) => {
                match method_name {
                    "length" => Ok(Value::Int(s.len() as i32)),
                    "toString" => Ok(Value::String(s.clone())),
                    _ => Err(format!("No method '{}' on string", method_name)),
                }
            }
            _ => Err(format!("Cannot call method '{}' on {:?}", method_name, instance)),
        }
    }

    pub(super) fn call_user_method(
        &self,
        instance: &Value,
        method: &MethodDecl,
        args: &[Value],
    ) -> Result<Value, String> {
        if args.len() != method.params.len() {
            return Err(format!(
                "Method {} expects {} arguments, got {}",
                method.name, method.params.len(), args.len()
            ));
        }

        let method_env = Rc::new(RefCell::new(Env::with_parent(self.env.clone())));
        method_env.borrow_mut().set("this", instance.clone());
        for (i, param) in method.params.iter().enumerate() {
            method_env.borrow_mut().set(&param.name, args[i].clone());
        }

        let cf = self.exec_block(&method.body, method_env.clone())?;

        match cf {
            ControlFlow::Return(v) => Ok(v),
            ControlFlow::None => Ok(Value::Void),
            ControlFlow::Break => Err("Break outside of loop".to_string()),
            ControlFlow::Continue => Err("Continue outside of loop".to_string()),
        }
    }

    pub(super) fn call_arraylist_method(
        &self,
        fields: &Rc<RefCell<HashMap<String, Value>>>,
        method: &str,
        args: &[Value],
    ) -> Result<Value, String> {
        match method {
            "add" => {
                let item = args.first().ok_or("ArrayList.add requires 1 argument")?.clone();
                let mut elements = match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                elements.push(item);
                fields.borrow_mut().insert("_elements".to_string(), Value::Array { elements });
                Ok(Value::Void)
            }
            "get" => {
                let idx = match args.first() {
                    Some(Value::Int(i)) => *i as usize,
                    Some(Value::Long(i)) => *i as usize,
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
            "sort" => {
                Ok(Value::Void)
            }
            "length" => {
                match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => Ok(Value::Int(elements.len() as i32)),
                    _ => Ok(Value::Int(0)),
                }
            }
            "forEach" => {
                let closure = args.first().ok_or("ArrayList.forEach requires 1 argument (closure)")?.clone();
                let elements = match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                for elem in &elements {
                    self.call_closure_value(&closure, std::slice::from_ref(elem))?;
                }
                Ok(Value::Void)
            }
            "toString" => {
                let elements = match fields.borrow().get("_elements") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let items: Vec<String> = elements.iter().map(|e| e.display_string()).collect();
                Ok(Value::String(format!("[{}]", items.join(", "))))
            }
            _ => Err(format!("Unknown ArrayList method '{}'", method)),
        }
    }

    pub(super) fn call_hashmap_method(
        &self,
        fields: &Rc<RefCell<HashMap<String, Value>>>,
        method: &str,
        args: &[Value],
    ) -> Result<Value, String> {
        match method {
            "put" => {
                let key = args.first().ok_or("HashMap.put requires 2 arguments")?.clone();
                let value = args.get(1).ok_or("HashMap.put requires 2 arguments")?.clone();
                let mut keys = match fields.borrow().get("_keys") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                let mut values = match fields.borrow().get("_values") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => vec![],
                };
                // Check if key already exists
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
                let key = args.first().ok_or("HashMap.get requires 1 argument")?;
                let keys = match fields.borrow().get("_keys") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => return Ok(Value::Null),
                };
                let values = match fields.borrow().get("_values") {
                    Some(Value::Array { elements }) => elements.clone(),
                    _ => return Ok(Value::Null),
                };
                for (i, k) in keys.iter().enumerate() {
                    if k == key {
                        return Ok(values.get(i).cloned().ok_or("HashMap internal error")?);
                    }
                }
                Ok(Value::Null)
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
                    fields: Rc::new(RefCell::new(al_fields)),
                    vtable: HashMap::new(),
                })
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
                Ok(Value::String(format!("{{{}}}", items.join(", "))))
            }
            _ => Err(format!("Unknown HashMap method '{}'", method)),
        }
    }

    pub(super) fn call_closure_value(&self, closure: &Value, args: &[Value]) -> Result<Value, String> {
        match closure {
            Value::Closure { params, body, expr, captured_env } => {
                if args.len() != params.len() {
                    return Err(format!(
                        "Closure expects {} arguments, got {}",
                        params.len(), args.len()
                    ));
                }
                let closure_env = Rc::new(RefCell::new(Env::with_parent(captured_env.clone())));
                for (i, (name, _)) in params.iter().enumerate() {
                    closure_env.borrow_mut().set(name, args[i].clone());
                }
                if let Some(expr) = expr {
                    self.eval_expr_with_env(expr, &closure_env)
                } else {
                    let cf = self.exec_block(body, closure_env)?;
                    match cf {
                        ControlFlow::Return(v) => Ok(v),
                        _ => Ok(Value::Void),
                    }
                }
            }
            _ => Err("forEach: expected a closure".to_string()),
        }
    }

    pub(super) fn call_function(&self, fn_decl: &ast::FnDecl, args: &[Value]) -> Result<ControlFlow, String> {
        if args.len() != fn_decl.params.len() {
            return Err(format!(
                "Function {} expects {} arguments, got {}",
                fn_decl.name, fn_decl.params.len(), args.len()
            ));
        }

        let func_env = Rc::new(RefCell::new(Env::with_parent(self.env.clone())));
        for (i, param) in fn_decl.params.iter().enumerate() {
            func_env.borrow_mut().set(&param.name, args[i].clone());
        }

        self.exec_block(&fn_decl.body, func_env)
    }

    pub(super) fn call_closure(
        &self,
        params: &[(String, ast::Type)],
        body: &[ast::Stmt],
        expr: &Option<Box<ast::Expr>>,
        captured_env: &Rc<RefCell<Env>>,
        args: &[Value],
    ) -> Result<Value, String> {
        if args.len() != params.len() {
            return Err(format!(
                "Closure expects {} arguments, got {}",
                params.len(), args.len()
            ));
        }

        // Create a new scope with the captured environment as parent.
        let closure_env = Rc::new(RefCell::new(Env::with_parent(captured_env.clone())));
        for (i, (name, _)) in params.iter().enumerate() {
            closure_env.borrow_mut().set(name, args[i].clone());
        }

        // Execute the closure body.
        if let Some(ref e) = expr {
            // Expression body: fn(x) => x * 2
            self.eval_expr_with_env(e, &closure_env)
        } else {
            // Block body: fn(x) { return x + 1; }
            let cf = self.exec_block(body, closure_env)?;
            match cf {
                ControlFlow::Return(v) => Ok(v),
                ControlFlow::None => Ok(Value::Void),
                ControlFlow::Break => Err("Break outside of loop".to_string()),
                ControlFlow::Continue => Err("Continue outside of loop".to_string()),
            }
        }
    }

    pub(super) fn call_builtin_fn(&self, name: &str, args: &[Value]) -> Result<Value, String> {
        match name {
            "super" => {
                // No-op super() call when class has no parent
                Ok(Value::Void)
            }
            "Ok" => {
                let val = args.first().cloned().unwrap_or(Value::Void);
                Ok(Value::ResultOk(Box::new(val)))
            }
            "Err" => {
                let val = args.first().cloned().unwrap_or(Value::Void);
                Ok(Value::ResultErr(Box::new(val)))
            }
            "println" => {
                let output = match args.first() {
                    Some(v) => v.display_string(),
                    None => "void".to_string(),
                };
                self.output.borrow_mut().push(output);
                Ok(Value::Void)
            }
            "toString" => {
                match args.first() {
                    Some(v) => Ok(Value::String(v.display_string())),
                    None => Err("toString requires 1 argument".to_string()),
                }
            }
            "parseInt" => {
                match args.first() {
                    Some(Value::String(s)) => {
                        match s.trim().parse::<i64>() {
                            Ok(v) => Ok(Value::Long(v)),
                            Err(_) => Err(format!("Cannot parse '{}' as integer", s)),
                        }
                    }
                    Some(other) => Err(format!("parseInt expects a string, got {:?}", other)),
                    None => Err("parseInt requires 1 argument".to_string()),
                }
            }
            _ => Err(format!("Unknown builtin function '{}'", name)),
        }
    }

    pub(super) fn call_builtin_object(&self, name: &str, _args: &[Value]) -> Result<Value, String> {
        match name {
            "ArrayList" => {
                let mut fields = HashMap::new();
                fields.insert("_elements".to_string(), Value::Array { elements: vec![] });
                Ok(Value::ClassInstance {
                    class_name: "ArrayList".to_string(),
                    fields: Rc::new(RefCell::new(fields)),
                    vtable: HashMap::new(),
                })
            }
            "HashMap" => {
                let mut fields = HashMap::new();
                fields.insert("_keys".to_string(), Value::Array { elements: vec![] });
                fields.insert("_values".to_string(), Value::Array { elements: vec![] });
                Ok(Value::ClassInstance {
                    class_name: "HashMap".to_string(),
                    fields: Rc::new(RefCell::new(fields)),
                    vtable: HashMap::new(),
                })
            }
            _ => Err(format!("Cannot call builtin object '{}' as constructor", name)),
        }
    }
}
