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
            if matches!(arg_ty, Type::Ref(_) | Type::MutRef(_)) {
                // Native lowers borrows to raw pointers; print the VM's
                // Value::Ref display form instead of casting the pointer.
                let idx = match arg {
                    Expr::Identifier(name, _) => self.ref_indices.get(name).copied().unwrap_or(0),
                    _ => 0,
                };
                let s = self.make_string_global(&format!("ref({})", idx));
                if method == "println" {
                    self.build_println_string(s)?;
                } else {
                    self.build_print_string(s)?;
                }
                let i32_ty = self.context.i32_type();
                return Ok(i32_ty.const_int(0, false).into());
            }
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

        // These are stdlib wrappers that return compiled class objects, not
        // opaque native handles. Route to the wrapper so callers get a usable
        // instance (Json.parse converts the handle; File.open builds a File).
        if ((class_name == "Json" && (method == "parse" || method == "stringify"))
            || (class_name == "File" && method == "open"))
            && (self.functions.contains_key(method) || self.function_decls.contains_key(method))
        {
            let saved = self.call_module_hint.replace(class_name.to_string());
            let result = self.compile_call(
                &Expr::Identifier(method.to_string(), crate::ast::Span::unknown()),
                args,
            );
            self.call_module_hint = saved;
            return result;
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
            let result = self.compile_native_call(&native_name, &arg_vals, &arg_types)?;
            // The native serializer emits spaces after ':' and ','; the VM's
            // JsonValue.toString() emits compact JSON. Normalize the spacing.
            if native_name == "Json_stringify" && result.is_struct_value() {
                let sv = self.basic_to_string_value(result)?;
                let compact = self.compact_json_string(sv)?;
                return self.string_value_to_basic(compact);
            }
            return Ok(result);
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
        // Static-style call to a method on the current class, e.g.
        // `Hash._pow16(i)` inside class Hash. The method is compiled as
        // `Class_method` and takes `this` as its first argument.
        let method_fn_name = format!("{}_{}", class_name, method);
        if let Some(this) = self.current_this {
            if let Some(fn_val) = self.module.get_function(&method_fn_name) {
                let mut arg_vals: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> =
                    vec![this.into()];
                for arg in args {
                    arg_vals.push(self.compile_expr(arg)?.into());
                }
                let call = self.builder.build_call(fn_val, &arg_vals, "call")
                    .map_err(|e| format!("build_call '{}' failed: {:?}", method_fn_name, e))?;
                match call.try_as_basic_value() {
                    inkwell::values::ValueKind::Basic(v) => return Ok(v),
                    _ => return Ok(self.context.i32_type().const_zero().into()),
                }
            }
        }
        // Module-style calls on a top-level function: `Hmac.sha256(k, m)`
        // resolves to the imported `sha256` function. Reuse the ordinary call
        // dispatch, which handles direct calls and forward references. The hint
        // makes bare-name resolution pick the overload declared in `class_name`.
        if self.functions.contains_key(method) || self.function_decls.contains_key(method) {
            let saved = self.call_module_hint.replace(class_name.to_string());
            let result = self.compile_call(
                &Expr::Identifier(method.to_string(), crate::ast::Span::unknown()),
                args,
            );
            self.call_module_hint = saved;
            return result;
        }
        // Static-style call to a class method from outside the class (e.g.
        // `Hash.hashBytes(k, d)` in a top-level function). Class methods are
        // compiled as `Class_method` with a leading `this` pointer; with no
        // instance available, pass a null `this`.
        let method_fn_name = format!("{}_{}", class_name, method);
        let argc = args.len() + 1;
        if let Some(fn_val) = self.resolve_user_fn(&method_fn_name, argc) {
            let i8_ptr = self.context.ptr_type(inkwell::AddressSpace::default());
            let mut arg_vals: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> =
                vec![i8_ptr.const_null().into()];
            for arg in args {
                arg_vals.push(self.compile_expr(arg)?.into());
            }
            let call = self.builder.build_call(fn_val, &arg_vals, "call")
                .map_err(|e| format!("build_call '{}' failed: {:?}", method_fn_name, e))?;
            return match call.try_as_basic_value() {
                inkwell::values::ValueKind::Basic(v) => Ok(v),
                _ => Ok(self.context.i32_type().const_zero().into()),
            };
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
