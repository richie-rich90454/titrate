// Binary operator lowering

use super::types as llvm_types;
use super::LlvmBackend;
use crate::ast::{Expr, Operator};
use inkwell::values::BasicValueEnum;

impl<'ctx> LlvmBackend<'ctx> {
    /// Compile a binary expression.
    pub(super) fn compile_binary(
        &mut self,
        left: &Expr,
        op: &Operator,
        right: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        // Short-circuit logical operators.
        match op {
            Operator::And => return self.compile_short_circuit(left, right, false),
            Operator::Or => return self.compile_short_circuit(left, right, true),
            _ => {}
        }

        // String concatenation.
        let left_ty = self.infer_expr_type(left);
        let right_ty = self.infer_expr_type(right);
        if *op == Operator::Add
            && llvm_types::is_string(&left_ty)
            && llvm_types::is_string(&right_ty)
        {
            let l = self.compile_string_expr(left)?;
            let r = self.compile_string_expr(right)?;
            let sv = self.build_string_concat(l, r)?;
            return self.string_value_to_basic(sv);
        }

        // String comparison: ==, !=, <, >, <=, >=
        // Also handle cases where the Titrate type is "unknown" but the LLVM value is a string struct.
        let lv = self.compile_expr(left)?;
        let rv = self.compile_expr(right)?;

        // Fallback string concatenation: both operands are string structs
        // but type inference didn't detect them as strings (e.g., user-defined
        // function calls where infer_expr_type returns "unknown").
        if *op == Operator::Add
            && lv.is_struct_value()
            && rv.is_struct_value()
            && self.is_string_struct(&lv)
            && self.is_string_struct(&rv)
        {
            let l = self.basic_to_string_value(lv)?;
            let r = self.basic_to_string_value(rv)?;
            let sv = self.build_string_concat(l, r)?;
            return self.string_value_to_basic(sv);
        }
        let is_comparison = matches!(
            op,
            Operator::Eq | Operator::Ne | Operator::Lt | Operator::Gt | Operator::Le | Operator::Ge
        );
        let left_is_str =
            llvm_types::is_string(&left_ty) || (lv.is_struct_value() && self.is_string_struct(&lv));
        let right_is_str = rv.is_struct_value() && self.is_string_struct(&rv);
        let is_str_cmp = is_comparison && (left_is_str || right_is_str);
        if let Some(v) = self.compile_binary_str_cmp(op, lv, rv, is_str_cmp)? {
            return Ok(v);
        }

        // lv and rv are already compiled above (lines 855-856).
        // Do NOT re-compile here — that creates duplicate LLVM instructions
        // and can produce different types on the second pass.

        // Extract numeric values from {i64, ptr} structs for non-string operations.
        // When ArrayList_get/HashMap_get goes through the native bridge, it returns
        // a string struct {i64, ptr} even for non-string element types. The actual
        // numeric value is packed in the i64 field (field 0).
        let (lv, rv) = if (lv.is_struct_value() || rv.is_struct_value()) && !is_str_cmp {
            let extract = |v: BasicValueEnum<'ctx>| -> Result<BasicValueEnum<'ctx>, String> {
                if let BasicValueEnum::StructValue(sv) = v {
                    if sv.get_type().count_fields() == 2 {
                        if let Ok(f0) = self.builder.build_extract_value(sv, 0, "sv.0") {
                            return Ok(f0);
                        }
                    }
                }
                Ok(v)
            };
            (extract(lv)?, extract(rv)?)
        } else {
            (lv, rv)
        };

        if let Some(v) = self.compile_binary_struct_cmp(op, lv, rv, is_comparison)? {
            return Ok(v);
        }

        // Coerce operands to the same type if they differ (e.g., i32 var + i64 literal).
        let (lv, rv) = self.coerce_binary_operands(lv, rv)?;

        // Determine if this is an integer or float operation.
        let is_float = lv.is_float_value();
        let is_int = lv.is_int_value();

        if is_float {
            let l = lv.into_float_value();
            let r = rv.into_float_value();
            // Float comparisons produce i1, not float — handle them here
            // before dispatching to compile_float_binary (which only does
            // arithmetic and would otherwise return a FloatValue).
            if let Some(pred) = Self::float_compare_predicate(op) {
                let cmp = self
                    .builder
                    .build_float_compare(pred, l, r, "fcmp")
                    .map_err(|e| format!("build_float_compare failed: {:?}", e))?;
                return Ok(cmp.into());
            }
            return Ok(self.compile_float_binary(op, l, r)?.into());
        }

        if is_int {
            let l = lv.into_int_value();
            let r = rv.into_int_value();
            return Ok(self.compile_int_binary(op, l, r, &left_ty)?.into());
        }

        // Pointer comparison (e.g., entries == null, entries != null)
        if lv.is_pointer_value() && rv.is_pointer_value() {
            let l = lv.into_pointer_value();
            let r = rv.into_pointer_value();
            match op {
                Operator::Eq => {
                    let cmp = self
                        .builder
                        .build_int_compare(inkwell::IntPredicate::EQ, l, r, "ptr.eq")
                        .map_err(|e| format!("build_int_compare ptr failed: {:?}", e))?;
                    return Ok(cmp.into());
                }
                Operator::Ne => {
                    let cmp = self
                        .builder
                        .build_int_compare(inkwell::IntPredicate::NE, l, r, "ptr.ne")
                        .map_err(|e| format!("build_int_compare ptr failed: {:?}", e))?;
                    return Ok(cmp.into());
                }
                _ => {}
            }
        }

        // Struct comparison (string == string) - compare length and pointer
        if lv.is_struct_value() && rv.is_struct_value() {
            let ls = lv.into_struct_value();
            let rs = rv.into_struct_value();
            if ls.get_type() == rs.get_type()
                && self.is_string_struct(&BasicValueEnum::StructValue(ls))
            {
                let l_len = self
                    .builder
                    .build_extract_value(ls, 0, "l.len")
                    .map_err(|e| format!("extract l.len failed: {:?}", e))?;
                let r_len = self
                    .builder
                    .build_extract_value(rs, 0, "r.len")
                    .map_err(|e| format!("extract r.len failed: {:?}", e))?;
                let l_ptr = self
                    .builder
                    .build_extract_value(ls, 1, "l.ptr")
                    .map_err(|e| format!("extract l.ptr failed: {:?}", e))?;
                let r_ptr = self
                    .builder
                    .build_extract_value(rs, 1, "r.ptr")
                    .map_err(|e| format!("extract r.ptr failed: {:?}", e))?;
                match op {
                    Operator::Eq => {
                        let len_eq = self
                            .builder
                            .build_int_compare(
                                inkwell::IntPredicate::EQ,
                                l_len.into_int_value(),
                                r_len.into_int_value(),
                                "len.eq",
                            )
                            .map_err(|e| format!("build_int_compare len failed: {:?}", e))?;
                        let ptr_eq = self
                            .builder
                            .build_int_compare(
                                inkwell::IntPredicate::EQ,
                                l_ptr.into_pointer_value(),
                                r_ptr.into_pointer_value(),
                                "ptr.eq",
                            )
                            .map_err(|e| format!("build_int_compare ptr failed: {:?}", e))?;
                        let result = self
                            .builder
                            .build_and(len_eq, ptr_eq, "str.eq")
                            .map_err(|e| format!("build and failed: {:?}", e))?;
                        return Ok(result.into());
                    }
                    Operator::Ne => {
                        let len_eq = self
                            .builder
                            .build_int_compare(
                                inkwell::IntPredicate::EQ,
                                l_len.into_int_value(),
                                r_len.into_int_value(),
                                "len.eq",
                            )
                            .map_err(|e| format!("build_int_compare len failed: {:?}", e))?;
                        let ptr_eq = self
                            .builder
                            .build_int_compare(
                                inkwell::IntPredicate::EQ,
                                l_ptr.into_pointer_value(),
                                r_ptr.into_pointer_value(),
                                "ptr.eq",
                            )
                            .map_err(|e| format!("build_int_compare ptr failed: {:?}", e))?;
                        let eq_val = self
                            .builder
                            .build_and(len_eq, ptr_eq, "str.eq.tmp")
                            .map_err(|e| format!("build and failed: {:?}", e))?;
                        let result = self
                            .builder
                            .build_not(eq_val, "str.ne")
                            .map_err(|e| format!("build not failed: {:?}", e))?;
                        return Ok(result.into());
                    }
                    _ => {}
                }
            }
        }

        if let Some(v) = self.compile_binary_num_cmp(op, lv, rv, is_comparison)? {
            return Ok(v);
        }

        Err(format!(
            "codegen: unsupported binary operand types: {:?} {:?}",
            lv.get_type(),
            rv.get_type()
        ))
    }
}
