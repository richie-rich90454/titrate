// Closures, tuples, results, and errors

use super::types as llvm_types;
use super::{CatchContext, LlvmBackend, LocalVar};
use crate::ast::{Expr, Stmt, Type};
use inkwell::module::Linkage;
use inkwell::types::BasicType;
use inkwell::values::{BasicValueEnum, PointerValue};
use inkwell::AddressSpace;

/// Fallback capture discovery for closures where the analyzer left
/// `captured_vars` empty. Collect identifiers from the closure body that are
/// bound in the enclosing function and are not closure parameters.
fn scan_capture_expr(
    expr: &Expr,
    locals: &std::collections::HashSet<&str>,
    params: &[String],
    out: &mut Vec<String>,
) {
    let add = |name: &str, out: &mut Vec<String>| {
        if !params.iter().any(|p| p == name)
            && locals.contains(name)
            && !out.iter().any(|n| n == name)
        {
            out.push(name.to_string());
        }
    };
    match expr {
        Expr::Identifier(n, _) => add(n, out),
        Expr::Binary(l, _, r, _)
        | Expr::Assign(l, r, _)
        | Expr::Index(l, r, _)
        | Expr::Range(l, r, _)
        | Expr::RangeInclusive(l, r, _) => {
            scan_capture_expr(l, locals, params, out);
            scan_capture_expr(r, locals, params, out);
        }
        Expr::Unary(_, x, _)
        | Expr::OwnedDeref(x, _)
        | Expr::ErrorPropagation(x, _)
        | Expr::Cast(x, _, _)
        | Expr::Is(x, _, _)
        | Expr::RefExpr(x, _, _)
        | Expr::RegionAlloc(_, x, _)
        | Expr::MemberAccess(x, _, _) => scan_capture_expr(x, locals, params, out),
        Expr::Call(c, args, _, _) => {
            scan_capture_expr(c, locals, params, out);
            for a in args {
                scan_capture_expr(a, locals, params, out);
            }
        }
        Expr::New(_, args, _)
        | Expr::Tuple(args, _)
        | Expr::StaticCall { args, .. } => {
            for a in args {
                scan_capture_expr(a, locals, params, out);
            }
        }
        Expr::Ternary {
            condition,
            then_expr,
            else_expr,
            ..
        } => {
            scan_capture_expr(condition, locals, params, out);
            scan_capture_expr(then_expr, locals, params, out);
            scan_capture_expr(else_expr, locals, params, out);
        }
        Expr::Closure { body, expr, .. } => {
            for s in body {
                scan_capture_stmt(s, locals, params, out);
            }
            if let Some(e) = expr {
                scan_capture_expr(e, locals, params, out);
            }
        }
        Expr::UnsafeBlock(b, _) | Expr::RegionBlock(b, _) => {
            for s in b {
                scan_capture_stmt(s, locals, params, out);
            }
        }
        _ => {}
    }
}

fn scan_capture_stmt(
    stmt: &Stmt,
    locals: &std::collections::HashSet<&str>,
    params: &[String],
    out: &mut Vec<String>,
) {
    match stmt {
        Stmt::Expr(e) | Stmt::Throw(e, _) => scan_capture_expr(e, locals, params, out),
        Stmt::VarDecl(vd) | Stmt::ConstDecl(vd) => {
            if let Some(e) = &vd.init {
                scan_capture_expr(e, locals, params, out);
            }
        }
        Stmt::Return(Some(e)) => scan_capture_expr(e, locals, params, out),
        Stmt::If(s) => {
            scan_capture_expr(&s.condition, locals, params, out);
            for x in &s.then_branch {
                scan_capture_stmt(x, locals, params, out);
            }
            if let Some(b) = &s.else_branch {
                for x in b {
                    scan_capture_stmt(x, locals, params, out);
                }
            }
        }
        Stmt::While(s) => {
            scan_capture_expr(&s.condition, locals, params, out);
            for x in &s.body {
                scan_capture_stmt(x, locals, params, out);
            }
        }
        Stmt::Block(b) => {
            for x in b {
                scan_capture_stmt(x, locals, params, out);
            }
        }
        _ => {}
    }
}

impl<'ctx> LlvmBackend<'ctx> {
    /// Compile an `Owned<T>` dereference expression (`*expr`).
    ///
    /// The operand must evaluate to an `Owned<T>` value, which we represent
    /// as an `i8*` heap pointer stored in an alloca. The result is the
    /// loaded inner value.
    ///
    /// For Phase 1, we look up the operand's alloca (if it's an identifier)
    /// and use `super::ownership::load_owned_value` to load the inner value. The
    /// inner type is inferred from the variable's declared type.
    pub(super) fn compile_owned_deref(&mut self, inner: &Expr) -> Result<BasicValueEnum<'ctx>, String> {
        // Get the pointer alloca for the Owned<T> value.
        let (ptr_alloca, inner_ty) = match inner {
            Expr::Identifier(name, _) => {
                let var = self.locals.get(name).ok_or_else(|| {
                    format!("codegen: unknown variable '{}' for owned deref", name)
                })?;
                // The variable's LLVM type is `i8*` (an opaque pointer).
                // We don't track the inner type separately, so we default
                // to i64 for the loaded value. This is sufficient for
                // Phase 1 micro-tests.
                let inner_ty = self.context.i64_type().into();
                (var.ptr, inner_ty)
            }
            _ => {
                // For non-identifier operands, evaluate the expression to
                // get a pointer value, store it in a temporary alloca, and
                // load from it.
                let v = self.compile_expr(inner)?;
                let i8_ptr_ty = self.context.ptr_type(AddressSpace::default());
                let alloca = self
                    .builder
                    .build_alloca(i8_ptr_ty, "deref.tmp")
                    .map_err(|e| format!("build_alloca deref.tmp failed: {:?}", e))?;
                self.builder
                    .build_store(alloca, v)
                    .map_err(|e| format!("build_store deref.tmp failed: {:?}", e))?;
                (alloca, self.context.i64_type().into())
            }
        };
        super::ownership::load_owned_value(self.context, &self.builder, inner_ty, ptr_alloca, "deref")
    }

    /// Compile a borrow expression (`&expr` or `&mut expr`).
    ///
    /// For an identifier, this returns the address of the existing alloca.
    /// For other expressions, the value is evaluated and stored in a fresh
    /// alloca, whose address is returned.
    pub(super) fn compile_ref_expr(
        &mut self,
        inner: &Expr,
        _ref_kind: &crate::ast::RefKind,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let i8_ptr_ty = self.context.ptr_type(AddressSpace::default());
        match inner {
            Expr::Identifier(name, _) => {
                let var = self
                    .locals
                    .get(name)
                    .ok_or_else(|| format!("codegen: unknown variable '{}' for borrow", name))?;
                // Cast the alloca pointer to i8* (opaque pointer in LLVM 15+).
                let ptr = if var.ptr.get_type() == i8_ptr_ty {
                    var.ptr
                } else {
                    self.builder
                        .build_bit_cast(var.ptr, i8_ptr_ty, "ref.cast")
                        .map_err(|e| format!("build_bit_cast ref failed: {:?}", e))?
                        .into_pointer_value()
                };
                Ok(ptr.into())
            }
            _ => {
                // Evaluate the expression and store it in a temporary alloca.
                let v = self.compile_expr(inner)?;
                let ty = v.get_type();
                let alloca = self
                    .builder
                    .build_alloca(ty, "ref.tmp")
                    .map_err(|e| format!("build_alloca ref.tmp failed: {:?}", e))?;
                self.builder
                    .build_store(alloca, v)
                    .map_err(|e| format!("build_store ref.tmp failed: {:?}", e))?;
                let ptr = if alloca.get_type() == i8_ptr_ty {
                    alloca
                } else {
                    self.builder
                        .build_bit_cast(alloca, i8_ptr_ty, "ref.tmp.cast")
                        .map_err(|e| format!("build_bit_cast ref.tmp failed: {:?}", e))?
                        .into_pointer_value()
                };
                Ok(ptr.into())
            }
        }
    }

    /// Compile a `region.alloc<T>(expr)` expression.
    ///
    /// For Phase 1, this is treated like an `Owned<T>` allocation: we
    /// allocate heap memory, store the initial value, and return the
    /// pointer. Region lifetime markers are not emitted here (they are
    /// emitted by `compile_unsafe_block` for `region` blocks).
    pub(super) fn compile_region_alloc(
        &mut self,
        ty: &Type,
        init: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let inner_ty = llvm_types::llvm_type(self.context, ty)?;
        let init_val = self.compile_expr(init)?;
        let init_val = self.cast_value_to_type(init_val, inner_ty)?;
        let (ptr_alloca, _drop_flag) = super::ownership::alloc_owned(
            self.context,
            &self.builder,
            &self.module,
            init_val,
            inner_ty,
            "region.alloc",
        )?;
        // Return the raw i8* pointer.
        let i8_ptr_ty = self.context.ptr_type(AddressSpace::default());
        let raw = self
            .builder
            .build_load(i8_ptr_ty, ptr_alloca, "region.ptr")
            .map_err(|e| format!("build_load region.ptr failed: {:?}", e))?;
        Ok(raw)
    }

    /// Compile an `unsafe { ... }` block.
    ///
    /// For Phase 1, this simply compiles the body statements in the current
    /// scope without any safety checks. The block evaluates to the last
    /// expression statement's value (or `0` if the block is empty).
    pub(super) fn compile_unsafe_block(&mut self, block: &[Stmt]) -> Result<BasicValueEnum<'ctx>, String> {
        let i32_ty = self.context.i32_type();
        let mut last_val: BasicValueEnum<'ctx> = i32_ty.const_int(0, false).into();
        self.ownership.enter_scope();
        for s in block {
            // If the statement is an expression statement, capture its value.
            if let Stmt::Expr(e) = s {
                last_val = self.compile_expr(e)?;
            } else {
                self.compile_stmt(s)?;
            }
        }
        self.emit_scope_cleanup()?;
        Ok(last_val)
    }

    /// Compile a `with` statement.
    ///
    /// For Phase 1, this evaluates the resource expression, binds it to the
    /// optional variable, compiles the body, and that's it. No automatic
    /// `.close()` call is emitted (Phase 1 limitation).
    pub(super) fn compile_with(&mut self, with_stmt: &crate::ast::WithStmt) -> Result<(), String> {
        // Evaluate the resource expression.
        let resource_val = self.compile_expr(&with_stmt.resource_expr)?;
        // Bind it to the variable if a name was given.
        if let Some(name) = &with_stmt.var_name {
            let ty = with_stmt
                .var_type
                .as_ref()
                .map(|t| llvm_types::llvm_type(self.context, t))
                .transpose()?
                .unwrap_or_else(|| resource_val.get_type());
            let alloca = self
                .builder
                .build_alloca(ty, name)
                .map_err(|e| format!("build_alloca with '{}' failed: {:?}", name, e))?;
            let val = self.cast_value_to_type(resource_val, ty)?;
            self.builder
                .build_store(alloca, val)
                .map_err(|e| format!("build_store with '{}' failed: {:?}", name, e))?;
            self.locals.insert(
                name.clone(),
                LocalVar {
                    ptr: alloca,
                    ty,
                    full_type: with_stmt.var_type.clone(),
                    titrate_type: with_stmt.var_type.as_ref().map(|t| t.name().to_string()),
                    shared: false,
                    closure_sig: None,
                },
            );
        }
        // Compile the body with scope-based cleanup tracking. A `return`
        // inside must not become a tail call: emit_scope_cleanup below is
        // cleanup that a tail call would skip.
        self.ownership.enter_scope();
        self.cleanup_scope_depth += 1;
        for s in &with_stmt.body {
            self.compile_stmt(s)?;
        }
        self.cleanup_scope_depth -= 1;
        self.emit_scope_cleanup()?;
        Ok(())
    }

    /// Emit cleanup actions for the current scope. This pops the current
    /// scope's cleanup actions from the ownership stack and emits them
    /// (only if the current basic block has no terminator).
    pub(super) fn emit_scope_cleanup(&mut self) -> Result<(), String> {
        let actions = self.ownership.exit_scope();
        let has_terminator = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_some();
        if !has_terminator && !actions.is_empty() {
            super::ownership::emit_cleanup(self.context, &self.builder, &self.module, &actions)?;
        }
        Ok(())
    }

    /// Compile a tuple expression `(a, b, c)` into an anonymous struct.
    pub(super) fn compile_tuple(&mut self, elements: &[Expr]) -> Result<BasicValueEnum<'ctx>, String> {
        let mut vals = Vec::with_capacity(elements.len());
        for e in elements {
            vals.push(self.compile_expr(e)?);
        }
        super::tuple_codegen::emit_tuple_construct(self.context, &self.builder, &self.module, &vals)
    }

    /// Compile tuple destructuring: `let (a, b) = tuple_expr`
    pub(super) fn compile_tuple_destructure(&mut self, names: &[String], expr: &Expr) -> Result<(), String> {
        let tuple_val = self.compile_expr(expr)?;
        let struct_val = match tuple_val {
            BasicValueEnum::StructValue(sv) => sv,
            _ => return Err("tuple destructure: expected struct value".to_string()),
        };
        for (i, name) in names.iter().enumerate() {
            let field_val =
                super::tuple_codegen::emit_tuple_field_access(&self.builder, struct_val.into(), i as u32)?;
            let ty = field_val.get_type();
            let alloca = self
                .builder
                .build_alloca(ty, name)
                .map_err(|e| format!("build_alloca destructure '{}' failed: {:?}", name, e))?;
            self.builder
                .build_store(alloca, field_val)
                .map_err(|e| format!("build_store destructure '{}' failed: {:?}", name, e))?;
            self.locals.insert(
                name.clone(),
                LocalVar {
                    ptr: alloca,
                    ty,
                    full_type: None,
                    titrate_type: None,
                    shared: false,
                closure_sig: None,
                },
            );
        }
        Ok(())
    }

    /// Recover a closure's concrete return type when its literal omitted the
    /// annotation (the parser defaults those to `void`). Dropping the value of
    /// such a closure leaves callers reading garbage from a void trampoline.
    fn infer_closure_return_type(
        &self,
        params: &[(String, Type)],
        body: &[Stmt],
        expr: Option<&Expr>,
    ) -> Option<Type> {
        fn param_type(params: &[(String, Type)], e: &Expr) -> Option<Type> {
            match e {
                Expr::Identifier(name, _) => params
                    .iter()
                    .find(|(n, _)| n == name)
                    .map(|(_, t)| t.clone()),
                Expr::Binary(l, _, r, _) => param_type(params, l).or_else(|| param_type(params, r)),
                Expr::Unary(_, x, _) | Expr::Cast(x, _, _) | Expr::ErrorPropagation(x, _) => {
                    param_type(params, x)
                }
                _ => None,
            }
        }
        let from_expr = |e: &Expr| {
            let ty = param_type(params, e).unwrap_or_else(|| self.infer_expr_type(e));
            if ty.name() == "void" || ty.name() == "unknown" {
                None
            } else {
                Some(ty)
            }
        };
        if let Some(e) = expr {
            return from_expr(e);
        }
        for stmt in body.iter().rev() {
            if let Stmt::Return(Some(e)) = stmt {
                if let Expr::Identifier(name, _) = e {
                    for prior in body {
                        if let Stmt::VarDecl(vd) | Stmt::ConstDecl(vd) = prior {
                            if &vd.name == name {
                                if let Some(t) = &vd.typ {
                                    return Some(t.clone());
                                }
                            }
                        }
                    }
                }
                return from_expr(e);
            }
        }
        None
    }

    /// Compile a closure expression into a `{ i8*, i8* }` struct.
    ///
    /// Phase 1: generates a trampoline function for each closure. The closure
    /// value is a 2-pointer struct:
    ///   - field 0: bitcast of the trampoline function to i8*
    ///   - field 1: capture data pointer (null for non-capturing closures)
    ///
    /// The trampoline signature is `fn(capture_data: i8*, params...) -> ret`.
    pub(super) fn compile_closure(
        &mut self,
        params: &[(String, Type)],
        return_type: &Type,
        body: &[Stmt],
        expr: Option<&Expr>,
        captured_vars: &[String],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let i8_ptr_ty = self.context.ptr_type(AddressSpace::default());
        let void_ty = self.context.void_type();

        // The analyzer occasionally leaves `captured_vars` empty for a closure
        // returned from a function. Recover captures from the body so the
        // trampoline still gets the enclosing bindings it needs.
        let param_names: Vec<String> = params.iter().map(|(n, _)| n.clone()).collect();
        let mut inferred = Vec::new();
        if captured_vars.is_empty() {
            let locals: std::collections::HashSet<&str> =
                self.locals.keys().map(|s| s.as_str()).collect();
            for s in body {
                scan_capture_stmt(s, &locals, &param_names, &mut inferred);
            }
            if let Some(e) = expr {
                scan_capture_expr(e, &locals, &param_names, &mut inferred);
            }
        }
        let captured_vars: &[String] = if inferred.is_empty() {
            captured_vars
        } else {
            &inferred
        };

        // Generate a unique name for the trampoline.
        let closure_id = self.closure_counter;
        self.closure_counter += 1;
        let tramp_name = format!("_closure_{}", closure_id);

        // Build the trampoline function type: fn(i8* capture_data, params...) -> ret
        // Container params pass as slot pointers (reference semantics, like
        // user functions); the trampoline binds them with bind_param.
        let mut param_tys: Vec<inkwell::types::BasicMetadataTypeEnum> = Vec::new();
        param_tys.push(i8_ptr_ty.into()); // capture_data
        for (_, ty) in params {
            let llvm_ty = self.user_param_llvm_type(ty)?;
            param_tys.push(llvm_ty.into());
        }
        let effective_ret = if llvm_types::is_void(return_type) {
            self.infer_closure_return_type(params, body, expr)
                .unwrap_or_else(|| return_type.clone())
        } else {
            return_type.clone()
        };
        let ret_ty = self.sig_ret_llvm_type(Some(&effective_ret))?;
        let fn_type = match ret_ty {
            Some(rt) => rt.fn_type(&param_tys, false),
            None => void_ty.fn_type(&param_tys, false),
        };

        let tramp_fn = self
            .module
            .add_function(&tramp_name, fn_type, Some(Linkage::Internal));

        // Save current state (including the insert position: everything below
        // must be emitted into the trampoline, and the caller resumes in its
        // own block afterwards).
        let saved_block = self.builder.get_insert_block();
        let saved_ret_ty = self.current_ret_ty.clone();
        self.current_ret_ty = Some(effective_ret.clone());
        let saved_locals = self.locals.clone();
        self.locals.clear();
        // A self-recursive call in the closure body must not reuse the
        // enclosing function's tail-call trampoline (its block and slots
        // belong to another function). Likewise a `throw` in the closure
        // must not target the caller's catch block.
        let saved_tco = self.tco_trampoline.take();
        let saved_catch_stack = std::mem::take(&mut self.catch_stack);

        // Create entry block.
        let entry = self.context.append_basic_block(tramp_fn, "entry");
        self.builder.position_at_end(entry);

        // Bind the capture_data parameter (i8*).
        let capture_data = tramp_fn
            .get_first_param()
            .ok_or("codegen: closure trampoline has no capture_data param")?;
        let capture_data_ptr = capture_data.into_pointer_value();

        // Bind closure parameters (skip capture_data, already handled).
        // Container params alias the caller's slot (reference semantics).
        let mut param_iter = tramp_fn.get_params().into_iter().skip(1);
        for (name, ty) in params {
            let param_val = param_iter
                .next()
                .ok_or_else(|| format!("missing param '{}'", name))?;
            self.bind_param(name, ty, param_val)?;
        }

        // If there are captured variables, bind them from the capture data.
        // Phase 1: captured vars are stored sequentially in the capture buffer.
        if !captured_vars.is_empty() {
            let i32_ty = self.context.i32_type();
            // Capture offsets are byte offsets: GEP must use i8 as its element
            // type, otherwise an opaque `ptr` element scales index 8 to 64 bytes.
            let i8_ty = self.context.i8_type();
            for (i, var_name) in captured_vars.iter().enumerate() {
                let offset = (i as u64) * 8;
                let offset_val = i32_ty.const_int(offset, false);
                let gep = unsafe {
                    self.builder.build_gep(
                        i8_ty,
                        capture_data_ptr,
                        &[offset_val],
                        &format!("capture.{}.gep", var_name),
                    )
                }
                .map_err(|e| format!("capture gep '{}': {:?}", var_name, e))?;
                let opaque_ptr = self.context.ptr_type(AddressSpace::default());
                let orig = saved_locals.get(var_name).cloned().ok_or_else(|| {
                    format!("codegen: no outer binding for captured var '{}'", var_name)
                })?;
                let slot_ptr = self
                    .builder
                    .build_bit_cast(gep, opaque_ptr, &format!("capture.{}.ptr", var_name))
                    .map_err(|e| format!("capture bit_cast '{}': {:?}", var_name, e))?
                    .into_pointer_value();
                let cell = self
                    .builder
                    .build_load(opaque_ptr, slot_ptr, &format!("capture.{}.cell", var_name))
                    .map_err(|e| format!("capture load '{}': {:?}", var_name, e))?
                    .into_pointer_value();
                self.locals.insert(
                    var_name.clone(),
                    LocalVar {
                        ptr: cell,
                        ty: orig.ty,
                        full_type: orig.full_type.clone(),
                        titrate_type: orig.titrate_type.clone(),
                        shared: true,
                        closure_sig: orig.closure_sig.clone(),
                    },
                );
            }
        }

        // Compile the closure body.
        for s in body {
            self.compile_stmt(s)?;
        }

        // If there's an expression body (arrow closure), compile and return it.
        if let Some(e) = expr {
            let val = self.compile_expr(e)?;
            if tramp_fn.get_type().get_return_type().is_some() {
                self.builder
                    .build_return(Some(&val))
                    .map_err(|e| format!("closure return build: {:?}", e))?;
            }
        }

        // If the function has no terminator, add one.
        if self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_none()
        {
            self.builder
                .build_return(None)
                .map_err(|e| format!("closure ret void: {:?}", e))?;
        }

        // Restore caller state and resume emitting in the caller's block.
        // Without this, the closure struct and all following code would be
        // emitted into the trampoline (after its terminator), corrupting
        // both functions.
        self.locals = saved_locals;
        self.current_ret_ty = saved_ret_ty;
        self.tco_trampoline = saved_tco;
        self.catch_stack = saved_catch_stack;
        if let Some(block) = saved_block {
            self.builder.position_at_end(block);
        }

        // Build the closure struct: { i8*, i8* } = { fn_ptr, capture_data }
        let closure_struct_ty = self
            .context
            .struct_type(&[i8_ptr_ty.into(), i8_ptr_ty.into()], false);

        let fn_ptr = tramp_fn.as_global_value().as_pointer_value();
        let fn_ptr_i8 = self
            .builder
            .build_bit_cast(fn_ptr, i8_ptr_ty, &format!("{}.cast", tramp_name))
            .map_err(|e| format!("fn_ptr bit_cast: {:?}", e))?;

        let capture_ptr = if captured_vars.is_empty() {
            i8_ptr_ty.const_null()
        } else {
            // Allocate the capture buffer and store captured values.
            // Phase 1: each captured variable is an i8* (8 bytes).
            let i64_ty = self.context.i64_type();
            let capture_size = i64_ty.const_int((captured_vars.len() * 8) as u64, false);
            let malloc_fn = self.get_function("titrate_malloc");
            let capture_buf = self
                .builder
                .build_call(
                    malloc_fn,
                    &[capture_size.into()],
                    &format!("{}.capture_buf", tramp_name),
                )
                .map_err(|e| format!("capture malloc: {:?}", e))?;
            let capture_buf = match capture_buf.try_as_basic_value() {
                inkwell::values::ValueKind::Basic(v) => v.into_pointer_value(),
                _ => return Err("capture malloc returned non-value".to_string()),
            };

            // Store each captured variable's value into the buffer.
            let i32_ty = self.context.i32_type();
            let i8_ty = self.context.i8_type();
            for (i, var_name) in captured_vars.iter().enumerate() {
                let offset = (i as u64) * 8;
                let offset_val = i32_ty.const_int(offset, false);
                let gep = unsafe {
                    self.builder.build_gep(
                        i8_ty,
                        capture_buf,
                        &[offset_val],
                        &format!("{}.capture{}.gep", tramp_name, i),
                    )
                }
                .map_err(|e| format!("capture store gep: {:?}", e))?;
                let ptr_ptr_ty = self.context.ptr_type(AddressSpace::default());
                let slot_val = self
                    .builder
                    .build_bit_cast(
                        gep,
                        ptr_ptr_ty,
                        &format!("{}.capture{}.slot", tramp_name, i),
                    )
                    .map_err(|e| format!("capture store bit_cast: {:?}", e))?;
                let slot = slot_val.into_pointer_value();

                let local_info = self.locals.get(var_name).cloned();
                if let Some(local) = local_info {
                    // Copy the value into a heap cell so the closure can outlive
                    // the defining stack frame and mutations persist across calls.
                    let cell_size = i64_ty.const_int(24, false);
                    // Allocate in the entry block: a later closure in a
                    // different branch reuses this cell and must be dominated
                    // by its allocation.
                    let emit_at = self.builder.get_insert_block();
                    if let Some(entry) = emit_at
                        .and_then(|b| b.get_parent())
                        .and_then(|f| f.get_first_basic_block())
                    {
                        if let Some(first) = entry.get_first_instruction() {
                            self.builder.position_before(&first);
                        } else {
                            self.builder.position_at_end(entry);
                        }
                    }
                    let cell_call = self
                        .builder
                        .build_call(
                            malloc_fn,
                            &[cell_size.into()],
                            &format!("{}.capture{}.cell", tramp_name, i),
                        )
                        .map_err(|e| format!("capture cell malloc: {:?}", e))?;
                    if let Some(block) = emit_at {
                        self.builder.position_at_end(block);
                    }
                    let cell = match cell_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(v) => v.into_pointer_value(),
                        _ => return Err("capture cell malloc returned non-value".to_string()),
                    };
                    let val = self
                        .builder
                        .build_load(
                            local.ty,
                            local.ptr,
                            &format!("{}.capture{}.val", tramp_name, i),
                        )
                        .map_err(|e| format!("capture load: {:?}", e))?;
                    self.builder
                        .build_store(cell, val)
                        .map_err(|e| format!("capture cell store: {:?}", e))?;
                    self.builder
                        .build_store(slot, cell)
                        .map_err(|e| format!("capture slot store: {:?}", e))?;
                }
            }

            capture_buf
        };

        // Build the closure struct: { i8* fn_ptr, i8* capture_data }
        let undef = closure_struct_ty.const_zero();
        let result = self
            .builder
            .build_insert_value(undef, fn_ptr_i8, 0, &format!("{}.fn", tramp_name))
            .map_err(|e| format!("closure insert fn_ptr: {:?}", e))?;
        let result = self
            .builder
            .build_insert_value(result, capture_ptr, 1, &format!("{}.capture", tramp_name))
            .map_err(|e| format!("closure insert capture: {:?}", e))?;

        let result: BasicValueEnum<'ctx> = match result {
            inkwell::values::AggregateValueEnum::StructValue(sv) => sv.into(),
            _ => return Err("unexpected aggregate value".to_string()),
        };
        Ok(result)
    }
    /// Get the global exception pointer (`__titrate_exception`).
    pub(super) fn get_exception_global(&self) -> PointerValue<'ctx> {
        self.module
            .get_global("__titrate_exception")
            .unwrap_or_else(|| panic!("__titrate_exception not declared"))
            .as_pointer_value()
    }

    /// Flag set by `throw` when no catch block exists in the current function.
    /// A caller's `try` checks it after each statement so exceptions can cross
    /// a function boundary without branching to a block in another function.
    pub(super) fn get_exception_flag(&self) -> PointerValue<'ctx> {
        if let Some(g) = self.module.get_global("__titrate_thrown") {
            return g.as_pointer_value();
        }
        let i1_ty = self.context.bool_type();
        let g = self.module.add_global(i1_ty, None, "__titrate_thrown");
        g.set_linkage(Linkage::Internal);
        g.set_initializer(&i1_ty.const_zero());
        g.as_pointer_value()
    }

    /// Emit a terminator that returns a zero value of the current function's
    /// return type. Used after a non-local throw: the caller's `try` observes
    /// the flag and redirects control to its catch handler.
    fn build_throw_return(&mut self) -> Result<(), String> {
        // Use the emitted LLVM signature, not the Titrate return type: `main`
        // is declared `void` in Titrate but emits `i32 @main()`.
        let ret_ty = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .and_then(|f| f.get_type().get_return_type());
        match ret_ty {
            Some(ty) => {
                let zero: BasicValueEnum<'ctx> = match ty {
                    inkwell::types::BasicTypeEnum::IntType(it) => it.const_zero().into(),
                    inkwell::types::BasicTypeEnum::FloatType(ft) => ft.const_zero().into(),
                    inkwell::types::BasicTypeEnum::PointerType(pt) => pt.const_null().into(),
                    inkwell::types::BasicTypeEnum::StructType(st) => st.const_zero().into(),
                    inkwell::types::BasicTypeEnum::ArrayType(at) => at.const_zero().into(),
                    inkwell::types::BasicTypeEnum::VectorType(vt) => vt.const_zero().into(),
                    inkwell::types::BasicTypeEnum::ScalableVectorType(vt) => vt.const_zero().into(),
                };
                self.builder
                    .build_return(Some(&zero))
                    .map_err(|e| format!("throw return failed: {:?}", e))?;
            }
            None => {
                self.builder
                    .build_return(None)
                    .map_err(|e| format!("throw void return failed: {:?}", e))?;
            }
        }
        Ok(())
    }

    /// Compile `ok(value)` or `err(value)` into a `Result<T, E>` struct.
    ///
    /// The Result struct is `{ i32, i8* }` where:
    ///   - field 0: tag (0 = Ok, 1 = Err)
    ///   - field 1: heap-allocated payload pointer
    pub(super) fn compile_result_ctor(
        &mut self,
        name: &str,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() != 1 {
            return Err(format!("codegen: `{}` expects exactly 1 argument", name));
        }
        let tag = if name == "ok" || name == "Ok" { 0u64 } else { 1u64 };
        let i32_ty = self.context.i32_type();
        let _i8_ptr_ty = self.context.ptr_type(AddressSpace::default());
        let result_ty = llvm_types::result_type(self.context).into_struct_type();

        // Compile the argument.
        let arg_val = self.compile_expr(&args[0])?;
        let arg_ty = arg_val.get_type();
        let size = arg_ty
            .size_of()
            .ok_or_else(|| format!("cannot compute size of type {:?}", arg_ty))?;

        // Allocate heap memory for the payload.
        let malloc_fn = self.get_function("titrate_malloc");
        let payload_ptr = self
            .builder
            .build_call(malloc_fn, &[size.into()], &format!("{}.payload", name))
            .map_err(|e| format!("build_call titrate_malloc for {} failed: {:?}", name, e))?;
        let payload_ptr = match payload_ptr.try_as_basic_value() {
            inkwell::values::ValueKind::Basic(v) => v.into_pointer_value(),
            _ => {
                return Err(format!(
                    "titrate_malloc did not return a value for {}",
                    name
                ))
            }
        };

        // Store the value into the heap allocation.
        self.builder
            .build_store(payload_ptr, arg_val)
            .map_err(|e| format!("build_store {} payload failed: {:?}", name, e))?;

        // Build the Result struct: { i32 tag, i8* payload }.
        let tag_val = i32_ty.const_int(tag, false);
        let undef = result_ty.const_zero();
        let result = self
            .builder
            .build_insert_value(undef, tag_val, 0, &format!("{}.tag", name))
            .map_err(|e| format!("build_insert_value tag failed: {:?}", e))?;
        let result = self
            .builder
            .build_insert_value(result, payload_ptr, 1, &format!("{}.ptr", name))
            .map_err(|e| format!("build_insert_value payload failed: {:?}", e))?;

        let result: BasicValueEnum<'ctx> = match result {
            inkwell::values::AggregateValueEnum::StructValue(sv) => sv.into(),
            _ => return Err("unexpected aggregate value".to_string()),
        };
        Ok(result)
    }
    /// Compile an error propagation expression (`expr?`).
    ///
    /// The operand must evaluate to a `Result<T, E>` struct `{ i32, i8* }`.
    /// If the tag is 1 (Err), the payload is stored in `__titrate_exception`
    /// and control branches to the nearest catch block. If the tag is 0 (Ok),
    /// the payload is loaded from the heap and returned as the unwrapped value.
    pub(super) fn compile_error_propagation(&mut self, inner: &Expr) -> Result<BasicValueEnum<'ctx>, String> {
        let i32_ty = self.context.i32_type();
        let _i8_ptr_ty = self.context.ptr_type(AddressSpace::default());
        let _result_ty = llvm_types::result_type(self.context).into_struct_type();

        // Compile the expression to get the Result struct.
        let result_val = self.compile_expr(inner)?;
        let result_struct = match result_val {
            BasicValueEnum::StructValue(sv) => sv,
            _ => return Err("codegen: `?` operand must be a Result struct".to_string()),
        };

        // Extract the tag (field 0).
        let tag = self
            .builder
            .build_extract_value(result_struct, 0, "q.tag")
            .map_err(|e| format!("build_extract_value tag failed: {:?}", e))?;
        let tag = tag.into_int_value();

        // Compare tag with 1 (Err).
        let one = i32_ty.const_int(1, false);
        let is_err = self
            .builder
            .build_int_compare(inkwell::IntPredicate::EQ, tag, one, "q.is_err")
            .map_err(|e| format!("build_int_compare is_err failed: {:?}", e))?;

        let current_block = self
            .builder
            .get_insert_block()
            .ok_or("codegen: no insert block for `?`")?;
        let ok_block = self.context.insert_basic_block_after(current_block, "q.ok");
        let err_block = self.context.insert_basic_block_after(ok_block, "q.err");
        let end_block = self.context.insert_basic_block_after(err_block, "q.end");

        self.builder
            .build_conditional_branch(is_err, err_block, ok_block)
            .map_err(|e| format!("build_cond_br ? failed: {:?}", e))?;

        // Ok block: extract payload, load value from heap, branch to end.
        self.builder.position_at_end(ok_block);
        let payload_ptr = self
            .builder
            .build_extract_value(result_struct, 1, "q.payload")
            .map_err(|e| format!("build_extract_value payload failed: {:?}", e))?;
        let payload_ptr = payload_ptr.into_pointer_value();
        let ok_val = self
            .builder
            .build_load(i32_ty, payload_ptr, "q.ok.val")
            .map_err(|e| format!("build_load ok value failed: {:?}", e))?;
        let ok_block_end = self
            .builder
            .get_insert_block()
            .ok_or("codegen: no insert block after ok")?;
        self.builder
            .build_unconditional_branch(end_block)
            .map_err(|e| format!("build_br ?.end failed: {:?}", e))?;

        // Err block: extract payload, store in __titrate_exception, branch to catch.
        self.builder.position_at_end(err_block);
        let err_payload = self
            .builder
            .build_extract_value(result_struct, 1, "q.err.payload")
            .map_err(|e| format!("build_extract_value err payload failed: {:?}", e))?;
        let err_payload = err_payload.into_pointer_value();
        let exception_global = self.get_exception_global();
        self.builder
            .build_store(exception_global, err_payload)
            .map_err(|e| format!("build_store exception failed: {:?}", e))?;

        // Branch to the nearest catch block, or unreachable if none.
        if let Some(ctx) = self.catch_stack.last() {
            self.builder
                .build_unconditional_branch(ctx.catch_block)
                .map_err(|e| format!("build_br catch failed: {:?}", e))?;
        } else {
            // No catch handler - emit an unreachable.
            self.builder
                .build_unreachable()
                .map_err(|e| format!("build_unreachable failed: {:?}", e))?;
        }

        // End block: phi the ok value.
        self.builder.position_at_end(end_block);
        let phi = self
            .builder
            .build_phi(i32_ty, "q.result")
            .map_err(|e| format!("build_phi ? failed: {:?}", e))?;
        phi.add_incoming(&[(&ok_val, ok_block_end)]);

        Ok(phi.as_basic_value())
    }

    /// Compile a `throw expr;` statement.
    ///
    /// The expression is evaluated, its payload is stored in the global
    /// `__titrate_exception`, and `__titrate_thrown` is set. A catch block in
    /// the *same* function is branched to directly. Otherwise the current
    /// function returns a zero value of its return type and the flag lets the
    /// caller's `try` redirect to its catch handler. A cross-function catch
    /// block is never targeted: it belongs to a different LLVM function.
    pub(super) fn compile_throw(&mut self, expr: &Expr) -> Result<(), String> {
        let i8_ptr_ty = self.context.ptr_type(AddressSpace::default());

        let v = self.compile_expr(expr)?;

        // Composite payloads live in a heap slot: the throwing frame is about
        // to be left, so a stack alloca would dangle when the caller reads it.
        let ptr = if v.is_pointer_value() {
            v.into_pointer_value()
        } else if v.is_int_value() {
            let iv = v.into_int_value();
            self.builder
                .build_int_to_ptr(iv, i8_ptr_ty, "throw.ptr")
                .map_err(|e| format!("build_int_to_ptr throw failed: {:?}", e))?
        } else {
            let size = v
                .get_type()
                .size_of()
                .ok_or_else(|| "codegen: cannot size thrown value".to_string())?;
            let malloc_fn = self.get_function("titrate_malloc");
            let raw = self
                .builder
                .build_call(malloc_fn, &[size.into()], "throw.slot")
                .map_err(|e| format!("throw malloc failed: {:?}", e))?;
            let slot = match raw.try_as_basic_value() {
                inkwell::values::ValueKind::Basic(b) => b.into_pointer_value(),
                _ => return Err("throw malloc returned no value".to_string()),
            };
            self.builder
                .build_store(slot, v)
                .map_err(|e| format!("build_store throw value failed: {:?}", e))?;
            slot
        };

        let exception_global = self.get_exception_global();
        self.builder
            .build_store(exception_global, ptr)
            .map_err(|e| format!("build_store throw exception failed: {:?}", e))?;
        let flag = self.get_exception_flag();
        self.builder
            .build_store(flag, self.context.bool_type().const_all_ones())
            .map_err(|e| format!("build_store throw flag failed: {:?}", e))?;

        let local_catch = self.catch_stack.last().filter(|ctx| {
            self.builder
                .get_insert_block()
                .and_then(|b| b.get_parent())
                == ctx.catch_block.get_parent()
        });
        if let Some(ctx) = local_catch {
            self.builder
                .build_unconditional_branch(ctx.catch_block)
                .map_err(|e| format!("build_br throw catch failed: {:?}", e))?;
        } else {
            self.build_throw_return()?;
        }

        Ok(())
    }

    /// Compile a `try { ... } catch (e: T) { ... }` statement.
    ///
    /// Sets up a catch block that receives the error value from
    /// `__titrate_exception`. The try body is compiled normally; if an
    /// exception is thrown (via `throw` or `?`), control jumps to the
    /// catch block. Otherwise, the catch block is skipped.
    pub(super) fn compile_try_catch(
        &mut self,
        try_block: &[Stmt],
        catch_var: &str,
        catch_var_type: Option<&Type>,
        catch_block: &[Stmt],
    ) -> Result<(), String> {
        let i8_ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i1_ty = self.context.bool_type();

        let current_block = self
            .builder
            .get_insert_block()
            .ok_or("codegen: no insert block for try")?;
        // Unreachable statements (e.g. after `return`) are still lowered. Never
        // append the entry branch to a block that already has a terminator:
        // continue in a fresh block instead so every block ends exactly once.
        let predecessor = if current_block.get_terminator().is_some() {
            let parent = current_block
                .get_parent()
                .ok_or("codegen: terminated try block has no parent")?;
            let fresh = self.context.append_basic_block(parent, "try.unreachable");
            self.builder.position_at_end(fresh);
            fresh
        } else {
            current_block
        };
        let try_body_block = self
            .context
            .insert_basic_block_after(predecessor, "try.body");
        let catch_handler_block = self
            .context
            .insert_basic_block_after(try_body_block, "try.catch");
        let end_block = self
            .context
            .insert_basic_block_after(catch_handler_block, "try.end");

        // Allocate a slot for the error value.
        let error_alloca = self
            .builder
            .build_alloca(i8_ptr_ty, "try.error")
            .map_err(|e| format!("build_alloca try.error failed: {:?}", e))?;

        // Push the catch context.
        self.catch_stack.push(CatchContext {
            catch_block: catch_handler_block,
            error_alloca,
        });

        // Branch to the try body.
        self.builder
            .build_unconditional_branch(try_body_block)
            .map_err(|e| format!("build_br try.body failed: {:?}", e))?;

        // Compile the try body, checking the throw flag after every statement
        // so a throw from a callee redirects to this catch immediately.
        let flag = self.get_exception_flag();
        self.builder.position_at_end(try_body_block);
        self.builder
            .build_store(flag, i1_ty.const_zero())
            .map_err(|e| format!("clear throw flag failed: {:?}", e))?;
        let mut current = try_body_block;
        for (i, s) in try_block.iter().enumerate() {
            if current.get_terminator().is_some() {
                break;
            }
            self.builder.position_at_end(current);
            self.compile_stmt(s)?;
            let after = self
                .builder
                .get_insert_block()
                .unwrap_or(current);
            if after.get_terminator().is_some() {
                current = after;
                break;
            }
            let check = self
                .context
                .insert_basic_block_after(after, &format!("try.check.{}", i));
            self.builder
                .build_unconditional_branch(check)
                .map_err(|e| format!("build_br try.check failed: {:?}", e))?;
            self.builder.position_at_end(check);
            let thrown = self
                .builder
                .build_load(i1_ty, flag, "try.thrown")
                .map_err(|e| format!("load throw flag failed: {:?}", e))?
                .into_int_value();
            let next = if i + 1 == try_block.len() {
                end_block
            } else {
                self.context
                    .insert_basic_block_after(check, &format!("try.next.{}", i))
            };
            self.builder
                .build_conditional_branch(thrown, catch_handler_block, next)
                .map_err(|e| format!("build_cond_br try.check failed: {:?}", e))?;
            current = next;
        }
        // Pop the catch context.
        self.catch_stack.pop();

        // If the try body completed normally (no throw), branch to end. The
        // last check already targets `end_block`; do not self-branch it.
        if current != end_block && current.get_terminator().is_none() {
            self.builder.position_at_end(current);
            self.builder
                .build_unconditional_branch(end_block)
                .map_err(|e| format!("build_br try.end failed: {:?}", e))?;
        }

        // Catch handler: load the error from __titrate_exception, bind it
        // to the catch variable, and compile the catch body.
        self.builder.position_at_end(catch_handler_block);
        self.builder
            .build_store(flag, i1_ty.const_zero())
            .map_err(|e| format!("clear throw flag in catch failed: {:?}", e))?;
        let exception_global = self.get_exception_global();
        let error_val = self
            .builder
            .build_load(i8_ptr_ty, exception_global, "catch.err")
            .map_err(|e| format!("build_load exception failed: {:?}", e))?;

        // Determine the catch variable type. Default to i8* (pointer).
        let var_ty = match catch_var_type {
            Some(t) if llvm_types::is_string(t) => llvm_types::string_type(self.context),
            Some(t) => llvm_types::llvm_type(self.context, t)?,
            None => i8_ptr_ty.into(),
        };

        // Allocate the catch variable and store the error value.
        let var_alloca = self
            .builder
            .build_alloca(var_ty, catch_var)
            .map_err(|e| format!("build_alloca catch '{}' failed: {:?}", catch_var, e))?;

        // Store the error value into the catch variable. The exception global
        // holds a pointer: a string catch reads the composite from that slot.
        let error_ptr = error_val.into_pointer_value();
        let stored = if var_ty.is_struct_type() {
            self.builder
                .build_load(var_ty, error_ptr, "catch.err.value")
                .map_err(|e| format!("build_load catch value failed: {:?}", e))?
        } else if var_ty.is_int_type() {
            self.builder
                .build_ptr_to_int(error_ptr, var_ty.into_int_type(), "catch.err.int")
                .map_err(|e| format!("ptr_to_int catch failed: {:?}", e))?
                .into()
        } else {
            error_ptr.into()
        };
        self.builder
            .build_store(var_alloca, stored)
            .map_err(|e| format!("build_store catch error failed: {:?}", e))?;

        // Register the catch variable in locals.
        let prev = self.locals.insert(
            catch_var.to_string(),
            LocalVar {
                ptr: var_alloca,
                ty: var_ty,
                full_type: None,
                titrate_type: None,
                shared: false,
                closure_sig: None,
            },
        );

        // Compile the catch body.
        for s in catch_block {
            self.compile_stmt(s)?;
        }
        // Restore previous binding.
        if let Some(p) = prev {
            self.locals.insert(catch_var.to_string(), p);
        } else {
            self.locals.remove(catch_var);
        }

        if self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_none()
        {
            self.builder
                .build_unconditional_branch(end_block)
                .map_err(|e| format!("build_br try.end 2 failed: {:?}", e))?;
        }

        self.builder.position_at_end(end_block);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer;
    use crate::ast::Declaration;
    use crate::lexer;
    use crate::parser;

    fn compile_program_checked(source: &str) -> Result<String, String> {
        let tokens = lexer::tokenize(source).map_err(|e| format!("tokenize: {}", e))?;
        let ast = parser::parse(tokens).map_err(|e| format!("parse: {}", e))?;
        let typed = analyzer::analyze(&ast).map_err(|e| format!("analyze: {:?}", e))?;
        let context = inkwell::context::Context::create();
        let mut backend = LlvmBackend::new(&context, "throw_test");
        backend.declare_natives();
        for decl in &typed.declarations {
            match decl {
                Declaration::Function(f) if f.type_params.is_empty() => {
                    backend.function_decls.insert(f.name.clone(), f.clone());
                }
                Declaration::Class(c) => {
                    backend.class_decls.insert(c.name.clone(), c.clone());
                }
                _ => {}
            }
        }
        for decl in &typed.declarations {
            if let Declaration::Function(f) = decl {
                if f.name != "main" && f.type_params.is_empty() {
                    backend.compile_function(f)?;
                }
            }
        }
        let main = typed
            .declarations
            .iter()
            .find_map(|d| match d {
                Declaration::Function(f) if f.name == "main" => Some(f),
                _ => None,
            })
            .ok_or("no main function")?;
        backend.compile_main(main)?;
        backend.module.verify().map_err(|e| {
            format!(
                "LLVM module verification failed: {}\n{}",
                e,
                backend.module.print_to_string()
            )
        })?;
        Ok(backend.module.print_to_string().to_string())
    }

    fn compile_program(source: &str) -> String {
        compile_program_checked(source).expect("compile program")
    }

    /// Legacy `--compat` path: the analyzer rejects unannotated closures
    /// assigned to typed variables, but the CLI compiles those programs.
    fn compile_program_compat(source: &str) -> String {
        let tokens = lexer::tokenize(source).expect("tokenize");
        let ast = parser::parse(tokens).expect("parse");
        let context = inkwell::context::Context::create();
        let mut backend = LlvmBackend::new(&context, "compat_test");
        backend.declare_natives();
        for decl in &ast.declarations {
            match decl {
                Declaration::Function(f) if f.type_params.is_empty() => {
                    backend.function_decls.insert(f.name.clone(), f.clone());
                }
                Declaration::Class(c) => {
                    backend.class_decls.insert(c.name.clone(), c.clone());
                }
                _ => {}
            }
        }
        for decl in &ast.declarations {
            if let Declaration::Function(f) = decl {
                if f.name != "main" && f.type_params.is_empty() {
                    backend.compile_function(f).expect("compile function");
                }
            }
        }
        let main = ast
            .declarations
            .iter()
            .find_map(|d| match d {
                Declaration::Function(f) if f.name == "main" => Some(f),
                _ => None,
            })
            .expect("no main function");
        backend.compile_main(main).expect("compile main");
        backend.module.verify().expect("verify module");
        backend.module.print_to_string().to_string()
    }

    fn function_ir<'a>(ir: &'a str, name: &str) -> &'a str {
        let needle = format!("@{}(", name);
        for chunk in ir.split("define ").skip(1) {
            let header = chunk.lines().next().unwrap_or("");
            if header.contains(&needle) {
                let end = chunk.find("\n}").unwrap_or(chunk.len());
                return &chunk[..end];
            }
        }
        panic!("function {} not found in IR:\n{}", name, ir)
    }

    /// A `throw` in a method with a non-primitive return type must set the
    /// propagation flag and return safely; it must not trap with `unreachable`
    /// or branch to a catch block owned by the caller's function.
    #[test]
    fn throw_from_non_primitive_method_does_not_fault() {
        let ir = compile_program(
            r#"
            public class Box {
                public int value;
                public fn init(v: int) { this.value = v; }
                public fn get(): Box {
                    if (this.value < 0) {
                        throw "BoxError: negative";
                    }
                    return this;
                }
            }
            public fn main(): void {
                try {
                    let b = new Box(-1);
                    b.get();
                    io::println("FAIL: should have thrown");
                } catch (e: string) {
                    io::println("PASS: caught");
                }
            }
            "#,
        );
        let body = function_ir(&ir, "Box_get");
        assert!(body.contains("@__titrate_thrown"), "throw must set the flag: {}", body);
        assert!(!body.contains("unreachable"), "throw must not trap: {}", body);
        assert!(body.contains(" ret "), "throw must return safely: {}", body);
        let caller = function_ir(&ir, "main");
        assert!(caller.contains("@__titrate_thrown"), "try must check the flag: {}", caller);
    }

    fn assert_control_flow(ir: &str, case: &str) {
        for chunk in ir.split("define ").skip(1) {
            let header = chunk.lines().next().unwrap_or("");
            let fn_name = header
                .split('@')
                .nth(1)
                .unwrap_or("")
                .split('(')
                .next()
                .unwrap_or("");
            let mut labels: std::collections::HashSet<&str> = std::collections::HashSet::new();
            for line in chunk.lines() {
                if let Some((label, _)) = line.split_once(':') {
                    let label = label.trim();
                    if !label.is_empty()
                        && !label.contains(|c: char| c.is_whitespace())
                        && label
                            .chars()
                            .next()
                            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
                    {
                        labels.insert(label);
                    }
                }
            }
            for line in chunk.lines() {
                for part in line.split("label %").skip(1) {
                    let target = part
                        .split(|c: char| c == ',' || c.is_whitespace())
                        .next()
                        .unwrap_or("");
                    assert!(
                        labels.contains(target),
                        "{}: {} branches to undefined block {}:\n{}",
                        case,
                        fn_name,
                        target,
                        chunk
                    );
                }
            }
            let mut block = String::new();
            let mut terminated = false;
            for line in chunk.lines() {
                let t = line.trim();
                if t.is_empty() {
                    continue;
                }
                if t.starts_with('}') {
                    break;
                }
                if let Some((label, rest)) = t.split_once(':') {
                    let label = label.trim();
                    if !label.is_empty()
                        && !label.contains(|c: char| c.is_whitespace())
                        && label
                            .chars()
                            .next()
                            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
                        && (rest.trim().is_empty() || rest.trim().starts_with(';'))
                    {
                        assert!(
                            block.is_empty() || terminated,
                            "{}: block {} lacks a terminator:\n{}",
                            case,
                            block,
                            chunk
                        );
                        block = label.to_string();
                        terminated = false;
                        continue;
                    }
                }
                assert!(
                    !terminated,
                    "{}: instruction after terminator in block {}:\n{}",
                    case,
                    block,
                    chunk
                );
                terminated = t.starts_with("ret ")
                    || t.starts_with("br ")
                    || t.starts_with("unreachable")
                    || t.starts_with("switch ");
            }
            assert!(
                block.is_empty() || terminated,
                "{}: block {} lacks a terminator:\n{}",
                case,
                block,
                chunk
            );
        }
    }

    /// try/catch must always emit well-formed basic blocks: each block ends in
    /// exactly one terminator, no instruction follows a terminator, and every
    /// branch target is defined in its own function. This also covers lowering
    /// of unreachable try/catch statements (after `return`), which previously
    /// appended the entry branch after an existing terminator in `try.end`.
    #[test]
    fn try_catch_blocks_verify_and_preserve_semantics() {
        let cases: &[(&str, &str)] = &[
            (
                "simple",
                r#"
                public fn main(): void {
                    try { throw "boom"; } catch (e: string) { io::println("caught"); }
                    io::println("done");
                }
                "#,
            ),
            (
                "sequential",
                r#"
                public fn main(): void {
                    try { throw "a"; } catch (e: string) { io::println("a"); }
                    try { throw "b"; } catch (e: string) { io::println("b"); }
                    io::println("done");
                }
                "#,
            ),
            (
                "unreachable_after_return",
                r#"
                public fn main(): void {
                    try { throw "a"; } catch (e: string) { io::println("a"); }
                    return;
                    try { throw "b"; } catch (e: string) { io::println("b"); }
                }
                "#,
            ),
            (
                "nested",
                r#"
                public fn main(): void {
                    try {
                        try { throw "inner"; } catch (e: string) { io::println("inner"); }
                        io::println("outer");
                    } catch (e: string) { io::println("outer caught"); }
                    io::println("done");
                }
                "#,
            ),
            (
                "if_in_try",
                r#"
                public fn main(): void {
                    let c = true;
                    try {
                        if (c) { throw "boom"; } else { io::println("no"); }
                        io::println("after");
                    } catch (e: string) { io::println("caught"); }
                }
                "#,
            ),
            (
                "try_in_loop",
                r#"
                public fn main(): void {
                    var i = 0;
                    while (i < 2) {
                        i = i + 1;
                        try { throw "x"; } catch (e: string) { io::println("c"); }
                    }
                }
                "#,
            ),
            (
                "cross_function_throw",
                r#"
                public class Box {
                    public int value;
                    public fn init(v: int) { this.value = v; }
                    public fn get(): int {
                        if (this.value < 0) { throw "BoxError: negative"; }
                        return this.value;
                    }
                }
                public fn main(): void {
                    try {
                        let b = new Box(-1);
                        io::println(Integer.toString(b.get()));
                    } catch (e: string) {
                        io::println("caught");
                    }
                }
                "#,
            ),
        ];
        for (name, src) in cases {
            let ir = compile_program_checked(src)
                .unwrap_or_else(|e| panic!("{} failed to compile and verify:\n{}", name, e));
            assert_control_flow(&ir, name);
        }
        // Cross-function propagation must still use the throw flag: the callee
        // sets it and returns; the caller checks it and redirects to its catch.
        let ir = compile_program_checked(cases[6].1).expect("cross-function case");
        let body = function_ir(&ir, "Box_get");
        assert!(body.contains("@__titrate_thrown"), "callee must set flag: {}", body);
        assert!(body.contains(" ret "), "callee must return after throw: {}", body);
        let caller = function_ir(&ir, "main");
        assert!(caller.contains("@__titrate_thrown"), "caller must check flag: {}", caller);
    }

    /// Capture slots are byte offsets into a malloc'd buffer. GEPs must use i8
    /// elements: an opaque `ptr` element would scale offset 8 to 64 bytes and
    /// corrupt the heap for any closure capturing two or more variables.
    #[test]
    fn closure_capture_slots_use_byte_offsets() {
        let ir = compile_program(
            r#"
            public fn main(): void {
                let a = 1;
                let b = 2;
                let c: fn(int): int = fn(x: int): int { return a + b + x; };
                io::println(Integer.toString(c(0)));
            }
            "#,
        );
        assert!(
            ir.contains("_closure_0.capture1.gep = getelementptr i8, ptr"),
            "capture slot GEPs must be byte-offset: {}",
            ir
        );
        let trampoline = function_ir(&ir, "_closure_0");
        assert!(
            trampoline.contains("getelementptr i8, ptr %0, i32 8"),
            "capture reads must be byte-offset: {}",
            trampoline
        );
    }

    /// A closure literal without an explicit return type parses as `void`.
    /// Its trampoline must still return the compiled value, or callers read
    /// garbage from a void function.
    #[test]
    fn closure_without_return_annotation_returns_value() {
        let ir = compile_program_compat(
            r#"
            public fn main(): void {
                let twice: fn(int): int = fn(x: int) { return x * 2; };
                io::println(Integer.toString(twice(3)));
            }
            "#,
        );
        let trampoline = function_ir(&ir, "_closure_0");
        assert!(
            trampoline.starts_with("internal i32 @_closure_0("),
            "unannotated closure must infer an i32 return: {}",
            trampoline
        );
        assert!(trampoline.contains(" ret i32 "), "closure must return its value: {}", trampoline);
    }

    /// A capture cell is only initialised where the closure is constructed.
    /// Redirecting the enclosing local to that cell makes other branches read
    /// an uninitialised (often null) value.
    #[test]
    fn conditional_capture_keeps_enclosing_local() {
        let ir = compile_program(
            r#"
            public fn pick(flag: bool, f: fn(double): double): double {
                if (flag) {
                    let g: fn(double): double = fn(t: double): double { return f(t); };
                    return g(1.0);
                }
                return f(2.0);
            }
            public fn main(): void {
                let f: fn(double): double = fn(x: double): double { return x * x; };
                io::println(Double.toString(pick(false, f)));
            }
            "#,
        );
        let body = function_ir(&ir, "pick");
        assert!(
            body.contains("load { ptr, ptr }, ptr %f"),
            "the non-capture branch must read the original parameter slot: {}",
            body
        );
    }
}
