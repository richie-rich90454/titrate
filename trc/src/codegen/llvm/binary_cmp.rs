// Binary comparison lowering (strings, structs, numerics)

use super::{LlvmBackend};
use crate::ast::Operator;
use inkwell::module::Linkage;
use inkwell::values::BasicValueEnum;

impl<'ctx> LlvmBackend<'ctx> {
    /// Declare (once) the C `memcmp(i8*, i8*, i64) -> i32` used for string
    /// content comparisons.
    pub(super) fn declare_memcmp(&mut self) -> inkwell::values::FunctionValue<'ctx> {
        let memcmp_fn_name = "memcmp";
        if let Some(f) = self.module.get_function(memcmp_fn_name) {
            return f;
        }
        let i32_ty = self.context.i32_type();
        let i8_ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
        let fn_type = i32_ty.fn_type(
            &[
                i8_ptr_ty.into(),
                i8_ptr_ty.into(),
                self.context.i64_type().into(),
            ],
            false,
        );
        self.module.add_function(
            memcmp_fn_name,
            fn_type,
            Some(inkwell::module::Linkage::External),
        )
    }

    pub(super) fn compile_binary_str_cmp(&mut self, op: &Operator, lv: BasicValueEnum<'ctx>, rv: BasicValueEnum<'ctx>, is_str_cmp: bool) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        if is_str_cmp {
            // Mixed struct vs non-struct: extract length from struct and compare
            if (lv.is_struct_value() && !rv.is_struct_value())
                || (!lv.is_struct_value() && rv.is_struct_value())
            {
                let (struct_val, other_val, struct_is_left) = if lv.is_struct_value() {
                    (lv, rv, true)
                } else {
                    (rv, lv, false)
                };
                let sv = struct_val.into_struct_value();
                if sv.get_type().count_fields() == 2 {
                    let len_val = self
                        .builder
                        .build_extract_value(sv, 0, "s.len")
                        .map_err(|e| format!("extract s.len failed: {:?}", e))?;
                    if other_val.is_int_value() {
                        let (l, r) = if struct_is_left {
                            self.coerce_binary_operands(len_val, other_val)?
                        } else {
                            self.coerce_binary_operands(other_val, len_val)?
                        };
                        let pred = match op {
                            Operator::Eq => inkwell::IntPredicate::EQ,
                            Operator::Ne => inkwell::IntPredicate::NE,
                            Operator::Lt => inkwell::IntPredicate::SLT,
                            Operator::Gt => inkwell::IntPredicate::SGT,
                            Operator::Le => inkwell::IntPredicate::SLE,
                            Operator::Ge => inkwell::IntPredicate::SGE,
                            _ => unreachable!(),
                        };
                        let (l, r) = if struct_is_left { (l, r) } else { (r, l) };
                        let pred = if struct_is_left {
                            pred
                        } else {
                            match op {
                                Operator::Lt => inkwell::IntPredicate::SGT,
                                Operator::Gt => inkwell::IntPredicate::SLT,
                                Operator::Le => inkwell::IntPredicate::SGE,
                                Operator::Ge => inkwell::IntPredicate::SLE,
                                _ => pred,
                            }
                        };
                        return Ok(Some(self
                            .builder
                            .build_int_compare(pred, l.into_int_value(), r.into_int_value(), "cmp")
                            .map_err(|e| format!("icmp: {:?}", e))?
                            .into()));
                    }
                    if other_val.is_float_value() {
                        let f64_ty = self.context.f64_type();
                        let len_f = self
                            .builder
                            .build_signed_int_to_float(len_val.into_int_value(), f64_ty, "to_f64")
                            .map_err(|e| format!("sitofp: {:?}", e))?;
                        let of = other_val.into_float_value();
                        let of = if of.get_type() != f64_ty {
                            self.builder
                                .build_float_ext(of, f64_ty, "ext")
                                .map_err(|e| format!("fpext: {:?}", e))?
                        } else {
                            of
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
                        let (lf, rf) = if struct_is_left {
                            (len_f, of)
                        } else {
                            (of, len_f)
                        };
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
                            .build_float_compare(fpred, lf, rf, "cmp")
                            .map_err(|e| format!("fcmp: {:?}", e))?
                            .into()));
                    }
                    if other_val.is_pointer_value() {
                        let data_ptr = self
                            .builder
                            .build_extract_value(sv, 1, "s.ptr")
                            .map_err(|e| format!("extract s.ptr failed: {:?}", e))?
                            .into_pointer_value();
                        let pred = match op {
                            Operator::Eq => inkwell::IntPredicate::EQ,
                            Operator::Ne => inkwell::IntPredicate::NE,
                            _ => return Err("unsupported op for struct vs pointer".into()),
                        };
                        let (lp, rp) = if struct_is_left {
                            (data_ptr, other_val.into_pointer_value())
                        } else {
                            (other_val.into_pointer_value(), data_ptr)
                        };
                        return Ok(Some(self
                            .builder
                            .build_int_compare(pred, lp, rp, "cmp")
                            .map_err(|e| format!("icmp: {:?}", e))?
                            .into()));
                    }
                }
            }
            if lv.is_struct_value() && rv.is_struct_value() {
                let ls = lv.into_struct_value();
                let rs = rv.into_struct_value();
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
                    Operator::Eq | Operator::Ne => {
                        let len_eq = self
                            .builder
                            .build_int_compare(
                                inkwell::IntPredicate::EQ,
                                l_len.into_int_value(),
                                r_len.into_int_value(),
                                "len.eq",
                            )
                            .map_err(|e| format!("build_int_compare len failed: {:?}", e))?;
                        // Content equality via memcmp: distinct buffers
                        // with identical bytes must compare equal. Compare
                        // only min_len bytes so a shorter buffer is never
                        // over-read; length equality decides the rest.
                        let memcmp_fn = self.declare_memcmp();
                        let l_len_int = l_len.into_int_value();
                        let r_le_l = self
                            .builder
                            .build_int_compare(
                                inkwell::IntPredicate::ULE,
                                l_len_int,
                                r_len.into_int_value(),
                                "min.cond",
                            )
                            .map_err(|e| format!("build_int_compare failed: {:?}", e))?;
                        let min_len = self
                            .builder
                            .build_select(
                                r_le_l,
                                l_len_int,
                                r_len.into_int_value(),
                                "min.len",
                            )
                            .map_err(|e| format!("build_select failed: {:?}", e))?;
                        let cmp_result = self
                            .builder
                            .build_call(
                                memcmp_fn,
                                &[l_ptr.into(), r_ptr.into(), min_len.into()],
                                "memcmp",
                            )
                            .map_err(|e| format!("build_call memcmp failed: {:?}", e))?;
                        let content_eq = match cmp_result.try_as_basic_value() {
                            inkwell::values::ValueKind::Basic(basic_val) => {
                                let i32_ty = self.context.i32_type();
                                self.builder
                                    .build_int_compare(
                                        inkwell::IntPredicate::EQ,
                                        basic_val.into_int_value(),
                                        i32_ty.const_int(0, true),
                                        "content.eq",
                                    )
                                    .map_err(|e| {
                                        format!("build_int_compare failed: {:?}", e)
                                    })?
                            }
                            _ => {
                                return Err(
                                    "codegen: memcmp did not return a value".to_string()
                                )
                            }
                        };
                        let result = if *op == Operator::Eq {
                            self.builder.build_and(len_eq, content_eq, "str.eq")
                        } else {
                            let eq_val = self
                                .builder
                                .build_and(len_eq, content_eq, "str.eq.tmp")
                                .map_err(|e| format!("build_and failed: {:?}", e))?;
                            self.builder.build_not(eq_val, "str.ne")
                        }
                        .map_err(|e| format!("build logic failed: {:?}", e))?;
                        return Ok(Some(result.into()));
                    }
                    _ => {
                        // Ordering comparisons: use memcmp
                        let i32_ty = self.context.i32_type();
                        let i8_ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
                        // Declare memcmp(i8*, i8*, i64) -> i32
                        let memcmp_fn_name = "memcmp";
                        let memcmp_fn = if let Some(f) = self.module.get_function(memcmp_fn_name) {
                            f
                        } else {
                            let fn_type = i32_ty.fn_type(
                                &[
                                    i8_ptr_ty.into(),
                                    i8_ptr_ty.into(),
                                    self.context.i64_type().into(),
                                ],
                                false,
                            );
                            self.module.add_function(
                                memcmp_fn_name,
                                fn_type,
                                Some(Linkage::External),
                            )
                        };
                        // min(l_len, r_len) using select: min = r_len <= l_len ? r_len : l_len
                        let r_le_l = self
                            .builder
                            .build_int_compare(
                                inkwell::IntPredicate::ULE,
                                l_len.into_int_value(),
                                r_len.into_int_value(),
                                "r.le.l",
                            )
                            .map_err(|e| format!("build_int_compare failed: {:?}", e))?;
                        let min_len = self
                            .builder
                            .build_select(
                                r_le_l,
                                l_len.into_int_value(),
                                r_len.into_int_value(),
                                "min.len",
                            )
                            .map_err(|e| format!("build_select failed: {:?}", e))?;
                        let cmp_result = self
                            .builder
                            .build_call(
                                memcmp_fn,
                                &[l_ptr.into(), r_ptr.into(), min_len.into_int_value().into()],
                                "memcmp",
                            )
                            .map_err(|e| format!("build_call memcmp failed: {:?}", e))?;
                        if let inkwell::values::ValueKind::Basic(basic_val) =
                            cmp_result.try_as_basic_value()
                        {
                            let cmp_int = basic_val.into_int_value();
                            let zero = i32_ty.const_int(0, true);
                            // If memcmp returned 0, strings are equal up to min_len; compare lengths
                            let is_zero = self
                                .builder
                                .build_int_compare(
                                    inkwell::IntPredicate::EQ,
                                    cmp_int,
                                    zero,
                                    "cmp.zero",
                                )
                                .map_err(|e| format!("build_int_compare failed: {:?}", e))?;
                            let len_cmp = match *op {
                                Operator::Lt => self.builder.build_int_compare(
                                    inkwell::IntPredicate::ULT,
                                    l_len.into_int_value(),
                                    r_len.into_int_value(),
                                    "len.lt",
                                ),
                                Operator::Gt => self.builder.build_int_compare(
                                    inkwell::IntPredicate::ULT,
                                    r_len.into_int_value(),
                                    l_len.into_int_value(),
                                    "len.gt",
                                ),
                                Operator::Le => self.builder.build_int_compare(
                                    inkwell::IntPredicate::ULE,
                                    l_len.into_int_value(),
                                    r_len.into_int_value(),
                                    "len.le",
                                ),
                                Operator::Ge => self.builder.build_int_compare(
                                    inkwell::IntPredicate::ULE,
                                    r_len.into_int_value(),
                                    l_len.into_int_value(),
                                    "len.ge",
                                ),
                                _ => unreachable!(),
                            }
                            .map_err(|e| format!("build_int_compare len failed: {:?}", e))?;
                            let memcmp_lt = match *op {
                                Operator::Lt => self.builder.build_int_compare(
                                    inkwell::IntPredicate::SLT,
                                    cmp_int,
                                    zero,
                                    "cmp.lt",
                                ),
                                Operator::Gt => self.builder.build_int_compare(
                                    inkwell::IntPredicate::SGT,
                                    cmp_int,
                                    zero,
                                    "cmp.gt",
                                ),
                                Operator::Le => self.builder.build_int_compare(
                                    inkwell::IntPredicate::SLE,
                                    cmp_int,
                                    zero,
                                    "cmp.le",
                                ),
                                Operator::Ge => self.builder.build_int_compare(
                                    inkwell::IntPredicate::SGE,
                                    cmp_int,
                                    zero,
                                    "cmp.ge",
                                ),
                                _ => unreachable!(),
                            }
                            .map_err(|e| format!("build_int_compare memcmp failed: {:?}", e))?;
                            // Result = (memcmp != 0 && memcmp matches op) || (memcmp == 0 && len matches op)
                            let nonzero_and_match = self
                                .builder
                                .build_and(
                                    self.builder
                                        .build_not(is_zero, "cmp.nonzero")
                                        .map_err(|e| format!("build_not failed: {:?}", e))?,
                                    memcmp_lt,
                                    "nonzero.match",
                                )
                                .map_err(|e| format!("build_and failed: {:?}", e))?;
                            let result = self
                                .builder
                                .build_or(nonzero_and_match, len_cmp, "str.cmp.result")
                                .map_err(|e| format!("build_or failed: {:?}", e))?;
                            return Ok(Some(result.into()));
                        }
                        return Err("codegen: memcmp did not return a value".to_string());
                    }
                }
            }
        }
        Ok(None)
    }

    pub(super) fn compile_binary_struct_cmp(&mut self, op: &Operator, lv: BasicValueEnum<'ctx>, rv: BasicValueEnum<'ctx>, is_comparison: bool) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        // Handle struct vs other type comparisons before coercion (which can't handle structs).
        if (lv.is_struct_value() || rv.is_struct_value()) && is_comparison {
            let pred_i = |o: &Operator| match o {
                Operator::Eq => inkwell::IntPredicate::EQ,
                Operator::Ne => inkwell::IntPredicate::NE,
                Operator::Lt => inkwell::IntPredicate::SLT,
                Operator::Gt => inkwell::IntPredicate::SGT,
                Operator::Le => inkwell::IntPredicate::SLE,
                Operator::Ge => inkwell::IntPredicate::SGE,
                _ => unreachable!(),
            };
            let fpred = |o: &Operator| match o {
                Operator::Eq => inkwell::FloatPredicate::OEQ,
                Operator::Ne => inkwell::FloatPredicate::ONE,
                Operator::Lt => inkwell::FloatPredicate::OLT,
                Operator::Gt => inkwell::FloatPredicate::OGT,
                Operator::Le => inkwell::FloatPredicate::OLE,
                Operator::Ge => inkwell::FloatPredicate::OGE,
                _ => unreachable!(),
            };
            // Extract i64 length from {i64, ptr} struct
            let extract_len =
                |v: BasicValueEnum<'ctx>| -> Result<inkwell::values::IntValue<'ctx>, String> {
                    let sv = v.into_struct_value();
                    if sv.get_type().count_fields() < 2 {
                        return Err("not a {i64,ptr} struct".into());
                    }
                    self.builder
                        .build_extract_value(sv, 0, "len")
                        .map_err(|e| format!("ext: {:?}", e))
                        .map(|v| v.into_int_value())
                };
            // Struct vs Int
            if lv.is_struct_value() && rv.is_int_value() {
                let len = extract_len(lv)?;
                let (l, r) = self.coerce_binary_operands(len.into(), rv)?;
                return Ok(Some(self
                    .builder
                    .build_int_compare(pred_i(op), l.into_int_value(), r.into_int_value(), "cmp")
                    .map_err(|e| format!("icmp: {:?}", e))?
                    .into()));
            }
            if rv.is_struct_value() && lv.is_int_value() {
                let len = extract_len(rv)?;
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
                    .build_int_compare(pred_i(&sop), l.into_int_value(), r.into_int_value(), "cmp")
                    .map_err(|e| format!("icmp: {:?}", e))?
                    .into()));
            }
            // Struct vs Float
            if lv.is_struct_value() && rv.is_float_value() {
                let len = extract_len(lv)?;
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
                return Ok(Some(self
                    .builder
                    .build_float_compare(fpred(op), lf, rf, "cmp")
                    .map_err(|e| format!("fcmp: {:?}", e))?
                    .into()));
            }
            if rv.is_struct_value() && lv.is_float_value() {
                let len = extract_len(rv)?;
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
                let sop = match op {
                    Operator::Lt => Operator::Gt,
                    Operator::Gt => Operator::Lt,
                    Operator::Le => Operator::Ge,
                    Operator::Ge => Operator::Le,
                    x => x.clone(),
                };
                return Ok(Some(self
                    .builder
                    .build_float_compare(fpred(&sop), lf, rf, "cmp")
                    .map_err(|e| format!("fcmp: {:?}", e))?
                    .into()));
            }
            // Struct vs Pointer (null check)
            if lv.is_struct_value() && rv.is_pointer_value() {
                let sv = lv.into_struct_value();
                let dp = self
                    .builder
                    .build_extract_value(sv, 1, "dp")
                    .map_err(|e| format!("ext: {:?}", e))?
                    .into_pointer_value();
                return Ok(Some(self
                    .builder
                    .build_int_compare(pred_i(op), dp, rv.into_pointer_value(), "cmp")
                    .map_err(|e| format!("icmp: {:?}", e))?
                    .into()));
            }
            if rv.is_struct_value() && lv.is_pointer_value() {
                let sv = rv.into_struct_value();
                let dp = self
                    .builder
                    .build_extract_value(sv, 1, "dp")
                    .map_err(|e| format!("ext: {:?}", e))?
                    .into_pointer_value();
                return Ok(Some(self
                    .builder
                    .build_int_compare(pred_i(op), lv.into_pointer_value(), dp, "cmp")
                    .map_err(|e| format!("icmp: {:?}", e))?
                    .into()));
            }
            // Struct vs Struct
            if lv.is_struct_value() && rv.is_struct_value() {
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
                    match op {
                        Operator::Eq | Operator::Ne => {
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
                // Different struct types: compare data pointers
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
                let pred = match op {
                    Operator::Eq => inkwell::IntPredicate::EQ,
                    Operator::Ne => inkwell::IntPredicate::NE,
                    _ => return Err("unsupported".into()),
                };
                return Ok(Some(self
                    .builder
                    .build_int_compare(pred, lp, rp, "cmp")
                    .map_err(|e| format!("icmp: {:?}", e))?
                    .into()));
            }
        }
        Ok(None)
    }
}
