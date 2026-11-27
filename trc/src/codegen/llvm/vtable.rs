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

use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};
use inkwell::types::{AnyTypeEnum, BasicMetadataTypeEnum, BasicType, BasicTypeEnum, StructType};
use inkwell::values::{
    BasicMetadataValueEnum, BasicValueEnum, FunctionValue, GlobalValue, PointerValue, StructValue,
};
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

/// Only an emitted body can serve as a vtable target. A declaration left by
/// an aborted pass would become an undefined linker symbol.
fn with_body<'ctx>(f: Option<FunctionValue<'ctx>>) -> Option<FunctionValue<'ctx>> {
    f.filter(|f| f.count_basic_blocks() > 0)
}

/// Locate a compiled `Class_method` implementation.
///
/// Specialized generic classes emit methods under mangled names and arity
/// suffixes (`Box__int_get__a2`), so the caller's map can hold only a bodyless
/// declaration while the real body lives in the module. Prefix matching is
/// confined to this class's own symbols; unrelated classes never win.
fn find_compiled_class_method<'ctx>(
    module: &Module<'ctx>,
    class_name: &str,
    method_name: &str,
) -> Option<FunctionValue<'ctx>> {
    let base = format!("{}_{}", class_name, method_name);
    with_body(module.get_function(&base)).or_else(|| {
        let suffixed = format!("{}__", base);
        module.get_functions().find(|f| {
            f.get_name()
                .to_str()
                .is_ok_and(|name| name.starts_with(suffixed.as_str()))
                && f.count_basic_blocks() > 0
        })
    })
}

/// Emit (or reuse) the runtime stub used for a vtable slot with no compiled
/// method. The stub names the class and method, prints, and aborts, so a
/// missing specialization is diagnosable instead of jumping to address zero.
/// Never creates a second, suffixed copy on repeated compilation: the same
/// symbol and message global are reused for the same class/method.
fn missing_method_stub<'ctx>(
    context: &'ctx Context,
    module: &Module<'ctx>,
    symbol: &str,
    message: &str,
) -> FunctionValue<'ctx> {
    let fn_name = format!("_null_vtable_stub_{}", symbol);
    if let Some(existing) = module.get_function(&fn_name) {
        if existing.count_basic_blocks() > 0 {
            return existing;
        }
    }

    let i8_ptr = context.ptr_type(AddressSpace::default());
    let void_ty = context.void_type();
    // Dispatch sites build their own signature; a zero-argument callee may be
    // called with extra arguments on every supported ABI. Varargs would need
    // the caller to set the SSE count register, which it does not.
    let fn_ty = void_ty.fn_type(&[], false);
    let stub = match module.get_function(&fn_name) {
        Some(existing) => existing,
        None => module.add_function(&fn_name, fn_ty, None),
    };
    stub.set_linkage(Linkage::Internal);

    let msg_name = format!("_null_vtable_msg_{}", symbol);
    let msg = match module.get_global(&msg_name) {
        Some(existing) => existing,
        None => {
            let bytes = message.as_bytes();
            let global = module.add_global(
                context.i8_type().array_type((bytes.len() + 1) as u32),
                None,
                &msg_name,
            );
            global.set_linkage(Linkage::Internal);
            global.set_constant(true);
            global.set_initializer(&context.const_string(bytes, true));
            global
        }
    };

    let entry = context.append_basic_block(stub, "entry");
    let builder = context.create_builder();
    builder.position_at_end(entry);

    let println_ty = void_ty.fn_type(&[context.i64_type().into(), i8_ptr.into()], false);
    let println = module.get_function("titrate_println").unwrap_or_else(|| {
        module.add_function("titrate_println", println_ty, Some(Linkage::External))
    });
    let abort = module.get_function("abort").unwrap_or_else(|| {
        module.add_function("abort", void_ty.fn_type(&[], false), Some(Linkage::External))
    });

    builder
        .build_call(
            println,
            &[
                context
                    .i64_type()
                    .const_int(message.len() as u64, false)
                    .into(),
                msg.as_pointer_value().into(),
            ],
            "missing.method.print",
        )
        .expect("emit missing-method diagnostic");
    builder
        .build_call(abort, &[], "missing.method.abort")
        .expect("emit missing-method abort");
    builder
        .build_unreachable()
        .expect("terminate missing-method stub");
    stub
}

/// Install an `i8*` array vtable initializer, reusing a prior global of the
/// same shape so repeated passes cannot leave a dangling suffixed global.
fn set_vtable_global<'ctx>(
    context: &'ctx Context,
    module: &Module<'ctx>,
    name: &str,
    entries: &[PointerValue<'ctx>],
) -> GlobalValue<'ctx> {
    let i8_ptr = context.ptr_type(AddressSpace::default());
    let arr_type = i8_ptr.array_type(entries.len() as u32);
    let reusable = module.get_global(name).is_some_and(|existing| {
        matches!(
            existing.get_value_type(),
            AnyTypeEnum::ArrayType(at)
                if at.len() == entries.len() as u32 && at.get_element_type() == i8_ptr.into()
        )
    });
    let global = if reusable {
        module.get_global(name).expect("checked above")
    } else {
        module.add_global(arr_type, None, name)
    };
    global.set_linkage(Linkage::Internal);
    global.set_constant(true);
    global.set_initializer(&i8_ptr.const_array(entries));
    global
}

/// Create the vtable global constant for a class.
///
/// The vtable is an array of `i8*` function pointers in the caller's order.
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

    // Preserve the caller's order: class vtables arrive prefix-stable
    // (ancestors first) and interface vtables in declaration order, so
    // every index site agrees without re-sorting.
    let mut entries: Vec<PointerValue<'ctx>> = Vec::with_capacity(method_names.len());
    for mname in method_names {
        // Keep the slot even when the method is missing: indices are
        // positional and every dispatch site agrees on them. A body-bearing
        // implementation found in the module completes specialized generic
        // classes whose map entry is only a declaration.
        let fn_ptr_i8 = match with_body(method_functions.get(mname).copied())
            .or_else(|| find_compiled_class_method(module, class_name, mname))
        {
            Some(fn_val) => fn_val.as_global_value().as_pointer_value().const_cast(i8_ptr),
            None => {
                eprintln!(
                    "codegen: vtable for class '{}' is missing method '{}'; emitting null-safe stub",
                    class_name, mname
                );
                let message = format!(
                    "titrate: missing method '{}' on class '{}'",
                    mname, class_name
                );
                let stub = missing_method_stub(
                    context,
                    module,
                    &format!("{}_{}", class_name, mname),
                    &message,
                );
                stub.as_global_value().as_pointer_value().const_cast(i8_ptr)
            }
        };
        entries.push(fn_ptr_i8);
    }

    let vtable_name = format!("_vtable_{}", class_name);
    Some(set_vtable_global(context, module, &vtable_name, &entries))
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

/// Get the method index in the vtable (positional: layouts are built
/// prefix-stable, so every site agrees without re-sorting).
pub fn method_vtable_index(method_names: &[String], method_name: &str) -> Option<u32> {
    method_names
        .iter()
        .position(|n| n == method_name)
        .map(|i| i as u32)
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
    let malloc_args = shape_call_args(context, builder, malloc_fn, &[size.into()], "new.ptr")?;
    let call = builder
        .build_call(malloc_fn, &malloc_args, "new.ptr")
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
        let memset_fn = match module.get_function("llvm.memset.p0.i64") {
            Some(f) => f,
            None => {
                let fn_ty = void_ty.fn_type(
                    &[i8_ptr_ty.into(), i8_ty.into(), i64_ty.into(), i1_ty.into()],
                    false,
                );
                module.add_function("llvm.memset.p0.i64", fn_ty, None)
            }
        };
        let zero_val = i8_ty.const_int(0, false);
        let is_volatile = i1_ty.const_int(0, false);
        let memset_args = shape_call_args(
            context,
            builder,
            memset_fn,
            &[
                instance_ptr.into(),
                zero_val.into(),
                size.into(),
                is_volatile.into(),
            ],
            "new.memset",
        )?;
        builder
            .build_call(memset_fn, &memset_args, "new.memset")
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
        let mut ctor_args: Vec<BasicValueEnum> = Vec::with_capacity(args.len() + 1);
        // First arg: this pointer (as i8*).
        let this_i8 = builder
            .build_bit_cast(struct_ptr, i8_ptr, "this.cast")
            .map_err(|e| format!("build_bit_cast this failed: {:?}", e))?;
        ctor_args.push(this_i8);
        // Remaining args: the constructor args.
        ctor_args.extend_from_slice(args);
        let ctor_args = shape_call_args(context, builder, ctor, &ctor_args, "ctor.call")?;
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

/// Compute the address of a field slot on a class instance (GEP without load).
///
/// Used for container reference semantics: passing `obj.field` to a function
/// whose parameter is a container shares the field slot instead of copying it.
pub fn emit_field_slot_ptr<'ctx>(
    context: &'ctx Context,
    builder: &inkwell::builder::Builder<'ctx>,
    class_info: &ClassInfo<'ctx>,
    instance_ptr: PointerValue<'ctx>,
    field_name: &str,
) -> Result<PointerValue<'ctx>, String> {
    let struct_ptr_type = context.ptr_type(AddressSpace::default());
    let struct_ptr = if instance_ptr.get_type() == struct_ptr_type {
        instance_ptr
    } else {
        builder
            .build_bit_cast(instance_ptr, struct_ptr_type, "field.slot.cast")
            .map_err(|e| format!("build_bit_cast field slot failed: {:?}", e))?
            .into_pointer_value()
    };
    let field_idx = field_index(class_info, field_name).ok_or_else(|| {
        format!(
            "field '{}' not found in class '{}'",
            field_name, class_info.name
        )
    })?;
    builder
        .build_struct_gep(
            class_info.struct_type,
            struct_ptr,
            field_idx,
            &format!("{}.slot", field_name),
        )
        .map_err(|e| format!("build_struct_gep field slot '{}' failed: {:?}", field_name, e))
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

/// Refuse to emit an indirect call through a statically null pointer, and
/// guard dynamic pointers with a trap path so a bad vtable entry fails at a
/// known point instead of jumping to address zero.
fn guard_indirect_target<'ctx>(
    context: &'ctx Context,
    builder: &Builder<'ctx>,
    ptr: PointerValue<'ctx>,
    what: &str,
) -> Result<(), String> {
    if ptr.is_null() {
        return Err(format!(
            "codegen: null {}; refusing to emit indirect call",
            what
        ));
    }
    let Some(block) = builder.get_insert_block() else {
        return Ok(());
    };
    let Some(parent) = block.get_parent() else {
        return Ok(());
    };
    let is_null = builder
        .build_is_null(ptr, &format!("{}.is_null", what))
        .map_err(|e| format!("build_is_null {} failed: {:?}", what, e))?;
    let trap_bb = context.append_basic_block(parent, &format!("{}.null.trap", what));
    let cont_bb = context.append_basic_block(parent, &format!("{}.null.cont", what));
    builder
        .build_conditional_branch(is_null, trap_bb, cont_bb)
        .map_err(|e| format!("build_conditional_branch {} failed: {:?}", what, e))?;
    builder.position_at_end(trap_bb);
    builder
        .build_unreachable()
        .map_err(|e| format!("build_unreachable {} failed: {:?}", what, e))?;
    builder.position_at_end(cont_bb);
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
    guard_indirect_target(context, builder, vtable_ptr, "vtable")?;

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
    guard_indirect_target(context, builder, fn_ptr, "vtable.fn")?;

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

/// Convert an LLVM function-parameter type (metadata form) to a basic type.
/// Returns `None` for metadata-only parameters, which cannot be call args.
pub(super) fn basic_metadata_to_basic_type<'ctx>(
    ty: BasicMetadataTypeEnum<'ctx>,
) -> Option<BasicTypeEnum<'ctx>> {
    match ty {
        BasicMetadataTypeEnum::ArrayType(t) => Some(t.into()),
        BasicMetadataTypeEnum::PointerType(t) => Some(t.into()),
        BasicMetadataTypeEnum::IntType(t) => Some(t.into()),
        BasicMetadataTypeEnum::FloatType(t) => Some(t.into()),
        BasicMetadataTypeEnum::StructType(t) => Some(t.into()),
        BasicMetadataTypeEnum::VectorType(t) => Some(t.into()),
        BasicMetadataTypeEnum::ScalableVectorType(t) => Some(t.into()),
        BasicMetadataTypeEnum::MetadataType(_) => None,
    }
}

/// True for `{i64, ptr}` container slots and `{ptr, ptr}` interface fat
/// pointers: the only struct shapes that are interchangeable at call sites.
pub(super) fn is_container_or_fat_ptr_shape<'ctx>(
    ty: inkwell::types::StructType<'ctx>,
) -> bool {
    ty.count_fields() == 2
        && ty
            .get_field_type_at_index(1)
            .is_some_and(|t| t.is_pointer_type())
        && ty
            .get_field_type_at_index(0)
            .is_some_and(|t| t.is_int_type() || t.is_pointer_type())
}

/// Rebuild a container slot `{i64, ptr}` as an interface fat pointer
/// `{ptr, ptr}`, or the reverse, field by field. LLVM does not permit
/// bitcasting aggregate (struct) values, so the length word is converted with
/// inttoptr/ptrtoint and the data pointer is copied unchanged.
pub(super) fn reshape_container_or_fat_ptr<'ctx>(
    builder: &Builder<'ctx>,
    v: StructValue<'ctx>,
    to_st: StructType<'ctx>,
) -> Result<StructValue<'ctx>, String> {
    let from_st = v.get_type();
    if from_st.count_fields() != 2 || to_st.count_fields() != 2 {
        return Err(format!(
            "codegen: cannot reshape '{}' to '{}': expected two-field structs",
            from_st.print_to_string().to_string(),
            to_st.print_to_string().to_string()
        ));
    }
    let mut result = to_st.const_zero();
    for idx in 0..2u32 {
        let field = builder
            .build_extract_value(v, idx, "call.arg.field")
            .map_err(|e| format!("call arg field extract failed: {:?}", e))?;
        let to_field = to_st
            .get_field_type_at_index(idx)
            .ok_or_else(|| "call arg: missing target struct field".to_string())?;
        // Every field combination must be shaped explicitly. Falling through
        // with the source value would insert a mismatched type into the
        // struct and trip LLVM's verifier (or silently change the bits).
        let shaped: BasicValueEnum<'ctx> = match (field, to_field) {
            (BasicValueEnum::IntValue(iv), BasicTypeEnum::IntType(it)) => {
                if iv.get_type() == it {
                    iv.into()
                } else if iv.get_type().get_bit_width() < it.get_bit_width() {
                    builder
                        .build_int_s_extend(iv, it, "call.arg.sext")
                        .map_err(|e| format!("call arg field sext failed: {:?}", e))?
                        .into()
                } else {
                    builder
                        .build_int_truncate(iv, it, "call.arg.trunc")
                        .map_err(|e| format!("call arg field trunc failed: {:?}", e))?
                        .into()
                }
            }
            (BasicValueEnum::IntValue(iv), BasicTypeEnum::PointerType(pt)) => builder
                .build_int_to_ptr(iv, pt, "call.arg.itop")
                .map_err(|e| format!("call arg int_to_ptr failed: {:?}", e))?
                .into(),
            (BasicValueEnum::PointerValue(pv), BasicTypeEnum::IntType(it)) => builder
                .build_ptr_to_int(pv, it, "call.arg.ptoi")
                .map_err(|e| format!("call arg ptr_to_int failed: {:?}", e))?
                .into(),
            (BasicValueEnum::PointerValue(pv), BasicTypeEnum::PointerType(pt)) => {
                if pv.get_type() == pt {
                    pv.into()
                } else {
                    builder
                        .build_bit_cast(pv, pt, "call.arg.ptr.bitcast")
                        .map_err(|e| format!("call arg pointer bitcast failed: {:?}", e))?
                }
            }
            _ => {
                return Err(format!(
                    "codegen: cannot reshape field {} of '{}' to '{}'",
                    idx,
                    from_st.print_to_string().to_string(),
                    to_st.print_to_string().to_string()
                ))
            }
        };
        result = builder
            .build_insert_value(result, shaped, idx, "call.arg.insert")
            .map_err(|e| format!("call arg field insert failed: {:?}", e))?
            .into_struct_value();
    }
    Ok(result)
}

/// Coerce a compiled argument to the exact LLVM type the callee declares.
///
/// LLVM aborts the compiler with a C++ assertion ("Calling a function with a
/// bad signature!") when an argument value's type differs from
/// `FTy->getParamType(i)`. Reshape every argument before emission: intcast for
/// width differences, fpcast for float widths, inttoptr/ptrtoint for numeric
/// and pointer shape differences, and field-wise rebuilding for interface fat
/// pointers `{ptr, ptr}` vs container slots `{i64, ptr}` (LLVM forbids struct
/// bitcasts).
pub(super) fn coerce_call_arg<'ctx>(
    context: &'ctx Context,
    builder: &Builder<'ctx>,
    v: BasicValueEnum<'ctx>,
    target_ty: BasicTypeEnum<'ctx>,
) -> Result<BasicValueEnum<'ctx>, String> {
    if v.get_type() == target_ty {
        return Ok(v);
    }
    // int -> int (integer width differences): intcast.
    if v.is_int_value() && target_ty.is_int_type() {
        let from = v.into_int_value();
        let to_ty = target_ty.into_int_type();
        let result = if from.get_type().get_bit_width() < to_ty.get_bit_width() {
            builder.build_int_s_extend(from, to_ty, "call.arg.sext")
        } else {
            builder.build_int_truncate(from, to_ty, "call.arg.trunc")
        }
        .map_err(|e| format!("call arg intcast failed: {:?}", e))?;
        return Ok(result.into());
    }
    // float -> float (width differences): fpcast.
    if v.is_float_value() && target_ty.is_float_type() {
        let from = v.into_float_value();
        let to_ty = target_ty.into_float_type();
        let result = if from.get_type().get_bit_width() < to_ty.get_bit_width() {
            builder.build_float_ext(from, to_ty, "call.arg.fpext")
        } else {
            builder.build_float_trunc(from, to_ty, "call.arg.fptrunc")
        }
        .map_err(|e| format!("call arg fpcast failed: {:?}", e))?;
        return Ok(result.into());
    }
    // numeric -> pointer: inttoptr. Floats first bitcast to a same-width int.
    if target_ty.is_pointer_type() {
        let to_ptr = target_ty.into_pointer_type();
        if v.is_int_value() {
            let cast = builder
                .build_int_to_ptr(v.into_int_value(), to_ptr, "call.arg.itop")
                .map_err(|e| format!("call arg int_to_ptr failed: {:?}", e))?;
            return Ok(cast.into());
        }
        if v.is_float_value() {
            let from = v.into_float_value();
            let bits = from.get_type().get_bit_width();
            let int_ty = match bits {
                16 => context.i16_type(),
                32 => context.i32_type(),
                64 => context.i64_type(),
                128 => context.i128_type(),
                _ => {
                    return Err(format!(
                        "codegen: cannot coerce call arg of type 'f{}' to pointer",
                        bits
                    ))
                }
            };
            let as_int = builder
                .build_bit_cast(from, int_ty, "call.arg.ftoi")
                .map_err(|e| format!("call arg float bitcast failed: {:?}", e))?;
            let cast = builder
                .build_int_to_ptr(as_int.into_int_value(), to_ptr, "call.arg.itop")
                .map_err(|e| format!("call arg int_to_ptr failed: {:?}", e))?;
            return Ok(cast.into());
        }
    }
    // pointer -> int: ptrtoint.
    if v.is_pointer_value() && target_ty.is_int_type() {
        let cast = builder
            .build_ptr_to_int(v.into_pointer_value(), target_ty.into_int_type(), "call.arg.ptoi")
            .map_err(|e| format!("call arg ptr_to_int failed: {:?}", e))?;
        return Ok(cast.into());
    }
    // pointer -> pointer: bitcast (different address spaces).
    if v.is_pointer_value() && target_ty.is_pointer_type() {
        let cast = builder
            .build_bit_cast(v, target_ty, "call.arg.bitcast")
            .map_err(|e| format!("call arg bitcast failed: {:?}", e))?;
        return Ok(cast);
    }
    // struct -> struct: reshape only between container slots `{i64, ptr}` and
    // interface fat pointers `{ptr, ptr}`. Other struct shapes must not be
    // silently reinterpreted.
    if v.is_struct_value() && target_ty.is_struct_type() {
        let from_st = v.into_struct_value().get_type();
        let to_st = target_ty.into_struct_type();
        if is_container_or_fat_ptr_shape(from_st) && is_container_or_fat_ptr_shape(to_st) {
            return Ok(reshape_container_or_fat_ptr(builder, v.into_struct_value(), to_st)?.into());
        }
    }
    Err(format!(
        "codegen: cannot coerce call arg of type '{}' to '{}'",
        v.get_type().print_to_string().to_string(),
        target_ty.print_to_string().to_string()
    ))
}

/// Verify a direct call's arguments against the callee's declared LLVM
/// parameter types before emission. LLVM aborts the compiler ("Calling a
/// function with a bad signature!") when a value's type differs from the
/// declared parameter type; report a descriptive error instead.
fn check_call_args<'ctx>(
    f: FunctionValue<'ctx>,
    args: &[BasicMetadataValueEnum<'ctx>],
    name: &str,
) -> Result<(), String> {
    let declared = f.get_type().get_param_types();
    let fn_name = f.get_name().to_str().unwrap_or("?").to_string();
    if declared.len() != args.len() && !f.get_type().is_var_arg() {
        return Err(format!(
            "codegen: bad call '{}' to '{}': expects {} args, got {}",
            name,
            fn_name,
            declared.len(),
            args.len()
        ));
    }
    for (i, (want, got)) in declared.iter().zip(args.iter()).enumerate() {
        let got_ty: Option<inkwell::types::BasicMetadataTypeEnum<'ctx>> = match got {
            BasicMetadataValueEnum::ArrayValue(v) => Some(v.get_type().into()),
            BasicMetadataValueEnum::IntValue(v) => Some(v.get_type().into()),
            BasicMetadataValueEnum::FloatValue(v) => Some(v.get_type().into()),
            BasicMetadataValueEnum::PointerValue(v) => Some(v.get_type().into()),
            BasicMetadataValueEnum::StructValue(v) => Some(v.get_type().into()),
            BasicMetadataValueEnum::VectorValue(v) => Some(v.get_type().into()),
            BasicMetadataValueEnum::ScalableVectorValue(v) => Some(v.get_type().into()),
            BasicMetadataValueEnum::MetadataValue(_) => None,
        };
        if let Some(got_ty) = got_ty {
            if want.print_to_string() != got_ty.print_to_string() {
                return Err(format!(
                    "codegen: bad call '{}' to '{}': arg {} expected {}, got {}",
                    name,
                    fn_name,
                    i,
                    want.print_to_string(),
                    got_ty.print_to_string()
                ));
            }
        }
    }
    Ok(())
}

/// Coerce every argument of a direct call to the callee's declared LLVM
/// parameter type (and verify arity) before emission. Reshaping fixes width
/// and container/fat-pointer mismatches; a shape that cannot be coerced
/// returns a descriptive error instead of aborting LLVM.
fn shape_call_args<'ctx>(
    context: &'ctx Context,
    builder: &Builder<'ctx>,
    f: FunctionValue<'ctx>,
    args: &[BasicValueEnum<'ctx>],
    name: &str,
) -> Result<Vec<BasicMetadataValueEnum<'ctx>>, String> {
    let declared = f.get_type().get_param_types();
    let fn_name = f.get_name().to_str().unwrap_or("?").to_string();
    if declared.len() != args.len() && !f.get_type().is_var_arg() {
        return Err(format!(
            "codegen: bad call '{}' to '{}': expects {} args, got {}",
            name,
            fn_name,
            declared.len(),
            args.len()
        ));
    }
    let mut shaped = Vec::with_capacity(args.len());
    for (i, arg) in args.iter().enumerate() {
        let v = match declared.get(i).copied().and_then(basic_metadata_to_basic_type) {
            Some(ty) => coerce_call_arg(context, builder, *arg, ty)?,
            None => *arg,
        };
        shaped.push(v.into());
    }
    Ok(shaped)
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
    let declared_params = method_fn.get_type().get_param_types();

    let mut call_args: Vec<inkwell::values::BasicMetadataValueEnum> = Vec::new();
    // First arg is `this`. Shape it to the callee's declared receiver type;
    // with opaque pointers that is `ptr`, so the cast is usually a no-op.
    let this_target = declared_params
        .first()
        .copied()
        .and_then(basic_metadata_to_basic_type);
    let this_i8 = match this_target {
        Some(ty) if ty.is_pointer_type() => {
            coerce_call_arg(context, builder, instance_ptr.into(), ty)?.into_pointer_value()
        }
        _ => {
            if instance_ptr.get_type() == i8_ptr {
                instance_ptr
            } else {
                builder
                    .build_bit_cast(instance_ptr, i8_ptr, "this.i8")
                    .map_err(|e| format!("build_bit_cast this.i8 failed: {:?}", e))?
                    .into_pointer_value()
            }
        }
    };
    call_args.push(this_i8.into());
    // Reshape each argument to the exact declared LLVM parameter type so the
    // emitted call never trips LLVM's bad-signature assertion. Extra args on a
    // vararg callee have no declared type and are passed through unchanged.
    for (i, arg) in args.iter().enumerate() {
        let declared = declared_params
            .get(i + 1)
            .copied()
            .and_then(basic_metadata_to_basic_type);
        let shaped = match declared {
            Some(ty) => coerce_call_arg(context, builder, *arg, ty)?,
            None => *arg,
        };
        call_args.push(shaped.into());
    }

    check_call_args(method_fn, &call_args, "direct")?;
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
    let i8_ptr = context.ptr_type(AddressSpace::default());

    // Keep exactly the interface's slot list: emit_interface_method_call
    // derives its index from the same names, so adding slots here would
    // silently redirect every later method. Completeness means filling each
    // existing slot, not changing its position. A stale interface info with
    // no names can still be recovered from the compiled declaration defaults.
    let mut sorted_names: Vec<String> = if interface_methods.is_empty() {
        default_methods.keys().cloned().collect()
    } else {
        interface_methods.to_vec()
    };
    sorted_names.sort();
    if sorted_names.is_empty() {
        return None;
    }

    let mut entries: Vec<PointerValue<'ctx>> = Vec::with_capacity(sorted_names.len());
    for mname in &sorted_names {
        // Prefer a body-bearing implementation. Specialized generic classes
        // carry bodyless declarations in the maps while the real body lives in
        // the module as `Class_method` (per-class default clone or arity-
        // suffixed method). `Iface_default_method` is emitted from the real
        // interface declaration and is the final fallback when a default is
        // missing from the passed map.
        let fn_val = with_body(class_method_functions.get(mname).copied())
            .or_else(|| find_compiled_class_method(module, class_name, mname))
            .or_else(|| with_body(default_methods.get(mname).copied()))
            .or_else(|| {
                with_body(module.get_function(&format!(
                    "{}_default_{}",
                    interface_name, mname
                )))
            });
        let fn_ptr_i8 = match fn_val {
            Some(f) => f.as_global_value().as_pointer_value().const_cast(i8_ptr),
            None => {
                eprintln!(
                    "codegen: interface vtable for class '{}' implementing '{}' has no compiled body for '{}'; emitting null-safe stub",
                    class_name, interface_name, mname
                );
                let message = format!(
                    "titrate: missing method '{}' on class '{}' (interface '{}')",
                    mname, class_name, interface_name
                );
                let stub = missing_method_stub(
                    context,
                    module,
                    &format!("{}_{}_{}", class_name, interface_name, mname),
                    &message,
                );
                stub.as_global_value().as_pointer_value().const_cast(i8_ptr)
            }
        };
        entries.push(fn_ptr_i8);
    }

    // A provisional vtable is registered before default-method clones are
    // compiled (so `this` calls in those clones dispatch through this class).
    // set_vtable_global reuses that global in place instead of emitting a
    // second, suffixed symbol that nothing reads.
    let vtable_name = format!("_ivtable_{}_{}", interface_name, class_name);
    Some(set_vtable_global(context, module, &vtable_name, &entries))
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

    // A zero object pointer makes every interface call dereference null.
    if object_ptr.is_null() {
        return Err("codegen: null object pointer in interface fat pointer".to_string());
    }

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
    guard_indirect_target(context, builder, obj_ptr, "iface.obj")?;

    let vt_ptr = builder
        .build_extract_value(fat_ptr_struct, 1, "iface.vt")
        .map_err(|e| format!("build_extract_value iface.vt failed: {:?}", e))?
        .into_pointer_value();
    guard_indirect_target(context, builder, vt_ptr, "iface.vtable")?;

    // create_interface_vtable sorts method names, so mirror that order here;
    // using declaration order would dispatch to the wrong vtable slot.
    let mut slot_names: Vec<&String> = interface_methods.iter().collect();
    slot_names.sort();
    let method_idx = slot_names
        .iter()
        .position(|n| n.as_str() == method_name)
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
    guard_indirect_target(context, builder, fn_ptr, "iface.fn")?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use inkwell::module::Module;

    fn positioned_builder<'ctx>(context: &'ctx Context, module: &Module<'ctx>) -> Builder<'ctx> {
        let builder = context.create_builder();
        let fn_ty = context.void_type().fn_type(&[], false);
        let f = module.add_function("reshape_harness", fn_ty, None);
        let entry = context.append_basic_block(f, "entry");
        builder.position_at_end(entry);
        builder
    }

    fn container_type<'ctx>(context: &'ctx Context) -> StructType<'ctx> {
        let ptr = context.ptr_type(AddressSpace::default());
        context.struct_type(&[context.i64_type().into(), ptr.into()], false)
    }

    // Force real instructions instead of LLVM constant folding the reshape.
    fn non_const<'ctx>(
        builder: &Builder<'ctx>,
        ty: StructType<'ctx>,
        value: StructValue<'ctx>,
    ) -> StructValue<'ctx> {
        let slot = builder.build_alloca(ty, "arg.slot").expect("alloca");
        builder.build_store(slot, value).expect("store");
        builder
            .build_load(ty, slot, "arg.val")
            .expect("load")
            .into_struct_value()
    }

    #[test]
    fn reshape_container_to_fat_ptr_converts_length_word() {
        let context = Context::create();
        let module = context.create_module("reshape_container");
        let builder = positioned_builder(&context, &module);
        let from = container_type(&context);
        let to = build_interface_fat_ptr_type(&context);
        let ptr_ty = context.ptr_type(AddressSpace::default());
        let value = from.const_named_struct(&[
            context.i64_type().const_int(42, false).into(),
            ptr_ty.const_null().into(),
        ]);
        let input = non_const(&builder, from, value);
        let out = reshape_container_or_fat_ptr(&builder, input, to).expect("reshape");
        assert_eq!(out.get_type(), to);
        let sink = builder.build_alloca(to, "out.slot").expect("alloca");
        builder.build_store(sink, out).expect("store");
        let ir = module.print_to_string().to_string();
        assert!(ir.contains("inttoptr"), "expected inttoptr in:\n{}", ir);
    }

    #[test]
    fn reshape_fat_ptr_to_container_converts_object_pointer() {
        let context = Context::create();
        let module = context.create_module("reshape_fat");
        let builder = positioned_builder(&context, &module);
        let from = build_interface_fat_ptr_type(&context);
        let to = container_type(&context);
        let ptr_ty = context.ptr_type(AddressSpace::default());
        let value = from.const_named_struct(&[ptr_ty.const_null().into(), ptr_ty.const_null().into()]);
        let input = non_const(&builder, from, value);
        let out = reshape_container_or_fat_ptr(&builder, input, to).expect("reshape");
        assert_eq!(out.get_type(), to);
        let sink = builder.build_alloca(to, "out.slot").expect("alloca");
        builder.build_store(sink, out).expect("store");
        let ir = module.print_to_string().to_string();
        assert!(ir.contains("ptrtoint"), "expected ptrtoint in:\n{}", ir);
    }

    #[test]
    fn reshape_widens_and_narrows_int_field() {
        let context = Context::create();
        let module = context.create_module("reshape_int");
        let builder = positioned_builder(&context, &module);
        let ptr_ty = context.ptr_type(AddressSpace::default());
        let small = context.struct_type(&[context.i32_type().into(), ptr_ty.into()], false);
        let large = container_type(&context);
        let value = small.const_named_struct(&[
            context.i32_type().const_int(7, false).into(),
            ptr_ty.const_null().into(),
        ]);
        let input = non_const(&builder, small, value);
        let out = reshape_container_or_fat_ptr(&builder, input, large).expect("reshape");
        assert_eq!(out.get_type(), large);
        let sink = builder.build_alloca(large, "out.slot").expect("alloca");
        builder.build_store(sink, out).expect("store");
        let ir = module.print_to_string().to_string();
        assert!(ir.contains("sext"), "expected sext in:\n{}", ir);

        let back = reshape_container_or_fat_ptr(&builder, non_const(&builder, large, out), small)
            .expect("reshape back");
        assert_eq!(back.get_type(), small);
        let sink = builder.build_alloca(small, "back.slot").expect("alloca");
        builder.build_store(sink, back).expect("store");
        let ir = module.print_to_string().to_string();
        assert!(ir.contains("trunc"), "expected trunc in:\n{}", ir);
    }

    #[test]
    fn reshape_rejects_bad_shapes() {
        let context = Context::create();
        let module = context.create_module("reshape_bad");
        let builder = positioned_builder(&context, &module);
        let ptr_ty = context.ptr_type(AddressSpace::default());
        let three = context.struct_type(
            &[context.i64_type().into(), ptr_ty.into(), context.i64_type().into()],
            false,
        );
        let value = three.const_named_struct(&[
            context.i64_type().const_int(1, false).into(),
            ptr_ty.const_null().into(),
            context.i64_type().const_int(2, false).into(),
        ]);
        let err = reshape_container_or_fat_ptr(&builder, value, container_type(&context))
            .expect_err("three-field struct must be rejected");
        assert!(err.contains("expected two-field structs"), "got: {}", err);

        let bad = context.struct_type(&[context.f64_type().into(), ptr_ty.into()], false);
        let value = bad.const_named_struct(&[
            context.f64_type().const_float(1.0).into(),
            ptr_ty.const_null().into(),
        ]);
        let err = reshape_container_or_fat_ptr(&builder, value, build_interface_fat_ptr_type(&context))
            .expect_err("float field must be rejected");
        assert!(err.contains("cannot reshape field 0"), "got: {}", err);
    }

    #[test]
    fn interface_vtable_prefers_compiled_body_over_declaration() {
        let context = Context::create();
        let module = context.create_module("iface_vt");
        let ptr = context.ptr_type(AddressSpace::default());
        let fn_ty = context.void_type().fn_type(&[ptr.into()], false);

        // A bodyless declaration must not win over a compiled default clone.
        let mut class_methods = HashMap::new();
        class_methods.insert("run".to_string(), module.add_function("Other_run", fn_ty, None));
        let clone = module.add_function("MyClass_run", fn_ty, None);
        context.append_basic_block(clone, "entry");

        let vt = create_interface_vtable(
            &context,
            &module,
            "MyClass",
            "Runner",
            &["run".to_string()],
            &class_methods,
            &HashMap::new(),
        )
        .expect("vtable");
        assert!(vt.as_pointer_value().is_const());
        let ir = module.print_to_string().to_string();
        assert!(
            ir.contains("@MyClass_run"),
            "expected the compiled clone in:\n{}",
            ir
        );
    }

    #[test]
    fn interface_vtable_never_references_a_bodyless_default() {
        let context = Context::create();
        let module = context.create_module("iface_vt_bodyless");
        let ptr = context.ptr_type(AddressSpace::default());
        let fn_ty = context.void_type().fn_type(&[ptr.into()], false);

        let mut defaults = HashMap::new();
        defaults.insert(
            "run".to_string(),
            module.add_function("Runner_default_run", fn_ty, None),
        );
        let compiled = module.add_function("Runner_default_stop", fn_ty, None);
        context.append_basic_block(compiled, "entry");
        defaults.insert("stop".to_string(), compiled);

        let vt = create_interface_vtable(
            &context,
            &module,
            "MyClass",
            "Runner",
            &["run".to_string(), "stop".to_string()],
            &HashMap::new(),
            &defaults,
        )
        .expect("vtable");
        assert!(vt.as_pointer_value().is_const());
        let ir = module.print_to_string().to_string();
        let initializer = ir
            .lines()
            .find(|line| line.starts_with("@_ivtable_Runner_MyClass ="))
            .expect("vtable global");
        assert!(
            !initializer.contains("@Runner_default_run"),
            "bodyless default must not become an undefined linker symbol:\n{}",
            initializer
        );
        assert!(
            initializer.contains("@Runner_default_stop"),
            "compiled default must be used:\n{}",
            initializer
        );
    }

    #[test]
    fn interface_vtable_reuses_its_provisional_global() {
        let context = Context::create();
        let module = context.create_module("iface_vt_reuse");
        let ptr = context.ptr_type(AddressSpace::default());
        let fn_ty = context.void_type().fn_type(&[ptr.into()], false);
        let function = module.add_function("MyClass_run", fn_ty, None);
        context.append_basic_block(function, "entry");
        let mut methods = HashMap::new();
        methods.insert("run".to_string(), function);

        let first = create_interface_vtable(
            &context,
            &module,
            "MyClass",
            "Runner",
            &["run".to_string()],
            &methods,
            &HashMap::new(),
        )
        .expect("first vtable");
        let second = create_interface_vtable(
            &context,
            &module,
            "MyClass",
            "Runner",
            &["run".to_string()],
            &methods,
            &HashMap::new(),
        )
        .expect("final vtable");
        assert_eq!(first, second, "the final table must replace, not duplicate");
        assert_eq!(
            module
                .get_globals()
                .filter(|g| g
                    .get_name()
                    .to_str()
                    .unwrap_or("")
                    .starts_with("_ivtable_Runner_MyClass"))
                .count(),
            1
        );
    }

    fn initializer_line(ir: &str, global_prefix: &str) -> String {
        ir.lines()
            .find(|line| line.starts_with(global_prefix))
            .unwrap_or("")
            .to_string()
    }

    #[test]
    fn specialized_generic_class_vtable_fills_slots_from_compiled_methods() {
        let context = Context::create();
        let module = context.create_module("generic_vt");
        let ptr = context.ptr_type(AddressSpace::default());
        let fn_ty = context.void_type().fn_type(&[ptr.into()], false);

        // The map carries a bodyless declaration; the real body was emitted
        // under an arity-suffixed name by monomorphization.
        let mut methods = HashMap::new();
        methods.insert(
            "get".to_string(),
            module.add_function("Box__int_get_decl", fn_ty, None),
        );
        let compiled = module.add_function("Box__int_get__a2", fn_ty, None);
        context.append_basic_block(compiled, "entry");

        let vt = create_vtable_global(
            &context,
            &module,
            "Box__int",
            &["get".to_string()],
            &methods,
        )
        .expect("class vtable");
        let ir = module.print_to_string().to_string();
        let initializer = initializer_line(&ir, "@_vtable_Box__int =");
        assert!(
            initializer.contains("@Box__int_get__a2"),
            "compiled specialization must fill the slot:\n{}",
            initializer
        );
        assert!(
            !initializer.contains("null"),
            "no slot may be left unresolved:\n{}",
            initializer
        );
        assert!(vt.as_pointer_value().is_const());
    }

    #[test]
    fn specialized_generic_class_interface_vtable_uses_per_class_default_clone() {
        let context = Context::create();
        let module = context.create_module("generic_iface_vt");
        let ptr = context.ptr_type(AddressSpace::default());
        let fn_ty = context.void_type().fn_type(&[ptr.into()], false);

        let mut class_methods = HashMap::new();
        class_methods.insert(
            "label".to_string(),
            module.add_function("Box__int_label_decl", fn_ty, None),
        );
        let clone = module.add_function("Box__int_label", fn_ty, None);
        context.append_basic_block(clone, "entry");

        // The shared default comes from the real interface declaration; the
        // per-class clone must still win so `this` calls dispatch to Box__int.
        let mut defaults = HashMap::new();
        let shared = module.add_function("Tag_default_label", fn_ty, None);
        context.append_basic_block(shared, "entry");
        defaults.insert("label".to_string(), shared);

        create_interface_vtable(
            &context,
            &module,
            "Box__int",
            "Tag",
            &["label".to_string()],
            &class_methods,
            &defaults,
        )
        .expect("interface vtable");
        let ir = module.print_to_string().to_string();
        let initializer = initializer_line(&ir, "@_ivtable_Tag_Box__int =");
        assert!(
            initializer.contains("@Box__int_label"),
            "per-class clone must fill the interface slot:\n{}",
            initializer
        );
        assert!(
            !initializer.contains("null"),
            "interface vtable must be complete:\n{}",
            initializer
        );
    }

    #[test]
    fn missing_vtable_method_uses_descriptive_aborting_stub() {
        let context = Context::create();
        let module = context.create_module("missing_stub");
        let vt = create_vtable_global(
            &context,
            &module,
            "Broken",
            &["missing".to_string()],
            &HashMap::new(),
        )
        .expect("vtable");

        let ir = module.print_to_string().to_string();
        assert!(
            ir.contains("_null_vtable_stub_Broken_missing"),
            "expected a named stub in:\n{}",
            ir
        );
        assert!(
            ir.contains("missing method 'missing' on class 'Broken'"),
            "stub must name the class and method in:\n{}",
            ir
        );
        assert!(ir.contains("call void @abort()"), "stub must abort:\n{}", ir);
        assert!(
            !vt.to_string().contains(", null"),
            "no bare null slot:\n{}",
            vt
        );
        assert!(module.verify().is_ok(), "stub IR must verify:\n{}", ir);
    }

    #[test]
    fn repeated_interface_compilation_reuses_stub_and_vtable() {
        let context = Context::create();
        let module = context.create_module("repeated_iface");
        for _ in 0..2 {
            create_interface_vtable(
                &context,
                &module,
                "Box__int",
                "Tag",
                &["label".to_string()],
                &HashMap::new(),
                &HashMap::new(),
            )
            .expect("interface vtable");
        }
        let stubs = module
            .get_functions()
            .filter(|f| f
                .get_name()
                .to_str()
                .is_ok_and(|n| n.starts_with("_null_vtable_stub_")))
            .count();
        assert_eq!(stubs, 1, "a second pass must reuse the stub");
        let messages = module
            .get_globals()
            .filter(|g| g
                .get_name()
                .to_str()
                .is_ok_and(|n| n.starts_with("_null_vtable_msg_")))
            .count();
        assert_eq!(messages, 1, "a second pass must reuse the message");
        let vtables = module
            .get_globals()
            .filter(|g| g
                .get_name()
                .to_str()
                .is_ok_and(|n| n.starts_with("_ivtable_Tag_Box__int")))
            .count();
        assert_eq!(vtables, 1, "a second pass must reuse the vtable global");
        assert!(module.verify().is_ok());
    }
}
