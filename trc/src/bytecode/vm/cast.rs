// Titrate Alpha 0.2 – bytecode virtual machine: cast operations
// Precision in every step – richie-rich90454, 2026

use super::super::opcodes::{CastTarget, TypeTag};
use super::super::value::Value;
use super::Vm;
use std::rc::Rc;

impl Vm {
    pub(super) fn check_type_tag(&self, val: &Value, tag: TypeTag) -> bool {
        match tag {
            TypeTag::I8 => matches!(val, Value::Byte(_)),
            TypeTag::I16 => matches!(val, Value::Short(_)),
            TypeTag::I32 => matches!(val, Value::Int(_)),
            TypeTag::I64 => matches!(val, Value::Long(_)),
            TypeTag::I128 => matches!(val, Value::Vast(_)),
            TypeTag::U128 => matches!(val, Value::Uvast(_)),
            TypeTag::F32 => matches!(val, Value::Float(_)),
            TypeTag::F64 => matches!(val, Value::Double(_)),
            TypeTag::Bool => matches!(val, Value::Bool(_)),
            TypeTag::Char => matches!(val, Value::Char(_)),
            TypeTag::String => matches!(val, Value::String(_)),
            TypeTag::Null => matches!(val, Value::Null),
            TypeTag::Void => matches!(val, Value::Void),
            TypeTag::Class => matches!(val, Value::ClassInstance { .. }),
            TypeTag::Enum => matches!(val, Value::EnumInstance { .. }),
            TypeTag::Array => matches!(val, Value::Array { .. }),
            TypeTag::Ref => matches!(val, Value::Ref(_)),
            TypeTag::Owned => matches!(val, Value::Owned(_)),
            TypeTag::Result => {
                matches!(val, Value::ResultOk(_) | Value::ResultErr(_))
            }
            TypeTag::Function => matches!(val, Value::Function(_)),
        }
    }

    // -----------------------------------------------------------------------
    // Cast – mirrors the old tree-walking interpreter exactly
    // -----------------------------------------------------------------------

    pub(super) fn eval_cast(&self, val: &Value, target: CastTarget) -> Result<Value, String> {
        // Casting null to any type preserves null. This is required for
        // generic code like `null as T` where T is a primitive type —
        // the actual type is unknown at compile time and null is the
        // sentinel for "no value" in generic contexts (e.g. popFront on
        // an empty ForwardList<int>).
        if matches!(val, Value::Null) {
            return Ok(Value::Null);
        }
        match target {
            CastTarget::Byte => {
                let v = val.to_i64().ok_or_else(|| format!("Cannot cast {:?} to byte", val))?;
                Ok(Value::Byte(v as i8))
            }
            CastTarget::Short => {
                let v = val.to_i64().ok_or_else(|| format!("Cannot cast {:?} to short", val))?;
                Ok(Value::Short(v as i16))
            }
            CastTarget::Int => {
                let v = val.to_i64().ok_or_else(|| format!("Cannot cast {:?} to int", val))?;
                Ok(Value::Int(v as i32))
            }
            CastTarget::Long => {
                let v = val.to_i64().ok_or_else(|| format!("Cannot cast {:?} to long", val))?;
                Ok(Value::Long(v))
            }
            CastTarget::Vast => {
                let v = val.to_i64().ok_or_else(|| format!("Cannot cast {:?} to vast", val))?;
                Ok(Value::Vast(v as i128))
            }
            CastTarget::Uvast => {
                let v = val.to_u128().ok_or_else(|| format!("Cannot cast {:?} to uvast", val))?;
                Ok(Value::Uvast(v))
            }
            CastTarget::Float => {
                let v = val.to_f64().ok_or_else(|| format!("Cannot cast {:?} to float", val))?;
                Ok(Value::Float(v as f32))
            }
            CastTarget::Double => {
                let v = val.to_f64().ok_or_else(|| format!("Cannot cast {:?} to double", val))?;
                Ok(Value::Double(v))
            }
            CastTarget::Half => {
                let v = val.to_f64().ok_or_else(|| format!("Cannot cast {:?} to half", val))?;
                Ok(Value::Half(v as f32))
            }
            CastTarget::Quad => {
                let v = val.to_f64().ok_or_else(|| format!("Cannot cast {:?} to quad", val))?;
                Ok(Value::Quad(v))
            }
            CastTarget::Char => {
                let v = val.to_i64().ok_or_else(|| format!("Cannot cast {:?} to char", val))?;
                Ok(Value::Char(v as u8 as char))
            }
            CastTarget::String => {
                Ok(Value::String(Rc::new(val.display_string())))
            }
            CastTarget::Bool => {
                Ok(Value::Bool(val.is_truthy()))
            }
        }
    }
}

impl TryFrom<u8> for CastTarget {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(CastTarget::Byte),
            1 => Ok(CastTarget::Short),
            2 => Ok(CastTarget::Int),
            3 => Ok(CastTarget::Long),
            4 => Ok(CastTarget::Vast),
            5 => Ok(CastTarget::Uvast),
            6 => Ok(CastTarget::Float),
            7 => Ok(CastTarget::Double),
            8 => Ok(CastTarget::Half),
            9 => Ok(CastTarget::Quad),
            10 => Ok(CastTarget::Char),
            11 => Ok(CastTarget::String),
            12 => Ok(CastTarget::Bool),
            _ => Err(value),
        }
    }
}

// ---------------------------------------------------------------------------
// TypeTag conversion from u8

impl TryFrom<u8> for TypeTag {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(TypeTag::I8),
            1 => Ok(TypeTag::I16),
            2 => Ok(TypeTag::I32),
            3 => Ok(TypeTag::I64),
            4 => Ok(TypeTag::I128),
            5 => Ok(TypeTag::U128),
            6 => Ok(TypeTag::F32),
            7 => Ok(TypeTag::F64),
            8 => Ok(TypeTag::Bool),
            9 => Ok(TypeTag::Char),
            10 => Ok(TypeTag::String),
            11 => Ok(TypeTag::Null),
            12 => Ok(TypeTag::Void),
            13 => Ok(TypeTag::Class),
            14 => Ok(TypeTag::Enum),
            15 => Ok(TypeTag::Array),
            16 => Ok(TypeTag::Ref),
            17 => Ok(TypeTag::Owned),
            18 => Ok(TypeTag::Result),
            19 => Ok(TypeTag::Function),
            _ => Err(value),
        }
    }
}

