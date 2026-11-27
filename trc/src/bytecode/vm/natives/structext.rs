// StructExt natives backing lib/tt/binary/StructExt.tr.
// Binary data crosses the VM boundary as Latin-1 strings: one char per byte.

use super::super::super::value::Value;
use super::super::object::{parse_struct_format, value_to_f64, value_to_i64};
use std::rc::Rc;

const PACK: &str = "StructExt_pack";
const PACK_INTO: &str = "StructExt_packInto";
const UNPACK: &str = "StructExt_unpack";
const UNPACK_FROM: &str = "StructExt_unpackFrom";

fn byte_string(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| b as char).collect()
}

fn string_bytes(value: &Value, op: &str) -> Result<Vec<u8>, String> {
    match value {
        Value::String(s) => s
            .chars()
            .map(|c| {
                if (c as u32) <= 0xFF {
                    Ok(c as u8)
                } else {
                    Err(format!(
                        "{}: byte string contains non-Latin-1 character U+{:04X}",
                        op, c as u32
                    ))
                }
            })
            .collect(),
        _ => Err(format!("{}: expected a String", op)),
    }
}

fn values_of(value: &Value, op: &str) -> Result<Vec<Value>, String> {
    match value {
        Value::Array { elements } => Ok(elements.clone()),
        Value::ClassInstance { fields, .. } => match fields.borrow().get("_elements") {
            Some(Value::Array { elements }) => Ok(elements.clone()),
            _ => Err(format!("{}: values must be an ArrayList", op)),
        },
        _ => Err(format!("{}: values must be an ArrayList", op)),
    }
}

fn int_value(value: &Value, fc: char) -> Result<i64, String> {
    value_to_i64(value)
        .ok_or_else(|| format!("StructExt: field '{}' needs a numeric value", fc))
}

fn float_value(value: &Value, fc: char) -> Result<f64, String> {
    value_to_f64(value)
        .ok_or_else(|| format!("StructExt: field '{}' needs a numeric value", fc))
}

pub(crate) fn native_structext_pack(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err(format!("{}: expected (format, values)", PACK));
    }
    let format = match &args[0] {
        Value::String(s) => s.as_str(),
        _ => return Err(format!("{}: format must be a String", PACK)),
    };
    let values = values_of(&args[1], PACK)?;
    let (little, fields) = parse_struct_format(format);
    let mut bytes = Vec::new();
    let mut vi = 0usize;
    for (fc, count) in fields {
        for _ in 0..count {
            let value = values
                .get(vi)
                .ok_or_else(|| format!("{}: not enough values for format '{}'", PACK, format))?;
            vi += 1;
            match fc {
                'b' | 'B' | '?' => bytes.push(int_value(value, fc)? as u8),
                'h' | 'H' => {
                    let n = int_value(value, fc)? as i16;
                    if little {
                        bytes.extend_from_slice(&n.to_le_bytes());
                    } else {
                        bytes.extend_from_slice(&n.to_be_bytes());
                    }
                }
                'i' | 'I' | 'l' | 'L' => {
                    let n = int_value(value, fc)? as i32;
                    if little {
                        bytes.extend_from_slice(&n.to_le_bytes());
                    } else {
                        bytes.extend_from_slice(&n.to_be_bytes());
                    }
                }
                'q' | 'Q' => {
                    let n = int_value(value, fc)?;
                    if little {
                        bytes.extend_from_slice(&n.to_le_bytes());
                    } else {
                        bytes.extend_from_slice(&n.to_be_bytes());
                    }
                }
                'f' => {
                    let n = float_value(value, fc)? as f32;
                    if little {
                        bytes.extend_from_slice(&n.to_le_bytes());
                    } else {
                        bytes.extend_from_slice(&n.to_be_bytes());
                    }
                }
                'd' => {
                    let n = float_value(value, fc)?;
                    if little {
                        bytes.extend_from_slice(&n.to_le_bytes());
                    } else {
                        bytes.extend_from_slice(&n.to_be_bytes());
                    }
                }
                's' | 'p' => bytes.push(match value {
                    Value::String(s) => s.chars().next().map(|c| c as u32 as u8).unwrap_or(0),
                    Value::Char(c) => *c as u32 as u8,
                    _ => int_value(value, fc)? as u8,
                }),
                _ => return Err(format!("{}: unsupported format character '{}'", PACK, fc)),
            }
        }
    }
    Ok(Value::String(Rc::new(byte_string(&bytes))))
}

fn take<'a>(data: &'a [u8], pos: &mut usize, len: usize, fc: char) -> Result<&'a [u8], String> {
    let end = pos
        .checked_add(len)
        .ok_or_else(|| format!("StructExt: offset overflow for field '{}'", fc))?;
    let slice = data
        .get(*pos..end)
        .ok_or_else(|| format!("StructExt: truncated data for field '{}'", fc))?;
    *pos = end;
    Ok(slice)
}

fn unpack_at(format: &str, data: &[u8], offset: usize) -> Result<Vec<Value>, String> {
    let (little, fields) = parse_struct_format(format);
    let mut result = Vec::new();
    let mut pos = offset;
    for (fc, count) in fields {
        for _ in 0..count {
            match fc {
                'b' | 'B' | '?' => {
                    let b = take(data, &mut pos, 1, fc)?[0];
                    result.push(Value::Int((b as i8) as i32));
                }
                'h' | 'H' => {
                    let b = take(data, &mut pos, 2, fc)?;
                    let n = if little {
                        i16::from_le_bytes([b[0], b[1]])
                    } else {
                        i16::from_be_bytes([b[0], b[1]])
                    };
                    result.push(Value::Int(n as i32));
                }
                'i' | 'I' | 'l' | 'L' => {
                    let b = take(data, &mut pos, 4, fc)?;
                    let n = if little {
                        i32::from_le_bytes([b[0], b[1], b[2], b[3]])
                    } else {
                        i32::from_be_bytes([b[0], b[1], b[2], b[3]])
                    };
                    result.push(Value::Int(n));
                }
                'q' | 'Q' => {
                    let b = take(data, &mut pos, 8, fc)?;
                    let mut arr = [0u8; 8];
                    arr.copy_from_slice(b);
                    let n = if little {
                        i64::from_le_bytes(arr)
                    } else {
                        i64::from_be_bytes(arr)
                    };
                    result.push(Value::Long(n));
                }
                'f' => {
                    let b = take(data, &mut pos, 4, fc)?;
                    let n = if little {
                        f32::from_le_bytes([b[0], b[1], b[2], b[3]])
                    } else {
                        f32::from_be_bytes([b[0], b[1], b[2], b[3]])
                    };
                    result.push(Value::Double(n as f64));
                }
                'd' => {
                    let b = take(data, &mut pos, 8, fc)?;
                    let mut arr = [0u8; 8];
                    arr.copy_from_slice(b);
                    let n = if little {
                        f64::from_le_bytes(arr)
                    } else {
                        f64::from_be_bytes(arr)
                    };
                    result.push(Value::Double(n));
                }
                's' | 'p' => {
                    let b = take(data, &mut pos, 1, fc)?[0];
                    result.push(Value::String(Rc::new((b as char).to_string())));
                }
                _ => return Err(format!("StructExt: unsupported format character '{}'", fc)),
            }
        }
    }
    Ok(result)
}

fn array(value: Vec<Value>) -> Value {
    Value::Array { elements: value }
}

pub(crate) fn native_structext_unpack(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err(format!("{}: expected (format, data)", UNPACK));
    }
    let format = match &args[0] {
        Value::String(s) => s.as_str(),
        _ => return Err(format!("{}: format must be a String", UNPACK)),
    };
    let data = string_bytes(&args[1], UNPACK)?;
    Ok(array(unpack_at(format, &data, 0)?))
}

pub(crate) fn native_structext_unpack_from(args: &[Value]) -> Result<Value, String> {
    if args.len() < 3 {
        return Err(format!("{}: expected (format, buffer, offset)", UNPACK_FROM));
    }
    let format = match &args[0] {
        Value::String(s) => s.as_str(),
        _ => return Err(format!("{}: format must be a String", UNPACK_FROM)),
    };
    let data = string_bytes(&args[1], UNPACK_FROM)?;
    let offset = match &args[2] {
        Value::Int(i) => *i as i64,
        Value::Long(i) => *i,
        _ => return Err(format!("{}: offset must be an int", UNPACK_FROM)),
    };
    let start = usize::try_from(offset)
        .map_err(|_| format!("{}: offset must not be negative", UNPACK_FROM))?;
    Ok(array(unpack_at(format, &data, start)?))
}

pub(crate) fn native_structext_pack_into(args: &[Value]) -> Result<Value, String> {
    if args.len() < 3 {
        return Err(format!("{}: expected (buffer, offset, packed)", PACK_INTO));
    }
    let offset = match &args[1] {
        Value::Int(i) => *i as i64,
        Value::Long(i) => *i,
        _ => return Err(format!("{}: offset must be an int", PACK_INTO)),
    };
    let start = usize::try_from(offset)
        .map_err(|_| format!("{}: offset must not be negative", PACK_INTO))?;
    let packed = string_bytes(&args[2], PACK_INTO)?;
    let write = |elements: &mut Vec<Value>| -> Result<(), String> {
        for (i, b) in packed.iter().enumerate() {
            let pos = start
                .checked_add(i)
                .ok_or_else(|| format!("{}: offset overflow", PACK_INTO))?;
            let byte = Value::Int(*b as i32);
            if pos < elements.len() {
                elements[pos] = byte;
            } else {
                elements.push(byte);
            }
        }
        Ok(())
    };
    match &args[0] {
        Value::ClassInstance { fields, .. } => {
            let mut fields = fields.borrow_mut();
            match fields.get_mut("_elements") {
                Some(Value::Array { elements }) => write(elements)?,
                _ => return Err(format!("{}: buffer must be an ArrayList", PACK_INTO)),
            }
            Ok(Value::Void)
        }
        Value::Array { elements } => {
            let mut updated = elements.clone();
            write(&mut updated)?;
            Ok(array(updated))
        }
        Value::String(s) => {
            let mut chars: Vec<char> = s.chars().collect();
            for (i, b) in packed.iter().enumerate() {
                let pos = start
                    .checked_add(i)
                    .ok_or_else(|| format!("{}: offset overflow", PACK_INTO))?;
                if pos < chars.len() {
                    chars[pos] = *b as char;
                } else {
                    chars.push(*b as char);
                }
            }
            Ok(Value::String(Rc::new(chars.into_iter().collect())))
        }
        _ => Err(format!("{}: buffer must be an ArrayList or String", PACK_INTO)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(text: &str) -> Value {
        Value::String(Rc::new(text.to_string()))
    }

    #[test]
    fn pack_unpack_round_trip() {
        let packed = native_structext_pack(&[
            s(">ih"),
            array(vec![Value::Int(0x0102_0304), Value::Int(-2)]),
        ])
        .unwrap();
        assert_eq!(packed.display_string(), "\u{1}\u{2}\u{3}\u{4}\u{ff}\u{fe}");
        let unpacked = native_structext_unpack(&[s(">ih"), packed]).unwrap();
        match unpacked {
            Value::Array { elements } => {
                assert_eq!(elements[0].to_i64(), Some(0x0102_0304));
                assert_eq!(elements[1].to_i64(), Some(-2));
            }
            _ => panic!("expected an ArrayList"),
        }
    }

    #[test]
    fn pack_into_writes_bytes_at_offset() {
        let buffer = array(vec![Value::Int(0), Value::Int(0), Value::Int(0)]);
        let updated = native_structext_pack_into(&[buffer, Value::Int(1), s("ab")]).unwrap();
        match updated {
            Value::Array { elements } => {
                assert_eq!(elements[0].to_i64(), Some(0));
                assert_eq!(elements[1].to_i64(), Some(97));
                assert_eq!(elements[2].to_i64(), Some(98));
            }
            _ => panic!("expected an ArrayList"),
        }
    }
}
