// Expression compilation

use crate::ast;
use super::super::opcodes::{OpCode, CastTarget};
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
                let bytes = (*v).to_be_bytes();
                for &b in &bytes {
                    self.emit_u8(b, line);
                }
            }
            ast::Literal::Float(v) => {
                self.emit_opcode(OpCode::PUSH_F64, line);
                let bytes = (*v).to_be_bytes();
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
            // Bare function reference (e.g. passing a top-level function as
            // an argument). Package it as a Closure value with no upvalues so
            // that CALL_CLOSURE can later invoke it.
            self.emit_opcode(OpCode::CLOSURE_NEW, line);
            self.emit_u16(fn_idx, line);
            self.emit_u8(0, line);
            return Ok(());
        }

        // Same-module functions referenced bare (e.g. passing a private
        // callback like _strictHandler). They live in function_map under
        // the current module's mangled name.
        if !self.current_module.is_empty() {
            let qualified = format!("{}.{}", self.current_module, name);
            if let Some(&fn_idx) = self.function_map.get(&qualified) {
                self.emit_opcode(OpCode::CLOSURE_NEW, line);
                self.emit_u16(fn_idx, line);
                self.emit_u8(0, line);
                return Ok(());
            }
        }

        // Check the imported symbol table.
        if let Some(symbol) = self.symbol_table.get(name).cloned() {
            match symbol {
                Symbol::Function(fn_idx) => {
                    // Imported function reference – package as a Closure.
                    self.emit_opcode(OpCode::CLOSURE_NEW, line);
                    self.emit_u16(fn_idx, line);
                    self.emit_u8(0, line);
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
                Symbol::Global(global_idx) => {
                    // Imported public const — load from the global slot that
                    // was registered for the mangled name in the defining module.
                    self.emit_opcode(OpCode::LOAD_GLOBAL, line);
                    self.emit_u16(global_idx, line);
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

        // Unknown identifier. There is no sound fallback: every name that
        // can resolve (locals, globals, functions, imports, variants) was
        // checked above, so emitting any slot would read an unrelated
        // variable. Report it honestly instead of corrupting the program.
        Err(format!("unknown variable '{}'", name))
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
pub(super) fn find_free_variables(
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
