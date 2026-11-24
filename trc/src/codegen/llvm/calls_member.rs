// Member-access and native-fallback call dispatch

use super::native_bridge;
use super::vtable::{emit_direct_call, emit_field_access, emit_interface_method_call, emit_virtual_call};
use super::{IfaceMethodSig, LlvmBackend};
use crate::ast::{ClassMember, Expr, Type};
use inkwell::types::{BasicMetadataTypeEnum, BasicType};
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum};

impl<'ctx> LlvmBackend<'ctx> {
    /// True when `class_name` declares an instance method named `method`.
    /// Native dispatch must not hijack such calls: e.g. `Regex.match(pattern)`
    /// is a class method taking a string, while the `Regex_match` native takes
    /// (pattern, input) and would receive the Regex struct as its pattern.
    fn class_declares_method(&self, class_name: &str, method: &str) -> bool {
        // Builtin containers have stdlib method declarations but dispatch
        // through native bridge calls, not user class methods.
        if matches!(class_name, "ArrayList" | "HashMap" | "array") {
            return false;
        }
        self.class_decls.get(class_name).is_some_and(|cd| {
            cd.members.iter().any(|m| match m {
                ClassMember::Method(mm) => mm.name == method,
                _ => false,
            })
        })
    }

    /// True when `class_name` or any ancestor declares an instance method
    /// named `method`. A fn-valued field must not shadow ordinary dispatch.
    fn class_chain_declares_method(&self, class_name: &str, method: &str) -> bool {
        let mut current = Some(class_name.to_string());
        let mut visited = std::collections::HashSet::new();
        while let Some(name) = current {
            if !visited.insert(name.clone()) {
                break;
            }
            let has_method = self
                .class_infos
                .get(&name)
                .is_some_and(|ci| ci.method_names.iter().any(|m| m == method))
                || self.class_decls.get(&name).is_some_and(|cd| {
                    cd.members
                        .iter()
                        .any(|m| matches!(m, ClassMember::Method(mm) if mm.name == method))
                });
            if has_method {
                return true;
            }
            current = self
                .class_infos
                .get(&name)
                .and_then(|ci| ci.parent.clone())
                .or_else(|| {
                    self.class_decls
                        .get(&name)
                        .and_then(|cd| cd.parent.as_ref().map(|t| t.name().to_string()))
                });
        }
        false
    }

    /// Sentinel carried in an int-typed slot to mean "null" (a missing key).
    /// `Integer.toString` renders it as "null" just like the VM does.
    const NULL_INT_SENTINEL: i64 = i32::MIN as i64;

    /// Call `HashMap_get` and reinterpret its raw payload as the map's declared
    /// value type. Reads the raw `TitrateValue` so a missing key (TV_NULL /
    /// TV_VOID) can be distinguished from a stored zero.
    fn compile_hashmap_get_typed(
        &mut self,
        obj: &Expr,
        key: &Expr,
        value_ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let obj_val = self.compile_expr(obj)?;
        let key_val = self.compile_expr(key)?;
        let obj_ty = self.infer_expr_type(obj);
        let key_ty = self.infer_expr_type(key);
        let tv = self.compile_native_call_raw(
            "HashMap_get",
            &[obj_val, key_val],
            &[obj_ty, key_ty],
        )?;
        let tag = self
            .builder
            .build_extract_value(tv, 0, "map.get.tag")
            .map_err(|e| format!("map get tag extract failed: {:?}", e))?
            .into_int_value();
        let i32_ty = self.context.i32_type();
        let is_null = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::ULE,
                tag,
                i32_ty.const_int(native_bridge::TV_NULL as u64, false),
                "map.get.isnull",
            )
            .map_err(|e| format!("map get null cmp failed: {:?}", e))?;
        let payload = self
            .builder
            .build_extract_value(tv, 2, "map.get.payload")
            .map_err(|e| format!("map get payload extract failed: {:?}", e))?
            .into_array_value();
        let payload_ty = self.context.i8_type().array_type(16);
        let slot = self
            .builder
            .build_alloca(payload_ty, "map.get.payload.slot")
            .map_err(|e| format!("map get payload alloca failed: {:?}", e))?;
        self.builder
            .build_store(slot, payload)
            .map_err(|e| format!("map get payload store failed: {:?}", e))?;
        let val_ptr = self
            .builder
            .build_bit_cast(slot, self.context.ptr_type(Default::default()), "map.get.valptr")
            .map_err(|e| format!("map get payload bitcast failed: {:?}", e))?
            .into_pointer_value();
        // Class instances (e.g. JsonValue) are stored in the container as a
        // raw pointer in the payload. Read it back as a pointer, not an int,
        // otherwise `get` yields a truncated bogus address.
        if self.class_infos.contains_key(value_ty.name()) {
            let bits = self
                .builder
                .build_load(self.context.i64_type(), val_ptr, "map.get.ptr.bits")
                .map_err(|e| format!("map get ptr load failed: {:?}", e))?
                .into_int_value();
            let ptr_ty = self.context.ptr_type(Default::default());
            let ptr = self
                .builder
                .build_int_to_ptr(bits, ptr_ty, "map.get.ptr")
                .map_err(|e| format!("map get int_to_ptr failed: {:?}", e))?;
            return self
                .builder
                .build_select(is_null, ptr_ty.const_null(), ptr, "map.get.value")
                .map_err(|e| format!("map get ptr select failed: {:?}", e));
        }
        let raw = match value_ty.name() {
            "double" | "float" | "half" | "quad" => {
                let d = self
                    .builder
                    .build_load(self.context.f64_type(), val_ptr, "map.get.double")
                    .map_err(|e| format!("map get double load failed: {:?}", e))?;
                return Ok(d);
            }
            "long" | "u64" | "size" => self
                .builder
                .build_load(self.context.i64_type(), val_ptr, "map.get.long")
                .map_err(|e| format!("map get long load failed: {:?}", e))?
                .into_int_value(),
            _ => self
                .builder
                .build_load(i32_ty, val_ptr, "map.get.int")
                .map_err(|e| format!("map get int load failed: {:?}", e))?
                .into_int_value(),
        };
        let sentinel = i32_ty.const_int(Self::NULL_INT_SENTINEL as u64, true);
        let result = self
            .builder
            .build_select(is_null, sentinel, raw, "map.get.value")
            .map_err(|e| format!("map get select failed: {:?}", e))?;
        if value_ty.name() == "bool" {
            return self
                .builder
                .build_int_truncate(result.into_int_value(), self.context.bool_type(), "map.get.bool")
                .map(|v| v.into())
                .map_err(|e| format!("map get bool trunc failed: {:?}", e));
        }
        Ok(result)
    }

    /// Find a field named `field` whose declared Titrate type is `fn(...)`,
    /// walking the class and its ancestors. The full fn type is returned with
    /// the return type as its last parameter (see `Type::Display`).
    fn fn_field_type(&self, class_name: &str, field: &str) -> Option<Type> {
        let mut current = Some(class_name.to_string());
        let mut visited = std::collections::HashSet::new();
        while let Some(name) = current {
            if !visited.insert(name.clone()) {
                break;
            }
            if let Some(ty) = self.field_titrate_type_names.get(&format!("{}_{}", name, field)) {
                if ty.name() == "fn" {
                    return Some(ty.clone());
                }
            }
            if let Some(ty) = self.class_decls.get(&name).and_then(|cd| {
                cd.members.iter().find_map(|m| match m {
                    ClassMember::Field(f) if f.name == field => Some(f.typ.clone()),
                    _ => None,
                })
            }) {
                return (ty.name() == "fn").then_some(ty);
            }
            current = self
                .class_infos
                .get(&name)
                .and_then(|ci| ci.parent.clone())
                .or_else(|| {
                    self.class_decls
                        .get(&name)
                        .and_then(|cd| cd.parent.as_ref().map(|t| t.name().to_string()))
                });
        }
        None
    }

    /// Dispatch `obj.fnField(args)` through the closure struct held in the
    /// field, rather than treating `fnField` as a virtual method on `obj`.
    /// The field stores `{fn_ptr, capture}` (see `compile_closure`), so the
    /// trampoline is called with `capture` as its hidden first argument.
    fn try_compile_fn_field_call(
        &mut self,
        callee: &Expr,
        args: &[Expr],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        let Expr::MemberAccess(obj, field, _) = callee else {
            return Ok(None);
        };
        let obj_ty = self.infer_expr_type(obj);
        // Generic receivers are compiled under a mangled name. Resolve before
        // any class_infos lookup or field-slot search, as method dispatch does.
        let class_name = self.resolve_mono_class(&obj_ty)?;
        let Some(ty) = self.fn_field_type(&class_name, field) else {
            return Ok(None);
        };
        // Ordinary methods win over a fn-valued field of the same name.
        if self.class_chain_declares_method(&class_name, field) {
            return Ok(None);
        }
        let Type::Named { params, .. } = ty else {
            return Ok(None);
        };
        if params.is_empty() {
            return Ok(None);
        }
        let (sig_params, ret_part) = params.split_at(params.len() - 1);
        let ret = &ret_part[0];
        // A generic signature has no concrete calling convention; leave such
        // calls to the existing fallbacks (they appear only in uninstantiated
        // default methods).
        if sig_params.iter().any(super::fn_sig_has_type_param)
            || super::fn_sig_has_type_param(ret)
        {
            return Ok(None);
        }
        if args.len() != sig_params.len() {
            return Ok(None);
        }

        // Confirm the receiver's static layout holds a {fn_ptr, capture}
        // closure before emitting any IR. If the field is not a struct here,
        // loading it would leave dead instructions behind on Ok(None).
        let Some(class_info) = self.class_infos.get(&class_name).cloned() else {
            return Ok(None);
        };
        let Some(field_idx) = super::vtable::field_index(&class_info, field) else {
            return Ok(None);
        };
        if !class_info
            .struct_type
            .get_field_types()
            .get(field_idx as usize)
            .is_some_and(|ty| ty.is_struct_type())
        {
            return Ok(None);
        }

        // All structural checks passed; only now emit IR. Resolve the receiver
        // to an object pointer first: an element read from an ArrayList arrives
        // as a {i64, ptr} payload whose field 1 holds the object pointer. The
        // same extraction is used for ordinary method dispatch below.
        let obj_val = self.compile_expr(obj)?;
        let obj_ptr = if obj_val.is_pointer_value() {
            obj_val.into_pointer_value()
        } else if obj_val.is_struct_value() && self.is_string_struct(&obj_val) {
            let raw = self
                .builder
                .build_extract_value(obj_val.into_struct_value(), 1, "fnfield.obj.ptr")
                .map_err(|e| format!("extract fn-field receiver ptr failed: {:?}", e))?;
            if !raw.is_pointer_value() {
                return Ok(None);
            }
            raw.into_pointer_value()
        } else {
            return Ok(None);
        };
        // Use the resolved class (not the receiver's raw inferred name) so the
        // field slot comes from the mangled specialization when applicable.
        let closure =
            emit_field_access(self.context, &self.builder, &class_info, obj_ptr, field)?;
        if !closure.is_struct_value() {
            return Ok(None);
        }
        let sv = closure.into_struct_value();
        let fn_i8 = self
            .builder
            .build_extract_value(sv, 0, "fnfield.fn")
            .map_err(|e| format!("extract fn field fn_ptr failed: {:?}", e))?
            .into_pointer_value();
        let capture = self
            .builder
            .build_extract_value(sv, 1, "fnfield.capture")
            .map_err(|e| format!("extract fn field capture failed: {:?}", e))?
            .into_pointer_value();

        // Build the indirect call with the trampoline signature:
        // fn(i8* capture_data, params...) -> ret (closures.rs:425-429).
        let i8_ptr = self.context.ptr_type(inkwell::AddressSpace::default());
        let mut param_tys: Vec<BasicMetadataTypeEnum<'ctx>> = Vec::new();
        param_tys.push(i8_ptr.into());
        for p in sig_params {
            param_tys.push(self.user_param_llvm_type(p)?.into());
        }
        let ret_ty = self.sig_ret_llvm_type(Some(ret))?;
        let fn_ty = match ret_ty {
            Some(rt) => rt.fn_type(&param_tys, false),
            None => self.context.void_type().fn_type(&param_tys, false),
        };
        let callee_ptr = self
            .builder
            .build_bit_cast(fn_i8, i8_ptr, "fnfield.callee")
            .map_err(|e| format!("bitcast fn field callee failed: {:?}", e))?
            .into_pointer_value();
        let mut call_args: Vec<BasicMetadataValueEnum<'ctx>> = Vec::new();
        call_args.push(capture.into());
        for (i, arg) in args.iter().enumerate() {
            call_args.push(self.coerce_user_call_arg(arg, sig_params.get(i), None)?);
        }
        let call = self
            .builder
            .build_indirect_call(fn_ty, callee_ptr, &call_args, "fnfield.call")
            .map_err(|e| format!("indirect call through fn field failed: {:?}", e))?;
        let result = if ret_ty.is_some() {
            match call.try_as_basic_value() {
                inkwell::values::ValueKind::Basic(v) => v,
                _ => return Err("fn field call did not return a value".to_string()),
            }
        } else {
            self.context.i32_type().const_int(0, false).into()
        };
        Ok(Some(result))
    }

    pub(super) fn compile_call_member(&mut self, callee: &Expr, args: &[Expr]) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        if let Expr::Identifier(name, _) = callee {
            if name == "Sys_args" && args.is_empty() {
                return self.compile_sys_args().map(Some);
            }
        }

        // A function-valued field is a closure struct, not a vtable method.
        // Try it only when the name is not a real method on the receiver's
        // class, so ordinary method dispatch is never shadowed. Native and
        // container fallbacks still run afterward for non-field calls.
        if let Some(v) = self.try_compile_fn_field_call(callee, args)? {
            return Ok(Some(v));
        }

        // Builtin containers have no dedicated native for `.clone()` or the
        // single-argument `.update(other)` merge. Lower them through the
        // container buffer / HashMap_put native before the generic
        // TypeName_methodName dispatch, which cannot resolve those names.
        if let Expr::MemberAccess(obj, method, _) = callee {
            let obj_ty = self.infer_expr_type(obj);
            let type_name = obj_ty.name();
            if matches!(type_name, "ArrayList" | "array" | "HashMap") {
                if method == "clone" && args.is_empty() {
                    return self.compile_container_clone(obj).map(Some);
                }
                if type_name == "HashMap" && method == "update" {
                    if args.len() == 2 {
                        let map_val = self.compile_expr(obj)?;
                        let key_val = self.compile_expr(&args[0])?;
                        let val_val = self.compile_expr(&args[1])?;
                        let key_ty = self.infer_expr_type(&args[0]);
                        let val_ty = self.infer_expr_type(&args[1]);
                        let result = self.compile_native_call_declared(
                            "HashMap_put",
                            &[map_val, key_val, val_val],
                            &[obj_ty.clone(), key_ty, val_ty],
                            Some(&obj_ty),
                        )?;
                        self.store_back_container(obj, result)?;
                        return Ok(Some(result));
                    }
                    if args.len() == 1 {
                        let result = self.compile_hashmap_update(obj, &args[0])?;
                        self.store_back_container(obj, result)?;
                        return Ok(Some(result));
                    }
                }
            }
        }

        // Result<T, E> methods on a returned Result value: isOk()/isErr().
        if let Expr::MemberAccess(obj, method, _) = callee {
            if self.infer_expr_type(obj).name() == "Result"
                && (method == "isOk" || method == "isErr")
            {
                let val = self.compile_expr(obj)?;
                if let BasicValueEnum::StructValue(sv) = val {
                    let tag = self.builder.build_extract_value(sv, 0, "result.tag")
                        .map_err(|e| format!("result tag extract failed: {:?}", e))?
                        .into_int_value();
                    let ok = self.builder.build_int_compare(
                        inkwell::IntPredicate::EQ,
                        tag,
                        self.context.i32_type().const_zero(),
                        "result.isok",
                    ).map_err(|e| format!("result isOk compare failed: {:?}", e))?;
                    let ret = if method == "isOk" { ok } else {
                        self.builder.build_not(ok, "result.iserr")
                            .map_err(|e| format!("result isErr failed: {:?}", e))?
                    };
                    return Ok(Some(ret.into()));
                }
            }
        }

        // Primitive receiver method call: `x.toString()`. The generic prefix
        // trial below would bind `double.toString()` to `ArrayList_toString`
        // (same native name shape) and pass a double where a container is
        // expected. Marshal directly through the `toString` native instead.
        if let Expr::MemberAccess(obj, method, _) = callee {
            let obj_ty = self.infer_expr_type(obj);
            if method == "toString"
                && matches!(obj_ty.name(),
                    "int" | "double" | "float" | "long" | "bool" | "char"
                    | "byte" | "short" | "half" | "quad" | "vast" | "uvast")
            {
                let val = self.compile_expr(obj)?;
                let is_int_ty = obj_ty.name() == "int";
                let normal = self.compile_native_call_declared(
                    "toString",
                    &[val],
                    &[obj_ty],
                    Some(&Type::simple("string")),
                )?;
                // An int sentinel from a missing HashMap.get prints as "null".
                if is_int_ty && val.is_int_value() {
                    let iv = val.into_int_value();
                    let is_null = self
                        .builder
                        .build_int_compare(
                            inkwell::IntPredicate::EQ,
                            iv,
                            iv.get_type().const_int(Self::NULL_INT_SENTINEL as u64, true),
                            "to_string.isnull",
                        )
                        .map_err(|e| format!("toString null cmp failed: {:?}", e))?;
                    let null_sv = self.make_string_global("null");
                    let null_str = self.string_value_to_basic(null_sv)?;
                    let result = self
                        .builder
                        .build_select(is_null, null_str, normal, "to_string.value")
                        .map_err(|e| format!("toString null select failed: {:?}", e))?;
                    return Ok(Some(result));
                }
                return Ok(Some(normal));
            }
        }

        // Native function call: Math.sin(x), String.length(s), parseInt(s), etc.
        // This is a fallback after user-defined functions have been checked.
        // But first check for container method calls (obj.method(args)) which need
        // special dispatch to compile_array_get_string, compile_array_length, etc.
        if let Expr::MemberAccess(obj, method, _) = callee {
            if let Expr::Identifier(name, _) = obj.as_ref() {
                if self.locals.contains_key(name.as_str()) {
                    if let Some(titrate_type) = self
                        .locals
                        .get(name.as_str())
                        .and_then(|v| v.titrate_type.as_ref())
                    {
                        if *titrate_type == "ArrayList" {
                            if method == "get" && args.len() == 1 {
                                return (self.compile_array_get_string(obj, &args[0])).map(Some);
                            }
                            if method == "size" && args.is_empty() {
                                return (self.compile_array_length(obj)).map(Some);
                            }
                            if method == "sort" && args.is_empty() {
                                return (self.compile_array_sort(obj)).map(Some);
                            }
                        }
                    }
                }
            }
        }

        if let Some(native_name) = native_bridge::try_native_call_name(callee) {
            let mut arg_vals: Vec<BasicValueEnum> = Vec::new();
            let mut arg_types: Vec<Type> = Vec::new();
            for arg in args {
                let arg_ty = self.infer_expr_type(arg);
                let val = self.compile_expr(arg)?;
                arg_vals.push(val);
                arg_types.push(arg_ty);
            }
            let result = self.compile_native_call_declared(
                &native_name,
                &arg_vals,
                &arg_types,
                None,
            )?;
            // A missing HashMap.get carries the int null sentinel; render it
            // as "null" just like the VM's dynamically typed toString does.
            if native_name == "Integer_toString" && arg_vals.len() == 1 && arg_vals[0].is_int_value() {
                let iv = arg_vals[0].into_int_value();
                let is_null = self
                    .builder
                    .build_int_compare(
                        inkwell::IntPredicate::EQ,
                        iv,
                        iv.get_type().const_int(Self::NULL_INT_SENTINEL as u64, true),
                        "to_string.isnull",
                    )
                    .map_err(|e| format!("toString null cmp failed: {:?}", e))?;
                let null_sv = self.make_string_global("null");
                let null_str = self.string_value_to_basic(null_sv)?;
                let result = self
                    .builder
                    .build_select(is_null, null_str, result, "to_string.value")
                    .map_err(|e| format!("toString null select failed: {:?}", e))?;
                return Ok(Some(result));
            }
            return Ok(Some(result));
        }

        // Container method call: args.size(), parts.get(i), etc.
        // Resolve the type of the object to find the correct native function name.
        if let Expr::MemberAccess(obj, method, _) = callee {
            // For Sys_args().size() or String.split(x, y).size() style calls.
            // Check if the obj is a Call or StaticCall returning a known container type.
            let obj_type_name: Option<String> = match obj.as_ref() {
                Expr::Call(inner_callee, _, _, _) => native_bridge::try_native_call_name(inner_callee)
                    .map(|n| {
                        native_bridge::infer_native_return_type(&n)
                            .name()
                            .to_string()
                    }),
                Expr::StaticCall {
                    class_name,
                    method: inner_method,
                    ..
                } => {
                    let native_name = format!("{}_{}", class_name, inner_method);
                    if native_bridge::is_native_function(&native_name) {
                        Some(
                            native_bridge::infer_native_return_type(&native_name)
                                .name()
                                .to_string(),
                        )
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if let Some(ref type_name_str) = obj_type_name {
                let native_prefix: &str = match type_name_str.as_str() {
                    "ArrayList" | "array" => "ArrayList",
                    "HashMap" => "HashMap",
                    "string" => "String",
                    other => other,
                };
                // Method name aliases: Titrate method names that differ from native function names.
                let mut resolved_method = match (native_prefix, method.as_str()) {
                    ("HashMap", "hasKey") => "containsKey".to_string(),
                    ("Regex", "matches") => "match".to_string(),
                    _ => method.clone(),
                };
                // ArrayList.remove(int) removes by index; the native
                // `ArrayList_remove` removes by value. Use removeAt instead.
                if native_prefix == "ArrayList" && method == "remove" && args.len() == 1
                    && matches!(self.infer_expr_type(&args[0]).name(),
                        "int" | "long" | "u32" | "u64" | "size" | "byte" | "short" | "u8" | "u16")
                {
                    resolved_method = "removeAt".to_string();
                }
                let full_native = format!("{}_{}", native_prefix, resolved_method);
                // Special handling for ArrayList.get — use type-aware compile_array_get_string.
                if native_prefix == "ArrayList" && method == "get" && args.len() == 1 {
                    return (self.compile_array_get_string(obj, &args[0])).map(Some);
                }
                // Special handling for ArrayList.size — use compile_array_length.
                if native_prefix == "ArrayList" && method == "size" && args.is_empty() {
                    return (self.compile_array_length(obj)).map(Some);
                }
                // No ArrayList_subList native exists; lower the slice inline.
                if native_prefix == "ArrayList" && method == "subList" && args.len() == 2 {
                    return self.compile_sublist(obj, &args[0], &args[1]).map(Some);
                }
                if native_bridge::is_native_function(&full_native)
                    && self.module.get_function(&full_native).is_none()
                {
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
                    return (self.compile_native_call_declared(
                        &full_native,
                        &arg_vals,
                        &arg_types,
                        None,
                    )).map(Some);
                }
            }
            // Collect all Identifier / MemberAccess objects to try
            let titrate_type_name: Option<String> = match obj.as_ref() {
                Expr::Identifier(name, _) => self
                    .locals
                    .get(name.as_str())
                    .and_then(|v| v.titrate_type.as_deref())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        self.globals
                            .get(name.as_str())
                            .map(|g| g.titrate_ty.name().to_string())
                    }),
                Expr::MemberAccess(_, _, _) => {
                    // The receiver is a field access (e.g. `this.x`): use the
                    // field's type, not the enclosing object's class name.
                    let ty = self.infer_expr_type(obj);
                    Some(ty.name().to_string())
                }
                Expr::Call(_, _, _, _) => {
                    // Call infer_expr_type on the Call itself (not its callee),
                    // so that e.g. g.get(a).put(b,d) correctly resolves via
                    // the Expr::Call branch which handles MemberAccess callees.
                    let ty = self.infer_expr_type(obj);
                    Some(ty.name().to_string())
                }
                _ => None,
            };
            if let Some(ref type_name) = titrate_type_name {
                let native_prefix: &str = match type_name.as_str() {
                    "ArrayList" | "array" => "ArrayList",
                    "HashMap" => "HashMap",
                    "string" => "String",
                    other => other,
                };
                // Method name aliases: Titrate method names that differ from native function names.
                let mut resolved_method = match (native_prefix, method.as_str()) {
                    ("HashMap", "hasKey") => "containsKey".to_string(),
                    ("Regex", "matches") => "match".to_string(),
                    _ => method.clone(),
                };
                if native_prefix == "ArrayList" && method == "remove" && args.len() == 1
                    && matches!(self.infer_expr_type(&args[0]).name(),
                        "int" | "long" | "u32" | "u64" | "size" | "byte" | "short" | "u8" | "u16")
                {
                    resolved_method = "removeAt".to_string();
                }
                let native_name = format!("{}_{}", native_prefix, resolved_method);
                // Element-type-aware ArrayList.get / ArrayList.size for any
                // ArrayList-typed receiver (locals, `this.field`, `obj.field`).
                // The generic native bridge returns a `{i64, ptr}` string struct
                // for every `ArrayList_get` result, which corrupts numeric
                // elements (the codegen then reinterprets the payload bits).
                if native_prefix == "ArrayList" && method == "get" && args.len() == 1 {
                    return (self.compile_array_get_string(obj, &args[0])).map(Some);
                }
                if native_prefix == "HashMap" && method == "get" && args.len() == 1 {
                    if let Some(value_ty) = self.infer_expr_type(obj).params().get(1).cloned() {
                        if matches!(value_ty.name(),
                            "int" | "double" | "float" | "long" | "bool" | "byte"
                            | "short" | "half" | "quad" | "u8" | "u16" | "u32" | "u64" | "size")
                            || self.class_infos.contains_key(value_ty.name())
                        {
                            return self.compile_hashmap_get_typed(obj, &args[0], &value_ty).map(Some);
                        }
                    }
                }
                if native_prefix == "ArrayList" && method == "size" && args.is_empty() {
                    return (self.compile_array_length(obj)).map(Some);
                }
                if native_prefix == "ArrayList" && method == "subList" && args.len() == 2 {
                    return self.compile_sublist(obj, &args[0], &args[1]).map(Some);
                }
                if native_bridge::is_native_function(&native_name)
                    && !self.class_declares_method(type_name, method)
                    && self.module.get_function(&native_name).is_none()
                {
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
                    let result = self.compile_native_call_declared(
                        &native_name,
                        &arg_vals,
                        &arg_types,
                        None,
                    )?;
                    if Self::is_container_mutator(&native_name) {
                        self.store_back_container(obj, result)?;
                    }
                    return Ok(Some(result));
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
                    // Shape args and the return type by the declared interface
                    // signature so the indirect call matches every
                    // implementation (container slots, nested fat pointers).
                    let sig = self
                        .iface_method_sigs
                        .get(&(iface_name.to_string(), method.clone()))
                        .cloned()
                        .unwrap_or(IfaceMethodSig {
                            params: Vec::new(),
                            ret: None,
                        });
                    let mut arg_vals: Vec<BasicValueEnum> = Vec::new();
                    for (i, arg) in args.iter().enumerate() {
                        arg_vals.push(Self::meta_to_basic(
                            self.coerce_user_call_arg(arg, sig.params.get(i), None)?,
                        )?);
                    }
                    let ret_ty = self.sig_ret_llvm_type(sig.ret.as_ref())?;
                    return (emit_interface_method_call(
                        self.context,
                        &self.builder,
                        iface_info.fat_ptr_type,
                        obj_val,
                        &iface_info.method_names,
                        method,
                        &arg_vals,
                        ret_ty,
                    )).map(Some);
                }
                // Route container .get() and .size() to typed implementations
                // when the object is an ArrayList (struct value {i64, ptr}).
                if let Expr::Identifier(name, _) = &**obj {
                    if let Some(local) = self.locals.get(name) {
                        if let Some(ref titrate_type) = local.titrate_type {
                            if titrate_type == "ArrayList" || titrate_type == "array" {
                                if method == "get" && args.len() == 1 {
                                    return (self.compile_array_get_string(obj, &args[0])).map(Some);
                                }
                                if method == "size" && args.is_empty() {
                                    return (self.compile_array_length(obj)).map(Some);
                                }
                                // For other methods, fall through to native bridge
                            }
                        }
                    }
                }
            }
            let obj_ptr_opt = if obj_val.is_pointer_value() {
                Some(obj_val.into_pointer_value())
            } else if obj_val.is_struct_value() && self.is_string_struct(&obj_val) {
                // A class element read from a generic ArrayList arrives as a
                // {i64, ptr} struct; field 1 holds the object pointer.
                if let BasicValueEnum::StructValue(sv) = obj_val {
                    self.builder.build_extract_value(sv, 1, "obj.ptr").ok()
                        .and_then(|v| if v.is_pointer_value() { Some(v.into_pointer_value()) } else { None })
                } else {
                    None
                }
            } else {
                None
            };
            if let Some(obj_ptr) = obj_ptr_opt {
                // Look up the class name from the object's type,
                // instantiating generic receivers on demand.
                let obj_type = self.infer_expr_type(obj);
                let resolved = self.resolve_mono_class(&obj_type)?;
                let class_name = resolved.as_str();
                if let Some(class_info) = self.class_infos.get(class_name).cloned() {
                    // Statically dispatch only when no subclass overrides
                    // the method; a subclass instance in this variable must
                    // reach its override through dynamic dispatch.
                    let overridden = self.class_infos.values().any(|info| {
                        info.name != class_name
                            && info.method_names.iter().any(|m| m == method)
                            && self.is_subclass_of(&info.name, class_name)
                    });
                    // First try direct method call (look up the method function by name).
                    let method_fn_name = format!("{}_{}", class_name, method);
                    if !overridden {
                        let method_fn = self
                            .fn_overloads
                            .get(&(method_fn_name.clone(), args.len() + 1))
                            .copied()
                            .or_else(|| self.module.get_function(&method_fn_name));
                        if let Some(method_fn) = method_fn {
                            let method_params = self.method_decl_params(class_name, method);
                            let arg_vals = self.build_this_call_args(
                                method_fn,
                                method_params.as_deref(),
                                args,
                            )?;
                            return (emit_direct_call(
                                self.context,
                                &self.builder,
                                method_fn,
                                obj_ptr,
                                &arg_vals,
                            )).map(Some);
                        }
                        // Inherited method: no override exists below this
                        // class, so the nearest ancestor implementation is
                        // the correct target. Walk the parent chain (child
                        // structs embed parent fields first, so `this`
                        // passes through unchanged).
                        let mut ancestor = class_info.parent.clone();
                        let mut seen = std::collections::HashSet::new();
                        while let Some(pname) = ancestor {
                            if !seen.insert(pname.clone()) {
                                break;
                            }
                            let cand = format!("{}_{}", pname, method);
                            if let Some(mf) = self.module.get_function(&cand) {
                                let mp = self.method_decl_params(&pname, method);
                                let arg_vals = self.build_this_call_args(
                                    mf,
                                    mp.as_deref(),
                                    args,
                                )?;
                                return (emit_direct_call(
                                    self.context,
                                    &self.builder,
                                    mf,
                                    obj_ptr,
                                    &arg_vals,
                                )).map(Some);
                            }
                            ancestor = self
                                .class_infos
                                .get(&pname)
                                .and_then(|i| i.parent.clone());
                        }
                    }
                    // Fall back to virtual call. Declared parameter types shape
                    // the args (container slots, fat pointers) like the
                    // overrides expect; the indirect type follows the args.
                    let method_params = self.method_decl_params(class_name, method);
                    let mut arg_vals: Vec<BasicValueEnum> = Vec::new();
                    for (i, arg) in args.iter().enumerate() {
                        match method_params.as_ref().and_then(|p| p.get(i)) {
                            Some(ty) => arg_vals.push(Self::meta_to_basic(
                                self.coerce_user_call_arg(arg, Some(ty), None)?,
                            )?),
                            None => {
                                if Self::is_shared_container_ty(&self.infer_expr_type(arg)) {
                                    arg_vals.push(self.container_slot_ptr(arg)?.into());
                                } else {
                                    arg_vals.push(self.compile_expr(arg)?);
                                }
                            }
                        }
                    }
                    let ret_ty = self
                        .resolve_method_return(class_name, method)
                        .and_then(|t| self.sig_ret_llvm_type(Some(&t)).ok().flatten());
                    return (emit_virtual_call(
                        self.context,
                        &self.builder,
                        &class_info,
                        obj_ptr,
                        method,
                        &arg_vals,
                        ret_ty,
                    )).map(Some);
                }
                // Fallback: look up the method function directly by convention
                // ClassName_methodName, even without class_infos.
                let method_fn_name = format!("{}_{}", class_name, method);
                if let Some(method_fn) = self.module.get_function(&method_fn_name) {
                    let method_params = self.method_decl_params(class_name, method);
                    let arg_vals = self.build_this_call_args(
                        method_fn,
                        method_params.as_deref(),
                        args,
                    )?;
                    return (emit_direct_call(
                        self.context,
                        &self.builder,
                        method_fn,
                        obj_ptr,
                        &arg_vals,
                    )).map(Some);
                }
                // Try generic container methods: ArrayList_get, HashMap_get, etc.
                if let Some(method_fn) = self.module.get_function(&format!("ArrayList_{}", method))
                {
                    let arg_vals = self.build_this_call_args(method_fn, None, args)?;
                    return (emit_direct_call(
                        self.context,
                        &self.builder,
                        method_fn,
                        obj_ptr,
                        &arg_vals,
                    )).map(Some);
                }
                if let Some(method_fn) = self.module.get_function(&format!("HashMap_{}", method)) {
                    let arg_vals = self.build_this_call_args(method_fn, None, args)?;
                    return (emit_direct_call(
                        self.context,
                        &self.builder,
                        method_fn,
                        obj_ptr,
                        &arg_vals,
                    )).map(Some);
                }
            }
        }

        // General native fallback for MemberAccess calls that weren't caught above.
        // Tries TypeName_methodName as a native function for any object type.
        if let Expr::MemberAccess(obj, method, _) = callee {
            let obj_ty = self.infer_expr_type(obj);
            let type_name = obj_ty.name();
            let native_prefix: &str = match type_name {
                "ArrayList" | "array" => "ArrayList",
                "HashMap" => "HashMap",
                "string" => "String",
                other => other,
            };
            let native_name = format!("{}_{}", native_prefix, method);
            // Element-type-aware ArrayList.get / ArrayList.size for any
            // ArrayList-typed receiver. The generic native bridge returns a
            // `{i64, ptr}` string struct for every `ArrayList_get` result,
            // which corrupts numeric elements (the codegen then reinterprets
            // the payload bits as a string).
            if native_prefix == "ArrayList" && method == "get" && args.len() == 1 {
                return (self.compile_array_get_string(obj, &args[0])).map(Some);
            }
            if native_prefix == "ArrayList" && method == "size" && args.is_empty() {
                return (self.compile_array_length(obj)).map(Some);
            }
            if native_prefix == "ArrayList" && method == "sort" && args.is_empty() {
                return (self.compile_array_sort(obj)).map(Some);
            }
            if native_bridge::is_native_function(&native_name)
                && !self.class_declares_method(type_name, method)
            {
                let mut arg_vals: Vec<BasicValueEnum> = Vec::new();
                let mut arg_types: Vec<Type> = Vec::new();
                let obj_val = self.compile_expr(obj)?;
                let obj_t = self.infer_expr_type(obj);
                arg_vals.push(obj_val);
                arg_types.push(obj_t);
                for arg in args {
                    let arg_ty = self.infer_expr_type(arg);
                    let val = self.compile_expr(arg)?;
                    arg_vals.push(val);
                    arg_types.push(arg_ty);
                }
                let result = self.compile_native_call_declared(
                    &native_name,
                    &arg_vals,
                    &arg_types,
                    None,
                )?;
                // Container mutators return (the updated container).map(Some); store it back
                // into the receiver so mutations persist across the value-copy
                // native bridge (e.g. `this.xs.add(v)` updates the field).
                if Self::is_container_mutator(&native_name) {
                    self.store_back_container(obj, result)?;
                }
                return Ok(Some(result));
            }
            // Last resort: compile obj as pointer and try ClassName_methodName as direct LLVM function.
            if let Ok(obj_val) = self.compile_expr(obj) {
                if obj_val.is_pointer_value() {
                    let obj_ptr = obj_val.into_pointer_value();
                    let direct_fn_name = format!("{}_{}", type_name, method);
                    if let Some(method_fn) = self.module.get_function(&direct_fn_name) {
                        let mut arg_vals: Vec<BasicValueEnum> = Vec::new();
                        for arg in args {
                            arg_vals.push(self.compile_expr(arg)?);
                        }
                        return (emit_direct_call(
                            self.context,
                            &self.builder,
                            method_fn,
                            obj_ptr,
                            &arg_vals,
                        )).map(Some);
                    }
                    // Try ArrayList_/HashMap_ as direct functions for generic containers.
                    if let Some(method_fn) =
                        self.module.get_function(&format!("ArrayList_{}", method))
                    {
                        let mut arg_vals: Vec<BasicValueEnum> = Vec::new();
                        for arg in args {
                            arg_vals.push(self.compile_expr(arg)?);
                        }
                        return (emit_direct_call(
                            self.context,
                            &self.builder,
                            method_fn,
                            obj_ptr,
                            &arg_vals,
                        )).map(Some);
                    }
                    if let Some(method_fn) =
                        self.module.get_function(&format!("HashMap_{}", method))
                    {
                        let mut arg_vals: Vec<BasicValueEnum> = Vec::new();
                        for arg in args {
                            arg_vals.push(self.compile_expr(arg)?);
                        }
                        return (emit_direct_call(
                            self.context,
                            &self.builder,
                            method_fn,
                            obj_ptr,
                            &arg_vals,
                        )).map(Some);
                    }
                }
            }
        }

        // Container fallback: for chained calls like g.get(a).put(b,d) or
        // Static-style calls through a class name (`Char.toString(x)`) are not
        // value receivers: route them to the static dispatcher. The prefix
        // trial below would otherwise hijack them (e.g. Char.toString
        // resolving to ArrayList_toString with a null receiver).
        if let Expr::MemberAccess(obj, method, _) = callee {
            if let Expr::Identifier(name, _) = obj.as_ref() {
                if !self.locals.contains_key(name.as_str())
                    && (self.class_infos.contains_key(name.as_str())
                        || name.starts_with(|c: char| c.is_uppercase()))
                {
                    return (self.compile_static_call(name, method, args)).map(Some);
                }
            }
        }
        // this._adj.containsKey(u), try all known container type prefixes.
        // This catches cases where the type inference returned "string" or
        // "unknown" instead of the actual container type.
        if let Expr::MemberAccess(obj, method, _) = callee {
            let resolved_method = match method.as_str() {
                "hasKey" => "containsKey",
                "matches" => "match",
                _ => method,
            };
            // Try the inferred type's own prefix first: the fixed order below
            // otherwise resolves `"a,b".split(",")` to Regex_split (same
            // arity, reversed argument order) before String is ever tried.
            let inferred_ty = self.infer_expr_type(obj);
            let inferred_prefix: &str = match inferred_ty.name() {
                "ArrayList" | "array" => "ArrayList",
                "HashMap" => "HashMap",
                "string" => "String",
                other => other,
            };
            for prefix in std::iter::once(inferred_prefix).chain(
                [
                    "ArrayList",
                    "HashMap",
                    "JsonValue",
                    "Regex",
                    "ZipFile",
                    "File",
                    "String",
                ]
                .into_iter()
                .filter(move |p| *p != inferred_prefix),
            ) {
                let native_name = format!("{}_{}", prefix, resolved_method);
                if native_bridge::is_native_function(&native_name) {
                    if let Ok(obj_val) = self.compile_expr(obj) {
                        let obj_t = self.infer_expr_type(obj);
                        let mut arg_vals: Vec<BasicValueEnum> = Vec::new();
                        let mut arg_types: Vec<Type> = Vec::new();
                        arg_vals.push(obj_val);
                        arg_types.push(obj_t);
                        for arg in args {
                            let arg_ty = self.infer_expr_type(arg);
                            let val = self.compile_expr(arg)?;
                            arg_vals.push(val);
                            arg_types.push(arg_ty);
                        }
                        let result = self.compile_native_call_declared(
                            &native_name,
                            &arg_vals,
                            &arg_types,
                            None,
                        )?;
                        if Self::is_container_mutator(&native_name) {
                            self.store_back_container(obj, result)?;
                        }
                        return Ok(Some(result));
                    }
                }
            }
        }
        Ok(None)
    }
}
