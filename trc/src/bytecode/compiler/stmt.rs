// Statement compilation

use crate::ast;
use super::super::opcodes::OpCode;
use super::Compiler;

impl Compiler {
    // -----------------------------------------------------------------------
    // Second-pass compilation
    // -----------------------------------------------------------------------

    pub(super) fn compile_function(&mut self, fn_decl: &ast::FnDecl) -> Result<(), String> {
        let fn_idx = *self
            .function_map
            .get(&fn_decl.name)
            .ok_or_else(|| format!("Function '{}' not registered", fn_decl.name))?
            as usize;

        let saved_function = self.current_function;
        let saved_locals = std::mem::take(&mut self.locals);
        let saved_local_count = self.local_count;
        let saved_scope_depth = self.scope_depth;

        self.current_function = fn_idx;
        self.locals.clear();
        self.local_count = 0;
        self.scope_depth = 0;

        self.begin_scope();

        // Parameters become local variables (slot 0, 1, 2, ...).
        for param in &fn_decl.params {
            self.declare_local(&param.name)?;
        }

        self.compile_block(&fn_decl.body)?;

        // Ensure every function ends with RET.
        self.emit_opcode(OpCode::PUSH_VOID, 0);
        self.emit_opcode(OpCode::RET, 0);

        self.end_scope();

        // Store the number of local slots needed by this function.
        self.functions[fn_idx].local_count = self.local_count;

        self.current_function = saved_function;
        self.locals = saved_locals;
        self.local_count = saved_local_count;
        self.scope_depth = saved_scope_depth;

        Ok(())
    }

    pub(super) fn compile_class_methods(&mut self, class_decl: &ast::ClassDecl) -> Result<(), String> {
        let class_idx = *self
            .class_map
            .get(&class_decl.name)
            .ok_or_else(|| format!("Class '{}' not registered", class_decl.name))?
            as u16;

        let saved_class = self.current_class;
        self.current_class = Some(class_idx);

        // Compile field initialisers.
        for member in &class_decl.members {
            if let ast::ClassMember::Field(field_decl) = member {
                if let Some(ref init_expr) = field_decl.init {
                    // Find the matching field_init slot in the class def.
                    let saved_fn = self.current_function;
                    // We compile field inits into the main chunk for simplicity.
                    // The VM will execute them during object construction.
                    self.current_function = 0;
                    self.compile_expr(init_expr)?;
                    self.emit_opcode(OpCode::POP, 0);
                    self.current_function = saved_fn;
                }
            }
        }

        // Compile each method body.
        for member in &class_decl.members {
            match member {
                ast::ClassMember::Method(method_decl) => {
                    let method_fn_idx = self
                        .classes
                        .get(class_idx as usize)
                        .and_then(|c| c.methods.get(&method_decl.name))
                        .copied()
                        .ok_or_else(|| {
                            format!(
                                "Method '{}' not found in class '{}'",
                                method_decl.name, class_decl.name
                            )
                        })?;

                    self.compile_method_body(
                        method_fn_idx as usize,
                        &method_decl.params,
                        &method_decl.body,
                    )?;
                }
                ast::ClassMember::Constructor(ctor_decl) => {
                    // Find the constructor function entry by matching arity.
                    // The class may have multiple constructors (overloaded by
                    // arity).  Each was registered with name "<class>.<init>"
                    // and the corresponding arity during register_class.
                    let class_name = &class_decl.name;
                    let ctor_pattern = format!("{}.<init>", class_name);
                    let ctor_arity = ctor_decl.params.len();
                    let ctor_fn_idx = self.functions.iter().enumerate()
                        .find(|(_, f)| f.name == ctor_pattern && f.arity == ctor_arity)
                        .map(|(i, _)| i as u16)
                        .or_else(|| {
                            self.classes.get(class_idx as usize)
                                .and_then(|c| c.constructor)
                        })
                        .ok_or_else(|| {
                            format!("Constructor not found in class '{}'", class_decl.name)
                        })?;

                    self.compile_method_body(
                        ctor_fn_idx as usize,
                        &ctor_decl.params,
                        &ctor_decl.body,
                    )?;
                }
                ast::ClassMember::Field(_) => {}
            }
        }

        self.current_class = saved_class;
        Ok(())
    }

    pub(super) fn compile_method_body(
        &mut self,
        fn_idx: usize,
        params: &[ast::Param],
        body: &ast::Block,
    ) -> Result<(), String> {
        let saved_function = self.current_function;
        let saved_locals = std::mem::take(&mut self.locals);
        let saved_local_count = self.local_count;
        let saved_scope_depth = self.scope_depth;

        self.current_function = fn_idx;
        self.locals.clear();
        self.local_count = 0;
        self.scope_depth = 0;

        self.begin_scope();

        // Slot 0 = "this"
        self.declare_local("this")?;

        // Parameters start at slot 1.
        for param in params {
            self.declare_local(&param.name)?;
        }

        self.compile_block(body)?;

        // Ensure every method ends with RET.
        self.emit_opcode(OpCode::PUSH_VOID, 0);
        self.emit_opcode(OpCode::RET, 0);

        self.end_scope();

        // Store the number of local slots needed by this method.
        self.functions[fn_idx].local_count = self.local_count;

        self.current_function = saved_function;
        self.locals = saved_locals;
        self.local_count = saved_local_count;
        self.scope_depth = saved_scope_depth;

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Statement compilation
    // -----------------------------------------------------------------------

    pub(super) fn compile_block(&mut self, block: &ast::Block) -> Result<(), String> {
        for stmt in block {
            self.compile_stmt(stmt)?;
        }
        Ok(())
    }

    pub(super) fn compile_stmt(&mut self, stmt: &ast::Stmt) -> Result<(), String> {
        match stmt {
            ast::Stmt::VarDecl(var_decl) => {
                self.compile_var_decl(var_decl)?;
            }
            ast::Stmt::ConstDecl(const_decl) => {
                self.compile_var_decl(const_decl)?;
            }
            ast::Stmt::Expr(expr) => {
                self.compile_expr(expr)?;
                // Expression statements: the result stays on the stack.
                // Pop it unless it's void (the VM may optimise this).
                self.emit_opcode(OpCode::POP, 0);
            }
            ast::Stmt::If(if_stmt) => {
                self.compile_if(if_stmt)?;
            }
            ast::Stmt::While(while_stmt) => {
                self.compile_while(while_stmt)?;
            }
            ast::Stmt::DoWhile(do_while_stmt) => {
                self.compile_do_while(do_while_stmt)?;
            }
            ast::Stmt::WhileLet(while_let_stmt) => {
                self.compile_while_let(while_let_stmt)?;
            }
            ast::Stmt::For(for_stmt) => {
                self.compile_for(for_stmt)?;
            }
            ast::Stmt::CFor(cfor_stmt) => {
                self.compile_c_for(cfor_stmt)?;
            }
            ast::Stmt::Return(expr) => {
                if let Some(value) = expr {
                    self.compile_expr(value)?;
                } else {
                    self.emit_opcode(OpCode::PUSH_VOID, 0);
                }
                self.emit_opcode(OpCode::RET, 0);
            }
            ast::Stmt::Break => {
                self.compile_break(0)?;
            }
            ast::Stmt::Continue => {
                self.compile_continue(0)?;
            }
            ast::Stmt::Switch(switch_stmt) => {
                self.compile_switch(switch_stmt)?;
            }
            ast::Stmt::With(with_stmt) => {
                self.compile_with(with_stmt)?;
            }
            ast::Stmt::Block(block) => {
                self.begin_scope();
                self.compile_block(block)?;
                self.end_scope();
            }
            ast::Stmt::TupleDestructure { names, expr, mutable: _, span } => {
                self.compile_tuple_destructure(names, expr, span.line)?;
            }
            ast::Stmt::Throw(expr, span) => {
                // Compile the expression and pop it (throw is a runtime concern)
                self.compile_expr(expr)?;
                self.emit_opcode(OpCode::POP, span.line);
            }
            ast::Stmt::TryCatch { try_block, catch_var, catch_var_type: _, catch_block, span: _ } => {
                // Compile try block; on error, the VM would jump to catch.
                // For the bytecode VM, we compile both blocks sequentially.
                self.compile_block(try_block)?;
                let slot = self.declare_local(catch_var)?;
                self.emit_opcode(OpCode::PUSH_NULL, 0);
                self.emit_opcode(OpCode::STORE_LOCAL, 0);
                self.emit_u8(slot, 0);
                self.compile_block(catch_block)?;
            }
        }
        Ok(())
    }

    pub(super) fn compile_var_decl(&mut self, var_decl: &ast::VarDecl) -> Result<(), String> {
        let line = var_decl.span.line;
        if let Some(ref init) = var_decl.init {
            self.compile_expr(init)?;
        } else {
            self.emit_opcode(OpCode::PUSH_NULL, line);
        }
        let slot = self.declare_local(&var_decl.name)?;
        self.emit_opcode(OpCode::STORE_LOCAL, line);
        self.emit_u8(slot, line);
        Ok(())
    }

    pub(super) fn compile_tuple_destructure(&mut self, names: &[String], expr: &ast::Expr, line: u32) -> Result<(), String> {
        // Compile the tuple expression, then extract each element
        self.compile_expr(expr)?;
        // For each name, duplicate the tuple, get element at index, store to local
        for (i, name) in names.iter().enumerate() {
            self.emit_opcode(OpCode::DUP, line);
            self.emit_opcode(OpCode::TUPLE_GET, line);
            self.emit_u8(i as u8, line);
            let slot = self.declare_local(name)?;
            self.emit_opcode(OpCode::STORE_LOCAL, line);
            self.emit_u8(slot, line);
        }
        // Pop the remaining tuple from the stack
        self.emit_opcode(OpCode::POP, line);
        Ok(())
    }

    pub(super) fn compile_if(&mut self, if_stmt: &ast::IfStmt) -> Result<(), String> {
        let line = if_stmt.span.line;
        // compile condition
        self.compile_expr(&if_stmt.condition)?;
        self.emit_opcode(OpCode::JMP_IF_FALSE, line);
        let else_jump_offset = self.current_ip();
        self.emit_i16(0, line); // placeholder

        // compile then_branch
        self.compile_block(&if_stmt.then_branch)?;

        if let Some(ref else_branch) = if_stmt.else_branch {
            // Jump over else branch after then.
            self.emit_opcode(OpCode::JMP, line);
            let end_jump_offset = self.current_ip();
            self.emit_i16(0, line); // placeholder

            // Patch else jump to land here.
            let else_start = self.current_ip();
            let then_instr_end = else_jump_offset + 2; // past the i16 operand
            let offset = (else_start - then_instr_end) as i16;
            self.patch_i16_at(else_jump_offset, offset);

            // compile else_branch
            self.compile_block(else_branch)?;

            // Patch end jump.
            let end_ip = self.current_ip();
            let end_instr_end = end_jump_offset + 2;
            let offset = (end_ip - end_instr_end) as i16;
            self.patch_i16_at(end_jump_offset, offset);
        } else {
            // No else branch: patch the JMP_IF_FALSE to jump to end.
            let end_ip = self.current_ip();
            let then_instr_end = else_jump_offset + 2;
            let offset = (end_ip - then_instr_end) as i16;
            self.patch_i16_at(else_jump_offset, offset);
        }

        Ok(())
    }

    pub(super) fn compile_while(&mut self, while_stmt: &ast::WhileStmt) -> Result<(), String> {
        let line = while_stmt.span.line;
        let loop_start = self.current_ip();

        self.loop_stack.push(super::LoopInfo {
            continue_ip: loop_start,
            break_patches: Vec::new(),
        });

        // compile condition
        self.compile_expr(&while_stmt.condition)?;
        self.emit_opcode(OpCode::JMP_IF_FALSE, line);
        let exit_jump_offset = self.current_ip();
        self.emit_i16(0, line); // placeholder

        // compile body
        self.compile_block(&while_stmt.body)?;

        // Jump back to loop start.
        self.emit_opcode(OpCode::JMP, line);
        let current = self.current_ip() + 2; // after the i16 operand
        let offset = (loop_start as isize - current as isize) as i16;
        self.emit_i16(offset, line);

        // Patch the exit jump.
        let end_ip = self.current_ip();
        let exit_instr_end = exit_jump_offset + 2;
        let offset = (end_ip - exit_instr_end) as i16;
        self.patch_i16_at(exit_jump_offset, offset);

        // Patch all break jumps.
        let loop_info = self.loop_stack.pop().unwrap();
        for (patch_offset, _line) in &loop_info.break_patches {
            let patch_instr_end = *patch_offset + 2;
            let offset = (end_ip - patch_instr_end) as i16;
            self.patch_i16_at(*patch_offset, offset);
        }

        Ok(())
    }

    pub(super) fn compile_do_while(&mut self, do_while_stmt: &ast::DoWhileStmt) -> Result<(), String> {
        let line = do_while_stmt.span.line;
        let loop_start = self.current_ip();

        // Push with a placeholder continue_ip; we'll update it after compiling
        // the body so that `continue` jumps to the condition check, not the
        // body start.
        self.loop_stack.push(super::LoopInfo {
            continue_ip: loop_start, // placeholder, updated below
            break_patches: Vec::new(),
        });

        // Compile body first (guaranteed first execution)
        self.compile_block(&do_while_stmt.body)?;

        // Now we know where the condition check begins — update continue_ip
        // so that `continue` jumps here instead of back to the body start.
        let condition_ip = self.current_ip();
        self.loop_stack.last_mut().unwrap().continue_ip = condition_ip;

        // Then compile condition
        self.compile_expr(&do_while_stmt.condition)?;

        // If condition is true, jump back to body start
        self.emit_opcode(OpCode::JMP_IF_TRUE, line);
        let current = self.current_ip() + 2; // after the i16 operand
        let offset = (loop_start as isize - current as isize) as i16;
        self.emit_i16(offset, line);

        // Patch all break jumps.
        let end_ip = self.current_ip();
        let loop_info = self.loop_stack.pop().unwrap();
        for (patch_offset, _line) in &loop_info.break_patches {
            let patch_instr_end = *patch_offset + 2;
            let offset = (end_ip - patch_instr_end) as i16;
            self.patch_i16_at(*patch_offset, offset);
        }

        Ok(())
    }

    pub(super) fn compile_while_let(&mut self, while_let_stmt: &ast::WhileLetStmt) -> Result<(), String> {
        let line = while_let_stmt.span.line;
        self.begin_scope();

        let loop_start = self.current_ip();

        self.loop_stack.push(super::LoopInfo {
            continue_ip: loop_start,
            break_patches: Vec::new(),
        });

        // Compile the expression (e.g., file.readLine()?)
        self.compile_expr(&while_let_stmt.expr)?;

        // MATCH_OK: checks if top of stack is ResultOk
        // If ResultOk, replaces with inner value and pushes true
        // Otherwise, pushes false
        self.emit_opcode(OpCode::MATCH_OK, line);
        self.emit_i16(0, line); // placeholder jump offset (consumed but not used for jumping)

        // Stack: [inner_value, true] if ResultOk, or [false] if not
        // JMP_IF_FALSE pops the top (the bool) and jumps if false
        self.emit_opcode(OpCode::JMP_IF_FALSE, line);
        let exit_jump_offset = self.current_ip();
        self.emit_i16(0, line); // placeholder

        // Stack: [inner_value] — store it in the variable
        let slot = self.declare_local(&while_let_stmt.var_name)?;
        self.emit_opcode(OpCode::STORE_LOCAL, line);
        self.emit_u8(slot, line);

        // Compile body
        self.compile_block(&while_let_stmt.body)?;

        // Jump back to loop start
        self.emit_opcode(OpCode::JMP, line);
        let current = self.current_ip() + 2;
        let offset = (loop_start as isize - current as isize) as i16;
        self.emit_i16(offset, line);

        // Patch the exit jump
        let end_ip = self.current_ip();
        let exit_instr_end = exit_jump_offset + 2;
        let offset = (end_ip - exit_instr_end) as i16;
        self.patch_i16_at(exit_jump_offset, offset);

        // Patch all break jumps.
        let loop_info = self.loop_stack.pop().unwrap();
        for (patch_offset, _line) in &loop_info.break_patches {
            let patch_instr_end = *patch_offset + 2;
            let offset = (end_ip - patch_instr_end) as i16;
            self.patch_i16_at(*patch_offset, offset);
        }

        self.end_scope();
        Ok(())
    }

    pub(super) fn compile_for(&mut self, for_stmt: &ast::ForStmt) -> Result<(), String> {
        let line = for_stmt.span.line;
        self.begin_scope();

        // Compile the iterable expression and store it in a local.
        self.compile_expr(&for_stmt.iterable)?;
        let iter_slot = self.declare_local("__iter")?;
        self.emit_opcode(OpCode::STORE_LOCAL, line);
        self.emit_u8(iter_slot, line);

        // Initialize the index counter to 0.
        self.emit_opcode(OpCode::PUSH_I64, line);
        let bytes = 0i64.to_be_bytes();
        for &b in &bytes {
            self.emit_u8(b, line);
        }
        let idx_slot = self.declare_local("__iter_idx")?;
        self.emit_opcode(OpCode::STORE_LOCAL, line);
        self.emit_u8(idx_slot, line);

        // Get the length of the iterable and store it.
        self.emit_opcode(OpCode::LOAD_LOCAL, line);
        self.emit_u8(iter_slot, line);
        self.emit_opcode(OpCode::ARRAY_LEN, line);
        let len_slot = self.declare_local("__iter_len")?;
        self.emit_opcode(OpCode::STORE_LOCAL, line);
        self.emit_u8(len_slot, line);

        let loop_start = self.current_ip();

        self.loop_stack.push(super::LoopInfo {
            continue_ip: loop_start,
            break_patches: Vec::new(),
        });

        // Check: idx < len
        self.emit_opcode(OpCode::LOAD_LOCAL, line);
        self.emit_u8(idx_slot, line);
        self.emit_opcode(OpCode::LOAD_LOCAL, line);
        self.emit_u8(len_slot, line);
        self.emit_opcode(OpCode::LT_I64, line);
        self.emit_opcode(OpCode::JMP_IF_FALSE, line);
        let exit_jump_offset = self.current_ip();
        self.emit_i16(0, line); // placeholder

        // Load the element: __iter[__iter_idx]
        self.emit_opcode(OpCode::LOAD_LOCAL, line);
        self.emit_u8(iter_slot, line);
        self.emit_opcode(OpCode::LOAD_LOCAL, line);
        self.emit_u8(idx_slot, line);
        self.emit_opcode(OpCode::ARRAY_GET, line);

        // Store the element in the loop variable.
        let loop_var_slot = self.declare_local(&for_stmt.var)?;
        self.emit_opcode(OpCode::STORE_LOCAL, line);
        self.emit_u8(loop_var_slot, line);
        self.compile_block(&for_stmt.body)?;

        // Increment the index: __iter_idx = __iter_idx + 1
        self.emit_opcode(OpCode::LOAD_LOCAL, line);
        self.emit_u8(idx_slot, line);
        self.emit_opcode(OpCode::PUSH_I64, line);
        let one_bytes = 1i64.to_be_bytes();
        for &b in &one_bytes {
            self.emit_u8(b, line);
        }
        self.emit_opcode(OpCode::ADD_I64, line);
        self.emit_opcode(OpCode::STORE_LOCAL, line);
        self.emit_u8(idx_slot, line);

        // Jump back to loop start.
        self.emit_opcode(OpCode::JMP, line);
        let current = self.current_ip() + 2;
        let offset = (loop_start as isize - current as isize) as i16;
        self.emit_i16(offset, line);

        // Patch the exit jump.
        let end_ip = self.current_ip();
        let exit_instr_end = exit_jump_offset + 2;
        let offset = (end_ip - exit_instr_end) as i16;
        self.patch_i16_at(exit_jump_offset, offset);

        // Patch all break jumps.
        let loop_info = self.loop_stack.pop().unwrap();
        for (patch_offset, _line) in &loop_info.break_patches {
            let patch_instr_end = *patch_offset + 2;
            let offset = (end_ip - patch_instr_end) as i16;
            self.patch_i16_at(*patch_offset, offset);
        }

        self.end_scope();
        Ok(())
    }

    pub(super) fn compile_c_for(&mut self, cfor_stmt: &ast::CForStmt) -> Result<(), String> {
        let line = cfor_stmt.span.line;
        self.begin_scope();

        // Compile init statement
        if let Some(ref init) = cfor_stmt.init {
            self.compile_stmt(init)?;
        }

        let loop_start = self.current_ip();

        self.loop_stack.push(super::LoopInfo {
            continue_ip: loop_start,
            break_patches: Vec::new(),
        });

        // Compile condition (if present)
        let exit_jump_offset = if let Some(ref cond) = cfor_stmt.condition {
            self.compile_expr(cond)?;
            self.emit_opcode(OpCode::JMP_IF_FALSE, line);
            let offset = self.current_ip();
            self.emit_i16(0, line); // placeholder
            Some(offset)
        } else {
            None
        };

        // Compile body
        self.compile_block(&cfor_stmt.body)?;

        // Compile increment (if present)
        if let Some(ref incr) = cfor_stmt.increment {
            self.compile_expr(incr)?;
            self.emit_opcode(OpCode::POP, line); // discard increment result
        }

        // Jump back to loop start
        self.emit_opcode(OpCode::JMP, line);
        let current = self.current_ip() + 2;
        let offset = (loop_start as isize - current as isize) as i16;
        self.emit_i16(offset, line);

        // Patch the exit jump
        let end_ip = self.current_ip();
        if let Some(exit_offset) = exit_jump_offset {
            let exit_instr_end = exit_offset + 2;
            let offset = (end_ip - exit_instr_end) as i16;
            self.patch_i16_at(exit_offset, offset);
        }

        // Patch all break jumps
        let loop_info = self.loop_stack.pop().unwrap();
        for (patch_offset, _line) in &loop_info.break_patches {
            let patch_instr_end = *patch_offset + 2;
            let offset = (end_ip - patch_instr_end) as i16;
            self.patch_i16_at(*patch_offset, offset);
        }

        self.end_scope();
        Ok(())
    }

    pub(super) fn compile_break(&mut self, line: u32) -> Result<(), String> {
        if self.loop_stack.is_empty() {
            return Err("'break' outside of loop".to_string());
        }
        self.emit_opcode(OpCode::JMP, line);
        let patch_offset = self.current_ip();
        self.emit_i16(0, line); // placeholder
        self.loop_stack
            .last_mut()
            .unwrap()
            .break_patches
            .push((patch_offset, line));
        Ok(())
    }

    pub(super) fn compile_continue(&mut self, line: u32) -> Result<(), String> {
        if self.loop_stack.is_empty() {
            return Err("'continue' outside of loop".to_string());
        }
        let continue_ip = self.loop_stack.last().unwrap().continue_ip;
        self.emit_opcode(OpCode::JMP, line);
        let current = self.current_ip() + 2;
        let offset = (continue_ip as isize - current as isize) as i16;
        self.emit_i16(offset, line);
        Ok(())
    }

    pub(super) fn compile_switch(&mut self, switch_stmt: &ast::SwitchStmt) -> Result<(), String> {
        let line = switch_stmt.span.line;
        // Compile the subject expression.
        self.compile_expr(&switch_stmt.expr)?;

        let mut end_jumps: Vec<usize> = Vec::new();

        for case in &switch_stmt.cases {
            // DUP the subject for matching.
            self.emit_opcode(OpCode::DUP, line);

            // Compile the pattern match.
            self.compile_pattern_match(&case.pattern)?;

            // If pattern doesn't match, jump to next case.
            self.emit_opcode(OpCode::JMP_IF_FALSE, line);
            let next_case_offset = self.current_ip();
            self.emit_i16(0, line); // placeholder

            // Pattern matched: store extracted fields into local variables.
            if let ast::Pattern::Constructor { bindings, .. } = &case.pattern {
                if !bindings.is_empty() {
                    // Fields are on stack in order (first deepest, last on top).
                    // Store them in reverse order.
                    for binding in bindings.iter().rev() {
                        if binding != "_" {
                            let slot = self.declare_local(binding)?;
                            self.emit_opcode(OpCode::STORE_LOCAL, line);
                            self.emit_u8(slot, line);
                        } else {
                            // Wildcard: just pop the field.
                            self.emit_opcode(OpCode::POP, line);
                        }
                    }
                }
            }

            // POP the subject.
            self.emit_opcode(OpCode::POP, line);

            // Compile case body.
            self.compile_block(&case.body)?;

            // Jump to end of switch (so we don't fall through).
            self.emit_opcode(OpCode::JMP, line);
            let end_jump_offset = self.current_ip();
            self.emit_i16(0, line); // placeholder
            end_jumps.push(end_jump_offset);

            // Patch the next-case jump to land here.
            let next_ip = self.current_ip();
            let next_instr_end = next_case_offset + 2;
            let offset = (next_ip - next_instr_end) as i16;
            self.patch_i16_at(next_case_offset, offset);
        }

        // Default case (if any).
        if let Some(ref default_body) = switch_stmt.default {
            // POP the subject (no case matched).
            self.emit_opcode(OpCode::POP, line);
            self.compile_block(default_body)?;
        } else {
            // No default: just pop the subject.
            self.emit_opcode(OpCode::POP, line);
        }

        // Patch all end jumps.
        let end_ip = self.current_ip();
        for offset in &end_jumps {
            let instr_end = *offset + 2;
            let jump = (end_ip - instr_end) as i16;
            self.patch_i16_at(*offset, jump);
        }

        Ok(())
    }

    pub(super) fn compile_with(&mut self, with_stmt: &ast::WithStmt) -> Result<(), String> {
        let line = with_stmt.span.line;

        // Compile the resource expression — result on stack.
        self.compile_expr(&with_stmt.resource_expr)?;

        // Begin a new scope for the with-statement.
        self.begin_scope();

        // Store the resource into a local variable.
        let var_name = match &with_stmt.var_name {
            Some(name) => name.clone(),
            None => "__with_resource".to_string(),
        };
        let resource_slot = self.declare_local(&var_name)?;
        self.emit_opcode(OpCode::STORE_LOCAL, line);
        self.emit_u8(resource_slot, line);

        // Compile the body.
        self.compile_block(&with_stmt.body)?;

        // After the body completes normally, call .close() on the resource.
        // Load the resource, invoke close(), pop the result.
        self.emit_opcode(OpCode::LOAD_LOCAL, line);
        self.emit_u8(resource_slot, line);
        let close_idx = self.intern_string("close");
        self.emit_opcode(OpCode::INVOKE_VIRTUAL, line);
        self.emit_u16(close_idx, line);
        self.emit_u8(0, line); // 0 args
        self.emit_opcode(OpCode::POP, line); // discard close() return

        self.end_scope();

        Ok(())
    }

    pub(super) fn compile_pattern_match(&mut self, pattern: &ast::Pattern) -> Result<(), String> {
        match pattern {
            ast::Pattern::Literal(lit) => {
                // Compile the literal, then emit equality comparison.
                self.compile_literal(lit, 0)?;
                let ty = self.infer_literal_type(lit);
                self.emit_eq_opcode(ty, 0);
            }
            ast::Pattern::Wildcard => {
                // Always matches. Replace the DUP'd subject with true.
                self.emit_opcode(OpCode::POP, 0);
                self.emit_opcode(OpCode::PUSH_BOOL, 0);
                self.emit_u8(1, 0);
            }
            ast::Pattern::Constructor { name, bindings } => {
                // Check if this is an enum variant.
                if let Some((_enum_name, _variant_idx)) = self.variant_map.get(name) {
                    // MATCH_ENUM: u16 variant name index + i16 jump offset
                    let variant_idx = self.intern_string(name);
                    self.emit_opcode(OpCode::MATCH_ENUM, 0);
                    self.emit_u16(variant_idx, 0);
                    // The jump offset for MATCH_ENUM is handled by the VM at runtime.
                    // We emit 0 as placeholder; the VM reads the variant name and decides.
                    self.emit_i16(0, 0);
                } else if name == "Ok" {
                    self.emit_opcode(OpCode::MATCH_OK, 0);
                    self.emit_i16(0, 0);
                } else if name == "Err" {
                    self.emit_opcode(OpCode::MATCH_ERR, 0);
                    self.emit_i16(0, 0);
                } else {
                    return Err(format!("Unknown pattern constructor '{}'", name));
                }

                // If there are bindings, we don't need to do anything extra here;
                // the VM will extract the fields after a successful match.
                let _ = bindings;
            }
        }
        Ok(())
    }
}
