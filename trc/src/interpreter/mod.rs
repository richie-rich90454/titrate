// Phase 4: Tree-walking interpreter for the Titrate language
// Precision in every step – richie-rich90454, 2026

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::ast;

mod value;
mod heap;
mod env;
mod execution;
mod eval;
mod methods;
mod operators;
#[cfg(test)]
mod tests;

pub use value::{Value, MethodDecl, ParamDecl};
pub use heap::Memory;
pub use env::{Env, ControlFlow};

/// Map an AST operator to its operator method name for the interpreter.
pub(crate) fn interpreter_operator_method_name(op: &ast::Operator) -> String {
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
        ast::Operator::BitUshr => "operator>>>".to_string(),
        ast::Operator::And | ast::Operator::Or => String::new(), // not overloadable
    }
}

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

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
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
