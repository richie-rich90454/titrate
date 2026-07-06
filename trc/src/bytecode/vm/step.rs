// Titrate Alpha 0.2 – bytecode virtual machine: step dispatch
// Precision in every step – richie-rich90454, 2026

use super::super::frame::Frame;
use super::super::opcodes::{OpCode, CastTarget, TypeTag};
use super::super::value::Value;
use super::Vm;
use std::char;
use std::rc::Rc;

impl Vm {
    pub(super) fn step(&mut self) -> Result<(), String> {
        self.step_count += 1;
        if self.step_count > 10_000_000 {
            return Err(format!("VM exceeded 10 million steps (infinite loop detected). frames={}, stack_len={}", self.frames.len(), self.stack.len()));
        }
        let op_byte = self.read_u8();
        let op = OpCode::try_from(op_byte)
            .map_err(|v| format!("Unknown opcode: {}", v))?;

        match op {
            // -- Constants -------------------------------------------------------
            OpCode::PUSH_I8 => {
                let val = self.read_u8() as i8;
                self.push(Value::Byte(val));
            }
            OpCode::PUSH_I16 => {
                let val = self.read_i16();
                self.push(Value::Short(val));
            }
            OpCode::PUSH_I32 => {
                let val = self.read_i32();
                self.push(Value::Int(val));
            }
            OpCode::PUSH_I64 => {
                let val = self.read_i64();
                self.push(Value::Long(val));
            }
            OpCode::PUSH_F32 => {
                let val = self.read_f32();
                self.push(Value::Float(val));
            }
            OpCode::PUSH_F64 => {
                let val = self.read_f64();
                self.push(Value::Double(val));
            }
            OpCode::PUSH_BOOL => {
                let val = self.read_u8();
                self.push(Value::Bool(val != 0));
            }
            OpCode::PUSH_CHAR => {
                let code_point = {
                    let frame = self.current_frame();
                    let ip = frame.ip;
                    let chunk = &self.functions[frame.function_index as usize].chunk;
                    let val = u32::from_be_bytes([
                        chunk.code[ip],
                        chunk.code[ip + 1],
                        chunk.code[ip + 2],
                        chunk.code[ip + 3],
                    ]);
                    self.current_frame_mut().ip += 4;
                    val
                };
                let c = char::from_u32(code_point)
                    .ok_or_else(|| format!("Invalid Unicode code point: {}", code_point))?;
                self.push(Value::Char(c));
            }
            OpCode::PUSH_STRING => {
                let idx = self.read_u16() as usize;
                let frame = self.current_frame();
                let chunk = &self.functions[frame.function_index as usize].chunk;
                let s = chunk.strings[idx].clone();
                self.push(Value::String(Rc::new(s)));
            }
            OpCode::PUSH_NULL => {
                self.push(Value::Null);
            }
            OpCode::PUSH_VOID => {
                self.push(Value::Void);
            }

            // -- Stack -----------------------------------------------------------
            OpCode::POP => {
                self.pop();
            }
            OpCode::DUP => {
                let val = self.peek(0);
                self.push(val);
            }
            OpCode::SWAP => {
                let len = self.stack.len();
                self.stack.swap(len - 1, len - 2);
            }

            // -- Arithmetic ------------------------------------------------------
            OpCode::ADD_I32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x.wrapping_add(*y))),
                    _ => return Err(format!("ADD_I32: type mismatch {:?} + {:?}", a, b)),
                }
            }
            OpCode::ADD_I64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => self.push(Value::Long(x.wrapping_add(*y))),
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x.wrapping_add(*y))),
                    (Value::Int(x), Value::Long(y)) => self.push(Value::Long((*x as i64).wrapping_add(*y))),
                    (Value::Long(x), Value::Int(y)) => self.push(Value::Long(x.wrapping_add(*y as i64))),
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Double(x + y)),
                    (Value::Double(x), Value::Long(y)) => self.push(Value::Double(x + (*y as f64))),
                    (Value::Long(x), Value::Double(y)) => self.push(Value::Double((*x as f64) + y)),
                    (Value::Double(x), Value::Int(y)) => self.push(Value::Double(x + (*y as f64))),
                    (Value::Int(x), Value::Double(y)) => self.push(Value::Double((*x as f64) + y)),
                    (Value::Float(x), Value::Float(y)) => self.push(Value::Float(x + y)),
                    _ => return Err(format!("ADD_I64: type mismatch {:?} + {:?}", a, b)),
                }
            }
            OpCode::ADD_F32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Float(x), Value::Float(y)) => self.push(Value::Float(x + y)),
                    _ => return Err(format!("ADD_F32: type mismatch {:?} + {:?}", a, b)),
                }
            }
            OpCode::ADD_F64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Double(x + y)),
                    _ => return Err(format!("ADD_F64: type mismatch {:?} + {:?}", a, b)),
                }
            }
            OpCode::SUB_I32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x.wrapping_sub(*y))),
                    _ => return Err(format!("SUB_I32: type mismatch {:?} - {:?}", a, b)),
                }
            }
            OpCode::SUB_I64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => self.push(Value::Long(x.wrapping_sub(*y))),
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x.wrapping_sub(*y))),
                    (Value::Int(x), Value::Long(y)) => self.push(Value::Long((*x as i64).wrapping_sub(*y))),
                    (Value::Long(x), Value::Int(y)) => self.push(Value::Long(x.wrapping_sub(*y as i64))),
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Double(x - y)),
                    (Value::Double(x), Value::Long(y)) => self.push(Value::Double(x - (*y as f64))),
                    (Value::Long(x), Value::Double(y)) => self.push(Value::Double((*x as f64) - y)),
                    (Value::Double(x), Value::Int(y)) => self.push(Value::Double(x - (*y as f64))),
                    (Value::Int(x), Value::Double(y)) => self.push(Value::Double((*x as f64) - y)),
                    (Value::Float(x), Value::Float(y)) => self.push(Value::Float(x - y)),
                    _ => return Err(format!("SUB_I64: type mismatch {:?} - {:?}", a, b)),
                }
            }
            OpCode::SUB_F32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Float(x), Value::Float(y)) => self.push(Value::Float(x - y)),
                    _ => return Err(format!("SUB_F32: type mismatch {:?} - {:?}", a, b)),
                }
            }
            OpCode::SUB_F64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Double(x - y)),
                    _ => return Err(format!("SUB_F64: type mismatch {:?} - {:?}", a, b)),
                }
            }
            OpCode::MUL_I32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x.wrapping_mul(*y))),
                    _ => return Err(format!("MUL_I32: type mismatch {:?} * {:?}", a, b)),
                }
            }
            OpCode::MUL_I64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => self.push(Value::Long(x.wrapping_mul(*y))),
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x.wrapping_mul(*y))),
                    (Value::Int(x), Value::Long(y)) => self.push(Value::Long((*x as i64).wrapping_mul(*y))),
                    (Value::Long(x), Value::Int(y)) => self.push(Value::Long(x.wrapping_mul(*y as i64))),
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Double(x * y)),
                    (Value::Double(x), Value::Long(y)) => self.push(Value::Double(x * (*y as f64))),
                    (Value::Long(x), Value::Double(y)) => self.push(Value::Double((*x as f64) * y)),
                    (Value::Double(x), Value::Int(y)) => self.push(Value::Double(x * (*y as f64))),
                    (Value::Int(x), Value::Double(y)) => self.push(Value::Double((*x as f64) * y)),
                    (Value::Float(x), Value::Float(y)) => self.push(Value::Float(x * y)),
                    _ => return Err(format!("MUL_I64: type mismatch {:?} * {:?}", a, b)),
                }
            }
            OpCode::MUL_F32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Float(x), Value::Float(y)) => self.push(Value::Float(x * y)),
                    _ => return Err(format!("MUL_F32: type mismatch {:?} * {:?}", a, b)),
                }
            }
            OpCode::MUL_F64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Double(x * y)),
                    _ => return Err(format!("MUL_F64: type mismatch {:?} * {:?}", a, b)),
                }
            }
            OpCode::DIV_I32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Int(_), Value::Int(0)) => {
                        return Err("Division by zero (int)".to_string());
                    }
                    (Value::Int(x), Value::Int(y)) => {
                        self.push(Value::Int(x.wrapping_div(*y)));
                    }
                    _ => return Err(format!("DIV_I32: type mismatch {:?} / {:?}", a, b)),
                }
            }
            OpCode::DIV_I64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Long(_), Value::Long(0)) => {
                        return Err("Division by zero (long)".to_string());
                    }
                    (Value::Long(x), Value::Long(y)) => {
                        self.push(Value::Long(x.wrapping_div(*y)));
                    }
                    (Value::Int(_), Value::Int(0)) => {
                        return Err("Division by zero (int)".to_string());
                    }
                    (Value::Int(x), Value::Int(y)) => {
                        self.push(Value::Int(x.wrapping_div(*y)));
                    }
                    (Value::Int(_), Value::Long(0)) | (Value::Long(_), Value::Int(0)) => {
                        return Err("Division by zero".to_string());
                    }
                    (Value::Int(x), Value::Long(y)) => {
                        self.push(Value::Long((*x as i64).wrapping_div(*y)));
                    }
                    (Value::Long(x), Value::Int(y)) => {
                        self.push(Value::Long(x.wrapping_div(*y as i64)));
                    }
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Double(x / y)),
                    (Value::Double(x), Value::Long(y)) => self.push(Value::Double(x / (*y as f64))),
                    (Value::Long(x), Value::Double(y)) => self.push(Value::Double((*x as f64) / y)),
                    (Value::Double(x), Value::Int(y)) => self.push(Value::Double(x / (*y as f64))),
                    (Value::Int(x), Value::Double(y)) => self.push(Value::Double((*x as f64) / y)),
                    _ => return Err(format!("DIV_I64: type mismatch {:?} / {:?}", a, b)),
                }
            }
            OpCode::DIV_F32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Float(x), Value::Float(y)) => self.push(Value::Float(x / y)),
                    _ => return Err(format!("DIV_F32: type mismatch {:?} / {:?}", a, b)),
                }
            }
            OpCode::DIV_F64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Double(x / y)),
                    _ => return Err(format!("DIV_F64: type mismatch {:?} / {:?}", a, b)),
                }
            }
            OpCode::MOD_I32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Int(_), Value::Int(0)) => {
                        return Err("Remainder division by zero (int)".to_string());
                    }
                    (Value::Int(x), Value::Int(y)) => {
                        self.push(Value::Int(x.wrapping_rem(*y)));
                    }
                    _ => return Err(format!("MOD_I32: type mismatch {:?} % {:?}", a, b)),
                }
            }
            OpCode::MOD_I64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Long(_), Value::Long(0)) => {
                        return Err("Remainder division by zero (long)".to_string());
                    }
                    (Value::Long(x), Value::Long(y)) => {
                        self.push(Value::Long(x.wrapping_rem(*y)));
                    }
                    _ => return Err(format!("MOD_I64: type mismatch {:?} % {:?}", a, b)),
                }
            }
            OpCode::MOD_F32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Float(x), Value::Float(y)) => self.push(Value::Float(x % y)),
                    _ => return Err(format!("MOD_F32: type mismatch {:?} % {:?}", a, b)),
                }
            }
            OpCode::MOD_F64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Double(x % y)),
                    _ => return Err(format!("MOD_F64: type mismatch {:?} % {:?}", a, b)),
                }
            }
            OpCode::NEG_I32 => {
                let a = self.pop();
                match a {
                    Value::Int(x) => self.push(Value::Int(x.wrapping_neg())),
                    _ => return Err(format!("NEG_I32: type mismatch {:?}", a)),
                }
            }
            OpCode::NEG_I64 => {
                let a = self.pop();
                match a {
                    Value::Long(x) => self.push(Value::Long(x.wrapping_neg())),
                    _ => return Err(format!("NEG_I64: type mismatch {:?}", a)),
                }
            }
            OpCode::NEG_F32 => {
                let a = self.pop();
                match a {
                    Value::Float(x) => self.push(Value::Float(-x)),
                    _ => return Err(format!("NEG_F32: type mismatch {:?}", a)),
                }
            }
            OpCode::NEG_F64 => {
                let a = self.pop();
                match a {
                    Value::Double(x) => self.push(Value::Double(-x)),
                    _ => return Err(format!("NEG_F64: type mismatch {:?}", a)),
                }
            }

            // -- Bitwise ---------------------------------------------------------
            OpCode::BITAND_I32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x & y)),
                    _ => return Err(format!("BITAND_I32: type mismatch {:?}", a)),
                }
            }
            OpCode::BITAND_I64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => self.push(Value::Long(x & y)),
                    _ => return Err(format!("BITAND_I64: type mismatch {:?}", a)),
                }
            }
            OpCode::BITOR_I32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x | y)),
                    _ => return Err(format!("BITOR_I32: type mismatch {:?}", a)),
                }
            }
            OpCode::BITOR_I64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => self.push(Value::Long(x | y)),
                    _ => return Err(format!("BITOR_I64: type mismatch {:?}", a)),
                }
            }
            OpCode::BITXOR_I32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x ^ y)),
                    _ => return Err(format!("BITXOR_I32: type mismatch {:?}", a)),
                }
            }
            OpCode::BITXOR_I64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => self.push(Value::Long(x ^ y)),
                    _ => return Err(format!("BITXOR_I64: type mismatch {:?}", a)),
                }
            }
            OpCode::SHL_I32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x.wrapping_shl(*y as u32))),
                    _ => return Err(format!("SHL_I32: type mismatch {:?}", a)),
                }
            }
            OpCode::SHL_I64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => self.push(Value::Long(x.wrapping_shl(*y as u32))),
                    _ => return Err(format!("SHL_I64: type mismatch {:?}", a)),
                }
            }
            OpCode::SHR_I32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x.wrapping_shr(*y as u32))),
                    _ => return Err(format!("SHR_I32: type mismatch {:?}", a)),
                }
            }
            OpCode::SHR_I64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => self.push(Value::Long(x.wrapping_shr(*y as u32))),
                    _ => return Err(format!("SHR_I64: type mismatch {:?}", a)),
                }
            }
            OpCode::USHR_I32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => {
                        // Unsigned (logical) right shift: cast to u32, shift, cast back
                        self.push(Value::Int(((*x as u32) >> (*y as u32)) as i32));
                    }
                    _ => return Err(format!("USHR_I32: type mismatch {:?}", a)),
                }
            }
            OpCode::USHR_I64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => {
                        // Unsigned (logical) right shift: cast to u64, shift, cast back
                        self.push(Value::Long(((*x as u64) >> (*y as u32)) as i64));
                    }
                    _ => return Err(format!("USHR_I64: type mismatch {:?}", a)),
                }
            }
            OpCode::BITNOT_I32 => {
                let a = self.pop();
                match a {
                    Value::Int(x) => self.push(Value::Int(!x)),
                    _ => return Err(format!("BITNOT_I32: type mismatch {:?}", a)),
                }
            }
            OpCode::BITNOT_I64 => {
                let a = self.pop();
                match a {
                    Value::Long(x) => self.push(Value::Long(!x)),
                    _ => return Err(format!("BITNOT_I64: type mismatch {:?}", a)),
                }
            }

            // -- Comparison ------------------------------------------------------
            OpCode::EQ_I32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x == y)),
                    _ => return Err(format!("EQ_I32: type mismatch {:?}", a)),
                }
            }
            OpCode::EQ_I64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => self.push(Value::Bool(x == y)),
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x == y)),
                    (Value::Int(x), Value::Long(y)) => self.push(Value::Bool((*x as i64) == *y)),
                    (Value::Long(x), Value::Int(y)) => self.push(Value::Bool(*x == (*y as i64))),
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Bool(x == y)),
                    (Value::String(x), Value::String(y)) => self.push(Value::Bool(x == y)),
                    (Value::Bool(x), Value::Bool(y)) => self.push(Value::Bool(x == y)),
                    (Value::Null, Value::String(_)) | (Value::String(_), Value::Null) => self.push(Value::Bool(false)),
                    (Value::Null, Value::Null) => self.push(Value::Bool(true)),
                    _ => return Err(format!("EQ_I64: type mismatch {:?} == {:?}", a, b)),
                }
            }
            OpCode::EQ_F32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Float(x), Value::Float(y)) => {
                        self.push(Value::Bool(x.to_bits() == y.to_bits()))
                    }
                    _ => return Err(format!("EQ_F32: type mismatch {:?}", a)),
                }
            }
            OpCode::EQ_F64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Double(x), Value::Double(y)) => {
                        self.push(Value::Bool(x.to_bits() == y.to_bits()))
                    }
                    _ => return Err(format!("EQ_F64: type mismatch {:?}", a)),
                }
            }
            OpCode::EQ_BOOL => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Bool(x), Value::Bool(y)) => self.push(Value::Bool(x == y)),
                    _ => return Err(format!("EQ_BOOL: type mismatch {:?}", a)),
                }
            }
            OpCode::EQ_CHAR => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Char(x), Value::Char(y)) => self.push(Value::Bool(x == y)),
                    _ => return Err(format!("EQ_CHAR: type mismatch {:?}", a)),
                }
            }
            OpCode::EQ_STRING => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::String(x), Value::String(y)) => self.push(Value::Bool(x == y)),
                    (Value::Null, Value::String(_)) | (Value::String(_), Value::Null) => self.push(Value::Bool(false)),
                    (Value::Null, Value::Null) => self.push(Value::Bool(true)),
                    _ => return Err(format!("EQ_STRING: type mismatch {:?}", a)),
                }
            }
            OpCode::NE_I32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x != y)),
                    _ => return Err(format!("NE_I32: type mismatch {:?}", a)),
                }
            }
            OpCode::NE_I64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => self.push(Value::Bool(x != y)),
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x != y)),
                    (Value::Int(x), Value::Long(y)) => self.push(Value::Bool((*x as i64) != *y)),
                    (Value::Long(x), Value::Int(y)) => self.push(Value::Bool(*x != (*y as i64))),
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Bool(x != y)),
                    (Value::String(x), Value::String(y)) => self.push(Value::Bool(x != y)),
                    (Value::Bool(x), Value::Bool(y)) => self.push(Value::Bool(x != y)),
                    (Value::Null, Value::String(_)) | (Value::String(_), Value::Null) => self.push(Value::Bool(true)),
                    (Value::Null, Value::Null) => self.push(Value::Bool(false)),
                    _ => return Err(format!("NE_I64: type mismatch {:?} != {:?}", a, b)),
                }
            }
            OpCode::NE_F32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Float(x), Value::Float(y)) => {
                        self.push(Value::Bool(x.to_bits() != y.to_bits()))
                    }
                    _ => return Err(format!("NE_F32: type mismatch {:?}", a)),
                }
            }
            OpCode::NE_F64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Double(x), Value::Double(y)) => {
                        self.push(Value::Bool(x.to_bits() != y.to_bits()))
                    }
                    _ => return Err(format!("NE_F64: type mismatch {:?}", a)),
                }
            }
            OpCode::LT_I32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x < y)),
                    _ => return Err(format!("LT_I32: type mismatch {:?}", a)),
                }
            }
            OpCode::LT_I64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => self.push(Value::Bool(x < y)),
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x < y)),
                    (Value::Int(x), Value::Long(y)) => self.push(Value::Bool((*x as i64) < *y)),
                    (Value::Long(x), Value::Int(y)) => self.push(Value::Bool(*x < (*y as i64))),
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Bool(x < y)),
                    (Value::Double(x), Value::Long(y)) => self.push(Value::Bool(x < &(*y as f64))),
                    (Value::Long(x), Value::Double(y)) => self.push(Value::Bool(&(*x as f64) < y)),
                    (Value::Double(x), Value::Int(y)) => self.push(Value::Bool(x < &(*y as f64))),
                    (Value::Int(x), Value::Double(y)) => self.push(Value::Bool(&(*x as f64) < y)),
                    (Value::Bool(x), Value::Bool(y)) => self.push(Value::Bool((*x as i64) < (*y as i64))),
                    (Value::Bool(x), Value::Int(y)) => self.push(Value::Bool((*x as i64) < (*y as i64))),
                    (Value::Int(x), Value::Bool(y)) => self.push(Value::Bool((*x as i64) < (*y as i64))),
                    (Value::Bool(x), Value::Long(y)) => self.push(Value::Bool((*x as i64) < *y)),
                    (Value::Long(x), Value::Bool(y)) => self.push(Value::Bool(*x < (*y as i64))),
                    _ => return Err(format!("LT_I64: type mismatch {:?} < {:?}", a, b)),
                }
            }
            OpCode::LT_F32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Float(x), Value::Float(y)) => self.push(Value::Bool(x < y)),
                    _ => return Err(format!("LT_F32: type mismatch {:?}", a)),
                }
            }
            OpCode::LT_F64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Bool(x < y)),
                    _ => return Err(format!("LT_F64: type mismatch {:?}", a)),
                }
            }
            OpCode::LE_I32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x <= y)),
                    _ => return Err(format!("LE_I32: type mismatch {:?}", a)),
                }
            }
            OpCode::LE_I64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => self.push(Value::Bool(x <= y)),
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x <= y)),
                    (Value::Int(x), Value::Long(y)) => self.push(Value::Bool((*x as i64) <= *y)),
                    (Value::Long(x), Value::Int(y)) => self.push(Value::Bool(*x <= (*y as i64))),
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Bool(x <= y)),
                    (Value::Double(x), Value::Long(y)) => self.push(Value::Bool(x <= &(*y as f64))),
                    (Value::Long(x), Value::Double(y)) => self.push(Value::Bool(&(*x as f64) <= y)),
                    (Value::Double(x), Value::Int(y)) => self.push(Value::Bool(x <= &(*y as f64))),
                    (Value::Int(x), Value::Double(y)) => self.push(Value::Bool(&(*x as f64) <= y)),
                    (Value::Bool(x), Value::Bool(y)) => self.push(Value::Bool((*x as i64) <= (*y as i64))),
                    (Value::Bool(x), Value::Int(y)) => self.push(Value::Bool((*x as i64) <= (*y as i64))),
                    (Value::Int(x), Value::Bool(y)) => self.push(Value::Bool((*x as i64) <= (*y as i64))),
                    (Value::Bool(x), Value::Long(y)) => self.push(Value::Bool((*x as i64) <= *y)),
                    (Value::Long(x), Value::Bool(y)) => self.push(Value::Bool(*x <= (*y as i64))),
                    _ => return Err(format!("LE_I64: type mismatch {:?} <= {:?}", a, b)),
                }
            }
            OpCode::LE_F32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Float(x), Value::Float(y)) => self.push(Value::Bool(x <= y)),
                    _ => return Err(format!("LE_F32: type mismatch {:?}", a)),
                }
            }
            OpCode::LE_F64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Bool(x <= y)),
                    _ => return Err(format!("LE_F64: type mismatch {:?}", a)),
                }
            }
            OpCode::GT_I32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x > y)),
                    _ => return Err(format!("GT_I32: type mismatch {:?}", a)),
                }
            }
            OpCode::GT_I64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => self.push(Value::Bool(x > y)),
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x > y)),
                    (Value::Int(x), Value::Long(y)) => self.push(Value::Bool((*x as i64) > *y)),
                    (Value::Long(x), Value::Int(y)) => self.push(Value::Bool(*x > (*y as i64))),
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Bool(x > y)),
                    (Value::Double(x), Value::Long(y)) => self.push(Value::Bool(x > &(*y as f64))),
                    (Value::Long(x), Value::Double(y)) => self.push(Value::Bool(&(*x as f64) > y)),
                    (Value::Double(x), Value::Int(y)) => self.push(Value::Bool(x > &(*y as f64))),
                    (Value::Int(x), Value::Double(y)) => self.push(Value::Bool(&(*x as f64) > y)),
                    (Value::Bool(x), Value::Bool(y)) => self.push(Value::Bool((*x as i64) > (*y as i64))),
                    (Value::Bool(x), Value::Int(y)) => self.push(Value::Bool((*x as i64) > (*y as i64))),
                    (Value::Int(x), Value::Bool(y)) => self.push(Value::Bool((*x as i64) > (*y as i64))),
                    (Value::Bool(x), Value::Long(y)) => self.push(Value::Bool((*x as i64) > *y)),
                    (Value::Long(x), Value::Bool(y)) => self.push(Value::Bool(*x > (*y as i64))),
                    _ => return Err(format!("GT_I64: type mismatch {:?} > {:?}", a, b)),
                }
            }
            OpCode::GT_F32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Float(x), Value::Float(y)) => self.push(Value::Bool(x > y)),
                    _ => return Err(format!("GT_F32: type mismatch {:?}", a)),
                }
            }
            OpCode::GT_F64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Bool(x > y)),
                    _ => return Err(format!("GT_F64: type mismatch {:?}", a)),
                }
            }
            OpCode::GE_I32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x >= y)),
                    _ => return Err(format!("GE_I32: type mismatch {:?}", a)),
                }
            }
            OpCode::GE_I64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => self.push(Value::Bool(x >= y)),
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x >= y)),
                    (Value::Int(x), Value::Long(y)) => self.push(Value::Bool((*x as i64) >= *y)),
                    (Value::Long(x), Value::Int(y)) => self.push(Value::Bool(*x >= (*y as i64))),
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Bool(x >= y)),
                    (Value::Double(x), Value::Long(y)) => self.push(Value::Bool(x >= &(*y as f64))),
                    (Value::Long(x), Value::Double(y)) => self.push(Value::Bool(&(*x as f64) >= y)),
                    (Value::Double(x), Value::Int(y)) => self.push(Value::Bool(x >= &(*y as f64))),
                    (Value::Int(x), Value::Double(y)) => self.push(Value::Bool(&(*x as f64) >= y)),
                    (Value::Bool(x), Value::Bool(y)) => self.push(Value::Bool((*x as i64) >= (*y as i64))),
                    (Value::Bool(x), Value::Int(y)) => self.push(Value::Bool((*x as i64) >= (*y as i64))),
                    (Value::Int(x), Value::Bool(y)) => self.push(Value::Bool((*x as i64) >= (*y as i64))),
                    (Value::Bool(x), Value::Long(y)) => self.push(Value::Bool((*x as i64) >= *y)),
                    (Value::Long(x), Value::Bool(y)) => self.push(Value::Bool(*x >= (*y as i64))),
                    _ => return Err(format!("GE_I64: type mismatch {:?} >= {:?}", a, b)),
                }
            }
            OpCode::GE_F32 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Float(x), Value::Float(y)) => self.push(Value::Bool(x >= y)),
                    _ => return Err(format!("GE_F32: type mismatch {:?}", a)),
                }
            }
            OpCode::GE_F64 => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Bool(x >= y)),
                    _ => return Err(format!("GE_F64: type mismatch {:?}", a)),
                }
            }

            // -- Logic -----------------------------------------------------------
            OpCode::AND => {
                let b = self.pop();
                let a = self.pop();
                self.push(Value::Bool(a.is_truthy() && b.is_truthy()));
            }
            OpCode::OR => {
                let b = self.pop();
                let a = self.pop();
                self.push(Value::Bool(a.is_truthy() || b.is_truthy()));
            }
            OpCode::NOT => {
                let a = self.pop();
                self.push(Value::Bool(!a.is_truthy()));
            }

            // -- String ----------------------------------------------------------
            OpCode::STR_CONCAT => {
                let b = self.pop();
                let a = self.pop();
                match (&a, &b) {
                    (Value::String(x), Value::String(y)) => {
                        let mut result = (**x).clone();
                        result.push_str(&**y);
                        self.push(Value::String(Rc::new(result)));
                    }
                    _ => return Err(format!("STR_CONCAT: type mismatch {:?}, {:?}", a, b)),
                }
            }
            OpCode::STR_CONCAT_RIGHT => {
                let b = self.pop();
                let a = self.pop();
                match &a {
                    Value::String(x) => {
                        let result = format!("{}{}", &**x, b.display_string());
                        self.push(Value::String(Rc::new(result)));
                    }
                    _ => return Err(format!("STR_CONCAT_RIGHT: left must be String, got {:?}", a)),
                }
            }
            OpCode::STR_CONCAT_LEFT => {
                let b = self.pop();
                let a = self.pop();
                match &b {
                    Value::String(y) => {
                        let result = format!("{}{}", a.display_string(), &**y);
                        self.push(Value::String(Rc::new(result)));
                    }
                    _ => return Err(format!("STR_CONCAT_LEFT: right must be String, got {:?}", b)),
                }
            }

            // -- Control flow ----------------------------------------------------
            OpCode::JMP => {
                let offset = self.read_i16();
                // offset is relative to the IP *after* reading the operand
                let new_ip = self.current_frame().ip as isize + offset as isize;
                self.current_frame_mut().ip = new_ip as usize;
            }
            OpCode::JMP_IF_FALSE => {
                let offset = self.read_i16();
                let val = self.pop();
                if !val.is_truthy() {
                    let new_ip = self.current_frame().ip as isize + offset as isize;
                    self.current_frame_mut().ip = new_ip as usize;
                }
            }
            OpCode::JMP_IF_TRUE => {
                let offset = self.read_i16();
                let val = self.pop();
                if val.is_truthy() {
                    let new_ip = self.current_frame().ip as isize + offset as isize;
                    self.current_frame_mut().ip = new_ip as usize;
                }
            }
            OpCode::CALL => {
                let func_idx = self.read_u16();
                let arg_count = self.read_u8();
                self.call_function(func_idx, arg_count)?;
            }
            OpCode::RET => {
                let return_value = self.pop();
                let frame = self.frames.pop().expect("No frame to return from");
                // If returning from a constructor, the result is the instance ("this")
                let result = if self.functions[frame.function_index as usize].is_constructor {
                    self.stack[frame.base].clone()
                } else {
                    return_value
                };
                // If this was the last frame, push the return value and we're done
                if self.frames.is_empty() {
                    self.push(result);
                } else {
                    // Trim the callee's locals off the stack
                    self.stack.truncate(frame.base);
                    self.push(result);
                }
            }
            OpCode::CALL_NATIVE => {
                let native_idx = self.read_u16();
                let arg_count = self.read_u8();
                self.call_native_fn(native_idx, arg_count)?;
            }

            // -- Variables -------------------------------------------------------
            OpCode::LOAD_LOCAL => {
                let slot = self.read_u8();
                let base = self.current_frame().base;
                let idx = base + slot as usize;
                if idx < self.stack.len() {
                    let val = self.stack[idx].clone();
                    self.push(val);
                } else {
                    // Slot was pre-allocated but popped by end_scope or similar.
                    // Push Null as the default value.
                    self.push(Value::Null);
                }
            }
            OpCode::STORE_LOCAL => {
                let slot = self.read_u8();
                let val = self.stack.pop().unwrap_or(Value::Null);
                let base = self.current_frame().base;
                let idx = base + slot as usize;
                // Ensure the stack is large enough to hold this slot.
                // This handles the case where pre-allocated slots were popped
                // by end_scope or other operations.
                while self.stack.len() <= idx {
                    self.stack.push(Value::Null);
                }
                self.stack[idx] = val;
            }
            OpCode::LOAD_UPVALUE => {
                let slot = self.read_u8() as usize;
                let upvalues = self.current_frame().upvalues.clone();
                match &upvalues {
                    Some(uvs) => {
                        if slot < uvs.len() {
                            self.push(uvs[slot].clone());
                        } else {
                            return Err(format!(
                                "LOAD_UPVALUE: index {} out of bounds (upvalue count {})",
                                slot,
                                uvs.len()
                            ));
                        }
                    }
                    None => {
                        return Err("LOAD_UPVALUE: no upvalues in current frame".to_string());
                    }
                }
            }
            OpCode::STORE_UPVALUE => {
                let slot = self.read_u8() as usize;
                let val = self.pop();
                let upvalues = self.current_frame_mut().upvalues.as_mut();
                match upvalues {
                    Some(uvs) => {
                        if slot < uvs.len() {
                            uvs[slot] = val;
                        } else {
                            return Err(format!(
                                "STORE_UPVALUE: index {} out of bounds (upvalue count {})",
                                slot,
                                uvs.len()
                            ));
                        }
                    }
                    None => {
                        return Err("STORE_UPVALUE: no upvalues in current frame".to_string());
                    }
                }
            }

            // -- Objects ---------------------------------------------------------
            OpCode::NEW => {
                let class_idx = self.read_u16();
                let arg_count = self.read_u8();
                self.exec_new(class_idx, arg_count)?;
            }
            OpCode::INVOKE_VIRTUAL => {
                let method_name_idx = self.read_u16();
                let arg_count = self.read_u8();
                self.invoke_method(method_name_idx, arg_count)?;
            }
            OpCode::INVOKE_OPERATOR => {
                let method_name_idx = self.read_u16();
                let arg_count = self.read_u8();
                self.invoke_operator(method_name_idx, arg_count)?;
            }
            OpCode::GET_FIELD => {
                let field_name_idx = self.read_u16();
                let field_name = {
                    let frame = self.current_frame();
                    let chunk = &self.functions[frame.function_index as usize].chunk;
                    chunk.strings[field_name_idx as usize].clone()
                };
                let instance = self.pop();
                match &instance {
                    Value::ClassInstance { fields, .. } => {
                        let val = fields
                            .borrow()
                            .get(&field_name)
                            .cloned()
                            .unwrap_or(Value::Null);
                        self.push(val);
                    }
                    Value::Tuple { elements } => {
                        // Numeric tuple field access: t.0, t.1, ...
                        match field_name.parse::<usize>() {
                            Ok(idx) if idx < elements.len() => {
                                self.push(elements[idx].clone());
                            }
                            _ => {
                                return Err(format!(
                                    "GET_FIELD: invalid tuple index '{}' on tuple of length {}",
                                    field_name, elements.len()
                                ))
                            }
                        }
                    }
                    _ => {
                        return Err(format!(
                            "GET_FIELD: cannot get field '{}' on {:?}",
                            field_name, instance
                        ))
                    }
                }
            }
            OpCode::SET_FIELD => {
                let field_name_idx = self.read_u16();
                let field_name = {
                    let frame = self.current_frame();
                    let chunk = &self.functions[frame.function_index as usize].chunk;
                    chunk.strings[field_name_idx as usize].clone()
                };
                let instance = self.pop();
                let value = self.pop();
                match &instance {
                    Value::ClassInstance { fields, .. } => {
                        fields.borrow_mut().insert(field_name, value.clone());
                        self.push(value);
                    }
                    _ => {
                        return Err(format!(
                            "SET_FIELD: cannot set field '{}' on {:?}",
                            field_name, instance
                        ))
                    }
                }
            }

            // -- Arrays ----------------------------------------------------------
            OpCode::ARRAY_NEW => {
                let size = self.read_u16() as usize;
                let mut elements = Vec::with_capacity(size);
                for _ in 0..size {
                    elements.push(Value::Null);
                }
                // Pop `size` values from the stack and fill in reverse order
                for i in (0..size).rev() {
                    elements[i] = self.pop();
                }
                self.push(Value::Array { elements });
            }
            OpCode::ARRAY_GET => {
                let index = self.pop();
                let array = self.pop();
                match (&array, &index) {
                    (Value::String(s), Value::Int(i)) => {
                        let idx = *i as usize;
                        match s.chars().nth(idx) {
                            Some(ch) => self.push(Value::Char(ch)),
                            None => return Err(format!("String index out of bounds: {}", idx)),
                        }
                    }
                    (Value::Array { elements }, Value::Int(i)) => {
                        let idx = *i as usize;
                        if idx < elements.len() {
                            self.push(elements[idx].clone());
                        } else {
                            return Err(format!("Array index out of bounds: {}", idx));
                        }
                    }
                    (Value::Array { elements }, Value::Long(i)) => {
                        let idx = *i as usize;
                        if idx < elements.len() {
                            self.push(elements[idx].clone());
                        } else {
                            return Err(format!("Array index out of bounds: {}", idx));
                        }
                    }
                    // ArrayList support: get from _elements field
                    (Value::ClassInstance { class_name, fields, .. }, Value::Int(i))
                        if class_name.starts_with("ArrayList") =>
                    {
                        let idx = *i as usize;
                        match fields.borrow().get("_elements") {
                            Some(Value::Array { elements }) => {
                                if idx < elements.len() {
                                    self.push(elements[idx].clone());
                                } else {
                                    return Err(format!("ArrayList index out of bounds: {}", idx));
                                }
                            }
                            _ => return Err("ArrayList has no elements".to_string()),
                        }
                    }
                    (Value::ClassInstance { class_name, fields, .. }, Value::Long(i))
                        if class_name.starts_with("ArrayList") =>
                    {
                        let idx = *i as usize;
                        match fields.borrow().get("_elements") {
                            Some(Value::Array { elements }) => {
                                if idx < elements.len() {
                                    self.push(elements[idx].clone());
                                } else {
                                    return Err(format!("ArrayList index out of bounds: {}", idx));
                                }
                            }
                            _ => return Err("ArrayList has no elements".to_string()),
                        }
                    }
                    _ => {
                        return Err(format!(
                            "ARRAY_GET: invalid index type on array: {:?}[{:?}]",
                            array, index
                        ))
                    }
                }
            }
            OpCode::ARRAY_SET => {
                let index = self.pop();
                let array = self.pop();
                let value = self.pop();
                match (&array, &index) {
                    (Value::Array { .. }, Value::Int(i)) => {
                        let idx = *i as usize;
                        let mut elements = match array {
                            Value::Array { elements } => elements,
                            _ => unreachable!(),
                        };
                        if idx < elements.len() {
                            elements[idx] = value.clone();
                            self.push(Value::Array { elements });
                        } else {
                            return Err(format!("Array index out of bounds: {}", idx));
                        }
                    }
                    (Value::Array { .. }, Value::Long(i)) => {
                        let idx = *i as usize;
                        let mut elements = match array {
                            Value::Array { elements } => elements,
                            _ => unreachable!(),
                        };
                        if idx < elements.len() {
                            elements[idx] = value.clone();
                            self.push(Value::Array { elements });
                        } else {
                            return Err(format!("Array index out of bounds: {}", idx));
                        }
                    }
                    _ => {
                        return Err(format!(
                            "ARRAY_SET: invalid index type on array: {:?}[{:?}]",
                            array, index
                        ))
                    }
                }
            }
            OpCode::ARRAY_LEN => {
                let array = self.pop();
                match array {
                    Value::Array { elements } => {
                        self.push(Value::Long(elements.len() as i64));
                    }
                    Value::ClassInstance { class_name, fields, .. } if class_name.starts_with("ArrayList") => {
                        match fields.borrow().get("_elements") {
                            Some(Value::Array { elements }) => {
                                self.push(Value::Long(elements.len() as i64));
                            }
                            _ => self.push(Value::Long(0)),
                        }
                    }
                    _ => return Err(format!("ARRAY_LEN: not an array: {:?}", array)),
                }
            }

            // -- Ownership -------------------------------------------------------
            OpCode::BOX_VALUE => {
                let val = self.pop();
                self.push(Value::Owned(Box::new(val)));
            }
            OpCode::UNBOX_VALUE => {
                let val = self.pop();
                match val {
                    Value::Owned(inner) => self.push(*inner),
                    _ => return Err(format!("UNBOX_VALUE: not an Owned value: {:?}", val)),
                }
            }
            OpCode::REGION_ALLOC => {
                let val = self.pop();
                let idx = self.heap.len();
                self.heap.push(val);
                if let Some(region) = self.region_stack.last_mut() {
                    region.push(idx);
                }
                self.push(Value::Ref(idx));
            }
            OpCode::FREE_REGION => {
                // Pop the current region and mark its heap slots as freed.
                if let Some(indices) = self.region_stack.pop() {
                    for idx in indices {
                        if idx < self.heap.len() {
                            self.heap[idx] = Value::Null;
                        }
                    }
                }
            }
            OpCode::REF_IMMUTABLE => {
                let val = self.pop();
                let idx = self.heap.len();
                self.heap.push(val);
                self.push(Value::Ref(idx));
            }
            OpCode::REF_MUTABLE => {
                let val = self.pop();
                let idx = self.heap.len();
                self.heap.push(val);
                self.push(Value::Ref(idx));
            }
            OpCode::DEREF => {
                let val = self.pop();
                match val {
                    Value::Ref(idx) => {
                        let heap_val = self.heap.get(idx).cloned().ok_or_else(|| {
                            format!("DEREF: invalid heap reference {}", idx)
                        })?;
                        self.push(heap_val);
                    }
                    _ => return Err(format!("DEREF: not a Ref: {:?}", val)),
                }
            }

            // -- Enum ------------------------------------------------------------
            OpCode::ENUM_NEW => {
                let enum_name_idx = self.read_u16();
                let variant_name_idx = self.read_u16();
                let field_count = self.read_u8() as usize;
                // Look up strings from the current chunk
                let (enum_name, variant) = {
                    let frame = self.current_frame();
                    let chunk = &self.functions[frame.function_index as usize].chunk;
                    (
                        chunk.strings[enum_name_idx as usize].clone(),
                        chunk.strings[variant_name_idx as usize].clone(),
                    )
                };
                let mut fields = Vec::with_capacity(field_count);
                for _ in 0..field_count {
                    fields.push(self.pop());
                }
                fields.reverse();
                self.push(Value::EnumInstance {
                    enum_name,
                    variant,
                    fields,
                });
            }

            // -- Result ----------------------------------------------------------
            OpCode::RESULT_OK => {
                let val = self.pop();
                self.push(Value::ResultOk(Box::new(val)));
            }
            OpCode::RESULT_ERR => {
                let val = self.pop();
                self.push(Value::ResultErr(Box::new(val)));
            }
            OpCode::UNWRAP_OR_PROPAGATE => {
                let val = self.pop();
                match &val {
                    Value::ResultErr(_) => {
                        // Propagate: return from the current function with the Err
                        let frame = self.frames.pop().expect("No frame to return from");
                        if self.frames.is_empty() {
                            self.push(val);
                        } else {
                            self.stack.truncate(frame.base);
                            self.push(val);
                            // Signal that we need to keep propagating up
                            // We do this by returning from this step and letting
                            // the outer frame encounter the ResultErr on its stack
                            // and continue propagation.
                            // Actually, we should keep popping frames until we find
                            // a handler. The simplest approach: return the Err as
                            // the function's return value. The caller's code will
                            // have another UNWRAP_OR_PROPAGATE if needed.
                            return Ok(());
                        }
                    }
                    Value::ResultOk(inner) => {
                        self.push(*inner.clone());
                    }
                    _ => {
                        return Err(format!(
                            "UNWRAP_OR_PROPAGATE: not a Result: {:?}",
                            val
                        ))
                    }
                }
            }

            // -- Cast ------------------------------------------------------------
            OpCode::CAST => {
                let target_byte = self.read_u8();
                let target = CastTarget::try_from(target_byte)
                    .map_err(|v| format!("Unknown cast target: {}", v))?;
                let val = self.pop();
                let result = self.eval_cast(&val, target)?;
                self.push(result);
            }

            // -- Iteration -------------------------------------------------------
            OpCode::ITER_NEXT => {
                let offset = self.read_i16();
                // The iterator state is stored as a local variable.
                // We expect the top of stack to hold the iterator value.
                // If it's exhausted (Null or past end), jump by offset.
                // Otherwise push the next element.
                // For simplicity, the compiler manages the iterator counter
                // as a local. The VM just checks: if the value on top is
                // a "sentinel" (Null), jump. Otherwise the value is already
                // the next element.
                let val = self.peek(0);
                if val == Value::Null {
                    // Exhausted – consume the iterator marker and jump
                    self.pop();
                    let new_ip = self.current_frame().ip as isize + offset as isize;
                    self.current_frame_mut().ip = new_ip as usize;
                }
                // If not Null, the value stays on the stack as the iteration value
            }

            // -- Pattern matching ------------------------------------------------
            OpCode::MATCH_ENUM => {
                let variant_name_idx = self.read_u16();
                let _offset = self.read_i16(); // consumed but not used for jumping
                let val = self.pop();
                match &val {
                    Value::EnumInstance { variant, fields, .. } => {
                        let expected_variant = {
                            let frame = self.current_frame();
                            let chunk = &self.functions[frame.function_index as usize].chunk;
                            chunk.strings[variant_name_idx as usize].clone()
                        };
                        if variant == &expected_variant {
                            // Match: push the fields as individual values, then true
                            for f in fields {
                                self.push(f.clone());
                            }
                            self.push(Value::Bool(true));
                        } else {
                            // No match: push false
                            self.push(Value::Bool(false));
                        }
                    }
                    _ => {
                        // Not an enum instance: push false
                        self.push(Value::Bool(false));
                    }
                }
            }
            OpCode::MATCH_OK => {
                let _offset = self.read_i16(); // consumed but not used for jumping
                let val = self.pop();
                match &val {
                    Value::ResultOk(inner) => {
                        self.push(*inner.clone());
                        self.push(Value::Bool(true));
                    }
                    _ => {
                        // Not ResultOk: push false
                        self.push(Value::Bool(false));
                    }
                }
            }
            OpCode::MATCH_ERR => {
                let _offset = self.read_i16(); // consumed but not used for jumping
                let val = self.pop();
                match &val {
                    Value::ResultErr(inner) => {
                        self.push(*inner.clone());
                        self.push(Value::Bool(true));
                    }
                    _ => {
                        // Not ResultErr: push false
                        self.push(Value::Bool(false));
                    }
                }
            }

            // -- Static calls ----------------------------------------------------
            OpCode::STATIC_CALL => {
                let class_name_idx = self.read_u16();
                let method_name_idx = self.read_u16();
                let arg_count = self.read_u8();
                self.exec_static_call(class_name_idx, method_name_idx, arg_count)?;
            }

            // -- Type narrowing --------------------------------------------------
            OpCode::TYPE_CHECK => {
                let tag_byte = self.read_u8();
                let tag = TypeTag::try_from(tag_byte)
                    .map_err(|v| format!("Unknown type tag: {}", v))?;
                let val = self.pop();
                let matches = self.check_type_tag(&val, tag);
                self.push(Value::Bool(matches));
            }

            // -- Super constructor call -----------------------------------------
            OpCode::CALL_SUPER => {
                // Operands: func_idx (u16), user_arg_count (u8)
                // Stack: [this, user_arg0, user_arg1, ...]
                // This is like CALL but the base is set to `this` position
                // and arity check uses user_arg_count (not including `this`).
                let func_idx = self.read_u16();
                let user_arg_count = self.read_u8() as usize;
                let fi = func_idx as usize;
                if fi >= self.functions.len() {
                    return Err(format!("CALL_SUPER: function index {} out of range", func_idx));
                }
                // The total items on stack for this call: 1 (this) + user_arg_count
                // Base points to `this`
                let base = self.stack.len() - 1 - user_arg_count;
                self.frames.push(Frame::new(func_idx, base));
                // Pre-allocate local slots
                let local_count = self.functions[fi].local_count;
                let needed = base + local_count;
                while self.stack.len() < needed {
                    self.stack.push(Value::Null);
                }
            }
            OpCode::TUPLE_NEW => {
                let count = self.read_u16() as usize;
                let mut elements = Vec::with_capacity(count);
                for _ in 0..count {
                    elements.push(Value::Null);
                }
                // Pop values from stack in reverse order
                for i in (0..count).rev() {
                    elements[i] = self.pop();
                }
                self.push(Value::Tuple { elements });
            }
            OpCode::TUPLE_GET => {
                let index = self.read_u8() as usize;
                let tuple = self.pop();
                match &tuple {
                    Value::Tuple { elements } => {
                        if index < elements.len() {
                            self.push(elements[index].clone());
                        } else {
                            return Err(format!(
                                "TUPLE_GET: index {} out of bounds (length {})",
                                index,
                                elements.len()
                            ));
                        }
                    }
                    _ => {
                        return Err(format!(
                            "TUPLE_GET: expected tuple, found {:?}",
                            tuple
                        ))
                    }
                }
            }

            // -- Closures --------------------------------------------------------
            OpCode::CLOSURE_NEW => {
                let func_idx = self.read_u16() as usize;
                let upvalue_count = self.read_u8() as usize;
                let mut upvalues = Vec::with_capacity(upvalue_count);
                for _ in 0..upvalue_count {
                    upvalues.push(self.pop());
                }
                upvalues.reverse();
                self.push(Value::Closure {
                    func_idx,
                    upvalues,
                });
            }
            OpCode::GET_UPVALUE => {
                let slot = self.read_u8() as usize;
                let upvalues = self.current_frame().upvalues.clone();
                match &upvalues {
                    Some(uvs) => {
                        if slot < uvs.len() {
                            self.push(uvs[slot].clone());
                        } else {
                            return Err(format!(
                                "GET_UPVALUE: index {} out of bounds (upvalue count {})",
                                slot,
                                uvs.len()
                            ));
                        }
                    }
                    None => {
                        return Err("GET_UPVALUE: no upvalues in current frame".to_string());
                    }
                }
            }
            OpCode::SET_UPVALUE => {
                let slot = self.read_u8() as usize;
                let val = self.pop();
                let upvalues = self.current_frame_mut().upvalues.as_mut();
                match upvalues {
                    Some(uvs) => {
                        if slot < uvs.len() {
                            uvs[slot] = val;
                        } else {
                            return Err(format!(
                                "SET_UPVALUE: index {} out of bounds (upvalue count {})",
                                slot,
                                uvs.len()
                            ));
                        }
                    }
                    None => {
                        return Err("SET_UPVALUE: no upvalues in current frame".to_string());
                    }
                }
            }
            OpCode::CLOSURE_NEW_CAPTURED => {
                let func_idx = self.read_u16() as usize;
                let capture_count = self.read_u8() as usize;
                let mut upvalues = Vec::with_capacity(capture_count);
                for _ in 0..capture_count {
                    upvalues.push(self.pop());
                }
                upvalues.reverse();
                self.push(Value::Closure {
                    func_idx,
                    upvalues,
                });
            }
            OpCode::CLOSURE_CAPTURE => {
                let slot = self.read_u8() as usize;
                let frame = self.current_frame();
                let base = frame.base;
                if base + slot < self.stack.len() {
                    let val = self.stack[base + slot].clone();
                    self.push(val);
                } else {
                    return Err(format!(
                        "CLOSURE_CAPTURE: local slot {} out of bounds (stack len {})",
                        slot,
                        self.stack.len()
                    ));
                }
            }
        }

        Ok(())
    }
}
