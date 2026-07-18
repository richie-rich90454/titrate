// Value trait implementations and tests
// Extracted from value.rs for better organization.

use std::fmt;
use std::rc::Rc;

use super::value::Value;

// ---------------------------------------------------------------------------
// Debug formatting (matches the old tree-walking interpreter)
// ---------------------------------------------------------------------------

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Void => write!(f, "void"),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Byte(v) => write!(f, "{}b", v),
            Value::Short(v) => write!(f, "{}s", v),
            Value::Int(v) => write!(f, "{}", v),
            Value::Long(v) => write!(f, "{}L", v),
            Value::Vast(v) => write!(f, "{}V", v),
            Value::Uvast(v) => write!(f, "{}U", v),
            Value::Float(v) => write!(f, "{}f", v),
            Value::Double(v) => write!(f, "{}d", v),
            Value::Half(v) => write!(f, "{}h", v),
            Value::Quad(v) => write!(f, "{}q", v),
            Value::Char(c) => write!(f, "'{}'", c),
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::ClassInstance { class_name, fields, .. } => {
                write!(f, "{}(", class_name)?;
                let mut first = true;
                for (k, v) in fields.borrow().iter() {
                    if !first {
                        write!(f, ", ")?;
                    }
                    first = false;
                    write!(f, "{}: {:?}", k, v)?;
                }
                write!(f, ")")
            }
            Value::EnumInstance {
                enum_name,
                variant,
                fields,
            } => {
                write!(f, "{}::{}", enum_name, variant)?;
                if !fields.is_empty() {
                    write!(f, "(")?;
                    for (i, v) in fields.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{:?}", v)?;
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
            Value::Owned(v) => write!(f, "Owned({:?})", v),
            Value::Array { elements } => {
                write!(f, "[")?;
                for (i, v) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:?}", v)?;
                }
                write!(f, "]")
            }
            Value::Tuple { elements } => {
                write!(f, "(")?;
                for (i, v) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:?}", v)?;
                }
                write!(f, ")")
            }
            Value::Ref(idx) => write!(f, "ref({})", idx),
            Value::RawPtr(idx) => write!(f, "raw_ptr({})", idx),
            Value::Function(idx) => write!(f, "fn#{}", idx),
            Value::NativeFn(idx) => write!(f, "native_fn#{}", idx),
            Value::ResultOk(v) => write!(f, "Ok({:?})", v),
            Value::ResultErr(v) => write!(f, "Err({:?})", v),
            Value::Null => write!(f, "null"),
            Value::Moved => write!(f, "<moved>"),
            Value::EnumVariant {
                enum_name,
                variant,
                ..
            } => write!(f, "<enum_variant {}::{}>", enum_name, variant),
            Value::FileHandle(_) => write!(f, "<file_handle>"),
            Value::Socket(_) => write!(f, "<socket>"),
            Value::Listener(_) => write!(f, "<listener>"),
            Value::Closure { func_idx, upvalues } => {
                write!(f, "<closure #{} [", func_idx)?;
                for (i, uv) in upvalues.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{:?}", uv.borrow())?;
                }
                write!(f, "]>")
            }
            Value::Cell(rc) => write!(f, "Cell({:?})", rc.borrow()),
        }
    }
}

// ---------------------------------------------------------------------------
// Equality (floats compare by bit pattern, matching the old interpreter)
// ---------------------------------------------------------------------------

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Void, Value::Void) => true,
            (Value::Null, Value::Null) => true,
            (Value::Moved, Value::Moved) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Byte(a), Value::Byte(b)) => a == b,
            (Value::Short(a), Value::Short(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Long(a), Value::Long(b)) => a == b,
            (Value::Vast(a), Value::Vast(b)) => a == b,
            (Value::Uvast(a), Value::Uvast(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a.to_bits() == b.to_bits(),
            (Value::Double(a), Value::Double(b)) => a.to_bits() == b.to_bits(),
            (Value::Half(a), Value::Half(b)) => a.to_bits() == b.to_bits(),
            (Value::Quad(a), Value::Quad(b)) => a.to_bits() == b.to_bits(),
            (Value::Char(a), Value::Char(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Ref(a), Value::Ref(b)) => a == b,
            (Value::RawPtr(a), Value::RawPtr(b)) => a == b,
            (Value::ResultOk(a), Value::ResultOk(b)) => a == b,
            (Value::ResultErr(a), Value::ResultErr(b)) => a == b,
            (Value::Function(a), Value::Function(b)) => a == b,
            (Value::NativeFn(a), Value::NativeFn(b)) => a == b,
            (Value::FileHandle(a), Value::FileHandle(b)) => Rc::ptr_eq(a, b),
            (Value::Socket(a), Value::Socket(b)) => Rc::ptr_eq(a, b),
            (Value::Listener(a), Value::Listener(b)) => Rc::ptr_eq(a, b),
            (Value::Closure { func_idx: a, .. }, Value::Closure { func_idx: b, .. }) => a == b,
            (Value::Tuple { elements: a }, Value::Tuple { elements: b }) => a == b,
            (Value::Cell(a), Value::Cell(b)) => a.borrow().eq(&b.borrow()),
            (Value::Cell(rc), other) | (other, Value::Cell(rc)) => {
                rc.borrow().eq(other)
            }
            // Cross-variant integer comparison: Byte/Short/Int/Long/Vast/Uvast
            // with different variants should compare by numeric value, not by
            // variant tag. This fixes HashMap key lookup when a key stored as
            // Long (from an int literal) is looked up as Int (after a cast).
            (a, b) if is_integer_variant(a) && is_integer_variant(b) => {
                a.to_i64() == b.to_i64()
            }
            // Cross-variant float comparison: Float/Double/Half/Quad with
            // different variants compare by numeric value.
            (a, b) if is_float_variant(a) && is_float_variant(b) => {
                a.to_f64() == b.to_f64()
            }
            // Cross-variant Char/String comparison: a Value::Char is equal to
            // a Value::String that contains exactly one character, when that
            // character matches. This is required because String.charAt
            // returns Value::Char (for the `as int` code-point cast pattern),
            // but HashMap keys are typically Value::String. Without this
            // bridge, `map.containsKey(String.charAt(s, i))` fails even when
            // the key is present.
            (Value::Char(c), Value::String(s)) | (Value::String(s), Value::Char(c)) => {
                s.chars().count() == 1 && s.chars().next() == Some(*c)
            }
            _ => false,
        }
    }
}

/// Returns true if the value is one of the integer numeric variants
/// (Byte, Short, Int, Long, Vast, Uvast).
fn is_integer_variant(v: &Value) -> bool {
    matches!(v, Value::Byte(_) | Value::Short(_) | Value::Int(_) | Value::Long(_) | Value::Vast(_) | Value::Uvast(_))
}

/// Returns true if the value is one of the floating-point variants
/// (Float, Double, Half, Quad).
fn is_float_variant(v: &Value) -> bool {
    matches!(v, Value::Float(_) | Value::Double(_) | Value::Half(_) | Value::Quad(_))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    // -- truthy ---------------------------------------------------------------

    #[test]
    fn test_value_truthy() {
        // Numeric types: zero is false, non-zero is true
        assert!(!Value::Byte(0).is_truthy());
        assert!(Value::Byte(1).is_truthy());
        assert!(!Value::Short(0).is_truthy());
        assert!(Value::Short(-1).is_truthy());
        assert!(!Value::Int(0).is_truthy());
        assert!(Value::Int(42).is_truthy());
        assert!(!Value::Long(0).is_truthy());
        assert!(Value::Long(-99).is_truthy());
        assert!(!Value::Vast(0).is_truthy());
        assert!(Value::Vast(100).is_truthy());
        assert!(!Value::Uvast(0).is_truthy());
        assert!(Value::Uvast(1).is_truthy());

        // Floating-point: zero is false, non-zero is true
        assert!(!Value::Float(0.0).is_truthy());
        assert!(Value::Float(3.14).is_truthy());
        assert!(!Value::Double(0.0).is_truthy());
        assert!(Value::Double(-1.0).is_truthy());
        assert!(!Value::Half(0.0).is_truthy());
        assert!(Value::Half(1.5).is_truthy());
        assert!(!Value::Quad(0.0).is_truthy());
        assert!(Value::Quad(2.7).is_truthy());

        // Bool
        assert!(!Value::Bool(false).is_truthy());
        assert!(Value::Bool(true).is_truthy());

        // String: empty is false, non-empty is true
        assert!(!Value::String(Rc::new(String::new())).is_truthy());
        assert!(Value::String(Rc::new("hello".to_string())).is_truthy());

        // Null / Void / Moved are always false
        assert!(!Value::Null.is_truthy());
        assert!(!Value::Void.is_truthy());
        assert!(!Value::Moved.is_truthy());

        // Everything else defaults to true
        assert!(Value::Char('a').is_truthy());
        assert!(Value::Ref(0).is_truthy());
        assert!(Value::RawPtr(0).is_truthy());
        assert!(Value::Function(0).is_truthy());
        assert!(Value::NativeFn(0).is_truthy());
        assert!(Value::ResultOk(Box::new(Value::Int(1))).is_truthy());
        assert!(Value::ResultErr(Box::new(Value::String(Rc::new("err".to_string())))).is_truthy());
        assert!(Value::Array { elements: vec![] }.is_truthy());
        assert!(Value::Owned(Box::new(Value::Int(1))).is_truthy());
        assert!(Value::EnumVariant {
            enum_name: "E".to_string(),
            variant: "A".to_string(),
            field_count: 0,
        }.is_truthy());
    }

    // -- conversions ----------------------------------------------------------

    #[test]
    fn test_value_conversions() {
        // to_i64
        assert_eq!(Value::Byte(-1).to_i64(), Some(-1));
        assert_eq!(Value::Short(1000).to_i64(), Some(1000));
        assert_eq!(Value::Int(42).to_i64(), Some(42));
        assert_eq!(Value::Long(-999).to_i64(), Some(-999));
        assert_eq!(Value::Vast(12345).to_i64(), Some(12345));
        assert_eq!(Value::Uvast(999).to_i64(), Some(999));
        assert_eq!(Value::Char('A').to_i64(), Some(65));
        assert_eq!(Value::Bool(true).to_i64(), Some(1));
        assert_eq!(Value::Bool(false).to_i64(), Some(0));
        assert_eq!(Value::Float(3.0).to_i64(), Some(3));
        assert_eq!(Value::Double(7.0).to_i64(), Some(7));
        assert_eq!(Value::Half(2.0).to_i64(), Some(2));
        assert_eq!(Value::Quad(5.0).to_i64(), Some(5));
        assert_eq!(Value::Null.to_i64(), None);
        assert_eq!(Value::Void.to_i64(), None);
        assert_eq!(Value::String(Rc::new("x".to_string())).to_i64(), None);

        // to_f64
        assert_eq!(Value::Float(3.14).to_f64(), Some(3.14_f32 as f64));
        assert_eq!(Value::Double(2.718).to_f64(), Some(2.718));
        assert_eq!(Value::Half(1.5).to_f64(), Some(1.5_f32 as f64));
        assert_eq!(Value::Quad(0.1).to_f64(), Some(0.1));
        assert_eq!(Value::Byte(3).to_f64(), Some(3.0));
        assert_eq!(Value::Short(-5).to_f64(), Some(-5.0));
        assert_eq!(Value::Int(10).to_f64(), Some(10.0));
        assert_eq!(Value::Long(100).to_f64(), Some(100.0));
        assert_eq!(Value::Vast(-1).to_f64(), Some(-1.0));
        assert_eq!(Value::Uvast(7).to_f64(), Some(7.0));
        assert_eq!(Value::Null.to_f64(), None);
        assert_eq!(Value::Bool(true).to_f64(), None);
    }

    // -- equality -------------------------------------------------------------

    #[test]
    fn test_value_equality() {
        // Same-type equality
        assert_eq!(Value::Bool(true), Value::Bool(true));
        assert_eq!(Value::Byte(7), Value::Byte(7));
        assert_eq!(Value::Short(10), Value::Short(10));
        assert_eq!(Value::Int(42), Value::Int(42));
        assert_eq!(Value::Long(-1), Value::Long(-1));
        assert_eq!(Value::Vast(100), Value::Vast(100));
        assert_eq!(Value::Uvast(50), Value::Uvast(50));
        assert_eq!(Value::Char('x'), Value::Char('x'));
        assert_eq!(
            Value::String(Rc::new("hi".to_string())),
            Value::String(Rc::new("hi".to_string()))
        );
        assert_eq!(Value::Ref(3), Value::Ref(3));
        assert_eq!(Value::RawPtr(0), Value::RawPtr(0));
        assert_eq!(Value::Function(1), Value::Function(1));
        assert_eq!(Value::NativeFn(2), Value::NativeFn(2));
        assert_eq!(Value::Void, Value::Void);
        assert_eq!(Value::Null, Value::Null);
        assert_eq!(Value::Moved, Value::Moved);

        // Floats compare by bits (so NaN == NaN, -0.0 != +0.0)
        let nan_f32 = f32::NAN;
        assert_eq!(
            Value::Float(nan_f32),
            Value::Float(nan_f32)
        );
        assert_ne!(
            Value::Double(-0.0),
            Value::Double(0.0)
        );

        // ResultOk / ResultErr
        assert_eq!(
            Value::ResultOk(Box::new(Value::Int(1))),
            Value::ResultOk(Box::new(Value::Int(1)))
        );
        assert_eq!(
            Value::ResultErr(Box::new(Value::String(Rc::new("e".to_string())))),
            Value::ResultErr(Box::new(Value::String(Rc::new("e".to_string()))))
        );

        // Cross-variant integer comparison: equal by numeric value
        assert_eq!(Value::Int(0), Value::Long(0));
        assert_eq!(Value::Byte(1), Value::Short(1));
        assert_eq!(Value::Byte(7), Value::Long(7));
        assert_eq!(Value::Short(100), Value::Int(100));
        assert_eq!(Value::Vast(42), Value::Int(42));
        assert_eq!(Value::Uvast(5), Value::Byte(5));
        // Cross-variant float comparison: equal by numeric value
        assert_eq!(Value::Float(1.0), Value::Double(1.0));
        assert_eq!(Value::Half(2.0), Value::Float(2.0));
        assert_eq!(Value::Quad(3.5), Value::Double(3.5));
        // Different categories are still not equal
        assert_ne!(Value::Null, Value::Void);
        assert_ne!(Value::Bool(true), Value::Int(1));
        assert_ne!(Value::Int(1), Value::Float(1.0));
    }

    // -- display_string -------------------------------------------------------

    #[test]
    fn test_display_string() {
        assert_eq!(Value::Void.display_string(), "void");
        assert_eq!(Value::Bool(true).display_string(), "true");
        assert_eq!(Value::Bool(false).display_string(), "false");
        assert_eq!(Value::Byte(-3).display_string(), "-3");
        assert_eq!(Value::Short(1000).display_string(), "1000");
        assert_eq!(Value::Int(42).display_string(), "42");
        assert_eq!(Value::Long(-1).display_string(), "-1");
        assert_eq!(Value::Vast(999).display_string(), "999");
        assert_eq!(Value::Uvast(7).display_string(), "7");
        assert_eq!(Value::Float(3.14).display_string(), "3.14");
        assert_eq!(Value::Double(2.718).display_string(), "2.718");
        assert_eq!(Value::Half(1.5).display_string(), "1.5");
        assert_eq!(Value::Quad(0.1).display_string(), "0.1");
        assert_eq!(Value::Char('A').display_string(), "A");
        assert_eq!(
            Value::String(Rc::new("hello".to_string())).display_string(),
            "hello"
        );
        assert_eq!(Value::Null.display_string(), "null");
        assert_eq!(Value::Moved.display_string(), "<moved>");
        assert_eq!(Value::Ref(5).display_string(), "ref(5)");
        assert_eq!(Value::RawPtr(99).display_string(), "raw_ptr(99)");
        assert_eq!(Value::Function(3).display_string(), "<fn #3>");
        assert_eq!(Value::NativeFn(7).display_string(), "<native fn #7>");

        // ResultOk / ResultErr
        assert_eq!(
            Value::ResultOk(Box::new(Value::Int(1))).display_string(),
            "Ok(1)"
        );
        assert_eq!(
            Value::ResultErr(Box::new(Value::String(Rc::new("fail".to_string())))).display_string(),
            "Err(fail)"
        );

        // Array
        assert_eq!(
            Value::Array {
                elements: vec![Value::Int(1), Value::Int(2), Value::Int(3)]
            }
            .display_string(),
            "[1, 2, 3]"
        );

        // Owned delegates to inner
        assert_eq!(
            Value::Owned(Box::new(Value::Int(42))).display_string(),
            "42"
        );

        // EnumVariant
        assert_eq!(
            Value::EnumVariant {
                enum_name: "Option".to_string(),
                variant: "Some".to_string(),
                field_count: 1,
            }
            .display_string(),
            "<variant Some>"
        );

        // ClassInstance
        let fields = Rc::new(RefCell::new({
            let mut m = HashMap::new();
            m.insert("x".to_string(), Value::Int(10));
            m
        }));
        let vtable = HashMap::new();
        assert_eq!(
            Value::ClassInstance {
                class_name: "Point".to_string(),
                fields,
                vtable,
            }
            .display_string(),
            "Point(x: 10)"
        );

        // EnumInstance
        assert_eq!(
            Value::EnumInstance {
                enum_name: "Color".to_string(),
                variant: "Red".to_string(),
                fields: vec![],
            }
            .display_string(),
            "Red"
        );
        assert_eq!(
            Value::EnumInstance {
                enum_name: "Color".to_string(),
                variant: "RGB".to_string(),
                fields: vec![Value::Int(255), Value::Int(128), Value::Int(0)],
            }
            .display_string(),
            "RGB(255, 128, 0)"
        );
    }

    // -- type_name ------------------------------------------------------------

    #[test]
    fn test_type_name() {
        assert_eq!(Value::Void.type_name(), "void");
        assert_eq!(Value::Null.type_name(), "null");
        assert_eq!(Value::Moved.type_name(), "moved");
        assert_eq!(Value::Bool(false).type_name(), "bool");
        assert_eq!(Value::Byte(0).type_name(), "byte");
        assert_eq!(Value::Short(0).type_name(), "short");
        assert_eq!(Value::Int(0).type_name(), "int");
        assert_eq!(Value::Long(0).type_name(), "long");
        assert_eq!(Value::Vast(0).type_name(), "vast");
        assert_eq!(Value::Uvast(0).type_name(), "uvast");
        assert_eq!(Value::Float(0.0).type_name(), "float");
        assert_eq!(Value::Double(0.0).type_name(), "double");
        assert_eq!(Value::Half(0.0).type_name(), "half");
        assert_eq!(Value::Quad(0.0).type_name(), "quad");
        assert_eq!(Value::Char('a').type_name(), "char");
        assert_eq!(Value::String(Rc::new(String::new())).type_name(), "string");
        assert_eq!(Value::Ref(0).type_name(), "ref");
        assert_eq!(Value::RawPtr(0).type_name(), "raw_ptr");
        assert_eq!(Value::Function(0).type_name(), "function");
        assert_eq!(Value::NativeFn(0).type_name(), "native_fn");
        assert_eq!(Value::ResultOk(Box::new(Value::Int(1))).type_name(), "result");
        assert_eq!(Value::ResultErr(Box::new(Value::Int(1))).type_name(), "result");
        assert_eq!(Value::Array { elements: vec![] }.type_name(), "array");
        assert_eq!(Value::Owned(Box::new(Value::Int(1))).type_name(), "owned");

        // ClassInstance and EnumInstance return their category names
        let fields = Rc::new(RefCell::new(HashMap::new()));
        let vtable = HashMap::new();
        assert_eq!(
            Value::ClassInstance {
                class_name: "MyClass".to_string(),
                fields,
                vtable,
            }
            .type_name(),
            "class_instance"
        );
        assert_eq!(
            Value::EnumInstance {
                enum_name: "MyEnum".to_string(),
                variant: "A".to_string(),
                fields: vec![],
            }
            .type_name(),
            "enum_instance"
        );
        assert_eq!(
            Value::EnumVariant {
                enum_name: "E".to_string(),
                variant: "V".to_string(),
                field_count: 0,
            }
            .type_name(),
            "enum_variant"
        );
    }
}
