//! Tuple codegen: anonymous structs, construction, field access, and destructuring.
//!
//! Tuple layout:
//!   An anonymous LLVM struct type `{ element0_ty, element1_ty, ... }`.
//!
//! Construction: `(expr0, expr1, ...)`
//!   - Compiles each element expression and packs them into an anonymous struct.
//!
//! Field access: `tuple.0`, `tuple.1`, etc.
//!   - Uses `extractvalue` to get the element at the given index.
//!
//! Destructuring: `let (a, b) = tuple_expr`
//!   - Compiles the tuple expression, then extracts each element and stores
//!     it into the corresponding named variable.

use inkwell::context::Context;
use inkwell::types::BasicTypeEnum;
use inkwell::values::BasicValueEnum;
use inkwell::AddressSpace;

/// Emit a tuple construction: pack the given values into an anonymous struct.
///
/// Returns a `BasicValueEnum::StructValue` containing the packed elements.
pub fn emit_tuple_construct<'ctx>(
    context: &'ctx Context,
    builder: &inkwell::builder::Builder<'ctx>,
    _module: &inkwell::module::Module<'ctx>,
    elements: &[BasicValueEnum<'ctx>],
) -> Result<BasicValueEnum<'ctx>, String> {
    if elements.is_empty() {
        // Unit type: return an opaque value (e.g., i32 0).
        let i32_type = context.i32_type();
        return Ok(i32_type.const_int(0, false).into());
    }

    let elem_tys: Vec<BasicTypeEnum<'ctx>> = elements.iter().map(|v| v.get_type()).collect();
    let tuple_type = context.struct_type(&elem_tys, false);

    // Allocate space on the stack for the tuple.
    let alloca = builder
        .build_alloca(tuple_type, "tuple")
        .map_err(|e| format!("build_alloca tuple failed: {:?}", e))?;

    // Store each element into the struct field.
    for (i, elem) in elements.iter().enumerate() {
        let gep = builder
            .build_struct_gep(tuple_type, alloca, i as u32, &format!("tuple.{}.gep", i))
            .map_err(|e| format!("build_struct_gep tuple field {} failed: {:?}", i, e))?;
        builder
            .build_store(gep, *elem)
            .map_err(|e| format!("build_store tuple field {} failed: {:?}", i, e))?;
    }

    // Load the struct value.
    builder
        .build_load(tuple_type, alloca, "tuple.val")
        .map_err(|e| format!("build_load tuple failed: {:?}", e))
}

/// Emit access to a tuple field by index.
///
/// `tuple_val` is the struct value, `index` is the 0-based field index.
/// Returns the extracted field value.
pub fn emit_tuple_field_access<'ctx>(
    builder: &inkwell::builder::Builder<'ctx>,
    tuple_val: BasicValueEnum<'ctx>,
    index: u32,
) -> Result<BasicValueEnum<'ctx>, String> {
    let struct_val = match tuple_val {
        BasicValueEnum::StructValue(sv) => sv,
        _ => return Err(format!("tuple field access: expected struct value, got {:?}", tuple_val.get_type())),
    };

    builder
        .build_extract_value(struct_val, index, &format!("tuple.{}", index))
        .map_err(|e| format!("build_extract_value tuple field {} failed: {:?}", index, e))
}

/// Emit a tuple field access via pointer (for mutable tuple access or when
/// the tuple is stored in memory).
///
/// `tuple_ptr` is a pointer to the tuple struct, `tuple_type` is the LLVM
/// struct type, `index` is the 0-based field index.
pub fn emit_tuple_field_access_ptr<'ctx>(
    context: &'ctx Context,
    builder: &inkwell::builder::Builder<'ctx>,
    tuple_type: inkwell::types::StructType<'ctx>,
    tuple_ptr: inkwell::values::PointerValue<'ctx>,
    index: u32,
) -> Result<BasicValueEnum<'ctx>, String> {
    let ptr_type = context.ptr_type(AddressSpace::default());
    let struct_ptr = if tuple_ptr.get_type() == ptr_type {
        tuple_ptr
    } else {
        builder
            .build_bit_cast(tuple_ptr, ptr_type, "tuple.ptr.cast")
            .map_err(|e| format!("build_bit_cast tuple ptr failed: {:?}", e))?
            .into_pointer_value()
    };

    let gep = builder
        .build_struct_gep(tuple_type, struct_ptr, index, &format!("tuple.{}.gep", index))
        .map_err(|e| format!("build_struct_gep tuple field {} failed: {:?}", index, e))?;

    let field_types = tuple_type.get_field_types();
    let field_ty = field_types
        .get(index as usize)
        .copied()
        .ok_or_else(|| format!("tuple field index {} out of bounds", index))?;

    builder
        .build_load(field_ty, gep, &format!("tuple.{}", index))
        .map_err(|e| format!("build_load tuple field {} failed: {:?}", index, e))
}

/// Emit a store into a tuple field via pointer.
pub fn emit_tuple_field_store<'ctx>(
    context: &'ctx Context,
    builder: &inkwell::builder::Builder<'ctx>,
    tuple_type: inkwell::types::StructType<'ctx>,
    tuple_ptr: inkwell::values::PointerValue<'ctx>,
    index: u32,
    value: BasicValueEnum<'ctx>,
) -> Result<(), String> {
    let ptr_type = context.ptr_type(AddressSpace::default());
    let struct_ptr = if tuple_ptr.get_type() == ptr_type {
        tuple_ptr
    } else {
        builder
            .build_bit_cast(tuple_ptr, ptr_type, "tuple.ptr.cast")
            .map_err(|e| format!("build_bit_cast tuple ptr failed: {:?}", e))?
            .into_pointer_value()
    };

    let gep = builder
        .build_struct_gep(tuple_type, struct_ptr, index, &format!("tuple.{}.gep", index))
        .map_err(|e| format!("build_struct_gep tuple field {} failed: {:?}", index, e))?;

    builder
        .build_store(gep, value)
        .map_err(|e| format!("build_store tuple field {} failed: {:?}", index, e))?;
    Ok(())
}