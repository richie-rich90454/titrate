/// Semantic analyzer for the Titrate language.
/// Every drop matters – richie-rich90454, 2026
///
/// Performs symbol resolution, type checking, ownership analysis,
/// error-propagation validation, and toString desugaring.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::ast;

// ---------------------------------------------------------------------------
// Symbol types
// ---------------------------------------------------------------------------

/// Ownership state of a variable at a program point.
#[derive(Debug, Clone, PartialEq)]
pub enum VarState {
    Live,
    Moved,
    BorrowedImmutable,
    BorrowedMutable,
}

/// A resolved symbol in scope.
#[derive(Debug, Clone)]
pub enum Symbol {
    Variable {
        typ: ast::Type,
        mutable: bool,
    },
    Function(ast::FnDecl),
    Class(ast::ClassDecl),
    Interface(ast::InterfaceDecl),
    Enum(ast::EnumDecl),
    Variant {
        enum_name: String,
        variant_name: String,
        fields: Vec<ast::Param>,
    },
}

// ---------------------------------------------------------------------------
// Scope
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct Scope {
    parent: Option<Rc<RefCell<Scope>>>,
    symbols: HashMap<String, Symbol>,
}

impl Scope {
    fn new(parent: Option<Rc<RefCell<Scope>>>) -> Self {
        Scope {
            parent,
            symbols: HashMap::new(),
        }
    }

    fn define(&mut self, name: String, symbol: Symbol) {
        self.symbols.insert(name, symbol);
    }

    fn lookup(&self, name: &str) -> Option<Symbol> {
        if let Some(sym) = self.symbols.get(name) {
            return Some(sym.clone());
        }
        if let Some(ref p) = self.parent {
            return p.borrow().lookup(name);
        }
        None
    }
}

// ---------------------------------------------------------------------------
// Type helpers
// ---------------------------------------------------------------------------

const INTEGER_TYPES: &[&str] = &[
    "byte", "short", "int", "long", "vast", "uvast", "u8", "u16", "u32", "u64", "size",
];

const FLOAT_TYPES: &[&str] = &["float", "double", "half", "quad"];

fn is_integer_type(t: &ast::Type) -> bool {
    INTEGER_TYPES.contains(&t.name())
}

fn is_float_type(t: &ast::Type) -> bool {
    FLOAT_TYPES.contains(&t.name())
}

fn is_numeric_type(t: &ast::Type) -> bool {
    is_integer_type(t) || is_float_type(t)
}

fn is_bool_type(t: &ast::Type) -> bool {
    t.name() == "bool"
}

fn is_string_type(t: &ast::Type) -> bool {
    t.name() == "string"
}

fn is_owned_type(t: &ast::Type) -> bool {
    t.name() == "Owned"
}

fn is_result_type(t: &ast::Type) -> bool {
    t.name() == "Result"
}

fn is_ref_type(t: &ast::Type) -> bool {
    matches!(t, ast::Type::Ref(_) | ast::Type::MutRef(_))
}

fn is_void_type(t: &ast::Type) -> bool {
    t.name() == "void"
}

fn is_unknown_type(t: &ast::Type) -> bool {
    t.name() == "unknown" || t.name() == "any"
}

/// Map an AST operator to its operator method name (e.g. Add → "operator+").
fn operator_method_name(op: &ast::Operator) -> String {
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

/// Check if the given type is a class that has an operator overload method for the given operator.
fn class_has_operator_method(left_type: &ast::Type, op: &ast::Operator, scope: &Rc<RefCell<Scope>>) -> bool {
    let method_name = operator_method_name(op);
    if method_name.is_empty() {
        return false;
    }
    if let Some(Symbol::Class(class_decl)) = scope.borrow().lookup(left_type.name()) {
        for member in &class_decl.members {
            if let ast::ClassMember::Method(m) = member {
                if m.name == method_name {
                    return true;
                }
            }
        }
    }
    false
}

/// Determine the type of a literal.
fn literal_type(lit: &ast::Literal) -> ast::Type {
    match lit {
        ast::Literal::Int(_) => ast::Type::simple("int"),
        ast::Literal::Float(_) => ast::Type::simple("double"),
        ast::Literal::Bool(_) => ast::Type::simple("bool"),
        ast::Literal::Char(_) => ast::Type::simple("char"),
        ast::Literal::String(_) => ast::Type::simple("string"),
        ast::Literal::Null => ast::Type::simple("void"),
    }
}

/// Check if `source` can be assigned to `target`.
fn is_assignable(source: &ast::Type, target: &ast::Type) -> bool {
    // Same type is always assignable.
    if source == target {
        return true;
    }
    // "any" or "unknown" can be assigned to anything (from builtins).
    if is_unknown_type(source) {
        return true;
    }
    // &mut T can be assigned to &T (immutable ref is a supertype of mutable ref)
    if let (ast::Type::MutRef(src_inner), ast::Type::Ref(tgt_inner)) = (source, target) {
        return is_assignable(src_inner, tgt_inner);
    }
    // &T can be assigned to &T, &mut T to &mut T (same inner type)
    if let (ast::Type::Ref(src_inner), ast::Type::Ref(tgt_inner)) = (source, target) {
        return is_assignable(src_inner, tgt_inner);
    }
    if let (ast::Type::MutRef(src_inner), ast::Type::MutRef(tgt_inner)) = (source, target) {
        return is_assignable(src_inner, tgt_inner);
    }
    // Integer literal (int) can be assigned to any integer type.
    if source.name() == "int" && is_integer_type(target) {
        return true;
    }
    // Float literal (double) can be assigned to any float type.
    if source.name() == "double" && is_float_type(target) {
        return true;
    }
    // null can be assigned to Owned types and reference types.
    if source.name() == "void" && target.name() == "Owned" {
        return true;
    }
    // Owned<T> can be assigned to Owned<T>.
    if source.name() == "Owned" && target.name() == "Owned" {
        if source.params().len() == target.params().len() {
            return source.params() == target.params();
        }
    }
    // new T(...) returns T, which can be assigned to Owned<T>.
    // This handles `let x: Owned<int> = new int(5)`.
    if target.name() == "Owned" {
        if let Some(inner) = target.params().first() {
            if is_assignable(source, inner) {
                return true;
            }
        }
    }
    // Result<any, any> can be assigned to any Result<T, E>.
    if source.name() == "Result" && target.name() == "Result" {
        if source.params().iter().any(|p| is_unknown_type(p)) {
            return true;
        }
    }
    // For the Alpha, we are relaxed: any type name mismatch that isn't caught
    // by the above rules is a type error.
    false
}

/// Map a primitive type name to its static class name for toString desugaring.
fn static_class_for_primitive(t: &ast::Type) -> Option<String> {
    match t.name() {
        "int" => Some("Integer".to_string()),
        "long" => Some("Long".to_string()),
        "short" => Some("Short".to_string()),
        "byte" => Some("Byte".to_string()),
        "vast" => Some("Vast".to_string()),
        "uvast" => Some("Uvast".to_string()),
        "u8" => Some("U8".to_string()),
        "u16" => Some("U16".to_string()),
        "u32" => Some("U32".to_string()),
        "u64" => Some("U64".to_string()),
        "size" => Some("Size".to_string()),
        "float" => Some("Float".to_string()),
        "double" => Some("Double".to_string()),
        "half" => Some("Half".to_string()),
        "quad" => Some("Quad".to_string()),
        "bool" => Some("Boolean".to_string()),
        "char" => Some("Char".to_string()),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// ExhaustiveMode
// ---------------------------------------------------------------------------

/// Controls how non-exhaustive pattern matches are reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExhaustiveMode {
    /// Emit a warning for non-exhaustive matches (default).
    #[default]
    Warning,
    /// Emit an error for non-exhaustive matches.
    Error,
}

// ---------------------------------------------------------------------------
// Analyzer
// ---------------------------------------------------------------------------

struct Analyzer {
    errors: Vec<String>,
    warnings: Vec<String>,
    /// Ownership state per variable in the current function scope.
    /// Keyed by a scope-depth-qualified name to handle shadowing.
    var_states: HashMap<String, VarState>,
    /// Track which variables are locals in the current function (for borrow-checking).
    local_vars: Vec<String>,
    /// Current function return type (for return-checking and ?-operator).
    current_return_type: Option<ast::Type>,
    /// Whether we are inside an unsafe block.
    in_unsafe: bool,
    /// How to report non-exhaustive pattern matches.
    exhaustive_mode: ExhaustiveMode,
}

impl Analyzer {
    fn new() -> Self {
        Analyzer {
            errors: Vec::new(),
            warnings: Vec::new(),
            var_states: HashMap::new(),
            local_vars: Vec::new(),
            current_return_type: None,
            in_unsafe: false,
            exhaustive_mode: ExhaustiveMode::default(),
        }
    }

    fn with_exhaustive_mode(mode: ExhaustiveMode) -> Self {
        Analyzer {
            errors: Vec::new(),
            warnings: Vec::new(),
            var_states: HashMap::new(),
            local_vars: Vec::new(),
            current_return_type: None,
            in_unsafe: false,
            exhaustive_mode: mode,
        }
    }

    fn error(&mut self, msg: impl Into<String>) {
        self.errors.push(msg.into());
    }

    fn warn(&mut self, msg: impl Into<String>) {
        self.warnings.push(msg.into());
    }

    // -----------------------------------------------------------------------
    // Top-level analysis
    // -----------------------------------------------------------------------

    fn analyze_program(&mut self, program: &mut ast::Program) {
        let global_scope = Rc::new(RefCell::new(Scope::new(None)));

        // Register built-in symbols (io, Integer, Double, etc.)
        self.register_builtins(&global_scope);

        // First pass: register all top-level declarations.
        for decl in &program.declarations {
            self.register_declaration(decl, &global_scope);
        }

        // Second pass: analyze each declaration body.
        for decl in &mut program.declarations {
            self.analyze_declaration(decl, &global_scope);
        }
    }

    /// Register built-in names that the interpreter provides.
    fn register_builtins(&self, scope: &Rc<RefCell<Scope>>) {
        // Built-in objects (used as namespaces for static methods)
        for name in &["io", "Integer", "Double", "Float", "Long", "Byte", "Short",
                       "Half", "Quad", "Vast", "Uvast", "Boolean", "Char",
                       "malloc", "free"] {
            scope.borrow_mut().define(
                name.to_string(),
                Symbol::Variable {
                    typ: ast::Type::simple(name),
                    mutable: false,
                },
            );
        }
        // Built-in classes (used with `new`)
        for name in &["ArrayList", "HashMap"] {
            scope.borrow_mut().define(
                name.to_string(),
                Symbol::Class(ast::ClassDecl {
                    name: name.to_string(),
                    type_params: vec![],
                    parent: None,
                    ifaces: vec![],
                    members: vec![],
                    span: ast::Span::unknown(),
                }),
            );
        }
        // Result constructors
        for name in &["Ok", "Err"] {
            scope.borrow_mut().define(
                name.to_string(),
                Symbol::Function(ast::FnDecl {
                    access: ast::Access::Public,
                    name: name.to_string(),
                    type_params: vec![],
                    params: vec![ast::Param {
                        name: "value".to_string(),
                        typ: ast::Type::simple("any"),
                    }],
                    return_type: Some(ast::Type::generic("Result", vec![
                        ast::Type::simple("any"),
                        ast::Type::simple("any"),
                    ])),
                    body: vec![],
                    sugar: false,
                    where_clause: vec![],
                    span: ast::Span::unknown(),
                }),
            );
        }
    }

    fn register_declaration(&mut self, decl: &ast::Declaration, scope: &Rc<RefCell<Scope>>) {
        match decl {
            ast::Declaration::Function(f) => {
                scope.borrow_mut().define(f.name.clone(), Symbol::Function(f.clone()));
            }
            ast::Declaration::Class(c) => {
                scope.borrow_mut().define(c.name.clone(), Symbol::Class(c.clone()));
                // Register class members in a sub-scope.
                let class_scope = Rc::new(RefCell::new(Scope::new(Some(scope.clone()))));
                for member in &c.members {
                    match member {
                        ast::ClassMember::Method(m) => {
                            let fn_decl = ast::FnDecl {
                                access: m.access.clone(),
                                name: m.name.clone(),
                                type_params: m.type_params.clone(),
                                params: m.params.clone(),
                                return_type: m.return_type.clone(),
                                body: m.body.clone(),
                                sugar: false,
                                where_clause: m.where_clause.clone(),
                                span: m.span,
                            };
                            class_scope.borrow_mut().define(m.name.clone(), Symbol::Function(fn_decl));
                        }
                        ast::ClassMember::Constructor(m) => {
                            let fn_decl = ast::FnDecl {
                                access: m.access.clone(),
                                name: m.name.clone(),
                                type_params: m.type_params.clone(),
                                params: m.params.clone(),
                                return_type: m.return_type.clone(),
                                body: m.body.clone(),
                                sugar: false,
                                where_clause: m.where_clause.clone(),
                                span: m.span,
                            };
                            class_scope.borrow_mut().define(m.name.clone(), Symbol::Function(fn_decl));
                        }
                        ast::ClassMember::Field(_) => {
                            // Fields are accessed via `this`, not as bare names.
                        }
                    }
                }
                // Store the class scope as a child — we won't look it up later
                // but it ensures the symbols exist for reference.
                let _ = class_scope;
            }
            ast::Declaration::Interface(i) => {
                scope.borrow_mut().define(i.name.clone(), Symbol::Interface(i.clone()));
            }
            ast::Declaration::Enum(e) => {
                scope.borrow_mut().define(e.name.clone(), Symbol::Enum(e.clone()));
                // Register each variant as a symbol.
                for variant in &e.variants {
                    scope.borrow_mut().define(
                        variant.name.clone(),
                        Symbol::Variant {
                            enum_name: e.name.clone(),
                            variant_name: variant.name.clone(),
                            fields: variant.fields.clone(),
                        },
                    );
                }
            }
            ast::Declaration::VarDecl(v) => {
                if let Some(ref typ) = v.typ {
                    scope.borrow_mut().define(
                        v.name.clone(),
                        Symbol::Variable {
                            typ: typ.clone(),
                            mutable: v.mutable,
                        },
                    );
                } else if let Some(ref init) = v.init {
                    let typ = self.infer_expr_type(init, scope);
                    scope.borrow_mut().define(
                        v.name.clone(),
                        Symbol::Variable {
                            typ,
                            mutable: v.mutable,
                        },
                    );
                }
            }
            ast::Declaration::ConstDecl(v) => {
                if let Some(ref typ) = v.typ {
                    scope.borrow_mut().define(
                        v.name.clone(),
                        Symbol::Variable {
                            typ: typ.clone(),
                            mutable: false,
                        },
                    );
                } else if let Some(ref init) = v.init {
                    let typ = self.infer_expr_type(init, scope);
                    scope.borrow_mut().define(
                        v.name.clone(),
                        Symbol::Variable {
                            typ,
                            mutable: false,
                        },
                    );
                }
            }
        }
    }

    fn analyze_declaration(&mut self, decl: &mut ast::Declaration, scope: &Rc<RefCell<Scope>>) {
        match decl {
            ast::Declaration::Function(f) => {
                self.analyze_fn_decl(f, scope);
            }
            ast::Declaration::Class(c) => {
                let class_scope = Rc::new(RefCell::new(Scope::new(Some(scope.clone()))));
                class_scope.borrow_mut().define("this".to_string(), Symbol::Variable {
                    typ: ast::Type::simple(&c.name),
                    mutable: false,
                });
                // "self" is an alias for "this" in method bodies
                class_scope.borrow_mut().define("self".to_string(), Symbol::Variable {
                    typ: ast::Type::simple(&c.name),
                    mutable: false,
                });
                for member in &mut c.members {
                    match member {
                        ast::ClassMember::Method(m) => {
                            self.analyze_method(m, &class_scope);
                        }
                        ast::ClassMember::Constructor(m) => {
                            self.analyze_method(m, &class_scope);
                        }
                        ast::ClassMember::Field(f) => {
                            if let Some(ref mut init) = f.init {
                                self.analyze_expr(init, &class_scope);
                            }
                        }
                    }
                }
            }
            ast::Declaration::Interface(_) => {
                // Interfaces have no bodies to analyze.
            }
            ast::Declaration::Enum(_) => {
                // Enums have no bodies to analyze.
            }
            ast::Declaration::VarDecl(v) => {
                self.analyze_var_decl(v, scope);
            }
            ast::Declaration::ConstDecl(v) => {
                self.analyze_var_decl(v, scope);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Function / method analysis
    // -----------------------------------------------------------------------

    fn analyze_fn_decl(&mut self, f: &mut ast::FnDecl, scope: &Rc<RefCell<Scope>>) {
        let fn_scope = Rc::new(RefCell::new(Scope::new(Some(scope.clone()))));
        let prev_return = self.current_return_type.clone();
        self.current_return_type = f.return_type.clone();

        // Reset ownership state for this function.
        let prev_states = std::mem::take(&mut self.var_states);
        let prev_locals = std::mem::take(&mut self.local_vars);

        for param in &f.params {
            fn_scope.borrow_mut().define(
                param.name.clone(),
                Symbol::Variable {
                    typ: param.typ.clone(),
                    mutable: false,
                },
            );
            self.var_states.insert(param.name.clone(), VarState::Live);
            self.local_vars.push(param.name.clone());
        }

        self.analyze_block(&mut f.body, &fn_scope);

        // Restore.
        self.var_states = prev_states;
        self.local_vars = prev_locals;
        self.current_return_type = prev_return;
    }

    fn analyze_method(&mut self, m: &mut ast::MethodDecl, scope: &Rc<RefCell<Scope>>) {
        let method_scope = Rc::new(RefCell::new(Scope::new(Some(scope.clone()))));
        let prev_return = self.current_return_type.clone();
        self.current_return_type = m.return_type.clone();

        let prev_states = std::mem::take(&mut self.var_states);
        let prev_locals = std::mem::take(&mut self.local_vars);

        for param in &m.params {
            method_scope.borrow_mut().define(
                param.name.clone(),
                Symbol::Variable {
                    typ: param.typ.clone(),
                    mutable: false,
                },
            );
            self.var_states.insert(param.name.clone(), VarState::Live);
            self.local_vars.push(param.name.clone());
        }

        self.analyze_block(&mut m.body, &method_scope);

        self.var_states = prev_states;
        self.local_vars = prev_locals;
        self.current_return_type = prev_return;
    }

    // -----------------------------------------------------------------------
    // Block / statement analysis
    // -----------------------------------------------------------------------

    fn analyze_block(&mut self, block: &mut ast::Block, scope: &Rc<RefCell<Scope>>) {
        let block_scope = Rc::new(RefCell::new(Scope::new(Some(scope.clone()))));
        for stmt in block.iter_mut() {
            self.analyze_stmt(stmt, &block_scope);
        }
    }

    fn analyze_stmt(&mut self, stmt: &mut ast::Stmt, scope: &Rc<RefCell<Scope>>) {
        match stmt {
            ast::Stmt::Block(b) => {
                self.analyze_block(b, scope);
            }
            ast::Stmt::Expr(e) => {
                self.analyze_expr(e, scope);
            }
            ast::Stmt::If(if_stmt) => {
                self.analyze_expr(&mut if_stmt.condition, scope);
                let cond_type = self.infer_expr_type(&if_stmt.condition, scope);
                if !is_bool_type(&cond_type) {
                    self.error(format!(
                        "if condition must be bool, found {}",
                        cond_type
                    ));
                }
                self.analyze_block(&mut if_stmt.then_branch, scope);
                if let Some(ref mut else_b) = if_stmt.else_branch {
                    self.analyze_block(else_b, scope);
                }
            }
            ast::Stmt::While(while_stmt) => {
                self.analyze_expr(&mut while_stmt.condition, scope);
                let cond_type = self.infer_expr_type(&while_stmt.condition, scope);
                if !is_bool_type(&cond_type) {
                    self.error(format!(
                        "while condition must be bool, found {}",
                        cond_type
                    ));
                }
                self.analyze_block(&mut while_stmt.body, scope);
            }
            ast::Stmt::WhileLet(while_let_stmt) => {
                self.analyze_expr(&mut while_let_stmt.expr, scope);
                let while_let_scope = Rc::new(RefCell::new(Scope::new(Some(scope.clone()))));
                // Register the loop variable in the scope
                let expr_type = self.infer_expr_type(&while_let_stmt.expr, scope);
                let var_type = if is_result_type(&expr_type) {
                    if let Some(ok_type) = expr_type.params().first() {
                        ok_type.clone()
                    } else {
                        ast::Type::simple("any")
                    }
                } else {
                    expr_type
                };
                while_let_scope.borrow_mut().define(
                    while_let_stmt.var_name.clone(),
                    Symbol::Variable {
                        typ: var_type,
                        mutable: false,
                    },
                );
                self.var_states.insert(while_let_stmt.var_name.clone(), VarState::Live);
                self.local_vars.push(while_let_stmt.var_name.clone());
                self.analyze_block(&mut while_let_stmt.body, &while_let_scope);
                self.var_states.remove(&while_let_stmt.var_name);
                self.local_vars.retain(|v| v != &while_let_stmt.var_name);
            }
            ast::Stmt::For(for_stmt) => {
                let for_scope = Rc::new(RefCell::new(Scope::new(Some(scope.clone()))));
                self.analyze_expr(&mut for_stmt.iterable, scope);
                let iter_type = self.infer_expr_type(&for_stmt.iterable, scope);
                for_scope.borrow_mut().define(
                    for_stmt.var.clone(),
                    Symbol::Variable {
                        // For now, use the iterable's element type or the iterable type itself.
                        typ: iter_type,
                        mutable: false,
                    },
                );
                self.var_states.insert(for_stmt.var.clone(), VarState::Live);
                self.local_vars.push(for_stmt.var.clone());
                self.analyze_block(&mut for_stmt.body, &for_scope);
                self.var_states.remove(&for_stmt.var);
                self.local_vars.retain(|v| v != &for_stmt.var);
            }
            ast::Stmt::CFor(cfor_stmt) => {
                let cfor_scope = Rc::new(RefCell::new(Scope::new(Some(scope.clone()))));
                if let Some(ref mut init) = cfor_stmt.init {
                    self.analyze_stmt(init, &cfor_scope);
                }
                if let Some(ref mut cond) = cfor_stmt.condition {
                    self.analyze_expr(cond, &cfor_scope);
                    let cond_type = self.infer_expr_type(cond, &cfor_scope);
                    if !is_bool_type(&cond_type) {
                        self.error(format!(
                            "for condition must be bool, found {}",
                            cond_type
                        ));
                    }
                }
                self.analyze_block(&mut cfor_stmt.body, &cfor_scope);
                if let Some(ref mut incr) = cfor_stmt.increment {
                    self.analyze_expr(incr, &cfor_scope);
                }
            }
            ast::Stmt::Return(opt_expr) => {
                if let Some(ref mut expr) = opt_expr {
                    self.analyze_expr(expr, scope);
                    let ret_type = self.infer_expr_type(expr, scope);
                    if let Some(ref expected) = self.current_return_type {
                        if !is_assignable(&ret_type, expected) && !is_void_type(expected) {
                            self.error(format!(
                                "return type mismatch: expected {}, found {}",
                                expected, ret_type
                            ));
                        }
                    }
                } else if let Some(ref expected) = self.current_return_type {
                    if !is_void_type(expected) {
                        self.error(format!(
                            "function with return type {} must return a value",
                            expected
                        ));
                    }
                }

                // Check for returning a borrow of a local.
                if let Some(ref expr) = opt_expr {
                    if self.expr_borrows_local(expr) {
                        self.error("cannot return a borrow of a local variable".to_string());
                    }
                }
            }
            ast::Stmt::Break => {}
            ast::Stmt::Continue => {}
            ast::Stmt::Switch(sw) => {
                self.analyze_switch(sw, scope);
            }
            ast::Stmt::VarDecl(v) => {
                self.analyze_var_decl(v, scope);
            }
            ast::Stmt::ConstDecl(v) => {
                self.analyze_var_decl(v, scope);
            }
            ast::Stmt::TupleDestructure { names, expr, mutable: _, span: _ } => {
                self.analyze_expr(expr, scope);
                let expr_type = self.infer_expr_type(expr, scope);
                // Check that the expression is a tuple type
                if let ast::Type::Tuple(element_types) = &expr_type {
                    if names.len() != element_types.len() {
                        self.error(format!(
                            "tuple destructuring expects {} elements, found {}",
                            element_types.len(),
                            names.len()
                        ));
                    }
                } else if !is_unknown_type(&expr_type) {
                    self.error(format!(
                        "tuple destructuring requires a tuple type, found {}",
                        expr_type
                    ));
                }
                // Define each variable in scope
                let element_types = match &expr_type {
                    ast::Type::Tuple(types) => types.clone(),
                    _ => vec![ast::Type::simple("any"); names.len()],
                };
                for (i, name) in names.iter().enumerate() {
                    let typ = element_types.get(i).cloned().unwrap_or_else(|| ast::Type::simple("any"));
                    scope.borrow_mut().define(
                        name.clone(),
                        Symbol::Variable { typ, mutable: false },
                    );
                    self.var_states.insert(name.clone(), VarState::Live);
                    self.local_vars.push(name.clone());
                }
            }
        }
    }

    fn analyze_var_decl(&mut self, v: &mut ast::VarDecl, scope: &Rc<RefCell<Scope>>) {
        // Analyze the initializer first.
        if let Some(ref mut init) = v.init {
            self.analyze_expr(init, scope);
            let init_type = self.infer_expr_type(init, scope);

            if let Some(ref declared) = v.typ {
                // Type check: initializer must be assignable to declared type.
                if !is_assignable(&init_type, declared) {
                    self.error(format!(
                        "type mismatch: cannot assign {} to {}",
                        init_type, declared
                    ));
                }
            } else {
                // Infer type from initializer.
                v.typ = Some(init_type);
            }

            // Move tracking: if the initializer is an Owned variable, mark it as moved.
            if !self.in_unsafe {
                if let ast::Expr::Identifier(src_name, _) = init {
                    let src_sym = scope.borrow().lookup(src_name);
                    if let Some(Symbol::Variable { typ, .. }) = src_sym {
                        if is_owned_type(&typ) {
                            self.var_states.insert(src_name.clone(), VarState::Moved);
                        }
                    }
                }
            }
        }

        // Register in scope.
        if let Some(ref typ) = v.typ {
            scope.borrow_mut().define(
                v.name.clone(),
                Symbol::Variable {
                    typ: typ.clone(),
                    mutable: v.mutable,
                },
            );
            self.var_states.insert(v.name.clone(), VarState::Live);
            self.local_vars.push(v.name.clone());
        }
    }

    fn analyze_switch(&mut self, sw: &mut ast::SwitchStmt, scope: &Rc<RefCell<Scope>>) {
        self.analyze_expr(&mut sw.expr, scope);
        let expr_type = self.infer_expr_type(&sw.expr, scope);

        // The matched expression should be an enum type.
        let enum_name = expr_type.name().to_string();
        let is_enum = scope.borrow().lookup(&enum_name).map_or(false, |sym| {
            matches!(sym, Symbol::Enum(_))
        });

        for case in &mut sw.cases {
            // Create a new scope for each case body so pattern bindings are visible
            let case_scope = Rc::new(RefCell::new(Scope::new(Some(scope.clone()))));

            match &case.pattern {
                ast::Pattern::Literal(_) => {
                    // Fine for primitive matching.
                }
                ast::Pattern::Wildcard => {}
                ast::Pattern::Constructor { name, bindings } => {
                    if is_enum {
                        // Verify that the constructor name is a variant of the enum.
                        let variant_sym = scope.borrow().lookup(name);
                        match variant_sym {
                            Some(Symbol::Variant { enum_name: en, .. }) => {
                                if en != enum_name {
                                    self.error(format!(
                                        "variant {} does not belong to enum {}",
                                        name, enum_name
                                    ));
                                }
                            }
                            Some(_) => {
                                self.error(format!("{} is not an enum variant", name));
                            }
                            None => {
                                // Could be Ok/Err — allow for Result matching
                            }
                        }
                    }

                    // Register pattern bindings in the case scope
                    for binding in bindings {
                        case_scope.borrow_mut().define(
                            binding.clone(),
                            Symbol::Variable {
                                typ: ast::Type::simple("any"),
                                mutable: false,
                            },
                        );
                        self.var_states.insert(binding.clone(), VarState::Live);
                        self.local_vars.push(binding.clone());
                    }
                }
            }
            self.analyze_block(&mut case.body, &case_scope);

            // Clean up pattern bindings from ownership tracking
            if let ast::Pattern::Constructor { bindings, .. } = &case.pattern {
                for binding in bindings {
                    self.var_states.remove(binding);
                    self.local_vars.retain(|v| v != binding);
                }
            }
        }

        if let Some(ref mut default) = sw.default {
            self.analyze_block(default, scope);
        }

        // Exhaustiveness check: if switching on an enum with no default,
        // verify that all variants are covered.
        if is_enum && sw.default.is_none() {
            // Collect the names of all variants covered by Constructor patterns.
            let covered: Vec<&str> = sw.cases.iter().filter_map(|case| {
                if let ast::Pattern::Constructor { name, .. } = &case.pattern {
                    Some(name.as_str())
                } else {
                    None
                }
            }).collect();

            // Look up the enum definition to get all variant names.
            if let Some(Symbol::Enum(enum_decl)) = scope.borrow().lookup(&enum_name) {
                let all_variants: Vec<&str> = enum_decl.variants.iter()
                    .map(|v| v.name.as_str())
                    .collect();
                let missing: Vec<&str> = all_variants.iter()
                    .filter(|v| !covered.contains(v))
                    .copied()
                    .collect();

                if !missing.is_empty() {
                    let msg = format!(
                        "non-exhaustive pattern match: missing variant{} {}",
                        if missing.len() > 1 { "s" } else { "" },
                        missing.join(", ")
                    );
                    match self.exhaustive_mode {
                        ExhaustiveMode::Warning => self.warn(msg),
                        ExhaustiveMode::Error => self.error(msg),
                    }
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Expression analysis
    // -----------------------------------------------------------------------

    fn analyze_expr(&mut self, expr: &mut ast::Expr, scope: &Rc<RefCell<Scope>>) {
        match expr {
            ast::Expr::Literal(_, _) => {}
            ast::Expr::Unit(_) => {}
            ast::Expr::Identifier(name, _) => {
                // Symbol resolution.
                let sym = scope.borrow().lookup(name);
                match sym {
                    None => {
                        self.error(format!("undeclared identifier: {}", name));
                    }
                    Some(Symbol::Variable { .. }) => {
                        // Ownership check: is the variable still live?
                        if !self.in_unsafe {
                            if let Some(state) = self.var_states.get(name) {
                                match state {
                                    VarState::Moved => {
                                        self.error(format!(
                                            "use of moved variable: {}",
                                            name
                                        ));
                                    }
                                    VarState::BorrowedMutable => {
                                        // Reading while mutably borrowed is an error.
                                        self.error(format!(
                                            "cannot read variable {} while it is mutably borrowed",
                                            name
                                        ));
                                    }
                                    VarState::Live | VarState::BorrowedImmutable => {}
                                }
                            }
                        }
                    }
                    Some(Symbol::Variant { .. }) => {
                        // Variant name used as constructor — fine.
                    }
                    Some(Symbol::Function(_)) => {
                        // Function name used as value — fine.
                    }
                    Some(Symbol::Class(_)) => {
                        // Class name used as type/constructor — fine.
                    }
                    Some(Symbol::Interface(_)) => {
                        // Interface name — fine.
                    }
                    Some(Symbol::Enum(_)) => {
                        // Enum name — fine.
                    }
                }
            }
            ast::Expr::Binary(left, op, right, _) => {
                self.analyze_expr(left, scope);
                self.analyze_expr(right, scope);

                let left_type = self.infer_expr_type(left, scope);
                let right_type = self.infer_expr_type(right, scope);

                // Check if the left type is a class with an operator overload method
                let has_operator_overload = class_has_operator_method(&left_type, op, scope);

                // Skip type checking when types are unknown (from builtins, field access, etc.)
                // or when the left operand has an operator overload method
                if has_operator_overload {
                    // Operator overload method exists — fine
                } else if is_unknown_type(&left_type) || is_unknown_type(&right_type) {
                    // Cannot verify — skip
                } else {
                match op {
                    ast::Operator::Add => {
                        // Numeric addition or string concatenation.
                        if is_numeric_type(&left_type) && is_numeric_type(&right_type) {
                            // Fine.
                        } else if is_string_type(&left_type) || is_string_type(&right_type) {
                            // String concatenation — fine.
                        } else {
                            self.error(format!(
                                "operator + cannot be applied to {} and {}",
                                left_type, right_type
                            ));
                        }
                    }
                    ast::Operator::Sub
                    | ast::Operator::Mul
                    | ast::Operator::Div
                    | ast::Operator::Mod => {
                        if !is_numeric_type(&left_type) || !is_numeric_type(&right_type) {
                            self.error(format!(
                                "arithmetic operator requires numeric operands, found {} and {}",
                                left_type, right_type
                            ));
                        }
                    }
                    ast::Operator::Eq
                    | ast::Operator::Ne
                    | ast::Operator::Lt
                    | ast::Operator::Gt
                    | ast::Operator::Le
                    | ast::Operator::Ge => {
                        // Comparison operators — fine for most types.
                    }
                    ast::Operator::And | ast::Operator::Or => {
                        if !is_bool_type(&left_type) || !is_bool_type(&right_type) {
                            self.error(format!(
                                "logical operator requires bool operands, found {} and {}",
                                left_type, right_type
                            ));
                        }
                    }
                    ast::Operator::BitAnd
                    | ast::Operator::BitOr
                    | ast::Operator::BitXor
                    | ast::Operator::BitShl
                    | ast::Operator::BitShr => {
                        if !is_integer_type(&left_type) || !is_integer_type(&right_type) {
                            self.error(format!(
                                "bitwise operator requires integer operands, found {} and {}",
                                left_type, right_type
                            ));
                        }
                    }
                }
                }
            }
            ast::Expr::Unary(unop, operand, _) => {
                self.analyze_expr(operand, scope);
                let operand_type = self.infer_expr_type(operand, scope);
                match unop {
                    ast::UnOp::Neg => {
                        if !is_numeric_type(&operand_type) {
                            self.error(format!(
                                "unary - requires numeric operand, found {}",
                                operand_type
                            ));
                        }
                    }
                    ast::UnOp::Not => {
                        if !is_bool_type(&operand_type) {
                            self.error(format!(
                                "unary ! requires bool operand, found {}",
                                operand_type
                            ));
                        }
                    }
                    ast::UnOp::BitNot => {
                        if !is_integer_type(&operand_type) {
                            self.error(format!(
                                "unary ~ requires integer operand, found {}",
                                operand_type
                            ));
                        }
                    }
                }
            }
            ast::Expr::Call(callee, args, _) => {
                // Check for toString desugaring BEFORE analyzing (which borrows immutably).
                // We need to detect: Call(MemberAccess(obj, "toString"), [])
                let desugar_info: Option<(String, ast::Expr)> = match callee.as_ref() {
                    ast::Expr::MemberAccess(obj, method, _) if method == "toString" => {
                        let obj_type = self.infer_expr_type(obj, scope);
                        static_class_for_primitive(&obj_type).map(|class_name| {
                            // We'll replace the whole expression after this match.
                            (class_name, *obj.clone())
                        })
                    }
                    _ => None,
                };

                if let Some((class_name, obj_expr)) = desugar_info {
                    *expr = ast::Expr::StaticCall {
                        class_name,
                        method: "toString".to_string(),
                        args: vec![obj_expr],
                        span: ast::Span::unknown(),
                    };
                    // Re-analyze the desugared form.
                    self.analyze_expr(expr, scope);
                    return;
                }

                self.analyze_expr(callee, scope);
                for arg in args.iter_mut() {
                    self.analyze_expr(arg, scope);
                }

                // Check if callee is a function and validate argument count.
                match callee.as_ref() {
                    ast::Expr::Identifier(name, _) => {
                        if let Some(Symbol::Function(f)) = scope.borrow().lookup(name) {
                            if args.len() != f.params.len() {
                                self.error(format!(
                                    "function {} expects {} arguments, found {}",
                                    name,
                                    f.params.len(),
                                    args.len()
                                ));
                            }
                        }
                    }
                    ast::Expr::MemberAccess(obj, method, _) => {
                        // Check method calls on known types.
                        let obj_type = self.infer_expr_type(obj, scope);
                        if let Some(Symbol::Class(class_decl)) = scope.borrow().lookup(obj_type.name()) {
                            for member in &class_decl.members {
                                if let ast::ClassMember::Method(m) = member {
                                    if m.name == *method {
                                        if args.len() != m.params.len() {
                                            self.error(format!(
                                                "method {} expects {} arguments, found {}",
                                                method,
                                                m.params.len(),
                                                args.len()
                                            ));
                                        }
                                        break;
                                    }
                                }
                                if let ast::ClassMember::Constructor(m) = member {
                                    if m.name == *method {
                                        if args.len() != m.params.len() {
                                            self.error(format!(
                                                "constructor {} expects {} arguments, found {}",
                                                method,
                                                m.params.len(),
                                                args.len()
                                            ));
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            ast::Expr::MemberAccess(obj, field, _) => {
                self.analyze_expr(obj.as_mut(), scope);
                // If it's a method call, it will be handled in the Call branch above.
                // For field access, we just validate the object is not moved.
                let _ = field;
            }
            ast::Expr::Index(obj, idx, _) => {
                self.analyze_expr(obj.as_mut(), scope);
                self.analyze_expr(idx.as_mut(), scope);
            }
            ast::Expr::New(typ, args, _) => {
                for arg in args.iter_mut() {
                    self.analyze_expr(arg, scope);
                }
                // Verify the type exists.
                let type_name = typ.name();
                let sym = scope.borrow().lookup(type_name);
                match sym {
                    None => {
                        // Could be a primitive type like int, bool, etc.
                        if !INTEGER_TYPES.contains(&type_name)
                            && !FLOAT_TYPES.contains(&type_name)
                            && type_name != "bool"
                            && type_name != "char"
                            && type_name != "string"
                        {
                            self.error(format!("undeclared type: {}", type_name));
                        }
                    }
                    Some(Symbol::Class(_)) => {}
                    Some(Symbol::Enum(_)) => {}
                    Some(Symbol::Interface(_)) => {}
                    Some(Symbol::Function(_)) => {
                        self.error(format!("{} is a function, not a type", type_name));
                    }
                    Some(Symbol::Variable { .. }) => {
                        self.error(format!("{} is a variable, not a type", type_name));
                    }
                    Some(Symbol::Variant { .. }) => {
                        self.error(format!("{} is a variant, not a type", type_name));
                    }
                }
            }
            ast::Expr::This(_) => {}
            ast::Expr::Super(_) => {}
            ast::Expr::OwnedDeref(inner, _) => {
                self.analyze_expr(inner, scope);
                let inner_type = self.infer_expr_type(inner, scope);
                if !is_owned_type(&inner_type) && !is_unknown_type(&inner_type) {
                    self.error(format!(
                        "owned dereference requires Owned type, found {}",
                        inner_type
                    ));
                }
            }
            ast::Expr::RegionAlloc(_typ, region_expr, _) => {
                self.analyze_expr(region_expr, scope);
                // Track that this allocation belongs to the current region.
                // We'll check that region-allocated values don't escape.
            }
            ast::Expr::RefExpr(inner, ref_kind, _) => {
                self.analyze_expr(inner, scope);

                if self.in_unsafe {
                    return;
                }

                // Borrow checking.
                match inner.as_ref() {
                    ast::Expr::Identifier(name, _) => {
                        match ref_kind {
                            ast::RefKind::Immutable => {
                                if let Some(state) = self.var_states.get(name) {
                                    match state {
                                        VarState::Moved => {
                                            self.error(format!(
                                                "cannot borrow moved variable: {}",
                                                name
                                            ));
                                        }
                                        VarState::BorrowedMutable => {
                                            self.error(format!(
                                                "cannot immutably borrow {} while it is mutably borrowed",
                                                name
                                            ));
                                        }
                                        VarState::Live | VarState::BorrowedImmutable => {
                                            self.var_states.insert(name.clone(), VarState::BorrowedImmutable);
                                        }
                                    }
                                }
                            }
                            ast::RefKind::Mutable => {
                                if let Some(state) = self.var_states.get(name) {
                                    match state {
                                        VarState::Moved => {
                                            self.error(format!(
                                                "cannot mutably borrow moved variable: {}",
                                                name
                                            ));
                                        }
                                        VarState::BorrowedImmutable => {
                                            self.error(format!(
                                                "cannot mutably borrow {} while it is immutably borrowed",
                                                name
                                            ));
                                        }
                                        VarState::BorrowedMutable => {
                                            self.error(format!(
                                                "cannot mutably borrow {} more than once",
                                                name
                                            ));
                                        }
                                        VarState::Live => {
                                            self.var_states.insert(name.clone(), VarState::BorrowedMutable);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        // Borrowing non-identifier expressions — fine for now.
                    }
                }
            }
            ast::Expr::UnsafeBlock(block, _) => {
                let prev_unsafe = self.in_unsafe;
                self.in_unsafe = true;
                self.analyze_block(block, scope);
                self.in_unsafe = prev_unsafe;
            }
            ast::Expr::ErrorPropagation(inner, _) => {
                self.analyze_expr(inner, scope);
                let inner_type = self.infer_expr_type(inner, scope);
                // The operand must be a Result type.
                if !is_result_type(&inner_type) {
                    // For Alpha, we relax this — the interpreter handles it.
                }
                // The function return type must be a Result.
                if let Some(ref ret) = self.current_return_type {
                    if !is_result_type(ret) {
                        self.error(format!(
                            "? operator can only be used in functions returning Result, found return type {}",
                            ret
                        ));
                    }
                } else {
                    self.error("? operator used in function with no return type".to_string());
                }
            }
            ast::Expr::Cast(inner, target_type, _) => {
                self.analyze_expr(inner, scope);
                let inner_type = self.infer_expr_type(inner, scope);
                // Casts between numeric types are fine.
                if is_numeric_type(&inner_type) && is_numeric_type(target_type) {
                    // Fine.
                } else if is_numeric_type(&inner_type) && is_bool_type(target_type) {
                    // Fine (non-zero to true).
                } else if is_bool_type(&inner_type) && is_numeric_type(target_type) {
                    // Fine.
                } else {
                    self.error(format!(
                        "cannot cast from {} to {}",
                        inner_type, target_type
                    ));
                }
            }
            ast::Expr::StaticCall { class_name, method, args, span: _ } => {
                for arg in args.iter_mut() {
                    self.analyze_expr(arg, scope);
                }
                // Verify the class exists.
                let sym = scope.borrow().lookup(class_name);
                match sym {
                    None => {
                        // Could be a built-in like Integer, Boolean, etc.
                        // We don't error on these for the Alpha.
                    }
                    Some(Symbol::Class(_)) => {}
                    Some(Symbol::Variable { .. }) => {
                        // Built-in type wrappers like Integer, Double, etc.
                        // are registered as Variable symbols. Allow StaticCall on them.
                        let builtin_wrappers = [
                            "Integer", "Double", "Float", "Long", "Byte", "Short",
                            "Half", "Quad", "Vast", "Uvast", "Boolean", "Char",
                            "String_", "io", "Result", "Ok", "Err",
                        ];
                        if !builtin_wrappers.contains(&class_name.as_str()) {
                            self.error(format!("{} is not a class", class_name));
                        }
                    }
                    Some(_) => {
                        self.error(format!("{} is not a class", class_name));
                    }
                }
                let _ = method;
            }
            ast::Expr::Assign(target, value, _) => {
                self.analyze_expr(value, scope);
                let value_type = self.infer_expr_type(value, scope);

                // Analyze the target expression as well.
                self.analyze_expr(target, scope);

                // Check the target is assignable.
                match target.as_ref() {
                    ast::Expr::Identifier(name, _) => {
                        let sym = scope.borrow().lookup(name);
                        match sym {
                            None => {
                                self.error(format!("undeclared identifier in assignment: {}", name));
                            }
                            Some(Symbol::Variable { typ, mutable }) => {
                                if !mutable {
                                    self.error(format!(
                                        "cannot assign to immutable variable: {}",
                                        name
                                    ));
                                }
                                if !is_assignable(&value_type, &typ) {
                                    self.error(format!(
                                        "type mismatch in assignment: cannot assign {} to {}",
                                        value_type, typ
                                    ));
                                }

                                // Ownership check: cannot assign to a borrowed variable.
                                if !self.in_unsafe {
                                    if let Some(state) = self.var_states.get(name) {
                                        match state {
                                            VarState::BorrowedImmutable => {
                                                self.error(format!(
                                                    "cannot assign to {} while it is immutably borrowed",
                                                    name
                                                ));
                                            }
                                            VarState::BorrowedMutable => {
                                                self.error(format!(
                                                    "cannot assign to {} while it is mutably borrowed",
                                                    name
                                                ));
                                            }
                                            VarState::Moved | VarState::Live => {}
                                        }
                                    }
                                }

                                // After assignment, the variable is live again.
                                self.var_states.insert(name.clone(), VarState::Live);
                            }
                            Some(_) => {
                                self.error(format!("cannot assign to non-variable: {}", name));
                            }
                        }
                    }
                    ast::Expr::MemberAccess(_, _, _) | ast::Expr::Index(_, _, _) => {
                        // Already analyzed above.
                    }
                    _ => {
                        self.error("invalid assignment target".to_string());
                    }
                }

                // Check if the value being assigned is a move of an Owned variable.
                if !self.in_unsafe {
                    if let ast::Expr::Identifier(src_name, _) = value.as_ref() {
                        let src_sym = scope.borrow().lookup(src_name);
                        if let Some(Symbol::Variable { typ, .. }) = src_sym {
                            if is_owned_type(&typ) {
                                self.var_states.insert(src_name.clone(), VarState::Moved);
                            }
                        }
                    }
                }
            }
            ast::Expr::Tuple(elements, _) => {
                for elem in elements.iter_mut() {
                    self.analyze_expr(elem, scope);
                }
            }
            ast::Expr::Closure {
                params,
                return_type: _,
                body,
                expr: closure_expr,
                captured_vars,
                span: _,
            } => {
                // Create a new scope for the closure body.
                let closure_scope = Rc::new(RefCell::new(Scope::new(Some(scope.clone()))));

                // Define parameters in the closure scope.
                for (name, typ) in &mut *params {
                    closure_scope.borrow_mut().define(
                        name.clone(),
                        Symbol::Variable {
                            typ: typ.clone(),
                            mutable: false,
                        },
                    );
                }

                // Analyze the closure body in the new scope.
                if let Some(ref mut e) = closure_expr {
                    self.analyze_expr(e, &closure_scope);
                }
                for stmt in body.iter_mut() {
                    self.analyze_stmt(stmt, &closure_scope);
                }

                // Track which variables from outer scopes are referenced.
                // We do a simple scan: any identifier in the closure body
                // that is not a parameter and exists in the outer scope
                // is a captured variable.
                let mut captured = Vec::new();
                self.collect_captured_vars(closure_expr, &params.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>(), scope, &mut captured);
                for stmt in body.iter() {
                    self.collect_captured_vars_from_stmt(stmt, &params.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>(), scope, &mut captured);
                }
                captured.sort();
                captured.dedup();
                *captured_vars = captured;
            }
        }
    }

    /// Collect identifiers from a closure expression that reference variables
    /// in the enclosing scope (i.e., captured variables).
    fn collect_captured_vars(
        &self,
        expr: &Option<Box<ast::Expr>>,
        param_names: &[String],
        outer_scope: &Rc<RefCell<Scope>>,
        captured: &mut Vec<String>,
    ) {
        if let Some(e) = expr {
            self.collect_captured_vars_from_expr(e, param_names, outer_scope, captured);
        }
    }

    fn collect_captured_vars_from_expr(
        &self,
        expr: &ast::Expr,
        param_names: &[String],
        outer_scope: &Rc<RefCell<Scope>>,
        captured: &mut Vec<String>,
    ) {
        match expr {
            ast::Expr::Identifier(name, _) => {
                if !param_names.contains(name) && outer_scope.borrow().lookup(name).is_some() {
                    if let Some(Symbol::Variable { .. }) = outer_scope.borrow().lookup(name) {
                        captured.push(name.clone());
                    }
                }
            }
            ast::Expr::Binary(left, _, right, _) => {
                self.collect_captured_vars_from_expr(left, param_names, outer_scope, captured);
                self.collect_captured_vars_from_expr(right, param_names, outer_scope, captured);
            }
            ast::Expr::Unary(_, operand, _) => {
                self.collect_captured_vars_from_expr(operand, param_names, outer_scope, captured);
            }
            ast::Expr::Call(callee, args, _) => {
                self.collect_captured_vars_from_expr(callee, param_names, outer_scope, captured);
                for arg in args {
                    self.collect_captured_vars_from_expr(arg, param_names, outer_scope, captured);
                }
            }
            ast::Expr::MemberAccess(obj, _, _) => {
                self.collect_captured_vars_from_expr(obj, param_names, outer_scope, captured);
            }
            ast::Expr::Index(obj, index, _) => {
                self.collect_captured_vars_from_expr(obj, param_names, outer_scope, captured);
                self.collect_captured_vars_from_expr(index, param_names, outer_scope, captured);
            }
            ast::Expr::Assign(target, value, _) => {
                self.collect_captured_vars_from_expr(target, param_names, outer_scope, captured);
                self.collect_captured_vars_from_expr(value, param_names, outer_scope, captured);
            }
            ast::Expr::New(_, args, _) => {
                for arg in args {
                    self.collect_captured_vars_from_expr(arg, param_names, outer_scope, captured);
                }
            }
            ast::Expr::OwnedDeref(inner, _) | ast::Expr::ErrorPropagation(inner, _) | ast::Expr::Cast(inner, _, _) => {
                self.collect_captured_vars_from_expr(inner, param_names, outer_scope, captured);
            }
            ast::Expr::RefExpr(inner, _, _) => {
                self.collect_captured_vars_from_expr(inner, param_names, outer_scope, captured);
            }
            ast::Expr::RegionAlloc(_, init, _) => {
                self.collect_captured_vars_from_expr(init, param_names, outer_scope, captured);
            }
            ast::Expr::UnsafeBlock(block, _) => {
                for stmt in block {
                    self.collect_captured_vars_from_stmt(stmt, param_names, outer_scope, captured);
                }
            }
            ast::Expr::StaticCall { args, .. } => {
                for arg in args {
                    self.collect_captured_vars_from_expr(arg, param_names, outer_scope, captured);
                }
            }
            ast::Expr::Closure { body, expr: closure_expr, .. } => {
                if let Some(ref e) = closure_expr {
                    self.collect_captured_vars_from_expr(e, param_names, outer_scope, captured);
                }
                for stmt in body {
                    self.collect_captured_vars_from_stmt(stmt, param_names, outer_scope, captured);
                }
            }
            _ => {}
        }
    }

    fn collect_captured_vars_from_stmt(
        &self,
        stmt: &ast::Stmt,
        param_names: &[String],
        outer_scope: &Rc<RefCell<Scope>>,
        captured: &mut Vec<String>,
    ) {
        match stmt {
            ast::Stmt::Expr(expr) => {
                self.collect_captured_vars_from_expr(expr, param_names, outer_scope, captured);
            }
            ast::Stmt::VarDecl(vd) | ast::Stmt::ConstDecl(vd) => {
                if let Some(ref init) = vd.init {
                    self.collect_captured_vars_from_expr(init, param_names, outer_scope, captured);
                }
            }
            ast::Stmt::If(if_stmt) => {
                self.collect_captured_vars_from_expr(&if_stmt.condition, param_names, outer_scope, captured);
                for s in &if_stmt.then_branch {
                    self.collect_captured_vars_from_stmt(s, param_names, outer_scope, captured);
                }
                if let Some(ref else_branch) = if_stmt.else_branch {
                    for s in else_branch {
                        self.collect_captured_vars_from_stmt(s, param_names, outer_scope, captured);
                    }
                }
            }
            ast::Stmt::While(ws) => {
                self.collect_captured_vars_from_expr(&ws.condition, param_names, outer_scope, captured);
                for s in &ws.body {
                    self.collect_captured_vars_from_stmt(s, param_names, outer_scope, captured);
                }
            }
            ast::Stmt::For(fs) => {
                self.collect_captured_vars_from_expr(&fs.iterable, param_names, outer_scope, captured);
                for s in &fs.body {
                    self.collect_captured_vars_from_stmt(s, param_names, outer_scope, captured);
                }
            }
            ast::Stmt::Return(expr) => {
                if let Some(ref e) = expr {
                    self.collect_captured_vars_from_expr(e, param_names, outer_scope, captured);
                }
            }
            ast::Stmt::Block(block) => {
                for s in block {
                    self.collect_captured_vars_from_stmt(s, param_names, outer_scope, captured);
                }
            }
            ast::Stmt::Switch(ss) => {
                self.collect_captured_vars_from_expr(&ss.expr, param_names, outer_scope, captured);
                for case in &ss.cases {
                    for s in &case.body {
                        self.collect_captured_vars_from_stmt(s, param_names, outer_scope, captured);
                    }
                }
                if let Some(ref default) = ss.default {
                    for s in default {
                        self.collect_captured_vars_from_stmt(s, param_names, outer_scope, captured);
                    }
                }
            }
            ast::Stmt::WhileLet(wls) => {
                self.collect_captured_vars_from_expr(&wls.expr, param_names, outer_scope, captured);
                for s in &wls.body {
                    self.collect_captured_vars_from_stmt(s, param_names, outer_scope, captured);
                }
            }
            ast::Stmt::CFor(cfs) => {
                if let Some(ref init) = cfs.init {
                    self.collect_captured_vars_from_stmt(init, param_names, outer_scope, captured);
                }
                if let Some(ref cond) = cfs.condition {
                    self.collect_captured_vars_from_expr(cond, param_names, outer_scope, captured);
                }
                if let Some(ref inc) = cfs.increment {
                    self.collect_captured_vars_from_expr(inc, param_names, outer_scope, captured);
                }
                for s in &cfs.body {
                    self.collect_captured_vars_from_stmt(s, param_names, outer_scope, captured);
                }
            }
            ast::Stmt::Break | ast::Stmt::Continue => {}
            ast::Stmt::TupleDestructure { expr, .. } => {
                self.collect_captured_vars_from_expr(expr, param_names, outer_scope, captured);
            }
        }
    }

    // -----------------------------------------------------------------------

    fn infer_expr_type(&self, expr: &ast::Expr, scope: &Rc<RefCell<Scope>>) -> ast::Type {
        match expr {
            ast::Expr::Literal(lit, _) => literal_type(lit),
            ast::Expr::Unit(_) => ast::Type::simple("void"),
            ast::Expr::Identifier(name, _) => {
                match scope.borrow().lookup(name) {
                    Some(Symbol::Variable { typ, .. }) => typ,
                    Some(Symbol::Function(f)) => {
                        // Function as a value — we return a function type.
                        // For simplicity, return the return type.
                        f.return_type.clone().unwrap_or(ast::Type::simple("void"))
                    }
                    Some(Symbol::Variant { enum_name, .. }) => {
                        ast::Type::simple(&enum_name)
                    }
                    Some(Symbol::Class(c)) => ast::Type::simple(&c.name),
                    Some(Symbol::Enum(e)) => ast::Type::simple(&e.name),
                    Some(Symbol::Interface(i)) => ast::Type::simple(&i.name),
                    None => ast::Type::simple("unknown"),
                }
            }
            ast::Expr::Binary(left, op, _right, _) => {
                let left_type = self.infer_expr_type(left, scope);
                // Check for operator overload — if found, return the method's return type
                let method_name = operator_method_name(op);
                if !method_name.is_empty() {
                    if let Some(Symbol::Class(class_decl)) = scope.borrow().lookup(left_type.name()) {
                        for member in &class_decl.members {
                            if let ast::ClassMember::Method(m) = member {
                                if m.name == method_name {
                                    return m.return_type.clone().unwrap_or(ast::Type::simple("unknown"));
                                }
                            }
                        }
                    }
                }
                match op {
                    ast::Operator::Add => {
                        if is_string_type(&left_type) {
                            return ast::Type::simple("string");
                        }
                        left_type
                    }
                    ast::Operator::Sub
                    | ast::Operator::Mul
                    | ast::Operator::Div
                    | ast::Operator::Mod => left_type,
                    ast::Operator::Eq
                    | ast::Operator::Ne
                    | ast::Operator::Lt
                    | ast::Operator::Gt
                    | ast::Operator::Le
                    | ast::Operator::Ge => ast::Type::simple("bool"),
                    ast::Operator::And | ast::Operator::Or => ast::Type::simple("bool"),
                    ast::Operator::BitAnd
                    | ast::Operator::BitOr
                    | ast::Operator::BitXor
                    | ast::Operator::BitShl
                    | ast::Operator::BitShr => left_type,
                }
            }
            ast::Expr::Unary(unop, operand, _) => {
                let operand_type = self.infer_expr_type(operand, scope);
                match unop {
                    ast::UnOp::Neg => operand_type,
                    ast::UnOp::Not => ast::Type::simple("bool"),
                    ast::UnOp::BitNot => operand_type,
                }
            }
            ast::Expr::Call(callee, _args, _) => {
                match callee.as_ref() {
                    ast::Expr::Identifier(name, _) => {
                        if let Some(Symbol::Function(f)) = scope.borrow().lookup(name) {
                            f.return_type.clone().unwrap_or(ast::Type::simple("void"))
                        } else if let Some(Symbol::Variant { enum_name, .. }) = scope.borrow().lookup(name) {
                            ast::Type::simple(&enum_name)
                        } else {
                            ast::Type::simple("unknown")
                        }
                    }
                    ast::Expr::MemberAccess(obj, method, _) => {
                        let obj_type = self.infer_expr_type(obj, scope);
                        if let Some(Symbol::Class(class_decl)) = scope.borrow().lookup(obj_type.name()) {
                            for member in &class_decl.members {
                                match member {
                                    ast::ClassMember::Method(m) if m.name == *method => {
                                        return m.return_type.clone().unwrap_or(ast::Type::simple("void"));
                                    }
                                    ast::ClassMember::Constructor(m) if m.name == *method => {
                                        return m.return_type.clone().unwrap_or(ast::Type::simple("void"));
                                    }
                                    _ => {}
                                }
                            }
                        }
                        // toString on primitives returns string.
                        if method == "toString" {
                            return ast::Type::simple("string");
                        }
                        ast::Type::simple("unknown")
                    }
                    _ => ast::Type::simple("unknown"),
                }
            }
            ast::Expr::MemberAccess(obj, _field, _) => {
                let _obj_type = self.infer_expr_type(obj, scope);
                // Without class field info, return unknown.
                ast::Type::simple("unknown")
            }
            ast::Expr::Index(_obj, _idx, _) => {
                // Without array element type info, return unknown.
                ast::Type::simple("unknown")
            }
            ast::Expr::New(typ, _args, _) => typ.clone(),
            ast::Expr::This(_) => {
                // Look up "this" in scope.
                if let Some(Symbol::Variable { typ, .. }) = scope.borrow().lookup("this") {
                    typ
                } else {
                    ast::Type::simple("unknown")
                }
            }
            ast::Expr::Super(_) => {
                if let Some(Symbol::Variable { typ, .. }) = scope.borrow().lookup("this") {
                    typ
                } else {
                    ast::Type::simple("unknown")
                }
            }
            ast::Expr::OwnedDeref(inner, _) => {
                let inner_type = self.infer_expr_type(inner, scope);
                if is_owned_type(&inner_type) {
                    if let Some(inner_param) = inner_type.params().first() {
                        return inner_param.clone();
                    }
                }
                inner_type
            }
            ast::Expr::RegionAlloc(typ, _region, _) => typ.clone(),
            ast::Expr::RefExpr(inner, ref_kind, _) => {
                let inner_type = self.infer_expr_type(inner, scope);
                match ref_kind {
                    ast::RefKind::Immutable => {
                        ast::Type::Ref(Box::new(inner_type))
                    }
                    ast::RefKind::Mutable => {
                        ast::Type::MutRef(Box::new(inner_type))
                    }
                }
            }
            ast::Expr::UnsafeBlock(block, _) => {
                // Type of an unsafe block is the type of its last expression.
                if let Some(last_stmt) = block.last() {
                    match last_stmt {
                        ast::Stmt::Expr(e) => self.infer_expr_type(e, scope),
                        _ => ast::Type::simple("void"),
                    }
                } else {
                    ast::Type::simple("void")
                }
            }
            ast::Expr::ErrorPropagation(inner, _) => {
                let inner_type = self.infer_expr_type(inner, scope);
                if is_result_type(&inner_type) {
                    if let Some(ok_type) = inner_type.params().first() {
                        return ok_type.clone();
                    }
                }
                inner_type
            }
            ast::Expr::Cast(_inner, target_type, _) => target_type.clone(),
            ast::Expr::StaticCall { .. } => {
                // For toString, returns string.
                ast::Type::simple("string")
            }
            ast::Expr::Assign(_target, value, _) => {
                self.infer_expr_type(value, scope)
            }
            ast::Expr::Tuple(elements, _) => {
                let types: Vec<ast::Type> = elements
                    .iter()
                    .map(|e| self.infer_expr_type(e, scope))
                    .collect();
                ast::Type::Tuple(types)
            }
            ast::Expr::Closure { .. } => ast::Type::simple("function"),
        }
    }

    // -----------------------------------------------------------------------
    // Borrow-escape detection
    // -----------------------------------------------------------------------

    /// Check if an expression is a borrow of a local variable.
    fn expr_borrows_local(&self, expr: &ast::Expr) -> bool {
        match expr {
            ast::Expr::RefExpr(inner, _, _) => {
                if let ast::Expr::Identifier(name, _) = inner.as_ref() {
                    if self.local_vars.contains(name) {
                        return true;
                    }
                }
                false
            }
            ast::Expr::Call(callee, args, _) => {
                // A function call might return a borrow.
                // For Alpha, we check if any argument is a borrow of a local.
                if self.expr_borrows_local(callee) {
                    return true;
                }
                for arg in args {
                    if self.expr_borrows_local(arg) {
                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Analyze a Titrate program, performing symbol resolution, type checking,
/// ownership analysis, and toString desugaring.
///
/// Returns the (possibly modified) program on success, or a vector of
/// error strings describing all semantic errors found.
pub fn analyze(program: &ast::Program) -> Result<ast::Program, Vec<String>> {
    analyze_with_mode(program, ExhaustiveMode::default())
}

/// Analyze with an explicit exhaustiveness mode.
/// Returns `Ok(program)` if there are no errors (warnings are reported
/// but do not cause a failure), or `Err(errors)` otherwise.
pub fn analyze_with_mode(program: &ast::Program, mode: ExhaustiveMode) -> Result<ast::Program, Vec<String>> {
    analyze_with_mode_and_warnings(program, mode).map(|(prog, _)| prog)
}

/// Analyze with an explicit exhaustiveness mode, returning warnings alongside the result.
pub fn analyze_with_mode_and_warnings(program: &ast::Program, mode: ExhaustiveMode) -> Result<(ast::Program, Vec<String>), Vec<String>> {
    let mut program = program.clone();
    let mut analyzer = Analyzer::with_exhaustive_mode(mode);
    analyzer.analyze_program(&mut program);
    let warnings = analyzer.warnings.clone();
    if analyzer.errors.is_empty() {
        Ok((program, warnings))
    } else {
        Err(analyzer.errors)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;

    // Helper to build a simple program with one declaration.
    fn program_with(decl: Declaration) -> Program {
        Program {
            imports: vec![],
            declarations: vec![decl],
        }
    }

    // Helper: empty program.
    fn empty_program() -> Program {
        Program {
            imports: vec![],
            declarations: vec![],
        }
    }

    // -----------------------------------------------------------------------
    // Symbol resolution tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_resolve_variable() {
        let prog = program_with(Declaration::VarDecl(VarDecl {
            name: "x".to_string(),
            typ: Some(Type::simple("int")),
            init: Some(Expr::Literal(Literal::Int(42), Span::unknown())),
            mutable: false,
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_resolve_function() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("int")),
            body: vec![Stmt::Return(Some(Expr::Literal(Literal::Int(1), Span::unknown())))],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_resolve_class() {
        let prog = program_with(Declaration::Class(ClassDecl {
            name: "MyClass".to_string(),
            type_params: vec![],
            parent: None,
            ifaces: vec![],
            members: vec![ClassMember::Field(FieldDecl {
                access: Access::Private,
                name: "val".to_string(),
                typ: Type::simple("int"),
                init: None,
                span: Span::unknown(),
            })],
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_resolve_enum() {
        let prog = program_with(Declaration::Enum(EnumDecl {
            name: "Color".to_string(),
            type_params: vec![],
            variants: vec![
                Variant {
                    name: "Red".to_string(),
                    fields: vec![],
                },
                Variant {
                    name: "Blue".to_string(),
                    fields: vec![],
                },
            ],
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_undeclared_identifier() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::Expr(Expr::Identifier("unknown_var".to_string(), Span::unknown()))],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("undeclared")));
    }

    #[test]
    fn test_variable_in_scope() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::simple("int")),
                    init: Some(Expr::Literal(Literal::Int(5), Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::Expr(Expr::Identifier("x".to_string(), Span::unknown())),
            ],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    // -----------------------------------------------------------------------
    // Type checking tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_type_mismatch_var_decl() {
        let prog = program_with(Declaration::VarDecl(VarDecl {
            name: "x".to_string(),
            typ: Some(Type::simple("int")),
            init: Some(Expr::Literal(Literal::Bool(true), Span::unknown())),
            mutable: false,
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("type mismatch")));
    }

    #[test]
    fn test_int_literal_fits_long() {
        let prog = program_with(Declaration::VarDecl(VarDecl {
            name: "x".to_string(),
            typ: Some(Type::simple("long")),
            init: Some(Expr::Literal(Literal::Int(42), Span::unknown())),
            mutable: false,
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_if_condition_must_be_bool() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::If(IfStmt {
                condition: Expr::Literal(Literal::Int(1), Span::unknown()),
                then_branch: vec![],
                else_branch: None,
                span: Span::unknown(),
            })],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("if condition must be bool")));
    }

    #[test]
    fn test_while_condition_must_be_bool() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::While(WhileStmt {
                condition: Expr::Literal(Literal::Int(1), Span::unknown()),
                body: vec![],
                span: Span::unknown(),
            })],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("while condition must be bool")));
    }

    #[test]
    fn test_return_type_mismatch() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("int")),
            body: vec![Stmt::Return(Some(Expr::Literal(Literal::Bool(true), Span::unknown())))],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("return type mismatch")));
    }

    #[test]
    fn test_valid_return_type() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("int")),
            body: vec![Stmt::Return(Some(Expr::Literal(Literal::Int(42), Span::unknown())))],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_arithmetic_type_check() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::Expr(Expr::Binary(
                Box::new(Expr::Literal(Literal::Bool(true), Span::unknown())),
                Operator::Add,
                Box::new(Expr::Literal(Literal::Int(1), Span::unknown())),
                Span::unknown(),
            ))],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
    }

    #[test]
    fn test_logical_operators_require_bool() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::Expr(Expr::Binary(
                Box::new(Expr::Literal(Literal::Int(1), Span::unknown())),
                Operator::And,
                Box::new(Expr::Literal(Literal::Bool(true), Span::unknown())),
                Span::unknown(),
            ))],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Ownership analysis tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_use_after_move() {
        // let x: Owned<int> = new int(5); let y = x; io::println(x); -> error
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::generic("Owned", vec![Type::simple("int")])),
                    init: Some(Expr::New(Type::simple("int"), vec![Expr::Literal(Literal::Int(5), Span::unknown())], Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::VarDecl(VarDecl {
                    name: "y".to_string(),
                    typ: Some(Type::generic("Owned", vec![Type::simple("int")])),
                    init: Some(Expr::Identifier("x".to_string(), Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::Expr(Expr::Identifier("x".to_string(), Span::unknown())),
            ],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("use of moved variable")));
    }

    #[test]
    fn test_borrow_then_move() {
        // let x: Owned<int> = new int(5); let y = &x; x = new int(6); -> error
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::generic("Owned", vec![Type::simple("int")])),
                    init: Some(Expr::New(Type::simple("int"), vec![Expr::Literal(Literal::Int(5), Span::unknown())], Span::unknown())),
                    mutable: true,
                    span: Span::unknown(),
                }),
                Stmt::VarDecl(VarDecl {
                    name: "y".to_string(),
                    typ: Some(Type::generic("Ref", vec![Type::generic("Owned", vec![Type::simple("int")])])),
                    init: Some(Expr::RefExpr(
                        Box::new(Expr::Identifier("x".to_string(), Span::unknown())),
                        RefKind::Immutable,
                        Span::unknown(),
                    )),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::Expr(Expr::Assign(
                    Box::new(Expr::Identifier("x".to_string(), Span::unknown())),
                    Box::new(Expr::New(Type::simple("int"), vec![Expr::Literal(Literal::Int(6), Span::unknown())], Span::unknown())),
                    Span::unknown(),
                )),
            ],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs.iter().any(|e| e.contains("borrowed") || e.contains("Borrowed")), "expected borrow error, got: {:?}", errs);
    }

    #[test]
    fn test_return_borrow_of_local() {
        // fn foo(): &int { let x = 5; return &x; } -> error
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::generic("Ref", vec![Type::simple("int")])),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::simple("int")),
                    init: Some(Expr::Literal(Literal::Int(5), Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::Return(Some(Expr::RefExpr(
                    Box::new(Expr::Identifier("x".to_string(), Span::unknown())),
                    RefKind::Immutable,
                    Span::unknown(),
                ))),
            ],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("cannot return a borrow")));
    }

    #[test]
    fn test_mutable_and_immutable_borrow_conflict() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::simple("int")),
                    init: Some(Expr::Literal(Literal::Int(5), Span::unknown())),
                    mutable: true,
                    span: Span::unknown(),
                }),
                Stmt::VarDecl(VarDecl {
                    name: "y".to_string(),
                    typ: Some(Type::generic("Ref", vec![Type::simple("int")])),
                    init: Some(Expr::RefExpr(
                        Box::new(Expr::Identifier("x".to_string(), Span::unknown())),
                        RefKind::Immutable,
                        Span::unknown(),
                    )),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::VarDecl(VarDecl {
                    name: "z".to_string(),
                    typ: Some(Type::generic("RefMut", vec![Type::simple("int")])),
                    init: Some(Expr::RefExpr(
                        Box::new(Expr::Identifier("x".to_string(), Span::unknown())),
                        RefKind::Mutable,
                        Span::unknown(),
                    )),
                    mutable: false,
                    span: Span::unknown(),
                }),
            ],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
    }

    #[test]
    fn test_unsafe_skips_ownership_checks() {
        // In unsafe, use-after-move should not error.
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::Expr(Expr::UnsafeBlock(vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::generic("Owned", vec![Type::simple("int")])),
                    init: Some(Expr::New(Type::simple("int"), vec![Expr::Literal(Literal::Int(5), Span::unknown())], Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::VarDecl(VarDecl {
                    name: "y".to_string(),
                    typ: Some(Type::generic("Owned", vec![Type::simple("int")])),
                    init: Some(Expr::Identifier("x".to_string(), Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::Expr(Expr::Identifier("x".to_string(), Span::unknown())),
            ], Span::unknown()))],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    // -----------------------------------------------------------------------
    // Valid program tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_valid_empty_program() {
        let prog = empty_program();
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_valid_function_with_params() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "add".to_string(),
            type_params: vec![],
            params: vec![
                Param { name: "a".to_string(), typ: Type::simple("int") },
                Param { name: "b".to_string(), typ: Type::simple("int") },
            ],
            return_type: Some(Type::simple("int")),
            body: vec![Stmt::Return(Some(Expr::Binary(
                Box::new(Expr::Identifier("a".to_string(), Span::unknown())),
                Operator::Add,
                Box::new(Expr::Identifier("b".to_string(), Span::unknown())),
                Span::unknown(),
            )))],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_valid_if_else() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::If(IfStmt {
                condition: Expr::Literal(Literal::Bool(true), Span::unknown()),
                then_branch: vec![],
                else_branch: Some(vec![]),
                span: Span::unknown(),
            })],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_valid_while_loop() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::While(WhileStmt {
                condition: Expr::Literal(Literal::Bool(false), Span::unknown()),
                body: vec![Stmt::Break],
                span: Span::unknown(),
            })],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_valid_string_concat() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::VarDecl(VarDecl {
                name: "s".to_string(),
                typ: Some(Type::simple("string")),
                init: Some(Expr::Binary(
                    Box::new(Expr::Literal(Literal::String("hello".to_string()), Span::unknown())),
                    Operator::Add,
                    Box::new(Expr::Literal(Literal::String(" world".to_string()), Span::unknown())),
                    Span::unknown(),
                )),
                mutable: false,
                span: Span::unknown(),
            })],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    // -----------------------------------------------------------------------
    // toString desugaring tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_tostring_desugaring_int() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::simple("int")),
                    init: Some(Expr::Literal(Literal::Int(42), Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::Expr(Expr::Call(
                    Box::new(Expr::MemberAccess(
                        Box::new(Expr::Identifier("x".to_string(), Span::unknown())),
                        "toString".to_string(),
                        Span::unknown(),
                    )),
                    vec![],
                    Span::unknown(),
                )),
            ],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_ok());

        // Verify the expression was desugared.
        let analyzed = result.unwrap();
        let body = match &analyzed.declarations[0] {
            Declaration::Function(f) => &f.body,
            _ => panic!("expected function"),
        };
        match &body[1] {
            Stmt::Expr(Expr::StaticCall { class_name, method, args, span: _ }) => {
                assert_eq!(class_name, "Integer");
                assert_eq!(method, "toString");
                assert_eq!(args.len(), 1);
            }
            other => panic!("expected StaticCall, got {:?}", other),
        }
    }

    #[test]
    fn test_tostring_desugaring_bool() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "b".to_string(),
                    typ: Some(Type::simple("bool")),
                    init: Some(Expr::Literal(Literal::Bool(true), Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::Expr(Expr::Call(
                    Box::new(Expr::MemberAccess(
                        Box::new(Expr::Identifier("b".to_string(), Span::unknown())),
                        "toString".to_string(),
                        Span::unknown(),
                    )),
                    vec![],
                    Span::unknown(),
                )),
            ],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_ok());

        let analyzed = result.unwrap();
        let body = match &analyzed.declarations[0] {
            Declaration::Function(f) => &f.body,
            _ => panic!("expected function"),
        };
        match &body[1] {
            Stmt::Expr(Expr::StaticCall { class_name, method, span: _, .. }) => {
                assert_eq!(class_name, "Boolean");
                assert_eq!(method, "toString");
            }
            other => panic!("expected StaticCall, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Switch / enum tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_switch_enum_valid() {
        let prog = Program {
            imports: vec![],
            declarations: vec![
                Declaration::Enum(EnumDecl {
                    name: "Color".to_string(),
                    type_params: vec![],
                    variants: vec![
                        Variant { name: "Red".to_string(), fields: vec![] },
                        Variant { name: "Blue".to_string(), fields: vec![] },
                    ],
                    span: Span::unknown(),
                }),
                Declaration::Function(FnDecl {
                    access: Access::Public,
                    name: "test".to_string(),
                    type_params: vec![],
                    params: vec![Param { name: "c".to_string(), typ: Type::simple("Color") }],
                    return_type: Some(Type::simple("void")),
                    body: vec![Stmt::Switch(SwitchStmt {
                        expr: Expr::Identifier("c".to_string(), Span::unknown()),
                        cases: vec![
                            Case {
                                pattern: Pattern::Constructor {
                                    name: "Red".to_string(),
                                    bindings: vec![],
                                },
                                body: vec![],
                            },
                        ],
                        default: None,
                        span: Span::unknown(),
                    })],
                    sugar: false,
                    where_clause: vec![],
                    span: Span::unknown(),
                }),
            ],
        };
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_switch_enum_wrong_variant() {
        let prog = Program {
            imports: vec![],
            declarations: vec![
                Declaration::Enum(EnumDecl {
                    name: "Color".to_string(),
                    type_params: vec![],
                    variants: vec![
                        Variant { name: "Red".to_string(), fields: vec![] },
                    ],
                    span: Span::unknown(),
                }),
                Declaration::Enum(EnumDecl {
                    name: "Shape".to_string(),
                    type_params: vec![],
                    variants: vec![
                        Variant { name: "Circle".to_string(), fields: vec![] },
                    ],
                    span: Span::unknown(),
                }),
                Declaration::Function(FnDecl {
                    access: Access::Public,
                    name: "test".to_string(),
                    type_params: vec![],
                    params: vec![Param { name: "c".to_string(), typ: Type::simple("Color") }],
                    return_type: Some(Type::simple("void")),
                    body: vec![Stmt::Switch(SwitchStmt {
                        expr: Expr::Identifier("c".to_string(), Span::unknown()),
                        cases: vec![
                            Case {
                                pattern: Pattern::Constructor {
                                    name: "Circle".to_string(),
                                    bindings: vec![],
                                },
                                body: vec![],
                            },
                        ],
                        default: None,
                        span: Span::unknown(),
                    })],
                    sugar: false,
                    where_clause: vec![],
                    span: Span::unknown(),
                }),
            ],
        };
        let result = analyze(&prog);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Error propagation tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_error_propagation_non_result_function() {
        // Use ? operator in a function that doesn't return Result.
        // We use a literal wrapped in ErrorPropagation to avoid undeclared identifier issues.
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("int")),
            body: vec![Stmt::Expr(Expr::ErrorPropagation(
                Box::new(Expr::Literal(Literal::Int(42), Span::unknown())),
                Span::unknown(),
            ))],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs.iter().any(|e| e.contains("? operator")), "expected '? operator' error, got: {:?}", errs);
    }

    #[test]
    fn test_error_propagation_in_result_function() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::generic("Result", vec![Type::simple("int"), Type::simple("string")])),
            body: vec![Stmt::Return(Some(Expr::ErrorPropagation(
                Box::new(Expr::Call(
                    Box::new(Expr::Identifier("bar".to_string(), Span::unknown())),
                    vec![],
                    Span::unknown(),
                )),
                Span::unknown(),
            )))],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        // This should pass because the function returns Result.
        // The function "bar" is undeclared, but that's a separate concern.
        // The ? operator check should pass.
        let result = analyze(&prog);
        // May error on undeclared "bar" but not on ? operator.
        if let Err(errs) = &result {
            assert!(!errs.iter().any(|e| e.contains("? operator")), "unexpected ? operator error: {:?}", errs);
        }
    }

    // -----------------------------------------------------------------------
    // Cast tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_valid_numeric_cast() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::VarDecl(VarDecl {
                name: "x".to_string(),
                typ: Some(Type::simple("long")),
                init: Some(Expr::Cast(
                    Box::new(Expr::Literal(Literal::Int(42), Span::unknown())),
                    Type::simple("long"),
                    Span::unknown(),
                )),
                mutable: false,
                span: Span::unknown(),
            })],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_invalid_cast() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::Expr(Expr::Cast(
                Box::new(Expr::Literal(Literal::String("hello".to_string()), Span::unknown())),
                Type::simple("int"),
                Span::unknown(),
            ))],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Immutability tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_assign_to_immutable() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::simple("int")),
                    init: Some(Expr::Literal(Literal::Int(5), Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::Expr(Expr::Assign(
                    Box::new(Expr::Identifier("x".to_string(), Span::unknown())),
                    Box::new(Expr::Literal(Literal::Int(10), Span::unknown())),
                    Span::unknown(),
                )),
            ],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("immutable")));
    }

    #[test]
    fn test_assign_to_mutable() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::simple("int")),
                    init: Some(Expr::Literal(Literal::Int(5), Span::unknown())),
                    mutable: true,
                    span: Span::unknown(),
                }),
                Stmt::Expr(Expr::Assign(
                    Box::new(Expr::Identifier("x".to_string(), Span::unknown())),
                    Box::new(Expr::Literal(Literal::Int(10), Span::unknown())),
                    Span::unknown(),
                )),
            ],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    // -----------------------------------------------------------------------
    // Bitwise operator tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_bitwise_on_integers() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::Expr(Expr::Binary(
                Box::new(Expr::Literal(Literal::Int(1), Span::unknown())),
                Operator::BitAnd,
                Box::new(Expr::Literal(Literal::Int(2), Span::unknown())),
                Span::unknown(),
            ))],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_bitwise_on_non_integers() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::Expr(Expr::Binary(
                Box::new(Expr::Literal(Literal::Bool(true), Span::unknown())),
                Operator::BitAnd,
                Box::new(Expr::Literal(Literal::Int(2), Span::unknown())),
                Span::unknown(),
            ))],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Unary operator tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_unary_neg_on_numeric() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::Expr(Expr::Unary(
                UnOp::Neg,
                Box::new(Expr::Literal(Literal::Int(5), Span::unknown())),
                Span::unknown(),
            ))],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_unary_not_on_bool() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::Expr(Expr::Unary(
                UnOp::Not,
                Box::new(Expr::Literal(Literal::Bool(true), Span::unknown())),
                Span::unknown(),
            ))],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_unary_not_on_non_bool() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::Expr(Expr::Unary(
                UnOp::Not,
                Box::new(Expr::Literal(Literal::Int(5), Span::unknown())),
                Span::unknown(),
            ))],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Interface test
    // -----------------------------------------------------------------------

    #[test]
    fn test_interface_declaration() {
        let prog = program_with(Declaration::Interface(InterfaceDecl {
            name: "Printable".to_string(),
            type_params: vec![],
            parents: vec![],
            methods: vec![MethodSig {
                name: "toString".to_string(),
                params: vec![],
                return_type: Some(Type::simple("string")),
            }],
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    // -----------------------------------------------------------------------
    // Multiple declarations interaction test
    // -----------------------------------------------------------------------

    #[test]
    fn test_function_calling_function() {
        let prog = Program {
            imports: vec![],
            declarations: vec![
                Declaration::Function(FnDecl {
                    access: Access::Public,
                    name: "helper".to_string(),
                    type_params: vec![],
                    params: vec![],
                    return_type: Some(Type::simple("int")),
                    body: vec![Stmt::Return(Some(Expr::Literal(Literal::Int(1), Span::unknown())))],
                    sugar: false,
                    where_clause: vec![],
                    span: Span::unknown(),
                }),
                Declaration::Function(FnDecl {
                    access: Access::Public,
                    name: "main".to_string(),
                    type_params: vec![],
                    params: vec![],
                    return_type: Some(Type::simple("void")),
                    body: vec![Stmt::Expr(Expr::Call(
                        Box::new(Expr::Identifier("helper".to_string(), Span::unknown())),
                        vec![],
                        Span::unknown(),
                    ))],
                    sugar: false,
                    where_clause: vec![],
                    span: Span::unknown(),
                }),
            ],
        };
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_function_wrong_arg_count() {
        let prog = Program {
            imports: vec![],
            declarations: vec![
                Declaration::Function(FnDecl {
                    access: Access::Public,
                    name: "add".to_string(),
                    type_params: vec![],
                    params: vec![
                        Param { name: "a".to_string(), typ: Type::simple("int") },
                        Param { name: "b".to_string(), typ: Type::simple("int") },
                    ],
                    return_type: Some(Type::simple("int")),
                    body: vec![Stmt::Return(Some(Expr::Binary(
                        Box::new(Expr::Identifier("a".to_string(), Span::unknown())),
                        Operator::Add,
                        Box::new(Expr::Identifier("b".to_string(), Span::unknown())),
                        Span::unknown(),
                    )))],
                    sugar: false,
                    where_clause: vec![],
                    span: Span::unknown(),
                }),
                Declaration::Function(FnDecl {
                    access: Access::Public,
                    name: "main".to_string(),
                    type_params: vec![],
                    params: vec![],
                    return_type: Some(Type::simple("void")),
                    body: vec![Stmt::Expr(Expr::Call(
                        Box::new(Expr::Identifier("add".to_string(), Span::unknown())),
                        vec![Expr::Literal(Literal::Int(1), Span::unknown())],
                        Span::unknown(),
                    ))],
                    sugar: false,
                    where_clause: vec![],
                    span: Span::unknown(),
                }),
            ],
        };
        let result = analyze(&prog);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("arguments")));
    }

    // -----------------------------------------------------------------------
    // Owned deref test
    // -----------------------------------------------------------------------

    #[test]
    fn test_owned_deref() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::generic("Owned", vec![Type::simple("int")])),
                    init: Some(Expr::New(Type::simple("int"), vec![Expr::Literal(Literal::Int(5), Span::unknown())], Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::VarDecl(VarDecl {
                    name: "y".to_string(),
                    typ: Some(Type::simple("int")),
                    init: Some(Expr::OwnedDeref(Box::new(Expr::Identifier("x".to_string(), Span::unknown())), Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
            ],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_owned_deref_on_non_owned() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::simple("int")),
                    init: Some(Expr::Literal(Literal::Int(5), Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::Expr(Expr::OwnedDeref(Box::new(Expr::Identifier("x".to_string(), Span::unknown())), Span::unknown())),
            ],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("Owned")));
    }

    // -----------------------------------------------------------------------
    // Const declaration test
    // -----------------------------------------------------------------------

    #[test]
    fn test_const_decl() {
        let prog = program_with(Declaration::ConstDecl(VarDecl {
            name: "PI".to_string(),
            typ: Some(Type::simple("double")),
            init: Some(Expr::Literal(Literal::Float(3.14159), Span::unknown())),
            mutable: false,
            span: Span::unknown(),
        }));
        assert!(analyze(&prog).is_ok());
    }

    // -----------------------------------------------------------------------
    // For loop test
    // -----------------------------------------------------------------------

    #[test]
    fn test_for_loop() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::For(ForStmt {
                var: "i".to_string(),
                iterable: Expr::Identifier("range".to_string(), Span::unknown()),
                body: vec![Stmt::Expr(Expr::Identifier("i".to_string(), Span::unknown()))],
                span: Span::unknown(),
            })],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        // Will error on undeclared "range" but "i" should be in scope.
        let result = analyze(&prog);
        // "range" is undeclared, so this should error.
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Type inference test
    // -----------------------------------------------------------------------

    #[test]
    fn test_type_inference_no_declared_type() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: None,
                    init: Some(Expr::Literal(Literal::Int(42), Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                // x should be inferred as int.
                Stmt::Expr(Expr::Identifier("x".to_string(), Span::unknown())),
            ],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_ok());
        // Verify type was inferred.
        let analyzed = result.unwrap();
        match &analyzed.declarations[0] {
            Declaration::Function(f) => {
                match &f.body[0] {
                    Stmt::VarDecl(v) => {
                        assert_eq!(v.typ, Some(Type::simple("int")));
                    }
                    _ => panic!("expected var decl"),
                }
            }
            _ => panic!("expected function"),
        }
    }

    // -----------------------------------------------------------------------
    // Double mutable borrow test
    // -----------------------------------------------------------------------

    #[test]
    fn test_double_mutable_borrow() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::simple("int")),
                    init: Some(Expr::Literal(Literal::Int(5), Span::unknown())),
                    mutable: true,
                    span: Span::unknown(),
                }),
                Stmt::VarDecl(VarDecl {
                    name: "y".to_string(),
                    typ: Some(Type::generic("RefMut", vec![Type::simple("int")])),
                    init: Some(Expr::RefExpr(
                        Box::new(Expr::Identifier("x".to_string(), Span::unknown())),
                        RefKind::Mutable,
                        Span::unknown(),
                    )),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::VarDecl(VarDecl {
                    name: "z".to_string(),
                    typ: Some(Type::generic("RefMut", vec![Type::simple("int")])),
                    init: Some(Expr::RefExpr(
                        Box::new(Expr::Identifier("x".to_string(), Span::unknown())),
                        RefKind::Mutable,
                        Span::unknown(),
                    )),
                    mutable: false,
                    span: Span::unknown(),
                }),
            ],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Borrow of moved variable test
    // -----------------------------------------------------------------------

    #[test]
    fn test_borrow_after_move() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::generic("Owned", vec![Type::simple("int")])),
                    init: Some(Expr::New(Type::simple("int"), vec![Expr::Literal(Literal::Int(5), Span::unknown())], Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::VarDecl(VarDecl {
                    name: "y".to_string(),
                    typ: Some(Type::generic("Owned", vec![Type::simple("int")])),
                    init: Some(Expr::Identifier("x".to_string(), Span::unknown())),
                    mutable: false,
                    span: Span::unknown(),
                }),
                Stmt::VarDecl(VarDecl {
                    name: "z".to_string(),
                    typ: Some(Type::generic("Ref", vec![Type::generic("Owned", vec![Type::simple("int")])])),
                    init: Some(Expr::RefExpr(
                        Box::new(Expr::Identifier("x".to_string(), Span::unknown())),
                        RefKind::Immutable,
                        Span::unknown(),
                    )),
                    mutable: false,
                    span: Span::unknown(),
                }),
            ],
            sugar: false,
            where_clause: vec![],
            span: Span::unknown(),
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Exhaustiveness checking tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_exhaustive_switch_all_variants_covered() {
        // Switch covering all enum variants → no warning, no error
        let prog = Program {
            imports: vec![],
            declarations: vec![
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
                Declaration::Function(FnDecl {
                    access: Access::Public,
                    name: "test".to_string(),
                    type_params: vec![],
                    params: vec![Param { name: "c".to_string(), typ: Type::simple("Color") }],
                    return_type: Some(Type::simple("void")),
                    body: vec![Stmt::Switch(SwitchStmt {
                        expr: Expr::Identifier("c".to_string(), Span::unknown()),
                        cases: vec![
                            Case {
                                pattern: Pattern::Constructor { name: "Red".to_string(), bindings: vec![] },
                                body: vec![],
                            },
                            Case {
                                pattern: Pattern::Constructor { name: "Green".to_string(), bindings: vec![] },
                                body: vec![],
                            },
                            Case {
                                pattern: Pattern::Constructor { name: "Blue".to_string(), bindings: vec![] },
                                body: vec![],
                            },
                        ],
                        default: None,
                        span: Span::unknown(),
                    })],
                    sugar: false,
                    where_clause: vec![],
                    span: Span::unknown(),
                }),
            ],
        };
        let result = analyze_with_mode_and_warnings(&prog, ExhaustiveMode::Warning);
        assert!(result.is_ok());
        let (_, warnings) = result.unwrap();
        assert!(warnings.is_empty(), "expected no warnings for exhaustive switch, got: {:?}", warnings);
    }

    #[test]
    fn test_non_exhaustive_switch_produces_warning() {
        // Switch missing some variants → warning in Warning mode
        let prog = Program {
            imports: vec![],
            declarations: vec![
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
                Declaration::Function(FnDecl {
                    access: Access::Public,
                    name: "test".to_string(),
                    type_params: vec![],
                    params: vec![Param { name: "c".to_string(), typ: Type::simple("Color") }],
                    return_type: Some(Type::simple("void")),
                    body: vec![Stmt::Switch(SwitchStmt {
                        expr: Expr::Identifier("c".to_string(), Span::unknown()),
                        cases: vec![
                            Case {
                                pattern: Pattern::Constructor { name: "Red".to_string(), bindings: vec![] },
                                body: vec![],
                            },
                        ],
                        default: None,
                        span: Span::unknown(),
                    })],
                    sugar: false,
                    where_clause: vec![],
                    span: Span::unknown(),
                }),
            ],
        };
        let result = analyze_with_mode_and_warnings(&prog, ExhaustiveMode::Warning);
        assert!(result.is_ok(), "Warning mode should not produce an error");
        let (_, warnings) = result.unwrap();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("non-exhaustive pattern match"));
        assert!(warnings[0].contains("Green"));
        assert!(warnings[0].contains("Blue"));
    }

    #[test]
    fn test_non_exhaustive_switch_error_mode() {
        // Switch missing some variants → error in Error mode
        let prog = Program {
            imports: vec![],
            declarations: vec![
                Declaration::Enum(EnumDecl {
                    name: "Color".to_string(),
                    type_params: vec![],
                    variants: vec![
                        Variant { name: "Red".to_string(), fields: vec![] },
                        Variant { name: "Green".to_string(), fields: vec![] },
                    ],
                    span: Span::unknown(),
                }),
                Declaration::Function(FnDecl {
                    access: Access::Public,
                    name: "test".to_string(),
                    type_params: vec![],
                    params: vec![Param { name: "c".to_string(), typ: Type::simple("Color") }],
                    return_type: Some(Type::simple("void")),
                    body: vec![Stmt::Switch(SwitchStmt {
                        expr: Expr::Identifier("c".to_string(), Span::unknown()),
                        cases: vec![
                            Case {
                                pattern: Pattern::Constructor { name: "Red".to_string(), bindings: vec![] },
                                body: vec![],
                            },
                        ],
                        default: None,
                        span: Span::unknown(),
                    })],
                    sugar: false,
                    where_clause: vec![],
                    span: Span::unknown(),
                }),
            ],
        };
        let result = analyze_with_mode(&prog, ExhaustiveMode::Error);
        assert!(result.is_err(), "Error mode should produce an error for non-exhaustive switch");
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("non-exhaustive pattern match")));
    }

    #[test]
    fn test_switch_with_default_no_warning() {
        // Switch with default case → no exhaustiveness warning even if variants are missing
        let prog = Program {
            imports: vec![],
            declarations: vec![
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
                Declaration::Function(FnDecl {
                    access: Access::Public,
                    name: "test".to_string(),
                    type_params: vec![],
                    params: vec![Param { name: "c".to_string(), typ: Type::simple("Color") }],
                    return_type: Some(Type::simple("void")),
                    body: vec![Stmt::Switch(SwitchStmt {
                        expr: Expr::Identifier("c".to_string(), Span::unknown()),
                        cases: vec![
                            Case {
                                pattern: Pattern::Constructor { name: "Red".to_string(), bindings: vec![] },
                                body: vec![],
                            },
                        ],
                        default: Some(vec![]),
                        span: Span::unknown(),
                    })],
                    sugar: false,
                    where_clause: vec![],
                    span: Span::unknown(),
                }),
            ],
        };
        let result = analyze_with_mode_and_warnings(&prog, ExhaustiveMode::Warning);
        assert!(result.is_ok());
        let (_, warnings) = result.unwrap();
        assert!(warnings.is_empty(), "expected no warnings for switch with default, got: {:?}", warnings);
    }
}
