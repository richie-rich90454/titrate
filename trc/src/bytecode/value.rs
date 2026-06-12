// Titrate bytecode VM – runtime value type
// Precision in every step – richie-rich90454, 2026

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::rc::Rc;

// ---------------------------------------------------------------------------
// Native function signature
// ---------------------------------------------------------------------------

/// A native (host) function callable from the VM.
pub type NativeFn = fn(&[Value]) -> Result<Value, String>;

// ---------------------------------------------------------------------------
// Control flow signal
// ---------------------------------------------------------------------------

/// Signals used by bytecode instructions to alter control flow.
#[derive(Clone, Debug)]
pub enum ControlFlow {
    None,
    Break,
    Continue,
    Return(Value),
}

// ---------------------------------------------------------------------------
// Runtime value
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub enum Value {
    Void,
    Null,
    Moved,
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
    String(Rc<String>),
    ClassInstance {
        class_name: String,
        fields: Rc<RefCell<HashMap<String, Value>>>,
        vtable: HashMap<String, u16>,
    },
    EnumInstance {
        enum_name: String,
        variant: String,
        fields: Vec<Value>,
    },
    Array {
        elements: Vec<Value>,
    },
    Tuple {
        elements: Vec<Value>,
    },
    Owned(Box<Value>),
    Ref(usize),
    RawPtr(usize),
    Function(u16),
    NativeFn(u16),
    ResultOk(Box<Value>),
    ResultErr(Box<Value>),
    EnumVariant {
        enum_name: String,
        variant: String,
        field_count: usize,
    },
    FileHandle(Rc<RefCell<Option<File>>>),
    Socket(Rc<RefCell<Option<std::net::TcpStream>>>),
    Listener(Rc<RefCell<Option<std::net::TcpListener>>>),
    Closure {
        func_idx: usize,
        upvalues: Vec<Value>,
    },
}

// ---------------------------------------------------------------------------
// Value methods
// ---------------------------------------------------------------------------

impl Value {
    /// Returns whether this value is considered true in a boolean context.
    /// Mirrors the old tree-walking interpreter exactly.
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Byte(v) => *v != 0,
            Value::Short(v) => *v != 0,
            Value::Int(v) => *v != 0,
            Value::Long(v) => *v != 0,
            Value::Vast(v) => *v != 0,
            Value::Uvast(v) => *v != 0,
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

    /// Attempt to convert the value to `i64`.
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

    /// Attempt to convert the value to `u128`.
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

    /// Attempt to convert the value to `f64`.
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

    /// Produce the user-facing display string for this value.
    /// Matches the old tree-walking interpreter's `display_string` output.
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
            Value::String(s) => (**s).clone(),
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
            Value::ClassInstance {
                class_name, fields, ..
            } => {
                let items: Vec<String> = fields
                    .borrow()
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v.display_string()))
                    .collect();
                format!("{}({})", class_name, items.join(", "))
            }
            Value::EnumInstance {
                variant, fields, ..
            } => {
                if fields.is_empty() {
                    variant.clone()
                } else {
                    let items: Vec<String> =
                        fields.iter().map(|v| v.display_string()).collect();
                    format!("{}({})", variant, items.join(", "))
                }
            }
            Value::Moved => "<moved>".to_string(),
            Value::Ref(idx) => format!("ref({})", idx),
            Value::RawPtr(idx) => format!("raw_ptr({})", idx),
            Value::Owned(v) => v.display_string(),
            Value::Function(idx) => format!("<fn #{}>", idx),
            Value::NativeFn(idx) => format!("<native fn #{}>", idx),
            Value::EnumVariant { variant, .. } => format!("<variant {}>", variant),
            Value::FileHandle(_) => "<file_handle>".to_string(),
            Value::Socket(_) => "<socket>".to_string(),
            Value::Listener(_) => "<listener>".to_string(),
            Value::Closure { func_idx, .. } => format!("<closure #{}>", func_idx),
        }
    }

    /// Return the Titrate type name for this value.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Void => "void",
            Value::Null => "null",
            Value::Moved => "moved",
            Value::Bool(_) => "bool",
            Value::Byte(_) => "byte",
            Value::Short(_) => "short",
            Value::Int(_) => "int",
            Value::Long(_) => "long",
            Value::Vast(_) => "vast",
            Value::Uvast(_) => "uvast",
            Value::Float(_) => "float",
            Value::Double(_) => "double",
            Value::Half(_) => "half",
            Value::Quad(_) => "quad",
            Value::Char(_) => "char",
            Value::String(_) => "string",
            Value::ClassInstance { .. } => "class_instance",
            Value::EnumInstance { .. } => "enum_instance",
            Value::Array { .. } => "array",
            Value::Tuple { .. } => "tuple",
            Value::Owned(_) => "owned",
            Value::Ref(_) => "ref",
            Value::RawPtr(_) => "raw_ptr",
            Value::Function(_) => "function",
            Value::NativeFn(_) => "native_fn",
            Value::ResultOk(_) => "result",
            Value::ResultErr(_) => "result",
            Value::EnumVariant { .. } => "enum_variant",
            Value::FileHandle(_) => "FileHandle",
            Value::Socket(_) => "Socket",
            Value::Listener(_) => "Listener",
            Value::Closure { .. } => "closure",
        }
    }
}
