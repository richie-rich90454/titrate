// Assignment, casts, and ternaries

use super::types as llvm_types;
use super::vtable::emit_field_store;
use super::{ClosureSig, LlvmBackend};
use crate::ast::Expr;
use inkwell::AddressSpace;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::BasicValueEnum;

impl<'ctx> LlvmBackend<'ctx> {
    pub(super) fn compile_assign(
        &mut self,
        target: &Expr,
        value: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if let Expr::Identifier(name, _) = target {
            // Clone the LocalVar to release the immutable borrow
            // of self.locals before we call &mut self methods below.
            let mut var = self
                .locals
                .get(name)
                .cloned()
                .ok_or_else(|| format!("codegen: assignment to unknown variable '{}'", name))?;
            // Rebinding a shared container detaches to a private copy: the
            // previous owner keeps the old list (VM reference semantics).
            // In-place mutations still flow through store_back_container.
            let var_is_container = var
                .full_type
                .as_ref()
                .map(Self::is_shared_container_ty)
                .unwrap_or(false)
                || matches!(
                    var.titrate_type.as_deref(),
                    Some("ArrayList" | "HashMap" | "array")
                );
            if var.shared && var_is_container {
                let fresh = self
                    .builder
                    .build_alloca(var.ty, name)
                    .map_err(|e| format!("build_alloca detach '{}' failed: {:?}", name, e))?;
                var.ptr = fresh;
                var.shared = false;
                self.locals.insert(name.clone(), var.clone());
            }
            // String assignment.
            if llvm_types::is_string(&self.llvm_basic_type_to_titrate_type(var.ty)) {
                let sv = self.compile_string_expr(value)?;
                let string_ty = llvm_types::string_type(self.context).into_struct_type();
                let alloca = self
                    .builder
                    .build_alloca(string_ty, "assign.tmp")
                    .map_err(|e| format!("build_alloca assign.tmp failed: {:?}", e))?;
                let len_ptr = self
                    .builder
                    .build_struct_gep(string_ty, alloca, 0, "assign.len.ptr")
                    .map_err(|e| format!("build_struct_gep 0 failed: {:?}", e))?;
                let ptr_ptr = self
                    .builder
                    .build_struct_gep(string_ty, alloca, 1, "assign.ptr.ptr")
                    .map_err(|e| format!("build_struct_gep 1 failed: {:?}", e))?;
                self.builder
                    .build_store(len_ptr, sv.len)
                    .map_err(|e| format!("build_store len failed: {:?}", e))?;
                self.builder
                    .build_store(ptr_ptr, sv.ptr)
                    .map_err(|e| format!("build_store ptr failed: {:?}", e))?;
                let struct_val = self
                    .builder
                    .build_load(string_ty, alloca, "assign.val")
                    .map_err(|e| format!("build_load assign.val failed: {:?}", e))?;
                self.builder
                    .build_store(var.ptr, struct_val)
                    .map_err(|e| format!("build_store struct failed: {:?}", e))?;
                return Ok(struct_val);
            }
            // Primitive assignment.
            let v = self.compile_expr(value)?;
            // A closure literal needs a struct slot; an old pointer-typed
            // slot (from the opaque `fn` lowering) cannot hold it.
            if matches!(value, Expr::Closure { .. }) && !var.ty.is_struct_type() {
                let i8_ptr = self.context.ptr_type(AddressSpace::default());
                let struct_ty: BasicTypeEnum<'ctx> = self
                    .context
                    .struct_type(&[i8_ptr.into(), i8_ptr.into()], false)
                    .into();
                let slot = self
                    .builder
                    .build_alloca(struct_ty, name)
                    .map_err(|e| format!("build_alloca closure '{}' failed: {:?}", name, e))?;
                self.builder
                    .build_store(slot, v)
                    .map_err(|e| format!("build_store closure '{}' failed: {:?}", name, e))?;
                var.ptr = slot;
                var.ty = struct_ty;
            } else {
                // Cast to the variable's type if needed.
                let v = self.cast_value_to_type(v, var.ty)?;
                self.builder
                    .build_store(var.ptr, v)
                    .map_err(|e| format!("build_store assign failed: {:?}", e))?;
            }
            // Rebinding replaces any closure signature (or installs a new one
            // when the right-hand side is a closure literal).
            var.closure_sig = match value {
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
            self.locals.insert(name.clone(), var);
            return Ok(v);
        }
        // Field assignment: obj.field = value
        if let Expr::MemberAccess(obj, field, _) = target {
            let obj_val = self.compile_expr(obj)?;
            if obj_val.is_pointer_value() {
                let obj_ptr = obj_val.into_pointer_value();
                let obj_type = self.infer_expr_type(obj);
                let class_name = obj_type.name();
                let class_info = self.class_infos.get(class_name).cloned().ok_or_else(|| {
                    format!("codegen: class '{}' not found for field store", class_name)
                })?;
                let v = self.compile_expr(value)?;
                emit_field_store(self.context, &self.builder, &class_info, obj_ptr, field, v)?;
                return Ok(v);
            }
        }
        Err(format!(
            "codegen: unsupported assignment target: {:?}",
            target
        ))
    }

    /// Cast a value to match the given LLVM type (for assignments and
    /// declarations where the declared type may differ from the literal type).
    pub(super) fn cast_value_to_type(
        &self,
        v: BasicValueEnum<'ctx>,
        target_ty: BasicTypeEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if v.get_type() == target_ty {
            return Ok(v);
        }
        // int -> int (different widths)
        if v.is_int_value() && target_ty.is_int_type() {
            let from = v.into_int_value();
            let to_ty = target_ty.into_int_type();
            let from_bits = from.get_type().get_bit_width();
            let to_bits = to_ty.get_bit_width();
            let result = if from_bits < to_bits {
                self.builder
                    .build_int_s_extend(from, to_ty, "cast.sext")
                    .map_err(|e| format!("build_int_s_extend failed: {:?}", e))?
            } else if from_bits > to_bits {
                self.builder
                    .build_int_truncate(from, to_ty, "cast.trunc")
                    .map_err(|e| format!("build_int_truncate failed: {:?}", e))?
            } else {
                from
            };
            return Ok(result.into());
        }
        // float -> float (different widths)
        if v.is_float_value() && target_ty.is_float_type() {
            let from = v.into_float_value();
            let to_ty = target_ty.into_float_type();
            let from_str = from.get_type().print_to_string().to_string();
            let to_str = to_ty.print_to_string().to_string();
            let result = if from_str == "half" && to_str == "float" {
                self.builder
                    .build_float_ext(from, to_ty, "cast.ext")
                    .map_err(|e| format!("build_float_ext failed: {:?}", e))?
            } else if from_str == "half" && to_str == "double" {
                self.builder
                    .build_float_ext(from, to_ty, "cast.ext")
                    .map_err(|e| format!("build_float_ext failed: {:?}", e))?
            } else if from_str == "float" && to_str == "double" {
                self.builder
                    .build_float_ext(from, to_ty, "cast.ext")
                    .map_err(|e| format!("build_float_ext failed: {:?}", e))?
            } else if from_str == "double" && to_str == "float" {
                self.builder
                    .build_float_trunc(from, to_ty, "cast.trunc")
                    .map_err(|e| format!("build_float_trunc failed: {:?}", e))?
            } else if from_str == "double" && to_str == "half" {
                self.builder
                    .build_float_trunc(from, to_ty, "cast.trunc")
                    .map_err(|e| format!("build_float_trunc failed: {:?}", e))?
            } else if from_str == "float" && to_str == "half" {
                self.builder
                    .build_float_trunc(from, to_ty, "cast.trunc")
                    .map_err(|e| format!("build_float_trunc failed: {:?}", e))?
            } else {
                from
            };
            return Ok(result.into());
        }
        // int -> float (implicit widening in assignments)
        if v.is_int_value() && target_ty.is_float_type() {
            let from = v.into_int_value();
            let to_ty = target_ty.into_float_type();
            let result = self
                .builder
                .build_signed_int_to_float(from, to_ty, "cast.itof")
                .map_err(|e| format!("build_signed_int_to_float failed: {:?}", e))?;
            return Ok(result.into());
        }
        // float -> int (explicit `as` cast, e.g. `3.14 as int`)
        if v.is_float_value() && target_ty.is_int_type() {
            let from = v.into_float_value();
            let to_ty = target_ty.into_int_type();
            let result = self
                .builder
                .build_float_to_signed_int(from, to_ty, "cast.ftoi")
                .map_err(|e| format!("build_float_to_signed_int failed: {:?}", e))?;
            return Ok(result.into());
        }
        // struct -> int: extract field 0 (i64) from {i64, ptr} struct, then coerce.
        if v.is_struct_value() && target_ty.is_int_type() {
            if let BasicValueEnum::StructValue(sv) = v {
                if sv.get_type().count_fields() == 2 {
                    if let Ok(f0) = self.builder.build_extract_value(sv, 0, "cast.struct.i64") {
                        let i64_val = f0.into_int_value();
                        let to_ty = target_ty.into_int_type();
                        let result = if i64_val.get_type().get_bit_width() < to_ty.get_bit_width() {
                            self.builder
                                .build_int_s_extend(i64_val, to_ty, "cast.sext")
                                .map_err(|e| format!("build_int_s_extend failed: {:?}", e))?
                        } else if i64_val.get_type().get_bit_width() > to_ty.get_bit_width() {
                            self.builder
                                .build_int_truncate(i64_val, to_ty, "cast.trunc")
                                .map_err(|e| format!("build_int_truncate failed: {:?}", e))?
                        } else {
                            i64_val
                        };
                        return Ok(result.into());
                    }
                }
            }
        }
        // struct -> float: extract field 0 (i64) from {i64, ptr} struct, bitcast to float, then coerce.
        if v.is_struct_value() && target_ty.is_float_type() {
            if let BasicValueEnum::StructValue(sv) = v {
                if sv.get_type().count_fields() == 2 {
                    if let Ok(f0) = self.builder.build_extract_value(sv, 0, "cast.struct.i64") {
                        let i64_val = f0.into_int_value();
                        let to_ty = target_ty.into_float_type();
                        // Bitcast i64 to f64, then extend/truncate to target float type.
                        let f64_ty = self.context.f64_type();
                        let f64_val = self
                            .builder
                            .build_bit_cast(i64_val, f64_ty, "cast.i64tof64")
                            .map_err(|e| format!("build_bit_cast failed: {:?}", e))?;
                        let f64_val = f64_val.into_float_value();
                        let result = self
                            .builder
                            .build_float_ext(f64_val, to_ty, "cast.fext")
                            .map_err(|e| format!("build_float_ext failed: {:?}", e))?;
                        return Ok(result.into());
                    }
                }
            }
        }
        // struct -> pointer: extract field 1 (data ptr) from {i64, ptr} struct.
        if v.is_struct_value() && target_ty.is_pointer_type() {
            if let BasicValueEnum::StructValue(sv) = v {
                if sv.get_type().count_fields() == 2 {
                    if let Ok(f1) = self.builder.build_extract_value(sv, 1, "cast.struct.ptr") {
                        let ptr_val = f1.into_pointer_value();
                        let to_ptr = target_ty.into_pointer_type();
                        if ptr_val.get_type() == to_ptr {
                            return Ok(ptr_val.into());
                        }
                        let cast = self
                            .builder
                            .build_bit_cast(ptr_val, to_ptr, "cast.ptr")
                            .map_err(|e| format!("build_bit_cast ptr failed: {:?}", e))?;
                        return Ok(cast);
                    }
                }
            }
        }
        // int -> pointer: inttoptr.
        if v.is_int_value() && target_ty.is_pointer_type() {
            let int_val = v.into_int_value();
            let to_ptr = target_ty.into_pointer_type();
            let cast = self
                .builder
                .build_int_to_ptr(int_val, to_ptr, "cast.itop")
                .map_err(|e| format!("build_int_to_ptr failed: {:?}", e))?;
            return Ok(cast.into());
        }
        // pointer -> int: ptrtoint.
        if v.is_pointer_value() && target_ty.is_int_type() {
            let ptr_val = v.into_pointer_value();
            let to_ty = target_ty.into_int_type();
            let cast = self
                .builder
                .build_ptr_to_int(ptr_val, to_ty, "cast.ptoi")
                .map_err(|e| format!("build_ptr_to_int failed: {:?}", e))?;
            return Ok(cast.into());
        }
        // No conversion exists: fail honestly with both types named.
        // Returning the value as-is would trip an LLVM C++ assertion
        // inside build_call and abort the compiler without a message.
        Err(format!(
            "codegen: cannot convert value of type '{}' to '{}'",
            v.get_type().print_to_string().to_string(),
            target_ty.print_to_string().to_string()
        ))
    }

    /// Compile a ternary expression using a phi node.
    pub(super) fn compile_ternary(
        &mut self,
        condition: &Expr,
        then_expr: &Expr,
        else_expr: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let cond_val = self.compile_expr(condition)?;
        let cond = if cond_val.is_int_value() {
            cond_val.into_int_value()
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
                        return Err("codegen: ternary condition must be bool".into());
                    }
                } else {
                    return Err("codegen: ternary condition must be bool".into());
                }
            } else {
                unreachable!()
            }
        } else {
            return Err(format!(
                "codegen: ternary condition must be bool, got {:?}",
                cond_val.get_type()
            ));
        };
        let current_block = self
            .builder
            .get_insert_block()
            .ok_or("codegen: no insert block for ternary")?;
        let then_block = self
            .context
            .insert_basic_block_after(current_block, "tern.then");
        let else_block = self
            .context
            .insert_basic_block_after(then_block, "tern.else");
        let end_block = self
            .context
            .insert_basic_block_after(else_block, "tern.end");

        self.builder
            .build_conditional_branch(cond, then_block, else_block)
            .map_err(|e| format!("build_cond_br ternary failed: {:?}", e))?;

        self.builder.position_at_end(then_block);
        let then_val = self.compile_expr(then_expr)?;
        self.builder
            .build_unconditional_branch(end_block)
            .map_err(|e| format!("build_br tern end failed: {:?}", e))?;
        let then_block_end = self
            .builder
            .get_insert_block()
            .ok_or("codegen: no insert block after then")?;

        self.builder.position_at_end(else_block);
        let else_val = self.compile_expr(else_expr)?;
        self.builder
            .build_unconditional_branch(end_block)
            .map_err(|e| format!("build_br tern end 2 failed: {:?}", e))?;
        let else_block_end = self
            .builder
            .get_insert_block()
            .ok_or("codegen: no insert block after else")?;

        // Determine common type. If types differ, rebuild the blocks with coercion.
        if then_val.get_type() != else_val.get_type() {
            // For int type width mismatches, we need to rebuild with sext.
            // Clear and redo the whole ternary with proper coercion.
            match (then_val, else_val) {
                (BasicValueEnum::IntValue(lv), BasicValueEnum::IntValue(rv)) => {
                    let target = if lv.get_type().get_bit_width() >= rv.get_type().get_bit_width() {
                        lv.get_type()
                    } else {
                        rv.get_type()
                    };

                    // Rebuild then block with coercion
                    self.builder.position_at_end(then_block_end);
                    // Remove the terminator
                    then_block_end
                        .get_terminator()
                        .unwrap()
                        .erase_from_basic_block();
                    let lv_coerced = self
                        .builder
                        .build_int_s_extend(lv, target, "coerce.then")
                        .map_err(|e| format!("coerce then failed: {:?}", e))?;
                    self.builder
                        .build_unconditional_branch(end_block)
                        .map_err(|e| format!("rebuild br then failed: {:?}", e))?;

                    // Rebuild else block with coercion
                    self.builder.position_at_end(else_block_end);
                    else_block_end
                        .get_terminator()
                        .unwrap()
                        .erase_from_basic_block();
                    let rv_coerced = self
                        .builder
                        .build_int_s_extend(rv, target, "coerce.else")
                        .map_err(|e| format!("coerce else failed: {:?}", e))?;
                    self.builder
                        .build_unconditional_branch(end_block)
                        .map_err(|e| format!("rebuild br else failed: {:?}", e))?;

                    self.builder.position_at_end(end_block);
                    let phi = self
                        .builder
                        .build_phi(target.as_basic_type_enum(), "tern.result")
                        .map_err(|e| format!("build_phi ternary failed: {:?}", e))?;
                    phi.add_incoming(&[
                        (&BasicValueEnum::IntValue(lv_coerced), then_block_end),
                        (&BasicValueEnum::IntValue(rv_coerced), else_block_end),
                    ]);
                    return Ok(phi.as_basic_value());
                }
                (BasicValueEnum::FloatValue(lv), BasicValueEnum::FloatValue(rv)) => {
                    let target = if lv.get_type().get_bit_width() >= rv.get_type().get_bit_width() {
                        lv.get_type()
                    } else {
                        rv.get_type()
                    };
                    self.builder.position_at_end(then_block_end);
                    then_block_end
                        .get_terminator()
                        .unwrap()
                        .erase_from_basic_block();
                    let lv_coerced = self
                        .builder
                        .build_float_ext(lv, target, "coerce.then")
                        .map_err(|e| format!("coerce then failed: {:?}", e))?;
                    self.builder
                        .build_unconditional_branch(end_block)
                        .map_err(|e| format!("rebuild br then failed: {:?}", e))?;

                    self.builder.position_at_end(else_block_end);
                    else_block_end
                        .get_terminator()
                        .unwrap()
                        .erase_from_basic_block();
                    let rv_coerced = self
                        .builder
                        .build_float_ext(rv, target, "coerce.else")
                        .map_err(|e| format!("coerce else failed: {:?}", e))?;
                    self.builder
                        .build_unconditional_branch(end_block)
                        .map_err(|e| format!("rebuild br else failed: {:?}", e))?;

                    self.builder.position_at_end(end_block);
                    let phi = self
                        .builder
                        .build_phi(target.as_basic_type_enum(), "tern.result")
                        .map_err(|e| format!("build_phi ternary failed: {:?}", e))?;
                    phi.add_incoming(&[
                        (&BasicValueEnum::FloatValue(lv_coerced), then_block_end),
                        (&BasicValueEnum::FloatValue(rv_coerced), else_block_end),
                    ]);
                    return Ok(phi.as_basic_value());
                }
                (BasicValueEnum::IntValue(lv), BasicValueEnum::FloatValue(rv)) => {
                    // int -> float
                    self.builder.position_at_end(then_block_end);
                    then_block_end
                        .get_terminator()
                        .unwrap()
                        .erase_from_basic_block();
                    let lv_coerced = self
                        .builder
                        .build_signed_int_to_float(lv, rv.get_type(), "coerce.itof")
                        .map_err(|e| format!("coerce then failed: {:?}", e))?;
                    self.builder
                        .build_unconditional_branch(end_block)
                        .map_err(|e| format!("rebuild br then failed: {:?}", e))?;

                    self.builder.position_at_end(end_block);
                    let phi = self
                        .builder
                        .build_phi(rv.get_type().as_basic_type_enum(), "tern.result")
                        .map_err(|e| format!("build_phi ternary failed: {:?}", e))?;
                    phi.add_incoming(&[
                        (&BasicValueEnum::FloatValue(lv_coerced), then_block_end),
                        (&else_val, else_block_end),
                    ]);
                    return Ok(phi.as_basic_value());
                }
                (BasicValueEnum::FloatValue(lv), BasicValueEnum::IntValue(rv)) => {
                    // float <- int
                    self.builder.position_at_end(else_block_end);
                    else_block_end
                        .get_terminator()
                        .unwrap()
                        .erase_from_basic_block();
                    let rv_coerced = self
                        .builder
                        .build_signed_int_to_float(rv, lv.get_type(), "coerce.itof")
                        .map_err(|e| format!("coerce else failed: {:?}", e))?;
                    self.builder
                        .build_unconditional_branch(end_block)
                        .map_err(|e| format!("rebuild br else failed: {:?}", e))?;

                    self.builder.position_at_end(end_block);
                    let phi = self
                        .builder
                        .build_phi(lv.get_type().as_basic_type_enum(), "tern.result")
                        .map_err(|e| format!("build_phi ternary failed: {:?}", e))?;
                    phi.add_incoming(&[
                        (&then_val, then_block_end),
                        (&BasicValueEnum::FloatValue(rv_coerced), else_block_end),
                    ]);
                    return Ok(phi.as_basic_value());
                }
                // struct from {i64, ptr} vs int: extract i64 from struct, coerce to int
                (BasicValueEnum::StructValue(sv), BasicValueEnum::IntValue(rv))
                    if sv.get_type().count_fields() == 2 =>
                {
                    let target = rv.get_type();
                    self.builder.position_at_end(then_block_end);
                    then_block_end
                        .get_terminator()
                        .unwrap()
                        .erase_from_basic_block();
                    let f0 = self
                        .builder
                        .build_extract_value(sv, 0, "tern.sv.i64")
                        .map_err(|e| format!("extract failed: {:?}", e))?
                        .into_int_value();
                    let lv_coerced = self
                        .builder
                        .build_int_truncate(f0, target, "coerce.stri")
                        .map_err(|e| format!("coerce then failed: {:?}", e))?;
                    self.builder
                        .build_unconditional_branch(end_block)
                        .map_err(|e| format!("rebuild br then failed: {:?}", e))?;

                    self.builder.position_at_end(end_block);
                    let phi = self
                        .builder
                        .build_phi(target.as_basic_type_enum(), "tern.result")
                        .map_err(|e| format!("build_phi ternary failed: {:?}", e))?;
                    phi.add_incoming(&[
                        (&BasicValueEnum::IntValue(lv_coerced), then_block_end),
                        (&BasicValueEnum::IntValue(rv), else_block_end),
                    ]);
                    return Ok(phi.as_basic_value());
                }
                // int vs struct from {i64, ptr}: extract i64 from struct, coerce to int
                (BasicValueEnum::IntValue(lv), BasicValueEnum::StructValue(sv))
                    if sv.get_type().count_fields() == 2 =>
                {
                    let target = lv.get_type();
                    self.builder.position_at_end(else_block_end);
                    else_block_end
                        .get_terminator()
                        .unwrap()
                        .erase_from_basic_block();
                    let f0 = self
                        .builder
                        .build_extract_value(sv, 0, "tern.sv.i64")
                        .map_err(|e| format!("extract failed: {:?}", e))?
                        .into_int_value();
                    let rv_coerced = self
                        .builder
                        .build_int_truncate(f0, target, "coerce.stru")
                        .map_err(|e| format!("coerce else failed: {:?}", e))?;
                    self.builder
                        .build_unconditional_branch(end_block)
                        .map_err(|e| format!("rebuild br else failed: {:?}", e))?;

                    self.builder.position_at_end(end_block);
                    let phi = self
                        .builder
                        .build_phi(target.as_basic_type_enum(), "tern.result")
                        .map_err(|e| format!("build_phi ternary failed: {:?}", e))?;
                    phi.add_incoming(&[
                        (&BasicValueEnum::IntValue(lv), then_block_end),
                        (&BasicValueEnum::IntValue(rv_coerced), else_block_end),
                    ]);
                    return Ok(phi.as_basic_value());
                }
                _ => {}
            }
        }

        self.builder.position_at_end(end_block);
        let phi = self
            .builder
            .build_phi(then_val.get_type(), "tern.result")
            .map_err(|e| format!("build_phi ternary failed: {:?}", e))?;
        phi.add_incoming(&[(&then_val, then_block_end), (&else_val, else_block_end)]);

        Ok(phi.as_basic_value())
    }
}
