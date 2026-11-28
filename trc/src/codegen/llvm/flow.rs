// Loops and switch lowering

use super::types as llvm_types;
use super::{LlvmBackend, LocalVar, LoopContext};
use crate::ast::{Expr, Literal, Type};
use inkwell::module::Linkage;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicValueEnum, IntValue, PointerValue};
use inkwell::AddressSpace;

impl<'ctx> LlvmBackend<'ctx> {
    /// Compile `for (item in collection)` over an `ArrayList`/`array` value.
    /// Iterates by index, reading each element with the element-aware
    /// `titrate_array_get_*` accessor so numeric elements are not corrupted.
    pub(super) fn compile_for_collection(
        &mut self,
        for_stmt: &crate::ast::ForStmt,
        iterable: &Expr,
    ) -> Result<(), String> {
        let iterable_ty = self.infer_expr_type(iterable);
        let is_string = matches!(iterable_ty.name(), "string" | "String");
        let mut arr_val = self.compile_expr(iterable)?;
        // Iterating a HashMap yields its keys (matching the VM). The raw
        // container stores [key, value] pairs, so ask the native for keys.
        if iterable_ty.name() == "HashMap" {
            arr_val = self.compile_native_call_declared(
                "HashMap_keys",
                &[arr_val],
                std::slice::from_ref(&iterable_ty),
                Some(&Type::simple("array")),
            )?;
        }
        if !arr_val.is_struct_value() {
            return Err(format!(
                "codegen: for-in iterable must be an ArrayList/array/string, got {:?}",
                iterable
            ));
        }
        let i64_ty = self.context.i64_type();
        // A string iterates Unicode characters, so use String_length (character
        // count) rather than the byte length stored in the string struct.
        let len_val = if is_string {
            let n = self.compile_native_call_declared(
                "String_length",
                &[arr_val],
                std::slice::from_ref(&iterable_ty),
                Some(&Type::simple("int")),
            )?;
            let n = n.into_int_value();
            if n.get_type() == i64_ty {
                n
            } else {
                self.builder
                    .build_int_s_extend(n, i64_ty, "for.str.len")
                    .map_err(|e| format!("for-in String_length extend failed: {:?}", e))?
            }
        } else {
            let sv = arr_val.into_struct_value();
            self.builder
                .build_extract_value(sv, 0, "for.len")
                .map_err(|e| format!("extract for-in len failed: {:?}", e))?
                .into_int_value()
        };

        let elem_name = if is_string {
            "string".to_string()
        } else {
            self.array_element_type_name(iterable)
                .unwrap_or_else(|| "string".to_string())
        };
        let elem_ty = llvm_types::llvm_type(self.context, &Type::simple(&elem_name))
            .map_err(|e| format!("for-in element type: {}", e))?;
        let accessor = if elem_ty.is_pointer_type() {
            // Class instances and enum values are stored as raw pointers in
            // the element payload; the string accessor would return an empty
            // `{i64, ptr}` struct that overflows this pointer-typed slot.
            "titrate_array_get_ptr"
        } else {
            match elem_name.as_str() {
                "int" | "u32" | "byte" | "short" | "u8" | "u16" | "bool" | "char" => {
                    "titrate_array_get_int"
                }
                "long" | "u64" | "size" => "titrate_array_get_long",
                "double" | "float" | "half" | "quad" => "titrate_array_get_double",
                _ => "titrate_array_get_string",
            }
        };

        // Allocate the loop counter (i64) and the loop variable.
        let counter_alloca = self
            .builder
            .build_alloca(i64_ty, "for.i")
            .map_err(|e| format!("build_alloca for.i failed: {:?}", e))?;
        self.builder
            .build_store(counter_alloca, i64_ty.const_int(0, false))
            .map_err(|e| format!("store for.i failed: {:?}", e))?;
        let loop_var_alloca = self
            .builder
            .build_alloca(elem_ty, &for_stmt.var)
            .map_err(|e| format!("build_alloca '{}' failed: {:?}", for_stmt.var, e))?;

        let current_block = self
            .builder
            .get_insert_block()
            .ok_or("codegen: no insert block for for-in")?;
        let cond_block = self
            .context
            .insert_basic_block_after(current_block, "for.cond");
        let body_block = self
            .context
            .insert_basic_block_after(cond_block, "for.body");
        let inc_block = self.context.insert_basic_block_after(body_block, "for.inc");
        let end_block = self.context.insert_basic_block_after(inc_block, "for.end");

        self.builder
            .build_unconditional_branch(cond_block)
            .map_err(|e| format!("build_br for.cond failed: {:?}", e))?;

        self.builder.position_at_end(cond_block);
        let counter_val = self
            .builder
            .build_load(i64_ty, counter_alloca, "for.i.val")
            .map_err(|e| format!("load for.i failed: {:?}", e))?
            .into_int_value();
        let cmp = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::SLT,
                counter_val,
                len_val,
                "for.cmp",
            )
            .map_err(|e| format!("for-in cmp failed: {:?}", e))?;
        self.builder
            .build_conditional_branch(cmp, body_block, end_block)
            .map_err(|e| format!("for-in cond br failed: {:?}", e))?;

        self.builder.position_at_end(body_block);
        let elem_raw = if is_string {
            // String_charAt returns a one-character string, matching
            // String.charAt and the VM's per-character iteration.
            let idx: BasicValueEnum = counter_val.into();
            self.compile_native_call_declared(
                "String_charAt",
                &[arr_val, idx],
                &[iterable_ty.clone(), Type::simple("int")],
                Some(&Type::simple("string")),
            )?
        } else {
            // Load the current element via the element-aware accessor.
            let i8_ptr = self.context.ptr_type(AddressSpace::default());
            let arr_ty = self
                .context
                .struct_type(&[i64_ty.into(), i8_ptr.into()], false);
            let arr_slot = self
                .builder
                .build_alloca(arr_ty, "for.arr.slot")
                .map_err(|e| format!("build_alloca for.arr.slot failed: {:?}", e))?;
            let sv = arr_val.into_struct_value();
            self.builder
                .build_store(arr_slot, sv)
                .map_err(|e| format!("store for.arr.slot failed: {:?}", e))?;
            let is_string_elem = accessor == "titrate_array_get_string";
            let ret_ty: BasicTypeEnum<'ctx> = if accessor == "titrate_array_get_double" {
                self.context.f64_type().into()
            } else if is_string_elem {
                llvm_types::string_type(self.context)
            } else if accessor == "titrate_array_get_ptr" {
                i8_ptr.into()
            } else {
                i64_ty.into()
            };
            // String elements come back through an out-pointer (struct returns
            // disagree across the FFI); scalars return directly as before.
            let fn_type = if is_string_elem {
                self.context.void_type().fn_type(
                    &[i8_ptr.into(), i64_ty.into(), i8_ptr.into()],
                    false,
                )
            } else {
                ret_ty.fn_type(&[i8_ptr.into(), i64_ty.into()], false)
            };
            let fn_val = if let Some(f) = self.module.get_function(accessor) {
                f
            } else {
                self.module.add_function(accessor, fn_type, Some(Linkage::External))
            };
            if is_string_elem {
                let out_slot = self
                    .builder
                    .build_alloca(
                        llvm_types::string_type(self.context).into_struct_type(),
                        "for.elem.out",
                    )
                    .map_err(|e| format!("build_alloca for.elem.out failed: {:?}", e))?;
                self.builder
                    .build_call(
                        fn_val,
                        &[arr_slot.into(), counter_val.into(), out_slot.into()],
                        "for.elem",
                    )
                    .map_err(|e| format!("for-in array_get failed: {:?}", e))?;
                self.builder
                    .build_load(
                        llvm_types::string_type(self.context).into_struct_type(),
                        out_slot,
                        "for.elem.val",
                    )
                    .map_err(|e| format!("load for.elem.val failed: {:?}", e))?
            } else {
                let elem_call = self
                    .builder
                    .build_call(
                        fn_val,
                        &[arr_slot.into(), counter_val.into()],
                        "for.elem",
                    )
                    .map_err(|e| format!("for-in array_get failed: {:?}", e))?;
                match elem_call.try_as_basic_value() {
                    inkwell::values::ValueKind::Basic(v) => v,
                    _ => return Err("for-in array_get did not return a value".to_string()),
                }
            }
        };
        // Truncate i64 results to the element type when narrower (e.g. int).
        let elem_val = if elem_raw.get_type() != elem_ty {
            if elem_raw.is_int_value() && elem_ty.is_int_type() {
                let src_w = elem_raw.get_type().into_int_type().get_bit_width();
                let dst_w = elem_ty.into_int_type().get_bit_width();
                if src_w > dst_w {
                    self.builder
                        .build_int_truncate(elem_raw.into_int_value(), elem_ty.into_int_type(), "for.elem.trunc")
                        .map_err(|e| format!("for-in truncate failed: {:?}", e))?
                        .into()
                } else {
                    elem_raw
                }
            } else {
                elem_raw
            }
        } else {
            elem_raw
        };
        self.builder
            .build_store(loop_var_alloca, elem_val)
            .map_err(|e| format!("store loop var failed: {:?}", e))?;

        let prev = self.locals.insert(
            for_stmt.var.clone(),
            LocalVar {
                ptr: loop_var_alloca,
                ty: elem_ty,
                full_type: None,
                titrate_type: Some(elem_name.clone()),
                shared: false,
                closure_sig: None,
            },
        );
        self.loop_stack.push(LoopContext {
            continue_block: inc_block,
            break_block: end_block,
        });
        for s in &for_stmt.body {
            self.compile_stmt(s)?;
        }
        self.loop_stack.pop();
        if let Some(p) = prev {
            self.locals.insert(for_stmt.var.clone(), p);
        } else {
            self.locals.remove(&for_stmt.var);
        }
        if self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_none()
        {
            self.builder
                .build_unconditional_branch(inc_block)
                .map_err(|e| format!("build_br for.inc failed: {:?}", e))?;
        }

        self.builder.position_at_end(inc_block);
        let counter_val = self
            .builder
            .build_load(i64_ty, counter_alloca, "for.i.inc")
            .map_err(|e| format!("load for.i.inc failed: {:?}", e))?
            .into_int_value();
        let next = self
            .builder
            .build_int_add(counter_val, i64_ty.const_int(1, false), "for.i.next")
            .map_err(|e| format!("for.i.inc failed: {:?}", e))?;
        self.builder
            .build_store(counter_alloca, next)
            .map_err(|e| format!("store for.i.next failed: {:?}", e))?;
        self.builder
            .build_unconditional_branch(cond_block)
            .map_err(|e| format!("build_br for.cond 2 failed: {:?}", e))?;

        self.builder.position_at_end(end_block);
        Ok(())
    }

    /// Emit an optimized array-iteration loop using pointer arithmetic.
    ///
    /// Instead of recomputing `array[i]` via GEP on each iteration (which
    /// requires a multiply + add for the index), this loads the base pointer
    /// once and increments it by the element size on each iteration. This is
    /// the canonical pattern LLVM's loop vectorizer recognizes.
    ///
    /// The emitted loop structure is:
    ///   ```text
    ///   ptr = base; end = base + count;
    ///   while (ptr != end) {
    ///       elem = load ptr;
    ///       ... body ...
    ///       ptr = ptr + 1;  // gep by element size
    ///   }
    ///   ```
    ///
    /// This is used as the foundation for `for (x in array)` loops once array
    /// iteration is fully supported by the codegen.
    #[allow(dead_code)]
    pub(super) fn emit_array_loop(
        &mut self,
        base_ptr: PointerValue<'ctx>,
        count: IntValue<'ctx>,
        elem_ty: BasicTypeEnum<'ctx>,
        var_name: &str,
        body: impl FnOnce(&mut Self, BasicValueEnum<'ctx>) -> Result<(), String>,
    ) -> Result<(), String> {
        let i8_ptr_ty = self.context.ptr_type(AddressSpace::default());

        // Compute the end pointer: end = base + count (GEP by element).
        let end_ptr = unsafe {
            self.builder
                .build_in_bounds_gep(elem_ty, base_ptr, &[count], "arr.end.ptr")
        }
        .map_err(|e| format!("build_gep arr.end.ptr failed: {:?}", e))?;

        // Allocate the loop pointer (current position).
        let cur_ptr_alloca = self
            .builder
            .build_alloca(i8_ptr_ty, "arr.cur.ptr")
            .map_err(|e| format!("build_alloca arr.cur.ptr failed: {:?}", e))?;
        // Store the base pointer into it.
        let base_as_i8 = self
            .builder
            .build_bit_cast(base_ptr, i8_ptr_ty, "arr.base.i8")
            .map_err(|e| format!("build_bit_cast base failed: {:?}", e))?;
        self.builder
            .build_store(cur_ptr_alloca, base_as_i8)
            .map_err(|e| format!("build_store cur.ptr failed: {:?}", e))?;

        // Allocate the loop variable (visible to the body).
        let loop_var_alloca = self
            .builder
            .build_alloca(elem_ty, var_name)
            .map_err(|e| format!("build_alloca '{}' failed: {:?}", var_name, e))?;

        let current_block = self
            .builder
            .get_insert_block()
            .ok_or("codegen: no insert block for array loop")?;
        let cond_block = self
            .context
            .insert_basic_block_after(current_block, "arr.cond");
        let body_block = self
            .context
            .insert_basic_block_after(cond_block, "arr.body");
        let inc_block = self.context.insert_basic_block_after(body_block, "arr.inc");
        let end_block = self.context.insert_basic_block_after(inc_block, "arr.end");

        self.builder
            .build_unconditional_branch(cond_block)
            .map_err(|e| format!("build_br arr.cond failed: {:?}", e))?;

        // Condition block: while cur_ptr != end_ptr.
        self.builder.position_at_end(cond_block);
        let cur_ptr_val = self
            .builder
            .build_load(i8_ptr_ty, cur_ptr_alloca, "arr.cur.val")
            .map_err(|e| format!("build_load arr.cur failed: {:?}", e))?
            .into_pointer_value();
        let end_as_i8 = self
            .builder
            .build_bit_cast(end_ptr, i8_ptr_ty, "arr.end.i8")
            .map_err(|e| format!("build_bit_cast end failed: {:?}", e))?
            .into_pointer_value();
        // Convert pointers to integers for comparison.
        let intptr_ty = if cfg!(target_pointer_width = "64") {
            self.context.i64_type()
        } else {
            self.context.i32_type()
        };
        let cur_int = self
            .builder
            .build_ptr_to_int(cur_ptr_val, intptr_ty, "arr.cur.int")
            .map_err(|e| format!("build_ptr_to_int cur failed: {:?}", e))?;
        let end_int = self
            .builder
            .build_ptr_to_int(end_as_i8, intptr_ty, "arr.end.int")
            .map_err(|e| format!("build_ptr_to_int end failed: {:?}", e))?;
        let cmp = self
            .builder
            .build_int_compare(inkwell::IntPredicate::EQ, cur_int, end_int, "arr.cmp")
            .map_err(|e| format!("build_int_compare arr failed: {:?}", e))?;
        // If equal, we're done; otherwise enter body.
        self.builder
            .build_conditional_branch(cmp, end_block, body_block)
            .map_err(|e| format!("build_cond_br arr failed: {:?}", e))?;

        // Body block: load the current element and run the body.
        self.builder.position_at_end(body_block);
        let cur_for_elem = self
            .builder
            .build_load(i8_ptr_ty, cur_ptr_alloca, "arr.body.ptr")
            .map_err(|e| format!("build_load arr.body.ptr failed: {:?}", e))?
            .into_pointer_value();
        // Cast back to the element pointer type.
        let elem_ptr = self
            .builder
            .build_bit_cast(cur_for_elem, i8_ptr_ty, "arr.elem.ptr")
            .map_err(|e| format!("build_bit_cast elem.ptr failed: {:?}", e))?
            .into_pointer_value();
        let elem_val = self
            .builder
            .build_load(elem_ty, elem_ptr, "arr.elem")
            .map_err(|e| format!("build_load arr.elem failed: {:?}", e))?;
        self.builder
            .build_store(loop_var_alloca, elem_val)
            .map_err(|e| format!("build_store arr.elem failed: {:?}", e))?;

        // Run the body closure.
        body(self, elem_val)?;

        if self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_none()
        {
            self.builder
                .build_unconditional_branch(inc_block)
                .map_err(|e| format!("build_br arr.inc failed: {:?}", e))?;
        }

        // Increment block: advance the pointer by one element.
        self.builder.position_at_end(inc_block);
        let cur_ptr_val = self
            .builder
            .build_load(i8_ptr_ty, cur_ptr_alloca, "arr.inc.ptr")
            .map_err(|e| format!("build_load arr.inc.ptr failed: {:?}", e))?
            .into_pointer_value();
        // Advance by the element size in bytes.
        let elem_size = elem_ty
            .size_of()
            .ok_or_else(|| "codegen: cannot compute element size".to_string())?;
        let next_ptr = unsafe {
            self.builder.build_in_bounds_gep(
                self.context.i8_type(),
                cur_ptr_val,
                &[elem_size],
                "arr.next.ptr",
            )
        }
        .map_err(|e| format!("build_gep arr.next.ptr failed: {:?}", e))?;
        self.builder
            .build_store(cur_ptr_alloca, next_ptr)
            .map_err(|e| format!("build_store arr.next failed: {:?}", e))?;
        self.builder
            .build_unconditional_branch(cond_block)
            .map_err(|e| format!("build_br arr.cond 2 failed: {:?}", e))?;

        // In release mode, attach vectorization hints.
        if self.release_mode {
            let _ = self.add_vectorize_metadata(inc_block);
        }

        self.builder.position_at_end(end_block);
        Ok(())
    }

    /// Compile a C-style for loop.
    pub(super) fn compile_c_for(&mut self, cfor_stmt: &crate::ast::CForStmt) -> Result<(), String> {
        let current_block = self
            .builder
            .get_insert_block()
            .ok_or("codegen: no insert block for c-for")?;

        // Init statement (in the current block).
        if let Some(init) = &cfor_stmt.init {
            self.compile_stmt(init)?;
        }

        let cond_block = self.context.insert_basic_block_after(
            self.builder.get_insert_block().unwrap_or(current_block),
            "cfor.cond",
        );
        let body_block = self
            .context
            .insert_basic_block_after(cond_block, "cfor.body");
        let inc_block = self
            .context
            .insert_basic_block_after(body_block, "cfor.inc");
        let end_block = self.context.insert_basic_block_after(inc_block, "cfor.end");

        self.builder
            .build_unconditional_branch(cond_block)
            .map_err(|e| format!("build_br cfor.cond failed: {:?}", e))?;

        // Condition block.
        self.builder.position_at_end(cond_block);
        if let Some(cond) = &cfor_stmt.condition {
            let cond_val = self.compile_expr(cond)?.into_int_value();
            self.builder
                .build_conditional_branch(cond_val, body_block, end_block)
                .map_err(|e| format!("build_cond_br cfor failed: {:?}", e))?;
        } else {
            self.builder
                .build_unconditional_branch(body_block)
                .map_err(|e| format!("build_br cfor.body failed: {:?}", e))?;
        }

        // Body block.
        self.builder.position_at_end(body_block);
        self.loop_stack.push(LoopContext {
            continue_block: inc_block,
            break_block: end_block,
        });
        for s in &cfor_stmt.body {
            self.compile_stmt(s)?;
        }
        self.loop_stack.pop();
        if self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_none()
        {
            self.builder
                .build_unconditional_branch(inc_block)
                .map_err(|e| format!("build_br cfor.inc failed: {:?}", e))?;
        }

        // Increment block.
        self.builder.position_at_end(inc_block);
        if let Some(incr) = &cfor_stmt.increment {
            self.compile_expr(incr)?;
        }
        self.builder
            .build_unconditional_branch(cond_block)
            .map_err(|e| format!("build_br cfor.cond 2 failed: {:?}", e))?;

        self.builder.position_at_end(end_block);
        Ok(())
    }

    /// Compile a switch/case statement. For literal patterns, uses LLVM's
    /// `switch` instruction. For wildcard `_`, uses the default case.
    /// For enum and `Result` discriminators (tagged `{ i32, ptr }` values),
    /// branches on the variant tag and binds the payload fields into locals.
    pub(super) fn compile_switch(&mut self, switch_stmt: &crate::ast::SwitchStmt) -> Result<(), String> {
        use crate::ast::Pattern;
        // Resolve the full switch type (with generic params) so Result/enum
        // pattern bindings get the correct element types.
        let switch_ty = if let Expr::Identifier(name, _) = &switch_stmt.expr {
            self.locals
                .get(name)
                .and_then(|v| v.full_type.clone())
                .unwrap_or_else(|| self.infer_expr_type(&switch_stmt.expr))
        } else {
            self.infer_expr_type(&switch_stmt.expr)
        };
        let type_name = switch_ty.name();
        let val = self.compile_expr(&switch_stmt.expr)?;
        let i32_ty = self.context.i32_type();

        let enum_info = self.enum_infos.get(type_name).cloned();
        // Some stdlib natives return a Result in the analyzer but the local
        // type inference reports "void". Their Ok/Err cases still need a tag.
        let void_result = type_name != "Result"
            && switch_stmt.cases.iter().any(|c| {
                matches!(&c.pattern, crate::ast::Pattern::Constructor { name, .. }
                    if name == "Ok" || name == "Err")
            });
        let is_result = type_name == "Result" || void_result;

        // Compute the i32 discriminator:
        // - int: used directly
        // - Result { i32 tag, ptr payload }: extract tag
        // - enum pointer -> { i32 tag, ptr payload }: load struct, extract tag
        let disc: IntValue<'ctx> = if void_result {
            // The VM does not treat these native returns as Result values, so
            // neither Ok (0) nor Err (1) matches; take the default branch.
            i32_ty.const_int(2, false)
        } else if is_result {
            let sv = val
                .into_struct_value();
            self.builder
                .build_extract_value(sv, 0, "switch.tag")
                .map_err(|e| format!("build_extract_value switch tag failed: {:?}", e))?
                .into_int_value()
        } else if let Some(enum_info) = enum_info.as_ref() {
            let ptr = val.into_pointer_value();
            let enum_ty = enum_info.struct_type;
            let loaded = self
                .builder
                .build_load(enum_ty, ptr, "switch.enum")
                .map_err(|e| format!("build_load switch enum failed: {:?}", e))?
                .into_struct_value();
            self.builder
                .build_extract_value(loaded, 0, "switch.tag")
                .map_err(|e| format!("build_extract_value enum tag failed: {:?}", e))?
                .into_int_value()
        } else {
            val.into_int_value()
        };

        let current_block = self
            .builder
            .get_insert_block()
            .ok_or("codegen: no insert block for switch")?;
        let end_block = self
            .context
            .insert_basic_block_after(current_block, "switch.end");

        // Create basic blocks for each case.
        let mut case_blocks: Vec<(inkwell::basic_block::BasicBlock<'ctx>, &crate::ast::Case)> =
            Vec::new();
        let mut prev_block = current_block;
        for case in &switch_stmt.cases {
            let cb = self
                .context
                .insert_basic_block_after(prev_block, "switch.case");
            case_blocks.push((cb, case));
            prev_block = cb;
        }
        let default_block = if switch_stmt.default.is_some() {
            let db = self
                .context
                .insert_basic_block_after(prev_block, "switch.default");
            Some(db)
        } else {
            None
        };

        // Build the switch instruction: collect (value, block) pairs.
        let default_target = default_block.unwrap_or(end_block);
        let mut cases_vec: Vec<(IntValue<'ctx>, inkwell::basic_block::BasicBlock<'ctx>)> =
            Vec::new();
        for (cb, case) in &case_blocks {
            match &case.pattern {
                Pattern::Literal(Literal::Int(v)) => {
                    cases_vec.push((i32_ty.const_int(*v as u64, *v < 0), *cb));
                }
                Pattern::Constructor { name, .. } => {
                    let tag: Option<u64> = if is_result {
                        match name.as_str() {
                            "Ok" => Some(0),
                            "Err" => Some(1),
                            _ => None,
                        }
                    } else if let Some(enum_info) = &enum_info {
                        super::enum_codegen::variant_tag(&enum_info.variant_names, name).map(|t| t as u64)
                    } else {
                        None
                    };
                    if let Some(tag) = tag {
                        cases_vec.push((i32_ty.const_int(tag, false), *cb));
                    }
                }
                _ => {}
            }
        }
        self.builder
            .build_switch(disc, default_target, &cases_vec)
            .map_err(|e| format!("build_switch failed: {:?}", e))?;

        // Compile each case body.
        for (cb, case) in case_blocks {
            self.builder.position_at_end(cb);
            // Bind constructor pattern payload fields into locals.
            if let Pattern::Constructor { name, bindings } = &case.pattern {
                if void_result {
                    // A native that the analyzer types as Result returns either
                    // its payload struct (e.g. File.readFile) or a scalar. Bind
                    // the payload to each pattern name so the Ok case works.
                    for b in bindings {
                        if b != "_" {
                            if val.is_struct_value() {
                                self.bind_switch_local(b, val.get_type(), val)?;
                            } else {
                                let ty: BasicTypeEnum<'ctx> = self.context.i64_type().into();
                                self.bind_switch_local(b, ty, ty.into_int_type().const_zero().into())?;
                            }
                        }
                    }
                } else {
                    self.bind_switch_pattern(&switch_ty, &enum_info, is_result, name, bindings, val)?;
                }
            }
            for s in &case.body {
                self.compile_stmt(s)?;
            }
            if self
                .builder
                .get_insert_block()
                .and_then(|b| b.get_terminator())
                .is_none()
            {
                self.builder
                    .build_unconditional_branch(end_block)
                    .map_err(|e| format!("build_br switch.end failed: {:?}", e))?;
            }
        }

        // Default case.
        if let Some(db) = default_block {
            if let Some(default_body) = &switch_stmt.default {
                self.builder.position_at_end(db);
                for s in default_body {
                    self.compile_stmt(s)?;
                }
                if self
                    .builder
                    .get_insert_block()
                    .and_then(|b| b.get_terminator())
                    .is_none()
                {
                    self.builder
                        .build_unconditional_branch(end_block)
                        .map_err(|e| format!("build_br switch.end 2 failed: {:?}", e))?;
                }
            }
        }

        self.builder.position_at_end(end_block);
        Ok(())
    }

    /// Bind the payload fields of a matched `Constructor` pattern into locals.
    pub(super) fn bind_switch_pattern(
        &mut self,
        switch_ty: &Type,
        enum_info: &Option<super::enum_codegen::EnumInfo<'ctx>>,
        is_result: bool,
        name: &str,
        bindings: &[String],
        subject: BasicValueEnum<'ctx>,
    ) -> Result<(), String> {
        let i8_ptr = self.context.ptr_type(AddressSpace::default());

        // Extract the payload pointer (field 1 of the { i32, ptr } tag/value).
        let payload_ptr: PointerValue<'ctx> = if is_result {
            let sv = subject.into_struct_value();
            let p = self
                .builder
                .build_extract_value(sv, 1, "switch.payload")
                .map_err(|e| format!("extract switch payload failed: {:?}", e))?;
            p.into_pointer_value()
        } else {
            let enum_info = enum_info
                .as_ref()
                .ok_or_else(|| format!("enum info not found for '{}'", switch_ty.name()))?;
            let ptr = subject.into_pointer_value();
            let loaded = self
                .builder
                .build_load(enum_info.struct_type, ptr, "switch.enum.payload")
                .map_err(|e| format!("load enum payload failed: {:?}", e))?
                .into_struct_value();
            let p = self
                .builder
                .build_extract_value(loaded, 1, "switch.payload")
                .map_err(|e| format!("extract enum payload failed: {:?}", e))?;
            p.into_pointer_value()
        };

        // Resolve each binding's LLVM type and source slot.
        for (i, binding) in bindings.iter().enumerate() {
            if binding == "_" {
                continue;
            }
            if is_result {
                // Result payload: a heap pointer to a single value of T or E.
                let params = switch_ty.params();
                let ty = if name == "Ok" {
                    params.first()
                } else {
                    params.get(1)
                };
                let binding_ty = match ty {
                    Some(t) => llvm_types::llvm_type(self.context, t)
                        .map_err(|e| format!("switch binding type: {}", e))?,
                    None => self.context.i64_type().into(),
                };
                let typed_ptr = self
                    .builder
                    .build_bit_cast(payload_ptr, i8_ptr, "switch.payload.cast")
                    .map_err(|e| format!("bit_cast payload failed: {:?}", e))?
                    .into_pointer_value();
                let field_val = self
                    .builder
                    .build_load(binding_ty, typed_ptr, "switch.field.val")
                    .map_err(|e| format!("load switch field failed: {:?}", e))?;
                self.bind_switch_local(binding, binding_ty, field_val)?;
            } else {
                let enum_info = enum_info
                    .as_ref()
                    .ok_or_else(|| format!("enum info not found for '{}'", switch_ty.name()))?;
                let variant_idx = super::enum_codegen::variant_tag(&enum_info.variant_names, name)
                    .ok_or_else(|| format!("variant '{}' not found in enum '{}'", name, switch_ty.name()))?
                    as usize;
                let fields = &enum_info.variant_fields[variant_idx];
                if fields.is_empty() {
                    // Simple variant (no fields): bind a dummy zero.
                    let binding_ty: BasicTypeEnum<'ctx> = self.context.i64_type().into();
                    let zero: BasicValueEnum<'ctx> = binding_ty.into_int_type().const_int(0, false).into();
                    self.bind_switch_local(binding, binding_ty, zero)?;
                } else {
                    let (_, binding_ty) = fields
                        .get(i)
                        .cloned()
                        .ok_or_else(|| format!("variant '{}' has no field {}", name, i))?;
                    let payload_struct_ty = enum_info.variant_payload_types[variant_idx]
                        .ok_or_else(|| format!("variant '{}' has no payload", name))?;
                    let payload_typed = self
                        .builder
                        .build_bit_cast(payload_ptr, i8_ptr, "switch.payload.cast")
                        .map_err(|e| format!("bit_cast payload failed: {:?}", e))?
                        .into_pointer_value();
                    let gep = self
                        .builder
                        .build_struct_gep(payload_struct_ty, payload_typed, i as u32, "switch.field")
                        .map_err(|e| format!("struct_gep switch field failed: {:?}", e))?;
                    let field_val = self
                        .builder
                        .build_load(binding_ty, gep, "switch.field.val")
                        .map_err(|e| format!("load switch field failed: {:?}", e))?;
                    self.bind_switch_local(binding, binding_ty, field_val)?;
                }
            }
        }
        Ok(())
    }

    /// Create a local for a switch pattern binding and store the value.
    pub(super) fn bind_switch_local(
        &mut self,
        name: &str,
        ty: BasicTypeEnum<'ctx>,
        value: BasicValueEnum<'ctx>,
    ) -> Result<(), String> {
        let alloca = self
            .builder
            .build_alloca(ty, name)
            .map_err(|e| format!("build_alloca switch binding failed: {:?}", e))?;
        self.builder
            .build_store(alloca, value)
            .map_err(|e| format!("store switch binding failed: {:?}", e))?;
        self.locals.insert(
            name.to_string(),
            LocalVar {
                ptr: alloca,
                ty,
                full_type: None,
                titrate_type: None,
                shared: false,
                closure_sig: None,
            },
        );
        Ok(())
    }

    /// Return the name of the enum that declares a variant, if any.
    pub(super) fn enum_name_for_variant(&self, variant: &str) -> Option<String> {
        self.enum_infos
            .iter()
            .find(|(_, info)| info.variant_names.iter().any(|v| v == variant))
            .map(|(name, _)| name.clone())
    }
}

#[cfg(test)]
mod tests {
    use crate::analyzer;
    use crate::ast::Declaration;
    use crate::lexer;
    use crate::parser;

    fn compile_generic_iterator() -> String {
        let source = r#"
            public class Bag<T> {
                private ArrayList<T> data;
                public fn init() { this.data = new ArrayList<T>(); }
                public fn add(x: T): void { this.data.add(x); }
                public fn iterator(): ArrayList<T> {
                    return this.data;
                }
            }
            public fn main(): void {
                let b = new Bag<int>();
                b.add(7);
                for (x in b.iterator()) {
                    io::println(x);
                }
            }
        "#;
        let tokens = lexer::tokenize(source).expect("tokenize");
        let ast = parser::parse(tokens).expect("parse");
        let typed = analyzer::analyze(&ast).expect("analyze");
        let context = inkwell::context::Context::create();
        let mut backend = super::LlvmBackend::new(&context, "iterator_test");
        backend.declare_natives();
        for decl in &typed.declarations {
            match decl {
                Declaration::Class(c) => {
                    backend.class_decls.insert(c.name.clone(), c.clone());
                }
                Declaration::Function(f) if f.type_params.is_empty() => {
                    backend.function_decls.insert(f.name.clone(), f.clone());
                }
                _ => {}
            }
        }
        let main = typed
            .declarations
            .iter()
            .find_map(|d| match d {
                Declaration::Function(f) if f.name == "main" => Some(f),
                _ => None,
            })
            .expect("main");
        backend.compile_main(main).expect("compile main");
        assert!(
            backend.module.verify().is_ok(),
            "iterator module failed verification"
        );
        backend.module.print_to_string().to_string()
    }

    fn function_ir<'a>(ir: &'a str, name: &str) -> &'a str {
        let needle = format!("@{}(", name);
        for chunk in ir.split("define ").skip(1) {
            let header = chunk.lines().next().unwrap_or("");
            if header.contains(&needle) {
                let end = chunk.find("\n}").unwrap_or(chunk.len());
                return &chunk[..end];
            }
        }
        panic!("function {} not found in IR:\n{}", name, ir)
    }

    /// A specialized generic iterator must GEP its own `this` parameter, not
    /// a stale parameter left over from another function's compilation.
    #[test]
    fn generic_iterator_uses_own_function_argument() {
        let ir = compile_generic_iterator();
        let body = function_ir(&ir, "Bag__int_iterator");
        assert!(
            body.contains("getelementptr") && body.contains(", ptr %0,"),
            "iterator field GEP must use its own argument: {}",
            body
        );
    }

    /// Every block emitted by generic iterator / for-in lowering ends in a
    /// terminator (module verification above enforces this globally).
    #[test]
    fn iterator_blocks_are_terminated() {
        let ir = compile_generic_iterator();
        for name in ["Bag__int_iterator", "Bag__int_add", "main"] {
            let body = function_ir(&ir, name);
            for block in body.split("\n\n").skip(1) {
                let last = block
                    .lines()
                    .rev()
                    .find(|l| !l.trim().is_empty())
                    .unwrap_or("")
                    .trim_start();
                assert!(
                    last.starts_with("ret ")
                        || last.starts_with("br ")
                        || last.starts_with("switch ")
                        || last.starts_with("unreachable"),
                    "unterminated block in {}:\n{}",
                    name,
                    block
                );
            }
        }
    }
}
