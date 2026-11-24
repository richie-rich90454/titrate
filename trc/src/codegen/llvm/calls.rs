// Function-call dispatch

use super::enum_codegen::emit_enum_construct;
use super::native_bridge;
use super::types as llvm_types;
use super::vtable::emit_direct_call;
use super::{fn_sig_has_type_param, ClosureSig, LlvmBackend, TcoTrampoline};
use inkwell::module::Linkage;

use crate::ast::{Expr, Type};
use inkwell::types::{BasicMetadataTypeEnum, BasicTypeEnum};
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum, FunctionValue};

impl<'ctx> LlvmBackend<'ctx> {
    fn check_call_args(
        f: inkwell::values::FunctionValue<'ctx>,
        args: &[BasicMetadataValueEnum<'ctx>],
        name: &str,
    ) -> Result<(), String> {
        let expected = f.get_type().get_param_types();
        let fn_name = f.get_name().to_str().unwrap_or("?").to_string();
        if expected.len() != args.len() && !f.get_type().is_var_arg() {
            return Err(format!("codegen: bad call '{}' to '{}': expects {} args, got {}",
                name, fn_name, expected.len(), args.len()));
        }
        for (i, (want, got)) in expected.iter().zip(args.iter()).enumerate() {
            let got = match got {
                BasicMetadataValueEnum::ArrayValue(v) => Some(v.get_type().into()),
                BasicMetadataValueEnum::IntValue(v) => Some(v.get_type().into()),
                BasicMetadataValueEnum::FloatValue(v) => Some(v.get_type().into()),
                BasicMetadataValueEnum::PointerValue(v) => Some(v.get_type().into()),
                BasicMetadataValueEnum::StructValue(v) => Some(v.get_type().into()),
                BasicMetadataValueEnum::VectorValue(v) => Some(v.get_type().into()),
                BasicMetadataValueEnum::ScalableVectorValue(v) => Some(v.get_type().into()),
                BasicMetadataValueEnum::MetadataValue(_) => None,
            };
            let Some(got): Option<BasicMetadataTypeEnum<'ctx>> = got else { continue };
            if want.print_to_string() != got.print_to_string() {
                return Err(format!(
                    "codegen: bad call '{}' to '{}': arg {} expected {}, got {}",
                    name,
                    fn_name,
                    i,
                    want.print_to_string(),
                    got.print_to_string()
                ));
            }
        }
        Ok(())
    }

    /// Produce a well-formed zero value for a declared Titrate type. Used where
    /// a generic-typed expression cannot be lowered to a concrete native value.
    fn default_value_for_type(&self, ty: &Type) -> Result<BasicValueEnum<'ctx>, String> {
        match self.sig_ret_llvm_type(Some(ty))? {
            None => Ok(self.context.i32_type().const_zero().into()),
            Some(BasicTypeEnum::IntType(t)) => Ok(t.const_zero().into()),
            Some(BasicTypeEnum::FloatType(t)) => Ok(t.const_zero().into()),
            Some(BasicTypeEnum::PointerType(t)) => Ok(t.const_null().into()),
            Some(BasicTypeEnum::StructType(t)) => Ok(t.const_zero().into()),
            Some(BasicTypeEnum::ArrayType(t)) => Ok(t.const_zero().into()),
            Some(BasicTypeEnum::VectorType(t)) => Ok(t.const_zero().into()),
            Some(BasicTypeEnum::ScalableVectorType(t)) => Ok(t.const_zero().into()),
        }
    }

    /// Instantiate and call a generic top-level function, honoring explicit
    /// type arguments when present. Returns `None` when the name is not a
    /// resolvable generic function so the caller can try other dispatch paths.
    fn compile_generic_fn_call(
        &mut self,
        name: &str,
        args: &[Expr],
        explicit_type_args: &[Type],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        let overloads = match self.generic_fn_decls.get(name).cloned() {
            Some(o) => o,
            None => return Ok(None),
        };
        let overload = overloads
            .iter()
            .find(|d| d.params.len() == args.len())
            .or_else(|| overloads.first())
            .cloned();
        let decl = match overload {
            Some(d) => d,
            None => return Ok(None),
        };
        let type_param_names: Vec<String> = decl
            .type_params
            .iter()
            .map(|tp| tp.name.clone())
            .collect();
        let formals: Vec<Type> = decl.params.iter().map(|p| p.typ.clone()).collect();
        let actuals: Vec<Type> = args.iter().map(|a| self.infer_expr_type(a)).collect();
        let deduced = if !explicit_type_args.is_empty()
            && explicit_type_args.len() == type_param_names.len()
        {
            Some(explicit_type_args.to_vec())
        } else {
            Self::deduce_type_args(&formals, &actuals, &type_param_names)
        };
        let deduced = match deduced {
            Some(d) => d,
            None => return Ok(None),
        };
        let mangled = self.instantiate_llvm_function(name, &deduced, args.len())?;
        let fn_val = match self.module.get_function(&mangled) {
            Some(f) => f,
            None => return Ok(None),
        };
        // Shape args with the substituted (concrete) parameter types, not the
        // generic template's.
        let decl_params: Vec<Type> = self
            .function_decls
            .get(&mangled)
            .map(|d| d.params.iter().map(|p| p.typ.clone()).collect())
            .unwrap_or_default();
        let mut arg_vals = Vec::with_capacity(args.len());
        for (i, arg) in args.iter().enumerate() {
            arg_vals.push(self.coerce_user_call_arg(arg, decl_params.get(i), None)?);
        }
        Self::check_call_args(fn_val, &arg_vals, "direct")?;
        let call = self
            .builder
            .build_call(fn_val, &arg_vals, "call")
            .map_err(|e| format!("build_call '{}' failed: {:?}", mangled, e))?;
        let ret_ty = self.sig_ret_llvm_type(
            self.function_decls
                .get(&mangled)
                .and_then(|d| d.return_type.as_ref()),
        )?;
        if ret_ty.is_some() {
            match call.try_as_basic_value() {
                inkwell::values::ValueKind::Basic(v) => Ok(Some(v)),
                _ => Err(format!("function '{}' did not return a value", mangled)),
            }
        } else {
            Ok(Some(self.context.i32_type().const_int(0, false).into()))
        }
    }

    /// Compile a function call without explicit type arguments.
    pub(super) fn compile_call(
        &mut self,
        callee: &Expr,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        self.compile_call_with_type_args(callee, args, &[])
    }

    /// Compile a function call, honouring explicit generic type arguments when
    /// present and falling back to deduction from the call arguments.
    pub(super) fn compile_call_with_type_args(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        explicit_type_args: &[Type],
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
                    if matches!(arg_ty, Type::Ref(_) | Type::MutRef(_)) {
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
                // Other io:: namespace calls are VM built-ins. stdin reads are
                // bridged through the IO_readLine / IO_readAll natives so native
                // builds see the same input as the bytecode VM.
                if ns == "io" {
                    match method.as_str() {
                        "readLine" | "readAll" => {
                            let native = if method == "readLine" {
                                "IO_readLine"
                            } else {
                                "IO_readAll"
                            };
                            let arg_types: Vec<Type> =
                                args.iter().map(|a| self.infer_expr_type(a)).collect();
                            let mut arg_vals = Vec::with_capacity(args.len());
                            for arg in args {
                                arg_vals.push(self.compile_expr(arg)?);
                            }
                            return self.compile_native_call_declared(
                                native,
                                &arg_vals,
                                &arg_types,
                                None,
                            );
                        }
                        "stderr" | "eprintln" | "eprint" => {
                            return Ok(self.context.i32_type().const_zero().into());
                        }
                        _ => {}
                    }
                }
            }
        }

        // Native-backed helpers like `toString` have many same-arity overloads
        // (one per primitive type) that collide under the bare name. When none
        // matches the argument types exactly, use the native bridge, which
        // marshals by each value's actual type and cannot bind the wrong LLVM
        // signature. This also covers calls from class methods compiled before
        // top-level functions are pre-declared, when `fn_typed` is still empty.
        if let Expr::Identifier(name, _) = callee {
            // Result constructors are also registered natives but must build a
            // `{ i32, ptr }` Result struct, not a handle-based TitrateValue.
            if native_bridge::is_native_function(name)
                && !matches!(name.as_str(), "ok" | "err" | "Ok" | "Err")
            {
                let arg_types: Vec<Type> = args.iter().map(|a| self.infer_expr_type(a)).collect();
                if self.resolve_user_fn_typed(name, &arg_types).is_none() {
                    // Json.parse uses the native result only as an optimization
                    // and falls back to its Titrate parser when it is null. The
                    // native result is an opaque handle that this backend cannot
                    // convert into a compiled JsonValue, so force the fallback
                    // by returning null. This preserves output exactly; only the
                    // parse path is slower on the native backend.
                    // `Json.parse` may be mangled (e.g. `parse__d2`) when other
                    // modules define same-signature `parse` functions, so match
                    // the current LLVM symbol back to its source declaration.
                    if name == "Json_parse"
                        && self
                            .builder
                            .get_insert_block()
                            .and_then(|b| b.get_parent())
                            .and_then(|f| f.get_name().to_str().ok().map(|s| s.to_string()))
                            .is_some_and(|cur| {
                                self.fn_decl_symbols
                                    .iter()
                                    .any(|(d, sym)| d.name == "parse" && *sym == cur)
                            })
                    {
                        return Ok(self.context.i64_type().const_zero().into());
                    }
                    let mut arg_vals = Vec::with_capacity(args.len());
                    for arg in args {
                        arg_vals.push(self.compile_expr(arg)?);
                    }
                    return self.compile_native_call_declared(
                        name,
                        &arg_vals,
                        &arg_types,
                        None,
                    );
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

        // Direct function call: identifier(args). A same-named class method
        // (e.g. `Hash.md5` mangled to `Hash_md5`) can collide with a bare native
        // of the same name; only use the function when the arity matches.
        if let Expr::Identifier(name, _) = callee {
            let arg_types: Vec<Type> = args.iter().map(|a| self.infer_expr_type(a)).collect();
            let fn_val = self
                .resolve_user_fn_typed(name, &arg_types)
                .or_else(|| self.resolve_user_fn(name, args.len()));
            if let Some(fn_val) = fn_val {
                let param_types = fn_val.get_type().get_param_types();
                let decl_params = self
                    .fn_decl_param_types_typed(name, &arg_types)
                    .or_else(|| self.fn_decl_param_types_arity(name, args.len()));
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
                Self::check_call_args(fn_val, &arg_vals, "direct")?;
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
            let arg_types: Vec<Type> = args.iter().map(|a| self.infer_expr_type(a)).collect();
            if let Some(fn_decl) = self
                .fn_decl_typed
                .get(&(name.clone(), Self::fn_type_key(&arg_types)))
                .or_else(|| self.fn_decl_overloads.get(&(name.clone(), args.len())))
                .or_else(|| self.function_decls.get(name))
                .cloned()
            {
                let fn_val = if let Some(existing) = self.resolve_user_fn(name, args.len()) {
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
                self.record_fn(name.clone(), fn_val);
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
                Self::check_call_args(fn_val, &arg_vals, "direct")?;
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
            let arg_types: Vec<Type> = args.iter().map(|a| self.infer_expr_type(a)).collect();
            let fn_val = self
                .resolve_user_fn_typed(name, &arg_types)
                .or_else(|| self.resolve_user_fn(name, args.len()));
            if let Some(fn_val) = fn_val {
                let param_types = fn_val.get_type().get_param_types();
                let decl_params = self
                    .fn_decl_param_types_typed(name, &arg_types)
                    .or_else(|| self.fn_decl_param_types_arity(name, args.len()));
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
                Self::check_call_args(fn_val, &arg_vals, "direct")?;
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
                            // A generic closure signature has no concrete native
                            // calling convention. Such invocations only appear in
                            // imported default methods that are compiled but not
                            // called; yield a default value to keep IR valid.
                            return self.default_value_for_type(&ret[0]);
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
            if let Some(v) = self.compile_generic_fn_call(name, args, explicit_type_args)? {
                return Ok(v);
            }
        }

        // Module-style call to a generic function: `Module.fn(args)`.
        // Resolve via generic_fn_decls so explicit type arguments survive;
        // the generic name denotes the function, not a field on the module.
        if let Expr::MemberAccess(obj, method, _) = callee {
            if let Expr::Identifier(module, _) = obj.as_ref() {
                if self.generic_fn_decls.contains_key(method.as_str()) {
                    let saved = self.call_module_hint.replace(module.clone());
                    let result = self.compile_generic_fn_call(method, args, explicit_type_args);
                    self.call_module_hint = saved;
                    if let Some(v) = result? {
                        return Ok(v);
                    }
                }
            }
        }

        if let Some(v) = self.compile_call_member(callee, args)? {
            return Ok(v);
        }

        // Bare call to a method on the current class (implicit `this`), e.g.
        // `this._singletonMap(...)` written as `_singletonMap(...)`.
        if let Expr::Identifier(name, _) = callee {
            if let (Some(this), Some(class)) = (self.current_this, self.current_class_name.as_ref()) {
                let method_fn_name = format!("{}_{}", class, name);
                if let Some(fn_val) = self.module.get_function(&method_fn_name) {
                    let method_params = self.method_decl_params(class, name);
                    let mut arg_vals: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> =
                        vec![this.into()];
                    for (i, arg) in args.iter().enumerate() {
                        let declared = method_params.as_ref().and_then(|p| p.get(i));
                        arg_vals.push(
                            self.coerce_user_call_arg(arg, declared, None)?,
                        );
                    }
                    Self::check_call_args(fn_val, &arg_vals, "direct")?;
                    let call = self.builder.build_call(fn_val, &arg_vals, "call")
                        .map_err(|e| format!("build_call '{}' failed: {:?}", method_fn_name, e))?;
                    return match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(v) => Ok(v),
                        _ => Ok(self.context.i32_type().const_zero().into()),
                    };
                }
            }
        }

        // Last resort: a bare identifier that names a registered native but
        // was not resolved as a user function (no matching overload in
        // functions/function_decls/generic_fn_decls). Emit a native call.
        if let Expr::Identifier(name, _) = callee {
            if !self.functions.contains_key(name)
                && !self.function_decls.contains_key(name)
                && !self.generic_fn_decls.contains_key(name)
                && native_bridge::is_native_function(name)
            {
                let arg_types: Vec<Type> =
                    args.iter().map(|a| self.infer_expr_type(a)).collect();
                let mut arg_vals = Vec::with_capacity(args.len());
                for arg in args {
                    arg_vals.push(self.compile_expr(arg)?);
                }
                return self.compile_native_call_declared(
                    name,
                    &arg_vals,
                    &arg_types,
                    None,
                );
            }
        }

        Err(format!("codegen: unsupported call target: {:?}", callee))
    }

    /// Lower a direct call in tail position as an LLVM tail call. Caller and
    /// callee signatures match exactly, so the result can be returned directly
    /// and LLVM can reuse the caller's stack frame instead of growing it.
    pub(super) fn compile_tail_call_direct(
        &mut self,
        name: &str,
        callee_fn: FunctionValue<'ctx>,
        args: &[Expr],
    ) -> Result<(), String> {
        let param_types = callee_fn.get_type().get_param_types();
        let decl_params = self.fn_decl_param_types_arity(name, args.len());
        let mut arg_vals = Vec::with_capacity(args.len());
        for (i, arg) in args.iter().enumerate() {
            let pty = if i < param_types.len() {
                Some(match param_types[i] {
                    inkwell::types::BasicMetadataTypeEnum::ArrayType(t) => t.into(),
                    inkwell::types::BasicMetadataTypeEnum::FloatType(t) => t.into(),
                    inkwell::types::BasicMetadataTypeEnum::IntType(t) => t.into(),
                    inkwell::types::BasicMetadataTypeEnum::PointerType(t) => t.into(),
                    inkwell::types::BasicMetadataTypeEnum::StructType(t) => t.into(),
                    inkwell::types::BasicMetadataTypeEnum::VectorType(t) => t.into(),
                    _ => return Err(format!("unsupported parameter type for tail call '{}'", name)),
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
        Self::check_call_args(callee_fn, &arg_vals, "tailcall")?;
        let call = self
            .builder
            .build_call(callee_fn, &arg_vals, "tailcall")
            .map_err(|e| format!("tail build_call '{}' failed: {:?}", name, e))?;
        // A pointer argument may alias a caller alloca (shared containers,
        // refs). LLVM's tail marker lets the optimizer drop that frame before
        // the callee reads it, so only mark calls with no pointer arguments.
        let has_pointer_arg = arg_vals
            .iter()
            .any(|v| matches!(v, BasicMetadataValueEnum::PointerValue(_)));
        if !has_pointer_arg {
            call.set_tail_call(true);
        }
        if callee_fn.get_type().get_return_type().is_some() {
            match call.try_as_basic_value() {
                inkwell::values::ValueKind::Basic(v) => {
                    self.builder
                        .build_return(Some(&v))
                        .map_err(|e| format!("tail build_return failed: {:?}", e))?;
                }
                _ => return Err(format!("tail call '{}' did not return a value", name)),
            }
        } else {
            self.builder
                .build_return(None)
                .map_err(|e| format!("tail build_return void failed: {:?}", e))?;
        }
        Ok(())
    }

    /// Lower a self-recursive direct tail call to a loop: rebind the current
    /// function's parameter slots with the call arguments and branch back to
    /// the body header. This is compiler-level tail-call elimination that does
    /// not rely on LLVM's optimizer, so it also holds in default (-O0) native
    /// builds. Only used when every parameter has a private entry-block alloca.
    pub(super) fn compile_self_tail_call(
        &mut self,
        tco: &TcoTrampoline<'ctx>,
        args: &[Expr],
    ) -> Result<(), String> {
        // Evaluate every argument before rebinding any parameter, so an
        // argument that reads a parameter keeps its pre-call meaning.
        let mut arg_vals = Vec::with_capacity(args.len());
        for (i, arg) in args.iter().enumerate() {
            let coerced = self.coerce_user_call_arg(arg, tco.param_types.get(i), None)?;
            arg_vals.push(Self::meta_to_basic(coerced)?);
        }
        for (slot, val) in tco.param_slots.iter().zip(arg_vals.iter()) {
            self.builder
                .build_store(*slot, *val)
                .map_err(|e| format!("tco build_store failed: {:?}", e))?;
        }
        self.builder
            .build_unconditional_branch(tco.loop_block)
            .map_err(|e| format!("tco build_br failed: {:?}", e))?;
        Ok(())
    }
}

#[cfg(test)]
mod explicit_type_arg_tests {
    use crate::analyzer;
    use crate::ast::Declaration;
    use crate::lexer;
    use crate::parser;
    use inkwell::context::Context;

    use super::LlvmBackend;

    fn compile_to_ir(source: &str) -> Result<String, String> {
        let tokens = lexer::tokenize(source).map_err(|e| format!("tokenize: {}", e))?;
        let ast = parser::parse(tokens).map_err(|e| format!("parse: {}", e))?;
        let typed = analyzer::analyze(&ast).map_err(|e| format!("analyze: {:?}", e))?;
        let context = Context::create();
        let mut backend = LlvmBackend::new(&context, "test");
        backend.declare_natives();
        for decl in &typed.declarations {
            match decl {
                Declaration::Function(f) if f.type_params.is_empty() => {
                    backend.function_decls.insert(f.name.clone(), f.clone());
                }
                Declaration::Function(f) => {
                    backend
                        .generic_fn_decls
                        .entry(f.name.clone())
                        .or_default()
                        .push(f.clone());
                }
                Declaration::Class(c) => {
                    backend.class_decls.insert(c.name.clone(), c.clone());
                }
                _ => {}
            }
        }
        for decl in &typed.declarations {
            if let Declaration::Function(f) = decl {
                if f.name != "main" && f.type_params.is_empty() {
                    backend
                        .compile_function(f)
                        .map_err(|e| format!("compile function '{}': {}", f.name, e))?;
                }
            }
        }
        let main = typed
            .declarations
            .iter()
            .find_map(|d| match d {
                Declaration::Function(f) if f.name == "main" => Some(f),
                _ => None,
            })
            .ok_or("no main")?;
        backend
            .compile_main(main)
            .map_err(|e| format!("compile main: {}", e))?;
        Ok(backend.module.print_to_string().to_string())
    }

    #[test]
    fn zero_arg_generic_factory_uses_explicit_type_arg() {
        let src = r#"
            public fn topologicalSorter<T>(): string {
                return "sorter";
            }
            public fn main(): void {
                let s = topologicalSorter<string>();
                io::println(s);
            }
        "#;
        let ir = compile_to_ir(src).expect("zero-arg generic factory should compile natively");
        assert!(
            ir.contains("topologicalSorter__string_a0"),
            "expected the string specialization to be monomorphized, got:\n{}",
            ir
        );
    }

    #[test]
    fn bare_native_call_emits_native_invocation() {
        let src = r#"
            public fn main(): void {
                let n = parseInt("42");
                io::println(n);
            }
        "#;
        let ir = compile_to_ir(src).expect("bare native call should compile natively");
        assert!(
            ir.contains("titrate_native_call_out"),
            "expected a native bridge call in IR, got:\n{}",
            ir
        );
    }

    #[test]
    fn generic_module_call_uses_explicit_type_arg() {
        let src = r#"
            public fn pick<T>(x: T): T { return x; }
            public fn main(): void {
                let s = Mod.pick<string>("hello");
                io::println(s);
            }
        "#;
        let ir = compile_to_ir(src).expect("generic module call should compile natively");
        assert!(
            ir.contains("pick__string_a1"),
            "expected the string specialization to be monomorphized, got:\n{}",
            ir
        );
    }
}
