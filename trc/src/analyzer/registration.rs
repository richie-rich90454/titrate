use super::*;
use super::types::{is_void_type};

impl Analyzer {

    // -----------------------------------------------------------------------
    // Top-level analysis
    // -----------------------------------------------------------------------

    pub(super) fn analyze_program(&mut self, program: &mut ast::Program) {
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
    pub(super) fn register_builtins(&self, scope: &Rc<RefCell<Scope>>) {
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

    pub(super) fn register_declaration(&mut self, decl: &ast::Declaration, scope: &Rc<RefCell<Scope>>) {
        match decl {
            ast::Declaration::Function(f) => {
                if scope.borrow().symbols.contains_key(&f.name) {
                    self.error(CompileError::new(format!(
                        "duplicate declaration: '{}' is already declared in this scope",
                        f.name
                    )).suggest(Suggestion {
                        message: "use a different name for this function".to_string(),
                        replacement: None,
                    }));
                }
                scope.borrow_mut().define(f.name.clone(), Symbol::Function(f.clone()));
            }
            ast::Declaration::Class(c) => {
                if scope.borrow().symbols.contains_key(&c.name) {
                    self.error(CompileError::new(format!(
                        "duplicate declaration: '{}' is already declared in this scope",
                        c.name
                    )).suggest(Suggestion {
                        message: "use a different name for this class".to_string(),
                        replacement: None,
                    }));
                }
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
                // Store the class scope as a child 鈥?we won't look it up later
                // but it ensures the symbols exist for reference.
                let _ = class_scope;
            }
            ast::Declaration::Interface(i) => {
                if scope.borrow().symbols.contains_key(&i.name) {
                    self.error(CompileError::new(format!(
                        "duplicate declaration: '{}' is already declared in this scope",
                        i.name
                    )).suggest(Suggestion {
                        message: "use a different name for this interface".to_string(),
                        replacement: None,
                    }));
                }
                scope.borrow_mut().define(i.name.clone(), Symbol::Interface(i.clone()));
            }
            ast::Declaration::Enum(e) => {
                if scope.borrow().symbols.contains_key(&e.name) {
                    self.error(CompileError::new(format!(
                        "duplicate declaration: '{}' is already declared in this scope",
                        e.name
                    )).suggest(Suggestion {
                        message: "use a different name for this enum".to_string(),
                        replacement: None,
                    }));
                }
                scope.borrow_mut().define(e.name.clone(), Symbol::Enum(e.clone()));
                // Register each variant as a symbol.
                for variant in &e.variants {
                    if scope.borrow().symbols.contains_key(&variant.name) {
                        self.error(CompileError::new(format!(
                            "duplicate declaration: variant '{}' is already declared in this scope",
                            variant.name
                        )).suggest(Suggestion {
                            message: "use a different name for this variant".to_string(),
                            replacement: None,
                        }));
                    }
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

    pub(super) fn analyze_declaration(&mut self, decl: &mut ast::Declaration, scope: &Rc<RefCell<Scope>>) {
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

    pub(super) fn analyze_fn_decl(&mut self, f: &mut ast::FnDecl, scope: &Rc<RefCell<Scope>>) {
        let fn_scope = Rc::new(RefCell::new(Scope::new(Some(scope.clone()))));
        let prev_return = self.current_return_type.clone();
        let prev_fn_name = self.current_fn_name.clone();
        self.current_return_type = f.return_type.clone();
        self.current_fn_name = Some(f.name.clone());

        // Reset ownership state for this function.
        let prev_states = std::mem::take(&mut self.var_states);
        let prev_locals = std::mem::take(&mut self.local_vars);
        let prev_used = std::mem::take(&mut self.used_vars);
        let prev_after_terminator = self.after_terminator;
        self.after_terminator = false;

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

        // Check for missing return statement in non-void functions.
        if let Some(ref ret_type) = f.return_type {
            if !is_void_type(ret_type) && !self.block_always_returns(&f.body) {
                let fn_name = f.name.clone();
                self.error(CompileError::new(format!(
                    "function '{}' is missing a return statement: expected return type {}",
                    fn_name, ret_type
                )).suggest(Suggestion {
                    message: "add a return statement at the end of the function body".to_string(),
                    replacement: None,
                }));
            }
        }

        // Check for unused variables.
        let unused: Vec<String> = self.local_vars.iter()
            .filter(|var_name| !self.used_vars.contains(*var_name) && !var_name.starts_with('_'))
            .cloned()
            .collect();
        for var_name in &unused {
            self.warn(format!("unused variable: {}", var_name));
        }

        // Restore.
        self.var_states = prev_states;
        self.local_vars = prev_locals;
        self.current_return_type = prev_return;
        self.current_fn_name = prev_fn_name;
        self.used_vars = prev_used;
        self.after_terminator = prev_after_terminator;
    }

    pub(super) fn analyze_method(&mut self, m: &mut ast::MethodDecl, scope: &Rc<RefCell<Scope>>) {
        let method_scope = Rc::new(RefCell::new(Scope::new(Some(scope.clone()))));
        let prev_return = self.current_return_type.clone();
        let prev_fn_name = self.current_fn_name.clone();
        self.current_return_type = m.return_type.clone();
        self.current_fn_name = Some(m.name.clone());

        let prev_states = std::mem::take(&mut self.var_states);
        let prev_locals = std::mem::take(&mut self.local_vars);
        let prev_used = std::mem::take(&mut self.used_vars);
        let prev_after_terminator = self.after_terminator;
        self.after_terminator = false;

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

        // Check for missing return statement in non-void methods.
        if let Some(ref ret_type) = m.return_type {
            if !is_void_type(ret_type) && !self.block_always_returns(&m.body) {
                let method_name = m.name.clone();
                self.error(CompileError::new(format!(
                    "method '{}' is missing a return statement: expected return type {}",
                    method_name, ret_type
                )).suggest(Suggestion {
                    message: "add a return statement at the end of the method body".to_string(),
                    replacement: None,
                }));
            }
        }

        // Check for unused variables.
        let unused: Vec<String> = self.local_vars.iter()
            .filter(|var_name| !self.used_vars.contains(*var_name) && !var_name.starts_with('_'))
            .cloned()
            .collect();
        for var_name in &unused {
            self.warn(format!("unused variable: {}", var_name));
        }

        self.var_states = prev_states;
        self.local_vars = prev_locals;
        self.current_return_type = prev_return;
        self.current_fn_name = prev_fn_name;
        self.used_vars = prev_used;
        self.after_terminator = prev_after_terminator;
    }
}