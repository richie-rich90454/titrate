// Phase 4: Expression evaluation for the Titrate interpreter
// Precision in every step – richie-rich90454, 2026

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::ast;

use super::{ControlFlow, Env, Interpreter, Value, interpreter_operator_method_name};

impl Interpreter {
    pub(super) fn eval_expr(&self, expr: &ast::Expr) -> Result<Value, String> {
        self.eval_expr_with_env(expr, &self.env.clone())
    }

    pub(super) fn eval_expr_with_env(&self, expr: &ast::Expr, env: &Rc<RefCell<Env>>) -> Result<Value, String> {
        match expr {
            ast::Expr::Literal(lit, _) => self.eval_literal(lit),

            ast::Expr::Identifier(name, _) => {
                env.borrow().get(name).ok_or_else(|| format!("Undefined variable '{}'", name))
            }

            ast::Expr::Binary(left, op, right, _) => {
                self.eval_binary(left, op, right, env)
            }

            ast::Expr::Unary(op, operand, _) => {
                self.eval_unary(op, operand, env)
            }

            ast::Expr::Call(callee, args, _) => {
                self.eval_call(callee, args, env)
            }

            ast::Expr::MemberAccess(obj, member, _) => {
                self.eval_member_access(obj, member, env)
            }

            ast::Expr::Index(obj, index, _) => {
                let obj_val = self.eval_expr_with_env(obj, env)?;
                let idx_val = self.eval_expr_with_env(index, env)?;
                match (&obj_val, &idx_val) {
                    (Value::Array { elements }, Value::Int(i)) => {
                        let idx = *i as usize;
                        if idx < elements.len() {
                            Ok(elements[idx].clone())
                        } else {
                            Err(format!("Array index out of bounds: {} (length {})", idx, elements.len()))
                        }
                    }
                    (Value::Array { elements }, Value::Long(i)) => {
                        let idx = *i as usize;
                        if idx < elements.len() {
                            Ok(elements[idx].clone())
                        } else {
                            Err(format!("Array index out of bounds: {} (length {})", idx, elements.len()))
                        }
                    }
                    (Value::Ref(mem_idx), _) => {
                        let mem_val = self.memory.borrow().read(*mem_idx)?;
                        match (&mem_val, &idx_val) {
                            (Value::Array { elements }, Value::Int(i)) => {
                                let idx = *i as usize;
                                if idx < elements.len() {
                                    Ok(elements[idx].clone())
                                } else {
                                    Err(format!("Array index out of bounds: {}", idx))
                                }
                            }
                            _ => Err(format!("Cannot index into {:?}", mem_val)),
                        }
                    }
                    _ => Err(format!("Cannot index into {:?} with {:?}", obj_val, idx_val)),
                }
            }

            ast::Expr::New(type_expr, args, _) => {
                self.eval_new(type_expr, args, env)
            }

            ast::Expr::This(_) => {
                env.borrow().get("this").ok_or_else(|| "'this' is not defined".to_string())
            }

            ast::Expr::Super(_) => {
                env.borrow().get("super").ok_or_else(|| "'super' is not defined".to_string())
            }

            ast::Expr::OwnedDeref(inner, _) => {
                let val = self.eval_expr_with_env(inner, env)?;
                match val {
                    Value::Owned(inner_val) => Ok(*inner_val),
                    Value::Moved => Err("Cannot dereference a moved value".to_string()),
                    _ => Err(format!("Cannot dereference non-Owned value: {:?}", val)),
                }
            }

            ast::Expr::RegionAlloc(_type_expr, init_expr, _) => {
                let val = self.eval_expr_with_env(init_expr, env)?;
                let idx = self.memory.borrow_mut().region_alloc(val);
                Ok(Value::Ref(idx))
            }

            ast::Expr::RefExpr(inner, ref_kind, _) => {
                let val = self.eval_expr_with_env(inner, env)?;
                let idx = self.memory.borrow_mut().alloc(val);
                match ref_kind {
                    ast::RefKind::Immutable => Ok(Value::Ref(idx)),
                    ast::RefKind::Mutable => Ok(Value::Ref(idx)),
                }
            }

            ast::Expr::UnsafeBlock(block, _) => {
                let block_env = Rc::new(RefCell::new(Env::with_parent(env.clone())));
                let cf = self.exec_block(block, block_env)?;
                match cf {
                    ControlFlow::Return(v) => Ok(v),
                    ControlFlow::None => Ok(Value::Void),
                    ControlFlow::Break => Err("Break outside of loop in unsafe block".to_string()),
                    ControlFlow::Continue => Err("Continue outside of loop in unsafe block".to_string()),
                }
            }

            ast::Expr::ErrorPropagation(inner, _) => {
                let val = self.eval_expr_with_env(inner, env)?;
                match val {
                    Value::ResultErr(e) => Ok(Value::ResultErr(e)),
                    Value::ResultOk(v) => Ok(*v),
                    other => Ok(other),
                }
            }

            ast::Expr::Cast(inner, target_type, _) => {
                let val = self.eval_expr_with_env(inner, env)?;
                self.eval_cast(&val, target_type.name())
            }

            ast::Expr::StaticCall { class_name, method, args, .. } => {
                self.eval_static_call(class_name, method, args, env)
            }

            ast::Expr::Assign(target, value, _) => {
                let val = self.eval_expr_with_env(value, env)?;
                self.eval_assign(target, val, env)
            }
            ast::Expr::Ternary { condition, then_expr, else_expr, .. } => {
                let cond_val = self.eval_expr_with_env(condition, env)?;
                match cond_val {
                    Value::Bool(true) => self.eval_expr_with_env(then_expr, env),
                    Value::Bool(false) => self.eval_expr_with_env(else_expr, env),
                    _ => Err("Ternary condition must be a boolean".to_string()),
                }
            }
            ast::Expr::Range(_, _, _) => {
                Err("Range expressions are not yet supported at runtime".to_string())
            }
            ast::Expr::RangeInclusive(_, _, _) => {
                Err("Inclusive range expressions are not yet supported at runtime".to_string())
            }
            ast::Expr::Unit(_) => Ok(Value::Void),
            ast::Expr::Tuple(elements, _) => {
                let mut values = Vec::new();
                for elem in elements {
                    values.push(self.eval_expr_with_env(elem, env)?);
                }
                Ok(Value::Tuple { elements: values })
            }
            ast::Expr::Closure {
                params,
                return_type: _,
                body,
                expr: closure_expr,
                captured_vars: _,
                span: _,
            } => {
                Ok(Value::Closure {
                    params: params.clone(),
                    body: body.clone(),
                    expr: closure_expr.clone(),
                    captured_env: env.clone(),
                })
            }
        }
    }

    pub(super) fn eval_literal(&self, lit: &ast::Literal) -> Result<Value, String> {
        match lit {
            ast::Literal::Int(v) => Ok(Value::Long(*v)),
            ast::Literal::Float(v) => Ok(Value::Double(*v)),
            ast::Literal::Bool(b) => Ok(Value::Bool(*b)),
            ast::Literal::Char(c) => Ok(Value::Char(*c)),
            ast::Literal::String(s) => Ok(Value::String(s.clone())),
            ast::Literal::Null => Ok(Value::Null),
        }
    }

    pub(super) fn eval_binary(
        &self,
        left: &ast::Expr,
        op: &ast::Operator,
        right: &ast::Expr,
        env: &Rc<RefCell<Env>>,
    ) -> Result<Value, String> {
        // Short-circuit for logical operators
        match op {
            ast::Operator::And => {
                let left_val = self.eval_expr_with_env(left, env)?;
                if !left_val.is_truthy() {
                    return Ok(Value::Bool(false));
                }
                let right_val = self.eval_expr_with_env(right, env)?;
                return Ok(Value::Bool(right_val.is_truthy()));
            }
            ast::Operator::Or => {
                let left_val = self.eval_expr_with_env(left, env)?;
                if left_val.is_truthy() {
                    return Ok(Value::Bool(true));
                }
                let right_val = self.eval_expr_with_env(right, env)?;
                return Ok(Value::Bool(right_val.is_truthy()));
            }
            _ => {}
        }

        let left_val = self.eval_expr_with_env(left, env)?;
        let right_val = self.eval_expr_with_env(right, env)?;

        // Operator overloading: if left is a ClassInstance with an operator method, call it
        if let Value::ClassInstance { vtable, class_name, .. } = &left_val {
            let method_name = interpreter_operator_method_name(op);
            if !method_name.is_empty() {
                // Check vtable first
                if vtable.contains_key(&method_name) {
                    return self.call_method(&left_val, &method_name, &[right_val]);
                }
                // Check class definition
                let class_defs = self.class_defs.borrow();
                if let Some(cd) = class_defs.get(class_name) {
                    if cd.methods.contains_key(&method_name) {
                        drop(class_defs); // release borrow before calling method
                        return self.call_method(&left_val, &method_name, &[right_val]);
                    }
                }
            }
        }

        // String concatenation
        if matches!(op, ast::Operator::Add) {
            match (&left_val, &right_val) {
                (Value::String(l), Value::String(r)) => {
                    return Ok(Value::String(format!("{}{}", l, r)));
                }
                (Value::String(l), r) => {
                    return Ok(Value::String(format!("{}{}", l, r.display_string())));
                }
                (l, Value::String(r)) => {
                    return Ok(Value::String(format!("{}{}", l.display_string(), r)));
                }
                _ => {}
            }
        }

        match op {
            ast::Operator::Add => self.arith_binop(|a, b| a.wrapping_add(b), |a, b| a + b, &left_val, &right_val),
            ast::Operator::Sub => self.arith_binop(|a, b| a.wrapping_sub(b), |a, b| a - b, &left_val, &right_val),
            ast::Operator::Mul => self.arith_binop(|a, b| a.wrapping_mul(b), |a, b| a * b, &left_val, &right_val),
            ast::Operator::Div => self.div_binop(&left_val, &right_val),
            ast::Operator::Mod => self.mod_binop(&left_val, &right_val),
            ast::Operator::Eq => Ok(Value::Bool(left_val == right_val)),
            ast::Operator::Ne => Ok(Value::Bool(left_val != right_val)),
            ast::Operator::Lt => self.cmp_binop(|a, b| a < b, |a, b| a < b, &left_val, &right_val),
            ast::Operator::Gt => self.cmp_binop(|a, b| a > b, |a, b| a > b, &left_val, &right_val),
            ast::Operator::Le => self.cmp_binop(|a, b| a <= b, |a, b| a <= b, &left_val, &right_val),
            ast::Operator::Ge => self.cmp_binop(|a, b| a >= b, |a, b| a >= b, &left_val, &right_val),
            ast::Operator::BitAnd => self.bit_binop(|a, b| a & b, |a, b| a & b, &left_val, &right_val),
            ast::Operator::BitOr => self.bit_binop(|a, b| a | b, |a, b| a | b, &left_val, &right_val),
            ast::Operator::BitXor => self.bit_binop(|a, b| a ^ b, |a, b| a ^ b, &left_val, &right_val),
            ast::Operator::BitShl => self.shift_binop(&left_val, &right_val, false),
            ast::Operator::BitShr => self.shift_binop(&left_val, &right_val, true),
            ast::Operator::BitUshr => self.unsigned_shift_binop(&left_val, &right_val),
            ast::Operator::And | ast::Operator::Or => {
                // Already handled above via short-circuit
                unreachable!()
            }
        }
    }

    pub(super) fn eval_unary(
        &self,
        op: &ast::UnOp,
        operand: &ast::Expr,
        env: &Rc<RefCell<Env>>,
    ) -> Result<Value, String> {
        let val = self.eval_expr_with_env(operand, env)?;
        match op {
            ast::UnOp::Neg => match val {
                Value::Byte(v) => Ok(Value::Byte(-v)),
                Value::Short(v) => Ok(Value::Short(-v)),
                Value::Int(v) => Ok(Value::Int(-v)),
                Value::Long(v) => Ok(Value::Long(-v)),
                Value::Vast(v) => Ok(Value::Vast(-v)),
                Value::Float(v) => Ok(Value::Float(-v)),
                Value::Double(v) => Ok(Value::Double(-v)),
                Value::Half(v) => Ok(Value::Half(-v)),
                Value::Quad(v) => Ok(Value::Quad(-v)),
                _ => Err(format!("Cannot negate {:?}", val)),
            },
            ast::UnOp::Not => match val {
                Value::Bool(v) => Ok(Value::Bool(!v)),
                _ => Ok(Value::Bool(!val.is_truthy())),
            },
            ast::UnOp::BitNot => match val {
                Value::Byte(v) => Ok(Value::Byte(!v)),
                Value::Short(v) => Ok(Value::Short(!v)),
                Value::Int(v) => Ok(Value::Int(!v)),
                Value::Long(v) => Ok(Value::Long(!v)),
                Value::Vast(v) => Ok(Value::Vast(!v)),
                Value::Uvast(v) => Ok(Value::Uvast(!v)),
                _ => Err(format!("Cannot bitwise-negate {:?}", val)),
            },
        }
    }

    pub(super) fn eval_call(
        &self,
        callee: &ast::Expr,
        args: &[ast::Expr],
        env: &Rc<RefCell<Env>>,
    ) -> Result<Value, String> {
        let arg_vals: Vec<Value> = args.iter()
            .map(|a| self.eval_expr_with_env(a, env))
            .collect::<Result<Vec<Value>, String>>()?;

        // Check for enum variant constructor or simple function call by name
        if let ast::Expr::Identifier(name, _) = callee {
            let callee_val = env.borrow().get(name);
            match callee_val {
                Some(Value::EnumVariant { enum_name, variant, field_count }) => {
                    if arg_vals.len() != field_count {
                        return Err(format!(
                            "Enum variant {}::{} expects {} fields, got {}",
                            enum_name, variant, field_count, arg_vals.len()
                        ));
                    }
                    return Ok(Value::EnumInstance {
                        enum_name,
                        variant,
                        fields: arg_vals,
                    });
                }
                Some(Value::Function(fn_decl)) => {
                    let cf = self.call_function(&fn_decl, &arg_vals)?;
                    return match cf {
                        ControlFlow::Return(v) => Ok(v),
                        ControlFlow::None => Ok(Value::Void),
                        ControlFlow::Break => Err("Break outside of loop".to_string()),
                        ControlFlow::Continue => Err("Continue outside of loop".to_string()),
                    };
                }
                Some(Value::BuiltinFn(name)) => {
                    return self.call_builtin_fn(&name, &arg_vals);
                }
                Some(Value::BuiltinObject(name)) => {
                    return self.call_builtin_object(&name, &arg_vals);
                }
                Some(Value::Closure { params, body, expr, captured_env }) => {
                    return self.call_closure(&params, &body, &expr, &captured_env, &arg_vals);
                }
                Some(other) => {
                    return Err(format!("Cannot call {:?} as a function", other));
                }
                None => {}
            }
        }

        // Check for method call: obj.method(args)
        if let ast::Expr::MemberAccess(obj_expr, method_name, _) = callee {
            let obj_val = self.eval_expr_with_env(obj_expr, env)?;
            return self.call_method(&obj_val, method_name, &arg_vals);
        }

        // General case: evaluate callee
        let callee_val = self.eval_expr_with_env(callee, env)?;
        match callee_val {
            Value::Function(fn_decl) => {
                let cf = self.call_function(&fn_decl, &arg_vals)?;
                match cf {
                    ControlFlow::Return(v) => Ok(v),
                    ControlFlow::None => Ok(Value::Void),
                    ControlFlow::Break => Err("Break outside of loop".to_string()),
                    ControlFlow::Continue => Err("Continue outside of loop".to_string()),
                }
            }
            Value::BuiltinFn(name) => self.call_builtin_fn(&name, &arg_vals),
            Value::BuiltinObject(name) => self.call_builtin_object(&name, &arg_vals),
            Value::EnumVariant { enum_name, variant, field_count } => {
                if arg_vals.len() != field_count {
                    return Err(format!(
                        "Enum variant {}::{} expects {} fields, got {}",
                        enum_name, variant, field_count, arg_vals.len()
                    ));
                }
                Ok(Value::EnumInstance {
                    enum_name,
                    variant,
                    fields: arg_vals,
                })
            }
            Value::Closure { params, body, expr, captured_env } => {
                self.call_closure(&params, &body, &expr, &captured_env, &arg_vals)
            }
            other => Err(format!("Cannot call {:?} as a function", other)),
        }
    }

    pub(super) fn eval_member_access(
        &self,
        obj: &ast::Expr,
        member: &str,
        env: &Rc<RefCell<Env>>,
    ) -> Result<Value, String> {
        let obj_val = self.eval_expr_with_env(obj, env)?;

        match &obj_val {
            Value::ClassInstance { class_name, fields, vtable } => {
                if let Some(v) = fields.borrow().get(member) {
                    return Ok(v.clone());
                }
                if vtable.contains_key(member) {
                    return Ok(Value::BuiltinFn(format!("{}_{}", class_name, member)));
                }
                let class_defs = self.class_defs.borrow();
                if let Some(class_def) = class_defs.get(class_name) {
                    if class_def.methods.contains_key(member) {
                        return Ok(Value::BuiltinFn(format!("{}_{}", class_name, member)));
                    }
                }
                Err(format!("No member '{}' on class '{}'", member, class_name))
            }
            Value::BuiltinObject(name) => {
                self.access_builtin_object(name, member)
            }
            Value::String(s) => {
                match member {
                    "length" => Ok(Value::Int(s.len() as i32)),
                    "toString" => Ok(Value::BuiltinFn("toString".to_string())),
                    _ => Err(format!("No member '{}' on string", member)),
                }
            }
            Value::Array { elements } => {
                match member {
                    "length" => Ok(Value::Int(elements.len() as i32)),
                    "size" => Ok(Value::Int(elements.len() as i32)),
                    _ => Err(format!("No member '{}' on array", member)),
                }
            }
            Value::EnumInstance { variant, fields: enum_fields, .. } => {
                match member {
                    "variant" => Ok(Value::String(variant.clone())),
                    "fields" => Ok(Value::Array { elements: enum_fields.clone() }),
                    _ => Err(format!("No member '{}' on enum instance", member)),
                }
            }
            Value::Ref(idx) => {
                let mem_val = self.memory.borrow().read(*idx)?;
                match &mem_val {
                    Value::ClassInstance { class_name, fields, vtable } => {
                        if let Some(v) = fields.borrow().get(member) {
                            return Ok(v.clone());
                        }
                        if vtable.contains_key(member) {
                            return Ok(Value::BuiltinFn(format!("{}_{}", class_name, member)));
                        }
                        let class_defs = self.class_defs.borrow();
                        if let Some(class_def) = class_defs.get(class_name) {
                            if class_def.methods.contains_key(member) {
                                return Ok(Value::BuiltinFn(format!("{}_{}", class_name, member)));
                            }
                        }
                        Err(format!("No member '{}' on class '{}'", member, class_name))
                    }
                    Value::BuiltinObject(name) => self.access_builtin_object(name, member),
                    Value::String(s) => {
                        match member {
                            "length" => Ok(Value::Int(s.len() as i32)),
                            _ => Err(format!("No member '{}' on string", member)),
                        }
                    }
                    other => Err(format!("Cannot access member '{}' on {:?}", member, other)),
                }
            }
            other => Err(format!("Cannot access member '{}' on {:?}", member, other)),
        }
    }

    pub(super) fn access_builtin_object(&self, name: &str, member: &str) -> Result<Value, String> {
        match name {
            "io" => {
                match member {
                    "println" => Ok(Value::BuiltinFn("println".to_string())),
                    _ => Err(format!("No member '{}' on io", member)),
                }
            }
            "Integer" => {
                match member {
                    "toString" => Ok(Value::BuiltinFn("toString".to_string())),
                    "parseInt" => Ok(Value::BuiltinFn("parseInt".to_string())),
                    _ => Err(format!("No member '{}' on Integer", member)),
                }
            }
            "Double" => {
                match member {
                    "toString" => Ok(Value::BuiltinFn("toString".to_string())),
                    _ => Err(format!("No member '{}' on Double", member)),
                }
            }
            "Byte" | "Short" | "Half" | "Quad" | "Vast" | "Uvast" | "String_" => {
                match member {
                    "toString" => Ok(Value::BuiltinFn("toString".to_string())),
                    _ => Err(format!("No member '{}' on {}", member, name)),
                }
            }
            _ => Err(format!("Unknown builtin object '{}'", name)),
        }
    }

    pub(super) fn eval_new(
        &self,
        type_expr: &ast::Type,
        args: &[ast::Expr],
        env: &Rc<RefCell<Env>>,
    ) -> Result<Value, String> {
        let class_name = type_expr.name();
        let arg_vals: Vec<Value> = args.iter()
            .map(|a| self.eval_expr_with_env(a, env))
            .collect::<Result<Vec<Value>, String>>()?;

        // Handle builtin classes
        match class_name {
            "ArrayList" => {
                return self.call_builtin_object("ArrayList", &arg_vals);
            }
            "HashMap" => {
                return self.call_builtin_object("HashMap", &arg_vals);
            }
            _ => {}
        }

        // User-defined class
        let class_def = self.class_defs.borrow().get(class_name).cloned();
        match class_def {
            Some(def) => {
                let mut fields = HashMap::new();
                for field_def in &def.fields {
                    let init_val = match &field_def.init {
                        Some(init_expr) => self.eval_expr_with_env(init_expr, env)?,
                        None => Value::Null,
                    };
                    fields.insert(field_def.name.clone(), init_val);
                }

                let mut vtable = HashMap::new();
                for (name, method) in &def.methods {
                    vtable.insert(name.clone(), method.clone());
                }

                let instance = Value::ClassInstance {
                    class_name: class_name.to_string(),
                    fields: Rc::new(RefCell::new(fields)),
                    vtable,
                };

                // Call constructor if present
                if let Some(ref ctor) = def.constructor {
                    let ctor_env = Rc::new(RefCell::new(Env::with_parent(self.env.clone())));
                    ctor_env.borrow_mut().set("this", instance.clone());
                    if let Some(ref parent_name) = def.parent {
                        ctor_env.borrow_mut().set("super", Value::BuiltinObject(parent_name.clone()));
                    } else {
                        // No parent class — super() is a no-op
                        ctor_env.borrow_mut().set("super", Value::BuiltinFn("super".to_string()));
                    }
                    for (i, param) in ctor.params.iter().enumerate() {
                        if i < arg_vals.len() {
                            ctor_env.borrow_mut().set(&param.name, arg_vals[i].clone());
                        }
                    }
                    let cf = self.exec_block(&ctor.body, ctor_env.clone())?;
                    let final_instance = ctor_env.borrow().get("this").ok_or_else(|| {
                        "'this' lost during constructor execution".to_string()
                    })?;
                    match cf {
                        ControlFlow::Return(_) => Ok(final_instance),
                        ControlFlow::None => Ok(final_instance),
                        ControlFlow::Break => Err("Break in constructor".to_string()),
                        ControlFlow::Continue => Err("Continue in constructor".to_string()),
                    }
                } else {
                    Ok(instance)
                }
            }
            None => Err(format!("Unknown class '{}'", class_name)),
        }
    }

    pub(super) fn eval_static_call(
        &self,
        class_name: &str,
        method: &str,
        args: &[ast::Expr],
        env: &Rc<RefCell<Env>>,
    ) -> Result<Value, String> {
        let arg_vals: Vec<Value> = args.iter()
            .map(|a| self.eval_expr_with_env(a, env))
            .collect::<Result<Vec<Value>, String>>()?;

        match (class_name, method) {
            ("io", "println") => {
                self.call_builtin_fn("println", &arg_vals)
            }
            ("Integer", "toString") => {
                self.call_builtin_fn("toString", &arg_vals)
            }
            ("Integer", "parseInt") => {
                self.call_builtin_fn("parseInt", &arg_vals)
            }
            ("Double", "toString") | ("Byte", "toString") | ("Short", "toString") |
            ("Half", "toString") | ("Quad", "toString") | ("Vast", "toString") |
            ("Uvast", "toString") | ("String_", "toString") => {
                self.call_builtin_fn("toString", &arg_vals)
            }
            _ => {
                let method_decl = {
                    let class_defs = self.class_defs.borrow();
                    class_defs.get(class_name).and_then(|cd| cd.methods.get(method).cloned())
                };
                if let Some(method_def) = method_decl {
                    let func_env = Rc::new(RefCell::new(Env::with_parent(self.env.clone())));
                    for (i, param) in method_def.params.iter().enumerate() {
                        if i < arg_vals.len() {
                            func_env.borrow_mut().set(&param.name, arg_vals[i].clone());
                        }
                    }
                    let cf = self.exec_block(&method_def.body, func_env)?;
                    match cf {
                        ControlFlow::Return(v) => Ok(v),
                        ControlFlow::None => Ok(Value::Void),
                        ControlFlow::Break => Err("Break outside of loop".to_string()),
                        ControlFlow::Continue => Err("Continue outside of loop".to_string()),
                    }
                } else {
                    let class_defs = self.class_defs.borrow();
                    if class_defs.get(class_name).is_some() {
                        Err(format!("No static method '{}' on class '{}'", method, class_name))
                    } else {
                        Err(format!("Unknown class '{}' for static call", class_name))
                    }
                }
            }
        }
    }

    pub(super) fn eval_cast(&self, val: &Value, target: &str) -> Result<Value, String> {
        match target {
            "byte" => {
                let v = val.to_i64().ok_or_else(|| format!("Cannot cast {:?} to byte", val))?;
                Ok(Value::Byte(v as i8))
            }
            "short" => {
                let v = val.to_i64().ok_or_else(|| format!("Cannot cast {:?} to short", val))?;
                Ok(Value::Short(v as i16))
            }
            "int" => {
                let v = val.to_i64().ok_or_else(|| format!("Cannot cast {:?} to int", val))?;
                Ok(Value::Int(v as i32))
            }
            "long" => {
                let v = val.to_i64().ok_or_else(|| format!("Cannot cast {:?} to long", val))?;
                Ok(Value::Long(v))
            }
            "vast" => {
                let v = val.to_i64().ok_or_else(|| format!("Cannot cast {:?} to vast", val))?;
                Ok(Value::Vast(v as i128))
            }
            "uvast" => {
                let v = val.to_u128().ok_or_else(|| format!("Cannot cast {:?} to uvast", val))?;
                Ok(Value::Uvast(v))
            }
            "float" => {
                let v = val.to_f64().ok_or_else(|| format!("Cannot cast {:?} to float", val))?;
                Ok(Value::Float(v as f32))
            }
            "double" => {
                let v = val.to_f64().ok_or_else(|| format!("Cannot cast {:?} to double", val))?;
                Ok(Value::Double(v))
            }
            "half" => {
                let v = val.to_f64().ok_or_else(|| format!("Cannot cast {:?} to half", val))?;
                Ok(Value::Half(v as f32))
            }
            "quad" => {
                let v = val.to_f64().ok_or_else(|| format!("Cannot cast {:?} to quad", val))?;
                Ok(Value::Quad(v))
            }
            "char" => {
                let v = val.to_i64().ok_or_else(|| format!("Cannot cast {:?} to char", val))?;
                Ok(Value::Char(v as u8 as char))
            }
            "string" => {
                Ok(Value::String(val.display_string()))
            }
            "bool" => {
                Ok(Value::Bool(val.is_truthy()))
            }
            _ => Err(format!("Unknown target type for cast: '{}'", target)),
        }
    }

    pub(super) fn eval_assign(
        &self,
        target: &ast::Expr,
        value: Value,
        env: &Rc<RefCell<Env>>,
    ) -> Result<Value, String> {
        match target {
            ast::Expr::Identifier(name, _) => {
                env.borrow_mut().update(name, value.clone())?;
                Ok(value)
            }
            ast::Expr::MemberAccess(obj, member, _) => {
                let obj_val = self.eval_expr_with_env(obj, env)?;
                match &obj_val {
                    Value::ClassInstance { fields, .. } => {
                        fields.borrow_mut().insert(member.clone(), value.clone());
                        Ok(value)
                    }
                    _ => Err(format!("Cannot assign to member '{}' on {:?}", member, obj_val)),
                }
            }
            ast::Expr::Index(obj, index, _) => {
                let idx_val = self.eval_expr_with_env(index, env)?;
                if let ast::Expr::Identifier(obj_name, _) = obj.as_ref() {
                    let current = env.borrow().get(obj_name).ok_or_else(|| {
                        format!("Undefined variable '{}'", obj_name)
                    })?;
                    match current {
                        Value::Array { mut elements } => {
                            match idx_val {
                                Value::Int(i) => {
                                    let idx = i as usize;
                                    if idx < elements.len() {
                                        elements[idx] = value.clone();
                                        env.borrow_mut().update(obj_name, Value::Array { elements })?;
                                        Ok(value)
                                    } else {
                                        Err(format!("Array index out of bounds: {}", idx))
                                    }
                                }
                                Value::Long(i) => {
                                    let idx = i as usize;
                                    if idx < elements.len() {
                                        elements[idx] = value.clone();
                                        env.borrow_mut().update(obj_name, Value::Array { elements })?;
                                        Ok(value)
                                    } else {
                                        Err(format!("Array index out of bounds: {}", idx))
                                    }
                                }
                                _ => Err(format!("Array index must be integer, got {:?}", idx_val)),
                            }
                        }
                        _ => Err(format!("Cannot index-assign to {:?}", current)),
                    }
                } else {
                    Err("Cannot index-assign to non-identifier".to_string())
                }
            }
            _ => Err("Invalid assignment target".to_string()),
        }
    }
}
