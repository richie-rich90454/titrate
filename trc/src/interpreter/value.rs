// Phase 4: Runtime value types for the Titrate interpreter
// Precision in every step – richie-rich90454, 2026

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use crate::ast;

use super::Env;

#[derive(Clone)]
pub enum Value {
    Void,
    Bool(bool),
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Vast(i128),
    Uvast(u128),
    Float(f32),
    Double(f64),
    Half(f32),
    Quad(f64),
    Char(char),
    String(String),
    ClassInstance {
        class_name: String,
        fields: Rc<RefCell<HashMap<String, Value>>>,
        vtable: HashMap<String, MethodDecl>,
    },
    EnumInstance {
        enum_name: String,
        variant: String,
        fields: Vec<Value>,
    },
    Owned(Box<Value>),
    Array {
        elements: Vec<Value>,
    },
    Tuple {
        elements: Vec<Value>,
    },
    Ref(usize),
    RawPtr(usize),
    Function(Rc<ast::FnDecl>),
    ResultOk(Box<Value>),
    ResultErr(Box<Value>),
    Null,
    Moved,
    BuiltinFn(String),
    BuiltinObject(String),
    EnumVariant {
        enum_name: String,
        variant: String,
        field_count: usize,
    },
    Closure {
        params: Vec<(String, ast::Type)>,
        body: Vec<ast::Stmt>,
        expr: Option<Box<ast::Expr>>,
        captured_env: Rc<RefCell<Env>>,
    },
}

#[derive(Clone)]
pub struct MethodDecl {
    pub name: String,
    pub params: Vec<ParamDecl>,
    pub return_type: Option<String>,
    pub body: Vec<ast::Stmt>,
}

#[derive(Clone)]
pub struct ParamDecl {
    pub name: String,
    pub typ: String,
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Void => write!(f, "void"),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Byte(v) => write!(f, "{}b", v),
            Value::Short(v) => write!(f, "{}s", v),
            Value::Int(v) => write!(f, "{}", v),
            Value::Long(v) => write!(f, "{}L", v),
            Value::Vast(v) => write!(f, "{}V", v),
            Value::Uvast(v) => write!(f, "{}U", v),
            Value::Float(v) => write!(f, "{}f", v),
            Value::Double(v) => write!(f, "{}d", v),
            Value::Half(v) => write!(f, "{}h", v),
            Value::Quad(v) => write!(f, "{}q", v),
            Value::Char(c) => write!(f, "'{}'", c),
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::ClassInstance { class_name, fields, .. } => {
                write!(f, "{}(", class_name)?;
                let mut first = true;
                for (k, v) in fields.borrow().iter() {
                    if !first { write!(f, ", ")?; }
                    first = false;
                    write!(f, "{}: {:?}", k, v)?;
                }
                write!(f, ")")
            }
            Value::EnumInstance { enum_name, variant, fields } => {
                write!(f, "{}::{}", enum_name, variant)?;
                if !fields.is_empty() {
                    write!(f, "(")?;
                    for (i, v) in fields.iter().enumerate() {
                        if i > 0 { write!(f, ", ")?; }
                        write!(f, "{:?}", v)?;
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
            Value::Owned(v) => write!(f, "Owned({:?})", v),
            Value::Array { elements } => {
                write!(f, "[")?;
                for (i, v) in elements.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{:?}", v)?;
                }
                write!(f, "]")
            }
            Value::Tuple { elements } => {
                write!(f, "(")?;
                for (i, v) in elements.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{:?}", v)?;
                }
                write!(f, ")")
            }
            Value::Ref(idx) => write!(f, "ref({})", idx),
            Value::RawPtr(idx) => write!(f, "raw_ptr({})", idx),
            Value::Function(fn_decl) => write!(f, "fn {}", fn_decl.name),
            Value::ResultOk(v) => write!(f, "Ok({:?})", v),
            Value::ResultErr(v) => write!(f, "Err({:?})", v),
            Value::Null => write!(f, "null"),
            Value::Moved => write!(f, "<moved>"),
            Value::BuiltinFn(name) => write!(f, "<builtin {}>", name),
            Value::BuiltinObject(name) => write!(f, "<builtin {}>", name),
            Value::EnumVariant { enum_name, variant, .. } => {
                write!(f, "<enum_variant {}::{}>", enum_name, variant)
            }
            Value::Closure { .. } => write!(f, "<closure>"),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Void, Value::Void) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Byte(a), Value::Byte(b)) => a == b,
            (Value::Short(a), Value::Short(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Long(a), Value::Long(b)) => a == b,
            (Value::Vast(a), Value::Vast(b)) => a == b,
            (Value::Uvast(a), Value::Uvast(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a.to_bits() == b.to_bits(),
            (Value::Double(a), Value::Double(b)) => a.to_bits() == b.to_bits(),
            (Value::Half(a), Value::Half(b)) => a.to_bits() == b.to_bits(),
            (Value::Quad(a), Value::Quad(b)) => a.to_bits() == b.to_bits(),
            (Value::Char(a), Value::Char(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Null, Value::Null) => true,
            (Value::Moved, Value::Moved) => true,
            (Value::ResultOk(a), Value::ResultOk(b)) => a == b,
            (Value::ResultErr(a), Value::ResultErr(b)) => a == b,
            (Value::Ref(a), Value::Ref(b)) => a == b,
            (Value::RawPtr(a), Value::RawPtr(b)) => a == b,
            (Value::Tuple { elements: a }, Value::Tuple { elements: b }) => a == b,
            (Value::Closure { .. }, Value::Closure { .. }) => false, // closures are never equal
            _ => false,
        }
    }
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(v) => *v != 0,
            Value::Long(v) => *v != 0,
            Value::Vast(v) => *v != 0,
            Value::Uvast(v) => *v != 0,
            Value::Byte(v) => *v != 0,
            Value::Short(v) => *v != 0,
            Value::Float(v) => *v != 0.0,
            Value::Double(v) => *v != 0.0,
            Value::Half(v) => *v != 0.0,
            Value::Quad(v) => *v != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Null => false,
            Value::Void => false,
            Value::Moved => false,
            _ => true,
        }
    }

    pub fn to_i64(&self) -> Option<i64> {
        match self {
            Value::Byte(v) => Some(*v as i64),
            Value::Short(v) => Some(*v as i64),
            Value::Int(v) => Some(*v as i64),
            Value::Long(v) => Some(*v),
            Value::Vast(v) => Some(*v as i64),
            Value::Uvast(v) => Some(*v as i64),
            Value::Char(c) => Some(*c as i64),
            Value::Bool(b) => Some(if *b { 1 } else { 0 }),
            Value::Float(v) => Some(*v as i64),
            Value::Double(v) => Some(*v as i64),
            Value::Half(v) => Some(*v as i64),
            Value::Quad(v) => Some(*v as i64),
            _ => None,
        }
    }

    pub fn to_u128(&self) -> Option<u128> {
        match self {
            Value::Byte(v) => Some(*v as u128),
            Value::Short(v) => Some(*v as u128),
            Value::Int(v) => Some(*v as u128),
            Value::Long(v) => Some(*v as u128),
            Value::Vast(v) => Some(*v as u128),
            Value::Uvast(v) => Some(*v),
            Value::Char(c) => Some(*c as u128),
            Value::Float(v) => Some(*v as u128),
            Value::Double(v) => Some(*v as u128),
            Value::Half(v) => Some(*v as u128),
            Value::Quad(v) => Some(*v as u128),
            _ => None,
        }
    }

    pub fn to_f64(&self) -> Option<f64> {
        match self {
            Value::Float(v) => Some(*v as f64),
            Value::Double(v) => Some(*v),
            Value::Half(v) => Some(*v as f64),
            Value::Quad(v) => Some(*v),
            Value::Byte(v) => Some(*v as f64),
            Value::Short(v) => Some(*v as f64),
            Value::Int(v) => Some(*v as f64),
            Value::Long(v) => Some(*v as f64),
            Value::Vast(v) => Some(*v as f64),
            Value::Uvast(v) => Some(*v as f64),
            _ => None,
        }
    }

    pub fn display_string(&self) -> String {
        match self {
            Value::Void => "void".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Byte(v) => v.to_string(),
            Value::Short(v) => v.to_string(),
            Value::Int(v) => v.to_string(),
            Value::Long(v) => v.to_string(),
            Value::Vast(v) => v.to_string(),
            Value::Uvast(v) => v.to_string(),
            Value::Float(v) => v.to_string(),
            Value::Double(v) => v.to_string(),
            Value::Half(v) => v.to_string(),
            Value::Quad(v) => v.to_string(),
            Value::Char(c) => c.to_string(),
            Value::String(s) => s.clone(),
            Value::Null => "null".to_string(),
            Value::ResultOk(v) => format!("Ok({})", v.display_string()),
            Value::ResultErr(v) => format!("Err({})", v.display_string()),
            Value::Array { elements } => {
                let items: Vec<String> = elements.iter().map(|e| e.display_string()).collect();
                format!("[{}]", items.join(", "))
            }
            Value::Tuple { elements } => {
                let items: Vec<String> = elements.iter().map(|e| e.display_string()).collect();
                format!("({})", items.join(", "))
            }
            Value::ClassInstance { class_name, fields, .. } => {
                let items: Vec<String> = fields.borrow().iter()
                    .map(|(k, v)| format!("{}: {}", k, v.display_string()))
                    .collect();
                format!("{}({})", class_name, items.join(", "))
            }
            Value::EnumInstance { variant, fields, .. } => {
                if fields.is_empty() {
                    variant.clone()
                } else {
                    let items: Vec<String> = fields.iter().map(|v| v.display_string()).collect();
                    format!("{}({})", variant, items.join(", "))
                }
            }
            Value::Moved => "<moved>".to_string(),
            Value::Ref(idx) => format!("ref({})", idx),
            Value::RawPtr(idx) => format!("raw_ptr({})", idx),
            Value::Owned(v) => v.display_string(),
            Value::Function(fn_decl) => format!("<fn {}>", fn_decl.name),
            Value::BuiltinFn(name) => format!("<builtin fn {}>", name),
            Value::BuiltinObject(name) => format!("<builtin {}>", name),
            Value::EnumVariant { variant, .. } => format!("<variant {}>", variant),
            Value::Closure { .. } => "<closure>".to_string(),
        }
    }
}
