// Titrate Alpha 0.2 – bytecode virtual machine: step dispatch
// Precision in every step – richie-rich90454, 2026

use super::super::frame::{ExceptionHandler, Frame};
use super::super::opcodes::{OpCode, CastTarget, TypeTag};
use super::super::value::{Value, values_eq};
use super::Vm;
use std::cell::RefCell;
use std::char;
use std::rc::Rc;

impl Vm {
    /// Pop a value, auto-unwrapping ResultOk to its inner value.
    /// This allows arithmetic/comparison operations to work seamlessly
    /// when a function returns Result<T> but the caller uses the value directly.
    fn pop_unwrapped(&mut self) -> Value {
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
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x.wrapping_add(*y))),
                    // String/Char concat takes priority over numeric coercion
                    (Value::String(_), _) | (_, Value::String(_))
                        | (Value::Char(_), _) | (_, Value::Char(_)) => {
                        self.push(Value::String(Rc::new(format!("{}{}", a.display_string(), b.display_string()))));
                    }
                    // Numeric coercion fallback
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_i64(), b.to_i64()) {
                            self.push(Value::Int(x.wrapping_add(y) as i32));
                        } else {
                            return Err(format!("ADD_I32: type mismatch {:?} + {:?}", a, b));
                        }
                    }
                }
            }
            OpCode::ADD_I64 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
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
                    (Value::String(x), Value::String(y)) => {
                        self.push(Value::String(Rc::new(format!("{}{}", x, y))))
                    }
                    (Value::String(x), Value::Char(y)) => {
                        self.push(Value::String(Rc::new(format!("{}{}", x, y))))
                    }
                    (Value::Char(x), Value::String(y)) => {
                        self.push(Value::String(Rc::new(format!("{}{}", x, y))))
                    }
                    (Value::Char(x), Value::Char(y)) => {
                        self.push(Value::String(Rc::new(format!("{}{}", x, y))))
                    }
                    (Value::Byte(x), Value::Byte(y)) => self.push(Value::Byte(x.wrapping_add(*y))),
                    (Value::Short(x), Value::Short(y)) => self.push(Value::Short(x.wrapping_add(*y))),
                    (Value::Byte(x), Value::Int(y)) => self.push(Value::Int((*x as i32).wrapping_add(*y))),
                    (Value::Int(x), Value::Byte(y)) => self.push(Value::Int(x.wrapping_add(*y as i32))),
                    (Value::Short(x), Value::Int(y)) => self.push(Value::Int((*x as i32).wrapping_add(*y))),
                    (Value::Int(x), Value::Short(y)) => self.push(Value::Int(x.wrapping_add(*y as i32))),
                    // String/Char concat for any remaining combinations (e.g. Long + Char)
                    (Value::String(_), _) | (_, Value::String(_))
                        | (Value::Char(_), _) | (_, Value::Char(_)) => {
                        self.push(Value::String(Rc::new(format!("{}{}", a.display_string(), b.display_string()))));
                    }
                    // Numeric coercion fallback (treats Null as 0)
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_i64(), b.to_i64()) {
                            self.push(Value::Long(x.wrapping_add(y)));
                        } else {
                            return Err(format!("ADD_I64: type mismatch {:?} + {:?}", a, b));
                        }
                    }
                }
            }
            OpCode::ADD_F32 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Float(x), Value::Float(y)) => self.push(Value::Float(x + y)),
                    // String/Char concat takes priority over numeric coercion
                    (Value::String(_), _) | (_, Value::String(_))
                        | (Value::Char(_), _) | (_, Value::Char(_)) => {
                        self.push(Value::String(Rc::new(format!("{}{}", a.display_string(), b.display_string()))));
                    }
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_f64(), b.to_f64()) {
                            self.push(Value::Float((x + y) as f32));
                        } else {
                            return Err(format!("ADD_F32: type mismatch {:?} + {:?}", a, b));
                        }
                    }
                }
            }
            OpCode::ADD_F64 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Double(x + y)),
                    // String/Char concat takes priority over numeric coercion
                    (Value::String(_), _) | (_, Value::String(_))
                        | (Value::Char(_), _) | (_, Value::Char(_)) => {
                        self.push(Value::String(Rc::new(format!("{}{}", a.display_string(), b.display_string()))));
                    }
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_f64(), b.to_f64()) {
                            self.push(Value::Double(x + y));
                        } else {
                            return Err(format!("ADD_F64: type mismatch {:?} + {:?}", a, b));
                        }
                    }
                }
            }
            OpCode::SUB_I32 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x.wrapping_sub(*y))),
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_i64(), b.to_i64()) {
                            self.push(Value::Int(x.wrapping_sub(y) as i32));
                        } else {
                            return Err(format!("SUB_I32: type mismatch {:?} - {:?}", a, b));
                        }
                    }
                }
            }
            OpCode::SUB_I64 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
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
                    _ => {
                        let a_is_float = matches!(a, Value::Float(_) | Value::Double(_) | Value::Half(_) | Value::Quad(_));
                        let b_is_float = matches!(b, Value::Float(_) | Value::Double(_) | Value::Half(_) | Value::Quad(_));
                        if a_is_float || b_is_float {
                            if let (Some(x), Some(y)) = (a.to_f64(), b.to_f64()) {
                                self.push(Value::Double(x - y));
                            } else {
                                return Err(format!("SUB_I64: type mismatch {:?} - {:?}", a, b));
                            }
                        } else if let (Some(x), Some(y)) = (a.to_i64(), b.to_i64()) {
                            self.push(Value::Long(x.wrapping_sub(y)));
                        } else {
                            return Err(format!("SUB_I64: type mismatch {:?} - {:?}", a, b));
                        }
                    }
                }
            }
            OpCode::SUB_F32 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Float(x), Value::Float(y)) => self.push(Value::Float(x - y)),
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_f64(), b.to_f64()) {
                            self.push(Value::Float((x - y) as f32));
                        } else {
                            return Err(format!("SUB_F32: type mismatch {:?} - {:?}", a, b));
                        }
                    }
                }
            }
            OpCode::SUB_F64 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Double(x - y)),
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_f64(), b.to_f64()) {
                            self.push(Value::Double(x - y));
                        } else {
                            return Err(format!("SUB_F64: type mismatch {:?} - {:?}", a, b));
                        }
                    }
                }
            }
            OpCode::MUL_I32 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x.wrapping_mul(*y))),
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_i64(), b.to_i64()) {
                            self.push(Value::Int(x.wrapping_mul(y) as i32));
                        } else {
                            return Err(format!("MUL_I32: type mismatch {:?} * {:?}", a, b));
                        }
                    }
                }
            }
            OpCode::MUL_I64 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
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
                    _ => {
                        let a_is_float = matches!(a, Value::Float(_) | Value::Double(_) | Value::Half(_) | Value::Quad(_));
                        let b_is_float = matches!(b, Value::Float(_) | Value::Double(_) | Value::Half(_) | Value::Quad(_));
                        if a_is_float || b_is_float {
                            if let (Some(x), Some(y)) = (a.to_f64(), b.to_f64()) {
                                self.push(Value::Double(x * y));
                            } else {
                                return Err(format!("MUL_I64: type mismatch {:?} * {:?}", a, b));
                            }
                        } else if let (Some(x), Some(y)) = (a.to_i64(), b.to_i64()) {
                            self.push(Value::Long(x.wrapping_mul(y)));
                        } else {
                            return Err(format!("MUL_I64: type mismatch {:?} * {:?}", a, b));
                        }
                    }
                }
            }
            OpCode::MUL_F32 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Float(x), Value::Float(y)) => self.push(Value::Float(x * y)),
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_f64(), b.to_f64()) {
                            self.push(Value::Float((x * y) as f32));
                        } else {
                            return Err(format!("MUL_F32: type mismatch {:?} * {:?}", a, b));
                        }
                    }
                }
            }
            OpCode::MUL_F64 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Double(x * y)),
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_f64(), b.to_f64()) {
                            self.push(Value::Double(x * y));
                        } else {
                            return Err(format!("MUL_F64: type mismatch {:?} * {:?}", a, b));
                        }
                    }
                }
            }
            OpCode::DIV_I32 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Int(_), Value::Int(0)) => {
                        return Err("Division by zero (int)".to_string());
                    }
                    (Value::Int(x), Value::Int(y)) => {
                        self.push(Value::Int(x.wrapping_div(*y)));
                    }
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_i64(), b.to_i64()) {
                            if y == 0 {
                                return Err("Division by zero (int)".to_string());
                            }
                            self.push(Value::Int(x.wrapping_div(y) as i32));
                        } else {
                            return Err(format!("DIV_I32: type mismatch {:?} / {:?}", a, b));
                        }
                    }
                }
            }
            OpCode::DIV_I64 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
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
                    _ => {
                        let a_is_float = matches!(a, Value::Float(_) | Value::Double(_) | Value::Half(_) | Value::Quad(_));
                        let b_is_float = matches!(b, Value::Float(_) | Value::Double(_) | Value::Half(_) | Value::Quad(_));
                        if a_is_float || b_is_float {
                            if let (Some(x), Some(y)) = (a.to_f64(), b.to_f64()) {
                                self.push(Value::Double(x / y));
                            } else {
                                return Err(format!("DIV_I64: type mismatch {:?} / {:?}", a, b));
                            }
                        } else if let (Some(x), Some(y)) = (a.to_i64(), b.to_i64()) {
                            if y == 0 {
                                return Err("Division by zero (long)".to_string());
                            }
                            self.push(Value::Long(x.wrapping_div(y)));
                        } else {
                            return Err(format!("DIV_I64: type mismatch {:?} / {:?}", a, b));
                        }
                    }
                }
            }
            OpCode::DIV_F32 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Float(x), Value::Float(y)) => self.push(Value::Float(x / y)),
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_f64(), b.to_f64()) {
                            self.push(Value::Float((x / y) as f32));
                        } else {
                            return Err(format!("DIV_F32: type mismatch {:?} / {:?}", a, b));
                        }
                    }
                }
            }
            OpCode::DIV_F64 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Double(x / y)),
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_f64(), b.to_f64()) {
                            self.push(Value::Double(x / y));
                        } else {
                            return Err(format!("DIV_F64: type mismatch {:?} / {:?}", a, b));
                        }
                    }
                }
            }
            OpCode::MOD_I32 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Int(_), Value::Int(0)) => {
                        return Err("Remainder division by zero (int)".to_string());
                    }
                    (Value::Int(x), Value::Int(y)) => {
                        self.push(Value::Int(x.wrapping_rem(*y)));
                    }
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_i64(), b.to_i64()) {
                            if y == 0 {
                                return Err("Remainder division by zero (int)".to_string());
                            }
                            self.push(Value::Int(x.wrapping_rem(y) as i32));
                        } else {
                            return Err(format!("MOD_I32: type mismatch {:?} % {:?}", a, b));
                        }
                    }
                }
            }
            OpCode::MOD_I64 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Long(_), Value::Long(0)) => {
                        return Err("Remainder division by zero (long)".to_string());
                    }
                    (Value::Long(x), Value::Long(y)) => {
                        self.push(Value::Long(x.wrapping_rem(*y)));
                    }
                    (Value::Int(_), Value::Int(0)) => {
                        return Err("Remainder division by zero (int)".to_string());
                    }
                    (Value::Int(x), Value::Int(y)) => {
                        self.push(Value::Int(x.wrapping_rem(*y)));
                    }
                    (Value::Int(x), Value::Long(y)) => {
                        if *y == 0 {
                            return Err("Remainder division by zero (int % long)".to_string());
                        }
                        self.push(Value::Long((*x as i64).wrapping_rem(*y)));
                    }
                    (Value::Long(x), Value::Int(y)) => {
                        if *y == 0 {
                            return Err("Remainder division by zero (long % int)".to_string());
                        }
                        self.push(Value::Long(x.wrapping_rem(*y as i64)));
                    }
                    (Value::Byte(_), Value::Byte(0)) | (Value::Byte(_), Value::Int(0)) | (Value::Byte(_), Value::Long(0)) => {
                        return Err("Remainder division by zero (byte)".to_string());
                    }
                    (Value::Byte(x), Value::Byte(y)) => self.push(Value::Byte(x.wrapping_rem(*y))),
                    (Value::Byte(x), Value::Int(y)) => self.push(Value::Byte(x.wrapping_rem(*y as i8))),
                    (Value::Byte(x), Value::Long(y)) => self.push(Value::Long((*x as i64).wrapping_rem(*y))),
                    (Value::Short(_), Value::Short(0)) | (Value::Short(_), Value::Int(0)) | (Value::Short(_), Value::Long(0)) => {
                        return Err("Remainder division by zero (short)".to_string());
                    }
                    (Value::Short(x), Value::Short(y)) => self.push(Value::Short(x.wrapping_rem(*y))),
                    (Value::Short(x), Value::Int(y)) => self.push(Value::Short(x.wrapping_rem(*y as i16))),
                    (Value::Short(x), Value::Long(y)) => self.push(Value::Long((*x as i64).wrapping_rem(*y))),
                    _ => {
                        let a_is_float = matches!(a, Value::Float(_) | Value::Double(_) | Value::Half(_) | Value::Quad(_));
                        let b_is_float = matches!(b, Value::Float(_) | Value::Double(_) | Value::Half(_) | Value::Quad(_));
                        if a_is_float || b_is_float {
                            if let (Some(x), Some(y)) = (a.to_f64(), b.to_f64()) {
                                self.push(Value::Double(x % y));
                            } else {
                                return Err(format!("MOD_I64: type mismatch {:?} % {:?}", a, b));
                            }
                        } else if let (Some(x), Some(y)) = (a.to_i64(), b.to_i64()) {
                            if y == 0 {
                                return Err("Remainder division by zero (long)".to_string());
                            }
                            self.push(Value::Long(x.wrapping_rem(y)));
                        } else {
                            return Err(format!("MOD_I64: type mismatch {:?} % {:?}", a, b));
                        }
                    }
                }
            }
            OpCode::MOD_F32 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Float(x), Value::Float(y)) => self.push(Value::Float(x % y)),
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_f64(), b.to_f64()) {
                            self.push(Value::Float((x % y) as f32));
                        } else {
                            return Err(format!("MOD_F32: type mismatch {:?} % {:?}", a, b));
                        }
                    }
                }
            }
            OpCode::MOD_F64 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Double(x % y)),
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_f64(), b.to_f64()) {
                            self.push(Value::Double(x % y));
                        } else {
                            return Err(format!("MOD_F64: type mismatch {:?} % {:?}", a, b));
                        }
                    }
                }
            }
            OpCode::NEG_I32 => {
                let a = self.pop_unwrapped();
                match a {
                    Value::Int(x) => self.push(Value::Int(x.wrapping_neg())),
                    other => {
                        if let Some(x) = other.to_i64() {
                            self.push(Value::Int(x.wrapping_neg() as i32));
                        } else {
                            return Err(format!("NEG_I32: type mismatch {:?}", other));
                        }
                    }
                }
            }
            OpCode::NEG_I64 => {
                let a = self.pop_unwrapped();
                match a {
                    Value::Long(x) => self.push(Value::Long(x.wrapping_neg())),
                    Value::Int(x) => self.push(Value::Int(x.wrapping_neg())),
                    Value::Double(x) => self.push(Value::Double(-x)),
                    Value::Float(x) => self.push(Value::Float(-x)),
                    Value::Byte(x) => self.push(Value::Byte(x.wrapping_neg())),
                    Value::Short(x) => self.push(Value::Short(x.wrapping_neg())),
                    other => {
                        if let Some(x) = other.to_i64() {
                            self.push(Value::Long(x.wrapping_neg()));
                        } else {
                            return Err(format!("NEG_I64: type mismatch {:?}", other));
                        }
                    }
                }
            }
            OpCode::NEG_F32 => {
                let a = self.pop_unwrapped();
                match a {
                    Value::Float(x) => self.push(Value::Float(-x)),
                    other => {
                        if let Some(x) = other.to_f64() {
                            self.push(Value::Float((-x) as f32));
                        } else {
                            return Err(format!("NEG_F32: type mismatch {:?}", other));
                        }
                    }
                }
            }
            OpCode::NEG_F64 => {
                let a = self.pop_unwrapped();
                match a {
                    Value::Double(x) => self.push(Value::Double(-x)),
                    other => {
                        if let Some(x) = other.to_f64() {
                            self.push(Value::Double(-x));
                        } else {
                            return Err(format!("NEG_F64: type mismatch {:?}", other));
                        }
                    }
                }
            }

            // -- Bitwise ---------------------------------------------------------
            OpCode::BITAND_I32 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x & y)),
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_i64(), b.to_i64()) {
                            self.push(Value::Int((x & y) as i32));
                        } else {
                            return Err(format!("BITAND_I32: type mismatch {:?}", a));
                        }
                    }
                }
            }
            OpCode::BITAND_I64 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => self.push(Value::Long(x & y)),
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_i64(), b.to_i64()) {
                            self.push(Value::Long(x & y));
                        } else {
                            return Err(format!("BITAND_I64: type mismatch {:?}", a));
                        }
                    }
                }
            }
            OpCode::BITOR_I32 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x | y)),
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_i64(), b.to_i64()) {
                            self.push(Value::Int((x | y) as i32));
                        } else {
                            return Err(format!("BITOR_I32: type mismatch {:?}", a));
                        }
                    }
                }
            }
            OpCode::BITOR_I64 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => self.push(Value::Long(x | y)),
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_i64(), b.to_i64()) {
                            self.push(Value::Long(x | y));
                        } else {
                            return Err(format!("BITOR_I64: type mismatch {:?}", a));
                        }
                    }
                }
            }
            OpCode::BITXOR_I32 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x ^ y)),
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_i64(), b.to_i64()) {
                            self.push(Value::Int((x ^ y) as i32));
                        } else {
                            return Err(format!("BITXOR_I32: type mismatch {:?}", a));
                        }
                    }
                }
            }
            OpCode::BITXOR_I64 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => self.push(Value::Long(x ^ y)),
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x ^ y)),
                    (Value::Int(x), Value::Long(y)) => self.push(Value::Long((*x as i64) ^ y)),
                    (Value::Long(x), Value::Int(y)) => self.push(Value::Long(x ^ (*y as i64))),
                    (Value::Byte(x), Value::Byte(y)) => self.push(Value::Byte(x ^ y)),
                    (Value::Short(x), Value::Short(y)) => self.push(Value::Short(x ^ y)),
                    (Value::Char(x), Value::Char(y)) => self.push(Value::Char(char::from_u32((*x as u32) ^ (*y as u32)).unwrap_or(*x))),
                    (Value::Char(x), Value::Int(y)) => self.push(Value::Int((*x as u32 as i32) ^ y)),
                    (Value::Int(x), Value::Char(y)) => self.push(Value::Int(x ^ (*y as u32 as i32))),
                    (Value::Char(x), Value::Long(y)) => self.push(Value::Long((*x as u32 as i64) ^ y)),
                    (Value::Long(x), Value::Char(y)) => self.push(Value::Long(x ^ (*y as u32 as i64))),
                    (Value::Bool(x), Value::Bool(y)) => self.push(Value::Bool(x ^ y)),
                    _ => return Err(format!("BITXOR_I64: type mismatch {:?} ^ {:?}", a, b)),
                }
            }
            OpCode::SHL_I32 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x.wrapping_shl(*y as u32))),
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_i64(), b.to_i64()) {
                            self.push(Value::Int(x.wrapping_shl(y as u32) as i32));
                        } else {
                            return Err(format!("SHL_I32: type mismatch {:?}", a));
                        }
                    }
                }
            }
            OpCode::SHL_I64 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => self.push(Value::Long(x.wrapping_shl(*y as u32))),
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_i64(), b.to_i64()) {
                            self.push(Value::Long(x.wrapping_shl(y as u32)));
                        } else {
                            return Err(format!("SHL_I64: type mismatch {:?}", a));
                        }
                    }
                }
            }
            OpCode::SHR_I32 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x.wrapping_shr(*y as u32))),
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_i64(), b.to_i64()) {
                            self.push(Value::Int(x.wrapping_shr(y as u32) as i32));
                        } else {
                            return Err(format!("SHR_I32: type mismatch {:?}", a));
                        }
                    }
                }
            }
            OpCode::SHR_I64 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => self.push(Value::Long(x.wrapping_shr(*y as u32))),
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_i64(), b.to_i64()) {
                            self.push(Value::Long(x.wrapping_shr(y as u32)));
                        } else {
                            return Err(format!("SHR_I64: type mismatch {:?}", a));
                        }
                    }
                }
            }
            OpCode::USHR_I32 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => {
                        // Unsigned (logical) right shift: cast to u32, shift, cast back
                        self.push(Value::Int(((*x as u32) >> (*y as u32)) as i32));
                    }
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_i64(), b.to_i64()) {
                            self.push(Value::Int(((x as u32) >> (y as u32)) as i32));
                        } else {
                            return Err(format!("USHR_I32: type mismatch {:?}", a));
                        }
                    }
                }
            }
            OpCode::USHR_I64 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => {
                        // Unsigned (logical) right shift: cast to u64, shift, cast back
                        self.push(Value::Long(((*x as u64) >> (*y as u32)) as i64));
                    }
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_i64(), b.to_i64()) {
                            self.push(Value::Long(((x as u64) >> (y as u32)) as i64));
                        } else {
                            return Err(format!("USHR_I64: type mismatch {:?}", a));
                        }
                    }
                }
            }
            OpCode::BITNOT_I32 => {
                let a = self.pop_unwrapped();
                match a {
                    Value::Int(x) => self.push(Value::Int(!x)),
                    other => {
                        if let Some(x) = other.to_i64() {
                            self.push(Value::Int(!x as i32));
                        } else {
                            return Err(format!("BITNOT_I32: type mismatch {:?}", other));
                        }
                    }
                }
            }
            OpCode::BITNOT_I64 => {
                let a = self.pop_unwrapped();
                match a {
                    Value::Long(x) => self.push(Value::Long(!x)),
                    other => {
                        if let Some(x) = other.to_i64() {
                            self.push(Value::Long(!x));
                        } else {
                            return Err(format!("BITNOT_I64: type mismatch {:?}", other));
                        }
                    }
                }
            }

            // -- Comparison ------------------------------------------------------
            OpCode::EQ_I32 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x == y)),
                    (Value::Null, Value::Null) => self.push(Value::Bool(true)),
                    (Value::Null, _) | (_, Value::Null) => self.push(Value::Bool(false)),
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_i64(), b.to_i64()) {
                            self.push(Value::Bool(x == y));
                        } else {
                            return Err(format!("EQ_I32: type mismatch {:?}", a));
                        }
                    }
                }
            }
            OpCode::EQ_I64 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => self.push(Value::Bool(x == y)),
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x == y)),
                    (Value::Int(x), Value::Long(y)) => self.push(Value::Bool((*x as i64) == *y)),
                    (Value::Long(x), Value::Int(y)) => self.push(Value::Bool(*x == (*y as i64))),
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Bool(x == y)),
                    (Value::String(x), Value::String(y)) => self.push(Value::Bool(x == y)),
                    (Value::Bool(x), Value::Bool(y)) => self.push(Value::Bool(x == y)),
                    (Value::Char(x), Value::Char(y)) => self.push(Value::Bool(x == y)),
                    (Value::Null, Value::Null) => self.push(Value::Bool(true)),
                    (Value::Null, _) | (_, Value::Null) => self.push(Value::Bool(false)),
                    (Value::ClassInstance { fields: f1, .. }, Value::ClassInstance { fields: f2, .. }) => {
                        self.push(Value::Bool(Rc::ptr_eq(f1, f2)))
                    }
                    (Value::EnumInstance { enum_name: en1, variant: v1, fields: f1 },
                     Value::EnumInstance { enum_name: en2, variant: v2, fields: f2 }) => {
                        let same = en1 == en2 && v1 == v2 && f1.len() == f2.len()
                            && f1.iter().zip(f2.iter()).all(|(x, y)| values_eq(x, y));
                        self.push(Value::Bool(same))
                    }
                    (Value::Array { elements: e1 }, Value::Array { elements: e2 }) => {
                        self.push(Value::Bool(e1.len() == e2.len() && e1.iter().zip(e2.iter()).all(|(x, y)| values_eq(x, y))))
                    }
                    (Value::Tuple { elements: e1 }, Value::Tuple { elements: e2 }) => {
                        self.push(Value::Bool(e1.len() == e2.len() && e1.iter().zip(e2.iter()).all(|(x, y)| values_eq(x, y))))
                    }
                    // Cross-type comparisons: different types are never equal.
                    (Value::Bool(_), _) | (_, Value::Bool(_)) => self.push(Value::Bool(false)),
                    (Value::EnumInstance { .. }, _) | (_, Value::EnumInstance { .. }) => self.push(Value::Bool(false)),
                    (Value::ClassInstance { .. }, _) | (_, Value::ClassInstance { .. }) => self.push(Value::Bool(false)),
                    (Value::Array { .. }, _) | (_, Value::Array { .. }) => self.push(Value::Bool(false)),
                    (Value::Tuple { .. }, _) | (_, Value::Tuple { .. }) => self.push(Value::Bool(false)),
                    (Value::Char(x), Value::String(y)) => {
                        self.push(Value::Bool(y.starts_with(*x)))
                    }
                    (Value::String(x), Value::Char(y)) => {
                        self.push(Value::Bool(x.starts_with(*y)))
                    }
                    (Value::Char(_), _) | (_, Value::Char(_)) => self.push(Value::Bool(false)),
                    _ => {
                        match (a.to_i64(), b.to_i64()) {
                            (Some(x), Some(y)) => self.push(Value::Bool(x == y)),
                            _ => return Err(format!("EQ_I64: type mismatch {:?} == {:?}", a, b)),
                        }
                    }
                }
            }
            OpCode::EQ_F32 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Float(x), Value::Float(y)) => {
                        self.push(Value::Bool(x == y))
                    }
                    _ => {
                        match (a.to_f64(), b.to_f64()) {
                            (Some(x), Some(y)) => self.push(Value::Bool(x == y)),
                            _ => self.push(Value::Bool(false)),
                        }
                    }
                }
            }
            OpCode::EQ_F64 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Double(x), Value::Double(y)) => {
                        self.push(Value::Bool(x == y))
                    }
                    _ => {
                        match (a.to_f64(), b.to_f64()) {
                            (Some(x), Some(y)) => self.push(Value::Bool(x == y)),
                            _ => self.push(Value::Bool(false)),
                        }
                    }
                }
            }
            OpCode::EQ_BOOL => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Bool(x), Value::Bool(y)) => self.push(Value::Bool(x == y)),
                    _ => return Err(format!("EQ_BOOL: type mismatch {:?}", a)),
                }
            }
            OpCode::EQ_CHAR => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Char(x), Value::Char(y)) => self.push(Value::Bool(x == y)),
                    _ => return Err(format!("EQ_CHAR: type mismatch {:?}", a)),
                }
            }
            OpCode::EQ_STRING => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::String(x), Value::String(y)) => self.push(Value::Bool(x == y)),
                    (Value::Char(x), Value::Char(y)) => self.push(Value::Bool(x == y)),
                    (Value::String(x), Value::Char(y)) => {
                        self.push(Value::Bool(x.starts_with(*y)))
                    }
                    (Value::Char(x), Value::String(y)) => {
                        self.push(Value::Bool(y.starts_with(*x)))
                    }
                    (Value::Null, Value::String(_)) | (Value::String(_), Value::Null) => self.push(Value::Bool(false)),
                    (Value::Null, Value::Null) => self.push(Value::Bool(true)),
                    _ => return Err(format!("EQ_STRING: type mismatch {:?}", a)),
                }
            }
            OpCode::NE_I32 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x != y)),
                    (Value::Null, Value::Null) => self.push(Value::Bool(false)),
                    (Value::Null, _) | (_, Value::Null) => self.push(Value::Bool(true)),
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_i64(), b.to_i64()) {
                            self.push(Value::Bool(x != y));
                        } else {
                            return Err(format!("NE_I32: type mismatch {:?}", a));
                        }
                    }
                }
            }
            OpCode::NE_I64 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Long(x), Value::Long(y)) => self.push(Value::Bool(x != y)),
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x != y)),
                    (Value::Int(x), Value::Long(y)) => self.push(Value::Bool((*x as i64) != *y)),
                    (Value::Long(x), Value::Int(y)) => self.push(Value::Bool(*x != (*y as i64))),
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Bool(x != y)),
                    (Value::String(x), Value::String(y)) => self.push(Value::Bool(x != y)),
                    (Value::Bool(x), Value::Bool(y)) => self.push(Value::Bool(x != y)),
                    (Value::Char(x), Value::Char(y)) => self.push(Value::Bool(x != y)),
                    (Value::Null, Value::Null) => self.push(Value::Bool(false)),
                    (Value::Null, _) | (_, Value::Null) => self.push(Value::Bool(true)),
                    (Value::ClassInstance { fields: f1, .. }, Value::ClassInstance { fields: f2, .. }) => {
                        self.push(Value::Bool(!Rc::ptr_eq(f1, f2)))
                    }
                    (Value::EnumInstance { enum_name: en1, variant: v1, fields: f1 },
                     Value::EnumInstance { enum_name: en2, variant: v2, fields: f2 }) => {
                        let same = en1 == en2 && v1 == v2 && f1.len() == f2.len()
                            && f1.iter().zip(f2.iter()).all(|(x, y)| values_eq(x, y));
                        self.push(Value::Bool(!same))
                    }
                    (Value::Array { elements: e1 }, Value::Array { elements: e2 }) => {
                        self.push(Value::Bool(!(e1.len() == e2.len() && e1.iter().zip(e2.iter()).all(|(x, y)| values_eq(x, y)))))
                    }
                    (Value::Tuple { elements: e1 }, Value::Tuple { elements: e2 }) => {
                        self.push(Value::Bool(!(e1.len() == e2.len() && e1.iter().zip(e2.iter()).all(|(x, y)| values_eq(x, y)))))
                    }
                    // Cross-type comparisons: different types are always not-equal.
                    (Value::Bool(_), _) | (_, Value::Bool(_)) => self.push(Value::Bool(true)),
                    (Value::EnumInstance { .. }, _) | (_, Value::EnumInstance { .. }) => self.push(Value::Bool(true)),
                    (Value::ClassInstance { .. }, _) | (_, Value::ClassInstance { .. }) => self.push(Value::Bool(true)),
                    (Value::Array { .. }, _) | (_, Value::Array { .. }) => self.push(Value::Bool(true)),
                    (Value::Tuple { .. }, _) | (_, Value::Tuple { .. }) => self.push(Value::Bool(true)),
                    (Value::Char(x), Value::String(y)) => {
                        self.push(Value::Bool(!y.starts_with(*x)))
                    }
                    (Value::String(x), Value::Char(y)) => {
                        self.push(Value::Bool(!x.starts_with(*y)))
                    }
                    (Value::Char(_), _) | (_, Value::Char(_)) => self.push(Value::Bool(true)),
                    _ => {
                        match (a.to_i64(), b.to_i64()) {
                            (Some(x), Some(y)) => self.push(Value::Bool(x != y)),
                            _ => return Err(format!("NE_I64: type mismatch {:?} != {:?}", a, b)),
                        }
                    }
                }
            }
            OpCode::NE_F32 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Float(x), Value::Float(y)) => {
                        self.push(Value::Bool(x != y))
                    }
                    _ => {
                        match (a.to_f64(), b.to_f64()) {
                            (Some(x), Some(y)) => self.push(Value::Bool(x != y)),
                            _ => self.push(Value::Bool(true)),
                        }
                    }
                }
            }
            OpCode::NE_F64 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Double(x), Value::Double(y)) => {
                        self.push(Value::Bool(x != y))
                    }
                    _ => {
                        match (a.to_f64(), b.to_f64()) {
                            (Some(x), Some(y)) => self.push(Value::Bool(x != y)),
                            _ => self.push(Value::Bool(true)),
                        }
                    }
                }
            }
            OpCode::LT_I32 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x < y)),
                    (Value::Null, _) | (_, Value::Null) => self.push(Value::Bool(false)),
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_i64(), b.to_i64()) {
                            self.push(Value::Bool(x < y));
                        } else {
                            return Err(format!("LT_I32: type mismatch {:?}", a));
                        }
                    }
                }
            }
            OpCode::LT_I64 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
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
                    (Value::Char(x), Value::Char(y)) => self.push(Value::Bool(*x < *y)),
                    (Value::String(x), Value::String(y)) => self.push(Value::Bool(**x < **y)),
                    (Value::Char(x), Value::String(y)) => {
                        let xs: String = x.to_string();
                        self.push(Value::Bool(xs < **y));
                    }
                    (Value::String(x), Value::Char(y)) => {
                        let ys: String = y.to_string();
                        self.push(Value::Bool(**x < ys));
                    }
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_i64(), b.to_i64()) {
                            self.push(Value::Bool(x < y));
                        } else {
                            return Err(format!("LT_I64: type mismatch {:?} < {:?}", a, b));
                        }
                    }
                }
            }
            OpCode::LT_F32 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Float(x), Value::Float(y)) => self.push(Value::Bool(x < y)),
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_f64(), b.to_f64()) {
                            self.push(Value::Bool(x < y));
                        } else {
                            return Err(format!("LT_F32: type mismatch {:?}", a));
                        }
                    }
                }
            }
            OpCode::LT_F64 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Bool(x < y)),
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_f64(), b.to_f64()) {
                            self.push(Value::Bool(x < y));
                        } else {
                            return Err(format!("LT_F64: type mismatch {:?}", a));
                        }
                    }
                }
            }
            OpCode::LE_I32 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x <= y)),
                    (Value::Null, _) | (_, Value::Null) => self.push(Value::Bool(false)),
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_i64(), b.to_i64()) {
                            self.push(Value::Bool(x <= y));
                        } else {
                            return Err(format!("LE_I32: type mismatch {:?}", a));
                        }
                    }
                }
            }
            OpCode::LE_I64 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
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
                    (Value::Char(x), Value::Char(y)) => self.push(Value::Bool(*x <= *y)),
                    (Value::String(x), Value::String(y)) => self.push(Value::Bool(**x <= **y)),
                    (Value::Char(x), Value::String(y)) => {
                        let xs: String = x.to_string();
                        self.push(Value::Bool(xs <= **y));
                    }
                    (Value::String(x), Value::Char(y)) => {
                        let ys: String = y.to_string();
                        self.push(Value::Bool(**x <= ys));
                    }
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_i64(), b.to_i64()) {
                            self.push(Value::Bool(x <= y));
                        } else {
                            return Err(format!("LE_I64: type mismatch {:?} <= {:?}", a, b));
                        }
                    }
                }
            }
            OpCode::LE_F32 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Float(x), Value::Float(y)) => self.push(Value::Bool(x <= y)),
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_f64(), b.to_f64()) {
                            self.push(Value::Bool(x <= y));
                        } else {
                            return Err(format!("LE_F32: type mismatch {:?}", a));
                        }
                    }
                }
            }
            OpCode::LE_F64 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Bool(x <= y)),
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_f64(), b.to_f64()) {
                            self.push(Value::Bool(x <= y));
                        } else {
                            return Err(format!("LE_F64: type mismatch {:?}", a));
                        }
                    }
                }
            }
            OpCode::GT_I32 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x > y)),
                    (Value::Null, _) | (_, Value::Null) => self.push(Value::Bool(false)),
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_i64(), b.to_i64()) {
                            self.push(Value::Bool(x > y));
                        } else {
                            return Err(format!("GT_I32: type mismatch {:?}", a));
                        }
                    }
                }
            }
            OpCode::GT_I64 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
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
                    (Value::Char(x), Value::Char(y)) => self.push(Value::Bool(*x > *y)),
                    (Value::String(x), Value::String(y)) => self.push(Value::Bool(**x > **y)),
                    (Value::Char(x), Value::String(y)) => {
                        let xs: String = x.to_string();
                        self.push(Value::Bool(xs > **y));
                    }
                    (Value::String(x), Value::Char(y)) => {
                        let ys: String = y.to_string();
                        self.push(Value::Bool(**x > ys));
                    }
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_i64(), b.to_i64()) {
                            self.push(Value::Bool(x > y));
                        } else {
                            return Err(format!("GT_I64: type mismatch {:?} > {:?}", a, b));
                        }
                    }
                }
            }
            OpCode::GT_F32 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Float(x), Value::Float(y)) => self.push(Value::Bool(x > y)),
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_f64(), b.to_f64()) {
                            self.push(Value::Bool(x > y));
                        } else {
                            return Err(format!("GT_F32: type mismatch {:?}", a));
                        }
                    }
                }
            }
            OpCode::GT_F64 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Bool(x > y)),
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_f64(), b.to_f64()) {
                            self.push(Value::Bool(x > y));
                        } else {
                            return Err(format!("GT_F64: type mismatch {:?}", a));
                        }
                    }
                }
            }
            OpCode::GE_I32 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x >= y)),
                    (Value::Null, _) | (_, Value::Null) => self.push(Value::Bool(false)),
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_i64(), b.to_i64()) {
                            self.push(Value::Bool(x >= y));
                        } else {
                            return Err(format!("GE_I32: type mismatch {:?}", a));
                        }
                    }
                }
            }
            OpCode::GE_I64 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
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
                    (Value::Char(x), Value::Char(y)) => self.push(Value::Bool(*x >= *y)),
                    (Value::String(x), Value::String(y)) => self.push(Value::Bool(**x >= **y)),
                    (Value::Char(x), Value::String(y)) => {
                        let xs: String = x.to_string();
                        self.push(Value::Bool(xs >= **y));
                    }
                    (Value::String(x), Value::Char(y)) => {
                        let ys: String = y.to_string();
                        self.push(Value::Bool(**x >= ys));
                    }
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_i64(), b.to_i64()) {
                            self.push(Value::Bool(x >= y));
                        } else {
                            return Err(format!("GE_I64: type mismatch {:?} >= {:?}", a, b));
                        }
                    }
                }
            }
            OpCode::GE_F32 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Float(x), Value::Float(y)) => self.push(Value::Bool(x >= y)),
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_f64(), b.to_f64()) {
                            self.push(Value::Bool(x >= y));
                        } else {
                            return Err(format!("GE_F32: type mismatch {:?}", a));
                        }
                    }
                }
            }
            OpCode::GE_F64 => {
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::Double(x), Value::Double(y)) => self.push(Value::Bool(x >= y)),
                    _ => {
                        if let (Some(x), Some(y)) = (a.to_f64(), b.to_f64()) {
                            self.push(Value::Bool(x >= y));
                        } else {
                            return Err(format!("GE_F64: type mismatch {:?}", a));
                        }
                    }
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
                let b = self.pop_unwrapped();
                let a = self.pop_unwrapped();
                match (&a, &b) {
                    (Value::String(x), Value::String(y)) => {
                        let mut result = (**x).clone();
                        result.push_str(y);
                        self.push(Value::String(Rc::new(result)));
                    }
                    _ => {
                        let result = format!("{}{}", a.display_string(), b.display_string());
                        self.push(Value::String(Rc::new(result)));
                    }
                }
            }
            OpCode::STR_CONCAT_RIGHT => {
                let b = self.pop();
                let a = self.pop();
                match &a {
                    Value::String(x) => {
                        let result = format!("{}{}", **x, b.display_string());
                        self.push(Value::String(Rc::new(result)));
                    }
                    Value::Char(c) => {
                        let result = format!("{}{}", c, b.display_string());
                        self.push(Value::String(Rc::new(result)));
                    }
                    _ => {
                        let result = format!("{}{}", a.display_string(), b.display_string());
                        self.push(Value::String(Rc::new(result)));
                    }
                }
            }
            OpCode::STR_CONCAT_LEFT => {
                let b = self.pop();
                let a = self.pop();
                match &b {
                    Value::String(y) => {
                        let result = format!("{}{}", a.display_string(), **y);
                        self.push(Value::String(Rc::new(result)));
                    }
                    Value::Char(c) => {
                        let result = format!("{}{}", a.display_string(), c);
                        self.push(Value::String(Rc::new(result)));
                    }
                    _ => {
                        let result = format!("{}{}", a.display_string(), b.display_string());
                        self.push(Value::String(Rc::new(result)));
                    }
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
                let frame = match self.frames.pop() {
                    Some(f) => f,
                    None => return Err("RET: no frame to return from".to_string()),
                };
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
            OpCode::CALL_CLOSURE => {
                let arg_count = self.read_u8();
                self.call_closure_from_stack(arg_count)?;
            }

            // -- Variables -------------------------------------------------------
            OpCode::LOAD_LOCAL => {
                let slot = self.read_u8();
                let base = self.current_frame().base;
                let idx = base + slot as usize;
                if idx < self.stack.len() {
                    let val = self.stack[idx].clone();
                    // Auto-dereference Cell values so that captured locals
                    // read through the shared cell.
                    let val = match &val {
                        Value::Cell(rc) => rc.borrow().clone(),
                        _ => val,
                    };
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
                // If the slot holds a Cell (captured local), write through
                // the shared cell so mutations are visible to closures.
                match &self.stack[idx] {
                    Value::Cell(rc) => {
                        *rc.borrow_mut() = val;
                    }
                    _ => {
                        self.stack[idx] = val;
                    }
                }
            }
            OpCode::LOAD_GLOBAL => {
                let idx = self.read_u16() as usize;
                if idx < self.globals.len() {
                    self.push(self.globals[idx].clone());
                } else {
                    return Err(format!("LOAD_GLOBAL: index {} out of bounds (globals count {})", idx, self.globals.len()));
                }
            }
            OpCode::STORE_GLOBAL => {
                let idx = self.read_u16() as usize;
                let val = self.stack.pop().unwrap_or(Value::Null);
                if idx < self.globals.len() {
                    self.globals[idx] = val;
                } else {
                    return Err(format!("STORE_GLOBAL: index {} out of bounds (globals count {})", idx, self.globals.len()));
                }
            }
            OpCode::LOAD_UPVALUE => {
                let slot = self.read_u8() as usize;
                let upvalues = self.current_frame().upvalues.clone();
                match &upvalues {
                    Some(uvs) => {
                        if slot < uvs.len() {
                            self.push(uvs[slot].borrow().clone());
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
                            *uvs[slot].borrow_mut() = val;
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
                match (array, &index) {
                    (Value::Array { elements }, Value::Int(i)) => {
                        let idx = *i as usize;
                        let mut elements = elements;
                        if idx < elements.len() {
                            elements[idx] = value.clone();
                            self.push(Value::Array { elements });
                        } else {
                            return Err(format!("Array index out of bounds: {}", idx));
                        }
                    }
                    (Value::Array { elements }, Value::Long(i)) => {
                        let idx = *i as usize;
                        let mut elements = elements;
                        if idx < elements.len() {
                            elements[idx] = value.clone();
                            self.push(Value::Array { elements });
                        } else {
                            return Err(format!("Array index out of bounds: {}", idx));
                        }
                    }
                    (array, index) => {
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
                        let frame = match self.frames.pop() {
                            Some(f) => f,
                            None => return Err("UNWRAP_OR_PROPAGATE: no frame to return from".to_string()),
                        };
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

            // -- Instance-of check (class) ----------------------------------------
            OpCode::INSTANCE_OF => {
                let class_name_idx = self.read_u16();
                let class_name = {
                    let frame = self.current_frame();
                    let chunk = &self.functions[frame.function_index as usize].chunk;
                    chunk.strings[class_name_idx as usize].clone()
                };
                let val = self.pop();
                let matches = match &val {
                    Value::ClassInstance { class_name: cn, .. } => {
                        // Match by simple class name. Imported classes are
                        // stored with a mangled module-qualified name (e.g.
                        // "tt.lang.Variant"), so we accept both a prefix match
                        // (covers generic specializations like "ArrayList__int")
                        // and a "<module>.<class>" suffix match.
                        cn.starts_with(&class_name)
                            || cn.ends_with(&format!(".{}", class_name))
                    }
                    Value::EnumInstance { enum_name: en, .. } |
                    Value::EnumVariant { enum_name: en, .. } => {
                        en == &class_name
                            || en.starts_with(&class_name)
                            || en.ends_with(&format!(".{}", class_name))
                    }
                    _ => false,
                };
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
                // If a captured value is a Cell, extract its inner Rc so that
                // the closure shares the same cell as the enclosing scope.
                // Otherwise wrap the value in a fresh cell as before.
                let shared: Vec<Rc<RefCell<Value>>> = upvalues
                    .into_iter()
                    .map(|v| match v {
                        Value::Cell(rc) => rc,
                        other => Rc::new(RefCell::new(other)),
                    })
                    .collect();
                self.push(Value::Closure {
                    func_idx,
                    upvalues: shared,
                });
            }
            OpCode::GET_UPVALUE => {
                let slot = self.read_u8() as usize;
                let upvalues = self.current_frame().upvalues.clone();
                match &upvalues {
                    Some(uvs) => {
                        if slot < uvs.len() {
                            self.push(uvs[slot].borrow().clone());
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
                            *uvs[slot].borrow_mut() = val;
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
                let shared: Vec<Rc<RefCell<Value>>> = upvalues
                    .into_iter()
                    .map(|v| Rc::new(RefCell::new(v)))
                    .collect();
                self.push(Value::Closure {
                    func_idx,
                    upvalues: shared,
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
            OpCode::CAPTURE_LOCAL => {
                // Box a local variable into a shared Cell so that closures
                // capturing it see mutations and vice versa. If the local is
                // already a Cell (captured by an earlier closure), reuse the
                // existing cell. Otherwise, wrap the current value in a new
                // Cell and replace the stack slot.
                let slot = self.read_u8() as usize;
                let base = self.current_frame().base;
                let idx = base + slot;
                if idx < self.stack.len() {
                    let cell = match &self.stack[idx] {
                        Value::Cell(rc) => rc.clone(),
                        other => {
                            let rc = Rc::new(RefCell::new(other.clone()));
                            self.stack[idx] = Value::Cell(rc.clone());
                            rc
                        }
                    };
                    self.push(Value::Cell(cell));
                } else {
                    return Err(format!(
                        "CAPTURE_LOCAL: local slot {} out of bounds (stack len {})",
                        slot,
                        self.stack.len()
                    ));
                }
            }

            // -- Exception handling ----------------------------------------------
            OpCode::THROW => {
                let val = self.pop();
                // Look for the nearest exception handler.
                if let Some(handler) = self.exception_handlers.last().cloned() {
                    // Unwind frames until we reach the handler's frame depth.
                    while self.frames.len() > handler.frame_depth {
                        self.frames.pop();
                    }
                    if self.frames.is_empty() {
                        // No frame to return to — treat as uncaught.
                        let msg = match val {
                            Value::String(s) => (*s).clone(),
                            ref v => format!("{:?}", v),
                        };
                        return Err(msg);
                    }
                    // Restore the stack to the handler's recorded depth.
                    while self.stack.len() > handler.stack_depth {
                        self.stack.pop();
                    }
                    // Push the thrown value for the catch block to consume.
                    self.push(val);
                    // Jump to the catch block entry point.
                    self.current_frame_mut().ip = handler.catch_ip;
                    // Pop the handler now that it has been consumed.
                    self.exception_handlers.pop();
                } else {
                    // No active handler — convert to a VM error so the
                    // outermost run loop terminates with a message.
                    let msg = match val {
                        Value::String(s) => (*s).clone(),
                        ref v => format!("{:?}", v),
                    };
                    return Err(msg);
                }
            }
            OpCode::PUSH_HANDLER => {
                let catch_ip = self.read_u16() as usize;
                let function_index = self.current_frame().function_index;
                let stack_depth = self.stack.len();
                let frame_depth = self.frames.len();
                self.exception_handlers.push(ExceptionHandler {
                    function_index,
                    catch_ip,
                    stack_depth,
                    frame_depth,
                });
            }
            OpCode::POP_HANDLER => {
                self.exception_handlers.pop();
            }
        }

        Ok(())
    }
}
