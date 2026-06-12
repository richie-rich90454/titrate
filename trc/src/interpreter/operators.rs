// Phase 4: Operator helper functions for the Titrate interpreter
// Precision in every step – richie-rich90454, 2026

use super::Value;

impl Interpreter {
    pub(super) fn arith_binop(
        &self,
        int_op: fn(i64, i64) -> i64,
        float_op: fn(f64, f64) -> f64,
        left: &Value,
        right: &Value,
    ) -> Result<Value, String> {
        match (left, right) {
            (Value::Byte(a), Value::Byte(b)) => Ok(Value::Byte(int_op(*a as i64, *b as i64) as i8)),
            (Value::Short(a), Value::Short(b)) => Ok(Value::Short(int_op(*a as i64, *b as i64) as i16)),
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(int_op(*a as i64, *b as i64) as i32)),
            (Value::Long(a), Value::Long(b)) => Ok(Value::Long(int_op(*a, *b))),
            (Value::Vast(a), Value::Vast(b)) => {
                let result = int_op(*a as i64, *b as i64);
                Ok(Value::Vast(result as i128))
            }
            (Value::Uvast(a), Value::Uvast(b)) => {
                let result = int_op(*a as i64, *b as i64);
                Ok(Value::Uvast(result as u128))
            }
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(float_op(*a as f64, *b as f64) as f32)),
            (Value::Double(a), Value::Double(b)) => Ok(Value::Double(float_op(*a, *b))),
            (Value::Half(a), Value::Half(b)) => Ok(Value::Half(float_op(*a as f64, *b as f64) as f32)),
            (Value::Quad(a), Value::Quad(b)) => Ok(Value::Quad(float_op(*a, *b))),
            (Value::Int(a), Value::Long(b)) => Ok(Value::Long(int_op(*a as i64, *b))),
            (Value::Long(a), Value::Int(b)) => Ok(Value::Long(int_op(*a, *b as i64))),
            (Value::Byte(a), Value::Int(b)) => Ok(Value::Int(int_op(*a as i64, *b as i64) as i32)),
            (Value::Int(a), Value::Byte(b)) => Ok(Value::Int(int_op(*a as i64, *b as i64) as i32)),
            (Value::Int(a), Value::Double(b)) => Ok(Value::Double(float_op(*a as f64, *b))),
            (Value::Long(a), Value::Double(b)) => Ok(Value::Double(float_op(*a as f64, *b))),
            (Value::Double(a), Value::Int(b)) => Ok(Value::Double(float_op(*a, *b as f64))),
            (Value::Double(a), Value::Long(b)) => Ok(Value::Double(float_op(*a, *b as f64))),
            (Value::Float(a), Value::Double(b)) => Ok(Value::Double(float_op(*a as f64, *b))),
            (Value::Double(a), Value::Float(b)) => Ok(Value::Double(float_op(*a, *b as f64))),
            _ => Err(format!("Cannot apply arithmetic to {:?} and {:?}", left, right)),
        }
    }

    pub(super) fn div_binop(&self, left: &Value, right: &Value) -> Result<Value, String> {
        match (left, right) {
            (Value::Byte(a), Value::Byte(b)) => {
                if *b == 0 { return Err("Division by zero".to_string()); }
                Ok(Value::Byte(*a / *b))
            }
            (Value::Short(a), Value::Short(b)) => {
                if *b == 0 { return Err("Division by zero".to_string()); }
                Ok(Value::Short(*a / *b))
            }
            (Value::Int(a), Value::Int(b)) => {
                if *b == 0 { return Err("Division by zero".to_string()); }
                Ok(Value::Int(*a / *b))
            }
            (Value::Long(a), Value::Long(b)) => {
                if *b == 0 { return Err("Division by zero".to_string()); }
                Ok(Value::Long(*a / *b))
            }
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(*a / *b)),
            (Value::Double(a), Value::Double(b)) => Ok(Value::Double(*a / *b)),
            (Value::Half(a), Value::Half(b)) => Ok(Value::Half(*a / *b)),
            (Value::Quad(a), Value::Quad(b)) => Ok(Value::Quad(*a / *b)),
            (Value::Int(a), Value::Long(b)) => {
                if *b == 0 { return Err("Division by zero".to_string()); }
                Ok(Value::Long(*a as i64 / *b))
            }
            (Value::Long(a), Value::Int(b)) => {
                if *b == 0 { return Err("Division by zero".to_string()); }
                Ok(Value::Long(*a / *b as i64))
            }
            _ => Err(format!("Cannot divide {:?} by {:?}", left, right)),
        }
    }

    pub(super) fn mod_binop(&self, left: &Value, right: &Value) -> Result<Value, String> {
        match (left, right) {
            (Value::Byte(a), Value::Byte(b)) => {
                if *b == 0 { return Err("Division by zero".to_string()); }
                Ok(Value::Byte(*a % *b))
            }
            (Value::Short(a), Value::Short(b)) => {
                if *b == 0 { return Err("Division by zero".to_string()); }
                Ok(Value::Short(*a % *b))
            }
            (Value::Int(a), Value::Int(b)) => {
                if *b == 0 { return Err("Division by zero".to_string()); }
                Ok(Value::Int(*a % *b))
            }
            (Value::Long(a), Value::Long(b)) => {
                if *b == 0 { return Err("Division by zero".to_string()); }
                Ok(Value::Long(*a % *b))
            }
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(*a % *b)),
            (Value::Double(a), Value::Double(b)) => Ok(Value::Double(*a % *b)),
            _ => Err(format!("Cannot mod {:?} by {:?}", left, right)),
        }
    }

    pub(super) fn cmp_binop(
        &self,
        int_op: fn(i64, i64) -> bool,
        float_op: fn(f64, f64) -> bool,
        left: &Value,
        right: &Value,
    ) -> Result<Value, String> {
        match (left, right) {
            (Value::Byte(a), Value::Byte(b)) => Ok(Value::Bool(int_op(*a as i64, *b as i64))),
            (Value::Short(a), Value::Short(b)) => Ok(Value::Bool(int_op(*a as i64, *b as i64))),
            (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(int_op(*a as i64, *b as i64))),
            (Value::Long(a), Value::Long(b)) => Ok(Value::Bool(int_op(*a, *b))),
            (Value::Vast(a), Value::Vast(b)) => Ok(Value::Bool(int_op(*a as i64, *b as i64))),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(float_op(*a as f64, *b as f64))),
            (Value::Double(a), Value::Double(b)) => Ok(Value::Bool(float_op(*a, *b))),
            (Value::Half(a), Value::Half(b)) => Ok(Value::Bool(float_op(*a as f64, *b as f64))),
            (Value::Quad(a), Value::Quad(b)) => Ok(Value::Bool(float_op(*a, *b))),
            (Value::Int(a), Value::Long(b)) => Ok(Value::Bool(int_op(*a as i64, *b))),
            (Value::Long(a), Value::Int(b)) => Ok(Value::Bool(int_op(*a, *b as i64))),
            (Value::Byte(a), Value::Int(b)) => Ok(Value::Bool(int_op(*a as i64, *b as i64))),
            (Value::Int(a), Value::Byte(b)) => Ok(Value::Bool(int_op(*a as i64, *b as i64))),
            (Value::Int(a), Value::Double(b)) => Ok(Value::Bool(float_op(*a as f64, *b))),
            (Value::Long(a), Value::Double(b)) => Ok(Value::Bool(float_op(*a as f64, *b))),
            (Value::Double(a), Value::Int(b)) => Ok(Value::Bool(float_op(*a, *b as f64))),
            (Value::Double(a), Value::Long(b)) => Ok(Value::Bool(float_op(*a, *b as f64))),
            _ => Err(format!("Cannot compare {:?} and {:?}", left, right)),
        }
    }

    pub(super) fn bit_binop(
        &self,
        int_op: fn(i64, i64) -> i64,
        uint_op: fn(u128, u128) -> u128,
        left: &Value,
        right: &Value,
    ) -> Result<Value, String> {
        match (left, right) {
            (Value::Byte(a), Value::Byte(b)) => Ok(Value::Byte(int_op(*a as i64, *b as i64) as i8)),
            (Value::Short(a), Value::Short(b)) => Ok(Value::Short(int_op(*a as i64, *b as i64) as i16)),
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(int_op(*a as i64, *b as i64) as i32)),
            (Value::Long(a), Value::Long(b)) => Ok(Value::Long(int_op(*a, *b))),
            (Value::Vast(a), Value::Vast(b)) => Ok(Value::Vast(int_op(*a as i64, *b as i64) as i128)),
            (Value::Uvast(a), Value::Uvast(b)) => Ok(Value::Uvast(uint_op(*a, *b))),
            (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(int_op(*a as i64, *b as i64) != 0)),
            _ => Err(format!("Cannot apply bitwise operation to {:?} and {:?}", left, right)),
        }
    }

    pub(super) fn shift_binop(&self, left: &Value, right: &Value, is_right: bool) -> Result<Value, String> {
        let lv = left.to_i64().ok_or_else(|| format!("Cannot shift {:?}", left))?;
        let rv = right.to_i64().ok_or_else(|| format!("Shift amount must be integer, got {:?}", right))?;
        if rv < 0 {
            return Err("Negative shift amount".to_string());
        }
        let result = if is_right {
            lv.wrapping_shr(rv as u32)
        } else {
            lv.wrapping_shl(rv as u32)
        };
        match left {
            Value::Byte(_) => Ok(Value::Byte(result as i8)),
            Value::Short(_) => Ok(Value::Short(result as i16)),
            Value::Int(_) => Ok(Value::Int(result as i32)),
            Value::Long(_) => Ok(Value::Long(result)),
            Value::Vast(_) => Ok(Value::Vast(result as i128)),
            Value::Uvast(_) => Ok(Value::Uvast(result as u128)),
            _ => Err(format!("Cannot shift {:?}", left)),
        }
    }
}

use super::Interpreter;
