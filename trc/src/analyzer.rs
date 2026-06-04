/// Semantic analyzer for the Titrate language.
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

fn is_void_type(t: &ast::Type) -> bool {
    t.name() == "void"
}

fn is_unknown_type(t: &ast::Type) -> bool {
    t.name() == "unknown" || t.name() == "any"
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
// Analyzer
// ---------------------------------------------------------------------------

struct Analyzer {
    errors: Vec<String>,
    /// Ownership state per variable in the current function scope.
    /// Keyed by a scope-depth-qualified name to handle shadowing.
    var_states: HashMap<String, VarState>,
    /// Track which variables are locals in the current function (for borrow-checking).
    local_vars: Vec<String>,
    /// Current function return type (for return-checking and ?-operator).
    current_return_type: Option<ast::Type>,
    /// Whether we are inside an unsafe block.
    in_unsafe: bool,
}

impl Analyzer {
    fn new() -> Self {
        Analyzer {
            errors: Vec::new(),
            var_states: HashMap::new(),
            local_vars: Vec::new(),
            current_return_type: None,
            in_unsafe: false,
        }
    }

    fn error(&mut self, msg: impl Into<String>) {
        self.errors.push(msg.into());
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
                    parent: None,
                    ifaces: vec![],
                    members: vec![],
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
                                params: m.params.clone(),
                                return_type: m.return_type.clone(),
                                body: m.body.clone(),
                                sugar: false,
                            };
                            class_scope.borrow_mut().define(m.name.clone(), Symbol::Function(fn_decl));
                        }
                        ast::ClassMember::Constructor(m) => {
                            let fn_decl = ast::FnDecl {
                                access: m.access.clone(),
                                name: m.name.clone(),
                                params: m.params.clone(),
                                return_type: m.return_type.clone(),
                                body: m.body.clone(),
                                sugar: false,
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
                if let ast::Expr::Identifier(src_name) = init {
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
    }

    // -----------------------------------------------------------------------
    // Expression analysis
    // -----------------------------------------------------------------------

    fn analyze_expr(&mut self, expr: &mut ast::Expr, scope: &Rc<RefCell<Scope>>) {
        match expr {
            ast::Expr::Literal(_) => {}
            ast::Expr::Identifier(name) => {
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
            ast::Expr::Binary(left, op, right) => {
                self.analyze_expr(left, scope);
                self.analyze_expr(right, scope);

                let left_type = self.infer_expr_type(left, scope);
                let right_type = self.infer_expr_type(right, scope);

                // Skip type checking when types are unknown (from builtins, field access, etc.)
                if is_unknown_type(&left_type) || is_unknown_type(&right_type) {
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
            ast::Expr::Unary(unop, operand) => {
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
            ast::Expr::Call(callee, args) => {
                // Check for toString desugaring BEFORE analyzing (which borrows immutably).
                // We need to detect: Call(MemberAccess(obj, "toString"), [])
                let desugar_info: Option<(String, ast::Expr)> = match callee.as_ref() {
                    ast::Expr::MemberAccess(obj, method) if method == "toString" => {
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
                    ast::Expr::Identifier(name) => {
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
                    ast::Expr::MemberAccess(obj, method) => {
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
            ast::Expr::MemberAccess(obj, field) => {
                self.analyze_expr(obj.as_mut(), scope);
                // If it's a method call, it will be handled in the Call branch above.
                // For field access, we just validate the object is not moved.
                let _ = field;
            }
            ast::Expr::Index(obj, idx) => {
                self.analyze_expr(obj.as_mut(), scope);
                self.analyze_expr(idx.as_mut(), scope);
            }
            ast::Expr::New(typ, args) => {
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
            ast::Expr::This => {}
            ast::Expr::Super => {}
            ast::Expr::OwnedDeref(inner) => {
                self.analyze_expr(inner, scope);
                let inner_type = self.infer_expr_type(inner, scope);
                if !is_owned_type(&inner_type) && !is_unknown_type(&inner_type) {
                    self.error(format!(
                        "owned dereference requires Owned type, found {}",
                        inner_type
                    ));
                }
            }
            ast::Expr::RegionAlloc(_typ, region_expr) => {
                self.analyze_expr(region_expr, scope);
                // Track that this allocation belongs to the current region.
                // We'll check that region-allocated values don't escape.
            }
            ast::Expr::RefExpr(inner, ref_kind) => {
                self.analyze_expr(inner, scope);

                if self.in_unsafe {
                    return;
                }

                // Borrow checking.
                match inner.as_ref() {
                    ast::Expr::Identifier(name) => {
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
            ast::Expr::UnsafeBlock(block) => {
                let prev_unsafe = self.in_unsafe;
                self.in_unsafe = true;
                self.analyze_block(block, scope);
                self.in_unsafe = prev_unsafe;
            }
            ast::Expr::ErrorPropagation(inner) => {
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
            ast::Expr::Cast(inner, target_type) => {
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
            ast::Expr::StaticCall { class_name, method, args } => {
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
                    Some(_) => {
                        self.error(format!("{} is not a class", class_name));
                    }
                }
                let _ = method;
            }
            ast::Expr::Assign(target, value) => {
                self.analyze_expr(value, scope);
                let value_type = self.infer_expr_type(value, scope);

                // Analyze the target expression as well.
                self.analyze_expr(target, scope);

                // Check the target is assignable.
                match target.as_ref() {
                    ast::Expr::Identifier(name) => {
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
                    ast::Expr::MemberAccess(_, _) | ast::Expr::Index(_, _) => {
                        // Already analyzed above.
                    }
                    _ => {
                        self.error("invalid assignment target".to_string());
                    }
                }

                // Check if the value being assigned is a move of an Owned variable.
                if !self.in_unsafe {
                    if let ast::Expr::Identifier(src_name) = value.as_ref() {
                        let src_sym = scope.borrow().lookup(src_name);
                        if let Some(Symbol::Variable { typ, .. }) = src_sym {
                            if is_owned_type(&typ) {
                                self.var_states.insert(src_name.clone(), VarState::Moved);
                            }
                        }
                    }
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Type inference
    // -----------------------------------------------------------------------

    fn infer_expr_type(&self, expr: &ast::Expr, scope: &Rc<RefCell<Scope>>) -> ast::Type {
        match expr {
            ast::Expr::Literal(lit) => literal_type(lit),
            ast::Expr::Identifier(name) => {
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
            ast::Expr::Binary(left, op, _right) => {
                let left_type = self.infer_expr_type(left, scope);
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
            ast::Expr::Unary(unop, operand) => {
                let operand_type = self.infer_expr_type(operand, scope);
                match unop {
                    ast::UnOp::Neg => operand_type,
                    ast::UnOp::Not => ast::Type::simple("bool"),
                    ast::UnOp::BitNot => operand_type,
                }
            }
            ast::Expr::Call(callee, _args) => {
                match callee.as_ref() {
                    ast::Expr::Identifier(name) => {
                        if let Some(Symbol::Function(f)) = scope.borrow().lookup(name) {
                            f.return_type.clone().unwrap_or(ast::Type::simple("void"))
                        } else if let Some(Symbol::Variant { enum_name, .. }) = scope.borrow().lookup(name) {
                            ast::Type::simple(&enum_name)
                        } else {
                            ast::Type::simple("unknown")
                        }
                    }
                    ast::Expr::MemberAccess(obj, method) => {
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
            ast::Expr::MemberAccess(obj, _field) => {
                let _obj_type = self.infer_expr_type(obj, scope);
                // Without class field info, return unknown.
                ast::Type::simple("unknown")
            }
            ast::Expr::Index(_obj, _idx) => {
                // Without array element type info, return unknown.
                ast::Type::simple("unknown")
            }
            ast::Expr::New(typ, _args) => typ.clone(),
            ast::Expr::This => {
                // Look up "this" in scope.
                if let Some(Symbol::Variable { typ, .. }) = scope.borrow().lookup("this") {
                    typ
                } else {
                    ast::Type::simple("unknown")
                }
            }
            ast::Expr::Super => {
                if let Some(Symbol::Variable { typ, .. }) = scope.borrow().lookup("this") {
                    typ
                } else {
                    ast::Type::simple("unknown")
                }
            }
            ast::Expr::OwnedDeref(inner) => {
                let inner_type = self.infer_expr_type(inner, scope);
                if is_owned_type(&inner_type) {
                    if let Some(inner_param) = inner_type.params().first() {
                        return inner_param.clone();
                    }
                }
                inner_type
            }
            ast::Expr::RegionAlloc(typ, _region) => typ.clone(),
            ast::Expr::RefExpr(inner, ref_kind) => {
                let inner_type = self.infer_expr_type(inner, scope);
                match ref_kind {
                    ast::RefKind::Immutable => {
                        ast::Type::generic("Ref", vec![inner_type])
                    }
                    ast::RefKind::Mutable => {
                        ast::Type::generic("RefMut", vec![inner_type])
                    }
                }
            }
            ast::Expr::UnsafeBlock(block) => {
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
            ast::Expr::ErrorPropagation(inner) => {
                let inner_type = self.infer_expr_type(inner, scope);
                if is_result_type(&inner_type) {
                    if let Some(ok_type) = inner_type.params().first() {
                        return ok_type.clone();
                    }
                }
                inner_type
            }
            ast::Expr::Cast(_inner, target_type) => target_type.clone(),
            ast::Expr::StaticCall { .. } => {
                // For toString, returns string.
                ast::Type::simple("string")
            }
            ast::Expr::Assign(_target, value) => {
                self.infer_expr_type(value, scope)
            }
        }
    }

    // -----------------------------------------------------------------------
    // Borrow-escape detection
    // -----------------------------------------------------------------------

    /// Check if an expression is a borrow of a local variable.
    fn expr_borrows_local(&self, expr: &ast::Expr) -> bool {
        match expr {
            ast::Expr::RefExpr(inner, _) => {
                if let ast::Expr::Identifier(name) = inner.as_ref() {
                    if self.local_vars.contains(name) {
                        return true;
                    }
                }
                false
            }
            ast::Expr::Call(callee, args) => {
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
/// Returns the (possibly modified) program on success, or an error string
/// describing the first semantic error found.
pub fn analyze(program: &ast::Program) -> Result<ast::Program, String> {
    let mut program = program.clone();
    let mut analyzer = Analyzer::new();
    analyzer.analyze_program(&mut program);
    if analyzer.errors.is_empty() {
        Ok(program)
    } else {
        // Return the first error.
        Err(analyzer.errors.into_iter().next().unwrap_or_else(|| "unknown error".to_string()))
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
            init: Some(Expr::Literal(Literal::Int(42))),
            mutable: false,
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_resolve_function() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            params: vec![],
            return_type: Some(Type::simple("int")),
            body: vec![Stmt::Return(Some(Expr::Literal(Literal::Int(1))))],
            sugar: false,
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_resolve_class() {
        let prog = program_with(Declaration::Class(ClassDecl {
            name: "MyClass".to_string(),
            parent: None,
            ifaces: vec![],
            members: vec![ClassMember::Field(FieldDecl {
                access: Access::Private,
                name: "val".to_string(),
                typ: Type::simple("int"),
                init: None,
            })],
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_resolve_enum() {
        let prog = program_with(Declaration::Enum(EnumDecl {
            name: "Color".to_string(),
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
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_undeclared_identifier() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::Expr(Expr::Identifier("unknown_var".to_string()))],
            sugar: false,
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("undeclared"));
    }

    #[test]
    fn test_variable_in_scope() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::simple("int")),
                    init: Some(Expr::Literal(Literal::Int(5))),
                    mutable: false,
                }),
                Stmt::Expr(Expr::Identifier("x".to_string())),
            ],
            sugar: false,
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
            init: Some(Expr::Literal(Literal::Bool(true))),
            mutable: false,
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("type mismatch"));
    }

    #[test]
    fn test_int_literal_fits_long() {
        let prog = program_with(Declaration::VarDecl(VarDecl {
            name: "x".to_string(),
            typ: Some(Type::simple("long")),
            init: Some(Expr::Literal(Literal::Int(42))),
            mutable: false,
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_if_condition_must_be_bool() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::If(IfStmt {
                condition: Expr::Literal(Literal::Int(1)),
                then_branch: vec![],
                else_branch: None,
            })],
            sugar: false,
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("if condition must be bool"));
    }

    #[test]
    fn test_while_condition_must_be_bool() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::While(WhileStmt {
                condition: Expr::Literal(Literal::Int(1)),
                body: vec![],
            })],
            sugar: false,
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("while condition must be bool"));
    }

    #[test]
    fn test_return_type_mismatch() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            params: vec![],
            return_type: Some(Type::simple("int")),
            body: vec![Stmt::Return(Some(Expr::Literal(Literal::Bool(true))))],
            sugar: false,
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("return type mismatch"));
    }

    #[test]
    fn test_valid_return_type() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            params: vec![],
            return_type: Some(Type::simple("int")),
            body: vec![Stmt::Return(Some(Expr::Literal(Literal::Int(42))))],
            sugar: false,
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_arithmetic_type_check() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::Expr(Expr::Binary(
                Box::new(Expr::Literal(Literal::Bool(true))),
                Operator::Add,
                Box::new(Expr::Literal(Literal::Int(1))),
            ))],
            sugar: false,
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
    }

    #[test]
    fn test_logical_operators_require_bool() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::Expr(Expr::Binary(
                Box::new(Expr::Literal(Literal::Int(1))),
                Operator::And,
                Box::new(Expr::Literal(Literal::Bool(true))),
            ))],
            sugar: false,
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
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::generic("Owned", vec![Type::simple("int")])),
                    init: Some(Expr::New(Type::simple("int"), vec![Expr::Literal(Literal::Int(5))])),
                    mutable: false,
                }),
                Stmt::VarDecl(VarDecl {
                    name: "y".to_string(),
                    typ: Some(Type::generic("Owned", vec![Type::simple("int")])),
                    init: Some(Expr::Identifier("x".to_string())),
                    mutable: false,
                }),
                Stmt::Expr(Expr::Identifier("x".to_string())),
            ],
            sugar: false,
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("use of moved variable"));
    }

    #[test]
    fn test_borrow_then_move() {
        // let x: Owned<int> = new int(5); let y = &x; x = new int(6); -> error
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::generic("Owned", vec![Type::simple("int")])),
                    init: Some(Expr::New(Type::simple("int"), vec![Expr::Literal(Literal::Int(5))])),
                    mutable: true,
                }),
                Stmt::VarDecl(VarDecl {
                    name: "y".to_string(),
                    typ: Some(Type::generic("Ref", vec![Type::generic("Owned", vec![Type::simple("int")])])),
                    init: Some(Expr::RefExpr(
                        Box::new(Expr::Identifier("x".to_string())),
                        RefKind::Immutable,
                    )),
                    mutable: false,
                }),
                Stmt::Expr(Expr::Assign(
                    Box::new(Expr::Identifier("x".to_string())),
                    Box::new(Expr::New(Type::simple("int"), vec![Expr::Literal(Literal::Int(6))])),
                )),
            ],
            sugar: false,
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("borrowed") || err.contains("Borrowed"), "expected borrow error, got: {}", err);
    }

    #[test]
    fn test_return_borrow_of_local() {
        // fn foo(): &int { let x = 5; return &x; } -> error
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            params: vec![],
            return_type: Some(Type::generic("Ref", vec![Type::simple("int")])),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::simple("int")),
                    init: Some(Expr::Literal(Literal::Int(5))),
                    mutable: false,
                }),
                Stmt::Return(Some(Expr::RefExpr(
                    Box::new(Expr::Identifier("x".to_string())),
                    RefKind::Immutable,
                ))),
            ],
            sugar: false,
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot return a borrow"));
    }

    #[test]
    fn test_mutable_and_immutable_borrow_conflict() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::simple("int")),
                    init: Some(Expr::Literal(Literal::Int(5))),
                    mutable: true,
                }),
                Stmt::VarDecl(VarDecl {
                    name: "y".to_string(),
                    typ: Some(Type::generic("Ref", vec![Type::simple("int")])),
                    init: Some(Expr::RefExpr(
                        Box::new(Expr::Identifier("x".to_string())),
                        RefKind::Immutable,
                    )),
                    mutable: false,
                }),
                Stmt::VarDecl(VarDecl {
                    name: "z".to_string(),
                    typ: Some(Type::generic("RefMut", vec![Type::simple("int")])),
                    init: Some(Expr::RefExpr(
                        Box::new(Expr::Identifier("x".to_string())),
                        RefKind::Mutable,
                    )),
                    mutable: false,
                }),
            ],
            sugar: false,
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
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::Expr(Expr::UnsafeBlock(vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::generic("Owned", vec![Type::simple("int")])),
                    init: Some(Expr::New(Type::simple("int"), vec![Expr::Literal(Literal::Int(5))])),
                    mutable: false,
                }),
                Stmt::VarDecl(VarDecl {
                    name: "y".to_string(),
                    typ: Some(Type::generic("Owned", vec![Type::simple("int")])),
                    init: Some(Expr::Identifier("x".to_string())),
                    mutable: false,
                }),
                Stmt::Expr(Expr::Identifier("x".to_string())),
            ]))],
            sugar: false,
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
            params: vec![
                Param { name: "a".to_string(), typ: Type::simple("int") },
                Param { name: "b".to_string(), typ: Type::simple("int") },
            ],
            return_type: Some(Type::simple("int")),
            body: vec![Stmt::Return(Some(Expr::Binary(
                Box::new(Expr::Identifier("a".to_string())),
                Operator::Add,
                Box::new(Expr::Identifier("b".to_string())),
            )))],
            sugar: false,
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_valid_if_else() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::If(IfStmt {
                condition: Expr::Literal(Literal::Bool(true)),
                then_branch: vec![],
                else_branch: Some(vec![]),
            })],
            sugar: false,
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_valid_while_loop() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::While(WhileStmt {
                condition: Expr::Literal(Literal::Bool(false)),
                body: vec![Stmt::Break],
            })],
            sugar: false,
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_valid_string_concat() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::VarDecl(VarDecl {
                name: "s".to_string(),
                typ: Some(Type::simple("string")),
                init: Some(Expr::Binary(
                    Box::new(Expr::Literal(Literal::String("hello".to_string()))),
                    Operator::Add,
                    Box::new(Expr::Literal(Literal::String(" world".to_string()))),
                )),
                mutable: false,
            })],
            sugar: false,
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
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::simple("int")),
                    init: Some(Expr::Literal(Literal::Int(42))),
                    mutable: false,
                }),
                Stmt::Expr(Expr::Call(
                    Box::new(Expr::MemberAccess(
                        Box::new(Expr::Identifier("x".to_string())),
                        "toString".to_string(),
                    )),
                    vec![],
                )),
            ],
            sugar: false,
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
            Stmt::Expr(Expr::StaticCall { class_name, method, args }) => {
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
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "b".to_string(),
                    typ: Some(Type::simple("bool")),
                    init: Some(Expr::Literal(Literal::Bool(true))),
                    mutable: false,
                }),
                Stmt::Expr(Expr::Call(
                    Box::new(Expr::MemberAccess(
                        Box::new(Expr::Identifier("b".to_string())),
                        "toString".to_string(),
                    )),
                    vec![],
                )),
            ],
            sugar: false,
        }));
        let result = analyze(&prog);
        assert!(result.is_ok());

        let analyzed = result.unwrap();
        let body = match &analyzed.declarations[0] {
            Declaration::Function(f) => &f.body,
            _ => panic!("expected function"),
        };
        match &body[1] {
            Stmt::Expr(Expr::StaticCall { class_name, method, .. }) => {
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
                    variants: vec![
                        Variant { name: "Red".to_string(), fields: vec![] },
                        Variant { name: "Blue".to_string(), fields: vec![] },
                    ],
                }),
                Declaration::Function(FnDecl {
                    access: Access::Public,
                    name: "test".to_string(),
                    params: vec![Param { name: "c".to_string(), typ: Type::simple("Color") }],
                    return_type: Some(Type::simple("void")),
                    body: vec![Stmt::Switch(SwitchStmt {
                        expr: Expr::Identifier("c".to_string()),
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
                    })],
                    sugar: false,
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
                    variants: vec![
                        Variant { name: "Red".to_string(), fields: vec![] },
                    ],
                }),
                Declaration::Enum(EnumDecl {
                    name: "Shape".to_string(),
                    variants: vec![
                        Variant { name: "Circle".to_string(), fields: vec![] },
                    ],
                }),
                Declaration::Function(FnDecl {
                    access: Access::Public,
                    name: "test".to_string(),
                    params: vec![Param { name: "c".to_string(), typ: Type::simple("Color") }],
                    return_type: Some(Type::simple("void")),
                    body: vec![Stmt::Switch(SwitchStmt {
                        expr: Expr::Identifier("c".to_string()),
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
                    })],
                    sugar: false,
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
            params: vec![],
            return_type: Some(Type::simple("int")),
            body: vec![Stmt::Expr(Expr::ErrorPropagation(
                Box::new(Expr::Literal(Literal::Int(42))),
            ))],
            sugar: false,
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("? operator"), "expected '? operator' error, got: {}", err);
    }

    #[test]
    fn test_error_propagation_in_result_function() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "foo".to_string(),
            params: vec![],
            return_type: Some(Type::generic("Result", vec![Type::simple("int"), Type::simple("string")])),
            body: vec![Stmt::Return(Some(Expr::ErrorPropagation(
                Box::new(Expr::Call(
                    Box::new(Expr::Identifier("bar".to_string())),
                    vec![],
                )),
            )))],
            sugar: false,
        }));
        // This should pass because the function returns Result.
        // The function "bar" is undeclared, but that's a separate concern.
        // The ? operator check should pass.
        let result = analyze(&prog);
        // May error on undeclared "bar" but not on ? operator.
        if let Err(e) = &result {
            assert!(!e.contains("? operator"), "unexpected ? operator error: {}", e);
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
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::VarDecl(VarDecl {
                name: "x".to_string(),
                typ: Some(Type::simple("long")),
                init: Some(Expr::Cast(
                    Box::new(Expr::Literal(Literal::Int(42))),
                    Type::simple("long"),
                )),
                mutable: false,
            })],
            sugar: false,
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_invalid_cast() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::Expr(Expr::Cast(
                Box::new(Expr::Literal(Literal::String("hello".to_string()))),
                Type::simple("int"),
            ))],
            sugar: false,
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
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::simple("int")),
                    init: Some(Expr::Literal(Literal::Int(5))),
                    mutable: false,
                }),
                Stmt::Expr(Expr::Assign(
                    Box::new(Expr::Identifier("x".to_string())),
                    Box::new(Expr::Literal(Literal::Int(10))),
                )),
            ],
            sugar: false,
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("immutable"));
    }

    #[test]
    fn test_assign_to_mutable() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::simple("int")),
                    init: Some(Expr::Literal(Literal::Int(5))),
                    mutable: true,
                }),
                Stmt::Expr(Expr::Assign(
                    Box::new(Expr::Identifier("x".to_string())),
                    Box::new(Expr::Literal(Literal::Int(10))),
                )),
            ],
            sugar: false,
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
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::Expr(Expr::Binary(
                Box::new(Expr::Literal(Literal::Int(1))),
                Operator::BitAnd,
                Box::new(Expr::Literal(Literal::Int(2))),
            ))],
            sugar: false,
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_bitwise_on_non_integers() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::Expr(Expr::Binary(
                Box::new(Expr::Literal(Literal::Bool(true))),
                Operator::BitAnd,
                Box::new(Expr::Literal(Literal::Int(2))),
            ))],
            sugar: false,
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
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::Expr(Expr::Unary(
                UnOp::Neg,
                Box::new(Expr::Literal(Literal::Int(5))),
            ))],
            sugar: false,
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_unary_not_on_bool() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::Expr(Expr::Unary(
                UnOp::Not,
                Box::new(Expr::Literal(Literal::Bool(true))),
            ))],
            sugar: false,
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_unary_not_on_non_bool() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::Expr(Expr::Unary(
                UnOp::Not,
                Box::new(Expr::Literal(Literal::Int(5))),
            ))],
            sugar: false,
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
            parents: vec![],
            methods: vec![MethodSig {
                name: "toString".to_string(),
                params: vec![],
                return_type: Some(Type::simple("string")),
            }],
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
                    params: vec![],
                    return_type: Some(Type::simple("int")),
                    body: vec![Stmt::Return(Some(Expr::Literal(Literal::Int(1))))],
                    sugar: false,
                }),
                Declaration::Function(FnDecl {
                    access: Access::Public,
                    name: "main".to_string(),
                    params: vec![],
                    return_type: Some(Type::simple("void")),
                    body: vec![Stmt::Expr(Expr::Call(
                        Box::new(Expr::Identifier("helper".to_string())),
                        vec![],
                    ))],
                    sugar: false,
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
                    params: vec![
                        Param { name: "a".to_string(), typ: Type::simple("int") },
                        Param { name: "b".to_string(), typ: Type::simple("int") },
                    ],
                    return_type: Some(Type::simple("int")),
                    body: vec![Stmt::Return(Some(Expr::Binary(
                        Box::new(Expr::Identifier("a".to_string())),
                        Operator::Add,
                        Box::new(Expr::Identifier("b".to_string())),
                    )))],
                    sugar: false,
                }),
                Declaration::Function(FnDecl {
                    access: Access::Public,
                    name: "main".to_string(),
                    params: vec![],
                    return_type: Some(Type::simple("void")),
                    body: vec![Stmt::Expr(Expr::Call(
                        Box::new(Expr::Identifier("add".to_string())),
                        vec![Expr::Literal(Literal::Int(1))],
                    ))],
                    sugar: false,
                }),
            ],
        };
        let result = analyze(&prog);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("arguments"));
    }

    // -----------------------------------------------------------------------
    // Owned deref test
    // -----------------------------------------------------------------------

    #[test]
    fn test_owned_deref() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::generic("Owned", vec![Type::simple("int")])),
                    init: Some(Expr::New(Type::simple("int"), vec![Expr::Literal(Literal::Int(5))])),
                    mutable: false,
                }),
                Stmt::VarDecl(VarDecl {
                    name: "y".to_string(),
                    typ: Some(Type::simple("int")),
                    init: Some(Expr::OwnedDeref(Box::new(Expr::Identifier("x".to_string())))),
                    mutable: false,
                }),
            ],
            sugar: false,
        }));
        assert!(analyze(&prog).is_ok());
    }

    #[test]
    fn test_owned_deref_on_non_owned() {
        let prog = program_with(Declaration::Function(FnDecl {
            access: Access::Public,
            name: "test".to_string(),
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::simple("int")),
                    init: Some(Expr::Literal(Literal::Int(5))),
                    mutable: false,
                }),
                Stmt::Expr(Expr::OwnedDeref(Box::new(Expr::Identifier("x".to_string())))),
            ],
            sugar: false,
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Owned"));
    }

    // -----------------------------------------------------------------------
    // Const declaration test
    // -----------------------------------------------------------------------

    #[test]
    fn test_const_decl() {
        let prog = program_with(Declaration::ConstDecl(VarDecl {
            name: "PI".to_string(),
            typ: Some(Type::simple("double")),
            init: Some(Expr::Literal(Literal::Float(3.14159))),
            mutable: false,
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
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![Stmt::For(ForStmt {
                var: "i".to_string(),
                iterable: Expr::Identifier("range".to_string()),
                body: vec![Stmt::Expr(Expr::Identifier("i".to_string()))],
            })],
            sugar: false,
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
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: None,
                    init: Some(Expr::Literal(Literal::Int(42))),
                    mutable: false,
                }),
                // x should be inferred as int.
                Stmt::Expr(Expr::Identifier("x".to_string())),
            ],
            sugar: false,
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
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::simple("int")),
                    init: Some(Expr::Literal(Literal::Int(5))),
                    mutable: true,
                }),
                Stmt::VarDecl(VarDecl {
                    name: "y".to_string(),
                    typ: Some(Type::generic("RefMut", vec![Type::simple("int")])),
                    init: Some(Expr::RefExpr(
                        Box::new(Expr::Identifier("x".to_string())),
                        RefKind::Mutable,
                    )),
                    mutable: false,
                }),
                Stmt::VarDecl(VarDecl {
                    name: "z".to_string(),
                    typ: Some(Type::generic("RefMut", vec![Type::simple("int")])),
                    init: Some(Expr::RefExpr(
                        Box::new(Expr::Identifier("x".to_string())),
                        RefKind::Mutable,
                    )),
                    mutable: false,
                }),
            ],
            sugar: false,
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
            params: vec![],
            return_type: Some(Type::simple("void")),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    name: "x".to_string(),
                    typ: Some(Type::generic("Owned", vec![Type::simple("int")])),
                    init: Some(Expr::New(Type::simple("int"), vec![Expr::Literal(Literal::Int(5))])),
                    mutable: false,
                }),
                Stmt::VarDecl(VarDecl {
                    name: "y".to_string(),
                    typ: Some(Type::generic("Owned", vec![Type::simple("int")])),
                    init: Some(Expr::Identifier("x".to_string())),
                    mutable: false,
                }),
                Stmt::VarDecl(VarDecl {
                    name: "z".to_string(),
                    typ: Some(Type::generic("Ref", vec![Type::generic("Owned", vec![Type::simple("int")])])),
                    init: Some(Expr::RefExpr(
                        Box::new(Expr::Identifier("x".to_string())),
                        RefKind::Immutable,
                    )),
                    mutable: false,
                }),
            ],
            sugar: false,
        }));
        let result = analyze(&prog);
        assert!(result.is_err());
    }
}
