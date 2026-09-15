// Numeric and fallback struct comparisons

use super::LlvmBackend;
use crate::ast::Operator;
use inkwell::types::BasicTypeEnum;
use inkwell::values::BasicValueEnum;

impl<'ctx> LlvmBackend<'ctx> {
    pub(super) fn compile_binary_num_cmp(&mut self, op: &Operator, lv: BasicValueEnum<'ctx>, rv: BasicValueEnum<'ctx>, is_comparison: bool) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        // Fallback for struct comparisons not handled above.
        // Handles: StructType vs IntType, StructType vs PointerType,
        // StructType vs FloatType, StructType vs StructType (non-string).
        if is_comparison {
            fn is_len_ptr_struct(v: &BasicValueEnum<'_>) -> bool {
                if let BasicValueEnum::StructValue(sv) = v {
                    let st = sv.get_type();
                    if st.count_fields() == 2 {
                        matches!(
                            (st.get_field_type_at_index(0), st.get_field_type_at_index(1)),
                            (
                                Some(BasicTypeEnum::IntType(_)),
                                Some(BasicTypeEnum::PointerType(_))
                            )
                        )
                    } else {
                        false
                    }
                } else {
                    false
                }
            }

            // Struct vs Pointer (e.g. entries == null)
            if lv.is_struct_value() && rv.is_pointer_value() && is_len_ptr_struct(&lv) {
                let sv = lv.into_struct_value();
                let data_ptr = self
                    .builder
                    .build_extract_value(sv, 1, "struct.data.ptr")
                    .map_err(|e| format!("extract failed: {:?}", e))?
                    .into_pointer_value();
                let pred = match op {
                    Operator::Eq => inkwell::IntPredicate::EQ,
                    Operator::Ne => inkwell::IntPredicate::NE,
                    _ => return Err("unsupported op for struct vs pointer".into()),
                };
                return Ok(Some(self
                    .builder
                    .build_int_compare(pred, data_ptr, rv.into_pointer_value(), "cmp")
                    .map_err(|e| format!("icmp failed: {:?}", e))?
                    .into()));
            }
            if rv.is_struct_value() && lv.is_pointer_value() && is_len_ptr_struct(&rv) {
                let sv = rv.into_struct_value();
                let data_ptr = self
                    .builder
                    .build_extract_value(sv, 1, "struct.data.ptr")
                    .map_err(|e| format!("extract failed: {:?}", e))?
                    .into_pointer_value();
                let pred = match op {
                    Operator::Eq => inkwell::IntPredicate::EQ,
                    Operator::Ne => inkwell::IntPredicate::NE,
                    _ => return Err("unsupported op".into()),
                };
                return Ok(Some(self
                    .builder
                    .build_int_compare(pred, lv.into_pointer_value(), data_ptr, "cmp")
                    .map_err(|e| format!("icmp failed: {:?}", e))?
                    .into()));
            }

            // Struct vs Int (e.g. size() comparison)
            if lv.is_struct_value() && rv.is_int_value() && is_len_ptr_struct(&lv) {
                let sv = lv.into_struct_value();
                let len_val = self
                    .builder
                    .build_extract_value(sv, 0, "struct.len")
                    .map_err(|e| format!("extract failed: {:?}", e))?
                    .into_int_value();
                let (l, r) = self.coerce_binary_operands(len_val.into(), rv)?;
                let pred = match op {
                    Operator::Eq => inkwell::IntPredicate::EQ,
                    Operator::Ne => inkwell::IntPredicate::NE,
                    Operator::Lt => inkwell::IntPredicate::SLT,
                    Operator::Gt => inkwell::IntPredicate::SGT,
                    Operator::Le => inkwell::IntPredicate::SLE,
                    Operator::Ge => inkwell::IntPredicate::SGE,
                    _ => unreachable!(),
                };
                return Ok(Some(self
                    .builder
                    .build_int_compare(pred, l.into_int_value(), r.into_int_value(), "cmp")
                    .map_err(|e| format!("icmp failed: {:?}", e))?
                    .into()));
            }
            if rv.is_struct_value() && lv.is_int_value() && is_len_ptr_struct(&rv) {
                let sv = rv.into_struct_value();
                let len_val = self
                    .builder
                    .build_extract_value(sv, 0, "struct.len")
                    .map_err(|e| format!("extract failed: {:?}", e))?
                    .into_int_value();
                let (l, r) = self.coerce_binary_operands(lv, len_val.into())?;
                let pred = match op {
                    Operator::Eq => inkwell::IntPredicate::EQ,
                    Operator::Ne => inkwell::IntPredicate::NE,
                    Operator::Lt => inkwell::IntPredicate::SGT,
                    Operator::Gt => inkwell::IntPredicate::SLT,
                    Operator::Le => inkwell::IntPredicate::SGE,
                    Operator::Ge => inkwell::IntPredicate::SLE,
                    _ => unreachable!(),
                };
                return Ok(Some(self
                    .builder
                    .build_int_compare(pred, l.into_int_value(), r.into_int_value(), "cmp")
                    .map_err(|e| format!("icmp failed: {:?}", e))?
                    .into()));
            }

            // Struct vs Float
            if lv.is_struct_value() && rv.is_float_value() && is_len_ptr_struct(&lv) {
                let sv = lv.into_struct_value();
                let len_val = self
                    .builder
                    .build_extract_value(sv, 0, "struct.len")
                    .map_err(|e| format!("extract failed: {:?}", e))?
                    .into_int_value();
                let f64_ty = self.context.f64_type();
                let len_f = self
                    .builder
                    .build_signed_int_to_float(len_val, f64_ty, "to_f64")
                    .map_err(|e| format!("sitofp failed: {:?}", e))?;
                let r_f = rv.into_float_value();
                let r_f = if r_f.get_type() != f64_ty {
                    self.builder
                        .build_float_ext(r_f, f64_ty, "ext")
                        .map_err(|e| format!("fpext failed: {:?}", e))?
                } else {
                    r_f
                };
                let pred = match op {
                    Operator::Eq => inkwell::FloatPredicate::OEQ,
                    Operator::Ne => inkwell::FloatPredicate::ONE,
                    Operator::Lt => inkwell::FloatPredicate::OLT,
                    Operator::Gt => inkwell::FloatPredicate::OGT,
                    Operator::Le => inkwell::FloatPredicate::OLE,
                    Operator::Ge => inkwell::FloatPredicate::OGE,
                    _ => unreachable!(),
                };
                return Ok(Some(self
                    .builder
                    .build_float_compare(pred, len_f, r_f, "cmp")
                    .map_err(|e| format!("fcmp failed: {:?}", e))?
                    .into()));
            }
            if rv.is_struct_value() && lv.is_float_value() && is_len_ptr_struct(&rv) {
                let sv = rv.into_struct_value();
                let len_val = self
                    .builder
                    .build_extract_value(sv, 0, "struct.len")
                    .map_err(|e| format!("extract failed: {:?}", e))?
                    .into_int_value();
                let f64_ty = self.context.f64_type();
                let len_f = self
                    .builder
                    .build_signed_int_to_float(len_val, f64_ty, "to_f64")
                    .map_err(|e| format!("sitofp failed: {:?}", e))?;
                let l_f = lv.into_float_value();
                let l_f = if l_f.get_type() != f64_ty {
                    self.builder
                        .build_float_ext(l_f, f64_ty, "ext")
                        .map_err(|e| format!("fpext failed: {:?}", e))?
                } else {
                    l_f
                };
                let pred = match op {
                    Operator::Eq => inkwell::FloatPredicate::OEQ,
                    Operator::Ne => inkwell::FloatPredicate::ONE,
                    Operator::Lt => inkwell::FloatPredicate::OGT,
                    Operator::Gt => inkwell::FloatPredicate::OLT,
                    Operator::Le => inkwell::FloatPredicate::OGE,
                    Operator::Ge => inkwell::FloatPredicate::OLE,
                    _ => unreachable!(),
                };
                return Ok(Some(self
                    .builder
                    .build_float_compare(pred, l_f, len_f, "cmp")
                    .map_err(|e| format!("fcmp failed: {:?}", e))?
                    .into()));
            }

            // Struct vs Struct (non-string or same type)
            if lv.is_struct_value()
                && rv.is_struct_value()
                && is_len_ptr_struct(&lv)
                && is_len_ptr_struct(&rv)
            {
                let ls = lv.into_struct_value();
                let rs = rv.into_struct_value();
                if ls.get_type() == rs.get_type() {
                    let l_len = self
                        .builder
                        .build_extract_value(ls, 0, "l.len")
                        .map_err(|e| format!("extract: {:?}", e))?;
                    let r_len = self
                        .builder
                        .build_extract_value(rs, 0, "r.len")
                        .map_err(|e| format!("extract: {:?}", e))?;
                    let l_ptr = self
                        .builder
                        .build_extract_value(ls, 1, "l.ptr")
                        .map_err(|e| format!("extract: {:?}", e))?
                        .into_pointer_value();
                    let r_ptr = self
                        .builder
                        .build_extract_value(rs, 1, "r.ptr")
                        .map_err(|e| format!("extract: {:?}", e))?
                        .into_pointer_value();
                    match op {
                        Operator::Eq | Operator::Ne => {
                            let eq = self
                                .builder
                                .build_and(
                                    self.builder
                                        .build_int_compare(
                                            inkwell::IntPredicate::EQ,
                                            l_len.into_int_value(),
                                            r_len.into_int_value(),
                                            "leq",
                                        )
                                        .map_err(|e| format!("icmp: {:?}", e))?,
                                    self.builder
                                        .build_int_compare(
                                            inkwell::IntPredicate::EQ,
                                            l_ptr,
                                            r_ptr,
                                            "peq",
                                        )
                                        .map_err(|e| format!("icmp: {:?}", e))?,
                                    "eq",
                                )
                                .map_err(|e| format!("and: {:?}", e))?;
                            return Ok(Some(if *op == Operator::Eq {
                                eq
                            } else {
                                self.builder
                                    .build_not(eq, "ne")
                                    .map_err(|e| format!("not: {:?}", e))?
                            }
                            .into()));
                        }
                        _ => {
                            let (l, r) = if matches!(op, Operator::Lt | Operator::Le) {
                                (l_len.into_int_value(), r_len.into_int_value())
                            } else {
                                (r_len.into_int_value(), l_len.into_int_value())
                            };
                            let pred = match op {
                                Operator::Lt | Operator::Gt => inkwell::IntPredicate::ULT,
                                _ => inkwell::IntPredicate::ULE,
                            };
                            return Ok(Some(self
                                .builder
                                .build_int_compare(pred, l, r, "cmp")
                                .map_err(|e| format!("icmp: {:?}", e))?
                                .into()));
                        }
                    }
                }
                let l_ptr = self
                    .builder
                    .build_extract_value(ls, 1, "l.ptr")
                    .map_err(|e| format!("extract: {:?}", e))?
                    .into_pointer_value();
                let r_ptr = self
                    .builder
                    .build_extract_value(rs, 1, "r.ptr")
                    .map_err(|e| format!("extract: {:?}", e))?
                    .into_pointer_value();
                let pred = match op {
                    Operator::Eq => inkwell::IntPredicate::EQ,
                    Operator::Ne => inkwell::IntPredicate::NE,
                    _ => return Err("unsupported op for different struct types".into()),
                };
                return Ok(Some(self
                    .builder
                    .build_int_compare(pred, l_ptr, r_ptr, "cmp")
                    .map_err(|e| format!("icmp: {:?}", e))?
                    .into()));
            }
        }

        // Catch-all: two identical {i64, ptr} structs that weren't caught by the
        // string-specific or dedicated struct handlers above. Compare field 0 (i64)
        // for ordering operators; compare both fields for Eq/Ne.
        if is_comparison && lv.is_struct_value() && rv.is_struct_value() {
            let lsv = lv.into_struct_value();
            let rsv = rv.into_struct_value();
            if lsv.get_type() == rsv.get_type() && lsv.get_type().count_fields() == 2 {
                let ll = self
                    .builder
                    .build_extract_value(lsv, 0, "ll")
                    .map_err(|e| format!("ext: {:?}", e))?;
                let rl = self
                    .builder
                    .build_extract_value(rsv, 0, "rl")
                    .map_err(|e| format!("ext: {:?}", e))?;
                match op {
                    Operator::Eq | Operator::Ne => {
                        let lp = self
                            .builder
                            .build_extract_value(lsv, 1, "lp")
                            .map_err(|e| format!("ext: {:?}", e))?
                            .into_pointer_value();
                        let rp = self
                            .builder
                            .build_extract_value(rsv, 1, "rp")
                            .map_err(|e| format!("ext: {:?}", e))?
                            .into_pointer_value();
                        let eq = self
                            .builder
                            .build_and(
                                self.builder
                                    .build_int_compare(
                                        inkwell::IntPredicate::EQ,
                                        ll.into_int_value(),
                                        rl.into_int_value(),
                                        "leq",
                                    )
                                    .map_err(|e| format!("icmp: {:?}", e))?,
                                self.builder
                                    .build_int_compare(inkwell::IntPredicate::EQ, lp, rp, "peq")
                                    .map_err(|e| format!("icmp: {:?}", e))?,
                                "eq",
                            )
                            .map_err(|e| format!("and: {:?}", e))?;
                        return Ok(Some(if *op == Operator::Eq {
                            eq
                        } else {
                            self.builder
                                .build_not(eq, "ne")
                                .map_err(|e| format!("not: {:?}", e))?
                        }
                        .into()));
                    }
                    _ => {
                        let (l, r) = if matches!(op, Operator::Lt | Operator::Le) {
                            (ll.into_int_value(), rl.into_int_value())
                        } else {
                            (rl.into_int_value(), ll.into_int_value())
                        };
                        let pred = match op {
                            Operator::Lt | Operator::Gt => inkwell::IntPredicate::ULT,
                            _ => inkwell::IntPredicate::ULE,
                        };
                        return Ok(Some(self
                            .builder
                            .build_int_compare(pred, l, r, "cmp")
                            .map_err(|e| format!("icmp: {:?}", e))?
                            .into()));
                    }
                }
            }
        }

        // Catch-all: struct vs float — extract i64 from the {i64, ptr} struct, convert to float.
        if is_comparison
            && ((lv.is_struct_value() && rv.is_float_value())
                || (rv.is_struct_value() && lv.is_float_value()))
        {
            let (struct_val, float_val, struct_is_left) = if lv.is_struct_value() {
                (lv, rv, true)
            } else {
                (rv, lv, false)
            };
            let sv = struct_val.into_struct_value();
            if sv.get_type().count_fields() == 2 {
                if let Ok(bv) = self.builder.build_extract_value(sv, 0, "s.len") {
                    let len = bv.into_int_value();
                    let f64 = self.context.f64_type();
                    let lf = self
                        .builder
                        .build_signed_int_to_float(len, f64, "f")
                        .map_err(|e| format!("sitofp: {:?}", e))?;
                    let rf = float_val.into_float_value();
                    let rf = if rf.get_type() != f64 {
                        self.builder
                            .build_float_ext(rf, f64, "ext")
                            .map_err(|e| format!("fpext: {:?}", e))?
                    } else {
                        rf
                    };
                    let fpred = match op {
                        Operator::Eq => inkwell::FloatPredicate::OEQ,
                        Operator::Ne => inkwell::FloatPredicate::ONE,
                        Operator::Lt => inkwell::FloatPredicate::OLT,
                        Operator::Gt => inkwell::FloatPredicate::OGT,
                        Operator::Le => inkwell::FloatPredicate::OLE,
                        Operator::Ge => inkwell::FloatPredicate::OGE,
                        _ => unreachable!(),
                    };
                    let (lf2, rf2) = if struct_is_left { (lf, rf) } else { (rf, lf) };
                    let fpred = if struct_is_left {
                        fpred
                    } else {
                        match op {
                            Operator::Eq => inkwell::FloatPredicate::OEQ,
                            Operator::Ne => inkwell::FloatPredicate::ONE,
                            Operator::Lt => inkwell::FloatPredicate::OGT,
                            Operator::Gt => inkwell::FloatPredicate::OLT,
                            Operator::Le => inkwell::FloatPredicate::OGE,
                            Operator::Ge => inkwell::FloatPredicate::OLE,
                            _ => fpred,
                        }
                    };
                    return Ok(Some(self
                        .builder
                        .build_float_compare(fpred, lf2, rf2, "cmp")
                        .map_err(|e| format!("fcmp: {:?}", e))?
                        .into()));
                }
            }
        }

        // Final fallback: generic struct-vs-int or struct-vs-struct for {i64, ptr} types
        // that weren't caught by the string-specific or dedicated struct handlers above.
        if is_comparison && (lv.is_struct_value() || rv.is_struct_value()) {
            let pred_i = |o: &Operator| match o {
                Operator::Eq => inkwell::IntPredicate::EQ,
                Operator::Ne => inkwell::IntPredicate::NE,
                Operator::Lt => inkwell::IntPredicate::SLT,
                Operator::Gt => inkwell::IntPredicate::SGT,
                Operator::Le => inkwell::IntPredicate::SLE,
                Operator::Ge => inkwell::IntPredicate::SGE,
                _ => unreachable!(),
            };

            // Struct vs Int: extract i64 length from {i64, ptr} and compare.
            if lv.is_struct_value() && rv.is_int_value() {
                if let BasicValueEnum::StructValue(sv) = lv {
                    if sv.get_type().count_fields() == 2 {
                        if let Ok(bv) = self.builder.build_extract_value(sv, 0, "s.len") {
                            let len = bv.into_int_value();
                            let (l, r) = self.coerce_binary_operands(len.into(), rv)?;
                            return Ok(Some(self
                                .builder
                                .build_int_compare(
                                    pred_i(op),
                                    l.into_int_value(),
                                    r.into_int_value(),
                                    "cmp",
                                )
                                .map_err(|e| format!("icmp: {:?}", e))?
                                .into()));
                        }
                    }
                }
            }
            // Int vs Struct: swap and flip comparison.
            if rv.is_struct_value() && lv.is_int_value() {
                if let BasicValueEnum::StructValue(sv) = rv {
                    if sv.get_type().count_fields() == 2 {
                        if let Ok(bv) = self.builder.build_extract_value(sv, 0, "s.len") {
                            let len = bv.into_int_value();
                            let (l, r) = self.coerce_binary_operands(lv, len.into())?;
                            let sop = match op {
                                Operator::Lt => Operator::Gt,
                                Operator::Gt => Operator::Lt,
                                Operator::Le => Operator::Ge,
                                Operator::Ge => Operator::Le,
                                x => x.clone(),
                            };
                            return Ok(Some(self
                                .builder
                                .build_int_compare(
                                    pred_i(&sop),
                                    l.into_int_value(),
                                    r.into_int_value(),
                                    "cmp",
                                )
                                .map_err(|e| format!("icmp: {:?}", e))?
                                .into()));
                        }
                    }
                }
            }
            // Struct vs Float: convert length to float and compare.
            if lv.is_struct_value() && rv.is_float_value() {
                if let BasicValueEnum::StructValue(sv) = lv {
                    if sv.get_type().count_fields() == 2 {
                        if let Ok(bv) = self.builder.build_extract_value(sv, 0, "s.len") {
                            let len = bv.into_int_value();
                            let f64 = self.context.f64_type();
                            let lf = self
                                .builder
                                .build_signed_int_to_float(len, f64, "f")
                                .map_err(|e| format!("sitofp: {:?}", e))?;
                            let rf = rv.into_float_value();
                            let rf = if rf.get_type() != f64 {
                                self.builder
                                    .build_float_ext(rf, f64, "ext")
                                    .map_err(|e| format!("fpext: {:?}", e))?
                            } else {
                                rf
                            };
                            let fpred = match op {
                                Operator::Eq => inkwell::FloatPredicate::OEQ,
                                Operator::Ne => inkwell::FloatPredicate::ONE,
                                Operator::Lt => inkwell::FloatPredicate::OLT,
                                Operator::Gt => inkwell::FloatPredicate::OGT,
                                Operator::Le => inkwell::FloatPredicate::OLE,
                                Operator::Ge => inkwell::FloatPredicate::OGE,
                                _ => unreachable!(),
                            };
                            return Ok(Some(self
                                .builder
                                .build_float_compare(fpred, lf, rf, "cmp")
                                .map_err(|e| format!("fcmp: {:?}", e))?
                                .into()));
                        }
                    }
                }
            }
            if rv.is_struct_value() && lv.is_float_value() {
                if let BasicValueEnum::StructValue(sv) = rv {
                    if sv.get_type().count_fields() == 2 {
                        if let Ok(bv) = self.builder.build_extract_value(sv, 0, "s.len") {
                            let len = bv.into_int_value();
                            let f64 = self.context.f64_type();
                            let lf = lv.into_float_value();
                            let lf = if lf.get_type() != f64 {
                                self.builder
                                    .build_float_ext(lf, f64, "ext")
                                    .map_err(|e| format!("fpext: {:?}", e))?
                            } else {
                                lf
                            };
                            let rf = self
                                .builder
                                .build_signed_int_to_float(len, f64, "f")
                                .map_err(|e| format!("sitofp: {:?}", e))?;
                            let fpred = match op {
                                Operator::Eq => inkwell::FloatPredicate::OEQ,
                                Operator::Ne => inkwell::FloatPredicate::ONE,
                                Operator::Lt => inkwell::FloatPredicate::OGT,
                                Operator::Gt => inkwell::FloatPredicate::OLT,
                                Operator::Le => inkwell::FloatPredicate::OGE,
                                Operator::Ge => inkwell::FloatPredicate::OLE,
                                _ => unreachable!(),
                            };
                            return Ok(Some(self
                                .builder
                                .build_float_compare(fpred, lf, rf, "cmp")
                                .map_err(|e| format!("fcmp: {:?}", e))?
                                .into()));
                        }
                    }
                }
            }
            // Struct vs Pointer (null check): extract data pointer and compare.
            if lv.is_struct_value() && rv.is_pointer_value() {
                if let BasicValueEnum::StructValue(sv) = lv {
                    if sv.get_type().count_fields() == 2 {
                        if let Ok(bv) = self.builder.build_extract_value(sv, 1, "s.ptr") {
                            let dp = bv.into_pointer_value();
                            let pred = match op {
                                Operator::Eq => inkwell::IntPredicate::EQ,
                                Operator::Ne => inkwell::IntPredicate::NE,
                                _ => return Err("unsupported op for struct vs pointer".into()),
                            };
                            return Ok(Some(self
                                .builder
                                .build_int_compare(pred, dp, rv.into_pointer_value(), "cmp")
                                .map_err(|e| format!("icmp: {:?}", e))?
                                .into()));
                        }
                    }
                }
            }
            if rv.is_struct_value() && lv.is_pointer_value() {
                if let BasicValueEnum::StructValue(sv) = rv {
                    if sv.get_type().count_fields() == 2 {
                        if let Ok(bv) = self.builder.build_extract_value(sv, 1, "s.ptr") {
                            let dp = bv.into_pointer_value();
                            let pred = match op {
                                Operator::Eq => inkwell::IntPredicate::EQ,
                                Operator::Ne => inkwell::IntPredicate::NE,
                                _ => return Err("unsupported op for pointer vs struct".into()),
                            };
                            return Ok(Some(self
                                .builder
                                .build_int_compare(pred, lv.into_pointer_value(), dp, "cmp")
                                .map_err(|e| format!("icmp: {:?}", e))?
                                .into()));
                        }
                    }
                }
            }
            // Struct vs Struct: same type → compare length fields; different → pointer Eq/Ne.
            if lv.is_struct_value() && rv.is_struct_value() {
                let ls = lv.into_struct_value();
                let rs = rv.into_struct_value();
                if ls.get_type() == rs.get_type() && ls.get_type().count_fields() == 2 {
                    let ll = self
                        .builder
                        .build_extract_value(ls, 0, "ll")
                        .map_err(|e| format!("ext: {:?}", e))?
                        .into_int_value();
                    let rl = self
                        .builder
                        .build_extract_value(rs, 0, "rl")
                        .map_err(|e| format!("ext: {:?}", e))?
                        .into_int_value();
                    let lp = self
                        .builder
                        .build_extract_value(ls, 1, "lp")
                        .map_err(|e| format!("ext: {:?}", e))?
                        .into_pointer_value();
                    let rp = self
                        .builder
                        .build_extract_value(rs, 1, "rp")
                        .map_err(|e| format!("ext: {:?}", e))?
                        .into_pointer_value();
                    match op {
                        Operator::Eq | Operator::Ne => {
                            let eq = self
                                .builder
                                .build_and(
                                    self.builder
                                        .build_int_compare(inkwell::IntPredicate::EQ, ll, rl, "leq")
                                        .map_err(|e| format!("icmp: {:?}", e))?,
                                    self.builder
                                        .build_int_compare(inkwell::IntPredicate::EQ, lp, rp, "peq")
                                        .map_err(|e| format!("icmp: {:?}", e))?,
                                    "eq",
                                )
                                .map_err(|e| format!("and: {:?}", e))?;
                            return Ok(Some(if *op == Operator::Eq {
                                eq
                            } else {
                                self.builder
                                    .build_not(eq, "ne")
                                    .map_err(|e| format!("not: {:?}", e))?
                            }
                            .into()));
                        }
                        _ => {
                            let (l, r) = if matches!(op, Operator::Lt | Operator::Le) {
                                (ll, rl)
                            } else {
                                (rl, ll)
                            };
                            let pred = match op {
                                Operator::Lt | Operator::Gt => inkwell::IntPredicate::ULT,
                                _ => inkwell::IntPredicate::ULE,
                            };
                            return Ok(Some(self
                                .builder
                                .build_int_compare(pred, l, r, "cmp")
                                .map_err(|e| format!("icmp: {:?}", e))?
                                .into()));
                        }
                    }
                }
                // Different struct types: compare data pointers for Eq/Ne.
                if ls.get_type().count_fields() >= 2 && rs.get_type().count_fields() >= 2 {
                    if let (Ok(bvl), Ok(bvr)) = (
                        self.builder.build_extract_value(ls, 1, "lp"),
                        self.builder.build_extract_value(rs, 1, "rp"),
                    ) {
                        let lp = bvl.into_pointer_value();
                        let rp = bvr.into_pointer_value();
                        let pred = match op {
                            Operator::Eq => inkwell::IntPredicate::EQ,
                            Operator::Ne => inkwell::IntPredicate::NE,
                            _ => return Err("unsupported op for different struct types".into()),
                        };
                        return Ok(Some(self
                            .builder
                            .build_int_compare(pred, lp, rp, "cmp")
                            .map_err(|e| format!("icmp: {:?}", e))?
                            .into()));
                    }
                }
            }
        }
        Ok(None)
    }
}
