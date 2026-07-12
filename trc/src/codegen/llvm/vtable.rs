//! Class struct layout, vtable emission, and class-related codegen.
//!
//! Class layout:
//!   { vtable_ptr: i8*, field0, field1, ... }
//!
//! Vtable layout:
//!   A global constant array of `i8*` function pointers, one per method,
//!   sorted alphabetically. The vtable pointer itself is used as a type
//!   tag for `is` checks (comparing vtable addresses).
//!
//! `new ClassName(args)`:
//!   Calls `titrate_malloc` with the struct size, then calls the
//!   constructor. The constructor is a regular function that takes
//!   `this` as its first parameter.
//!
//! Field access: `obj.field` via GEP on the class struct.
//!
//! Method calls: Direct calls to the function. Virtual calls load the
//! vtable pointer from the object, GEP to the method index, load the
//! function pointer, then call it via indirect call.

use std::collections::HashMap;

use inkwell::context::Context;
use inkwell::module::Linkage;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicValueEnum, PointerValue};
use inkwell::AddressSpace;

use crate::ast::FieldDecl;

/// Information about a compiled class.
#[derive(Clone)]
pub struct ClassInfo<'ctx> {
    /// The class name.
    pub name: String,
    /// LLVM struct type for the class instance (vtable_ptr + fields).
    pub struct_type: inkwell::types::StructType<'ctx>,
    /// Field names in declaration order, with their LLVM types.
    pub fields: Vec<(String, BasicTypeEnum<'ctx>)>,
    /// Method names from the class (sorted alphabetically for vtable indexing).
    pub method_names: Vec<String>,
    /// The LLVM function for the constructor (if any).
    pub constructor: Option<inkwell::values::FunctionValue<'ctx>>,
    /// The vtable global constant (if the class has methods).
    pub vtable_global: Option<inkwell::values::GlobalValue<'ctx>>,
    /// The parent class name (if any).
    #[allow(dead_code)]
    pub parent: Option<String>,
    /// Interface names this class implements.
    #[allow(dead_code)]
    pub ifaces: Vec<String>,
}

/// Build the LLVM struct type for a class.
///
/// The struct layout is: `{ i8* vtable_ptr, field0_ty, field1_ty, ... }`.
pub fn build_class_struct_type<'ctx>(
    context: &'ctx Context,
    _class_name: &str,
    fields: &[FieldDecl],
    field_types: &HashMap<String, BasicTypeEnum<'ctx>>,
    parent_struct_type: Option<inkwell::types::StructType<'ctx>>,
) -> inkwell::types::StructType<'ctx> {
    let i8_ptr = context.ptr_type(AddressSpace::default());
    let mut field_tys: Vec<BasicTypeEnum<'ctx>> = Vec::new();
    // First field is always the vtable pointer.
    field_tys.push(i8_ptr.into());

    // If there's a parent, embed its fields after the vtable ptr.
    if let Some(parent_st) = parent_struct_type {
        let parent_field_types = parent_st.get_field_types();
        // Skip the first field (vtable ptr) of the parent.
        for ft in parent_field_types.iter().skip(1) {
            field_tys.push(*ft);
        }
    }

    for field in fields {
        let ft = field_types.get(&field.name).copied().unwrap_or_else(|| {
            i8_ptr.into()
        });
        field_tys.push(ft);
    }

    context.struct_type(&field_tys, false)
}

/// Create the vtable global constant for a class.
///
/// The vtable is an array of `i8*` function pointers. Methods are sorted
/// alphabetically for deterministic ordering.
pub fn create_vtable_global<'ctx>(
    context: &'ctx Context,
    module: &inkwell::module::Module<'ctx>,
    class_name: &str,
    method_names: &[String],
    method_functions: &HashMap<String, inkwell::values::FunctionValue<'ctx>>,
) -> Option<inkwell::values::GlobalValue<'ctx>> {
    if method_names.is_empty() {
        return None;
    }

    let i8_ptr = context.ptr_type(AddressSpace::default());

    // Sort method names for deterministic ordering.
    let mut sorted_names: Vec<String> = method_names.to_vec();
    sorted_names.sort();

    let mut entries: Vec<PointerValue<'ctx>> = Vec::with_capacity(sorted_names.len());
    for mname in &sorted_names {
        let fn_val = method_functions.get(mname).unwrap_or_else(|| {
            panic!("vtable: method '{}' not found in class '{}'", mname, class_name)
        });
        let fn_ptr = fn_val.as_global_value().as_pointer_value();
        let fn_ptr_i8 = fn_ptr.const_cast(i8_ptr);
        entries.push(fn_ptr_i8);
    }

    let arr_type = i8_ptr.array_type(entries.len() as u32);
    let vtable_name = format!("_vtable_{}", class_name);
    let vtable = module.add_global(arr_type, None, &vtable_name);
    vtable.set_linkage(Linkage::Internal);
    vtable.set_constant(true);

    let const_arr = i8_ptr.const_array(&entries);
    vtable.set_initializer(&const_arr);

    Some(vtable)
}

/// Get the field index in the class struct (0-based, where 0 is the vtable ptr).
/// Returns the GEP index for the field.
pub fn field_index(class_info: &ClassInfo, field_name: &str) -> Option<u32> {
    // Field 0 is vtable ptr, so real fields start at index 1.
    class_info
        .fields
        .iter()
        .position(|(name, _)| name == field_name)
        .map(|i| (i + 1) as u32)
}

/// Get the method index in the vtable.
pub fn method_vtable_index(method_names: &[String], method_name: &str) -> Option<u32> {
    let mut sorted: Vec<&String> = method_names.iter().collect();
    sorted.sort();
    sorted.iter().position(|&n| n == method_name).map(|i| i as u32)
}

/// Emit a `new ClassName(...)` allocation.
///
/// Returns the `i8*` pointer to the allocated instance.
pub fn emit_new_allocation<'ctx>(
    context: &'ctx Context,
    builder: &inkwell::builder::Builder<'ctx>,
    module: &inkwell::module::Module<'ctx>,
    class_info: &ClassInfo<'ctx>,
    constructor_fn: Option<inkwell::values::FunctionValue<'ctx>>,
    args: &[BasicValueEnum<'ctx>],
) -> Result<BasicValueEnum<'ctx>, String> {
    let i8_ptr = context.ptr_type(AddressSpace::default());

    // Compute the size of the class struct.
    let size = class_info
        .struct_type
        .size_of()
        .ok_or_else(|| format!("cannot compute size of class '{}'", class_info.name))?;

    // Call titrate_malloc.
    let malloc_fn = module
        .get_function("titrate_malloc")
        .ok_or("titrate_malloc not declared")?;
    let call = builder
        .build_call(malloc_fn, &[size.into()], "new.ptr")
        .map_err(|e| format!("build_call titrate_malloc failed: {:?}", e))?;
    let instance_ptr = match call.try_as_basic_value() {
        inkwell::values::ValueKind::Basic(v) => v.into_pointer_value(),
        _ => return Err("titrate_malloc did not return a value".to_string()),
    };

    // Zero-initialize the allocated memory using llvm.memset so that all
    // fields start at their default (zero) value before the constructor
    // runs. This is faster than emitting per-field stores and avoids
    // use-of-uninitialized-memory issues.
    {
        let i8_ty = context.i8_type();
        let i64_ty = context.i64_type();
        let i1_ty = context.bool_type();
        let i8_ptr_ty = context.ptr_type(AddressSpace::default());
        let void_ty = context.void_type();
        let memset_fn = match module.get_function("llvm.memset.p0i8.i64") {
            Some(f) => f,
            None => {
                let fn_ty = void_ty.fn_type(
                    &[i8_ptr_ty.into(), i8_ty.into(), i64_ty.into(), i1_ty.into()],
                    false,
                );
                module.add_function("llvm.memset.p0i8.i64", fn_ty, None)
            }
        };
        let zero_val = i8_ty.const_int(0, false);
        let is_volatile = i1_ty.const_int(0, false);
        builder
            .build_call(
                memset_fn,
                &[
                    instance_ptr.into(),
                    zero_val.into(),
                    size.into(),
                    is_volatile.into(),
                ],
                "new.memset",
            )
            .map_err(|e| format!("build_call memset failed: {:?}", e))?;
    }

    // Cast the i8* to the class struct pointer type.
    let struct_ptr_type = context.ptr_type(AddressSpace::default());
    let struct_ptr = if instance_ptr.get_type() == struct_ptr_type {
        instance_ptr
    } else {
        builder
            .build_bit_cast(instance_ptr, struct_ptr_type, "new.cast")
            .map_err(|e| format!("build_bit_cast new.cast failed: {:?}", e))?
            .into_pointer_value()
    };

    // Store the vtable pointer in the first field.
    if let Some(vtable_global) = &class_info.vtable_global {
        let vtable_ptr = vtable_global.as_pointer_value();
        let vtable_i8 = builder
            .build_bit_cast(vtable_ptr, i8_ptr, "vtable.cast")
            .map_err(|e| format!("build_bit_cast vtable failed: {:?}", e))?;
        let vtable_gep = builder
            .build_struct_gep(class_info.struct_type, struct_ptr, 0, "vtable.gep")
            .map_err(|e| format!("build_struct_gep vtable failed: {:?}", e))?;
        builder
            .build_store(vtable_gep, vtable_i8)
            .map_err(|e| format!("build_store vtable failed: {:?}", e))?;
    }

    // Call the constructor if one exists.
    if let Some(ctor) = constructor_fn {
        let mut ctor_args: Vec<inkwell::values::BasicMetadataValueEnum> = Vec::new();
        // First arg: this pointer (as i8*).
        let this_i8 = builder
            .build_bit_cast(struct_ptr, i8_ptr, "this.cast")
            .map_err(|e| format!("build_bit_cast this failed: {:?}", e))?;
        ctor_args.push(this_i8.into());
        // Remaining args: the constructor args.
        for arg in args {
            ctor_args.push((*arg).into());
        }
        builder
            .build_call(ctor, &ctor_args, "ctor.call")
            .map_err(|e| format!("build_call ctor failed: {:?}", e))?;
    }

    // Return the i8* pointer.
    let result = builder
        .build_bit_cast(struct_ptr, i8_ptr, "new.result")
        .map_err(|e| format!("build_bit_cast new.result failed: {:?}", e))?;
    Ok(result)
}

/// Emit a field access on a class instance.
///
/// `instance_ptr` is the i8* pointer to the class instance.
/// Returns the loaded value of the field.
pub fn emit_field_access<'ctx>(
    context: &'ctx Context,
    builder: &inkwell::builder::Builder<'ctx>,
    class_info: &ClassInfo<'ctx>,
    instance_ptr: PointerValue<'ctx>,
    field_name: &str,
) -> Result<BasicValueEnum<'ctx>, String> {
    let struct_ptr_type = context.ptr_type(AddressSpace::default());

    // Cast the i8* to the struct pointer type.
    let struct_ptr = if instance_ptr.get_type() == struct_ptr_type {
        instance_ptr
    } else {
        builder
            .build_bit_cast(instance_ptr, struct_ptr_type, "field.cast")
            .map_err(|e| format!("build_bit_cast field failed: {:?}", e))?
            .into_pointer_value()
    };

    let field_idx = field_index(class_info, field_name).ok_or_else(|| {
        format!(
            "field '{}' not found in class '{}'",
            field_name, class_info.name
        )
    })?;

    let gep = builder
        .build_struct_gep(
            class_info.struct_type,
            struct_ptr,
            field_idx,
            &format!("{}.gep", field_name),
        )
        .map_err(|e| format!("build_struct_gep field '{}' failed: {:?}", field_name, e))?;

    // Get the field type from the struct.
    let field_types = class_info.struct_type.get_field_types();
    let field_ty = field_types
        .get(field_idx as usize)
        .copied()
        .unwrap_or_else(|| context.ptr_type(AddressSpace::default()).into());

    builder
        .build_load(field_ty, gep, field_name)
        .map_err(|e| format!("build_load field '{}' failed: {:?}", field_name, e))
}

/// Emit a field store on a class instance.
pub fn emit_field_store<'ctx>(
    context: &'ctx Context,
    builder: &inkwell::builder::Builder<'ctx>,
    class_info: &ClassInfo<'ctx>,
    instance_ptr: PointerValue<'ctx>,
    field_name: &str,
    value: BasicValueEnum<'ctx>,
) -> Result<(), String> {
    let struct_ptr_type = context.ptr_type(AddressSpace::default());

    let struct_ptr = if instance_ptr.get_type() == struct_ptr_type {
        instance_ptr
    } else {
        builder
            .build_bit_cast(instance_ptr, struct_ptr_type, "field_store.cast")
            .map_err(|e| format!("build_bit_cast field_store failed: {:?}", e))?
            .into_pointer_value()
    };

    let field_idx = field_index(class_info, field_name).ok_or_else(|| {
        format!(
            "field '{}' not found in class '{}'",
            field_name, class_info.name
        )
    })?;

    let gep = builder
        .build_struct_gep(
            class_info.struct_type,
            struct_ptr,
            field_idx,
            &format!("{}.store.gep", field_name),
        )
        .map_err(|e| format!("build_struct_gep field_store '{}' failed: {:?}", field_name, e))?;

    builder
        .build_store(gep, value)
        .map_err(|e| format!("build_store field '{}' failed: {:?}", field_name, e))?;
    Ok(())
}

/// Emit a virtual method call.
///
/// Loads the vtable pointer, GEPs to the method index, loads the function
/// pointer, then calls it via indirect call.
pub fn emit_virtual_call<'ctx>(
    context: &'ctx Context,
    builder: &inkwell::builder::Builder<'ctx>,
    class_info: &ClassInfo<'ctx>,
    instance_ptr: PointerValue<'ctx>,
    method_name: &str,
    args: &[BasicValueEnum<'ctx>],
    return_type: Option<BasicTypeEnum<'ctx>>,
) -> Result<BasicValueEnum<'ctx>, String> {
    let i8_ptr = context.ptr_type(AddressSpace::default());
    let struct_ptr_type = context.ptr_type(AddressSpace::default());

    // Cast the instance to the struct pointer type.
    let struct_ptr = if instance_ptr.get_type() == struct_ptr_type {
        instance_ptr
    } else {
        builder
            .build_bit_cast(instance_ptr, struct_ptr_type, "vcall.cast")
            .map_err(|e| format!("build_bit_cast vcall failed: {:?}", e))?
            .into_pointer_value()
    };

    // Load the vtable pointer (field 0).
    let vtable_gep = builder
        .build_struct_gep(class_info.struct_type, struct_ptr, 0, "vtable.ptr")
        .map_err(|e| format!("build_struct_gep vtable failed: {:?}", e))?;
    let vtable_ptr = builder
        .build_load(i8_ptr, vtable_gep, "vtable")
        .map_err(|e| format!("build_load vtable failed: {:?}", e))?
        .into_pointer_value();

    // GEP to the method index in the vtable.
    let method_idx = method_vtable_index(&class_info.method_names, method_name).ok_or_else(
        || {
            format!(
                "method '{}' not found in vtable of '{}'",
                method_name, class_info.name
            )
        },
    )?;
    let i32_type = context.i32_type();
    let idx_val = i32_type.const_int(method_idx as u64, false);
    let method_ptr_ptr = unsafe {
        builder.build_gep(i8_ptr, vtable_ptr, &[idx_val], "vtable.method.gep")
    }
    .map_err(|e| format!("build_gep vtable method failed: {:?}", e))?;

    // Load the function pointer.
    let fn_ptr = builder
        .build_load(i8_ptr, method_ptr_ptr, "vtable.method")
        .map_err(|e| format!("build_load vtable method failed: {:?}", e))?
        .into_pointer_value();

    // Build the function type for the indirect call.
    let mut param_tys: Vec<inkwell::types::BasicMetadataTypeEnum> = Vec::new();
    // First param is always `this` (i8*).
    param_tys.push(i8_ptr.into());
    for arg in args {
        param_tys.push(arg.get_type().into());
    }
    let void_ty = context.void_type();
    let fn_type = match return_type {
        Some(rt) => rt.fn_type(&param_tys, false),
        None => void_ty.fn_type(&param_tys, false),
    };
    let fn_ptr_type = context.ptr_type(AddressSpace::default());
    let fn_ptr_typed = builder
        .build_bit_cast(fn_ptr, fn_ptr_type, "vtable.fn.cast")
        .map_err(|e| format!("build_bit_cast fn_ptr failed: {:?}", e))?;

    let mut call_args: Vec<inkwell::values::BasicMetadataValueEnum> = Vec::new();
    let this_i8 = builder
        .build_bit_cast(struct_ptr, i8_ptr, "this.i8")
        .map_err(|e| format!("build_bit_cast this.i8 failed: {:?}", e))?;
    call_args.push(this_i8.into());
    for arg in args {
        call_args.push((*arg).into());
    }

    // Use indirect call through the function pointer.
    let call = builder
        .build_indirect_call(fn_type, fn_ptr_typed.into_pointer_value(), &call_args, &format!("vcall.{}", method_name))
        .map_err(|e| format!("build_indirect_call vcall '{}' failed: {:?}", method_name, e))?;

    if return_type.is_some() {
        match call.try_as_basic_value() {
            inkwell::values::ValueKind::Basic(v) => Ok(v),
            _ => Err(format!(
                "virtual call to '{}' did not return a value",
                method_name
            )),
        }
    } else {
        let i32_type = context.i32_type();
        Ok(i32_type.const_int(0, false).into())
    }
}

/// Emit a direct method call on a class instance.
pub fn emit_direct_call<'ctx>(
    context: &'ctx Context,
    builder: &inkwell::builder::Builder<'ctx>,
    method_fn: inkwell::values::FunctionValue<'ctx>,
    instance_ptr: PointerValue<'ctx>,
    args: &[BasicValueEnum<'ctx>],
) -> Result<BasicValueEnum<'ctx>, String> {
    let i8_ptr = context.ptr_type(AddressSpace::default());

    let mut call_args: Vec<inkwell::values::BasicMetadataValueEnum> = Vec::new();
    // First arg is `this` (as i8*).
    let this_i8 = if instance_ptr.get_type() == i8_ptr {
        instance_ptr
    } else {
        builder
            .build_bit_cast(instance_ptr, i8_ptr, "this.i8")
            .map_err(|e| format!("build_bit_cast this.i8 failed: {:?}", e))?
            .into_pointer_value()
    };
    call_args.push(this_i8.into());
    for arg in args {
        call_args.push((*arg).into());
    }

    let call = builder
        .build_call(method_fn, &call_args, "call.direct")
        .map_err(|e| format!("build_call direct failed: {:?}", e))?;

    if method_fn.get_type().get_return_type().is_some() {
        match call.try_as_basic_value() {
            inkwell::values::ValueKind::Basic(v) => Ok(v),
            _ => Err("direct call did not return a value".to_string()),
        }
    } else {
        let i32_type = context.i32_type();
        Ok(i32_type.const_int(0, false).into())
    }
}

/// Perform an `is` type check by comparing vtable pointers.
///
/// Loads the vtable pointer from the object and compares it against the
/// expected vtable global address.
pub fn emit_is_check<'ctx>(
    context: &'ctx Context,
    builder: &inkwell::builder::Builder<'ctx>,
    instance_ptr: PointerValue<'ctx>,
    expected_vtable: PointerValue<'ctx>,
) -> Result<BasicValueEnum<'ctx>, String> {
    let i8_ptr = context.ptr_type(AddressSpace::default());

    // Load the vtable pointer from the instance (first field = i8*).
    let vtable_ptr = builder
        .build_load(i8_ptr, instance_ptr, "is.vtable")
        .map_err(|e| format!("build_load is.vtable failed: {:?}", e))?
        .into_pointer_value();

    // Cast the expected vtable to i8*.
    let expected_i8 = builder
        .build_bit_cast(expected_vtable, i8_ptr, "is.expected")
        .map_err(|e| format!("build_bit_cast is.expected failed: {:?}", e))?
        .into_pointer_value();

    // Compare the two pointers.
    let cmp = builder
        .build_int_compare(
            inkwell::IntPredicate::EQ,
            vtable_ptr,
            expected_i8,
            "is.cmp",
        )
        .map_err(|e| format!("build_int_compare is failed: {:?}", e))?;

    Ok(cmp.into())
}

/// Emit an `as` cast. For class casts, returns the same pointer (identity cast).
/// For interface casts, this will create the fat pointer.
pub fn emit_as_cast<'ctx>(
    _context: &'ctx Context,
    _builder: &inkwell::builder::Builder<'ctx>,
    instance_ptr: PointerValue<'ctx>,
) -> Result<BasicValueEnum<'ctx>, String> {
    // Simple identity cast: return the same pointer.
    Ok(instance_ptr.into())
}

/// Information about a compiled interface.
#[derive(Clone)]
pub struct InterfaceInfo<'ctx> {
    /// The interface name.
    pub name: String,
    /// LLVM struct type for the interface fat pointer: { i8*, i8* }.
    pub fat_ptr_type: inkwell::types::StructType<'ctx>,
    /// Method names in the interface (sorted alphabetically).
    pub method_names: Vec<String>,
    /// Default method implementations (LLVM functions), keyed by method name.
    pub default_methods: HashMap<String, inkwell::values::FunctionValue<'ctx>>,
    /// The interface vtable type (array of i8*).
    pub vtable_type: Option<inkwell::types::ArrayType<'ctx>>,
}

/// Build the fat pointer struct type for an interface: { i8*, i8* }.
pub fn build_interface_fat_ptr_type<'ctx>(context: &'ctx Context) -> inkwell::types::StructType<'ctx> {
    let i8_ptr = context.ptr_type(AddressSpace::default());
    context.struct_type(&[i8_ptr.into(), i8_ptr.into()], false)
}

/// Create an interface vtable global for a class that implements an interface.
///
/// The interface vtable maps interface method names to the class's method
/// implementations. If a method is not found in the class, it falls back to
/// the interface's default method (if any).
pub fn create_interface_vtable<'ctx>(
    context: &'ctx Context,
    module: &inkwell::module::Module<'ctx>,
    class_name: &str,
    interface_name: &str,
    interface_methods: &[String],
    class_method_functions: &HashMap<String, inkwell::values::FunctionValue<'ctx>>,
    default_methods: &HashMap<String, inkwell::values::FunctionValue<'ctx>>,
) -> Option<inkwell::values::GlobalValue<'ctx>> {
    if interface_methods.is_empty() {
        return None;
    }

    let i8_ptr = context.ptr_type(AddressSpace::default());

    // Sort method names for deterministic ordering.
    let mut sorted_names: Vec<String> = interface_methods.to_vec();
    sorted_names.sort();

    let mut entries: Vec<PointerValue<'ctx>> = Vec::with_capacity(sorted_names.len());
    for mname in &sorted_names {
        let fn_val = if let Some(mf) = class_method_functions.get(mname) {
            mf.as_global_value().as_pointer_value()
        } else if let Some(df) = default_methods.get(mname) {
            df.as_global_value().as_pointer_value()
        } else {
            panic!(
                "interface vtable: method '{}' not found in class '{}' for interface '{}'",
                mname, class_name, interface_name
            )
        };
        let fn_ptr_i8 = fn_val.const_cast(i8_ptr);
        entries.push(fn_ptr_i8);
    }

    let arr_type = i8_ptr.array_type(entries.len() as u32);
    let vtable_name = format!("_ivtable_{}_{}", interface_name, class_name);
    let vtable = module.add_global(arr_type, None, &vtable_name);
    vtable.set_linkage(Linkage::Internal);
    vtable.set_constant(true);

    let const_arr = i8_ptr.const_array(&entries);
    vtable.set_initializer(&const_arr);

    Some(vtable)
}

/// Emit an interface fat pointer from an object pointer and interface vtable.
///
/// Returns a `{ i8*, i8* }` struct value.
pub fn emit_interface_fat_ptr<'ctx>(
    context: &'ctx Context,
    builder: &inkwell::builder::Builder<'ctx>,
    fat_ptr_type: inkwell::types::StructType<'ctx>,
    object_ptr: PointerValue<'ctx>,
    vtable_ptr: PointerValue<'ctx>,
) -> Result<BasicValueEnum<'ctx>, String> {
    let i8_ptr = context.ptr_type(AddressSpace::default());

    // Cast object pointer to i8* if needed.
    let obj_i8 = if object_ptr.get_type() == i8_ptr {
        object_ptr
    } else {
        builder
            .build_bit_cast(object_ptr, i8_ptr, "iface.obj.cast")
            .map_err(|e| format!("build_bit_cast iface.obj failed: {:?}", e))?
            .into_pointer_value()
    };

    // Cast vtable to i8*.
    let vt_i8 = builder
        .build_bit_cast(vtable_ptr, i8_ptr, "iface.vt.cast")
        .map_err(|e| format!("build_bit_cast iface.vt failed: {:?}", e))?
        .into_pointer_value();

    // Allocate space for the fat pointer and store the two fields.
    let alloca = builder
        .build_alloca(fat_ptr_type, "iface.fat")
        .map_err(|e| format!("build_alloca iface.fat failed: {:?}", e))?;

    let obj_gep = builder
        .build_struct_gep(fat_ptr_type, alloca, 0, "iface.obj.gep")
        .map_err(|e| format!("build_struct_gep iface.obj failed: {:?}", e))?;
    builder
        .build_store(obj_gep, obj_i8)
        .map_err(|e| format!("build_store iface.obj failed: {:?}", e))?;

    let vt_gep = builder
        .build_struct_gep(fat_ptr_type, alloca, 1, "iface.vt.gep")
        .map_err(|e| format!("build_struct_gep iface.vt failed: {:?}", e))?;
    builder
        .build_store(vt_gep, vt_i8)
        .map_err(|e| format!("build_store iface.vt failed: {:?}", e))?;

    builder
        .build_load(fat_ptr_type, alloca, "iface.fat.val")
        .map_err(|e| format!("build_load iface.fat failed: {:?}", e))
}

/// Emit an `is` check for interfaces.
///
/// Compares the vtable pointer from the fat pointer against the expected
/// interface vtable.
pub fn emit_interface_is_check<'ctx>(
    context: &'ctx Context,
    builder: &inkwell::builder::Builder<'ctx>,
    _fat_ptr_type: inkwell::types::StructType<'ctx>,
    fat_ptr_val: BasicValueEnum<'ctx>,
    expected_vtable: PointerValue<'ctx>,
) -> Result<BasicValueEnum<'ctx>, String> {
    let i8_ptr = context.ptr_type(AddressSpace::default());

    // Extract the vtable pointer from the fat pointer (field 1).
    let fat_ptr_struct = match fat_ptr_val {
        BasicValueEnum::StructValue(sv) => sv,
        _ => return Err("interface is check: expected struct value".to_string()),
    };

    let vt_ptr = builder
        .build_extract_value(fat_ptr_struct, 1, "is.vt")
        .map_err(|e| format!("build_extract_value is.vt failed: {:?}", e))?
        .into_pointer_value();

    // Cast expected vtable to i8*.
    let expected_i8 = builder
        .build_bit_cast(expected_vtable, i8_ptr, "is.expected")
        .map_err(|e| format!("build_bit_cast is.expected failed: {:?}", e))?
        .into_pointer_value();

    // Compare.
    builder
        .build_int_compare(
            inkwell::IntPredicate::EQ,
            vt_ptr,
            expected_i8,
            "is.iface.cmp",
        )
        .map_err(|e| format!("build_int_compare is.iface failed: {:?}", e))
        .map(|v| v.into())
}

/// Emit an interface method call through a fat pointer.
#[allow(clippy::too_many_arguments)]
pub fn emit_interface_method_call<'ctx>(
    context: &'ctx Context,
    builder: &inkwell::builder::Builder<'ctx>,
    _fat_ptr_type: inkwell::types::StructType<'ctx>,
    fat_ptr_val: BasicValueEnum<'ctx>,
    interface_methods: &[String],
    method_name: &str,
    args: &[BasicValueEnum<'ctx>],
    return_type: Option<BasicTypeEnum<'ctx>>,
) -> Result<BasicValueEnum<'ctx>, String> {
    let i8_ptr = context.ptr_type(AddressSpace::default());

    // Extract the fat pointer fields.
    let fat_ptr_struct = match fat_ptr_val {
        BasicValueEnum::StructValue(sv) => sv,
        _ => return Err("interface method call: expected struct value".to_string()),
    };

    let obj_ptr = builder
        .build_extract_value(fat_ptr_struct, 0, "iface.obj")
        .map_err(|e| format!("build_extract_value iface.obj failed: {:?}", e))?
        .into_pointer_value();

    let vt_ptr = builder
        .build_extract_value(fat_ptr_struct, 1, "iface.vt")
        .map_err(|e| format!("build_extract_value iface.vt failed: {:?}", e))?
        .into_pointer_value();

    // Find the method index in the interface vtable.
    let mut sorted_methods: Vec<&String> = interface_methods.iter().collect();
    sorted_methods.sort();
    let method_idx = sorted_methods
        .iter()
        .position(|&n| n == method_name)
        .ok_or_else(|| format!("method '{}' not found in interface vtable", method_name))?;

    // GEP to the method entry in the vtable.
    let i32_type = context.i32_type();
    let idx_val = i32_type.const_int(method_idx as u64, false);
    let method_ptr_ptr = unsafe {
        builder.build_gep(i8_ptr, vt_ptr, &[idx_val], "iface.method.gep")
    }
    .map_err(|e| format!("build_gep iface.method failed: {:?}", e))?;

    // Load the function pointer.
    let fn_ptr = builder
        .build_load(i8_ptr, method_ptr_ptr, "iface.method")
        .map_err(|e| format!("build_load iface.method failed: {:?}", e))?
        .into_pointer_value();

    // Build the function type for the indirect call.
    let mut param_tys: Vec<inkwell::types::BasicMetadataTypeEnum> = Vec::new();
    param_tys.push(i8_ptr.into());
    for arg in args {
        param_tys.push(arg.get_type().into());
    }
    let void_ty = context.void_type();
    let fn_type = match return_type {
        Some(rt) => rt.fn_type(&param_tys, false),
        None => void_ty.fn_type(&param_tys, false),
    };
    let fn_ptr_type = context.ptr_type(AddressSpace::default());
    let fn_ptr_typed = builder
        .build_bit_cast(fn_ptr, fn_ptr_type, "iface.fn.cast")
        .map_err(|e| format!("build_bit_cast iface.fn failed: {:?}", e))?;

    let mut call_args: Vec<inkwell::values::BasicMetadataValueEnum> = Vec::new();
    call_args.push(obj_ptr.into());
    for arg in args {
        call_args.push((*arg).into());
    }

    let call = builder
        .build_indirect_call(
            fn_type,
            fn_ptr_typed.into_pointer_value(),
            &call_args,
            &format!("iface.call.{}", method_name),
        )
        .map_err(|e| format!("build_indirect_call iface '{}' failed: {:?}", method_name, e))?;

    if return_type.is_some() {
        match call.try_as_basic_value() {
            inkwell::values::ValueKind::Basic(v) => Ok(v),
            _ => Err(format!("interface call to '{}' did not return a value", method_name)),
        }
    } else {
        let i32_type = context.i32_type();
        Ok(i32_type.const_int(0, false).into())
    }
}
