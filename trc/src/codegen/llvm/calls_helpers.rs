// Call-argument shaping and dispatch helpers

use super::native_bridge;
use super::types as llvm_types;
use super::vtable::{build_interface_fat_ptr_type, emit_field_slot_ptr, emit_field_store, emit_interface_fat_ptr};
use super::{body_binds_name, stmt_contains_return, MethodBody};
use super::{ClosureSig, LlvmBackend, LocalVar};
use crate::ast::{ClassMember, Expr, Stmt, Type};
use inkwell::AddressSpace;
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
    /// prebuilt vtable for the (interface, class) pair.
    pub(super) fn wrap_object_ptr_in_interface(
        &self,
        obj_ptr: PointerValue<'ctx>,
        iface: &str,
        class_name: &str,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let fat_ty = self
            .interface_infos
            .get(iface)
            .map(|info| info.fat_ptr_type)
            .ok_or_else(|| format!("codegen: unknown interface '{}'", iface))?;
        let vt = self
            .interface_vtables
            .get(&(iface.to_string(), class_name.to_string()))
            .cloned()
            .ok_or_else(|| {
                format!(
                    "codegen: class '{}' does not implement interface '{}'",
                    class_name, iface
                )
            })?;
        emit_interface_fat_ptr(
            self.context,
            &self.builder,
            fat_ty,
            obj_ptr,
            vt.as_pointer_value(),
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
            let ptr = val.into_pointer_value();
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
            Expr::Call(callee, call_args, _) => {
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
            Expr::Call(callee, call_args, _) => {
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
    /// Declared parameter types of a top-level function, if registered.
    pub(super) fn fn_decl_param_types(&self, name: &str) -> Option<Vec<Type>> {
        self.function_decls
            .get(name)
            .map(|d| d.params.iter().map(|p| p.typ.clone()).collect())
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
                        return Some(m.params.iter().map(|p| p.typ.clone()).collect());
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
    /// pointers; without them, keep the legacy LLVM-pointer heuristic.
    pub(super) fn build_this_call_args(
        &mut self,
        callee: FunctionValue<'ctx>,
        params: Option<&[Type]>,
        args: &[Expr],
    ) -> Result<Vec<BasicValueEnum<'ctx>>, String> {
        let llvm_params = callee.get_type().get_param_types();
        let mut arg_vals = Vec::with_capacity(args.len());
        for (i, arg) in args.iter().enumerate() {
            if let Some(ty) = params.and_then(|p| p.get(i)) {
                let coerced = self.coerce_user_call_arg(arg, Some(ty), None)?;
                arg_vals.push(Self::meta_to_basic(coerced)?);
                continue;
            }
            let is_ptr = matches!(
                llvm_params.get(i + 1),
                Some(inkwell::types::BasicMetadataTypeEnum::PointerType(_))
            );
            if is_ptr && Self::is_shared_container_ty(&self.infer_expr_type(arg)) {
                arg_vals.push(self.container_slot_ptr(arg)?.into());
            } else {
                arg_vals.push(self.compile_expr(arg)?);
            }
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
            _ => Ok(()),
        }
    }
}
