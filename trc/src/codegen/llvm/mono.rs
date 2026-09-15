// Monomorphization for generic classes and functions

use super::{fn_sig_has_type_param, LlvmBackend};
use std::collections::HashMap;
use crate::ast::{ClassDecl, ClassMember, Type};

impl<'ctx> LlvmBackend<'ctx> {
    // -----------------------------------------------------------------------
    // Monomorphization for generic classes and functions.
    // -----------------------------------------------------------------------

    /// Find a generic class declaration by bare or module-suffixed name.
    pub(super) fn find_generic_class_decl(&self, base: &str) -> Option<(String, ClassDecl)> {
        if let Some(d) = self.class_decls.get(base) {
            if !d.type_params.is_empty() {
                return Some((base.to_string(), d.clone()));
            }
        }
        let suffix = format!(".{}", base);
        self.class_decls
            .iter()
            .find(|(k, d)| !d.type_params.is_empty() && k.ends_with(&suffix))
            .map(|(k, d)| (k.clone(), d.clone()))
    }

    /// Substitute a generic method's return type for a concrete receiver
    /// without compiling anything. Used by inference paths before the
    /// specialization is ensured.
    pub(super) fn generic_method_return(&self, recv_ty: &Type, method: &str) -> Option<Type> {
        let base = recv_ty.name();
        let (_, decl) = self.find_generic_class_decl(base)?;
        if decl.type_params.len() != recv_ty.params().len() {
            return None;
        }
        let map: std::collections::HashMap<String, Type> = decl
            .type_params
            .iter()
            .map(|tp| tp.name.clone())
            .zip(recv_ty.params().iter().cloned())
            .collect();
        for member in &decl.members {
            match member {
                crate::ast::ClassMember::Method(m) if m.name == method => {
                    return m.return_type.as_ref().map(|t| {
                        crate::bytecode::compiler::Compiler::substitute_type(t, &map)
                    });
                }
                _ => {}
            }
        }
        None
    }

    /// Pure name resolution for a possibly-generic class type: returns the
    /// mangled name without compiling anything. Used by inference paths.
    pub(super) fn mono_mangled_for(&self, ty: &Type) -> Option<String> {
        use crate::bytecode::compiler::Compiler as BcCompiler;
        let base = ty.name();
        if ty.params().is_empty() {
            return None;
        }
        if base == "ArrayList" || base == "HashMap" || base == "array" {
            return None;
        }
        let (key, decl) = self.find_generic_class_decl(base)?;
        if decl.type_params.len() != ty.params().len() {
            return None;
        }
        Some(BcCompiler::mangle_name(&key, ty.params()))
    }

    /// Instantiate a generic class with concrete type arguments: substitute
    /// members, register under the mangled name, and compile. Returns the
    /// mangled name. Results are cached; nesting is depth-bounded.
    pub(super) fn instantiate_llvm_class(&mut self, base: &str, args: &[Type]) -> Result<String, String> {
        use crate::bytecode::compiler::Compiler as BcCompiler;
        let (_, decl) = self.find_generic_class_decl(base).ok_or_else(|| {
            format!("codegen: generic class '{}' not found", base)
        })?;
        if args.len() != decl.type_params.len() {
            return Err(format!(
                "codegen: generic class '{}' expects {} type argument(s), got {}",
                base,
                decl.type_params.len(),
                args.len()
            ));
        }
        if args.iter().any(fn_sig_has_type_param) {
            return Err(format!(
                "codegen: cannot instantiate generic class '{}' with unresolved type arguments",
                base
            ));
        }
        let map: std::collections::HashMap<String, Type> = decl
            .type_params
            .iter()
            .map(|tp| tp.name.clone())
            .zip(args.iter().cloned())
            .collect();
        {
            let checker = BcCompiler::new();
            for tp in &decl.type_params {
                if let Some(c) = &tp.constraint {
                    if let Some(concrete) = map.get(&tp.name) {
                        checker.check_constraint(concrete, c)?;
                    }
                }
            }
        }
        let mangled = BcCompiler::mangle_name(&decl.name, args);
        if self.mono_cache.contains(&mangled) {
            return Ok(mangled);
        }
        if self.mono_active.contains(&mangled) {
            // Re-entrant use while its own methods compile (e.g. `clone`
            // constructing the same instantiation): the struct layout is
            // already registered, and the missing constructor resolves
            // once the outer instantiation finishes compiling its members
            // in declaration order.
            return Ok(mangled);
        }
        if self.mono_depth >= 32 {
            return Err(format!(
                "codegen: monomorphization too deep for '{}' (cyclic generic instantiation?)",
                mangled
            ));
        }
        self.mono_depth += 1;
        let result = self.instantiate_llvm_class_inner(&decl, &map, &mangled);
        self.mono_depth -= 1;
        if result.is_ok() {
            self.mono_cache.insert(mangled.clone());
        }
        result?;
        Ok(mangled)
    }

    /// Substitute, register, and compile one specialization. The parent (if
    /// generic) is instantiated first so field layout and super calls see a
    /// complete struct.
    pub(super) fn instantiate_llvm_class_inner(
        &mut self,
        decl: &ClassDecl,
        map: &std::collections::HashMap<String, Type>,
        mangled: &str,
    ) -> Result<(), String> {
        use crate::bytecode::compiler::Compiler as BcCompiler;
        let members: Vec<ClassMember> = decl
            .members
            .iter()
            .map(|m| BcCompiler::substitute_class_member(m, map))
            .collect();
        let parent = decl
            .parent
            .as_ref()
            .map(|t| BcCompiler::substitute_type(t, map));
        let ifaces: Vec<Type> = decl
            .ifaces
            .iter()
            .map(|t| BcCompiler::substitute_type(t, map))
            .collect();
        // Instantiate a generic parent first so its struct exists for field
        // layout; then rewrite the link to the mangled name.
        let parent = match parent {
            Some(p) if !p.params().is_empty() => {
                let resolved = self.instantiate_llvm_class(p.name(), p.params())?;
                Some(Type::generic(&resolved, vec![]))
            }
            other => other,
        };
        let specialized = ClassDecl {
            name: mangled.to_string(),
            type_params: vec![],
            parent,
            ifaces,
            members,
            span: decl.span,
        };
        self.mono_active.insert(mangled.to_string());
        // Save the caller's emission state: compiling methods moves the
        // builder all over the module.
        let saved_block = self.builder.get_insert_block();
        let saved_locals = self.locals.clone();
        let saved_this = self.current_this;
        let saved_class_name = self.current_class_name.clone();
        let saved_ret_ty = self.current_ret_ty.clone();
        self.class_decls
            .insert(mangled.to_string(), specialized.clone());
        let mut methods = HashMap::new();
        for member in &specialized.members {
            if let ClassMember::Method(m) | ClassMember::Constructor(m) = member {
                let ret = m
                    .return_type
                    .clone()
                    .unwrap_or_else(|| Type::simple("void"));
                methods.insert(m.name.clone(), ret);
            }
        }
        self.class_method_returns
            .insert(mangled.to_string(), methods);
        let result = self.compile_class_decl(&specialized);
        self.locals = saved_locals;
        self.current_this = saved_this;
        self.current_class_name = saved_class_name;
        self.current_ret_ty = saved_ret_ty;
        if let Some(block) = saved_block {
            self.builder.position_at_end(block);
        }
        self.mono_active.remove(mangled);
        result
    }

    /// Unify one formal parameter type against an actual argument type,
    /// binding generic parameters into `bound`. Concrete mismatches are
    /// ignored here (downstream coercion or errors handle them); only
    /// conflicting bindings fail.
    pub(super) fn unify_type_arg(
        formal: &Type,
        actual: &Type,
        params: &[String],
        bound: &mut std::collections::HashMap<String, Type>,
    ) -> bool {
        if let Type::Named { name, params: fp } = formal {
            if fp.is_empty()
                && name.len() == 1
                && name.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                && params.iter().any(|p| p == name)
            {
                if let Some(prev) = bound.get(name) {
                    return prev == actual;
                }
                bound.insert(name.clone(), actual.clone());
                return true;
            }
            if let Type::Named {
                name: aname,
                params: aparams,
            } = actual
            {
                if name == aname && fp.len() == aparams.len() {
                    return fp
                        .iter()
                        .zip(aparams.iter())
                        .all(|(f, a)| Self::unify_type_arg(f, a, params, bound));
                }
            }
            return true;
        }
        match (formal, actual) {
            (Type::Ref(f), Type::Ref(a)) | (Type::MutRef(f), Type::MutRef(a)) => {
                Self::unify_type_arg(f, a, params, bound)
            }
            (Type::Tuple(fs), Type::Tuple(ats)) => {
                fs.len() == ats.len()
                    && fs
                        .iter()
                        .zip(ats.iter())
                        .all(|(f, a)| Self::unify_type_arg(f, a, params, bound))
            }
            _ => true,
        }
    }

    /// Deduce concrete type arguments for generic parameters from call
    /// arguments. Returns None when any parameter stays unbound.
    pub(super) fn deduce_type_args(
        formals: &[Type],
        actuals: &[Type],
        params: &[String],
    ) -> Option<Vec<Type>> {
        let mut bound = std::collections::HashMap::new();
        for (f, a) in formals.iter().zip(actuals.iter()) {
            if !Self::unify_type_arg(f, a, params, &mut bound) {
                return None;
            }
        }
        params.iter().map(|p| bound.get(p).cloned()).collect()
    }

    /// Instantiate a generic top-level function with concrete type arguments.
    /// Returns the mangled name. Results are cached; nesting is bounded.
    pub(super) fn instantiate_llvm_function(
        &mut self,
        base: &str,
        args: &[Type],
        arity: usize,
    ) -> Result<String, String> {
        use crate::bytecode::compiler::Compiler as BcCompiler;
        let overloads = self.generic_fn_decls.get(base).cloned().ok_or_else(|| {
            format!("codegen: generic function '{}' not found", base)
        })?;
        let decl = overloads
            .iter()
            .find(|d| d.params.len() == arity)
            .or_else(|| overloads.first())
            .cloned()
            .ok_or_else(|| format!("codegen: generic function '{}' not found", base))?;
        let type_param_names: Vec<String> =
            decl.type_params.iter().map(|tp| tp.name.clone()).collect();
        if args.len() != type_param_names.len() {
            return Err(format!(
                "codegen: generic function '{}' expects {} type argument(s), got {}",
                base,
                type_param_names.len(),
                args.len()
            ));
        }
        let map: std::collections::HashMap<String, Type> = type_param_names
            .iter()
            .cloned()
            .zip(args.iter().cloned())
            .collect();
        {
            let checker = BcCompiler::new();
            for tp in &decl.type_params {
                if let Some(c) = &tp.constraint {
                    if let Some(concrete) = map.get(&tp.name) {
                        checker.check_constraint(concrete, c)?;
                    }
                }
            }
        }
        let mangled = format!(
            "{}_a{}",
            BcCompiler::mangle_name(base, args),
            arity
        );
        if self.mono_cache.contains(&mangled) {
            return Ok(mangled);
        }
        if self.mono_active.contains(&mangled) {
            return Ok(mangled);
        }
        if self.mono_depth >= 32 {
            return Err(format!(
                "codegen: monomorphization too deep for '{}' (cyclic generic instantiation?)",
                mangled
            ));
        }
        self.mono_depth += 1;
        let specialized_params: Vec<crate::ast::Param> = decl
            .params
            .iter()
            .map(|p| crate::ast::Param {
                name: p.name.clone(),
                typ: BcCompiler::substitute_type(&p.typ, &map),
            })
            .collect();
        let specialized_return_type = decl
            .return_type
            .as_ref()
            .map(|t| BcCompiler::substitute_type(t, &map));
        let specialized_body: Vec<crate::ast::Stmt> = decl
            .body
            .iter()
            .map(|s| BcCompiler::substitute_stmt(s, &map))
            .collect();
        let specialized = crate::ast::FnDecl {
            access: decl.access.clone(),
            name: mangled.clone(),
            type_params: vec![],
            params: specialized_params,
            return_type: specialized_return_type,
            body: specialized_body,
            sugar: decl.sugar,
            where_clause: vec![],
            span: decl.span,
        };
        self.mono_active.insert(mangled.clone());
        self.function_decls
            .insert(mangled.clone(), specialized.clone());
        let saved_block = self.builder.get_insert_block();
        let saved_locals = self.locals.clone();
        let saved_this = self.current_this;
        let saved_class_name = self.current_class_name.clone();
        let saved_ret_ty = self.current_ret_ty.clone();
        let result = self.compile_function(&specialized);
        self.locals = saved_locals;
        self.current_this = saved_this;
        self.current_class_name = saved_class_name;
        self.current_ret_ty = saved_ret_ty;
        if let Some(block) = saved_block {
            self.builder.position_at_end(block);
        }
        self.mono_active.remove(&mangled);
        self.mono_depth -= 1;
        if result.is_ok() {
            self.mono_cache.insert(mangled.clone());
        }
        result?;
        Ok(mangled)
    }

    /// Resolve a possibly-generic class type to its compiled name,
    /// instantiating on demand. Builtins and concrete types pass through.
    pub(super) fn resolve_mono_class(&mut self, ty: &Type) -> Result<String, String> {
        let base = ty.name().to_string();
        if ty.params().is_empty() {
            return Ok(base);
        }
        if base == "ArrayList" || base == "HashMap" || base == "array" {
            return Ok(base);
        }
        match self.find_generic_class_decl(&base) {
            Some(_) => self.instantiate_llvm_class(&base, ty.params()),
            None => Ok(base),
        }
    }
}
