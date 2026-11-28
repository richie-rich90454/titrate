// Call-argument shaping and dispatch helpers

use super::native_bridge;
use super::types as llvm_types;
use super::vtable::{
    basic_metadata_to_basic_type, build_interface_fat_ptr_type, coerce_call_arg,
    emit_field_slot_ptr, emit_field_store, emit_interface_fat_ptr,
};
use super::{body_binds_name, stmt_contains_return, MethodBody};
use super::{ClosureSig, LlvmBackend, LocalVar};
use crate::ast::{ClassMember, Expr, Literal, Stmt, Type};
use inkwell::AddressSpace;
use inkwell::module::Linkage;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicValueEnum, FunctionValue, PointerValue};

impl<'ctx> LlvmBackend<'ctx> {
    /// Container methods that mutate the container and return the updated
    /// value, requiring the codegen to store the result back into the receiver.
    /// True for container types with VM reference semantics (shared when
    /// passed to functions or aliased by `let`): `ArrayList<T>`, `HashMap<K,V>`
    /// and builtin `array<T>`. These lower to an inline `{i64, ptr}` struct;
    /// sharing the caller's slot keeps mutations visible across calls, while
    /// whole-value rebinding detaches to a private copy. Explicit `&T`/`&mut T`
    /// references are already pointers and keep the normal path.
    pub(super) fn is_shared_container_ty(ty: &Type) -> bool {
        match ty {
            Type::Ref(_) | Type::MutRef(_) => false,
            Type::Named { name, .. } => {
                name == "ArrayList" || name == "HashMap" || name == "array"
            }
            Type::Tuple(_) => false,
        }
    }

    /// LLVM type of a user-function parameter: containers pass as an opaque
    /// pointer to the caller's `{i64, ptr}` slot (reference semantics), and
    /// interfaces pass as the `{i8*, i8*}` fat pointer (object + vtable).
    pub(super) fn user_param_llvm_type(
        &self,
        ty: &Type,
    ) -> Result<BasicTypeEnum<'ctx>, String> {
        if Self::is_shared_container_ty(ty) {
            Ok(self.context.ptr_type(AddressSpace::default()).into())
        } else if self.is_interface_ty(ty) {
            Ok(build_interface_fat_ptr_type(self.context).into())
        } else {
            llvm_types::llvm_type(self.context, ty)
        }
    }

    /// LLVM return type of a user function: interfaces return fat pointers
    /// like parameters do. Containers still return the `{i64, ptr}` struct by
    /// value (callers snapshot it; see passthrough transparency).
    pub(super) fn sig_ret_llvm_type(
        &self,
        ty: Option<&Type>,
    ) -> Result<Option<BasicTypeEnum<'ctx>>, String> {
        match ty {
            None => Ok(None),
            Some(t) if llvm_types::is_void(t) => Ok(None),
            Some(t) if self.is_interface_ty(t) => {
                Ok(Some(build_interface_fat_ptr_type(self.context).into()))
            }
            Some(t) => llvm_types::llvm_type(self.context, t).map(Some),
        }
    }

    /// True when `ty` names a declared interface.
    pub(super) fn is_interface_ty(&self, ty: &Type) -> bool {
        match ty {
            Type::Named { name, .. } => self.interface_infos.contains_key(name.as_str()),
            Type::Ref(inner) | Type::MutRef(inner) => self.is_interface_ty(inner),
            Type::Tuple(_) => false,
        }
    }

    /// Wrap a class-instance pointer as an interface fat pointer, using the
    /// vtable for the (interface, class) pair. Class method bodies compile
    /// before the interface-vtable pass, so build a concrete class's pair on
    /// demand rather than emitting a null fat pointer that would crash on the
    /// first interface call.
    pub(super) fn wrap_object_ptr_in_interface(
        &mut self,
        obj_ptr: PointerValue<'ctx>,
        iface: &str,
        class_name: &str,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let fat_ty = self
            .interface_infos
            .get(iface)
            .map(|info| info.fat_ptr_type)
            .ok_or_else(|| format!("codegen: unknown interface '{}'", iface))?;
        let key = (iface.to_string(), class_name.to_string());
        let concrete = self
            .class_decls
            .get(class_name)
            .is_some_and(|d| d.type_params.is_empty());
        if concrete && !self.interface_vtables.contains_key(&key) {
            let decl = self
                .class_decls
                .get(class_name)
                .cloned()
                .ok_or_else(|| format!("codegen: unknown class '{}'", class_name))?;
            let iface_decls = self.interface_decls.clone();
            // Building default clones moves the builder; restore our position.
            let saved_block = self.builder.get_insert_block();
            let built = self.build_interface_vtables_for_class(&decl, &iface_decls);
            if let Some(block) = saved_block {
                self.builder.position_at_end(block);
            }
            built?;
        }
        let vt = match self.interface_vtables.get(&key).copied() {
            Some(vt) => vt,
            None if concrete => {
                return Err(format!(
                    "codegen: class '{}' has no vtable for interface '{}'",
                    class_name, iface
                ));
            }
            None => {
                // Only an uninstantiated generic class lacks a real vtable;
                // its body is never executed. Keep the IR well-formed.
                return Ok(fat_ty.const_zero().into());
            }
        };
        emit_interface_fat_ptr(
            self.context,
            &self.builder,
            fat_ty,
            obj_ptr,
            vt.as_pointer_value(),
        )
    }

    /// Compile-only interface vtable for shared default bodies. These are
    /// emitted before any implementing class exists and are never installed in
    /// a vtable (per-class clones replace them), so null entries are enough to
    /// keep the indirect call well-formed.
    fn placeholder_interface_vtable(
        &mut self,
        iface: &str,
        method_names: &[String],
    ) -> Result<PointerValue<'ctx>, String> {
        let name = format!("_iface_default_vt_{}", iface);
        let i8_ptr = self.context.ptr_type(AddressSpace::default());
        let len = method_names.len().max(1) as u32;
        let arr_ty = i8_ptr.array_type(len);
        let global = match self.module.get_global(name.as_str()) {
            Some(g) => g,
            None => {
                let g = self.module.add_global(arr_ty, None, &name);
                g.set_linkage(Linkage::Internal);
                g.set_constant(true);
                let entries: Vec<PointerValue<'ctx>> =
                    (0..len).map(|_| i8_ptr.const_null()).collect();
                g.set_initializer(&i8_ptr.const_array(&entries));
                g
            }
        };
        Ok(global.as_pointer_value())
    }

    /// Wrap the raw `this` pointer of an interface default-method body as an
    /// interface fat pointer. A class clone uses its real interface vtable;
    /// the shared body (compiled before classes exist) uses the placeholder.
    pub(super) fn wrap_interface_this_ptr(
        &mut self,
        iface: &str,
        method_names: &[String],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let this_ptr = self.current_this.ok_or_else(|| {
            format!("codegen: 'this' used outside a method in interface '{}'", iface)
        })?;
        let class_name = self.current_class_name.clone().unwrap_or_else(|| iface.to_string());
        let key = (iface.to_string(), class_name.clone());
        if self.interface_vtables.contains_key(&key) {
            return self.wrap_object_ptr_in_interface(this_ptr, iface, &class_name);
        }
        let vt_ptr = self.placeholder_interface_vtable(iface, method_names)?;
        emit_interface_fat_ptr(
            self.context,
            &self.builder,
            build_interface_fat_ptr_type(self.context),
            this_ptr,
            vt_ptr,
        )
    }

    /// Compile an argument expected to have interface type: pass through
    /// values already of that interface, wrap class instances as fat
    /// pointers, and reject anything else with an honest error.
    pub(super) fn wrap_interface_arg(
        &mut self,
        arg: &Expr,
        iface: &str,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let arg_ty = self.infer_expr_type(arg);
        if arg_ty.name() == iface {
            return self.compile_expr(arg);
        }
        let obj_val = self.compile_expr(arg)?;
        if !obj_val.is_pointer_value() {
            return Err(format!(
                "codegen: cannot pass non-object of type '{}' as interface '{}'",
                arg_ty.name(),
                iface
            ));
        }
        self.wrap_object_ptr_in_interface(obj_val.into_pointer_value(), iface, arg_ty.name())
    }

    /// Bind one function/method parameter. Container parameters alias the
    /// caller's slot (the incoming value is already a slot pointer);
    /// interface parameters arrive as fat-pointer structs; all other
    /// parameters copy into a fresh alloca as before.
    pub(super) fn bind_param(
        &mut self,
        name: &str,
        ty: &Type,
        val: BasicValueEnum<'ctx>,
    ) -> Result<(), String> {
        if Self::is_shared_container_ty(ty) {
            let ptr = if val.is_pointer_value() {
                val.into_pointer_value()
            } else {
                // A container may arrive by value (e.g. a string struct
                // mis-typed by inference). Materialize a slot for it.
                let slot = self
                    .builder
                    .build_alloca(val.get_type(), &format!("{}.slot", name))
                    .map_err(|e| format!("build_alloca container slot failed: {:?}", e))?;
                self.builder
                    .build_store(slot, val)
                    .map_err(|e| format!("build_store container slot failed: {:?}", e))?;
                slot
            };
            let struct_ty: BasicTypeEnum<'ctx> =
                native_bridge::titrate_array_type(self.context).into();
            self.locals.insert(
                name.to_string(),
                LocalVar {
                    ptr,
                    ty: struct_ty,
                    full_type: Some(ty.clone()),
                    titrate_type: Some(ty.name().to_string()),
                    shared: true,
                    closure_sig: None,
                },
            );
            return Ok(());
        }
        let llvm_ty = self.user_param_llvm_type(ty)?;
        let alloca = self
            .builder
            .build_alloca(llvm_ty, name)
            .map_err(|e| format!("build_alloca param '{}' failed: {:?}", name, e))?;
        // For interface params the incoming value is already the fat struct;
        // for everything else it is the plain value. Either way the alloca
        // type matches user_param_llvm_type, so a direct store is correct.
        // (A raw object pointer here means a caller bypassed argument
        // coercion; storing it would corrupt the slot, so fail loudly.)
        if self.is_interface_ty(ty) && !val.is_struct_value() {
            return Err(format!(
                "codegen: interface param '{}' received a non-fat-pointer value",
                name
            ));
        }
        self.builder
            .build_store(alloca, val)
            .map_err(|e| format!("build_store param '{}' failed: {:?}", name, e))?;
        self.locals.insert(
            name.to_string(),
            LocalVar {
                ptr: alloca,
                ty: llvm_ty,
                full_type: Some(ty.clone()),
                titrate_type: Some(ty.name().to_string()),
                shared: false,
                closure_sig: None,
            },
        );
        Ok(())
    }

    /// Address of the slot holding a container argument. Calls always
    /// execute (for their side effects); a transparent result then shares
    /// its argument's pure slot, anything else snapshots. Every expression
    /// is compiled at most once.
    pub(super) fn container_slot_ptr(
        &mut self,
        arg: &Expr,
    ) -> Result<PointerValue<'ctx>, String> {
        let struct_ty = native_bridge::titrate_array_type(self.context);
        // Compile calls up front: effects must run whether the result is
        // shared or snapshotted.
        let compiled = if let Expr::Call(..) = arg {
            let v = self.compile_expr(arg)?;
            Some(self.cast_value_to_type(v, struct_ty.into())?)
        } else {
            None
        };
        if let Some(slot) = self.pure_slot_ptr(arg)? {
            return Ok(slot);
        }
        if let Expr::MemberAccess(obj, field, _) = arg {
            if let Some(gep) = self.container_field_slot(obj, field)? {
                return Ok(gep);
            }
        }
        let v = match compiled {
            Some(v) => v,
            None => {
                let v = self.compile_expr(arg)?;
                self.cast_value_to_type(v, struct_ty.into())?
            }
        };
        let tmp = self
            .builder
            .build_alloca(struct_ty, "container.arg.tmp")
            .map_err(|e| format!("build_alloca container.arg.tmp failed: {:?}", e))?;
        self.builder
            .build_store(tmp, v)
            .map_err(|e| format!("build_store container.arg.tmp failed: {:?}", e))?;
        Ok(tmp)
    }

    /// Slot for a container expression resolvable without side effects:
    /// aliased locals, `this` fields (GEP only), and transparent call chains
    /// bottoming out at such slots. Anything else yields `None` (the caller
    /// compiles it normally).
    pub(super) fn pure_slot_ptr(
        &mut self,
        arg: &Expr,
    ) -> Result<Option<PointerValue<'ctx>>, String> {
        let struct_ty = native_bridge::titrate_array_type(self.context);
        match arg {
            Expr::Identifier(name, _) => Ok(self
                .locals
                .get(name.as_str())
                .filter(|local| local.ty == BasicTypeEnum::StructType(struct_ty))
                .map(|local| local.ptr)),
            Expr::MemberAccess(obj, field, _) if matches!(obj.as_ref(), Expr::This(_)) => {
                self.container_field_slot(obj, field)
            }
            Expr::Call(callee, call_args, _, _) => {
                let mut visited = std::collections::HashSet::new();
                match self.transparent_param_index(callee, call_args, &mut visited) {
                    Some(idx) => match call_args.get(idx) {
                        Some(inner) => self.pure_slot_ptr(inner),
                        None => Ok(None),
                    },
                    None => Ok(None),
                }
            }
            _ => Ok(None),
        }
    }

    /// Index of the parameter a function/method body provably returns:
    /// the last statement must be `return <expr>` aliasing a parameter, and
    /// no other `return` may exist anywhere else in the body (it would take
    /// precedence on its path). Parameters are immutable, so preceding
    /// statements can only mutate through the shared slot, never reseat it.
    pub(super) fn return_alias_param(
        &self,
        body: &[Stmt],
        params: &[(String, Type)],
        visited: &mut std::collections::HashSet<String>,
    ) -> Option<usize> {
        let (last, rest) = body.split_last()?;
        let ret_expr = match last {
            Stmt::Return(Some(e)) => e,
            _ => return None,
        };
        if rest.iter().any(stmt_contains_return) {
            return None;
        }
        let idx = self.expr_alias_param(ret_expr, params, visited)?;
        // A body-local binding shadowing the parameter name would make the
        // returned name resolve to the wrong slot.
        if body_binds_name(body, &params[idx].0) {
            return None;
        }
        Some(idx)
    }

    /// Index of `params` that a return-position expression provably aliases:
    /// a parameter itself, or a transparent call whose aliased argument
    /// recursively resolves to one.
    pub(super) fn expr_alias_param(
        &self,
        expr: &Expr,
        params: &[(String, Type)],
        visited: &mut std::collections::HashSet<String>,
    ) -> Option<usize> {
        match expr {
            Expr::Identifier(name, _) => params.iter().position(|(n, _)| n == name),
            Expr::Call(callee, call_args, _, _) => {
                let j = self.transparent_param_index(callee, call_args, visited)?;
                self.expr_alias_param(call_args.get(j)?, params, visited)
            }
            _ => None,
        }
    }

    /// Index of the call argument whose live slot a call's result provably
    /// aliases (`None` when unprovable — the caller snapshots instead):
    /// - same-scope closures whose body passes a parameter through;
    /// - top-level functions with passthrough bodies (transitively);
    /// - direct, un-overridden methods with passthrough bodies (interface
    ///   receivers are vetoed: implementations vary).
    ///
    /// Walkers below are exhaustive over `Stmt`/`Expr` (no wildcards), so
    /// future AST additions fail compilation here instead of silently
    /// weakening the analysis.
    pub(super) fn transparent_param_index(
        &self,
        callee: &Expr,
        _call_args: &[Expr],
        visited: &mut std::collections::HashSet<String>,
    ) -> Option<usize> {
        match callee {
            Expr::Identifier(name, _) => {
                // Same-scope closure passthrough (leaf: no deeper analysis).
                if let Some(local) = self.locals.get(name.as_str()) {
                    if let Some(sig) = local.closure_sig.as_ref() {
                        return sig.returns_param;
                    }
                }
                // Top-level function passthrough (possibly transitive).
                if !visited.insert(format!("fn:{}", name)) {
                    return None;
                }
                let decl = self.function_decls.get(name.as_str())?;
                let param_names: Vec<(String, Type)> = decl
                    .params
                    .iter()
                    .map(|p| (p.name.clone(), p.typ.clone()))
                    .collect();
                self.return_alias_param(&decl.body, &param_names, visited)
            }
            Expr::MemberAccess(obj, method, _) => {
                let obj_ty = self.infer_expr_type(obj);
                if self.is_interface_ty(&obj_ty) {
                    return None;
                }
                let class_name = obj_ty.name().to_string();
                let (owner, body, params) = self.find_method_body(&class_name, method)?;
                if self.method_overridden(&owner, method) {
                    return None;
                }
                if !visited.insert(format!("m:{}:{}", owner, method)) {
                    return None;
                }
                self.return_alias_param(&body, &params, visited)
            }
            _ => None,
        }
    }

    /// Locate a method's (owner class, body, params), walking the parent
    /// chain like virtual dispatch. Returns `None` when unresolvable.
    pub(super) fn find_method_body(
        &self,
        class_name: &str,
        method: &str,
    ) -> Option<MethodBody> {
        let mut current = Some(class_name.to_string());
        let mut visited = std::collections::HashSet::new();
        while let Some(name) = current {
            if !visited.insert(name.clone()) {
                break;
            }
            if let Some(decl) = self.class_decls.get(&name) {
                for member in &decl.members {
                    let m = match member {
                        ClassMember::Method(m) => m,
                        ClassMember::Constructor(m) => m,
                        _ => continue,
                    };
                    if m.name == method
                        || (method == "init" && matches!(member, ClassMember::Constructor(_)))
                    {
                        return Some((
                            name.clone(),
                            m.body.clone(),
                            m.params
                                .iter()
                                .map(|p| (p.name.clone(), p.typ.clone()))
                                .collect(),
                        ));
                    }
                }
                current = decl.parent.as_ref().map(|t| t.name().to_string());
            } else {
                break;
            }
        }
        None
    }

    /// True when `child` is a strict subclass of `ancestor`, walking the
    /// recorded parent chain with a cycle guard.
    pub(super) fn is_subclass_of(&self, child: &str, ancestor: &str) -> bool {
        let mut current = self
            .class_infos
            .get(child)
            .and_then(|info| info.parent.clone());
        let mut guard = 0usize;
        while let Some(name) = current {
            if name == ancestor {
                return true;
            }
            guard += 1;
            if guard > 64 {
                break;
            }
            current = self
                .class_infos
                .get(&name)
                .and_then(|info| info.parent.clone());
        }
        false
    }

    /// True when some class other than `owner` (a strict subclass of it, or
    /// an unrelated class that could still flow in — conservatively, any
    /// other class) defines `method`. Conservative: any other definition
    /// anywhere vetoes transparency, since dynamic dispatch could run it.
    pub(super) fn method_overridden(&self, owner: &str, method: &str) -> bool {
        for (class_name, decl) in &self.class_decls {
            if class_name == owner {
                continue;
            }
            for member in &decl.members {
                match member {
                    ClassMember::Method(m) | ClassMember::Constructor(m)
                        if m.name == method =>
                    {
                        return true;
                    }
                    _ => {}
                }
            }
        }
        false
    }

    /// Address of a container-typed field slot, or `None` when the receiver
    /// is not a resolvable object slot (caller falls back to a snapshot).
    pub(super) fn container_field_slot(
        &mut self,
        obj: &Expr,
        field: &str,
    ) -> Result<Option<PointerValue<'ctx>>, String> {
        let (base_ptr, class_name) = if let Expr::This(_) = obj {
            match (self.current_this, &self.current_class_name) {
                (Some(this_ptr), Some(class_name)) => (this_ptr, class_name.clone()),
                _ => return Ok(None),
            }
        } else {
            let base_val = self.compile_expr(obj)?;
            if !base_val.is_pointer_value() {
                return Ok(None);
            }
            let base_ty = self.infer_expr_type(obj);
            (base_val.into_pointer_value(), base_ty.name().to_string())
        };
        let class_info = match self.class_infos.get(&class_name).cloned() {
            Some(info) => info,
            None => return Ok(None),
        };
        let field_ty = class_info
            .fields
            .iter()
            .find(|(name, _)| name == field)
            .map(|(_, ty)| *ty);
        let struct_ty: BasicTypeEnum<'ctx> =
            native_bridge::titrate_array_type(self.context).into();
        if field_ty != Some(struct_ty) {
            return Ok(None);
        }
        emit_field_slot_ptr(
            self.context,
            &self.builder,
            &class_info,
            base_ptr,
            field,
        )
        .map(Option::Some)
    }

    /// Coerce one argument for a user-function call. When the callee's LLVM
    /// parameter is a pointer and the argument is a shared container, pass
    /// the caller's slot (reference semantics). Otherwise keep the existing
    /// by-value plus cast behavior. `param_llvm` is `None` when the callee
    /// signature is unknown.
    /// Resolve a user function by name and call arity, so overloads with
    /// different parameter counts reach their own LLVM functions.
    pub(super) fn resolve_user_fn(
        &self,
        name: &str,
        argc: usize,
    ) -> Option<inkwell::values::FunctionValue<'ctx>> {
        self.fn_overloads
            .get(&(name.to_string(), argc))
            .copied()
            .or_else(|| {
                // A bare name can collide with a mangled class method whose
                // arity differs (e.g. native `File_seek` vs `File.seek`).
                // Never resolve a function with the wrong arity.
                self.functions
                    .get(name)
                    .copied()
                    .filter(|f| f.get_type().get_param_types().len() == argc)
            })
    }

    /// Declared parameter types of a specific overload (by call arity).
    pub(super) fn fn_decl_param_types_arity(
        &self,
        name: &str,
        argc: usize,
    ) -> Option<Vec<Type>> {
        self.fn_decl_overloads
            .get(&(name.to_string(), argc))
            .or_else(|| self.function_decls.get(name))
            .map(|d| d.params.iter().map(|p| p.typ.clone()).collect())
    }

    /// Type-name key for an overload, used to separate functions that share a
    /// name and arity but differ in parameter types (`Integer.max` vs
    /// `Math.max`).
    pub(super) fn fn_type_key(params: &[Type]) -> Vec<String> {
        params.iter().map(|p| p.name().to_string()).collect()
    }

    /// Resolve a function whose declared parameter types match the given
    /// inferred argument types exactly.
    pub(super) fn resolve_user_fn_typed(
        &self,
        name: &str,
        arg_types: &[Type],
    ) -> Option<inkwell::values::FunctionValue<'ctx>> {
        // While dispatching `Module.fn(...)`, prefer the function declared in
        // that module. Otherwise same-signature overloads from other modules
        // (e.g. Json.load vs DataFile.load) can win by insertion order.
        if let Some(module) = &self.call_module_hint {
            let key = (
                module.clone(),
                name.to_string(),
                Self::fn_type_key(arg_types),
            );
            if let Some(f) = self.module_fn_typed.get(&key) {
                return Some(*f);
            }
        }
        let key = Self::fn_type_key(arg_types);
        self.fn_typed.get(&(name.to_string(), key)).copied()
    }

    /// Declared return type of a module-qualified user function, recovered by
    /// mapping the resolved LLVM symbol back to its source declaration.
    pub(super) fn module_fn_return_type(
        &self,
        module: &str,
        name: &str,
        arg_types: &[Type],
    ) -> Option<Type> {
        let key = (
            module.to_string(),
            name.to_string(),
            Self::fn_type_key(arg_types),
        );
        let fv = self.module_fn_typed.get(&key)?;
        let sym = fv.get_name().to_str().ok()?;
        self.fn_decl_symbols
            .iter()
            .find(|(_, s)| s == sym)
            .and_then(|(d, _)| d.return_type.clone())
    }

    /// Declared parameter types for an exact type match.
    pub(super) fn fn_decl_param_types_typed(
        &self,
        name: &str,
        arg_types: &[Type],
    ) -> Option<Vec<Type>> {
        let key = Self::fn_type_key(arg_types);
        self.fn_decl_typed
            .get(&(name.to_string(), key))
            .map(|d| d.params.iter().map(|p| p.typ.clone()).collect())
    }

    /// Box a native-call argument whose declared Titrate type is `Variant`
    /// (a two-word `{i64, ptr}` slot) but whose compiled LLVM value is still a
    /// scalar. `as Variant` casts and erased generics can leave the scalar
    /// unwrapped; native marshalling and argument validation both require the
    /// declared shape.
    pub(super) fn coerce_native_arg_value(
        &self,
        val: BasicValueEnum<'ctx>,
        ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if ty.name() != "Variant" || !(val.is_int_value() || val.is_float_value()) {
            return Ok(val);
        }
        let target = llvm_types::llvm_type(self.context, ty)?;
        self.cast_value_to_type(val, target)
    }

    pub(super) fn coerce_user_call_arg(
        &mut self,
        arg: &Expr,
        param_ty: Option<&Type>,
        param_llvm: Option<BasicTypeEnum<'ctx>>,
    ) -> Result<inkwell::values::BasicMetadataValueEnum<'ctx>, String> {
        let arg_is_container = Self::is_shared_container_ty(&self.infer_expr_type(arg));
        if let Some(ty) = param_ty {
            // Declared container params (and `&Container` refs, which alias by
            // nature) share the caller's slot; interface params wrap class
            // instances as fat pointers; anything else keeps by-value.
            let by_ref = Self::is_shared_container_ty(ty)
                || matches!(ty, Type::Ref(inner) | Type::MutRef(inner)
                    if Self::is_shared_container_ty(inner));
            if by_ref && arg_is_container {
                return Ok(self.container_slot_ptr(arg)?.into());
            }
            if by_ref && matches!(arg, Expr::Literal(Literal::Null, _)) {
                // A null literal passed to a by-reference container parameter
                // must decay to a slot holding a zeroed container, never a raw
                // null pointer: the callee loads `{ i64, ptr }` from that slot
                // and dereferencing address zero faults.
                let llvm_ty = self.user_param_llvm_type(ty)?;
                let slot = self
                    .builder
                    .build_alloca(llvm_ty, "null.container.slot")
                    .map_err(|e| format!("build_alloca null container slot failed: {:?}", e))?;
                let zero: inkwell::values::BasicValueEnum<'ctx> = match llvm_ty {
                    BasicTypeEnum::StructType(st) => st.const_zero().into(),
                    BasicTypeEnum::IntType(it) => it.const_zero().into(),
                    BasicTypeEnum::PointerType(pt) => pt.const_null().into(),
                    _ => self.context.i32_type().const_zero().into(),
                };
                self.builder
                    .build_store(slot, zero)
                    .map_err(|e| format!("build_store null container slot failed: {:?}", e))?;
                return Ok(slot.into());
            }
            if self.is_interface_ty(ty) {
                return Ok(self.wrap_interface_arg(arg, ty.name())?.into());
            }
            let v = self.compile_expr(arg)?;
            let llvm_ty = self.user_param_llvm_type(ty)?;
            let v = if v.get_type() != llvm_ty {
                self.cast_value_to_type(v, llvm_ty)?
            } else {
                v
            };
            return Ok(v.into());
        }
        // Unknown declaration: keep the legacy LLVM-signature behavior.
        if let Some(pty) = param_llvm {
            if pty.is_pointer_type() && arg_is_container {
                return Ok(self.container_slot_ptr(arg)?.into());
            }
            let v = self.compile_expr(arg)?;
            let v = if v.get_type() != pty {
                self.cast_value_to_type(v, pty)?
            } else {
                v
            };
            return Ok(v.into());
        }
        Ok(self.compile_expr(arg)?.into())
    }

    /// Invoke a closure value held by a local through its recorded signature.
    /// Container arguments share the caller's slot (the trampoline binds them
    /// by reference, like user functions). Arity mismatches are honest
    /// errors: the indirect call type must match the trampoline exactly.
    pub(super) fn compile_closure_call(
        &mut self,
        local: &LocalVar<'ctx>,
        sig: &ClosureSig,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() != sig.params.len() {
            return Err(format!(
                "codegen: closure expects {} argument(s), got {}",
                sig.params.len(),
                args.len()
            ));
        }
        if !local.ty.is_struct_type() {
            return Err("codegen: closure local does not hold a closure struct".to_string());
        }
        let sv = self
            .builder
            .build_load(local.ty, local.ptr, "closure.val")
            .map_err(|e| format!("build_load closure failed: {:?}", e))?
            .into_struct_value();
        let fn_i8 = self
            .builder
            .build_extract_value(sv, 0, "closure.fn")
            .map_err(|e| format!("extract closure.fn failed: {:?}", e))?
            .into_pointer_value();
        let capture = self
            .builder
            .build_extract_value(sv, 1, "closure.capture")
            .map_err(|e| format!("extract closure.capture failed: {:?}", e))?
            .into_pointer_value();
        let i8_ptr = self.context.ptr_type(AddressSpace::default());
        let mut param_tys: Vec<inkwell::types::BasicMetadataTypeEnum> = Vec::new();
        param_tys.push(i8_ptr.into());
        for p in &sig.params {
            param_tys.push(self.user_param_llvm_type(p)?.into());
        }
        let ret_ty = self.sig_ret_llvm_type(Some(&sig.ret))?;
        let fn_ty = match ret_ty {
            Some(ret) => ret.fn_type(&param_tys, false),
            None => self.context.void_type().fn_type(&param_tys, false),
        };
        let callee = self
            .builder
            .build_bit_cast(fn_i8, self.context.ptr_type(AddressSpace::default()), "closure.callee")
            .map_err(|e| format!("bit_cast closure.callee failed: {:?}", e))?
            .into_pointer_value();
        let mut call_args: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> = Vec::new();
        call_args.push(capture.into());
        for (i, arg) in args.iter().enumerate() {
            call_args.push(self.coerce_user_call_arg(arg, sig.params.get(i), None)?);
        }
        let call = self
            .builder
            .build_indirect_call(fn_ty, callee, &call_args, "closure.call")
            .map_err(|e| format!("build_indirect_call closure failed: {:?}", e))?;
        if ret_ty.is_some() {
            match call.try_as_basic_value() {
                inkwell::values::ValueKind::Basic(v) => Ok(v),
                _ => Err("closure did not return a value".to_string()),
            }
        } else {
            Ok(self.context.i32_type().const_int(0, false).into())
        }
    }

    /// Declared parameter types of `class_name.method` (walking the parent
    /// chain like virtual dispatch), or `None` when unresolvable. Used to
    /// shape calls whose callee LLVM signature alone cannot distinguish
    /// container slots from plain values or objects from fat pointers.
    pub(super) fn method_decl_params(&self, class_name: &str, method: &str) -> Option<Vec<Type>> {
        let mut current = Some(class_name.to_string());
        let mut visited = std::collections::HashSet::new();
        while let Some(name) = current {
            if !visited.insert(name.clone()) {
                break;
            }
            if let Some(decl) = self.class_decls.get(&name) {
                for member in &decl.members {
                    let m = match member {
                        ClassMember::Method(m) => m,
                        ClassMember::Constructor(m) => m,
                        _ => continue,
                    };
                    if m.name == method || (method == "init" && matches!(member, ClassMember::Constructor(_))) {
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
            } else {
                break;
            }
        }
        None
    }

    /// Build arguments for a method/constructor call whose first LLVM
    /// parameter is `this`. With declared Titrate parameter types, container
    /// arguments share the caller's slot and interface arguments wrap as fat
    /// pointers; without them, keep the legacy LLVM-pointer heuristic. Either
    /// way every argument is reshaped to the callee's exact declared LLVM
    /// parameter type so the emitted call cannot trip LLVM's bad-signature
    /// assertion (e.g. int widths, or `{i64, ptr}` vs `{ptr, ptr}`).
    pub(super) fn build_this_call_args(
        &mut self,
        callee: FunctionValue<'ctx>,
        params: Option<&[Type]>,
        args: &[Expr],
    ) -> Result<Vec<BasicValueEnum<'ctx>>, String> {
        let llvm_params = callee.get_type().get_param_types();
        let mut arg_vals = Vec::with_capacity(args.len());
        for (i, arg) in args.iter().enumerate() {
            // Callee's declared LLVM type for this argument (param 0 is `this`).
            let declared_llvm = llvm_params
                .get(i + 1)
                .copied()
                .and_then(basic_metadata_to_basic_type);
            let v = if let Some(ty) = params.and_then(|p| p.get(i)) {
                let coerced = self.coerce_user_call_arg(arg, Some(ty), None)?;
                Self::meta_to_basic(coerced)?
            } else if declared_llvm.is_some_and(|t| t.is_pointer_type())
                && Self::is_shared_container_ty(&self.infer_expr_type(arg))
            {
                self.container_slot_ptr(arg)?.into()
            } else {
                self.compile_expr(arg)?
            };
            // Final reshape to the exact declared type; a no-op when `v`
            // already has it. This also covers the `params = None` path.
            let shaped = match declared_llvm {
                Some(ty) => coerce_call_arg(self.context, &self.builder, v, ty)?,
                None => v,
            };
            arg_vals.push(shaped);
        }
        Ok(arg_vals)
    }

    /// Convert a coerced call argument back to a plain value for callees
    /// taking `BasicValueEnum` (direct/virtual/interface dispatch).
    pub(super) fn meta_to_basic(
        v: inkwell::values::BasicMetadataValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        match v {
            inkwell::values::BasicMetadataValueEnum::ArrayValue(v) => Ok(v.into()),
            inkwell::values::BasicMetadataValueEnum::IntValue(v) => Ok(v.into()),
            inkwell::values::BasicMetadataValueEnum::FloatValue(v) => Ok(v.into()),
            inkwell::values::BasicMetadataValueEnum::PointerValue(v) => Ok(v.into()),
            inkwell::values::BasicMetadataValueEnum::StructValue(v) => Ok(v.into()),
            inkwell::values::BasicMetadataValueEnum::VectorValue(v) => Ok(v.into()),
            inkwell::values::BasicMetadataValueEnum::ScalableVectorValue(_) => {
                Err("codegen: scalable vector cannot be a call argument".to_string())
            }
            inkwell::values::BasicMetadataValueEnum::MetadataValue(_) => {
                Err("codegen: metadata cannot be a call argument".to_string())
            }
        }
    }

    pub(super) fn is_container_mutator(native_name: &str) -> bool {
        matches!(
            native_name,
            "ArrayList_add" | "ArrayList_set" | "ArrayList_remove" | "ArrayList_removeAt"
                | "ArrayList_clear" | "ArrayList_pop" | "ArrayList_addAll"
                | "ArrayList_removeAll" | "ArrayList_retainAll"
                | "HashMap_put" | "HashMap_remove" | "HashMap_clear"
                | "HashMap_putIfAbsent" | "HashMap_replace" | "HashMap_merge"
        )
    }

    /// Store the updated container back into a receiver location so mutations
    /// persist across the value-copy native bridge. Supports local variables
    /// and `this.field` / `obj.field` receivers.
    pub(super) fn store_back_container(
        &mut self,
        receiver: &Expr,
        result: BasicValueEnum<'ctx>,
    ) -> Result<(), String> {
        match receiver {
            Expr::Identifier(name, _) => {
                if let Some(local) = self.locals.get(name.as_str()) {
                    let local_ptr = local.ptr;
                    self.builder
                        .build_store(local_ptr, result)
                        .map_err(|e| {
                            format!("codegen: store back container local '{}' failed: {:?}", name, e)
                        })?;
                    return Ok(());
                }
                // Module-level containers (e.g. DataFile's `_cache`) are
                // identifiers too; mutators such as `put` return a new handle
                // that must be written back to the global slot.
                if let Some(g) = self.globals.get(name.as_str()) {
                    let global_ptr = g.ptr;
                    self.builder
                        .build_store(global_ptr, result)
                        .map_err(|e| {
                            format!("codegen: store back container global '{}' failed: {:?}", name, e)
                        })?;
                }
                Ok(())
            }
            Expr::MemberAccess(obj, field, _) => {
                let (base_ptr, class_name) = if let Expr::This(_) = &**obj {
                    match (self.current_this, &self.current_class_name) {
                        (Some(this_ptr), Some(class_name)) => (this_ptr, class_name.clone()),
                        _ => return Ok(()),
                    }
                } else {
                    let base_val = self.compile_expr(obj)?;
                    if !base_val.is_pointer_value() {
                        return Ok(());
                    }
                    let base_ty = self.infer_expr_type(obj);
                    (base_val.into_pointer_value(), base_ty.name().to_string())
                };
                if let Some(class_info) = self.class_infos.get(&class_name).cloned() {
                    return emit_field_store(
                        self.context,
                        &self.builder,
                        &class_info,
                        base_ptr,
                        field,
                        result,
                    )
                    .map_err(|e| {
                        format!("codegen: store back container field '{}.{}' failed: {:?}", class_name, field, e)
                    });
                }
                Ok(())
            }
            // A mutator applied to `container.get(i)`: the native bridge
            // returns a new container by value, so write it back into the
            // source array's TitrateValue payload slot.
            Expr::Call(callee, call_args, _, _) => {
                if let Expr::MemberAccess(_, method, _) = &**callee {
                    if method == "get" && call_args.len() == 1 {
                        return self.store_back_array_element(receiver, &call_args[0], result);
                    }
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Write `result` into the payload slot of `container.get(index)`.
    fn store_back_array_element(
        &mut self,
        receiver: &Expr,
        index: &Expr,
        result: BasicValueEnum<'ctx>,
    ) -> Result<(), String> {
        let Expr::Call(callee, _, _, _) = receiver else {
            return Ok(());
        };
        let Expr::MemberAccess(obj, _, _) = &**callee else {
            return Ok(());
        };
        // A HashMap value is returned by copy; write the updated value back
        // with `HashMap_put(map, key, value)`.
        if self.infer_expr_type(obj).name() == "HashMap" {
            let map_val = self.compile_expr(obj)?;
            let key_val = self.compile_expr(index)?;
            let map_ty = self.infer_expr_type(obj);
            let key_ty = self.infer_expr_type(index);
            let value_ty = map_ty
                .params()
                .get(1)
                .cloned()
                .unwrap_or_else(|| Type::simple("array"));
            let updated = self.compile_native_call(
                "HashMap_put",
                &[map_val, key_val, result],
                &[map_ty, key_ty, value_ty],
            )?;
            // HashMap_put returns a new map (value-copy bridge); store it back
            // into the receiver slot so subsequent gets see the mutation.
            return self.store_back_container(obj, updated);
        }
        let arr_val = self.compile_expr(obj)?;
        let sv = match arr_val {
            BasicValueEnum::StructValue(sv) => sv,
            _ => return Ok(()),
        };
        let data_ptr = match self.builder.build_extract_value(sv, 1, "writeback.data") {
            Ok(v) => v.into_pointer_value(),
            Err(_) => return Ok(()),
        };
        let idx_raw = self.compile_expr(index)?;
        if !idx_raw.is_int_value() {
            return Ok(());
        }
        let idx_val = idx_raw.into_int_value();
        let i64_ty = self.context.i64_type();
        let idx_i64 = if idx_val.get_type().get_bit_width() < 64 {
            match self.builder.build_int_z_extend(idx_val, i64_ty, "writeback.idx") {
                Ok(v) => v,
                Err(_) => return Ok(()),
            }
        } else {
            idx_val
        };
        let byte_ty = self.context.i8_type();
        let offset = match self.builder.build_int_mul(
            idx_i64,
            i64_ty.const_int(24, false),
            "writeback.offset",
        ) {
            Ok(v) => v,
            Err(_) => return Ok(()),
        };
        let elem = unsafe {
            self.builder
                .build_gep(byte_ty, data_ptr, &[offset], "writeback.elem")
        }
        .map_err(|e| format!("writeback elem failed: {:?}", e))?;
        let payload = unsafe {
            self.builder.build_gep(
                byte_ty,
                elem,
                &[i64_ty.const_int(8, false)],
                "writeback.payload",
            )
        }
        .map_err(|e| format!("writeback payload failed: {:?}", e))?;
        let ptr = self.context.ptr_type(inkwell::AddressSpace::default());
        let payload_typed = self
            .builder
            .build_bit_cast(payload, ptr, "writeback.cast")
            .map_err(|e| format!("writeback cast failed: {:?}", e))?
            .into_pointer_value();
        self.builder
            .build_store(payload_typed, result)
            .map_err(|e| format!("writeback store failed: {:?}", e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coerce_call_arg_reshapes_container_to_fat_pointer() {
        let context = inkwell::context::Context::create();
        let module = context.create_module("coerce_reshape");
        let builder = context.create_builder();
        let fn_ty = context.void_type().fn_type(&[], false);
        let f = module.add_function("coerce_harness", fn_ty, None);
        let entry = context.append_basic_block(f, "entry");
        builder.position_at_end(entry);

        let ptr_ty = context.ptr_type(AddressSpace::default());
        let container = context.struct_type(&[context.i64_type().into(), ptr_ty.into()], false);
        let fat = build_interface_fat_ptr_type(&context);
        let slot = builder.build_alloca(container, "arg").expect("alloca");
        let value = container.const_named_struct(&[
            context.i64_type().const_int(3, false).into(),
            ptr_ty.const_null().into(),
        ]);
        builder.build_store(slot, value).expect("store");
        let loaded = builder.build_load(container, slot, "arg.val").expect("load");

        let shaped = coerce_call_arg(&context, &builder, loaded, fat.into()).expect("coerce");
        assert!(shaped.is_struct_value());
        assert_eq!(shaped.into_struct_value().get_type(), fat);
    }

    #[test]
    fn variant_scalar_arg_is_boxed() {
        let context = inkwell::context::Context::create();
        let backend = LlvmBackend::new(&context, "variant_arg_box");
        let fn_ty = context.void_type().fn_type(&[], false);
        let f = backend.module.add_function("variant_box_host", fn_ty, None);
        let entry = context.append_basic_block(f, "entry");
        backend.builder.position_at_end(entry);

        let variant_ty = llvm_types::llvm_type(&context, &Type::simple("Variant")).expect("Variant type");
        assert!(variant_ty.is_struct_type());

        let int_arg = context.i32_type().const_int(42, false);
        let boxed = backend
            .coerce_native_arg_value(int_arg.into(), &Type::simple("Variant"))
            .expect("int Variant arg should box");
        assert_eq!(boxed.get_type(), variant_ty);

        let double_arg = context.f64_type().const_float(2.5);
        let boxed_double = backend
            .coerce_native_arg_value(double_arg.into(), &Type::simple("Variant"))
            .expect("double Variant arg should box");
        assert_eq!(boxed_double.get_type(), variant_ty);

        // A non-Variant scalar must pass through untouched.
        let plain = backend
            .coerce_native_arg_value(int_arg.into(), &Type::simple("int"))
            .expect("int arg should pass through");
        assert!(plain.is_int_value());

        backend.builder.build_return(None).expect("return");
        assert!(backend.module.verify().is_ok(), "invalid IR after Variant boxing");
    }

    #[test]
    fn wrap_interface_this_ptr_uses_one_placeholder_vtable() {
        let context = inkwell::context::Context::create();
        let mut backend = LlvmBackend::new(&context, "iface_this_wrap");
        let ptr_ty = context.ptr_type(AddressSpace::default());
        let f = backend.module.add_function(
            "this_host",
            context.void_type().fn_type(&[ptr_ty.into()], false),
            None,
        );
        let entry = context.append_basic_block(f, "entry");
        backend.builder.position_at_end(entry);
        backend.current_this = Some(f.get_nth_param(0).unwrap().into_pointer_value());
        backend.current_class_name = Some("Iterator".to_string());

        let names = vec!["hasNext".to_string(), "next".to_string()];
        let fat = backend
            .wrap_interface_this_ptr("Iterator", &names)
            .expect("wrap this");
        assert!(fat.is_struct_value());
        assert_eq!(fat.into_struct_value().get_type().count_fields(), 2);

        backend
            .wrap_interface_this_ptr("Iterator", &names)
            .expect("wrap this again");
        let placeholders = backend
            .module
            .get_globals()
            .filter(|g| {
                g.get_name()
                    .to_str()
                    .unwrap_or("")
                    .starts_with("_iface_default_vt_Iterator")
            })
            .count();
        assert_eq!(placeholders, 1);
    }
}
