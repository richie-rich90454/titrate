// Phase 4: Statement execution for the Titrate interpreter
// Precision in every step – richie-rich90454, 2026

use std::cell::RefCell;
use std::rc::Rc;

use crate::ast;

use super::{ControlFlow, Env, Interpreter, Value};

impl Interpreter {
    pub(super) fn exec_block(&self, stmts: &[ast::Stmt], env: Rc<RefCell<Env>>) -> Result<ControlFlow, String> {
        for stmt in stmts {
            let cf = self.exec_stmt(stmt, env.clone())?;
            match cf {
                ControlFlow::None => {}
                ControlFlow::Break | ControlFlow::Continue | ControlFlow::Return(_) => return Ok(cf),
            }
        }
        Ok(ControlFlow::None)
    }

    pub(super) fn exec_stmt(&self, stmt: &ast::Stmt, env: Rc<RefCell<Env>>) -> Result<ControlFlow, String> {
        match stmt {
            ast::Stmt::Block(block) => {
                let block_env = Rc::new(RefCell::new(Env::with_parent(env)));
                self.exec_block(block, block_env)
            }
            ast::Stmt::Expr(expr) => {
                let val = self.eval_expr_with_env(expr, &env)?;
                match val {
                    Value::ResultErr(e) => Ok(ControlFlow::Return(Value::ResultErr(e))),
                    _ => Ok(ControlFlow::None),
                }
            }
            ast::Stmt::If(if_stmt) => {
                let cond = self.eval_expr_with_env(&if_stmt.condition, &env)?;
                if cond.is_truthy() {
                    let then_env = Rc::new(RefCell::new(Env::with_parent(env.clone())));
                    self.exec_block(&if_stmt.then_branch, then_env)
                } else if let Some(ref else_branch) = if_stmt.else_branch {
                    let else_env = Rc::new(RefCell::new(Env::with_parent(env)));
                    self.exec_block(else_branch, else_env)
                } else {
                    Ok(ControlFlow::None)
                }
            }
            ast::Stmt::While(while_stmt) => {
                loop {
                    let cond = self.eval_expr_with_env(&while_stmt.condition, &env)?;
                    if !cond.is_truthy() {
                        break;
                    }
                    let body_env = Rc::new(RefCell::new(Env::with_parent(env.clone())));
                    let cf = self.exec_block(&while_stmt.body, body_env)?;
                    match cf {
                        ControlFlow::Break => break,
                        ControlFlow::Continue => continue,
                        ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                        ControlFlow::None => {}
                    }
                }
                Ok(ControlFlow::None)
            }
            ast::Stmt::DoWhile(do_while_stmt) => {
                loop {
                    let body_env = Rc::new(RefCell::new(Env::with_parent(env.clone())));
                    let cf = self.exec_block(&do_while_stmt.body, body_env)?;
                    match cf {
                        ControlFlow::Break => break,
                        ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                        _ => {}
                    }
                    let cond = self.eval_expr_with_env(&do_while_stmt.condition, &env)?;
                    if !cond.is_truthy() {
                        break;
                    }
                }
                Ok(ControlFlow::None)
            }
            ast::Stmt::WhileLet(while_let_stmt) => {
                loop {
                    let val = self.eval_expr_with_env(&while_let_stmt.expr, &env)?;
                    match val {
                        Value::ResultOk(inner) => {
                            let body_env = Rc::new(RefCell::new(Env::with_parent(env.clone())));
                            body_env.borrow_mut().set(&while_let_stmt.var_name, *inner);
                            let cf = self.exec_block(&while_let_stmt.body, body_env)?;
                            match cf {
                                ControlFlow::Break => break,
                                ControlFlow::Continue => continue,
                                ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                                ControlFlow::None => {}
                            }
                        }
                        Value::ResultErr(_) | Value::Null => break,
                        _ => break,
                    }
                }
                Ok(ControlFlow::None)
            }
            ast::Stmt::For(for_stmt) => {
                let iterable = self.eval_expr_with_env(&for_stmt.iterable, &env)?;
                let elements = match &iterable {
                    Value::Array { elements } => elements.clone(),
                    Value::ClassInstance { class_name, fields, .. } if class_name == "ArrayList" => {
                        match fields.borrow().get("_elements") {
                            Some(Value::Array { elements }) => elements.clone(),
                            _ => return Err("ArrayList has no elements".to_string()),
                        }
                    }
                    _ => {
                        return Err(format!("For loop requires an iterable, got {:?}", iterable));
                    }
                };
                for elem in elements {
                    let loop_env = Rc::new(RefCell::new(Env::with_parent(env.clone())));
                    loop_env.borrow_mut().set(&for_stmt.var, elem);
                    let cf = self.exec_block(&for_stmt.body, loop_env)?;
                    match cf {
                        ControlFlow::Break => break,
                        ControlFlow::Continue => continue,
                        ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                        ControlFlow::None => {}
                    }
                }
                Ok(ControlFlow::None)
            }
            ast::Stmt::CFor(cfor_stmt) => {
                let cfor_env = Rc::new(RefCell::new(Env::with_parent(env.clone())));
                if let Some(ref init) = cfor_stmt.init {
                    self.exec_stmt(init, cfor_env.clone())?;
                }
                loop {
                    if let Some(ref cond) = cfor_stmt.condition {
                        let cond_val = self.eval_expr_with_env(cond, &cfor_env)?;
                        if !cond_val.is_truthy() {
                            break;
                        }
                    }
                    let body_env = Rc::new(RefCell::new(Env::with_parent(cfor_env.clone())));
                    let cf = self.exec_block(&cfor_stmt.body, body_env)?;
                    match cf {
                        ControlFlow::Break => break,
                        ControlFlow::Continue => {}
                        ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                        ControlFlow::None => {}
                    }
                    if let Some(ref incr) = cfor_stmt.increment {
                        self.eval_expr_with_env(incr, &cfor_env)?;
                    }
                }
                Ok(ControlFlow::None)
            }
            ast::Stmt::Return(expr) => {
                let val = match expr {
                    Some(e) => self.eval_expr_with_env(e, &env)?,
                    None => Value::Void,
                };
                Ok(ControlFlow::Return(val))
            }
            ast::Stmt::Break => Ok(ControlFlow::Break),
            ast::Stmt::Continue => Ok(ControlFlow::Continue),
            ast::Stmt::Switch(switch_stmt) => {
                let subject = self.eval_expr_with_env(&switch_stmt.expr, &env)?;
                for case in &switch_stmt.cases {
                    if self.pattern_matches(&case.pattern, &subject, &env)? {
                        let case_env = Rc::new(RefCell::new(Env::with_parent(env.clone())));
                        let cf = self.exec_block(&case.body, case_env)?;
                        return Ok(cf);
                    }
                }
                if let Some(ref default) = switch_stmt.default {
                    let default_env = Rc::new(RefCell::new(Env::with_parent(env)));
                    self.exec_block(default, default_env)
                } else {
                    Ok(ControlFlow::None)
                }
            }
            ast::Stmt::With(with_stmt) => {
                let resource = self.eval_expr_with_env(&with_stmt.resource_expr, &env)?;
                let with_env = Rc::new(RefCell::new(Env::with_parent(env.clone())));

                // Bind the resource to a variable if a name was given.
                if let Some(ref name) = with_stmt.var_name {
                    with_env.borrow_mut().set(name, resource.clone());
                }

                // Execute the body.
                let cf = self.exec_block(&with_stmt.body, with_env.clone());

                // Always call .close() on the resource (finally-like semantics).
                let _ = self.call_method(&resource, "close", &[]);

                cf
            }
            ast::Stmt::VarDecl(var_decl) => {
                let val = match &var_decl.init {
                    Some(init) => self.eval_expr_with_env(init, &env)?,
                    None => Value::Null,
                };
                env.borrow_mut().set(&var_decl.name, val);
                Ok(ControlFlow::None)
            }
            ast::Stmt::ConstDecl(const_decl) => {
                let val = match &const_decl.init {
                    Some(init) => self.eval_expr_with_env(init, &env)?,
                    None => Value::Null,
                };
                env.borrow_mut().set(&const_decl.name, val);
                Ok(ControlFlow::None)
            }
            ast::Stmt::TupleDestructure { names, expr, mutable: _, span: _ } => {
                let val = self.eval_expr_with_env(expr, &env)?;
                match &val {
                    Value::Tuple { elements } => {
                        for (i, name) in names.iter().enumerate() {
                            if i < elements.len() {
                                env.borrow_mut().set(name, elements[i].clone());
                            } else {
                                env.borrow_mut().set(name, Value::Null);
                            }
                        }
                    }
                    _ => {
                        return Err(format!(
                            "tuple destructuring requires a tuple, found {:?}",
                            val
                        ))
                    }
                }
                Ok(ControlFlow::None)
            }
        }
    }

    pub(super) fn pattern_matches(
        &self,
        pattern: &ast::Pattern,
        value: &Value,
        env: &Rc<RefCell<Env>>,
    ) -> Result<bool, String> {
        match pattern {
            ast::Pattern::Literal(lit) => {
                let lit_val = self.eval_literal(lit)?;
                Ok(*value == lit_val)
            }
            ast::Pattern::Wildcard => Ok(true),
            ast::Pattern::Constructor { name, bindings } => {
                match value {
                    Value::EnumInstance { variant, fields, .. } => {
                        if variant != name {
                            return Ok(false);
                        }
                        if bindings.len() > fields.len() {
                            return Err(format!(
                                "Pattern binding count {} exceeds variant field count {}",
                                bindings.len(), fields.len()
                            ));
                        }
                        for (i, binding_name) in bindings.iter().enumerate() {
                            env.borrow_mut().set(binding_name, fields[i].clone());
                        }
                        Ok(true)
                    }
                    Value::ResultOk(inner) => {
                        if name != "Ok" {
                            return Ok(false);
                        }
                        if let Some(binding_name) = bindings.first() {
                            env.borrow_mut().set(binding_name, (**inner).clone());
                        }
                        Ok(true)
                    }
                    Value::ResultErr(inner) => {
                        if name != "Err" {
                            return Ok(false);
                        }
                        if let Some(binding_name) = bindings.first() {
                            env.borrow_mut().set(binding_name, (**inner).clone());
                        }
                        Ok(true)
                    }
                    _ => Ok(false),
                }
            }
        }
    }
}
