// Member-access and native-fallback call dispatch

use super::native_bridge;
use super::vtable::{emit_direct_call, emit_interface_method_call, emit_virtual_call};
use super::{IfaceMethodSig, LlvmBackend};
use crate::ast::{ClassMember, Expr, Type};
use inkwell::values::BasicValueEnum;

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

    /// Call `HashMap_get` and reinterpret its raw `{ i64, ptr }` payload as the
    /// map's declared value type. The generic native bridge always unmarshals
    /// `HashMap_get` as a string struct, which corrupts int/double/bool values.
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
        let result = self.compile_native_call("HashMap_get", &[obj_val, key_val], &[obj_ty, key_ty])?;
        let sv = match result {
            BasicValueEnum::StructValue(sv) if sv.get_type().count_fields() == 2 => sv,
            _ => return Ok(result),
        };
        let raw = self
            .builder
            .build_extract_value(sv, 0, "map.get.raw")
            .map_err(|e| format!("map get extract failed: {:?}", e))?
            .into_int_value();
        match value_ty.name() {
            "double" | "float" | "half" | "quad" => self
                .builder
                .build_bit_cast(raw, self.context.f64_type(), "map.get.double")
                .map_err(|e| format!("map get bitcast failed: {:?}", e)),
            "long" | "u64" | "size" => Ok(raw.into()),
            "bool" => self
                .builder
                .build_int_truncate(raw, self.context.bool_type(), "map.get.bool")
                .map(|v| v.into())
                .map_err(|e| format!("map get trunc failed: {:?}", e)),
            _ => self
                .builder
                .build_int_truncate(raw, self.context.i32_type(), "map.get.int")
                .map(|v| v.into())
                .map_err(|e| format!("map get trunc failed: {:?}", e)),
        }
    }

    pub(super) fn compile_call_member(&mut self, callee: &Expr, args: &[Expr]) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        if let Expr::Identifier(name, _) = callee {
            if name == "Sys_args" && args.is_empty() {
                return self.compile_sys_args().map(Some);
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

        // A JsonValue is an opaque native handle; its class-method toString
        // cannot run in native mode. The `Json_stringify` native produces the
        // same compact JSON the VM's JsonValue.toString() emits.
        if let Expr::MemberAccess(obj, method, _) = callee {
            if method == "toString" && self.infer_expr_type(obj).name() == "JsonValue" {
                let val = self.compile_expr(obj)?;
                return self
                    .compile_native_call("Json_stringify", &[val], &[Type::simple("JsonValue")])
                    .map(Some);
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
                return self.compile_native_call("toString", &[val], &[obj_ty]).map(Some);
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
            return (self.compile_native_call(&native_name, &arg_vals, &arg_types)).map(Some);
        }

        // Container method call: args.size(), parts.get(i), etc.
        // Resolve the type of the object to find the correct native function name.
        if let Expr::MemberAccess(obj, method, _) = callee {
            // For Sys_args().size() or String.split(x, y).size() style calls.
            // Check if the obj is a Call or StaticCall returning a known container type.
            let obj_type_name: Option<String> = match obj.as_ref() {
                Expr::Call(inner_callee, _, _) => native_bridge::try_native_call_name(inner_callee)
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
                    return (self.compile_native_call(&full_native, &arg_vals, &arg_types)).map(Some);
                }
            }
            // Collect all Identifier / MemberAccess objects to try
            let titrate_type_name: Option<String> = match obj.as_ref() {
                Expr::Identifier(name, _) => self
                    .locals
                    .get(name.as_str())
                    .and_then(|v| v.titrate_type.as_deref())
                    .map(|s| s.to_string()),
                Expr::MemberAccess(_, _, _) => {
                    // The receiver is a field access (e.g. `this.x`): use the
                    // field's type, not the enclosing object's class name.
                    let ty = self.infer_expr_type(obj);
                    Some(ty.name().to_string())
                }
                Expr::Call(_, _, _) => {
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
                    let result = self.compile_native_call(&native_name, &arg_vals, &arg_types)?;
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
                let result = self.compile_native_call(&native_name, &arg_vals, &arg_types)?;
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
                        let result =
                            self.compile_native_call(&native_name, &arg_vals, &arg_types)?;
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
