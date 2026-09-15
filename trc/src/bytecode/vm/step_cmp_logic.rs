// Comparison and logic opcodes

use super::super::opcodes::OpCode;
use super::super::value::{Value, values_eq};
use std::rc::Rc;

use super::Vm;

impl Vm {
    pub(super) fn step_cmp_logic(&mut self, op: OpCode) -> Result<(), String> {
        match op {
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
            _ => return Err(format!("step_cmp_logic: unexpected opcode {:?}", op)),         }          Ok(())     } }
