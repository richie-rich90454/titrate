// Expression compilation

use crate::ast;
use super::super::opcodes::OpCode;
use super::{Compiler, InferredType, Symbol};

impl Compiler {
    // -----------------------------------------------------------------------
    // Expression compilation
    // -----------------------------------------------------------------------

    pub(super) fn compile_expr(&mut self, expr: &ast::Expr) -> Result<(), String> {
        match expr {
            ast::Expr::Literal(lit, span) => {
                self.compile_literal(lit, span.line)?;
            }
            ast::Expr::Identifier(name, span) => {
                self.compile_identifier(name, span.line)?;
            }
            ast::Expr::Binary(left, op, right, span) => {
                self.compile_binary(left, op, right, span.line)?;
            }
            ast::Expr::Unary(op, operand, span) => {
                self.compile_unary(op, operand, span.line)?;
            }
            ast::Expr::Call(callee, args, span) => {
                self.compile_call(callee, args, span.line)?;
            }
            ast::Expr::MemberAccess(obj, member, span) => {
                self.compile_member_access(obj, member, span.line)?;
            }
            ast::Expr::Index(obj, index, span) => {
                self.compile_expr(obj)?;
                self.compile_expr(index)?;
                self.emit_opcode(OpCode::ARRAY_GET, span.line);
            }
            ast::Expr::New(typ, args, span) => {
                self.compile_new(typ, args, span.line)?;
            }
            ast::Expr::This(span) => {
                // In methods, "this" is always slot 0.
                self.emit_opcode(OpCode::LOAD_LOCAL, span.line);
                self.emit_u8(0, span.line);
            }
            ast::Expr::Super(span) => {
                // "super" resolves to "this" for method dispatch.
                self.emit_opcode(OpCode::LOAD_LOCAL, span.line);
                self.emit_u8(0, span.line);
            }
            ast::Expr::OwnedDeref(inner, span) => {
                self.compile_expr(inner)?;
                self.emit_opcode(OpCode::UNBOX_VALUE, span.line);
            }
            ast::Expr::RegionAlloc(_typ, init, span) => {
                self.compile_expr(init)?;
                self.emit_opcode(OpCode::REGION_ALLOC, span.line);
            }
            ast::Expr::RefExpr(inner, kind, span) => {
                self.compile_expr(inner)?;
                match kind {
                    ast::RefKind::Immutable => self.emit_opcode(OpCode::REF_IMMUTABLE, span.line),
                    ast::RefKind::Mutable => self.emit_opcode(OpCode::REF_MUTABLE, span.line),
                }
            }
            ast::Expr::UnsafeBlock(block, _span) => {
                // Compile as a regular block.
                self.begin_scope();
                self.compile_block(block)?;
                self.end_scope();
            }
            ast::Expr::ErrorPropagation(inner, span) => {
                self.compile_expr(inner)?;
                self.emit_opcode(OpCode::UNWRAP_OR_PROPAGATE, span.line);
            }
            ast::Expr::Cast(inner, target_type, span) => {
                self.compile_expr(inner)?;
                let cast_target = self.type_to_cast_target(target_type);
                self.emit_opcode(OpCode::CAST, span.line);
                self.emit_u8(cast_target as u8, span.line);
            }
            ast::Expr::StaticCall {
                class_name,
                method,
                args,
                span,
            } => {
                self.compile_static_call(class_name, method, args, span.line)?;
            }
            ast::Expr::Assign(target, value, span) => {
                self.compile_assign(target, value, span.line)?;
            }
            ast::Expr::Ternary { condition, then_expr, else_expr, span } => {
                self.compile_ternary(condition, then_expr, else_expr, span.line)?;
            }
            ast::Expr::Unit(span) => {
                self.emit_opcode(OpCode::PUSH_VOID, span.line);
            }
            ast::Expr::Tuple(elements, span) => {
                for elem in elements {
                    self.compile_expr(elem)?;
                }
                self.emit_opcode(OpCode::TUPLE_NEW, span.line);
                self.emit_u16(elements.len() as u16, span.line);
            }
            ast::Expr::Closure {
                params,
                return_type: _,
                body,
                expr,
                captured_vars,
                span,
            } => {
                self.compile_closure(params, body, expr, captured_vars, span.line)?;
            }
            ast::Expr::Range(_, _, span) | ast::Expr::RangeInclusive(_, _, span) => {
                return Err(format!("Range expressions are not yet supported in the bytecode compiler (line {})", span.line));
            }
        }
        Ok(())
    }

    pub(super) fn compile_literal(&mut self, lit: &ast::Literal, line: u32) -> Result<(), String> {
        match lit {
            ast::Literal::Int(v) => {
                self.emit_opcode(OpCode::PUSH_I64, line);
                let bytes = (*v as i64).to_be_bytes();
                for &b in &bytes {
                    self.emit_u8(b, line);
                }
            }
            ast::Literal::Float(v) => {
                self.emit_opcode(OpCode::PUSH_F64, line);
                let bytes = (*v as f64).to_be_bytes();
                for &b in &bytes {
                    self.emit_u8(b, line);
                }
            }
            ast::Literal::Bool(b) => {
                self.emit_opcode(OpCode::PUSH_BOOL, line);
                self.emit_u8(if *b { 1 } else { 0 }, line);
            }
            ast::Literal::Char(c) => {
                self.emit_opcode(OpCode::PUSH_CHAR, line);
                let bytes = (*c as u32).to_be_bytes();
                for &b in &bytes {
                    self.emit_u8(b, line);
                }
            }
            ast::Literal::String(s) => {
                let idx = self.intern_string(s);
                self.emit_opcode(OpCode::PUSH_STRING, line);
                self.emit_u16(idx, line);
            }
            ast::Literal::Null => {
                self.emit_opcode(OpCode::PUSH_NULL, line);
            }
        }
        Ok(())
    }

    pub(super) fn compile_identifier(&mut self, name: &str, line: u32) -> Result<(), String> {
        // Check locals first.
        if let Some(slot) = self.resolve_local(name) {
            self.emit_opcode(OpCode::LOAD_LOCAL, line);
            self.emit_u8(slot, line);
            return Ok(());
        }

        // Check if it's a known function.
        if let Some(&fn_idx) = self.function_map.get(name) {
            self.emit_opcode(OpCode::PUSH_VOID, line); // placeholder – function refs not yet in value
            let _ = fn_idx;
            // For now, function calls are handled directly in compile_call.
            // If we reach here, it's a bare function reference.
            return Ok(());
        }

        // Check the imported symbol table.
        if let Some(symbol) = self.symbol_table.get(name).cloned() {
            match symbol {
                Symbol::Function(fn_idx) => {
                    // Imported function reference – handled in compile_call.
                    self.emit_opcode(OpCode::PUSH_VOID, line);
                    let _ = fn_idx;
                    return Ok(());
                }
                Symbol::Class(class_idx) => {
                    // Imported class reference – handled in compile_new.
                    let _ = class_idx;
                    self.emit_opcode(OpCode::PUSH_NULL, line);
                    return Ok(());
                }
                Symbol::Enum(enum_idx) => {
                    // Imported enum reference.
                    let _ = enum_idx;
                    self.emit_opcode(OpCode::PUSH_NULL, line);
                    return Ok(());
                }
            }
        }

        // Check if it's an enum variant (bare reference without call).
        if self.variant_map.contains_key(name) {
            // This is a partial application – the variant will be called later.
            // For now, emit a placeholder.
            self.emit_opcode(OpCode::PUSH_NULL, line);
            return Ok(());
        }

        // Unknown identifier – could be a global or builtin.
        // Emit a LOAD_LOCAL with slot 0 as a fallback; the VM should handle this.
        // In practice, the analyzer should catch undefined variables.
        self.emit_opcode(OpCode::LOAD_LOCAL, line);
        self.emit_u8(0, line);
        Ok(())
    }

    /// Map an AST operator to its operator method name (e.g. Add → "operator+").
    fn operator_method_name(op: &ast::Operator) -> String {
        match op {
            ast::Operator::Add => "operator+".to_string(),
            ast::Operator::Sub => "operator-".to_string(),
            ast::Operator::Mul => "operator*".to_string(),
            ast::Operator::Div => "operator/".to_string(),
            ast::Operator::Mod => "operator%".to_string(),
            ast::Operator::Eq => "operator==".to_string(),
            ast::Operator::Ne => "operator!=".to_string(),
            ast::Operator::Lt => "operator<".to_string(),
            ast::Operator::Gt => "operator>".to_string(),
            ast::Operator::Le => "operator<=".to_string(),
            ast::Operator::Ge => "operator>=".to_string(),
            ast::Operator::BitAnd => "operator&".to_string(),
            ast::Operator::BitOr => "operator|".to_string(),
            ast::Operator::BitXor => "operator^".to_string(),
            ast::Operator::BitShl => "operator<<".to_string(),
            ast::Operator::BitShr => "operator>>".to_string(),
            ast::Operator::BitUshr => "operator>>>".to_string(),
            ast::Operator::And | ast::Operator::Or => unreachable!("And/Or are short-circuit"),
        }
    }

    pub(super) fn compile_binary(
        &mut self,
        left: &ast::Expr,
        op: &ast::Operator,
        right: &ast::Expr,
        line: u32,
    ) -> Result<(), String> {
        // Short-circuit for And/Or.
        match op {
            ast::Operator::And => {
                // And: compile left, JMP_IF_FALSE(skip), compile right, JMP(end),
                //      (skip:) PUSH_BOOL(false), (end:)
                self.compile_expr(left)?;
                self.emit_opcode(OpCode::JMP_IF_FALSE, line);
                let skip_offset = self.current_ip();
                self.emit_i16(0, line); // placeholder

                self.compile_expr(right)?;
                self.emit_opcode(OpCode::JMP, line);
                let end_offset = self.current_ip();
                self.emit_i16(0, line); // placeholder

                // skip: PUSH_BOOL(false)
                let skip_ip = self.current_ip();
                self.patch_i16_at(skip_offset, (skip_ip - (skip_offset + 2)) as i16);
                self.emit_opcode(OpCode::PUSH_BOOL, line);
                self.emit_u8(0, line);

                // end:
                let end_ip = self.current_ip();
                self.patch_i16_at(end_offset, (end_ip - (end_offset + 2)) as i16);
                return Ok(());
            }
            ast::Operator::Or => {
                // Or: compile left, JMP_IF_TRUE(skip), compile right, JMP(end),
                //     (skip:) PUSH_BOOL(true), (end:)
                self.compile_expr(left)?;
                self.emit_opcode(OpCode::JMP_IF_TRUE, line);
                let skip_offset = self.current_ip();
                self.emit_i16(0, line); // placeholder

                self.compile_expr(right)?;
                self.emit_opcode(OpCode::JMP, line);
                let end_offset = self.current_ip();
                self.emit_i16(0, line); // placeholder

                // skip: PUSH_BOOL(true)
                let skip_ip = self.current_ip();
                self.patch_i16_at(skip_offset, (skip_ip - (skip_offset + 2)) as i16);
                self.emit_opcode(OpCode::PUSH_BOOL, line);
                self.emit_u8(1, line);

                // end:
                let end_ip = self.current_ip();
                self.patch_i16_at(end_offset, (end_ip - (end_offset + 2)) as i16);
                return Ok(());
            }
            _ => {}
        }

        // Non-short-circuit binary operators.
        self.compile_expr(left)?;
        self.compile_expr(right)?;

        let left_type = self.infer_expr_type(left);
        let right_type = self.infer_expr_type(right);
        let result_type = self.wider_type(left_type, right_type);

        // If the left operand is a class instance, emit INVOKE_OPERATOR.
        // The VM will look up the operator method on the class and call it,
        // falling back to built-in behavior if no operator method exists.
        if left_type == InferredType::Class {
            let method_name = Self::operator_method_name(op);
            let name_idx = self.current_chunk().add_string(&method_name);
            self.emit_opcode(OpCode::INVOKE_OPERATOR, line);
            self.emit_u16(name_idx, line);
            self.emit_u8(1, line); // arg count = 1 (the right operand)
            return Ok(());
        }

        match op {
            ast::Operator::Add => {
                if result_type == InferredType::String {
                    // Pick the right string concatenation opcode based on operand types.
                    if left_type == InferredType::String && right_type == InferredType::String {
                        self.emit_opcode(OpCode::STR_CONCAT, line);
                    } else if left_type == InferredType::String {
                        // String + non-String
                        self.emit_opcode(OpCode::STR_CONCAT_RIGHT, line);
                    } else if right_type == InferredType::String {
                        // non-String + String
                        self.emit_opcode(OpCode::STR_CONCAT_LEFT, line);
                    } else {
                        // Both non-String but result is String (e.g., toString calls)
                        self.emit_opcode(OpCode::STR_CONCAT, line);
                    }
                } else {
                    self.emit_add_opcode(result_type, line);
                }
            }
            ast::Operator::Sub => self.emit_sub_opcode(result_type, line),
            ast::Operator::Mul => self.emit_mul_opcode(result_type, line),
            ast::Operator::Div => self.emit_div_opcode(result_type, line),
            ast::Operator::Mod => self.emit_mod_opcode(result_type, line),
            ast::Operator::Eq => self.emit_eq_opcode(result_type, line),
            ast::Operator::Ne => self.emit_ne_opcode(result_type, line),
            ast::Operator::Lt => self.emit_lt_opcode(result_type, line),
            ast::Operator::Gt => self.emit_gt_opcode(result_type, line),
            ast::Operator::Le => self.emit_le_opcode(result_type, line),
            ast::Operator::Ge => self.emit_ge_opcode(result_type, line),
            ast::Operator::BitAnd => self.emit_bitand_opcode(result_type, line),
            ast::Operator::BitOr => self.emit_bitor_opcode(result_type, line),
            ast::Operator::BitXor => self.emit_bitxor_opcode(result_type, line),
            ast::Operator::BitShl => self.emit_shl_opcode(result_type, line),
            ast::Operator::BitShr => self.emit_shr_opcode(result_type, line),
            ast::Operator::BitUshr => self.emit_ushr_opcode(result_type, line),
            ast::Operator::And | ast::Operator::Or => {
                unreachable!("And/Or handled above")
            }
        }

        Ok(())
    }

    pub(super) fn compile_unary(&mut self, op: &ast::UnOp, operand: &ast::Expr, line: u32) -> Result<(), String> {
        self.compile_expr(operand)?;
        let ty = self.infer_expr_type(operand);
        match op {
            ast::UnOp::Neg => self.emit_neg_opcode(ty, line),
            ast::UnOp::Not => {
                self.emit_opcode(OpCode::NOT, line);
            }
            ast::UnOp::BitNot => self.emit_bitnot_opcode(ty, line),
        }
        Ok(())
    }

    pub(super) fn compile_call(&mut self, callee: &ast::Expr, args: &[ast::Expr], line: u32) -> Result<(), String> {
        // Special case: super(...) call in a constructor.
        if let ast::Expr::Super(_) = callee {
            // super(args) calls the parent class's constructor.
            // We look up the parent class's constructor function index and
            // call it directly with `this` as the first argument.
            let parent_ctor_idx = if let Some(class_idx) = self.current_class {
                let parent_idx = self.classes[class_idx as usize].parent;
                if let Some(pidx) = parent_idx {
                    self.classes[pidx as usize].constructor
                } else {
                    return Err("super() call but class has no parent".to_string());
                }
            } else {
                return Err("super() call outside of class constructor".to_string());
            };

            match parent_ctor_idx {
                Some(ctor_fn_idx) => {
                    // Stack: [this, arg0, arg1, ...]
                    // Load `this` (slot 0 in method body)
                    self.emit_opcode(OpCode::LOAD_LOCAL, line);
                    self.emit_u8(0, line);
                    // Compile arguments
                    for arg in args {
                        self.compile_expr(arg)?;
                    }
                    // Use CALL_SUPER which sets base to `this` position
                    self.emit_opcode(OpCode::CALL_SUPER, line);
                    self.emit_u16(ctor_fn_idx, line);
                    self.emit_u8(args.len() as u8, line);
                    // The constructor returns the instance; pop it.
                    self.emit_opcode(OpCode::POP, line);
                    self.emit_opcode(OpCode::PUSH_VOID, line);
                }
                None => {
                    // Parent has no explicit constructor; just discard args.
                    for arg in args {
                        self.compile_expr(arg)?;
                    }
                    for _ in args {
                        self.emit_opcode(OpCode::POP, line);
                    }
                    self.emit_opcode(OpCode::PUSH_VOID, line);
                }
            }
            return Ok(());
        }

        // Special case: Identifier("Ok") → RESULT_OK
        if let ast::Expr::Identifier(name, _) = callee {
            if name == "Ok" {
                if args.len() != 1 {
                    return Err("Ok() expects exactly 1 argument".to_string());
                }
                self.compile_expr(&args[0])?;
                self.emit_opcode(OpCode::RESULT_OK, line);
                return Ok(());
            }
            if name == "Err" {
                if args.len() != 1 {
                    return Err("Err() expects exactly 1 argument".to_string());
                }
                self.compile_expr(&args[0])?;
                self.emit_opcode(OpCode::RESULT_ERR, line);
                return Ok(());
            }

            // Check if it's an enum variant constructor.
            if let Some((enum_name, _variant_idx)) = self.variant_map.get(name) {
                let enum_idx = *self.enum_map.get(enum_name).unwrap() as u16;
                let variant_name_idx = self.intern_string(name);
                for arg in args {
                    self.compile_expr(arg)?;
                }
                self.emit_opcode(OpCode::ENUM_NEW, line);
                self.emit_u16(enum_idx, line);
                self.emit_u16(variant_name_idx, line);
                self.emit_u8(args.len() as u8, line);
                return Ok(());
            }

            // Check if it's a known function.
            if let Some(&fn_idx) = self.function_map.get(name) {
                for arg in args {
                    self.compile_expr(arg)?;
                }
                self.emit_opcode(OpCode::CALL, line);
                self.emit_u16(fn_idx, line);
                self.emit_u8(args.len() as u8, line);
                return Ok(());
            }

            // Try mangled name (for private functions within the current module).
            let mangled_candidates: Vec<String> = self.function_map.keys()
                .filter(|k| k.ends_with(&format!(".{}", name)))
                .cloned()
                .collect();
            if let Some(mangled) = mangled_candidates.first() {
                if let Some(&fn_idx) = self.function_map.get(mangled) {
                    for arg in args {
                        self.compile_expr(arg)?;
                    }
                    self.emit_opcode(OpCode::CALL, line);
                    self.emit_u16(fn_idx, line);
                    self.emit_u8(args.len() as u8, line);
                    return Ok(());
                }
            }

            // Check the imported symbol table for functions.
            if let Some(Symbol::Function(fn_idx)) = self.symbol_table.get(name).cloned() {
                for arg in args {
                    self.compile_expr(arg)?;
                }
                self.emit_opcode(OpCode::CALL, line);
                self.emit_u16(fn_idx, line);
                self.emit_u8(args.len() as u8, line);
                return Ok(());
            }

            // Check if it's a generic function (needs type arguments).
            if self.generic_function_map.contains_key(name) {
                return Err(format!(
                    "Cannot call generic function '{}' without type arguments",
                    name
                ));
            }

            // Check if it's a native function call using the Module_function
            // convention (e.g., Math_sqrt, String_length). We emit a STATIC_CALL
            // so the VM can resolve it via its native lookup fallback.
            if let Some(idx) = name.find('_') {
                let class_name = &name[..idx];
                let method_name = &name[idx + 1..];
                for arg in args {
                    self.compile_expr(arg)?;
                }
                let class_idx = self.intern_string(class_name);
                let method_idx = self.intern_string(method_name);
                self.emit_opcode(OpCode::STATIC_CALL, line);
                self.emit_u16(class_idx, line);
                self.emit_u16(method_idx, line);
                self.emit_u8(args.len() as u8, line);
                return Ok(());
            }
        }

        // Special case: MemberAccess callee → method call.
        if let ast::Expr::MemberAccess(ref obj, ref method, _) = *callee {
            // Check for static calls like io.println, Integer.toString, etc.
            if let ast::Expr::Identifier(ref obj_name, _) = **obj {
                if self.is_builtin_object(obj_name) {
                    self.compile_static_call(obj_name, method, args, line)?;
                    return Ok(());
                }
                // Check if obj_name is a class name.
                if self.class_map.contains_key(obj_name) {
                    self.compile_static_call(obj_name, method, args, line)?;
                    return Ok(());
                }
                // Check if obj_name is a module name (not a local variable) and
                // method is a known function. This handles module-qualified calls
                // like Indicators.sma(...), Risk.valueAtRisk(...), etc.
                if self.resolve_local(obj_name).is_none() {
                    // Try symbol table (imported functions registered by name).
                    if let Some(Symbol::Function(fn_idx)) = self.symbol_table.get(method).cloned() {
                        for arg in args {
                            self.compile_expr(arg)?;
                        }
                        self.emit_opcode(OpCode::CALL, line);
                        self.emit_u16(fn_idx, line);
                        self.emit_u8(args.len() as u8, line);
                        return Ok(());
                    }
                    // Try mangled name lookup: any key ending with ".module.function".
                    let suffix = format!(".{}.{}", obj_name, method);
                    let mangled_key = self.function_map.keys()
                        .find(|k| k.ends_with(&suffix))
                        .cloned();
                    if let Some(key) = mangled_key {
                        if let Some(&fn_idx) = self.function_map.get(&key) {
                            for arg in args {
                                self.compile_expr(arg)?;
                            }
                            self.emit_opcode(OpCode::CALL, line);
                            self.emit_u16(fn_idx, line);
                            self.emit_u8(args.len() as u8, line);
                            return Ok(());
                        }
                    }
                }
            }

            // Regular method call: compile obj, then args, then INVOKE_VIRTUAL.
            self.compile_expr(obj)?;
            for arg in args {
                self.compile_expr(arg)?;
            }
            let method_idx = self.intern_string(method);
            self.emit_opcode(OpCode::INVOKE_VIRTUAL, line);
            self.emit_u16(method_idx, line);
            self.emit_u8(args.len() as u8, line);
            return Ok(());
        }

        // General case: compile callee, then args, then CALL.
        self.compile_expr(callee)?;
        for arg in args {
            self.compile_expr(arg)?;
        }
        self.emit_opcode(OpCode::CALL, line);
        // Use function index 0 as placeholder; the VM will use the callee on the stack.
        self.emit_u16(0, line);
        self.emit_u8(args.len() as u8, line);

        Ok(())
    }

    pub(super) fn compile_member_access(&mut self, obj: &ast::Expr, member: &str, line: u32) -> Result<(), String> {
        // Check for static member access patterns.
        if let ast::Expr::Identifier(ref obj_name, _) = *obj {
            // io.println etc. are handled in compile_call via MemberAccess callee.
            // Here we handle bare member access (not a call).
            if self.is_builtin_object(obj_name) {
                // This is a reference to a builtin object's member.
                // It will typically be used in a call context, which is handled above.
                self.emit_opcode(OpCode::PUSH_NULL, line);
                return Ok(());
            }
        }

        // Regular field access: compile obj, then GET_FIELD.
        self.compile_expr(obj)?;
        let field_idx = self.intern_string(member);
        self.emit_opcode(OpCode::GET_FIELD, line);
        self.emit_u16(field_idx, line);

        Ok(())
    }

    pub(super) fn compile_new(&mut self, typ: &ast::Type, args: &[ast::Expr], line: u32) -> Result<(), String> {
        let class_name = typ.name();
        let type_params = typ.params();

        // Handle built-in types that aren't user-defined classes
        match class_name {
            "ArrayList" | "HashMap" => {
                let class_idx = self.get_or_create_builtin_class(class_name, type_params);
                for arg in args {
                    self.compile_expr(arg)?;
                }
                self.emit_opcode(OpCode::NEW, line);
                self.emit_u16(class_idx, line);
                return Ok(());
            }
            _ => {}
        }

        // Check if it's a generic class instantiation.
        if !type_params.is_empty() && self.generic_class_map.contains_key(class_name) {
            let class_idx = self.instantiate_generic_class(class_name, type_params)?;
            for arg in args {
                self.compile_expr(arg)?;
            }
            self.emit_opcode(OpCode::NEW, line);
            self.emit_u16(class_idx, line);
            return Ok(());
        }

        let class_idx = if let Some(&idx) = self.class_map.get(class_name) {
            idx
        } else if let Some(Symbol::Class(idx)) = self.symbol_table.get(class_name) {
            *idx
        } else {
            // Try mangled name (for classes within the current module).
            let mangled_key = self.class_map.keys()
                .find(|k| k.ends_with(&format!(".{}", class_name)))
                .cloned();
            if let Some(key) = mangled_key {
                *self.class_map.get(&key).unwrap()
            } else {
                return Err(format!("Unknown class '{}' in new expression", class_name));
            }
        };

        // Compile arguments.
        for arg in args {
            self.compile_expr(arg)?;
        }

        self.emit_opcode(OpCode::NEW, line);
        self.emit_u16(class_idx, line);

        // If the class has a constructor, the VM will call it after allocation.
        // The constructor call is implicit in the NEW opcode.

        Ok(())
    }

    pub(super) fn compile_static_call(
        &mut self,
        class_name: &str,
        method: &str,
        args: &[ast::Expr],
        line: u32,
    ) -> Result<(), String> {
        // Compile arguments.
        for arg in args {
            self.compile_expr(arg)?;
        }

        let class_idx = self.intern_string(class_name);
        let method_idx = self.intern_string(method);

        self.emit_opcode(OpCode::STATIC_CALL, line);
        self.emit_u16(class_idx, line);
        self.emit_u16(method_idx, line);
        self.emit_u8(args.len() as u8, line);

        Ok(())
    }

    pub(super) fn compile_assign(&mut self, target: &ast::Expr, value: &ast::Expr, line: u32) -> Result<(), String> {
        self.compile_expr(value)?;

        match target {
            ast::Expr::Identifier(name, _) => {
                if let Some(slot) = self.resolve_local(name) {
                    // DUP the value so it remains on the stack after STORE_LOCAL
                    // consumes the copy. The caller (compile_stmt) will emit POP
                    // for expression statements, which pops this DUP'd value.
                    self.emit_opcode(OpCode::DUP, line);
                    self.emit_opcode(OpCode::STORE_LOCAL, line);
                    self.emit_u8(slot, line);
                } else {
                    return Err(format!("Cannot assign to undefined variable '{}'", name));
                }
            }
            ast::Expr::MemberAccess(obj, member, _) => {
                self.compile_expr(obj)?;
                let field_idx = self.intern_string(member);
                self.emit_opcode(OpCode::SET_FIELD, line);
                self.emit_u16(field_idx, line);
            }
            ast::Expr::Index(obj, index, _) => {
                self.compile_expr(obj)?;
                self.compile_expr(index)?;
                self.emit_opcode(OpCode::ARRAY_SET, line);
            }
            _ => {
                return Err("Invalid assignment target".to_string());
            }
        }

        Ok(())
    }

    pub(super) fn compile_ternary(
        &mut self,
        condition: &ast::Expr,
        then_expr: &ast::Expr,
        else_expr: &ast::Expr,
        line: u32,
    ) -> Result<(), String> {
        // 1. Compile condition
        self.compile_expr(condition)?;
        self.emit_opcode(OpCode::JMP_IF_FALSE, line);
        let else_jump_offset = self.current_ip();
        self.emit_i16(0, line); // placeholder

        // 2. Compile then_expr (leaves value on stack)
        self.compile_expr(then_expr)?;

        // 3. Jump over else_expr
        self.emit_opcode(OpCode::JMP, line);
        let end_jump_offset = self.current_ip();
        self.emit_i16(0, line); // placeholder

        // 4. Patch else jump
        let else_start = self.current_ip();
        let else_instr_end = else_jump_offset + 2;
        let offset = (else_start - else_instr_end) as i16;
        self.patch_i16_at(else_jump_offset, offset);

        // 5. Compile else_expr (leaves value on stack)
        self.compile_expr(else_expr)?;

        // 6. Patch end jump
        let end_ip = self.current_ip();
        let end_instr_end = end_jump_offset + 2;
        let offset = (end_ip - end_instr_end) as i16;
        self.patch_i16_at(end_jump_offset, offset);

        Ok(())
    }

    pub(super) fn compile_closure(
        &mut self,
        params: &[(String, ast::Type)],
        body: &ast::Block,
        expr: &Option<Box<ast::Expr>>,
        captured_vars: &[String],
        line: u32,
    ) -> Result<(), String> {
        // Generate a unique name for the closure function.
        let closure_name = format!("$closure_{}", self.closure_counter);
        self.closure_counter += 1;

        // Register the closure as a new function.
        let fn_idx = self.functions.len() as u16;
        let arity = params.len();

        self.functions.push(super::FunctionDef {
            name: closure_name.clone(),
            arity,
            chunk: super::Chunk::new(),
            is_method: false,
            is_constructor: false,
            local_count: 0,
        });
        self.function_map.insert(closure_name.clone(), fn_idx);

        // Save current compilation state.
        let saved_function = self.current_function;
        let saved_locals = std::mem::take(&mut self.locals);
        let saved_local_count = self.local_count;
        let saved_scope_depth = self.scope_depth;

        self.current_function = fn_idx as usize;
        self.locals.clear();
        self.local_count = 0;
        self.scope_depth = 0;

        self.begin_scope();

        // Parameters become local variables.
        for (name, _typ) in params {
            self.declare_local(name)?;
        }

        // Compile the closure body.
        if let Some(ref e) = expr {
            // Expression body: fn(x) => x * 2
            self.compile_expr(e)?;
            self.emit_opcode(OpCode::RET, line);
        } else {
            // Block body: fn(x) { return x + 1; }
            self.compile_block(body)?;
            // Ensure every function ends with RET.
            self.emit_opcode(OpCode::PUSH_VOID, line);
            self.emit_opcode(OpCode::RET, line);
        }

        self.end_scope();

        // Store the number of local slots needed.
        self.functions[fn_idx as usize].local_count = self.local_count;

        // Restore compilation state.
        self.current_function = saved_function;
        self.locals = saved_locals;
        self.local_count = saved_local_count;
        self.scope_depth = saved_scope_depth;

        // For each captured variable, load its value onto the stack.
        for var_name in captured_vars {
            if let Some(slot) = self.resolve_local(var_name) {
                self.emit_opcode(OpCode::LOAD_LOCAL, line);
                self.emit_u8(slot, line);
            } else {
                // If not a local, push null as placeholder.
                self.emit_opcode(OpCode::PUSH_NULL, line);
            }
        }

        // Emit CLOSURE_NEW with function index and upvalue count.
        self.emit_opcode(OpCode::CLOSURE_NEW, line);
        self.emit_u16(fn_idx, line);
        self.emit_u8(captured_vars.len() as u8, line);

        Ok(())
    }
}
