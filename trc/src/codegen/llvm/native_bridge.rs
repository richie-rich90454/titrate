//! Native function bridge for the LLVM codegen.
//!
//! This module provides the mapping from Titrate native function names
//! (e.g. `Math_sin`) to C-ABI wrapper symbols (e.g. `titrate_Math_sin`),
//! and the marshalling code that converts between LLVM-level values and
//! the C-ABI `TitrateValue` tagged union.
//!
//! All native wrappers share the same C signature:
//!   `TitrateValue titrate_<Name>(const TitrateValue* args, size_t arg_count)`
//!
//! The LLVM representation of `TitrateValue` is:
//!   `{ i32 tag, i32 pad, [16 x i8] payload }`  (24 bytes)
//!
//! Marshalling strategy:
//! - Primitives (int, long, double, float, bool, char) are stored directly
//!   in the 16-byte payload via type-punning through an alloca.
//! - Strings (`{ i64, i8* }`) are stored directly (they fit in 16 bytes).
//! - The result is unmarshalled by reading the payload back as the expected
//!   LLVM type.

use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};
use inkwell::types::{BasicTypeEnum, StructType};
use inkwell::values::{BasicValueEnum, PointerValue};
use inkwell::AddressSpace;

use crate::ast::{Expr, Type};
use crate::bytecode::vm::natives::lookup_builtin_native;

// C-ABI type tags (must match titrate_native/src/lib.rs).
pub const TV_VOID: i32 = 0;
pub const TV_NULL: i32 = 1;
pub const TV_BOOL: i32 = 2;
pub const TV_BYTE: i32 = 3;
pub const TV_SHORT: i32 = 4;
pub const TV_INT: i32 = 5;
pub const TV_LONG: i32 = 6;
/// Vast/Uvast values are carried as handles registered in the runtime's
/// thread-local registry (the C payload union cannot hold i128 directly).
pub const TV_VAST: i32 = 7;
pub const TV_UVAST: i32 = 8;
pub const TV_DOUBLE: i32 = 10;
pub const TV_FLOAT: i32 = 9;
pub const TV_CHAR: i32 = 11;
pub const TV_STRING: i32 = 12;
pub const TV_ARRAY: i32 = 13;
/// Opaque handle (JsonValue, class instances registered by the native runtime).
pub const TV_CLASS_INSTANCE: i32 = 14;
/// Runtime halves are stored as f32; LLVM lowers `half` to f16.
pub const TV_HALF: i32 = 19;
/// Runtime quads are stored as f64; LLVM lowers `quad` to fp128.
pub const TV_QUAD: i32 = 20;

/// Build an alloca in the function's entry block so loop-body temporaries do
/// not grow the stack on every iteration.
fn entry_alloca<'ctx>(
    builder: &Builder<'ctx>,
    ty: BasicTypeEnum<'ctx>,
    name: &str,
) -> Result<PointerValue<'ctx>, String> {
    let saved = builder.get_insert_block();
    if let Some(entry) = saved
        .and_then(|b| b.get_parent())
        .and_then(|f| f.get_first_basic_block())
    {
        if let Some(first) = entry.get_first_instruction() {
            builder.position_before(&first);
        } else {
            builder.position_at_end(entry);
        }
    }
    let result = builder
        .build_alloca(ty, name)
        .map_err(|e| format!("build_alloca '{}' failed: {:?}", name, e));
    // Mirror Rust's 8-byte TitrateValue alignment (LLVM computes only 4 for
    // {i32, i32, [16 x i8]}); native array iteration dereferences aligned.
    if let Ok(ptr) = &result {
        if let Some(instr) = ptr.as_instruction() {
            let _ = instr.set_alignment(8);
        }
    }
    if let Some(saved) = saved {
        builder.position_at_end(saved);
    }
    result
}

/// Return the LLVM struct type for `TitrateValue`: `{ i32, i32, [16 x i8] }`.
pub fn titrate_value_type<'ctx>(context: &'ctx Context) -> StructType<'ctx> {
    let i32_ty = context.i32_type();
    let payload_ty = context.i8_type().array_type(16);
    context.struct_type(&[i32_ty.into(), i32_ty.into(), payload_ty.into()], false)
}

/// Return the LLVM struct type for `TitrateArray`: `{ i64, ptr }`.
pub fn titrate_array_type<'ctx>(context: &'ctx Context) -> StructType<'ctx> {
    let i64_ty = context.i64_type();
    let ptr_ty = context.ptr_type(AddressSpace::default());
    context.struct_type(&[i64_ty.into(), ptr_ty.into()], false)
}

/// Convert a native function name (e.g. `Math_sin`) to its C wrapper symbol
/// (e.g. `titrate_Math_sin`).
pub fn native_name_to_c_name(name: &str) -> String {
    format!("titrate_{}", name)
}

/// Return true if `name` is a registered native function.
pub fn is_native_function(name: &str) -> bool {
    lookup_builtin_native(name).is_some()
}

/// True when a Titrate type name crosses the C ABI as an object reference or
/// an opaque runtime handle rather than as a primitive scalar/struct. Such
/// values are carried as 64-bit bits under a class-instance tag; storing them
/// under `TV_INT` would truncate a pointer and mislabel the payload.
pub fn is_opaque_reference_type(name: &str) -> bool {
    if matches!(
        name,
        "bool"
            | "byte"
            | "short"
            | "int"
            | "long"
            | "vast"
            | "uvast"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "size"
            | "float"
            | "double"
            | "half"
            | "quad"
            | "char"
            | "string"
            | "void"
            | "array"
            | "ArrayList"
            | "HashMap"
            | "Result"
            | "Ok"
            | "Err"
            | "Optional"
    ) {
        return false;
    }
    name.chars().next().is_some_and(|c| c.is_ascii_uppercase())
}

/// Convert an object pointer or handle id into the 64-bit payload slot used
/// by the class-instance / handle tags.
fn opaque_payload_bits<'ctx>(
    context: &'ctx Context,
    builder: &Builder<'ctx>,
    value: BasicValueEnum<'ctx>,
    name: &str,
) -> Result<BasicValueEnum<'ctx>, String> {
    let i64_ty = context.i64_type();
    if value.is_pointer_value() {
        let bits = builder
            .build_ptr_to_int(value.into_pointer_value(), i64_ty, name)
            .map_err(|e| format!("build_ptr_to_int {} failed: {:?}", name, e))?;
        Ok(bits.into())
    } else if value.is_int_value() {
        let iv = value.into_int_value();
        let width = iv.get_type().get_bit_width();
        if width == 64 {
            Ok(iv.into())
        } else if width < 64 {
            let ext = builder
                .build_int_z_extend(iv, i64_ty, name)
                .map_err(|e| format!("build_int_z_extend {} failed: {:?}", name, e))?;
            Ok(ext.into())
        } else {
            let trunc = builder
                .build_int_truncate(iv, i64_ty, name)
                .map_err(|e| format!("build_int_truncate {} failed: {:?}", name, e))?;
            Ok(trunc.into())
        }
    } else {
        Err(format!("codegen: cannot marshal '{}' value as an opaque handle", name))
    }
}

/// Try to extract a native function name from a call callee expression.
///
/// Returns `Some("Math_sin")` for:
/// - `Math.sin(x)`  -> MemberAccess(Identifier("Math"), "sin")
/// - `Math::sin(x)` -> StaticCall { class_name: "Math", method: "sin" }
///
/// Returns `Some("parseInt")` for:
/// - `parseInt(s)`  -> Identifier("parseInt")
///
/// Returns `None` if the callee is not a recognizable native call pattern.
pub fn try_native_call_name(callee: &Expr) -> Option<String> {
    match callee {
        // Module.method(args): Math.sin(x) -> Math_sin
        Expr::MemberAccess(namespace, method, _) => {
            if let Expr::Identifier(ns, _) = &**namespace {
                // These are stdlib wrappers that return compiled class objects
                // (Json.parse converts a native handle, File.open builds a real
                // File instance). Calling the same-named native directly would
                // leave an opaque handle where compiled code expects a pointer.
                if (ns == "Json" && (method == "parse" || method == "stringify"))
                    || (ns == "File" && method == "open")
                {
                    return None;
                }
                let native_name = format!("{}_{}", ns, method);
                if is_native_function(&native_name) {
                    return Some(native_name);
                }
            }
            None
        }
        // ClassName::method(args): Math::sin(x) -> Math_sin
        Expr::StaticCall { class_name, method, .. } => {
            let native_name = format!("{}_{}", class_name, method);
            if is_native_function(&native_name) {
                return Some(native_name);
            }
            None
        }
        // Bare function: parseInt(s) -> parseInt
        Expr::Identifier(name, _) => {
            if is_native_function(name) {
                Some(name.clone())
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Declare a single native wrapper function in the LLVM module.
///
/// The function signature is:
///   `TitrateValue titrate_<name>(TitrateValue* args, usize arg_count)`
pub fn declare_native<'ctx>(
    context: &'ctx Context,
    module: &Module<'ctx>,
    native_name: &str,
) {
    let tv_ty = titrate_value_type(context);
    let tv_ptr = context.ptr_type(AddressSpace::default());
    let usize_type = if cfg!(target_pointer_width = "64") {
        context.i64_type()
    } else {
        context.i32_type()
    };
    let fn_ty = tv_ty.fn_type(&[tv_ptr.into(), usize_type.into()], false);
    let c_name = native_name_to_c_name(native_name);
    if module.get_function(&c_name).is_none() {
        module.add_function(&c_name, fn_ty, Some(Linkage::External));
    }
}

/// Declare all native wrapper functions in the LLVM module.
pub fn declare_all_natives<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    // We declare natives lazily (on first use) to avoid declaring 359
    // functions that may never be called. The `declare_native` function
    // is called from `emit_native_call` when a native is first referenced.
    let _ = (context, module);
}

/// Marshal an LLVM value into a `TitrateValue` struct value.
///
/// The `ty` parameter is the Titrate type of the value, used to determine
/// the correct tag.
pub fn marshal_to_titrate<'ctx>(
    context: &'ctx Context,
    builder: &Builder<'ctx>,
    value: BasicValueEnum<'ctx>,
    ty: &Type,
) -> Result<inkwell::values::StructValue<'ctx>, String> {
    let i32_ty = context.i32_type();
    let payload_ty = context.i8_type().array_type(16);
    let tv_ty = titrate_value_type(context);

    let type_name = ty.name();
    // The C ABI carries vast/uvast through a thread-local handle registry that
    // generated IR cannot populate, and quads as f64 (narrowing an fp128 would
    // silently lose precision). Reject them instead of corrupting the operand.
    match type_name {
        "vast" | "uvast" => {
            return Err(format!(
                "codegen: native bridge cannot marshal '{}': the runtime carries it through a handle registry that generated IR cannot populate",
                type_name
            ))
        }
        "quad" => {
            return Err(
                "codegen: native bridge cannot marshal 'quad': the runtime carries quads as f64, which would truncate an fp128 operand"
                    .to_string(),
            )
        }
        _ => {}
    }

    let (tag, stored_value): (i32, BasicValueEnum<'ctx>) = match type_name {
        "bool" => (TV_BOOL, value),
        "byte" => (TV_BYTE, value),
        "short" => (TV_SHORT, value),
        "int" | "u8" | "u16" | "u32" => (TV_INT, value),
        "long" | "u64" | "size" => (TV_LONG, value),
        "float" => (TV_FLOAT, value),
        // The runtime's Half payload is an f32; widen the LLVM f16 so the
        // first four payload bytes decode to the exact original value.
        "half" => {
            let f16 = value.into_float_value();
            let f32 = builder
                .build_float_ext(f16, context.f32_type(), "tv.half.ext")
                .map_err(|e| format!("build_float_ext half failed: {:?}", e))?;
            (TV_HALF, f32.into())
        }
        "double" => (TV_DOUBLE, value),
        "char" => (TV_CHAR, value),
        "string" => (TV_STRING, value),
        "array" | "ArrayList" | "HashMap" | "Variant" => (TV_ARRAY, value),
        // Only the `null` literal infers to void; tag it NULL so string
        // conversion yields "null" like the VM instead of int 0.
        "void" => (TV_NULL, value),
        // JsonValue, user classes and runtime handles all carry a 64-bit
        // pointer/id in the payload. Class-instance ids that are absent from
        // the runtime registry are preserved verbatim by the C ABI, so raw
        // compiled object pointers survive the round trip.
        name if is_opaque_reference_type(name) => {
            let bits = opaque_payload_bits(context, builder, value, "tv.obj.bits")?;
            (TV_CLASS_INSTANCE, bits)
        }
        _ if value.is_pointer_value() => {
            let bits = opaque_payload_bits(context, builder, value, "tv.ptr.bits")?;
            (TV_CLASS_INSTANCE, bits)
        }
        _ => (TV_INT, value),
    };

    let tag_val = i32_ty.const_int(tag as u64, false);
    let pad_val = i32_ty.const_int(0, false);

    // Build the payload by storing the value into a [16 x i8] alloca.
    let payload_alloca = entry_alloca(builder, payload_ty.into(), "tv.payload")?;

    // Zero the payload first so the load below never reads uninitialized
    // (poison) bytes. Reading poison makes LLVM's optimizer treat the
    // marshalled value as undefined; when the stored value carries a `range`
    // attribute from an inlined function, it mis-compiles the marshal (into
    // `store to poison`), corrupting the argument or crashing the binary.
    builder.build_store(payload_alloca, payload_ty.const_zero())
        .map_err(|e| format!("build_store payload zero failed: {:?}", e))?;

    // Bitcast the payload alloca to the value's type and store.
    let val_ptr = builder.build_bit_cast(payload_alloca, context.ptr_type(AddressSpace::default()), "tv.valptr")
        .map_err(|e| format!("build_bit_cast valptr failed: {:?}", e))?
        .into_pointer_value();
    builder.build_store(val_ptr, stored_value)
        .map_err(|e| format!("build_store payload failed: {:?}", e))?;

    // Load the payload bytes.
    let payload_val = builder.build_load(payload_ty, payload_alloca, "tv.payload.val")
        .map_err(|e| format!("build_load payload failed: {:?}", e))?;

    // Assemble the struct.
    let undef = tv_ty.const_zero();
    let result = builder.build_insert_value(undef, tag_val, 0, "tv.tag")
        .map_err(|e| format!("build_insert_value tag failed: {:?}", e))?;
    let result = builder.build_insert_value(result, pad_val, 1, "tv.pad")
        .map_err(|e| format!("build_insert_value pad failed: {:?}", e))?;
    let result = builder.build_insert_value(result, payload_val, 2, "tv.payload.final")
        .map_err(|e| format!("build_insert_value payload failed: {:?}", e))?;

    match result {
        inkwell::values::AggregateValueEnum::StructValue(sv) => Ok(sv),
        _ => Err("expected struct value from insert_value".to_string()),
    }
}

/// Unmarshal a `TitrateValue` struct back into an LLVM value of the expected type.
///
/// This reads the 16-byte payload and reinterprets it as the expected type
/// via type-punning through an alloca. Use
/// [`unmarshal_from_titrate_declared`] when the call site knows the declared
/// Titrate return type; the name-table-inferred `expected_ty` is only a last
/// resort and can misclassify generic or user-typed calls (e.g. an
/// `ArrayList.get` element declared as a class but listed as `string`).
pub fn unmarshal_from_titrate<'ctx>(
    context: &'ctx Context,
    builder: &Builder<'ctx>,
    tv: inkwell::values::StructValue<'ctx>,
    expected_ty: &Type,
) -> Result<BasicValueEnum<'ctx>, String> {
    unmarshal_from_titrate_declared(context, builder, tv, expected_ty, None)
}

/// Unmarshal with the call site's declared Titrate return type taking
/// precedence over the name-table-inferred type. `void`/`unknown` declarations
/// carry no usable shape and are ignored.
pub fn unmarshal_from_titrate_declared<'ctx>(
    context: &'ctx Context,
    builder: &Builder<'ctx>,
    tv: inkwell::values::StructValue<'ctx>,
    expected_ty: &Type,
    declared_ty: Option<&Type>,
) -> Result<BasicValueEnum<'ctx>, String> {
    let declared = declared_ty.filter(|t| {
        !matches!(t.name(), "void" | "unknown" | "")
    });
    let type_name = declared.unwrap_or(expected_ty).name();
    // Vast/Uvast arrive as handles whose value lives only in the runtime's
    // registry (not in the payload), and quads are narrowed to f64 by the C
    // ABI. Neither can be reconstructed as an i128/fp128 in generated IR.
    match type_name {
        "vast" | "uvast" => {
            return Err(format!(
                "codegen: native bridge cannot unmarshal '{}': the runtime returns a handle whose value is not present in the payload",
                type_name
            ))
        }
        "quad" => {
            return Err(
                "codegen: native bridge cannot unmarshal 'quad': the runtime carries quads as f64, which cannot reconstruct an fp128 value"
                    .to_string(),
            )
        }
        _ => {}
    }

    let payload_ty = context.i8_type().array_type(16);

    // Extract the payload from the TitrateValue struct (field 2 in {i32, i32, [16xi8]}).
    let payload = builder.build_extract_value(tv, 2, "tv.result.payload")
        .map_err(|e| format!("build_extract_value payload failed: {:?}", e))?;

    // Store the payload bytes into an alloca.
    let payload_alloca = entry_alloca(builder, payload_ty.into(), "tv.result.alloca")?;
    builder.build_store(payload_alloca, payload)
        .map_err(|e| format!("build_store result payload failed: {:?}", e))?;

    // Object pointers and runtime handles are carried as 64-bit bits. Reading
    // them as i32 truncates the address (the `{i64,ptr}` -> `{ptr,ptr}`
    // mismatch), so load i64 and reconstruct the pointer explicitly.
    let opaque = is_opaque_reference_type(type_name);

    // Determine the expected LLVM type and load from the alloca.
    let target_ty: BasicTypeEnum<'ctx> = match type_name {
        "bool" => context.bool_type().into(),
        "byte" => context.i8_type().into(),
        "short" => context.i16_type().into(),
        "int" | "u8" | "u16" | "u32" => context.i32_type().into(),
        "long" | "u64" | "size" | "JsonValue" => context.i64_type().into(),
        // Runtime halves are f32; load f32 then narrow to LLVM f16 below.
        "float" | "half" => context.f32_type().into(),
        "double" => context.f64_type().into(),
        "char" => context.i32_type().into(),
        "string" => context.struct_type(
            &[context.i64_type().into(), context.ptr_type(AddressSpace::default()).into()],
            false,
        ).into(),
        "array" | "ArrayList" | "HashMap" | "Variant" => context.struct_type(
            &[context.i64_type().into(), context.ptr_type(AddressSpace::default()).into()],
            false,
        ).into(),
        "void" => {
            // Void return: return a dummy i32.
            return Ok(context.i32_type().const_int(0, false).into());
        }
        _ if opaque => context.i64_type().into(),
        _ => context.i32_type().into(),
    };

    let val_ptr = builder.build_bit_cast(
        payload_alloca,
        context.ptr_type(AddressSpace::default()),
        "tv.result.valptr",
    )
    .map_err(|e| format!("build_bit_cast result valptr failed: {:?}", e))?
    .into_pointer_value();

    let loaded = builder.build_load(target_ty, val_ptr, "tv.result.val")
        .map_err(|e| format!("build_load result val failed: {:?}", e))?;
    if type_name == "half" {
        let f32 = loaded.into_float_value();
        let f16 = builder
            .build_float_trunc(f32, context.f16_type(), "tv.result.half")
            .map_err(|e| format!("build_float_trunc half failed: {:?}", e))?;
        Ok(f16.into())
    } else if opaque && loaded.is_int_value() {
        let ptr = builder
            .build_int_to_ptr(
                loaded.into_int_value(),
                context.ptr_type(AddressSpace::default()),
                "tv.result.objptr",
            )
            .map_err(|e| format!("build_int_to_ptr result objptr failed: {:?}", e))?;
        Ok(ptr.into())
    } else {
        Ok(loaded)
    }
}

/// Authoritative return type for every explicitly named native. Each native
/// appears exactly once. Earlier revisions used overlapping `matches!` lists
/// where the first matching list won, so a name could be silently reclassified
/// by an edit to an unrelated list. Prefix rules below apply only to names that
/// are absent from this table.
const NAMED_NATIVE_RETURNS: &[(&str, &str)] = &[
    // stdin helpers used by io::readLine / io::readAll in native builds
    ("IO_readLine", "string"),
    ("IO_readAll", "string"),
    // void
    ("println", "void"),
    ("Sys_exit", "void"),
    ("Sys_sleep", "void"),
    ("Sys_setEnv", "void"),
    ("Thread_sleep", "void"),
    ("Thread_yield", "void"),
    ("Thread_detach", "void"),
    ("Mutex_lock", "void"),
    ("Mutex_unlock", "void"),
    ("RecursiveMutex_lock", "void"),
    ("RecursiveMutex_unlock", "void"),
    ("SharedMutex_sharedLock", "void"),
    ("SharedMutex_sharedUnlock", "void"),
    ("SharedMutex_uniqueLock", "void"),
    ("SharedMutex_uniqueUnlock", "void"),
    ("CondVar_notifyOne", "void"),
    ("CondVar_notifyAll", "void"),
    ("Semaphore_release", "void"),
    ("Semaphore_acquire", "void"),
    ("File_close", "void"),
    ("File_flush", "void"),
    ("File_write", "void"),
    ("File_writeBytes", "void"),
    ("File_truncate", "void"),
    ("File_setModified", "void"),
    ("Socket_close", "void"),
    ("UdpSocket_close", "void"),
    ("Ssl_close", "void"),
    ("Sqlite_close", "void"),
    ("Mmap_close", "void"),
    ("Mmap_flush", "void"),
    ("ZipFile_close", "void"),
    ("ZipWriter_close", "void"),
    ("Hasher_reset", "void"),
    ("Hasher_close", "void"),
    ("Gc_collect", "void"),
    ("Random_seed", "void"),
    ("Os_chmod", "void"),
    ("Os_makedirs", "void"),
    ("Os_symlink", "void"),
    ("Os_kill", "void"),
    ("Os_umask", "void"),
    ("Os_removedirs", "void"),
    ("Os_renames", "void"),
    ("Os_replace", "void"),
    ("Os_link", "void"),
    ("Os_utime", "void"),
    ("Os_unsetenv", "void"),
    ("Signal_raise", "void"),
    ("File_delete", "void"),
    ("File_rename", "void"),
    ("Dir_remove", "void"),
    ("Dir_removeTree", "void"),
    ("Env_set", "void"),
    ("Os_setenv", "void"),
    ("Os_chdir", "void"),
    ("OnceFlag_callOnce", "void"),
    ("AtomicInt_set", "void"),
    ("AtomicBool_set", "void"),
    ("AtomicLong_set", "void"),
    ("AtomicRef_set", "void"),
    // long (includes the four math natives that return Value::Long)
    ("Math_absInt", "long"),
    ("Math_maxInt", "long"),
    ("Math_minInt", "long"),
    ("Math_round", "long"),
    ("Time_now", "long"),
    ("Time_millis", "long"),
    ("Time_monotonic", "long"),
    ("Time_perfCounter", "long"),
    ("Time_epochSeconds", "long"),
    ("Time_nanos", "long"),
    ("Time_sleep", "long"),
    ("Thread_getId", "long"),
    ("Thread_currentId", "long"),
    ("Process_id", "long"),
    ("Os_getpid", "long"),
    ("Os_getppid", "long"),
    ("Random_nextLong", "long"),
    ("File_size", "long"),
    ("File_tell", "long"),
    ("File_lastModified", "long"),
    ("Fs_size", "long"),
    ("Fs_totalSpace", "long"),
    ("Fs_freeSpace", "long"),
    ("Sqlite_lastInsertId", "long"),
    ("Sqlite_getInt", "long"),
    ("Socket_getLocalPort", "long"),
    ("Socket_getRemotePort", "long"),
    ("UdpSocket_lastSenderPort", "long"),
    // double
    ("JsonValue_asNumber", "double"),
    ("Double_parseDouble", "double"),
    ("Double_parse", "double"),
    // Pinned explicitly: the old Math_ prefix rule also yielded double for
    // this name, but it was additionally (and ineffectually) listed as int.
    ("Math_getExponent", "double"),
    // JsonValue handle
    ("Json_parse", "JsonValue"),
    // string
    ("toString", "string"),
    ("String_trim", "string"),
    ("String_trimStart", "string"),
    ("String_trimEnd", "string"),
    ("String_toUpperCase", "string"),
    ("String_toLowerCase", "string"),
    ("String_replace", "string"),
    ("String_fromCharCode", "string"),
    ("String_charAt", "string"),
    ("String_substring", "string"),
    ("String_padLeft", "string"),
    ("String_padRight", "string"),
    ("String_join", "string"),
    ("Path_join", "string"),
    ("Path_basename", "string"),
    ("Path_dirname", "string"),
    ("Path_extension", "string"),
    ("Json_stringify", "string"),
    ("Os_name", "string"),
    ("Os_arch", "string"),
    ("Os_family", "string"),
    ("Os_userName", "string"),
    ("Os_hostName", "string"),
    ("Os_release", "string"),
    ("Os_version", "string"),
    ("Os_strerror", "string"),
    ("Os_uname", "string"),
    ("Titrate_version", "string"),
    ("Hash_md5", "string"),
    ("Hash_sha1", "string"),
    ("Hash_sha256", "string"),
    ("Hash_sha384", "string"),
    ("Hash_sha512", "string"),
    ("Hash_sha224", "string"),
    ("Hash_sha3_256", "string"),
    ("Hash_sha3_384", "string"),
    ("Hash_sha3_512", "string"),
    ("Hash_sha3_224", "string"),
    ("Hash_blake2b", "string"),
    ("Hash_blake2s", "string"),
    ("Hash_shake128", "string"),
    ("Hash_shake256", "string"),
    ("Hasher_hexDigest", "string"),
    ("Hash_crc32", "string"),
    ("Base64_encode", "string"),
    ("Base64_decode", "string"),
    ("Hex_encode", "string"),
    ("Hex_decode", "string"),
    ("Url_encode", "string"),
    ("Url_decode", "string"),
    ("TypeName_of", "string"),
    ("Regex_replace", "string"),
    ("Regex_subN", "string"),
    // Regex finders return the "start,end,text" wire format as a string;
    // reading that payload as an int yields garbage and can loop forever.
    ("Regex_find", "string"),
    ("Regex_findGroups", "string"),
    ("Regex_findWithFlags", "string"),
    ("Regex_findNamedCapture", "string"),
    ("Regex_findAllNamedCaptures", "string"),
    ("Time_format", "string"),
    ("Sqlite_columnName", "string"),
    ("Sqlite_getString", "string"),
    ("Socket_getLocalAddress", "string"),
    ("Socket_getRemoteAddress", "string"),
    ("Socket_inetNtop", "string"),
    ("Socket_getAddrInfo", "string"),
    ("UdpSocket_lastSenderHost", "string"),
    ("ArrayList_get", "string"),
    ("ArrayList_first", "string"),
    ("ArrayList_last", "string"),
    ("ArrayList_toString", "string"),
    ("HashMap_get", "string"),
    ("HashMap_toString", "string"),
    ("File_readFile", "string"),
    ("File_readLine", "string"),
    ("File_readChunk", "string"),
    ("File_readLines", "string"),
    ("Boolean_toString", "string"),
    ("Integer_toString", "string"),
    ("Long_toString", "string"),
    ("Double_toString", "string"),
    ("Float_toString", "string"),
    ("Sys_workingDir", "string"),
    ("Os_getcwd", "string"),
    ("Os_getenv", "string"),
    ("Os_environ", "string"),
    ("JsonValue_asString", "string"),
    // bool (one-byte payload; reading it as i32 picks up garbage)
    ("Regex_match", "bool"),
    ("Regex_fullMatch", "bool"),
    ("Regex_matchWithFlags", "bool"),
    ("Path_exists", "bool"),
    ("Path_isFile", "bool"),
    ("Path_isDir", "bool"),
    ("Path_isSymlink", "bool"),
    ("Fs_exists", "bool"),
    ("Fs_isFile", "bool"),
    ("Fs_isDir", "bool"),
    ("Mutex_tryLock", "bool"),
    ("RecursiveMutex_tryLock", "bool"),
    ("SharedMutex_trySharedLock", "bool"),
    ("SharedMutex_tryUniqueLock", "bool"),
    ("Semaphore_tryAcquire", "bool"),
    ("AtomicBool_get", "bool"),
    ("AtomicInt_compareAndSwap", "bool"),
    ("AtomicBool_compareAndSwap", "bool"),
    ("AtomicLong_compareAndSwap", "bool"),
    ("AtomicRef_compareAndSwap", "bool"),
    ("Hmac_compareDigest", "bool"),
    ("Os_access", "bool"),
    ("String_startsWith", "bool"),
    ("String_endsWith", "bool"),
    ("String_contains", "bool"),
    ("JsonValue_isNull", "bool"),
    ("JsonValue_isBool", "bool"),
    ("JsonValue_isNumber", "bool"),
    ("JsonValue_isString", "bool"),
    ("JsonValue_isArray", "bool"),
    ("JsonValue_isObject", "bool"),
    ("JsonValue_hasKey", "bool"),
    ("ArrayList_contains", "bool"),
    ("ArrayList_isEmpty", "bool"),
    ("HashMap_containsKey", "bool"),
    ("HashMap_hasKey", "bool"),
    ("HashMap_containsValue", "bool"),
    ("HashMap_isEmpty", "bool"),
    // int
    ("parseInt", "int"),
    ("Integer_parseOr", "int"),
    ("String_length", "int"),
    ("String_indexOf", "int"),
    ("Regex_groupCount", "int"),
    ("Time_getYear", "int"),
    ("Time_getMonth", "int"),
    ("Time_getDay", "int"),
    ("Time_getHour", "int"),
    ("Time_getMinute", "int"),
    ("Time_getSecond", "int"),
    ("Time_dayOfWeek", "int"),
    ("Time_dayOfYear", "int"),
    ("Os_cpuCount", "int"),
    ("ZipFile_entryCount", "int"),
    ("Sqlite_columnCount", "int"),
    ("Sqlite_nextRow", "int"),
    ("Socket_createConnection", "int"),
    ("Socket_createServer", "int"),
    ("Semaphore_availablePermits", "int"),
    ("AtomicInt_get", "int"),
    ("AtomicInt_fetchAdd", "int"),
    ("AtomicInt_fetchSub", "int"),
    ("AtomicInt_fetchOr", "int"),
    ("AtomicInt_fetchAnd", "int"),
    ("AtomicInt_fetchXor", "int"),
    ("AtomicInt_exchange", "int"),
    ("AtomicLong_get", "int"),
    ("AtomicLong_fetchAdd", "int"),
    ("AtomicLong_fetchSub", "int"),
    ("Long_parseLong", "int"),
    ("Socket_inetPton", "int"),
    ("Subprocess_popenWrite", "int"),
    ("ArrayList_size", "int"),
    ("HashMap_size", "int"),
    ("JsonValue_size", "int"),
    // array
    ("Sys_args", "array"),
    ("String_split", "array"),
    ("Dir_list", "array"),
    ("HashMap_keys", "array"),
    ("JsonValue_keys", "array"),
    ("ZipFile_entries", "array"),
    ("Regex_split", "array"),
    ("ArrayList_new", "array"),
    ("HashMap_new", "array"),
    ("ArrayList_keys", "array"),
    ("ArrayList_values", "array"),
    ("ArrayList_entries", "array"),
    ("HashMap_entries", "array"),
    ("HashMap_iterator", "array"),
    ("ArrayList_add", "array"),
    ("ArrayList_set", "array"),
    ("ArrayList_remove", "array"),
    ("ArrayList_removeAt", "array"),
    ("ArrayList_clear", "array"),
    ("ArrayList_pop", "array"),
    ("ArrayList_addAll", "array"),
    ("ArrayList_removeAll", "array"),
    ("ArrayList_retainAll", "array"),
    ("HashMap_put", "array"),
    ("HashMap_remove", "array"),
    ("HashMap_clear", "array"),
    ("HashMap_putIfAbsent", "array"),
    ("HashMap_replace", "array"),
    ("HashMap_merge", "array"),
];

/// Infer the return type of a native function based on its name.
///
/// Named natives are looked up in the single authoritative table above;
/// prefix rules cover the open-ended math and JsonValue-constructor families.
pub fn infer_native_return_type(native_name: &str) -> Type {
    if let Some((_, ty)) = NAMED_NATIVE_RETURNS
        .iter()
        .find(|(name, _)| *name == native_name)
    {
        return Type::simple(ty);
    }
    // JsonValue constructors return runtime handles; their Titrate wrappers
    // (JsonValue.ofStr etc.) build real class instances.
    if native_name.starts_with("JsonValue_of") {
        return Type::simple("JsonValue");
    }
    // Unlisted math-family natives return double. The handful of exceptions
    // (Math_absInt and friends) are pinned to long in the table.
    if native_name.starts_with("Math_")
        || native_name.starts_with("MathAdvanced_")
        || native_name.starts_with("MathTrig_")
    {
        return Type::simple("double");
    }
    Type::simple("int")
}

/// Emit a call to a native function using the out-pointer pattern.
///
/// This function:
/// 1. Marshals each argument to a TitrateValue
/// 2. Allocates a stack TitrateValue for the return value
/// 3. Calls titrate_native_call_out(name, name_len, args, arg_count, out)
/// 4. Unmarshals the result from the out pointer
pub fn emit_native_call<'ctx>(
    context: &'ctx Context,
    builder: &Builder<'ctx>,
    module: &Module<'ctx>,
    native_name: &str,
    arg_values: &[BasicValueEnum<'ctx>],
    arg_types: &[Type],
    declared_return: Option<&Type>,
) -> Result<BasicValueEnum<'ctx>, String> {
    let tv_ty = titrate_value_type(context);
    let usize_type = if cfg!(target_pointer_width = "64") {
        context.i64_type()
    } else {
        context.i32_type()
    };

    // Declare titrate_native_call_out if not already declared.
    let out_fn_name = "titrate_native_call_out";
    if module.get_function(out_fn_name).is_none() {
        let i8_ptr = context.ptr_type(AddressSpace::default());
        let void_type = context.void_type();
        let fn_type = void_type.fn_type(&[
            i8_ptr.into(),       // name_ptr
            usize_type.into(),   // name_len
            i8_ptr.into(),       // args
            usize_type.into(),   // arg_count
            i8_ptr.into(),       // out
        ], false);
        module.add_function(out_fn_name, fn_type, Some(Linkage::External));
    }
    let out_fn = module.get_function(out_fn_name)
        .ok_or_else(|| format!("failed to declare '{}'", out_fn_name))?;

    // Marshal each argument to a TitrateValue and store into an array.
    let arg_count = arg_values.len();
    let array_ty = tv_ty.array_type(arg_count.max(1) as u32);
    let array_alloca = entry_alloca(builder, array_ty.into(), "native.args")?;

    for (i, (val, ty)) in arg_values.iter().zip(arg_types.iter()).enumerate() {
        let tv = marshal_to_titrate(context, builder, *val, ty)?;
        // Index with [0, i]: a single index on an array type would stride by
        // the whole array size (e.g. i=1 -> offset 48 for a 24-byte element),
        // corrupting every arg beyond the first. Two indices select element i.
        let elem_ptr = unsafe {
            builder.build_in_bounds_gep(
                array_ty,
                array_alloca,
                &[
                    context.i32_type().const_int(0, false),
                    context.i32_type().const_int(i as u64, false),
                ],
                &format!("native.arg.{}", i),
            )
        }
        .map_err(|e| format!("build_in_bounds_gep native arg {} failed: {:?}", i, e))?;
        builder.build_store(elem_ptr, tv)
            .map_err(|e| format!("build_store native arg {} failed: {:?}", i, e))?;
    }

    // Allocate a TitrateValue on the stack for the return value.
    let out_alloca = entry_alloca(builder, tv_ty.into(), "native.out")?;

    // Create the native name string as a global constant.
    let name_global = builder.build_global_string_ptr(native_name, "native.name")
        .map_err(|e| format!("build_global_string_ptr failed: {:?}", e))?;
    let name_ptr = name_global.as_pointer_value();
    let name_len_val = usize_type.const_int(native_name.len() as u64, false);

    // Call titrate_native_call_out(name_ptr, name_len, args, arg_count, out).
    let count_val = usize_type.const_int(arg_count as u64, false);
    builder.build_call(
        out_fn,
        &[
            name_ptr.into(),
            name_len_val.into(),
            array_alloca.into(),
            count_val.into(),
            out_alloca.into(),
        ],
        "native.call",
    )
    .map_err(|e| format!("build_call titrate_native_call_out failed: {:?}", e))?;

    // Unmarshal the result from the out pointer. The name table is only a
    // fallback: the call site's declared return type wins because the table
    // cannot know a generic element's concrete type.
    let name_ty = infer_native_return_type(native_name);
    let effective_void = match declared_return {
        Some(t) if !matches!(t.name(), "unknown" | "") => t.name() == "void",
        _ => name_ty.name() == "void",
    };
    if effective_void {
        return Ok(context.i32_type().const_int(0, false).into());
    }

    // Load the TitrateValue from the out alloca.
    let tv_result = builder.build_load(tv_ty, out_alloca, "native.result")
        .map_err(|e| format!("build_load native result failed: {:?}", e))?
        .into_struct_value();

    unmarshal_from_titrate_declared(context, builder, tv_result, &name_ty, declared_return)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn native_return_type_table_has_no_duplicate_names() {
        let mut seen = HashSet::new();
        for (name, _) in NAMED_NATIVE_RETURNS {
            assert!(
                seen.insert(*name),
                "duplicate native name in return-type table: {}",
                name
            );
        }
    }

    #[test]
    fn formerly_ambiguous_natives_resolve_to_their_first_list_type() {
        // These names used to appear in both the bool and int lists; first
        // match won. The authoritative table must preserve that outcome.
        for name in &[
            "Regex_fullMatch",
            "Path_exists",
            "Path_isFile",
            "Path_isDir",
            "Path_isSymlink",
            "AtomicBool_get",
            "String_startsWith",
            "String_endsWith",
            "Hmac_compareDigest",
            "Os_access",
        ] {
            assert_eq!(
                infer_native_return_type(name).name(),
                "bool",
                "{} must remain bool",
                name
            );
        }
        // Double-returning parses also appeared in the int list.
        for name in &["Double_parseDouble", "Double_parse"] {
            assert_eq!(
                infer_native_return_type(name).name(),
                "double",
                "{} must remain double",
                name
            );
        }
        // Process id appeared in both long and int lists.
        assert_eq!(infer_native_return_type("Process_id").name(), "long");
        assert_eq!(infer_native_return_type("Hash_crc32").name(), "string");
    }
}
