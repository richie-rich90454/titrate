// Optimization passes – constant folding, dead code elimination, unused string removal

use std::collections::{HashMap, HashSet};

use super::super::opcodes::OpCode;
use super::{Chunk, Compiler, DecodedInstr};

impl Compiler {
    /// Decode all instructions in a chunk into a vector of `DecodedInstr`.
    pub(super) fn decode_instructions(chunk: &Chunk) -> Vec<DecodedInstr> {
        let mut instructions = Vec::new();
        let mut byte_offsets = Vec::new(); // instruction index → byte offset
        let mut ip = 0;

        while ip < chunk.code.len() {
            let op_byte = chunk.code[ip];
            if let Ok(op) = OpCode::try_from(op_byte) {
                let operand_size = op.operand_size();
                byte_offsets.push(ip);
                let operands = if ip + 1 + operand_size <= chunk.code.len() {
                    chunk.code[ip + 1..ip + 1 + operand_size].to_vec()
                } else {
                    Vec::new()
                };
                let line = chunk.source_lines.get(ip).copied().unwrap_or(0);
                instructions.push(DecodedInstr {
                    opcode: op,
                    operands,
                    line,
                    jump_target_idx: None,
                });
                ip += 1 + operand_size;
            } else {
                ip += 1;
            }
        }

        // Build byte_offset → instruction_index mapping.
        let mut offset_to_idx = HashMap::new();
        for (idx, &offset) in byte_offsets.iter().enumerate() {
            offset_to_idx.insert(offset, idx);
        }

        // Resolve jump targets to instruction indices.
        for i in 0..instructions.len() {
            let instr = &instructions[i];
            let instr_start = byte_offsets[i];

            // PUSH_HANDLER stores an absolute u16 catch_ip (not a relative
            // offset like JMP).  Resolve it to an instruction index so the
            // dead-code eliminator knows the catch block is reachable.
            if instr.opcode == OpCode::PUSH_HANDLER && instr.operands.len() >= 2 {
                let abs_target = u16::from_be_bytes([instr.operands[0], instr.operands[1]]) as usize;
                let mut best = 0usize;
                for (idx, &off) in byte_offsets.iter().enumerate() {
                    if off <= abs_target {
                        best = idx;
                    } else {
                        break;
                    }
                }
                instructions[i].jump_target_idx = Some(best);
                continue;
            }

            let relative_offset: Option<i16> = match instr.opcode {
                OpCode::JMP
                | OpCode::JMP_IF_FALSE
                | OpCode::JMP_IF_TRUE
                | OpCode::ITER_NEXT
                | OpCode::MATCH_OK
                | OpCode::MATCH_ERR => {
                    if instr.operands.len() >= 2 {
                        Some(i16::from_be_bytes([instr.operands[0], instr.operands[1]]))
                    } else {
                        None
                    }
                }
                OpCode::MATCH_ENUM => {
                    if instr.operands.len() >= 4 {
                        Some(i16::from_be_bytes([instr.operands[2], instr.operands[3]]))
                    } else {
                        None
                    }
                }
                _ => None,
            };

            if let Some(rel) = relative_offset {
                // The IP after reading the full jump instruction:
                let ip_after = match instr.opcode {
                    OpCode::MATCH_ENUM => instr_start + 5,
                    OpCode::ITER_NEXT => instr_start + 3,
                    _ => instr_start + 3,
                };
                let abs_target = (ip_after as isize + rel as isize) as usize;

                // Find the instruction index at the target byte offset.
                // Use the closest instruction at or before the target.
                let mut best = 0usize;
                for (idx, &off) in byte_offsets.iter().enumerate() {
                    if off <= abs_target {
                        best = idx;
                    } else {
                        break;
                    }
                }
                instructions[i].jump_target_idx = Some(best);
            }
        }

        instructions
    }

    /// Encode a vector of decoded instructions back into a chunk, adjusting
    /// jump offsets to account for bytecode size changes from optimization.
    pub(super) fn encode_instructions(chunk: &mut Chunk, instructions: &[DecodedInstr]) {
        // Compute new byte offsets for each instruction.
        let mut new_byte_offsets = Vec::with_capacity(instructions.len());
        let mut offset = 0usize;
        for instr in instructions {
            new_byte_offsets.push(offset);
            offset += 1 + instr.operands.len();
        }

        chunk.code.clear();
        chunk.source_lines.clear();

        for (i, instr) in instructions.iter().enumerate() {
            chunk.code.push(instr.opcode as u8);
            chunk.source_lines.push(instr.line);

            // Compute adjusted operands for jump instructions.
            let adjusted_operands: Vec<u8> = match instr.opcode {
                OpCode::JMP
                | OpCode::JMP_IF_FALSE
                | OpCode::JMP_IF_TRUE
                | OpCode::ITER_NEXT
                | OpCode::MATCH_OK
                | OpCode::MATCH_ERR => {
                    if let Some(target_idx) = instr.jump_target_idx {
                        if target_idx < instructions.len() {
                            let new_target = new_byte_offsets[target_idx] as isize;
                            let new_ip_after = (new_byte_offsets[i] + 3) as isize;
                            let new_rel = (new_target - new_ip_after) as i16;
                            new_rel.to_be_bytes().to_vec()
                        } else {
                            instr.operands.clone()
                        }
                    } else {
                        instr.operands.clone()
                    }
                }
                OpCode::MATCH_ENUM => {
                    if let Some(target_idx) = instr.jump_target_idx {
                        if target_idx < instructions.len() && instr.operands.len() >= 4 {
                            let new_target = new_byte_offsets[target_idx] as isize;
                            let new_ip_after = (new_byte_offsets[i] + 5) as isize;
                            let new_rel = (new_target - new_ip_after) as i16;
                            let mut result = instr.operands[..2].to_vec();
                            result.extend_from_slice(&new_rel.to_be_bytes());
                            result
                        } else {
                            instr.operands.clone()
                        }
                    } else {
                        instr.operands.clone()
                    }
                }
                // PUSH_HANDLER stores an absolute u16 catch_ip.  Recompute it
                // from the resolved instruction index so it points to the
                // correct byte offset after optimization reshuffles the chunk.
                OpCode::PUSH_HANDLER => {
                    if let Some(target_idx) = instr.jump_target_idx {
                        if target_idx < instructions.len() {
                            let new_target = new_byte_offsets[target_idx] as u16;
                            new_target.to_be_bytes().to_vec()
                        } else {
                            instr.operands.clone()
                        }
                    } else {
                        instr.operands.clone()
                    }
                }
                _ => instr.operands.clone(),
            };

            for &b in &adjusted_operands {
                chunk.code.push(b);
                chunk.source_lines.push(instr.line);
            }
        }
    }

    /// Try to fold three consecutive instructions (push, push, binary-op)
    /// into a single push of the computed result.
    fn try_fold_constants(
        a: &DecodedInstr,
        b: &DecodedInstr,
        op: &DecodedInstr,
    ) -> Option<DecodedInstr> {
        match (a.opcode, b.opcode, op.opcode) {
            // I64 arithmetic
            (OpCode::PUSH_I64, OpCode::PUSH_I64, OpCode::ADD_I64) => {
                let va = i64::from_be_bytes(a.operands[..8].try_into().ok()?);
                let vb = i64::from_be_bytes(b.operands[..8].try_into().ok()?);
                let result = va.wrapping_add(vb);
                Some(DecodedInstr {
                    opcode: OpCode::PUSH_I64,
                    operands: result.to_be_bytes().to_vec(),
                    line: a.line,
                    jump_target_idx: None,
                })
            }
            (OpCode::PUSH_I64, OpCode::PUSH_I64, OpCode::SUB_I64) => {
                let va = i64::from_be_bytes(a.operands[..8].try_into().ok()?);
                let vb = i64::from_be_bytes(b.operands[..8].try_into().ok()?);
                let result = va.wrapping_sub(vb);
                Some(DecodedInstr {
                    opcode: OpCode::PUSH_I64,
                    operands: result.to_be_bytes().to_vec(),
                    line: a.line,
                    jump_target_idx: None,
                })
            }
            (OpCode::PUSH_I64, OpCode::PUSH_I64, OpCode::MUL_I64) => {
                let va = i64::from_be_bytes(a.operands[..8].try_into().ok()?);
                let vb = i64::from_be_bytes(b.operands[..8].try_into().ok()?);
                let result = va.wrapping_mul(vb);
                Some(DecodedInstr {
                    opcode: OpCode::PUSH_I64,
                    operands: result.to_be_bytes().to_vec(),
                    line: a.line,
                    jump_target_idx: None,
                })
            }
            // I32 arithmetic
            (OpCode::PUSH_I32, OpCode::PUSH_I32, OpCode::ADD_I32) => {
                let va = i32::from_be_bytes(a.operands[..4].try_into().ok()?);
                let vb = i32::from_be_bytes(b.operands[..4].try_into().ok()?);
                let result = va.wrapping_add(vb);
                Some(DecodedInstr {
                    opcode: OpCode::PUSH_I32,
                    operands: result.to_be_bytes().to_vec(),
                    line: a.line,
                    jump_target_idx: None,
                })
            }
            (OpCode::PUSH_I32, OpCode::PUSH_I32, OpCode::SUB_I32) => {
                let va = i32::from_be_bytes(a.operands[..4].try_into().ok()?);
                let vb = i32::from_be_bytes(b.operands[..4].try_into().ok()?);
                let result = va.wrapping_sub(vb);
                Some(DecodedInstr {
                    opcode: OpCode::PUSH_I32,
                    operands: result.to_be_bytes().to_vec(),
                    line: a.line,
                    jump_target_idx: None,
                })
            }
            (OpCode::PUSH_I32, OpCode::PUSH_I32, OpCode::MUL_I32) => {
                let va = i32::from_be_bytes(a.operands[..4].try_into().ok()?);
                let vb = i32::from_be_bytes(b.operands[..4].try_into().ok()?);
                let result = va.wrapping_mul(vb);
                Some(DecodedInstr {
                    opcode: OpCode::PUSH_I32,
                    operands: result.to_be_bytes().to_vec(),
                    line: a.line,
                    jump_target_idx: None,
                })
            }
            // String concatenation is handled in fold_constants_chunk
            // because it needs mutable access to the string table.
            (OpCode::PUSH_STRING, OpCode::PUSH_STRING, OpCode::STR_CONCAT) => None,
            _ => None,
        }
    }

    /// Fold constants in a single chunk's bytecode.
    pub(super) fn fold_constants_chunk(chunk: &mut Chunk) {
        let strings = chunk.strings.clone();
        let old_instructions = Self::decode_instructions(chunk);

        if old_instructions.is_empty() {
            return;
        }

        // Build mapping: old instruction index → old byte offset
        let mut old_byte_offsets = Vec::with_capacity(old_instructions.len());
        let mut off = 0usize;
        for instr in &old_instructions {
            old_byte_offsets.push(off);
            off += 1 + instr.operands.len();
        }

        // Do constant folding, building new instruction vec and tracking
        // the mapping from old instruction index to new instruction index.
        let mut new_instructions: Vec<DecodedInstr> = Vec::new();
        let mut old_to_new: Vec<Option<usize>> = vec![None; old_instructions.len()];

        let mut i = 0;
        while i < old_instructions.len() {
            if i + 2 < old_instructions.len() {
                // Try numeric folding
                if let Some(folded) =
                    Self::try_fold_constants(&old_instructions[i], &old_instructions[i + 1], &old_instructions[i + 2])
                {
                    let new_idx = new_instructions.len();
                    old_to_new[i] = Some(new_idx);
                    new_instructions.push(folded);
                    i += 3;
                    continue;
                }

                // Try string concatenation folding
                if old_instructions[i].opcode == OpCode::PUSH_STRING
                    && old_instructions[i + 1].opcode == OpCode::PUSH_STRING
                    && old_instructions[i + 2].opcode == OpCode::STR_CONCAT
                {
                    let idx_a = u16::from_be_bytes(
                        old_instructions[i].operands[..2].try_into().unwrap_or([0, 0]),
                    ) as usize;
                    let idx_b = u16::from_be_bytes(
                        old_instructions[i + 1].operands[..2].try_into().unwrap_or([0, 0]),
                    ) as usize;
                    if idx_a < strings.len() && idx_b < strings.len() {
                        let combined = format!("{}{}", strings[idx_a], strings[idx_b]);
                        let new_idx_str = chunk.add_string(&combined);
                        let new_idx = new_instructions.len();
                        old_to_new[i] = Some(new_idx);
                        new_instructions.push(DecodedInstr {
                            opcode: OpCode::PUSH_STRING,
                            operands: new_idx_str.to_be_bytes().to_vec(),
                            line: old_instructions[i].line,
                            jump_target_idx: None,
                        });
                        i += 3;
                        continue;
                    }
                }
            }

            let new_idx = new_instructions.len();
            old_to_new[i] = Some(new_idx);
            new_instructions.push(old_instructions[i].clone());
            i += 1;
        }

        // Remap jump targets from old instruction indices to new ones.
        for new_instr in &mut new_instructions {
            if let Some(old_target) = new_instr.jump_target_idx {
                // Find the new instruction index for this old target.
                // If the old target was folded (removed), use the nearest
                // earlier instruction that still exists.
                let mut new_target = None;
                for j in (0..=old_target).rev() {
                    if let Some(ni) = old_to_new[j] {
                        new_target = Some(ni);
                        break;
                    }
                }
                new_instr.jump_target_idx = new_target;
            }
        }

        Self::encode_instructions(chunk, &new_instructions);
    }

    /// Run the constant folding optimization pass on all compiled functions.
    pub fn fold_constants(&mut self) {
        for func in &mut self.functions {
            Self::fold_constants_chunk(&mut func.chunk);
        }
    }

    /// Eliminate dead code in a single chunk's bytecode.
    /// Removes unreachable code after unconditional jumps and returns,
    /// and removes unused string table entries.
    pub(super) fn eliminate_dead_code_chunk(chunk: &mut Chunk) {
        let old_instructions = Self::decode_instructions(chunk);

        if old_instructions.is_empty() {
            return;
        }

        // Collect all jump target instruction indices.
        let mut jump_targets: HashSet<usize> = HashSet::new();
        for instr in &old_instructions {
            if let Some(target) = instr.jump_target_idx {
                jump_targets.insert(target);
            }
        }

        // Mark instructions as reachable or dead.
        // After an unconditional JMP or RET, code is dead until a jump target.
        let mut reachable = vec![true; old_instructions.len()];
        let mut dead = false;
        for i in 0..old_instructions.len() {
            if dead {
                if jump_targets.contains(&i) {
                    dead = false;
                } else {
                    reachable[i] = false;
                    continue;
                }
            }
            match old_instructions[i].opcode {
                OpCode::JMP | OpCode::RET => {
                    dead = true;
                }
                _ => {}
            }
        }

        // Build new instruction vec, keeping only reachable instructions,
        // and track old→new index mapping.
        let mut new_instructions: Vec<DecodedInstr> = Vec::new();
        let mut old_to_new: Vec<Option<usize>> = vec![None; old_instructions.len()];

        for (i, instr) in old_instructions.into_iter().enumerate() {
            if reachable[i] {
                let new_idx = new_instructions.len();
                old_to_new[i] = Some(new_idx);
                new_instructions.push(instr);
            }
        }

        // Remap jump targets.
        for new_instr in &mut new_instructions {
            if let Some(old_target) = new_instr.jump_target_idx {
                let mut new_target = None;
                for j in (0..=old_target).rev() {
                    if let Some(ni) = old_to_new[j] {
                        new_target = Some(ni);
                        break;
                    }
                }
                new_instr.jump_target_idx = new_target;
            }
        }

        Self::encode_instructions(chunk, &new_instructions);

        // Skip string table compaction for now - it's complex to remap
        // all string references (PUSH_STRING, STATIC_CALL, etc.) correctly.
        // The memory savings are minimal for typical programs.
    }

    /// Remove string table entries that are not referenced by any PUSH_STRING
    /// instruction in the chunk, and remap the indices.
    #[allow(dead_code)]
    pub(super) fn remove_unused_strings(chunk: &mut Chunk) {
        // Find all used string indices from PUSH_STRING and STATIC_CALL.
        let mut used_indices: HashSet<u16> = HashSet::new();
        let instructions = Self::decode_instructions(chunk);
        for instr in &instructions {
            if instr.opcode == OpCode::PUSH_STRING && instr.operands.len() >= 2 {
                let idx = u16::from_be_bytes([instr.operands[0], instr.operands[1]]);
                used_indices.insert(idx);
            }
            // STATIC_CALL uses two string table indices: class_name and method_name
            if instr.opcode == OpCode::STATIC_CALL && instr.operands.len() >= 4 {
                let class_idx = u16::from_be_bytes([instr.operands[0], instr.operands[1]]);
                let method_idx = u16::from_be_bytes([instr.operands[2], instr.operands[3]]);
                used_indices.insert(class_idx);
                used_indices.insert(method_idx);
            }
        }

        if used_indices.is_empty() && chunk.strings.is_empty() {
            return;
        }

        // Build remapping: old index → new index.
        let mut remap: HashMap<u16, u16> = HashMap::new();
        let mut new_strings = Vec::new();
        for (old_idx, s) in chunk.strings.iter().enumerate() {
            if used_indices.contains(&(old_idx as u16)) {
                let new_idx = new_strings.len() as u16;
                remap.insert(old_idx as u16, new_idx);
                new_strings.push(s.clone());
            }
        }

        chunk.strings = new_strings;

        // Re-encode PUSH_STRING instructions with remapped indices.
        if remap.is_empty() {
            return;
        }

        let mut new_instructions = instructions;
        for instr in &mut new_instructions {
            if instr.opcode == OpCode::PUSH_STRING && instr.operands.len() >= 2 {
                let old_idx = u16::from_be_bytes([instr.operands[0], instr.operands[1]]);
                if let Some(&new_idx) = remap.get(&old_idx) {
                    instr.operands = new_idx.to_be_bytes().to_vec();
                }
            }
            // Remap STATIC_CALL string indices (class_name + method_name)
            if instr.opcode == OpCode::STATIC_CALL && instr.operands.len() >= 4 {
                let old_class = u16::from_be_bytes([instr.operands[0], instr.operands[1]]);
                let old_method = u16::from_be_bytes([instr.operands[2], instr.operands[3]]);
                let new_class = remap.get(&old_class).copied().unwrap_or(old_class);
                let new_method = remap.get(&old_method).copied().unwrap_or(old_method);
                instr.operands[0..2].copy_from_slice(&new_class.to_be_bytes());
                instr.operands[2..4].copy_from_slice(&new_method.to_be_bytes());
            }
        }

        Self::encode_instructions(chunk, &new_instructions);
    }

    /// Run the dead code elimination pass on all compiled functions.
    pub fn eliminate_dead_code(&mut self) {
        for func in &mut self.functions {
            Self::eliminate_dead_code_chunk(&mut func.chunk);
        }
    }
}
