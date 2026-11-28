// Member-access and native-fallback call dispatch

use super::native_bridge;
use super::vtable::{
    build_interface_fat_ptr_type, emit_direct_call, emit_field_access,
    emit_interface_method_call, emit_virtual_call,
};
use super::{IfaceMethodSig, LlvmBackend};
use crate::ast::{ClassMember, Expr, Type};
use inkwell::types::{BasicMetadataTypeEnum, BasicType};
use inkwell::values::{BasicMetadataValueEnum, BasicValue, BasicValueEnum};

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
        // A `Variant` value may be a boxed scalar (marshalled as TV_STRING
        // with `{value, marker}` payload) or a real class instance (raw
        // pointer in the first word). Rebuild the struct for the former;
        // reading its first word as a pointer produced address 5 for `5`.
        if value_ty.name() == "Variant" {
            let struct_ty = self.container_type();
            let struct_basic: inkwell::types::BasicTypeEnum = struct_ty.into();
            let raw = self
                .builder
                .build_load(struct_basic, val_ptr, "map.get.variant")
                .map_err(|e| format!("map get variant load failed: {:?}", e))?;
            let obj_bits = self
                .builder
                .build_load(self.context.i64_type(), val_ptr, "map.get.variant.obj.bits")
                .map_err(|e| format!("map get variant obj load failed: {:?}", e))?
                .into_int_value();
            let obj_ptr = self
                .builder
                .build_int_to_ptr(
                    obj_bits,
                    self.context.ptr_type(Default::default()),
                    "map.get.variant.obj",
                )
                .map_err(|e| format!("map get variant int_to_ptr failed: {:?}", e))?;
            let instance = self
                .builder
                .build_insert_value(
                    struct_ty.const_zero(),
                    obj_ptr,
                    1,
                    "map.get.variant.instance",
                )
                .map_err(|e| format!("map get variant insert failed: {:?}", e))?
                .into_struct_value()
                .into();
            let boxed_tags = [
                native_bridge::TV_STRING,
                native_bridge::TV_BOOL,
                native_bridge::TV_INT,
                native_bridge::TV_LONG,
                native_bridge::TV_FLOAT,
                native_bridge::TV_DOUBLE,
                native_bridge::TV_HALF,
                native_bridge::TV_QUAD,
            ];
            let mut is_boxed = self
                .builder
                .build_int_compare(
                    inkwell::IntPredicate::EQ,
                    tag,
                    i32_ty.const_int(boxed_tags[0] as u64, false),
                    "map.get.variant.boxed",
                )
                .map_err(|e| format!("map get variant boxed cmp failed: {:?}", e))?;
            for t in boxed_tags.iter().skip(1) {
                let eq = self
                    .builder
                    .build_int_compare(
                        inkwell::IntPredicate::EQ,
                        tag,
                        i32_ty.const_int(*t as u64, false),
                        "map.get.variant.boxed.more",
                    )
                    .map_err(|e| format!("map get variant boxed cmp failed: {:?}", e))?;
                is_boxed = self
                    .builder
                    .build_or(is_boxed, eq, "map.get.variant.boxed.or")
                    .map_err(|e| format!("map get variant boxed or failed: {:?}", e))?;
            }
            return self
                .builder
                .build_select(is_boxed, raw, instance, "map.get.value")
                .map_err(|e| format!("map get variant select failed: {:?}", e));
        }
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
        // The missing-key sentinel must have the same LLVM width as the map's
        // declared value type; an i32 sentinel for a long (i64) value trips
        // LLVM's select-operand assertion.
        let sentinel = raw.get_type().const_int(Self::NULL_INT_SENTINEL as u64, true);
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

    /// Store a container mutator's result back into a field receiver. The
    /// generic specialization is registered under a mangled class name, while
    /// the receiver's inferred type still carries the source name; point the
    /// lookup at the compiled class for the duration of the store.
    fn store_back_member_container(
        &mut self,
        receiver: &Expr,
        result: BasicValueEnum<'ctx>,
    ) -> Result<(), String> {
        if let Expr::MemberAccess(base, _, _) = receiver {
            if let Expr::Identifier(name, _) = base.as_ref() {
                let original = self.locals.get(name).cloned();
                if let Some(local) = &original {
                    if let Some(full) = &local.full_type {
                        if let Ok(mangled) = self.resolve_mono_class(full) {
                            if self.class_infos.contains_key(&mangled) && mangled != full.name() {
                                let mut patched = local.clone();
                                patched.full_type = Some(Type::simple(&mangled));
                                patched.titrate_type = Some(mangled);
                                self.locals.insert(name.clone(), patched);
                                let stored = self.store_back_container(receiver, result);
                                self.locals.insert(name.clone(), local.clone());
                                return stored;
                            }
                        }
                    }
                }
            }
        }
        self.store_back_container(receiver, result)
    }

    /// Lower `ArrayList.add(index, value)` as an insert. The native
    /// `ArrayList_add` is append-only and treats its second argument as the
    /// element, so passing the index there silently stores the index instead
    /// of the value (ChainMap.push appended `0` and later crashed).
    fn compile_arraylist_insert(
        &mut self,
        obj: &Expr,
        index: &Expr,
        value: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let obj_val = self.compile_expr(obj)?;
        let obj_ty = self.infer_expr_type(obj);
        let idx_val = self.compile_expr(index)?;
        let val_val = self.compile_expr(value)?;
        let val_ty = self.infer_expr_type(value);
        let elem_ty = self.array_element_type(obj).unwrap_or(val_ty.clone());
        let i32_ty = self.context.i32_type();

        // Append first: this is the only native that can grow the list, and
        // it works for an empty receiver. The old get/set/add "insert" failed
        // on an empty list (set was out of bounds) and put the value after
        // the displaced element rather than before it.
        let appended = self.compile_native_call_declared(
            "ArrayList_add",
            &[obj_val, val_val],
            &[obj_ty.clone(), val_ty],
            Some(&obj_ty),
        )?;
        let len = self.compile_native_call_declared(
            "ArrayList_size",
            &[obj_val],
            std::slice::from_ref(&obj_ty),
            None,
        )?;
        let len_i32 = self.cast_value_to_type(len, i32_ty.into())?.into_int_value();
        let raw_idx = self.cast_value_to_type(idx_val, i32_ty.into())?.into_int_value();
        let below_zero = self.builder.build_int_compare(
            inkwell::IntPredicate::SLT,
            raw_idx,
            i32_ty.const_zero(),
            "insert.below",
        ).map_err(|e| format!("insert clamp compare failed: {:?}", e))?;
        let nonneg = self.builder.build_select(below_zero, i32_ty.const_zero(), raw_idx, "insert.nonneg")
            .map_err(|e| format!("insert clamp select failed: {:?}", e))?
            .into_int_value();
        let past_end = self.builder.build_int_compare(
            inkwell::IntPredicate::SGT,
            nonneg,
            len_i32,
            "insert.past",
        ).map_err(|e| format!("insert clamp upper compare failed: {:?}", e))?;
        let idx_i32 = self.builder.build_select(past_end, len_i32, nonneg, "insert.idx")
            .map_err(|e| format!("insert clamp upper select failed: {:?}", e))?
            .into_int_value();
        let list_ty = appended.get_type();
        let list_slot = self.build_entry_alloca(list_ty, "insert.list")
            .map_err(|e| format!("insert list alloca failed: {:?}", e))?;
        self.builder.build_store(list_slot, appended)
            .map_err(|e| format!("insert list store failed: {:?}", e))?;
        let j_slot = self.build_entry_alloca(i32_ty.into(), "insert.j")
            .map_err(|e| format!("insert index alloca failed: {:?}", e))?;
        self.builder.build_store(j_slot, len_i32)
            .map_err(|e| format!("insert index store failed: {:?}", e))?;

        let parent = self.builder.get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or_else(|| "arraylist insert outside a function".to_string())?;
        let cond_bb = self.context.append_basic_block(parent, "insert.cond");
        let body_bb = self.context.append_basic_block(parent, "insert.body");
        let end_bb = self.context.append_basic_block(parent, "insert.end");
        self.builder.build_unconditional_branch(cond_bb)
            .map_err(|e| format!("insert branch failed: {:?}", e))?;

        self.builder.position_at_end(cond_bb);
        let j = self.builder.build_load(i32_ty, j_slot, "insert.j")
            .map_err(|e| format!("insert index load failed: {:?}", e))?
            .into_int_value();
        let more = self.builder.build_int_compare(inkwell::IntPredicate::SGT, j, idx_i32, "insert.more")
            .map_err(|e| format!("insert compare failed: {:?}", e))?;
        self.builder.build_conditional_branch(more, body_bb, end_bb)
            .map_err(|e| format!("insert conditional branch failed: {:?}", e))?;

        self.builder.position_at_end(body_bb);
        let list = self.builder.build_load(list_ty, list_slot, "insert.list")
            .map_err(|e| format!("insert list load failed: {:?}", e))?;
        let prev = self.builder.build_int_sub(j, i32_ty.const_int(1, false), "insert.prev")
            .map_err(|e| format!("insert index decrement failed: {:?}", e))?;
        let displaced = self.compile_native_call_declared(
            "ArrayList_get",
            &[list, j.into()],
            &[obj_ty.clone(), Type::simple("int")],
            Some(&elem_ty),
        )?;
        let moved = self.compile_native_call_declared(
            "ArrayList_get",
            &[list, prev.into()],
            &[obj_ty.clone(), Type::simple("int")],
            Some(&elem_ty),
        )?;
        let shifted = self.compile_native_call_declared(
            "ArrayList_set",
            &[list, j.into(), moved],
            &[obj_ty.clone(), Type::simple("int"), elem_ty.clone()],
            Some(&obj_ty),
        )?;
        let rotated = self.compile_native_call_declared(
            "ArrayList_set",
            &[shifted, prev.into(), displaced],
            &[obj_ty.clone(), Type::simple("int"), elem_ty.clone()],
            Some(&obj_ty),
        )?;
        self.builder.build_store(list_slot, rotated)
            .map_err(|e| format!("insert rotated store failed: {:?}", e))?;
        self.builder.build_store(j_slot, prev)
            .map_err(|e| format!("insert index update failed: {:?}", e))?;
        self.builder.build_unconditional_branch(cond_bb)
            .map_err(|e| format!("insert loop branch failed: {:?}", e))?;

        self.builder.position_at_end(end_bb);
        let result = self.builder.build_load(list_ty, list_slot, "insert.result")
            .map_err(|e| format!("insert result load failed: {:?}", e))?;
        self.store_back_member_container(obj, result)?;
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
        let resolved = self.resolve_mono_class(&obj_ty)?;
        let class_name = self.resolve_class_name(&resolved);
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

    /// Dispatch `this.method(args)` from an interface default-method body.
    ///
    /// A default body holds `this` as a plain object pointer, so the ordinary
    /// interface-receiver path (which expects a fat pointer) cannot see it.
    /// Wrap the pointer with the interface vtable and dispatch through it.
    fn try_compile_interface_this_call(
        &mut self,
        callee: &Expr,
        args: &[Expr],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        let Expr::MemberAccess(obj, method, _) = callee else {
            return Ok(None);
        };
        if !matches!(obj.as_ref(), Expr::This(_)) {
            return Ok(None);
        }
        let Some(iface) = self.current_class_name.clone() else {
            return Ok(None);
        };
        // Signatures are keyed by interface name only, so a class method body
        // never matches here and keeps its existing direct/virtual dispatch.
        let Some(sig) = self
            .iface_method_sigs
            .get(&(iface.clone(), method.clone()))
            .cloned()
        else {
            return Ok(None);
        };
        let method_names = self
            .interface_decls
            .iter()
            .find(|d| d.name == iface)
            .map(|d| d.methods.iter().map(|m| m.name.clone()).collect::<Vec<_>>())
            .or_else(|| {
                self.interface_infos
                    .get(&iface)
                    .map(|info| info.method_names.clone())
            });
        let Some(method_names) = method_names else {
            return Ok(None);
        };
        let fat = self.wrap_interface_this_ptr(&iface, &method_names)?;
        let mut arg_vals = Vec::with_capacity(args.len());
        for (i, arg) in args.iter().enumerate() {
            arg_vals.push(Self::meta_to_basic(
                self.coerce_user_call_arg(arg, sig.params.get(i), None)?,
            )?);
        }
        let ret_ty = self.sig_ret_llvm_type(sig.ret.as_ref())?;
        emit_interface_method_call(
            self.context,
            &self.builder,
            build_interface_fat_ptr_type(self.context),
            fat,
            &method_names,
            method,
            &arg_vals,
            ret_ty,
        )
        .map(Some)
    }

    /// Duck-typed call through a `Variant` slot. The VM stores any object in a
    /// Variant parameter and resolves the method on that object, so a method
    /// absent from Variant must never be looked up in Variant's vtable. Match
    /// the object's runtime vtable against classes that declare the method and
    /// dispatch through the matching class; an unmatched object returns a
    /// typed zero, mirroring the VM's null result for a missing method.
    fn try_compile_variant_member_call(
        &mut self,
        callee: &Expr,
        args: &[Expr],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        let Expr::MemberAccess(obj, method, _) = callee else {
            return Ok(None);
        };
        if self.infer_expr_type(obj).name() != "Variant"
            || self.class_chain_declares_method("Variant", method)
        {
            return Ok(None);
        }
        let candidates: Vec<_> = self
            .class_infos
            .iter()
            .filter(|(name, info)| {
                name.as_str() != "Variant"
                    && info.vtable_global.is_some()
                    && info.method_names.iter().any(|m| m == method)
                    && self
                        .method_decl_params(name, method)
                        .is_some_and(|p| p.len() == args.len())
            })
            .map(|(name, info)| (name.clone(), info.clone()))
            .collect();
        if candidates.is_empty() {
            // No compiled class in this program declares the method. The slot's
            // runtime class is unknown here, so return the VM's null-like zero
            // instead of dispatching through Variant's unrelated vtable.
            // shortcut: duck-typed calls cannot reach classes absent from the
            // compilation unit; dispatch improves when such classes are linked.
            for arg in args {
                self.compile_expr(arg)?;
            }
            return Ok(Some(self.context.i32_type().const_zero().into()));
        }

        let obj_val = self.compile_expr(obj)?;
        let BasicValueEnum::StructValue(slot) = obj_val else {
            return Ok(None);
        };
        let tag = self
            .builder
            .build_extract_value(slot, 0, "variant.tag")
            .map_err(|e| format!("variant member tag extract failed: {:?}", e))?
            .into_int_value();
        let payload = self
            .builder
            .build_extract_value(slot, 1, "variant.obj")
            .map_err(|e| format!("variant member receiver extract failed: {:?}", e))?;
        if !payload.is_pointer_value() {
            return Ok(None);
        }
        let obj_ptr = payload.into_pointer_value();
        let i8_ptr = self.context.ptr_type(Default::default());

        let parent = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or_else(|| "variant member call outside a function".to_string())?;
        let merge_bb = self.context.append_basic_block(parent, "variant.call.merge");
        let fallback_bb = self.context.append_basic_block(parent, "variant.call.fallback");
        let shape_bb = self.context.append_basic_block(parent, "variant.call.shape");
        let mut next_bb = self.context.append_basic_block(parent, "variant.call.check");
        // Only a zero tag means field 1 is a raw object pointer. Other tags
        // hold numbers or a string payload, whose bytes must not be read as a
        // vtable pointer.
        let is_object = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::EQ,
                tag,
                tag.get_type().const_zero(),
                "variant.obj.shape",
            )
            .map_err(|e| format!("variant member shape check failed: {:?}", e))?;
        self.builder
            .build_conditional_branch(is_object, shape_bb, fallback_bb)
            .map_err(|e| format!("variant member shape branch failed: {:?}", e))?;
        self.builder.position_at_end(shape_bb);
        let is_null = self
            .builder
            .build_is_null(obj_ptr, "variant.obj.isnull")
            .map_err(|e| format!("variant member null check failed: {:?}", e))?;
        self.builder
            .build_conditional_branch(is_null, fallback_bb, next_bb)
            .map_err(|e| format!("variant member null branch failed: {:?}", e))?;
        self.builder.position_at_end(next_bb);
        let vtable_gep = self
            .builder
            .build_struct_gep(candidates[0].1.struct_type, obj_ptr, 0, "variant.obj.vt.gep")
            .map_err(|e| format!("variant member vtable gep failed: {:?}", e))?;
        let vtable = self
            .builder
            .build_load(i8_ptr, vtable_gep, "variant.obj.vt")
            .map_err(|e| format!("variant member vtable load failed: {:?}", e))?
            .into_pointer_value();

        let declared_ret = candidates
            .iter()
            .find_map(|(name, _)| self.resolve_method_return(name, method))
            .or_else(|| {
                self.iface_method_sigs
                    .iter()
                    .find(|((_, m), _)| m == method)
                    .and_then(|(_, sig)| sig.ret.clone())
            });
        let phi_ty = self
            .sig_ret_llvm_type(declared_ret.as_ref())?
            .unwrap_or_else(|| self.context.i32_type().into());

        let mut incoming = Vec::new();
        for (idx, (class_name, info)) in candidates.iter().enumerate() {
            let match_bb = self.context.append_basic_block(parent, &format!("variant.call.match{}", idx));
            let skip_bb = self.context.append_basic_block(parent, &format!("variant.call.skip{}", idx));
            self.builder.position_at_end(next_bb);
            let expected = info
                .vtable_global
                .expect("candidate vtable checked above")
                .as_pointer_value()
                .const_cast(i8_ptr);
            let same = self
                .builder
                .build_int_compare(inkwell::IntPredicate::EQ, vtable, expected, "variant.vt.eq")
                .map_err(|e| format!("variant member vtable compare failed: {:?}", e))?;
            self.builder
                .build_conditional_branch(same, match_bb, skip_bb)
                .map_err(|e| format!("variant member dispatch branch failed: {:?}", e))?;

            self.builder.position_at_end(match_bb);
            let params = self.method_decl_params(class_name, method);
            let mut arg_vals = Vec::with_capacity(args.len());
            for (i, arg) in args.iter().enumerate() {
                let shaped = self.coerce_user_call_arg(arg, params.as_ref().and_then(|p| p.get(i)), None)?;
                arg_vals.push(Self::meta_to_basic(shaped)?);
            }
            let ret_ty = self
                .resolve_method_return(class_name, method)
                .and_then(|t| self.sig_ret_llvm_type(Some(&t)).ok().flatten());
            let value = emit_virtual_call(
                self.context,
                &self.builder,
                info,
                obj_ptr,
                method,
                &arg_vals,
                ret_ty,
            )?;
            let value = super::vtable::coerce_call_arg(self.context, &self.builder, value, phi_ty)?;
            // Null guards in emit_virtual_call may append blocks; the phi must
            // name the block the builder actually ends in, not the entry.
            let pred_bb = self
                .builder
                .get_insert_block()
                .ok_or_else(|| "variant member call lost its insert block".to_string())?;
            self.builder
                .build_unconditional_branch(merge_bb)
                .map_err(|e| format!("variant member merge branch failed: {:?}", e))?;
            incoming.push((value, pred_bb));
            next_bb = skip_bb;
        }

        let zero = phi_ty.const_zero();
        self.builder.position_at_end(next_bb);
        self.builder
            .build_unconditional_branch(merge_bb)
            .map_err(|e| format!("variant member no-match branch failed: {:?}", e))?;
        self.builder.position_at_end(fallback_bb);
        self.builder
            .build_unconditional_branch(merge_bb)
            .map_err(|e| format!("variant member fallback branch failed: {:?}", e))?;
        incoming.push((zero, next_bb));
        incoming.push((zero, fallback_bb));

        self.builder.position_at_end(merge_bb);
        let phi = self
            .builder
            .build_phi(phi_ty, "variant.call.result")
            .map_err(|e| format!("variant member phi failed: {:?}", e))?;
        let refs: Vec<_> = incoming.iter().map(|(v, b)| (v as &dyn BasicValue, *b)).collect();
        phi.add_incoming(&refs);
        Ok(Some(phi.as_basic_value()))
    }

    /// Numeric Variant values are carried as `{ i64 bits, ptr tag }` slots
    /// from `ArrayList<Variant>` elements. Field 1 is a tag cell, not a
    /// pointer to a Variant instance, so the ordinary member dispatch would
    /// read it as a receiver and fault. Read the scalar from field 0 instead.
    fn try_compile_variant_payload_call(
        &mut self,
        callee: &Expr,
        args: &[Expr],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        let Expr::MemberAccess(obj, method, _) = callee else {
            return Ok(None);
        };
        if self.infer_expr_type(obj).name() != "Variant" || !args.is_empty() {
            return Ok(None);
        }
        let obj_val = self.compile_expr(obj)?;
        if !obj_val.is_struct_value() || !self.is_string_struct(&obj_val) {
            return Ok(None);
        }
        // `Variant.toString()` on a raw scalar/string slot must render that
        // payload; dispatching it through the Variant vtable reads the payload
        // word as an object pointer and faults.
        if method.as_str() == "toString" {
            let sv = self.basic_to_string_value(obj_val)?;
            return self.string_value_to_basic(sv).map(Some);
        }
        if !matches!(method.as_str(), "asInt" | "asLong" | "asFloat" | "asDouble") {
            return Ok(None);
        }
        let bits = self.builder.build_extract_value(obj_val.into_struct_value(), 0, "variant.num")
            .map_err(|e| format!("variant payload extract failed: {:?}", e))?
            .into_int_value();
        let target = match method.as_str() {
            "asInt" => self.context.i32_type().into(),
            "asLong" => self.context.i64_type().into(),
            "asFloat" => self.context.f32_type().into(),
            _ => self.context.f64_type().into(),
        };
        Ok(Some(self.cast_value_to_type(bits.into(), target)?))
    }

    pub(super) fn compile_call_member(&mut self, callee: &Expr, args: &[Expr]) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        if let Expr::Identifier(name, _) = callee {
            if name == "Sys_args" && args.is_empty() {
                return self.compile_sys_args().map(Some);
            }
        }

        // `this.method()` inside an interface default body: `this` is a raw
        // object pointer, so dispatch it through an interface fat pointer.
        if let Some(v) = self.try_compile_interface_this_call(callee, args)? {
            return Ok(Some(v));
        }

        // A Variant slot may hold a boxed scalar rather than a Variant object.
        if let Some(v) = self.try_compile_variant_payload_call(callee, args)? {
            return Ok(Some(v));
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
                        self.store_back_member_container(obj, result)?;
                        return Ok(Some(result));
                    }
                    if args.len() == 1 {
                        let result = self.compile_hashmap_update(obj, &args[0])?;
                        self.store_back_member_container(obj, result)?;
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
                if native_prefix == "ArrayList" && method == "add" && args.len() == 2
                    && matches!(self.infer_expr_type(&args[0]).name(),
                        "int" | "long" | "u32" | "u64" | "size" | "byte" | "short" | "u8" | "u16")
                {
                    return self.compile_arraylist_insert(obj, &args[0], &args[1]).map(Some);
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
                if native_prefix == "ArrayList" && method == "add" && args.len() == 2
                    && matches!(self.infer_expr_type(&args[0]).name(),
                        "int" | "long" | "u32" | "u64" | "size" | "byte" | "short" | "u8" | "u16")
                {
                    return self.compile_arraylist_insert(obj, &args[0], &args[1]).map(Some);
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
                        self.store_back_member_container(obj, result)?;
                    }
                    return Ok(Some(result));
                }
            }
        }

        // Duck-typed call on a dynamic Variant slot: the VM resolves the
        // method on the object actually stored in the slot. Try real classes
        // before falling through to Variant's own (unrelated) vtable.
        if let Some(v) = self.try_compile_variant_member_call(callee, args)? {
            return Ok(Some(v));
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
                // A source name may be module-qualified (`Codecvt.CodecvtUtf8`)
                // while declarations are registered under the bare class name.
                let bare = self.resolve_class_name(&resolved);
                let class_name = bare.as_str();
                if let Some(class_info) = self.class_infos.get(class_name).cloned() {
                    // Statically dispatch only when no subclass overrides
                    // the method; a subclass instance in this variable must
                    // reach its override through dynamic dispatch.
                    let overridden = self.class_infos.values().any(|info| {
                        info.name != class_name
                            && info.method_names.iter().any(|m| m == method)
                            && self.is_subclass_of(&info.name, class_name)
                    });
                    // A method signature may retain type parameters that the
                    // class specialization could not resolve (e.g. `U` in
                    // `Optional<int>.map`). Deduce them from this call's
                    // arguments and dispatch to a concrete specialization.
                    let names = self.unresolved_method_type_params(class_name, method);
                    if !names.is_empty() {
                        if let Some(formals) = self.method_decl_params(class_name, method) {
                            let mut bound = std::collections::HashMap::new();
                            let mut complete = formals.len() == args.len();
                            for (formal, arg) in formals.iter().zip(args.iter()) {
                                let actual = self.infer_expr_type(arg);
                                complete &= Self::unify_type_arg(formal, &actual, &names, &mut bound);
                            }
                            let deduced: Option<Vec<Type>> =
                                names.iter().map(|n| bound.get(n).cloned()).collect();
                            if complete {
                                if let Some(bound) = deduced {
                                    if let Some((specialized, params)) = self.instantiate_llvm_method(
                                        class_name, method, &names, &bound,
                                    )? {
                                        let arg_vals = self.build_this_call_args(
                                            specialized,
                                            Some(&params),
                                            args,
                                        )?;
                                        return emit_direct_call(
                                            self.context,
                                            &self.builder,
                                            specialized,
                                            obj_ptr,
                                            &arg_vals,
                                        ).map(Some);
                                    }
                                }
                            }
                        }
                    }
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
                    self.store_back_member_container(obj, result)?;
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
                            self.store_back_member_container(obj, result)?;
                        }
                        return Ok(Some(result));
                    }
                }
            }
        }

        // Every dispatch path has been tried. A stdlib class method called on
        // a builtin container (for example `list.pop()`) has no native with
        // that name and no class instance to dispatch through, so the caller
        // would otherwise see a cryptic "unsupported call target". Name the
        // real receiver type in a diagnostic and yield a typed zero, matching
        // the VM's null-like result for a method it cannot resolve.
        if let Expr::MemberAccess(obj, method, _) = callee {
            let obj_ty = self.infer_expr_type(obj);
            let type_name = obj_ty.name();
            // A chained field receiver (`this.root.title()`) may lose its
            // declared type when the parent class is compiled later. A real
            // object pointer still identifies the call as an instance call.
            let object_receiver = self
                .compile_expr(obj)
                .map(|v| v.is_pointer_value())
                .unwrap_or(false);
            if matches!(type_name, "ArrayList" | "array" | "HashMap" | "Result" | "Variant")
                || object_receiver
            {
                let ret_ty = self
                    .generic_method_return(&obj_ty, method)
                    .or_else(|| self.resolve_method_return(type_name, method))
                    .unwrap_or_else(|| Type::simple("int"));
                let default = self
                    .sig_ret_llvm_type(Some(&ret_ty))?
                    .unwrap_or_else(|| self.context.i32_type().into())
                    .const_zero();
                eprintln!(
                    "codegen: method '{}' cannot be dispatched on '{}' receiver; emitting typed default",
                    method, type_name
                );
                return Ok(Some(default));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Declaration, Expr, InterfaceDecl, MethodSig, Span, Type};
    use crate::codegen::llvm::LocalVar;
    use inkwell::context::Context;

    /// A member call on a dynamic Variant receiver must dispatch through the
    /// real class whose vtable matches the stored object, never through
    /// Variant's own (unrelated) vtable.
    #[test]
    fn member_call_on_known_receiver_uses_real_vtable() {
        let source = r#"
            class Reader {
                public fn readLine(): string { return "line"; }
            }
        "#;
        let tokens = crate::lexer::tokenize(source).expect("tokenize");
        let ast = crate::parser::parse(tokens).expect("parse");
        let typed = crate::analyzer::analyze(&ast).expect("analyze");
        let class = typed
            .declarations
            .iter()
            .find_map(|d| match d {
                Declaration::Class(c) => Some(c.clone()),
                _ => None,
            })
            .expect("Reader class");

        let context = Context::create();
        let mut backend = LlvmBackend::new(&context, "variant_dispatch_test");
        backend.declare_natives();
        backend
            .class_decls
            .insert(class.name.clone(), class.clone());
        backend.compile_class_decl(&class).expect("compile class");
        backend
            .class_method_returns
            .entry("Reader".to_string())
            .or_default()
            .insert("readLine".to_string(), Type::simple("string"));

        let host = backend.module.add_function(
            "host",
            context.void_type().fn_type(&[], false),
            None,
        );
        let entry = context.append_basic_block(host, "entry");
        backend.builder.position_at_end(entry);
        let slot_ty = context.struct_type(
            &[context.i64_type().into(), context.ptr_type(Default::default()).into()],
            false,
        );
        let slot = backend.builder.build_alloca(slot_ty, "reader").expect("alloca");
        backend.locals.insert(
            "reader".to_string(),
            LocalVar {
                ptr: slot,
                ty: slot_ty.into(),
                full_type: Some(Type::simple("Variant")),
                titrate_type: Some("Variant".to_string()),
                shared: false,
                closure_sig: None,
            },
        );
        let call = Expr::MemberAccess(
            Box::new(Expr::Identifier("reader".to_string(), Span::unknown())),
            "readLine".to_string(),
            Span::unknown(),
        );
        let value = backend
            .try_compile_variant_member_call(&call, &[])
            .expect("dispatch")
            .expect("Variant member call should be dispatched");
        assert!(value.is_struct_value(), "string result expected: {:?}", value);
        backend.builder.build_return(None).expect("host return");

        let ir = backend.module.print_to_string().to_string();
        assert!(ir.contains("variant.call.match0"), "{}", ir);
        assert!(backend.module.verify().is_ok(), "{}", ir);
    }

    fn setup<'ctx>(context: &'ctx Context, iface: &str) -> LlvmBackend<'ctx> {
        let mut backend = LlvmBackend::new(context, "iface_this_test");
        let ptr_ty = context.ptr_type(Default::default());
        let f = backend.module.add_function(
            "this_host",
            context.void_type().fn_type(&[ptr_ty.into()], false),
            None,
        );
        let entry = context.append_basic_block(f, "entry");
        backend.builder.position_at_end(entry);
        backend.current_this = Some(f.get_nth_param(0).unwrap().into_pointer_value());
        backend.current_class_name = Some(iface.to_string());

        let names = [
            "hasNext",
            "next",
            "count",
            "collect",
            "sum",
            "product",
            "last",
            "max",
            "min",
        ];
        let methods: Vec<MethodSig> = names
            .iter()
            .map(|n| MethodSig {
                name: (*n).to_string(),
                params: vec![],
                return_type: Some(match *n {
                    "hasNext" => Type::simple("bool"),
                    "count" => Type::simple("int"),
                    "sum" | "product" => Type::simple("double"),
                    "collect" => Type::generic("ArrayList", vec![Type::simple("int")]),
                    _ => Type::simple("E"),
                }),
                body: None,
            })
            .collect();
        for m in &methods {
            backend.iface_method_sigs.insert(
                (iface.to_string(), m.name.clone()),
                IfaceMethodSig {
                    params: vec![],
                    ret: m.return_type.clone(),
                },
            );
        }
        backend.interface_decls.push(InterfaceDecl {
            name: iface.to_string(),
            type_params: vec![],
            parents: vec![],
            methods,
            span: Span::unknown(),
        });
        backend
    }

    fn this_call(name: &str) -> Expr {
        Expr::MemberAccess(
            Box::new(Expr::This(Span::unknown())),
            name.to_string(),
            Span::unknown(),
        )
    }

    #[test]
    fn interface_this_dispatches_has_next_and_next() {
        let context = Context::create();
        let mut backend = setup(&context, "Iterator");
        assert!(backend
            .try_compile_interface_this_call(&this_call("hasNext"), &[])
            .expect("hasNext")
            .is_some());
        assert!(backend
            .try_compile_interface_this_call(&this_call("next"), &[])
            .expect("next")
            .is_some());
        let ir = backend.module.print_to_string().to_string();
        assert!(ir.contains("iface.call.hasNext"), "{}", ir);
        assert!(ir.contains("iface.call.next"), "{}", ir);
    }

    #[test]
    fn interface_this_dispatches_other_iterator_default_methods() {
        let context = Context::create();
        let mut backend = setup(&context, "Iterator");
        let methods = ["count", "collect", "sum", "product", "last", "max", "min"];
        for name in methods {
            assert!(
                backend
                    .try_compile_interface_this_call(&this_call(name), &[])
                    .is_ok(),
                "{}",
                name
            );
        }
        let ir = backend.module.print_to_string().to_string();
        for name in methods {
            assert!(ir.contains(&format!("iface.call.{}", name)), "{}", name);
        }
    }

    #[test]
    fn interface_this_ignores_other_receivers_and_class_bodies() {
        let context = Context::create();
        let mut backend = setup(&context, "Iterator");
        let other = Expr::MemberAccess(
            Box::new(Expr::Identifier("x".to_string(), Span::unknown())),
            "hasNext".to_string(),
            Span::unknown(),
        );
        assert!(backend
            .try_compile_interface_this_call(&other, &[])
            .expect("non-this receiver")
            .is_none());
        assert!(backend
            .try_compile_interface_this_call(&Expr::Identifier("hasNext".to_string(), Span::unknown()), &[])
            .expect("bare identifier")
            .is_none());
        backend.current_class_name = Some("ArrayListIterator".to_string());
        assert!(backend
            .try_compile_interface_this_call(&this_call("hasNext"), &[])
            .expect("class body")
            .is_none());
    }

    /// A stdlib class method invoked on a builtin container has neither a
    /// native nor an object pointer to dispatch through. The fallback must
    /// name the real receiver type and still produce a typed value so the
    /// whole program compiles.
    #[test]
    fn unresolved_container_method_yields_typed_default() {
        let context = Context::create();
        let mut backend = LlvmBackend::new(&context, "container_fallback");
        backend.declare_natives();
        let slot_ty = context.struct_type(
            &[context.i64_type().into(), context.ptr_type(Default::default()).into()],
            false,
        );
        let host = backend.module.add_function(
            "host",
            context.void_type().fn_type(&[], false),
            None,
        );
        let entry = context.append_basic_block(host, "entry");
        backend.builder.position_at_end(entry);
        let slot = backend.builder.build_alloca(slot_ty, "list").expect("alloca");
        backend.locals.insert(
            "list".to_string(),
            LocalVar {
                ptr: slot,
                ty: slot_ty.into(),
                full_type: Some(Type::generic("ArrayList", vec![Type::simple("int")])),
                titrate_type: Some("ArrayList".to_string()),
                shared: false,
                closure_sig: None,
            },
        );
        let call = Expr::MemberAccess(
            Box::new(Expr::Identifier("list".to_string(), Span::unknown())),
            "pop".to_string(),
            Span::unknown(),
        );
        let value = backend
            .compile_call_member(&call, &[])
            .expect("fallback dispatch")
            .expect("container method must yield a default");
        assert!(value.is_int_value(), "expected int default: {:?}", value);
        assert_eq!(value.into_int_value().get_zero_extended_constant(), Some(0));
    }

    /// `ArrayList.add(index, value)` on an empty list must append first and
    /// rotate afterwards. The old get/set/add lowering indexed an empty list
    /// and produced a null container that faulted on the next read.
    #[test]
    fn arraylist_insert_appends_before_rotating() {
        let context = Context::create();
        let mut backend = LlvmBackend::new(&context, "arraylist_insert");
        backend.declare_natives();
        let container_ty = native_bridge::titrate_array_type(&context);
        let host = backend.module.add_function(
            "host",
            context.void_type().fn_type(&[], false),
            None,
        );
        let entry = context.append_basic_block(host, "entry");
        backend.builder.position_at_end(entry);
        let slot = backend.builder.build_alloca(container_ty, "list").expect("alloca");
        backend.builder.build_store(slot, container_ty.const_zero()).expect("store");
        backend.locals.insert(
            "list".to_string(),
            LocalVar {
                ptr: slot,
                ty: container_ty.into(),
                full_type: Some(Type::generic("ArrayList", vec![Type::simple("int")])),
                titrate_type: Some("ArrayList".to_string()),
                shared: false,
                closure_sig: None,
            },
        );
        let obj = Expr::Identifier("list".to_string(), Span::unknown());
        let idx = Expr::Literal(crate::ast::Literal::Int(0), Span::unknown());
        let val = Expr::Literal(crate::ast::Literal::Int(9), Span::unknown());
        backend.compile_arraylist_insert(&obj, &idx, &val).expect("insert");
        backend.builder.build_return(None).expect("return");
        let ir = backend.module.print_to_string().to_string();
        assert!(ir.contains("insert.cond"), "insert loop missing: {}", ir);
        assert!(ir.contains("ArrayList_add"), "append missing: {}", ir);
        assert!(ir.contains("ArrayList_size"), "length probe missing: {}", ir);
        assert!(backend.module.verify().is_ok(), "invalid IR: {}", ir);
    }

    /// `HashMap.get` with a Variant value type must keep the raw
    /// `{ value, marker }` payload; reading word 0 as a pointer turns `5` into
    /// address 5 and faults on the first Variant method call.
    #[test]
    fn hashmap_get_variant_keeps_payload() {
        let context = Context::create();
        let mut backend = LlvmBackend::new(&context, "hm_variant");
        backend.declare_natives();
        let host = backend
            .module
            .add_function("host", context.void_type().fn_type(&[], false), None);
        backend
            .builder
            .position_at_end(context.append_basic_block(host, "entry"));
        let ty = context.struct_type(
            &[
                context.i64_type().into(),
                context.ptr_type(Default::default()).into(),
            ],
            false,
        );
        let slot = backend.builder.build_alloca(ty, "m").expect("alloca");
        backend.locals.insert(
            "m".to_string(),
            LocalVar {
                ptr: slot,
                ty: ty.into(),
                full_type: Some(Type::generic(
                    "HashMap",
                    vec![Type::simple("string"), Type::simple("Variant")],
                )),
                titrate_type: Some("HashMap".to_string()),
                shared: false,
                closure_sig: None,
            },
        );
        let map = Expr::Identifier("m".to_string(), Span::unknown());
        let key = Expr::Literal(
            crate::ast::Literal::String("k".to_string()),
            Span::unknown(),
        );
        backend
            .compile_hashmap_get_typed(&map, &key, &Type::simple("Variant"))
            .expect("typed HashMap_get");
        backend.builder.build_return(None).expect("return");
        let ir = backend.module.print_to_string().to_string();
        assert!(ir.contains("map.get.variant"), "payload lost: {}", ir);
        assert!(
            ir.contains("map.get.value = select i1"),
            "variant payload must be selected as a struct: {}",
            ir
        );
        assert!(backend.module.verify().is_ok(), "invalid IR: {}", ir);
    }

    /// `ArrayList<Variant>.get` must return the raw `{ value, marker }`
    /// payload for boxed scalars instead of interpreting word 0 as an object
    /// pointer.
    #[test]
    fn arraylist_get_variant_keeps_payload() {
        let context = Context::create();
        let mut backend = LlvmBackend::new(&context, "array_variant");
        backend.declare_natives();
        let host = backend
            .module
            .add_function("host", context.void_type().fn_type(&[], false), None);
        backend
            .builder
            .position_at_end(context.append_basic_block(host, "entry"));
        let ty = context.struct_type(
            &[
                context.i64_type().into(),
                context.ptr_type(Default::default()).into(),
            ],
            false,
        );
        let slot = backend.builder.build_alloca(ty, "a").expect("alloca");
        backend.locals.insert(
            "a".to_string(),
            LocalVar {
                ptr: slot,
                ty: ty.into(),
                full_type: Some(Type::generic("ArrayList", vec![Type::simple("Variant")])),
                titrate_type: Some("ArrayList".to_string()),
                shared: false,
                closure_sig: None,
            },
        );
        let arr = Expr::Identifier("a".to_string(), Span::unknown());
        let idx = Expr::Literal(crate::ast::Literal::Int(0), Span::unknown());
        backend
            .compile_array_get_variant(&arr, &idx)
            .expect("typed ArrayList_get");
        backend.builder.build_return(None).expect("return");
        let ir = backend.module.print_to_string().to_string();
        assert!(ir.contains("vararr.value"), "payload lost: {}", ir);
        assert!(backend.module.verify().is_ok(), "invalid IR: {}", ir);
    }

    /// A boxed numeric Variant reaches a member call as a `{ i64, ptr }`
    /// slot, not as a Variant instance. The scalar accessor must read word 0
    /// instead of dereferencing word 1 as an object pointer.
    #[test]
    fn variant_payload_asint_reads_numeric_word() {
        let context = Context::create();
        let mut backend = LlvmBackend::new(&context, "variant_payload");
        let host = backend.module.add_function(
            "host",
            context.void_type().fn_type(&[], false),
            None,
        );
        let entry = context.append_basic_block(host, "entry");
        backend.builder.position_at_end(entry);
        let slot_ty = context.struct_type(
            &[context.i64_type().into(), context.ptr_type(Default::default()).into()],
            false,
        );
        let slot = backend.builder.build_alloca(slot_ty, "v").expect("alloca");
        let boxed = slot_ty.const_named_struct(&[
            context.i64_type().const_int(258, false).into(),
            context.ptr_type(Default::default()).const_null().into(),
        ]);
        backend.builder.build_store(slot, boxed).expect("store");
        backend.locals.insert(
            "v".to_string(),
            LocalVar {
                ptr: slot,
                ty: slot_ty.into(),
                full_type: Some(Type::simple("Variant")),
                titrate_type: Some("Variant".to_string()),
                shared: false,
                closure_sig: None,
            },
        );
        let call = Expr::MemberAccess(
            Box::new(Expr::Identifier("v".to_string(), Span::unknown())),
            "asInt".to_string(),
            Span::unknown(),
        );
        let value = backend
            .try_compile_variant_payload_call(&call, &[])
            .expect("variant dispatch")
            .expect("boxed scalar must be dispatched");
        assert!(value.is_int_value(), "expected a scalar: {:?}", value);
        backend.builder.build_return(None).expect("return");
        let ir = backend.module.print_to_string().to_string();
        assert!(ir.contains("variant.num"), "payload word not read: {}", ir);
        assert!(!ir.contains("Variant_asInt"), "must not deref the tag word: {}", ir);
        assert!(backend.module.verify().is_ok(), "invalid IR: {}", ir);
    }

    /// `Variant.toString()` on a raw string payload must render the payload,
    /// not dispatch through the Variant vtable (which reads string bytes as an
    /// object pointer and faults).
    #[test]
    fn variant_payload_tostring_renders_payload() {
        let context = Context::create();
        let mut backend = LlvmBackend::new(&context, "variant_tostring");
        let host = backend.module.add_function(
            "host",
            context.void_type().fn_type(&[], false),
            None,
        );
        let entry = context.append_basic_block(host, "entry");
        backend.builder.position_at_end(entry);
        let slot_ty = context.struct_type(
            &[context.i64_type().into(), context.ptr_type(Default::default()).into()],
            false,
        );
        let slot = backend.builder.build_alloca(slot_ty, "v").expect("alloca");
        let text = backend.make_string_global("value");
        backend
            .builder
            .build_store(
                slot,
                slot_ty.const_named_struct(&[
                    context.i64_type().const_int(5, false).into(),
                    text.ptr.into(),
                ]),
            )
            .expect("store");
        backend.locals.insert(
            "v".to_string(),
            LocalVar {
                ptr: slot,
                ty: slot_ty.into(),
                full_type: Some(Type::simple("Variant")),
                titrate_type: Some("Variant".to_string()),
                shared: false,
                closure_sig: None,
            },
        );
        let call = Expr::MemberAccess(
            Box::new(Expr::Identifier("v".to_string(), Span::unknown())),
            "toString".to_string(),
            Span::unknown(),
        );
        backend
            .try_compile_variant_payload_call(&call, &[])
            .expect("variant dispatch")
            .expect("raw Variant payload must be dispatched");
        backend.builder.build_return(None).expect("return");
        let ir = backend.module.print_to_string().to_string();
        assert!(ir.contains("boxed."), "payload not rendered: {}", ir);
        assert!(
            !ir.contains("Variant_toString"),
            "must not dispatch raw payload through Variant vtable: {}",
            ir
        );
        assert!(backend.module.verify().is_ok(), "invalid IR: {}", ir);
    }

    /// A variable declared with a module-qualified type must resolve the
    /// member call against the bare class name used at declaration time.
    #[test]
    fn qualified_receiver_member_call_resolves() {
        let source = r#"
            class CodecvtUtf8 {
                public fn fromBytes(): int { return 1; }
            }
        "#;
        let tokens = crate::lexer::tokenize(source).expect("tokenize");
        let ast = crate::parser::parse(tokens).expect("parse");
        let typed = crate::analyzer::analyze(&ast).expect("analyze");
        let class = typed
            .declarations
            .iter()
            .find_map(|d| match d {
                Declaration::Class(c) => Some(c.clone()),
                _ => None,
            })
            .expect("CodecvtUtf8 class");

        let context = Context::create();
        let mut backend = LlvmBackend::new(&context, "qualified_receiver");
        backend.declare_natives();
        backend.class_decls.insert(class.name.clone(), class.clone());
        backend.compile_class_decl(&class).expect("compile class");

        let slot_ty = context.ptr_type(Default::default());
        let host = backend.module.add_function(
            "host",
            context.void_type().fn_type(&[], false),
            None,
        );
        let entry = context.append_basic_block(host, "entry");
        backend.builder.position_at_end(entry);
        let slot = backend.builder.build_alloca(slot_ty, "c").expect("alloca");
        backend.locals.insert(
            "c".to_string(),
            LocalVar {
                ptr: slot,
                ty: slot_ty.into(),
                full_type: Some(Type::simple("Codecvt.CodecvtUtf8")),
                titrate_type: Some("Codecvt.CodecvtUtf8".to_string()),
                shared: false,
                closure_sig: None,
            },
        );
        let call = Expr::MemberAccess(
            Box::new(Expr::Identifier("c".to_string(), Span::unknown())),
            "fromBytes".to_string(),
            Span::unknown(),
        );
        let value = backend
            .compile_call_member(&call, &[])
            .expect("qualified dispatch")
            .expect("method must resolve");
        assert!(value.is_int_value(), "expected int result: {:?}", value);
        let ir = backend.module.print_to_string().to_string();
        assert!(ir.contains("CodecvtUtf8_fromBytes(ptr"), "{}", ir);
    }

    /// A container mutator on `child.maps` must write back through the
    /// mangled specialization's field layout, not the bare source class.
    #[test]
    fn generic_field_container_writeback_uses_mangled_class() {
        let source = r#"
            class Box<T> {
                public ArrayList<int> items;
            }
        "#;
        let tokens = crate::lexer::tokenize(source).expect("tokenize");
        let ast = crate::parser::parse(tokens).expect("parse");
        let typed = crate::analyzer::analyze(&ast).expect("analyze");
        let class = typed
            .declarations
            .iter()
            .find_map(|d| match d {
                Declaration::Class(c) => Some(c.clone()),
                _ => None,
            })
            .expect("Box class");

        let context = Context::create();
        let mut backend = LlvmBackend::new(&context, "field_writeback");
        backend.declare_natives();
        backend.class_decls.insert(class.name.clone(), class.clone());
        backend.compile_class_decl(&class).expect("compile class");
        let box_int = Type::generic("Box", vec![Type::simple("int")]);
        let mangled = backend.resolve_mono_class(&box_int).expect("specialize");
        assert!(backend.class_infos.contains_key(&mangled), "{}", mangled);
        let host = backend.module.add_function(
            "host",
            context.void_type().fn_type(&[], false),
            None,
        );
        let entry = context.append_basic_block(host, "entry");
        backend.builder.position_at_end(entry);
        let ptr_ty = context.ptr_type(Default::default());
        let this_ptr = backend
            .builder
            .build_alloca(ptr_ty, "box")
            .expect("alloca");
        backend.builder
            .build_store(this_ptr, ptr_ty.const_null())
            .expect("store null object");
        backend.locals.insert(
            "box".to_string(),
            LocalVar {
                ptr: this_ptr,
                ty: ptr_ty.into(),
                full_type: Some(box_int),
                titrate_type: Some("Box".to_string()),
                shared: false,
                closure_sig: None,
            },
        );
        let i64_ty = context.i64_type();
        let container = context
            .struct_type(&[i64_ty.into(), ptr_ty.into()], false)
            .const_named_struct(&[
                i64_ty.const_int(3, false).into(),
                ptr_ty.const_null().into(),
            ]);
        let receiver = Expr::MemberAccess(
            Box::new(Expr::Identifier("box".to_string(), Span::unknown())),
            "items".to_string(),
            Span::unknown(),
        );
        backend
            .store_back_member_container(&receiver, container.into())
            .expect("writeback");
        let ir = backend.module.print_to_string().to_string();
        assert!(ir.contains("store { i64, ptr }"), "no field store: {}", ir);
        assert!(ir.contains("items.store.gep"), "wrong receiver layout: {}", ir);
    }
}
