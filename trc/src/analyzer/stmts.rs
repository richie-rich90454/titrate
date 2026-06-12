use super::*;
use super::types::{
    is_bool_type, is_result_type, is_assignable, is_void_type,
    is_unknown_type, is_owned_type,
};

impl Analyzer {

    pub(super) fn analyze_block(&mut self, block: &mut ast::Block, scope: &Rc<RefCell<Scope>>) {
        let block_scope = Rc::new(RefCell::new(Scope::new(Some(scope.clone()))));
        let prev_after_terminator = self.after_terminator;
        self.after_terminator = false;
        for stmt in block.iter_mut() {
            if self.after_terminator {
                self.warn("unreachable code detected after return/break/continue".to_string());
                self.after_terminator = false;
            }
            self.analyze_stmt(stmt, &block_scope);
        }
        // If the block didn't end with a terminator, reset for the parent.
        if !self.after_terminator {
            self.after_terminator = prev_after_terminator;
        }
    }

    /// Check whether a block always returns (i.e., ends with a return statement
    /// or all branches of an if/else return).
    pub(super) fn block_always_returns(&self, block: &ast::Block) -> bool {
        for stmt in block.iter() {
            match stmt {
                ast::Stmt::Return(_) => return true,
                ast::Stmt::If(if_stmt) => {
                    let then_returns = self.block_always_returns(&if_stmt.then_branch);
                    let else_returns = if_stmt.else_branch.as_ref()
                        .map(|e| self.block_always_returns(e))
                        .unwrap_or(false);
                    if then_returns && else_returns {
                        return true;
                    }
                }
                ast::Stmt::Block(inner) => {
                    if self.block_always_returns(inner) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    pub(super) fn analyze_stmt(&mut self, stmt: &mut ast::Stmt, scope: &Rc<RefCell<Scope>>) {
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
                    self.error(CompileError::new(format!(
                        "if condition must be bool, found {}",
                        cond_type
                    )).suggest(Suggestion {
                        message: "use a comparison or boolean expression".to_string(),
                        replacement: None,
                    }));
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
                    self.error(CompileError::new(format!(
                        "while condition must be bool, found {}",
                        cond_type
                    )).suggest(Suggestion {
                        message: "use a comparison or boolean expression".to_string(),
                        replacement: None,
                    }));
                }
                self.analyze_block(&mut while_stmt.body, scope);
            }
            ast::Stmt::DoWhile(do_while_stmt) => {
                self.analyze_block(&mut do_while_stmt.body, scope);
                self.analyze_expr(&mut do_while_stmt.condition, scope);
                let cond_type = self.infer_expr_type(&do_while_stmt.condition, scope);
                if !is_bool_type(&cond_type) {
                    self.error(CompileError::new(format!(
                        "do-while condition must be bool, found {}",
                        cond_type
                    )).suggest(Suggestion {
                        message: "use a comparison or boolean expression".to_string(),
                        replacement: None,
                    }));
                }
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
                        self.error(CompileError::new(format!(
                            "for condition must be bool, found {}",
                            cond_type
                        )).suggest(Suggestion {
                            message: "use a comparison or boolean expression".to_string(),
                            replacement: None,
                        }));
                    }
                }
                self.analyze_block(&mut cfor_stmt.body, &cfor_scope);
                if let Some(ref mut incr) = cfor_stmt.increment {
                    self.analyze_expr(incr, &cfor_scope);
                }
            }
            ast::Stmt::Return(opt_expr) => {
                self.after_terminator = true;
                if let Some(ref mut expr) = opt_expr {
                    self.analyze_expr(expr, scope);
                    let ret_type = self.infer_expr_type(expr, scope);
                    if let Some(ref expected) = self.current_return_type {
                        if !is_assignable(&ret_type, expected) && !is_void_type(expected) {
                            let fn_name = self.current_fn_name.clone().unwrap_or_default();
                            self.error(CompileError::new(format!(
                                "return type mismatch in function '{}': expected {}, found {}",
                                fn_name, expected, ret_type
                            )).suggest(Suggestion {
                                message: format!("change the return expression to type {}", expected),
                                replacement: None,
                            }));
                        }
                    }
                } else if let Some(ref expected) = self.current_return_type {
                    if !is_void_type(expected) {
                        let fn_name = self.current_fn_name.clone().unwrap_or_default();
                        self.error(CompileError::new(format!(
                            "function '{}' with return type {} must return a value",
                            fn_name, expected
                        )).suggest(Suggestion {
                            message: "add a return value or change the return type to void".to_string(),
                            replacement: None,
                        }));
                    }
                }

                // Check for returning a borrow of a local.
                if let Some(ref expr) = opt_expr {
                    if self.expr_borrows_local(expr) {
                        self.error(CompileError::new(
                            "cannot return a borrow of a local variable".to_string()
                        ).suggest(Suggestion {
                            message: "return an owned value instead, or use a region-allocated reference".to_string(),
                            replacement: None,
                        }));
                    }
                }
            }
            ast::Stmt::Break => {
                self.after_terminator = true;
            }
            ast::Stmt::Continue => {
                self.after_terminator = true;
            }
            ast::Stmt::Switch(sw) => {
                self.analyze_switch(sw, scope);
            }
            ast::Stmt::With(with_stmt) => {
                self.analyze_expr(&mut with_stmt.resource_expr, scope);
                let with_scope = Rc::new(RefCell::new(Scope::new(Some(scope.clone()))));
                if let Some(ref name) = with_stmt.var_name {
                    let res_type = self.infer_expr_type(&with_stmt.resource_expr, scope);
                    with_scope.borrow_mut().define(
                        name.clone(),
                        Symbol::Variable {
                            typ: res_type,
                            mutable: false,
                        },
                    );
                    self.var_states.insert(name.clone(), VarState::Live);
                    self.local_vars.push(name.clone());
                }
                self.analyze_block(&mut with_stmt.body, &with_scope);
                if let Some(ref name) = with_stmt.var_name {
                    self.var_states.remove(name);
                    self.local_vars.retain(|v| v != name);
                }
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
                        self.error(CompileError::new(format!(
                            "tuple destructuring expects {} elements, found {}",
                            element_types.len(),
                            names.len()
                        )).suggest(Suggestion {
                            message: format!("the tuple has {} element(s)", element_types.len()),
                            replacement: None,
                        }));
                    }
                } else if !is_unknown_type(&expr_type) {
                    self.error(CompileError::new(format!(
                        "tuple destructuring requires a tuple type, found {}",
                        expr_type
                    )).suggest(Suggestion {
                        message: "use a tuple expression on the right-hand side".to_string(),
                        replacement: None,
                    }));
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

    pub(super) fn analyze_var_decl(&mut self, v: &mut ast::VarDecl, scope: &Rc<RefCell<Scope>>) {
        // Analyze the initializer first.
        if let Some(ref mut init) = v.init {
            self.analyze_expr(init, scope);
            let init_type = self.infer_expr_type(init, scope);

            if let Some(ref declared) = v.typ {
                // Type check: initializer must be assignable to declared type.
                if !is_assignable(&init_type, declared) {
                    self.error(CompileError::new(format!(
                        "type mismatch in variable '{}': cannot assign {} to {}",
                        v.name, init_type, declared
                    )).suggest(Suggestion {
                        message: format!("expected type {}, found {}", declared, init_type),
                        replacement: None,
                    }));
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

    pub(super) fn analyze_switch(&mut self, sw: &mut ast::SwitchStmt, scope: &Rc<RefCell<Scope>>) {
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
                                    self.error(CompileError::new(format!(
                                        "variant '{}' does not belong to enum '{}'",
                                        name, enum_name
                                    )).suggest(Suggestion {
                                        message: format!("this variant belongs to enum '{}'", en),
                                        replacement: None,
                                    }));
                                }
                            }
                            Some(_) => {
                                self.error(CompileError::new(format!(
                                    "'{}' is not an enum variant",
                                    name
                                )).suggest(Suggestion {
                                    message: "use a variant name from the matched enum".to_string(),
                                    replacement: None,
                                }));
                            }
                            None => {
                                // Could be Ok/Err 鈥?allow for Result matching
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
}
