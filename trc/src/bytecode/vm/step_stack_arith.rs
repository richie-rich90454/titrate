// Stack and arithmetic opcodes

use super::super::opcodes::OpCode;
use super::super::value::Value;
use std::char;
use std::rc::Rc;

use super::Vm;

impl Vm {
    pub(super) fn step_stack_arith(&mut self, op: OpCode) -> Result<(), String> {
        match op {
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
            _ => return Err(format!("step_stack_arith: unexpected opcode {:?}", op)),         }          Ok(())     } }
