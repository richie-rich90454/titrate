// Titrate Alpha 0.2 – bytecode virtual machine: step dispatch
// Precision in every step – richie-rich90454, 2026

use super::super::opcodes::OpCode;
use super::super::value::Value;
use super::Vm;

impl Vm {
    /// Pop a value, auto-unwrapping ResultOk to its inner value.
    /// This allows arithmetic/comparison operations to work seamlessly
    /// when a function returns Result<T> but the caller uses the value directly.
    pub(super) fn pop_unwrapped(&mut self) -> Value {
        let val = self.pop();
        match val {
            Value::ResultOk(inner) => *inner,
            v => v,
        }
    }

    pub(super) fn step(&mut self) -> Result<(), String> {
        self.step_count += 1;
        if self.step_count > 10_000_000 {
            return Err(format!("VM exceeded 10 million steps (infinite loop detected). frames={}, stack_len={}", self.frames.len(), self.stack.len()));
        }
        let op_byte = self.read_u8();
        let op = OpCode::try_from(op_byte)
            .map_err(|v| format!("Unknown opcode: {}", v))?;

        match op {
            
OpCode::PUSH_I8 | OpCode::PUSH_I16 | OpCode::PUSH_I32 | OpCode::PUSH_I64 | OpCode::PUSH_F32 | OpCode::PUSH_F64 | OpCode::PUSH_BOOL | OpCode::PUSH_CHAR | OpCode::PUSH_STRING | OpCode::PUSH_NULL | OpCode::PUSH_VOID | OpCode::POP | OpCode::DUP | OpCode::SWAP | OpCode::ADD_I32 | OpCode::ADD_I64 | OpCode::ADD_F32 | OpCode::ADD_F64 | OpCode::SUB_I32 | OpCode::SUB_I64 | OpCode::SUB_F32 | OpCode::SUB_F64 | OpCode::MUL_I32 | OpCode::MUL_I64 | OpCode::MUL_F32 | OpCode::MUL_F64 | OpCode::DIV_I32 | OpCode::DIV_I64 | OpCode::DIV_F32 | OpCode::DIV_F64 | OpCode::MOD_I32 | OpCode::MOD_I64 | OpCode::MOD_F32 | OpCode::MOD_F64 | OpCode::NEG_I32 | OpCode::NEG_I64 | OpCode::NEG_F32 | OpCode::NEG_F64 | OpCode::BITAND_I32 | OpCode::BITAND_I64 | OpCode::BITOR_I32 | OpCode::BITOR_I64 | OpCode::BITXOR_I32 | OpCode::BITXOR_I64 | OpCode::SHL_I32 | OpCode::SHL_I64 | OpCode::SHR_I32 | OpCode::SHR_I64 | OpCode::USHR_I32 | OpCode::USHR_I64 | OpCode::BITNOT_I32 | OpCode::BITNOT_I64
 => self.step_stack_arith(op)?,
            
OpCode::EQ_I32 | OpCode::EQ_I64 | OpCode::EQ_F32 | OpCode::EQ_F64 | OpCode::EQ_BOOL | OpCode::EQ_CHAR | OpCode::EQ_STRING | OpCode::NE_I32 | OpCode::NE_I64 | OpCode::NE_F32 | OpCode::NE_F64 | OpCode::LT_I32 | OpCode::LT_I64 | OpCode::LT_F32 | OpCode::LT_F64 | OpCode::LE_I32 | OpCode::LE_I64 | OpCode::LE_F32 | OpCode::LE_F64 | OpCode::GT_I32 | OpCode::GT_I64 | OpCode::GT_F32 | OpCode::GT_F64 | OpCode::GE_I32 | OpCode::GE_I64 | OpCode::GE_F32 | OpCode::GE_F64 | OpCode::AND | OpCode::OR | OpCode::NOT | OpCode::STR_CONCAT | OpCode::STR_CONCAT_RIGHT | OpCode::STR_CONCAT_LEFT
 => self.step_cmp_logic(op)?,
            _ => self.step_flow(op)?,
        }

        Ok(())
    }
}
