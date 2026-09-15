// Static calls and array builtins

use super::native_bridge;
use super::types as llvm_types;
use super::LlvmBackend;
use crate::ast::{Expr, Type};
use inkwell::types::BasicTypeEnum;
use inkwell::values::BasicValueEnum;

impl<'ctx> LlvmBackend<'ctx> {
    /// Compile a StaticCall expression: Class.method(args).
    /// This maps to native function calls like Integer_parseInt, String_length, etc.
    pub(super) fn compile_static_call(
        &mut self,
        class_name: &str,
        method: &str,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        // Handle io::println and io::print
        if class_name == "io" && (method == "println" || method == "print") {
            if args.len() != 1 {
                return Err(format!(
                    "codegen: io::{} expects 1 argument, got {}",
                    method,
                    args.len()
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
                return self.compile_native_call(&native_name, &arg_vals, &arg_types);
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
                    return self.compile_native_call(&alt_name, &arg_vals, &arg_types);
                }
            }
        }

        // File.exists -> Fs_exists mapping
        if class_name == "File" && method == "exists" {
            let mut arg_vals: Vec<BasicValueEnum> = Vec::new();
            let mut arg_types: Vec<Type> = Vec::new();
            for arg in args {
                let arg_ty = self.infer_expr_type(arg);
                let val = self.compile_expr(arg)?;
                arg_vals.push(val);
                arg_types.push(arg_ty);
            }
            return self.compile_native_call("Fs_exists", &arg_vals, &arg_types);
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
            return self.compile_native_call(&native_name, &arg_vals, &arg_types);
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
            return self.compile_native_call("toString", &arg_vals, &arg_types);
        }
        Err(format!(
            "codegen: unsupported static call: {}.{}",
            class_name, method
        ))
    }

    /// Returns true if the given LLVM value is a string struct { i64, ptr }.
    pub(super) fn is_string_struct(&self, val: &BasicValueEnum<'ctx>) -> bool {
        if let BasicValueEnum::StructValue(sv) = val {
            let st = sv.get_type();
            if st.count_fields() == 2 {
                matches!(
                    (st.get_field_type_at_index(0), st.get_field_type_at_index(1)),
                    (
                        Some(BasicTypeEnum::IntType(_)),
                        Some(BasicTypeEnum::PointerType(_))
                    )
                )
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Returns true if the given Titrate type is an unsigned integer type.
    pub(super) fn is_unsigned_type(ty: &Type) -> bool {
        matches!(ty.name(), "uvast" | "u8" | "u16" | "u32" | "u64" | "size")
    }
}
