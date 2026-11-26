// Assignment, casts, and ternaries

use super::types as llvm_types;
use super::vtable::{
    emit_field_store, field_index, is_container_or_fat_ptr_shape, reshape_container_or_fat_ptr,
};
use super::{ClosureSig, LlvmBackend};
use crate::ast::Expr;
use inkwell::context::Context;
use inkwell::AddressSpace;
use inkwell::types::{BasicType, BasicTypeEnum, FloatType, IntType};
use inkwell::values::{BasicValueEnum, FloatValue, IntValue};
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
        // string struct { i64 len, ptr data } -> int: a single-character string
        // yields that character's code (e.g. JsonValue.escapeString uses
        // `String.charAt(c, 0) as int`). The generic struct->int path below
        // would return the string length (always 1) instead.
        if v.is_struct_value() && target_ty.is_int_type() && self.is_string_struct(&v) {
            if let BasicValueEnum::StructValue(sv) = v {
                if sv.get_type().count_fields() == 2 {
                    if let (Some(len_v), Some(ptr_v)) = (
                        self.builder.build_extract_value(sv, 0, "cast.str.len").ok(),
                        self.builder.build_extract_value(sv, 1, "cast.str.ptr").ok(),
                    ) {
                        let len_i64 = len_v.into_int_value();
                        if let BasicValueEnum::PointerValue(data) = ptr_v {
                            let one = self.context.i64_type().const_int(1, false);
                            let is_one = self
                                .builder
                                .build_int_compare(inkwell::IntPredicate::EQ, len_i64, one, "cast.str.isone")
                                .map_err(|e| format!("cast str isone failed: {:?}", e))?;
                            let byte = self
                                .builder
                                .build_load(self.context.i8_type(), data, "cast.str.byte")
                                .map_err(|e| format!("cast str byte load failed: {:?}", e))?
                                .into_int_value();
                            let to_ty = target_ty.into_int_type();
                            let code = if byte.get_type().get_bit_width() < to_ty.get_bit_width() {
                                self.builder.build_int_z_extend(byte, to_ty, "cast.str.code")
                            } else {
                                self.builder.build_int_truncate(byte, to_ty, "cast.str.code")
                            }
                            .map_err(|e| format!("cast str code failed: {:?}", e))?;
                            let fallback = if len_i64.get_type().get_bit_width() > to_ty.get_bit_width() {
                                self.builder.build_int_truncate(len_i64, to_ty, "cast.str.fallback")
                            } else {
                                Ok(len_i64)
                            }
                            .map_err(|e| format!("cast str fallback failed: {:?}", e))?;
                            let result = self
                                .builder
                                .build_select(is_one, code, fallback, "cast.str.value")
                                .map_err(|e| format!("cast str select failed: {:?}", e))?;
                            return Ok(result);
                        }
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
        // struct -> pointer: extract field 1 (data ptr) from {i64, ptr} struct.
        if v.is_struct_value() && target_ty.is_pointer_type() && self.is_string_struct(&v) {
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
        // int -> Result struct { i32, ptr }: native Result-returning calls are
        // unmarshalled as a bare i32 handle; wrap it as a Result value. Most
        // imported stdlib functions only need a well-formed struct here.
        if v.is_int_value() && target_ty.is_struct_type() {
            let st = target_ty.into_struct_type();
            if st.count_fields() == 2
                && st.get_field_type_at_index(0).is_some_and(|t| t.is_int_type())
                && st.get_field_type_at_index(1).is_some_and(|t| t.is_pointer_type())
            {
                let field_ty = st
                    .get_field_type_at_index(0)
                    .ok_or_else(|| format!("codegen: '{}' has no tag field", st.print_to_string()))?
                    .into_int_type();
                let tag = self.cast_value_to_type(v, field_ty.into())?.into_int_value();
                let ptr_ty = self.context.ptr_type(AddressSpace::default());
                let res = self.builder.build_insert_value(
                    st.const_zero(), tag, 0, "cast.result.tag")
                    .map_err(|e| format!("build_insert_value result tag failed: {:?}", e))?;
                let res = self.builder.build_insert_value(
                    res, ptr_ty.const_null(), 1, "cast.result.ptr")
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
        let phi = self
            .builder
            .build_phi(then_val.get_type(), "tern.result")
            .map_err(|e| format!("build_phi ternary failed: {:?}", e))?;
        phi.add_incoming(&[(&then_val, then_block_end), (&else_val, else_block_end)]);

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
        assert!(back.contains("1.5") || back.contains("fptrunc"), "{}", back);
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
}
