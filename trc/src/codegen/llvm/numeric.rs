// Numeric binary operators

use super::LlvmBackend;
use crate::ast::{Expr, Operator, Type, UnOp};
use inkwell::AddressSpace;
use inkwell::types::BasicTypeEnum;
use inkwell::values::{BasicValueEnum, IntValue};

impl<'ctx> LlvmBackend<'ctx> {
    /// and comparison instructions don't trigger LLVM assertion failures.
    /// The narrower type is extended to the wider type (signed for ints,
    /// extended for floats). int+float mixes convert the int to float.
    pub(super) fn coerce_binary_operands(
        &self,
        lv: BasicValueEnum<'ctx>,
        rv: BasicValueEnum<'ctx>,
    ) -> Result<(BasicValueEnum<'ctx>, BasicValueEnum<'ctx>), String> {
        if lv.get_type() == rv.get_type() {
            return Ok((lv, rv));
        }
        // Both int: sign-extend the narrower one.
        if lv.is_int_value() && rv.is_int_value() {
            let l = lv.into_int_value();
            let r = rv.into_int_value();
            let l_bits = l.get_type().get_bit_width();
            let r_bits = r.get_type().get_bit_width();
            if l_bits < r_bits {
                let l2 = self
                    .builder
                    .build_int_s_extend(l, r.get_type(), "coerce.sext")
                    .map_err(|e| format!("build_int_s_extend coerce failed: {:?}", e))?;
                Ok((l2.into(), rv))
            } else {
                let r2 = self
                    .builder
                    .build_int_s_extend(r, l.get_type(), "coerce.sext")
                    .map_err(|e| format!("build_int_s_extend coerce failed: {:?}", e))?;
                Ok((lv, r2.into()))
            }
        } else if lv.is_float_value() && rv.is_float_value() {
            // Both float: extend the narrower one.
            let l = lv.into_float_value();
            let r = rv.into_float_value();
            let l_str = l.get_type().print_to_string().to_string();
            let r_str = r.get_type().print_to_string().to_string();
            let l_bits: u32 = match l_str.as_str() {
                "half" => 16,
                "float" => 32,
                "double" => 64,
                "fp128" => 128,
                _ => 64,
            };
            let r_bits: u32 = match r_str.as_str() {
                "half" => 16,
                "float" => 32,
                "double" => 64,
                "fp128" => 128,
                _ => 64,
            };
            if l_bits < r_bits {
                let l2 = self
                    .builder
                    .build_float_ext(l, r.get_type(), "coerce.fext")
                    .map_err(|e| format!("build_float_ext coerce failed: {:?}", e))?;
                Ok((l2.into(), rv))
            } else {
                let r2 = self
                    .builder
                    .build_float_ext(r, l.get_type(), "coerce.fext")
                    .map_err(|e| format!("build_float_ext coerce failed: {:?}", e))?;
                Ok((lv, r2.into()))
            }
        } else if lv.is_int_value() && rv.is_float_value() {
            // int + float: convert int to float.
            let l = lv.into_int_value();
            let r_ty = rv.into_float_value().get_type();
            let l2 = self
                .builder
                .build_signed_int_to_float(l, r_ty, "coerce.itof")
                .map_err(|e| format!("build_signed_int_to_float coerce failed: {:?}", e))?;
            Ok((l2.into(), rv))
        } else if lv.is_float_value() && rv.is_int_value() {
            // float + int: convert int to float.
            let r = rv.into_int_value();
            let l_ty = lv.into_float_value().get_type();
            let r2 = self
                .builder
                .build_signed_int_to_float(r, l_ty, "coerce.itof")
                .map_err(|e| format!("build_signed_int_to_float coerce failed: {:?}", e))?;
            Ok((lv, r2.into()))
        } else {
            // If we can't coerce, just return as-is and hope.
            Ok((lv, rv))
        }
    }

    /// Compile an integer binary operation.
    pub(super) fn compile_int_binary(
        &self,
        op: &Operator,
        l: IntValue<'ctx>,
        r: IntValue<'ctx>,
        ty: &Type,
    ) -> Result<IntValue<'ctx>, String> {
        use inkwell::IntPredicate::*;
        match op {
            Operator::Add => self
                .builder
                .build_int_add(l, r, "add")
                .map_err(|e| format!("build_int_add failed: {:?}", e)),
            Operator::Sub => self
                .builder
                .build_int_sub(l, r, "sub")
                .map_err(|e| format!("build_int_sub failed: {:?}", e)),
            Operator::Mul => self
                .builder
                .build_int_mul(l, r, "mul")
                .map_err(|e| format!("build_int_mul failed: {:?}", e)),
            Operator::Div => {
                // Signed or unsigned depending on type.
                if Self::is_unsigned_type(ty) {
                    self.builder
                        .build_int_unsigned_div(l, r, "udiv")
                        .map_err(|e| format!("build_int_unsigned_div failed: {:?}", e))
                } else {
                    self.builder
                        .build_int_signed_div(l, r, "sdiv")
                        .map_err(|e| format!("build_int_signed_div failed: {:?}", e))
                }
            }
            Operator::Mod => {
                if Self::is_unsigned_type(ty) {
                    self.builder
                        .build_int_unsigned_rem(l, r, "urem")
                        .map_err(|e| format!("build_int_unsigned_rem failed: {:?}", e))
                } else {
                    self.builder
                        .build_int_signed_rem(l, r, "srem")
                        .map_err(|e| format!("build_int_signed_rem failed: {:?}", e))
                }
            }
            Operator::Eq => self
                .builder
                .build_int_compare(EQ, l, r, "eq")
                .map_err(|e| format!("build_int_compare eq failed: {:?}", e)),
            Operator::Ne => self
                .builder
                .build_int_compare(NE, l, r, "ne")
                .map_err(|e| format!("build_int_compare ne failed: {:?}", e)),
            Operator::Lt => {
                let pred = if Self::is_unsigned_type(ty) { ULT } else { SLT };
                self.builder
                    .build_int_compare(pred, l, r, "lt")
                    .map_err(|e| format!("build_int_compare lt failed: {:?}", e))
            }
            Operator::Gt => {
                let pred = if Self::is_unsigned_type(ty) { UGT } else { SGT };
                self.builder
                    .build_int_compare(pred, l, r, "gt")
                    .map_err(|e| format!("build_int_compare gt failed: {:?}", e))
            }
            Operator::Le => {
                let pred = if Self::is_unsigned_type(ty) { ULE } else { SLE };
                self.builder
                    .build_int_compare(pred, l, r, "le")
                    .map_err(|e| format!("build_int_compare le failed: {:?}", e))
            }
            Operator::Ge => {
                let pred = if Self::is_unsigned_type(ty) { UGE } else { SGE };
                self.builder
                    .build_int_compare(pred, l, r, "ge")
                    .map_err(|e| format!("build_int_compare ge failed: {:?}", e))
            }
            Operator::BitAnd => self
                .builder
                .build_and(l, r, "band")
                .map_err(|e| format!("build_and failed: {:?}", e)),
            Operator::BitOr => self
                .builder
                .build_or(l, r, "bor")
                .map_err(|e| format!("build_or failed: {:?}", e)),
            Operator::BitXor => self
                .builder
                .build_xor(l, r, "bxor")
                .map_err(|e| format!("build_xor failed: {:?}", e)),
            Operator::BitShl => self
                .builder
                .build_left_shift(l, r, "shl")
                .map_err(|e| format!("build_left_shift failed: {:?}", e)),
            Operator::BitShr | Operator::BitUshr => {
                if Self::is_unsigned_type(ty) || *op == Operator::BitUshr {
                    self.builder
                        .build_right_shift(l, r, false, "ushr")
                        .map_err(|e| format!("build_right_shift ushr failed: {:?}", e))
                } else {
                    self.builder
                        .build_right_shift(l, r, true, "sshr")
                        .map_err(|e| format!("build_right_shift sshr failed: {:?}", e))
                }
            }
            Operator::And | Operator::Or => {
                unreachable!("short-circuit operators handled elsewhere")
            }
        }
    }

    /// Map a comparison operator to its LLVM float predicate, or return
    /// `None` if the operator is not a comparison (i.e. arithmetic).
    pub(super) fn float_compare_predicate(op: &Operator) -> Option<inkwell::FloatPredicate> {
        use inkwell::FloatPredicate::*;
        match op {
            Operator::Eq => Some(OEQ),
            Operator::Ne => Some(ONE),
            Operator::Lt => Some(OLT),
            Operator::Gt => Some(OGT),
            Operator::Le => Some(OLE),
            Operator::Ge => Some(OGE),
            _ => None,
        }
    }

    /// Compile a float binary operation (arithmetic only — comparisons are
    /// handled in `compile_binary` because they produce i1, not float).
    pub(super) fn compile_float_binary(
        &self,
        op: &Operator,
        l: inkwell::values::FloatValue<'ctx>,
        r: inkwell::values::FloatValue<'ctx>,
    ) -> Result<inkwell::values::FloatValue<'ctx>, String> {
        match op {
            Operator::Add => self
                .builder
                .build_float_add(l, r, "fadd")
                .map_err(|e| format!("build_float_add failed: {:?}", e)),
            Operator::Sub => self
                .builder
                .build_float_sub(l, r, "fsub")
                .map_err(|e| format!("build_float_sub failed: {:?}", e)),
            Operator::Mul => self
                .builder
                .build_float_mul(l, r, "fmul")
                .map_err(|e| format!("build_float_mul failed: {:?}", e)),
            Operator::Div => self
                .builder
                .build_float_div(l, r, "fdiv")
                .map_err(|e| format!("build_float_div failed: {:?}", e)),
            Operator::Mod => self
                .builder
                .build_float_rem(l, r, "frem")
                .map_err(|e| format!("build_float_rem failed: {:?}", e)),
            _ => Err(format!("codegen: unsupported float operator {:?}", op)),
        }
    }

    /// Compile a short-circuit logical operator (&& or ||).
    /// If `is_or` is true, this is ||; otherwise it's &&.
    pub(super) fn compile_short_circuit(
        &mut self,
        left: &Expr,
        right: &Expr,
        is_or: bool,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let i1_ty = self.context.bool_type();
        let current_block = self
            .builder
            .get_insert_block()
            .ok_or("codegen: no insert block for short-circuit")?;
        let rhs_block = self
            .context
            .insert_basic_block_after(current_block, "logic.rhs");
        let end_block = self
            .context
            .insert_basic_block_after(rhs_block, "logic.end");

        let lv_val = self.compile_expr(left)?;
        let lv = if lv_val.is_int_value() {
            let int_val = lv_val.into_int_value();
            if int_val.get_type() == self.context.bool_type() {
                int_val
            } else {
                let zero = int_val.get_type().const_int(0, false);
                self.builder
                    .build_int_compare(inkwell::IntPredicate::NE, int_val, zero, "sc.bool")
                    .map_err(|e| format!("build_int_compare sc bool coercion failed: {:?}", e))?
            }
        } else if lv_val.is_struct_value() {
            // Coerce struct to bool: check if data pointer is non-null.
            if let BasicValueEnum::StructValue(sv) = lv_val {
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
                        return Err("codegen: logical operand must be bool".into());
                    }
                } else {
                    return Err("codegen: logical operand must be bool".into());
                }
            } else {
                unreachable!()
            }
        } else {
            return Err(format!(
                "codegen: logical left operand must be bool, got {:?}",
                lv_val.get_type()
            ));
        };

        // compile_expr(left) may have created blocks (ternary, nested &&/||, etc.)
        // and moved the builder. Re-capture so the PHI references the correct
        // predecessor — the block where the conditional branch is actually built.
        let current_block = self
            .builder
            .get_insert_block()
            .ok_or("codegen: no insert block after left in short-circuit")?;

        if is_or {
            // ||: if left is true, short-circuit to end with true.
            self.builder
                .build_conditional_branch(lv, end_block, rhs_block)
                .map_err(|e| format!("build_cond_br or failed: {:?}", e))?;
        } else {
            // &&: if left is false, short-circuit to end with false.
            self.builder
                .build_conditional_branch(lv, rhs_block, end_block)
                .map_err(|e| format!("build_cond_br and failed: {:?}", e))?;
        }

        // RHS block: evaluate right, then go to end.
        self.builder.position_at_end(rhs_block);
        let rv_val = self.compile_expr(right)?;
        let rv = if rv_val.is_int_value() {
            let int_val = rv_val.into_int_value();
            if int_val.get_type() == self.context.bool_type() {
                int_val
            } else {
                let zero = int_val.get_type().const_int(0, false);
                self.builder
                    .build_int_compare(inkwell::IntPredicate::NE, int_val, zero, "sc.rhs.bool")
                    .map_err(|e| {
                        format!("build_int_compare sc rhs bool coercion failed: {:?}", e)
                    })?
            }
        } else if rv_val.is_struct_value() {
            if let BasicValueEnum::StructValue(sv) = rv_val {
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
                        return Err("codegen: logical operand must be bool".into());
                    }
                } else {
                    return Err("codegen: logical operand must be bool".into());
                }
            } else {
                unreachable!()
            }
        } else {
            return Err(format!(
                "codegen: logical right operand must be bool, got {:?}",
                rv_val.get_type()
            ));
        };
        let rhs_block_now = self
            .builder
            .get_insert_block()
            .ok_or("codegen: no insert block after rhs")?;
        self.builder
            .build_unconditional_branch(end_block)
            .map_err(|e| format!("build_br end failed: {:?}", e))?;

        // End block: phi together the results.
        self.builder.position_at_end(end_block);
        let phi = self
            .builder
            .build_phi(i1_ty, "logic.result")
            .map_err(|e| format!("build_phi failed: {:?}", e))?;
        let short_circuit_val = i1_ty.const_int(if is_or { 1 } else { 0 }, false);
        phi.add_incoming(&[(&short_circuit_val, current_block), (&rv, rhs_block_now)]);

        Ok(phi.as_basic_value())
    }

    /// Compile a unary expression.
    pub(super) fn compile_unary(&mut self, op: &UnOp, operand: &Expr) -> Result<BasicValueEnum<'ctx>, String> {
        let v = self.compile_expr(operand)?;
        match op {
            UnOp::Neg => {
                if v.is_float_value() {
                    let f = v.into_float_value();
                    let neg = self
                        .builder
                        .build_float_neg(f, "fneg")
                        .map_err(|e| format!("build_float_neg failed: {:?}", e))?;
                    Ok(neg.into())
                } else {
                    let i = v.into_int_value();
                    let zero = i.get_type().const_int(0, false);
                    let neg = self
                        .builder
                        .build_int_sub(zero, i, "ineg")
                        .map_err(|e| format!("build_int_sub neg failed: {:?}", e))?;
                    Ok(neg.into())
                }
            }
            UnOp::Not => {
                let i = v.into_int_value();
                let not = self
                    .builder
                    .build_not(i, "lnot")
                    .map_err(|e| format!("build_not failed: {:?}", e))?;
                Ok(not.into())
            }
            UnOp::BitNot => {
                let i = v.into_int_value();
                let not = self
                    .builder
                    .build_not(i, "bnot")
                    .map_err(|e| format!("build_not bit failed: {:?}", e))?;
                Ok(not.into())
            }
        }
    }

}
