// Function-call dispatch

use super::enum_codegen::emit_enum_construct;
use super::types as llvm_types;
use super::vtable::emit_direct_call;
use super::{fn_sig_has_type_param, ClosureSig, LlvmBackend};
use inkwell::module::Linkage;

use crate::ast::{Expr, Type};
use inkwell::types::BasicTypeEnum;
use inkwell::values::BasicValueEnum;

impl<'ctx> LlvmBackend<'ctx> {
    /// Compile a function call.
    pub(super) fn compile_call(
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
                        // Fallback: the value is a string struct even though
                        // type inference called it an int (e.g. a native call
                        // result whose return type was mis-inferred).
                        if v.is_struct_value() && self.is_string_struct(&v) {
                            let s = self.basic_to_string_value(v)?;
                            if method == "println" {
                                self.build_println_string(s)?;
                            } else {
                                self.build_print_string(s)?;
                            }
                        } else if method == "println" {
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

        // super.init(args) / super(args) / super.method(args): dispatch to the
        // parent class. The parent's fields are laid out first (after the
        // vtable pointer) in the derived struct, so the parent constructor or
        // method operates on the current instance at the correct offsets.
        let super_method = match callee {
            Expr::Super(_) => Some("init".to_string()),
            Expr::MemberAccess(obj, m, _) if matches!(&**obj, Expr::Super(_)) => Some(m.clone()),
            _ => None,
        };
        if let Some(method) = super_method {
            let current = self.current_class_name.clone().ok_or_else(|| {
                "codegen: super() call outside of a class constructor".to_string()
            })?;
            let current_info = self
                .class_infos
                .get(&current)
                .cloned()
                .ok_or_else(|| format!("codegen: current class '{}' not found", current))?;
            let parent_name = current_info.parent.clone().ok_or_else(|| {
                format!("codegen: class '{}' has no parent for super()", current)
            })?;
            let parent_info = self
                .class_infos
                .get(&parent_name)
                .cloned()
                .ok_or_else(|| format!("codegen: parent class '{}' not found", parent_name))?;
            let this_ptr = self.current_this.ok_or_else(|| {
                "codegen: super() call has no 'this' pointer".to_string()
            })?;
            if method == "init" {
                let ctor = parent_info.constructor.ok_or_else(|| {
                    format!("codegen: parent class '{}' has no constructor", parent_name)
                })?;
                let ctor_params = self.method_decl_params(&parent_name, "init");
                let arg_vals =
                    self.build_this_call_args(ctor, ctor_params.as_deref(), args)?;
                emit_direct_call(self.context, &self.builder, ctor, this_ptr, &arg_vals)?;
                let i32_ty = self.context.i32_type();
                return Ok(i32_ty.const_int(0, false).into());
            }
            let method_fn_name = format!("{}_{}", parent_name, method);
            let method_fn = self
                .module
                .get_function(&method_fn_name)
                .ok_or_else(|| {
                    format!(
                        "codegen: super method '{}.{}' not found",
                        parent_name, method
                    )
                })?;
            let method_params = self.method_decl_params(&parent_name, &method);
            let arg_vals =
                self.build_this_call_args(method_fn, method_params.as_deref(), args)?;
            return emit_direct_call(self.context, &self.builder, method_fn, this_ptr, &arg_vals);
        }

        // Enum construction: EnumName::Variant(args)
        if let Expr::StaticCall {
            class_name,
            method,
            args: static_args,
            ..
        } = callee
        {
            let enum_name = class_name.as_str();
            if let Some(enum_info) = self.enum_infos.get(enum_name).cloned() {
                let mut arg_vals: Vec<BasicValueEnum> = Vec::new();
                for arg in static_args {
                    arg_vals.push(self.compile_expr(arg)?);
                }
                return emit_enum_construct(
                    self.context,
                    &self.builder,
                    &self.module,
                    &enum_info,
                    method,
                    &arg_vals,
                );
            }
        }

        // Built-in `ok(value)` / `err(value)` / `Ok(value)` / `Err(value)` —
        // create Result<T, E> structs directly. The lowercase and capitalized
        // forms must NOT go through the native bridge, which returns a
        // handle-based TitrateValue that cannot round-trip a `{ i32, ptr }`
        // Result struct (mismatching the function's declared return type).
        if let Expr::Identifier(name, _) = callee {
            if name == "ok" || name == "err" || name == "Ok" || name == "Err" {
                return self.compile_result_ctor(name, args);
            }
        }

        // Bare enum variant construction: `North()`, `Ok(v)` handled above.
        // A variant name not declared as a function may belong to an enum.
        if let Expr::Identifier(name, _) = callee {
            if !self.functions.contains_key(name) && !self.function_decls.contains_key(name) {
                if let Some(enum_name) = self.enum_name_for_variant(name) {
                    if let Some(enum_info) = self.enum_infos.get(&enum_name).cloned() {
                        let mut arg_vals: Vec<BasicValueEnum> = Vec::new();
                        for arg in args {
                            arg_vals.push(self.compile_expr(arg)?);
                        }
                        return emit_enum_construct(
                            self.context,
                            &self.builder,
                            &self.module,
                            &enum_info,
                            name,
                            &arg_vals,
                        );
                    }
                }
            }
        }

        // Direct function call: identifier(args)
        if let Expr::Identifier(name, _) = callee {
            if let Some(&fn_val) = self.functions.get(name) {
                let param_types = fn_val.get_type().get_param_types();
                let decl_params = self.fn_decl_param_types(name);
                let mut arg_vals = Vec::with_capacity(args.len());
                for (i, arg) in args.iter().enumerate() {
                    // Container args share the caller's slot; others cast as before
                    // (e.g., i64 literal to i32 parameter).
                    let pty = if i < param_types.len() {
                        Some(match param_types[i] {
                            inkwell::types::BasicMetadataTypeEnum::ArrayType(t) => t.into(),
                            inkwell::types::BasicMetadataTypeEnum::FloatType(t) => t.into(),
                            inkwell::types::BasicMetadataTypeEnum::IntType(t) => t.into(),
                            inkwell::types::BasicMetadataTypeEnum::PointerType(t) => t.into(),
                            inkwell::types::BasicMetadataTypeEnum::StructType(t) => t.into(),
                            inkwell::types::BasicMetadataTypeEnum::VectorType(t) => t.into(),
                            _ => {
                                return Err(format!(
                                    "unsupported parameter type for function '{}'",
                                    name
                                ))
                            }
                        })
                    } else {
                        None
                    };
                    arg_vals.push(self.coerce_user_call_arg(
                        arg,
                        decl_params.as_ref().and_then(|p| p.get(i)),
                        pty,
                    )?);
                }
                let call = self
                    .builder
                    .build_call(fn_val, &arg_vals, "call")
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
                    let param_types: Vec<BasicTypeEnum> = fn_decl
                        .params
                        .iter()
                        .map(|p| self.user_param_llvm_type(&p.typ))
                        .collect::<Result<Vec<_>, _>>()?;
                    let fn_type = if fn_decl.params.is_empty() {
                        match self.sig_ret_llvm_type(fn_decl.return_type.as_ref())? {
                            Some(ret) => match ret {
                                BasicTypeEnum::IntType(t) => t.fn_type(&[], false),
                                BasicTypeEnum::FloatType(t) => t.fn_type(&[], false),
                                BasicTypeEnum::PointerType(t) => t.fn_type(&[], false),
                                BasicTypeEnum::StructType(t) => t.fn_type(&[], false),
                                BasicTypeEnum::ArrayType(t) => t.fn_type(&[], false),
                                _ => {
                                    return Err(format!(
                                        "unsupported return type for function '{}'",
                                        name
                                    ))
                                }
                            },
                            None => self.context.void_type().fn_type(&[], false),
                        }
                    } else {
                        let params: Vec<inkwell::types::BasicMetadataTypeEnum> =
                            param_types.iter().map(|t| (*t).into()).collect();
                        match self.sig_ret_llvm_type(fn_decl.return_type.as_ref())? {
                            Some(ret) => match ret {
                                BasicTypeEnum::IntType(t) => t.fn_type(&params, false),
                                BasicTypeEnum::FloatType(t) => t.fn_type(&params, false),
                                BasicTypeEnum::PointerType(t) => t.fn_type(&params, false),
                                BasicTypeEnum::StructType(t) => t.fn_type(&params, false),
                                BasicTypeEnum::ArrayType(t) => t.fn_type(&params, false),
                                _ => {
                                    return Err(format!(
                                        "unsupported return type for function '{}'",
                                        name
                                    ))
                                }
                            },
                            None => self.context.void_type().fn_type(&params, false),
                        }
                    };
                    self.module
                        .add_function(name, fn_type, Some(Linkage::Internal))
                };
                self.functions.insert(name.clone(), fn_val);
                let param_types = fn_val.get_type().get_param_types();
                let decl_params: Vec<Type> =
                    fn_decl.params.iter().map(|p| p.typ.clone()).collect();
                let mut arg_vals = Vec::with_capacity(args.len());
                for (i, arg) in args.iter().enumerate() {
                    // Container args share the caller's slot; others cast as before
                    // (e.g., i64 literal to i32 parameter).
                    let pty = if i < param_types.len() {
                        Some(match param_types[i] {
                            inkwell::types::BasicMetadataTypeEnum::ArrayType(t) => t.into(),
                            inkwell::types::BasicMetadataTypeEnum::FloatType(t) => t.into(),
                            inkwell::types::BasicMetadataTypeEnum::IntType(t) => t.into(),
                            inkwell::types::BasicMetadataTypeEnum::PointerType(t) => t.into(),
                            inkwell::types::BasicMetadataTypeEnum::StructType(t) => t.into(),
                            inkwell::types::BasicMetadataTypeEnum::VectorType(t) => t.into(),
                            _ => {
                                return Err(format!(
                                    "unsupported parameter type for function '{}'",
                                    name
                                ))
                            }
                        })
                    } else {
                        None
                    };
                    arg_vals.push(self.coerce_user_call_arg(
                        arg,
                        decl_params.get(i),
                        pty,
                    )?);
                }
                let call = self
                    .builder
                    .build_call(fn_val, &arg_vals, "call")
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

        // Module-level function lookup: the function may already be declared in the
        // LLVM module (e.g., a top-level function compiled earlier or declared by
        // another module). This catches forward references that aren't in function_decls.
        if let Expr::Identifier(name, _) = callee {
            if let Some(fn_val) = self.module.get_function(name) {
                let param_types = fn_val.get_type().get_param_types();
                let decl_params = self.fn_decl_param_types(name);
                let mut arg_vals = Vec::with_capacity(args.len());
                for (i, arg) in args.iter().enumerate() {
                    // Container args share the caller's slot; others cast as before
                    // (e.g., i64 literal to i32 parameter).
                    let pty = if i < param_types.len() {
                        Some(match param_types[i] {
                            inkwell::types::BasicMetadataTypeEnum::ArrayType(t) => t.into(),
                            inkwell::types::BasicMetadataTypeEnum::FloatType(t) => t.into(),
                            inkwell::types::BasicMetadataTypeEnum::IntType(t) => t.into(),
                            inkwell::types::BasicMetadataTypeEnum::PointerType(t) => t.into(),
                            inkwell::types::BasicMetadataTypeEnum::StructType(t) => t.into(),
                            inkwell::types::BasicMetadataTypeEnum::VectorType(t) => t.into(),
                            _ => {
                                return Err(format!(
                                    "unsupported parameter type for function '{}'",
                                    name
                                ))
                            }
                        })
                    } else {
                        None
                    };
                    arg_vals.push(self.coerce_user_call_arg(
                        arg,
                        decl_params.as_ref().and_then(|p| p.get(i)),
                        pty,
                    )?);
                }
                let call = self
                    .builder
                    .build_call(fn_val, &arg_vals, "call")
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

        // Direct invocation of a closure value held by a local: `f(args)`.
        // User bindings take precedence over natives with the same name.
        if let Expr::Identifier(name, _) = callee {
            if let Some(local) = self.locals.get(name.as_str()).cloned() {
                if let Some(sig) = local.closure_sig.clone() {
                    return self.compile_closure_call(&local, &sig, args);
                }
                // A function-typed parameter (or any binding whose declared
                // type retains the signature) carries no tracked literal
                // signature; rebuild it from the declared `fn` type so the
                // indirect call matches the trampoline exactly.
                if let Some(full) = local.full_type.clone() {
                    if full.name() == "fn" && !full.params().is_empty() {
                        let parts = full.params();
                        let (sig_params, ret) = parts.split_at(parts.len() - 1);
                        if sig_params.iter().any(fn_sig_has_type_param)
                            || fn_sig_has_type_param(&ret[0])
                        {
                            return Err(format!(
                                "codegen: cannot invoke fn value '{}' with generic signature natively",
                                name
                            ));
                        }
                        let sig = ClosureSig {
                            params: sig_params.to_vec(),
                            ret: ret[0].clone(),
                            returns_param: None,
                        };
                        return self.compile_closure_call(&local, &sig, args);
                    }
                }
            }
        }
        // Generic top-level function: deduce concrete type arguments
        // from the call arguments, instantiate, and call directly.
        // (Outside the locals block above: a generic name never denotes
        // a local binding.)
        if let Expr::Identifier(name, _) = callee {
            if let Some(overloads) = self.generic_fn_decls.get(name.as_str()).cloned() {
                let overload = overloads
                    .iter()
                    .find(|d| d.params.len() == args.len())
                    .or_else(|| overloads.first())
                    .cloned();
                if let Some(decl) = overload {
                    let type_param_names: Vec<String> = decl
                        .type_params
                        .iter()
                        .map(|tp| tp.name.clone())
                        .collect();
                    let formals: Vec<Type> = decl
                        .params
                        .iter()
                        .map(|p| p.typ.clone())
                        .collect();
                    let actuals: Vec<Type> =
                        args.iter().map(|a| self.infer_expr_type(a)).collect();
                    if let Some(deduced) =
                        Self::deduce_type_args(&formals, &actuals, &type_param_names)
                    {
                        let mangled =
                            self.instantiate_llvm_function(name, &deduced, args.len())?;
                        if let Some(fn_val) = self.module.get_function(&mangled) {
                            // Shape args with the substituted (concrete)
                            // parameter types, not the generic template's.
                            let decl_params: Vec<Type> = self
                                .function_decls
                                .get(&mangled)
                                .map(|d| d.params.iter().map(|p| p.typ.clone()).collect())
                                .unwrap_or_default();
                            let mut arg_vals = Vec::with_capacity(args.len());
                            for (i, arg) in args.iter().enumerate() {
                                arg_vals.push(self.coerce_user_call_arg(
                                    arg,
                                    decl_params.get(i),
                                    None,
                                )?);
                            }
                            let call = self
                                .builder
                                .build_call(fn_val, &arg_vals, "call")
                                .map_err(|e| {
                                    format!("build_call '{}' failed: {:?}", mangled, e)
                                })?;
                            let ret_ty = self.sig_ret_llvm_type(
                                self.function_decls
                                    .get(&mangled)
                                    .and_then(|d| d.return_type.as_ref()),
                            )?;
                            if ret_ty.is_some() {
                                match call.try_as_basic_value() {
                                    inkwell::values::ValueKind::Basic(v) => return Ok(v),
                                    _ => {
                                        return Err(format!(
                                            "function '{}' did not return a value",
                                            mangled
                                        ))
                                    }
                                }
                            } else {
                                let i32_ty = self.context.i32_type();
                                return Ok(i32_ty.const_int(0, false).into());
                            }
                        }
                    }
                }
            }
        }
        if let Some(v) = self.compile_call_member(callee, args)? {
            return Ok(v);
        }

        Err(format!("codegen: unsupported call target: {:?}", callee))
    }
}
