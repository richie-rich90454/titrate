// Statement lowering

use super::native_bridge;
use super::types as llvm_types;
use super::vtable::{build_interface_fat_ptr_type};
use super::{ClosureSig, LlvmBackend, LocalVar, LoopContext};
use crate::ast::{Expr, Stmt, Type};
use inkwell::types::{BasicTypeEnum};
use inkwell::values::{BasicValueEnum};
use inkwell::AddressSpace;

impl<'ctx> LlvmBackend<'ctx> {
    /// Compile a single statement.
    pub(super) fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::Expr(expr) => {
                self.compile_expr(expr)?;
                Ok(())
            }
            Stmt::VarDecl(decl) | Stmt::ConstDecl(decl) => {
                self.compile_var_decl(decl)?;
                Ok(())
            }
            Stmt::Return(expr) => {
                self.compile_return(expr)?;
                Ok(())
            }
            Stmt::If(if_stmt) => {
                self.compile_if(if_stmt)?;
                Ok(())
            }
            Stmt::While(while_stmt) => {
                self.compile_while(while_stmt)?;
                Ok(())
            }
            Stmt::DoWhile(do_while_stmt) => {
                self.compile_do_while(do_while_stmt)?;
                Ok(())
            }
            Stmt::For(for_stmt) => {
                self.compile_for(for_stmt)?;
                Ok(())
            }
            Stmt::CFor(cfor_stmt) => {
                self.compile_c_for(cfor_stmt)?;
                Ok(())
            }
            Stmt::Break => {
                let ctx = self
                    .loop_stack
                    .last()
                    .ok_or("codegen: break outside of loop")?;
                self.builder
                    .build_unconditional_branch(ctx.break_block)
                    .map_err(|e| format!("build_br break failed: {:?}", e))?;
                Ok(())
            }
            Stmt::Continue => {
                let ctx = self
                    .loop_stack
                    .last()
                    .ok_or("codegen: continue outside of loop")?;
                self.builder
                    .build_unconditional_branch(ctx.continue_block)
                    .map_err(|e| format!("build_br continue failed: {:?}", e))?;
                Ok(())
            }
            Stmt::Block(block) => {
                self.ownership.enter_scope();
                for s in block {
                    self.compile_stmt(s)?;
                }
                self.emit_scope_cleanup()?;
                Ok(())
            }
            Stmt::Switch(switch_stmt) => {
                self.compile_switch(switch_stmt)?;
                Ok(())
            }
            Stmt::Throw(expr, _) => {
                self.compile_throw(expr)?;
                Ok(())
            }
            Stmt::TryCatch {
                try_block,
                catch_var,
                catch_var_type,
                catch_block,
                ..
            } => {
                self.compile_try_catch(try_block, catch_var, catch_var_type.as_ref(), catch_block)?;
                Ok(())
            }
            Stmt::With(with_stmt) => {
                self.compile_with(with_stmt)?;
                Ok(())
            }
            Stmt::TupleDestructure { names, expr, .. } => {
                self.compile_tuple_destructure(names, expr)?;
                Ok(())
            }
            _ => Err(format!("codegen: unsupported statement: {:?}", stmt)),
        }
    }

    /// Compile a return statement.
    pub(super) fn compile_return(&mut self, expr: &Option<Expr>) -> Result<(), String> {
        match expr {
            None => {
                // Check the actual LLVM function return type to generate the right ret.
                let current_fn = self.builder.get_insert_block().and_then(|b| b.get_parent());
                if let Some(fn_val) = current_fn {
                    if fn_val.get_type().get_return_type().is_none() {
                        // True void function.
                        self.builder
                            .build_return(None)
                            .map_err(|e| format!("build_return void failed: {:?}", e))?;
                    } else {
                        // Function returns a value but we have no expression.
                        // Return default zero.
                        let ret_ty = fn_val.get_type().get_return_type().unwrap();
                        let zero: BasicValueEnum<'ctx> = match ret_ty {
                            BasicTypeEnum::IntType(it) => it.const_int(0, false).into(),
                            BasicTypeEnum::FloatType(ft) => ft.const_float(0.0).into(),
                            BasicTypeEnum::PointerType(pt) => pt.const_null().into(),
                            BasicTypeEnum::StructType(_) => {
                                self.builder.build_return(None).map_err(|e| {
                                    format!("build_return struct default failed: {:?}", e)
                                })?;
                                return Ok(());
                            }
                            _ => {
                                self.builder
                                    .build_return(None)
                                    .map_err(|e| format!("build_return default failed: {:?}", e))?;
                                return Ok(());
                            }
                        };
                        self.builder
                            .build_return(Some(&zero))
                            .map_err(|e| format!("build_return zero failed: {:?}", e))?;
                    }
                } else {
                    self.builder
                        .build_return(None)
                        .map_err(|e| format!("build_return void failed: {:?}", e))?;
                }
            }
            Some(e) => {
                // Infer the static type first: wrapping an object returned
                // where an interface is declared needs the class name, but
                // the value must be compiled exactly once.
                let expr_ty = self.infer_expr_type(e);
                let mut v = self.compile_expr(e)?;
                // An object returned where an interface is declared becomes a
                // fat pointer; values already of that interface pass through.
                if let Some(rt) = self.current_ret_ty.clone() {
                    if self.is_interface_ty(&rt) && v.is_pointer_value() {
                        if expr_ty.name() == rt.name() {
                            return Err(format!(
                                "codegen: interface '{}' value degraded to a raw pointer",
                                rt.name()
                            ));
                        }
                        v = self.wrap_object_ptr_in_interface(
                            v.into_pointer_value(),
                            rt.name(),
                            expr_ty.name(),
                        )?;
                    }
                }
                // If the current function returns void, discard the value and return void.
                let current_fn = self.builder.get_insert_block().and_then(|b| b.get_parent());
                if let Some(fn_val) = current_fn {
                    if fn_val.get_type().get_return_type().is_none() {
                        self.builder
                            .build_return(None)
                            .map_err(|e| format!("build_return void failed: {:?}", e))?;
                        return Ok(());
                    }
                    // Cast the return value to match the function's declared return type.
                    let ret_ty = fn_val.get_type().get_return_type().unwrap();
                    let v = self
                        .cast_value_to_type(v, ret_ty)
                        .map_err(|e| format!("return type cast failed: {:?}", e))?;
                    self.builder
                        .build_return(Some(&v))
                        .map_err(|e| format!("build_return failed: {:?}", e))?;
                } else {
                    self.builder
                        .build_return(Some(&v))
                        .map_err(|e| format!("build_return failed: {:?}", e))?;
                }
            }
        }
        Ok(())
    }

    /// Compile a `let`/`var`/`const` declaration.
    pub(super) fn compile_var_decl(&mut self, decl: &crate::ast::VarDecl) -> Result<(), String> {
        let init = decl
            .init
            .as_ref()
            .ok_or_else(|| format!("codegen: variable '{}' has no initializer", decl.name))?;

        // Determine the declared type (if any).
        let declared_ty = decl.typ.as_ref();

        // String variable.
        let is_string = declared_ty.map(llvm_types::is_string).unwrap_or(false);
        if is_string {
            let sv = self.compile_string_expr(init)?;
            let string_ty = llvm_types::string_type(self.context).into_struct_type();
            let alloca = self
                .builder
                .build_alloca(string_ty, &decl.name)
                .map_err(|e| format!("build_alloca '{}' failed: {:?}", decl.name, e))?;
            let len_ptr = self
                .builder
                .build_struct_gep(string_ty, alloca, 0, &format!("{}.len", decl.name))
                .map_err(|e| format!("build_struct_gep 0 failed: {:?}", e))?;
            let ptr_ptr = self
                .builder
                .build_struct_gep(string_ty, alloca, 1, &format!("{}.ptr", decl.name))
                .map_err(|e| format!("build_struct_gep 1 failed: {:?}", e))?;
            self.builder
                .build_store(len_ptr, sv.len)
                .map_err(|e| format!("build_store len failed: {:?}", e))?;
            self.builder
                .build_store(ptr_ptr, sv.ptr)
                .map_err(|e| format!("build_store ptr failed: {:?}", e))?;
            self.locals.insert(
                decl.name.clone(),
                LocalVar {
                    ptr: alloca,
                    ty: string_ty.into(),
                    full_type: declared_ty.cloned(),
                    titrate_type: Some("string".to_string()),
                    shared: false,
                closure_sig: None,
                },
            );
            return Ok(());
        }

        // Primitive (or inferred) variable.
        //
        // Reference binding: when the value is a shared container and the
        // initializer names an existing slot (`let y = nums`, `let y = obj.items`),
        // alias that slot so mutations stay visible (VM reference semantics).
        // Whole-value rebinding later detaches to a private copy.
        // The analyzer pre-fills unresolvable `let` types as `unknown`;
        // treat those as uninferred and fall back to codegen inference.
        let effective_declared: Option<&Type> = match declared_ty {
            Some(t) if t.name() == "unknown" => None,
            other => other,
        };
        let alias_ty: Option<Type> = match effective_declared {
            Some(t) if Self::is_shared_container_ty(t) => Some(t.clone()),
            Some(_) => None,
            None => {
                let inferred = self.infer_expr_type(init);
                if Self::is_shared_container_ty(&inferred) {
                    Some(inferred)
                } else {
                    None
                }
            }
        };
        if let Some(container_ty) = alias_ty {
            let slot = self.container_slot_ptr(init)?;
            let struct_ty: BasicTypeEnum<'ctx> =
                native_bridge::titrate_array_type(self.context).into();
            // Prefer a real declared type, but never the analyzer's
            // `unknown` placeholder: element-aware accessors (get/for-in)
            // need the true container type.
            let full_ty = match effective_declared {
                Some(t) => Some(t.clone()),
                None => Some(container_ty.clone()),
            };
            self.locals.insert(
                decl.name.clone(),
                LocalVar {
                    ptr: slot,
                    ty: struct_ty,
                    full_type: full_ty,
                    titrate_type: Some(container_ty.name().to_string()),
                    shared: true,
                    closure_sig: None,
                },
            );
            return Ok(());
        }
        // A closure literal always materializes as the {fn_ptr, capture}
        // struct, regardless of the declared `fn` type (which lowers to an
        // opaque pointer and cannot hold the struct).
        let is_closure_init = matches!(init, Expr::Closure { .. });
        let ty = if is_closure_init {
            let i8_ptr = self.context.ptr_type(AddressSpace::default());
            self.context
                .struct_type(&[i8_ptr.into(), i8_ptr.into()], false)
                .into()
        } else {
            match declared_ty {
                Some(t) => {
                    // Interface bindings hold the fat pointer so method
                    // calls dispatch (mirrors interface params).
                    if self.is_interface_ty(t) {
                        build_interface_fat_ptr_type(self.context).into()
                    } else {
                        // For unknown class types (e.g., JsonValue, Regex), fall back to opaque pointer.
                        llvm_types::llvm_type(self.context, t).unwrap_or_else(|_| {
                            self.context.ptr_type(AddressSpace::default()).into()
                        })
                    }
                }
                None => {
                    // Infer from the initializer.
                    let v = self.compile_expr(init)?;
                    v.get_type()
                }
            }
        };

        let alloca = self
            .builder
            .build_alloca(ty, &decl.name)
            .map_err(|e| format!("build_alloca '{}' failed: {:?}", decl.name, e))?;

        // Compile the initializer with a type hint for literals.
        let init_val = match init {
            Expr::Literal(lit, _) => self.compile_literal(lit, declared_ty)?,
            _ => self.compile_expr(init)?,
        };

        // Interface bindings wrap a raw object pointer with its vtable.
        // A value that is already a fat pointer passes through untouched.
        let init_val = match declared_ty {
            Some(t) if self.is_interface_ty(t) && init_val.is_pointer_value() => {
                let obj_ptr = init_val.into_pointer_value();
                let class_name = self.infer_expr_type(init).name().to_string();
                self.wrap_object_ptr_in_interface(obj_ptr, t.name(), &class_name)?
            }
            _ => self.cast_value_to_type(init_val, ty)?,
        };

        self.builder
            .build_store(alloca, init_val)
            .map_err(|e| format!("build_store '{}' failed: {:?}", decl.name, e))?;

        // Infer titrate_type: use declared type if present, otherwise try to infer from the init expression.
        let inferred_type = declared_ty.map(|t| t.name().to_string()).or_else(|| {
            // When no explicit type, infer from `new ClassName(...)` expressions.
            if let Expr::New(type_name, _, _) = init {
                Some(type_name.name().to_string())
            } else {
                None
            }
        });
        // A closure literal initializer records its declared signature so
        // direct invocations through this local can be typed exactly.
        let closure_sig = match init {
            Expr::Closure { params, return_type, body, .. } => Some(ClosureSig {
                params: params.iter().map(|(_, ty)| ty.clone()).collect(),
                ret: return_type.clone(),
                returns_param: self.return_alias_param(
                    body,
                    params,
                    &mut std::collections::HashSet::new(),
                ),
            }),
            _ => None,
        };
        // Keep generic arguments for `new Class<T>(...)` initializers so
        // later uses resolve the specialization (e.g. Box<int>.get()).
        let full_type = declared_ty.cloned().or_else(|| match init {
            Expr::New(type_name, _, _) => Some(type_name.clone()),
            _ => None,
        });
        self.locals.insert(
            decl.name.clone(),
            LocalVar {
                ptr: alloca,
                ty,
                full_type,
                titrate_type: inferred_type,
                shared: false,
                closure_sig,
            },
        );
        Ok(())
    }

    /// Compile an if/else statement.
    pub(super) fn compile_if(&mut self, if_stmt: &crate::ast::IfStmt) -> Result<(), String> {
        let cond_val = self.compile_expr(&if_stmt.condition)?;
        let cond = if cond_val.is_int_value() {
            let int_val = cond_val.into_int_value();
            if int_val.get_type() == self.context.bool_type() {
                int_val
            } else {
                // Coerce non-bool integer to bool (non-zero = true)
                let zero = int_val.get_type().const_int(0, false);
                self.builder
                    .build_int_compare(inkwell::IntPredicate::NE, int_val, zero, "cond.bool")
                    .map_err(|e| format!("build_int_compare bool coercion failed: {:?}", e))?
            }
        } else if cond_val.is_struct_value() {
            // Coerce struct to bool: check if the data pointer is non-null.
            if let BasicValueEnum::StructValue(sv) = cond_val {
                let st = sv.get_type();
                if st.count_fields() == 2 {
                    if let Some(BasicTypeEnum::PointerType(_)) = st.get_field_type_at_index(1) {
                        let ptr_val = self
                            .builder
                            .build_extract_value(sv, 1, "cond.ptr")
                            .map_err(|e| format!("extract failed: {:?}", e))?
                            .into_pointer_value();
                        let null_ptr = self.context.ptr_type(AddressSpace::default()).const_null();
                        self.builder
                            .build_int_compare(
                                inkwell::IntPredicate::NE,
                                ptr_val,
                                null_ptr,
                                "cond.bool",
                            )
                            .map_err(|e| format!("icmp failed: {:?}", e))?
                    } else {
                        return Err("codegen: unsupported struct condition type".into());
                    }
                } else {
                    return Err("codegen: unsupported struct condition type".into());
                }
            } else {
                unreachable!()
            }
        } else {
            return Err(format!(
                "codegen: if condition must be a bool, got {:?}",
                cond_val.get_type()
            ));
        };
        let current_block = self
            .builder
            .get_insert_block()
            .ok_or("codegen: no insert block for if")?;
        let then_block = self
            .context
            .insert_basic_block_after(current_block, "if.then");
        let end_block = self.context.insert_basic_block_after(then_block, "if.end");

        let else_block = if if_stmt.else_branch.is_some() {
            Some(self.context.insert_basic_block_after(then_block, "if.else"))
        } else {
            None
        };

        if let Some(eb) = else_block {
            self.builder
                .build_conditional_branch(cond, then_block, eb)
                .map_err(|e| format!("build_cond_br if failed: {:?}", e))?;
        } else {
            self.builder
                .build_conditional_branch(cond, then_block, end_block)
                .map_err(|e| format!("build_cond_br if failed: {:?}", e))?;
        }

        // Then block.
        self.builder.position_at_end(then_block);
        for s in &if_stmt.then_branch {
            self.compile_stmt(s)?;
        }
        // Only add the branch to end if the current block doesn't already
        // have a terminator (e.g. from a return statement).
        if self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_none()
        {
            self.builder
                .build_unconditional_branch(end_block)
                .map_err(|e| format!("build_br if.end failed: {:?}", e))?;
        }

        // Else block.
        if let (Some(eb), Some(else_branch)) = (else_block, &if_stmt.else_branch) {
            self.builder.position_at_end(eb);
            for s in else_branch {
                self.compile_stmt(s)?;
            }
            if self
                .builder
                .get_insert_block()
                .and_then(|b| b.get_terminator())
                .is_none()
            {
                self.builder
                    .build_unconditional_branch(end_block)
                    .map_err(|e| format!("build_br if.end 2 failed: {:?}", e))?;
            }
        }

        self.builder.position_at_end(end_block);
        Ok(())
    }

    /// Compile a while loop.
    pub(super) fn compile_while(&mut self, while_stmt: &crate::ast::WhileStmt) -> Result<(), String> {
        let current_block = self
            .builder
            .get_insert_block()
            .ok_or("codegen: no insert block for while")?;
        let cond_block = self
            .context
            .insert_basic_block_after(current_block, "while.cond");
        let body_block = self
            .context
            .insert_basic_block_after(cond_block, "while.body");
        let end_block = self
            .context
            .insert_basic_block_after(body_block, "while.end");

        self.builder
            .build_unconditional_branch(cond_block)
            .map_err(|e| format!("build_br while.cond failed: {:?}", e))?;

        // Condition block.
        self.builder.position_at_end(cond_block);
        let cond_val = self.compile_expr(&while_stmt.condition)?;
        let cond = if cond_val.is_int_value() {
            let int_val = cond_val.into_int_value();
            if int_val.get_type() == self.context.bool_type() {
                int_val
            } else {
                let zero = int_val.get_type().const_int(0, false);
                self.builder
                    .build_int_compare(inkwell::IntPredicate::NE, int_val, zero, "cond.bool")
                    .map_err(|e| format!("build_int_compare bool coercion failed: {:?}", e))?
            }
        } else if cond_val.is_struct_value() {
            if let BasicValueEnum::StructValue(sv) = cond_val {
                let st = sv.get_type();
                if st.count_fields() == 2 {
                    if let Some(BasicTypeEnum::PointerType(_)) = st.get_field_type_at_index(1) {
                        let ptr_val = self
                            .builder
                            .build_extract_value(sv, 1, "cond.ptr")
                            .map_err(|e| format!("extract failed: {:?}", e))?
                            .into_pointer_value();
                        let null_ptr = self.context.ptr_type(AddressSpace::default()).const_null();
                        self.builder
                            .build_int_compare(
                                inkwell::IntPredicate::NE,
                                ptr_val,
                                null_ptr,
                                "cond.bool",
                            )
                            .map_err(|e| format!("icmp failed: {:?}", e))?
                    } else {
                        return Err("codegen: unsupported struct condition type".into());
                    }
                } else {
                    return Err("codegen: unsupported struct condition type".into());
                }
            } else {
                unreachable!()
            }
        } else {
            return Err(format!(
                "codegen: while condition must be a bool, got {:?}",
                cond_val.get_type()
            ));
        };
        self.builder
            .build_conditional_branch(cond, body_block, end_block)
            .map_err(|e| format!("build_cond_br while failed: {:?}", e))?;

        // Body block.
        self.builder.position_at_end(body_block);
        self.loop_stack.push(LoopContext {
            continue_block: cond_block,
            break_block: end_block,
        });
        for s in &while_stmt.body {
            self.compile_stmt(s)?;
        }
        self.loop_stack.pop();
        if self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_none()
        {
            self.builder
                .build_unconditional_branch(cond_block)
                .map_err(|e| format!("build_br while.cond 2 failed: {:?}", e))?;
        }

        self.builder.position_at_end(end_block);
        Ok(())
    }

    /// Compile a do-while loop.
    pub(super) fn compile_do_while(&mut self, do_while_stmt: &crate::ast::DoWhileStmt) -> Result<(), String> {
        let current_block = self
            .builder
            .get_insert_block()
            .ok_or("codegen: no insert block for do-while")?;
        let body_block = self
            .context
            .insert_basic_block_after(current_block, "do.body");
        let cond_block = self.context.insert_basic_block_after(body_block, "do.cond");
        let end_block = self.context.insert_basic_block_after(cond_block, "do.end");

        self.builder
            .build_unconditional_branch(body_block)
            .map_err(|e| format!("build_br do.body failed: {:?}", e))?;

        // Body block.
        self.builder.position_at_end(body_block);
        self.loop_stack.push(LoopContext {
            continue_block: cond_block,
            break_block: end_block,
        });
        for s in &do_while_stmt.body {
            self.compile_stmt(s)?;
        }
        self.loop_stack.pop();
        if self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_none()
        {
            self.builder
                .build_unconditional_branch(cond_block)
                .map_err(|e| format!("build_br do.cond failed: {:?}", e))?;
        }

        // Condition block.
        self.builder.position_at_end(cond_block);
        let cond_val = self.compile_expr(&do_while_stmt.condition)?;
        let cond = if cond_val.is_int_value() {
            let int_val = cond_val.into_int_value();
            if int_val.get_type() == self.context.bool_type() {
                int_val
            } else {
                let zero = int_val.get_type().const_int(0, false);
                self.builder
                    .build_int_compare(inkwell::IntPredicate::NE, int_val, zero, "cond.bool")
                    .map_err(|e| format!("build_int_compare bool coercion failed: {:?}", e))?
            }
        } else if cond_val.is_struct_value() {
            if let BasicValueEnum::StructValue(sv) = cond_val {
                let st = sv.get_type();
                if st.count_fields() == 2 {
                    if let Some(BasicTypeEnum::PointerType(_)) = st.get_field_type_at_index(1) {
                        let ptr_val = self
                            .builder
                            .build_extract_value(sv, 1, "cond.ptr")
                            .map_err(|e| format!("extract failed: {:?}", e))?
                            .into_pointer_value();
                        let null_ptr = self.context.ptr_type(AddressSpace::default()).const_null();
                        self.builder
                            .build_int_compare(
                                inkwell::IntPredicate::NE,
                                ptr_val,
                                null_ptr,
                                "cond.bool",
                            )
                            .map_err(|e| format!("icmp failed: {:?}", e))?
                    } else {
                        return Err("codegen: unsupported struct condition type".into());
                    }
                } else {
                    return Err("codegen: unsupported struct condition type".into());
                }
            } else {
                unreachable!()
            }
        } else {
            return Err(format!(
                "codegen: do-while condition must be a bool, got {:?}",
                cond_val.get_type()
            ));
        };
        self.builder
            .build_conditional_branch(cond, body_block, end_block)
            .map_err(|e| format!("build_cond_br do-while failed: {:?}", e))?;

        self.builder.position_at_end(end_block);
        Ok(())
    }

    /// Compile a for-in loop. Supports Range iteration
    /// (`for (i in start..end)`, `for (i in start..=end)`) and collection
    /// iteration over `ArrayList`/`array` values (`for (item in list)`).
    pub(super) fn compile_for(&mut self, for_stmt: &crate::ast::ForStmt) -> Result<(), String> {
        // Range iteration.
        let (start_expr, end_expr, inclusive) = match &for_stmt.iterable {
            Expr::Range(s, e, _) => (s.as_ref(), e.as_ref(), false),
            Expr::RangeInclusive(s, e, _) => (s.as_ref(), e.as_ref(), true),
            other => {
                return self.compile_for_collection(for_stmt, other);
            }
        };

        let i64_ty = self.context.i64_type();

        // Compile start and end values.
        let start_val = self.compile_expr(start_expr)?;
        let end_val = self.compile_expr(end_expr)?;

        // Extend to i64 if needed.
        let start_i64 = if start_val.is_int_value() && start_val.get_type() != i64_ty.into() {
            self.builder
                .build_int_s_extend(start_val.into_int_value(), i64_ty, "for.start.ext")
                .map_err(|e| format!("build_int_s_extend start failed: {:?}", e))?
        } else {
            start_val.into_int_value()
        };
        let end_i64 = if end_val.is_int_value() && end_val.get_type() != i64_ty.into() {
            self.builder
                .build_int_s_extend(end_val.into_int_value(), i64_ty, "for.end.ext")
                .map_err(|e| format!("build_int_s_extend end failed: {:?}", e))?
        } else {
            end_val.into_int_value()
        };

        // Allocate the loop counter.
        let counter_alloca = self
            .builder
            .build_alloca(i64_ty, "for.i")
            .map_err(|e| format!("build_alloca for.i failed: {:?}", e))?;
        self.builder
            .build_store(counter_alloca, start_i64)
            .map_err(|e| format!("build_store for.i failed: {:?}", e))?;

        // Allocate the loop variable (visible to the body).
        let loop_var_ty = start_val.get_type();
        let loop_var_alloca = self
            .builder
            .build_alloca(loop_var_ty, &for_stmt.var)
            .map_err(|e| format!("build_alloca '{}' failed: {:?}", for_stmt.var, e))?;

        let current_block = self
            .builder
            .get_insert_block()
            .ok_or("codegen: no insert block for for")?;
        let cond_block = self
            .context
            .insert_basic_block_after(current_block, "for.cond");
        let body_block = self
            .context
            .insert_basic_block_after(cond_block, "for.body");
        let inc_block = self.context.insert_basic_block_after(body_block, "for.inc");
        let end_block = self.context.insert_basic_block_after(inc_block, "for.end");

        self.builder
            .build_unconditional_branch(cond_block)
            .map_err(|e| format!("build_br for.cond failed: {:?}", e))?;

        // Condition block: while counter < end (or <= for inclusive).
        self.builder.position_at_end(cond_block);
        let counter_val = self
            .builder
            .build_load(i64_ty, counter_alloca, "for.i.val")
            .map_err(|e| format!("build_load for.i failed: {:?}", e))?
            .into_int_value();
        let pred = if inclusive {
            inkwell::IntPredicate::SLE
        } else {
            inkwell::IntPredicate::SLT
        };
        let cmp = self
            .builder
            .build_int_compare(pred, counter_val, end_i64, "for.cmp")
            .map_err(|e| format!("build_int_compare for failed: {:?}", e))?;
        self.builder
            .build_conditional_branch(cmp, body_block, end_block)
            .map_err(|e| format!("build_cond_br for failed: {:?}", e))?;

        // Body block: store the counter (truncated to loop var type) in the
        // loop variable, then run the body.
        self.builder.position_at_end(body_block);
        let counter_for_body = if loop_var_ty != i64_ty.into() {
            self.builder
                .build_int_truncate(counter_val, loop_var_ty.into_int_type(), "for.i.trunc")
                .map_err(|e| format!("build_int_truncate for failed: {:?}", e))?
        } else {
            counter_val
        };
        self.builder
            .build_store(loop_var_alloca, counter_for_body)
            .map_err(|e| format!("build_store loop var failed: {:?}", e))?;
        // Register the loop variable in locals.
        let prev = self.locals.insert(
            for_stmt.var.clone(),
            LocalVar {
                ptr: loop_var_alloca,
                ty: loop_var_ty,
                full_type: None,
                titrate_type: None,
                shared: false,
                closure_sig: None,
            },
        );
        self.loop_stack.push(LoopContext {
            continue_block: inc_block,
            break_block: end_block,
        });
        for s in &for_stmt.body {
            self.compile_stmt(s)?;
        }
        self.loop_stack.pop();
        // Restore the previous binding (if any).
        if let Some(p) = prev {
            self.locals.insert(for_stmt.var.clone(), p);
        } else {
            self.locals.remove(&for_stmt.var);
        }
        if self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_none()
        {
            self.builder
                .build_unconditional_branch(inc_block)
                .map_err(|e| format!("build_br for.inc failed: {:?}", e))?;
        }

        // Increment block: counter += 1.
        self.builder.position_at_end(inc_block);
        let counter_val = self
            .builder
            .build_load(i64_ty, counter_alloca, "for.i.inc")
            .map_err(|e| format!("build_load for.i.inc failed: {:?}", e))?
            .into_int_value();
        let one = i64_ty.const_int(1, false);
        let next = self
            .builder
            .build_int_add(counter_val, one, "for.i.next")
            .map_err(|e| format!("build_int_add for.inc failed: {:?}", e))?;
        self.builder
            .build_store(counter_alloca, next)
            .map_err(|e| format!("build_store for.i.next failed: {:?}", e))?;
        self.builder
            .build_unconditional_branch(cond_block)
            .map_err(|e| format!("build_br for.cond 2 failed: {:?}", e))?;

        // In release mode, attach vectorization hints to the loop latch
        // (the back-edge branch in the increment block). This tells LLVM's
        // loop vectorizer that this loop is a candidate for SIMD.
        if self.release_mode {
            // Metadata attachment is best-effort: if it fails we still
            // produce correct (just unannotated) code.
            let _ = self.add_vectorize_metadata(inc_block);
        }

        self.builder.position_at_end(end_block);
        Ok(())
    }
}
