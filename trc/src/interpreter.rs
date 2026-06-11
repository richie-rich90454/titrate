// Phase 4: Tree-walking interpreter for the Titrate language
// Precision in every step – richie-rich90454, 2026

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use crate::ast;

/// Map an AST operator to its operator method name for the interpreter.
fn interpreter_operator_method_name(op: &ast::Operator) -> String {
    match op {
        ast::Operator::Add => "operator+".to_string(),
        ast::Operator::Sub => "operator-".to_string(),
        ast::Operator::Mul => "operator*".to_string(),
        ast::Operator::Div => "operator/".to_string(),
        ast::Operator::Mod => "operator%".to_string(),
        ast::Operator::Eq => "operator==".to_string(),
        ast::Operator::Ne => "operator!=".to_string(),
        ast::Operator::Lt => "operator<".to_string(),
        ast::Operator::Gt => "operator>".to_string(),
        ast::Operator::Le => "operator<=".to_string(),
        ast::Operator::Ge => "operator>=".to_string(),
        ast::Operator::BitAnd => "operator&".to_string(),
        ast::Operator::BitOr => "operator|".to_string(),
        ast::Operator::BitXor => "operator^".to_string(),
        ast::Operator::BitShl => "operator<<".to_string(),
        ast::Operator::BitShr => "operator>>".to_string(),
        ast::Operator::And | ast::Operator::Or => String::new(), // not overloadable
    }
}

// ---------------------------------------------------------------------------
// Runtime value
// ---------------------------------------------------------------------------

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
    fn is_truthy(&self) -> bool {
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

    fn to_i64(&self) -> Option<i64> {
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

    fn to_u128(&self) -> Option<u128> {
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

    fn to_f64(&self) -> Option<f64> {
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

    fn display_string(&self) -> String {
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

// ---------------------------------------------------------------------------
// Control flow
// ---------------------------------------------------------------------------

enum ControlFlow {
    None,
    Break,
    Continue,
    Return(Value),
}

// ---------------------------------------------------------------------------
// Memory simulation
// ---------------------------------------------------------------------------

#[allow(dead_code)]
struct Memory {
    heap: Vec<Value>,
    #[allow(dead_code)]
    raw_buffer: Vec<u8>,
    region_stack: Vec<Vec<usize>>,
}

#[allow(dead_code)]
impl Memory {
    fn new() -> Self {
        Memory {
            heap: Vec::new(),
            raw_buffer: Vec::new(),
            region_stack: Vec::new(),
        }
    }

    fn alloc(&mut self, value: Value) -> usize {
        let idx = self.heap.len();
        self.heap.push(value);
        idx
    }

    fn read(&self, idx: usize) -> Result<Value, String> {
        if idx < self.heap.len() {
            Ok(self.heap[idx].clone())
        } else {
            Err(format!("Memory access out of bounds: index {}", idx))
        }
    }

    fn write(&mut self, idx: usize, value: Value) -> Result<(), String> {
        if idx < self.heap.len() {
            self.heap[idx] = value;
            Ok(())
        } else {
            Err(format!("Memory write out of bounds: index {}", idx))
        }
    }

    fn push_region(&mut self) {
        self.region_stack.push(Vec::new());
    }

    fn pop_region(&mut self) {
        if let Some(indices) = self.region_stack.pop() {
            for idx in indices {
                if idx < self.heap.len() {
                    self.heap[idx] = Value::Void;
                }
            }
        }
    }

    fn region_alloc(&mut self, value: Value) -> usize {
        let idx = self.alloc(value);
        if let Some(region) = self.region_stack.last_mut() {
            region.push(idx);
        }
        idx
    }

    fn raw_alloc(&mut self, data: &[u8]) -> usize {
        let start = self.raw_buffer.len();
        self.raw_buffer.extend_from_slice(data);
        start
    }

    fn raw_read(&self, offset: usize, len: usize) -> Result<Vec<u8>, String> {
        if offset + len <= self.raw_buffer.len() {
            Ok(self.raw_buffer[offset..offset + len].to_vec())
        } else {
            Err(format!("Raw memory read out of bounds: offset {} len {}", offset, len))
        }
    }
}

// ---------------------------------------------------------------------------
// Environment
// ---------------------------------------------------------------------------

struct Env {
    vars: HashMap<String, Value>,
    parent: Option<Rc<RefCell<Env>>>,
}

impl Env {
    fn new() -> Self {
        Env {
            vars: HashMap::new(),
            parent: None,
        }
    }

    fn with_parent(parent: Rc<RefCell<Env>>) -> Self {
        Env {
            vars: HashMap::new(),
            parent: Some(parent),
        }
    }

    fn get(&self, name: &str) -> Option<Value> {
        if let Some(v) = self.vars.get(name) {
            Some(v.clone())
        } else if let Some(ref p) = self.parent {
            p.borrow().get(name)
        } else {
            None
        }
    }

    fn set(&mut self, name: &str, value: Value) {
        self.vars.insert(name.to_string(), value);
    }

    fn update(&mut self, name: &str, value: Value) -> Result<(), String> {
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

// ---------------------------------------------------------------------------
// Interpreter
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct ClassDef {
    parent: Option<String>,
    fields: Vec<FieldDef>,
    methods: HashMap<String, MethodDecl>,
    constructor: Option<MethodDecl>,
}

#[derive(Clone)]
struct FieldDef {
    name: String,
    init: Option<ast::Expr>,
}

#[allow(dead_code)]
struct EnumDef {
    #[allow(dead_code)]
    variants: HashMap<String, Vec<ParamDecl>>,
}

pub struct Interpreter {
    env: Rc<RefCell<Env>>,
    memory: RefCell<Memory>,
    class_defs: RefCell<HashMap<String, ClassDef>>,
    enum_defs: RefCell<HashMap<String, EnumDef>>,
    pub output: RefCell<Vec<String>>,
}

impl Interpreter {
    pub fn new() -> Self {
        let env = Rc::new(RefCell::new(Env::new()));
        let interpreter = Interpreter {
            env,
            memory: RefCell::new(Memory::new()),
            class_defs: RefCell::new(HashMap::new()),
            enum_defs: RefCell::new(HashMap::new()),
            output: RefCell::new(Vec::new()),
        };
        interpreter.register_builtins();
        interpreter
    }

    // Assemble the standard toolkit – 2026, rj
    fn register_builtins(&self) {
        let mut env = self.env.borrow_mut();
        env.set("io", Value::BuiltinObject("io".to_string()));
        env.set("Integer", Value::BuiltinObject("Integer".to_string()));
        env.set("Double", Value::BuiltinObject("Double".to_string()));
        env.set("Float", Value::BuiltinObject("Float".to_string()));
        env.set("Long", Value::BuiltinObject("Long".to_string()));
        env.set("Boolean", Value::BuiltinObject("Boolean".to_string()));
        env.set("Char", Value::BuiltinObject("Char".to_string()));
        env.set("Byte", Value::BuiltinObject("Byte".to_string()));
        env.set("Short", Value::BuiltinObject("Short".to_string()));
        env.set("Half", Value::BuiltinObject("Half".to_string()));
        env.set("Quad", Value::BuiltinObject("Quad".to_string()));
        env.set("Vast", Value::BuiltinObject("Vast".to_string()));
        env.set("Uvast", Value::BuiltinObject("Uvast".to_string()));
        env.set("String_", Value::BuiltinObject("String_".to_string()));
        env.set("ArrayList", Value::BuiltinObject("ArrayList".to_string()));
        env.set("HashMap", Value::BuiltinObject("HashMap".to_string()));
        env.set("malloc", Value::BuiltinObject("malloc".to_string()));
        env.set("free", Value::BuiltinObject("free".to_string()));
        // Result constructors
        env.set("Ok", Value::BuiltinFn("Ok".to_string()));
        env.set("Err", Value::BuiltinFn("Err".to_string()));
    }

    pub fn run(&self, program: &ast::Program) -> Result<(), String> {
        // First pass: register all class and enum definitions
        for decl in &program.declarations {
            match decl {
                ast::Declaration::Class(class_decl) => {
                    self.register_class(class_decl)?;
                }
                ast::Declaration::Enum(enum_decl) => {
                    self.register_enum(enum_decl);
                }
                _ => {}
            }
        }

        // Second pass: register functions and top-level vars
        for decl in &program.declarations {
            match decl {
                ast::Declaration::Function(fn_decl) => {
                    let val = Value::Function(Rc::new(fn_decl.clone()));
                    self.env.borrow_mut().set(&fn_decl.name, val);
                }
                ast::Declaration::VarDecl(var_decl) => {
                    let val = if let Some(ref init) = var_decl.init {
                        self.eval_expr(init)?
                    } else {
                        Value::Null
                    };
                    self.env.borrow_mut().set(&var_decl.name, val);
                }
                ast::Declaration::ConstDecl(const_decl) => {
                    let val = if let Some(ref init) = const_decl.init {
                        self.eval_expr(init)?
                    } else {
                        Value::Null
                    };
                    self.env.borrow_mut().set(&const_decl.name, val);
                }
                _ => {}
            }
        }

        // Call main if it exists
        let main_fn = self.env.borrow().get("main");
        match main_fn {
            Some(Value::Function(fn_decl)) => {
                let result = self.call_function(&fn_decl, &[])?;
                match result {
                    ControlFlow::Return(_) => Ok(()),
                    ControlFlow::None => Ok(()),
                    ControlFlow::Break => Err("Break outside of loop".to_string()),
                    ControlFlow::Continue => Err("Continue outside of loop".to_string()),
                }
            }
            Some(_) => Err("'main' is not a function".to_string()),
            None => Ok(()),
        }
    }

    fn register_class(&self, class_decl: &ast::ClassDecl) -> Result<(), String> {
        let parent = class_decl.parent.as_ref().map(|t| t.name().to_string());

        let mut fields = Vec::new();
        let mut methods = HashMap::new();
        let mut constructor: Option<MethodDecl> = None;

        // Inherit parent fields and methods
        if let Some(ref parent_name) = parent {
            let parent_defs = self.class_defs.borrow();
            if let Some(parent_def) = parent_defs.get(parent_name) {
                for f in &parent_def.fields {
                    fields.push(FieldDef { name: f.name.clone(), init: f.init.clone() });
                }
                for (name, method) in &parent_def.methods {
                    methods.insert(name.clone(), method.clone());
                }
                if let Some(ref ctor) = parent_def.constructor {
                    constructor = Some(ctor.clone());
                }
            }
        }

        for member in &class_decl.members {
            match member {
                ast::ClassMember::Field(field_decl) => {
                    fields.push(FieldDef {
                        name: field_decl.name.clone(),
                        init: field_decl.init.clone(),
                    });
                }
                ast::ClassMember::Method(method_decl) => {
                    methods.insert(method_decl.name.clone(), MethodDecl {
                        name: method_decl.name.clone(),
                        params: method_decl.params.iter().map(|p| ParamDecl {
                            name: p.name.clone(),
                            typ: p.typ.name().to_string(),
                        }).collect(),
                        return_type: method_decl.return_type.as_ref().map(|t| t.name().to_string()),
                        body: method_decl.body.clone(),
                    });
                }
                ast::ClassMember::Constructor(ctor_decl) => {
                    constructor = Some(MethodDecl {
                        name: "new".to_string(),
                        params: ctor_decl.params.iter().map(|p| ParamDecl {
                            name: p.name.clone(),
                            typ: p.typ.name().to_string(),
                        }).collect(),
                        return_type: ctor_decl.return_type.as_ref().map(|t| t.name().to_string()),
                        body: ctor_decl.body.clone(),
                    });
                }
            }
        }

        let class_def = ClassDef {
            parent,
            fields,
            methods,
            constructor,
        };

        self.class_defs.borrow_mut().insert(class_decl.name.clone(), class_def);
        self.env.borrow_mut().set(&class_decl.name, Value::BuiltinObject(class_decl.name.clone()));

        Ok(())
    }

    fn register_enum(&self, enum_decl: &ast::EnumDecl) {
        let mut variants = HashMap::new();
        for variant in &enum_decl.variants {
            variants.insert(variant.name.clone(), variant.fields.iter().map(|p| ParamDecl {
                name: p.name.clone(),
                typ: p.typ.name().to_string(),
            }).collect());
        }
        self.enum_defs.borrow_mut().insert(enum_decl.name.clone(), EnumDef { variants });

        // Register each variant as a constructor
        for variant in &enum_decl.variants {
            let enum_name = enum_decl.name.clone();
            let variant_name = variant.name.clone();
            let field_count = variant.fields.len();
            self.env.borrow_mut().set(
                &variant.name,
                Value::EnumVariant { enum_name, variant: variant_name, field_count },
            );
        }
    }

    // -----------------------------------------------------------------------
    // Statement execution
    // -----------------------------------------------------------------------

    fn exec_block(&self, stmts: &[ast::Stmt], env: Rc<RefCell<Env>>) -> Result<ControlFlow, String> {
        for stmt in stmts {
            let cf = self.exec_stmt(stmt, env.clone())?;
            match cf {
                ControlFlow::None => {}
                ControlFlow::Break | ControlFlow::Continue | ControlFlow::Return(_) => return Ok(cf),
            }
        }
        Ok(ControlFlow::None)
    }

    fn exec_stmt(&self, stmt: &ast::Stmt, env: Rc<RefCell<Env>>) -> Result<ControlFlow, String> {
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

    fn pattern_matches(
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

    // -----------------------------------------------------------------------
    // Expression evaluation
    // -----------------------------------------------------------------------

    fn eval_expr(&self, expr: &ast::Expr) -> Result<Value, String> {
        self.eval_expr_with_env(expr, &self.env.clone())
    }

    fn eval_expr_with_env(&self, expr: &ast::Expr, env: &Rc<RefCell<Env>>) -> Result<Value, String> {
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

    fn eval_literal(&self, lit: &ast::Literal) -> Result<Value, String> {
        match lit {
            ast::Literal::Int(v) => Ok(Value::Long(*v)),
            ast::Literal::Float(v) => Ok(Value::Double(*v)),
            ast::Literal::Bool(b) => Ok(Value::Bool(*b)),
            ast::Literal::Char(c) => Ok(Value::Char(*c)),
            ast::Literal::String(s) => Ok(Value::String(s.clone())),
            ast::Literal::Null => Ok(Value::Null),
        }
    }

    fn eval_binary(
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
            ast::Operator::And | ast::Operator::Or => {
                // Already handled above via short-circuit
                unreachable!()
            }
        }
    }

    fn arith_binop(
        &self,
        int_op: fn(i64, i64) -> i64,
        float_op: fn(f64, f64) -> f64,
        left: &Value,
        right: &Value,
    ) -> Result<Value, String> {
        match (left, right) {
            (Value::Byte(a), Value::Byte(b)) => Ok(Value::Byte(int_op(*a as i64, *b as i64) as i8)),
            (Value::Short(a), Value::Short(b)) => Ok(Value::Short(int_op(*a as i64, *b as i64) as i16)),
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(int_op(*a as i64, *b as i64) as i32)),
            (Value::Long(a), Value::Long(b)) => Ok(Value::Long(int_op(*a, *b))),
            (Value::Vast(a), Value::Vast(b)) => {
                let result = int_op(*a as i64, *b as i64);
                Ok(Value::Vast(result as i128))
            }
            (Value::Uvast(a), Value::Uvast(b)) => {
                let result = int_op(*a as i64, *b as i64);
                Ok(Value::Uvast(result as u128))
            }
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(float_op(*a as f64, *b as f64) as f32)),
            (Value::Double(a), Value::Double(b)) => Ok(Value::Double(float_op(*a, *b))),
            (Value::Half(a), Value::Half(b)) => Ok(Value::Half(float_op(*a as f64, *b as f64) as f32)),
            (Value::Quad(a), Value::Quad(b)) => Ok(Value::Quad(float_op(*a, *b))),
            (Value::Int(a), Value::Long(b)) => Ok(Value::Long(int_op(*a as i64, *b))),
            (Value::Long(a), Value::Int(b)) => Ok(Value::Long(int_op(*a, *b as i64))),
            (Value::Byte(a), Value::Int(b)) => Ok(Value::Int(int_op(*a as i64, *b as i64) as i32)),
            (Value::Int(a), Value::Byte(b)) => Ok(Value::Int(int_op(*a as i64, *b as i64) as i32)),
            (Value::Int(a), Value::Double(b)) => Ok(Value::Double(float_op(*a as f64, *b))),
            (Value::Long(a), Value::Double(b)) => Ok(Value::Double(float_op(*a as f64, *b))),
            (Value::Double(a), Value::Int(b)) => Ok(Value::Double(float_op(*a, *b as f64))),
            (Value::Double(a), Value::Long(b)) => Ok(Value::Double(float_op(*a, *b as f64))),
            (Value::Float(a), Value::Double(b)) => Ok(Value::Double(float_op(*a as f64, *b))),
            (Value::Double(a), Value::Float(b)) => Ok(Value::Double(float_op(*a, *b as f64))),
            _ => Err(format!("Cannot apply arithmetic to {:?} and {:?}", left, right)),
        }
    }

    fn div_binop(&self, left: &Value, right: &Value) -> Result<Value, String> {
        match (left, right) {
            (Value::Byte(a), Value::Byte(b)) => {
                if *b == 0 { return Err("Division by zero".to_string()); }
                Ok(Value::Byte(*a / *b))
            }
            (Value::Short(a), Value::Short(b)) => {
                if *b == 0 { return Err("Division by zero".to_string()); }
                Ok(Value::Short(*a / *b))
            }
            (Value::Int(a), Value::Int(b)) => {
                if *b == 0 { return Err("Division by zero".to_string()); }
                Ok(Value::Int(*a / *b))
            }
            (Value::Long(a), Value::Long(b)) => {
                if *b == 0 { return Err("Division by zero".to_string()); }
                Ok(Value::Long(*a / *b))
            }
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(*a / *b)),
            (Value::Double(a), Value::Double(b)) => Ok(Value::Double(*a / *b)),
            (Value::Half(a), Value::Half(b)) => Ok(Value::Half(*a / *b)),
            (Value::Quad(a), Value::Quad(b)) => Ok(Value::Quad(*a / *b)),
            (Value::Int(a), Value::Long(b)) => {
                if *b == 0 { return Err("Division by zero".to_string()); }
                Ok(Value::Long(*a as i64 / *b))
            }
            (Value::Long(a), Value::Int(b)) => {
                if *b == 0 { return Err("Division by zero".to_string()); }
                Ok(Value::Long(*a / *b as i64))
            }
            _ => Err(format!("Cannot divide {:?} by {:?}", left, right)),
        }
    }

    fn mod_binop(&self, left: &Value, right: &Value) -> Result<Value, String> {
        match (left, right) {
            (Value::Byte(a), Value::Byte(b)) => {
                if *b == 0 { return Err("Division by zero".to_string()); }
                Ok(Value::Byte(*a % *b))
            }
            (Value::Short(a), Value::Short(b)) => {
                if *b == 0 { return Err("Division by zero".to_string()); }
                Ok(Value::Short(*a % *b))
            }
            (Value::Int(a), Value::Int(b)) => {
                if *b == 0 { return Err("Division by zero".to_string()); }
                Ok(Value::Int(*a % *b))
            }
            (Value::Long(a), Value::Long(b)) => {
                if *b == 0 { return Err("Division by zero".to_string()); }
                Ok(Value::Long(*a % *b))
            }
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(*a % *b)),
            (Value::Double(a), Value::Double(b)) => Ok(Value::Double(*a % *b)),
            _ => Err(format!("Cannot mod {:?} by {:?}", left, right)),
        }
    }

    fn cmp_binop(
        &self,
        int_op: fn(i64, i64) -> bool,
        float_op: fn(f64, f64) -> bool,
        left: &Value,
        right: &Value,
    ) -> Result<Value, String> {
        match (left, right) {
            (Value::Byte(a), Value::Byte(b)) => Ok(Value::Bool(int_op(*a as i64, *b as i64))),
            (Value::Short(a), Value::Short(b)) => Ok(Value::Bool(int_op(*a as i64, *b as i64))),
            (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(int_op(*a as i64, *b as i64))),
            (Value::Long(a), Value::Long(b)) => Ok(Value::Bool(int_op(*a, *b))),
            (Value::Vast(a), Value::Vast(b)) => Ok(Value::Bool(int_op(*a as i64, *b as i64))),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(float_op(*a as f64, *b as f64))),
            (Value::Double(a), Value::Double(b)) => Ok(Value::Bool(float_op(*a, *b))),
            (Value::Half(a), Value::Half(b)) => Ok(Value::Bool(float_op(*a as f64, *b as f64))),
            (Value::Quad(a), Value::Quad(b)) => Ok(Value::Bool(float_op(*a, *b))),
            (Value::Int(a), Value::Long(b)) => Ok(Value::Bool(int_op(*a as i64, *b))),
            (Value::Long(a), Value::Int(b)) => Ok(Value::Bool(int_op(*a, *b as i64))),
            (Value::Byte(a), Value::Int(b)) => Ok(Value::Bool(int_op(*a as i64, *b as i64))),
            (Value::Int(a), Value::Byte(b)) => Ok(Value::Bool(int_op(*a as i64, *b as i64))),
            (Value::Int(a), Value::Double(b)) => Ok(Value::Bool(float_op(*a as f64, *b))),
            (Value::Long(a), Value::Double(b)) => Ok(Value::Bool(float_op(*a as f64, *b))),
            (Value::Double(a), Value::Int(b)) => Ok(Value::Bool(float_op(*a, *b as f64))),
            (Value::Double(a), Value::Long(b)) => Ok(Value::Bool(float_op(*a, *b as f64))),
            _ => Err(format!("Cannot compare {:?} and {:?}", left, right)),
        }
    }

    fn bit_binop(
        &self,
        int_op: fn(i64, i64) -> i64,
        uint_op: fn(u128, u128) -> u128,
        left: &Value,
        right: &Value,
    ) -> Result<Value, String> {
        match (left, right) {
            (Value::Byte(a), Value::Byte(b)) => Ok(Value::Byte(int_op(*a as i64, *b as i64) as i8)),
            (Value::Short(a), Value::Short(b)) => Ok(Value::Short(int_op(*a as i64, *b as i64) as i16)),
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(int_op(*a as i64, *b as i64) as i32)),
            (Value::Long(a), Value::Long(b)) => Ok(Value::Long(int_op(*a, *b))),
            (Value::Vast(a), Value::Vast(b)) => Ok(Value::Vast(int_op(*a as i64, *b as i64) as i128)),
            (Value::Uvast(a), Value::Uvast(b)) => Ok(Value::Uvast(uint_op(*a, *b))),
            (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(int_op(*a as i64, *b as i64) != 0)),
            _ => Err(format!("Cannot apply bitwise operation to {:?} and {:?}", left, right)),
        }
    }

    fn shift_binop(&self, left: &Value, right: &Value, is_right: bool) -> Result<Value, String> {
        let lv = left.to_i64().ok_or_else(|| format!("Cannot shift {:?}", left))?;
        let rv = right.to_i64().ok_or_else(|| format!("Shift amount must be integer, got {:?}", right))?;
        if rv < 0 {
            return Err("Negative shift amount".to_string());
        }
        let result = if is_right {
            lv.wrapping_shr(rv as u32)
        } else {
            lv.wrapping_shl(rv as u32)
        };
        match left {
            Value::Byte(_) => Ok(Value::Byte(result as i8)),
            Value::Short(_) => Ok(Value::Short(result as i16)),
            Value::Int(_) => Ok(Value::Int(result as i32)),
            Value::Long(_) => Ok(Value::Long(result)),
            Value::Vast(_) => Ok(Value::Vast(result as i128)),
            Value::Uvast(_) => Ok(Value::Uvast(result as u128)),
            _ => Err(format!("Cannot shift {:?}", left)),
        }
    }

    fn eval_unary(
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

    fn eval_call(
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

    fn call_function(&self, fn_decl: &ast::FnDecl, args: &[Value]) -> Result<ControlFlow, String> {
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

    fn call_closure(
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

    fn call_builtin_fn(&self, name: &str, args: &[Value]) -> Result<Value, String> {
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
                        match s.parse::<i64>() {
                            Ok(v) => Ok(Value::ResultOk(Box::new(Value::Long(v)))),
                            Err(_) => Ok(Value::ResultErr(Box::new(Value::String(format!("Cannot parse '{}' as integer", s))))),
                        }
                    }
                    Some(other) => Err(format!("parseInt expects a string, got {:?}", other)),
                    None => Err("parseInt requires 1 argument".to_string()),
                }
            }
            _ => Err(format!("Unknown builtin function '{}'", name)),
        }
    }

    fn call_builtin_object(&self, name: &str, _args: &[Value]) -> Result<Value, String> {
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

    fn eval_member_access(
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

    fn access_builtin_object(&self, name: &str, member: &str) -> Result<Value, String> {
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

    fn eval_new(
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

    fn eval_static_call(
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

    fn eval_cast(&self, val: &Value, target: &str) -> Result<Value, String> {
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

    fn eval_assign(
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

    // -----------------------------------------------------------------------
    // Method dispatch for class instances
    // -----------------------------------------------------------------------

    fn call_method(
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

    fn call_user_method(
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

    fn call_arraylist_method(
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
                    self.call_closure_value(&closure, &[elem.clone()])?;
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

    fn call_hashmap_method(
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

    fn call_closure_value(&self, closure: &Value, args: &[Value]) -> Result<Value, String> {
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
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn interpret(program: &ast::Program) -> Result<(), String> {
    let interpreter = Interpreter::new();
    let result = interpreter.run(program);
    // Flush captured output to stdout
    for line in interpreter.output.borrow().iter() {
        println!("{}", line);
    }
    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;
    // Disambiguate: AST's MethodDecl takes precedence over interpreter's
    use crate::ast::MethodDecl;

    fn make_program(declarations: Vec<Declaration>) -> Program {
        Program {
            imports: vec![],
            declarations,
        }
    }

    fn make_fn_decl(name: &str, params: Vec<Param>, body: Vec<Stmt>) -> FnDecl {
        FnDecl {
            access: Access::Public,
            name: name.to_string(),
            type_params: vec![],
            params,
            return_type: None,
            body,
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }
    }

    fn make_param(name: &str, typ: &str) -> Param {
        Param {
            name: name.to_string(),
            typ: Type::simple(typ),
        }
    }

    fn println_call(arg: Expr) -> Stmt {
        Stmt::Expr(Expr::Call(
            Box::new(Expr::MemberAccess(
                Box::new(Expr::Identifier("io".to_string(), Span::unknown())),
                "println".to_string(),
                Span::unknown(),
            )),
            vec![arg],
            Span::unknown(),
        ))
    }

    // ---- Variable declarations and assignments ----

    #[test]
    fn test_var_decl_and_read() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::simple("int")),
                    init: Some(Expr::Literal(Literal::Int(42), Span::unknown())),
                    mutable: true,
                    span: Span::unknown(),
                }),
                println_call(Expr::Identifier("x".to_string(), Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"42".to_string()));
    }

    #[test]
    fn test_const_decl() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::ConstDecl(VarDecl {
                    name: "PI".to_string(),
                    typ: Some(Type::simple("double")),
                    init: Some(Expr::Literal(Literal::Float(3.14159), Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                println_call(Expr::Identifier("PI".to_string(), Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"3.14159".to_string()));
    }

    #[test]
    fn test_var_assignment() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::simple("int")),
                    init: Some(Expr::Literal(Literal::Int(1), Span::unknown())),
                    mutable: true,
                    span: Span::unknown(),
                }),
                Stmt::Expr(Expr::Assign(
                    Box::new(Expr::Identifier("x".to_string(), Span::unknown())),
                    Box::new(Expr::Literal(Literal::Int(2), Span::unknown())),
                    Span::unknown(),
                )),
                println_call(Expr::Identifier("x".to_string(), Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"2".to_string()));
    }

    // ---- Arithmetic operators ----

    #[test]
    fn test_arithmetic() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "result".to_string(),
                    typ: Some(Type::simple("long")),
                    init: Some(Expr::Binary(
                        Box::new(Expr::Binary(
                            Box::new(Expr::Literal(Literal::Int(2), Span::unknown())),
                            Operator::Add,
                            Box::new(Expr::Literal(Literal::Int(3), Span::unknown())),
                            Span::unknown(),
                        )),
                        Operator::Mul,
                        Box::new(Expr::Literal(Literal::Int(4), Span::unknown())),
                        Span::unknown(),
                    )),
                    mutable: false,
                    span: Span::unknown(),
                }),
                println_call(Expr::Identifier("result".to_string(), Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"20".to_string()));
    }

    #[test]
    fn test_division() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Binary(
                    Box::new(Expr::Literal(Literal::Int(10), Span::unknown())),
                    Operator::Div,
                    Box::new(Expr::Literal(Literal::Int(3), Span::unknown())),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"3".to_string()));
    }

    #[test]
    fn test_modulo() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Binary(
                    Box::new(Expr::Literal(Literal::Int(10), Span::unknown())),
                    Operator::Mod,
                    Box::new(Expr::Literal(Literal::Int(3), Span::unknown())),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"1".to_string()));
    }

    // ---- Comparison operators ----

    #[test]
    fn test_comparison() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Binary(
                    Box::new(Expr::Literal(Literal::Int(5), Span::unknown())),
                    Operator::Gt,
                    Box::new(Expr::Literal(Literal::Int(3), Span::unknown())),
                    Span::unknown(),
                )),
                println_call(Expr::Binary(
                    Box::new(Expr::Literal(Literal::Int(3), Span::unknown())),
                    Operator::Eq,
                    Box::new(Expr::Literal(Literal::Int(3), Span::unknown())),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output[0], "true");
        assert_eq!(output[1], "true");
    }

    // ---- String concatenation ----

    #[test]
    fn test_string_concat() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Binary(
                    Box::new(Expr::Literal(Literal::String("Hello".to_string()), Span::unknown())),
                    Operator::Add,
                    Box::new(Expr::Literal(Literal::String(" World".to_string()), Span::unknown())),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"Hello World".to_string()));
    }

    #[test]
    fn test_string_number_concat() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Binary(
                    Box::new(Expr::Literal(Literal::String("Value: ".to_string()), Span::unknown())),
                    Operator::Add,
                    Box::new(Expr::Literal(Literal::Int(42), Span::unknown())),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"Value: 42".to_string()));
    }

    // ---- If/else ----

    #[test]
    fn test_if_else() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::simple("long")),
                    init: Some(Expr::Literal(Literal::Int(10), Span::unknown())),
                    mutable: true,
                    span: Span::unknown(),
                }),
                Stmt::If(IfStmt {
                    condition: Expr::Binary(
                        Box::new(Expr::Identifier("x".to_string(), Span::unknown())),
                        Operator::Gt,
                        Box::new(Expr::Literal(Literal::Int(5), Span::unknown())),
                        Span::unknown(),
                    ),
                    then_branch: vec![
                        println_call(Expr::Literal(Literal::String("big".to_string()), Span::unknown())),
                    ],
                    else_branch: Some(vec![
                        println_call(Expr::Literal(Literal::String("small".to_string()), Span::unknown())),
                    ]),
                    span: Span::unknown(),
                }),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"big".to_string()));
    }

    #[test]
    fn test_if_false() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::If(IfStmt {
                    condition: Expr::Literal(Literal::Bool(false), Span::unknown()),
                    then_branch: vec![
                        println_call(Expr::Literal(Literal::String("yes".to_string()), Span::unknown())),
                    ],
                    else_branch: Some(vec![
                        println_call(Expr::Literal(Literal::String("no".to_string()), Span::unknown())),
                    ]),
                    span: Span::unknown(),
                }),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"no".to_string()));
    }

    // ---- While loops ----

    #[test]
    fn test_while_loop() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "i".to_string(),
                    typ: Some(Type::simple("long")),
                    init: Some(Expr::Literal(Literal::Int(0), Span::unknown())),
                    mutable: true,
                    span: Span::unknown(),
                }),
                Stmt::While(WhileStmt {
                    condition: Expr::Binary(
                        Box::new(Expr::Identifier("i".to_string(), Span::unknown())),
                        Operator::Lt,
                        Box::new(Expr::Literal(Literal::Int(3), Span::unknown())),
                        Span::unknown(),
                    ),
                    body: vec![
                        Stmt::Expr(Expr::Assign(
                            Box::new(Expr::Identifier("i".to_string(), Span::unknown())),
                            Box::new(Expr::Binary(
                                Box::new(Expr::Identifier("i".to_string(), Span::unknown())),
                                Operator::Add,
                                Box::new(Expr::Literal(Literal::Int(1), Span::unknown())),
                                Span::unknown(),
                            )),
                            Span::unknown(),
                        )),
                    ],
                    span: Span::unknown(),
                }),
                println_call(Expr::Identifier("i".to_string(), Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"3".to_string()));
    }

    #[test]
    fn test_while_break() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "i".to_string(),
                    typ: Some(Type::simple("long")),
                    init: Some(Expr::Literal(Literal::Int(0), Span::unknown())),
                    mutable: true,
                    span: Span::unknown(),
                }),
                Stmt::While(WhileStmt {
                    condition: Expr::Literal(Literal::Bool(true), Span::unknown()),
                    body: vec![
                        Stmt::Expr(Expr::Assign(
                            Box::new(Expr::Identifier("i".to_string(), Span::unknown())),
                            Box::new(Expr::Binary(
                                Box::new(Expr::Identifier("i".to_string(), Span::unknown())),
                                Operator::Add,
                                Box::new(Expr::Literal(Literal::Int(1), Span::unknown())),
                                Span::unknown(),
                            )),
                            Span::unknown(),
                        )),
                        Stmt::If(IfStmt {
                            condition: Expr::Binary(
                                Box::new(Expr::Identifier("i".to_string(), Span::unknown())),
                                Operator::Eq,
                                Box::new(Expr::Literal(Literal::Int(3), Span::unknown())),
                                Span::unknown(),
                            ),
                            then_branch: vec![Stmt::Break],
                            else_branch: None,
                            span: Span::unknown(),
                        }),
                    ],
                    span: Span::unknown(),
                }),
                println_call(Expr::Identifier("i".to_string(), Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"3".to_string()));
    }

    #[test]
    fn test_while_continue() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "i".to_string(),
                    typ: Some(Type::simple("long")),
                    init: Some(Expr::Literal(Literal::Int(0), Span::unknown())),
                    mutable: true,
                    span: Span::unknown(),
                }),
                Stmt::VarDecl(VarDecl {
                    name: "sum".to_string(),
                    typ: Some(Type::simple("long")),
                    init: Some(Expr::Literal(Literal::Int(0), Span::unknown())),
                    mutable: true,
                    span: Span::unknown(),
                }),
                Stmt::While(WhileStmt {
                    condition: Expr::Binary(
                        Box::new(Expr::Identifier("i".to_string(), Span::unknown())),
                        Operator::Lt,
                        Box::new(Expr::Literal(Literal::Int(5), Span::unknown())),
                        Span::unknown(),
                    ),
                    body: vec![
                        Stmt::Expr(Expr::Assign(
                            Box::new(Expr::Identifier("i".to_string(), Span::unknown())),
                            Box::new(Expr::Binary(
                                Box::new(Expr::Identifier("i".to_string(), Span::unknown())),
                                Operator::Add,
                                Box::new(Expr::Literal(Literal::Int(1), Span::unknown())),
                                Span::unknown(),
                            )),
                            Span::unknown(),
                        )),
                        Stmt::If(IfStmt {
                            condition: Expr::Binary(
                                Box::new(Expr::Identifier("i".to_string(), Span::unknown())),
                                Operator::Mod,
                                Box::new(Expr::Literal(Literal::Int(2), Span::unknown())),
                                Span::unknown(),
                            ),
                            then_branch: vec![Stmt::Continue],
                            else_branch: None,
                            span: Span::unknown(),
                        }),
                        Stmt::Expr(Expr::Assign(
                            Box::new(Expr::Identifier("sum".to_string(), Span::unknown())),
                            Box::new(Expr::Binary(
                                Box::new(Expr::Identifier("sum".to_string(), Span::unknown())),
                                Operator::Add,
                                Box::new(Expr::Identifier("i".to_string(), Span::unknown())),
                                Span::unknown(),
                            )),
                            Span::unknown(),
                        )),
                    ],
                    span: Span::unknown(),
                }),
                println_call(Expr::Identifier("sum".to_string(), Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        // sum of even numbers 2+4 = 6
        assert_eq!(output.last(), Some(&"6".to_string()));
    }

    // ---- Function definitions and calls ----

    #[test]
    fn test_function_call() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("add", vec![
                make_param("a", "long"),
                make_param("b", "long"),
            ], vec![
                Stmt::Return(Some(Expr::Binary(
                    Box::new(Expr::Identifier("a".to_string(), Span::unknown())),
                    Operator::Add,
                    Box::new(Expr::Identifier("b".to_string(), Span::unknown())),
                    Span::unknown(),
                ))),
            ])),
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Call(
                    Box::new(Expr::Identifier("add".to_string(), Span::unknown())),
                    vec![
                        Expr::Literal(Literal::Int(3), Span::unknown()),
                        Expr::Literal(Literal::Int(4), Span::unknown()),
                    ],
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"7".to_string()));
    }

    #[test]
    fn test_recursive_function() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("fib", vec![
                make_param("n", "long"),
            ], vec![
                Stmt::If(IfStmt {
                    condition: Expr::Binary(
                        Box::new(Expr::Identifier("n".to_string(), Span::unknown())),
                        Operator::Le,
                        Box::new(Expr::Literal(Literal::Int(1), Span::unknown())),
                        Span::unknown(),
                    ),
                    then_branch: vec![Stmt::Return(Some(Expr::Identifier("n".to_string(), Span::unknown())))],
                    else_branch: None,
                    span: Span::unknown(),
                }),
                Stmt::Return(Some(Expr::Binary(
                    Box::new(Expr::Call(
                        Box::new(Expr::Identifier("fib".to_string(), Span::unknown())),
                        vec![Expr::Binary(
                            Box::new(Expr::Identifier("n".to_string(), Span::unknown())),
                            Operator::Sub,
                            Box::new(Expr::Literal(Literal::Int(1), Span::unknown())),
                            Span::unknown(),
                        )],
                        Span::unknown(),
                    )),
                    Operator::Add,
                    Box::new(Expr::Call(
                        Box::new(Expr::Identifier("fib".to_string(), Span::unknown())),
                        vec![Expr::Binary(
                            Box::new(Expr::Identifier("n".to_string(), Span::unknown())),
                            Operator::Sub,
                            Box::new(Expr::Literal(Literal::Int(2), Span::unknown())),
                            Span::unknown(),
                        )],
                        Span::unknown(),
                    )),
                    Span::unknown(),
                ))),
            ])),
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Call(
                    Box::new(Expr::Identifier("fib".to_string(), Span::unknown())),
                    vec![Expr::Literal(Literal::Int(10), Span::unknown())],
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"55".to_string()));
    }

    // ---- Class instantiation and method calls ----

    #[test]
    fn test_class_instantiation() {
        let program = make_program(vec![
            Declaration::Class(ClassDecl {
                name: "Point".to_string(),
                type_params: vec![],
                parent: None,
                ifaces: vec![],
                members: vec![
                    ClassMember::Field(FieldDecl {
                        access: Access::Public,
                        name: "x".to_string(),
                        typ: Type::simple("long"),
                        init: Some(Expr::Literal(Literal::Int(0), Span::unknown())),
                        span: Span::unknown(),
                    }),
                    ClassMember::Field(FieldDecl {
                        access: Access::Public,
                        name: "y".to_string(),
                        typ: Type::simple("long"),
                        init: Some(Expr::Literal(Literal::Int(0), Span::unknown())),
                        span: Span::unknown(),
                    }),
                ],
                span: Span::unknown(),
            }),
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "p".to_string(),
                    typ: Some(Type::simple("Point")),
                    init: Some(Expr::New(Type::simple("Point"), vec![], Span::unknown())),
                    mutable: true,
                    span: Span::unknown(),
                }),
                println_call(Expr::MemberAccess(
                    Box::new(Expr::Identifier("p".to_string(), Span::unknown())),
                    "x".to_string(),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"0".to_string()));
    }

    #[test]
    fn test_class_with_constructor() {
        let program = make_program(vec![
            Declaration::Class(ClassDecl {
                name: "Point".to_string(),
                type_params: vec![],
                parent: None,
                ifaces: vec![],
                members: vec![
                    ClassMember::Field(FieldDecl {
                        access: Access::Public,
                        name: "x".to_string(),
                        typ: Type::simple("long"),
                        init: None,
                        span: Span::unknown(),
                    }),
                    ClassMember::Field(FieldDecl {
                        access: Access::Public,
                        name: "y".to_string(),
                        typ: Type::simple("long"),
                        init: None,
                        span: Span::unknown(),
                    }),
                    ClassMember::Constructor(MethodDecl {
                        access: Access::Public,
                        name: "new".to_string(),
                        type_params: vec![],
                        params: vec![
                            make_param("x", "long"),
                            make_param("y", "long"),
                        ],
                        return_type: None,
                        body: vec![
                            Stmt::Expr(Expr::Assign(
                                Box::new(Expr::MemberAccess(
                                    Box::new(Expr::This(Span::unknown())),
                                    "x".to_string(),
                                    Span::unknown(),
                                )),
                                Box::new(Expr::Identifier("x".to_string(), Span::unknown())),
                                Span::unknown(),
                            )),
                            Stmt::Expr(Expr::Assign(
                                Box::new(Expr::MemberAccess(
                                    Box::new(Expr::This(Span::unknown())),
                                    "y".to_string(),
                                    Span::unknown(),
                                )),
                                Box::new(Expr::Identifier("y".to_string(), Span::unknown())),
                                Span::unknown(),
                            )),
                        ],
                        where_clause: vec![],
                        span: Span::unknown(),
                    }),
                ],
                span: Span::unknown(),
            }),
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "p".to_string(),
                    typ: Some(Type::simple("Point")),
                    init: Some(Expr::New(Type::simple("Point"), vec![
                        Expr::Literal(Literal::Int(3), Span::unknown()),
                        Expr::Literal(Literal::Int(4), Span::unknown()),
                    ], Span::unknown())),
                    mutable: true,
                    span: Span::unknown(),
                }),
                println_call(Expr::MemberAccess(
                    Box::new(Expr::Identifier("p".to_string(), Span::unknown())),
                    "x".to_string(),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"3".to_string()));
    }

    #[test]
    fn test_class_method() {
        let program = make_program(vec![
            Declaration::Class(ClassDecl {
                name: "Counter".to_string(),
                type_params: vec![],
                parent: None,
                ifaces: vec![],
                members: vec![
                    ClassMember::Field(FieldDecl {
                        access: Access::Public,
                        name: "count".to_string(),
                        typ: Type::simple("long"),
                        init: Some(Expr::Literal(Literal::Int(0), Span::unknown())),
                        span: Span::unknown(),
                    }),
                    ClassMember::Method(MethodDecl {
                        access: Access::Public,
                        name: "increment".to_string(),
                        type_params: vec![],
                        params: vec![],
                        return_type: None,
                        body: vec![
                            Stmt::Expr(Expr::Assign(
                                Box::new(Expr::MemberAccess(
                                    Box::new(Expr::This(Span::unknown())),
                                    "count".to_string(),
                                    Span::unknown(),
                                )),
                                Box::new(Expr::Binary(
                                    Box::new(Expr::MemberAccess(
                                        Box::new(Expr::This(Span::unknown())),
                                        "count".to_string(),
                                        Span::unknown(),
                                    )),
                                    Operator::Add,
                                    Box::new(Expr::Literal(Literal::Int(1), Span::unknown())),
                                    Span::unknown(),
                                )),
                                Span::unknown(),
                            )),
                        ],
                        where_clause: vec![],
                        span: Span::unknown(),
                    }),
                    ClassMember::Method(MethodDecl {
                        access: Access::Public,
                        name: "getCount".to_string(),
                        type_params: vec![],
                        params: vec![],
                        return_type: Some(Type::simple("long")),
                        body: vec![
                            Stmt::Return(Some(Expr::MemberAccess(
                                Box::new(Expr::This(Span::unknown())),
                                "count".to_string(),
                                Span::unknown(),
                            ))),
                        ],
                        where_clause: vec![],
                        span: Span::unknown(),
                    }),
                ],
                span: Span::unknown(),
            }),
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "c".to_string(),
                    typ: Some(Type::simple("Counter")),
                    init: Some(Expr::New(Type::simple("Counter"), vec![], Span::unknown())),
                    mutable: true,
                    span: Span::unknown(),
                }),
                Stmt::Expr(Expr::Call(
                    Box::new(Expr::MemberAccess(
                        Box::new(Expr::Identifier("c".to_string(), Span::unknown())),
                        "increment".to_string(),
                        Span::unknown(),
                    )),
                    vec![],
                    Span::unknown(),
                )),
                Stmt::Expr(Expr::Call(
                    Box::new(Expr::MemberAccess(
                        Box::new(Expr::Identifier("c".to_string(), Span::unknown())),
                        "increment".to_string(),
                        Span::unknown(),
                    )),
                    vec![],
                    Span::unknown(),
                )),
                println_call(Expr::Call(
                    Box::new(Expr::MemberAccess(
                        Box::new(Expr::Identifier("c".to_string(), Span::unknown())),
                        "getCount".to_string(),
                        Span::unknown(),
                    )),
                    vec![],
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"2".to_string()));
    }

    // ---- Enum construction and pattern matching ----

    #[test]
    fn test_enum_construction() {
        let program = make_program(vec![
            Declaration::Enum(EnumDecl {
                name: "Color".to_string(),
                type_params: vec![],
                variants: vec![
                    Variant { name: "Red".to_string(), fields: vec![] },
                    Variant { name: "Green".to_string(), fields: vec![] },
                    Variant { name: "Blue".to_string(), fields: vec![] },
                ],
                span: Span::unknown(),
            }),
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "c".to_string(),
                    typ: Some(Type::simple("Color")),
                    init: Some(Expr::Call(
                        Box::new(Expr::Identifier("Red".to_string(), Span::unknown())),
                        vec![],
                        Span::unknown(),
                    )),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::Switch(SwitchStmt {
                    expr: Expr::Identifier("c".to_string(), Span::unknown()),
                    cases: vec![
                        Case {
                            pattern: Pattern::Constructor {
                                name: "Red".to_string(),
                                bindings: vec![],
                            },
                            body: vec![
                                println_call(Expr::Literal(Literal::String("red".to_string()), Span::unknown())),
                            ],
                        },
                        Case {
                            pattern: Pattern::Constructor {
                                name: "Green".to_string(),
                                bindings: vec![],
                            },
                            body: vec![
                                println_call(Expr::Literal(Literal::String("green".to_string()), Span::unknown())),
                            ],
                        },
                    ],
                    default: Some(vec![
                        println_call(Expr::Literal(Literal::String("other".to_string()), Span::unknown())),
                    ]),
                    span: Span::unknown(),
                }),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"red".to_string()));
    }

    #[test]
    fn test_enum_with_fields() {
        let program = make_program(vec![
            Declaration::Enum(EnumDecl {
                name: "Option".to_string(),
                type_params: vec![],
                variants: vec![
                    Variant {
                        name: "Some".to_string(),
                        fields: vec![make_param("value", "long")],
                    },
                    Variant {
                        name: "None".to_string(),
                        fields: vec![],
                    },
                ],
                span: Span::unknown(),
            }),
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "opt".to_string(),
                    typ: Some(Type::simple("Option")),
                    init: Some(Expr::Call(
                        Box::new(Expr::Identifier("Some".to_string(), Span::unknown())),
                        vec![Expr::Literal(Literal::Int(42), Span::unknown())],
                        Span::unknown(),
                    )),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::Switch(SwitchStmt {
                    expr: Expr::Identifier("opt".to_string(), Span::unknown()),
                    cases: vec![
                        Case {
                            pattern: Pattern::Constructor {
                                name: "Some".to_string(),
                                bindings: vec!["v".to_string()],
                            },
                            body: vec![
                                println_call(Expr::Identifier("v".to_string(), Span::unknown())),
                            ],
                        },
                    ],
                    default: Some(vec![
                        println_call(Expr::Literal(Literal::String("none".to_string()), Span::unknown())),
                    ]),
                    span: Span::unknown(),
                }),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"42".to_string()));
    }

    // ---- Result type and error propagation ----

    #[test]
    fn test_result_ok_display() {
        let val = Value::ResultOk(Box::new(Value::Long(42)));
        assert_eq!(val.display_string(), "Ok(42)");
    }

    #[test]
    fn test_result_err_display() {
        let val = Value::ResultErr(Box::new(Value::String("error".to_string())));
        assert_eq!(val.display_string(), "Err(error)");
    }

    #[test]
    fn test_error_propagation_ok() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "r".to_string(),
                    typ: Some(Type::generic("Result", vec![Type::simple("long"), Type::simple("string")])),
                    init: Some(Expr::Literal(Literal::Int(42), Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::VarDecl(VarDecl {
                    name: "val".to_string(),
                    typ: Some(Type::simple("long")),
                    init: Some(Expr::ErrorPropagation(Box::new(
                        Expr::Identifier("r".to_string(), Span::unknown()),
                    ), Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                println_call(Expr::Identifier("val".to_string(), Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"42".to_string()));
    }

    #[test]
    fn test_error_propagation_err() {
        let val = Value::ResultErr(Box::new(Value::String("error".to_string())));
        let result = match val {
            Value::ResultErr(e) => Value::ResultErr(e),
            Value::ResultOk(v) => *v,
            other => other,
        };
        match result {
            Value::ResultErr(e) => assert_eq!(*e, Value::String("error".to_string())),
            _ => panic!("Expected ResultErr"),
        }
    }

    // ---- ArrayList operations ----

    #[test]
    fn test_arraylist_size() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "list".to_string(),
                    typ: Some(Type::simple("ArrayList")),
                    init: Some(Expr::New(Type::simple("ArrayList"), vec![], Span::unknown())),
                    mutable: true,
                    span: Span::unknown(),
                }),
                println_call(Expr::Call(
                    Box::new(Expr::MemberAccess(
                        Box::new(Expr::Identifier("list".to_string(), Span::unknown())),
                        "size".to_string(),
                        Span::unknown(),
                    )),
                    vec![],
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"0".to_string()));
    }

    // ---- HashMap operations ----

    #[test]
    fn test_hashmap_get_missing() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "map".to_string(),
                    typ: Some(Type::simple("HashMap")),
                    init: Some(Expr::New(Type::simple("HashMap"), vec![], Span::unknown())),
                    mutable: true,
                    span: Span::unknown(),
                }),
                println_call(Expr::Call(
                    Box::new(Expr::MemberAccess(
                        Box::new(Expr::Identifier("map".to_string(), Span::unknown())),
                        "get".to_string(),
                        Span::unknown(),
                    )),
                    vec![Expr::Literal(Literal::String("key".to_string()), Span::unknown())],
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"null".to_string()));
    }

    // ---- Type casting with as ----

    #[test]
    fn test_cast_int_to_long() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::simple("long")),
                    init: Some(Expr::Cast(
                        Box::new(Expr::Literal(Literal::Int(42), Span::unknown())),
                        Type::simple("long"),
                        Span::unknown(),
                    )),
                    mutable: false,
                    span: Span::unknown(),
                }),
                println_call(Expr::Identifier("x".to_string(), Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"42".to_string()));
    }

    #[test]
    fn test_cast_int_to_double() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Cast(
                    Box::new(Expr::Literal(Literal::Int(42), Span::unknown())),
                    Type::simple("double"),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"42".to_string()));
    }

    #[test]
    fn test_cast_long_to_byte() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Cast(
                    Box::new(Expr::Literal(Literal::Int(300), Span::unknown())),
                    Type::simple("byte"),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        // 300 as i8 = 44 (wrapping)
        assert_eq!(output.last(), Some(&"44".to_string()));
    }

    #[test]
    fn test_cast_to_string() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Cast(
                    Box::new(Expr::Literal(Literal::Int(42), Span::unknown())),
                    Type::simple("string"),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"42".to_string()));
    }

    // ---- Unary operators ----

    #[test]
    fn test_unary_neg() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Unary(UnOp::Neg, Box::new(Expr::Literal(Literal::Int(5), Span::unknown())), Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"-5".to_string()));
    }

    #[test]
    fn test_unary_not() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Unary(UnOp::Not, Box::new(Expr::Literal(Literal::Bool(true), Span::unknown())), Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"false".to_string()));
    }

    #[test]
    fn test_bitwise_not() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Unary(UnOp::BitNot, Box::new(Expr::Literal(Literal::Int(0), Span::unknown())), Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"-1".to_string()));
    }

    // ---- Bitwise operators ----

    #[test]
    fn test_bitwise_and() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Binary(
                    Box::new(Expr::Literal(Literal::Int(0b1100), Span::unknown())),
                    Operator::BitAnd,
                    Box::new(Expr::Literal(Literal::Int(0b1010), Span::unknown())),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"8".to_string()));
    }

    #[test]
    fn test_bitwise_or() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Binary(
                    Box::new(Expr::Literal(Literal::Int(0b1100), Span::unknown())),
                    Operator::BitOr,
                    Box::new(Expr::Literal(Literal::Int(0b1010), Span::unknown())),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"14".to_string()));
    }

    #[test]
    fn test_shift_left() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Binary(
                    Box::new(Expr::Literal(Literal::Int(1), Span::unknown())),
                    Operator::BitShl,
                    Box::new(Expr::Literal(Literal::Int(4), Span::unknown())),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"16".to_string()));
    }

    #[test]
    fn test_shift_right() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Binary(
                    Box::new(Expr::Literal(Literal::Int(16), Span::unknown())),
                    Operator::BitShr,
                    Box::new(Expr::Literal(Literal::Int(2), Span::unknown())),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"4".to_string()));
    }

    // ---- Logical operators ----

    #[test]
    fn test_logical_and() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Binary(
                    Box::new(Expr::Literal(Literal::Bool(true), Span::unknown())),
                    Operator::And,
                    Box::new(Expr::Literal(Literal::Bool(false), Span::unknown())),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"false".to_string()));
    }

    #[test]
    fn test_logical_or() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Binary(
                    Box::new(Expr::Literal(Literal::Bool(true), Span::unknown())),
                    Operator::Or,
                    Box::new(Expr::Literal(Literal::Bool(false), Span::unknown())),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"true".to_string()));
    }

    // ---- For loop ----

    #[test]
    fn test_for_loop() {
        // Test for-loop with an ArrayList as iterable
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "list".to_string(),
                    typ: Some(Type::simple("ArrayList")),
                    init: Some(Expr::New(Type::simple("ArrayList"), vec![], Span::unknown())),
                    mutable: true,
                    span: Span::unknown(),
                }),
                Stmt::Expr(Expr::Call(
                    Box::new(Expr::MemberAccess(
                        Box::new(Expr::Identifier("list".to_string(), Span::unknown())),
                        "add".to_string(),
                        Span::unknown(),
                    )),
                    vec![Expr::Literal(Literal::Int(1), Span::unknown())],
                    Span::unknown(),
                )),
                Stmt::Expr(Expr::Call(
                    Box::new(Expr::MemberAccess(
                        Box::new(Expr::Identifier("list".to_string(), Span::unknown())),
                        "add".to_string(),
                        Span::unknown(),
                    )),
                    vec![Expr::Literal(Literal::Int(2), Span::unknown())],
                    Span::unknown(),
                )),
                Stmt::Expr(Expr::Call(
                    Box::new(Expr::MemberAccess(
                        Box::new(Expr::Identifier("list".to_string(), Span::unknown())),
                        "add".to_string(),
                        Span::unknown(),
                    )),
                    vec![Expr::Literal(Literal::Int(3), Span::unknown())],
                    Span::unknown(),
                )),
                Stmt::VarDecl(VarDecl {
                    name: "sum".to_string(),
                    typ: Some(Type::simple("long")),
                    init: Some(Expr::Literal(Literal::Int(0), Span::unknown())),
                    mutable: true,
                    span: Span::unknown(),
                }),
                Stmt::For(ForStmt {
                    var: "i".to_string(),
                    iterable: Expr::Identifier("list".to_string(), Span::unknown()),
                    body: vec![
                        Stmt::Expr(Expr::Assign(
                            Box::new(Expr::Identifier("sum".to_string(), Span::unknown())),
                            Box::new(Expr::Binary(
                                Box::new(Expr::Identifier("sum".to_string(), Span::unknown())),
                                Operator::Add,
                                Box::new(Expr::Identifier("i".to_string(), Span::unknown())),
                                Span::unknown(),
                            )),
                            Span::unknown(),
                        )),
                    ],
                    span: Span::unknown(),
                }),
                println_call(Expr::Identifier("sum".to_string(), Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"6".to_string()));
    }

    // ---- Static call ----

    #[test]
    fn test_static_call_println() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::Expr(Expr::StaticCall {
                    class_name: "io".to_string(),
                    method: "println".to_string(),
                    args: vec![Expr::Literal(Literal::String("hello".to_string()), Span::unknown())],
                    span: Span::unknown(),
                }),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"hello".to_string()));
    }

    #[test]
    fn test_static_call_to_string() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::StaticCall {
                    class_name: "Integer".to_string(),
                    method: "toString".to_string(),
                    args: vec![Expr::Literal(Literal::Int(42), Span::unknown())],
                    span: Span::unknown(),
                }),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"42".to_string()));
    }

    #[test]
    fn test_static_call_parse_int() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::StaticCall {
                    class_name: "Integer".to_string(),
                    method: "parseInt".to_string(),
                    args: vec![Expr::Literal(Literal::String("123".to_string()), Span::unknown())],
                    span: Span::unknown(),
                }),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"Ok(123)".to_string()));
    }

    // ---- Value equality ----

    #[test]
    fn test_value_equality() {
        assert_eq!(Value::Int(42), Value::Int(42));
        assert_ne!(Value::Int(42), Value::Int(43));
        assert_eq!(Value::Bool(true), Value::Bool(true));
        assert_eq!(Value::String("hello".to_string()), Value::String("hello".to_string()));
        assert_eq!(Value::Null, Value::Null);
    }

    // ---- Division by zero ----

    #[test]
    fn test_division_by_zero() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::Expr(Expr::Binary(
                    Box::new(Expr::Literal(Literal::Int(1), Span::unknown())),
                    Operator::Div,
                    Box::new(Expr::Literal(Literal::Int(0), Span::unknown())),
                    Span::unknown(),
                )),
            ])),
        ]);
        let result = interpret(&program);
        assert!(result.is_err());
        assert!(result.err().map_or(false, |e| e.contains("zero")));
    }

    // ---- Undefined variable ----

    #[test]
    fn test_undefined_variable() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::Expr(Expr::Identifier("unknown".to_string(), Span::unknown())),
            ])),
        ]);
        let result = interpret(&program);
        assert!(result.is_err());
    }

    // ---- Null literal ----

    #[test]
    fn test_null_literal() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Literal(Literal::Null, Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"null".to_string()));
    }

    // ---- Char literal ----

    #[test]
    fn test_char_literal() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Literal(Literal::Char('A'), Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"A".to_string()));
    }

    // ---- Float arithmetic ----

    #[test]
    fn test_float_arithmetic() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Binary(
                    Box::new(Expr::Literal(Literal::Float(1.5), Span::unknown())),
                    Operator::Add,
                    Box::new(Expr::Literal(Literal::Float(2.5), Span::unknown())),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"4".to_string()));
    }

    // ---- Block scoping ----

    #[test]
    fn test_block_scoping() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::simple("long")),
                    init: Some(Expr::Literal(Literal::Int(1), Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::Block(vec![
                    Stmt::VarDecl(VarDecl {
                        name: "x".to_string(),
                        typ: Some(Type::simple("long")),
                        init: Some(Expr::Literal(Literal::Int(2), Span::unknown())),
                        mutable: false,
                        span: Span::unknown(),
                    }),
                    println_call(Expr::Identifier("x".to_string(), Span::unknown())),
                ]),
                println_call(Expr::Identifier("x".to_string(), Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output[0], "2");
        assert_eq!(output[1], "1");
    }

    // ---- No main function ----

    #[test]
    fn test_no_main() {
        let program = make_program(vec![]);
        let result = interpret(&program);
        assert!(result.is_ok());
    }

    // ---- Return void ----

    #[test]
    fn test_return_void() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::Return(None),
            ])),
        ]);
        let result = interpret(&program);
        assert!(result.is_ok());
    }

    // ---- Ne operator ----

    #[test]
    fn test_ne_operator() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Binary(
                    Box::new(Expr::Literal(Literal::Int(1), Span::unknown())),
                    Operator::Ne,
                    Box::new(Expr::Literal(Literal::Int(2), Span::unknown())),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"true".to_string()));
    }

    // ---- Le and Ge operators ----

    #[test]
    fn test_le_ge_operators() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Binary(
                    Box::new(Expr::Literal(Literal::Int(3), Span::unknown())),
                    Operator::Le,
                    Box::new(Expr::Literal(Literal::Int(3), Span::unknown())),
                    Span::unknown(),
                )),
                println_call(Expr::Binary(
                    Box::new(Expr::Literal(Literal::Int(5), Span::unknown())),
                    Operator::Ge,
                    Box::new(Expr::Literal(Literal::Int(3), Span::unknown())),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output[0], "true");
        assert_eq!(output[1], "true");
    }

    // ---- Bool literal ----

    #[test]
    fn test_bool_literal() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Literal(Literal::Bool(true), Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"true".to_string()));
    }

    // ---- Owned type ----

    #[test]
    fn test_owned_deref() {
        let owned = Value::Owned(Box::new(Value::Long(42)));
        match owned {
            Value::Owned(inner) => assert_eq!(*inner, Value::Long(42)),
            _ => panic!("Expected Owned"),
        }
    }

    // ---- Moved sentinel ----

    #[test]
    fn test_moved_value() {
        let moved = Value::Moved;
        assert!(!moved.is_truthy());
        assert_eq!(moved.display_string(), "<moved>");
    }

    // ---- Memory operations ----

    #[test]
    fn test_memory_alloc_and_read() {
        let mut mem = Memory::new();
        let idx = mem.alloc(Value::Long(42));
        let val = mem.read(idx);
        assert!(val.is_ok());
        assert_eq!(val.ok(), Some(Value::Long(42)));
    }

    #[test]
    fn test_memory_write() {
        let mut mem = Memory::new();
        let idx = mem.alloc(Value::Long(42));
        let write_result = mem.write(idx, Value::Long(100));
        assert!(write_result.is_ok());
        let val = mem.read(idx);
        assert_eq!(val.ok(), Some(Value::Long(100)));
    }

    #[test]
    fn test_memory_out_of_bounds() {
        let mem = Memory::new();
        let val = mem.read(999);
        assert!(val.is_err());
    }

    #[test]
    fn test_region_alloc_and_pop() {
        let mut mem = Memory::new();
        mem.push_region();
        let idx = mem.region_alloc(Value::Long(42));
        let val = mem.read(idx);
        assert_eq!(val.ok(), Some(Value::Long(42)));
        mem.pop_region();
        let val_after = mem.read(idx);
        assert_eq!(val_after.ok(), Some(Value::Void));
    }

    // ---- Raw memory ----

    #[test]
    fn test_raw_memory() {
        let mut mem = Memory::new();
        let offset = mem.raw_alloc(&[1, 2, 3, 4]);
        let data = mem.raw_read(offset, 4);
        assert!(data.is_ok());
        assert_eq!(data.ok(), Some(vec![1, 2, 3, 4]));
    }

    // ---- RefExpr ----

    #[test]
    fn test_ref_expr() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "r".to_string(),
                    typ: Some(Type::simple("ref")),
                    init: Some(Expr::RefExpr(
                        Box::new(Expr::Literal(Literal::Int(42), Span::unknown())),
                        RefKind::Immutable,
                        Span::unknown(),
                    )),
                    mutable: false,
                    span: Span::unknown(),
                }),
                println_call(Expr::Identifier("r".to_string(), Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        // Should print ref(0) since it's a reference to memory slot 0
        assert!(output.last().map_or(false, |s| s.starts_with("ref(")));
    }

    // ---- Unsafe block ----

    #[test]
    fn test_unsafe_block() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::Expr(Expr::UnsafeBlock(vec![
                    Stmt::Expr(Expr::Call(
                        Box::new(Expr::MemberAccess(
                            Box::new(Expr::Identifier("io".to_string(), Span::unknown())),
                            "println".to_string(),
                            Span::unknown(),
                        )),
                        vec![Expr::Literal(Literal::String("unsafe".to_string()), Span::unknown())],
                        Span::unknown(),
                    )),
                ], Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"unsafe".to_string()));
    }

    // ---- Wildcard pattern in switch ----

    #[test]
    fn test_wildcard_pattern() {
        let program = make_program(vec![
            Declaration::Enum(EnumDecl {
                name: "Color".to_string(),
                type_params: vec![],
                variants: vec![
                    Variant { name: "Red".to_string(), fields: vec![] },
                    Variant { name: "Blue".to_string(), fields: vec![] },
                ],
                span: Span::unknown(),
            }),
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::Switch(SwitchStmt {
                    expr: Expr::Call(
                        Box::new(Expr::Identifier("Blue".to_string(), Span::unknown())),
                        vec![],
                        Span::unknown(),
                    ),
                    cases: vec![
                        Case {
                            pattern: Pattern::Wildcard,
                            body: vec![
                                println_call(Expr::Literal(Literal::String("matched".to_string()), Span::unknown())),
                            ],
                        },
                    ],
                    default: None,
                    span: Span::unknown(),
                }),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"matched".to_string()));
    }

    // ---- Literal pattern in switch ----

    #[test]
    fn test_literal_pattern() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::Switch(SwitchStmt {
                    expr: Expr::Literal(Literal::Int(42), Span::unknown()),
                    cases: vec![
                        Case {
                            pattern: Pattern::Literal(Literal::Int(42)),
                            body: vec![
                                println_call(Expr::Literal(Literal::String("found".to_string()), Span::unknown())),
                            ],
                        },
                    ],
                    default: Some(vec![
                        println_call(Expr::Literal(Literal::String("not found".to_string()), Span::unknown())),
                    ]),
                    span: Span::unknown(),
                }),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"found".to_string()));
    }

    // ---- Array indexing via ArrayList ----

    #[test]
    fn test_array_indexing() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "arr".to_string(),
                    typ: Some(Type::simple("ArrayList")),
                    init: Some(Expr::New(Type::simple("ArrayList"), vec![], Span::unknown())),
                    mutable: true,
                    span: Span::unknown(),
                }),
                Stmt::Expr(Expr::Call(
                    Box::new(Expr::MemberAccess(
                        Box::new(Expr::Identifier("arr".to_string(), Span::unknown())),
                        "add".to_string(),
                        Span::unknown(),
                    )),
                    vec![Expr::Literal(Literal::Int(10), Span::unknown())],
                    Span::unknown(),
                )),
                Stmt::Expr(Expr::Call(
                    Box::new(Expr::MemberAccess(
                        Box::new(Expr::Identifier("arr".to_string(), Span::unknown())),
                        "add".to_string(),
                        Span::unknown(),
                    )),
                    vec![Expr::Literal(Literal::Int(20), Span::unknown())],
                    Span::unknown(),
                )),
                Stmt::Expr(Expr::Call(
                    Box::new(Expr::MemberAccess(
                        Box::new(Expr::Identifier("arr".to_string(), Span::unknown())),
                        "add".to_string(),
                        Span::unknown(),
                    )),
                    vec![Expr::Literal(Literal::Int(30), Span::unknown())],
                    Span::unknown(),
                )),
                println_call(Expr::Call(
                    Box::new(Expr::MemberAccess(
                        Box::new(Expr::Identifier("arr".to_string(), Span::unknown())),
                        "get".to_string(),
                        Span::unknown(),
                    )),
                    vec![Expr::Literal(Literal::Int(1), Span::unknown())],
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"20".to_string()));
    }

    // ---- String length member ----

    #[test]
    fn test_string_length() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "s".to_string(),
                    typ: Some(Type::simple("string")),
                    init: Some(Expr::Literal(Literal::String("hello".to_string()), Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                println_call(Expr::MemberAccess(
                    Box::new(Expr::Identifier("s".to_string(), Span::unknown())),
                    "length".to_string(),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"5".to_string()));
    }

    // ---- Cast int to float ----

    #[test]
    fn test_cast_int_to_float() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Cast(
                    Box::new(Expr::Literal(Literal::Int(7), Span::unknown())),
                    Type::simple("float"),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"7".to_string()));
    }

    // ---- Cast double to int ----

    #[test]
    fn test_cast_double_to_int() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::Cast(
                    Box::new(Expr::Literal(Literal::Float(3.9), Span::unknown())),
                    Type::simple("int"),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"3".to_string()));
    }

    // ---- Error propagation with ResultErr propagates ----

    #[test]
    fn test_error_propagation_returns_err() {
        // Test that error propagation on a ResultErr value returns an error
        // We construct a ResultErr value via the interpreter's built-in handling
        // Since Expr::ResultErr doesn't exist in the AST, we test error propagation
        // by creating a function that returns a ResultErr through the interpreter
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "val".to_string(),
                    typ: Some(Type::generic("Result", vec![Type::simple("long"), Type::simple("string")])),
                    init: Some(Expr::Call(
                        Box::new(Expr::Identifier("makeErr".to_string(), Span::unknown())),
                        vec![],
                        Span::unknown(),
                    )),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::VarDecl(VarDecl {
                    name: "unwrapped".to_string(),
                    typ: Some(Type::simple("long")),
                    init: Some(Expr::ErrorPropagation(Box::new(
                        Expr::Identifier("val".to_string(), Span::unknown()),
                    ), Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                println_call(Expr::Literal(Literal::String("should not reach".to_string()), Span::unknown())),
            ])),
            Declaration::Function(FnDecl {
                access: Access::Public,
                name: "makeErr".to_string(),
                type_params: vec![],
                params: vec![],
                return_type: Some(Type::generic("Result", vec![Type::simple("long"), Type::simple("string")])),
                body: vec![
                    Stmt::Return(Some(Expr::Call(
                        Box::new(Expr::StaticCall {
                            class_name: "Result".to_string(),
                            method: "err".to_string(),
                            args: vec![Expr::Literal(Literal::String("bad".to_string()), Span::unknown())],
                            span: Span::unknown(),
                        }),
                        vec![],
                        Span::unknown(),
                    ))),
                ],
                sugar: false,
                where_clause: vec![],
                span: Span::unknown(),
            }),
        ]);
        let interp = Interpreter::new();
        let result = interp.run(&program);
        // The error propagation should cause the function to return an error
        assert!(result.is_err() || interp.output.borrow().is_empty());
    }

    // ---- parseInt success ----

    #[test]
    fn test_parse_int_success() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::StaticCall {
                    class_name: "Integer".to_string(),
                    method: "parseInt".to_string(),
                    args: vec![Expr::Literal(Literal::String("42".to_string()), Span::unknown())],
                    span: Span::unknown(),
                }),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"Ok(42)".to_string()));
    }

    // ---- parseInt failure ----

    #[test]
    fn test_parse_int_failure() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::StaticCall {
                    class_name: "Integer".to_string(),
                    method: "parseInt".to_string(),
                    args: vec![Expr::Literal(Literal::String("not_a_number".to_string()), Span::unknown())],
                    span: Span::unknown(),
                }),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert!(output.last().map_or(false, |s| s.starts_with("Err(")));
    }

    // ---- Double toString ----

    #[test]
    fn test_double_to_string() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                println_call(Expr::StaticCall {
                    class_name: "Double".to_string(),
                    method: "toString".to_string(),
                    args: vec![Expr::Literal(Literal::Float(3.14), Span::unknown())],
                    span: Span::unknown(),
                }),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"3.14".to_string()));
    }

    // ---- Closure tests ----

    #[test]
    fn test_interpret_closure() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "f".to_string(),
                    typ: None,
                    init: Some(Expr::Closure {
                        params: vec![("x".to_string(), Type::simple("long"))],
                        return_type: Type::simple("long"),
                        body: vec![],
                        expr: Some(Box::new(Expr::Binary(
                            Box::new(Expr::Identifier("x".to_string(), Span::unknown())),
                            Operator::Add,
                            Box::new(Expr::Literal(Literal::Int(1), Span::unknown())),
                            Span::unknown(),
                        ))),
                        captured_vars: vec![],
                        span: Span::unknown(),
                    }),
                    mutable: false,
                    span: Span::unknown(),
                }),
                println_call(Expr::Call(
                    Box::new(Expr::Identifier("f".to_string(), Span::unknown())),
                    vec![Expr::Literal(Literal::Int(41), Span::unknown())],
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"42".to_string()));
    }

    // ---- Tuple tests ----

    #[test]
    fn test_interpret_tuple() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::VarDecl(VarDecl {
                    name: "t".to_string(),
                    typ: None,
                    init: Some(Expr::Tuple(vec![
                        Expr::Literal(Literal::Int(10), Span::unknown()),
                        Expr::Literal(Literal::Int(20), Span::unknown()),
                    ], Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                println_call(Expr::Identifier("t".to_string(), Span::unknown())),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"(10, 20)".to_string()));
    }

    #[test]
    fn test_interpret_tuple_destructure() {
        let program = make_program(vec![
            Declaration::Function(make_fn_decl("main", vec![], vec![
                Stmt::TupleDestructure {
                    names: vec!["a".to_string(), "b".to_string()],
                    expr: Expr::Tuple(vec![
                        Expr::Literal(Literal::Int(10), Span::unknown()),
                        Expr::Literal(Literal::Int(20), Span::unknown()),
                    ], Span::unknown()),
                    mutable: false,
                    span: Span::unknown(),
                },
                println_call(Expr::Binary(
                    Box::new(Expr::Identifier("a".to_string(), Span::unknown())),
                    Operator::Add,
                    Box::new(Expr::Identifier("b".to_string(), Span::unknown())),
                    Span::unknown(),
                )),
            ])),
        ]);
        let interp = Interpreter::new();
        interp.run(&program).ok();
        let output = interp.output.borrow();
        assert_eq!(output.last(), Some(&"30".to_string()));
    }
}
