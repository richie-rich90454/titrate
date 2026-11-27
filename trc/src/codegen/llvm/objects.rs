// Object construction, field access, and casts

use super::native_bridge;
use super::types as llvm_types;
use super::vtable::{emit_as_cast, emit_field_access, emit_interface_is_check, emit_is_check, emit_new_allocation};
use super::LlvmBackend;
use crate::ast::Expr;
use inkwell::AddressSpace;
use inkwell::values::BasicValueEnum;

impl<'ctx> LlvmBackend<'ctx> {
    /// Compile a `new ClassName(args)` expression.
    pub(super) fn compile_new(
        &mut self,
        type_name: &crate::ast::Type,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let class_name = type_name.name();

        // Handle built-in container types via native bridge.
        if class_name == "ArrayList" || class_name == "HashMap" {
            let native_name = format!("{}_new", class_name);
            let mut arg_vals: Vec<BasicValueEnum> = Vec::new();
            let mut arg_tys: Vec<crate::ast::Type> = Vec::new();
            for arg in args {
                arg_vals.push(self.compile_expr(arg)?);
                arg_tys.push(self.infer_expr_type(arg));
            }
            // Call the native function and ensure the result is a {i64, ptr} struct.
            let result = self.compile_native_call_declared(
                &native_name,
                &arg_vals,
                &arg_tys,
                Some(type_name),
            )?;
            // If the native bridge returned a non-struct (e.g., i32 due to incorrect type inference),
            // bitcast it to the expected {i64, ptr} container type.
            if !result.is_struct_value() {
                let string_ty = llvm_types::string_type(self.context).into_struct_type();
                let i64_val = if result.is_int_value() {
                    self.builder
                        .build_int_z_extend(
                            result.into_int_value(),
                            self.context.i64_type(),
                            "new.pad",
                        )
                        .map_err(|e| format!("build_int_z_extend failed: {:?}", e))?
                } else {
                    self.context.i64_type().const_int(0, false)
                };
                let sv = self
                    .builder
                    .build_insert_value(
                        self.builder
                            .build_insert_value(string_ty.const_zero(), i64_val, 0, "new.len")
                            .map_err(|e| format!("insert failed: {:?}", e))?,
                        self.context.ptr_type(AddressSpace::default()).const_null(),
                        1,
                        "new.ptr",
                    )
                    .map_err(|e| format!("insert failed: {:?}", e))?;
                match sv {
                    inkwell::values::AggregateValueEnum::StructValue(s) => {
                        Ok(BasicValueEnum::StructValue(s))
                    }
                    _ => Err("expected struct".into()),
                }
            } else {
                Ok(result)
            }
        } else {
            // Monomorphize generic instantiations (e.g. Box<int>) so the
            // rest of this path works with the specialized class.
            let effective = self.resolve_mono_class(type_name)?;
            let class_name = effective.as_str();
            // A `new B(...)` in a method of class A can be lowered before B's
            // declaration pass (imports compile in load order). B's ClassInfo
            // is then only a skeleton with no vtable or constructor, so this
            // allocation would be an uninitialized object whose first virtual
            // call dereferences a null vtable. Finish B on demand.
            if !self.classes_compiled.contains(class_name)
                && !self.classes_compiling.contains(class_name)
            {
                if let Some(decl) = self.class_decls.get(class_name).cloned() {
                    if decl.type_params.is_empty() {
                        let saved_block = self.builder.get_insert_block();
                        self.compile_class_decl(&decl)?;
                        if let Some(block) = saved_block {
                            self.builder.position_at_end(block);
                        }
                    }
                }
            }
            // Try class_infos first.
            if let Some(class_info) = self.class_infos.get(class_name).cloned() {
                let ctor = class_info.constructor;
                let arg_vals = match ctor {
                    Some(ctor_fn) => {
                        let ctor_params = self.method_decl_params(class_name, "init");
                        self.build_this_call_args(ctor_fn, ctor_params.as_deref(), args)?
                    }
                    None => {
                        let mut arg_vals: Vec<BasicValueEnum> = Vec::new();
                        for arg in args {
                            arg_vals.push(self.compile_expr(arg)?);
                        }
                        arg_vals
                    }
                };
                return emit_new_allocation(
                    self.context,
                    &self.builder,
                    &self.module,
                    &class_info,
                    ctor,
                    &arg_vals,
                );
            }
            // Fallback for imported classes: try ClassName_new as a native function.
            let native_name = format!("{}_new", class_name);
            if native_bridge::is_native_function(&native_name) {
                let mut arg_vals: Vec<BasicValueEnum> = Vec::new();
                let mut arg_tys: Vec<crate::ast::Type> = Vec::new();
                for arg in args {
                    arg_vals.push(self.compile_expr(arg)?);
                    arg_tys.push(self.infer_expr_type(arg));
                }
                return self.compile_native_call_declared(
                    &native_name,
                    &arg_vals,
                    &arg_tys,
                    Some(type_name),
                );
            }
            // A `new Generic<T>(...)` inside a generic default method has no
            // concrete instantiation. Such code is compiled but not executed;
            // hand back an opaque null pointer to keep the IR well-formed.
            if type_name.params().iter().any(super::fn_sig_has_type_param) {
                let ptr_ty = self.context.ptr_type(AddressSpace::default());
                return Ok(ptr_ty.const_null().into());
            }
            // Unknown classes are rejected by the analyzer, so reaching here
            // means an internal error. Report it honestly instead of
            // emitting a null pointer that would fail far from the cause.
            Err(format!(
                "codegen: unknown class in new expression: {}",
                class_name
            ))
        }
    }

    /// Compile a member access expression: obj.field
    pub(super) fn compile_member_access(
        &mut self,
        object: &Expr,
        field: &str,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let obj_val = self.compile_expr(object)?;
        let obj_ptr_opt = if obj_val.is_pointer_value() {
            Some(obj_val.into_pointer_value())
        } else if obj_val.is_struct_value() && self.is_string_struct(&obj_val) {
            // A class element read from a generic ArrayList arrives as a
            // { i64, ptr } struct; field 1 holds the object pointer.
            obj_val.into_struct_value()
                .get_type()
                .get_field_type_at_index(1)
                .and_then(|_| {
                    self.builder.build_extract_value(obj_val.into_struct_value(), 1, "obj.ptr").ok()
                })
                .and_then(|v| if v.is_pointer_value() { Some(v.into_pointer_value()) } else { None })
        } else {
            None
        };
        if let Some(obj_ptr) = obj_ptr_opt {
            let obj_type = self.infer_expr_type(object);
            let effective = self.resolve_mono_class(&obj_type)?;
            let class_name = effective.as_str();
            if let Some(class_info) = self.class_infos.get(class_name).cloned() {
                return emit_field_access(self.context, &self.builder, &class_info, obj_ptr, field);
            }
            // Unknown classes are rejected by the analyzer, so reaching here
            // means an internal error. Report it honestly instead of
            // emitting a null pointer that would fail far from the cause.
            return Err(format!(
                "codegen: member access on unknown class: {}",
                class_name
            ));
        }
        // If the value is a null pointer (from unknown type), return null.
        if obj_val.is_int_value() {
            let ptr_ty = self.context.ptr_type(AddressSpace::default());
            return Ok(ptr_ty.const_null().into());
        }
        Err(format!(
            "codegen: member access on non-pointer value: {:?}",
            object
        ))
    }

    /// Compile a `this` expression.
    pub(super) fn compile_this(&self, _span: &crate::ast::Span) -> Result<BasicValueEnum<'ctx>, String> {
        match self.current_this {
            Some(this_ptr) => Ok(this_ptr.into()),
            None => Err("codegen: 'this' used outside of a method".to_string()),
        }
    }

    /// Compile an `is` type check expression.
    pub(super) fn compile_is(
        &mut self,
        obj: &Expr,
        ty: &crate::ast::Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let type_name = ty.name();
        // Check if it's an interface.
        if let Some(iface_info) = self.interface_infos.get(type_name).cloned() {
            let obj_val = self.compile_expr(obj)?;
            if !obj_val.is_struct_value() {
                return Err(format!(
                    "codegen: 'is' check on non-struct (interface) value: {:?}",
                    obj
                ));
            }
            // For interface `is` check, we need an interface vtable. Look up
            // the first available class vtable for this interface.
            let fat_ptr_type = iface_info.fat_ptr_type;
            let i1_ty = self.context.bool_type();
            // Find any implementing class vtable for this interface.
            for ((iface_name, _class_name), vt_global) in &self.interface_vtables {
                if iface_name == type_name {
                    let expected_vt = vt_global.as_pointer_value();
                    let result = emit_interface_is_check(
                        self.context,
                        &self.builder,
                        fat_ptr_type,
                        obj_val,
                        expected_vt,
                    )?;
                    return Ok(result);
                }
            }
            return Ok(i1_ty.const_int(0, false).into());
        }
        // Primitive type checks (e.g. `value is bool` on a Variant): the
        // runtime tag is not available for opaque Variant values, so use the
        // statically inferred type as a best-effort approximation.
        if llvm_types::is_primitive_name(type_name)
            || matches!(type_name, "ArrayList" | "HashMap" | "array" | "Variant" | "Result")
        {
            let i1_ty = self.context.bool_type();
            let matches = self.infer_expr_type(obj).name() == type_name;
            return Ok(i1_ty.const_int(if matches { 1 } else { 0 }, false).into());
        }
        // Check if it's a class.
        let class_name = type_name;
        let class_info =
            self.class_infos.get(class_name).cloned().ok_or_else(|| {
                format!("codegen: type '{}' not found for 'is' check", class_name)
            })?;
        let obj_val = self.compile_expr(obj)?;
        if !obj_val.is_pointer_value() {
            // A non-pointer value cannot be an instance of a user class.
            return Ok(self.context.bool_type().const_zero().into());
        }
        let obj_ptr = obj_val.into_pointer_value();
        match &class_info.vtable_global {
            Some(vtable_global) => emit_is_check(
                self.context,
                &self.builder,
                obj_ptr,
                vtable_global.as_pointer_value(),
            ),
            None => {
                // Class has no vtable (no methods); return false.
                let i1_ty = self.context.bool_type();
                Ok(i1_ty.const_int(0, false).into())
            }
        }
    }

    /// Compile an `as` cast expression.
    pub(super) fn compile_cast(
        &mut self,
        obj: &Expr,
        ty: &crate::ast::Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let type_name = ty.name();
        // Check if casting to an interface. Route through the shared wrapper
        // so the vtable is built on demand and a null fat pointer is never
        // emitted for a concrete class.
        if self.interface_infos.contains_key(type_name) {
            let obj_val = self.compile_expr(obj)?;
            if !obj_val.is_pointer_value() {
                return Err(format!(
                    "codegen: 'as' cast to interface on non-pointer value: {:?}",
                    obj
                ));
            }
            let obj_ptr = obj_val.into_pointer_value();
            let obj_type = self.infer_expr_type(obj);
            let class_name = obj_type.name();
            return self.wrap_object_ptr_in_interface(obj_ptr, type_name, class_name);
        }
        // Numeric cast (int -> float, float -> int, int -> int, float -> float).
        // This handles expressions like `(i / 4) as double` where the operand
        // is a primitive numeric value, not a pointer.
        if llvm_types::is_numeric(ty) {
            let obj_val = self.compile_expr(obj)?;
            let target_ty = llvm_types::llvm_type(self.context, ty)?;
            if obj_val.is_pointer_value() {
                // A generic/dynamic value is an opaque pointer. Reinterpret its
                // bits as the requested numeric type so binary ops stay typed.
                let i64_ty = self.context.i64_type();
                let bits = self
                    .builder
                    .build_ptr_to_int(obj_val.into_pointer_value(), i64_ty, "cast.ptr.bits")
                    .map_err(|e| format!("build_ptr_to_int cast failed: {:?}", e))?;
                return self.cast_value_to_type(bits.into(), target_ty);
            }
            return self.cast_value_to_type(obj_val, target_ty);
        }
        // Simple class cast: identity.
        let obj_val = self.compile_expr(obj)?;
        if obj_val.is_pointer_value() {
            let obj_ptr = obj_val.into_pointer_value();
            return emit_as_cast(self.context, &self.builder, obj_ptr);
        }
        // int -> int (different widths) or same-width identity cast.
        // This handles cases like `(code as char)` where both source and target
        // are integer types but the target type wasn't recognized as numeric above.
        if obj_val.is_int_value() {
            let target_ty = llvm_types::llvm_type(self.context, ty)?;
            if target_ty.is_int_type() {
                return self.cast_value_to_type(obj_val, target_ty);
            }
        }
        // float -> int or int -> float for non-numeric target types.
        if obj_val.is_float_value() {
            let target_ty = llvm_types::llvm_type(self.context, ty)?;
            if target_ty.is_int_type() {
                return self.cast_value_to_type(obj_val, target_ty);
            }
        }
        if obj_val.is_int_value() {
            let target_ty = llvm_types::llvm_type(self.context, ty)?;
            if target_ty.is_float_type() {
                return self.cast_value_to_type(obj_val, target_ty);
            }
        }
        // As a last resort, if both types are the same, just return the value.
        Ok(obj_val)
    }
}
