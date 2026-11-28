// Object construction, field access, and casts

use super::native_bridge;
use super::types as llvm_types;
use super::vtable::{emit_as_cast, emit_field_access, emit_interface_is_check, emit_is_check, emit_new_allocation};
use super::LlvmBackend;
use crate::ast::Expr;
use inkwell::AddressSpace;
use inkwell::values::BasicValueEnum;

impl<'ctx> LlvmBackend<'ctx> {
    /// Resolve a possibly module-qualified class name (`Module.Inner` /
    /// `Module::Inner`) to the bare name classes are registered under.
    /// Source references may be qualified while declarations are not; exact
    /// keys win so a genuinely qualified registered name still resolves.
    pub(super) fn resolve_class_name(&self, name: &str) -> String {
        if self.class_infos.contains_key(name) || self.class_decls.contains_key(name) {
            return name.to_string();
        }
        let mut rest = name;
        while let Some((_, suffix)) = rest.split_once('.').or_else(|| rest.split_once("::")) {
            rest = suffix;
            if self.class_infos.contains_key(rest) || self.class_decls.contains_key(rest) {
                return rest.to_string();
            }
        }
        name.to_string()
    }

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
            let class_name = self.resolve_class_name(&effective);
            let class_name = class_name.as_str();
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
                // Pick the constructor whose arity matches this `new`; the
                // single ClassInfo slot holds only the first declaration and
                // would otherwise bind a wrong-arity overload.
                let ctor = self
                    .resolve_user_fn(&format!("{}_new", class_name), args.len() + 1)
                    .or(class_info.constructor);
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
            let class_name = self.resolve_class_name(&effective);
            if let Some(class_info) = self.class_infos.get(&class_name).cloned() {
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
        // Enum values are heap pointers, not classes with vtables. The bare
        // `is Enum` name must resolve against the enum declarations.
        if llvm_types::resolve_is_target_name(type_name, self.enum_infos.keys()).is_some() {
            let obj_val = self.compile_expr(obj)?;
            if !obj_val.is_pointer_value() {
                return Ok(self.context.bool_type().const_int(1, false).into());
            }
            let null = self.context.ptr_type(AddressSpace::default()).const_null();
            return self
                .builder
                .build_int_compare(inkwell::IntPredicate::NE, obj_val.into_pointer_value(), null, "is.enum")
                .map_err(|e| format!("build_int_compare is.enum failed: {:?}", e))
                .map(BasicValueEnum::from);
        }
        // Check if it's a class. Generic classes are registered under mangled
        // names, so resolve the bare source name against compiled names first.
        let class_name = llvm_types::resolve_is_target_name(type_name, self.class_infos.keys())
            .unwrap_or_else(|| self.resolve_class_name(type_name));
        let class_info =
            self.class_infos.get(&class_name).cloned().ok_or_else(|| {
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
            // A Variant payload has the same `{ i64, ptr }` shape as a string
            // but field 1 is a null tag word, not UTF-8 data. Extract field 0
            // directly so `variant_element as long` never dereferences it.
            let source_ty = self.infer_expr_type(obj);
            let source_name = source_ty.name();
            if obj_val.is_struct_value()
                && target_ty.is_int_type()
                && matches!(source_name, "Variant" | "unknown")
                && self.is_string_struct(&obj_val)
            {
                if let BasicValueEnum::StructValue(sv) = obj_val {
                    if let Ok(f0) = self.builder.build_extract_value(sv, 0, "cast.variant.num") {
                        return self.cast_value_to_type(f0, target_ty);
                    }
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::vtable::ClassInfo;
    use crate::ast::{Declaration, Program};

    fn parse_program(source: &str) -> Program {
        let tokens = crate::lexer::tokenize(source).expect("tokenize");
        crate::parser::parse(tokens).expect("parse")
    }

    #[test]
    fn variant_payload_cast_to_int_does_not_decode_utf8() {
        // Build the IR for an unknown-typed `{ i64, ptr }` local (a Variant
        // payload from a native array element) cast to int. The cast must
        // extract field 0 directly, never dereference the null tag word.
        let context = inkwell::context::Context::create();
        let mut backend = LlvmBackend::new(&context, "variant-cast");
        let f = backend.module.add_function(
            "check_variant_cast",
            context.void_type().fn_type(&[], false),
            None,
        );
        let entry = context.append_basic_block(f, "entry");
        backend.builder.position_at_end(entry);
        let container_ty = native_bridge::titrate_array_type(&context);
        let slot = backend.builder.build_alloca(container_ty, "variant.slot").expect("alloca");
        let zero = container_ty.const_zero();
        backend.builder.build_store(slot, zero).expect("store");
        backend.locals.insert(
            "v".to_string(),
            super::super::LocalVar {
                ptr: slot,
                ty: container_ty.into(),
                full_type: None,
                titrate_type: Some("unknown".to_string()),
                shared: false,
                closure_sig: None,
            },
        );
        let expr = Expr::Identifier("v".to_string(), crate::ast::Span::unknown());
        let result = backend
            .compile_cast(&expr, &crate::ast::Type::simple("int"))
            .expect("cast");
        assert!(result.is_int_value(), "cast must yield an int");
        let ir = backend.module.print_to_string().to_string();
        assert!(
            ir.contains("cast.variant.num"),
            "Variant numeric cast must extract field 0 directly: {}",
            ir
        );
    }

    fn backend_with_inner<'ctx>(
        context: &'ctx inkwell::context::Context,
        program: &Program,
    ) -> LlvmBackend<'ctx> {
        let mut backend = LlvmBackend::new(context, "nested_class");
        backend.declare_natives();
        for decl in &program.declarations {
            if let Declaration::Class(c) = decl {
                backend.class_decls.insert(c.name.clone(), c.clone());
                backend.register_class_skeleton(c).expect("skeleton");
                backend.compile_class_decl(c).expect("class decl");
            }
        }
        backend
    }

    #[test]
    fn nested_class_new_resolves_qualified_name() {
        let source = r#"
            public class Inner {
                public int value;
                public fn init(v: int) { this.value = v; }
            }
            public fn main(): void {
                let x = new Container.Inner(7);
            }
        "#;
        let program = parse_program(source);
        let context = inkwell::context::Context::create();
        let mut backend = backend_with_inner(&context, &program);
        let main = program
            .declarations
            .iter()
            .find_map(|d| match d {
                Declaration::Function(f) if f.name == "main" => Some(f),
                _ => None,
            })
            .expect("main");
        backend.compile_main(main).expect("qualified new must compile");
        assert!(
            backend.module.print_to_string().to_string().contains("@Inner_new("),
            "qualified new must call the bare class constructor"
        );
    }

    #[test]
    fn nested_class_member_access_resolves() {
        let source = r#"
            public class Inner {
                public int value;
                public fn init(v: int) { this.value = v; }
            }
            public fn main(): void {
                let x: Container.Inner = new Inner(3);
                let v: int = x.value;
            }
        "#;
        let program = parse_program(source);
        let context = inkwell::context::Context::create();
        let mut backend = backend_with_inner(&context, &program);
        let main = program
            .declarations
            .iter()
            .find_map(|d| match d {
                Declaration::Function(f) if f.name == "main" => Some(f),
                _ => None,
            })
            .expect("main");
        backend
            .compile_main(main)
            .expect("member access on a qualified class must compile");
        assert!(
            backend.module.print_to_string().to_string().contains("getelementptr"),
            "field access must emit a GEP into the resolved class struct"
        );
    }

    #[test]
    fn is_check_resolves_enum_and_generic_types() {
        let source = r#"
            enum ParseError { Syntax, Missing }
        "#;
        let program = parse_program(source);
        let context = inkwell::context::Context::create();
        let mut backend = LlvmBackend::new(&context, "is_check");
        backend.declare_natives();
        for decl in &program.declarations {
            if let Declaration::Enum(e) = decl {
                let info = super::super::enum_codegen::compile_enum_decl(&context, &backend.module, e)
                    .expect("enum decl");
                backend.enum_infos.insert(e.name.clone(), info);
            }
        }
        backend.class_infos.insert(
            "Array__int".to_string(),
            ClassInfo {
                name: "Array__int".to_string(),
                struct_type: context.struct_type(&[], false),
                fields: Vec::new(),
                method_names: Vec::new(),
                constructor: None,
                vtable_global: None,
                parent: None,
                ifaces: Vec::new(),
            },
        );

        let probe_ty = context.void_type().fn_type(&[], false);
        let probe = backend.module.add_function("is_probe", probe_ty, None);
        backend.builder.position_at_end(context.append_basic_block(probe, "entry"));
        let null = crate::ast::Expr::Literal(crate::ast::Literal::Null, crate::ast::Span::unknown());

        backend
            .compile_is(&null, &crate::ast::Type::simple("ParseError"))
            .expect("bare enum name must resolve against enum_infos");
        backend
            .compile_is(&null, &crate::ast::Type::simple("Array"))
            .expect("bare generic name must resolve against the mangled class");
    }
}
