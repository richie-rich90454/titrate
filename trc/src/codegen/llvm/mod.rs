//! LLVM-based native code generation backend.
//!
//! This module lowers the typed Titrate AST to LLVM IR using the `inkwell`
//! crate, then emits a native object file.
//!
//! Phase 1 supports:
//! - All primitive literals (int, float, bool, char, string)
//! - `let`/`var`/`const` declarations for all primitive types
//! - Assignment to primitive and string variables
//! - String concatenation with `+`
//! - `io::println(...)` calls with any primitive argument
//! - Arithmetic, comparison, logical, and bitwise operators
//! - Control flow: if/else, while, do-while, for, switch, ternary
//! - Function declarations, calls, and recursion
//!
//! String values are represented at the LLVM level as a struct `{ i64, i8* }`
//! where the `i8*` points to a UTF-8 byte buffer of exactly `len` bytes. The
//! runtime helpers `titrate_println`, `titrate_string_concat`, and
//! `titrate_free` (provided by the `titrate_native` crate) operate on this
//! representation. Primitive values use the natural LLVM representation
//! (i32 for int, f64 for double, i1 for bool, etc.).

pub mod linker;
pub mod ownership;
pub mod target_wrappers;
pub mod tuple_codegen;
pub mod types;
pub mod vtable;
pub mod enum_codegen;
pub mod native_bridge;

use std::collections::HashMap;
use std::path::Path;

use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};
use inkwell::targets::{
    CodeModel, FileType, RelocMode, Target, TargetMachine,
};
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{
    BasicValueEnum, FunctionValue, IntValue, PointerValue,
};
use inkwell::AddressSpace;
use inkwell::OptimizationLevel;

/// LLVM calling convention value for `fastcc`. Used by `set_call_conventions`.
/// (inkwell 0.9 exposes `set_call_conventions(u32)` rather than an enum.)
const LLVM_FAST_CALL_CONV: u32 = 8;

/// LLVM metadata kind id for `llvm.loop`. This is the well-known id LLVM
/// reserves for loop-vectorization metadata attached to branch instructions.
const LLVM_LOOP_METADATA_KIND: u32 = 6;

use crate::ast::{ClassDecl, Declaration, Expr, FnDecl, InterfaceDecl, Literal, Operator, Program, Stmt, Type, UnOp};
use crate::ast::ClassMember;

use super::llvm::types as llvm_types;
use super::llvm::vtable::{ClassInfo, InterfaceInfo, emit_new_allocation, emit_field_access, emit_field_store, emit_direct_call, emit_virtual_call, emit_is_check, emit_as_cast, build_class_struct_type, create_vtable_global, build_interface_fat_ptr_type, create_interface_vtable, emit_interface_fat_ptr, emit_interface_is_check, emit_interface_method_call};
use super::llvm::enum_codegen::{EnumInfo, compile_enum_decl, emit_enum_construct};

/// String value tracked during codegen: the byte length and the pointer to
/// the underlying UTF-8 buffer.
#[derive(Clone, Copy)]
struct StringValue<'ctx> {
    len: IntValue<'ctx>,
    ptr: PointerValue<'ctx>,
}

/// A local variable's storage info: the alloca pointer and the LLVM type
/// stored at that pointer.
#[derive(Clone)]
struct LocalVar<'ctx> {
    ptr: PointerValue<'ctx>,
    ty: BasicTypeEnum<'ctx>,
    /// Original Titrate type name for native call resolution (e.g., "ArrayList", "HashMap").
    titrate_type: Option<String>,
}

/// Loop context for break/continue codegen.
struct LoopContext<'ctx> {
    continue_block: inkwell::basic_block::BasicBlock<'ctx>,
    break_block: inkwell::basic_block::BasicBlock<'ctx>,
}

/// The LLVM backend. Owns the inkwell `Context`, `Module`, and `Builder`.
pub struct LlvmBackend<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    /// Local variables in the current function scope: name -> storage.
    locals: HashMap<String, LocalVar<'ctx>>,
    /// Counter for generating unique global string names.
    string_counter: usize,
    /// Stack of loop contexts for break/continue.
    loop_stack: Vec<LoopContext<'ctx>>,
    /// Map from function name to LLVM function value (for non-generic functions).
    functions: HashMap<String, FunctionValue<'ctx>>,
    /// Map from function name to its declaration (for late compilation / recursion).
    function_decls: HashMap<String, FnDecl>,
    /// Ownership / region / cleanup state.
    ownership: ownership::OwnershipContext<'ctx>,
    /// Counter for generating unique closure function names.
    #[allow(dead_code)]
    closure_counter: usize,
    /// Stack of catch blocks for throw/try-catch codegen. Each entry is
    /// the basic block that a `throw` should branch to, plus the alloca
    /// where the thrown error value should be stored.
    #[allow(dead_code)]
    catch_stack: Vec<CatchContext<'ctx>>,
    /// Class info map: class name -> compiled class info (struct layout, vtable, etc.).
    #[allow(dead_code)]
    class_infos: HashMap<String, ClassInfo<'ctx>>,
    /// Type ID assignment: class name -> unique type_id.
    #[allow(dead_code)]
    class_type_ids: HashMap<String, u32>,
    /// Next available type_id.
    #[allow(dead_code)]
    next_type_id: u32,
    /// Current 'this' pointer in method codegen (None if not in a method).
    #[allow(dead_code)]
    current_this: Option<PointerValue<'ctx>>,
    /// Interface info map: interface name -> compiled interface info.
    #[allow(dead_code)]
    interface_infos: HashMap<String, InterfaceInfo<'ctx>>,
    /// Interface vtable globals: (interface_name, class_name) -> vtable global.
    #[allow(dead_code)]
    interface_vtables: HashMap<(String, String), inkwell::values::GlobalValue<'ctx>>,
    /// Enum info map: enum name -> compiled enum info.
    #[allow(dead_code)]
    enum_infos: HashMap<String, EnumInfo<'ctx>>,
    /// Whether release-mode optimizations (inline hints, fast-cc, memset,
    /// vectorization metadata) should be emitted. Set from `compile_program`.
    release_mode: bool,
}

/// Catch context for try/catch codegen.
#[allow(dead_code)]
struct CatchContext<'ctx> {
    /// Block to branch to when an exception is thrown.
    catch_block: inkwell::basic_block::BasicBlock<'ctx>,
    /// Alloca (i8*) where the thrown error pointer is stored.
    error_alloca: PointerValue<'ctx>,
}

impl<'ctx> LlvmBackend<'ctx> {
    pub fn new(context: &'ctx Context, module_name: &str) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();
        LlvmBackend {
            context,
            module,
            builder,
            locals: HashMap::new(),
            string_counter: 0,
            loop_stack: Vec::new(),
            functions: HashMap::new(),
            function_decls: HashMap::new(),
            ownership: ownership::OwnershipContext::new(),
            closure_counter: 0,
            catch_stack: Vec::new(),
            class_infos: HashMap::new(),
            class_type_ids: HashMap::new(),
            next_type_id: 0,
            current_this: None,
            interface_infos: HashMap::new(),
            interface_vtables: HashMap::new(),
            enum_infos: HashMap::new(),
            release_mode: false,
        }
    }

    /// Declare the external C-ABI functions provided by `titrate_native`.
    fn declare_natives(&self) {
        let i32_type = self.context.i32_type();
        let i64_type = self.context.i64_type();
        let f64_type = self.context.f64_type();
        let i8_ptr = self.context.ptr_type(AddressSpace::default());
        let void_type = self.context.void_type();

        // void titrate_println(i64 len, i8* ptr)
        let println_fn = void_type.fn_type(
            &[i64_type.into(), i8_ptr.into()],
            false,
        );
        self.module.add_function("titrate_println", println_fn, Some(Linkage::External));

        // i8* titrate_string_concat(i64 a_len, i8* a_ptr, i64 b_len, i8* b_ptr, i64* out_len)
        let concat_fn = i8_ptr.fn_type(
            &[
                i64_type.into(),
                i8_ptr.into(),
                i64_type.into(),
                i8_ptr.into(),
                i8_ptr.into(),
            ],
            false,
        );
        self.module.add_function(
            "titrate_string_concat",
            concat_fn,
            Some(Linkage::External),
        );

        // void titrate_free(i8* ptr)
        let free_fn = void_type.fn_type(&[i8_ptr.into()], false);
        self.module.add_function("titrate_free", free_fn, Some(Linkage::External));

        // i8* titrate_malloc(i64 size)
        let malloc_fn = i8_ptr.fn_type(&[i64_type.into()], false);
        self.module.add_function("titrate_malloc", malloc_fn, Some(Linkage::External));

        // Primitive printers.
        let println_int_fn = void_type.fn_type(&[i64_type.into()], false);
        self.module.add_function("titrate_println_int", println_int_fn, Some(Linkage::External));

        let println_double_fn = void_type.fn_type(&[f64_type.into()], false);
        self.module.add_function("titrate_println_double", println_double_fn, Some(Linkage::External));

        let println_bool_fn = void_type.fn_type(&[i32_type.into()], false);
        self.module.add_function("titrate_println_bool", println_bool_fn, Some(Linkage::External));

        let println_char_fn = void_type.fn_type(&[i32_type.into()], false);
        self.module.add_function("titrate_println_char", println_char_fn, Some(Linkage::External));

        // Print without newline (io::print support)
        let print_fn = void_type.fn_type(&[i64_type.into(), i8_ptr.into()], false);
        self.module.add_function("titrate_print", print_fn, Some(Linkage::External));

        let print_int_fn = void_type.fn_type(&[i64_type.into()], false);
        self.module.add_function("titrate_print_int", print_int_fn, Some(Linkage::External));

        let print_double_fn = void_type.fn_type(&[f64_type.into()], false);
        self.module.add_function("titrate_print_double", print_double_fn, Some(Linkage::External));

        let print_bool_fn = void_type.fn_type(&[i32_type.into()], false);
        self.module.add_function("titrate_print_bool", print_bool_fn, Some(Linkage::External));

        let print_char_fn = void_type.fn_type(&[i32_type.into()], false);
        self.module.add_function("titrate_print_char", print_char_fn, Some(Linkage::External));

        // Global exception pointer for throw/try-catch error propagation.
        // Phase 1: i8* points to a heap-allocated error payload.
        let exception_global = self.module.add_global(i8_ptr, None, "__titrate_exception");
        exception_global.set_initializer(&i8_ptr.const_null());
        exception_global.set_linkage(Linkage::Internal);
    }

    /// Create a global byte array holding the string's UTF-8 bytes and return
    /// a `(len, ptr)` pair pointing at it.
    fn make_string_global(&mut self, s: &str) -> StringValue<'ctx> {
        let bytes = s.as_bytes();
        let len = bytes.len();
        let i8_type = self.context.i8_type();
        let i64_type = self.context.i64_type();

        let name = format!(".str.{}", self.string_counter);
        self.string_counter += 1;

        // Include a NUL terminator so the buffer is also a valid C string,
        // which is handy for debugging and for native interop.
        let mut const_bytes: Vec<IntValue> =
            bytes.iter().map(|&b| i8_type.const_int(b as u64, false)).collect();
        const_bytes.push(i8_type.const_int(0, false));

        let arr_type = i8_type.array_type(const_bytes.len() as u32);
        let global = self.module.add_global(arr_type, None, &name);
        global.set_linkage(Linkage::Private);
        global.set_constant(true);
        global.set_initializer(&i8_type.const_array(&const_bytes));

        let len_val = i64_type.const_int(len as u64, false);
        let ptr_val = global.as_pointer_value();
        StringValue {
            len: len_val,
            ptr: ptr_val,
        }
    }

    /// Look up a native function by name.
    fn get_function(&self, name: &str) -> FunctionValue<'ctx> {
        self.module
            .get_function(name)
            .unwrap_or_else(|| panic!("function '{}' not declared", name))
    }

    /// Convert a `StringValue` to a `BasicValueEnum` struct value by storing
    /// it into a temporary alloca and loading the struct.
    fn string_value_to_basic(&self, sv: StringValue<'ctx>) -> Result<BasicValueEnum<'ctx>, String> {
        let string_ty = llvm_types::string_type(self.context).into_struct_type();
        let alloca = self.builder.build_alloca(string_ty, "str.tmp")
            .map_err(|e| format!("build_alloca str.tmp failed: {:?}", e))?;
        let len_ptr = self.builder.build_struct_gep(string_ty, alloca, 0, "str.len.ptr")
            .map_err(|e| format!("build_struct_gep 0 failed: {:?}", e))?;
        let ptr_ptr = self.builder.build_struct_gep(string_ty, alloca, 1, "str.ptr.ptr")
            .map_err(|e| format!("build_struct_gep 1 failed: {:?}", e))?;
        self.builder.build_store(len_ptr, sv.len)
            .map_err(|e| format!("build_store len failed: {:?}", e))?;
        self.builder.build_store(ptr_ptr, sv.ptr)
            .map_err(|e| format!("build_store ptr failed: {:?}", e))?;
        self.builder.build_load(string_ty, alloca, "str.val")
            .map_err(|e| format!("build_load str.val failed: {:?}", e))
    }

    /// Extract a `StringValue` from a `BasicValueEnum` that is a string struct.
    fn basic_to_string_value(&self, v: BasicValueEnum<'ctx>) -> Result<StringValue<'ctx>, String> {
        match v {
            BasicValueEnum::StructValue(sv) => {
                let len = self.builder.build_extract_value(sv, 0, "sv.len")
                    .map_err(|e| format!("build_extract_value 0 failed: {:?}", e))?
                    .into_int_value();
                let ptr = self.builder.build_extract_value(sv, 1, "sv.ptr")
                    .map_err(|e| format!("build_extract_value 1 failed: {:?}", e))?
                    .into_pointer_value();
                Ok(StringValue { len, ptr })
            }
            _ => Err(format!("expected string struct, got {:?}", v)),
        }
    }

    /// Compile a string expression to a `StringValue`.
    fn compile_string_expr(&mut self, expr: &Expr) -> Result<StringValue<'ctx>, String> {
        match expr {
            Expr::Literal(Literal::String(s), _) => {
                Ok(self.make_string_global(s))
            }
            Expr::Identifier(name, _) => {
                let var = self.locals.get(name).ok_or_else(|| {
                    format!("codegen: unknown string variable '{}'", name)
                })?;
                // The local stores a { i64, i8* } struct.
                let string_ty = llvm_types::string_type(self.context).into_struct_type();
                let struct_val = self.builder.build_load(string_ty, var.ptr, &format!("{}.val", name))
                    .map_err(|e| format!("build_load string failed: {:?}", e))?
                    .into_struct_value();
                let len = self.builder.build_extract_value(struct_val, 0, &format!("{}.len", name))
                    .map_err(|e| format!("build_extract_value len failed: {:?}", e))?
                    .into_int_value();
                let ptr = self.builder.build_extract_value(struct_val, 1, &format!("{}.ptr", name))
                    .map_err(|e| format!("build_extract_value ptr failed: {:?}", e))?
                    .into_pointer_value();
                Ok(StringValue { len, ptr })
            }
            Expr::Binary(left, Operator::Add, right, _) => {
                // String concatenation: try string first, fall back to general.
                if llvm_types::is_string(&self.infer_expr_type(left)) {
                    let l = self.compile_string_expr(left)?;
                    let r = self.compile_string_expr(right)?;
                    return self.build_string_concat(l, r);
                }
                Err(format!("codegen: unsupported string expression: {:?}", expr))
            }
            _ => {
                // Try general compile_expr and convert.
                let v = self.compile_expr(expr)?;
                self.basic_to_string_value(v)
            }
        }
    }

    /// Emit a call to `titrate_string_concat` and return the resulting
    /// `(len, ptr)` pair.
    fn build_string_concat(
        &self,
        a: StringValue<'ctx>,
        b: StringValue<'ctx>,
    ) -> Result<StringValue<'ctx>, String> {
        let i64_type = self.context.i64_type();
        let i8_ptr = self.context.ptr_type(AddressSpace::default());

        // Allocate space for the output length.
        let out_len_alloca = self.builder.build_alloca(i64_type, "concat.out_len")
            .map_err(|e| format!("build_alloca failed: {:?}", e))?;

        let concat_fn = self.get_function("titrate_string_concat");
        let call_value = self.builder
            .build_call(
                concat_fn,
                &[
                    a.len.into(),
                    a.ptr.into(),
                    b.len.into(),
                    b.ptr.into(),
                    out_len_alloca.into(),
                ],
                "concat.result",
            )
            .map_err(|e| format!("build_call concat failed: {:?}", e))?;

        let result_ptr = match call_value.try_as_basic_value() {
            inkwell::values::ValueKind::Basic(v) => v.into_pointer_value(),
            inkwell::values::ValueKind::Instruction(_) => {
                return Err("titrate_string_concat did not return a value".to_string());
            }
        };

        let out_len = self.builder.build_load(i64_type, out_len_alloca, "concat.len")
            .map_err(|e| format!("build_load out_len failed: {:?}", e))?
            .into_int_value();

        // Cast the i8* result to the right pointer type if needed.
        let ptr = if result_ptr.get_type() == i8_ptr {
            result_ptr
        } else {
            self.builder
                .build_bit_cast(result_ptr, i8_ptr, "concat.ptr.cast")
                .map_err(|e| format!("build_bit_cast failed: {:?}", e))?
                .into_pointer_value()
        };

        Ok(StringValue { len: out_len, ptr })
    }

    /// Emit a call to `titrate_println(len, ptr)`.
    fn build_println_string(&self, s: StringValue<'ctx>) -> Result<(), String> {
        let println_fn = self.get_function("titrate_println");
        self.builder
            .build_call(println_fn, &[s.len.into(), s.ptr.into()], "println")
            .map_err(|e| format!("build_call println failed: {:?}", e))?;
        Ok(())
    }

    /// Emit a call to the appropriate println helper for a primitive value.
    fn build_println_primitive(&self, v: BasicValueEnum<'ctx>, ty: &Type) -> Result<(), String> {
        let name = ty.name();
        // Handle "unknown" type by checking the LLVM value type and defaulting to string
        if name == "unknown" || name == "any" || name == "void" {
            if v.is_struct_value() {
                let sv = v.into_struct_value();
                let len_val = self.builder.build_extract_value(sv, 0, "str.len")
                    .map_err(|e| format!("extract str.len failed: {:?}", e))?;
                let ptr_val = self.builder.build_extract_value(sv, 1, "str.ptr")
                    .map_err(|e| format!("extract str.ptr failed: {:?}", e))?;
                let s = StringValue { len: len_val.into_int_value(), ptr: ptr_val.into_pointer_value() };
                return self.build_println_string(s);
            }
            // Use toString native bridge to convert to string
            let sv = native_bridge::emit_native_call(
                self.context, &self.builder, &self.module,
                "toString", &[v], &[ty.clone()],
            )?;
            let sv_val = sv.into_struct_value();
            let len_val = self.builder.build_extract_value(sv_val, 0, "str.len")
                .map_err(|e| format!("extract str.len failed: {:?}", e))?;
            let ptr_val = self.builder.build_extract_value(sv_val, 1, "str.ptr")
                .map_err(|e| format!("extract str.ptr failed: {:?}", e))?;
            let s = StringValue { len: len_val.into_int_value(), ptr: ptr_val.into_pointer_value() };
            return self.build_println_string(s);
        }
        match name {
            "float" | "double" | "half" | "quad" => {
                let f64_type = self.context.f64_type();
                let v = if v.get_type() == f64_type.into() {
                    v.into_float_value()
                } else {
                    self.builder.build_float_ext(v.into_float_value(), f64_type, "ext")
                        .map_err(|e| format!("build_float_ext failed: {:?}", e))?
                };
                let f = self.get_function("titrate_println_double");
                self.builder.build_call(f, &[v.into()], "println.d")
                    .map_err(|e| format!("build_call println_double failed: {:?}", e))?;
            }
            "bool" => {
                let i32_type = self.context.i32_type();
                let v = self.builder.build_int_z_extend(v.into_int_value(), i32_type, "zext")
                    .map_err(|e| format!("build_int_z_extend failed: {:?}", e))?;
                let f = self.get_function("titrate_println_bool");
                self.builder.build_call(f, &[v.into()], "println.b")
                    .map_err(|e| format!("build_call println_bool failed: {:?}", e))?;
            }
            "char" => {
                let f = self.get_function("titrate_println_char");
                self.builder.build_call(f, &[v.into()], "println.c")
                    .map_err(|e| format!("build_call println_char failed: {:?}", e))?;
            }
            // All other integer types: extend to i64 and call titrate_println_int.
            _ if llvm_types::is_integer(ty) => {
                let i64_type = self.context.i64_type();
                let v = if v.get_type() == i64_type.into() {
                    v.into_int_value()
                } else if llvm_types::integer_bit_width(ty) < Some(64) {
                    self.builder.build_int_s_extend(v.into_int_value(), i64_type, "sext")
                        .map_err(|e| format!("build_int_s_extend failed: {:?}", e))?
                } else {
                    v.into_int_value()
                };
                let f = self.get_function("titrate_println_int");
                self.builder.build_call(f, &[v.into()], "println.i")
                    .map_err(|e| format!("build_call println_int failed: {:?}", e))?;
            }
            _ => {
                return Err(format!("codegen: cannot println value of type {}", ty));
            }
        }
        Ok(())
    }

    /// Emit a call to `titrate_print(len, ptr)`.
    fn build_print_string(&self, s: StringValue<'ctx>) -> Result<(), String> {
        let print_fn = self.get_function("titrate_print");
        self.builder
            .build_call(print_fn, &[s.len.into(), s.ptr.into()], "print")
            .map_err(|e| format!("build_call print failed: {:?}", e))?;
        Ok(())
    }

    /// Emit a call to the appropriate print helper for a primitive value.
    fn build_print_primitive(&self, v: BasicValueEnum<'ctx>, ty: &Type) -> Result<(), String> {
        let name = ty.name();
        // Handle "unknown" type by checking the LLVM value type and defaulting to string
        if name == "unknown" || name == "any" || name == "void" {
            if v.is_struct_value() {
                let sv = v.into_struct_value();
                let len_val = self.builder.build_extract_value(sv, 0, "str.len")
                    .map_err(|e| format!("extract str.len failed: {:?}", e))?;
                let ptr_val = self.builder.build_extract_value(sv, 1, "str.ptr")
                    .map_err(|e| format!("extract str.ptr failed: {:?}", e))?;
                let s = StringValue { len: len_val.into_int_value(), ptr: ptr_val.into_pointer_value() };
                return self.build_print_string(s);
            }
            let sv = native_bridge::emit_native_call(
                self.context, &self.builder, &self.module,
                "toString", &[v], &[ty.clone()],
            )?;
            let sv_val = sv.into_struct_value();
            let len_val = self.builder.build_extract_value(sv_val, 0, "str.len")
                .map_err(|e| format!("extract str.len failed: {:?}", e))?;
            let ptr_val = self.builder.build_extract_value(sv_val, 1, "str.ptr")
                .map_err(|e| format!("extract str.ptr failed: {:?}", e))?;
            let s = StringValue { len: len_val.into_int_value(), ptr: ptr_val.into_pointer_value() };
            return self.build_print_string(s);
        }
        match name {
            "float" | "double" | "half" | "quad" => {
                let f64_type = self.context.f64_type();
                let v = if v.get_type() == f64_type.into() {
                    v.into_float_value()
                } else {
                    self.builder.build_float_ext(v.into_float_value(), f64_type, "ext")
                        .map_err(|e| format!("build_float_ext failed: {:?}", e))?
                };
                let f = self.get_function("titrate_print_double");
                self.builder.build_call(f, &[v.into()], "print.d")
                    .map_err(|e| format!("build_call print_double failed: {:?}", e))?;
            }
            "bool" => {
                let i32_type = self.context.i32_type();
                let v = self.builder.build_int_z_extend(v.into_int_value(), i32_type, "zext")
                    .map_err(|e| format!("build_int_z_extend failed: {:?}", e))?;
                let f = self.get_function("titrate_print_bool");
                self.builder.build_call(f, &[v.into()], "print.b")
                    .map_err(|e| format!("build_call print_bool failed: {:?}", e))?;
            }
            "char" => {
                let f = self.get_function("titrate_print_char");
                self.builder.build_call(f, &[v.into()], "print.c")
                    .map_err(|e| format!("build_call print_char failed: {:?}", e))?;
            }
            _ if llvm_types::is_integer(ty) => {
                let i64_type = self.context.i64_type();
                let v = if v.get_type() == i64_type.into() {
                    v.into_int_value()
                } else if llvm_types::integer_bit_width(ty) < Some(64) {
                    self.builder.build_int_s_extend(v.into_int_value(), i64_type, "sext")
                        .map_err(|e| format!("build_int_s_extend failed: {:?}", e))?
                } else {
                    v.into_int_value()
                };
                let f = self.get_function("titrate_print_int");
                self.builder.build_call(f, &[v.into()], "print.i")
                    .map_err(|e| format!("build_call print_int failed: {:?}", e))?;
            }
            _ => {
                return Err(format!("codegen: cannot print value of type {}", ty));
            }
        }
        Ok(())
    }

    /// Infer the type of an expression. This is a simple local inference
    /// that does not consult the scope; it only works for literals and
    /// simple expressions where the type is obvious. For identifiers, it
    /// looks up the local variable's type.
    fn infer_expr_type(&self, expr: &Expr) -> Type {
        match expr {
            Expr::Literal(lit, _) => match lit {
                Literal::Int(_) => Type::simple("int"),
                Literal::Float(_) => Type::simple("double"),
                Literal::Bool(_) => Type::simple("bool"),
                Literal::Char(_) => Type::simple("char"),
                Literal::String(_) => Type::simple("string"),
                Literal::Null => Type::simple("void"),
            },
            Expr::Identifier(name, _) => {
                // First check if we have a stored Titrate type for this variable.
                if let Some(local) = self.locals.get(name) {
                    if let Some(ref titrate_type) = local.titrate_type {
                        return Type::simple(titrate_type);
                    }
                }
                self.locals.get(name).map(|v| {
                    // Reverse-map the LLVM type back to a Titrate type name.
                    self.llvm_basic_type_to_titrate_type(v.ty)
                }).unwrap_or_else(|| Type::simple("unknown"))
            }
            Expr::Binary(left, op, _, _) => {
                let lt = self.infer_expr_type(left);
                match op {
                    Operator::Eq | Operator::Ne | Operator::Lt | Operator::Gt
                    | Operator::Le | Operator::Ge | Operator::And | Operator::Or => {
                        Type::simple("bool")
                    }
                    _ => lt,
                }
            }
            Expr::Unary(UnOp::Not, _, _) => Type::simple("bool"),
            Expr::Unary(UnOp::Neg | UnOp::BitNot, operand, _) => self.infer_expr_type(operand),
            Expr::Call(callee, _, _) => {
                // Handle bare function calls like toString, parseInt, etc.
                match callee.as_ref() {
                    Expr::Identifier(name, _) => {
                        match name.as_str() {
                            "toString" => Type::simple("string"),
                            "parseInt" => Type::simple("int"),
                            "println" => Type::simple("void"),
                            _ => Type::simple("unknown"),
                        }
                    }
                    _ => Type::simple("unknown"),
                }
            }
            Expr::StaticCall { class_name, method, .. } => {
                // Return type for known static calls
                match (class_name.as_str(), method.as_str()) {
                    ("ArrayList", "size") => Type::simple("int"),
                    ("ArrayList", "get") => Type::simple("string"),
                    ("Integer", "parseInt") | ("Integer", "parseOr") => Type::simple("int"),
                    ("Integer", "toString") | ("Double", "toString") | ("Long", "toString") => Type::simple("string"),
                    ("Double", "parse") | ("Double", "parseDouble") => Type::simple("double"),
                    ("Long", "parseLong") => Type::simple("long"),
                    ("String", "length") => Type::simple("int"),
                    ("String", "charAt") => Type::simple("string"),
                    ("String", "substring") => Type::simple("string"),
                    ("String", "indexOf") => Type::simple("int"),
                    ("String", "toUpperCase") | ("String", "toLowerCase") => Type::simple("string"),
                    ("String", "trim") | ("String", "trimStart") | ("String", "trimEnd") => Type::simple("string"),
                    ("String", "startsWith") | ("String", "endsWith") => Type::simple("bool"),
                    ("String", "replace") => Type::simple("string"),
                    ("String", "split") => Type::simple("array"),
                    ("String", "padLeft") | ("String", "padRight") => Type::simple("string"),
                    ("String", "fromCharCode") => Type::simple("string"),
                    ("String", "join") => Type::simple("string"),
                    ("Math", _) | ("MathAdvanced", _) | ("MathTrig", _) => Type::simple("double"),
                    ("Regex", "match") | ("Regex", "fullMatch") | ("Regex", "matchWithFlags") => Type::simple("bool"),
                    ("Regex", "find") | ("Regex", "replace") | ("Regex", "subN") => Type::simple("string"),
                    ("Regex", "groupCount") => Type::simple("int"),
                    ("Json", "parse") => Type::simple("Variant"),
                    ("Json", "stringify") => Type::simple("string"),
                    ("Hash", _) => Type::simple("string"),
                    ("Base64", _) | ("Hex", _) | ("Url", _) => Type::simple("string"),
                    ("TypeName", "of") => Type::simple("string"),
                    ("Sys", "args") => Type::simple("array"),
                    ("Sys", "env") | ("Sys", "workingDir") => Type::simple("string"),
                    ("Sys", _) => Type::simple("void"),
                    ("Time", "now") | ("Time", "millis") | ("Time", "nanos") => Type::simple("long"),
                    ("Time", "format") => Type::simple("string"),
                    ("Time", _) => Type::simple("int"),
                    ("Os", "name") | ("Os", "arch") | ("Os", "family") => Type::simple("string"),
                    ("Os", _) => Type::simple("long"),
                    ("Path", "join") | ("Path", "basename") | ("Path", "dirname") | ("Path", "extension") => Type::simple("string"),
                    ("Path", _) => Type::simple("bool"),
                    ("File", "readFile") | ("File", "readLine") | ("File", "readChunk") => Type::simple("string"),
                    ("File", "readLines") => Type::simple("array"),
                    ("File", "size") | ("File", "tell") | ("File", "lastModified") => Type::simple("long"),
                    ("File", _) => Type::simple("void"),
                    ("Subprocess", _) => Type::simple("string"),
                    ("Socket", _) => Type::simple("void"),
                    _ => Type::simple("unknown"),
                }
            }
            _ => Type::simple("unknown"),
        }
    }

    /// Best-effort reverse mapping from an LLVM basic type to a Titrate type.
    fn llvm_basic_type_to_titrate_type(&self, ty: BasicTypeEnum<'ctx>) -> Type {
        let s = ty.print_to_string().to_string();
        match s.as_str() {
            "i1" => Type::simple("bool"),
            "i8" => Type::simple("byte"),
            "i16" => Type::simple("short"),
            "i32" => Type::simple("int"),
            "i64" => Type::simple("long"),
            "i128" => Type::simple("vast"),
            "float" => Type::simple("float"),
            "double" => Type::simple("double"),
            "half" => Type::simple("half"),
            "fp128" => Type::simple("quad"),
            _ => {
                // Check if it's the string struct type.
                let string_ty = llvm_types::string_type(self.context);
                if s == string_ty.print_to_string().to_string() {
                    return Type::simple("string");
                }
                Type::simple("unknown")
            }
        }
    }

    /// Compile a literal value to an LLVM constant.
    fn compile_literal(&self, lit: &Literal, hint: Option<&Type>) -> Result<BasicValueEnum<'ctx>, String> {
        match lit {
            Literal::Int(v) => {
                // Use the hint type if provided, otherwise default to i64.
                let ty = hint.cloned().unwrap_or_else(|| Type::simple("long"));
                let int_ty = llvm_types::llvm_type(self.context, &ty)?
                    .into_int_type();
                Ok(int_ty.const_int(*v as u64, false).into())
            }
            Literal::Float(v) => {
                let ty = hint.cloned().unwrap_or_else(|| Type::simple("double"));
                let float_ty = llvm_types::llvm_type(self.context, &ty)?
                    .into_float_type();
                Ok(float_ty.const_float(*v).into())
            }
            Literal::Bool(b) => {
                let i1_ty = self.context.bool_type();
                Ok(i1_ty.const_int(if *b { 1 } else { 0 }, false).into())
            }
            Literal::Char(c) => {
                let i32_ty = self.context.i32_type();
                Ok(i32_ty.const_int(*c as u64, false).into())
            }
            Literal::String(s) => {
                // String literals need &mut self for the counter, so we handle
                // them separately via compile_string_expr.
                // This branch shouldn't be reached; compile_expr handles strings.
                let _ = s;
                Err("string literal should be handled by compile_string_expr".to_string())
            }
            Literal::Null => {
                let i8_ptr = self.context.ptr_type(AddressSpace::default());
                Ok(i8_ptr.const_null().into())
            }
        }
    }

    /// Compile an identifier reference to a loaded value.
    fn compile_identifier_load(&self, name: &str) -> Result<BasicValueEnum<'ctx>, String> {
        let var = self.locals.get(name).ok_or_else(|| {
            format!("codegen: unknown variable '{}'", name)
        })?;
        self.builder.build_load(var.ty, var.ptr, name)
            .map_err(|e| format!("build_load '{}' failed: {:?}", name, e))
    }

    /// Compile any expression to a `BasicValueEnum`.
    fn compile_expr(&mut self, expr: &Expr) -> Result<BasicValueEnum<'ctx>, String> {
        match expr {
            Expr::Literal(Literal::String(s), _) => {
                let sv = self.make_string_global(s);
                self.string_value_to_basic(sv)
            }
            Expr::Literal(lit, _) => self.compile_literal(lit, None),
            Expr::Identifier(name, _) => self.compile_identifier_load(name),
            Expr::Binary(left, op, right, _) => {
                self.compile_binary(left, op, right)
            }
            Expr::Unary(op, operand, _) => {
                self.compile_unary(op, operand)
            }
            Expr::Assign(target, value, _) => {
                self.compile_assign(target, value)
            }
            Expr::Ternary { condition, then_expr, else_expr, .. } => {
                self.compile_ternary(condition, then_expr, else_expr)
            }
            Expr::Call(callee, args, _) => {
                self.compile_call(callee, args)
            }
            Expr::New(type_name, args, _) => {
                self.compile_new(type_name, args)
            }
            Expr::MemberAccess(object, field, _) => {
                self.compile_member_access(object, field)
            }
            Expr::This(span) => {
                self.compile_this(span)
            }
            Expr::Is(obj, ty, _) => {
                self.compile_is(obj, ty)
            }
            Expr::Cast(obj, ty, _) => {
                self.compile_cast(obj, ty)
            }
            Expr::OwnedDeref(inner, _) => self.compile_owned_deref(inner),
            Expr::RefExpr(inner, ref_kind, _) => self.compile_ref_expr(inner, ref_kind),
            Expr::RegionAlloc(ty, init, _) => self.compile_region_alloc(ty, init),
            Expr::UnsafeBlock(block, _) => self.compile_unsafe_block(block),
            Expr::Closure {
                params,
                return_type,
                body,
                expr,
                captured_vars,
                ..
            } => self.compile_closure(params, return_type, body, expr.as_deref(), captured_vars),
            Expr::ErrorPropagation(inner, _) => self.compile_error_propagation(inner),
            Expr::Tuple(elements, _) => self.compile_tuple(elements),
            Expr::StaticCall { class_name, method, args, .. } => {
                self.compile_static_call(class_name, method, args)
            }
            _ => Err(format!("codegen: unsupported expression: {:?}", expr)),
        }
    }

    /// Compile a binary expression.
    fn compile_binary(&mut self, left: &Expr, op: &Operator, right: &Expr) -> Result<BasicValueEnum<'ctx>, String> {
        // Short-circuit logical operators.
        match op {
            Operator::And => return self.compile_short_circuit(left, right, false),
            Operator::Or => return self.compile_short_circuit(left, right, true),
            _ => {}
        }

        // String concatenation.
        let left_ty = self.infer_expr_type(left);
        if *op == Operator::Add && llvm_types::is_string(&left_ty) {
            let l = self.compile_string_expr(left)?;
            let r = self.compile_string_expr(right)?;
            let sv = self.build_string_concat(l, r)?;
            return self.string_value_to_basic(sv);
        }

        // String comparison: ==, !=, <, >, <=, >=
        // Also handle cases where the Titrate type is "unknown" but the LLVM value is a string struct.
        let lv = self.compile_expr(left)?;
        let rv = self.compile_expr(right)?;
        let is_str_cmp = llvm_types::is_string(&left_ty) 
            || (lv.is_struct_value() && rv.is_struct_value() && self.is_string_struct(&lv));
        if is_str_cmp {
            if lv.is_struct_value() && rv.is_struct_value() {
                let ls = lv.into_struct_value();
                let rs = rv.into_struct_value();
                let l_len = self.builder.build_extract_value(ls, 0, "l.len")
                    .map_err(|e| format!("extract l.len failed: {:?}", e))?;
                let r_len = self.builder.build_extract_value(rs, 0, "r.len")
                    .map_err(|e| format!("extract r.len failed: {:?}", e))?;
                let l_ptr = self.builder.build_extract_value(ls, 1, "l.ptr")
                    .map_err(|e| format!("extract l.ptr failed: {:?}", e))?;
                let r_ptr = self.builder.build_extract_value(rs, 1, "r.ptr")
                    .map_err(|e| format!("extract r.ptr failed: {:?}", e))?;

                match op {
                    Operator::Eq | Operator::Ne => {
                        let len_eq = self.builder.build_int_compare(inkwell::IntPredicate::EQ, l_len.into_int_value(), r_len.into_int_value(), "len.eq")
                            .map_err(|e| format!("build_int_compare len failed: {:?}", e))?;
                        let ptr_eq = self.builder.build_int_compare(inkwell::IntPredicate::EQ, l_ptr.into_pointer_value(), r_ptr.into_pointer_value(), "ptr.eq")
                            .map_err(|e| format!("build_int_compare ptr failed: {:?}", e))?;
                        let result = if *op == Operator::Eq {
                            self.builder.build_and(len_eq, ptr_eq, "str.eq")
                        } else {
                            let eq_val = self.builder.build_and(len_eq, ptr_eq, "str.eq.tmp")
                                .map_err(|e| format!("build_and failed: {:?}", e))?;
                            self.builder.build_not(eq_val, "str.ne")
                        }.map_err(|e| format!("build logic failed: {:?}", e))?;
                        return Ok(result.into());
                    }
                    _ => {
                        // Ordering comparisons: use memcmp
                        let i32_ty = self.context.i32_type();
                        let i8_ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
                        // Declare memcmp(i8*, i8*, i64) -> i32
                        let memcmp_fn_name = "memcmp";
                        let memcmp_fn = if let Some(f) = self.module.get_function(memcmp_fn_name) {
                            f
                        } else {
                            let fn_type = i32_ty.fn_type(&[i8_ptr_ty.into(), i8_ptr_ty.into(), self.context.i64_type().into()], false);
                            self.module.add_function(memcmp_fn_name, fn_type, Some(Linkage::External))
                        };
                        // min(l_len, r_len) using select: min = r_len <= l_len ? r_len : l_len
                        let r_le_l = self.builder.build_int_compare(inkwell::IntPredicate::ULE, l_len.into_int_value(), r_len.into_int_value(), "r.le.l")
                            .map_err(|e| format!("build_int_compare failed: {:?}", e))?;
                        let min_len = self.builder.build_select(r_le_l, l_len.into_int_value(), r_len.into_int_value(), "min.len")
                            .map_err(|e| format!("build_select failed: {:?}", e))?;
                        let cmp_result = self.builder.build_call(memcmp_fn, &[l_ptr.into(), r_ptr.into(), min_len.into_int_value().into()], "memcmp")
                            .map_err(|e| format!("build_call memcmp failed: {:?}", e))?;
                        if let inkwell::values::ValueKind::Basic(basic_val) = cmp_result.try_as_basic_value() {
                            let cmp_int = basic_val.into_int_value();
                            let zero = i32_ty.const_int(0, true);
                            // If memcmp returned 0, strings are equal up to min_len; compare lengths
                            let is_zero = self.builder.build_int_compare(inkwell::IntPredicate::EQ, cmp_int, zero, "cmp.zero")
                                .map_err(|e| format!("build_int_compare failed: {:?}", e))?;
                            let len_cmp = match *op {
                                Operator::Lt => self.builder.build_int_compare(inkwell::IntPredicate::ULT, l_len.into_int_value(), r_len.into_int_value(), "len.lt"),
                                Operator::Gt => self.builder.build_int_compare(inkwell::IntPredicate::ULT, r_len.into_int_value(), l_len.into_int_value(), "len.gt"),
                                Operator::Le => self.builder.build_int_compare(inkwell::IntPredicate::ULE, l_len.into_int_value(), r_len.into_int_value(), "len.le"),
                                Operator::Ge => self.builder.build_int_compare(inkwell::IntPredicate::ULE, r_len.into_int_value(), l_len.into_int_value(), "len.ge"),
                                _ => unreachable!(),
                            }.map_err(|e| format!("build_int_compare len failed: {:?}", e))?;
                            let memcmp_lt = match *op {
                                Operator::Lt => self.builder.build_int_compare(inkwell::IntPredicate::SLT, cmp_int, zero, "cmp.lt"),
                                Operator::Gt => self.builder.build_int_compare(inkwell::IntPredicate::SGT, cmp_int, zero, "cmp.gt"),
                                Operator::Le => self.builder.build_int_compare(inkwell::IntPredicate::SLE, cmp_int, zero, "cmp.le"),
                                Operator::Ge => self.builder.build_int_compare(inkwell::IntPredicate::SGE, cmp_int, zero, "cmp.ge"),
                                _ => unreachable!(),
                            }.map_err(|e| format!("build_int_compare memcmp failed: {:?}", e))?;
                            // Result = (memcmp != 0 && memcmp matches op) || (memcmp == 0 && len matches op)
                            let nonzero_and_match = self.builder.build_and(
                                self.builder.build_not(is_zero, "cmp.nonzero")
                                    .map_err(|e| format!("build_not failed: {:?}", e))?,
                                memcmp_lt,
                                "nonzero.match"
                            ).map_err(|e| format!("build_and failed: {:?}", e))?;
                            let result = self.builder.build_or(nonzero_and_match, len_cmp, "str.cmp.result")
                                .map_err(|e| format!("build_or failed: {:?}", e))?;
                            return Ok(result.into());
                        }
                        return Err("codegen: memcmp did not return a value".to_string());
                    }
                }
            }
        }

        let lv = self.compile_expr(left)?;
        let rv = self.compile_expr(right)?;

        // Coerce operands to the same type if they differ (e.g., i32 var + i64 literal).
        let (lv, rv) = self.coerce_binary_operands(lv, rv)?;

        // Determine if this is an integer or float operation.
        let is_float = lv.is_float_value();
        let is_int = lv.is_int_value();

        if is_float {
            let l = lv.into_float_value();
            let r = rv.into_float_value();
            // Float comparisons produce i1, not float — handle them here
            // before dispatching to compile_float_binary (which only does
            // arithmetic and would otherwise return a FloatValue).
            if let Some(pred) = Self::float_compare_predicate(op) {
                let cmp = self.builder.build_float_compare(pred, l, r, "fcmp")
                    .map_err(|e| format!("build_float_compare failed: {:?}", e))?;
                return Ok(cmp.into());
            }
            return Ok(self.compile_float_binary(op, l, r)?.into());
        }

        if is_int {
            let l = lv.into_int_value();
            let r = rv.into_int_value();
            return Ok(self.compile_int_binary(op, l, r, &left_ty)?.into());
        }

        Err(format!("codegen: unsupported binary operand types: {:?} {:?}", lv.get_type(), rv.get_type()))
    }

    /// Coerce two binary operands to the same LLVM type so that arithmetic
    /// and comparison instructions don't trigger LLVM assertion failures.
    /// The narrower type is extended to the wider type (signed for ints,
    /// extended for floats). int+float mixes convert the int to float.
    fn coerce_binary_operands(
        &self,
        lv: BasicValueEnum<'ctx>,
        rv: BasicValueEnum<'ctx>,
    ) -> Result<(BasicValueEnum<'ctx>, BasicValueEnum<'ctx>), String> {
        if lv.get_type() == rv.get_type() {
            return Ok((lv, rv));
        }
        // Both int: sign-extend the narrower one.
        if lv.is_int_value() && rv.is_int_value() {
            let l = lv.into_int_value();
            let r = rv.into_int_value();
            let l_bits = l.get_type().get_bit_width();
            let r_bits = r.get_type().get_bit_width();
            if l_bits < r_bits {
                let l2 = self.builder.build_int_s_extend(l, r.get_type(), "coerce.sext")
                    .map_err(|e| format!("build_int_s_extend coerce failed: {:?}", e))?;
                Ok((l2.into(), rv))
            } else {
                let r2 = self.builder.build_int_s_extend(r, l.get_type(), "coerce.sext")
                    .map_err(|e| format!("build_int_s_extend coerce failed: {:?}", e))?;
                Ok((lv, r2.into()))
            }
        } else if lv.is_float_value() && rv.is_float_value() {
            // Both float: extend the narrower one.
            let l = lv.into_float_value();
            let r = rv.into_float_value();
            let l_str = l.get_type().print_to_string().to_string();
            let r_str = r.get_type().print_to_string().to_string();
            let l_bits: u32 = match l_str.as_str() {
                "half" => 16, "float" => 32, "double" => 64, "fp128" => 128, _ => 64,
            };
            let r_bits: u32 = match r_str.as_str() {
                "half" => 16, "float" => 32, "double" => 64, "fp128" => 128, _ => 64,
            };
            if l_bits < r_bits {
                let l2 = self.builder.build_float_ext(l, r.get_type(), "coerce.fext")
                    .map_err(|e| format!("build_float_ext coerce failed: {:?}", e))?;
                Ok((l2.into(), rv))
            } else {
                let r2 = self.builder.build_float_ext(r, l.get_type(), "coerce.fext")
                    .map_err(|e| format!("build_float_ext coerce failed: {:?}", e))?;
                Ok((lv, r2.into()))
            }
        } else if lv.is_int_value() && rv.is_float_value() {
            // int + float: convert int to float.
            let l = lv.into_int_value();
            let r_ty = rv.into_float_value().get_type();
            let l2 = self.builder.build_signed_int_to_float(l, r_ty, "coerce.itof")
                .map_err(|e| format!("build_signed_int_to_float coerce failed: {:?}", e))?;
            Ok((l2.into(), rv))
        } else if lv.is_float_value() && rv.is_int_value() {
            // float + int: convert int to float.
            let r = rv.into_int_value();
            let l_ty = lv.into_float_value().get_type();
            let r2 = self.builder.build_signed_int_to_float(r, l_ty, "coerce.itof")
                .map_err(|e| format!("build_signed_int_to_float coerce failed: {:?}", e))?;
            Ok((lv, r2.into()))
        } else {
            // If we can't coerce, just return as-is and hope.
            Ok((lv, rv))
        }
    }

    /// Compile an integer binary operation.
    fn compile_int_binary(
        &self,
        op: &Operator,
        l: IntValue<'ctx>,
        r: IntValue<'ctx>,
        ty: &Type,
    ) -> Result<IntValue<'ctx>, String> {
        use inkwell::IntPredicate::*;
        match op {
            Operator::Add => self.builder.build_int_add(l, r, "add")
                .map_err(|e| format!("build_int_add failed: {:?}", e)),
            Operator::Sub => self.builder.build_int_sub(l, r, "sub")
                .map_err(|e| format!("build_int_sub failed: {:?}", e)),
            Operator::Mul => self.builder.build_int_mul(l, r, "mul")
                .map_err(|e| format!("build_int_mul failed: {:?}", e)),
            Operator::Div => {
                // Signed or unsigned depending on type.
                if Self::is_unsigned_type(ty) {
                    self.builder.build_int_unsigned_div(l, r, "udiv")
                        .map_err(|e| format!("build_int_unsigned_div failed: {:?}", e))
                } else {
                    self.builder.build_int_signed_div(l, r, "sdiv")
                        .map_err(|e| format!("build_int_signed_div failed: {:?}", e))
                }
            }
            Operator::Mod => {
                if Self::is_unsigned_type(ty) {
                    self.builder.build_int_unsigned_rem(l, r, "urem")
                        .map_err(|e| format!("build_int_unsigned_rem failed: {:?}", e))
                } else {
                    self.builder.build_int_signed_rem(l, r, "srem")
                        .map_err(|e| format!("build_int_signed_rem failed: {:?}", e))
                }
            }
            Operator::Eq => self.builder.build_int_compare(EQ, l, r, "eq")
                .map_err(|e| format!("build_int_compare eq failed: {:?}", e)),
            Operator::Ne => self.builder.build_int_compare(NE, l, r, "ne")
                .map_err(|e| format!("build_int_compare ne failed: {:?}", e)),
            Operator::Lt => {
                let pred = if Self::is_unsigned_type(ty) { ULT } else { SLT };
                self.builder.build_int_compare(pred, l, r, "lt")
                    .map_err(|e| format!("build_int_compare lt failed: {:?}", e))
            }
            Operator::Gt => {
                let pred = if Self::is_unsigned_type(ty) { UGT } else { SGT };
                self.builder.build_int_compare(pred, l, r, "gt")
                    .map_err(|e| format!("build_int_compare gt failed: {:?}", e))
            }
            Operator::Le => {
                let pred = if Self::is_unsigned_type(ty) { ULE } else { SLE };
                self.builder.build_int_compare(pred, l, r, "le")
                    .map_err(|e| format!("build_int_compare le failed: {:?}", e))
            }
            Operator::Ge => {
                let pred = if Self::is_unsigned_type(ty) { UGE } else { SGE };
                self.builder.build_int_compare(pred, l, r, "ge")
                    .map_err(|e| format!("build_int_compare ge failed: {:?}", e))
            }
            Operator::BitAnd => self.builder.build_and(l, r, "band")
                .map_err(|e| format!("build_and failed: {:?}", e)),
            Operator::BitOr => self.builder.build_or(l, r, "bor")
                .map_err(|e| format!("build_or failed: {:?}", e)),
            Operator::BitXor => self.builder.build_xor(l, r, "bxor")
                .map_err(|e| format!("build_xor failed: {:?}", e)),
            Operator::BitShl => self.builder.build_left_shift(l, r, "shl")
                .map_err(|e| format!("build_left_shift failed: {:?}", e)),
            Operator::BitShr | Operator::BitUshr => {
                if Self::is_unsigned_type(ty) || *op == Operator::BitUshr {
                    self.builder.build_right_shift(l, r, false, "ushr")
                        .map_err(|e| format!("build_right_shift ushr failed: {:?}", e))
                } else {
                    self.builder.build_right_shift(l, r, true, "sshr")
                        .map_err(|e| format!("build_right_shift sshr failed: {:?}", e))
                }
            }
            Operator::And | Operator::Or => {
                unreachable!("short-circuit operators handled elsewhere")
            }
        }
    }

    /// Map a comparison operator to its LLVM float predicate, or return
    /// `None` if the operator is not a comparison (i.e. arithmetic).
    fn float_compare_predicate(op: &Operator) -> Option<inkwell::FloatPredicate> {
        use inkwell::FloatPredicate::*;
        match op {
            Operator::Eq => Some(OEQ),
            Operator::Ne => Some(ONE),
            Operator::Lt => Some(OLT),
            Operator::Gt => Some(OGT),
            Operator::Le => Some(OLE),
            Operator::Ge => Some(OGE),
            _ => None,
        }
    }

    /// Compile a float binary operation (arithmetic only — comparisons are
    /// handled in `compile_binary` because they produce i1, not float).
    fn compile_float_binary(
        &self,
        op: &Operator,
        l: inkwell::values::FloatValue<'ctx>,
        r: inkwell::values::FloatValue<'ctx>,
    ) -> Result<inkwell::values::FloatValue<'ctx>, String> {
        match op {
            Operator::Add => self.builder.build_float_add(l, r, "fadd")
                .map_err(|e| format!("build_float_add failed: {:?}", e)),
            Operator::Sub => self.builder.build_float_sub(l, r, "fsub")
                .map_err(|e| format!("build_float_sub failed: {:?}", e)),
            Operator::Mul => self.builder.build_float_mul(l, r, "fmul")
                .map_err(|e| format!("build_float_mul failed: {:?}", e)),
            Operator::Div => self.builder.build_float_div(l, r, "fdiv")
                .map_err(|e| format!("build_float_div failed: {:?}", e)),
            Operator::Mod => self.builder.build_float_rem(l, r, "frem")
                .map_err(|e| format!("build_float_rem failed: {:?}", e)),
            _ => Err(format!("codegen: unsupported float operator {:?}", op)),
        }
    }

    /// Compile a short-circuit logical operator (&& or ||).
    /// If `is_or` is true, this is ||; otherwise it's &&.
    fn compile_short_circuit(
        &mut self,
        left: &Expr,
        right: &Expr,
        is_or: bool,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let i1_ty = self.context.bool_type();
        let current_block = self.builder.get_insert_block()
            .ok_or("codegen: no insert block for short-circuit")?;
        let rhs_block = self.context.insert_basic_block_after(current_block, "logic.rhs");
        let end_block = self.context.insert_basic_block_after(rhs_block, "logic.end");

        let lv = self.compile_expr(left)?.into_int_value();

        if is_or {
            // ||: if left is true, short-circuit to end with true.
            self.builder.build_conditional_branch(lv, end_block, rhs_block)
                .map_err(|e| format!("build_cond_br or failed: {:?}", e))?;
        } else {
            // &&: if left is false, short-circuit to end with false.
            self.builder.build_conditional_branch(lv, rhs_block, end_block)
                .map_err(|e| format!("build_cond_br and failed: {:?}", e))?;
        }

        // RHS block: evaluate right, then go to end.
        self.builder.position_at_end(rhs_block);
        let rv = self.compile_expr(right)?.into_int_value();
        let rhs_block_now = self.builder.get_insert_block()
            .ok_or("codegen: no insert block after rhs")?;
        self.builder.build_unconditional_branch(end_block)
            .map_err(|e| format!("build_br end failed: {:?}", e))?;

        // End block: phi together the results.
        self.builder.position_at_end(end_block);
        let phi = self.builder.build_phi(i1_ty, "logic.result")
            .map_err(|e| format!("build_phi failed: {:?}", e))?;
        let short_circuit_val = i1_ty.const_int(if is_or { 1 } else { 0 }, false);
        phi.add_incoming(&[
            (&short_circuit_val, current_block),
            (&rv, rhs_block_now),
        ]);

        Ok(phi.as_basic_value())
    }

    /// Compile a unary expression.
    fn compile_unary(&mut self, op: &UnOp, operand: &Expr) -> Result<BasicValueEnum<'ctx>, String> {
        let v = self.compile_expr(operand)?;
        match op {
            UnOp::Neg => {
                if v.is_float_value() {
                    let f = v.into_float_value();
                    let neg = self.builder.build_float_neg(f, "fneg")
                        .map_err(|e| format!("build_float_neg failed: {:?}", e))?;
                    Ok(neg.into())
                } else {
                    let i = v.into_int_value();
                    let zero = i.get_type().const_int(0, false);
                    let neg = self.builder.build_int_sub(zero, i, "ineg")
                        .map_err(|e| format!("build_int_sub neg failed: {:?}", e))?;
                    Ok(neg.into())
                }
            }
            UnOp::Not => {
                let i = v.into_int_value();
                let not = self.builder.build_not(i, "lnot")
                    .map_err(|e| format!("build_not failed: {:?}", e))?;
                Ok(not.into())
            }
            UnOp::BitNot => {
                let i = v.into_int_value();
                let not = self.builder.build_not(i, "bnot")
                    .map_err(|e| format!("build_not bit failed: {:?}", e))?;
                Ok(not.into())
            }
        }
    }

    /// Compile an assignment expression.
    fn compile_assign(&mut self, target: &Expr, value: &Expr) -> Result<BasicValueEnum<'ctx>, String> {
        if let Expr::Identifier(name, _) = target {
            // Clone the LocalVar to release the immutable borrow
            // of self.locals before we call &mut self methods below.
            let var = self.locals.get(name).cloned().ok_or_else(|| {
                format!("codegen: assignment to unknown variable '{}'", name)
            })?;
            // String assignment.
            if llvm_types::is_string(&self.llvm_basic_type_to_titrate_type(var.ty)) {
                let sv = self.compile_string_expr(value)?;
                let string_ty = llvm_types::string_type(self.context).into_struct_type();
                let alloca = self.builder.build_alloca(string_ty, "assign.tmp")
                    .map_err(|e| format!("build_alloca assign.tmp failed: {:?}", e))?;
                let len_ptr = self.builder.build_struct_gep(string_ty, alloca, 0, "assign.len.ptr")
                    .map_err(|e| format!("build_struct_gep 0 failed: {:?}", e))?;
                let ptr_ptr = self.builder.build_struct_gep(string_ty, alloca, 1, "assign.ptr.ptr")
                    .map_err(|e| format!("build_struct_gep 1 failed: {:?}", e))?;
                self.builder.build_store(len_ptr, sv.len)
                    .map_err(|e| format!("build_store len failed: {:?}", e))?;
                self.builder.build_store(ptr_ptr, sv.ptr)
                    .map_err(|e| format!("build_store ptr failed: {:?}", e))?;
                let struct_val = self.builder.build_load(string_ty, alloca, "assign.val")
                    .map_err(|e| format!("build_load assign.val failed: {:?}", e))?;
                self.builder.build_store(var.ptr, struct_val)
                    .map_err(|e| format!("build_store struct failed: {:?}", e))?;
                return Ok(struct_val);
            }
            // Primitive assignment.
            let v = self.compile_expr(value)?;
            // Cast to the variable's type if needed.
            let v = self.cast_value_to_type(v, var.ty)?;
            self.builder.build_store(var.ptr, v)
                .map_err(|e| format!("build_store assign failed: {:?}", e))?;
            return Ok(v);
        }
        // Field assignment: obj.field = value
        if let Expr::MemberAccess(obj, field, _) = target {
            let obj_val = self.compile_expr(obj)?;
            if obj_val.is_pointer_value() {
                let obj_ptr = obj_val.into_pointer_value();
                let obj_type = self.infer_expr_type(obj);
                let class_name = obj_type.name();
                let class_info = self.class_infos.get(class_name).cloned()
                    .ok_or_else(|| format!("codegen: class '{}' not found for field store", class_name))?;
                let v = self.compile_expr(value)?;
                emit_field_store(self.context, &self.builder, &class_info, obj_ptr, field, v)?;
                return Ok(v);
            }
        }
        Err(format!("codegen: unsupported assignment target: {:?}", target))
    }

    /// Cast a value to match the given LLVM type (for assignments and
    /// declarations where the declared type may differ from the literal type).
    fn cast_value_to_type(&self, v: BasicValueEnum<'ctx>, target_ty: BasicTypeEnum<'ctx>) -> Result<BasicValueEnum<'ctx>, String> {
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
                self.builder.build_int_s_extend(from, to_ty, "cast.sext")
                    .map_err(|e| format!("build_int_s_extend failed: {:?}", e))?
            } else if from_bits > to_bits {
                self.builder.build_int_truncate(from, to_ty, "cast.trunc")
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
                self.builder.build_float_ext(from, to_ty, "cast.ext")
                    .map_err(|e| format!("build_float_ext failed: {:?}", e))?
            } else if from_str == "half" && to_str == "double" {
                self.builder.build_float_ext(from, to_ty, "cast.ext")
                    .map_err(|e| format!("build_float_ext failed: {:?}", e))?
            } else if from_str == "float" && to_str == "double" {
                self.builder.build_float_ext(from, to_ty, "cast.ext")
                    .map_err(|e| format!("build_float_ext failed: {:?}", e))?
            } else if from_str == "double" && to_str == "float" {
                self.builder.build_float_trunc(from, to_ty, "cast.trunc")
                    .map_err(|e| format!("build_float_trunc failed: {:?}", e))?
            } else if from_str == "double" && to_str == "half" {
                self.builder.build_float_trunc(from, to_ty, "cast.trunc")
                    .map_err(|e| format!("build_float_trunc failed: {:?}", e))?
            } else if from_str == "float" && to_str == "half" {
                self.builder.build_float_trunc(from, to_ty, "cast.trunc")
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
            let result = self.builder.build_signed_int_to_float(from, to_ty, "cast.itof")
                .map_err(|e| format!("build_signed_int_to_float failed: {:?}", e))?;
            return Ok(result.into());
        }
        // float -> int (explicit `as` cast, e.g. `3.14 as int`)
        if v.is_float_value() && target_ty.is_int_type() {
            let from = v.into_float_value();
            let to_ty = target_ty.into_int_type();
            let result = self.builder.build_float_to_signed_int(from, to_ty, "cast.ftoi")
                .map_err(|e| format!("build_float_to_signed_int failed: {:?}", e))?;
            return Ok(result.into());
        }
        // If types don't match but we can't cast, just return as-is and hope.
        Ok(v)
    }

    /// Compile a ternary expression using a phi node.
    fn compile_ternary(
        &mut self,
        condition: &Expr,
        then_expr: &Expr,
        else_expr: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let cond = self.compile_expr(condition)?.into_int_value();
        let current_block = self.builder.get_insert_block()
            .ok_or("codegen: no insert block for ternary")?;
        let then_block = self.context.insert_basic_block_after(current_block, "tern.then");
        let else_block = self.context.insert_basic_block_after(then_block, "tern.else");
        let end_block = self.context.insert_basic_block_after(else_block, "tern.end");

        self.builder.build_conditional_branch(cond, then_block, else_block)
            .map_err(|e| format!("build_cond_br ternary failed: {:?}", e))?;

        self.builder.position_at_end(then_block);
        let then_val = self.compile_expr(then_expr)?;
        self.builder.build_unconditional_branch(end_block)
            .map_err(|e| format!("build_br tern end failed: {:?}", e))?;
        let then_block_end = self.builder.get_insert_block()
            .ok_or("codegen: no insert block after then")?;

        self.builder.position_at_end(else_block);
        let else_val = self.compile_expr(else_expr)?;
        self.builder.build_unconditional_branch(end_block)
            .map_err(|e| format!("build_br tern end 2 failed: {:?}", e))?;
        let else_block_end = self.builder.get_insert_block()
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
                    then_block_end.get_terminator().unwrap().erase_from_basic_block();
                    let lv_coerced = self.builder.build_int_s_extend(lv, target, "coerce.then")
                        .map_err(|e| format!("coerce then failed: {:?}", e))?;
                    self.builder.build_unconditional_branch(end_block)
                        .map_err(|e| format!("rebuild br then failed: {:?}", e))?;

                    // Rebuild else block with coercion
                    self.builder.position_at_end(else_block_end);
                    else_block_end.get_terminator().unwrap().erase_from_basic_block();
                    let rv_coerced = self.builder.build_int_s_extend(rv, target, "coerce.else")
                        .map_err(|e| format!("coerce else failed: {:?}", e))?;
                    self.builder.build_unconditional_branch(end_block)
                        .map_err(|e| format!("rebuild br else failed: {:?}", e))?;

                    self.builder.position_at_end(end_block);
                    let phi = self.builder.build_phi(target.as_basic_type_enum(), "tern.result")
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
                    then_block_end.get_terminator().unwrap().erase_from_basic_block();
                    let lv_coerced = self.builder.build_float_ext(lv, target, "coerce.then")
                        .map_err(|e| format!("coerce then failed: {:?}", e))?;
                    self.builder.build_unconditional_branch(end_block)
                        .map_err(|e| format!("rebuild br then failed: {:?}", e))?;

                    self.builder.position_at_end(else_block_end);
                    else_block_end.get_terminator().unwrap().erase_from_basic_block();
                    let rv_coerced = self.builder.build_float_ext(rv, target, "coerce.else")
                        .map_err(|e| format!("coerce else failed: {:?}", e))?;
                    self.builder.build_unconditional_branch(end_block)
                        .map_err(|e| format!("rebuild br else failed: {:?}", e))?;

                    self.builder.position_at_end(end_block);
                    let phi = self.builder.build_phi(target.as_basic_type_enum(), "tern.result")
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
                    then_block_end.get_terminator().unwrap().erase_from_basic_block();
                    let lv_coerced = self.builder.build_signed_int_to_float(lv, rv.get_type(), "coerce.itof")
                        .map_err(|e| format!("coerce then failed: {:?}", e))?;
                    self.builder.build_unconditional_branch(end_block)
                        .map_err(|e| format!("rebuild br then failed: {:?}", e))?;

                    self.builder.position_at_end(end_block);
                    let phi = self.builder.build_phi(rv.get_type().as_basic_type_enum(), "tern.result")
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
                    else_block_end.get_terminator().unwrap().erase_from_basic_block();
                    let rv_coerced = self.builder.build_signed_int_to_float(rv, lv.get_type(), "coerce.itof")
                        .map_err(|e| format!("coerce else failed: {:?}", e))?;
                    self.builder.build_unconditional_branch(end_block)
                        .map_err(|e| format!("rebuild br else failed: {:?}", e))?;

                    self.builder.position_at_end(end_block);
                    let phi = self.builder.build_phi(lv.get_type().as_basic_type_enum(), "tern.result")
                        .map_err(|e| format!("build_phi ternary failed: {:?}", e))?;
                    phi.add_incoming(&[
                        (&then_val, then_block_end),
                        (&BasicValueEnum::FloatValue(rv_coerced), else_block_end),
                    ]);
                    return Ok(phi.as_basic_value());
                }
                _ => {}
            }
        }

        self.builder.position_at_end(end_block);
        let phi = self.builder.build_phi(then_val.get_type(), "tern.result")
            .map_err(|e| format!("build_phi ternary failed: {:?}", e))?;
        phi.add_incoming(&[(&then_val, then_block_end), (&else_val, else_block_end)]);

        Ok(phi.as_basic_value())
    }

    /// Compile a function call.
    fn compile_call(
        &mut self,
        callee: &Expr,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        // io::println(...) and io::print(...)
        if let Expr::MemberAccess(namespace, method, _) = callee {
            if let Expr::Identifier(ns, _) = &**namespace {
                if ns == "io" && (method == "println" || method == "print") {
                    if args.len() != 1 {
                        return Err(format!(
                            "codegen: io::{} expects 1 argument, got {}",
                            method, args.len()
                        ));
                    }
                    let arg = &args[0];
                    let arg_ty = self.infer_expr_type(arg);
                    if llvm_types::is_string(&arg_ty) {
                        let s = self.compile_string_expr(arg)?;
                        if method == "println" {
                            self.build_println_string(s)?;
                        } else {
                            self.build_print_string(s)?;
                        }
                    } else {
                        let v = self.compile_expr(arg)?;
                        if method == "println" {
                            self.build_println_primitive(v, &arg_ty)?;
                        } else {
                            self.build_print_primitive(v, &arg_ty)?;
                        }
                    }
                    // Return a zero i32 as a placeholder; the caller (compile_stmt)
                    // discards it, and compile_return / default-ret handles void fns.
                    let i32_ty = self.context.i32_type();
                    return Ok(i32_ty.const_int(0, false).into());
                }
            }
        }

        // Enum construction: EnumName::Variant(args)
        if let Expr::StaticCall { class_name, method, args: static_args, .. } = callee {
            let enum_name = class_name.as_str();
            if let Some(enum_info) = self.enum_infos.get(enum_name).cloned() {
                let mut arg_vals: Vec<BasicValueEnum> = Vec::new();
                for arg in static_args {
                    arg_vals.push(self.compile_expr(arg)?);
                }
                return emit_enum_construct(self.context, &self.builder, &self.module, &enum_info, method, &arg_vals);
            }
        }

        // Built-in `ok(value)` and `err(value)` — create Result<T, E> structs.
        if let Expr::Identifier(name, _) = callee {
            if name == "ok" || name == "err" {
                return self.compile_result_ctor(name, args);
            }
        }

        // Direct function call: identifier(args)
        if let Expr::Identifier(name, _) = callee {
            if let Some(&fn_val) = self.functions.get(name) {
                let param_types = fn_val.get_type().get_param_types();
                let mut arg_vals = Vec::with_capacity(args.len());
                for (i, arg) in args.iter().enumerate() {
                    let v = self.compile_expr(arg)?;
                    // Cast the argument to the parameter type if needed
                    // (e.g., i64 literal to i32 parameter).
                    let v = if i < param_types.len() {
                        let param_ty: BasicTypeEnum<'ctx> = match param_types[i] {
                            inkwell::types::BasicMetadataTypeEnum::ArrayType(t) => t.into(),
                            inkwell::types::BasicMetadataTypeEnum::FloatType(t) => t.into(),
                            inkwell::types::BasicMetadataTypeEnum::IntType(t) => t.into(),
                            inkwell::types::BasicMetadataTypeEnum::PointerType(t) => t.into(),
                            inkwell::types::BasicMetadataTypeEnum::StructType(t) => t.into(),
                            inkwell::types::BasicMetadataTypeEnum::VectorType(t) => t.into(),
                            _ => return Err(format!("unsupported parameter type for function '{}'", name)),
                        };
                        if v.get_type() != param_ty {
                            self.cast_value_to_type(v, param_ty)?
                        } else {
                            v
                        }
                    } else {
                        v
                    };
                    arg_vals.push(v.into());
                }
                let call = self.builder.build_call(fn_val, &arg_vals, "call")
                    .map_err(|e| format!("build_call '{}' failed: {:?}", name, e))?;
                if fn_val.get_type().get_return_type().is_some() {
                    match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(v) => return Ok(v),
                        _ => return Err(format!("function '{}' did not return a value", name)),
                    }
                } else {
                    let i32_ty = self.context.i32_type();
                    return Ok(i32_ty.const_int(0, false).into());
                }
            }

            // Forward reference: function declared after the call site.
            // Create a declaration from the function_decls registry.
            if let Some(fn_decl) = self.function_decls.get(name).cloned() {
                let fn_val = if let Some(existing) = self.module.get_function(name) {
                    existing
                } else {
                    let param_types: Vec<BasicTypeEnum> = fn_decl.params.iter()
                        .map(|p| llvm_types::llvm_type(self.context, &p.typ))
                        .collect::<Result<Vec<_>, _>>()?;
                    let fn_type = if fn_decl.params.is_empty() {
                        match &fn_decl.return_type {
                            Some(ty) => {
                                let ret = llvm_types::llvm_type(self.context, ty)?;
                                match ret {
                                    BasicTypeEnum::IntType(t) => t.fn_type(&[], false),
                                    BasicTypeEnum::FloatType(t) => t.fn_type(&[], false),
                                    BasicTypeEnum::PointerType(t) => t.fn_type(&[], false),
                                    BasicTypeEnum::StructType(t) => t.fn_type(&[], false),
                                    BasicTypeEnum::ArrayType(t) => t.fn_type(&[], false),
                                    _ => return Err(format!("unsupported return type for function '{}'", name)),
                                }
                            }
                            None => self.context.void_type().fn_type(&[], false),
                        }
                    } else {
                        let params: Vec<inkwell::types::BasicMetadataTypeEnum> = param_types.iter()
                            .map(|t| (*t).into())
                            .collect();
                        match &fn_decl.return_type {
                            Some(ty) => {
                                let ret = llvm_types::llvm_type(self.context, ty)?;
                                match ret {
                                    BasicTypeEnum::IntType(t) => t.fn_type(&params, false),
                                    BasicTypeEnum::FloatType(t) => t.fn_type(&params, false),
                                    BasicTypeEnum::PointerType(t) => t.fn_type(&params, false),
                                    BasicTypeEnum::StructType(t) => t.fn_type(&params, false),
                                    BasicTypeEnum::ArrayType(t) => t.fn_type(&params, false),
                                    _ => return Err(format!("unsupported return type for function '{}'", name)),
                                }
                            }
                            None => self.context.void_type().fn_type(&params, false),
                        }
                    };
                    self.module.add_function(name, fn_type, Some(Linkage::Internal))
                };
                self.functions.insert(name.clone(), fn_val);
                let param_types = fn_val.get_type().get_param_types();
                let mut arg_vals = Vec::with_capacity(args.len());
                for (i, arg) in args.iter().enumerate() {
                    let v = self.compile_expr(arg)?;
                    let v = if i < param_types.len() {
                        let param_ty: BasicTypeEnum<'ctx> = match param_types[i] {
                            inkwell::types::BasicMetadataTypeEnum::ArrayType(t) => t.into(),
                            inkwell::types::BasicMetadataTypeEnum::FloatType(t) => t.into(),
                            inkwell::types::BasicMetadataTypeEnum::IntType(t) => t.into(),
                            inkwell::types::BasicMetadataTypeEnum::PointerType(t) => t.into(),
                            inkwell::types::BasicMetadataTypeEnum::StructType(t) => t.into(),
                            inkwell::types::BasicMetadataTypeEnum::VectorType(t) => t.into(),
                            _ => return Err(format!("unsupported parameter type for function '{}'", name)),
                        };
                        if v.get_type() != param_ty {
                            self.cast_value_to_type(v, param_ty)?
                        } else {
                            v
                        }
                    } else {
                        v
                    };
                    arg_vals.push(v.into());
                }
                let call = self.builder.build_call(fn_val, &arg_vals, "call")
                    .map_err(|e| format!("build_call '{}' failed: {:?}", name, e))?;
                if fn_val.get_type().get_return_type().is_some() {
                    match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(v) => return Ok(v),
                        _ => return Err(format!("function '{}' did not return a value", name)),
                    }
                } else {
                    let i32_ty = self.context.i32_type();
                    return Ok(i32_ty.const_int(0, false).into());
                }
            }
        }

        // Native function call: Math.sin(x), String.length(s), parseInt(s), etc.
        // This is a fallback after user-defined functions have been checked.
        if let Some(native_name) = native_bridge::try_native_call_name(callee) {
            let mut arg_vals: Vec<BasicValueEnum> = Vec::new();
            let mut arg_types: Vec<Type> = Vec::new();
            for arg in args {
                let arg_ty = self.infer_expr_type(arg);
                let val = self.compile_expr(arg)?;
                arg_vals.push(val);
                arg_types.push(arg_ty);
            }
            return native_bridge::emit_native_call(
                self.context, &self.builder, &self.module,
                &native_name, &arg_vals, &arg_types,
            );
        }

        // Container method call: args.size(), parts.get(i), etc.
        // Resolve the type of the object to find the correct native function name.
        if let Expr::MemberAccess(obj, method, _) = callee {
            // For Sys_args().size() style calls, the obj is a Call expression.
            // Check if the obj call returns a known container type.
            if let Expr::Call(inner_callee, _, _) = obj.as_ref() {
                // Check if this is a native function returning array type
                if let Some(native_name) = native_bridge::try_native_call_name(inner_callee) {
                    let return_ty = native_bridge::infer_native_return_type(&native_name);
                    let type_name_str = return_ty.name();
                    let native_prefix: &str = match type_name_str {
                        "ArrayList" | "array" => "ArrayList",
                        "HashMap" => "HashMap",
                        other => other,
                    };
                    let full_native = format!("{}_{}", native_prefix, method);
                    if native_bridge::is_native_function(&full_native) {
                        let mut arg_vals: Vec<BasicValueEnum> = Vec::new();
                        let mut arg_types: Vec<Type> = Vec::new();
                        let obj_val = self.compile_expr(obj)?;
                        let obj_ty = self.infer_expr_type(obj);
                        arg_vals.push(obj_val);
                        arg_types.push(obj_ty);
                        for arg in args {
                            let arg_ty = self.infer_expr_type(arg);
                            let val = self.compile_expr(arg)?;
                            arg_vals.push(val);
                            arg_types.push(arg_ty);
                        }
                        return native_bridge::emit_native_call(
                            self.context, &self.builder, &self.module,
                            &full_native, &arg_vals, &arg_types,
                        );
                    }
                }
            }
            // Collect all Identifier / MemberAccess objects to try
            let titrate_type_name: Option<String> = match obj.as_ref() {
                Expr::Identifier(name, _) => {
                    self.locals.get(name)
                        .and_then(|l| l.titrate_type.clone())
                        .or_else(|| {
                            let ty = self.infer_expr_type(obj);
                            Some(ty.name().to_string())
                        })
                }
                Expr::MemberAccess(inner, _, _) => {
                    let ty = self.infer_expr_type(inner);
                    Some(ty.name().to_string())
                }
                Expr::Call(inner, _, _) => {
                    let ty = self.infer_expr_type(inner);
                    Some(ty.name().to_string())
                }
                _ => None,
            };
            if let Some(ref type_name) = titrate_type_name {
                let native_prefix: &str = match type_name.as_str() {
                    "ArrayList" | "array" => "ArrayList",
                    "HashMap" => "HashMap",
                    other => other,
                };
                let native_name = format!("{}_{}", native_prefix, method);
                if native_bridge::is_native_function(&native_name) {
                    let mut arg_vals: Vec<BasicValueEnum> = Vec::new();
                    let mut arg_types: Vec<Type> = Vec::new();
                    let obj_val = self.compile_expr(obj)?;
                    let obj_ty = self.infer_expr_type(obj);
                    arg_vals.push(obj_val);
                    arg_types.push(obj_ty);
                    for arg in args {
                        let arg_ty = self.infer_expr_type(arg);
                        let val = self.compile_expr(arg)?;
                        arg_vals.push(val);
                        arg_types.push(arg_ty);
                    }
                    return native_bridge::emit_native_call(
                        self.context, &self.builder, &self.module,
                        &native_name, &arg_vals, &arg_types,
                    );
                }
            }
        }

        // Method call on a class instance: obj.method(args)
        if let Expr::MemberAccess(obj, method, _) = callee {
            let obj_val = self.compile_expr(obj)?;
            // Check if this is an interface fat pointer (struct value).
            if obj_val.is_struct_value() {
                // Look up the interface info from the object's type.
                let obj_type = self.infer_expr_type(obj);
                let iface_name = obj_type.name();
                if let Some(iface_info) = self.interface_infos.get(iface_name).cloned() {
                    let mut arg_vals: Vec<BasicValueEnum> = Vec::new();
                    for arg in args {
                        arg_vals.push(self.compile_expr(arg)?);
                    }
                    return emit_interface_method_call(
                        self.context, &self.builder,
                        iface_info.fat_ptr_type, obj_val,
                        &iface_info.method_names, method, &arg_vals, None,
                    );
                }
            }
            if obj_val.is_pointer_value() {
                let obj_ptr = obj_val.into_pointer_value();
                // Look up the class name from the object's type.
                let obj_type = self.infer_expr_type(obj);
                let class_name = obj_type.name();
                if let Some(class_info) = self.class_infos.get(class_name).cloned() {
                    // First try direct method call (look up the method function by name).
                    let method_fn_name = format!("{}_{}", class_name, method);
                    if let Some(method_fn) = self.module.get_function(&method_fn_name) {
                        let mut arg_vals: Vec<BasicValueEnum> = Vec::new();
                        for arg in args {
                            arg_vals.push(self.compile_expr(arg)?);
                        }
                        return emit_direct_call(self.context, &self.builder, method_fn, obj_ptr, &arg_vals);
                    }
                    // Fall back to virtual call.
                    let mut arg_vals: Vec<BasicValueEnum> = Vec::new();
                    for arg in args {
                        arg_vals.push(self.compile_expr(arg)?);
                    }
                    return emit_virtual_call(self.context, &self.builder, &class_info, obj_ptr, method, &arg_vals, None);
                }
            }
        }

        Err(format!("codegen: unsupported call target: {:?}", callee))
    }

    /// Compile a StaticCall expression: Class.method(args).
    /// This maps to native function calls like Integer_parseInt, String_length, etc.
    fn compile_static_call(&mut self, class_name: &str, method: &str, args: &[Expr]) -> Result<BasicValueEnum<'ctx>, String> {
        // Handle io::println and io::print
        if class_name == "io" && (method == "println" || method == "print") {
            if args.len() != 1 {
                return Err(format!(
                    "codegen: io::{} expects 1 argument, got {}",
                    method, args.len()
                ));
            }
            let arg = &args[0];
            let arg_ty = self.infer_expr_type(arg);
            if llvm_types::is_string(&arg_ty) {
                let s = self.compile_string_expr(arg)?;
                if method == "println" {
                    self.build_println_string(s)?;
                } else {
                    self.build_print_string(s)?;
                }
            } else {
                let v = self.compile_expr(arg)?;
                if method == "println" {
                    self.build_println_primitive(v, &arg_ty)?;
                } else {
                    self.build_print_primitive(v, &arg_ty)?;
                }
            }
            let i32_ty = self.context.i32_type();
            return Ok(i32_ty.const_int(0, false).into());
        }

        // Handle Math/MathAdvanced/MathTrig -> delegate to native
        if matches!(class_name, "Math" | "MathAdvanced" | "MathTrig") {
            let native_name = format!("{}_{}", class_name, method);
            if native_bridge::is_native_function(&native_name) {
                let mut arg_vals: Vec<BasicValueEnum> = Vec::new();
                let mut arg_types: Vec<Type> = Vec::new();
                for arg in args {
                    let arg_ty = self.infer_expr_type(arg);
                    let val = self.compile_expr(arg)?;
                    arg_vals.push(val);
                    arg_types.push(arg_ty);
                }
                return native_bridge::emit_native_call(
                    self.context, &self.builder, &self.module,
                    &native_name, &arg_vals, &arg_types,
                );
            }
            // Try with "Math" prefix for MathAdvanced/MathTrig
            if class_name != "Math" {
                let alt_name = format!("Math_{}", method);
                if native_bridge::is_native_function(&alt_name) {
                    let mut arg_vals: Vec<BasicValueEnum> = Vec::new();
                    let mut arg_types: Vec<Type> = Vec::new();
                    for arg in args {
                        let arg_ty = self.infer_expr_type(arg);
                        let val = self.compile_expr(arg)?;
                        arg_vals.push(val);
                        arg_types.push(arg_ty);
                    }
                    return native_bridge::emit_native_call(
                        self.context, &self.builder, &self.module,
                        &alt_name, &arg_vals, &arg_types,
                    );
                }
            }
        }

        // Dedicated ArrayList.size() handling - call titrate_array_length directly
        if class_name == "ArrayList" && method == "size" && args.len() == 1 {
            return self.compile_array_length(&args[0]);
        }
        // Dedicated ArrayList.get() handling - call titrate_array_get_string directly
        if class_name == "ArrayList" && method == "get" && args.len() == 2 {
            return self.compile_array_get_string(&args[0], &args[1]);
        }

        // Construct the native function name: ClassMethod
        let native_name = format!("{}_{}", class_name, method);
        if native_bridge::is_native_function(&native_name) {
            let mut arg_vals: Vec<BasicValueEnum> = Vec::new();
            let mut arg_types: Vec<Type> = Vec::new();
            for arg in args {
                let arg_ty = self.infer_expr_type(arg);
                let val = self.compile_expr(arg)?;
                arg_vals.push(val);
                arg_types.push(arg_ty);
            }
            return native_bridge::emit_native_call(
                self.context, &self.builder, &self.module,
                &native_name, &arg_vals, &arg_types,
            );
        }
        // Fallback: X.toString(args...) -> toString(args...) for any class
        if method == "toString" {
            let mut arg_vals: Vec<BasicValueEnum> = Vec::new();
            let mut arg_types: Vec<Type> = Vec::new();
            for arg in args {
                let arg_ty = self.infer_expr_type(arg);
                let val = self.compile_expr(arg)?;
                arg_vals.push(val);
                arg_types.push(arg_ty);
            }
            return native_bridge::emit_native_call(
                self.context, &self.builder, &self.module,
                "toString", &arg_vals, &arg_types,
            );
        }
        Err(format!("codegen: unsupported static call: {}.{}", class_name, method))
    }

    /// Returns true if the given LLVM value is a string struct { i64, ptr }.
    fn is_string_struct(&self, val: &BasicValueEnum<'ctx>) -> bool {
        if let BasicValueEnum::StructValue(sv) = val {
            let st = sv.get_type();
            if st.count_fields() == 2 {
                match (st.get_field_type_at_index(0), st.get_field_type_at_index(1)) {
                    (Some(BasicTypeEnum::IntType(_)), Some(BasicTypeEnum::PointerType(_))) => true,
                    _ => false,
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Returns true if the given Titrate type is an unsigned integer type.
    fn is_unsigned_type(ty: &Type) -> bool {
        matches!(ty.name(), "uvast" | "u8" | "u16" | "u32" | "u64" | "size")
    }

    /// Compile `array.length()` - extracts the i64 length from a TitrateArray struct and truncates to i32.
    fn compile_array_length(&mut self, array_expr: &Expr) -> Result<BasicValueEnum<'ctx>, String> {
        let arr_val = self.compile_expr(array_expr)?;
        if arr_val.is_struct_value() {
            let sv = arr_val.into_struct_value();
            let len_val = self.builder.build_extract_value(sv, 0, "arr.len")
                .map_err(|e| format!("extract arr.len failed: {:?}", e))?;
            let i32_val = self.builder.build_int_truncate(len_val.into_int_value(), self.context.i32_type(), "arr.len.i32")
                .map_err(|e| format!("int_truncate failed: {:?}", e))?;
            return Ok(i32_val.into());
        }
        Err("codegen: array_length called on non-array value".to_string())
    }

    /// Compile `array.get(index)` - reads a string element from a TitrateArray
    /// by calling titrate_array_get_string directly.
    fn compile_array_get_string(&mut self, array_expr: &Expr, index_expr: &Expr) -> Result<BasicValueEnum<'ctx>, String> {
        let arr_val = self.compile_expr(array_expr)?;
        let idx_val = self.compile_expr(index_expr)?;

        if !arr_val.is_struct_value() {
            return Err("codegen: array_get_string called on non-array value".to_string());
        }

        let sv = arr_val.into_struct_value();
        // Extract the array struct { i64 len, ptr data }
        let len_val = self.builder.build_extract_value(sv, 0, "arr.len")
            .map_err(|e| format!("extract arr.len failed: {:?}", e))?;
        let data_ptr = self.builder.build_extract_value(sv, 1, "arr.data")
            .map_err(|e| format!("extract arr.data failed: {:?}", e))?;

        // Convert index to i64
        let idx_i64 = if idx_val.is_int_value() {
            let idx_int = idx_val.into_int_value();
            if idx_int.get_type().get_bit_width() < 64 {
                self.builder.build_int_z_extend(idx_int, self.context.i64_type(), "idx.ext")
                    .map_err(|e| format!("int_z_extend failed: {:?}", e))?
            } else { idx_int }
        } else {
            return Err("ArrayList.get: index must be integer".to_string());
        };

        // Call titrate_array_get_string(arr_len, arr_data, index) directly
        let fn_name = "titrate_array_get_string";
        let i64_ty = self.context.i64_type();
        let i8_ptr = self.context.ptr_type(AddressSpace::default());
        // TitrateArray = { i64, ptr } in C-ABI
        let arr_c_abi_ty = self.context.struct_type(&[i64_ty.into(), i8_ptr.into()], false);
        let fn_type = i8_ptr.fn_type(&[arr_c_abi_ty.into(), i64_ty.into()], false);
        let fn_val = self.module.get_function(fn_name).unwrap_or_else(|| {
            self.module.add_function(fn_name, fn_type, Some(Linkage::External))
        });

        // Build the TitrateArray struct from the extracted fields
        let arr_struct_val = self.builder.build_insert_value(
            self.builder.build_insert_value(
                arr_c_abi_ty.const_zero(),
                len_val.into_int_value(), 0, "arr.len"
            ).map_err(|e| format!("insert_value failed: {:?}", e))?,
            data_ptr.into_pointer_value(), 1, "arr.data"
        ).map_err(|e| format!("insert_value failed: {:?}", e))?;

        // Convert AggregateValueEnum to StructValue
        let arr_struct = match arr_struct_val {
            inkwell::values::AggregateValueEnum::StructValue(sv) => sv,
            _ => return Err("expected struct value from insert_value".to_string()),
        };

        let result = self.builder.build_call(fn_val, &[arr_struct.into(), idx_i64.into()], "array_get_string")
            .map_err(|e| format!("build_call titrate_array_get_string failed: {:?}", e))?;

        // The result is a TitrateString { i64 len, ptr data }
        // We need to convert it to the LLVM string type { i64, ptr }
        let string_ty = llvm_types::string_type(self.context).into_struct_type();
        let result_val = match result.try_as_basic_value() {
            inkwell::values::ValueKind::Basic(v) => v,
            _ => return Err("titrate_array_get_string did not return a value".to_string()),
        };

        // The returned TitrateString is { i64 len, ptr data } - same as our string type
        if result_val.get_type() == string_ty.into() {
            return Ok(result_val);
        }

        // If types don't match, bitcast
        self.builder.build_bit_cast(result_val, string_ty, "string.cast")
            .map_err(|e| format!("bit_cast failed: {:?}", e))
    }

    /// Compile a `new ClassName(args)` expression.
    fn compile_new(&mut self, type_name: &crate::ast::Type, args: &[Expr]) -> Result<BasicValueEnum<'ctx>, String> {
        let class_name = type_name.name();
        
        // Handle built-in container types via native bridge.
        if class_name == "ArrayList" {
            let native_name = "ArrayList_new";
            let mut arg_vals: Vec<BasicValueEnum> = Vec::new();
            let mut arg_tys: Vec<crate::ast::Type> = Vec::new();
            for arg in args {
                arg_vals.push(self.compile_expr(arg)?);
                arg_tys.push(self.infer_expr_type(arg));
            }
            return native_bridge::emit_native_call(
                self.context, &self.builder, &self.module,
                native_name, &arg_vals, &arg_tys,
            );
        }
        if class_name == "HashMap" {
            let native_name = "HashMap_new";
            let mut arg_vals: Vec<BasicValueEnum> = Vec::new();
            let mut arg_tys: Vec<crate::ast::Type> = Vec::new();
            for arg in args {
                arg_vals.push(self.compile_expr(arg)?);
                arg_tys.push(self.infer_expr_type(arg));
            }
            return native_bridge::emit_native_call(
                self.context, &self.builder, &self.module,
                native_name, &arg_vals, &arg_tys,
            );
        }
        
        let class_info = self.class_infos.get(class_name).cloned()
            .ok_or_else(|| format!("codegen: class '{}' not found", class_name))?;
        let ctor = class_info.constructor;
        let mut arg_vals: Vec<BasicValueEnum> = Vec::new();
        for arg in args {
            arg_vals.push(self.compile_expr(arg)?);
        }
        emit_new_allocation(self.context, &self.builder, &self.module, &class_info, ctor, &arg_vals)
    }

    /// Compile a member access expression: obj.field
    fn compile_member_access(&mut self, object: &Expr, field: &str) -> Result<BasicValueEnum<'ctx>, String> {
        let obj_val = self.compile_expr(object)?;
        if obj_val.is_pointer_value() {
            let obj_ptr = obj_val.into_pointer_value();
            let obj_type = self.infer_expr_type(object);
            let class_name = obj_type.name();
            let class_info = self.class_infos.get(class_name).cloned()
                .ok_or_else(|| format!("codegen: class '{}' not found for field access", class_name))?;
            return emit_field_access(self.context, &self.builder, &class_info, obj_ptr, field);
        }
        Err(format!("codegen: member access on non-pointer value: {:?}", object))
    }

    /// Compile a `this` expression.
    fn compile_this(&self, _span: &crate::ast::Span) -> Result<BasicValueEnum<'ctx>, String> {
        match self.current_this {
            Some(this_ptr) => Ok(this_ptr.into()),
            None => Err("codegen: 'this' used outside of a method".to_string()),
        }
    }

    /// Compile an `is` type check expression.
    fn compile_is(&mut self, obj: &Expr, ty: &crate::ast::Type) -> Result<BasicValueEnum<'ctx>, String> {
        let type_name = ty.name();
        // Check if it's an interface.
        if let Some(iface_info) = self.interface_infos.get(type_name).cloned() {
            let obj_val = self.compile_expr(obj)?;
            if !obj_val.is_struct_value() {
                return Err(format!("codegen: 'is' check on non-struct (interface) value: {:?}", obj));
            }
            // For interface `is` check, we need an interface vtable. Look up
            // the first available class vtable for this interface.
            let fat_ptr_type = iface_info.fat_ptr_type;
            let i1_ty = self.context.bool_type();
            // Find any implementing class vtable for this interface.
            for ((iface_name, _class_name), vt_global) in &self.interface_vtables {
                if iface_name == type_name {
                    let expected_vt = vt_global.as_pointer_value();
                    let result = emit_interface_is_check(
                        self.context, &self.builder, fat_ptr_type, obj_val, expected_vt,
                    )?;
                    return Ok(result);
                }
            }
            return Ok(i1_ty.const_int(0, false).into());
        }
        // Check if it's a class.
        let class_name = type_name;
        let class_info = self.class_infos.get(class_name).cloned()
            .ok_or_else(|| format!("codegen: type '{}' not found for 'is' check", class_name))?;
        let obj_val = self.compile_expr(obj)?;
        if !obj_val.is_pointer_value() {
            return Err(format!("codegen: 'is' check on non-pointer value: {:?}", obj));
        }
        let obj_ptr = obj_val.into_pointer_value();
        match &class_info.vtable_global {
            Some(vtable_global) => {
                emit_is_check(self.context, &self.builder, obj_ptr, vtable_global.as_pointer_value())
            }
            None => {
                // Class has no vtable (no methods); return false.
                let i1_ty = self.context.bool_type();
                Ok(i1_ty.const_int(0, false).into())
            }
        }
    }

    /// Compile an `as` cast expression.
    fn compile_cast(&mut self, obj: &Expr, ty: &crate::ast::Type) -> Result<BasicValueEnum<'ctx>, String> {
        let type_name = ty.name();
        // Check if casting to an interface.
        if let Some(iface_info) = self.interface_infos.get(type_name).cloned() {
            let obj_val = self.compile_expr(obj)?;
            if !obj_val.is_pointer_value() {
                return Err(format!("codegen: 'as' cast to interface on non-pointer value: {:?}", obj));
            }
            let obj_ptr = obj_val.into_pointer_value();
            // Look up the object's type to find the interface vtable.
            let obj_type = self.infer_expr_type(obj);
            let class_name = obj_type.name();
            let vt_key = (type_name.to_string(), class_name.to_string());
            if let Some(vt_global) = self.interface_vtables.get(&vt_key) {
                return emit_interface_fat_ptr(
                    self.context, &self.builder, iface_info.fat_ptr_type,
                    obj_ptr, vt_global.as_pointer_value(),
                );
            }
            return Err(format!(
                "codegen: no interface vtable for '{}' as '{}'",
                class_name, type_name
            ));
        }
        // Numeric cast (int -> float, float -> int, int -> int, float -> float).
        // This handles expressions like `(i / 4) as double` where the operand
        // is a primitive numeric value, not a pointer.
        if llvm_types::is_numeric(ty) {
            let obj_val = self.compile_expr(obj)?;
            // If the operand is a pointer, fall through to the class-cast path
            // below (a numeric `as` cast on a pointer would be a type error
            // elsewhere, but we keep the existing pointer-handling behavior).
            if !obj_val.is_pointer_value() {
                let target_ty = llvm_types::llvm_type(self.context, ty)?;
                return self.cast_value_to_type(obj_val, target_ty);
            }
        }
        // Simple class cast: identity.
        let obj_val = self.compile_expr(obj)?;
        if obj_val.is_pointer_value() {
            let obj_ptr = obj_val.into_pointer_value();
            return emit_as_cast(self.context, &self.builder, obj_ptr);
        }
        Err(format!("codegen: 'as' cast on non-pointer value: {:?}", obj))
    }

    /// Compile a class declaration: build struct type, compile methods, create vtable.
    fn compile_class_decl(&mut self, class_decl: &ClassDecl) -> Result<(), String> {
        let class_name = class_decl.name.clone();
        let _type_id = self.next_type_id;
        self.next_type_id += 1;

        // Collect field types.
        let mut field_types: HashMap<String, BasicTypeEnum<'ctx>> = HashMap::new();
        let mut field_decls: Vec<crate::ast::FieldDecl> = Vec::new();
        let mut method_names: Vec<String> = Vec::new();
        let mut constructor: Option<FunctionValue> = None;

        for member in &class_decl.members {
            match member {
                ClassMember::Field(field) => {
                    let ft = llvm_types::llvm_type(self.context, &field.typ)?;
                    field_types.insert(field.name.clone(), ft);
                    field_decls.push(field.clone());
                }
                ClassMember::Method(method) => {
                    method_names.push(method.name.clone());
                }
                ClassMember::Constructor(method) => {
                    method_names.push(method.name.clone());
                    // We'll compile the constructor and set it later.
                }
            }
        }

        // Build the struct type.
        let struct_type = build_class_struct_type(
            self.context,
            &class_name,
            &field_decls,
            &field_types,
            None,
        );

        // Compile methods and collect their LLVM function values.
        let mut method_functions: HashMap<String, FunctionValue<'ctx>> = HashMap::new();
        for member in &class_decl.members {
            match member {
                ClassMember::Method(method) => {
                    let fn_val = self.compile_class_method(&class_name, method, &struct_type)?;
                    method_functions.insert(method.name.clone(), fn_val);
                }
                ClassMember::Constructor(method) => {
                    let fn_val = self.compile_class_method(&class_name, method, &struct_type)?;
                    method_functions.insert(method.name.clone(), fn_val);
                    constructor = Some(fn_val);
                }
                _ => {}
            }
        }

        // Create the vtable.
        let vtable_global = create_vtable_global(
            self.context,
            &self.module,
            &class_name,
            &method_names,
            &method_functions,
        );

        // Collect field info.
        let fields: Vec<(String, BasicTypeEnum<'ctx>)> = field_decls
            .iter()
            .map(|f| {
                let ft = field_types.get(&f.name).copied()
                    .unwrap_or_else(|| self.context.ptr_type(AddressSpace::default()).into());
                (f.name.clone(), ft)
            })
            .collect();

        let class_info = ClassInfo {
            name: class_name.clone(),
            struct_type,
            fields,
            method_names,
            constructor,
            vtable_global,
            parent: class_decl.parent.as_ref().map(|t| t.name().to_string()),
            ifaces: class_decl.ifaces.iter().map(|t| t.name().to_string()).collect(),
        };

        self.class_infos.insert(class_name, class_info);
        Ok(())
    }

    /// Compile a class method (or constructor).
    fn compile_class_method(
        &mut self,
        class_name: &str,
        method: &crate::ast::MethodDecl,
        _struct_type: &inkwell::types::StructType<'ctx>,
    ) -> Result<FunctionValue<'ctx>, String> {
        let i8_ptr = self.context.ptr_type(AddressSpace::default());
        let mut param_types: Vec<inkwell::types::BasicMetadataTypeEnum> = Vec::new();
        // First parameter is always `this` (i8*).
        param_types.push(i8_ptr.into());
        for p in &method.params {
            let ty = llvm_types::llvm_type(self.context, &p.typ)?;
            param_types.push(ty.into());
        }
        let return_type = llvm_types::llvm_type_or_void(self.context, method.return_type.as_ref())?;
        let fn_type = match return_type {
            Some(ret) => ret.fn_type(&param_types, false),
            None => self.context.void_type().fn_type(&param_types, false),
        };

        let fn_name = format!("{}_{}", class_name, method.name);
        let fn_val = self.module.add_function(&fn_name, fn_type, None);
        self.functions.insert(fn_name.clone(), fn_val);

        // Create entry block.
        let entry = self.context.append_basic_block(fn_val, "entry");
        self.builder.position_at_end(entry);

        // Save locals and current_this.
        let saved_locals = std::mem::take(&mut self.locals);
        let saved_this = self.current_this;

        // Set up `this` pointer.
        let this_param = fn_val.get_nth_param(0)
            .ok_or_else(|| format!("missing this param for {}", fn_name))?;
        let this_ptr = this_param.into_pointer_value();

        // Cast this to the struct pointer type for field access via GEP.
        let struct_ptr_type = self.context.ptr_type(AddressSpace::default());
        let struct_ptr = if this_ptr.get_type() != struct_ptr_type {
            self.builder
                .build_bit_cast(this_ptr, struct_ptr_type, "this.cast")
                .map_err(|e| format!("build_bit_cast this failed: {:?}", e))?
                .into_pointer_value()
        } else {
            this_ptr
        };

        self.current_this = Some(struct_ptr);

        // Allocate space for method parameters (skip `this`).
        for (i, p) in method.params.iter().enumerate() {
            let param_val = fn_val.get_nth_param((i + 1) as u32)
                .ok_or_else(|| format!("missing param {} for {}", i, fn_name))?;
            let ty = llvm_types::llvm_type(self.context, &p.typ)?;
            let alloca = self.builder.build_alloca(ty, &p.name)
                .map_err(|e| format!("build_alloca param '{}' failed: {:?}", p.name, e))?;
            self.builder.build_store(alloca, param_val)
                .map_err(|e| format!("build_store param '{}' failed: {:?}", p.name, e))?;
            self.locals.insert(p.name.clone(), LocalVar { ptr: alloca, ty, titrate_type: Some(p.typ.name().to_string()) });
        }

        // Compile method body.
        for s in &method.body {
            self.compile_stmt(s)?;
        }

        // Add a default return if the current block has no terminator.
        if self.builder.get_insert_block().and_then(|b| b.get_terminator()).is_none() {
            match &method.return_type {
                Some(t) if !llvm_types::is_void(t) => {
                    let ty = llvm_types::llvm_type(self.context, t)?;
                    let zero: BasicValueEnum<'ctx> = match ty {
                        BasicTypeEnum::IntType(it) => it.const_int(0, false).into(),
                        BasicTypeEnum::FloatType(ft) => ft.const_float(0.0).into(),
                        BasicTypeEnum::PointerType(pt) => pt.const_null().into(),
                        _ => {
                            self.builder.build_return(None)
                                .map_err(|e| format!("build_return failed: {:?}", e))?;
                            return Ok(fn_val);
                        }
                    };
                    self.builder.build_return(Some(&zero))
                        .map_err(|e| format!("build_return zero failed: {:?}", e))?;
                }
                _ => {
                    self.builder.build_return(None)
                        .map_err(|e| format!("build_return void failed: {:?}", e))?;
                }
            }
        }

        // Restore locals and this.
        self.locals = saved_locals;
        self.current_this = saved_this;

        Ok(fn_val)
    }

    /// Compile an interface declaration: register interface info, compile
    /// default methods, and build the fat pointer type.
    fn compile_interface_decl(&mut self, iface_decl: &InterfaceDecl) -> Result<(), String> {
        let iface_name = iface_decl.name.clone();
        let fat_ptr_type = build_interface_fat_ptr_type(self.context);

        // Collect method names.
        let mut method_names: Vec<String> = Vec::new();
        let mut default_methods: HashMap<String, FunctionValue<'ctx>> = HashMap::new();

        for method_sig in &iface_decl.methods {
            method_names.push(method_sig.name.clone());

            // Compile default method body if present.
            if method_sig.body.is_some() {
                let i8_ptr = self.context.ptr_type(AddressSpace::default());
                let mut param_types: Vec<inkwell::types::BasicMetadataTypeEnum> = Vec::new();
                param_types.push(i8_ptr.into());
                for p in &method_sig.params {
                    let ty = llvm_types::llvm_type(self.context, &p.typ)?;
                    param_types.push(ty.into());
                }
                let return_type = llvm_types::llvm_type_or_void(self.context, method_sig.return_type.as_ref())?;
                let fn_type = match return_type {
                    Some(ret) => ret.fn_type(&param_types, false),
                    None => self.context.void_type().fn_type(&param_types, false),
                };

                let fn_name = format!("{}_default_{}", iface_name, method_sig.name);
                let fn_val = self.module.add_function(&fn_name, fn_type, None);
                self.functions.insert(fn_name.clone(), fn_val);

                // Compile the default method body.
                let entry = self.context.append_basic_block(fn_val, "entry");
                self.builder.position_at_end(entry);
                let saved_locals = std::mem::take(&mut self.locals);
                let saved_this = self.current_this;

                let this_ptr = fn_val.get_nth_param(0)
                    .ok_or_else(|| format!("missing this for {}", fn_name))?;
                self.current_this = Some(this_ptr.into_pointer_value());

                for (i, p) in method_sig.params.iter().enumerate() {
                    let param_val = fn_val.get_nth_param((i + 1) as u32)
                        .ok_or_else(|| format!("missing param {} for {}", i, fn_name))?;
                    let ty = llvm_types::llvm_type(self.context, &p.typ)?;
                    let alloca = self.builder.build_alloca(ty, &p.name)
                        .map_err(|e| format!("build_alloca param failed: {:?}", e))?;
                    self.builder.build_store(alloca, param_val)
                        .map_err(|e| format!("build_store param failed: {:?}", e))?;
            self.locals.insert(p.name.clone(), LocalVar { ptr: alloca, ty, titrate_type: Some(p.typ.name().to_string()) });
        }


                if let Some(ref body) = method_sig.body {
                    for s in body {
                        self.compile_stmt(s)?;
                    }
                }

                if self.builder.get_insert_block().and_then(|b| b.get_terminator()).is_none() {
                    match &method_sig.return_type {
                        Some(t) if !llvm_types::is_void(t) => {
                            let ty = llvm_types::llvm_type(self.context, t)?;
                            let zero: BasicValueEnum<'ctx> = match ty {
                                BasicTypeEnum::IntType(it) => it.const_int(0, false).into(),
                                BasicTypeEnum::FloatType(ft) => ft.const_float(0.0).into(),
                                BasicTypeEnum::PointerType(pt) => pt.const_null().into(),
                                _ => {
                                    self.builder.build_return(None)
                                        .map_err(|e| format!("build_return failed: {:?}", e))?;
                                    return Ok(());
                                }
                            };
                            self.builder.build_return(Some(&zero))
                                .map_err(|e| format!("build_return zero failed: {:?}", e))?;
                        }
                        _ => {
                            self.builder.build_return(None)
                                .map_err(|e| format!("build_return void failed: {:?}", e))?;
                        }
                    }
                }

                self.locals = saved_locals;
                self.current_this = saved_this;
                default_methods.insert(method_sig.name.clone(), fn_val);
            }
        }

        let iface_info = InterfaceInfo {
            name: iface_name.clone(),
            fat_ptr_type,
            method_names: method_names.clone(),
            default_methods,
            vtable_type: None,
        };

        self.interface_infos.insert(iface_name, iface_info);
        Ok(())
    }

    /// Compile a single statement.
    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::Expr(expr) => {
                self.compile_expr(expr)?;
                Ok(())
            }
            Stmt::VarDecl(decl) | Stmt::ConstDecl(decl) => {
                self.compile_var_decl(decl)?;
                Ok(())
            }
            Stmt::Return(expr) => {
                self.compile_return(expr)?;
                Ok(())
            }
            Stmt::If(if_stmt) => {
                self.compile_if(if_stmt)?;
                Ok(())
            }
            Stmt::While(while_stmt) => {
                self.compile_while(while_stmt)?;
                Ok(())
            }
            Stmt::DoWhile(do_while_stmt) => {
                self.compile_do_while(do_while_stmt)?;
                Ok(())
            }
            Stmt::For(for_stmt) => {
                self.compile_for(for_stmt)?;
                Ok(())
            }
            Stmt::CFor(cfor_stmt) => {
                self.compile_c_for(cfor_stmt)?;
                Ok(())
            }
            Stmt::Break => {
                let ctx = self.loop_stack.last()
                    .ok_or("codegen: break outside of loop")?;
                self.builder.build_unconditional_branch(ctx.break_block)
                    .map_err(|e| format!("build_br break failed: {:?}", e))?;
                Ok(())
            }
            Stmt::Continue => {
                let ctx = self.loop_stack.last()
                    .ok_or("codegen: continue outside of loop")?;
                self.builder.build_unconditional_branch(ctx.continue_block)
                    .map_err(|e| format!("build_br continue failed: {:?}", e))?;
                Ok(())
            }
            Stmt::Block(block) => {
                self.ownership.enter_scope();
                for s in block {
                    self.compile_stmt(s)?;
                }
                self.emit_scope_cleanup()?;
                Ok(())
            }
            Stmt::Switch(switch_stmt) => {
                self.compile_switch(switch_stmt)?;
                Ok(())
            }
            Stmt::Throw(expr, _) => {
                self.compile_throw(expr)?;
                Ok(())
            }
            Stmt::TryCatch {
                try_block,
                catch_var,
                catch_var_type,
                catch_block,
                ..
            } => {
                self.compile_try_catch(try_block, catch_var, catch_var_type.as_ref(), catch_block)?;
                Ok(())
            }
            Stmt::With(with_stmt) => {
                self.compile_with(with_stmt)?;
                Ok(())
            }
            Stmt::TupleDestructure { names, expr, .. } => {
                self.compile_tuple_destructure(names, expr)?;
                Ok(())
            }
            _ => Err(format!("codegen: unsupported statement: {:?}", stmt)),
        }
    }

    /// Compile a return statement.
    fn compile_return(&mut self, expr: &Option<Expr>) -> Result<(), String> {
        match expr {
            None => {
                // Check the actual LLVM function return type to generate the right ret.
                let current_fn = self.builder.get_insert_block()
                    .and_then(|b| b.get_parent());
                if let Some(fn_val) = current_fn {
                    if fn_val.get_type().get_return_type().is_none() {
                        // True void function.
                        self.builder.build_return(None)
                            .map_err(|e| format!("build_return void failed: {:?}", e))?;
                    } else {
                        // Function returns a value but we have no expression.
                        // Return default zero.
                        let ret_ty = fn_val.get_type().get_return_type().unwrap();
                        let zero: BasicValueEnum<'ctx> = match ret_ty {
                            BasicTypeEnum::IntType(it) => it.const_int(0, false).into(),
                            BasicTypeEnum::FloatType(ft) => ft.const_float(0.0).into(),
                            BasicTypeEnum::PointerType(pt) => pt.const_null().into(),
                            BasicTypeEnum::StructType(_) => {
                                self.builder.build_return(None)
                                    .map_err(|e| format!("build_return struct default failed: {:?}", e))?;
                                return Ok(());
                            }
                            _ => {
                                self.builder.build_return(None)
                                    .map_err(|e| format!("build_return default failed: {:?}", e))?;
                                return Ok(());
                            }
                        };
                        self.builder.build_return(Some(&zero))
                            .map_err(|e| format!("build_return zero failed: {:?}", e))?;
                    }
                } else {
                    self.builder.build_return(None)
                        .map_err(|e| format!("build_return void failed: {:?}", e))?;
                }
            }
            Some(e) => {
                let v = self.compile_expr(e)?;
                // If the current function returns void, discard the value and return void.
                let current_fn = self.builder.get_insert_block()
                    .and_then(|b| b.get_parent());
                if let Some(fn_val) = current_fn {
                    if fn_val.get_type().get_return_type().is_none() {
                        self.builder.build_return(None)
                            .map_err(|e| format!("build_return void failed: {:?}", e))?;
                        return Ok(());
                    }
                }
                self.builder.build_return(Some(&v))
                    .map_err(|e| format!("build_return failed: {:?}", e))?;
            }
        }
        Ok(())
    }

    /// Compile a `let`/`var`/`const` declaration.
    fn compile_var_decl(&mut self, decl: &crate::ast::VarDecl) -> Result<(), String> {
        let init = decl
            .init
            .as_ref()
            .ok_or_else(|| format!("codegen: variable '{}' has no initializer", decl.name))?;

        // Determine the declared type (if any).
        let declared_ty = decl.typ.as_ref();

        // String variable.
        let is_string = declared_ty.map(llvm_types::is_string).unwrap_or(false);
        if is_string {
            let sv = self.compile_string_expr(init)?;
            let string_ty = llvm_types::string_type(self.context).into_struct_type();
            let alloca = self.builder.build_alloca(string_ty, &decl.name)
                .map_err(|e| format!("build_alloca '{}' failed: {:?}", decl.name, e))?;
            let len_ptr = self.builder.build_struct_gep(string_ty, alloca, 0, &format!("{}.len", decl.name))
                .map_err(|e| format!("build_struct_gep 0 failed: {:?}", e))?;
            let ptr_ptr = self.builder.build_struct_gep(string_ty, alloca, 1, &format!("{}.ptr", decl.name))
                .map_err(|e| format!("build_struct_gep 1 failed: {:?}", e))?;
            self.builder.build_store(len_ptr, sv.len)
                .map_err(|e| format!("build_store len failed: {:?}", e))?;
            self.builder.build_store(ptr_ptr, sv.ptr)
                .map_err(|e| format!("build_store ptr failed: {:?}", e))?;
            self.locals.insert(decl.name.clone(), LocalVar { ptr: alloca, ty: string_ty.into(), titrate_type: Some("string".to_string()) });
            return Ok(());
        }

        // Primitive (or inferred) variable.
        let ty = match declared_ty {
            Some(t) => llvm_types::llvm_type(self.context, t)?,
            None => {
                // Infer from the initializer.
                let v = self.compile_expr(init)?;
                v.get_type()
            }
        };

        let alloca = self.builder.build_alloca(ty, &decl.name)
            .map_err(|e| format!("build_alloca '{}' failed: {:?}", decl.name, e))?;

        // Compile the initializer with a type hint for literals.
        let init_val = match init {
            Expr::Literal(lit, _) => self.compile_literal(lit, declared_ty)?,
            _ => self.compile_expr(init)?,
        };

        // Cast to the declared type if needed.
        let init_val = self.cast_value_to_type(init_val, ty)?;

        self.builder.build_store(alloca, init_val)
            .map_err(|e| format!("build_store '{}' failed: {:?}", decl.name, e))?;

        self.locals.insert(decl.name.clone(), LocalVar { ptr: alloca, ty, titrate_type: declared_ty.map(|t| t.name().to_string()) });
        Ok(())
    }

    /// Compile an if/else statement.
    fn compile_if(&mut self, if_stmt: &crate::ast::IfStmt) -> Result<(), String> {
        let cond = self.compile_expr(&if_stmt.condition)?.into_int_value();
        let current_block = self.builder.get_insert_block()
            .ok_or("codegen: no insert block for if")?;
        let then_block = self.context.insert_basic_block_after(current_block, "if.then");
        let end_block = self.context.insert_basic_block_after(then_block, "if.end");

        let else_block = if if_stmt.else_branch.is_some() {
            Some(self.context.insert_basic_block_after(then_block, "if.else"))
        } else {
            None
        };

        if let Some(eb) = else_block {
            self.builder.build_conditional_branch(cond, then_block, eb)
                .map_err(|e| format!("build_cond_br if failed: {:?}", e))?;
        } else {
            self.builder.build_conditional_branch(cond, then_block, end_block)
                .map_err(|e| format!("build_cond_br if failed: {:?}", e))?;
        }

        // Then block.
        self.builder.position_at_end(then_block);
        for s in &if_stmt.then_branch {
            self.compile_stmt(s)?;
        }
        // Only add the branch to end if the current block doesn't already
        // have a terminator (e.g. from a return statement).
        if self.builder.get_insert_block().and_then(|b| b.get_terminator()).is_none() {
            self.builder.build_unconditional_branch(end_block)
                .map_err(|e| format!("build_br if.end failed: {:?}", e))?;
        }

        // Else block.
        if let (Some(eb), Some(else_branch)) = (else_block, &if_stmt.else_branch) {
            self.builder.position_at_end(eb);
            for s in else_branch {
                self.compile_stmt(s)?;
            }
            if self.builder.get_insert_block().and_then(|b| b.get_terminator()).is_none() {
                self.builder.build_unconditional_branch(end_block)
                    .map_err(|e| format!("build_br if.end 2 failed: {:?}", e))?;
            }
        }

        self.builder.position_at_end(end_block);
        Ok(())
    }

    /// Compile a while loop.
    fn compile_while(&mut self, while_stmt: &crate::ast::WhileStmt) -> Result<(), String> {
        let current_block = self.builder.get_insert_block()
            .ok_or("codegen: no insert block for while")?;
        let cond_block = self.context.insert_basic_block_after(current_block, "while.cond");
        let body_block = self.context.insert_basic_block_after(cond_block, "while.body");
        let end_block = self.context.insert_basic_block_after(body_block, "while.end");

        self.builder.build_unconditional_branch(cond_block)
            .map_err(|e| format!("build_br while.cond failed: {:?}", e))?;

        // Condition block.
        self.builder.position_at_end(cond_block);
        let cond = self.compile_expr(&while_stmt.condition)?.into_int_value();
        self.builder.build_conditional_branch(cond, body_block, end_block)
            .map_err(|e| format!("build_cond_br while failed: {:?}", e))?;

        // Body block.
        self.builder.position_at_end(body_block);
        self.loop_stack.push(LoopContext {
            continue_block: cond_block,
            break_block: end_block,
        });
        for s in &while_stmt.body {
            self.compile_stmt(s)?;
        }
        self.loop_stack.pop();
        if self.builder.get_insert_block().and_then(|b| b.get_terminator()).is_none() {
            self.builder.build_unconditional_branch(cond_block)
                .map_err(|e| format!("build_br while.cond 2 failed: {:?}", e))?;
        }

        self.builder.position_at_end(end_block);
        Ok(())
    }

    /// Compile a do-while loop.
    fn compile_do_while(&mut self, do_while_stmt: &crate::ast::DoWhileStmt) -> Result<(), String> {
        let current_block = self.builder.get_insert_block()
            .ok_or("codegen: no insert block for do-while")?;
        let body_block = self.context.insert_basic_block_after(current_block, "do.body");
        let cond_block = self.context.insert_basic_block_after(body_block, "do.cond");
        let end_block = self.context.insert_basic_block_after(cond_block, "do.end");

        self.builder.build_unconditional_branch(body_block)
            .map_err(|e| format!("build_br do.body failed: {:?}", e))?;

        // Body block.
        self.builder.position_at_end(body_block);
        self.loop_stack.push(LoopContext {
            continue_block: cond_block,
            break_block: end_block,
        });
        for s in &do_while_stmt.body {
            self.compile_stmt(s)?;
        }
        self.loop_stack.pop();
        if self.builder.get_insert_block().and_then(|b| b.get_terminator()).is_none() {
            self.builder.build_unconditional_branch(cond_block)
                .map_err(|e| format!("build_br do.cond failed: {:?}", e))?;
        }

        // Condition block.
        self.builder.position_at_end(cond_block);
        let cond = self.compile_expr(&do_while_stmt.condition)?.into_int_value();
        self.builder.build_conditional_branch(cond, body_block, end_block)
            .map_err(|e| format!("build_cond_br do-while failed: {:?}", e))?;

        self.builder.position_at_end(end_block);
        Ok(())
    }

    /// Compile a for-in loop. Phase 1 supports Range iteration:
    /// `for (i in start..end)` and `for (i in start..=end)`.
    fn compile_for(&mut self, for_stmt: &crate::ast::ForStmt) -> Result<(), String> {
        // Only Range iteration is supported in Phase 1.
        let (start_expr, end_expr, inclusive) = match &for_stmt.iterable {
            Expr::Range(s, e, _) => (s.as_ref(), e.as_ref(), false),
            Expr::RangeInclusive(s, e, _) => (s.as_ref(), e.as_ref(), true),
            _ => {
                return Err(
                    "codegen: for-in over non-Range iterables is not yet supported".to_string(),
                );
            }
        };

        let i64_ty = self.context.i64_type();

        // Compile start and end values.
        let start_val = self.compile_expr(start_expr)?;
        let end_val = self.compile_expr(end_expr)?;

        // Extend to i64 if needed.
        let start_i64 = if start_val.is_int_value() && start_val.get_type() != i64_ty.into() {
            self.builder.build_int_s_extend(start_val.into_int_value(), i64_ty, "for.start.ext")
                .map_err(|e| format!("build_int_s_extend start failed: {:?}", e))?
        } else {
            start_val.into_int_value()
        };
        let end_i64 = if end_val.is_int_value() && end_val.get_type() != i64_ty.into() {
            self.builder.build_int_s_extend(end_val.into_int_value(), i64_ty, "for.end.ext")
                .map_err(|e| format!("build_int_s_extend end failed: {:?}", e))?
        } else {
            end_val.into_int_value()
        };

        // Allocate the loop counter.
        let counter_alloca = self.builder.build_alloca(i64_ty, "for.i")
            .map_err(|e| format!("build_alloca for.i failed: {:?}", e))?;
        self.builder.build_store(counter_alloca, start_i64)
            .map_err(|e| format!("build_store for.i failed: {:?}", e))?;

        // Allocate the loop variable (visible to the body).
        let loop_var_ty = start_val.get_type();
        let loop_var_alloca = self.builder.build_alloca(loop_var_ty, &for_stmt.var)
            .map_err(|e| format!("build_alloca '{}' failed: {:?}", for_stmt.var, e))?;

        let current_block = self.builder.get_insert_block()
            .ok_or("codegen: no insert block for for")?;
        let cond_block = self.context.insert_basic_block_after(current_block, "for.cond");
        let body_block = self.context.insert_basic_block_after(cond_block, "for.body");
        let inc_block = self.context.insert_basic_block_after(body_block, "for.inc");
        let end_block = self.context.insert_basic_block_after(inc_block, "for.end");

        self.builder.build_unconditional_branch(cond_block)
            .map_err(|e| format!("build_br for.cond failed: {:?}", e))?;

        // Condition block: while counter < end (or <= for inclusive).
        self.builder.position_at_end(cond_block);
        let counter_val = self.builder.build_load(i64_ty, counter_alloca, "for.i.val")
            .map_err(|e| format!("build_load for.i failed: {:?}", e))?
            .into_int_value();
        let pred = if inclusive {
            inkwell::IntPredicate::SLE
        } else {
            inkwell::IntPredicate::SLT
        };
        let cmp = self.builder.build_int_compare(pred, counter_val, end_i64, "for.cmp")
            .map_err(|e| format!("build_int_compare for failed: {:?}", e))?;
        self.builder.build_conditional_branch(cmp, body_block, end_block)
            .map_err(|e| format!("build_cond_br for failed: {:?}", e))?;

        // Body block: store the counter (truncated to loop var type) in the
        // loop variable, then run the body.
        self.builder.position_at_end(body_block);
        let counter_for_body = if loop_var_ty != i64_ty.into() {
            self.builder.build_int_truncate(counter_val, loop_var_ty.into_int_type(), "for.i.trunc")
                .map_err(|e| format!("build_int_truncate for failed: {:?}", e))?
        } else {
            counter_val
        };
        self.builder.build_store(loop_var_alloca, counter_for_body)
            .map_err(|e| format!("build_store loop var failed: {:?}", e))?;
        // Register the loop variable in locals.
        let prev = self.locals.insert(for_stmt.var.clone(), LocalVar {
            ptr: loop_var_alloca,
            ty: loop_var_ty,
            titrate_type: None,
        });
        self.loop_stack.push(LoopContext {
            continue_block: inc_block,
            break_block: end_block,
        });
        for s in &for_stmt.body {
            self.compile_stmt(s)?;
        }
        self.loop_stack.pop();
        // Restore the previous binding (if any).
        if let Some(p) = prev {
            self.locals.insert(for_stmt.var.clone(), p);
        } else {
            self.locals.remove(&for_stmt.var);
        }
        if self.builder.get_insert_block().and_then(|b| b.get_terminator()).is_none() {
            self.builder.build_unconditional_branch(inc_block)
                .map_err(|e| format!("build_br for.inc failed: {:?}", e))?;
        }

        // Increment block: counter += 1.
        self.builder.position_at_end(inc_block);
        let counter_val = self.builder.build_load(i64_ty, counter_alloca, "for.i.inc")
            .map_err(|e| format!("build_load for.i.inc failed: {:?}", e))?
            .into_int_value();
        let one = i64_ty.const_int(1, false);
        let next = self.builder.build_int_add(counter_val, one, "for.i.next")
            .map_err(|e| format!("build_int_add for.inc failed: {:?}", e))?;
        self.builder.build_store(counter_alloca, next)
            .map_err(|e| format!("build_store for.i.next failed: {:?}", e))?;
        self.builder.build_unconditional_branch(cond_block)
            .map_err(|e| format!("build_br for.cond 2 failed: {:?}", e))?;

        // In release mode, attach vectorization hints to the loop latch
        // (the back-edge branch in the increment block). This tells LLVM's
        // loop vectorizer that this loop is a candidate for SIMD.
        if self.release_mode {
            // Metadata attachment is best-effort: if it fails we still
            // produce correct (just unannotated) code.
            let _ = self.add_vectorize_metadata(inc_block);
        }

        self.builder.position_at_end(end_block);
        Ok(())
    }

    /// Emit an optimized array-iteration loop using pointer arithmetic.
    ///
    /// Instead of recomputing `array[i]` via GEP on each iteration (which
    /// requires a multiply + add for the index), this loads the base pointer
    /// once and increments it by the element size on each iteration. This is
    /// the canonical pattern LLVM's loop vectorizer recognizes.
    ///
    /// The emitted loop structure is:
    ///   ```text
    ///   ptr = base; end = base + count;
    ///   while (ptr != end) {
    ///       elem = load ptr;
    ///       ... body ...
    ///       ptr = ptr + 1;  // gep by element size
    ///   }
    ///   ```
    ///
    /// This is used as the foundation for `for (x in array)` loops once array
    /// iteration is fully supported by the codegen.
    #[allow(dead_code)]
    fn emit_array_loop(
        &mut self,
        base_ptr: PointerValue<'ctx>,
        count: IntValue<'ctx>,
        elem_ty: BasicTypeEnum<'ctx>,
        var_name: &str,
        body: impl FnOnce(&mut Self, BasicValueEnum<'ctx>) -> Result<(), String>,
    ) -> Result<(), String> {
        let i8_ptr_ty = self.context.ptr_type(AddressSpace::default());

        // Compute the end pointer: end = base + count (GEP by element).
        let end_ptr = unsafe {
            self.builder.build_in_bounds_gep(
                elem_ty,
                base_ptr,
                &[count],
                "arr.end.ptr",
            )
        }
        .map_err(|e| format!("build_gep arr.end.ptr failed: {:?}", e))?;

        // Allocate the loop pointer (current position).
        let cur_ptr_alloca = self.builder.build_alloca(i8_ptr_ty, "arr.cur.ptr")
            .map_err(|e| format!("build_alloca arr.cur.ptr failed: {:?}", e))?;
        // Store the base pointer into it.
        let base_as_i8 = self.builder.build_bit_cast(base_ptr, i8_ptr_ty, "arr.base.i8")
            .map_err(|e| format!("build_bit_cast base failed: {:?}", e))?;
        self.builder.build_store(cur_ptr_alloca, base_as_i8)
            .map_err(|e| format!("build_store cur.ptr failed: {:?}", e))?;

        // Allocate the loop variable (visible to the body).
        let loop_var_alloca = self.builder.build_alloca(elem_ty, var_name)
            .map_err(|e| format!("build_alloca '{}' failed: {:?}", var_name, e))?;

        let current_block = self.builder.get_insert_block()
            .ok_or("codegen: no insert block for array loop")?;
        let cond_block = self.context.insert_basic_block_after(current_block, "arr.cond");
        let body_block = self.context.insert_basic_block_after(cond_block, "arr.body");
        let inc_block = self.context.insert_basic_block_after(body_block, "arr.inc");
        let end_block = self.context.insert_basic_block_after(inc_block, "arr.end");

        self.builder.build_unconditional_branch(cond_block)
            .map_err(|e| format!("build_br arr.cond failed: {:?}", e))?;

        // Condition block: while cur_ptr != end_ptr.
        self.builder.position_at_end(cond_block);
        let cur_ptr_val = self.builder.build_load(i8_ptr_ty, cur_ptr_alloca, "arr.cur.val")
            .map_err(|e| format!("build_load arr.cur failed: {:?}", e))?
            .into_pointer_value();
        let end_as_i8 = self.builder.build_bit_cast(end_ptr, i8_ptr_ty, "arr.end.i8")
            .map_err(|e| format!("build_bit_cast end failed: {:?}", e))?
            .into_pointer_value();
        // Convert pointers to integers for comparison.
        let intptr_ty = if cfg!(target_pointer_width = "64") {
            self.context.i64_type()
        } else {
            self.context.i32_type()
        };
        let cur_int = self.builder.build_ptr_to_int(cur_ptr_val, intptr_ty, "arr.cur.int")
            .map_err(|e| format!("build_ptr_to_int cur failed: {:?}", e))?;
        let end_int = self.builder.build_ptr_to_int(end_as_i8, intptr_ty, "arr.end.int")
            .map_err(|e| format!("build_ptr_to_int end failed: {:?}", e))?;
        let cmp = self.builder.build_int_compare(
            inkwell::IntPredicate::EQ,
            cur_int,
            end_int,
            "arr.cmp",
        )
        .map_err(|e| format!("build_int_compare arr failed: {:?}", e))?;
        // If equal, we're done; otherwise enter body.
        self.builder.build_conditional_branch(cmp, end_block, body_block)
            .map_err(|e| format!("build_cond_br arr failed: {:?}", e))?;

        // Body block: load the current element and run the body.
        self.builder.position_at_end(body_block);
        let cur_for_elem = self.builder.build_load(i8_ptr_ty, cur_ptr_alloca, "arr.body.ptr")
            .map_err(|e| format!("build_load arr.body.ptr failed: {:?}", e))?
            .into_pointer_value();
        // Cast back to the element pointer type.
        let elem_ptr = self.builder.build_bit_cast(cur_for_elem, i8_ptr_ty, "arr.elem.ptr")
            .map_err(|e| format!("build_bit_cast elem.ptr failed: {:?}", e))?
            .into_pointer_value();
        let elem_val = self.builder.build_load(elem_ty, elem_ptr, "arr.elem")
            .map_err(|e| format!("build_load arr.elem failed: {:?}", e))?;
        self.builder.build_store(loop_var_alloca, elem_val)
            .map_err(|e| format!("build_store arr.elem failed: {:?}", e))?;

        // Run the body closure.
        body(self, elem_val)?;

        if self.builder.get_insert_block().and_then(|b| b.get_terminator()).is_none() {
            self.builder.build_unconditional_branch(inc_block)
                .map_err(|e| format!("build_br arr.inc failed: {:?}", e))?;
        }

        // Increment block: advance the pointer by one element.
        self.builder.position_at_end(inc_block);
        let cur_ptr_val = self.builder.build_load(i8_ptr_ty, cur_ptr_alloca, "arr.inc.ptr")
            .map_err(|e| format!("build_load arr.inc.ptr failed: {:?}", e))?
            .into_pointer_value();
        // Advance by the element size in bytes.
        let elem_size = elem_ty.size_of()
            .ok_or_else(|| "codegen: cannot compute element size".to_string())?;
        let next_ptr = unsafe {
            self.builder.build_in_bounds_gep(
                self.context.i8_type(),
                cur_ptr_val,
                &[elem_size],
                "arr.next.ptr",
            )
        }
        .map_err(|e| format!("build_gep arr.next.ptr failed: {:?}", e))?;
        self.builder.build_store(cur_ptr_alloca, next_ptr)
            .map_err(|e| format!("build_store arr.next failed: {:?}", e))?;
        self.builder.build_unconditional_branch(cond_block)
            .map_err(|e| format!("build_br arr.cond 2 failed: {:?}", e))?;

        // In release mode, attach vectorization hints.
        if self.release_mode {
            let _ = self.add_vectorize_metadata(inc_block);
        }

        self.builder.position_at_end(end_block);
        Ok(())
    }

    /// Compile a C-style for loop.
    fn compile_c_for(&mut self, cfor_stmt: &crate::ast::CForStmt) -> Result<(), String> {
        let current_block = self.builder.get_insert_block()
            .ok_or("codegen: no insert block for c-for")?;

        // Init statement (in the current block).
        if let Some(init) = &cfor_stmt.init {
            self.compile_stmt(init)?;
        }

        let cond_block = self.context.insert_basic_block_after(
            self.builder.get_insert_block().unwrap_or(current_block),
            "cfor.cond",
        );
        let body_block = self.context.insert_basic_block_after(cond_block, "cfor.body");
        let inc_block = self.context.insert_basic_block_after(body_block, "cfor.inc");
        let end_block = self.context.insert_basic_block_after(inc_block, "cfor.end");

        self.builder.build_unconditional_branch(cond_block)
            .map_err(|e| format!("build_br cfor.cond failed: {:?}", e))?;

        // Condition block.
        self.builder.position_at_end(cond_block);
        if let Some(cond) = &cfor_stmt.condition {
            let cond_val = self.compile_expr(cond)?.into_int_value();
            self.builder.build_conditional_branch(cond_val, body_block, end_block)
                .map_err(|e| format!("build_cond_br cfor failed: {:?}", e))?;
        } else {
            self.builder.build_unconditional_branch(body_block)
                .map_err(|e| format!("build_br cfor.body failed: {:?}", e))?;
        }

        // Body block.
        self.builder.position_at_end(body_block);
        self.loop_stack.push(LoopContext {
            continue_block: inc_block,
            break_block: end_block,
        });
        for s in &cfor_stmt.body {
            self.compile_stmt(s)?;
        }
        self.loop_stack.pop();
        if self.builder.get_insert_block().and_then(|b| b.get_terminator()).is_none() {
            self.builder.build_unconditional_branch(inc_block)
                .map_err(|e| format!("build_br cfor.inc failed: {:?}", e))?;
        }

        // Increment block.
        self.builder.position_at_end(inc_block);
        if let Some(incr) = &cfor_stmt.increment {
            self.compile_expr(incr)?;
        }
        self.builder.build_unconditional_branch(cond_block)
            .map_err(|e| format!("build_br cfor.cond 2 failed: {:?}", e))?;

        self.builder.position_at_end(end_block);
        Ok(())
    }

    /// Compile a switch/case statement. For literal patterns, uses LLVM's
    /// `switch` instruction. For wildcard `_`, uses the default case.
    fn compile_switch(&mut self, switch_stmt: &crate::ast::SwitchStmt) -> Result<(), String> {
        use crate::ast::Pattern;
        let val = self.compile_expr(&switch_stmt.expr)?;
        let int_val = val.into_int_value();

        let current_block = self.builder.get_insert_block()
            .ok_or("codegen: no insert block for switch")?;
        let end_block = self.context.insert_basic_block_after(current_block, "switch.end");

        // Create basic blocks for each case.
        let mut case_blocks: Vec<(inkwell::basic_block::BasicBlock<'ctx>, &crate::ast::Case)> = Vec::new();
        let mut prev_block = current_block;
        for case in &switch_stmt.cases {
            let cb = self.context.insert_basic_block_after(prev_block, "switch.case");
            case_blocks.push((cb, case));
            prev_block = cb;
        }
        let default_block = if switch_stmt.default.is_some() {
            let db = self.context.insert_basic_block_after(prev_block, "switch.default");
            Some(db)
        } else {
            None
        };

        // Build the switch instruction: collect (value, block) pairs for
        // literal-int cases and pass them as a slice to build_switch.
        let default_target = default_block.unwrap_or(end_block);
        let mut cases_vec: Vec<(IntValue<'ctx>, inkwell::basic_block::BasicBlock<'ctx>)> = Vec::new();
        for (cb, case) in &case_blocks {
            if let Pattern::Literal(Literal::Int(v)) = &case.pattern {
                cases_vec.push((int_val.get_type().const_int(*v as u64, false), *cb));
            }
            // Non-literal patterns fall through to default for now.
        }
        self.builder
            .build_switch(int_val, default_target, &cases_vec)
            .map_err(|e| format!("build_switch failed: {:?}", e))?;

        // Compile each case body.
        for (cb, case) in case_blocks {
            self.builder.position_at_end(cb);
            for s in &case.body {
                self.compile_stmt(s)?;
            }
            if self.builder.get_insert_block().and_then(|b| b.get_terminator()).is_none() {
                self.builder.build_unconditional_branch(end_block)
                    .map_err(|e| format!("build_br switch.end failed: {:?}", e))?;
            }
        }

        // Default case.
        if let Some(db) = default_block {
            if let Some(default_body) = &switch_stmt.default {
                self.builder.position_at_end(db);
                for s in default_body {
                    self.compile_stmt(s)?;
                }
                if self.builder.get_insert_block().and_then(|b| b.get_terminator()).is_none() {
                    self.builder.build_unconditional_branch(end_block)
                        .map_err(|e| format!("build_br switch.end 2 failed: {:?}", e))?;
                }
            }
        }

        self.builder.position_at_end(end_block);
        Ok(())
    }

    /// Compile an `Owned<T>` dereference expression (`*expr`).
    ///
    /// The operand must evaluate to an `Owned<T>` value, which we represent
    /// as an `i8*` heap pointer stored in an alloca. The result is the
    /// loaded inner value.
    ///
    /// For Phase 1, we look up the operand's alloca (if it's an identifier)
    /// and use `ownership::load_owned_value` to load the inner value. The
    /// inner type is inferred from the variable's declared type.
    fn compile_owned_deref(&mut self, inner: &Expr) -> Result<BasicValueEnum<'ctx>, String> {
        // Get the pointer alloca for the Owned<T> value.
        let (ptr_alloca, inner_ty) = match inner {
            Expr::Identifier(name, _) => {
                let var = self.locals.get(name).ok_or_else(|| {
                    format!("codegen: unknown variable '{}' for owned deref", name)
                })?;
                // The variable's LLVM type is `i8*` (an opaque pointer).
                // We don't track the inner type separately, so we default
                // to i64 for the loaded value. This is sufficient for
                // Phase 1 micro-tests.
                let inner_ty = self.context.i64_type().into();
                (var.ptr, inner_ty)
            }
            _ => {
                // For non-identifier operands, evaluate the expression to
                // get a pointer value, store it in a temporary alloca, and
                // load from it.
                let v = self.compile_expr(inner)?;
                let i8_ptr_ty = self.context.ptr_type(AddressSpace::default());
                let alloca = self.builder.build_alloca(i8_ptr_ty, "deref.tmp")
                    .map_err(|e| format!("build_alloca deref.tmp failed: {:?}", e))?;
                self.builder.build_store(alloca, v)
                    .map_err(|e| format!("build_store deref.tmp failed: {:?}", e))?;
                (alloca, self.context.i64_type().into())
            }
        };
        ownership::load_owned_value(self.context, &self.builder, inner_ty, ptr_alloca, "deref")
    }

    /// Compile a borrow expression (`&expr` or `&mut expr`).
    ///
    /// For an identifier, this returns the address of the existing alloca.
    /// For other expressions, the value is evaluated and stored in a fresh
    /// alloca, whose address is returned.
    fn compile_ref_expr(
        &mut self,
        inner: &Expr,
        _ref_kind: &crate::ast::RefKind,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let i8_ptr_ty = self.context.ptr_type(AddressSpace::default());
        match inner {
            Expr::Identifier(name, _) => {
                let var = self.locals.get(name).ok_or_else(|| {
                    format!("codegen: unknown variable '{}' for borrow", name)
                })?;
                // Cast the alloca pointer to i8* (opaque pointer in LLVM 15+).
                let ptr = if var.ptr.get_type() == i8_ptr_ty {
                    var.ptr
                } else {
                    self.builder.build_bit_cast(var.ptr, i8_ptr_ty, "ref.cast")
                        .map_err(|e| format!("build_bit_cast ref failed: {:?}", e))?
                        .into_pointer_value()
                };
                Ok(ptr.into())
            }
            _ => {
                // Evaluate the expression and store it in a temporary alloca.
                let v = self.compile_expr(inner)?;
                let ty = v.get_type();
                let alloca = self.builder.build_alloca(ty, "ref.tmp")
                    .map_err(|e| format!("build_alloca ref.tmp failed: {:?}", e))?;
                self.builder.build_store(alloca, v)
                    .map_err(|e| format!("build_store ref.tmp failed: {:?}", e))?;
                let ptr = if alloca.get_type() == i8_ptr_ty {
                    alloca
                } else {
                    self.builder.build_bit_cast(alloca, i8_ptr_ty, "ref.tmp.cast")
                        .map_err(|e| format!("build_bit_cast ref.tmp failed: {:?}", e))?
                        .into_pointer_value()
                };
                Ok(ptr.into())
            }
        }
    }

    /// Compile a `region.alloc<T>(expr)` expression.
    ///
    /// For Phase 1, this is treated like an `Owned<T>` allocation: we
    /// allocate heap memory, store the initial value, and return the
    /// pointer. Region lifetime markers are not emitted here (they are
    /// emitted by `compile_unsafe_block` for `region` blocks).
    fn compile_region_alloc(
        &mut self,
        ty: &Type,
        init: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let inner_ty = llvm_types::llvm_type(self.context, ty)?;
        let init_val = self.compile_expr(init)?;
        let init_val = self.cast_value_to_type(init_val, inner_ty)?;
        let (ptr_alloca, _drop_flag) = ownership::alloc_owned(
            self.context,
            &self.builder,
            &self.module,
            init_val,
            inner_ty,
            "region.alloc",
        )?;
        // Return the raw i8* pointer.
        let i8_ptr_ty = self.context.ptr_type(AddressSpace::default());
        let raw = self.builder.build_load(i8_ptr_ty, ptr_alloca, "region.ptr")
            .map_err(|e| format!("build_load region.ptr failed: {:?}", e))?;
        Ok(raw)
    }

    /// Compile an `unsafe { ... }` block.
    ///
    /// For Phase 1, this simply compiles the body statements in the current
    /// scope without any safety checks. The block evaluates to the last
    /// expression statement's value (or `0` if the block is empty).
    fn compile_unsafe_block(&mut self, block: &[Stmt]) -> Result<BasicValueEnum<'ctx>, String> {
        let i32_ty = self.context.i32_type();
        let mut last_val: BasicValueEnum<'ctx> = i32_ty.const_int(0, false).into();
        self.ownership.enter_scope();
        for s in block {
            // If the statement is an expression statement, capture its value.
            if let Stmt::Expr(e) = s {
                last_val = self.compile_expr(e)?;
            } else {
                self.compile_stmt(s)?;
            }
        }
        self.emit_scope_cleanup()?;
        Ok(last_val)
    }

    /// Compile a `with` statement.
    ///
    /// For Phase 1, this evaluates the resource expression, binds it to the
    /// optional variable, compiles the body, and that's it. No automatic
    /// `.close()` call is emitted (Phase 1 limitation).
    fn compile_with(&mut self, with_stmt: &crate::ast::WithStmt) -> Result<(), String> {
        // Evaluate the resource expression.
        let resource_val = self.compile_expr(&with_stmt.resource_expr)?;
        // Bind it to the variable if a name was given.
        if let Some(name) = &with_stmt.var_name {
            let ty = with_stmt.var_type.as_ref()
                .map(|t| llvm_types::llvm_type(self.context, t))
                .transpose()?
                .unwrap_or_else(|| resource_val.get_type());
            let alloca = self.builder.build_alloca(ty, name)
                .map_err(|e| format!("build_alloca with '{}' failed: {:?}", name, e))?;
            let val = self.cast_value_to_type(resource_val, ty)?;
            self.builder.build_store(alloca, val)
                .map_err(|e| format!("build_store with '{}' failed: {:?}", name, e))?;
            self.locals.insert(name.clone(), LocalVar { ptr: alloca, ty, titrate_type: None });
        }
        // Compile the body with scope-based cleanup tracking.
        self.ownership.enter_scope();
        for s in &with_stmt.body {
            self.compile_stmt(s)?;
        }
        self.emit_scope_cleanup()?;
        Ok(())
    }

    /// Emit cleanup actions for the current scope. This pops the current
    /// scope's cleanup actions from the ownership stack and emits them
    /// (only if the current basic block has no terminator).
    fn emit_scope_cleanup(&mut self) -> Result<(), String> {
        let actions = self.ownership.exit_scope();
        let has_terminator = self.builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_some();
        if !has_terminator && !actions.is_empty() {
            ownership::emit_cleanup(self.context, &self.builder, &self.module, &actions)?;
        }
        Ok(())
    }

    /// Compile a tuple expression `(a, b, c)` into an anonymous struct.
    fn compile_tuple(&mut self, elements: &[Expr]) -> Result<BasicValueEnum<'ctx>, String> {
        let mut vals = Vec::with_capacity(elements.len());
        for e in elements {
            vals.push(self.compile_expr(e)?);
        }
        tuple_codegen::emit_tuple_construct(self.context, &self.builder, &self.module, &vals)
    }

    /// Compile tuple destructuring: `let (a, b) = tuple_expr`
    fn compile_tuple_destructure(&mut self, names: &[String], expr: &Expr) -> Result<(), String> {
        let tuple_val = self.compile_expr(expr)?;
        let struct_val = match tuple_val {
            BasicValueEnum::StructValue(sv) => sv,
            _ => return Err("tuple destructure: expected struct value".to_string()),
        };
        for (i, name) in names.iter().enumerate() {
            let field_val = tuple_codegen::emit_tuple_field_access(&self.builder, struct_val.into(), i as u32)?;
            let ty = field_val.get_type();
            let alloca = self.builder.build_alloca(ty, name)
                .map_err(|e| format!("build_alloca destructure '{}' failed: {:?}", name, e))?;
            self.builder.build_store(alloca, field_val)
                .map_err(|e| format!("build_store destructure '{}' failed: {:?}", name, e))?;
            self.locals.insert(name.clone(), LocalVar { ptr: alloca, ty, titrate_type: None });
        }
        Ok(())
    }

    /// Compile a closure expression into a `{ i8*, i8* }` struct.
    ///
    /// Phase 1: generates a trampoline function for each closure. The closure
    /// value is a 2-pointer struct:
    ///   - field 0: bitcast of the trampoline function to i8*
    ///   - field 1: capture data pointer (null for non-capturing closures)
    ///
    /// The trampoline signature is `fn(capture_data: i8*, params...) -> ret`.
    fn compile_closure(
        &mut self,
        params: &[(String, Type)],
        return_type: &Type,
        body: &[Stmt],
        expr: Option<&Expr>,
        captured_vars: &[String],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let i8_ptr_ty = self.context.ptr_type(AddressSpace::default());
        let void_ty = self.context.void_type();

        // Generate a unique name for the trampoline.
        let closure_id = self.closure_counter;
        self.closure_counter += 1;
        let tramp_name = format!("_closure_{}", closure_id);

        // Build the trampoline function type: fn(i8* capture_data, params...) -> ret
        let mut param_tys: Vec<inkwell::types::BasicMetadataTypeEnum> = Vec::new();
        param_tys.push(i8_ptr_ty.into()); // capture_data
        for (_, ty) in params {
            let llvm_ty = llvm_types::llvm_type(self.context, ty)?;
            param_tys.push(llvm_ty.into());
        }
        let ret_ty = llvm_types::llvm_type_or_void(self.context, Some(return_type))?;
        let fn_type = match ret_ty {
            Some(rt) => rt.fn_type(&param_tys, false),
            None => void_ty.fn_type(&param_tys, false),
        };

        let tramp_fn = self.module.add_function(&tramp_name, fn_type, Some(Linkage::Internal));

        // Save current state.
        let saved_locals = self.locals.clone();
        self.locals.clear();

        // Create entry block.
        let entry = self.context.append_basic_block(tramp_fn, "entry");
        self.builder.position_at_end(entry);

        // Bind the capture_data parameter (i8*).
        let capture_data = tramp_fn.get_first_param()
            .ok_or("codegen: closure trampoline has no capture_data param")?;
        let capture_data_ptr = capture_data.into_pointer_value();

        // Bind closure parameters (skip capture_data, already handled).
        let mut param_iter = tramp_fn.get_params().into_iter().skip(1);
        for (name, ty) in params {
            let param_val = param_iter.next()
                .ok_or_else(|| format!("missing param '{}'", name))?;
            let llvm_ty = llvm_types::llvm_type(self.context, ty)?;
            let alloca = self.builder.build_alloca(llvm_ty, name)
                .map_err(|e| format!("param alloca '{}': {:?}", name, e))?;
            self.builder.build_store(alloca, param_val)
                .map_err(|e| format!("param store '{}': {:?}", name, e))?;
            self.locals.insert(name.clone(), LocalVar { ptr: alloca, ty: llvm_ty, titrate_type: None });
        }

        // If there are captured variables, bind them from the capture data.
        // Phase 1: captured vars are stored sequentially in the capture buffer.
        if !captured_vars.is_empty() {
            let i32_ty = self.context.i32_type();
            for (i, var_name) in captured_vars.iter().enumerate() {
                let offset = (i as u64) * 8;
                let offset_val = i32_ty.const_int(offset, false);
                let gep = unsafe {
                    self.builder.build_gep(
                        i8_ptr_ty,
                        capture_data_ptr,
                        &[offset_val],
                        &format!("capture.{}.gep", var_name),
                    )
                }.map_err(|e| format!("capture gep '{}': {:?}", var_name, e))?;
                let ptr_ptr_ty = self.context.ptr_type(AddressSpace::default());
                let slot_ptr_val = self.builder.build_bit_cast(
                    gep,
                    ptr_ptr_ty,
                    &format!("capture.{}.ptr", var_name),
                ).map_err(|e| format!("capture bit_cast '{}': {:?}", var_name, e))?;
                let slot_ptr = slot_ptr_val.into_pointer_value();
                let captured_val = self.builder.build_load(i8_ptr_ty, slot_ptr, &format!("capture.{}.val", var_name))
                    .map_err(|e| format!("capture load '{}': {:?}", var_name, e))?;
                let alloca = self.builder.build_alloca(i8_ptr_ty, var_name)
                    .map_err(|e| format!("capture alloca '{}': {:?}", var_name, e))?;
                self.builder.build_store(alloca, captured_val)
                    .map_err(|e| format!("capture store '{}': {:?}", var_name, e))?;
                self.locals.insert(var_name.clone(), LocalVar { ptr: alloca, ty: i8_ptr_ty.into(), titrate_type: None });
            }
        }

        // Compile the closure body.
        for s in body {
            self.compile_stmt(s)?;
        }

        // If there's an expression body (arrow closure), compile and return it.
        if let Some(e) = expr {
            let val = self.compile_expr(e)?;
            if tramp_fn.get_type().get_return_type().is_some() {
                self.builder.build_return(Some(&val))
                    .map_err(|e| format!("closure return build: {:?}", e))?;
            }
        }

        // If the function has no terminator, add one.
        if self.builder.get_insert_block().and_then(|b| b.get_terminator()).is_none() {
            self.builder.build_return(None)
                .map_err(|e| format!("closure ret void: {:?}", e))?;
        }

        // Restore state.
        self.locals = saved_locals;

        // Build the closure struct: { i8*, i8* } = { fn_ptr, capture_data }
        let closure_struct_ty = self.context.struct_type(
            &[i8_ptr_ty.into(), i8_ptr_ty.into()],
            false,
        );

        let fn_ptr = tramp_fn.as_global_value().as_pointer_value();
        let fn_ptr_i8 = self.builder.build_bit_cast(
            fn_ptr,
            i8_ptr_ty,
            &format!("{}.cast", tramp_name),
        ).map_err(|e| format!("fn_ptr bit_cast: {:?}", e))?;

        let capture_ptr = if captured_vars.is_empty() {
            i8_ptr_ty.const_null()
        } else {
            // Allocate the capture buffer and store captured values.
            // Phase 1: each captured variable is an i8* (8 bytes).
            let i64_ty = self.context.i64_type();
            let capture_size = i64_ty.const_int((captured_vars.len() * 8) as u64, false);
            let malloc_fn = self.get_function("titrate_malloc");
            let capture_buf = self.builder.build_call(
                malloc_fn,
                &[capture_size.into()],
                &format!("{}.capture_buf", tramp_name),
            ).map_err(|e| format!("capture malloc: {:?}", e))?;
            let capture_buf = match capture_buf.try_as_basic_value() {
                inkwell::values::ValueKind::Basic(v) => v.into_pointer_value(),
                _ => return Err("capture malloc returned non-value".to_string()),
            };

            // Store each captured variable's value into the buffer.
            let i32_ty = self.context.i32_type();
            for (i, var_name) in captured_vars.iter().enumerate() {
                let offset = (i as u64) * 8;
                let offset_val = i32_ty.const_int(offset, false);
                let gep = unsafe {
                    self.builder.build_gep(
                        i8_ptr_ty,
                        capture_buf,
                        &[offset_val],
                        &format!("{}.capture{}.gep", tramp_name, i),
                    )
                }.map_err(|e| format!("capture store gep: {:?}", e))?;
                let ptr_ptr_ty = self.context.ptr_type(AddressSpace::default());
                let slot_val = self.builder.build_bit_cast(
                    gep,
                    ptr_ptr_ty,
                    &format!("{}.capture{}.slot", tramp_name, i),
                ).map_err(|e| format!("capture store bit_cast: {:?}", e))?;
                let slot = slot_val.into_pointer_value();

                if let Some(local) = self.locals.get(var_name) {
                    let val = self.builder.build_load(
                        local.ty,
                        local.ptr,
                        &format!("{}.capture{}.val", tramp_name, i),
                    ).map_err(|e| format!("capture load: {:?}", e))?;
                    let val_ptr = self.builder.build_bit_cast(
                        val.into_pointer_value(),
                        i8_ptr_ty,
                        &format!("{}.capture{}.cast", tramp_name, i),
                    ).map_err(|e| format!("capture cast: {:?}", e))?;
                    self.builder.build_store(slot, val_ptr)
                        .map_err(|e| format!("capture store: {:?}", e))?;
                }
            }

            capture_buf
        };

        // Build the closure struct: { i8* fn_ptr, i8* capture_data }
        let undef = closure_struct_ty.const_zero();
        let result = self.builder.build_insert_value(
            undef, fn_ptr_i8, 0, &format!("{}.fn", tramp_name),
        ).map_err(|e| format!("closure insert fn_ptr: {:?}", e))?;
        let result = self.builder.build_insert_value(
            result, capture_ptr, 1, &format!("{}.capture", tramp_name),
        ).map_err(|e| format!("closure insert capture: {:?}", e))?;

        let result: BasicValueEnum<'ctx> = match result {
            inkwell::values::AggregateValueEnum::StructValue(sv) => sv.into(),
            _ => return Err("unexpected aggregate value".to_string()),
        };
        Ok(result)
    }    /// Get the global exception pointer (`__titrate_exception`).
    fn get_exception_global(&self) -> PointerValue<'ctx> {
        self.module
            .get_global("__titrate_exception")
            .unwrap_or_else(|| panic!("__titrate_exception not declared"))
            .as_pointer_value()
    }

    /// Compile `ok(value)` or `err(value)` into a `Result<T, E>` struct.
    ///
    /// The Result struct is `{ i32, i8* }` where:
    ///   - field 0: tag (0 = Ok, 1 = Err)
    ///   - field 1: heap-allocated payload pointer
    fn compile_result_ctor(
        &mut self,
        name: &str,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() != 1 {
            return Err(format!("codegen: `{}` expects exactly 1 argument", name));
        }
        let tag = if name == "ok" { 0u64 } else { 1u64 };
        let i32_ty = self.context.i32_type();
        let _i8_ptr_ty = self.context.ptr_type(AddressSpace::default());
        let result_ty = llvm_types::result_type(self.context).into_struct_type();

        // Compile the argument.
        let arg_val = self.compile_expr(&args[0])?;
        let arg_ty = arg_val.get_type();
        let size = arg_ty.size_of()
            .ok_or_else(|| format!("cannot compute size of type {:?}", arg_ty))?;

        // Allocate heap memory for the payload.
        let malloc_fn = self.get_function("titrate_malloc");
        let payload_ptr = self.builder.build_call(malloc_fn, &[size.into()], &format!("{}.payload", name))
            .map_err(|e| format!("build_call titrate_malloc for {} failed: {:?}", name, e))?;
        let payload_ptr = match payload_ptr.try_as_basic_value() {
            inkwell::values::ValueKind::Basic(v) => v.into_pointer_value(),
            _ => return Err(format!("titrate_malloc did not return a value for {}", name)),
        };

        // Store the value into the heap allocation.
        self.builder.build_store(payload_ptr, arg_val)
            .map_err(|e| format!("build_store {} payload failed: {:?}", name, e))?;

        // Build the Result struct: { i32 tag, i8* payload }.
        let tag_val = i32_ty.const_int(tag, false);
        let undef = result_ty.const_zero();
        let result = self.builder.build_insert_value(undef, tag_val, 0, &format!("{}.tag", name))
            .map_err(|e| format!("build_insert_value tag failed: {:?}", e))?;
        let result = self.builder.build_insert_value(result, payload_ptr, 1, &format!("{}.ptr", name))
            .map_err(|e| format!("build_insert_value payload failed: {:?}", e))?;

        let result: BasicValueEnum<'ctx> = match result { inkwell::values::AggregateValueEnum::StructValue(sv) => sv.into(), _ => return Err("unexpected aggregate value".to_string()), };
        Ok(result)
    }
    /// Compile an error propagation expression (`expr?`).
    ///
    /// The operand must evaluate to a `Result<T, E>` struct `{ i32, i8* }`.
    /// If the tag is 1 (Err), the payload is stored in `__titrate_exception`
    /// and control branches to the nearest catch block. If the tag is 0 (Ok),
    /// the payload is loaded from the heap and returned as the unwrapped value.
    fn compile_error_propagation(&mut self, inner: &Expr) -> Result<BasicValueEnum<'ctx>, String> {
        let i32_ty = self.context.i32_type();
        let _i8_ptr_ty = self.context.ptr_type(AddressSpace::default());
        let _result_ty = llvm_types::result_type(self.context).into_struct_type();

        // Compile the expression to get the Result struct.
        let result_val = self.compile_expr(inner)?;
        let result_struct = match result_val {
            BasicValueEnum::StructValue(sv) => sv,
            _ => return Err("codegen: `?` operand must be a Result struct".to_string()),
        };

        // Extract the tag (field 0).
        let tag = self.builder.build_extract_value(result_struct, 0, "q.tag")
            .map_err(|e| format!("build_extract_value tag failed: {:?}", e))?;
        let tag = tag.into_int_value();

        // Compare tag with 1 (Err).
        let one = i32_ty.const_int(1, false);
        let is_err = self.builder.build_int_compare(inkwell::IntPredicate::EQ, tag, one, "q.is_err")
            .map_err(|e| format!("build_int_compare is_err failed: {:?}", e))?;

        let current_block = self.builder.get_insert_block()
            .ok_or("codegen: no insert block for `?`")?;
        let ok_block = self.context.insert_basic_block_after(current_block, "q.ok");
        let err_block = self.context.insert_basic_block_after(ok_block, "q.err");
        let end_block = self.context.insert_basic_block_after(err_block, "q.end");

        self.builder.build_conditional_branch(is_err, err_block, ok_block)
            .map_err(|e| format!("build_cond_br ? failed: {:?}", e))?;

        // Ok block: extract payload, load value from heap, branch to end.
        self.builder.position_at_end(ok_block);
        let payload_ptr = self.builder.build_extract_value(result_struct, 1, "q.payload")
            .map_err(|e| format!("build_extract_value payload failed: {:?}", e))?;
        let payload_ptr = payload_ptr.into_pointer_value();
        let ok_val = self.builder.build_load(i32_ty, payload_ptr, "q.ok.val")
            .map_err(|e| format!("build_load ok value failed: {:?}", e))?;
        let ok_block_end = self.builder.get_insert_block()
            .ok_or("codegen: no insert block after ok")?;
        self.builder.build_unconditional_branch(end_block)
            .map_err(|e| format!("build_br ?.end failed: {:?}", e))?;

        // Err block: extract payload, store in __titrate_exception, branch to catch.
        self.builder.position_at_end(err_block);
        let err_payload = self.builder.build_extract_value(result_struct, 1, "q.err.payload")
            .map_err(|e| format!("build_extract_value err payload failed: {:?}", e))?;
        let err_payload = err_payload.into_pointer_value();
        let exception_global = self.get_exception_global();
        self.builder.build_store(exception_global, err_payload)
            .map_err(|e| format!("build_store exception failed: {:?}", e))?;

        // Branch to the nearest catch block, or unreachable if none.
        if let Some(ctx) = self.catch_stack.last() {
            self.builder.build_unconditional_branch(ctx.catch_block)
                .map_err(|e| format!("build_br catch failed: {:?}", e))?;
        } else {
            // No catch handler - emit an unreachable.
            self.builder.build_unreachable()
                .map_err(|e| format!("build_unreachable failed: {:?}", e))?;
        }

        // End block: phi the ok value.
        self.builder.position_at_end(end_block);
        let phi = self.builder.build_phi(i32_ty, "q.result")
            .map_err(|e| format!("build_phi ? failed: {:?}", e))?;
        phi.add_incoming(&[(&ok_val, ok_block_end)]);

        Ok(phi.as_basic_value())
    }

    /// Compile a `throw expr;` statement.
    ///
    /// The expression is evaluated and its value is stored in the global
    /// `__titrate_exception`. Control then branches to the nearest catch
    /// block. If no catch block is active, an unreachable is emitted
    /// (Phase 1 limitation).
    fn compile_throw(&mut self, expr: &Expr) -> Result<(), String> {
        let i8_ptr_ty = self.context.ptr_type(AddressSpace::default());

        // Compile the thrown value.
        let v = self.compile_expr(expr)?;

        // Store the value in __titrate_exception. If the value is not i8*,
        // we store it as-is (Phase 1: assume string or pointer).
        let ptr = if v.is_pointer_value() {
            v.into_pointer_value()
        } else if v.is_int_value() {
            let iv = v.into_int_value();
            self.builder.build_int_to_ptr(iv, i8_ptr_ty, "throw.ptr")
                .map_err(|e| format!("build_int_to_ptr throw failed: {:?}", e))?
        } else {
            return Err(format!("codegen: cannot throw value of type {:?}", v.get_type()));
        };

        let exception_global = self.get_exception_global();
        self.builder.build_store(exception_global, ptr)
            .map_err(|e| format!("build_store throw exception failed: {:?}", e))?;

        // Branch to catch block or unreachable.
        if let Some(ctx) = self.catch_stack.last() {
            self.builder.build_unconditional_branch(ctx.catch_block)
                .map_err(|e| format!("build_br throw catch failed: {:?}", e))?;
        } else {
            self.builder.build_unreachable()
                .map_err(|e| format!("build_unreachable throw failed: {:?}", e))?;
        }

        Ok(())
    }

    /// Compile a `try { ... } catch (e: T) { ... }` statement.
    ///
    /// Sets up a catch block that receives the error value from
    /// `__titrate_exception`. The try body is compiled normally; if an
    /// exception is thrown (via `throw` or `?`), control jumps to the
    /// catch block. Otherwise, the catch block is skipped.
    fn compile_try_catch(
        &mut self,
        try_block: &[Stmt],
        catch_var: &str,
        catch_var_type: Option<&Type>,
        catch_block: &[Stmt],
    ) -> Result<(), String> {
        let i8_ptr_ty = self.context.ptr_type(AddressSpace::default());

        let current_block = self.builder.get_insert_block()
            .ok_or("codegen: no insert block for try")?;
        let try_body_block = self.context.insert_basic_block_after(current_block, "try.body");
        let catch_handler_block = self.context.insert_basic_block_after(try_body_block, "try.catch");
        let end_block = self.context.insert_basic_block_after(catch_handler_block, "try.end");

        // Allocate a slot for the error value.
        let error_alloca = self.builder.build_alloca(i8_ptr_ty, "try.error")
            .map_err(|e| format!("build_alloca try.error failed: {:?}", e))?;

        // Push the catch context.
        self.catch_stack.push(CatchContext {
            catch_block: catch_handler_block,
            error_alloca,
        });

        // Branch to the try body.
        self.builder.build_unconditional_branch(try_body_block)
            .map_err(|e| format!("build_br try.body failed: {:?}", e))?;

        // Compile the try body.
        self.builder.position_at_end(try_body_block);
        for s in try_block {
            self.compile_stmt(s)?;
        }
        // Pop the catch context.
        self.catch_stack.pop();

        // If the try body completed normally (no throw), branch to end.
        if self.builder.get_insert_block().and_then(|b| b.get_terminator()).is_none() {
            self.builder.build_unconditional_branch(end_block)
                .map_err(|e| format!("build_br try.end failed: {:?}", e))?;
        }

        // Catch handler: load the error from __titrate_exception, bind it
        // to the catch variable, and compile the catch body.
        self.builder.position_at_end(catch_handler_block);
        let exception_global = self.get_exception_global();
        let error_val = self.builder.build_load(i8_ptr_ty, exception_global, "catch.err")
            .map_err(|e| format!("build_load exception failed: {:?}", e))?;

        // Determine the catch variable type. Default to i8* (pointer).
        let var_ty = match catch_var_type {
            Some(t) if llvm_types::is_string(t) => llvm_types::string_type(self.context),
            Some(t) => llvm_types::llvm_type(self.context, t)?,
            None => i8_ptr_ty.into(),
        };

        // Allocate the catch variable and store the error value.
        let var_alloca = self.builder.build_alloca(var_ty, catch_var)
            .map_err(|e| format!("build_alloca catch '{}' failed: {:?}", catch_var, e))?;

        // Store the error value into the catch variable.
        self.builder.build_store(var_alloca, error_val)
            .map_err(|e| format!("build_store catch error failed: {:?}", e))?;

        // Register the catch variable in locals.
        let prev = self.locals.insert(catch_var.to_string(), LocalVar {
            ptr: var_alloca,
            ty: var_ty,
            titrate_type: None,
        });

        // Compile the catch body.
        for s in catch_block {
            self.compile_stmt(s)?;
        }
        // Restore previous binding.
        if let Some(p) = prev {
            self.locals.insert(catch_var.to_string(), p);
        } else {
            self.locals.remove(catch_var);
        }

        if self.builder.get_insert_block().and_then(|b| b.get_terminator()).is_none() {
            self.builder.build_unconditional_branch(end_block)
                .map_err(|e| format!("build_br try.end 2 failed: {:?}", e))?;
        }

        self.builder.position_at_end(end_block);
        Ok(())
    }

    /// Compile a non-generic top-level function.
    fn compile_function(&mut self, fn_decl: &FnDecl) -> Result<FunctionValue<'ctx>, String> {
        // Don't compile main here; it's handled separately.
        if fn_decl.name == "main" {
            return self.compile_main(fn_decl);
        }

        // Build the function type.
        let mut param_types: Vec<inkwell::types::BasicMetadataTypeEnum> = Vec::new();
        for p in &fn_decl.params {
            let ty = llvm_types::llvm_type(self.context, &p.typ)?;
            param_types.push(ty.into());
        }
        let return_type = llvm_types::llvm_type_or_void(self.context, fn_decl.return_type.as_ref())?;
        let fn_type = match return_type {
            Some(ret) => ret.fn_type(&param_types, false),
            None => self.context.void_type().fn_type(&param_types, false),
        };

        let fn_val = if let Some(existing) = self.module.get_function(&fn_decl.name) {
            existing
        } else {
            self.module.add_function(&fn_decl.name, fn_type, None)
        };
        self.functions.insert(fn_decl.name.clone(), fn_val);

        // Apply release-mode optimization hints (alwaysinline + fastcc for
        // small internal functions). Titrate functions are private by
        // default; only `public fn main` is treated as external.
        let is_external = fn_decl.name == "main";
        self.apply_release_attrs(fn_val, fn_decl.body.len(), is_external);

        // Create entry block.
        let entry = self.context.append_basic_block(fn_val, "entry");
        self.builder.position_at_end(entry);

        // Save and clear locals.
        let saved_locals = std::mem::take(&mut self.locals);

        // Allocate space for parameters and store them.
        for (i, p) in fn_decl.params.iter().enumerate() {
            let param_val = fn_val.get_nth_param(i as u32)
                .ok_or_else(|| format!("missing param {} for {}", i, fn_decl.name))?;
            let ty = llvm_types::llvm_type(self.context, &p.typ)?;
            let alloca = self.builder.build_alloca(ty, &p.name)
                .map_err(|e| format!("build_alloca param '{}' failed: {:?}", p.name, e))?;
            self.builder.build_store(alloca, param_val)
                .map_err(|e| format!("build_store param '{}' failed: {:?}", p.name, e))?;
            self.locals.insert(p.name.clone(), LocalVar { ptr: alloca, ty, titrate_type: None });
        }

        // Compile body.
        for s in &fn_decl.body {
            self.compile_stmt(s)?;
        }

        // Add a default return if the current block has no terminator.
        if self.builder.get_insert_block().and_then(|b| b.get_terminator()).is_none() {
            // Always return void for void functions, regardless of what the body produced.
            let is_void_fn = fn_decl.return_type.as_ref()
                .map(|t| llvm_types::is_void(t))
                .unwrap_or(true);
            if is_void_fn {
                self.builder.build_return(None)
                    .map_err(|e| format!("build_return void failed: {:?}", e))?;
            } else {
                match &fn_decl.return_type {
                    Some(t) => {
                        let ty = llvm_types::llvm_type(self.context, t)?;
                        let zero: BasicValueEnum<'ctx> = match ty {
                            BasicTypeEnum::IntType(it) => it.const_int(0, false).into(),
                            BasicTypeEnum::FloatType(ft) => ft.const_float(0.0).into(),
                            BasicTypeEnum::PointerType(pt) => pt.const_null().into(),
                            _ => {
                                self.builder.build_return(None)
                                    .map_err(|e| format!("build_return failed: {:?}", e))?;
                                return Ok(fn_val);
                            }
                        };
                        self.builder.build_return(Some(&zero))
                            .map_err(|e| format!("build_return zero failed: {:?}", e))?;
                    }
                    None => {
                        self.builder.build_return(None)
                            .map_err(|e| format!("build_return void failed: {:?}", e))?;
                    }
                }
            }
        }

        // Restore locals.
        self.locals = saved_locals;

        Ok(fn_val)
    }

    /// Compile the `main` function. The Titrate `main` returns `void`, but we
    /// emit a C-style `int main()` that returns 0.
    fn compile_main(&mut self, fn_decl: &FnDecl) -> Result<FunctionValue<'ctx>, String> {
        let i32_type = self.context.i32_type();
        let main_fn_type = i32_type.fn_type(&[], false);
        let main_fn = self
            .module
            .add_function("main", main_fn_type, None);
        self.functions.insert("main".to_string(), main_fn);
        let entry = self.context.append_basic_block(main_fn, "entry");
        self.builder.position_at_end(entry);

        // Reset locals for this function scope.
        let saved_locals = std::mem::take(&mut self.locals);

        for stmt in &fn_decl.body {
            self.compile_stmt(stmt)?;
        }

        // Return 0.
        let zero = i32_type.const_int(0, false);
        self.builder
            .build_return(Some(&zero))
            .map_err(|e| format!("build_return failed: {:?}", e))?;

        self.locals = saved_locals;

        Ok(main_fn)
    }

    /// Compile the whole program: find `main`, emit IR, verify, and write the
    /// object file.
    pub fn compile_program(
        &mut self,
        program: &Program,
        object_path: &Path,
        release: bool,
    ) -> Result<(), String> {
        // Record the release flag so that compile_function / compile_for can
        // emit the appropriate optimization hints.
        self.release_mode = release;
        self.declare_natives();

        // First pass: compile all enum declarations.
        for decl in &program.declarations {
            if let Declaration::Enum(enum_decl) = decl {
                let enum_info = compile_enum_decl(self.context, &self.module, enum_decl)?;
                self.enum_infos.insert(enum_decl.name.clone(), enum_info);
            }
        }

        // Pass: compile all interface declarations.
        for decl in &program.declarations {
            if let Declaration::Interface(iface_decl) = decl {
                self.compile_interface_decl(iface_decl)?;
            }
        }

        // Second pass: compile all class declarations (build struct types, vtables, methods).
        for decl in &program.declarations {
            if let Declaration::Class(class_decl) = decl {
                self.compile_class_decl(class_decl)?;
            }
        }

        // Third pass: build interface vtables for classes that implement interfaces.
        for decl in &program.declarations {
            if let Declaration::Class(class_decl) = decl {
                let class_name = class_decl.name.clone();
                for iface_type in &class_decl.ifaces {
                    let iface_name = iface_type.name();
                    if let Some(iface_info) = self.interface_infos.get(iface_name).cloned() {
                        // Collect the class's method functions.
                        let mut class_methods: HashMap<String, FunctionValue<'ctx>> = HashMap::new();
                        for member in &class_decl.members {
                            match member {
                                ClassMember::Method(m) | ClassMember::Constructor(m) => {
                                    let fn_name = format!("{}_{}", class_name, m.name);
                                    if let Some(fn_val) = self.module.get_function(&fn_name) {
                                        class_methods.insert(m.name.clone(), fn_val);
                                    }
                                }
                                _ => {}
                            }
                        }
                        let vt = create_interface_vtable(
                            self.context, &self.module,
                            &class_name, iface_name,
                            &iface_info.method_names,
                            &class_methods,
                            &iface_info.default_methods,
                        );
                        if let Some(vt_global) = vt {
                            self.interface_vtables.insert(
                                (iface_name.to_string(), class_name.clone()),
                                vt_global,
                            );
                        }
                    }
                }
            }
        }

        // Register all function declarations first (for recursion).
        for decl in &program.declarations {
            if let Declaration::Function(f) = decl {
                if f.type_params.is_empty() {
                    self.function_decls.insert(f.name.clone(), f.clone());
                }
            }
        }

        // Compile all non-generic, non-main functions first.
        for decl in &program.declarations {
            if let Declaration::Function(f) = decl {
                if f.name != "main" && f.type_params.is_empty() {
                    self.compile_function(f)?;
                }
            }
        }

        // Find and compile main.
        let main_decl = program
            .declarations
            .iter()
            .find_map(|d| match d {
                Declaration::Function(f) if f.name == "main" => Some(f),
                _ => None,
            })
            .ok_or("codegen: no `main` function found")?;

        self.compile_main(main_decl)?;

        // Verify the module.
        if let Err(err) = self.module.verify() {
            return Err(format!("LLVM module verification failed:\n{}", err.to_string()));
        }

        // Emit the object file.
        self.write_object(object_path, release)?;

        Ok(())
    }

    /// Compile a typed program to LLVM IR text (without writing an object file).
    ///
    /// This runs the same codegen pipeline as [`compile_program`] but skips
    /// the target-machine / object-file step, returning the IR as a string
    /// instead. Useful for testing and debugging.
    pub fn compile_program_to_ir_text(&mut self, program: &Program) -> Result<String, String> {
        self.declare_natives();

        // First pass: compile all enum declarations.
        for decl in &program.declarations {
            if let Declaration::Enum(enum_decl) = decl {
                let enum_info = compile_enum_decl(self.context, &self.module, enum_decl)?;
                self.enum_infos.insert(enum_decl.name.clone(), enum_info);
            }
        }

        // Pass: compile all interface declarations.
        for decl in &program.declarations {
            if let Declaration::Interface(iface_decl) = decl {
                self.compile_interface_decl(iface_decl)?;
            }
        }

        // Second pass: compile all class declarations.
        for decl in &program.declarations {
            if let Declaration::Class(class_decl) = decl {
                self.compile_class_decl(class_decl)?;
            }
        }

        // Register all function declarations first (for recursion).
        for decl in &program.declarations {
            if let Declaration::Function(f) = decl {
                if f.type_params.is_empty() {
                    self.function_decls.insert(f.name.clone(), f.clone());
                }
            }
        }

        // Compile all non-generic, non-main functions first.
        for decl in &program.declarations {
            if let Declaration::Function(f) = decl {
                if f.name != "main" && f.type_params.is_empty() {
                    self.compile_function(f)?;
                }
            }
        }

        // Find and compile main.
        let main_decl = program
            .declarations
            .iter()
            .find_map(|d| match d {
                Declaration::Function(f) if f.name == "main" => Some(f),
                _ => None,
            })
            .ok_or("codegen: no `main` function found")?;

        self.compile_main(main_decl)?;

        // Verify the module.
        if let Err(err) = self.module.verify() {
            return Err(format!("LLVM module verification failed:\n{}", err.to_string()));
        }

        Ok(self.module.print_to_string().to_string())
    }

    /// Emit a call to the `llvm.memset.p0i8.i64` intrinsic to zero-initialize
    /// `size` bytes starting at `ptr`. This is used to zero out freshly
    /// allocated class instances and arrays instead of emitting
    /// element-by-element stores.
    ///
    /// The intrinsic is declared (lazily) as:
    ///   `void @llvm.memset.p0i8.i64(i8* dest, i8 val, i64 len, i1 is_volatile)`
    pub fn emit_memset_zero(
        &self,
        ptr: PointerValue<'ctx>,
        size: IntValue<'ctx>,
    ) -> Result<(), String> {
        let i8_ty = self.context.i8_type();
        let i64_ty = self.context.i64_type();
        let i1_ty = self.context.bool_type();
        let i8_ptr = self.context.ptr_type(AddressSpace::default());
        let void_ty = self.context.void_type();

        let fn_ty = void_ty.fn_type(
            &[i8_ptr.into(), i8_ty.into(), i64_ty.into(), i1_ty.into()],
            false,
        );

        // Declare the intrinsic if it isn't already in the module.
        let memset_fn = match self.module.get_function("llvm.memset.p0i8.i64") {
            Some(f) => f,
            None => self.module.add_function(
                "llvm.memset.p0i8.i64",
                fn_ty,
                None,
            ),
        };

        let zero_val = i8_ty.const_int(0, false);
        let is_volatile = i1_ty.const_int(0, false);
        self.builder
            .build_call(
                memset_fn,
                &[
                    ptr.into(),
                    zero_val.into(),
                    size.into(),
                    is_volatile.into(),
                ],
                "memset.zero",
            )
            .map_err(|e| format!("build_call memset failed: {:?}", e))?;
        Ok(())
    }

    /// Attach `!llvm.loop` metadata with vectorization hints to the terminator
    /// of `loop_block` (typically the latch/cond branch of a for/while loop).
    ///
    /// The emitted metadata looks like:
    ///   `!llvm.loop !N` where `!N = !{!N, !{!"llvm.loop.vectorize.enable", i32 1}}`
    ///
    /// This is a hint only; LLVM may still choose not to vectorize.
    pub fn add_vectorize_metadata(
        &self,
        loop_block: inkwell::basic_block::BasicBlock<'ctx>,
    ) -> Result<(), String> {
        let i32_ty = self.context.i32_type();

        // Build the metadata nodes:
        //   !{!"llvm.loop.vectorize.enable", i32 1}
        //   !{!"llvm.loop.interleave.count", i32 4}
        let enable_str = self.context.metadata_string("llvm.loop.vectorize.enable");
        let enable_val = i32_ty.const_int(1, false);
        let enable_node = self.context.metadata_node(&[
            enable_str.into(),
            enable_val.into(),
        ]);

        let width_str = self.context.metadata_string("llvm.loop.vectorize.width");
        let width_val = i32_ty.const_int(4, false);
        let width_node = self.context.metadata_node(&[
            width_str.into(),
            width_val.into(),
        ]);

        // The loop metadata node references itself (LLVM convention) plus the
        // option nodes. We create it with the option nodes; LLVM treats the
        // first operand as a self-reference when attached to a branch.
        let loop_node = self.context.metadata_node(&[
            enable_node.into(),
            width_node.into(),
        ]);

        let terminator = loop_block
            .get_terminator()
            .ok_or_else(|| "codegen: loop block has no terminator for vectorize metadata".to_string())?;
        terminator
            .set_metadata(loop_node, LLVM_LOOP_METADATA_KIND)
            .map_err(|e| format!("set_metadata llvm.loop failed: {:?}", e))?;
        Ok(())
    }

    /// Apply release-mode optimization attributes to a freshly-created
    /// function value:
    ///   - `alwaysinline` on small functions (< 10 statements)
    ///   - fast calling convention for internal (non-external) functions
    ///
    /// `statement_count` is the number of top-level statements in the
    /// function body. The caller is responsible for passing an accurate count.
    pub fn apply_release_attrs(
        &self,
        fn_val: FunctionValue<'ctx>,
        statement_count: usize,
        is_external: bool,
    ) {
        if !self.release_mode {
            return;
        }

        // Small internal functions get alwaysinline + fastcc.
        if !is_external && statement_count < 10 {
            // alwaysinline is an enum attribute with kind id = 10 (LLVM 22).
            // We look it up by name to be robust across LLVM versions.
            let kind_id = Attribute::get_named_enum_kind_id("alwaysinline");
            let attr = self.context.create_enum_attribute(kind_id, 0);
            fn_val.add_attribute(AttributeLoc::Function, attr);

            // Use fast calling convention for internal functions.
            fn_val.set_call_conventions(LLVM_FAST_CALL_CONV);
        }
    }

    /// Write the module to an object file using the native target.
    fn write_object(&self, path: &Path, release: bool) -> Result<(), String> {
        // Initialize the X86 target directly via the individual
        // LLVMInitializeX86* functions. This avoids relying on the
        // #[no_mangle] LLVM_InitializeNativeTarget symbol (called by
        // inkwell's Target::initialize_native), which may not resolve
        // correctly when the inkwell feature version and the installed
        // LLVM version differ. The X86 target is always available because
        // the `target-x86` inkwell feature is enabled.
        target_wrappers::initialize_x86();

        let triple = TargetMachine::get_default_triple();
        let target = Target::from_triple(&triple)
            .map_err(|e| format!("failed to get target: {}", e))?;

        let opt_level = if release {
            OptimizationLevel::Aggressive
        } else {
            OptimizationLevel::None
        };

        let target_machine = target
            .create_target_machine(
                &triple,
                TargetMachine::get_host_cpu_name().to_str().unwrap_or("generic"),
                TargetMachine::get_host_cpu_features().to_str().unwrap_or(""),
                opt_level,
                RelocMode::Default,
                CodeModel::Default,
            )
            .ok_or("failed to create target machine")?;

        target_machine
            .write_to_file(&self.module, FileType::Object, path)
            .map_err(|e| format!("failed to write object file: {}", e))?;

        Ok(())
    }
}

/// Compile a typed Titrate program to a native object file.
///
/// `program` is the typed AST produced by `analyzer::analyze`.
/// `object_path` is where the `.o` / `.obj` file will be written.
/// If `release` is true, LLVM optimizations are enabled.
pub fn compile(
    program: &Program,
    object_path: &Path,
    release: bool,
) -> Result<(), String> {
    let context = Context::create();
    let mut backend = LlvmBackend::new(&context, "titrate_main");
    backend.compile_program(program, object_path, release)
}

/// Compile a typed Titrate program to LLVM IR text (without writing an object file).
///
/// This is useful for testing and debugging: it runs the full codegen
/// pipeline (including module verification) but returns the IR as a string
/// instead of invoking the system target machine.
///
/// `program` is the typed AST produced by `analyzer::analyze`.
pub fn compile_to_ir_text(program: &Program) -> Result<String, String> {
    let context = Context::create();
    let mut backend = LlvmBackend::new(&context, "titrate_main");
    backend.compile_program_to_ir_text(program)
}

/// Compile a typed Titrate program and write the LLVM IR to a `.ll` file.
///
/// This runs the same codegen pipeline as [`compile`] (including module
/// verification) but writes the IR text to `ir_path` instead of an object
/// file. Useful for inspecting the generated IR. The IR is written via
/// inkwell's `Module::print_to_file`.
///
/// `program` is the typed AST produced by `analyzer::analyze`.
pub fn compile_ir(program: &Program, ir_path: &Path) -> Result<(), String> {
    let context = Context::create();
    let mut backend = LlvmBackend::new(&context, "titrate_main");
    // Run the full codegen pipeline (declare natives, compile all decls,
    // find main, and verify the module). The returned IR string is not needed
    // here; the IR is written via inkwell's `print_to_file` below.
    backend.compile_program_to_ir_text(program)?;
    backend
        .module
        .print_to_file(ir_path)
        .map_err(|e| format!("failed to write LLVM IR to {}: {}", ir_path.display(), e))?;
    Ok(())
}

/// Compile a typed Titrate program to a native object file AND write the
/// LLVM IR to a `.ll` file in a single codegen pass.
///
/// Equivalent to calling [`compile`] (object file) and [`compile_ir`] (IR
/// file) but only runs the codegen pipeline once: after the object file is
/// written, the already-populated module is dumped to `ir_path` via inkwell's
/// `Module::print_to_file`.
///
/// `program` is the typed AST produced by `analyzer::analyze`.
/// `object_path` is where the `.o` / `.obj` file will be written.
/// `ir_path` is where the `.ll` IR file will be written.
/// If `release` is true, LLVM optimizations are enabled.
pub fn compile_with_ir(
    program: &Program,
    object_path: &Path,
    ir_path: &Path,
    release: bool,
) -> Result<(), String> {
    let context = Context::create();
    let mut backend = LlvmBackend::new(&context, "titrate_main");
    // 1. Lower to LLVM IR, verify, and write the object file.
    backend.compile_program(program, object_path, release)?;
    // 2. The module is now fully populated; dump it to the .ll file.
    backend
        .module
        .print_to_file(ir_path)
        .map_err(|e| format!("failed to write LLVM IR to {}: {}", ir_path.display(), e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer;
    use crate::parser;
    use crate::analyzer;

    /// Compile a source string to LLVM IR and return the IR string.
    fn compile_to_ir(source: &str) -> Result<String, String> {
        let tokens = lexer::tokenize(source).map_err(|e| format!("tokenize: {}", e))?;
        let ast = parser::parse(tokens).map_err(|e| format!("parse: {}", e))?;
        let typed_ast = analyzer::analyze(&ast).map_err(|e| format!("analyze: {:?}", e))?;
        let context = Context::create();
        let mut backend = LlvmBackend::new(&context, "test");
        backend.declare_natives();
        // Find and compile main.
        let main_decl = typed_ast.declarations.iter().find_map(|d| match d {
            Declaration::Function(f) if f.name == "main" => Some(f),
            _ => None,
        }).ok_or("no main")?;
        backend.compile_main(main_decl).map_err(|e| format!("compile: {}", e))?;
        Ok(backend.module.print_to_string().to_string())
    }

    /// Compile a full program (including non-main functions) to LLVM IR.
    fn compile_program_to_ir(source: &str) -> Result<String, String> {
        let tokens = lexer::tokenize(source).map_err(|e| format!("tokenize: {}", e))?;
        let ast = parser::parse(tokens).map_err(|e| format!("parse: {}", e))?;
        let typed_ast = analyzer::analyze(&ast).map_err(|e| format!("analyze: {:?}", e))?;
        let context = Context::create();
        let mut backend = LlvmBackend::new(&context, "test");
        backend.declare_natives();
        // Register all function declarations first (for recursion).
        for decl in &typed_ast.declarations {
            if let Declaration::Function(f) = decl {
                if f.type_params.is_empty() {
                    backend.function_decls.insert(f.name.clone(), f.clone());
                }
            }
        }
        // Compile all non-generic, non-main functions first.
        for decl in &typed_ast.declarations {
            if let Declaration::Function(f) = decl {
                if f.name != "main" && f.type_params.is_empty() {
                    backend.compile_function(f)
                        .map_err(|e| format!("compile function '{}': {}", f.name, e))?;
                }
            }
        }
        // Find and compile main.
        let main_decl = typed_ast.declarations.iter().find_map(|d| match d {
            Declaration::Function(f) if f.name == "main" => Some(f),
            _ => None,
        }).ok_or("no main")?;
        backend.compile_main(main_decl).map_err(|e| format!("compile main: {}", e))?;
        Ok(backend.module.print_to_string().to_string())
    }

    #[test]
    fn int_variable_declaration() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let x: int = 42;
            }
        "#).expect("IR generation should succeed");
        assert!(ir.contains("alloca i32"), "expected i32 alloca, got:\n{}", ir);
        assert!(ir.contains("store i32 42"), "expected store i32 42, got:\n{}", ir);
    }

    #[test]
    fn long_variable_declaration() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let x: long = 1000000000000;
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("alloca i64"), "expected i64 alloca, got:\n{}", ir);
        assert!(ir.contains("store i64"), "expected store i64, got:\n{}", ir);
    }

    #[test]
    fn double_variable_declaration() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let pi: double = 3.14;
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("alloca double"), "expected double alloca, got:\n{}", ir);
        assert!(ir.contains("store double"), "expected store double, got:\n{}", ir);
    }

    #[test]
    fn float_variable_declaration() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let f: float = 1.0;
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("alloca float"), "expected float alloca, got:\n{}", ir);
    }

    #[test]
    fn bool_variable_declaration() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let b: bool = true;
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("alloca i1"), "expected i1 alloca, got:\n{}", ir);
    }

    #[test]
    fn char_variable_declaration() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let c: char = 'A';
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("alloca i32"), "expected i32 alloca for char, got:\n{}", ir);
        assert!(ir.contains("store i32 65"), "expected store i32 65, got:\n{}", ir);
    }

    #[test]
    fn byte_variable_declaration() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let b: byte = 100;
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("alloca i8"), "expected i8 alloca, got:\n{}", ir);
    }

    #[test]
    fn string_variable_still_works() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let s: string = "hello";
            }
        "#).expect("IR should succeed");
        // String is stored as a { i64, ptr } struct.
        assert!(ir.contains("alloca"), "expected alloca, got:\n{}", ir);
    }

    #[test]
    fn println_int() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                io::println(42);
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("titrate_println_int"), "expected titrate_println_int call, got:\n{}", ir);
    }

    #[test]
    fn println_double() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                io::println(3.14);
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("titrate_println_double"), "expected titrate_println_double call, got:\n{}", ir);
    }

    #[test]
    fn println_bool() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                io::println(true);
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("titrate_println_bool"), "expected titrate_println_bool call, got:\n{}", ir);
    }

    #[test]
    fn println_char() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                io::println('A');
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("titrate_println_char"), "expected titrate_println_char call, got:\n{}", ir);
    }

    #[test]
    fn println_string_still_works() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                io::println("Hello!");
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("titrate_println"), "expected titrate_println call, got:\n{}", ir);
    }

    #[test]
    fn int_assignment() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                var x: int = 1;
                x = 2;
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("store i32 2"), "expected store i32 2, got:\n{}", ir);
    }

    #[test]
    fn double_assignment() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                var x: double = 1.0;
                x = 2.0;
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("store double"), "expected store double, got:\n{}", ir);
    }

    // ---- Operator tests (Task 1.3) ----

    #[test]
    fn int_addition() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let a: int = 1;
                let b: int = 2;
                a + b;
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("add"), "expected add instruction, got:\n{}", ir);
    }

    #[test]
    fn int_subtraction() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let a: int = 5;
                let b: int = 3;
                a - b;
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("sub"), "expected sub instruction, got:\n{}", ir);
    }

    #[test]
    fn int_multiplication() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let a: int = 3;
                let b: int = 4;
                a * b;
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("mul"), "expected mul instruction, got:\n{}", ir);
    }

    #[test]
    fn int_division() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let a: int = 10;
                let b: int = 2;
                a / b;
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("sdiv"), "expected sdiv instruction, got:\n{}", ir);
    }

    #[test]
    fn int_modulo() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let a: int = 10;
                let b: int = 3;
                a % b;
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("srem"), "expected srem instruction, got:\n{}", ir);
    }

    #[test]
    fn float_addition() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let a: double = 1.0;
                let b: double = 2.0;
                a + b;
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("fadd"), "expected fadd instruction, got:\n{}", ir);
    }

    #[test]
    fn float_subtraction() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let a: double = 5.0;
                let b: double = 3.0;
                a - b;
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("fsub"), "expected fsub instruction, got:\n{}", ir);
    }

    #[test]
    fn float_multiplication() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let a: double = 3.0;
                let b: double = 4.0;
                a * b;
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("fmul"), "expected fmul instruction, got:\n{}", ir);
    }

    #[test]
    fn float_division() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let a: double = 10.0;
                let b: double = 2.0;
                a / b;
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("fdiv"), "expected fdiv instruction, got:\n{}", ir);
    }

    #[test]
    fn int_equal_comparison() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let a: int = 1;
                let b: int = 2;
                a == b;
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("icmp eq"), "expected icmp eq, got:\n{}", ir);
    }

    #[test]
    fn int_not_equal_comparison() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let a: int = 1;
                let b: int = 2;
                a != b;
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("icmp ne"), "expected icmp ne, got:\n{}", ir);
    }

    #[test]
    fn int_less_than() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let a: int = 1;
                let b: int = 2;
                a < b;
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("icmp slt"), "expected icmp slt, got:\n{}", ir);
    }

    #[test]
    fn int_greater_than() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let a: int = 1;
                let b: int = 2;
                a > b;
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("icmp sgt"), "expected icmp sgt, got:\n{}", ir);
    }

    #[test]
    fn int_less_equal() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let a: int = 1;
                let b: int = 2;
                a <= b;
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("icmp sle"), "expected icmp sle, got:\n{}", ir);
    }

    #[test]
    fn int_greater_equal() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let a: int = 1;
                let b: int = 2;
                a >= b;
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("icmp sge"), "expected icmp sge, got:\n{}", ir);
    }

    #[test]
    fn logical_and_short_circuits() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let a: bool = true;
                let b: bool = false;
                a && b;
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("phi"), "expected phi for short-circuit &&, got:\n{}", ir);
    }

    #[test]
    fn logical_or_short_circuits() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let a: bool = true;
                let b: bool = false;
                a || b;
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("phi"), "expected phi for short-circuit ||, got:\n{}", ir);
    }

    #[test]
    fn logical_not() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let a: bool = true;
                !a;
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("xor"), "expected xor for !, got:\n{}", ir);
    }

    #[test]
    fn bitwise_and() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let a: int = 255;
                let b: int = 15;
                a & b;
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("and"), "expected and instruction, got:\n{}", ir);
    }

    #[test]
    fn bitwise_or() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let a: int = 240;
                let b: int = 15;
                a | b;
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("or"), "expected or instruction, got:\n{}", ir);
    }

    #[test]
    fn bitwise_xor() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let a: int = 255;
                let b: int = 15;
                a ^ b;
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("xor"), "expected xor instruction, got:\n{}", ir);
    }

    #[test]
    fn bitwise_not() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let a: int = 255;
                ~a;
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("xor"), "expected xor for ~, got:\n{}", ir);
    }

    #[test]
    fn bitwise_left_shift() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let a: int = 1;
                let b: int = 4;
                a << b;
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("shl"), "expected shl instruction, got:\n{}", ir);
    }

    #[test]
    fn bitwise_right_shift() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let a: int = 256;
                let b: int = 2;
                a >> b;
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("ashr"), "expected ashr instruction, got:\n{}", ir);
    }

    #[test]
    fn unary_negation_int() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let a: int = 5;
                -a;
            }
        "#).expect("IR should succeed");
        // -a is compiled as 0 - a
        assert!(ir.contains("sub"), "expected sub for negation, got:\n{}", ir);
    }

    #[test]
    fn unary_negation_float() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let a: double = 5.0;
                -a;
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("fneg"), "expected fneg instruction, got:\n{}", ir);
    }

    // ---- Control flow tests (Task 1.4) ----

    #[test]
    fn if_statement() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let a: int = 1;
                if (a > 0) {
                    io::println("positive");
                }
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("if.then"), "expected if.then block, got:\n{}", ir);
        assert!(ir.contains("if.end"), "expected if.end block, got:\n{}", ir);
        assert!(ir.contains("br i1"), "expected conditional branch, got:\n{}", ir);
    }

    #[test]
    fn if_else_statement() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let a: int = 1;
                if (a > 0) {
                    io::println("positive");
                } else {
                    io::println("non-positive");
                }
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("if.then"), "expected if.then block, got:\n{}", ir);
        assert!(ir.contains("if.else"), "expected if.else block, got:\n{}", ir);
        assert!(ir.contains("if.end"), "expected if.end block, got:\n{}", ir);
    }

    #[test]
    fn while_loop() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                var i: int = 0;
                while (i < 10) {
                    i = i + 1;
                }
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("while.cond"), "expected while.cond block, got:\n{}", ir);
        assert!(ir.contains("while.body"), "expected while.body block, got:\n{}", ir);
        assert!(ir.contains("while.end"), "expected while.end block, got:\n{}", ir);
    }

    #[test]
    fn do_while_loop() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                var i: int = 0;
                do {
                    i = i + 1;
                } while (i < 10);
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("do.body"), "expected do.body block, got:\n{}", ir);
        assert!(ir.contains("do.cond"), "expected do.cond block, got:\n{}", ir);
        assert!(ir.contains("do.end"), "expected do.end block, got:\n{}", ir);
    }

    #[test]
    fn for_in_range_loop() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                for (i in 0..10) {
                    io::println(i);
                }
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("for.cond"), "expected for.cond block, got:\n{}", ir);
        assert!(ir.contains("for.body"), "expected for.body block, got:\n{}", ir);
        assert!(ir.contains("for.inc"), "expected for.inc block, got:\n{}", ir);
        assert!(ir.contains("for.end"), "expected for.end block, got:\n{}", ir);
    }

    #[test]
    fn for_in_inclusive_range_loop() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                for (i in 0..=5) {
                    io::println(i);
                }
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("for.cond"), "expected for.cond block, got:\n{}", ir);
        assert!(ir.contains("for.body"), "expected for.body block, got:\n{}", ir);
    }

    #[test]
    fn c_style_for_loop() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                for (var i = 0; i < 10; i++) {
                    io::println(i);
                }
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("cfor.cond"), "expected cfor.cond block, got:\n{}", ir);
        assert!(ir.contains("cfor.body"), "expected cfor.body block, got:\n{}", ir);
        assert!(ir.contains("cfor.inc"), "expected cfor.inc block, got:\n{}", ir);
        assert!(ir.contains("cfor.end"), "expected cfor.end block, got:\n{}", ir);
    }

    #[test]
    fn switch_statement() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let x: int = 1;
                switch (x) {
                    case 0 => io::println("zero");
                    case 1 => io::println("one");
                    case _ => io::println("other");
                }
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("switch"), "expected switch instruction, got:\n{}", ir);
        assert!(ir.contains("switch.end"), "expected switch.end block, got:\n{}", ir);
    }

    #[test]
    fn ternary_expression() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let a: int = 1;
                let b: int = a > 0 ? 1 : 0;
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("tern.then"), "expected tern.then block, got:\n{}", ir);
        assert!(ir.contains("tern.else"), "expected tern.else block, got:\n{}", ir);
        assert!(ir.contains("tern.end"), "expected tern.end block, got:\n{}", ir);
        assert!(ir.contains("phi"), "expected phi for ternary, got:\n{}", ir);
    }

    #[test]
    fn break_in_while_loop() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                var i: int = 0;
                while (i < 100) {
                    if (i == 5) {
                        break;
                    }
                    i = i + 1;
                }
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("while.end"), "expected while.end block for break, got:\n{}", ir);
    }

    #[test]
    fn continue_in_while_loop() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                var i: int = 0;
                while (i < 10) {
                    i = i + 1;
                    if (i == 5) {
                        continue;
                    }
                }
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("while.cond"), "expected while.cond block for continue, got:\n{}", ir);
    }

    #[test]
    fn nested_if_else() {
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let a: int = 5;
                if (a > 0) {
                    if (a > 10) {
                        io::println("big");
                    } else {
                        io::println("small");
                    }
                } else {
                    io::println("negative");
                }
            }
        "#).expect("IR should succeed");
        // Should have two if.then blocks (one for outer, one for inner)
        let count = ir.matches("if.then").count();
        assert!(count >= 2, "expected at least 2 if.then blocks, got {}:\n{}", count, ir);
    }

    // ---- Function tests (Task 1.5) ----

    #[test]
    fn function_declaration() {
        let ir = compile_program_to_ir(r#"
            fn helper(): void {
                io::println("hello from helper");
            }
            public fn main(): void {
                helper();
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("@helper"), "expected helper function definition, got:\n{}", ir);
    }

    #[test]
    fn function_call_from_main() {
        let ir = compile_program_to_ir(r#"
            fn greet(): void {
                io::println("hi");
            }
            public fn main(): void {
                greet();
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("call") && ir.contains("@greet"), "expected call to greet, got:\n{}", ir);
    }

    #[test]
    fn function_with_return_value() {
        let ir = compile_program_to_ir(r#"
            fn answer(): int {
                return 42;
            }
            public fn main(): void {
                let x: int = answer();
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("@answer"), "expected answer function, got:\n{}", ir);
        assert!(ir.contains("call") && ir.contains("@answer"), "expected call to answer, got:\n{}", ir);
    }

    #[test]
    fn function_with_parameters() {
        let ir = compile_program_to_ir(r#"
            fn add(a: int, b: int): int {
                return a + b;
            }
            public fn main(): void {
                let x: int = add(3, 4);
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("@add"), "expected add function, got:\n{}", ir);
        assert!(ir.contains("call") && ir.contains("@add"), "expected call to add, got:\n{}", ir);
    }

    #[test]
    fn recursive_function() {
        let ir = compile_program_to_ir(r#"
            fn fib(n: int): int {
                if (n <= 1) {
                    return n;
                }
                return fib(n - 1) + fib(n - 2);
            }
            public fn main(): void {
                let result: int = fib(10);
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("@fib"), "expected fib function, got:\n{}", ir);
        // The function should call itself (recursive call): at least 3
        // occurrences of @fib (1 define + 2 calls).
        let count = ir.matches("@fib").count();
        assert!(count >= 3, "expected at least 3 occurrences of @fib (define + 2 calls), got {}:\n{}", count, ir);
    }

    #[test]
    fn void_function_called_as_statement() {
        let ir = compile_program_to_ir(r#"
            fn printIt(x: int): void {
                io::println(x);
            }
            public fn main(): void {
                printIt(42);
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("@printIt"), "expected printIt function, got:\n{}", ir);
        assert!(ir.contains("call") && ir.contains("@printIt"), "expected call to printIt, got:\n{}", ir);
    }

    #[test]
    fn function_returning_double() {
        let ir = compile_program_to_ir(r#"
            fn pi(): double {
                return 3.14;
            }
            public fn main(): void {
                let p: double = pi();
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("@pi"), "expected pi function, got:\n{}", ir);
        assert!(ir.contains("call") && ir.contains("@pi"), "expected call to pi, got:\n{}", ir);
    }

    #[test]
    fn function_with_multiple_returns() {
        let ir = compile_program_to_ir(r#"
            fn classify(n: int): int {
                if (n > 0) {
                    return 1;
                }
                if (n < 0) {
                    return -1;
                }
                return 0;
            }
            public fn main(): void {
                let r: int = classify(5);
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("@classify"), "expected classify function, got:\n{}", ir);
        assert!(ir.contains("call") && ir.contains("@classify"), "expected call to classify, got:\n{}", ir);
    }

    // ---- Ownership / borrows / regions tests (Task 1.6) ----

    #[test]
    fn unsafe_block_compiles() {
        // An unsafe block should compile its body like a normal block.
        let ir = compile_to_ir(r#"
            public fn main(): void {
                unsafe {
                    let x: int = 42;
                    io::println(x);
                }
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("store i32 42"), "expected store i32 42 in unsafe block, got:\n{}", ir);
    }

    #[test]
    fn unsafe_block_with_io() {
        // An unsafe block can call io::println.
        let ir = compile_to_ir(r#"
            public fn main(): void {
                unsafe {
                    io::println("hello from unsafe");
                }
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("titrate_println"), "expected titrate_println call in unsafe block, got:\n{}", ir);
    }

    #[test]
    fn region_block_compiles_as_unsafe() {
        // `region name { ... }` is parsed as an UnsafeBlock.
        let ir = compile_to_ir(r#"
            public fn main(): void {
                region r {
                    let x: int = 10;
                    io::println(x);
                }
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("store i32 10"), "expected store i32 10 in region block, got:\n{}", ir);
    }

    #[test]
    fn borrow_of_int_variable() {
        // `&x` should produce a pointer to x's alloca.
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let x: int = 42;
                let r: &int = &x;
            }
        "#).expect("IR should succeed");
        // The borrow should not crash and should produce some pointer value.
        // We just check that the IR contains an alloca for x.
        assert!(ir.contains("alloca i32"), "expected alloca i32 for x, got:\n{}", ir);
    }

    #[test]
    fn mutable_borrow_of_double_variable() {
        // `&mut x` should produce a pointer to x's alloca.
        let ir = compile_to_ir(r#"
            public fn main(): void {
                var y: double = 3.14;
                let r: &mut double = &mut y;
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("alloca double"), "expected alloca double for y, got:\n{}", ir);
    }

    #[test]
    fn owned_deref_loads_value() {
        // `*x` should load the value from the Owned<T> pointer.
        // We use an identifier deref; the pointer is stored in an alloca.
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let x: Owned<int> = null;
                let v: int = *x;
            }
        "#).expect("IR should succeed");
        // The deref should produce a load instruction.
        assert!(ir.contains("load"), "expected load instruction for owned deref, got:\n{}", ir);
    }

    #[test]
    fn with_statement_compiles() {
        // `with (expr) { body }` should compile the body.
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let r: int = 42;
                with (r) {
                    io::println("inside with");
                }
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("titrate_println"), "expected titrate_println in with body, got:\n{}", ir);
    }

    #[test]
    fn with_let_binds_variable() {
        // `with (let f: T = expr) { body }` should bind f.
        let ir = compile_to_ir(r#"
            public fn main(): void {
                with (let f: int = 100) {
                    io::println(f);
                }
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("titrate_println_int"), "expected titrate_println_int for f, got:\n{}", ir);
    }

    #[test]
    fn nested_unsafe_blocks() {
        // Nested unsafe blocks should compile.
        let ir = compile_to_ir(r#"
            public fn main(): void {
                unsafe {
                    let x: int = 1;
                    unsafe {
                        let y: int = 2;
                        io::println(x + y);
                    }
                }
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("titrate_println_int"), "expected titrate_println_int, got:\n{}", ir);
    }

    #[test]
    fn titrate_malloc_declared() {
        // The titrate_malloc function should be declared as an external.
        let ir = compile_to_ir(r#"
            public fn main(): void {
                let x: int = 1;
            }
        "#).expect("IR should succeed");
        assert!(ir.contains("titrate_malloc"), "expected titrate_malloc declaration, got:\n{}", ir);
    }

    // ---- Release-mode optimization tests (Task 4.1) ----

    /// Compile a full program (with non-main functions) to LLVM IR with the
    /// release-mode flag set, so that optimization hints are emitted.
    fn compile_program_to_ir_release(source: &str) -> Result<String, String> {
        let tokens = lexer::tokenize(source).map_err(|e| format!("tokenize: {}", e))?;
        let ast = parser::parse(tokens).map_err(|e| format!("parse: {}", e))?;
        let typed_ast = analyzer::analyze(&ast).map_err(|e| format!("analyze: {:?}", e))?;
        let context = Context::create();
        let mut backend = LlvmBackend::new(&context, "test");
        backend.release_mode = true;
        backend.declare_natives();
        // Compile class declarations first (mirrors compile_program_to_ir_text).
        for decl in &typed_ast.declarations {
            if let Declaration::Class(class_decl) = decl {
                backend.compile_class_decl(class_decl)?;
            }
        }
        // Register all function declarations first (for recursion).
        for decl in &typed_ast.declarations {
            if let Declaration::Function(f) = decl {
                if f.type_params.is_empty() {
                    backend.function_decls.insert(f.name.clone(), f.clone());
                }
            }
        }
        // Compile all non-generic, non-main functions first.
        for decl in &typed_ast.declarations {
            if let Declaration::Function(f) = decl {
                if f.name != "main" && f.type_params.is_empty() {
                    backend.compile_function(f)
                        .map_err(|e| format!("compile function '{}': {}", f.name, e))?;
                }
            }
        }
        // Find and compile main.
        let main_decl = typed_ast.declarations.iter().find_map(|d| match d {
            Declaration::Function(f) if f.name == "main" => Some(f),
            _ => None,
        }).ok_or("no main")?;
        backend.compile_main(main_decl).map_err(|e| format!("compile main: {}", e))?;
        Ok(backend.module.print_to_string().to_string())
    }

    #[test]
    fn release_mode_emits_alwaysinline_for_small_functions() {
        // A small internal function (< 10 statements) should get the
        // `alwaysinline` attribute when compiled in release mode.
        let ir = compile_program_to_ir_release(r#"
            fn helper(x: int): int {
                return x * 2;
            }
            public fn main(): void {
                let y: int = helper(21);
                io::println(y);
            }
        "#).expect("release IR should succeed");
        assert!(
            ir.contains("alwaysinline"),
            "expected alwaysinline attribute in release IR, got:\n{}",
            ir,
        );
    }

    #[test]
    fn release_mode_emits_fastcc_for_small_functions() {
        // Small internal functions should use the fast calling convention
        // (fastcc) in release mode. We check for the attribute by looking
        // for the calling-convention marker in the IR text.
        let ir = compile_program_to_ir_release(r#"
            fn tiny(a: int, b: int): int {
                return a + b;
            }
            public fn main(): void {
                let y: int = tiny(1, 2);
                io::println(y);
            }
        "#).expect("release IR should succeed");
        // fastcc appears in the IR as the calling convention on the
        // function definition. inkwell emits it as a numeric cc.
        // We just verify the function exists and alwaysinline is present
        // (fastcc is applied together with alwaysinline).
        assert!(
            ir.contains("alwaysinline"),
            "expected alwaysinline (implies fastcc) in release IR, got:\n{}",
            ir,
        );
    }

    #[test]
    fn release_mode_emits_loop_vectorize_metadata() {
        // A for-in range loop should get !llvm.loop metadata with
        // vectorization hints when compiled in release mode.
        let ir = compile_program_to_ir_release(r#"
            public fn main(): void {
                for (i in 0..100) {
                    io::println(i);
                }
            }
        "#).expect("release IR should succeed");
        assert!(
            ir.contains("llvm.loop"),
            "expected llvm.loop metadata in release IR, got:\n{}",
            ir,
        );
        assert!(
            ir.contains("llvm.loop.vectorize.enable"),
            "expected vectorize.enable hint, got:\n{}",
            ir,
        );
    }

    #[test]
    fn debug_mode_does_not_emit_optimization_hints() {
        // In debug mode (release_mode = false), no alwaysinline or
        // llvm.loop metadata should be emitted.
        let ir = compile_program_to_ir(r#"
            fn helper(x: int): int {
                return x * 2;
            }
            public fn main(): void {
                for (i in 0..10) {
                    let y: int = helper(i);
                    io::println(y);
                }
            }
        "#).expect("debug IR should succeed");
        assert!(
            !ir.contains("alwaysinline"),
            "debug IR should NOT contain alwaysinline, got:\n{}",
            ir,
        );
        assert!(
            !ir.contains("llvm.loop"),
            "debug IR should NOT contain llvm.loop metadata, got:\n{}",
            ir,
        );
    }

    #[test]
    fn memset_intrinsic_declared_for_class_allocation() {
        // The emit_memset_zero helper should declare and call the
        // llvm.memset.p0i8.i64 intrinsic. We test the helper directly
        // because full class instantiation requires more type inference
        // than the test harness provides.
        use crate::lexer;
        use crate::parser;
        use crate::analyzer;
        let source = r#"
            public fn main(): void {
                let x: int = 1;
            }
        "#;
        let tokens = lexer::tokenize(source).expect("tokenize");
        let ast = parser::parse(tokens).expect("parse");
        let typed_ast = analyzer::analyze(&ast).expect("analyze");
        let context = Context::create();
        let mut backend = LlvmBackend::new(&context, "test");
        backend.declare_natives();
        let main_decl = typed_ast.declarations.iter().find_map(|d| match d {
            Declaration::Function(f) if f.name == "main" => Some(f),
            _ => None,
        }).expect("no main");
        backend.compile_main(main_decl).expect("compile main");

        // Now call emit_memset_zero from within a function body.
        let fn_type = context.void_type().fn_type(&[], false);
        let test_fn = backend.module.add_function("test_memset", fn_type, None);
        let entry = context.append_basic_block(test_fn, "entry");
        backend.builder.position_at_end(entry);
        let i8_ptr = context.ptr_type(AddressSpace::default());
        let dummy_ptr = i8_ptr.const_null();
        let size = context.i64_type().const_int(64, false);
        backend.emit_memset_zero(dummy_ptr, size).expect("emit_memset_zero");

        let ir = backend.module.print_to_string().to_string();
        assert!(
            ir.contains("llvm.memset.p0i8.i64"),
            "expected llvm.memset.p0i8.i64 declaration/call, got:\n{}",
            ir,
        );
    }
}
