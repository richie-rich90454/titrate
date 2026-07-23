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
use inkwell::values::BasicValueEnum;
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
pub const TV_DOUBLE: i32 = 10;
pub const TV_FLOAT: i32 = 9;
pub const TV_CHAR: i32 = 11;
pub const TV_STRING: i32 = 12;
pub const TV_ARRAY: i32 = 13;

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
    let (tag, _val_to_store): (i32, BasicTypeEnum<'ctx>) = match type_name {
        "bool" => (TV_BOOL, i32_ty.into()),
        "byte" => (TV_BYTE, context.i8_type().into()),
        "short" => (TV_SHORT, context.i16_type().into()),
        "int" | "u8" | "u16" | "u32" => (TV_INT, context.i32_type().into()),
        "long" | "u64" | "size" => (TV_LONG, context.i64_type().into()),
        "float" | "half" => (TV_FLOAT, context.f32_type().into()),
        "double" | "quad" => (TV_DOUBLE, context.f64_type().into()),
        "char" => (TV_CHAR, context.i32_type().into()),
        "string" => (
            TV_STRING,
            context.struct_type(
                &[context.i64_type().into(), context.ptr_type(AddressSpace::default()).into()],
                false,
            ).into(),
        ),
        "array" | "ArrayList" => (
            TV_ARRAY,
            context.struct_type(
                &[context.i64_type().into(), context.ptr_type(AddressSpace::default()).into()],
                false,
            ).into(),
        ),
        _ => {
            // Default: treat as int.
            (TV_INT, context.i32_type().into())
        }
    };

    let tag_val = i32_ty.const_int(tag as u64, false);
    let pad_val = i32_ty.const_int(0, false);

    // Build the payload by storing the value into a [16 x i8] alloca.
    let payload_alloca = builder.build_alloca(payload_ty, "tv.payload")
        .map_err(|e| format!("build_alloca payload failed: {:?}", e))?;

    // Bitcast the payload alloca to the value's type and store.
    let val_ptr = builder.build_bit_cast(payload_alloca, context.ptr_type(AddressSpace::default()), "tv.valptr")
        .map_err(|e| format!("build_bit_cast valptr failed: {:?}", e))?
        .into_pointer_value();
    builder.build_store(val_ptr, value)
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
/// via type-punning through an alloca.
pub fn unmarshal_from_titrate<'ctx>(
    context: &'ctx Context,
    builder: &Builder<'ctx>,
    tv: inkwell::values::StructValue<'ctx>,
    expected_ty: &Type,
) -> Result<BasicValueEnum<'ctx>, String> {
    let payload_ty = context.i8_type().array_type(16);

    // Extract the payload from the TitrateValue struct (field 2 in {i32, i32, [16xi8]}).
    let payload = builder.build_extract_value(tv, 2, "tv.result.payload")
        .map_err(|e| format!("build_extract_value payload failed: {:?}", e))?;

    // Store the payload bytes into an alloca.
    let payload_alloca = builder.build_alloca(payload_ty, "tv.result.alloca")
        .map_err(|e| format!("build_alloca result failed: {:?}", e))?;
    builder.build_store(payload_alloca, payload)
        .map_err(|e| format!("build_store result payload failed: {:?}", e))?;

    // Determine the expected LLVM type and load from the alloca.
    let type_name = expected_ty.name();
    let target_ty: BasicTypeEnum<'ctx> = match type_name {
        "bool" => context.i8_type().into(),
        "byte" => context.i8_type().into(),
        "short" => context.i16_type().into(),
        "int" | "u8" | "u16" | "u32" => context.i32_type().into(),
        "long" | "u64" | "size" => context.i64_type().into(),
        "float" | "half" => context.f32_type().into(),
        "double" | "quad" => context.f64_type().into(),
        "char" => context.i32_type().into(),
        "string" => context.struct_type(
            &[context.i64_type().into(), context.ptr_type(AddressSpace::default()).into()],
            false,
        ).into(),
        "array" => context.struct_type(
            &[context.i64_type().into(), context.ptr_type(AddressSpace::default()).into()],
            false,
        ).into(),
        "void" => {
            // Void return: return a dummy i32.
            return Ok(context.i32_type().const_int(0, false).into());
        }
        _ => context.i32_type().into(),
    };

    let val_ptr = builder.build_bit_cast(
        payload_alloca,
        context.ptr_type(AddressSpace::default()),
        "tv.result.valptr",
    )
    .map_err(|e| format!("build_bit_cast result valptr failed: {:?}", e))?
    .into_pointer_value();

    builder.build_load(target_ty, val_ptr, "tv.result.val")
        .map_err(|e| format!("build_load result val failed: {:?}", e))
}

/// Infer the return type of a native function based on its name.
///
/// This is a simple heuristic: functions starting with `String_` that don't
/// contain `length`/`indexOf`/`startsWith`/etc. return string; math functions
/// return double; etc.
pub fn infer_native_return_type(native_name: &str) -> Type {
    let name = native_name;
    // Void-returning natives.
    if matches!(name, "println" | "Sys_exit" | "Sys_sleep" | "Sys_setEnv"
        | "Thread_sleep" | "Thread_yield" | "Thread_detach"
        | "Mutex_lock" | "Mutex_unlock"
        | "RecursiveMutex_lock" | "RecursiveMutex_unlock"
        | "SharedMutex_sharedLock" | "SharedMutex_sharedUnlock"
        | "SharedMutex_uniqueLock" | "SharedMutex_uniqueUnlock"
        | "CondVar_notifyOne" | "CondVar_notifyAll"
        | "Semaphore_release" | "Semaphore_acquire"
        | "File_close" | "File_flush" | "File_write" | "File_writeBytes"
        | "File_truncate" | "File_setModified"
        | "Socket_close" | "UdpSocket_close" | "Ssl_close" | "Sqlite_close"
        | "Mmap_close" | "Mmap_flush" | "ZipFile_close" | "ZipWriter_close"
        | "Hasher_reset" | "Hasher_close" | "Gc_collect"
        | "Random_seed" | "Os_chmod" | "Os_makedirs" | "Os_symlink"
        | "Os_kill" | "Os_umask" | "Os_removedirs" | "Os_renames"
        | "Os_replace" | "Os_link" | "Os_utime" | "Os_unsetenv"
        | "Signal_raise" | "File_delete" | "Dir_remove" | "Dir_removeTree"
        | "Env_set" | "Os_setenv" | "Os_chdir" | "OnceFlag_callOnce"
        | "AtomicInt_set" | "AtomicBool_set" | "AtomicLong_set" | "AtomicRef_set"
    ) {
        return Type::simple("void");
    }

    // Double-returning math functions.
    if name.starts_with("Math_") && !name.contains("Int") && !name.contains("maxInt") && !name.contains("minInt") {
        if matches!(name, "Math_absInt" | "Math_maxInt" | "Math_minInt") {
            return Type::simple("int");
        }
        return Type::simple("double");
    }

    // String-returning functions.
    if matches!(name,
        "toString" | "String_trim" | "String_trimStart" | "String_trimEnd"
        | "String_toUpperCase" | "String_toLowerCase" | "String_replace"
        | "String_fromCharCode" | "String_charAt" | "String_substring"
        | "String_padLeft" | "String_padRight"
        | "Path_join" | "Path_basename" | "Path_dirname" | "Path_extension"
        | "Json_stringify" | "Os_name" | "Os_arch" | "Os_family"
        | "Os_userName" | "Os_hostName" | "Os_release" | "Os_version"
        | "Os_strerror" | "Os_uname" | "Titrate_version"
        | "Hash_md5" | "Hash_sha1" | "Hash_sha256" | "Hash_sha384" | "Hash_sha512"
        | "Hash_sha224" | "Hash_sha3_256" | "Hash_sha3_384" | "Hash_sha3_512"
        | "Hash_sha3_224" | "Hash_blake2b" | "Hash_blake2s" | "Hash_shake128"
        | "Hash_shake256" | "Hasher_hexDigest" | "Hash_crc32"
        | "Base64_encode" | "Base64_decode" | "Hex_encode" | "Hex_decode"
        | "Url_encode" | "Url_decode" | "TypeName_of"
        | "Regex_replace" | "Regex_subN"
        | "Time_format" | "Sqlite_columnName" | "Sqlite_getString"
        | "Socket_getLocalAddress" | "Socket_getRemoteAddress"
        | "Socket_inetNtop" | "Socket_getAddrInfo"
        | "UdpSocket_lastSenderHost"
        | "ArrayList_get"
        | "File_readFile" | "File_readLine" | "File_readChunk" | "File_readLines"
        | "Boolean_toString" | "Integer_toString" | "Long_toString"
        | "Double_toString" | "Float_toString"
        | "Sys_workingDir" | "Os_getcwd" | "Os_getenv" | "Os_name"
        | "Os_arch" | "Os_family" | "Os_userName" | "Os_hostName"
        | "Os_release" | "Os_version" | "Os_strerror" | "Os_uname"
        | "Os_environ"
    ) {
        return Type::simple("string");
    }

    // Long-returning functions.
    if matches!(name,
        "Time_now" | "Time_millis" | "Time_monotonic" | "Time_perfCounter"
        | "Time_epochSeconds" | "Time_nanos" | "Time_sleep"
        | "Thread_getId" | "Thread_currentId" | "Thread_sleep"
        | "Process_id" | "Os_getpid" | "Os_getppid"
        | "Random_nextLong" | "File_size" | "File_tell" | "File_lastModified"
        | "Fs_size" | "Fs_totalSpace" | "Fs_freeSpace"
        | "Sqlite_lastInsertId" | "Sqlite_getInt"
        | "Socket_getLocalPort" | "Socket_getRemotePort"
        | "UdpSocket_lastSenderPort"
    ) {
        return Type::simple("long");
    }

    // Int-returning functions.
    if matches!(name,
        "parseInt" | "Integer_parseOr" | "String_length" | "String_indexOf"
        | "String_startsWith" | "String_endsWith"
        | "Regex_match" | "Regex_find" | "Regex_fullMatch" | "Regex_groupCount"
        | "Regex_matchWithFlags" | "Regex_findWithFlags"
        | "Math_absInt" | "Math_maxInt" | "Math_minInt" | "Math_round"
        | "Math_getExponent" | "Math_floor" | "Math_ceil"
        | "Path_exists" | "Path_isFile" | "Path_isDir" | "Path_isSymlink"
        | "Fs_exists" | "Fs_isFile" | "Fs_isDir"
        | "Time_getYear" | "Time_getMonth" | "Time_getDay"
        | "Time_getHour" | "Time_getMinute" | "Time_getSecond"
        | "Time_dayOfWeek" | "Time_dayOfYear"
        | "Os_cpuCount" | "Os_access"
        | "Process_id" | "Os_getpid" | "Os_getppid"
        | "ZipFile_entryCount" | "Sqlite_columnCount" | "Sqlite_nextRow"
        | "Socket_createConnection" | "Socket_createServer"
        | "Mutex_tryLock" | "RecursiveMutex_tryLock"
        | "SharedMutex_trySharedLock" | "SharedMutex_tryUniqueLock"
        | "Semaphore_tryAcquire" | "Semaphore_availablePermits"
        | "AtomicInt_get" | "AtomicInt_fetchAdd" | "AtomicInt_fetchSub"
        | "AtomicInt_compareAndSwap" | "AtomicInt_fetchOr" | "AtomicInt_fetchAnd"
        | "AtomicInt_fetchXor" | "AtomicInt_exchange"
        | "AtomicBool_get" | "AtomicBool_compareAndSwap"
        | "AtomicLong_get" | "AtomicLong_fetchAdd" | "AtomicLong_fetchSub"
        | "AtomicLong_compareAndSwap"
        | "AtomicRef_compareAndSwap"
        | "Hmac_compareDigest" | "Double_parseDouble" | "Double_parse"
        | "Long_parseLong" | "Hash_crc32"
        | "Socket_inetPton" | "Subprocess_popenWrite"
        | "ArrayList_size"
    ) {
        return Type::simple("int");
    }

    // Bool-returning functions.
    if matches!(name,
        "Regex_match" | "Regex_fullMatch" | "Regex_matchWithFlags"
        | "Path_exists" | "Path_isFile" | "Path_isDir" | "Path_isSymlink"
        | "Fs_exists" | "Fs_isFile" | "Fs_isDir"
        | "Mutex_tryLock" | "RecursiveMutex_tryLock"
        | "SharedMutex_trySharedLock" | "SharedMutex_tryUniqueLock"
        | "Semaphore_tryAcquire" | "AtomicBool_get"
        | "AtomicInt_compareAndSwap" | "AtomicBool_compareAndSwap"
        | "AtomicLong_compareAndSwap" | "AtomicRef_compareAndSwap"
        | "Hmac_compareDigest" | "Os_access"
    ) {
        return Type::simple("bool");
    }

    // Array-returning functions.
    if matches!(name,
        "Sys_args" | "String_split" | "Dir_list"
    ) {
        return Type::simple("array");
    }

    // Default: return double for unknown math functions, int otherwise.
    if name.starts_with("Math_") {
        Type::simple("double")
    } else {
        Type::simple("int")
    }
}

/// Emit a call to a native function.
///
/// This function:
/// 1. Declares the native wrapper function in the module (if not already declared)
/// 2. Marshals each argument to a TitrateValue
/// 3. Allocates an array of TitrateValue on the stack
/// 4. Calls the native wrapper
/// 5. Unmarshals the result
pub fn emit_native_call<'ctx>(
    context: &'ctx Context,
    builder: &Builder<'ctx>,
    module: &Module<'ctx>,
    native_name: &str,
    arg_values: &[BasicValueEnum<'ctx>],
    arg_types: &[Type],
) -> Result<BasicValueEnum<'ctx>, String> {
    let tv_ty = titrate_value_type(context);
    let usize_type = if cfg!(target_pointer_width = "64") {
        context.i64_type()
    } else {
        context.i32_type()
    };

    // Declare the native function if not already declared.
    let c_name = native_name_to_c_name(native_name);
    let native_fn = match module.get_function(&c_name) {
        Some(f) => f,
        None => {
            declare_native(context, module, native_name);
            module.get_function(&c_name)
                .ok_or_else(|| format!("failed to declare native function '{}'", c_name))?
        }
    };

    // Marshal each argument to a TitrateValue and store into an array.
    let arg_count = arg_values.len();
    let array_ty = tv_ty.array_type(arg_count.max(1) as u32);
    let array_alloca = builder.build_alloca(array_ty, "native.args")
        .map_err(|e| format!("build_alloca native.args failed: {:?}", e))?;

    for (i, (val, ty)) in arg_values.iter().zip(arg_types.iter()).enumerate() {
        let tv = marshal_to_titrate(context, builder, *val, ty)?;
        let elem_ptr = unsafe {
            builder.build_in_bounds_gep(
                array_ty,
                array_alloca,
                &[context.i32_type().const_int(i as u64, false)],
                &format!("native.arg.{}", i),
            )
        }
        .map_err(|e| format!("build_in_bounds_gep native arg {} failed: {:?}", i, e))?;
        builder.build_store(elem_ptr, tv)
            .map_err(|e| format!("build_store native arg {} failed: {:?}", i, e))?;
    }

    // Call the native function.
    let count_val = usize_type.const_int(arg_count as u64, false);
    let call = builder.build_call(
        native_fn,
        &[array_alloca.into(), count_val.into()],
        "native.call",
    )
    .map_err(|e| format!("build_call native '{}' failed: {:?}", native_name, e))?;

    // Get the returned TitrateValue struct.
    let tv_result = match call.try_as_basic_value() {
        inkwell::values::ValueKind::Basic(v) => v.into_struct_value(),
        _ => return Err(format!("native '{}' did not return a value", native_name)),
    };

    // Unmarshal the result based on the inferred return type.
    let return_ty = infer_native_return_type(native_name);
    if return_ty.name() == "void" {
        return Ok(context.i32_type().const_int(0, false).into());
    }
    unmarshal_from_titrate(context, builder, tv_result, &return_ty)
}
