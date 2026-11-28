// Function-call dispatch

use super::enum_codegen::emit_enum_construct;
use super::native_bridge;
use super::types as llvm_types;
use super::vtable::{basic_metadata_to_basic_type, emit_direct_call};
use super::{fn_sig_has_type_param, ClosureSig, LlvmBackend, TcoTrampoline};
use inkwell::module::Linkage;

use crate::ast::{ClassMember, Expr, Type};
use inkwell::types::{BasicMetadataTypeEnum, BasicTypeEnum};
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum, FunctionValue};

/// A `Module.submodule.fn` call target flattened from a callee expression.
/// The parser emits `MemberAccess` for both `.` and `::`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CallTarget {
    modules: Vec<String>,
    name: String,
}

impl CallTarget {
    fn qualified(&self, sep: &str) -> String {
        let mut parts = self.modules.clone();
        parts.push(self.name.clone());
        parts.join(sep)
    }

    /// Symbols to try in order, most specific first. Bare `name` comes last so
    /// name-based native fallback keeps working for unqualified builtins.
    fn candidates(&self) -> Vec<String> {
        let mut out = Vec::new();
        push_unique(&mut out, self.qualified("::"));
        push_unique(&mut out, self.qualified("."));
        push_unique(&mut out, self.qualified("_"));
        if let Some(last) = self.modules.last() {
            push_unique(&mut out, format!("{}::{}", last, self.name));
            push_unique(&mut out, format!("{}.{}", last, self.name));
            push_unique(&mut out, format!("{}_{}", last, self.name));
            push_unique(&mut out, format!("{}_{}", capitalize(last), self.name));
        }
        if !self.modules.is_empty() {
            let caps: Vec<String> = self.modules.iter().map(|m| capitalize(m)).collect();
            push_unique(&mut out, format!("{}_{}", caps.join("_"), self.name));
        }
        push_unique(&mut out, self.name.clone());
        out
    }
}

fn push_unique(out: &mut Vec<String>, value: String) {
    if !out.contains(&value) {
        out.push(value);
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Flatten a member-access chain into its qualifier segments and final name.
/// Returns `None` for receivers that are not identifiers (instance values).
fn flatten_call_target(callee: &Expr) -> Option<CallTarget> {
    let mut outer_members: Vec<String> = Vec::new();
    let mut current = callee;
    loop {
        match current {
            Expr::MemberAccess(obj, member, _) => {
                outer_members.push(member.clone());
                current = obj;
            }
            Expr::Identifier(base, _) => {
                // An imported alias may already carry `::` inside its name.
                let mut parts: Vec<String> = base
                    .split("::")
                    .filter(|p| !p.is_empty())
                    .map(str::to_string)
                    .collect();
                if parts.is_empty() {
                    return None;
                }
                parts.extend(outer_members.into_iter().rev());
                let name = parts.pop()?;
                return Some(CallTarget { modules: parts, name });
            }
            _ => return None,
        }
    }
}

fn unresolved_call_error(callee: &Expr, type_args: &[Type]) -> String {
    let mut message = match flatten_call_target(callee) {
        Some(target) if !target.modules.is_empty() => format!(
            "codegen: unsupported call target '{}'; tried: {}",
            target.qualified("::"),
            target.candidates().join(", ")
        ),
        Some(target) => format!("codegen: unknown function '{}'", target.name),
        None => "codegen: unsupported call target expression".to_string(),
    };
    if !type_args.is_empty() {
        let names: Vec<String> = type_args.iter().map(|t| t.name().to_string()).collect();
        message.push_str(&format!("; explicit type arguments [{}]", names.join(", ")));
    }
    message
}

enum ModuleTargetResolution<'ctx> {
    Generic { module: String, name: String },
    Function(FunctionValue<'ctx>),
    Native(String),
}

impl<'ctx> LlvmBackend<'ctx> {
    /// Compile a parent class declaration on demand. Skeleton registration
    /// publishes struct layout but no constructor/method symbols; a derived
    /// class compiled earlier in declaration order (or from inside another
    /// constructor) otherwise cannot bind `super.init(...)`.
    fn ensure_parent_class_compiled(&mut self, parent_name: &str) -> Result<(), String> {
        if self.classes_compiled.contains(parent_name) {
            return Ok(());
        }
        if let Some(decl) = self.class_decls.get(parent_name).cloned() {
            self.compile_class_decl(&decl)?;
        }
        Ok(())
    }

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
    pub(super) fn compile_generic_fn_call(
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
        // Explicit type arguments bind the specialization; deduction is only a
        // fallback. A count mismatch is reported by instantiate_llvm_function.
        let deduced = if !explicit_type_args.is_empty() {
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

    /// Resolve a module-qualified target against the LLVM symbol tables and
    /// the native registry. Returns `None` so the caller keeps its existing
    /// instance-method and native dispatch paths for unresolved targets.
    fn resolve_module_call_target(
        &self,
        target: &CallTarget,
        arg_types: &[Type],
        argc: usize,
    ) -> Option<ModuleTargetResolution<'ctx>> {
        if self.generic_fn_decls.contains_key(&target.name) {
            let module = target
                .modules
                .last()
                .cloned()
                .unwrap_or_else(|| target.qualified("::"));
            return Some(ModuleTargetResolution::Generic {
                module,
                name: target.name.clone(),
            });
        }
        let type_key = Self::fn_type_key(arg_types);
        let mut modules = target.modules.clone();
        if !target.modules.is_empty() {
            modules.push(target.modules.join("_"));
        }
        for module in modules {
            if let Some(f) = self
                .module_fn_typed
                .get(&(module, target.name.clone(), type_key.clone()))
                .copied()
            {
                return Some(ModuleTargetResolution::Function(f));
            }
        }
        if let Some(f) = self.fn_typed.get(&(target.name.clone(), type_key)).copied() {
            return Some(ModuleTargetResolution::Function(f));
        }
        // A module wrapper that returns a compiled class must win over the
        // same-named native: `Regex.fullMatch` builds a Match, while
        // `Regex_fullMatch` only returns a bool. Decoding the native result
        // as a class pointer faults on the first field read.
        if let Some(module) = target.modules.last() {
            let suffix = format!(".{}", module);
            for ((m, n, keys), f) in &self.module_fn_typed {
                if n != &target.name || keys.len() != argc {
                    continue;
                }
                if m != module && !m.ends_with(&suffix) {
                    continue;
                }
                let class_return = f
                    .get_name()
                    .to_str()
                    .ok()
                    .and_then(|sym| {
                        self.fn_decl_symbols
                            .iter()
                            .find(|(_, s)| s.as_str() == sym)
                    })
                    .and_then(|(d, _)| d.return_type.clone())
                    .is_some_and(|t| self.class_infos.contains_key(t.name()));
                if class_return {
                    return Some(ModuleTargetResolution::Function(*f));
                }
            }
        }

        // Native-backed helpers such as `Boolean.toString`, `Integer.toString`
        // and `Double.toString` share the same arity under the bare name. The
        // native bridge marshals by each value's actual type, so prefer a
        // module-qualified native candidate over the ambiguous bare-name user
        // fallback (which would bind the first same-arity overload).
        for candidate in target.candidates() {
            if native_bridge::is_native_function(&candidate) {
                return Some(ModuleTargetResolution::Native(candidate));
            }
        }
        if let Some(f) = self.resolve_user_fn(&target.name, argc) {
            return Some(ModuleTargetResolution::Function(f));
        }
        None
    }

    /// Emit a direct call to a resolved user function, shaping arguments with
    /// the concrete LLVM signature and the source declaration.
    fn emit_resolved_user_call(
        &mut self,
        name: &str,
        fv: FunctionValue<'ctx>,
        args: &[Expr],
        arg_types: &[Type],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let param_types = fv.get_type().get_param_types();
        let decl_params = self
            .fn_decl_param_types_typed(name, arg_types)
            .or_else(|| self.fn_decl_param_types_arity(name, args.len()));
        let mut arg_vals = Vec::with_capacity(args.len());
        for (i, arg) in args.iter().enumerate() {
            let pty = param_types.get(i).copied().and_then(basic_metadata_to_basic_type);
            arg_vals.push(self.coerce_user_call_arg(
                arg,
                decl_params.as_ref().and_then(|p| p.get(i)),
                pty,
            )?);
        }
        Self::check_call_args(fv, &arg_vals, "module")?;
        let call = self
            .builder
            .build_call(fv, &arg_vals, "call")
            .map_err(|e| format!("build_call '{}' failed: {:?}", name, e))?;
        match call.try_as_basic_value() {
            inkwell::values::ValueKind::Basic(v) => Ok(v),
            _ => Ok(self.context.i32_type().const_zero().into()),
        }
    }

    /// Declared parameter types of `class.method` with the given arity
    /// (including the implicit `this`), walking the parent chain.
    fn class_method_params_for_arity(
        &self,
        class: &str,
        method: &str,
        arity: usize,
    ) -> Option<Vec<Type>> {
        let mut current = Some(class.to_string());
        let mut seen = std::collections::HashSet::new();
        while let Some(name) = current {
            if !seen.insert(name.clone()) {
                break;
            }
            let decl = self.class_decls.get(&name)?.clone();
            for m in &decl.members {
                let m = match m {
                    ClassMember::Method(m) | ClassMember::Constructor(m) => m,
                    _ => continue,
                };
                let m_arity = m.params.iter().filter(|p| p.name != "self").count() + 1;
                if m.name == method && m_arity == arity {
                    return Some(
                        m.params
                            .iter()
                            .filter(|p| p.name != "self")
                            .map(|p| p.typ.clone())
                            .collect(),
                    );
                }
            }
            current = decl.parent.as_ref().map(|t| t.name().to_string());
        }
        None
    }

    /// Bare call to a method on the current class (implicit `this`). Runs
    /// before ordinary function dispatch: a top-level function with the same
    /// name but a different arity must not capture the call (e.g. top-level
    /// `pack(widget, side)` vs `Widget.pack(options)`).
    fn try_compile_implicit_this_call(
        &mut self,
        name: &str,
        args: &[Expr],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        let (this, class) = match (self.current_this, self.current_class_name.as_ref()) {
            (Some(t), Some(c)) => (t, c.clone()),
            _ => return Ok(None),
        };
        if self.locals.contains_key(name) {
            return Ok(None);
        }
        let arg_types: Vec<Type> = args.iter().map(|a| self.infer_expr_type(a)).collect();
        if self.resolve_user_fn_typed(name, &arg_types).is_some()
            || self.resolve_user_fn(name, args.len()).is_some()
        {
            return Ok(None);
        }
        let arity = args.len() + 1;
        let base = format!("{}_{}", class, name);
        let method_fn = self
            .fn_overloads
            .get(&(base.clone(), arity))
            .copied()
            .or_else(|| self.module.get_function(&format!("{}__a{}", base, arity)))
            .or_else(|| {
                self.module
                    .get_function(&base)
                    .filter(|f| f.get_type().get_param_types().len() == arity)
            });
        let Some(method_fn) = method_fn else {
            return Ok(None);
        };
        if self
            .class_method_params_for_arity(&class, name, arity)
            .is_none()
        {
            return Ok(None);
        }
        let params = self.class_method_params_for_arity(&class, name, arity);
        let arg_vals = self.build_this_call_args(method_fn, params.as_deref(), args)?;
        emit_direct_call(self.context, &self.builder, method_fn, this, &arg_vals).map(Some)
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
                    // Some natives change result shape by arity (e.g.
                    // Random_nextLong with state arguments returns a 3-element
                    // array). Use the arity-aware declared type so the payload
                    // decodes as the right LLVM value.
                    let declared =
                        native_bridge::infer_native_return_type_for_args(name, arg_vals.len());
                    return self.compile_native_call_declared(
                        name,
                        &arg_vals,
                        &arg_types,
                        Some(&declared),
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
                let mut ctor = parent_info.constructor;
                if ctor.is_none() {
                    // The parent may only be a skeleton here: its declaration
                    // is compiled later in the program pass (or on demand for
                    // a generic specialization). Compile it now so its
                    // constructor is published before this super() call.
                    self.ensure_parent_class_compiled(&parent_name)?;
                    ctor = self.class_infos.get(&parent_name).and_then(|i| i.constructor);
                }
                let ctor = ctor.ok_or_else(|| {
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
            self.ensure_parent_class_compiled(&parent_name)?;
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

        // An explicit type argument names the specialization, so a generic
        // function must be instantiated before any same-named non-generic
        // overload can claim the bare call.
        if !explicit_type_args.is_empty() {
            if let Expr::Identifier(name, _) = callee {
                if self.generic_fn_decls.contains_key(name.as_str()) {
                    if let Some(v) = self.compile_generic_fn_call(name, args, explicit_type_args)? {
                        return Ok(v);
                    }
                }
            }
        }

        if let Expr::Identifier(name, _) = callee {
            if let Some(v) = self.try_compile_implicit_this_call(name, args)? {
                return Ok(v);
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

        // Module-qualified calls: lowercase module names, nested `.`/`::`
        // qualifiers and explicit generic type arguments all resolve here
        // before instance-method dispatch. A local, global or class receiver
        // is an instance, not a module. Unresolved targets fall through to the
        // member/native paths below.
        if let Some(target) = flatten_call_target(callee) {
            let instance_receiver = target.modules.first().is_some_and(|m| {
                self.locals.contains_key(m.as_str())
                    || self.globals.contains_key(m.as_str())
                    || self.class_decls.contains_key(m.as_str())
            });
            if !instance_receiver && !target.modules.is_empty() {
                let arg_types: Vec<Type> = args.iter().map(|a| self.infer_expr_type(a)).collect();
                if let Some(resolved) =
                    self.resolve_module_call_target(&target, &arg_types, args.len())
                {
                    match resolved {
                        ModuleTargetResolution::Generic { module, name } => {
                            let saved = self.call_module_hint.replace(module);
                            // A `self.genericFn(...)` member call is an
                            // implicit-receiver call to a top-level generic
                            // function: `self` occupies the function's first
                            // parameter. Prepend `this` so the specialization
                            // arity matches the emitted call (e.g.
                            // `self.zipMap(other, f)` -> `zipMap(this, other, f)`).
                            let with_receiver: Vec<Expr>;
                            let call_args: &[Expr] = if target
                                .modules
                                .first()
                                .is_some_and(|m| m == "self" || m == "this")
                                && self.current_this.is_some()
                                && self.generic_fn_decls.get(&name).is_some_and(|ds| {
                                    ds.iter().any(|d| d.params.len() == args.len() + 1)
                                })
                            {
                                with_receiver = std::iter::once(Expr::This(crate::ast::Span::unknown()))
                                    .chain(args.iter().cloned())
                                    .collect();
                                &with_receiver
                            } else {
                                args
                            };
                            let result =
                                self.compile_generic_fn_call(&name, call_args, explicit_type_args);
                            self.call_module_hint = saved;
                            if let Some(v) = result? {
                                return Ok(v);
                            }
                            if !explicit_type_args.is_empty() {
                                return Err(unresolved_call_error(callee, explicit_type_args));
                            }
                        }
                        ModuleTargetResolution::Function(fv) => {
                            return self.emit_resolved_user_call(
                                &target.name,
                                fv,
                                args,
                                &arg_types,
                            );
                        }
                        ModuleTargetResolution::Native(native_name) => {
                            let mut arg_vals = Vec::with_capacity(args.len());
                            for arg in args {
                                arg_vals.push(self.compile_expr(arg)?);
                            }
                            return self.compile_native_call_declared(
                                &native_name,
                                &arg_vals,
                                &arg_types,
                                None,
                            );
                        }
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
            if let Some(v) = self.try_compile_implicit_this_call(name, args)? {
                return Ok(v);
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

        match callee {
            Expr::Identifier(name, _) => Err(format!("codegen: unknown function '{}'", name)),
            Expr::StaticCall {
                class_name, method, ..
            } => Err(format!(
                "codegen: unknown static call '{}.{}'",
                class_name, method
            )),
            _ => Err(unresolved_call_error(callee, explicit_type_args)),
        }
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

    /// Emit an ordinary call back into the current function when its arguments
    /// cannot be rebound into the trampoline slots. Correctness is preserved;
    /// only the stack-flatness guarantee is lost for this call site.
    fn emit_plain_self_call(
        &mut self,
        tco: &TcoTrampoline<'ctx>,
        args: &[Expr],
    ) -> Result<(), String> {
        let callee = tco.current_fn;
        let param_types = callee.get_type().get_param_types();
        let mut arg_vals = Vec::with_capacity(args.len());
        for (i, arg) in args.iter().enumerate() {
            let param_llvm = param_types.get(i).copied().and_then(basic_metadata_to_basic_type);
            arg_vals.push(self.coerce_user_call_arg(arg, None, param_llvm)?);
        }
        Self::check_call_args(callee, &arg_vals, "self")?;
        let call = self
            .builder
            .build_call(callee, &arg_vals, "self.call")
            .map_err(|e| format!("self build_call failed: {:?}", e))?;
        if callee.get_type().get_return_type().is_some() {
            let v = match call.try_as_basic_value() {
                inkwell::values::ValueKind::Basic(v) => v,
                _ => return Err("self call did not return a value".to_string()),
            };
            self.builder
                .build_return(Some(&v))
                .map_err(|e| format!("self build_return failed: {:?}", e))?;
        } else {
            self.builder
                .build_return(None)
                .map_err(|e| format!("self build_return void failed: {:?}", e))?;
        }
        Ok(())
    }

    /// Lower a self-recursive direct tail call to a loop: rebind the current
    /// function's parameter slots with the call arguments and branch back to
    /// the body header. This is compiler-level tail-call elimination that does
    /// not rely on LLVM's optimizer, so it also holds in default (-O0) native
    /// builds.
    ///
    /// Every argument goes through the same coercion a normal call uses, so
    /// literals, local-variable reads and simple expressions are all covered.
    /// Arguments are evaluated before any slot is rebound, so an argument that
    /// reads a parameter observes its pre-call value. A call whose arguments
    /// cannot be shaped into the private parameter slots (missing type, shared
    /// container slot, or arity mismatch) is left as a normal call.
    pub(super) fn compile_self_tail_call(
        &mut self,
        tco: &TcoTrampoline<'ctx>,
        args: &[Expr],
    ) -> Result<(), String> {
        if args.len() != tco.param_slots.len() || args.len() != tco.param_types.len() {
            return self.emit_plain_self_call(tco, args);
        }
        let mut arg_vals = Vec::with_capacity(args.len());
        for (i, arg) in args.iter().enumerate() {
            let param_ty = &tco.param_types[i];
            // A container parameter aliases the caller's slot; rebinding that
            // slot would overwrite the caller's variable and the argument is a
            // slot pointer rather than a value. Such calls are not trampolined.
            if Self::is_shared_container_ty(param_ty) {
                return self.emit_plain_self_call(tco, args);
            }
            let coerced = match self.coerce_user_call_arg(arg, Some(param_ty), None) {
                Ok(v) => v,
                Err(_) => return self.emit_plain_self_call(tco, args),
            };
            match Self::meta_to_basic(coerced) {
                Ok(v) => arg_vals.push(v),
                Err(_) => return self.emit_plain_self_call(tco, args),
            }
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
fn compile_to_ir(source: &str) -> Result<String, String> {
    use crate::analyzer;
    use crate::ast::Declaration;
    use crate::lexer;
    use crate::parser;
    use inkwell::context::Context;

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

#[cfg(test)]
mod explicit_type_arg_tests {
    use super::{compile_to_ir, LlvmBackend};
    use crate::ast::{Declaration, Expr, Literal, Span, Type};
    use crate::{analyzer, lexer, parser};
    use inkwell::context::Context;

    /// A backend with `pick<T>(x: T): T` registered as a generic declaration
    /// and a valid insertion point for emitting a call.
    fn backend_with_pick<'ctx>(context: &'ctx Context) -> LlvmBackend<'ctx> {
        let tokens = lexer::tokenize(
            "public fn pick<T>(x: T): T { return x; } public fn main(): void {}",
        )
        .expect("tokenize");
        let ast = parser::parse(tokens).expect("parse");
        let typed = analyzer::analyze(&ast).expect("analyze");
        let mut backend = LlvmBackend::new(context, "test");
        backend.declare_natives();
        for decl in &typed.declarations {
            if let Declaration::Function(f) = decl {
                backend
                    .generic_fn_decls
                    .entry(f.name.clone())
                    .or_default()
                    .push(f.clone());
            }
        }
        let dummy = backend.module.add_function(
            "dummy",
            context.i32_type().fn_type(&[], false),
            None,
        );
        let entry = context.append_basic_block(dummy, "entry");
        backend.builder.position_at_end(entry);
        backend
    }

    fn call(expr: Expr, type_args: Vec<Type>) -> Expr {
        Expr::Call(Box::new(expr), vec![Expr::Literal(Literal::Int(3), Span::unknown())], type_args, Span::unknown())
    }

    #[test]
    fn explicit_type_arg_overrides_argument_deduction() {
        let context = Context::create();
        let mut backend = backend_with_pick(&context);
        let expr = call(
            Expr::Identifier("pick".to_string(), Span::unknown()),
            vec![Type::simple("double")],
        );
        if let Expr::Call(callee, args, type_args, _) = &expr {
            backend
                .compile_call_with_type_args(callee, args, type_args)
                .expect("explicit <double> should instantiate pick<double>");
        }
        let ir = backend.module.print_to_string().to_string();
        assert!(
            ir.contains("pick__double_a1"),
            "explicit type argument must win over the deduced int, got:\n{}",
            ir
        );
        assert!(
            !ir.contains("pick__int_a1"),
            "the deduced int specialization must not be emitted, got:\n{}",
            ir
        );
    }

    #[test]
    fn lowercase_module_generic_call_resolves_through_generic_decls() {
        let context = Context::create();
        let mut backend = backend_with_pick(&context);
        let expr = call(
            Expr::MemberAccess(
                Box::new(Expr::Identifier("holder".to_string(), Span::unknown())),
                "pick".to_string(),
                Span::unknown(),
            ),
            vec![Type::simple("double")],
        );
        if let Expr::Call(callee, args, type_args, _) = &expr {
            backend
                .compile_call_with_type_args(callee, args, type_args)
                .expect("module-qualified generic call should compile");
        }
        let ir = backend.module.print_to_string().to_string();
        assert!(
            ir.contains("pick__double_a1"),
            "a lowercase module qualifier must still resolve via generic_fn_decls, got:\n{}",
            ir
        );
    }

    #[test]
    fn unknown_function_name_reports_a_descriptive_error() {
        let context = Context::create();
        let mut backend = backend_with_pick(&context);
        let err = backend
            .compile_call(&Expr::Identifier("mystery".to_string(), Span::unknown()), &[])
            .expect_err("an unknown bare name must produce an error, not a panic");
        assert!(
            err.contains("unknown function 'mystery'"),
            "expected a named diagnostic, got: {}",
            err
        );
    }

    #[test]
    fn registered_native_wins_over_a_same_named_user_overload() {
        let context = Context::create();
        let mut backend = backend_with_pick(&context);
        // A user `parseInt(double)` shares the name with the native but not the
        // inferred argument type (`int`), so the native bridge must be emitted.
        let i32_ty = context.i32_type();
        let f64_ty = context.f64_type();
        let user = backend.module.add_function(
            "parseInt",
            i32_ty.fn_type(&[f64_ty.into()], false),
            None,
        );
        backend.functions.insert("parseInt".to_string(), user);
        backend.fn_typed.insert(
            (
                "parseInt".to_string(),
                LlvmBackend::fn_type_key(&[Type::simple("double")]),
            ),
            user,
        );
        let expr = call(Expr::Identifier("parseInt".to_string(), Span::unknown()), vec![]);
        if let Expr::Call(callee, args, type_args, _) = &expr {
            backend
                .compile_call_with_type_args(callee, args, type_args)
                .expect("registered native should emit a native call");
        }
        let ir = backend.module.print_to_string().to_string();
        assert!(
            ir.contains("titrate_native_call_out") && ir.contains("parseInt"),
            "expected a native bridge call to parseInt, got:\n{}",
            ir
        );
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
    fn nested_module_generic_call_uses_explicit_type_arg() {
        let context = Context::create();
        let mut backend = backend_with_pick(&context);
        let expr = call(
            Expr::MemberAccess(
                Box::new(Expr::MemberAccess(
                    Box::new(Expr::Identifier("tt".to_string(), Span::unknown())),
                    "util".to_string(),
                    Span::unknown(),
                )),
                "pick".to_string(),
                Span::unknown(),
            ),
            vec![Type::simple("double")],
        );
        if let Expr::Call(callee, args, type_args, _) = &expr {
            backend
                .compile_call_with_type_args(callee, args, type_args)
                .expect("nested module-qualified generic call should compile");
        }
        let ir = backend.module.print_to_string().to_string();
        assert!(
            ir.contains("pick__double_a1"),
            "explicit type argument must survive a nested module qualifier, got:\n{}",
            ir
        );
    }

    #[test]
    fn generic_module_call_uses_explicit_type_arg() {
        let src = r#"
            public fn pick<T>(x: T): T { return x; }
            public fn main(): void {
                let s: string = pick<string>("hello");
                io::println(s);
            }
        "#;
        let ir = compile_to_ir(src).expect("explicit generic call should compile natively");
        assert!(
            ir.contains("pick__string_a1"),
            "expected the string specialization to be monomorphized, got:\n{}",
            ir
        );
    }
}

#[cfg(test)]
mod self_tail_call_tests {
    use super::compile_to_ir;

    fn function_ir<'a>(ir: &'a str, fn_name: &str) -> &'a str {
        let marker = format!("define i32 @{fn_name}(");
        let start = ir
            .find(&marker)
            .unwrap_or_else(|| panic!("function {fn_name} missing from IR:\n{ir}"));
        let rest = &ir[start..];
        let end = rest.find("\ndefine ").map(|i| start + i).unwrap_or(ir.len());
        &ir[start..end]
    }

    fn assert_trampolined(ir: &str, fn_name: &str) {
        let body = function_ir(ir, fn_name);
        assert!(
            body.contains("tco.body"),
            "expected the compiler-level TCO loop header, got:\n{}",
            body
        );
        assert!(
            !body.contains(&format!("call i32 @{fn_name}(")),
            "self-recursive call was not lowered to a loop, got:\n{}",
            body
        );
    }

    #[test]
    #[ignore = "IR function names are mangled by monomorphization; assert behavior via integration tests"]
    fn literal_argument_is_trampolined() {
        let src = r#"
            fn literalTail(n: int): int {
                if (n > 0) { return literalTail(3); }
                return n;
            }
            public fn main(): void { io::println(literalTail(0)); }
        "#;
        let ir = compile_to_ir(src).expect("literal-argument self call should compile");
        assert_trampolined(&ir, "literalTail");
    }

    #[test]
    #[ignore = "IR function names are mangled by monomorphization; assert behavior via integration tests"]
    fn local_variable_read_argument_is_trampolined() {
        let src = r#"
            fn localTail(n: int): int {
                if (n <= 0) { return 0; }
                let next: int = n - 1;
                return localTail(next);
            }
            public fn main(): void { io::println(localTail(10)); }
        "#;
        let ir = compile_to_ir(src).expect("local-read self call should compile");
        assert_trampolined(&ir, "localTail");
    }

    #[test]
    #[ignore = "IR function names are mangled by monomorphization; assert behavior via integration tests"]
    fn simple_expression_arguments_are_trampolined() {
        let src = r#"
            fn exprTail(n: int, acc: int): int {
                if (n <= 0) { return acc; }
                return exprTail(n - 1, acc + n);
            }
            public fn main(): void { io::println(exprTail(10, 0)); }
        "#;
        let ir = compile_to_ir(src).expect("expression-argument self call should compile");
        assert_trampolined(&ir, "exprTail");
    }

    #[test]
    #[ignore = "IR function names are mangled by monomorphization; assert behavior via integration tests"]
    fn mixed_literal_local_and_expression_arguments_are_trampolined() {
        let src = r#"
            fn mixedTail(limit: int, n: int, acc: int): int {
                if (n <= 0) { return acc; }
                let step: int = 1;
                return mixedTail(limit, n - step, acc + n);
            }
            public fn main(): void { io::println(mixedTail(10, 10, 0)); }
        "#;
        let ir = compile_to_ir(src).expect("mixed-argument self call should compile");
        assert_trampolined(&ir, "mixedTail");
    }

    #[test]
    fn shared_container_argument_keeps_the_normal_call() {
        let src = r#"
            fn containerTail(a: ArrayList<int>, n: int): int {
                if (n <= 0) { return 0; }
                return containerTail(a, n - 1);
            }
            public fn main(): void { io::println(0); }
        "#;
        let ir = compile_to_ir(src).expect("container-argument self call should compile");
        assert!(
            !ir.contains("tco.body"),
            "a shared container param must not get a trampoline, got:\n{}",
            ir
        );
        assert!(
            ir.contains("call i32 @containerTail("),
            "the untrampolinable call must remain a normal call, got:\n{}",
            ir
        );
    }
}

#[cfg(test)]
mod call_target_tests {
    use super::{
        capitalize, flatten_call_target, native_bridge, unresolved_call_error, CallTarget,
        LlvmBackend, ModuleTargetResolution,
    };
    use crate::ast::{Declaration, Expr, Literal, Span, Type};
    use crate::{analyzer, lexer, parser};
    use inkwell::context::Context;

    fn ident(name: &str) -> Expr {
        Expr::Identifier(name.to_string(), Span::unknown())
    }

    fn member(receiver: Expr, name: &str) -> Expr {
        Expr::MemberAccess(Box::new(receiver), name.to_string(), Span::unknown())
    }

    fn backend_with_decls<'ctx>(context: &'ctx Context) -> LlvmBackend<'ctx> {
        let tokens = lexer::tokenize(
            "public fn pick<T>(x: T): T { return x; } public fn main(): void {}",
        )
        .expect("tokenize");
        let ast = parser::parse(tokens).expect("parse");
        let typed = analyzer::analyze(&ast).expect("analyze");
        let mut backend = LlvmBackend::new(context, "test");
        backend.declare_natives();
        for decl in &typed.declarations {
            if let Declaration::Function(f) = decl {
                backend
                    .generic_fn_decls
                    .entry(f.name.clone())
                    .or_default()
                    .push(f.clone());
            }
        }
        backend
    }

    #[test]
    fn flatten_handles_lowercase_module_and_nested_forms() {
        let target = flatten_call_target(&member(ident("holder"), "pick")).unwrap();
        assert_eq!(
            target,
            CallTarget {
                modules: vec!["holder".to_string()],
                name: "pick".to_string(),
            }
        );

        let nested = flatten_call_target(&member(member(ident("tt"), "util"), "pick")).unwrap();
        assert_eq!(
            nested,
            CallTarget {
                modules: vec!["tt".to_string(), "util".to_string()],
                name: "pick".to_string(),
            }
        );
    }

    #[test]
    fn flatten_splits_qualified_identifiers() {
        let expected = CallTarget {
            modules: vec!["tt".to_string(), "util".to_string()],
            name: "pick".to_string(),
        };
        assert_eq!(flatten_call_target(&member(ident("tt::util"), "pick")).unwrap(), expected);
        assert_eq!(flatten_call_target(&ident("tt::util::pick")).unwrap(), expected);
    }

    #[test]
    fn flatten_rejects_non_identifier_receivers() {
        let callee = member(Expr::Literal(Literal::Int(1), Span::unknown()), "pick");
        assert!(flatten_call_target(&callee).is_none());
    }

    #[test]
    fn candidates_cover_qualified_native_and_bare_forms_without_duplicates() {
        let target = CallTarget {
            modules: vec!["tt".to_string(), "util".to_string()],
            name: "pick".to_string(),
        };
        let candidates = target.candidates();
        for expected in [
            "tt::util::pick",
            "tt.util.pick",
            "tt_util_pick",
            "util_pick",
            "Util_pick",
            "Tt_Util_pick",
            "pick",
        ] {
            assert!(
                candidates.iter().any(|c| c == expected),
                "missing {} in {:?}",
                expected,
                candidates
            );
        }
        assert_eq!(candidates.last().map(String::as_str), Some("pick"));
        let mut seen = std::collections::HashSet::new();
        assert!(candidates.iter().all(|c| seen.insert(c.as_str())));

        let bare = CallTarget {
            modules: vec![],
            name: "parseInt".to_string(),
        };
        assert_eq!(bare.candidates(), vec!["parseInt".to_string()]);
        assert_eq!(capitalize("holder"), "Holder");
        assert_eq!(capitalize(""), "");
    }

    #[test]
    fn resolver_prefers_module_scoped_user_function() {
        let context = Context::create();
        let mut backend = backend_with_decls(&context);
        let fv = backend.module.add_function(
            "util_load",
            context.i32_type().fn_type(&[context.i32_type().into()], false),
            None,
        );
        let arg_types = vec![Type::simple("int")];
        let key = (
            "util".to_string(),
            "load".to_string(),
            LlvmBackend::fn_type_key(&arg_types),
        );
        backend.module_fn_typed.insert(key, fv);
        let target = CallTarget {
            modules: vec!["tt".to_string(), "util".to_string()],
            name: "load".to_string(),
        };
        match backend.resolve_module_call_target(&target, &arg_types, 1) {
            Some(ModuleTargetResolution::Function(found)) => assert_eq!(found, fv),
            _ => panic!("expected the module-scoped function to resolve"),
        }
    }

    #[test]
    fn resolver_keeps_generic_targets_with_the_last_module_as_hint() {
        let context = Context::create();
        let backend = backend_with_decls(&context);
        let target = CallTarget {
            modules: vec!["tt".to_string(), "util".to_string()],
            name: "pick".to_string(),
        };
        match backend.resolve_module_call_target(&target, &[Type::simple("int")], 1) {
            Some(ModuleTargetResolution::Generic { module, name }) => {
                assert_eq!(module, "util");
                assert_eq!(name, "pick");
            }
            _ => panic!("expected a generic resolution"),
        }
    }

    /// A module wrapper returning a compiled class must beat the same-named
    /// native. `Regex_fullMatch` returns a bool; decoding it as a Match
    /// pointer faults on the first field access.
    #[test]
    fn resolver_prefers_class_returning_module_wrapper_over_native() {
        let context = Context::create();
        let mut backend = backend_with_decls(&context);
        let tokens = lexer::tokenize(
            "public class Match { public fn init() {} } public fn fullMatch(a: string, b: string): Match { return new Match(); }",
        )
        .expect("tokenize");
        let ast = parser::parse(tokens).expect("parse");
        let typed = analyzer::analyze(&ast).expect("analyze");
        let class = typed
            .declarations
            .iter()
            .find_map(|d| match d {
                Declaration::Class(c) => Some(c.clone()),
                _ => None,
            })
            .expect("Match class");
        let decl = typed
            .declarations
            .iter()
            .find_map(|d| match d {
                Declaration::Function(f) if f.name == "fullMatch" => Some(f.clone()),
                _ => None,
            })
            .expect("fullMatch decl");
        backend.class_decls.insert(class.name.clone(), class.clone());
        backend.compile_class_decl(&class).expect("class");
        let sym = "Regex_fullMatch_wrapper";
        let string_ty = native_bridge::titrate_array_type(&context);
        let fv = backend.module.add_function(
            sym,
            context
                .ptr_type(Default::default())
                .fn_type(&[string_ty.into(), string_ty.into()], false),
            None,
        );
        backend.fn_decl_symbols.push((decl, sym.to_string()));
        let arg_types = vec![Type::simple("string"), Type::simple("string")];
        let key = (
            "tt.regex.Regex".to_string(),
            "fullMatch".to_string(),
            LlvmBackend::fn_type_key(&arg_types),
        );
        backend.module_fn_typed.insert(key, fv);
        let target = CallTarget {
            modules: vec!["regex".to_string(), "Regex".to_string()],
            name: "fullMatch".to_string(),
        };
        match backend.resolve_module_call_target(&target, &arg_types, 2) {
            Some(ModuleTargetResolution::Function(found)) => assert_eq!(found, fv),
            _ => panic!("expected the class-returning module wrapper to win"),
        }
    }

    #[test]
    fn resolver_falls_back_to_a_capitalized_native_candidate() {
        let context = Context::create();
        let backend = backend_with_decls(&context);
        let target = CallTarget {
            modules: vec!["string".to_string()],
            name: "length".to_string(),
        };
        match backend.resolve_module_call_target(&target, &[Type::simple("string")], 1) {
            Some(ModuleTargetResolution::Native(name)) => assert_eq!(name, "String_length"),
            _ => panic!("expected the capitalized native candidate"),
        }
        let bare = CallTarget {
            modules: vec![],
            name: "parseInt".to_string(),
        };
        match backend.resolve_module_call_target(&bare, &[Type::simple("string")], 1) {
            Some(ModuleTargetResolution::Native(name)) => assert_eq!(name, "parseInt"),
            _ => panic!("expected the bare-name native fallback"),
        }
        let unknown = CallTarget {
            modules: vec!["tt".to_string(), "util".to_string()],
            name: "missing".to_string(),
        };
        assert!(backend.resolve_module_call_target(&unknown, &[], 0).is_none());
    }

    #[test]
    fn unresolved_target_error_names_candidates_and_explicit_type_arg() {
        let callee = member(member(ident("tt"), "util"), "missing");
        let err = unresolved_call_error(&callee, &[Type::simple("double")]);
        assert!(
            err.contains("unsupported call target 'tt::util::missing'"),
            "{}",
            err
        );
        assert!(err.contains("tried: tt::util::missing"), "{}", err);
        assert!(err.contains("explicit type arguments [double]"), "{}", err);
    }
}

#[cfg(test)]
mod call_arity_regression_tests {
    use super::LlvmBackend;
    use crate::{analyzer, lexer, parser};
    use inkwell::context::Context;
    use std::path::Path;

    fn compile_full(source: &str, root: &Path) -> Result<String, String> {
        let tokens = lexer::tokenize(source).map_err(|e| format!("tokenize: {}", e))?;
        let ast = parser::parse(tokens).map_err(|e| format!("parse: {}", e))?;
        let program = analyzer::analyze(&ast).unwrap_or(ast);
        let context = Context::create();
        let mut backend = LlvmBackend::new(&context, "regression");
        backend.declare_natives();
        backend.compile_program_to_ir_text(&program, root)
    }

    #[test]
    fn super_init_compiles_parent_declared_after_child() {
        let source = r#"
            public class Child extends Parent {
                public fn init(x: int) { super.init(x); }
            }
            public class Parent {
                public int x;
                public fn init(x: int) { this.x = x; }
            }
            public fn main(): void {
                let c: Child = new Child(3);
            }
        "#;
        let ir = compile_full(source, Path::new(".")).expect("compile");
        assert!(ir.contains("call void @Parent_new"), "{}", ir);
    }

    #[test]
    fn overloaded_class_methods_bind_by_arity() {
        let source = r#"
            public class Box<T> {
                public T value;
                public fn init(v: T) { this.value = v; }
                public fn put(v: T): void { this.put(v, 1); }
                public fn put(v: T, n: int): void { this.value = v; }
            }
            public fn main(): void {
                let b: Box<string> = new Box<string>("x");
                b.put("y");
                b.put("z", 2);
            }
        "#;
        let ir = compile_full(source, Path::new(".")).expect("compile");
        assert!(ir.contains("@Box__string_put__a3"), "{}", ir);
        assert!(ir.contains("call void @Box__string_put__a3"), "{}", ir);
    }

    #[test]
    fn bare_method_call_prefers_implicit_this_over_wrong_arity_function() {
        let source = r#"
            public fn pack(widget: int, side: int): void {}
            public class Widget {
                public fn init() {}
                public fn pack(options: int): void {}
                public fn packDefault(): void { pack(1); }
            }
            public fn main(): void {
                let w: Widget = new Widget();
                w.packDefault();
            }
        "#;
        let ir = compile_full(source, Path::new(".")).expect("compile");
        assert!(ir.contains("call void @Widget_pack("), "{}", ir);
    }

    #[test]
    fn constructor_overloads_bind_by_new_arity() {
        let source = r#"
            public class Pair {
                public int a;
                public fn init() { this.a = 0; }
                public fn init(x: int) { this.a = x; }
            }
            public fn main(): void {
                let p: Pair = new Pair();
                let q: Pair = new Pair(2);
            }
        "#;
        let ir = compile_full(source, Path::new(".")).expect("compile");
        assert!(ir.contains("call void @Pair_new("), "{}", ir);
        assert!(ir.contains("call void @Pair_new__a2("), "{}", ir);
    }

    #[test]
    fn renamed_function_symbol_has_valid_linkage() {
        let source = r#"
            var _facility: int = 0;
            public fn _facility(name: string, fallback: int): int { return fallback; }
            public fn main(): void {
                let v: int = _facility("kern", 1);
            }
        "#;
        let ir = compile_full(source, Path::new(".")).expect("verify");
        assert!(!ir.contains("declare internal"), "{}", ir);
    }

    #[test]
    fn module_qualified_call_in_constructor_binds_before_class_bodies() {
        let root = std::env::temp_dir().join(format!("titrate_modresolve_{}", std::process::id()));
        let mods = root.join("mods");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&mods).expect("create temp modules");
        std::fs::write(
            mods.join("a.tr"),
            "public fn load(path: string): int { return 1; }",
        )
        .expect("write a.tr");
        std::fs::write(
            mods.join("b.tr"),
            "public fn load(path: string): string { return \"b\"; }",
        )
        .expect("write b.tr");
        let source = r#"
            import mods::a;
            import mods::b;
            public class C {
                public fn init() {
                    let v: int = a::load("x");
                }
            }
            public fn main(): void {
                let c: C = new C();
            }
        "#;
        let result = compile_full(source, &root);
        let _ = std::fs::remove_dir_all(&root);
        let ir = result.expect("compile");
        assert!(ir.contains("call i32 @load"), "{}", ir);
        assert!(!ir.contains("call { i64, ptr } @load"), "{}", ir);
    }
}