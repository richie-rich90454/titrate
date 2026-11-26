// Array builtins

use super::native_bridge;
use super::tuple_codegen;
use super::types as llvm_types;
use super::vtable::reshape_container_or_fat_ptr;
use super::LlvmBackend;
use inkwell::module::Linkage;
use crate::ast::{ClassMember, Expr, Literal, Type};
use inkwell::AddressSpace;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicValueEnum, IntValue, PointerValue, StructValue};

/// True for Titrate types that share the native `{ i64, ptr }` container
/// layout. `Variant` can hold a container and is indexed through the same
/// layout, so it belongs here even though its payload is dynamic.
pub(super) fn is_container_layout_name(name: &str) -> bool {
    matches!(name, "array" | "ArrayList" | "HashMap" | "Variant")
}

/// Element (or positional map key) type declared on a container type.
fn container_element_type(ty: &Type) -> Option<Type> {
    match ty {
        Type::Named { name, params }
            if matches!(name.as_str(), "array" | "ArrayList" | "HashMap") =>
        {
            params.first().cloned()
        }
        _ => None,
    }
}

impl<'ctx> LlvmBackend<'ctx> {
    /// Canonical native container layout: `{ i64 len, ptr data }`.
    fn container_type(&self) -> inkwell::types::StructType<'ctx> {
        native_bridge::titrate_array_type(self.context)
    }

    /// Normalize a lowered index target to the canonical `{ i64, ptr }`
    /// struct. Containers already in that layout pass through; interface fat
    /// pointers (`{ ptr, ptr }`) and pointers to container slots are reshaped
    /// instead of being read with the wrong field types.
    fn as_container_struct(
        &mut self,
        value: BasicValueEnum<'ctx>,
        what: &str,
    ) -> Result<StructValue<'ctx>, String> {
        match value {
            BasicValueEnum::StructValue(sv) => {
                let st = sv.get_type();
                if st.count_fields() != 2 {
                    return Err(format!(
                        "codegen: {} has {} fields, expected a two-field container",
                        what,
                        st.count_fields()
                    ));
                }
                let first = st.get_field_type_at_index(0).ok_or_else(|| {
                    format!("codegen: {} container has no length field", what)
                })?;
                let second = st.get_field_type_at_index(1).ok_or_else(|| {
                    format!("codegen: {} container has no data field", what)
                })?;
                if !second.is_pointer_type() {
                    return Err(format!(
                        "codegen: {} container data field is '{}', expected a pointer",
                        what,
                        second.print_to_string()
                    ));
                }
                if first.is_int_type() {
                    return Ok(sv);
                }
                if first.is_pointer_type() {
                    // Fat pointers share the two-word slot with containers;
                    // convert field by field since LLVM cannot bitcast structs.
                    return reshape_container_or_fat_ptr(&self.builder, sv, self.container_type())
                        .map_err(|e| format!("codegen: {} is not a container: {}", what, e));
                }
                Err(format!(
                    "codegen: {} container length field is '{}', expected i64 or ptr",
                    what,
                    first.print_to_string()
                ))
            }
            BasicValueEnum::PointerValue(ptr) => self
                .builder
                .build_load(self.container_type(), ptr, "index.container")
                .map(|v| v.into_struct_value())
                .map_err(|e| format!("codegen: {} container load failed: {:?}", what, e)),
            other => Err(format!(
                "codegen: {} has type '{}', expected a container",
                what,
                other.get_type().print_to_string()
            )),
        }
    }

    /// Lower an index to the i64 offset used by the runtime's TitrateValue
    /// array layout.
    fn index_to_i64(&mut self, index: &Expr, what: &str) -> Result<IntValue<'ctx>, String> {
        let value = self.compile_expr(index)?;
        if !value.is_int_value() {
            return Err(format!(
                "codegen: {} index must be an integer, got '{}'",
                what,
                value.get_type().print_to_string()
            ));
        }
        let iv = value.into_int_value();
        let bits = iv.get_type().get_bit_width();
        if bits < 64 {
            self.builder
                .build_int_z_extend(iv, self.context.i64_type(), "index.zext")
                .map_err(|e| format!("codegen: {} index extend failed: {:?}", what, e))
        } else if bits > 64 {
            self.builder
                .build_int_truncate(iv, self.context.i64_type(), "index.trunc")
                .map_err(|e| format!("codegen: {} index truncate failed: {:?}", what, e))
        } else {
            Ok(iv)
        }
    }

    /// Pointer to the 24-byte TitrateValue slot at `index`.
    fn index_element_ptr(
        &mut self,
        container: StructValue<'ctx>,
        idx: IntValue<'ctx>,
        what: &str,
    ) -> Result<PointerValue<'ctx>, String> {
        let data = self
            .builder
            .build_extract_value(container, 1, "index.data")
            .map_err(|e| format!("codegen: {} data extract failed: {:?}", what, e))?;
        let data_ptr = match data {
            BasicValueEnum::PointerValue(p) => p,
            _ => {
                return Err(format!(
                    "codegen: {} container data field is not a pointer",
                    what
                ))
            }
        };
        let i64_ty = self.context.i64_type();
        let byte_ty = self.context.i8_type();
        let offset = self
            .builder
            .build_int_mul(idx, i64_ty.const_int(24, false), "index.offset")
            .map_err(|e| format!("codegen: {} offset failed: {:?}", what, e))?;
        unsafe {
            self.builder
                .build_gep(byte_ty, data_ptr, &[offset], "index.elem")
        }
        .map_err(|e| format!("codegen: {} element gep failed: {:?}", what, e))
    }

    /// Pointer to the 16-byte payload inside a TitrateValue slot.
    fn index_payload_ptr(
        &mut self,
        container: StructValue<'ctx>,
        idx: IntValue<'ctx>,
        what: &str,
    ) -> Result<PointerValue<'ctx>, String> {
        let elem = self.index_element_ptr(container, idx, what)?;
        let i64_ty = self.context.i64_type();
        unsafe {
            self.builder.build_gep(
                self.context.i8_type(),
                elem,
                &[i64_ty.const_int(8, false)],
                "index.payload",
            )
        }
        .map_err(|e| format!("codegen: {} payload gep failed: {:?}", what, e))
    }

    /// Constant, non-negative numeric slot of a tuple or class instance.
    fn literal_slot(index: &Expr) -> Option<usize> {
        match index {
            Expr::Literal(Literal::Int(n), _) if *n >= 0 => Some(*n as usize),
            _ => None,
        }
    }

    /// Lower `tuple[slot]` / `instance[slot]`: a literal field index into an
    /// anonymous struct, or into a class instance (whose field 0 is the
    /// vtable pointer, so user fields start at LLVM slot 1).
    pub(super) fn compile_slot_index(
        &mut self,
        obj: &Expr,
        index: &Expr,
        what: &str,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let slot = Self::literal_slot(index).ok_or_else(|| {
            format!(
                "codegen: {} index must be a non-negative integer literal",
                what
            )
        })?;
        let obj_val = self.compile_expr(obj)?;
        match obj_val {
            BasicValueEnum::StructValue(sv) => {
                let count = sv.get_type().count_fields();
                if slot >= count as usize {
                    return Err(format!(
                        "codegen: {} slot index {} out of bounds ({} fields)",
                        what, slot, count
                    ));
                }
                tuple_codegen::emit_tuple_field_access(&self.builder, sv.into(), slot as u32)
            }
            BasicValueEnum::PointerValue(ptr) => {
                let ty_name = self.infer_expr_type(obj).name().to_string();
                let class_info = self.class_infos.get(&ty_name).cloned().ok_or_else(|| {
                    format!(
                        "codegen: cannot index pointer to '{}' as a struct slot",
                        ty_name
                    )
                })?;
                let slot = slot + 1;
                if slot >= class_info.struct_type.count_fields() as usize {
                    return Err(format!(
                        "codegen: {} slot index {} out of bounds for class '{}'",
                        what,
                        slot - 1,
                        ty_name
                    ));
                }
                let gep = self
                    .builder
                    .build_struct_gep(class_info.struct_type, ptr, slot as u32, "index.slot")
                    .map_err(|e| format!("codegen: {} slot gep failed: {:?}", what, e))?;
                let field_ty = class_info
                    .struct_type
                    .get_field_type_at_index(slot as u32)
                    .ok_or_else(|| format!("codegen: {} slot has no field type", what))?;
                self.builder
                    .build_load(field_ty, gep, "index.slot.val")
                    .map_err(|e| format!("codegen: {} slot load failed: {:?}", what, e))
            }
            other => Err(format!(
                "codegen: cannot index '{}' as {} slots",
                other.get_type().print_to_string(),
                what
            )),
        }
    }
    /// Compile `array.length()` - extracts the i64 length from a TitrateArray struct and truncates to i32.
    pub(super) fn compile_array_length(&mut self, array_expr: &Expr) -> Result<BasicValueEnum<'ctx>, String> {
        let arr_val = self.compile_expr(array_expr)?;
        let sv = self.as_container_struct(arr_val, "array_length")?;
        let len_raw = self
            .builder
            .build_extract_value(sv, 0, "arr.len")
            .map_err(|e| format!("extract arr.len failed: {:?}", e))?;
        let len_val = match len_raw {
            BasicValueEnum::IntValue(i) => i,
            _ => return Err("codegen: array_length container length is not an integer".to_string()),
        };
        let i32_val = self
            .builder
            .build_int_truncate(len_val, self.context.i32_type(), "arr.len.i32")
            .map_err(|e| format!("int_truncate failed: {:?}", e))?;
        Ok(i32_val.into())
    }

    /// Full element type of an array expression (the `T` in `ArrayList<T>`),
    /// preserving generic arguments for nested containers. Local declarations,
    /// fields, calls, static calls, casts and chained indexes all carry their
    /// container parameter through the inferred type.
    pub(super) fn array_element_type(&self, array_expr: &Expr) -> Option<Type> {
        match array_expr {
            Expr::Identifier(name, _) => {
                if let Some(full) = self.locals.get(name).and_then(|v| v.full_type.as_ref()) {
                    if let Some(elem) = container_element_type(full) {
                        return Some(elem);
                    }
                }
            }
            Expr::MemberAccess(obj, field, _) => {
                let obj_ty = self.infer_expr_type(obj);
                let class_name = self
                    .mono_mangled_for(&obj_ty)
                    .unwrap_or_else(|| obj_ty.name().to_string());
                if let Some(ty) = self
                    .field_titrate_type_names
                    .get(&format!("{}_{}", class_name, field))
                {
                    if let Some(elem) = container_element_type(ty) {
                        return Some(elem);
                    }
                }
                if let Some(cd) = self.class_decls.get(obj_ty.name()) {
                    if let Some(f) = cd.members.iter().find_map(|m| match m {
                        ClassMember::Field(f) if f.name == *field => Some(f),
                        _ => None,
                    }) {
                        if let Some(elem) = container_element_type(&f.typ) {
                            return Some(elem);
                        }
                    }
                }
            }
            // Chained indexing: `matrix[i][j]` has the element type of the
            // inner container's element, not of the scalar inner index result.
            Expr::Index(inner, _, _) => {
                if let Some(elem) = container_element_type(&self.infer_expr_type(inner)) {
                    return Some(elem);
                }
            }
            // Calls (including chained `get`), static calls and casts fall
            // through to the generic type-based lookup.
            _ => {}
        }
        container_element_type(&self.infer_expr_type(array_expr))
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
        let element_type = self.array_element_type(array_expr);
        let element_type_name = element_type.as_ref().map(|t| t.name().to_string());

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
                // Generic classes compile under a mangled name. Resolve the
                // element first; looking up the raw source name misses the
                // specialization and wrongly yields a raw `{ i64, ptr }`
                // payload where field access expects an object pointer.
                let resolved = element_type
                    .as_ref()
                    .map(|t| self.resolve_mono_class(t))
                    .transpose()?
                    .unwrap_or_else(|| elem.to_string());
                // User-class instances are marshalled as a raw object pointer in
                // the first 8 bytes of the TitrateValue payload. Hand it back as
                // a pointer so field access and method dispatch work as usual.
                if self.class_infos.contains_key(&resolved) {
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
        let sv = self.as_container_struct(arr_val, "array_get")?;
        let idx_i64 = self.index_to_i64(index_expr, "array_get")?;

        let len_raw = self
            .builder
            .build_extract_value(sv, 0, "arr.len")
            .map_err(|e| format!("extract arr.len failed: {:?}", e))?;
        let len_val = match len_raw {
            BasicValueEnum::IntValue(i) => i,
            _ => return Err("codegen: array_get container length is not an integer".to_string()),
        };
        let data_raw = self
            .builder
            .build_extract_value(sv, 1, "arr.data")
            .map_err(|e| format!("extract arr.data failed: {:?}", e))?;
        let data_ptr = match data_raw {
            BasicValueEnum::PointerValue(p) => p,
            _ => return Err("codegen: array_get container data is not a pointer".to_string()),
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
                        len_val,
                        0,
                        "arr.len",
                    )
                    .map_err(|e| format!("insert_value failed: {:?}", e))?,
                data_ptr,
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

    /// Lower `.clone()` on a builtin container by copying its `{ i64, ptr }`
    /// element buffer into fresh storage. Container elements are 24-byte
    /// TitrateValues, so a byte copy preserves every tag and payload without
    /// knowing the element type.
    pub(super) fn compile_container_clone(
        &mut self,
        receiver: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let arr_val = self.compile_expr(receiver)?;
        let sv = match arr_val {
            BasicValueEnum::StructValue(sv) => sv,
            _ => return Err("codegen: clone receiver is not a container".to_string()),
        };
        let i64_ty = self.context.i64_type();
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let len = self
            .builder
            .build_extract_value(sv, 0, "clone.len")
            .map_err(|e| format!("clone len extract failed: {:?}", e))?
            .into_int_value();
        let data = self
            .builder
            .build_extract_value(sv, 1, "clone.data")
            .map_err(|e| format!("clone data extract failed: {:?}", e))?
            .into_pointer_value();
        let bytes = self
            .builder
            .build_int_mul(len, i64_ty.const_int(24, false), "clone.bytes")
            .map_err(|e| format!("clone bytes failed: {:?}", e))?;
        let malloc_fn = self
            .module
            .get_function("titrate_malloc")
            .ok_or("clone: titrate_malloc not declared")?;
        let malloc_result = self
            .builder
            .build_call(malloc_fn, &[bytes.into()], "clone.malloc")
            .map_err(|e| format!("clone malloc failed: {:?}", e))?;
        let new_data = match malloc_result.try_as_basic_value() {
            inkwell::values::ValueKind::Basic(v) => v.into_pointer_value(),
            _ => return Err("clone: titrate_malloc returned no value".to_string()),
        };
        let memcpy_ty = self.context.void_type().fn_type(
            &[
                ptr_ty.into(),
                ptr_ty.into(),
                i64_ty.into(),
                self.context.bool_type().into(),
            ],
            false,
        );
        let memcpy_fn = if let Some(f) = self.module.get_function("llvm.memcpy.p0.p0.i64") {
            f
        } else {
            self.module
                .add_function("llvm.memcpy.p0.p0.i64", memcpy_ty, Some(Linkage::External))
        };
        self.builder
            .build_call(
                memcpy_fn,
                &[
                    new_data.into(),
                    data.into(),
                    bytes.into(),
                    self.context.bool_type().const_zero().into(),
                ],
                "clone.copy",
            )
            .map_err(|e| format!("clone memcpy failed: {:?}", e))?;
        let arr_ty = self.context.struct_type(&[i64_ty.into(), ptr_ty.into()], false);
        let result = self
            .builder
            .build_insert_value(arr_ty.const_zero(), len, 0, "clone.len")
            .map_err(|e| format!("clone len insert failed: {:?}", e))?;
        let result = self
            .builder
            .build_insert_value(result, new_data, 1, "clone.ptr")
            .map_err(|e| format!("clone ptr insert failed: {:?}", e))?;
        match result {
            inkwell::values::AggregateValueEnum::StructValue(sv) => Ok(sv.into()),
            _ => Err("clone: expected struct result".to_string()),
        }
    }

    /// Lower `HashMap.update(other)` by copying every `[key, value]` pair from
    /// `other` into the receiver through the `HashMap_put` native. The other
    /// map holds its pairs as `{ i64, ptr }` TitrateArrays inside TitrateValues.
    pub(super) fn compile_hashmap_update(
        &mut self,
        receiver: &Expr,
        other: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let map_ty = self.infer_expr_type(receiver);
        let key_ty = map_ty
            .params()
            .first()
            .cloned()
            .ok_or("codegen: HashMap.update needs a typed key")?;
        let val_ty = map_ty
            .params()
            .get(1)
            .cloned()
            .ok_or("codegen: HashMap.update needs a typed value")?;
        let map_val = self.compile_expr(receiver)?;
        let other_val = self.compile_expr(other)?;
        let other_sv = match other_val {
            BasicValueEnum::StructValue(sv) => sv,
            _ => return Err("codegen: HashMap.update other is not a map".to_string()),
        };
        let i64_ty = self.context.i64_type();
        let byte_ty = self.context.i8_type();
        let container_ty = native_bridge::titrate_array_type(self.context);
        let tv_ty = native_bridge::titrate_value_type(self.context);
        let len = self
            .builder
            .build_extract_value(other_sv, 0, "update.len")
            .map_err(|e| format!("update len extract failed: {:?}", e))?
            .into_int_value();
        let data = self
            .builder
            .build_extract_value(other_sv, 1, "update.data")
            .map_err(|e| format!("update data extract failed: {:?}", e))?
            .into_pointer_value();
        let map_slot = self.build_entry_alloca(container_ty.into(), "update.map")?;
        self.builder
            .build_store(map_slot, map_val)
            .map_err(|e| format!("update map store failed: {:?}", e))?;
        let idx_slot = self.build_entry_alloca(i64_ty.into(), "update.i")?;
        self.builder
            .build_store(idx_slot, i64_ty.const_zero())
            .map_err(|e| format!("update i store failed: {:?}", e))?;
        let current = self
            .builder
            .get_insert_block()
            .ok_or("codegen: no insert block for HashMap.update")?;
        let cond_block = self
            .context
            .insert_basic_block_after(current, "update.cond");
        let body_block = self
            .context
            .insert_basic_block_after(cond_block, "update.body");
        let end_block = self
            .context
            .insert_basic_block_after(body_block, "update.end");
        self.builder
            .build_unconditional_branch(cond_block)
            .map_err(|e| format!("update br cond failed: {:?}", e))?;

        self.builder.position_at_end(cond_block);
        let idx = self
            .builder
            .build_load(i64_ty, idx_slot, "update.i.val")
            .map_err(|e| format!("update i load failed: {:?}", e))?
            .into_int_value();
        let more = self
            .builder
            .build_int_compare(inkwell::IntPredicate::ULT, idx, len, "update.more")
            .map_err(|e| format!("update cmp failed: {:?}", e))?;
        self.builder
            .build_conditional_branch(more, body_block, end_block)
            .map_err(|e| format!("update br body failed: {:?}", e))?;

        self.builder.position_at_end(body_block);
        let offset = self
            .builder
            .build_int_mul(idx, i64_ty.const_int(24, false), "update.off")
            .map_err(|e| format!("update offset failed: {:?}", e))?;
        let entry = unsafe {
            self.builder
                .build_gep(byte_ty, data, &[offset], "update.entry")
        }
        .map_err(|e| format!("update entry gep failed: {:?}", e))?;
        let payload = unsafe {
            self.builder.build_gep(
                byte_ty,
                entry,
                &[i64_ty.const_int(8, false)],
                "update.payload",
            )
        }
        .map_err(|e| format!("update payload gep failed: {:?}", e))?;
        let pair = self
            .builder
            .build_load(container_ty, payload, "update.pair")
            .map_err(|e| format!("update pair load failed: {:?}", e))?
            .into_struct_value();
        let pair_data = self
            .builder
            .build_extract_value(pair, 1, "update.pair.data")
            .map_err(|e| format!("update pair data failed: {:?}", e))?
            .into_pointer_value();
        let key_tv = self
            .builder
            .build_load(tv_ty, pair_data, "update.key.tv")
            .map_err(|e| format!("update key load failed: {:?}", e))?
            .into_struct_value();
        let val_ptr = unsafe {
            self.builder.build_gep(
                tv_ty,
                pair_data,
                &[i64_ty.const_int(1, false)],
                "update.val.ptr",
            )
        }
        .map_err(|e| format!("update val gep failed: {:?}", e))?;
        let val_tv = self
            .builder
            .build_load(tv_ty, val_ptr, "update.val.tv")
            .map_err(|e| format!("update val load failed: {:?}", e))?
            .into_struct_value();
        let key = native_bridge::unmarshal_from_titrate_declared(
            self.context,
            &self.builder,
            key_tv,
            &key_ty,
            Some(&key_ty),
        )?;
        let val = native_bridge::unmarshal_from_titrate_declared(
            self.context,
            &self.builder,
            val_tv,
            &val_ty,
            Some(&val_ty),
        )?;
        let map = self
            .builder
            .build_load(container_ty, map_slot, "update.map.val")
            .map_err(|e| format!("update map load failed: {:?}", e))?;
        let updated = self.compile_native_call_declared(
            "HashMap_put",
            &[map, key, val],
            &[map_ty.clone(), key_ty, val_ty],
            Some(&map_ty),
        )?;
        self.builder
            .build_store(map_slot, updated)
            .map_err(|e| format!("update map store back failed: {:?}", e))?;
        let next = self
            .builder
            .build_int_add(idx, i64_ty.const_int(1, false), "update.next")
            .map_err(|e| format!("update next failed: {:?}", e))?;
        self.builder
            .build_store(idx_slot, next)
            .map_err(|e| format!("update i next store failed: {:?}", e))?;
        self.builder
            .build_unconditional_branch(cond_block)
            .map_err(|e| format!("update br back failed: {:?}", e))?;

        self.builder.position_at_end(end_block);
        self.builder
            .build_load(container_ty, map_slot, "update.result")
            .map_err(|e| format!("update result load failed: {:?}", e))
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
        let sv = self.as_container_struct(arr_val, "object array_get")?;
        let idx_i64 = self.index_to_i64(index_expr, "object array_get")?;
        let payload = self.index_payload_ptr(sv, idx_i64, "object array_get")?;
        let i64_ty = self.context.i64_type();
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
        let sv = self.as_container_struct(arr_val, "raw array_get")?;
        let idx_i64 = self.index_to_i64(index_expr, "raw array_get")?;
        let payload = self.index_payload_ptr(sv, idx_i64, "raw array_get")?;
        self.builder
            .build_load(self.container_type(), payload, "carr.value")
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
        let sv = self.as_container_struct(arr_val, "bool array_get")?;
        let idx_i64 = self.index_to_i64(index_expr, "bool array_get")?;
        let elem = self.index_element_ptr(sv, idx_i64, "bool array_get")?;
        let tag = self
            .builder
            .build_load(self.context.i32_type(), elem, "boolarr.tag")
            .map_err(|e| format!("tag load failed: {:?}", e))?
            .into_int_value();
        let payload = self.index_payload_ptr(sv, idx_i64, "bool array_get")?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Access, FnDecl, Span};
    use super::super::LocalVar;
    use inkwell::context::Context;

    fn backend_with_block<'ctx>(context: &'ctx Context) -> LlvmBackend<'ctx> {
        let mut backend = LlvmBackend::new(context, "index_test");
        let fn_ty = context.void_type().fn_type(&[], false);
        let f = backend.module.add_function("index_test_fn", fn_ty, None);
        let entry = context.append_basic_block(f, "entry");
        backend.builder.position_at_end(entry);
        backend
    }

    #[test]
    fn containers_share_the_two_field_layout() {
        assert!(is_container_layout_name("array"));
        assert!(is_container_layout_name("ArrayList"));
        assert!(is_container_layout_name("HashMap"));
        assert!(is_container_layout_name("Variant"));
        assert!(!is_container_layout_name("string"));
        assert!(!is_container_layout_name("int"));
    }

    #[test]
    fn container_element_type_uses_first_type_parameter() {
        let list = Type::generic("ArrayList", vec![Type::simple("double")]);
        assert_eq!(
            container_element_type(&list).map(|t| t.name().to_string()),
            Some("double".to_string())
        );
        let map = Type::generic(
            "HashMap",
            vec![Type::simple("string"), Type::simple("int")],
        );
        assert_eq!(
            container_element_type(&map).map(|t| t.name().to_string()),
            Some("string".to_string())
        );
        assert!(container_element_type(&Type::simple("string")).is_none());
    }

    #[test]
    fn index_element_type_follows_locals_and_call_results() {
        let context = Context::create();
        let mut backend = backend_with_block(&context);
        let slot_ty: BasicTypeEnum<'_> = native_bridge::titrate_array_type(&context).into();
        let slot = backend
            .builder
            .build_alloca(slot_ty, "list")
            .expect("alloca list");
        backend.locals.insert(
            "list".to_string(),
            LocalVar {
                ptr: slot,
                ty: slot_ty,
                titrate_type: Some("ArrayList".to_string()),
                full_type: Some(Type::generic(
                    "ArrayList",
                    vec![Type::simple("double")],
                )),
                shared: false,
                closure_sig: None,
            },
        );
        let local = Expr::Identifier("list".to_string(), Span::unknown());
        assert_eq!(
            backend.array_element_type(&local).map(|t| t.name().to_string()),
            Some("double".to_string())
        );

        backend.function_decls.insert(
            "make".to_string(),
            FnDecl {
                access: Access::Public,
                name: "make".to_string(),
                type_params: vec![],
                params: vec![],
                return_type: Some(Type::generic("ArrayList", vec![Type::simple("int")])),
                body: vec![],
                sugar: false,
                where_clause: vec![],
                span: Span::unknown(),
            },
        );
        let call = Expr::Call(
            Box::new(Expr::Identifier("make".to_string(), Span::unknown())),
            vec![],
            vec![],
            Span::unknown(),
        );
        assert_eq!(
            backend.array_element_type(&call).map(|t| t.name().to_string()),
            Some("int".to_string())
        );
        let chained = Expr::Index(Box::new(call), Box::new(local), Span::unknown());
        assert_eq!(
            backend.array_element_type(&chained).map(|t| t.name().to_string()),
            Some("int".to_string())
        );
    }

    #[test]
    fn container_index_lowers_to_typed_element_accessor() {
        let context = Context::create();
        let mut backend = backend_with_block(&context);
        let slot_ty: BasicTypeEnum<'_> = native_bridge::titrate_array_type(&context).into();
        let slot = backend
            .builder
            .build_alloca(slot_ty, "list")
            .expect("alloca list");
        backend.locals.insert(
            "list".to_string(),
            LocalVar {
                ptr: slot,
                ty: slot_ty,
                titrate_type: Some("ArrayList".to_string()),
                full_type: Some(Type::generic("ArrayList", vec![Type::simple("double")])),
                shared: false,
                closure_sig: None,
            },
        );
        let obj = Expr::Identifier("list".to_string(), Span::unknown());
        let index = Expr::Literal(Literal::Int(1), Span::unknown());
        let value = backend
            .compile_index_expr(&obj, &index)
            .expect("ArrayList<double> index should lower");
        assert!(value.is_float_value());
        let ir = backend.module.print_to_string().to_string();
        assert!(ir.contains("titrate_array_get_double"), "{}", ir);
    }

    #[test]
    fn fat_pointer_target_reshapes_to_container_layout() {
        let context = Context::create();
        let mut backend = backend_with_block(&context);
        let ptr_ty = context.ptr_type(AddressSpace::default());
        let fat_ty = context.struct_type(&[ptr_ty.into(), ptr_ty.into()], false);
        let value = fat_ty.const_named_struct(&[ptr_ty.const_null().into(), ptr_ty.const_null().into()]);

        let container = backend
            .as_container_struct(value.into(), "test target")
            .expect("fat pointer should reshape to a container");
        let st = container.get_type();
        assert_eq!(st.count_fields(), 2);
        assert!(st.get_field_type_at_index(0).unwrap().is_int_type());
        assert!(st.get_field_type_at_index(1).unwrap().is_pointer_type());
    }

    #[test]
    fn unrelated_index_target_reports_precise_error() {
        let context = Context::create();
        let mut backend = backend_with_block(&context);
        let err = backend
            .as_container_struct(context.i32_type().const_zero().into(), "index target")
            .err()
            .expect("a scalar must not be read as a container");
        assert!(err.contains("expected a container"), "{}", err);
        assert!(err.contains("index target"), "{}", err);
    }

    #[test]
    fn tuple_slot_index_extracts_literal_field() {
        let context = Context::create();
        let mut backend = backend_with_block(&context);
        let tuple = Expr::Tuple(
            vec![
                Expr::Literal(Literal::Int(7), Span::unknown()),
                Expr::Literal(Literal::Bool(true), Span::unknown()),
            ],
            Span::unknown(),
        );
        let index = Expr::Literal(Literal::Int(1), Span::unknown());
        let value = backend
            .compile_slot_index(&tuple, &index, "tuple")
            .expect("constant tuple index");
        assert!(value.is_int_value());
        let ir = backend.module.print_to_string().to_string();
        assert!(ir.contains("extractvalue"), "{}", ir);
    }

    #[test]
    fn tuple_slot_index_rejects_dynamic_index() {
        let context = Context::create();
        let mut backend = backend_with_block(&context);
        let tuple = Expr::Tuple(
            vec![Expr::Literal(Literal::Int(7), Span::unknown())],
            Span::unknown(),
        );
        let index = Expr::Identifier("i".to_string(), Span::unknown());
        let err = backend
            .compile_slot_index(&tuple, &index, "tuple")
            .err()
            .expect("dynamic tuple index must be rejected");
        assert!(err.contains("non-negative integer literal"), "{}", err);
    }
}
