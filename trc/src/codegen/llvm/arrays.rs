// Array builtins

use super::native_bridge;
use super::types as llvm_types;
use super::LlvmBackend;
use inkwell::module::Linkage;
use crate::ast::{ClassMember, Expr, Type};
use inkwell::AddressSpace;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::BasicValueEnum;

impl<'ctx> LlvmBackend<'ctx> {
    /// Compile `array.length()` - extracts the i64 length from a TitrateArray struct and truncates to i32.
    pub(super) fn compile_array_length(&mut self, array_expr: &Expr) -> Result<BasicValueEnum<'ctx>, String> {
        let arr_val = self.compile_expr(array_expr)?;
        if arr_val.is_struct_value() {
            let sv = arr_val.into_struct_value();
            let len_val = self
                .builder
                .build_extract_value(sv, 0, "arr.len")
                .map_err(|e| format!("extract arr.len failed: {:?}", e))?;
            let i32_val = self
                .builder
                .build_int_truncate(
                    len_val.into_int_value(),
                    self.context.i32_type(),
                    "arr.len.i32",
                )
                .map_err(|e| format!("int_truncate failed: {:?}", e))?;
            return Ok(i32_val.into());
        }
        Err("codegen: array_length called on non-array value".to_string())
    }

    /// Full element type of an array expression (the `T` in `ArrayList<T>`),
    /// preserving generic arguments for nested containers.
    pub(super) fn array_element_type(&self, array_expr: &Expr) -> Option<Type> {
        match array_expr {
            Expr::Identifier(name, _) => self
                .locals
                .get(name)
                .and_then(|v| v.full_type.as_ref())
                .and_then(|t| t.params().first().cloned()),
            Expr::MemberAccess(obj, field, _) => {
                let obj_ty = self.infer_expr_type(obj);
                let class_name = obj_ty.name();
                self.class_decls
                    .get(class_name)
                    .and_then(|cd| {
                        cd.members.iter().find_map(|m| match m {
                            ClassMember::Field(f) if f.name == *field => Some(f.typ.clone()),
                            _ => None,
                        })
                    })
                    .and_then(|t| t.params().first().cloned())
            }
            // A chained `list.get(i)` returning a container: use its inferred
            // element type (e.g. `centroids.get(i).get(0)`).
            Expr::Call(callee, _, _) => {
                if matches!(&**callee, Expr::MemberAccess(_, m, _) if m == "get") {
                    self.infer_expr_type(array_expr).params().first().cloned()
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Determine the element type name of an array expression (e.g. the `T`
    /// in `ArrayList<T>`), for locals and `obj.field` accesses.
    pub(super) fn array_element_type_name(&self, array_expr: &Expr) -> Option<String> {
        self.array_element_type(array_expr).map(|t| t.name().to_string())
    }

    /// Compile `array.get(index)` - reads an element from a TitrateArray
    /// by calling the appropriate titrate_array_get_* function based on element type.
    pub(super) fn compile_array_get_string(
        &mut self,
        array_expr: &Expr,
        index_expr: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        // Determine the element type from the array variable's full_type.
        let element_type_name = self.array_element_type_name(array_expr);

        // Bool elements are tagged TV_BOOL in the runtime, which the generic
        // int accessor does not accept. Read the TitrateValue payload directly.
        if element_type_name.as_deref() == Some("bool") {
            return self.compile_array_get_bool(array_expr, index_expr);
        }

        // Container elements (e.g. ArrayList<ArrayList<T>>) are stored as a
        // TitrateArray payload inside TitrateValue. The string accessor rejects
        // them (tag != TV_STRING), so read the raw `{ i64, ptr }` payload.
        if let Some(elem) = element_type_name.as_deref() {
            let primitive = matches!(elem,
                "int" | "u8" | "u16" | "u32" | "byte" | "short" | "char"
                | "long" | "u64" | "size" | "float" | "double" | "half" | "quad"
                | "string");
            if !primitive {
                // User-class instances are marshalled as a raw object pointer in
                // the first 8 bytes of the TitrateValue payload. Hand it back as
                // a pointer so field access and method dispatch work as usual.
                // JsonValue is excluded: its payload is a native handle id.
                if self.class_infos.contains_key(elem) && elem != "JsonValue" {
                    return self.compile_array_get_object_ptr(array_expr, index_expr);
                }
                // Nested containers store a `{ i64 len, ptr data }` payload.
                return self.compile_array_get_raw_payload(array_expr, index_expr);
            }
        }

        // Choose the right native function based on element type.
        let (fn_name, return_is_struct): (&str, bool) = match element_type_name.as_deref() {
            Some("int") | Some("u32") | Some("byte") | Some("short") | Some("u8") | Some("u16")
            | Some("bool") | Some("char") => {
                ("titrate_array_get_int", false)
            }
            Some("long") | Some("u64") | Some("size") => ("titrate_array_get_long", false),
            Some("double") | Some("float") | Some("half") | Some("quad") => {
                ("titrate_array_get_double", false)
            }
            _ => ("titrate_array_get_string", true),
        };
        let arr_val = self.compile_expr(array_expr)?;
        let idx_val = self.compile_expr(index_expr)?;

        if !arr_val.is_struct_value() {
            return Err("codegen: array_get_string called on non-array value".to_string());
        }

        let sv = arr_val.into_struct_value();
        let len_val = self
            .builder
            .build_extract_value(sv, 0, "arr.len")
            .map_err(|e| format!("extract arr.len failed: {:?}", e))?;
        let data_ptr = self
            .builder
            .build_extract_value(sv, 1, "arr.data")
            .map_err(|e| format!("extract arr.data failed: {:?}", e))?;

        let idx_i64 = if idx_val.is_int_value() {
            let idx_int = idx_val.into_int_value();
            if idx_int.get_type().get_bit_width() < 64 {
                self.builder
                    .build_int_z_extend(idx_int, self.context.i64_type(), "idx.ext")
                    .map_err(|e| format!("int_z_extend failed: {:?}", e))?
            } else {
                idx_int
            }
        } else {
            return Err("ArrayList.get: index must be integer".to_string());
        };

        let i64_ty = self.context.i64_type();
        let i8_ptr = self.context.ptr_type(AddressSpace::default());
        let arr_c_abi_ty = self
            .context
            .struct_type(&[i64_ty.into(), i8_ptr.into()], false);

        let arr_struct_val = self
            .builder
            .build_insert_value(
                self.builder
                    .build_insert_value(
                        arr_c_abi_ty.const_zero(),
                        len_val.into_int_value(),
                        0,
                        "arr.len",
                    )
                    .map_err(|e| format!("insert_value failed: {:?}", e))?,
                data_ptr.into_pointer_value(),
                1,
                "arr.data",
            )
            .map_err(|e| format!("insert_value failed: {:?}", e))?;

        let arr_struct = match arr_struct_val {
            inkwell::values::AggregateValueEnum::StructValue(sv) => sv,
            _ => return Err("expected struct value from insert_value".to_string()),
        };

        if return_is_struct {
            // titrate_array_get_string(arr, index, out): the string comes back
            // through `out` (struct returns disagree across the FFI).
            let string_ty = llvm_types::string_type(self.context).into_struct_type();
            let void_ty = self.context.void_type();
            let fn_type =
                void_ty.fn_type(&[i8_ptr.into(), i64_ty.into(), i8_ptr.into()], false);
            let fn_val = self.module.get_function(fn_name).unwrap_or_else(|| {
                self.module
                    .add_function(fn_name, fn_type, Some(Linkage::External))
            });
            let arr_ptr = self.store_array_arg_to_ptr(arr_struct)?;
            let out_slot = self
                .builder
                .build_alloca(string_ty, "arr.get.out")
                .map_err(|e| format!("build_alloca arr.get.out failed: {:?}", e))?;
            self.builder
                .build_call(
                    fn_val,
                    &[arr_ptr.into(), idx_i64.into(), out_slot.into()],
                    "array_get_string",
                )
                .map_err(|e| format!("build_call {} failed: {:?}", fn_name, e))?;
            let result_val = self
                .builder
                .build_load(string_ty, out_slot, "arr.get.val")
                .map_err(|e| format!("build_load arr.get.val failed: {:?}", e))?;
            if result_val.get_type() == string_ty.into() {
                Ok(result_val)
            } else {
                self.builder
                    .build_bit_cast(result_val, string_ty, "arr.get.cast")
                    .map_err(|e| format!("bit_cast failed: {:?}", e))
            }
        } else {
            // titrate_array_get_int/long/double(*const TitrateArray, i64) -> scalar
            let ret_ty: BasicTypeEnum<'ctx> = if fn_name == "titrate_array_get_double" {
                self.context.f64_type().into()
            } else {
                i64_ty.into()
            };
            let fn_type = ret_ty.fn_type(&[i8_ptr.into(), i64_ty.into()], false);
            let fn_val = self.module.get_function(fn_name).unwrap_or_else(|| {
                self.module
                    .add_function(fn_name, fn_type, Some(Linkage::External))
            });
            let arr_ptr = self.store_array_arg_to_ptr(arr_struct)?;
            let result = self
                .builder
                .build_call(
                    fn_val,
                    &[arr_ptr.into(), idx_i64.into()],
                    "array_get_val",
                )
                .map_err(|e| format!("build_call {} failed: {:?}", fn_name, e))?;
            match result.try_as_basic_value() {
                inkwell::values::ValueKind::Basic(v) => Ok(v),
                _ => Err("array_get did not return a value".to_string()),
            }
        }
    }

    /// Store a `{ i64, ptr }` TitrateArray struct into a stack slot and return
    /// its address. The runtime array accessors take the array by pointer to
    /// avoid the Win64 by-value struct-passing ABI ambiguity.
    pub(super) fn store_array_arg_to_ptr(
        &mut self,
        arr_struct: inkwell::values::StructValue<'ctx>,
    ) -> Result<inkwell::values::PointerValue<'ctx>, String> {
        let i64_ty = self.context.i64_type();
        let i8_ptr = self.context.ptr_type(AddressSpace::default());
        let arr_ty = self
            .context
            .struct_type(&[i64_ty.into(), i8_ptr.into()], false);
        let slot = self
            .builder
            .build_alloca(arr_ty, "arr.slot")
            .map_err(|e| format!("build_alloca arr.slot failed: {:?}", e))?;
        self.builder
            .build_store(slot, arr_struct)
            .map_err(|e| format!("build_store arr.slot failed: {:?}", e))?;
        Ok(slot)
    }

    /// Lower `ArrayList.subList(from, to)` by copying the raw TitrateValue
    /// slice into a newly allocated array. No `ArrayList_subList` native
    /// exists, and the bytecode VM's implementation is not compiled into the
    /// LLVM module (ArrayList is a builtin container there).
    pub(super) fn compile_sublist(
        &mut self,
        array_expr: &Expr,
        from_expr: &Expr,
        to_expr: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let arr_val = self.compile_expr(array_expr)?;
        let sv = arr_val.into_struct_value();
        let data = self
            .builder
            .build_extract_value(sv, 1, "sublist.data")
            .map_err(|e| format!("sublist data failed: {:?}", e))?
            .into_pointer_value();
        let i64_ty = self.context.i64_type();
        let byte_ty = self.context.i8_type();
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let from_raw = self.compile_expr(from_expr)?;
        let from_val = from_raw
            .into_int_value()
            .get_type()
            .get_bit_width() < 64;
        let from_iv = from_raw.into_int_value();
        let from = if from_val {
            self.builder
                .build_int_z_extend(from_iv, i64_ty, "sublist.from")
                .map_err(|e| format!("sublist extend failed: {:?}", e))?
        } else {
            from_iv
        };
        let to_raw = self.compile_expr(to_expr)?;
        let to_iv = to_raw.into_int_value();
        let to = if to_iv.get_type().get_bit_width() < 64 {
            self.builder
                .build_int_z_extend(to_iv, i64_ty, "sublist.to")
                .map_err(|e| format!("sublist extend failed: {:?}", e))?
        } else {
            to_iv
        };
        let count = self
            .builder
            .build_int_sub(to, from, "sublist.count")
            .map_err(|e| format!("sublist count failed: {:?}", e))?;
        let bytes = self
            .builder
            .build_int_mul(count, i64_ty.const_int(24, false), "sublist.bytes")
            .map_err(|e| format!("sublist bytes failed: {:?}", e))?;
        let malloc_fn = self
            .module
            .get_function("titrate_malloc")
            .ok_or("sublist: titrate_malloc not declared")?;
        let malloc_result = self
            .builder
            .build_call(malloc_fn, &[bytes.into()], "sublist.malloc")
            .map_err(|e| format!("sublist malloc failed: {:?}", e))?;
        let new_data = match malloc_result.try_as_basic_value() {
            inkwell::values::ValueKind::Basic(v) => v.into_pointer_value(),
            _ => return Err("sublist malloc returned no value".to_string()),
        };
        let src_offset = self
            .builder
            .build_int_mul(from, i64_ty.const_int(24, false), "sublist.src.off")
            .map_err(|e| format!("sublist offset failed: {:?}", e))?;
        let src_ptr = unsafe {
            self.builder
                .build_gep(byte_ty, data, &[src_offset], "sublist.src")
        }
        .map_err(|e| format!("sublist src gep failed: {:?}", e))?;
        let memcpy_ty = self
            .context
            .void_type()
            .fn_type(&[ptr_ty.into(), ptr_ty.into(), i64_ty.into(), self.context.bool_type().into()], false);
        let memcpy_fn = if let Some(f) = self.module.get_function("llvm.memcpy.p0.p0.i64") {
            f
        } else {
            self.module
                .add_function("llvm.memcpy.p0.p0.i64", memcpy_ty, Some(Linkage::External))
        };
        self.builder
            .build_call(
                memcpy_fn,
                &[new_data.into(), src_ptr.into(), bytes.into(), self.context.bool_type().const_zero().into()],
                "sublist.copy",
            )
            .map_err(|e| format!("sublist memcpy failed: {:?}", e))?;
        let arr_ty = self.context.struct_type(&[i64_ty.into(), ptr_ty.into()], false);
        let result = self
            .builder
            .build_insert_value(arr_ty.const_zero(), count, 0, "sublist.len")
            .map_err(|e| format!("sublist len failed: {:?}", e))?;
        let result = self
            .builder
            .build_insert_value(result, new_data, 1, "sublist.ptr")
            .map_err(|e| format!("sublist ptr failed: {:?}", e))?;
        match result {
            inkwell::values::AggregateValueEnum::StructValue(sv) => Ok(sv.into()),
            _ => Err("sublist: expected struct result".to_string()),
        }
    }

    /// Generate (once) a `qsort` comparator for TitrateValue arrays. Strings
    /// are compared lexicographically via `memcmp`; other tags compare their
    /// numeric payload as i64.
    fn ensure_sort_helpers(&mut self) -> Result<(), String> {
        let saved_block = self.builder.get_insert_block();
        let result = self.build_sort_helpers();
        if let Some(block) = saved_block {
            self.builder.position_at_end(block);
        }
        result
    }

    fn build_sort_helpers(&mut self) -> Result<(), String> {
        let ptr = self.context.ptr_type(AddressSpace::default());
        let i32 = self.context.i32_type();
        let i64 = self.context.i64_type();
        if self.module.get_function("memcmp").is_none() {
            self.module.add_function(
                "memcmp",
                i32.fn_type(&[ptr.into(), ptr.into(), i64.into()], false),
                Some(Linkage::External),
            );
        }
        if self.module.get_function("qsort").is_none() {
            self.module.add_function(
                "qsort",
                self.context
                    .void_type()
                    .fn_type(&[ptr.into(), i64.into(), i64.into(), ptr.into()], false),
                Some(Linkage::External),
            );
        }
        if self.module.get_function("titrate_sort_cmp").is_some() {
            return Ok(());
        }

        let tv_ty = native_bridge::titrate_value_type(self.context);
        let cmp_fn = self.module.add_function(
            "titrate_sort_cmp",
            i32.fn_type(&[ptr.into(), ptr.into()], false),
            Some(Linkage::Internal),
        );
        let a = cmp_fn.get_nth_param(0).unwrap().into_pointer_value();
        let b = cmp_fn.get_nth_param(1).unwrap().into_pointer_value();

        let entry = self.context.append_basic_block(cmp_fn, "entry");
        let strings = self.context.append_basic_block(cmp_fn, "strings");
        let numbers = self.context.append_basic_block(cmp_fn, "numbers");
        let ret_c = self.context.append_basic_block(cmp_fn, "ret.c");
        let cmp_len = self.context.append_basic_block(cmp_fn, "cmp.len");
        let len_lt = self.context.append_basic_block(cmp_fn, "len.lt");
        let len_ge = self.context.append_basic_block(cmp_fn, "len.ge");
        let len_gt = self.context.append_basic_block(cmp_fn, "len.gt");
        let len_eq = self.context.append_basic_block(cmp_fn, "len.eq");

        self.builder.position_at_end(entry);
        let payload_idx: u32 = 2;
        let pa = unsafe {
            self.builder
                .build_gep(tv_ty, a, &[i32.const_zero(), i32.const_int(payload_idx as u64, false)], "cmp.pa")
        }.map_err(|e| format!("sort cmp gep failed: {:?}", e))?;
        let pb = unsafe {
            self.builder
                .build_gep(tv_ty, b, &[i32.const_zero(), i32.const_int(payload_idx as u64, false)], "cmp.pb")
        }.map_err(|e| format!("sort cmp gep failed: {:?}", e))?;
        let tag_a = self.builder.build_load(i32, a, "cmp.tag.a")
            .map_err(|e| format!("sort cmp load failed: {:?}", e))?.into_int_value();
        let tag_b = self.builder.build_load(i32, b, "cmp.tag.b")
            .map_err(|e| format!("sort cmp load failed: {:?}", e))?.into_int_value();
        let is_str_a = self.builder.build_int_compare(inkwell::IntPredicate::EQ, tag_a, i32.const_int(native_bridge::TV_STRING as u64, false), "cmp.isa")
            .map_err(|e| format!("sort cmp icmp failed: {:?}", e))?;
        let is_str_b = self.builder.build_int_compare(inkwell::IntPredicate::EQ, tag_b, i32.const_int(native_bridge::TV_STRING as u64, false), "cmp.isb")
            .map_err(|e| format!("sort cmp icmp failed: {:?}", e))?;
        let both_str = self.builder.build_and(is_str_a, is_str_b, "cmp.bothstr")
            .map_err(|e| format!("sort cmp and failed: {:?}", e))?;
        self.builder.build_conditional_branch(both_str, strings, numbers)
            .map_err(|e| format!("sort cmp br failed: {:?}", e))?;

        // String payload is { i64 len, ptr data } in the 16-byte payload.
        let str_ty = self.context.struct_type(&[i64.into(), ptr.into()], false);
        self.builder.position_at_end(strings);
        let sa = self.builder.build_bit_cast(pa, ptr, "cmp.sa")
            .map_err(|e| format!("sort cmp bitcast failed: {:?}", e))?.into_pointer_value();
        let sb = self.builder.build_bit_cast(pb, ptr, "cmp.sb")
            .map_err(|e| format!("sort cmp bitcast failed: {:?}", e))?.into_pointer_value();
        let lena = self.builder.build_load(i64, sa, "cmp.lena")
            .map_err(|e| format!("sort cmp load failed: {:?}", e))?.into_int_value();
        let lenb = self.builder.build_load(i64, sb, "cmp.lenb")
            .map_err(|e| format!("sort cmp load failed: {:?}", e))?.into_int_value();
        let data_ptr_a = unsafe { self.builder.build_gep(str_ty, sa, &[i32.const_zero(), i32.const_int(1, false)], "cmp.dpa") }
            .map_err(|e| format!("sort cmp gep failed: {:?}", e))?;
        let data_ptr_b = unsafe { self.builder.build_gep(str_ty, sb, &[i32.const_zero(), i32.const_int(1, false)], "cmp.dpb") }
            .map_err(|e| format!("sort cmp gep failed: {:?}", e))?;
        let data_a = self.builder.build_load(ptr, data_ptr_a, "cmp.dataa")
            .map_err(|e| format!("sort cmp load failed: {:?}", e))?.into_pointer_value();
        let data_b = self.builder.build_load(ptr, data_ptr_b, "cmp.datab")
            .map_err(|e| format!("sort cmp load failed: {:?}", e))?.into_pointer_value();
        let slt = self.builder.build_int_compare(inkwell::IntPredicate::SLT, lena, lenb, "cmp.lt")
            .map_err(|e| format!("sort cmp icmp failed: {:?}", e))?;
        let min_len = self.builder.build_select(slt, lena, lenb, "cmp.minlen")
            .map_err(|e| format!("sort cmp select failed: {:?}", e))?.into_int_value();
        let memcmp = self.module.get_function("memcmp").unwrap();
        let c_call = self.builder.build_call(memcmp, &[data_a.into(), data_b.into(), min_len.into()], "cmp.memcmp")
            .map_err(|e| format!("sort cmp call failed: {:?}", e))?;
        let c = match c_call.try_as_basic_value() {
            inkwell::values::ValueKind::Basic(v) => v.into_int_value(),
            _ => return Err("sort cmp: memcmp returned no value".to_string()),
        };
        let c_ne = self.builder.build_int_compare(inkwell::IntPredicate::NE, c, i32.const_zero(), "cmp.cne")
            .map_err(|e| format!("sort cmp icmp failed: {:?}", e))?;
        self.builder.build_conditional_branch(c_ne, ret_c, cmp_len)
            .map_err(|e| format!("sort cmp br failed: {:?}", e))?;

        self.builder.position_at_end(ret_c);
        self.builder.build_return(Some(&c))
            .map_err(|e| format!("sort cmp ret failed: {:?}", e))?;

        self.builder.position_at_end(cmp_len);
        let lt = self.builder.build_int_compare(inkwell::IntPredicate::SLT, lena, lenb, "cmp.lenlt")
            .map_err(|e| format!("sort cmp icmp failed: {:?}", e))?;
        let eq = self.builder.build_int_compare(inkwell::IntPredicate::EQ, lena, lenb, "cmp.leneq")
            .map_err(|e| format!("sort cmp icmp failed: {:?}", e))?;
        self.builder.build_conditional_branch(lt, len_lt, len_ge)
            .map_err(|e| format!("sort cmp br failed: {:?}", e))?;
        self.builder.position_at_end(len_ge);
        self.builder.build_conditional_branch(eq, len_eq, len_gt)
            .map_err(|e| format!("sort cmp br failed: {:?}", e))?;
        self.builder.position_at_end(len_lt);
        self.builder.build_return(Some(&i32.const_int(-1i64 as u64, true)))
            .map_err(|e| format!("sort cmp ret failed: {:?}", e))?;
        self.builder.position_at_end(len_gt);
        self.builder.build_return(Some(&i32.const_int(1, false)))
            .map_err(|e| format!("sort cmp ret failed: {:?}", e))?;
        self.builder.position_at_end(len_eq);
        self.builder.build_return(Some(&i32.const_zero()))
            .map_err(|e| format!("sort cmp ret failed: {:?}", e))?;

        // Non-string tags: compare the numeric payload as i64.
        self.builder.position_at_end(numbers);
        let ia = self.builder.build_bit_cast(pa, ptr, "cmp.ia")
            .map_err(|e| format!("sort cmp bitcast failed: {:?}", e))?.into_pointer_value();
        let ib = self.builder.build_bit_cast(pb, ptr, "cmp.ib")
            .map_err(|e| format!("sort cmp bitcast failed: {:?}", e))?.into_pointer_value();
        let va = self.builder.build_load(i64, ia, "cmp.va")
            .map_err(|e| format!("sort cmp load failed: {:?}", e))?.into_int_value();
        let vb = self.builder.build_load(i64, ib, "cmp.vb")
            .map_err(|e| format!("sort cmp load failed: {:?}", e))?.into_int_value();
        let nlt = self.builder.build_int_compare(inkwell::IntPredicate::SLT, va, vb, "cmp.nlt")
            .map_err(|e| format!("sort cmp icmp failed: {:?}", e))?;
        let neq = self.builder.build_int_compare(inkwell::IntPredicate::EQ, va, vb, "cmp.neq")
            .map_err(|e| format!("sort cmp icmp failed: {:?}", e))?;
        let neg = self.builder.build_select(nlt, i32.const_int(-1i64 as u64, true), i32.const_int(1, false), "cmp.neg")
            .map_err(|e| format!("sort cmp select failed: {:?}", e))?.into_int_value();
        let res = self.builder.build_select(neq, i32.const_zero(), neg, "cmp.res")
            .map_err(|e| format!("sort cmp select failed: {:?}", e))?;
        self.builder.build_return(Some(&res))
            .map_err(|e| format!("sort cmp ret failed: {:?}", e))?;

        Ok(())
    }

    /// Read a user-class element stored in a TitrateArray. Object references are
    /// marshalled by storing the raw pointer in the first 8 bytes of the
    /// element's 16-byte payload, so return those bytes as a pointer directly.
    pub(super) fn compile_array_get_object_ptr(
        &mut self,
        array_expr: &Expr,
        index_expr: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let arr_val = self.compile_expr(array_expr)?;
        let idx_val = self.compile_expr(index_expr)?;
        if !arr_val.is_struct_value() {
            return Err("codegen: object array_get on non-array value".to_string());
        }
        let data_ptr = self
            .builder
            .build_extract_value(arr_val.into_struct_value(), 1, "objarr.data")
            .map_err(|e| format!("extract object array data failed: {:?}", e))?
            .into_pointer_value();
        let idx_i64 = if idx_val.is_int_value() {
            let iv = idx_val.into_int_value();
            if iv.get_type().get_bit_width() < 64 {
                self.builder
                    .build_int_z_extend(iv, self.context.i64_type(), "objarr.idx.ext")
                    .map_err(|e| format!("object array idx zext failed: {:?}", e))?
            } else {
                iv
            }
        } else {
            return Err("ArrayList.get: index must be integer".to_string());
        };
        let i64_ty = self.context.i64_type();
        let byte_ty = self.context.i8_type();
        let offset = self
            .builder
            .build_int_mul(idx_i64, i64_ty.const_int(24, false), "objarr.elem.off")
            .map_err(|e| format!("object array elem offset failed: {:?}", e))?;
        let elem = unsafe {
            self.builder.build_gep(byte_ty, data_ptr, &[offset], "objarr.elem")
        }
        .map_err(|e| format!("object array elem gep failed: {:?}", e))?;
        // The payload begins 8 bytes into the 24-byte TitrateValue.
        let payload = unsafe {
            self.builder.build_gep(byte_ty, elem, &[i64_ty.const_int(8, false)], "objarr.payload")
        }
        .map_err(|e| format!("object array payload gep failed: {:?}", e))?;
        let bits = self
            .builder
            .build_load(i64_ty, payload, "objarr.ptr.bits")
            .map_err(|e| format!("object array ptr load failed: {:?}", e))?
            .into_int_value();
        let ptr = self
            .builder
            .build_int_to_ptr(
                bits,
                self.context.ptr_type(AddressSpace::default()),
                "objarr.ptr",
            )
            .map_err(|e| format!("object array int_to_ptr failed: {:?}", e))?;
        Ok(ptr.into())
    }

    /// Read the raw 16-byte payload of an element as `{ i64, ptr }`. Used for
    /// nested containers, whose TitrateValue payload is a TitrateArray struct.
    pub(super) fn compile_array_get_raw_payload(
        &mut self,
        array_expr: &Expr,
        index_expr: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let arr_val = self.compile_expr(array_expr)?;
        let idx_val = self.compile_expr(index_expr)?;
        if !arr_val.is_struct_value() {
            return Err("codegen: container array_get on non-array value".to_string());
        }
        let data_ptr = self
            .builder
            .build_extract_value(arr_val.into_struct_value(), 1, "carr.data")
            .map_err(|e| format!("extract data failed: {:?}", e))?
            .into_pointer_value();
        let idx_i64 = if idx_val.is_int_value() {
            let iv = idx_val.into_int_value();
            if iv.get_type().get_bit_width() < 64 {
                self.builder
                    .build_int_z_extend(iv, self.context.i64_type(), "idx.ext")
                    .map_err(|e| format!("zext failed: {:?}", e))?
            } else {
                iv
            }
        } else {
            return Err("ArrayList.get: index must be integer".to_string());
        };
        let i64_ty = self.context.i64_type();
        let byte_ty = self.context.i8_type();
        let offset = self
            .builder
            .build_int_mul(idx_i64, i64_ty.const_int(24, false), "elem.offset")
            .map_err(|e| format!("offset failed: {:?}", e))?;
        let elem = unsafe {
            self.builder.build_gep(byte_ty, data_ptr, &[offset], "carr.elem")
        }
        .map_err(|e| format!("elem gep failed: {:?}", e))?;
        let payload = unsafe {
            self.builder.build_gep(
                byte_ty,
                elem,
                &[i64_ty.const_int(8, false)],
                "carr.payload",
            )
        }
        .map_err(|e| format!("payload gep failed: {:?}", e))?;
        let i8_ptr = self.context.ptr_type(AddressSpace::default());
        let container_ty = self.context.struct_type(&[i64_ty.into(), i8_ptr.into()], false);
        let payload_typed = self
            .builder
            .build_bit_cast(payload, i8_ptr, "carr.payload.cast")
            .map_err(|e| format!("payload cast failed: {:?}", e))?
            .into_pointer_value();
        self.builder
            .build_load(container_ty, payload_typed, "carr.value")
            .map_err(|e| format!("container load failed: {:?}", e))
    }

    /// Read a bool element from a TitrateArray. The runtime stores bools with
    /// tag TV_BOOL (2); the generic int accessor ignores that tag, so read the
    /// tag and payload bytes directly.
    pub(super) fn compile_array_get_bool(
        &mut self,
        array_expr: &Expr,
        index_expr: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let arr_val = self.compile_expr(array_expr)?;
        let idx_val = self.compile_expr(index_expr)?;
        if !arr_val.is_struct_value() {
            return Err("codegen: bool array_get on non-array value".to_string());
        }
        let data_ptr = self
            .builder
            .build_extract_value(arr_val.into_struct_value(), 1, "boolarr.data")
            .map_err(|e| format!("extract data failed: {:?}", e))?
            .into_pointer_value();
        let idx_i64 = if idx_val.is_int_value() {
            let iv = idx_val.into_int_value();
            if iv.get_type().get_bit_width() < 64 {
                self.builder
                    .build_int_z_extend(iv, self.context.i64_type(), "idx.ext")
                    .map_err(|e| format!("zext failed: {:?}", e))?
            } else {
                iv
            }
        } else {
            return Err("ArrayList.get: index must be integer".to_string());
        };
        let i64_ty = self.context.i64_type();
        let offset = self
            .builder
            .build_int_mul(idx_i64, i64_ty.const_int(24, false), "elem.offset")
            .map_err(|e| format!("offset failed: {:?}", e))?;
        let byte_ty = self.context.i8_type();
        let elem = unsafe {
            self.builder.build_gep(byte_ty, data_ptr, &[offset], "boolarr.elem")
        }
        .map_err(|e| format!("elem gep failed: {:?}", e))?;
        let tag = self
            .builder
            .build_load(self.context.i32_type(), elem, "boolarr.tag")
            .map_err(|e| format!("tag load failed: {:?}", e))?
            .into_int_value();
        let payload = unsafe {
            self.builder.build_gep(
                byte_ty,
                elem,
                &[i64_ty.const_int(8, false)],
                "boolarr.payload",
            )
        }
        .map_err(|e| format!("payload gep failed: {:?}", e))?;
        let byte = self
            .builder
            .build_load(self.context.i8_type(), payload, "boolarr.val")
            .map_err(|e| format!("payload load failed: {:?}", e))?
            .into_int_value();
        let tag_ok = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::EQ,
                tag,
                self.context.i32_type().const_int(2, false),
                "tag.is_bool",
            )
            .map_err(|e| format!("tag cmp failed: {:?}", e))?;
        let nonzero = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::NE,
                byte,
                self.context.i8_type().const_zero(),
                "val.nonzero",
            )
            .map_err(|e| format!("val cmp failed: {:?}", e))?;
        Ok(self
            .builder
            .build_and(tag_ok, nonzero, "boolarr.result")
            .map_err(|e| format!("and failed: {:?}", e))?
            .into())
    }

    /// Sort a `{ i64, ptr }` array in place using the generated comparator.
    pub(super) fn compile_array_sort(&mut self, array_expr: &Expr) -> Result<BasicValueEnum<'ctx>, String> {
        self.ensure_sort_helpers()?;
        let arr_val = self.compile_expr(array_expr)?;
        if !arr_val.is_struct_value() {
            return Ok(self.context.i32_type().const_zero().into());
        }
        let sv = arr_val.into_struct_value();
        let len = self.builder.build_extract_value(sv, 0, "sort.len")
            .map_err(|e| format!("sort extract len failed: {:?}", e))?.into_int_value();
        let data = self.builder.build_extract_value(sv, 1, "sort.data")
            .map_err(|e| format!("sort extract data failed: {:?}", e))?.into_pointer_value();
        let cmp = self.module.get_function("titrate_sort_cmp").unwrap();
        let qsort = self.module.get_function("qsort").unwrap();
        // TitrateValue is { i32, i32, [16 x i8] } = 24 bytes on all targets.
        let elem_size = self.context.i64_type().const_int(24, false);
        self.builder.build_call(
            qsort,
            &[data.into(), len.into(), elem_size.into(), cmp.as_global_value().as_pointer_value().into()],
            "array.sort",
        ).map_err(|e| format!("sort qsort call failed: {:?}", e))?;
        Ok(self.context.i32_type().const_zero().into())
    }
}
