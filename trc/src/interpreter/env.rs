// Phase 4: Environment for variable scoping in the Titrate interpreter
// Precision in every step – richie-rich90454, 2026

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::Value;

pub struct Env {
    pub vars: HashMap<String, Value>,
    pub parent: Option<Rc<RefCell<Env>>>,
}

impl Default for Env {
    fn default() -> Self {
        Self::new()
    }
}

impl Env {
    pub fn new() -> Self {
        Env {
            vars: HashMap::new(),
            parent: None,
        }
    }

    pub fn with_parent(parent: Rc<RefCell<Env>>) -> Self {
        Env {
            vars: HashMap::new(),
            parent: Some(parent),
        }
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        if let Some(v) = self.vars.get(name) {
            Some(v.clone())
        } else if let Some(ref p) = self.parent {
            p.borrow().get(name)
        } else {
            None
        }
    }

    pub fn set(&mut self, name: &str, value: Value) {
        self.vars.insert(name.to_string(), value);
    }

    pub fn update(&mut self, name: &str, value: Value) -> Result<(), String> {
        if self.vars.contains_key(name) {
            self.vars.insert(name.to_string(), value);
            Ok(())
        } else if let Some(ref p) = self.parent {
            p.borrow_mut().update(name, value)
        } else {
            Err(format!("Undefined variable '{}'", name))
        }
    }
}

pub enum ControlFlow {
    None,
    Break,
    Continue,
    Return(Value),
}
