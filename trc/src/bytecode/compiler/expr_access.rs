// Member access, construction, assignment, and closure compilation

use crate::ast;
use super::super::opcodes::{OpCode, TypeTag};
use super::{expr::find_free_variables, Compiler, Symbol};

impl Compiler {
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
            // Cross-module public const access (e.g. Token.INDENT). Only
            // when obj is not a local or global, so real field access on
            // variables is unaffected.
            if self.resolve_local(obj_name).is_none() && self.lookup_global(obj_name).is_none() {
                let module_suffix = format!(".{}", obj_name);
                let mut const_slot: Option<u16> = None;
                for m in &self.modules {
                    if m.name != *obj_name && !m.name.ends_with(&module_suffix) {
                        continue;
                    }
                    let Some(ref prog) = m.program else {
                        continue;
                    };
                    let is_const = prog.declarations.iter().any(|d| {
                        matches!(d, ast::Declaration::ConstDecl(c) if c.name == member)
                    });
                    if is_const {
                        let mangled = format!("{}.{}", m.name, member);
                        if let Some(&slot) = self.global_map.get(&mangled) {
                            const_slot = Some(slot);
                            break;
                        }
                    }
                }
                if let Some(slot) = const_slot {
                    self.emit_opcode(OpCode::LOAD_GLOBAL, line);
                    self.emit_u16(slot, line);
                    return Ok(());
                }
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
                // Try all modules: register public classes from any module that
                // contains a class with this name.
                let mut found_idx = None;
                // First try class_map suffix match across all keys
                let suffix = format!(".{}", class_name);
                if let Some(key) = self.class_map.keys().find(|k| k.ends_with(&suffix)).cloned() {
                    found_idx = self.class_map.get(&key).copied();
                }
                if found_idx.is_none() {
                    for m in &self.modules {
                        if let Some(ref prog) = m.program {
                            for decl in &prog.declarations {
                                if let ast::Declaration::Class(c) = decl {
                                    if c.name == *class_name {
                                        let mangled = format!("{}.{}", m.name, class_name);
                                        if let Some(&idx) = self.class_map.get(&mangled) {
                                            found_idx = Some(idx);
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        if found_idx.is_some() { break; }
                    }
                }
                if let Some(idx) = found_idx {
                    idx
                } else {
                    let keys: Vec<String> = self.class_map.keys().take(20).cloned().collect();
                    return Err(format!("Unknown class '{}' in new expression (class_map has {} entries: {:?})", class_name, self.class_map.len(), keys));
                }
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
        // Resolve `Class.method` to a top-level module function when one
        // exists. The VM's STATIC_CALL resolution prefers natives (the
        // `Module_function` convention, e.g. `Json_parse`), which would
        // otherwise shadow a module function with the same logical name
        // (e.g. `Json.parse` in tt/json/Json.tr) and bypass its wrapping
        // logic. Resolving here keeps the .tr wrapper as the target, while
        // the wrapper's own bare `Json_parse(...)` delegation still reaches
        // the native (it compiles to STATIC_CALL, and the wrapper function
        // is found at compile time only when the arity matches this call).
        let target_arity = args.len();
        let suffix = format!(".{}.{}", class_name, method);
        let module_fn = self.functions.iter().enumerate()
            .find(|(_, f)| f.name.ends_with(&suffix) && !f.is_method && f.arity == target_arity)
            .map(|(i, _)| i as u16);
        if let Some(fn_idx) = module_fn {
            for arg in args {
                self.compile_expr(arg)?;
            }
            self.emit_opcode(OpCode::CALL, line);
            self.emit_u16(fn_idx, line);
            self.emit_u8(target_arity as u8, line);
            return Ok(());
        }

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
        //    enclosing local. A user-declared `self` local wins; otherwise
        //    `self` captures the method receiver (`this`), matching
        //    resolve_local.
        let mut captured: Vec<(String, u8)> = Vec::new();
        for var_name in &free_vars {
            let lookup_name = if var_name == "self"
                && !enclosing_locals.iter().rev().any(|l| l.name == "self")
            {
                "this"
            } else {
                var_name
            };
            if let Some(local) = enclosing_locals.iter().rev().find(|l| l.name == lookup_name) {
                // Store under the resolved name so the upvalue local can be
                // found by `resolve_local` (same actual-first rule).
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
        let param_types: Vec<String> = params.iter().map(|(_, t)| t.name().to_string()).collect();

        self.functions.push(super::FunctionDef {
            name: closure_name.clone(),
            arity,
            chunk: super::Chunk::new(),
            is_method: false,
            is_constructor: false,
            local_count: 0,
            param_types,
        });
        self.function_map.insert(closure_name.clone(), fn_idx);

        // 5. Save current compilation state, then start a fresh scope for
        //    the closure body.
        let saved_function = self.current_function;
        let saved_locals = std::mem::take(&mut self.locals);
        let saved_local_count = self.local_count;
        let saved_scope_depth = self.scope_depth;
        let saved_handler_depth = self.handler_depth;

        self.current_function = fn_idx as usize;
        self.locals.clear();
        self.local_count = 0;
        self.scope_depth = 0;
        self.handler_depth = 0;

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
        self.handler_depth = saved_handler_depth;

        // 10. In the enclosing scope, push each captured variable's value
        //     onto the stack so CLOSURE_NEW can pop them into the upvalue
        //     vector. If the enclosing local is itself an upvalue (nested
        //     closure scenario), emit GET_UPVALUE instead of LOAD_LOCAL.
        //
        //     For direct locals we emit CAPTURE_LOCAL instead of LOAD_LOCAL.
        //     CAPTURE_LOCAL replaces the local's stack slot with a shared
        //     `Value::Cell` and pushes that Cell. CLOSURE_NEW then extracts
        //     the inner `Rc<RefCell<Value>>` so that mutations through
        //     SET_UPVALUE are visible to the enclosing scope (and vice versa).
        for (_name, slot) in &captured {
            let is_uv = enclosing_locals
                .iter()
                .rev()
                .find(|l| l.slot == *slot)
                .is_some_and(|l| l.is_upvalue);
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
                self.emit_opcode(OpCode::CAPTURE_LOCAL, line);
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
