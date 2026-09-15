// Loop compilation (while-let, for-in, C-style for)

use crate::ast;
use super::super::opcodes::OpCode;
use super::Compiler;

impl Compiler {
    pub(super) fn compile_while_let(&mut self, while_let_stmt: &ast::WhileLetStmt) -> Result<(), String> {
        let line = while_let_stmt.span.line;
        self.begin_scope();

        let loop_start = self.current_ip();

        self.loop_stack.push(super::LoopInfo {
            continue_ip: loop_start,
            break_patches: Vec::new(),
            continue_patches: Vec::new(),
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

        // Range iteration: for (i in start..end) / for (i in start..=end).
        // The bounds evaluate once; the loop variable takes each integer.
        let range_inclusive = match &for_stmt.iterable {
            ast::Expr::Range(_, _, _) => Some(false),
            ast::Expr::RangeInclusive(_, _, _) => Some(true),
            _ => None,
        };
        if let Some(inclusive) = range_inclusive {
            let (start_expr, end_expr) = match &for_stmt.iterable {
                ast::Expr::Range(s, e, _) | ast::Expr::RangeInclusive(s, e, _) => (s, e),
                _ => unreachable!("range iterable re-matched"),
            };
            // Evaluate the end bound once and store it.
            self.compile_expr(end_expr)?;
            let end_slot = self.declare_local("__range_end")?;
            self.emit_opcode(OpCode::STORE_LOCAL, line);
            self.emit_u8(end_slot, line);

            // Initialize the counter to the start bound.
            self.compile_expr(start_expr)?;
            let idx_slot = self.declare_local("__range_idx")?;
            self.emit_opcode(OpCode::STORE_LOCAL, line);
            self.emit_u8(idx_slot, line);

            let loop_start = self.current_ip();

            // `continue` defers to a patch list so it can target the
            // increment below instead of skipping it.
            self.loop_stack.push(super::LoopInfo {
                continue_ip: super::CONTINUE_PENDING,
                break_patches: Vec::new(),
                continue_patches: Vec::new(),
            });

            // Check: idx < end (exclusive) or idx <= end (inclusive).
            self.emit_opcode(OpCode::LOAD_LOCAL, line);
            self.emit_u8(idx_slot, line);
            self.emit_opcode(OpCode::LOAD_LOCAL, line);
            self.emit_u8(end_slot, line);
            if inclusive {
                self.emit_opcode(OpCode::LE_I64, line);
            } else {
                self.emit_opcode(OpCode::LT_I64, line);
            }
            self.emit_opcode(OpCode::JMP_IF_FALSE, line);
            let exit_jump_offset = self.current_ip();
            self.emit_i16(0, line); // placeholder

            // Store the counter in the loop variable.
            self.emit_opcode(OpCode::LOAD_LOCAL, line);
            self.emit_u8(idx_slot, line);
            let loop_var_slot = self.declare_local(&for_stmt.var)?;
            self.emit_opcode(OpCode::STORE_LOCAL, line);
            self.emit_u8(loop_var_slot, line);
            self.compile_block(&for_stmt.body)?;

            // `continue` must run the increment below: patch the deferred
            // continues recorded while compiling the body to jump here.
            let incr_ip = self.current_ip();
            let continue_patches: Vec<(usize, u32)> =
                self.loop_stack.last().unwrap().continue_patches.clone();
            for (patch_offset, _line) in &continue_patches {
                let patch_instr_end = *patch_offset + 2;
                let offset = (incr_ip as isize - patch_instr_end as isize) as i16;
                self.patch_i16_at(*patch_offset, offset);
            }

            // Increment the counter: __range_idx = __range_idx + 1
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
            return Ok(());
        }

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

        // `continue` defers to a patch list so it can target the increment
        // below instead of skipping it.
        self.loop_stack.push(super::LoopInfo {
            continue_ip: super::CONTINUE_PENDING,
            break_patches: Vec::new(),
            continue_patches: Vec::new(),
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

        // `continue` must run the increment: patch the deferred continues
        // recorded while compiling the body to jump here.
        let incr_ip = self.current_ip();
        let continue_patches: Vec<(usize, u32)> =
            self.loop_stack.last().unwrap().continue_patches.clone();
        for (patch_offset, _line) in &continue_patches {
            let patch_instr_end = *patch_offset + 2;
            let offset = (incr_ip as isize - patch_instr_end as isize) as i16;
            self.patch_i16_at(*patch_offset, offset);
        }

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

        // `continue` defers to a patch list so it can target the increment
        // below instead of skipping it.
        self.loop_stack.push(super::LoopInfo {
            continue_ip: super::CONTINUE_PENDING,
            break_patches: Vec::new(),
            continue_patches: Vec::new(),
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

        // `continue` must run the increment: patch the deferred continues
        // recorded while compiling the body to jump here.
        let incr_ip = self.current_ip();
        let continue_patches: Vec<(usize, u32)> =
            self.loop_stack.last().unwrap().continue_patches.clone();
        for (patch_offset, _line) in &continue_patches {
            let patch_instr_end = *patch_offset + 2;
            let offset = (incr_ip as isize - patch_instr_end as isize) as i16;
            self.patch_i16_at(*patch_offset, offset);
        }

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
}
