// Numeric binary operators

use super::LlvmBackend;
use crate::ast::{Expr, Operator, Type, UnOp};
use inkwell::AddressSpace;
use inkwell::types::{BasicTypeEnum, FloatType};
use inkwell::values::{BasicValueEnum, IntValue};

/// Float bit widths by LLVM spelling. Unknown types are rejected by coercion
/// instead of being guessed, so we never emit an invalid `fpext`/`fptrunc`.
fn float_type_bits(ft: FloatType<'_>) -> Option<u32> {
    match ft.print_to_string().to_string().as_str() {
        "half" => Some(16),
        "float" => Some(32),
        "double" => Some(64),
        "x86_fp80" => Some(80),
        "fp128" | "ppc_fp128" => Some(128),
        _ => None,
    }
}

impl<'ctx> LlvmBackend<'ctx> {
    /// Coerce operands so arithmetic and comparison instructions don't
    /// trigger LLVM assertion failures. The narrower type is extended to the
    /// wider type (signed for ints, extended for floats). int+float mixes
    /// convert the int to float.
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
            // Both float: extend the narrower one. Never call build_float_ext
            // when the widths are equal but the types differ (that would be a
            // truncation and trip an LLVM assertion).
            let l = lv.into_float_value();
            let r = rv.into_float_value();
            let l_bits = float_type_bits(l.get_type()).ok_or_else(|| {
                format!(
                    "codegen: unsupported float type {} in coercion",
                    l.get_type().print_to_string()
                )
            })?;
            let r_bits = float_type_bits(r.get_type()).ok_or_else(|| {
                format!(
                    "codegen: unsupported float type {} in coercion",
                    r.get_type().print_to_string()
                )
            })?;
            if l_bits < r_bits {
                let l2 = self
                    .builder
                    .build_float_ext(l, r.get_type(), "coerce.fext")
                    .map_err(|e| format!("build_float_ext coerce failed: {:?}", e))?;
                Ok((l2.into(), rv))
            } else if r_bits < l_bits {
                let r2 = self
                    .builder
                    .build_float_ext(r, l.get_type(), "coerce.fext")
                    .map_err(|e| format!("build_float_ext coerce failed: {:?}", e))?;
                Ok((lv, r2.into()))
            } else {
                Err(format!(
                    "codegen: cannot coerce distinct float types of equal width: {} and {}",
                    l.get_type().print_to_string(),
                    r.get_type().print_to_string()
                ))
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
            // Returning mismatched operands here makes the panicking
            // `into_int_value`/`into_float_value` accessors downstream blow up
            // with no source context. Fail here instead.
            let func = self
                .builder
                .get_insert_block()
                .and_then(|b| b.get_parent())
                .and_then(|f| f.get_name().to_str().ok().map(|s| s.to_string()))
                .unwrap_or_else(|| "?".into());
            Err(format!(
                "codegen: cannot coerce binary operands of different types: {} and {} in function {}",
                lv.get_type().print_to_string().to_string(),
                rv.get_type().print_to_string().to_string(),
                func
            ))
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
        // LLVM asserts when integer operands of different widths reach a
        // binary instruction. Reject the mismatch with source-facing context.
        if l.get_type() != r.get_type() {
            return Err(format!(
                "codegen: integer {:?} operands must have equal widths, got {} and {} bits",
                op,
                l.get_type().get_bit_width(),
                r.get_type().get_bit_width()
            ));
        }
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
            Operator::Div | Operator::Mod => {
                // LLVM asserts when constant-folding a division by zero.
                if r.get_zero_extended_constant() == Some(0) {
                    return Err(format!("codegen: integer {:?} with zero divisor", op));
                }
                // Signed or unsigned depending on type.
                if Self::is_unsigned_type(ty) {
                    if *op == Operator::Div {
                        self.builder
                            .build_int_unsigned_div(l, r, "udiv")
                            .map_err(|e| format!("build_int_unsigned_div failed: {:?}", e))
                    } else {
                        self.builder
                            .build_int_unsigned_rem(l, r, "urem")
                            .map_err(|e| format!("build_int_unsigned_rem failed: {:?}", e))
                    }
                } else if *op == Operator::Div {
                    self.builder
                        .build_int_signed_div(l, r, "sdiv")
                        .map_err(|e| format!("build_int_signed_div failed: {:?}", e))
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
            Operator::BitShl => {
                if let Some(amount) = r.get_zero_extended_constant() {
                    if amount >= l.get_type().get_bit_width() as u64 {
                        return Err(format!(
                            "codegen: shift amount {} exceeds operand width {} in {:?}",
                            amount,
                            l.get_type().get_bit_width(),
                            op
                        ));
                    }
                }
                self.builder
                    .build_left_shift(l, r, "shl")
                    .map_err(|e| format!("build_left_shift failed: {:?}", e))
            }
            Operator::BitShr | Operator::BitUshr => {
                if let Some(amount) = r.get_zero_extended_constant() {
                    if amount >= l.get_type().get_bit_width() as u64 {
                        return Err(format!(
                            "codegen: shift amount {} exceeds operand width {} in {:?}",
                            amount,
                            l.get_type().get_bit_width(),
                            op
                        ));
                    }
                }
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
                Err("codegen: short-circuit operators are not integer operations".into())
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
        if l.get_type() != r.get_type() {
            return Err(format!(
                "codegen: float {:?} operands must have matching types, got {} and {}",
                op,
                l.get_type().print_to_string(),
                r.get_type().print_to_string()
            ));
        }
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
                return Err("codegen: logical operand must be bool".into());
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
                return Err("codegen: logical operand must be bool".into());
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
                } else if v.is_int_value() {
                    let i = v.into_int_value();
                    let zero = i.get_type().const_int(0, false);
                    let neg = self
                        .builder
                        .build_int_sub(zero, i, "ineg")
                        .map_err(|e| format!("build_int_sub neg failed: {:?}", e))?;
                    Ok(neg.into())
                } else {
                    Err(format!(
                        "codegen: unary '-' requires a numeric operand, got {}",
                        v.get_type().print_to_string()
                    ))
                }
            }
            UnOp::Not => {
                if !v.is_int_value() {
                    return Err(format!(
                        "codegen: unary '!' requires an integer operand, got {}",
                        v.get_type().print_to_string()
                    ));
                }
                let i = v.into_int_value();
                let not = self
                    .builder
                    .build_not(i, "lnot")
                    .map_err(|e| format!("build_not failed: {:?}", e))?;
                Ok(not.into())
            }
            UnOp::BitNot => {
                if !v.is_int_value() {
                    return Err(format!(
                        "codegen: unary '~' requires an integer operand, got {}",
                        v.get_type().print_to_string()
                    ));
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Literal, Span};
    use inkwell::context::Context;

    fn backend_with_block<'ctx>(context: &'ctx Context) -> LlvmBackend<'ctx> {
        let mut backend = LlvmBackend::new(context, "unary-test");
        let f = backend
            .module
            .add_function("unary_test_fn", context.void_type().fn_type(&[], false), None);
        let entry = context.append_basic_block(f, "entry");
        backend.builder.position_at_end(entry);
        backend
    }

    #[test]
    fn unary_neg_rejects_pointer_operand() {
        let context = Context::create();
        let mut backend = backend_with_block(&context);
        let operand = Expr::Literal(Literal::Null, Span::unknown());

        let err = backend
            .compile_unary(&UnOp::Neg, &operand)
            .expect_err("negating a pointer must be an error, not a panic");
        assert!(err.contains("numeric"), "{}", err);
        assert!(err.contains("ptr"), "{}", err);
    }

    #[test]
    fn unary_not_rejects_string_struct_operand() {
        let context = Context::create();
        let mut backend = backend_with_block(&context);
        let operand = Expr::Literal(Literal::String("x".into()), Span::unknown());

        let err = backend
            .compile_unary(&UnOp::Not, &operand)
            .expect_err("logical-not on a string struct must be an error, not a panic");
        assert!(err.contains("integer"), "{}", err);
    }

    #[test]
    fn unary_bitnot_rejects_float_operand() {
        let context = Context::create();
        let mut backend = backend_with_block(&context);
        let operand = Expr::Literal(Literal::Float(1.5), Span::unknown());

        let err = backend
            .compile_unary(&UnOp::BitNot, &operand)
            .expect_err("bitwise-not on a float must be an error, not a panic");
        assert!(err.contains("integer"), "{}", err);
    }

    #[test]
    fn coerce_rejects_int_pointer_mismatch() {
        let context = Context::create();
        let backend = LlvmBackend::new(&context, "coerce-test");
        let i = context.i32_type().const_int(1, false);
        let ptr = context.ptr_type(AddressSpace::default()).const_null();

        let err = backend
            .coerce_binary_operands(i.into(), ptr.into())
            .expect_err("int/pointer operands must not be coerced silently");
        assert!(
            err.contains("cannot coerce"),
            "unexpected error: {}",
            err
        );
        assert!(err.contains("i32"), "error should name the int type: {}", err);
        assert!(err.contains("ptr"), "error should name the pointer type: {}", err);
    }

    #[test]
    fn coerce_accepts_matching_int_operands() {
        let context = Context::create();
        let backend = LlvmBackend::new(&context, "coerce-test");
        let a = context.i32_type().const_int(1, false);
        let b = context.i32_type().const_int(2, false);

        let (l, r) = backend
            .coerce_binary_operands(a.into(), b.into())
            .expect("matching int operands should coerce");
        assert!(l.is_int_value() && r.is_int_value());
    }

    #[test]
    fn int_binary_rejects_width_mismatch() {
        let context = Context::create();
        let backend = backend_with_block(&context);
        let l = context.i32_type().const_int(1, false);
        let r = context.i64_type().const_int(2, false);

        let err = backend
            .compile_int_binary(&Operator::Add, l, r, &Type::simple("int"))
            .expect_err("mixed-width integers must be rejected before LLVM asserts");
        assert!(err.contains("equal widths"), "{}", err);
        assert!(err.contains("32") && err.contains("64"), "{}", err);
    }

    #[test]
    fn int_binary_rejects_zero_divisor() {
        let context = Context::create();
        let backend = backend_with_block(&context);
        let one = context.i32_type().const_int(1, false);
        let zero = context.i32_type().const_zero();

        let err = backend
            .compile_int_binary(&Operator::Div, one, zero, &Type::simple("int"))
            .expect_err("constant division by zero must not reach LLVM folding");
        assert!(err.contains("zero divisor"), "{}", err);
    }

    #[test]
    fn int_binary_rejects_oversized_shift() {
        let context = Context::create();
        let backend = backend_with_block(&context);
        let one = context.i32_type().const_int(1, false);
        let thirty_two = context.i32_type().const_int(32, false);

        let err = backend
            .compile_int_binary(&Operator::BitShl, one, thirty_two, &Type::simple("int"))
            .expect_err("shift at or beyond operand width must be rejected");
        assert!(err.contains("shift amount"), "{}", err);
    }

    #[test]
    fn float_binary_rejects_type_mismatch() {
        let context = Context::create();
        let backend = backend_with_block(&context);
        let l = context.f32_type().const_float(1.0);
        let r = context.f64_type().const_float(2.0);

        let err = backend
            .compile_float_binary(&Operator::Add, l, r)
            .expect_err("mixed float types must be rejected before LLVM asserts");
        assert!(err.contains("matching types"), "{}", err);
    }
}
