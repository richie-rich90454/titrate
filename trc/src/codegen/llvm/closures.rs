// Closures, tuples, results, and errors

use super::types as llvm_types;
use super::{CatchContext, LlvmBackend, LocalVar};
use crate::ast::{Expr, Stmt, Type};
use inkwell::module::Linkage;
use inkwell::types::BasicType;
use inkwell::values::{BasicValueEnum, PointerValue};
use inkwell::AddressSpace;

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
                    full_type: None,
                    titrate_type: None,
                    shared: false,
                closure_sig: None,
                },
            );
        }
        // Compile the body with scope-based cleanup tracking.
        self.ownership.enter_scope();
        for s in &with_stmt.body {
            self.compile_stmt(s)?;
        }
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
        let ret_ty = self.sig_ret_llvm_type(Some(return_type))?;
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
        self.current_ret_ty = Some(return_type.clone());
        let saved_locals = self.locals.clone();
        self.locals.clear();

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
            for (i, var_name) in captured_vars.iter().enumerate() {
                let offset = (i as u64) * 8;
                let offset_val = i32_ty.const_int(offset, false);
                let gep = unsafe {
                    self.builder.build_gep(
                        i8_ptr_ty,
                        capture_data_ptr,
                        &[offset_val],
                        &format!("capture.{}.gep", var_name),
                    )
                }
                .map_err(|e| format!("capture gep '{}': {:?}", var_name, e))?;
                let ptr_ptr_ty = self.context.ptr_type(AddressSpace::default());
                let slot_ptr_val = self
                    .builder
                    .build_bit_cast(gep, ptr_ptr_ty, &format!("capture.{}.ptr", var_name))
                    .map_err(|e| format!("capture bit_cast '{}': {:?}", var_name, e))?;
                let slot_ptr = slot_ptr_val.into_pointer_value();
                let captured_val = self
                    .builder
                    .build_load(i8_ptr_ty, slot_ptr, &format!("capture.{}.val", var_name))
                    .map_err(|e| format!("capture load '{}': {:?}", var_name, e))?;
                let alloca = self
                    .builder
                    .build_alloca(i8_ptr_ty, var_name)
                    .map_err(|e| format!("capture alloca '{}': {:?}", var_name, e))?;
                self.builder
                    .build_store(alloca, captured_val)
                    .map_err(|e| format!("capture store '{}': {:?}", var_name, e))?;
                self.locals.insert(
                    var_name.clone(),
                    LocalVar {
                        ptr: alloca,
                        ty: i8_ptr_ty.into(),
                        full_type: None,
                        titrate_type: None,
                        shared: false,
                closure_sig: None,
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
            for (i, var_name) in captured_vars.iter().enumerate() {
                let offset = (i as u64) * 8;
                let offset_val = i32_ty.const_int(offset, false);
                let gep = unsafe {
                    self.builder.build_gep(
                        i8_ptr_ty,
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

                if let Some(local) = self.locals.get(var_name) {
                    let val = self
                        .builder
                        .build_load(
                            local.ty,
                            local.ptr,
                            &format!("{}.capture{}.val", tramp_name, i),
                        )
                        .map_err(|e| format!("capture load: {:?}", e))?;
                    let val_ptr = self
                        .builder
                        .build_bit_cast(
                            val.into_pointer_value(),
                            i8_ptr_ty,
                            &format!("{}.capture{}.cast", tramp_name, i),
                        )
                        .map_err(|e| format!("capture cast: {:?}", e))?;
                    self.builder
                        .build_store(slot, val_ptr)
                        .map_err(|e| format!("capture store: {:?}", e))?;
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
    /// The expression is evaluated and its value is stored in the global
    /// `__titrate_exception`. Control then branches to the nearest catch
    /// block. If no catch block is active, an unreachable is emitted
    /// (Phase 1 limitation).
    pub(super) fn compile_throw(&mut self, expr: &Expr) -> Result<(), String> {
        let i8_ptr_ty = self.context.ptr_type(AddressSpace::default());

        // Compile the thrown value.
        let v = self.compile_expr(expr)?;

        // Store the value in __titrate_exception. If the value is not i8*,
        // we store it as-is (Phase 1: assume string or pointer).
        let ptr = if v.is_pointer_value() {
            v.into_pointer_value()
        } else if v.is_int_value() {
            let iv = v.into_int_value();
            self.builder
                .build_int_to_ptr(iv, i8_ptr_ty, "throw.ptr")
                .map_err(|e| format!("build_int_to_ptr throw failed: {:?}", e))?
        } else {
            return Err(format!(
                "codegen: cannot throw value of type {:?}",
                v.get_type()
            ));
        };

        let exception_global = self.get_exception_global();
        self.builder
            .build_store(exception_global, ptr)
            .map_err(|e| format!("build_store throw exception failed: {:?}", e))?;

        // Branch to catch block or unreachable.
        if let Some(ctx) = self.catch_stack.last() {
            self.builder
                .build_unconditional_branch(ctx.catch_block)
                .map_err(|e| format!("build_br throw catch failed: {:?}", e))?;
        } else {
            self.builder
                .build_unreachable()
                .map_err(|e| format!("build_unreachable throw failed: {:?}", e))?;
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

        let current_block = self
            .builder
            .get_insert_block()
            .ok_or("codegen: no insert block for try")?;
        let try_body_block = self
            .context
            .insert_basic_block_after(current_block, "try.body");
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

        // Compile the try body.
        self.builder.position_at_end(try_body_block);
        for s in try_block {
            self.compile_stmt(s)?;
        }
        // Pop the catch context.
        self.catch_stack.pop();

        // If the try body completed normally (no throw), branch to end.
        if self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_none()
        {
            self.builder
                .build_unconditional_branch(end_block)
                .map_err(|e| format!("build_br try.end failed: {:?}", e))?;
        }

        // Catch handler: load the error from __titrate_exception, bind it
        // to the catch variable, and compile the catch body.
        self.builder.position_at_end(catch_handler_block);
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

        // Store the error value into the catch variable.
        self.builder
            .build_store(var_alloca, error_val)
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
