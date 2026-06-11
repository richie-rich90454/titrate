// Titrate Alpha 0.2 – bytecode virtual machine: built-in operators
// Precision in every step – richie-rich90454, 2026

use super::super::value::Value;
use super::Vm;
use std::rc::Rc;

impl Vm {
    pub(super) fn builtin_add(&self, left: &Value, right: &Value) -> Result<Value, String> {
        match (left, right) {
            (Value::Byte(a), Value::Byte(b)) => Ok(Value::Byte(a.wrapping_add(*b))),
            (Value::Short(a), Value::Short(b)) => Ok(Value::Short(a.wrapping_add(*b))),
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a.wrapping_add(*b))),
            (Value::Long(a), Value::Long(b)) => Ok(Value::Long(a.wrapping_add(*b))),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
            (Value::Double(a), Value::Double(b)) => Ok(Value::Double(a + b)),
            (Value::Half(a), Value::Half(b)) => Ok(Value::Half(a + b)),
            (Value::Quad(a), Value::Quad(b)) => Ok(Value::Quad(a + b)),
            (Value::Int(a), Value::Long(b)) => Ok(Value::Long(*a as i64 + b)),
            (Value::Long(a), Value::Int(b)) => Ok(Value::Long(a + *b as i64)),
            (Value::String(a), Value::String(b)) => Ok(Value::String(Rc::new(format!("{}{}", a, b)))),
            _ => Err(format!("Cannot add {:?} and {:?}", left, right)),
        }
    }

    pub(super) fn builtin_sub(&self, left: &Value, right: &Value) -> Result<Value, String> {
        match (left, right) {
            (Value::Byte(a), Value::Byte(b)) => Ok(Value::Byte(a.wrapping_sub(*b))),
            (Value::Short(a), Value::Short(b)) => Ok(Value::Short(a.wrapping_sub(*b))),
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a.wrapping_sub(*b))),
            (Value::Long(a), Value::Long(b)) => Ok(Value::Long(a.wrapping_sub(*b))),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
            (Value::Double(a), Value::Double(b)) => Ok(Value::Double(a - b)),
            (Value::Half(a), Value::Half(b)) => Ok(Value::Half(a - b)),
            (Value::Quad(a), Value::Quad(b)) => Ok(Value::Quad(a - b)),
            (Value::Int(a), Value::Long(b)) => Ok(Value::Long(*a as i64 - b)),
            (Value::Long(a), Value::Int(b)) => Ok(Value::Long(a - *b as i64)),
            _ => Err(format!("Cannot subtract {:?} and {:?}", left, right)),
        }
    }

    pub(super) fn builtin_mul(&self, left: &Value, right: &Value) -> Result<Value, String> {
        match (left, right) {
            (Value::Byte(a), Value::Byte(b)) => Ok(Value::Byte(a.wrapping_mul(*b))),
            (Value::Short(a), Value::Short(b)) => Ok(Value::Short(a.wrapping_mul(*b))),
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a.wrapping_mul(*b))),
            (Value::Long(a), Value::Long(b)) => Ok(Value::Long(a.wrapping_mul(*b))),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
            (Value::Double(a), Value::Double(b)) => Ok(Value::Double(a * b)),
            (Value::Half(a), Value::Half(b)) => Ok(Value::Half(a * b)),
            (Value::Quad(a), Value::Quad(b)) => Ok(Value::Quad(a * b)),
            (Value::Int(a), Value::Long(b)) => Ok(Value::Long(*a as i64 * b)),
            (Value::Long(a), Value::Int(b)) => Ok(Value::Long(a * *b as i64)),
            _ => Err(format!("Cannot multiply {:?} and {:?}", left, right)),
        }
    }

    pub(super) fn builtin_div(&self, left: &Value, right: &Value) -> Result<Value, String> {
        match (left, right) {
            (Value::Byte(a), Value::Byte(b)) => {
                if *b == 0 { return Err("Division by zero".to_string()); }
                Ok(Value::Byte(a / b))
            }
            (Value::Short(a), Value::Short(b)) => {
                if *b == 0 { return Err("Division by zero".to_string()); }
                Ok(Value::Short(a / b))
            }
            (Value::Int(a), Value::Int(b)) => {
                if *b == 0 { return Err("Division by zero".to_string()); }
                Ok(Value::Int(a / b))
            }
            (Value::Long(a), Value::Long(b)) => {
                if *b == 0 { return Err("Division by zero".to_string()); }
                Ok(Value::Long(a / b))
            }
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a / b)),
            (Value::Double(a), Value::Double(b)) => Ok(Value::Double(a / b)),
            (Value::Half(a), Value::Half(b)) => Ok(Value::Half(a / b)),
            (Value::Quad(a), Value::Quad(b)) => Ok(Value::Quad(a / b)),
            _ => Err(format!("Cannot divide {:?} by {:?}", left, right)),
        }
    }

    pub(super) fn builtin_mod(&self, left: &Value, right: &Value) -> Result<Value, String> {
        match (left, right) {
            (Value::Byte(a), Value::Byte(b)) => {
                if *b == 0 { return Err("Division by zero".to_string()); }
                Ok(Value::Byte(a % b))
            }
            (Value::Short(a), Value::Short(b)) => {
                if *b == 0 { return Err("Division by zero".to_string()); }
                Ok(Value::Short(a % b))
            }
            (Value::Int(a), Value::Int(b)) => {
                if *b == 0 { return Err("Division by zero".to_string()); }
                Ok(Value::Int(a % b))
            }
            (Value::Long(a), Value::Long(b)) => {
                if *b == 0 { return Err("Division by zero".to_string()); }
                Ok(Value::Long(a % b))
            }
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a % b)),
            (Value::Double(a), Value::Double(b)) => Ok(Value::Double(a % b)),
            (Value::Half(a), Value::Half(b)) => Ok(Value::Half(a % b)),
            (Value::Quad(a), Value::Quad(b)) => Ok(Value::Quad(a % b)),
            _ => Err(format!("Cannot modulo {:?} and {:?}", left, right)),
        }
    }

    pub(super) fn builtin_cmp(
        &self,
        left: &Value,
        right: &Value,
        int_op: fn(i64, i64) -> bool,
        float_op: fn(f64, f64) -> bool,
    ) -> Result<Value, String> {
        match (left, right) {
            (Value::Byte(a), Value::Byte(b)) => Ok(Value::Bool(int_op(*a as i64, *b as i64))),
            (Value::Short(a), Value::Short(b)) => Ok(Value::Bool(int_op(*a as i64, *b as i64))),
            (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(int_op(*a as i64, *b as i64))),
            (Value::Long(a), Value::Long(b)) => Ok(Value::Bool(int_op(*a, *b))),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(float_op(*a as f64, *b as f64))),
            (Value::Double(a), Value::Double(b)) => Ok(Value::Bool(float_op(*a, *b))),
            (Value::Half(a), Value::Half(b)) => Ok(Value::Bool(float_op(*a as f64, *b as f64))),
            (Value::Quad(a), Value::Quad(b)) => Ok(Value::Bool(float_op(*a, *b))),
            _ => Err(format!("Cannot compare {:?} and {:?}", left, right)),
        }
    }

    pub(super) fn builtin_bitwise(
        &self,
        left: &Value,
        right: &Value,
        int_op: fn(i64, i64) -> i64,
        _long_op: fn(i64, i64) -> i64,
    ) -> Result<Value, String> {
        match (left, right) {
            (Value::Byte(a), Value::Byte(b)) => Ok(Value::Byte(int_op(*a as i64, *b as i64) as i8)),
            (Value::Short(a), Value::Short(b)) => Ok(Value::Short(int_op(*a as i64, *b as i64) as i16)),
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(int_op(*a as i64, *b as i64) as i32)),
            (Value::Long(a), Value::Long(b)) => Ok(Value::Long(int_op(*a, *b))),
            _ => Err(format!("Cannot apply bitwise op to {:?} and {:?}", left, right)),
        }
    }

    pub(super) fn builtin_shift(&self, left: &Value, right: &Value, is_right: bool) -> Result<Value, String> {
        match (left, right) {
            (Value::Byte(a), Value::Int(b)) => {
                if is_right { Ok(Value::Byte((*a as i64).wrapping_shr(*b as u32) as i8)) }
                else { Ok(Value::Byte((*a as i64).wrapping_shl(*b as u32) as i8)) }
            }
            (Value::Short(a), Value::Int(b)) => {
                if is_right { Ok(Value::Short((*a as i64).wrapping_shr(*b as u32) as i16)) }
                else { Ok(Value::Short((*a as i64).wrapping_shl(*b as u32) as i16)) }
            }
            (Value::Int(a), Value::Int(b)) => {
                if is_right { Ok(Value::Int(a.wrapping_shr(*b as u32))) }
                else { Ok(Value::Int(a.wrapping_shl(*b as u32))) }
            }
            (Value::Long(a), Value::Int(b)) => {
                if is_right { Ok(Value::Long(a.wrapping_shr(*b as u32))) }
                else { Ok(Value::Long(a.wrapping_shl(*b as u32))) }
            }
            _ => Err(format!("Cannot shift {:?} by {:?}", left, right)),
        }
    }
}
