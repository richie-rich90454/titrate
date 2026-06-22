//! Enum codegen: tag-payload layout, construction, and pattern matching.
//!
//! Enum layout:
//!   { i32 tag, i8* payload }
//!
//!   - tag: 0 for the first variant, 1 for the second, etc.
//!   - payload: for simple variants (no fields), this is null.
//!     For variants with fields, this points to a heap-allocated struct
//!     containing the variant's fields.
//!
//! Construction: `EnumName::Variant(args)` or `EnumName::Variant`
//!   - Allocates the enum struct on the heap via titrate_malloc.
//!   - Sets the tag and optionally allocates and stores the payload.
//!
//! Pattern matching: `switch (value) { case Variant(x) => ... }`
//!   - Extracts the tag, branches on it, and extracts payload fields.

use inkwell::context::Context;
use inkwell::types::BasicTypeEnum;
use inkwell::values::{BasicValueEnum, PointerValue};
use inkwell::AddressSpace;

use crate::ast::EnumDecl;

/// Information about a compiled enum.
#[derive(Clone)]
pub struct EnumInfo<'ctx> {
    /// The enum name.
    pub name: String,
    /// LLVM struct type for the enum: { i32, i8* }.
    pub struct_type: inkwell::types::StructType<'ctx>,
    /// Variant names in declaration order.
    pub variant_names: Vec<String>,
    /// For each variant, the field types (empty vec for simple variants).
    pub variant_fields: Vec<Vec<(String, BasicTypeEnum<'ctx>)>>,
    /// For each variant with fields, the LLVM struct type for the payload.
    pub variant_payload_types: Vec<Option<inkwell::types::StructType<'ctx>>>,
}

/// Build the LLVM struct type for an enum: { i32, i8* }.
pub fn build_enum_struct_type<'ctx>(context: &'ctx Context) -> inkwell::types::StructType<'ctx> {
    let i32_type = context.i32_type();
    let i8_ptr = context.ptr_type(AddressSpace::default());
    context.struct_type(&[i32_type.into(), i8_ptr.into()], false)
}

/// Get the tag value (i32) for a variant by name.
pub fn variant_tag(variant_names: &[String], name: &str) -> Option<u32> {
    variant_names.iter().position(|n| n == name).map(|i| i as u32)
}

/// Emit construction of an enum variant.
///
/// Returns a pointer to the heap-allocated enum struct.
pub fn emit_enum_construct<'ctx>(
    context: &'ctx Context,
    builder: &inkwell::builder::Builder<'ctx>,
    module: &inkwell::module::Module<'ctx>,
    enum_info: &EnumInfo<'ctx>,
    variant_name: &str,
    args: &[BasicValueEnum<'ctx>],
) -> Result<BasicValueEnum<'ctx>, String> {
    let i32_type = context.i32_type();
    let i8_ptr = context.ptr_type(AddressSpace::default());
    let _i64_type = context.i64_type();

    let tag = variant_tag(&enum_info.variant_names, variant_name)
        .ok_or_else(|| format!("variant '{}' not found in enum '{}'", variant_name, enum_info.name))?;

    // Allocate the enum struct on the heap.
    let malloc_fn = module
        .get_function("titrate_malloc")
        .ok_or("titrate_malloc not declared")?;
    let size = enum_info
        .struct_type
        .size_of()
        .ok_or_else(|| format!("cannot compute size of enum '{}'", enum_info.name))?;
    let call = builder
        .build_call(malloc_fn, &[size.into()], "enum.alloc")
        .map_err(|e| format!("build_call titrate_malloc failed: {:?}", e))?;
    let enum_ptr = match call.try_as_basic_value() {
        inkwell::values::ValueKind::Basic(v) => v.into_pointer_value(),
        _ => return Err("titrate_malloc did not return a value".to_string()),
    };

    // Store the tag (field 0).
    let tag_gep = builder
        .build_struct_gep(enum_info.struct_type, enum_ptr, 0, "enum.tag.gep")
        .map_err(|e| format!("build_struct_gep enum tag failed: {:?}", e))?;
    builder
        .build_store(tag_gep, i32_type.const_int(tag as u64, false))
        .map_err(|e| format!("build_store enum tag failed: {:?}", e))?;

    // If the variant has fields, allocate and store the payload.
    let variant_idx = tag as usize;
    if let Some(payload_type) = enum_info.variant_payload_types.get(variant_idx).and_then(|pt| *pt) {
        if !args.is_empty() {
            // Allocate the payload struct.
            let payload_size = payload_type
                .size_of()
                .ok_or_else(|| format!("cannot compute payload size for variant '{}'", variant_name))?;
            let payload_call = builder
                .build_call(malloc_fn, &[payload_size.into()], "payload.alloc")
                .map_err(|e| format!("build_call payload malloc failed: {:?}", e))?;
            let payload_ptr = match payload_call.try_as_basic_value() {
                inkwell::values::ValueKind::Basic(v) => v.into_pointer_value(),
                _ => return Err("payload malloc did not return a value".to_string()),
            };

            // Store each field into the payload struct.
            let field_names = &enum_info.variant_fields[variant_idx];
            for (i, (fname, _fty)) in field_names.iter().enumerate() {
                if i < args.len() {
                    let gep = builder
                        .build_struct_gep(payload_type, payload_ptr, i as u32, &format!("payload.{}.gep", fname))
                        .map_err(|e| format!("build_struct_gep payload field failed: {:?}", e))?;
                    builder
                        .build_store(gep, args[i])
                        .map_err(|e| format!("build_store payload field failed: {:?}", e))?;
                }
            }

            // Store the payload pointer in the enum (field 1).
            let payload_gep = builder
                .build_struct_gep(enum_info.struct_type, enum_ptr, 1, "enum.payload.gep")
                .map_err(|e| format!("build_struct_gep enum payload failed: {:?}", e))?;
            let payload_i8 = builder
                .build_bit_cast(payload_ptr, i8_ptr, "payload.cast")
                .map_err(|e| format!("build_bit_cast payload failed: {:?}", e))?;
            builder
                .build_store(payload_gep, payload_i8)
                .map_err(|e| format!("build_store enum payload failed: {:?}", e))?;
        }
    } else {
        // Simple variant: store null as payload.
        let payload_gep = builder
            .build_struct_gep(enum_info.struct_type, enum_ptr, 1, "enum.payload.gep")
            .map_err(|e| format!("build_struct_gep enum payload failed: {:?}", e))?;
        builder
            .build_store(payload_gep, i8_ptr.const_null())
            .map_err(|e| format!("build_store enum null payload failed: {:?}", e))?;
    }

    // Return the enum pointer (as i8*).
    let result = builder
        .build_bit_cast(enum_ptr, i8_ptr, "enum.result")
        .map_err(|e| format!("build_bit_cast enum result failed: {:?}", e))?;
    Ok(result)
}

/// Emit extraction of the tag from an enum value.
pub fn emit_enum_get_tag<'ctx>(
    builder: &inkwell::builder::Builder<'ctx>,
    enum_info: &EnumInfo<'ctx>,
    enum_ptr: PointerValue<'ctx>,
) -> Result<BasicValueEnum<'ctx>, String> {
    let tag_gep = builder
        .build_struct_gep(enum_info.struct_type, enum_ptr, 0, "tag.gep")
        .map_err(|e| format!("build_struct_gep tag failed: {:?}", e))?;
    builder
        .build_load(enum_info.struct_type.get_field_types()[0], tag_gep, "tag")
        .map_err(|e| format!("build_load tag failed: {:?}", e))
}

/// Emit extraction of a payload field from an enum value.
///
/// `variant_name` is the variant we're matching, `field_name` is the field to extract.
pub fn emit_enum_get_field<'ctx>(
    context: &'ctx Context,
    builder: &inkwell::builder::Builder<'ctx>,
    enum_info: &EnumInfo<'ctx>,
    enum_ptr: PointerValue<'ctx>,
    variant_name: &str,
    field_name: &str,
) -> Result<BasicValueEnum<'ctx>, String> {
    let variant_idx = variant_tag(&enum_info.variant_names, variant_name)
        .ok_or_else(|| format!("variant '{}' not found in enum '{}'", variant_name, enum_info.name))? as usize;

    let payload_type = enum_info.variant_payload_types[variant_idx]
        .ok_or_else(|| format!("variant '{}' has no payload fields", variant_name))?;

    let field_names = &enum_info.variant_fields[variant_idx];
    let field_idx = field_names
        .iter()
        .position(|(name, _)| name == field_name)
        .ok_or_else(|| format!("field '{}' not found in variant '{}'", field_name, variant_name))?;

    // Load the payload pointer (field 1 of enum struct).
    let i8_ptr = context.ptr_type(AddressSpace::default());
    let payload_gep = builder
        .build_struct_gep(enum_info.struct_type, enum_ptr, 1, "payload.gep")
        .map_err(|e| format!("build_struct_gep payload failed: {:?}", e))?;
    let payload_ptr = builder
        .build_load(i8_ptr, payload_gep, "payload")
        .map_err(|e| format!("build_load payload failed: {:?}", e))?
        .into_pointer_value();

    // Cast payload to the right struct type.
    let payload_typed = builder
        .build_bit_cast(payload_ptr, context.ptr_type(AddressSpace::default()), "payload.cast")
        .map_err(|e| format!("build_bit_cast payload cast failed: {:?}", e))?
        .into_pointer_value();

    // GEP to the field.
    let field_gep = builder
        .build_struct_gep(payload_type, payload_typed, field_idx as u32, &format!("{}.gep", field_name))
        .map_err(|e| format!("build_struct_gep field failed: {:?}", e))?;

    let field_ty = payload_type.get_field_types()[field_idx];
    builder
        .build_load(field_ty, field_gep, field_name)
        .map_err(|e| format!("build_load field failed: {:?}", e))
}

/// Compile an enum declaration: register the enum info and build payload types.
#[allow(unused_variables)]
pub fn compile_enum_decl<'ctx>(
    context: &'ctx Context,
    module: &inkwell::module::Module<'ctx>,
    enum_decl: &EnumDecl,
) -> Result<EnumInfo<'ctx>, String> {
    let struct_type = build_enum_struct_type(context);

    let mut variant_names: Vec<String> = Vec::new();
    let mut variant_fields: Vec<Vec<(String, BasicTypeEnum<'ctx>)>> = Vec::new();
    let mut variant_payload_types: Vec<Option<inkwell::types::StructType<'ctx>>> = Vec::new();

    for variant in &enum_decl.variants {
        variant_names.push(variant.name.clone());

        if variant.fields.is_empty() {
            variant_fields.push(Vec::new());
            variant_payload_types.push(None);
        } else {
            let mut fields: Vec<(String, BasicTypeEnum<'ctx>)> = Vec::new();
            let mut field_tys: Vec<BasicTypeEnum<'ctx>> = Vec::new();
            for param in &variant.fields {
                let ty = crate::codegen::llvm::types::llvm_type(context, &param.typ)?;
                fields.push((param.name.clone(), ty));
                field_tys.push(ty);
            }
            let payload_type = context.struct_type(&field_tys, false);
            variant_fields.push(fields);
            variant_payload_types.push(Some(payload_type));
        }
    }

    Ok(EnumInfo {
        name: enum_decl.name.clone(),
        struct_type,
        variant_names,
        variant_fields,
        variant_payload_types,
    })
}