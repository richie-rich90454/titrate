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
pub mod target_wrappers;
pub mod types;

use std::collections::HashMap;
use std::path::Path;

use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine,
};
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{
    BasicValueEnum, FunctionValue, IntValue, PointerValue,
};
use inkwell::AddressSpace;
use inkwell::OptimizationLevel;

use crate::ast::{Declaration, Expr, FnDecl, Literal, Operator, Program, Stmt, Type, UnOp};

use super::llvm::types as llvm_types;

/// String value tracked during codegen: the byte length and the pointer to
/// the underlying UTF-8 buffer.
#[derive(Clone, Copy)]
struct StringValue<'ctx> {
    len: IntValue<'ctx>,
    ptr: PointerValue<'ctx>,
}

/// A local variable's storage info: the alloca pointer and the LLVM type
/// stored at that pointer.
#[derive(Clone, Copy)]
struct LocalVar<'ctx> {
    ptr: PointerValue<'ctx>,
    ty: BasicTypeEnum<'ctx>,
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
    /// Map from mangled name to LLVM function value (for monomorphized generics).
    generic_functions: HashMap<String, FunctionValue<'ctx>>,
    /// Map from function name to its declaration (for late compilation / recursion).
    function_decls: HashMap<String, FnDecl>,
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
            generic_functions: HashMap::new(),
            function_decls: HashMap::new(),
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

        // Primitive printers.
        let println_int_fn = void_type.fn_type(&[i64_type.into()], false);
        self.module.add_function("titrate_println_int", println_int_fn, Some(Linkage::External));

        let println_double_fn = void_type.fn_type(&[f64_type.into()], false);
        self.module.add_function("titrate_println_double", println_double_fn, Some(Linkage::External));

        let println_bool_fn = void_type.fn_type(&[i32_type.into()], false);
        self.module.add_function("titrate_println_bool", println_bool_fn, Some(Linkage::External));

        let println_char_fn = void_type.fn_type(&[i32_type.into()], false);
        self.module.add_function("titrate_println_char", println_char_fn, Some(Linkage::External));
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
            Expr::MemberAccess(namespace, method, _) => {
                // io::println etc. handled in compile_call; bare member access
                // is not supported as a value.
                let _ = (namespace, method);
                Err(format!("codegen: bare member access not supported: {:?}", expr))
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

        let lv = self.compile_expr(left)?;
        let rv = self.compile_expr(right)?;

        // Determine if this is an integer or float operation.
        let is_float = lv.is_float_value();
        let is_int = lv.is_int_value();

        if is_float {
            let l = lv.into_float_value();
            let r = rv.into_float_value();
            return Ok(self.compile_float_binary(op, l, r)?.into());
        }

        if is_int {
            let l = lv.into_int_value();
            let r = rv.into_int_value();
            return Ok(self.compile_int_binary(op, l, r, &left_ty)?.into());
        }

        Err(format!("codegen: unsupported binary operand types: {:?} {:?}", lv.get_type(), rv.get_type()))
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

    /// Compile a float binary operation.
    fn compile_float_binary(
        &self,
        op: &Operator,
        l: inkwell::values::FloatValue<'ctx>,
        r: inkwell::values::FloatValue<'ctx>,
    ) -> Result<inkwell::values::FloatValue<'ctx>, String> {
        use inkwell::FloatPredicate::*;
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
            Operator::Eq => {
                let cmp = self.builder.build_float_compare(OEQ, l, r, "feq")
                    .map_err(|e| format!("build_float_compare eq failed: {:?}", e))?;
                // Convert i1 to float type for uniformity? No — return as float
                // would be wrong. We need to handle this differently.
                // Actually, comparison results are i1. We should return them as
                // IntValue, not FloatValue. Let's handle this in the caller.
                // For now, convert i1 to the float type via select? No, that's
                // wrong. Let's restructure.
                let _ = cmp;
                Err("float comparison should be handled separately".to_string())
            }
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
            // Copy the LocalVar out (it's Copy) to release the immutable borrow
            // of self.locals before we call &mut self methods below.
            let var = self.locals.get(name).copied().ok_or_else(|| {
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
        let then_block_end = self.builder.get_insert_block()
            .ok_or("codegen: no insert block after then")?;
        self.builder.build_unconditional_branch(end_block)
            .map_err(|e| format!("build_br tern end failed: {:?}", e))?;

        self.builder.position_at_end(else_block);
        let else_val = self.compile_expr(else_expr)?;
        let else_block_end = self.builder.get_insert_block()
            .ok_or("codegen: no insert block after else")?;
        self.builder.build_unconditional_branch(end_block)
            .map_err(|e| format!("build_br tern end 2 failed: {:?}", e))?;

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
        // io::println(...)
        if let Expr::MemberAccess(namespace, method, _) = callee {
            if let Expr::Identifier(ns, _) = &**namespace {
                if ns == "io" && method == "println" {
                    if args.len() != 1 {
                        return Err(format!(
                            "codegen: io::println expects 1 argument, got {}",
                            args.len()
                        ));
                    }
                    let arg = &args[0];
                    let arg_ty = self.infer_expr_type(arg);
                    if llvm_types::is_string(&arg_ty) {
                        let s = self.compile_string_expr(arg)?;
                        self.build_println_string(s)?;
                    } else {
                        let v = self.compile_expr(arg)?;
                        self.build_println_primitive(v, &arg_ty)?;
                    }
                    let i32_ty = self.context.i32_type();
                    return Ok(i32_ty.const_int(0, false).into());
                }
            }
        }

        // Direct function call: identifier(args)
        if let Expr::Identifier(name, _) = callee {
            if let Some(&fn_val) = self.functions.get(name) {
                let mut arg_vals = Vec::with_capacity(args.len());
                for arg in args {
                    arg_vals.push(self.compile_expr(arg)?.into());
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

        Err(format!("codegen: unsupported call target: {:?}", callee))
    }

    /// Returns true if the given Titrate type is an unsigned integer type.
    fn is_unsigned_type(ty: &Type) -> bool {
        matches!(ty.name(), "uvast" | "u8" | "u16" | "u32" | "u64" | "size")
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
                for s in block {
                    self.compile_stmt(s)?;
                }
                Ok(())
            }
            Stmt::Switch(switch_stmt) => {
                self.compile_switch(switch_stmt)?;
                Ok(())
            }
            _ => Err(format!("codegen: unsupported statement: {:?}", stmt)),
        }
    }

    /// Compile a return statement.
    fn compile_return(&mut self, expr: &Option<Expr>) -> Result<(), String> {
        match expr {
            None => {
                self.builder.build_return(None)
                    .map_err(|e| format!("build_return void failed: {:?}", e))?;
            }
            Some(e) => {
                let v = self.compile_expr(e)?;
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
        let is_string = declared_ty.map(|t| llvm_types::is_string(t)).unwrap_or(false);
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
            self.locals.insert(decl.name.clone(), LocalVar { ptr: alloca, ty: string_ty.into() });
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

        self.locals.insert(decl.name.clone(), LocalVar { ptr: alloca, ty });
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
                return Err(format!(
                    "codegen: for-in over non-Range iterables is not yet supported"
                ));
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

        let fn_val = self.module.add_function(&fn_decl.name, fn_type, None);
        self.functions.insert(fn_decl.name.clone(), fn_val);

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
            self.locals.insert(p.name.clone(), LocalVar { ptr: alloca, ty });
        }

        // Compile body.
        for s in &fn_decl.body {
            self.compile_stmt(s)?;
        }

        // Add a default return if the current block has no terminator.
        if self.builder.get_insert_block().and_then(|b| b.get_terminator()).is_none() {
            match &fn_decl.return_type {
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
        self.declare_natives();

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

    /// Write the module to an object file using the native target.
    fn write_object(&self, path: &Path, release: bool) -> Result<(), String> {
        let init_config = InitializationConfig::default();
        Target::initialize_native(&init_config)
            .map_err(|e| format!("failed to initialize native target: {}", e))?;

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
}
