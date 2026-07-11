// Expression compilation

use crate::ast;
use super::super::opcodes::{OpCode, TypeTag, CastTarget};
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
                // In methods, "this" is slot 0. Inside a closure, "this" may
                // have been captured as an upvalue — emit GET_UPVALUE in that case.
                if let Some(slot) = self.resolve_local("this") {
                    if self.is_local_upvalue(slot) {
                        let idx = self.get_upvalue_index(slot);
                        self.emit_opcode(OpCode::GET_UPVALUE, span.line);
                        self.emit_u8(idx, span.line);
                    } else {
                        self.emit_opcode(OpCode::LOAD_LOCAL, span.line);
                        self.emit_u8(slot, span.line);
                    }
                } else {
                    // Fallback: assume slot 0 (top-level method body).
                    self.emit_opcode(OpCode::LOAD_LOCAL, span.line);
                    self.emit_u8(0, span.line);
                }
            }
            ast::Expr::Super(span) => {
                // "super" resolves to "this" for method dispatch.
                if let Some(slot) = self.resolve_local("this") {
                    if self.is_local_upvalue(slot) {
                        let idx = self.get_upvalue_index(slot);
                        self.emit_opcode(OpCode::GET_UPVALUE, span.line);
                        self.emit_u8(idx, span.line);
                    } else {
                        self.emit_opcode(OpCode::LOAD_LOCAL, span.line);
                        self.emit_u8(slot, span.line);
                    }
                } else {
                    self.emit_opcode(OpCode::LOAD_LOCAL, span.line);
                    self.emit_u8(0, span.line);
                }
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
                let target_name = target_type.name();
                let cast_target = self.type_to_cast_target(target_type);
                // For class types, cast is a no-op at the bytecode level
                if cast_target == CastTarget::Long && target_name != "long" && target_name != "int" {
                    // No-op: class type cast, skip
                } else {
                    self.emit_opcode(OpCode::CAST, span.line);
                    self.emit_u8(cast_target as u8, span.line);
                }
            }
            ast::Expr::Is(inner, target_type, span) => {
                self.compile_expr(inner)?;
                self.compile_is_type_check(target_type, span.line)?;
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
            if self.is_local_upvalue(slot) {
                let idx = self.get_upvalue_index(slot);
                self.emit_opcode(OpCode::GET_UPVALUE, line);
                self.emit_u8(idx, line);
            } else {
                self.emit_opcode(OpCode::LOAD_LOCAL, line);
                self.emit_u8(slot, line);
            }
            return Ok(());
        }

        // Check module-level globals.
        if let Some(global_idx) = self.lookup_global(name) {
            self.emit_opcode(OpCode::LOAD_GLOBAL, line);
            self.emit_u16(global_idx, line);
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
                Symbol::GenericFunction(_) => {
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
            ast::Operator::BitShl => self.emit_shl_opcode(left_type, line),
            ast::Operator::BitShr => self.emit_shr_opcode(left_type, line),
            ast::Operator::BitUshr => self.emit_ushr_opcode(left_type, line),
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

        // Special case: this(args) call in a constructor (constructor delegation).
        // this(args) calls another constructor of the same class with matching arity.
        if let ast::Expr::This(_) = callee {
            if let Some(class_idx) = self.current_class {
                let class_name = self.classes[class_idx as usize].name.clone();
                let ctor_pattern = format!("{}.<init>", class_name);
                let ctor_arity = args.len();
                // Find the constructor function entry by matching arity.
                let ctor_fn_idx = self.functions.iter().enumerate()
                    .find(|(_, f)| f.name == ctor_pattern && f.arity == ctor_arity)
                    .map(|(i, _)| i as u16);
                if let Some(fn_idx) = ctor_fn_idx {
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
                    self.emit_u16(fn_idx, line);
                    self.emit_u8(args.len() as u8, line);
                    // The delegated constructor returns void; pop it.
                    self.emit_opcode(OpCode::POP, line);
                    self.emit_opcode(OpCode::PUSH_VOID, line);
                    return Ok(());
                }
            }
        }

        // Special case: Identifier("Ok"/"ok") → RESULT_OK, ("Err"/"err") → RESULT_ERR
        if let ast::Expr::Identifier(name, _) = callee {
            if name == "Ok" || name == "ok" {
                if args.len() != 1 {
                    return Err("Ok() expects exactly 1 argument".to_string());
                }
                self.compile_expr(&args[0])?;
                self.emit_opcode(OpCode::RESULT_OK, line);
                return Ok(());
            }
            if name == "Err" || name == "err" {
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
            // When multiple modules define a function with the same name, prefer:
            //   1. The current module's version
            //   2. A candidate whose arity matches the call
            //   3. The first candidate (fallback)
            let mangled_candidates: Vec<String> = self.function_map.keys()
                .filter(|k| k.ends_with(&format!(".{}", name)))
                .cloned()
                .collect();
            if !mangled_candidates.is_empty() {
                let arg_count = args.len();
                let current_module = &self.current_module;
                let in_module = |k: &str| {
                    !current_module.is_empty()
                        && current_module != "<main>"
                        && k.starts_with(&format!("{}.", current_module))
                };
                let arity_matches = |k: &str| {
                    self.function_map.get(k)
                        .and_then(|&idx| self.functions.get(idx as usize))
                        .map(|f| f.arity == arg_count)
                        .unwrap_or(false)
                };
                let mangled = mangled_candidates.iter()
                    .find(|k| in_module(k) && arity_matches(k))
                    .or_else(|| mangled_candidates.iter().find(|k| in_module(k)))
                    .or_else(|| mangled_candidates.iter().find(|k| arity_matches(k)))
                    .or_else(|| mangled_candidates.first())
                    .cloned();
                if let Some(mangled) = mangled {
                    if let Some(&fn_idx) = self.function_map.get(&mangled) {
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

            // Check the imported symbol table for functions.
            if let Some(symbol) = self.symbol_table.get(name).cloned() {
                match symbol {
                    Symbol::Function(fn_idx) => {
                        for arg in args {
                            self.compile_expr(arg)?;
                        }
                        self.emit_opcode(OpCode::CALL, line);
                        self.emit_u16(fn_idx, line);
                        self.emit_u8(args.len() as u8, line);
                        return Ok(());
                    }
                    Symbol::GenericFunction(idx) => {
                        let key = self.generic_functions[idx].name.clone();
                        let type_param_count = self.generic_functions[idx].type_params.len();
                        let placeholder = ast::Type::Named { name: "Variant".to_string(), params: vec![] };
                        let type_args: Vec<ast::Type> = (0..type_param_count).map(|_| placeholder.clone()).collect();
                        let fn_idx = self.instantiate_generic_function(&key, &type_args, args.len())?;
                        for arg in args {
                            self.compile_expr(arg)?;
                        }
                        self.emit_opcode(OpCode::CALL, line);
                        self.emit_u16(fn_idx, line);
                        self.emit_u8(args.len() as u8, line);
                        return Ok(());
                    }
                    _ => {}
                }
            }

            // Check if it's a generic function (needs type arguments).
            // For module-level generics, the key is the mangled name
            // (e.g. "tt.math.ndarray.NDArrayMath.map"). Check both short
            // name and mangled name (current_module + "." + name).
            let generic_key = if self.generic_function_map.contains_key(name) {
                Some(name.to_string())
            } else if !self.current_module.is_empty() && self.current_module != "<main>" {
                let mangled = format!("{}.{}", self.current_module, name);
                if self.generic_function_map.contains_key(&mangled) {
                    Some(mangled)
                } else {
                    None
                }
            } else {
                None
            };
            if let Some(key) = generic_key {
                let type_param_count = self.generic_functions[self.generic_function_map[&key]].type_params.len();
                let placeholder = ast::Type::Named { name: "Variant".to_string(), params: vec![] };
                let type_args: Vec<ast::Type> = (0..type_param_count).map(|_| placeholder.clone()).collect();
                let fn_idx = self.instantiate_generic_function(&key, &type_args, args.len())?;
                for arg in args {
                    self.compile_expr(arg)?;
                }
                self.emit_opcode(OpCode::CALL, line);
                self.emit_u16(fn_idx, line);
                self.emit_u8(args.len() as u8, line);
                return Ok(());
            }

            // Check if it's a native function call using the Module_function
            // convention (e.g., Math_sqrt, String_length). We emit a STATIC_CALL
            // so the VM can resolve it via its native lookup fallback.
            // Guard against identifiers that start with '_' (e.g. private helpers
            // like _quickSort) or are otherwise not in the Module_name form.
            if let Some(idx) = name.find('_') {
                let class_name = &name[..idx];
                let method_name = &name[idx + 1..];
                if !class_name.is_empty() {
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

            // Implicit this.method() inside a class method: when a bare
            // identifier call is not a function, native, or variable, treat
            // it as a virtual call on the current instance if the current
            // class (or any parent) declares the method.
            if let Some(class_idx) = self.current_class {
                let mut search_idx = Some(class_idx);
                let mut found_method = None;
                while let Some(idx) = search_idx {
                    let class_def = &self.classes[idx as usize];
                    if let Some(&method_fn_idx) = class_def.methods.get(name) {
                        found_method = Some(method_fn_idx);
                        break;
                    }
                    search_idx = class_def.parent;
                }
                if let Some(_method_fn_idx) = found_method {
                    // Load `this` (slot 0 in a method body).
                    self.emit_opcode(OpCode::LOAD_LOCAL, line);
                    self.emit_u8(0, line);
                    for arg in args {
                        self.compile_expr(arg)?;
                    }
                    let method_idx = self.intern_string(name);
                    self.emit_opcode(OpCode::INVOKE_VIRTUAL, line);
                    self.emit_u16(method_idx, line);
                    self.emit_u8(args.len() as u8, line);
                    return Ok(());
                }
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
                    // Try mangled name lookup FIRST: any key ending with ".module.function".
                    // This is more specific than the symbol table (which may contain
                    // a different module's function with the same name).
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
                    // Try generic function lookup: search generic_function_map
                    // for a key ending with ".module.function". This handles
                    // Module.genericFunction<T>(args) calls where the type args
                    // were discarded by the parser.
                    let generic_mangled_key = self.generic_function_map.keys()
                        .find(|k| k.ends_with(&suffix))
                        .cloned();
                    if let Some(key) = generic_mangled_key {
                        let type_param_count = {
                            let idx = self.generic_function_map[&key];
                            self.generic_functions[idx].type_params.len()
                        };
                        let placeholder = ast::Type::Named { name: "Variant".to_string(), params: vec![] };
                        let type_args: Vec<ast::Type> = (0..type_param_count).map(|_| placeholder.clone()).collect();
                        let fn_idx = self.instantiate_generic_function(&key, &type_args, args.len())?;
                        for arg in args {
                            self.compile_expr(arg)?;
                        }
                        self.emit_opcode(OpCode::CALL, line);
                        self.emit_u16(fn_idx, line);
                        self.emit_u8(args.len() as u8, line);
                        return Ok(());
                    }
                    // Fall back to symbol table (imported functions registered by name).
                    if let Some(symbol) = self.symbol_table.get(method).cloned() {
                        match symbol {
                            Symbol::Function(fn_idx) => {
                                for arg in args {
                                    self.compile_expr(arg)?;
                                }
                                self.emit_opcode(OpCode::CALL, line);
                                self.emit_u16(fn_idx, line);
                                self.emit_u8(args.len() as u8, line);
                                return Ok(());
                            }
                            Symbol::GenericFunction(idx) => {
                                let key = self.generic_functions[idx].name.clone();
                                let type_param_count = self.generic_functions[idx].type_params.len();
                                let placeholder = ast::Type::Named { name: "Variant".to_string(), params: vec![] };
                                let type_args: Vec<ast::Type> = (0..type_param_count).map(|_| placeholder.clone()).collect();
                                let fn_idx = self.instantiate_generic_function(&key, &type_args, args.len())?;
                                for arg in args {
                                    self.compile_expr(arg)?;
                                }
                                self.emit_opcode(OpCode::CALL, line);
                                self.emit_u16(fn_idx, line);
                                self.emit_u8(args.len() as u8, line);
                                return Ok(());
                            }
                            _ => {}
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

        // General case: callee is a complex expression (e.g., a closure variable
        // or a parenthesised expression). We need to emit a CALL that uses the
        // function index stored in the Closure value on the stack.
        //
        // If the callee is an Identifier that resolved to a local variable, it
        // might be holding a Closure. We emit CALL_CLOSURE which pops the callee
        // from the stack and calls it by its embedded function index.
        if let ast::Expr::Identifier(name, _) = callee {
            // Check if it's a local variable (possibly holding a closure).
            if let Some(slot) = self.resolve_local(name) {
                // Load the closure value, then args, then CALL_CLOSURE.
                if self.is_local_upvalue(slot) {
                    let idx = self.get_upvalue_index(slot);
                    self.emit_opcode(OpCode::GET_UPVALUE, line);
                    self.emit_u8(idx, line);
                } else {
                    self.emit_opcode(OpCode::LOAD_LOCAL, line);
                    self.emit_u8(slot, line);
                }
                for arg in args {
                    self.compile_expr(arg)?;
                }
                self.emit_opcode(OpCode::CALL_CLOSURE, line);
                self.emit_u8(args.len() as u8, line);
                return Ok(());
            }
            // Check if it's a global variable (possibly holding a closure).
            if let Some(gidx) = self.lookup_global(name) {
                self.emit_opcode(OpCode::LOAD_GLOBAL, line);
                self.emit_u16(gidx, line);
                for arg in args {
                    self.compile_expr(arg)?;
                }
                self.emit_opcode(OpCode::CALL_CLOSURE, line);
                self.emit_u8(args.len() as u8, line);
                return Ok(());
            }
            return Err(format!("Cannot resolve function '{}' at line {}", name, line));
        }
        // For non-Identifier callees (e.g., parenthesised expressions, index
        // expressions), compile the callee and use CALL_CLOSURE.
        self.compile_expr(callee)?;
        for arg in args {
            self.compile_expr(arg)?;
        }
        self.emit_opcode(OpCode::CALL_CLOSURE, line);
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

    pub(super) fn compile_is_type_check(&mut self, target_type: &ast::Type, line: u32) -> Result<(), String> {
        let name = target_type.name();
        let tag = self.type_to_type_tag(name);
        if tag != TypeTag::Class {
            self.emit_opcode(OpCode::TYPE_CHECK, line);
            self.emit_u8(tag as u8, line);
        } else {
            let class_name_idx = self.intern_string(name);
            self.emit_opcode(OpCode::INSTANCE_OF, line);
            self.emit_u16(class_name_idx, line);
        }
        Ok(())
    }

    fn type_to_type_tag(&self, name: &str) -> TypeTag {
        match name {
            "byte" => TypeTag::I8,
            "short" => TypeTag::I16,
            "int" => TypeTag::I32,
            "long" => TypeTag::I64,
            "vast" => TypeTag::I128,
            "uvast" => TypeTag::U128,
            "float" => TypeTag::F32,
            "double" => TypeTag::F64,
            "half" => TypeTag::F32,
            "quad" => TypeTag::F64,
            "bool" => TypeTag::Bool,
            "char" => TypeTag::Char,
            "string" | "String" => TypeTag::String,
            "void" => TypeTag::Void,
            "null" => TypeTag::Null,
            _ => TypeTag::Class,
        }
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
                self.emit_u8(args.len() as u8, line);
                return Ok(());
            }
            _ => {}
        }

        // Check if it's a generic class instantiation.
        if !type_params.is_empty() {
            // Resolve the generic class name: try direct lookup first, then
            // mangled name lookup for imported generic classes (e.g. "Pair"
            // → "tt.util.Pair.Pair").
            let resolved_name = if self.generic_class_map.contains_key(class_name) {
                Some(class_name.to_string())
            } else {
                self.generic_class_map.keys()
                    .find(|k| k.ends_with(&format!(".{}", class_name)))
                    .cloned()
            };
            if let Some(name) = resolved_name {
                let class_idx = self.instantiate_generic_class(&name, type_params)?;
                for arg in args {
                    self.compile_expr(arg)?;
                }
                self.emit_opcode(OpCode::NEW, line);
                self.emit_u16(class_idx, line);
                self.emit_u8(args.len() as u8, line);
                return Ok(());
            }
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
        self.emit_u8(args.len() as u8, line);

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
                    if self.is_local_upvalue(slot) {
                        let idx = self.get_upvalue_index(slot);
                        self.emit_opcode(OpCode::SET_UPVALUE, line);
                        self.emit_u8(idx, line);
                    } else {
                        self.emit_opcode(OpCode::STORE_LOCAL, line);
                        self.emit_u8(slot, line);
                    }
                } else if let Some(global_idx) = self.lookup_global(name) {
                    self.emit_opcode(OpCode::DUP, line);
                    self.emit_opcode(OpCode::STORE_GLOBAL, line);
                    self.emit_u16(global_idx, line);
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
        _captured_vars: &[String], // ignored – we compute our own
        line: u32,
    ) -> Result<(), String> {
        // 1. Find free variables referenced inside the closure body.
        let free_vars = find_free_variables(params, body, expr);

        // 2. Snapshot the enclosing locals so we can decide which free vars
        //    are actually capturable from this scope.
        let enclosing_locals = self.locals.clone();

        // 3. For each free variable, check whether it resolves to an
        //    enclosing local. Map "self" → "this" so that closures inside
        //    methods capture `this` correctly.
        let mut captured: Vec<(String, u8)> = Vec::new();
        for var_name in &free_vars {
            let lookup_name = if var_name == "self" { "this" } else { var_name };
            if let Some(local) = enclosing_locals.iter().rev().find(|l| l.name == lookup_name) {
                // Store under the resolved name so the upvalue local can be
                // found by `resolve_local` (which also maps "self" → "this").
                captured.push((lookup_name.to_string(), local.slot));
            }
        }
        // Dedup by name, preserving first occurrence.
        let mut seen = std::collections::HashSet::new();
        captured.retain(|(name, _)| seen.insert(name.clone()));

        // 4. Register the closure as a new function.
        let closure_name = format!("$closure_{}", self.closure_counter);
        self.closure_counter += 1;
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

        // 5. Save current compilation state, then start a fresh scope for
        //    the closure body.
        let saved_function = self.current_function;
        let saved_locals = std::mem::take(&mut self.locals);
        let saved_local_count = self.local_count;
        let saved_scope_depth = self.scope_depth;

        self.current_function = fn_idx as usize;
        self.locals.clear();
        self.local_count = 0;
        self.scope_depth = 0;

        self.begin_scope();

        // 6. Parameters become local variables.
        for (name, _typ) in params {
            self.declare_local(name)?;
        }

        // 7. Declare each captured variable as an upvalue local so that
        //    identifier resolution inside the body finds it. The upvalue
        //    index matches the position in `captured`, which is the order
        //    we'll push values onto the stack below.
        for (idx, (name, _)) in captured.iter().enumerate() {
            self.declare_upvalue(name, idx as u8)?;
        }

        // 8. Compile the closure body. Identifier reads/writes for the
        //    captured locals will emit GET_UPVALUE/SET_UPVALUE (see
        //    compile_identifier / compile_assign).
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

        // Store the number of local slots needed (params + upvalues + body locals).
        self.functions[fn_idx as usize].local_count = self.local_count;

        // 9. Restore the enclosing compilation state.
        self.current_function = saved_function;
        self.locals = saved_locals;
        self.local_count = saved_local_count;
        self.scope_depth = saved_scope_depth;

        // 10. In the enclosing scope, push each captured variable's value
        //     onto the stack so CLOSURE_NEW can pop them into the upvalue
        //     vector. If the enclosing local is itself an upvalue (nested
        //     closure scenario), emit GET_UPVALUE instead of LOAD_LOCAL.
        for (_name, slot) in &captured {
            let is_uv = enclosing_locals
                .iter()
                .rev()
                .find(|l| l.slot == *slot)
                .map_or(false, |l| l.is_upvalue);
            if is_uv {
                let idx = enclosing_locals
                    .iter()
                    .rev()
                    .find(|l| l.slot == *slot && l.is_upvalue)
                    .map(|l| l.upvalue_idx)
                    .unwrap_or(0);
                self.emit_opcode(OpCode::GET_UPVALUE, line);
                self.emit_u8(idx, line);
            } else {
                self.emit_opcode(OpCode::LOAD_LOCAL, line);
                self.emit_u8(*slot, line);
            }
        }

        // 11. Emit CLOSURE_NEW with the function index and upvalue count.
        //     The VM pops `captured.len()` values from the stack (in reverse
        //     order, then reverses them) to populate the closure's upvalues.
        self.emit_opcode(OpCode::CLOSURE_NEW, line);
        self.emit_u16(fn_idx, line);
        self.emit_u8(captured.len() as u8, line);

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Free-variable analysis for closure capture
// ---------------------------------------------------------------------------

/// Helper that walks a closure body and returns the names of identifiers
/// referenced but not declared within the closure. These are candidates for
/// upvalue capture; the caller filters them against the enclosing scope.
struct FreeVarAnalyzer {
    free: Vec<String>,
    free_set: std::collections::HashSet<String>,
}

impl FreeVarAnalyzer {
    fn new() -> Self {
        Self {
            free: Vec::new(),
            free_set: std::collections::HashSet::new(),
        }
    }

    fn add_free(&mut self, name: &str) {
        if !self.free_set.contains(name) {
            self.free_set.insert(name.to_string());
            self.free.push(name.to_string());
        }
    }

    fn analyze_block(&mut self, block: &[ast::Stmt], declared: &std::collections::HashSet<String>) {
        // Clone so that declarations inside this block don't leak out.
        let mut block_declared = declared.clone();
        for stmt in block {
            self.analyze_stmt(stmt, &mut block_declared);
        }
    }

    fn analyze_stmt(&mut self, stmt: &ast::Stmt, declared: &mut std::collections::HashSet<String>) {
        match stmt {
            ast::Stmt::Block(block) => {
                self.analyze_block(block, declared);
            }
            ast::Stmt::Expr(expr) => {
                self.analyze_expr(expr, declared);
            }
            ast::Stmt::If(if_stmt) => {
                self.analyze_expr(&if_stmt.condition, declared);
                self.analyze_block(&if_stmt.then_branch, declared);
                if let Some(else_branch) = &if_stmt.else_branch {
                    self.analyze_block(else_branch, declared);
                }
            }
            ast::Stmt::While(while_stmt) => {
                self.analyze_expr(&while_stmt.condition, declared);
                self.analyze_block(&while_stmt.body, declared);
            }
            ast::Stmt::DoWhile(do_while_stmt) => {
                self.analyze_block(&do_while_stmt.body, declared);
                self.analyze_expr(&do_while_stmt.condition, declared);
            }
            ast::Stmt::WhileLet(while_let_stmt) => {
                self.analyze_expr(&while_let_stmt.expr, declared);
                let mut body_declared = declared.clone();
                body_declared.insert(while_let_stmt.var_name.clone());
                self.analyze_block(&while_let_stmt.body, &body_declared);
            }
            ast::Stmt::For(for_stmt) => {
                self.analyze_expr(&for_stmt.iterable, declared);
                let mut body_declared = declared.clone();
                body_declared.insert(for_stmt.var.clone());
                self.analyze_block(&for_stmt.body, &body_declared);
            }
            ast::Stmt::CFor(cfor_stmt) => {
                let mut body_declared = declared.clone();
                if let Some(init) = &cfor_stmt.init {
                    self.analyze_stmt(init, &mut body_declared);
                }
                if let Some(cond) = &cfor_stmt.condition {
                    self.analyze_expr(cond, &body_declared);
                }
                self.analyze_block(&cfor_stmt.body, &body_declared);
                if let Some(inc) = &cfor_stmt.increment {
                    self.analyze_expr(inc, &body_declared);
                }
            }
            ast::Stmt::Return(expr) => {
                if let Some(e) = expr {
                    self.analyze_expr(e, declared);
                }
            }
            ast::Stmt::Break | ast::Stmt::Continue => {}
            ast::Stmt::Switch(switch_stmt) => {
                self.analyze_expr(&switch_stmt.expr, declared);
                for case in &switch_stmt.cases {
                    let mut case_declared = declared.clone();
                    if let ast::Pattern::Constructor { bindings, .. } = &case.pattern {
                        for b in bindings {
                            case_declared.insert(b.clone());
                        }
                    }
                    self.analyze_block(&case.body, &case_declared);
                }
                if let Some(default) = &switch_stmt.default {
                    self.analyze_block(default, declared);
                }
            }
            ast::Stmt::With(with_stmt) => {
                self.analyze_expr(&with_stmt.resource_expr, declared);
                let mut body_declared = declared.clone();
                if let Some(name) = &with_stmt.var_name {
                    body_declared.insert(name.clone());
                }
                self.analyze_block(&with_stmt.body, &body_declared);
            }
            ast::Stmt::VarDecl(var_decl) | ast::Stmt::ConstDecl(var_decl) => {
                if let Some(init) = &var_decl.init {
                    self.analyze_expr(init, declared);
                }
                declared.insert(var_decl.name.clone());
            }
            ast::Stmt::TupleDestructure { names, expr, .. } => {
                self.analyze_expr(expr, declared);
                for name in names {
                    declared.insert(name.clone());
                }
            }
            ast::Stmt::Throw(expr, _) => {
                self.analyze_expr(expr, declared);
            }
            ast::Stmt::TryCatch { try_block, catch_var, catch_block, .. } => {
                self.analyze_block(try_block, declared);
                let mut catch_declared = declared.clone();
                catch_declared.insert(catch_var.clone());
                self.analyze_block(catch_block, &catch_declared);
            }
        }
    }

    fn analyze_expr(&mut self, expr: &ast::Expr, declared: &std::collections::HashSet<String>) {
        match expr {
            ast::Expr::Identifier(name, _) => {
                if !declared.contains(name) {
                    self.add_free(name);
                }
            }
            ast::Expr::This(_) => {
                // Closures often reference `this` from the enclosing method.
                if !declared.contains("this") {
                    self.add_free("this");
                }
            }
            ast::Expr::Super(_) => {
                if !declared.contains("this") {
                    self.add_free("this");
                }
            }
            ast::Expr::Binary(left, _, right, _) => {
                self.analyze_expr(left, declared);
                self.analyze_expr(right, declared);
            }
            ast::Expr::Unary(_, operand, _) => {
                self.analyze_expr(operand, declared);
            }
            ast::Expr::Call(callee, args, _) => {
                self.analyze_expr(callee, declared);
                for arg in args {
                    self.analyze_expr(arg, declared);
                }
            }
            ast::Expr::MemberAccess(obj, _, _) => {
                self.analyze_expr(obj, declared);
            }
            ast::Expr::Index(obj, index, _) => {
                self.analyze_expr(obj, declared);
                self.analyze_expr(index, declared);
            }
            ast::Expr::New(_, args, _) => {
                for arg in args {
                    self.analyze_expr(arg, declared);
                }
            }
            ast::Expr::OwnedDeref(inner, _) => {
                self.analyze_expr(inner, declared);
            }
            ast::Expr::RegionAlloc(_, init, _) => {
                self.analyze_expr(init, declared);
            }
            ast::Expr::RefExpr(inner, _, _) => {
                self.analyze_expr(inner, declared);
            }
            ast::Expr::UnsafeBlock(block, _) => {
                self.analyze_block(block, declared);
            }
            ast::Expr::ErrorPropagation(inner, _) => {
                self.analyze_expr(inner, declared);
            }
            ast::Expr::Cast(inner, _, _) => {
                self.analyze_expr(inner, declared);
            }
            ast::Expr::Is(inner, _, _) => {
                self.analyze_expr(inner, declared);
            }
            ast::Expr::StaticCall { args, .. } => {
                for arg in args {
                    self.analyze_expr(arg, declared);
                }
            }
            ast::Expr::Assign(target, value, _) => {
                self.analyze_expr(target, declared);
                self.analyze_expr(value, declared);
            }
            ast::Expr::Ternary { condition, then_expr, else_expr, .. } => {
                self.analyze_expr(condition, declared);
                self.analyze_expr(then_expr, declared);
                self.analyze_expr(else_expr, declared);
            }
            ast::Expr::Unit(_) => {}
            ast::Expr::Tuple(elements, _) => {
                for elem in elements {
                    self.analyze_expr(elem, declared);
                }
            }
            ast::Expr::Closure { params, body, expr, .. } => {
                // Nested closure: its params shadow, but any free vars it
                // references are also free vars of this closure (they'll be
                // captured by us and re-captured by the inner closure).
                let mut child_declared = declared.clone();
                for (name, _) in params {
                    child_declared.insert(name.clone());
                }
                if let Some(e) = expr {
                    self.analyze_expr(e, &child_declared);
                }
                self.analyze_block(body, &child_declared);
            }
            ast::Expr::Range(left, right, _) | ast::Expr::RangeInclusive(left, right, _) => {
                self.analyze_expr(left, declared);
                self.analyze_expr(right, declared);
            }
            ast::Expr::Literal(_, _) => {}
        }
    }
}

/// Walk a closure body (statements + optional expression body) and return
/// the names of identifiers referenced but not declared within the closure.
/// The result is ordered by first occurrence and deduplicated.
fn find_free_variables(
    params: &[(String, ast::Type)],
    body: &ast::Block,
    expr: &Option<Box<ast::Expr>>,
) -> Vec<String> {
    let mut declared: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (name, _) in params {
        declared.insert(name.clone());
    }

    let mut analyzer = FreeVarAnalyzer::new();
    analyzer.analyze_block(body, &declared);
    if let Some(e) = expr {
        analyzer.analyze_expr(e, &declared);
    }
    analyzer.free
}
