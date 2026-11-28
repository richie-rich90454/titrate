// Assignment, casts, and ternaries

use super::types as llvm_types;
use super::vtable::{
    emit_field_store, field_index, is_container_or_fat_ptr_shape, reshape_container_or_fat_ptr,
};
use super::{native_bridge, ClosureSig, LlvmBackend};
use crate::ast::{Expr, Type};
use inkwell::context::Context;
use inkwell::AddressSpace;
use inkwell::types::{BasicType, BasicTypeEnum, FloatType, IntType};
use inkwell::values::{BasicValueEnum, FloatValue, IntValue, PointerValue};
#[cfg(test)]
use inkwell::values::AnyValue;

// Width codes stored in field 1 of a float numeric box. The tag field can be
// fully occupied by f64/f128 bits, so the marker cannot live there; field 1 is
// null for numeric boxes. A null field 1 is reserved for untagged boxes (raw
// f64 bits in an i64 tag), so older values keep decoding the same way.
const FLOAT_BOX_HALF: u64 = 1;
const FLOAT_BOX_FLOAT: u64 = 2;
const FLOAT_BOX_DOUBLE: u64 = 3;
const FLOAT_BOX_QUAD: u64 = 4;

#[cfg(test)]
const FLOAT_BOX_WIDTH_CODES: &[(u64, u32, &str)] = &[
    (FLOAT_BOX_HALF, 16, "inttoptr (i64 1 to ptr)"),
    (FLOAT_BOX_FLOAT, 32, "inttoptr (i64 2 to ptr)"),
    (FLOAT_BOX_DOUBLE, 64, "inttoptr (i64 3 to ptr)"),
    (FLOAT_BOX_QUAD, 128, "inttoptr (i64 4 to ptr)"),
];

fn float_box_code_for_bits(bits: u32) -> Option<u64> {
    match bits {
        16 => Some(FLOAT_BOX_HALF),
        32 => Some(FLOAT_BOX_FLOAT),
        64 => Some(FLOAT_BOX_DOUBLE),
        128 => Some(FLOAT_BOX_QUAD),
        _ => None,
    }
}

fn float_box_width_for_code(code: u64) -> Option<u32> {
    match code {
        FLOAT_BOX_HALF => Some(16),
        FLOAT_BOX_FLOAT => Some(32),
        FLOAT_BOX_DOUBLE => Some(64),
        FLOAT_BOX_QUAD => Some(128),
        _ => None,
    }
}

fn float_box_float_type<'ctx>(context: &'ctx Context, bits: u32) -> Option<FloatType<'ctx>> {
    match bits {
        16 => Some(context.f16_type()),
        32 => Some(context.f32_type()),
        64 => Some(context.f64_type()),
        128 => Some(context.f128_type()),
        _ => None,
    }
}

fn float_box_int_type<'ctx>(context: &'ctx Context, bits: u32) -> Option<IntType<'ctx>> {
    match bits {
        16 => Some(context.i16_type()),
        32 => Some(context.i32_type()),
        64 => Some(context.i64_type()),
        128 => Some(context.i128_type()),
        _ => None,
    }
}

/// Size of the shared zero-filled cell behind a boxed numeric Variant. It is
/// large enough for the equality comparison on ordinary integer payloads to
/// read without leaving the allocation.
const NUMERIC_BOX_BYTES: u32 = 4096;
const NUMERIC_BOX_GLOBAL: &str = ".titrate.numeric.box";

impl<'ctx> LlvmBackend<'ctx> {
    pub(super) fn compile_assign(
        &mut self,
        target: &Expr,
        value: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if let Expr::Identifier(name, _) = target {
            // Module-level global: store into its LLVM global slot.
            if !self.locals.contains_key(name) {
                if let Some(g) = self.globals.get(name).cloned() {
                    let v = self.compile_expr(value)?;
                    let v = if v.get_type() != g.ty {
                        self.cast_value_to_type(v, g.ty)?
                    } else {
                        v
                    };
                    self.builder
                        .build_store(g.ptr, v)
                        .map_err(|e| format!("build_store global '{}' failed: {:?}", name, e))?;
                    return Ok(v);
                }
            }
            // Clone the LocalVar to release the immutable borrow
            // of self.locals before we call &mut self methods below.
            let mut var = self
                .locals
                .get(name)
                .cloned()
                .ok_or_else(|| format!("codegen: assignment to unknown variable '{}'", name))?;
            let var_is_container = var
                .full_type
                .as_ref()
                .map(Self::is_shared_container_ty)
                .unwrap_or(false)
                || matches!(
                    var.titrate_type.as_deref(),
                    Some("ArrayList" | "HashMap" | "array")
                );
            // String assignment. Containers share the `{ i64, ptr }` LLVM
            // layout with strings, so use the declared type, not the layout,
            // to avoid routing `list = f(list)` through the string path.
            if !var_is_container
                && llvm_types::is_string(&self.llvm_basic_type_to_titrate_type(var.ty))
            {
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
                // Rebinding a shared container detaches to a private copy so
                // the previous owner keeps the old list (VM ref semantics).
                // Do this after the RHS is compiled: `x = f(x)` passes the old
                // slot to f, so repointing x first would hand f an empty slot.
                if var.shared && var_is_container {
                    let fresh = self.build_entry_alloca(var.ty, name)?;
                    var.ptr = fresh;
                    var.shared = false;
                }
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
            // Class objects normally load as a pointer. Variant/container
            // locals lower to an inline { i64, ptr } slot whose field 1 holds
            // the object pointer (same convention as compile_member_access),
            // so unwrap it before GEP-ing the field.
            let obj_ptr = if obj_val.is_pointer_value() {
                Some(obj_val.into_pointer_value())
            } else if obj_val.is_struct_value() && self.is_string_struct(&obj_val) {
                self.builder
                    .build_extract_value(obj_val.into_struct_value(), 1, "assign.obj.ptr")
                    .ok()
                    .and_then(|p| if p.is_pointer_value() { Some(p.into_pointer_value()) } else { None })
            } else {
                None
            };
            if let Some(obj_ptr) = obj_ptr {
                let obj_type = self.infer_expr_type(obj);
                let class_name = self.resolve_mono_class(&obj_type)?;
                let class_info = self.class_infos.get(&class_name).cloned().ok_or_else(|| {
                    format!("codegen: class '{}' not found for field store", class_name)
                })?;
                let v = self.compile_expr(value)?;
                // Coerce to the declared field type so the store is well-typed.
                let slot = field_index(&class_info, field).ok_or_else(|| {
                    format!("field '{}' not found in class '{}'", field, class_name)
                })?;
                let field_ty = class_info
                    .struct_type
                    .get_field_type_at_index(slot)
                    .ok_or_else(|| format!("field '{}' has no type slot", field))?;
                let v = self.cast_value_to_type(v, field_ty)?;
                emit_field_store(self.context, &self.builder, &class_info, obj_ptr, field, v)?;
                return Ok(v);
            }
        }
        Err(format!(
            "codegen: unsupported assignment target: {:?}",
            target
        ))
    }

    /// Reinterpret boxed IEEE-754 bits of a known width as `to_ty`.
    fn build_unboxed_float(
        &self,
        bits: IntValue<'ctx>,
        width: u32,
        to_ty: FloatType<'ctx>,
    ) -> Result<FloatValue<'ctx>, String> {
        let from_ty = float_box_float_type(self.context, width)
            .ok_or_else(|| format!("codegen: cannot bitcast an i{} field to a float", bits.get_type().get_bit_width()))?;
        let raw_ty = float_box_int_type(self.context, width)
            .ok_or_else(|| format!("codegen: cannot bitcast an i{} field to a float", bits.get_type().get_bit_width()))?;
        let field_width = bits.get_type().get_bit_width();
        let exact_bits = if field_width > width {
            self.builder
                .build_int_truncate(bits, raw_ty, "cast.unbox.trunc")
                .map_err(|e| format!("cast unbox truncate failed: {:?}", e))?
        } else if field_width < width {
            // A foreign box with too few bits cannot hold this width; it is
            // still safe to read as a narrow value after zero-extension.
            self.builder
                .build_int_z_extend(bits, raw_ty, "cast.unbox.zext")
                .map_err(|e| format!("cast unbox extend failed: {:?}", e))?
        } else {
            bits
        };
        let raw = self
            .builder
            .build_bit_cast(exact_bits, from_ty, "cast.unbox.bits")
            .map_err(|e| format!("build_bit_cast failed: {:?}", e))?
            .into_float_value();
        if from_ty == to_ty {
            return Ok(raw);
        }
        if width < to_ty.get_bit_width() {
            self.builder.build_float_ext(raw, to_ty, "cast.fext")
        } else {
            self.builder.build_float_trunc(raw, to_ty, "cast.ftrunc")
        }
        .map_err(|e| format!("cast struct-to-float failed: {:?}", e))
    }

    /// Cast a value to match the given LLVM type (for assignments and
    /// declarations where the declared type may differ from the literal type).
    /// Shared, zero-filled cell used as the non-null data word of a numeric
    /// `{ i64 bits, ptr }` Variant box. A null word made `isEmpty()`/`get()`
    /// mistake every boxed number for null and left equality comparisons
    /// dereferencing address zero.
    pub(super) fn numeric_box_marker(&self) -> PointerValue<'ctx> {
        let ty = self.context.i8_type().array_type(NUMERIC_BOX_BYTES);
        if let Some(g) = self.module.get_global(NUMERIC_BOX_GLOBAL) {
            return g.as_pointer_value();
        }
        let g = self.module.add_global(ty, None, NUMERIC_BOX_GLOBAL);
        g.set_initializer(&ty.const_zero());
        g.set_constant(true);
        g.as_pointer_value()
    }

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
        // string struct { i64 len, ptr data } -> int: decode the Unicode
        // scalar of a one-character string (String.charAt returns a 1-4 byte
        // UTF-8 string). The generic struct->int path below would return the
        // byte length (2 for "é") instead of the code point (233).
        if v.is_struct_value() && target_ty.is_int_type() && self.is_string_struct(&v) {
            if let BasicValueEnum::StructValue(sv) = v {
                if sv.get_type().count_fields() == 2 {
                    let len_v = self.builder.build_extract_value(sv, 0, "cast.str.len").ok();
                    let ptr_v = self.builder.build_extract_value(sv, 1, "cast.str.ptr").ok();
                    if let (Some(len_v), Some(BasicValueEnum::PointerValue(data))) = (len_v, ptr_v) {
                        // A numeric Variant payload has the `{ i64 bits, ptr }`
                        // shape: its data word is either null (legacy) or the
                        // shared numeric-box cell. Only decode UTF-8 for a
                        // genuine string; otherwise field 0 is the number.
                        let i64_ty = self.context.i64_type();
                        let data_bits = self.builder
                            .build_ptr_to_int(data, i64_ty, "cast.str.data.bits")
                            .map_err(|e| format!("cast str data bits failed: {:?}", e))?;
                        let data_null = self.builder
                            .build_int_compare(
                                inkwell::IntPredicate::EQ,
                                data_bits,
                                i64_ty.const_zero(),
                                "cast.str.data.null",
                            )
                            .map_err(|e| format!("cast str data null check failed: {:?}", e))?;
                        let marker_bits = self.builder
                            .build_ptr_to_int(self.numeric_box_marker(), i64_ty, "cast.num.marker.bits")
                            .map_err(|e| format!("cast numeric marker bits failed: {:?}", e))?;
                        let is_marker = self.builder
                            .build_int_compare(
                                inkwell::IntPredicate::EQ,
                                data_bits,
                                marker_bits,
                                "cast.str.data.marker",
                            )
                            .map_err(|e| format!("cast data marker check failed: {:?}", e))?;
                        let is_numeric = self.builder
                            .build_or(data_null, is_marker, "cast.str.data.numeric")
                            .map_err(|e| format!("cast data numeric check failed: {:?}", e))?;
                        let to_ty = target_ty.into_int_type();
                        let coerce = |raw: IntValue<'ctx>, name: &str| -> Result<IntValue<'ctx>, String> {
                            if raw.get_type().get_bit_width() < to_ty.get_bit_width() {
                                self.builder.build_int_z_extend(raw, to_ty, name)
                            } else if raw.get_type().get_bit_width() > to_ty.get_bit_width() {
                                self.builder.build_int_truncate(raw, to_ty, name)
                            } else {
                                Ok(raw)
                            }
                            .map_err(|e| format!("{} failed: {:?}", name, e))
                        };
                        if let Some(flag) = is_numeric.get_zero_extended_constant() {
                            if flag != 0 {
                                return Ok(coerce(len_v.into_int_value(), "cast.num.zext")?.into());
                            }
                        }
                        let scalar = self.build_string_scalar(len_v.into_int_value(), data)?;
                        let scalar = coerce(scalar, "cast.str.code")?;
                        let raw = coerce(len_v.into_int_value(), "cast.num.select")?;
                        let result = self.builder
                            .build_select(is_numeric, raw, scalar, "cast.num.or.scalar")
                            .map_err(|e| format!("cast numeric/string select failed: {:?}", e))?;
                        return Ok(result);
                    }
                }
            }
        }
        // struct -> int: extract field 0 (i64) from {i64, ptr} struct, then coerce.
        if v.is_struct_value() && target_ty.is_int_type() && self.is_string_struct(&v) {
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
        // struct -> float: field 0 holds raw IEEE-754 bits and field 1 holds
        // the source width code (or null for legacy untagged boxes). The width
        // must come from the box: an f32 zero-extended into an i64 tag is not
        // an f64, and bitcasting it as one silently changes the value.
        if v.is_struct_value() && target_ty.is_float_type() && self.is_string_struct(&v) {
            if let BasicValueEnum::StructValue(sv) = v {
                let f0 = self.builder.build_extract_value(sv, 0, "cast.struct.bits").ok();
                let f1 = self.builder.build_extract_value(sv, 1, "cast.struct.width").ok();
                if let (Some(f0), Some(f1)) = (f0, f1) {
                    let bits = f0.into_int_value();
                    let to_ty = target_ty.into_float_type();
                    let code = self
                        .builder
                        .build_ptr_to_int(f1.into_pointer_value(), self.context.i64_type(), "cast.unbox.code")
                        .map_err(|e| format!("build_ptr_to_int float box width failed: {:?}", e))?;
                    let known_width = code
                        .get_zero_extended_constant()
                        .and_then(float_box_width_for_code);
                    if let Some(width) = known_width {
                        let result = self.build_unboxed_float(bits, width, to_ty)?;
                        return Ok(result.into());
                    }
                    if code.get_zero_extended_constant() == Some(0) {
                        // Legacy untagged box: field width is the only clue.
                        let width = bits.get_type().get_bit_width();
                        let result = self.build_unboxed_float(bits, width, to_ty)?;
                        return Ok(result.into());
                    }
                    // A box produced elsewhere with a dynamic width code:
                    // dispatch on it rather than guessing from the tag width.
                    let current = self
                        .builder
                        .get_insert_block()
                        .ok_or("codegen: no insert block for struct-to-float cast")?;
                    let merge = self.context.insert_basic_block_after(current, "cast.unbox.merge");
                    let legacy = self.context.insert_basic_block_after(current, "cast.unbox.legacy");
                    self.builder.position_at_end(legacy);
                    let legacy_width = bits.get_type().get_bit_width();
                    let legacy_val = self.build_unboxed_float(bits, legacy_width, to_ty)?;
                    self.builder
                        .build_unconditional_branch(merge)
                        .map_err(|e| format!("build_br unbox legacy failed: {:?}", e))?;
                    let mut incoming = vec![(legacy_val, legacy)];
                    let mut cases = Vec::new();
                    for code in [
                        FLOAT_BOX_HALF,
                        FLOAT_BOX_FLOAT,
                        FLOAT_BOX_DOUBLE,
                        FLOAT_BOX_QUAD,
                    ] {
                        let width = float_box_width_for_code(code)
                            .ok_or_else(|| format!("codegen: bad float box width code {}", code))?;
                        let block = self.context.insert_basic_block_after(current, "cast.unbox.width");
                        self.builder.position_at_end(block);
                        let val = self.build_unboxed_float(bits, width, to_ty)?;
                        self.builder
                            .build_unconditional_branch(merge)
                            .map_err(|e| format!("build_br unbox width failed: {:?}", e))?;
                        cases.push((self.context.i64_type().const_int(code, false), block));
                        incoming.push((val, block));
                    }
                    self.builder.position_at_end(current);
                    self.builder
                        .build_switch(code, legacy, &cases)
                        .map_err(|e| format!("build_switch float box width failed: {:?}", e))?;
                    self.builder.position_at_end(merge);
                    let phi = self
                        .builder
                        .build_phi(to_ty.as_basic_type_enum(), "cast.unbox.float")
                        .map_err(|e| format!("build_phi float box failed: {:?}", e))?;
                    for (val, block) in incoming {
                        phi.add_incoming(&[(&val, block)]);
                    }
                    return Ok(phi.as_basic_value());
                }
            }
        }
        // struct -> pointer: extract the pointer word from a two-word Titrate
        // value. Containers/strings/Variants keep data at field 1; interface
        // fat pointers and closures keep the object/function at field 0.
        if v.is_struct_value() && target_ty.is_pointer_type() {
            if let BasicValueEnum::StructValue(sv) = v {
                let st = sv.get_type();
                if is_container_or_fat_ptr_shape(st) {
                    let idx = if st
                        .get_field_type_at_index(0)
                        .is_some_and(|t| t.is_pointer_type())
                    {
                        0
                    } else {
                        1
                    };
                    if let Ok(f) = self.builder.build_extract_value(sv, idx, "cast.struct.ptr") {
                        if f.is_pointer_value() {
                            let ptr_val = f.into_pointer_value();
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
        // int -> { i64, ptr } slot: native Result calls arrive as a bare
        // handle, and `42 as Variant` boxes a number in the same shape. Keep
        // the numeric bits in field 0 and point field 1 at a valid zero cell:
        // a null data word made every boxed number look like null, and string
        // equality on it dereferenced address zero.
        if v.is_int_value() && target_ty.is_struct_type() {
            let st = target_ty.into_struct_type();
            if st.count_fields() == 2
                && st.get_field_type_at_index(0).is_some_and(|t| t.is_int_type())
                && st.get_field_type_at_index(1).is_some_and(|t| t.is_pointer_type())
            {
                // `char as string` lowers to an int cast followed by a return
                // cast to the string slot. Materialise the actual character
                // instead of boxing its code point as a numeric Variant.
                if self.current_ret_ty.as_ref().is_some_and(|t| t.name() == "string")
                    && v.into_int_value().get_type().get_bit_width() <= 32
                {
                    let int_ty = self.context.i32_type();
                    let arg = if v.get_type() == int_ty.into() {
                        v
                    } else {
                        self.builder
                            .build_int_z_extend(v.into_int_value(), int_ty, "cast.char.zext")
                            .map_err(|e| format!("cast char code failed: {:?}", e))?
                            .into()
                    };
                    return native_bridge::emit_native_call(
                        self.context,
                        &self.builder,
                        &self.module,
                        "String_fromCharCode",
                        &[arg],
                        &[Type::simple("int")],
                        Some(&Type::simple("string")),
                    );
                }
                let field_ty = st
                    .get_field_type_at_index(0)
                    .ok_or_else(|| format!("codegen: '{}' has no tag field", st.print_to_string()))?
                    .into_int_type();
                let tag = self.cast_value_to_type(v, field_ty.into())?.into_int_value();
                let res = self.builder.build_insert_value(
                    st.const_zero(), tag, 0, "cast.result.tag")
                    .map_err(|e| format!("build_insert_value result tag failed: {:?}", e))?;
                // A zero literal is the null value, not a boxed number. Keep
                // its data word null so null checks still recognise it.
                let is_zero_literal = v
                    .into_int_value()
                    .get_zero_extended_constant()
                    .is_some_and(|bits| bits == 0);
                let data_word = if is_zero_literal {
                    self.context.ptr_type(AddressSpace::default()).const_null()
                } else {
                    self.numeric_box_marker()
                };
                let res = self.builder.build_insert_value(
                    res, data_word, 1, "cast.result.ptr")
                    .map_err(|e| format!("build_insert_value result ptr failed: {:?}", e))?;
                if let inkwell::values::AggregateValueEnum::StructValue(sv) = res {
                    return Ok(sv.into());
                }
            }
        }
        // float -> { int, ptr } numeric box (Variant/dynamic slot): field 0
        // keeps the exact IEEE-754 bits and field 1 records the source float
        // width, so the struct -> float path decodes it instead of assuming
        // the tag field's width. Narrow tag fields are rejected rather than
        // silently truncating bits. Other target shapes fall through to the
        // precise error below.
        if v.is_float_value() && target_ty.is_struct_type() {
            let st = target_ty.into_struct_type();
            if st.count_fields() == 2
                && st.get_field_type_at_index(0).is_some_and(|t| t.is_int_type())
                && st.get_field_type_at_index(1).is_some_and(|t| t.is_pointer_type())
            {
                let field_ty = st
                    .get_field_type_at_index(0)
                    .ok_or_else(|| format!("codegen: '{}' has no tag field", st.print_to_string()))?
                    .into_int_type();
                let from = v.into_float_value();
                let bits = from.get_type().get_bit_width();
                let code = float_box_code_for_bits(bits).ok_or_else(|| {
                    format!(
                        "codegen: cannot convert value of type 'f{}' to '{}' without losing bits",
                        bits,
                        st.print_to_string()
                    )
                })?;
                let int_ty = float_box_int_type(self.context, bits).ok_or_else(|| {
                    format!(
                        "codegen: cannot convert value of type 'f{}' to '{}' without losing bits",
                        bits,
                        st.print_to_string()
                    )
                })?;
                if field_ty.get_bit_width() < bits {
                    return Err(format!(
                        "codegen: cannot preserve f{} bits in '{}' tag field (needs i{} or wider)",
                        bits,
                        st.print_to_string(),
                        bits
                    ));
                }
                let raw = self
                    .builder
                    .build_bit_cast(from, int_ty, "cast.fbox.bits")
                    .map_err(|e| format!("build_bit_cast float box failed: {:?}", e))?
                    .into_int_value();
                // These are bit patterns, not signed numbers: zero-extend.
                let tag = if raw.get_type().get_bit_width() < field_ty.get_bit_width() {
                    self.builder
                        .build_int_z_extend(raw, field_ty, "cast.fbox.zext")
                } else {
                    Ok(raw)
                }
                .map_err(|e| format!("cast float box tag failed: {:?}", e))?;
                let ptr_ty = st
                    .get_field_type_at_index(1)
                    .ok_or_else(|| format!("codegen: '{}' has no payload field", st.print_to_string()))?
                    .into_pointer_type();
                let width = self
                    .builder
                    .build_int_to_ptr(
                        self.context.i64_type().const_int(code, false),
                        ptr_ty,
                        "cast.fbox.width",
                    )
                    .map_err(|e| format!("build_int_to_ptr float box width failed: {:?}", e))?;
                let res = self
                    .builder
                    .build_insert_value(st.const_zero(), tag, 0, "cast.fbox.tag")
                    .map_err(|e| format!("build_insert_value float box tag failed: {:?}", e))?;
                let res = self
                    .builder
                    .build_insert_value(res, width, 1, "cast.fbox.ptr")
                    .map_err(|e| format!("build_insert_value float box width failed: {:?}", e))?;
                if let inkwell::values::AggregateValueEnum::StructValue(sv) = res {
                    return Ok(sv.into());
                }
            }
        }
        // ptr -> string struct { i64, ptr }: a Variant/object cast to string
        // yields a raw pointer. Wrap it with a zero length to keep the IR
        // well-formed (these paths are typechecked but rarely executed).
        if v.is_pointer_value() && target_ty.is_struct_type() {
            let st = target_ty.into_struct_type();
            if st.count_fields() == 2
                && st.get_field_type_at_index(0).is_some_and(|t| t.is_int_type())
                && st.get_field_type_at_index(1).is_some_and(|t| t.is_pointer_type())
            {
                let len = st
                    .get_field_type_at_index(0)
                    .ok_or_else(|| format!("codegen: '{}' has no length field", st.print_to_string()))?
                    .into_int_type()
                    .const_zero();
                let res = self.builder.build_insert_value(
                    st.const_zero(), len, 0, "cast.str.len")
                    .map_err(|e| format!("build_insert_value str len failed: {:?}", e))?;
                let res = self.builder.build_insert_value(
                    res, v.into_pointer_value(), 1, "cast.str.ptr")
                    .map_err(|e| format!("build_insert_value str ptr failed: {:?}", e))?;
                if let inkwell::values::AggregateValueEnum::StructValue(sv) = res {
                    return Ok(sv.into());
                }
            }
        }
        // ptr -> { ptr, ptr } interface/closure slot: a bare object or function
        // pointer has no vtable/capture, so store it in field 0 and a null word
        // in field 1. This is the inverse of struct -> pointer above.
        if v.is_pointer_value() && target_ty.is_struct_type() {
            let st = target_ty.into_struct_type();
            if st.count_fields() == 2
                && st
                    .get_field_type_at_index(0)
                    .is_some_and(|t| t.is_pointer_type())
                && st
                    .get_field_type_at_index(1)
                    .is_some_and(|t| t.is_pointer_type())
            {
                let null_word = st
                    .get_field_type_at_index(1)
                    .ok_or_else(|| "codegen: fat pointer has no second word".to_string())?
                    .into_pointer_type()
                    .const_null();
                let res = self
                    .builder
                    .build_insert_value(st.const_zero(), v.into_pointer_value(), 0, "cast.fatptr.obj")
                    .map_err(|e| format!("build_insert_value fatptr obj failed: {:?}", e))?;
                let res = self
                    .builder
                    .build_insert_value(res, null_word, 1, "cast.fatptr.capture")
                    .map_err(|e| format!("build_insert_value fatptr capture failed: {:?}", e))?;
                if let inkwell::values::AggregateValueEnum::StructValue(sv) = res {
                    return Ok(sv.into());
                }
            }
        }
        // { i64, ptr } container slot <-> { ptr, ptr } interface fat pointer:
        // reshape the length word field by field (LLVM forbids struct
        // bitcasts). Restrict to these shapes so unrelated structs are not
        // silently reinterpreted.
        if v.is_struct_value() && target_ty.is_struct_type() {
            if let BasicValueEnum::StructValue(sv) = v {
                let from_st = sv.get_type();
                let to_st = target_ty.into_struct_type();
                if is_container_or_fat_ptr_shape(from_st)
                    && is_container_or_fat_ptr_shape(to_st)
                {
                    let reshaped =
                        reshape_container_or_fat_ptr(&self.builder, sv, to_st)?;
                    return Ok(reshaped.into());
                }
            }
        }
        // A `null` literal in a struct slot (e.g. `return null` from a
        // tuple-returning function) carries no fields. Zero the slot instead
        // of rejecting the cast; a non-null pointer is never discarded here.
        if v.is_pointer_value() && target_ty.is_struct_type() && v.into_pointer_value().is_null() {
            return Ok(target_ty.into_struct_type().const_zero().into());
        }
        // float -> pointer: bitcast to a same-width int, then inttoptr.
        // (int -> pointer is handled above.)
        if v.is_float_value() && target_ty.is_pointer_type() {
            let from = v.into_float_value();
            let bits = from.get_type().get_bit_width();
            let int_ty = match bits {
                16 => self.context.i16_type(),
                32 => self.context.i32_type(),
                64 => self.context.i64_type(),
                128 => self.context.i128_type(),
                _ => {
                    return Err(format!(
                        "codegen: cannot convert f{} to pointer",
                        bits
                    ))
                }
            };
            let as_int = self
                .builder
                .build_bit_cast(from, int_ty, "cast.ftoi.ptr")
                .map_err(|e| format!("build_bit_cast float-to-int failed: {:?}", e))?;
            let cast = self
                .builder
                .build_int_to_ptr(
                    as_int.into_int_value(),
                    target_ty.into_pointer_type(),
                    "cast.itop",
                )
                .map_err(|e| format!("build_int_to_ptr failed: {:?}", e))?;
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

    /// Decode the UTF-8 scalar held in a single-character string. Loads are
    /// clamped to the last valid byte so short strings never read past the end.
    fn build_string_scalar(
        &self,
        len: IntValue<'ctx>,
        data: inkwell::values::PointerValue<'ctx>,
    ) -> Result<IntValue<'ctx>, String> {
        let i8_ty = self.context.i8_type();
        let i64_ty = self.context.i64_type();
        let zero = i64_ty.const_zero();
        let one = i64_ty.const_int(1, false);
        let nonempty = self
            .builder
            .build_int_compare(inkwell::IntPredicate::SGT, len, zero, "str.scalar.nonempty")
            .map_err(|e| format!("str scalar nonempty failed: {:?}", e))?;
        let dec = self
            .builder
            .build_int_sub(len, one, "str.scalar.last")
            .map_err(|e| format!("str scalar decrement failed: {:?}", e))?;
        let last = self
            .builder
            .build_select(nonempty, dec, zero, "str.scalar.clamped")
            .map_err(|e| format!("str scalar clamp failed: {:?}", e))?
            .into_int_value();

        let byte_at = |i: u64, name: &str| -> Result<IntValue<'ctx>, String> {
            let idx = i64_ty.const_int(i, false);
            let in_bounds = self
                .builder
                .build_int_compare(inkwell::IntPredicate::ULT, idx, len, "str.scalar.inbounds")
                .map_err(|e| format!("str scalar bounds failed: {:?}", e))?;
            let offset = self
                .builder
                .build_select(in_bounds, idx, last, "str.scalar.offset")
                .map_err(|e| format!("str scalar offset failed: {:?}", e))?
                .into_int_value();
            let ptr = unsafe { self.builder.build_gep(i8_ty, data, &[offset], "str.scalar.ptr") }
                .map_err(|e| format!("str scalar gep failed: {:?}", e))?;
            let byte = self
                .builder
                .build_load(i8_ty, ptr, name)
                .map_err(|e| format!("str scalar load failed: {:?}", e))?
                .into_int_value();
            self.builder
                .build_int_z_extend(byte, i64_ty, "str.scalar.byte64")
                .map_err(|e| format!("str scalar zext failed: {:?}", e))
        };
        let b0 = byte_at(0, "str.scalar.b0")?;
        let b1 = byte_at(1, "str.scalar.b1")?;
        let b2 = byte_at(2, "str.scalar.b2")?;
        let b3 = byte_at(3, "str.scalar.b3")?;

        let masked = |v: IntValue<'ctx>, mask: u64, name: &str| -> Result<IntValue<'ctx>, String> {
            self.builder
                .build_and(v, i64_ty.const_int(mask, false), name)
                .map_err(|e| format!("str scalar mask failed: {:?}", e))
        };
        let shifted = |v: IntValue<'ctx>, bits: u64, name: &str| -> Result<IntValue<'ctx>, String> {
            self.builder
                .build_left_shift(v, i64_ty.const_int(bits, false), name)
                .map_err(|e| format!("str scalar shift failed: {:?}", e))
        };
        let join = |a: IntValue<'ctx>, b: IntValue<'ctx>, name: &str| -> Result<IntValue<'ctx>, String> {
            self.builder
                .build_or(a, b, name)
                .map_err(|e| format!("str scalar join failed: {:?}", e))
        };
        let at_least = |v: IntValue<'ctx>, threshold: u64, name: &str| -> Result<IntValue<'ctx>, String> {
            self.builder
                .build_int_compare(
                    inkwell::IntPredicate::UGE,
                    v,
                    i64_ty.const_int(threshold, false),
                    name,
                )
                .map_err(|e| format!("str scalar compare failed: {:?}", e))
        };
        let choose = |cond: IntValue<'ctx>, yes: IntValue<'ctx>, no: IntValue<'ctx>, name: &str| {
            self.builder
                .build_select(cond, yes, no, name)
                .map(|v| v.into_int_value())
                .map_err(|e| format!("str scalar select failed: {:?}", e))
        };

        let c1 = masked(b0, 0x7F, "str.scalar.c1")?;
        let c2 = join(
            shifted(masked(b0, 0x1F, "str.scalar.c2.hi")?, 6, "str.scalar.c2.sh")?,
            masked(b1, 0x3F, "str.scalar.c2.lo")?,
            "str.scalar.c2",
        )?;
        let c3 = join(
            join(
                shifted(masked(b0, 0x0F, "str.scalar.c3.hi")?, 12, "str.scalar.c3.sh")?,
                shifted(masked(b1, 0x3F, "str.scalar.c3.mid")?, 6, "str.scalar.c3.msh")?,
                "str.scalar.c3.hi2",
            )?,
            masked(b2, 0x3F, "str.scalar.c3.lo")?,
            "str.scalar.c3",
        )?;
        let c4 = join(
            join(
                join(
                    shifted(masked(b0, 0x07, "str.scalar.c4.hi")?, 18, "str.scalar.c4.sh")?,
                    shifted(masked(b1, 0x3F, "str.scalar.c4.b1")?, 12, "str.scalar.c4.b1sh")?,
                    "str.scalar.c4.hi2",
                )?,
                shifted(masked(b2, 0x3F, "str.scalar.c4.b2")?, 6, "str.scalar.c4.b2sh")?,
                "str.scalar.c4.hi3",
            )?,
            masked(b3, 0x3F, "str.scalar.c4.lo")?,
            "str.scalar.c4",
        )?;

        let two = choose(at_least(b0, 0xC0, "str.scalar.two")?, c2, c1, "str.scalar.2")?;
        let three = choose(at_least(b0, 0xE0, "str.scalar.three")?, c3, two, "str.scalar.3")?;
        let four = choose(at_least(b0, 0xF0, "str.scalar.four")?, c4, three, "str.scalar.4")?;
        let empty = self
            .builder
            .build_int_compare(inkwell::IntPredicate::EQ, len, zero, "str.scalar.empty")
            .map_err(|e| format!("str scalar empty failed: {:?}", e))?;
        choose(empty, zero, four, "str.scalar.value")
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
                return Err("codegen: ternary condition must be bool".into());
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
                        .ok_or("codegen: ternary block has no terminator")?
                        .erase_from_basic_block();
                    let lv_coerced = if lv.get_type() == target {
                        lv
                    } else {
                        self.builder
                            .build_int_s_extend(lv, target, "coerce.then")
                            .map_err(|e| format!("coerce then failed: {:?}", e))?
                    };
                    self.builder
                        .build_unconditional_branch(end_block)
                        .map_err(|e| format!("rebuild br then failed: {:?}", e))?;

                    // Rebuild else block with coercion
                    self.builder.position_at_end(else_block_end);
                    else_block_end
                        .get_terminator()
                        .ok_or("codegen: ternary block has no terminator")?
                        .erase_from_basic_block();
                    let rv_coerced = if rv.get_type() == target {
                        rv
                    } else {
                        self.builder
                            .build_int_s_extend(rv, target, "coerce.else")
                            .map_err(|e| format!("coerce else failed: {:?}", e))?
                    };
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
                        .ok_or("codegen: ternary block has no terminator")?
                        .erase_from_basic_block();
                    let lv_coerced = if lv.get_type() == target {
                        lv
                    } else {
                        self.builder
                            .build_float_ext(lv, target, "coerce.then")
                            .map_err(|e| format!("coerce then failed: {:?}", e))?
                    };
                    self.builder
                        .build_unconditional_branch(end_block)
                        .map_err(|e| format!("rebuild br then failed: {:?}", e))?;

                    self.builder.position_at_end(else_block_end);
                    else_block_end
                        .get_terminator()
                        .ok_or("codegen: ternary block has no terminator")?
                        .erase_from_basic_block();
                    let rv_coerced = if rv.get_type() == target {
                        rv
                    } else {
                        self.builder
                            .build_float_ext(rv, target, "coerce.else")
                            .map_err(|e| format!("coerce else failed: {:?}", e))?
                    };
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
                        .ok_or("codegen: ternary block has no terminator")?
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
                        .ok_or("codegen: ternary block has no terminator")?
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
                    if sv.get_type().count_fields() == 2
                        && sv.get_type().get_field_type_at_index(0).is_some_and(|t| t.is_int_type()) =>
                {
                    let target = rv.get_type();
                    self.builder.position_at_end(then_block_end);
                    then_block_end
                        .get_terminator()
                        .ok_or("codegen: ternary block has no terminator")?
                        .erase_from_basic_block();
                    let f0 = self
                        .builder
                        .build_extract_value(sv, 0, "tern.sv.i64")
                        .map_err(|e| format!("extract failed: {:?}", e))?
                        .into_int_value();
                    let lv_coerced = self
                        .cast_value_to_type(f0.into(), target.as_basic_type_enum())?
                        .into_int_value();
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
                    if sv.get_type().count_fields() == 2
                        && sv.get_type().get_field_type_at_index(0).is_some_and(|t| t.is_int_type()) =>
                {
                    let target = lv.get_type();
                    self.builder.position_at_end(else_block_end);
                    else_block_end
                        .get_terminator()
                        .ok_or("codegen: ternary block has no terminator")?
                        .erase_from_basic_block();
                    let f0 = self
                        .builder
                        .build_extract_value(sv, 0, "tern.sv.i64")
                        .map_err(|e| format!("extract failed: {:?}", e))?
                        .into_int_value();
                    let rv_coerced = self
                        .cast_value_to_type(f0.into(), target.as_basic_type_enum())?
                        .into_int_value();
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
        // Both incoming values must have the phi's exact LLVM type. A branch
        // that yields `null` lowers to an opaque pointer while the other yields
        // a `{ i64, ptr }` container; prefer the container type and zero it for
        // the null branch instead of feeding mismatched types to LLVM.
        let phi_ty = if then_val.is_pointer_value() && !else_val.is_pointer_value() {
            else_val.get_type()
        } else {
            then_val.get_type()
        };
        let then_in = if then_val.get_type() == phi_ty {
            then_val
        } else if then_val.is_pointer_value() && phi_ty.is_struct_type() {
            phi_ty.into_struct_type().const_zero().into()
        } else {
            self.cast_value_to_type(then_val, phi_ty)?
        };
        let else_in = if else_val.get_type() == phi_ty {
            else_val
        } else if else_val.is_pointer_value() && phi_ty.is_struct_type() {
            phi_ty.into_struct_type().const_zero().into()
        } else {
            self.cast_value_to_type(else_val, phi_ty)?
        };
        let phi = self
            .builder
            .build_phi(phi_ty, "tern.result")
            .map_err(|e| format!("build_phi ternary failed: {:?}", e))?;
        phi.add_incoming(&[(&then_in, then_block_end), (&else_in, else_block_end)]);

        Ok(phi.as_basic_value())
    }
}

#[cfg(test)]
mod tests {
    use super::super::LocalVar;
    use super::*;
    use crate::ast::{Literal, Span};

    fn backend_with_block<'ctx>(context: &'ctx Context) -> LlvmBackend<'ctx> {
        let mut backend = LlvmBackend::new(context, "assign_test");
        let fn_ty = context.void_type().fn_type(&[], false);
        let f = backend.module.add_function("assign_test_fn", fn_ty, None);
        let entry = context.append_basic_block(f, "entry");
        backend.builder.position_at_end(entry);
        backend
    }

    fn numeric_slot_ty<'ctx>(context: &'ctx Context) -> inkwell::types::StructType<'ctx> {
        context.struct_type(
            &[
                context.i64_type().into(),
                context.ptr_type(AddressSpace::default()).into(),
            ],
            false,
        )
    }

    fn box_bits_and_code<'ctx>(
        backend: &LlvmBackend<'ctx>,
        context: &Context,
        boxed: BasicValueEnum<'ctx>,
    ) -> (u64, u64) {
        let sv = boxed.into_struct_value();
        let tag = backend
            .builder
            .build_extract_value(sv, 0, "test.tag")
            .expect("extract tag")
            .into_int_value();
        let width_ptr = backend
            .builder
            .build_extract_value(sv, 1, "test.width")
            .expect("extract width")
            .into_pointer_value();
        // The width is encoded as `inttoptr i64 <code> to ptr`. LLVM may or
        // may not fold it into a constant, so read the code from the IR text.
        let text = width_ptr.print_to_string().to_string();
        let code = FLOAT_BOX_WIDTH_CODES
            .iter()
            .find(|(_, _, needle)| text.contains(needle))
            .map(|(code, _, _)| *code)
            .unwrap_or_else(|| panic!("no width code in '{}'", text));
        (
            tag.get_zero_extended_constant().expect("constant tag bits"),
            code,
        )
    }

    fn unboxed_text<'ctx>(
        backend: &LlvmBackend<'ctx>,
        boxed: BasicValueEnum<'ctx>,
        to_ty: inkwell::types::FloatType<'ctx>,
    ) -> String {
        backend
            .cast_value_to_type(boxed, to_ty.as_basic_type_enum())
            .expect("numeric slot should unbox")
            .into_float_value()
            .print_to_string()
            .to_string()
    }

    #[test]
    fn string_char_to_int_decodes_utf8_scalar() {
        use inkwell::execution_engine::JitFunction;
        use inkwell::OptimizationLevel;

        super::super::target_wrappers::initialize_x86();
        let cases: &[(&str, &[u8], i32)] = &[
            ("ascii", b"A", 65),
            ("two_byte", &[0xC3, 0xA9], 233),          // é
            ("three_byte", &[0xE2, 0x82, 0xAC], 8364), // €
        ];
        for (name, bytes, expected) in cases {
            let context = Context::create();
            let backend = LlvmBackend::new(&context, "utf8_scalar_test");
            let fn_ty = context.i32_type().fn_type(&[], false);
            let f = backend.module.add_function("utf8_scalar_fn", fn_ty, None);
            let entry = context.append_basic_block(f, "entry");
            backend.builder.position_at_end(entry);
            let arr_ty = context.i8_type().array_type(bytes.len() as u32);
            let global = backend.module.add_global(arr_ty, None, name);
            global.set_initializer(&context.const_string(bytes, false));
            global.set_constant(true);
            let slot = numeric_slot_ty(&context).const_named_struct(&[
                context.i64_type().const_int(bytes.len() as u64, false).into(),
                global.as_pointer_value().into(),
            ]);
            let casted = backend
                .cast_value_to_type(slot.into(), context.i32_type().into())
                .unwrap_or_else(|e| panic!("{} should cast to int: {}", name, e))
                .into_int_value();
            backend.builder.build_return(Some(&casted)).expect("return");
            assert!(backend.module.verify().is_ok(), "invalid IR for {}", name);
            let engine = backend
                .module
                .create_jit_execution_engine(OptimizationLevel::None)
                .unwrap_or_else(|e| panic!("JIT engine for {}: {:?}", name, e));
            type CastFn = unsafe extern "C" fn() -> i32;
            let f: JitFunction<CastFn> = unsafe { engine.get_function("utf8_scalar_fn") }
                .unwrap_or_else(|_| panic!("JIT function for {}", name));
            assert_eq!(unsafe { f.call() }, *expected, "{} decoded scalar", name);
        }
    }

    #[test]
    fn container_to_fat_ptr_coercion_roundtrips() {
        let context = Context::create();
        let backend = backend_with_block(&context);
        let ptr_ty = context.ptr_type(AddressSpace::default());
        let container = context.struct_type(&[context.i64_type().into(), ptr_ty.into()], false);
        let fat = context.struct_type(&[ptr_ty.into(), ptr_ty.into()], false);

        // Bare pointer -> fat pointer (a function/class pointer stored in a
        // closure/interface slot) must insert a null second word.
        let bare = ptr_ty.const_null();
        let as_fat = backend
            .cast_value_to_type(bare.into(), fat.into())
            .expect("ptr should coerce to fat pointer");
        assert_eq!(as_fat.get_type(), fat.into());
        let back = backend
            .cast_value_to_type(as_fat, ptr_ty.into())
            .expect("fat pointer should coerce back to ptr");
        assert_eq!(back.get_type(), ptr_ty.into());

        // Container <-> fat pointer must reshape field by field in both
        // directions without losing the data pointer.
        let data = ptr_ty.const_null();
        let slot = container.const_named_struct(&[
            context.i64_type().const_int(7, false).into(),
            data.into(),
        ]);
        let reshaped = backend
            .cast_value_to_type(slot.into(), fat.into())
            .expect("container should reshape to fat pointer");
        assert_eq!(reshaped.get_type(), fat.into());
        let restored = backend
            .cast_value_to_type(reshaped, container.into())
            .expect("fat pointer should reshape back to container");
        assert_eq!(restored.get_type(), container.into());
        backend.builder.build_return(None).expect("return");
        assert!(
            backend.module.verify().is_ok(),
            "invalid IR after container/fat-pointer coercion"
        );
    }

    #[test]
    fn null_ptr_zeroes_struct_return_slot() {
        let context = Context::create();
        let backend = backend_with_block(&context);
        let tuple = context.struct_type(
            &[numeric_slot_ty(&context).into(), context.i32_type().into()],
            false,
        );
        let ptr_ty = context.ptr_type(AddressSpace::default());
        let casted = backend
            .cast_value_to_type(ptr_ty.const_null().into(), tuple.into())
            .expect("null should zero a struct slot");
        assert_eq!(casted.get_type(), tuple.into());
    }

    #[test]
    fn double_boxes_into_i64_slot_preserving_bits() {
        let context = Context::create();
        let backend = backend_with_block(&context);
        let value = context.f64_type().const_float(3.5);
        let boxed = backend
            .cast_value_to_type(value.into(), numeric_slot_ty(&context).into())
            .expect("double should box into { i64, ptr }");
        let (bits, code) = box_bits_and_code(&backend, &context, boxed);
        assert_eq!(bits, 0x400c_0000_0000_0000);
        assert_eq!(code, FLOAT_BOX_DOUBLE);
    }

    #[test]
    fn float_fielded_struct_to_int_reports_precise_error() {
        let context = Context::create();
        let backend = backend_with_block(&context);
        let st = context.struct_type(
            &[
                context.f64_type().into(),
                context.ptr_type(AddressSpace::default()).into(),
            ],
            false,
        );
        let value = st.const_named_struct(&[
            context.f64_type().const_float(1.0).into(),
            context.ptr_type(AddressSpace::default()).const_null().into(),
        ]);

        let err = backend
            .cast_value_to_type(value.into(), context.i64_type().into())
            .expect_err("a float field must not be read as an int");
        assert!(err.contains("cannot convert"), "{}", err);
    }

    #[test]
    fn double_roundtrips_through_numeric_slot_preserving_bits() {
        let context = Context::create();
        let backend = backend_with_block(&context);
        let slot = numeric_slot_ty(&context);
        let boxed = backend
            .cast_value_to_type(context.f64_type().const_float(3.5).into(), slot.into())
            .expect("double should box");
        let back = unboxed_text(&backend, boxed, context.f64_type());
        assert!(back.contains("3.5") || back.contains("bitcast i64"), "{}", back);
        backend.builder.build_return(None).expect("return");
        assert!(backend.module.verify().is_ok(), "invalid IR after double unbox");
    }

    #[test]
    fn float_roundtrips_through_i64_slot_without_becoming_double() {
        let context = Context::create();
        let backend = backend_with_block(&context);
        let boxed = backend
            .cast_value_to_type(
                context.f32_type().const_float(2.5).into(),
                numeric_slot_ty(&context).into(),
            )
            .expect("float should box into { i64, ptr }");
        let (bits, code) = box_bits_and_code(&backend, &context, boxed);
        assert_eq!(bits, 0x4020_0000, "raw f32 bits must be preserved");
        assert_eq!(code, FLOAT_BOX_FLOAT, "box must record the f32 width");

        let back = unboxed_text(&backend, boxed, context.f32_type());
        assert!(back.contains("2.5") || back.contains("bitcast i32"), "{}", back);
        let ir = backend.module.print_to_string().to_string();
        assert!(
            !ir.contains("bitcast i64 1075838976 to double"),
            "f32 bits were reinterpreted as f64: {}",
            ir
        );
        backend.builder.build_return(None).expect("return");
        assert!(backend.module.verify().is_ok(), "invalid IR after float unbox");
    }

    #[test]
    fn boxed_float_widens_to_double_using_recorded_width() {
        let context = Context::create();
        let backend = backend_with_block(&context);
        let boxed = backend
            .cast_value_to_type(
                context.f32_type().const_float(2.5).into(),
                numeric_slot_ty(&context).into(),
            )
            .expect("float should box");
        let back = unboxed_text(&backend, boxed, context.f64_type());
        let ir = backend.module.print_to_string().to_string();
        assert!(
            back.contains("2.5") || ir.contains("fpext (bitcast i32"),
            "expected f32 bits widened from float, got {}\n{}",
            back,
            ir
        );
        assert!(
            !ir.contains("bitcast i64 1075838976 to double"),
            "zero-extended f32 bits were read as an f64: {}",
            ir
        );
        backend.builder.build_return(None).expect("return");
        assert!(backend.module.verify().is_ok(), "invalid IR after widening unbox");
    }

    #[test]
    fn narrow_float_box_widens_to_double() {
        let context = Context::create();
        let backend = backend_with_block(&context);
        let slot = context.struct_type(
            &[
                context.i32_type().into(),
                context.ptr_type(AddressSpace::default()).into(),
            ],
            false,
        );
        let boxed = backend
            .cast_value_to_type(context.f32_type().const_float(2.5).into(), slot.into())
            .expect("float should box into an i32 tag slot");
        let (bits, code) = box_bits_and_code(&backend, &context, boxed);
        assert_eq!(bits, 0x4020_0000);
        assert_eq!(code, FLOAT_BOX_FLOAT);
        let back = unboxed_text(&backend, boxed, context.f64_type());
        assert!(back.contains("2.5") || back.contains("fpext"), "{}", back);
    }

    #[test]
    fn half_roundtrips_through_i64_slot() {
        let context = Context::create();
        let backend = backend_with_block(&context);
        let boxed = backend
            .cast_value_to_type(
                context.f16_type().const_float(1.5).into(),
                numeric_slot_ty(&context).into(),
            )
            .expect("half should box into { i64, ptr }");
        let (bits, code) = box_bits_and_code(&backend, &context, boxed);
        assert_eq!(bits, 0x3e00, "raw f16 bits must be preserved");
        assert_eq!(code, FLOAT_BOX_HALF);
        let back = unboxed_text(&backend, boxed, context.f16_type());
        // LLVM versions render an f16 constant as `1.5`, `0xH3E00` or via a
        // truncating bitcast; accept any spelling that carries the exact bits.
        assert!(
            back.contains("1.5") || back.contains("fptrunc") || back.contains("3E00"),
            "{}",
            back
        );
    }

    #[test]
    fn quad_roundtrips_through_i128_slot() {
        let context = Context::create();
        let backend = backend_with_block(&context);
        let slot = context.struct_type(
            &[
                context.i128_type().into(),
                context.ptr_type(AddressSpace::default()).into(),
            ],
            false,
        );
        let boxed = backend
            .cast_value_to_type(context.f128_type().const_float(2.5).into(), slot.into())
            .expect("quad should box into an i128 tag slot");
        let width_ptr = backend
            .builder
            .build_extract_value(boxed.into_struct_value(), 1, "test.width")
            .expect("extract width")
            .into_pointer_value();
        let text = width_ptr.print_to_string().to_string();
        let code = FLOAT_BOX_WIDTH_CODES
            .iter()
            .find(|(_, _, needle)| text.contains(needle))
            .map(|(code, _, _)| *code)
            .unwrap_or_else(|| panic!("no width code in '{}'", text));
        assert_eq!(code, FLOAT_BOX_QUAD);
        let back = unboxed_text(&backend, boxed, context.f128_type());
        assert!(back.contains("fp128"), "{}", back);
        backend.builder.build_return(None).expect("return");
        assert!(backend.module.verify().is_ok(), "invalid IR after quad unbox");
    }

    #[test]
    fn float_box_refuses_narrow_tag_field() {
        let context = Context::create();
        let backend = backend_with_block(&context);
        let narrow = context.struct_type(
            &[
                context.i32_type().into(),
                context.ptr_type(AddressSpace::default()).into(),
            ],
            false,
        );

        let err = backend
            .cast_value_to_type(context.f64_type().const_float(3.5).into(), narrow.into())
            .expect_err("narrowing the tag would lose bits");
        assert!(err.contains("cannot preserve f64 bits"), "{}", err);

        let quad = backend
            .cast_value_to_type(
                context.f128_type().const_float(2.5).into(),
                numeric_slot_ty(&context).into(),
            )
            .expect_err("quad cannot fit in an i64 tag");
        assert!(quad.contains("cannot preserve f128 bits"), "{}", quad);
    }

    #[test]
    fn ternary_wider_integer_branch_is_not_re_extended() {
        let context = Context::create();
        let mut backend = LlvmBackend::new(&context, "ternary-test");
        let f = backend
            .module
            .add_function("tern", context.i64_type().fn_type(&[], false), None);
        let entry = context.append_basic_block(f, "entry");
        backend.builder.position_at_end(entry);

        let i64_ty = context.i64_type();
        let ptr = backend
            .builder
            .build_alloca(i64_ty, "wide")
            .expect("alloca wide");
        backend
            .builder
            .build_store(ptr, i64_ty.const_int(7, false))
            .expect("store wide");
        backend.locals.insert(
            "wide".to_string(),
            LocalVar {
                ptr,
                ty: i64_ty.into(),
                titrate_type: None,
                full_type: None,
                shared: false,
                closure_sig: None,
            },
        );

        let cond = Expr::Literal(Literal::Bool(true), Span::unknown());
        let then_expr = Expr::Identifier("wide".to_string(), Span::unknown());
        let else_expr = Expr::Literal(Literal::Int(2), Span::unknown());
        let value = backend
            .compile_ternary(&cond, &then_expr, &else_expr)
            .expect("ternary should compile");
        backend
            .builder
            .build_return(Some(&value))
            .expect("return ternary result");
        assert!(backend.module.verify().is_ok(), "invalid IR after ternary");
    }

    #[test]
    fn non_numeric_struct_target_reports_precise_error() {
        let context = Context::create();
        let backend = backend_with_block(&context);
        let value = context.f64_type().const_float(3.5);
        let bad = context.struct_type(
            &[context.f64_type().into(), context.f64_type().into()],
            false,
        );
        let err = backend
            .cast_value_to_type(value.into(), bad.into())
            .expect_err("invalid target should error");
        assert!(
            err.contains("cannot convert value of type 'double'"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn boxed_int_uses_non_null_numeric_marker() {
        let context = Context::create();
        let backend = backend_with_block(&context);
        let slot = numeric_slot_ty(&context);
        let boxed = backend
            .cast_value_to_type(context.i64_type().const_int(42, false).into(), slot.into())
            .expect("int should box into { i64, ptr }");
        let text = boxed.print_to_string().to_string();
        assert!(
            text.contains(NUMERIC_BOX_GLOBAL),
            "boxed number must not carry a null data word: {}",
            text
        );
        let tag = backend
            .builder
            .build_extract_value(boxed.into_struct_value(), 0, "test.tag")
            .expect("extract tag")
            .into_int_value();
        assert_eq!(tag.get_zero_extended_constant(), Some(42));
        backend.builder.build_return(None).expect("return");
        assert!(backend.module.verify().is_ok(), "invalid IR after int box");
    }

    #[test]
    fn numeric_marker_box_unboxes_to_original_int() {
        use inkwell::execution_engine::JitFunction;
        use inkwell::OptimizationLevel;

        super::super::target_wrappers::initialize_x86();
        let context = Context::create();
        let backend = LlvmBackend::new(&context, "numeric_marker_unbox");
        let fn_ty = context.i32_type().fn_type(&[], false);
        let f = backend.module.add_function("numeric_marker_unbox_fn", fn_ty, None);
        let entry = context.append_basic_block(f, "entry");
        backend.builder.position_at_end(entry);

        let slot = numeric_slot_ty(&context);
        let boxed = backend
            .cast_value_to_type(context.i64_type().const_int(42, false).into(), slot.into())
            .expect("int should box");
        let back = backend
            .cast_value_to_type(boxed, context.i32_type().into())
            .expect("numeric marker box should unbox")
            .into_int_value();
        backend.builder.build_return(Some(&back)).expect("return");
        assert!(backend.module.verify().is_ok(), "invalid IR after unbox");

        let engine = backend
            .module
            .create_jit_execution_engine(OptimizationLevel::None)
            .expect("JIT engine");
        type UnboxFn = unsafe extern "C" fn() -> i32;
        let unbox: JitFunction<UnboxFn> = unsafe { engine.get_function("numeric_marker_unbox_fn") }
            .expect("JIT function");
        assert_eq!(unsafe { unbox.call() }, 42, "unboxed number must survive");
    }
}
