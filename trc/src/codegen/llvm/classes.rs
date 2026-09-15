// Class and interface declaration lowering

use super::types as llvm_types;
use super::vtable::{build_class_struct_type, build_interface_fat_ptr_type, create_vtable_global, ClassInfo, InterfaceInfo};
use super::{IfaceMethodSig, LlvmBackend};
use crate::ast::{ClassDecl, ClassMember, InterfaceDecl, Stmt, Type};
use inkwell::AddressSpace;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicValueEnum, FunctionValue};
use std::collections::HashMap;

impl<'ctx> LlvmBackend<'ctx> {
    /// Build the LLVM struct type for a class declaration, recursively
    /// embedding parent fields. Used when the parent class is declared after
    /// the child and its ClassInfo is not yet available.
    pub(super) fn build_class_struct_type_for(&self, class_decl: &ClassDecl) -> Result<inkwell::types::StructType<'ctx>, String> {
        let mut field_types: HashMap<String, BasicTypeEnum<'ctx>> = HashMap::new();
        let mut field_decls: Vec<crate::ast::FieldDecl> = Vec::new();
        for member in &class_decl.members {
            if let ClassMember::Field(field) = member {
                let ft = llvm_types::llvm_type(self.context, &field.typ)?;
                field_types.insert(field.name.clone(), ft);
                field_decls.push(field.clone());
            }
        }
        let parent_struct_type = if let Some(parent) = &class_decl.parent {
            let parent_name = parent.name();
            if let Some(pi) = self.class_infos.get(parent_name) {
                Some(pi.struct_type)
            } else if let Some(pd) = self.class_decls.get(parent_name).cloned() {
                Some(self.build_class_struct_type_for(&pd)?)
            } else {
                None
            }
        } else {
            None
        };
        Ok(build_class_struct_type(
            self.context,
            &class_decl.name,
            &field_decls,
            &field_types,
            parent_struct_type,
        ))
    }

    /// Compile a class declaration: build struct type, compile methods, create vtable.
    pub(super) fn compile_class_decl(&mut self, class_decl: &ClassDecl) -> Result<(), String> {
        // A concrete class extending a generic instantiation (e.g.
        // `class Child extends Base<int>`) links to the specialization:
        // instantiate the parent first and rewrite the link to the
        // mangled name so struct layout, super calls, and vtable
        // chains all resolve through one compiled parent.
        let normalized;
        let class_decl = match &class_decl.parent {
            Some(p) if !p.params().is_empty() => {
                let mangled = self.instantiate_llvm_class(p.name(), p.params())?;
                let mut owned = class_decl.clone();
                owned.parent = Some(Type::generic(&mangled, vec![]));
                normalized = owned;
                &normalized
            }
            _ => class_decl,
        };
        if class_decl.parent.as_ref().is_some_and(|p| p.params().is_empty())
            && self
                .class_decls
                .get(&class_decl.name)
                .is_some_and(|d| d.parent != class_decl.parent)
        {
            self.class_decls
                .insert(class_decl.name.clone(), class_decl.clone());
        }
        let class_name = class_decl.name.clone();
        let _type_id = self.next_type_id;
        self.next_type_id += 1;

        // Collect field types.
        let mut field_types: HashMap<String, BasicTypeEnum<'ctx>> = HashMap::new();
        let mut field_decls: Vec<crate::ast::FieldDecl> = Vec::new();
        let mut method_names: Vec<String> = Vec::new();
        let mut constructor: Option<FunctionValue> = None;

        for member in &class_decl.members {
            match member {
                ClassMember::Field(field) => {
                    let ft = llvm_types::llvm_type(self.context, &field.typ)?;
                    field_types.insert(field.name.clone(), ft);
                    field_decls.push(field.clone());
                    // Cache the Titrate type name for this field so that
                    // infer_expr_type can resolve field types (e.g., this._adj -> HashMap).
                    self.field_titrate_type_names.insert(
                        format!("{}_{}", class_name, field.name),
                        field.typ.name().to_string(),
                    );
                }
                ClassMember::Method(method) => {
                    method_names.push(method.name.clone());
                }
                ClassMember::Constructor(method) => {
                    method_names.push(method.name.clone());
                    // We'll compile the constructor and set it later.
                }
            }
        }

        // Build the struct type, embedding the parent's fields (after the
        // vtable pointer) so the parent's constructor and inherited field
        // access operate on the same offsets.
        let parent_struct_type = if let Some(parent) = &class_decl.parent {
            let parent_name = parent.name();
            if let Some(pi) = self.class_infos.get(parent_name) {
                Some(pi.struct_type)
            } else if let Some(pd) = self.class_decls.get(parent_name).cloned() {
                Some(self.build_class_struct_type_for(&pd)?)
            } else {
                None
            }
        } else {
            None
        };
        let struct_type = build_class_struct_type(
            self.context,
            &class_name,
            &field_decls,
            &field_types,
            parent_struct_type,
        );

        // Collect the FULL field list (inherited fields first, then own
        // fields) so `field_index` resolves inherited fields and the GEP
        // offsets match the struct layout.
        let mut fields: Vec<(String, BasicTypeEnum<'ctx>)> = Vec::new();
        {
            let mut chain: Vec<String> = Vec::new();
            let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
            let mut cur = class_decl
                .parent
                .as_ref()
                .map(|t| t.name().to_string());
            while let Some(pname) = cur {
                if visited.contains(&pname) {
                    break;
                }
                visited.insert(pname.clone());
                chain.push(pname.clone());
                cur = self
                    .class_decls
                    .get(&pname)
                    .and_then(|cd| cd.parent.as_ref().map(|t| t.name().to_string()));
            }
            for pname in chain.iter().rev() {
                if let Some(pc) = self.class_decls.get(pname) {
                    for m in &pc.members {
                        if let ClassMember::Field(f) = m {
                            let ft = llvm_types::llvm_type(self.context, &f.typ)?;
                            fields.push((f.name.clone(), ft));
                        }
                    }
                }
            }
        }
        for f in &field_decls {
            let ft = field_types
                .get(&f.name)
                .copied()
                .unwrap_or_else(|| self.context.ptr_type(AddressSpace::default()).into());
            fields.push((f.name.clone(), ft));
        }

        // Insert a skeleton class_info before compiling methods so that
        // this.field stores during method compilation can find the class.
        {
            let skeleton = ClassInfo {
                name: class_name.clone(),
                struct_type,
                fields: fields.clone(),
                method_names: method_names.clone(),
                constructor: None,
                vtable_global: None,
                parent: class_decl.parent.as_ref().map(|t| t.name().to_string()),
                ifaces: class_decl
                    .ifaces
                    .iter()
                    .map(|t| t.name().to_string())
                    .collect(),
            };
            self.class_infos.insert(class_name.clone(), skeleton);
        }

        // Compile methods and collect their LLVM function values.
        let mut method_functions: HashMap<String, FunctionValue<'ctx>> = HashMap::new();
        for member in &class_decl.members {
            match member {
                ClassMember::Method(method) => {
                    let fn_val = self.compile_class_method(&class_name, method, &struct_type)?;
                    method_functions.insert(method.name.clone(), fn_val);
                }
                ClassMember::Constructor(method) => {
                    let fn_val = self.compile_class_method(&class_name, method, &struct_type)?;
                    method_functions.insert(method.name.clone(), fn_val);
                    constructor = Some(fn_val);
                }
                _ => {}
            }
        }

        // Create the vtable. Order is prefix-stable: root-most ancestor
        // methods first, then this class's own new methods, so a virtual
        // call indexed through any static ancestor type lands on the
        // right slot. Overrides keep the ancestor's slot: `method_functions`
        // already holds this class's implementations, and the loops below
        // only fill names the class itself does not define.
        let mut all_method_names: Vec<String> = Vec::new();
        let mut all_method_functions: HashMap<String, FunctionValue<'ctx>> = method_functions;
        let own_names: std::collections::HashSet<String> =
            method_names.iter().cloned().collect();
        let mut chain: Vec<String> = Vec::new();
        {
            let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
            let mut cur = class_decl.parent.as_ref().map(|t| t.name().to_string());
            while let Some(pname) = cur {
                if visited.contains(&pname) {
                    break;
                }
                visited.insert(pname.clone());
                chain.push(pname.clone());
                cur = self
                    .class_decls
                    .get(&pname)
                    .and_then(|cd| cd.parent.as_ref().map(|t| t.name().to_string()));
            }
        }
        for pname in chain.iter().rev() {
            // Prefer the parent's own compiled order (already
            // prefix-stable); fall back to its declaration order.
            let parent_order: Vec<String> = if let Some(pi) = self.class_infos.get(pname) {
                pi.method_names.clone()
            } else if let Some(pd) = self.class_decls.get(pname) {
                pd.members
                    .iter()
                    .filter_map(|m| match m {
                        ClassMember::Method(mm) => Some(mm.name.clone()),
                        _ => None,
                    })
                    .collect()
            } else {
                Vec::new()
            };
            for mname in parent_order {
                // The slot always comes from the ancestor; only the
                // implementation is skipped when this class overrides it
                // (its own function value already won in the map).
                if !all_method_names.contains(&mname) {
                    all_method_names.push(mname.clone());
                }
                if !own_names.contains(&mname)
                    && !all_method_functions.contains_key(&mname)
                {
                    let fn_name = format!("{}_{}", pname, mname);
                    if let Some(f) = self.module.get_function(&fn_name) {
                        all_method_functions.insert(mname.clone(), f);
                    }
                }
            }
            // Declaration-order sweep for members the compiled order may
            // predate (keeps the set complete).
            if let Some(pd) = self.class_decls.get(pname) {
                for m in &pd.members {
                    if let ClassMember::Method(mm) = m {
                        if !all_method_names.contains(&mm.name) {
                            all_method_names.push(mm.name.clone());
                        }
                        if !own_names.contains(&mm.name)
                            && !all_method_functions.contains_key(&mm.name)
                        {
                            let fn_name = format!("{}_{}", pname, mm.name);
                            if let Some(f) = self.module.get_function(&fn_name) {
                                all_method_functions.insert(mm.name.clone(), f);
                            }
                        }
                    }
                }
            }
        }
        for n in &method_names {
            if !all_method_names.contains(n) {
                all_method_names.push(n.clone());
            }
        }
        let vtable_global = create_vtable_global(
            self.context,
            &self.module,
            &class_name,
            &all_method_names,
            &all_method_functions,
        );

        // Update class_info with the final vtable_global and constructor.
        let final_class_info = ClassInfo {
            name: class_name.clone(),
            struct_type,
            fields,
            method_names: all_method_names,
            constructor,
            vtable_global,
            parent: class_decl.parent.as_ref().map(|t| t.name().to_string()),
            ifaces: class_decl
                .ifaces
                .iter()
                .map(|t| t.name().to_string())
                .collect(),
        };

        self.class_infos.insert(class_name, final_class_info);
        Ok(())
    }

    /// Compile a class method (or constructor).
    pub(super) fn compile_class_method(
        &mut self,
        class_name: &str,
        method: &crate::ast::MethodDecl,
        _struct_type: &inkwell::types::StructType<'ctx>,
    ) -> Result<FunctionValue<'ctx>, String> {
        let i8_ptr = self.context.ptr_type(AddressSpace::default());
        let mut param_types: Vec<inkwell::types::BasicMetadataTypeEnum> = Vec::new();
        // First parameter is always `this` (i8*); container params pass by slot pointer.
        param_types.push(i8_ptr.into());
        for p in &method.params {
            let ty = self.user_param_llvm_type(&p.typ)?;
            param_types.push(ty.into());
        }
        let return_type = self.sig_ret_llvm_type(method.return_type.as_ref())?;
        let fn_type = match return_type {
            Some(ret) => ret.fn_type(&param_types, false),
            None => self.context.void_type().fn_type(&param_types, false),
        };

        let fn_name = format!("{}_{}", class_name, method.name);
        let fn_val = self.module.add_function(&fn_name, fn_type, None);
        self.functions.insert(fn_name.clone(), fn_val);

        // Create entry block.
        let entry = self.context.append_basic_block(fn_val, "entry");
        self.builder.position_at_end(entry);

        // Save locals and current_this.
        let saved_locals = std::mem::take(&mut self.locals);
        let saved_this = self.current_this;
        let saved_class_name = self.current_class_name.clone();
        let saved_ret_ty = self.current_ret_ty.clone();
        self.current_ret_ty = method.return_type.clone();

        // Set up `this` pointer.
        let this_param = fn_val
            .get_nth_param(0)
            .ok_or_else(|| format!("missing this param for {}", fn_name))?;
        let this_ptr = this_param.into_pointer_value();

        // Cast this to the struct pointer type for field access via GEP.
        let struct_ptr_type = self.context.ptr_type(AddressSpace::default());
        let struct_ptr = if this_ptr.get_type() != struct_ptr_type {
            self.builder
                .build_bit_cast(this_ptr, struct_ptr_type, "this.cast")
                .map_err(|e| format!("build_bit_cast this failed: {:?}", e))?
                .into_pointer_value()
        } else {
            this_ptr
        };

        self.current_this = Some(struct_ptr);
        self.current_class_name = Some(class_name.to_string());

        // Bind method parameters (skip `this`); container params alias the caller.
        for (i, p) in method.params.iter().enumerate() {
            let param_val = fn_val
                .get_nth_param((i + 1) as u32)
                .ok_or_else(|| format!("missing param {} for {}", i, fn_name))?;
            self.bind_param(&p.name, &p.typ, param_val)?;
        }

        // Compile method body.
        for s in &method.body {
            self.compile_stmt(s)?;
        }

        // Add a default return if the current block has no terminator.
        if self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_none()
        {
            match &method.return_type {
                Some(t) if !llvm_types::is_void(t) => {
                    let ty = self.sig_ret_llvm_type(Some(t))?.ok_or_else(|| {
                        format!("codegen: void return for '{}'", fn_name)
                    })?;
                    let zero: BasicValueEnum<'ctx> = match ty {
                        BasicTypeEnum::IntType(it) => it.const_int(0, false).into(),
                        BasicTypeEnum::FloatType(ft) => ft.const_float(0.0).into(),
                        BasicTypeEnum::PointerType(pt) => pt.const_null().into(),
                        _ => {
                            self.builder
                                .build_return(None)
                                .map_err(|e| format!("build_return failed: {:?}", e))?;
                            self.locals = saved_locals;
                            self.current_this = saved_this;
                            self.current_class_name = saved_class_name;
                            self.current_ret_ty = saved_ret_ty;
                            return Ok(fn_val);
                        }
                    };
                    self.builder
                        .build_return(Some(&zero))
                        .map_err(|e| format!("build_return zero failed: {:?}", e))?;
                }
                _ => {
                    self.builder
                        .build_return(None)
                        .map_err(|e| format!("build_return void failed: {:?}", e))?;
                }
            }
        }

        // Restore locals and this.
        self.locals = saved_locals;
        self.current_this = saved_this;
        self.current_class_name = saved_class_name;
        self.current_ret_ty = saved_ret_ty;

        Ok(fn_val)
    }

    /// Compile a per-class clone of an interface default method so calls on
    /// `this` inside the body resolve against the implementing class (the
    /// shared default only sees the interface). Clones are memoized by
    /// `Class_method` name; a default that calls itself resolves to the
    /// in-progress clone, matching recursive-method behavior.
    pub(super) fn ensure_default_clone(
        &mut self,
        class_name: &str,
        method_sig: &crate::ast::MethodSig,
        body: &[Stmt],
    ) -> Result<FunctionValue<'ctx>, String> {
        let fn_name = format!("{}_{}", class_name, method_sig.name);
        if let Some(existing) = self.module.get_function(&fn_name) {
            return Ok(existing);
        }
        let i8_ptr = self.context.ptr_type(AddressSpace::default());
        let mut param_types: Vec<inkwell::types::BasicMetadataTypeEnum> = Vec::new();
        param_types.push(i8_ptr.into());
        for p in &method_sig.params {
            let ty = self.user_param_llvm_type(&p.typ)?;
            param_types.push(ty.into());
        }
        let return_type = self.sig_ret_llvm_type(method_sig.return_type.as_ref())?;
        let fn_type = match return_type {
            Some(ret) => ret.fn_type(&param_types, false),
            None => self.context.void_type().fn_type(&param_types, false),
        };
        let fn_val = self.module.add_function(&fn_name, fn_type, None);
        self.functions.insert(fn_name.clone(), fn_val);

        let entry = self.context.append_basic_block(fn_val, "entry");
        self.builder.position_at_end(entry);
        let saved_locals = std::mem::take(&mut self.locals);
        let saved_this = self.current_this;
        let saved_class_name = self.current_class_name.clone();
        let saved_ret_ty = self.current_ret_ty.clone();
        self.current_ret_ty = method_sig.return_type.clone();

        let this_param = fn_val
            .get_nth_param(0)
            .ok_or_else(|| format!("missing this param for {}", fn_name))?;
        self.current_this = Some(this_param.into_pointer_value());
        self.current_class_name = Some(class_name.to_string());

        for (i, p) in method_sig.params.iter().enumerate() {
            let param_val = fn_val
                .get_nth_param((i + 1) as u32)
                .ok_or_else(|| format!("missing param {} for {}", i, fn_name))?;
            self.bind_param(&p.name, &p.typ, param_val)?;
        }

        for s in body {
            self.compile_stmt(s)?;
        }

        if self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_none()
        {
            match &method_sig.return_type {
                Some(t) if !llvm_types::is_void(t) => {
                    let ty = self.sig_ret_llvm_type(Some(t))?.ok_or_else(|| {
                        format!("codegen: void return for '{}'", fn_name)
                    })?;
                    let zero: BasicValueEnum<'ctx> = match ty {
                        BasicTypeEnum::IntType(it) => it.const_int(0, false).into(),
                        BasicTypeEnum::FloatType(ft) => ft.const_float(0.0).into(),
                        BasicTypeEnum::PointerType(pt) => pt.const_null().into(),
                        _ => {
                            self.builder
                                .build_return(None)
                                .map_err(|e| format!("build_return failed: {:?}", e))?;
                            self.locals = saved_locals;
                            self.current_this = saved_this;
                            self.current_class_name = saved_class_name;
                            self.current_ret_ty = saved_ret_ty;
                            return Ok(fn_val);
                        }
                    };
                    self.builder
                        .build_return(Some(&zero))
                        .map_err(|e| format!("build_return zero failed: {:?}", e))?;
                }
                _ => {
                    self.builder
                        .build_return(None)
                        .map_err(|e| format!("build_return void failed: {:?}", e))?;
                }
            }
        }

        self.locals = saved_locals;
        self.current_this = saved_this;
        self.current_class_name = saved_class_name;
        self.current_ret_ty = saved_ret_ty;

        Ok(fn_val)
    }

    /// Compile an interface declaration: register interface info, compile
    /// default methods, and build the fat pointer type.
    pub(super) fn compile_interface_decl(&mut self, iface_decl: &InterfaceDecl) -> Result<(), String> {
        let iface_name = iface_decl.name.clone();
        let fat_ptr_type = build_interface_fat_ptr_type(self.context);

        // Collect method names.
        let mut method_names: Vec<String> = Vec::new();
        let mut default_methods: HashMap<String, FunctionValue<'ctx>> = HashMap::new();

        for method_sig in &iface_decl.methods {
            method_names.push(method_sig.name.clone());
            self.iface_method_sigs.insert(
                (iface_name.clone(), method_sig.name.clone()),
                IfaceMethodSig {
                    params: method_sig.params.iter().map(|p| p.typ.clone()).collect(),
                    ret: method_sig.return_type.clone(),
                },
            );

            // Compile default method body if present.
            if method_sig.body.is_some() {
                let i8_ptr = self.context.ptr_type(AddressSpace::default());
                let mut param_types: Vec<inkwell::types::BasicMetadataTypeEnum> = Vec::new();
                param_types.push(i8_ptr.into());
                for p in &method_sig.params {
                    let ty = self.user_param_llvm_type(&p.typ)?;
                    param_types.push(ty.into());
                }
                let return_type =
                    self.sig_ret_llvm_type(method_sig.return_type.as_ref())?;
                let fn_type = match return_type {
                    Some(ret) => ret.fn_type(&param_types, false),
                    None => self.context.void_type().fn_type(&param_types, false),
                };

                let fn_name = format!("{}_default_{}", iface_name, method_sig.name);
                let fn_val = self.module.add_function(&fn_name, fn_type, None);
                self.functions.insert(fn_name.clone(), fn_val);

                // The body is intentionally not compiled here: without an
                // implementing class, calls on `this` cannot resolve. Each
                // implementing class gets a compiled per-class clone (see
                // ensure_default_clone) that carries the real code, so this
                // shared declaration is only a vtable fallback entry.
                default_methods.insert(method_sig.name.clone(), fn_val);
            }
        }

        let iface_info = InterfaceInfo {
            name: iface_name.clone(),
            fat_ptr_type,
            method_names: method_names.clone(),
            default_methods,
            vtable_type: None,
        };

        self.interface_infos.insert(iface_name, iface_info);
        Ok(())
    }
}
