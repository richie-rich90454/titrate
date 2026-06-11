// Monomorphization: name mangling, type substitution, instantiation

use std::collections::HashMap;

use crate::ast;
use super::Compiler;

impl Compiler {
    /// Generate a mangled name for a generic specialization.
    /// E.g. mangle_name("Box", [int]) → "Box__int"
    pub(super) fn mangle_name(base: &str, type_args: &[ast::Type]) -> String {
        if type_args.is_empty() {
            return base.to_string();
        }
        let mut name = base.to_string();
        for arg in type_args {
            name.push_str("__");
            name.push_str(&Self::type_to_mangle_string(arg));
        }
        name
    }

    fn type_to_mangle_string(ty: &ast::Type) -> String {
        match ty {
            ast::Type::Named { name, params } => {
                if params.is_empty() {
                    name.clone()
                } else {
                    let mut s = name.clone();
                    for p in params {
                        s.push_str("__");
                        s.push_str(&Self::type_to_mangle_string(p));
                    }
                    s
                }
            }
            ast::Type::Ref(inner) => format!("Ref_{}", Self::type_to_mangle_string(inner)),
            ast::Type::MutRef(inner) => format!("MutRef_{}", Self::type_to_mangle_string(inner)),
            ast::Type::Tuple(types) => {
                let inner: Vec<String> = types.iter().map(|t| Self::type_to_mangle_string(t)).collect();
                format!("Tuple_{}", inner.join("_"))
            }
        }
    }

    /// Substitute type parameters with concrete types.
    /// E.g. if type_args = {"T": int}, then T → int, Owned<T> → Owned<int>.
    pub(super) fn substitute_type(ty: &ast::Type, type_args: &HashMap<String, ast::Type>) -> ast::Type {
        match ty {
            ast::Type::Named { name, params } => {
                // If this is a simple type parameter reference, substitute it.
                if params.is_empty() {
                    if let Some(concrete) = type_args.get(name) {
                        return concrete.clone();
                    }
                }
                // Otherwise, recursively substitute in params.
                let new_params: Vec<ast::Type> = params
                    .iter()
                    .map(|p| Self::substitute_type(p, type_args))
                    .collect();
                ast::Type::Named {
                    name: name.clone(),
                    params: new_params,
                }
            }
            ast::Type::Ref(inner) => {
                ast::Type::Ref(Box::new(Self::substitute_type(inner, type_args)))
            }
            ast::Type::MutRef(inner) => {
                ast::Type::MutRef(Box::new(Self::substitute_type(inner, type_args)))
            }
            ast::Type::Tuple(types) => {
                let new_types: Vec<ast::Type> = types
                    .iter()
                    .map(|t| Self::substitute_type(t, type_args))
                    .collect();
                ast::Type::Tuple(new_types)
            }
        }
    }

    pub(super) fn substitute_expr(expr: &ast::Expr, type_args: &HashMap<String, ast::Type>) -> ast::Expr {
        match expr {
            ast::Expr::Literal(lit, span) => ast::Expr::Literal(lit.clone(), *span),
            ast::Expr::Identifier(name, span) => ast::Expr::Identifier(name.clone(), *span),
            ast::Expr::Binary(left, op, right, span) => ast::Expr::Binary(
                Box::new(Self::substitute_expr(left, type_args)),
                op.clone(),
                Box::new(Self::substitute_expr(right, type_args)),
                *span,
            ),
            ast::Expr::Unary(op, operand, span) => ast::Expr::Unary(
                op.clone(),
                Box::new(Self::substitute_expr(operand, type_args)),
                *span,
            ),
            ast::Expr::Call(callee, args, span) => ast::Expr::Call(
                Box::new(Self::substitute_expr(callee, type_args)),
                args.iter().map(|a| Self::substitute_expr(a, type_args)).collect(),
                *span,
            ),
            ast::Expr::MemberAccess(obj, member, span) => ast::Expr::MemberAccess(
                Box::new(Self::substitute_expr(obj, type_args)),
                member.clone(),
                *span,
            ),
            ast::Expr::Index(obj, index, span) => ast::Expr::Index(
                Box::new(Self::substitute_expr(obj, type_args)),
                Box::new(Self::substitute_expr(index, type_args)),
                *span,
            ),
            ast::Expr::New(typ, args, span) => ast::Expr::New(
                Self::substitute_type(typ, type_args),
                args.iter().map(|a| Self::substitute_expr(a, type_args)).collect(),
                *span,
            ),
            ast::Expr::This(span) => ast::Expr::This(*span),
            ast::Expr::Super(span) => ast::Expr::Super(*span),
            ast::Expr::OwnedDeref(inner, span) => ast::Expr::OwnedDeref(
                Box::new(Self::substitute_expr(inner, type_args)),
                *span,
            ),
            ast::Expr::RegionAlloc(typ, init, span) => ast::Expr::RegionAlloc(
                Self::substitute_type(typ, type_args),
                Box::new(Self::substitute_expr(init, type_args)),
                *span,
            ),
            ast::Expr::RefExpr(inner, kind, span) => ast::Expr::RefExpr(
                Box::new(Self::substitute_expr(inner, type_args)),
                kind.clone(),
                *span,
            ),
            ast::Expr::UnsafeBlock(block, span) => ast::Expr::UnsafeBlock(
                block.iter().map(|s| Self::substitute_stmt(s, type_args)).collect(),
                *span,
            ),
            ast::Expr::ErrorPropagation(inner, span) => ast::Expr::ErrorPropagation(
                Box::new(Self::substitute_expr(inner, type_args)),
                *span,
            ),
            ast::Expr::Cast(inner, target_type, span) => ast::Expr::Cast(
                Box::new(Self::substitute_expr(inner, type_args)),
                Self::substitute_type(target_type, type_args),
                *span,
            ),
            ast::Expr::StaticCall { class_name, method, args, span } => ast::Expr::StaticCall {
                class_name: class_name.clone(),
                method: method.clone(),
                args: args.iter().map(|a| Self::substitute_expr(a, type_args)).collect(),
                span: *span,
            },
            ast::Expr::Assign(target, value, span) => ast::Expr::Assign(
                Box::new(Self::substitute_expr(target, type_args)),
                Box::new(Self::substitute_expr(value, type_args)),
                *span,
            ),
            ast::Expr::Unit(span) => ast::Expr::Unit(*span),
            ast::Expr::Closure {
                params,
                return_type,
                body,
                expr,
                captured_vars,
                span,
            } => ast::Expr::Closure {
                params: params.iter().map(|(n, t)| (n.clone(), Self::substitute_type(t, type_args))).collect(),
                return_type: Self::substitute_type(return_type, type_args),
                body: body.iter().map(|s| Self::substitute_stmt(s, type_args)).collect(),
                expr: expr.as_ref().map(|e| Box::new(Self::substitute_expr(e, type_args))),
                captured_vars: captured_vars.clone(),
                span: *span,
            },
            ast::Expr::Tuple(elements, span) => ast::Expr::Tuple(
                elements.iter().map(|e| Self::substitute_expr(e, type_args)).collect(),
                *span,
            ),
            ast::Expr::Range(start, end, span) => ast::Expr::Range(
                Box::new(Self::substitute_expr(start, type_args)),
                Box::new(Self::substitute_expr(end, type_args)),
                *span,
            ),
            ast::Expr::RangeInclusive(start, end, span) => ast::Expr::RangeInclusive(
                Box::new(Self::substitute_expr(start, type_args)),
                Box::new(Self::substitute_expr(end, type_args)),
                *span,
            ),
            ast::Expr::Ternary { condition, then_expr, else_expr, span } => ast::Expr::Ternary {
                condition: Box::new(Self::substitute_expr(condition, type_args)),
                then_expr: Box::new(Self::substitute_expr(then_expr, type_args)),
                else_expr: Box::new(Self::substitute_expr(else_expr, type_args)),
                span: *span,
            },
        }
    }

    pub(super) fn substitute_stmt(stmt: &ast::Stmt, type_args: &HashMap<String, ast::Type>) -> ast::Stmt {
        match stmt {
            ast::Stmt::VarDecl(var_decl) => {
                ast::Stmt::VarDecl(Self::substitute_var_decl(var_decl, type_args))
            }
            ast::Stmt::ConstDecl(var_decl) => {
                ast::Stmt::ConstDecl(Self::substitute_var_decl(var_decl, type_args))
            }
            ast::Stmt::Expr(expr) => {
                ast::Stmt::Expr(Self::substitute_expr(expr, type_args))
            }
            ast::Stmt::If(if_stmt) => ast::Stmt::If(ast::IfStmt {
                condition: Self::substitute_expr(&if_stmt.condition, type_args),
                then_branch: if_stmt.then_branch.iter().map(|s| Self::substitute_stmt(s, type_args)).collect(),
                else_branch: if_stmt.else_branch.as_ref().map(|b| b.iter().map(|s| Self::substitute_stmt(s, type_args)).collect()),
                span: if_stmt.span,
            }),
            ast::Stmt::While(while_stmt) => ast::Stmt::While(ast::WhileStmt {
                condition: Self::substitute_expr(&while_stmt.condition, type_args),
                body: while_stmt.body.iter().map(|s| Self::substitute_stmt(s, type_args)).collect(),
                span: while_stmt.span,
            }),
            ast::Stmt::DoWhile(do_while_stmt) => ast::Stmt::DoWhile(ast::DoWhileStmt {
                body: do_while_stmt.body.iter().map(|s| Self::substitute_stmt(s, type_args)).collect(),
                condition: Self::substitute_expr(&do_while_stmt.condition, type_args),
                span: do_while_stmt.span,
            }),
            ast::Stmt::WhileLet(while_let_stmt) => ast::Stmt::WhileLet(ast::WhileLetStmt {
                var_name: while_let_stmt.var_name.clone(),
                expr: Self::substitute_expr(&while_let_stmt.expr, type_args),
                body: while_let_stmt.body.iter().map(|s| Self::substitute_stmt(s, type_args)).collect(),
                span: while_let_stmt.span,
            }),
            ast::Stmt::For(for_stmt) => ast::Stmt::For(ast::ForStmt {
                var: for_stmt.var.clone(),
                iterable: Self::substitute_expr(&for_stmt.iterable, type_args),
                body: for_stmt.body.iter().map(|s| Self::substitute_stmt(s, type_args)).collect(),
                span: for_stmt.span,
            }),
            ast::Stmt::CFor(cfor_stmt) => ast::Stmt::CFor(ast::CForStmt {
                init: cfor_stmt.init.as_ref().map(|s| Box::new(Self::substitute_stmt(s, type_args))),
                condition: cfor_stmt.condition.as_ref().map(|e| Self::substitute_expr(e, type_args)),
                increment: cfor_stmt.increment.as_ref().map(|e| Self::substitute_expr(e, type_args)),
                body: cfor_stmt.body.iter().map(|s| Self::substitute_stmt(s, type_args)).collect(),
                span: cfor_stmt.span,
            }),
            ast::Stmt::Return(expr) => {
                ast::Stmt::Return(expr.as_ref().map(|e| Self::substitute_expr(e, type_args)))
            }
            ast::Stmt::Break => ast::Stmt::Break,
            ast::Stmt::Continue => ast::Stmt::Continue,
            ast::Stmt::Switch(switch_stmt) => ast::Stmt::Switch(ast::SwitchStmt {
                expr: Self::substitute_expr(&switch_stmt.expr, type_args),
                cases: switch_stmt.cases.iter().map(|c| ast::Case {
                    pattern: c.pattern.clone(),
                    body: c.body.iter().map(|s| Self::substitute_stmt(s, type_args)).collect(),
                }).collect(),
                default: switch_stmt.default.as_ref().map(|b| b.iter().map(|s| Self::substitute_stmt(s, type_args)).collect()),
                span: switch_stmt.span,
            }),
            ast::Stmt::With(with_stmt) => ast::Stmt::With(ast::WithStmt {
                resource_expr: Self::substitute_expr(&with_stmt.resource_expr, type_args),
                var_name: with_stmt.var_name.clone(),
                var_type: with_stmt.var_type.as_ref().map(|t| Self::substitute_type(t, type_args)),
                body: with_stmt.body.iter().map(|s| Self::substitute_stmt(s, type_args)).collect(),
                span: with_stmt.span,
            }),
            ast::Stmt::Block(block) => {
                ast::Stmt::Block(block.iter().map(|s| Self::substitute_stmt(s, type_args)).collect())
            }
            ast::Stmt::TupleDestructure { names, expr, mutable, span } => {
                ast::Stmt::TupleDestructure {
                    names: names.clone(),
                    expr: Self::substitute_expr(expr, type_args),
                    mutable: *mutable,
                    span: *span,
                }
            }
        }
    }

    pub(super) fn substitute_var_decl(var_decl: &ast::VarDecl, type_args: &HashMap<String, ast::Type>) -> ast::VarDecl {
        ast::VarDecl {
            name: var_decl.name.clone(),
            typ: var_decl.typ.as_ref().map(|t| Self::substitute_type(t, type_args)),
            init: var_decl.init.as_ref().map(|e| Self::substitute_expr(e, type_args)),
            mutable: var_decl.mutable,
            span: var_decl.span,
        }
    }

    pub(super) fn substitute_class_member(member: &ast::ClassMember, type_args: &HashMap<String, ast::Type>) -> ast::ClassMember {
        match member {
            ast::ClassMember::Field(field_decl) => {
                ast::ClassMember::Field(ast::FieldDecl {
                    access: field_decl.access.clone(),
                    name: field_decl.name.clone(),
                    typ: Self::substitute_type(&field_decl.typ, type_args),
                    init: field_decl.init.as_ref().map(|e| Self::substitute_expr(e, type_args)),
                    span: field_decl.span,
                })
            }
            ast::ClassMember::Method(method_decl) => {
                ast::ClassMember::Method(Self::substitute_method_decl(method_decl, type_args))
            }
            ast::ClassMember::Constructor(ctor_decl) => {
                ast::ClassMember::Constructor(Self::substitute_method_decl(ctor_decl, type_args))
            }
        }
    }

    pub(super) fn substitute_method_decl(method_decl: &ast::MethodDecl, type_args: &HashMap<String, ast::Type>) -> ast::MethodDecl {
        let specialized_params: Vec<ast::Param> = method_decl.params.iter()
            .map(|p| ast::Param {
                name: p.name.clone(),
                typ: Self::substitute_type(&p.typ, type_args),
            })
            .collect();

        let specialized_return_type = method_decl.return_type.as_ref()
            .map(|t| Self::substitute_type(t, type_args));

        let specialized_body: Vec<ast::Stmt> = method_decl.body.iter()
            .map(|s| Self::substitute_stmt(s, type_args))
            .collect();

        ast::MethodDecl {
            access: method_decl.access.clone(),
            name: method_decl.name.clone(),
            type_params: method_decl.type_params.clone(),
            params: specialized_params,
            return_type: specialized_return_type,
            body: specialized_body,
            where_clause: method_decl.where_clause.clone(),
            span: method_decl.span,
        }
    }

    /// Check that a concrete type satisfies a constraint (e.g. `T: Display`).
    pub(super) fn check_constraint(&self, concrete_type: &ast::Type, constraint: &ast::Type) -> Result<(), String> {
        let type_name = concrete_type.name();
        let constraint_name = constraint.name();

        match constraint_name {
            "Display" => {
                match type_name {
                    "int" | "long" | "byte" | "short" | "vast" | "uvast" |
                    "float" | "double" | "half" | "quad" |
                    "bool" | "char" | "string" => Ok(()),
                    _ => {
                        // Check if the class implements Display
                        // For now, accept all class types (they all have toString)
                        Ok(())
                    }
                }
            }
            "Numeric" => {
                match type_name {
                    "int" | "long" | "byte" | "short" | "vast" | "uvast" |
                    "float" | "double" | "half" | "quad" => Ok(()),
                    _ => Err(format!(
                        "Type '{}' does not satisfy constraint 'Numeric'",
                        type_name
                    )),
                }
            }
            "Comparable" => {
                match type_name {
                    "int" | "long" | "byte" | "short" | "vast" | "uvast" |
                    "float" | "double" | "half" | "quad" |
                    "char" | "string" => Ok(()),
                    _ => {
                        // Check if the class implements Comparable
                        Ok(())
                    }
                }
            }
            _ => {
                // Unknown constraint - accept for forward compatibility
                Ok(())
            }
        }
    }

    /// Instantiate a generic function with concrete type arguments.
    /// Returns the function index of the specialized function.
    pub(super) fn instantiate_generic_function(&mut self, base_name: &str, type_args: &[ast::Type]) -> Result<u16, String> {
        let mangled = Self::mangle_name(base_name, type_args);

        // Check cache.
        if let Some(&idx) = self.mono_cache.get(&mangled) {
            return Ok(idx);
        }

        // Find the generic function declaration.
        let generic_idx = *self.generic_function_map.get(base_name)
            .ok_or_else(|| format!("Generic function '{}' not found", base_name))?;
        let generic_fn = self.generic_functions[generic_idx].clone();

        if type_args.len() != generic_fn.type_params.len() {
            return Err(format!(
                "Generic function '{}' expects {} type argument(s), got {}",
                base_name, generic_fn.type_params.len(), type_args.len()
            ));
        }

        // Build type_args map.
        let type_args_map: HashMap<String, ast::Type> = generic_fn.type_params.iter()
            .zip(type_args.iter())
            .map(|(tp, arg)| (tp.name.clone(), arg.clone()))
            .collect();

        // Check constraints from type_params.
        for tp in &generic_fn.type_params {
            if let Some(ref constraint) = tp.constraint {
                if let Some(concrete) = type_args_map.get(&tp.name) {
                    self.check_constraint(concrete, constraint)?;
                }
            }
        }

        // Check constraints from where_clause.
        for tp in &generic_fn.where_clause {
            if let Some(ref constraint) = tp.constraint {
                if let Some(concrete) = type_args_map.get(&tp.name) {
                    self.check_constraint(concrete, constraint)?;
                }
            }
        }

        // Substitute types in the function declaration.
        let specialized_params: Vec<ast::Param> = generic_fn.params.iter()
            .map(|p| ast::Param {
                name: p.name.clone(),
                typ: Self::substitute_type(&p.typ, &type_args_map),
            })
            .collect();

        let specialized_return_type = generic_fn.return_type.as_ref()
            .map(|t| Self::substitute_type(t, &type_args_map));

        let specialized_body: Vec<ast::Stmt> = generic_fn.body.iter()
            .map(|s| Self::substitute_stmt(s, &type_args_map))
            .collect();

        let specialized_fn = ast::FnDecl {
            access: generic_fn.access.clone(),
            name: mangled.clone(),
            type_params: vec![],
            params: specialized_params,
            return_type: specialized_return_type,
            body: specialized_body,
            sugar: generic_fn.sugar,
            where_clause: vec![],
            span: generic_fn.span,
        };

        // Register the specialized function.
        let fn_idx = self.functions.len() as u16;
        self.function_map.insert(mangled.clone(), fn_idx);
        self.functions.push(super::FunctionDef {
            name: mangled.clone(),
            arity: specialized_fn.params.len(),
            chunk: super::Chunk::new(),
            is_method: false,
            is_constructor: false,
            local_count: 0,
        });

        // Compile the specialized function body.
        self.compile_function(&specialized_fn)?;

        // Cache the result.
        self.mono_cache.insert(mangled, fn_idx);

        Ok(fn_idx)
    }

    /// Instantiate a generic class with concrete type arguments.
    /// Returns the class index of the specialized class.
    pub(super) fn instantiate_generic_class(&mut self, base_name: &str, type_args: &[ast::Type]) -> Result<u16, String> {
        let mangled = Self::mangle_name(base_name, type_args);

        // Check cache.
        if let Some(&idx) = self.mono_cache.get(&mangled) {
            return Ok(idx);
        }

        // Find the generic class declaration.
        let generic_idx = *self.generic_class_map.get(base_name)
            .ok_or_else(|| format!("Generic class '{}' not found", base_name))?;
        let generic_class = self.generic_classes[generic_idx].clone();

        if type_args.len() != generic_class.type_params.len() {
            return Err(format!(
                "Generic class '{}' expects {} type argument(s), got {}",
                base_name, generic_class.type_params.len(), type_args.len()
            ));
        }

        // Build type_args map.
        let type_args_map: HashMap<String, ast::Type> = generic_class.type_params.iter()
            .zip(type_args.iter())
            .map(|(tp, arg)| (tp.name.clone(), arg.clone()))
            .collect();

        // Check constraints.
        for tp in &generic_class.type_params {
            if let Some(ref constraint) = tp.constraint {
                if let Some(concrete) = type_args_map.get(&tp.name) {
                    self.check_constraint(concrete, constraint)?;
                }
            }
        }

        // Substitute types in class members.
        let specialized_members: Vec<ast::ClassMember> = generic_class.members.iter()
            .map(|m| Self::substitute_class_member(m, &type_args_map))
            .collect();

        let specialized_parent = generic_class.parent.as_ref()
            .map(|t| Self::substitute_type(t, &type_args_map));

        let specialized_ifaces: Vec<ast::Type> = generic_class.ifaces.iter()
            .map(|t| Self::substitute_type(t, &type_args_map))
            .collect();

        let specialized_class = ast::ClassDecl {
            name: mangled.clone(),
            type_params: vec![],
            parent: specialized_parent,
            ifaces: specialized_ifaces,
            members: specialized_members,
            span: generic_class.span,
        };

        // Register the specialized class.
        self.register_class(&specialized_class)?;

        // Compile the specialized class methods.
        self.compile_class_methods(&specialized_class)?;

        // Get the class index.
        let class_idx = *self.class_map.get(&mangled).unwrap();

        // Cache the result.
        self.mono_cache.insert(mangled, class_idx);

        Ok(class_idx)
    }
}
