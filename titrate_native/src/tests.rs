use super::print::{titrate_free, titrate_string_concat};
use super::*;

    #[test]
    fn concat_two_strings() {
        let a = b"Hello, ";
        let b = b"World!";
        let mut out_len: i64 = 0;
        let ptr = unsafe {
            titrate_string_concat(
                a.len() as i64,
                a.as_ptr(),
                b.len() as i64,
                b.as_ptr(),
                &mut out_len,
            )
        };
        assert_eq!(out_len, (a.len() + b.len()) as i64);
        let combined = unsafe { std::slice::from_raw_parts(ptr, out_len as usize) };
        assert_eq!(combined, b"Hello, World!");
        unsafe { titrate_free(ptr) };
    }

    #[test]
    fn concat_empty_inputs() {
        let mut out_len: i64 = -1;
        let ptr = unsafe {
            titrate_string_concat(0, std::ptr::null(), -1, std::ptr::null(), &mut out_len)
        };
        assert_eq!(out_len, 0);
        unsafe { titrate_free(ptr) };
    }

    #[test]
    fn serialize_int() {
        let v = Value::Int(42);
        let mut buf = vec![0u8; 128];
        let n = serialize_value(&v, &mut buf);
        assert_eq!(n, 8); // 4 tag + 4 int
        assert_eq!(buf[0], 1); // TAG_INT
        let (v2, n2) = deserialize_value(&buf).unwrap();
        assert_eq!(n2, 8);
        assert!(matches!(v2, Value::Int(42)));
    }

    #[test]
    fn serialize_long() {
        let v = Value::Long(1234567890);
        let mut buf = vec![0u8; 128];
        let n = serialize_value(&v, &mut buf);
        assert_eq!(n, 12); // 4 tag + 8 long
        let (v2, _) = deserialize_value(&buf).unwrap();
        assert!(matches!(v2, Value::Long(1234567890)));
    }

    #[test]
    // 3.14 is the exact round-trip fixture value, not an approximation of PI.
    #[allow(clippy::approx_constant)]
    fn serialize_double() {
        let v = Value::Double(3.14);
        let mut buf = vec![0u8; 128];
        let n = serialize_value(&v, &mut buf);
        assert_eq!(n, 12); // 4 tag + 8 double
        let (v2, _) = deserialize_value(&buf).unwrap();
        match v2 {
            Value::Double(d) => assert!((d - 3.14).abs() < 0.001),
            _ => panic!("expected double"),
        }
    }

    #[test]
    fn serialize_bool() {
        let v = Value::Bool(true);
        let mut buf = vec![0u8; 128];
        let n = serialize_value(&v, &mut buf);
        assert_eq!(n, 5); // 4 tag + 1 bool
        let (v2, _) = deserialize_value(&buf).unwrap();
        assert!(matches!(v2, Value::Bool(true)));
    }

    #[test]
    fn serialize_string() {
        let v = Value::String(Rc::new("hello".to_string()));
        let mut buf = vec![0u8; 128];
        let n = serialize_value(&v, &mut buf);
        assert_eq!(n, 4 + 8 + 5); // 4 tag + 8 len + 5 bytes
        let (v2, _) = deserialize_value(&buf).unwrap();
        match v2 {
            Value::String(s) => assert_eq!(*s, "hello"),
            _ => panic!("expected string"),
        }
    }

    #[test]
    fn serialize_null() {
        let v = Value::Null;
        let mut buf = vec![0u8; 128];
        let n = serialize_value(&v, &mut buf);
        assert_eq!(n, 4); // just tag
        let (v2, _) = deserialize_value(&buf).unwrap();
        assert!(matches!(v2, Value::Null));
    }

    #[test]
    fn serialize_result_ok() {
        let v = Value::ResultOk(Box::new(Value::Int(99)));
        let mut buf = vec![0u8; 128];
        let _n = serialize_value(&v, &mut buf);
        let (v2, _) = deserialize_value(&buf).unwrap();
        match v2 {
            Value::ResultOk(inner) => assert!(matches!(*inner, Value::Int(99))),
            _ => panic!("expected ResultOk"),
        }
    }

    #[test]
    fn serialize_array() {
        let v = Value::Array {
            elements: vec![Value::Int(1), Value::Int(2), Value::Int(3)],
        };
        let mut buf = vec![0u8; 256];
        let _n = serialize_value(&v, &mut buf);
        let (v2, _) = deserialize_value(&buf).unwrap();
        match v2 {
            Value::Array { elements } => {
                assert_eq!(elements.len(), 3);
                assert!(matches!(elements[0], Value::Int(1)));
            }
            _ => panic!("expected Array"),
        }
    }

    #[test]
    fn native_call_math_sqrt() {
        // Serialize argument: double 16.0
        let arg = Value::Double(16.0);
        let mut arg_buf = vec![0u8; 128];
        let _arg_size = serialize_value(&arg, &mut arg_buf);

        let name = b"Math_sqrt";
        let mut result_buf = vec![0u8; 256];
        let mut result_cap: i64 = result_buf.len() as i64;

        let rc = unsafe {
            titrate_native_call(
                name.as_ptr(),
                name.len() as i64,
                arg_buf.as_ptr(),
                1,
                result_buf.as_mut_ptr(),
                &mut result_cap,
            )
        };
        assert_eq!(rc, 0);
        // Result should be a double ~4.0
        let (val, _) = deserialize_value(&result_buf).unwrap();
        match val {
            Value::Double(d) => assert!((d - 4.0).abs() < 0.001),
            other => panic!("expected double, got {:?}", other),
        }
    }

    #[test]
    fn native_call_string_length() {
        let arg = Value::String(Rc::new("hello".to_string()));
        let mut arg_buf = vec![0u8; 256];
        let _arg_size = serialize_value(&arg, &mut arg_buf);

        let name = b"String_length";
        let mut result_buf = vec![0u8; 256];
        let mut result_cap: i64 = result_buf.len() as i64;

        let rc = unsafe {
            titrate_native_call(
                name.as_ptr(),
                name.len() as i64,
                arg_buf.as_ptr(),
                1,
                result_buf.as_mut_ptr(),
                &mut result_cap,
            )
        };
        assert_eq!(rc, 0);
        let (val, _) = deserialize_value(&result_buf).unwrap();
        match val {
            Value::Int(5) => {}
            Value::Long(5) => {}
            other => panic!("expected 5, got {:?}", other),
        }
    }

    #[test]
    fn native_call_unknown() {
        let name = b"NoSuchFunction";
        let arg_buf = [0u8; 4];
        let mut result_buf = vec![0u8; 256];
        let mut result_cap: i64 = result_buf.len() as i64;

        let rc = unsafe {
            titrate_native_call(
                name.as_ptr(),
                name.len() as i64,
                arg_buf.as_ptr(),
                0,
                result_buf.as_mut_ptr(),
                &mut result_cap,
            )
        };
        assert_eq!(rc, 1);
    }

    // --- TitrateValue round-trip tests ---

    #[test]
    fn tv_roundtrip_int() {
        let v = Value::Int(-12345);
        let tv = value_to_titrate(&v);
        assert_eq!(tv.tag, TV_INT);
        let back = titrate_to_value(&tv);
        assert!(matches!(back, Value::Int(-12345)));
    }

    #[test]
    fn tv_roundtrip_long() {
        let v = Value::Long(0x7FFF_FFFF_FFFF_FFFF);
        let tv = value_to_titrate(&v);
        assert_eq!(tv.tag, TV_LONG);
        let back = titrate_to_value(&tv);
        assert!(matches!(back, Value::Long(0x7FFF_FFFF_FFFF_FFFF)));
    }

    #[test]
    fn tv_roundtrip_double() {
        let v = Value::Double(std::f64::consts::PI);
        let tv = value_to_titrate(&v);
        assert_eq!(tv.tag, TV_DOUBLE);
        let back = titrate_to_value(&tv);
        match back {
            Value::Double(d) => assert_eq!(d, std::f64::consts::PI),
            _ => panic!("expected double"),
        }
    }

    #[test]
    fn tv_roundtrip_bool() {
        let v = Value::Bool(true);
        let tv = value_to_titrate(&v);
        assert_eq!(tv.tag, TV_BOOL);
        assert!(matches!(titrate_to_value(&tv), Value::Bool(true)));

        let v = Value::Bool(false);
        let tv = value_to_titrate(&v);
        assert_eq!(tv.tag, TV_BOOL);
        assert!(matches!(titrate_to_value(&tv), Value::Bool(false)));
    }

    #[test]
    fn tv_roundtrip_char() {
        let v = Value::Char('A');
        let tv = value_to_titrate(&v);
        assert_eq!(tv.tag, TV_CHAR);
        assert!(matches!(titrate_to_value(&tv), Value::Char('A')));
    }

    #[test]
    fn tv_roundtrip_string() {
        let v = Value::String(Rc::new("hello world".to_string()));
        let mut tv = value_to_titrate(&v);
        assert_eq!(tv.tag, TV_STRING);
        let back = titrate_to_value(&tv);
        match back {
            Value::String(s) => assert_eq!(*s, "hello world"),
            _ => panic!("expected string"),
        }
        free_titrate_value(&mut tv);
    }

    #[test]
    fn tv_roundtrip_array() {
        let v = Value::Array {
            elements: vec![Value::Int(1), Value::Int(2), Value::Int(3)],
        };
        let mut tv = value_to_titrate(&v);
        assert_eq!(tv.tag, TV_ARRAY);
        let back = titrate_to_value(&tv);
        match back {
            Value::Array { elements } => {
                assert_eq!(elements.len(), 3);
                assert!(matches!(elements[0], Value::Int(1)));
                assert!(matches!(elements[2], Value::Int(3)));
            }
            _ => panic!("expected array"),
        }
        free_titrate_value(&mut tv);
    }

    #[test]
    fn tv_roundtrip_void_null() {
        let tv = value_to_titrate(&Value::Void);
        assert_eq!(tv.tag, TV_VOID);
        assert!(matches!(titrate_to_value(&tv), Value::Void));

        let tv = value_to_titrate(&Value::Null);
        assert_eq!(tv.tag, TV_NULL);
        assert!(matches!(titrate_to_value(&tv), Value::Null));
    }

    #[test]
    fn tv_roundtrip_result_ok() {
        let v = Value::ResultOk(Box::new(Value::Int(99)));
        let tv = value_to_titrate(&v);
        assert_eq!(tv.tag, TV_RESULT_OK);
        let back = titrate_to_value(&tv);
        match back {
            Value::ResultOk(inner) => assert!(matches!(*inner, Value::Int(99))),
            _ => panic!("expected ResultOk"),
        }
    }

    #[test]
    fn args_to_values_basic() {
        let args = vec![
            value_to_titrate(&Value::Int(10)),
            value_to_titrate(&Value::Double(2.5)),
        ];
        let vals = args_to_values(&args);
        assert_eq!(vals.len(), 2);
        assert!(matches!(vals[0], Value::Int(10)));
        match &vals[1] {
            Value::Double(d) => assert!((d - 2.5).abs() < 1e-12),
            _ => panic!("expected double"),
        }
    }

    #[test]
    fn titrate_value_size_matches_llvm() {
        // LLVM defines TitrateValue as { i32, i32, [16 x i8] } = 24 bytes.
        assert_eq!(
            std::mem::size_of::<TitrateValue>(),
            24,
            "TitrateValue must be 24 bytes to match LLVM layout"
        );
        assert_eq!(
            std::mem::align_of::<TitrateValue>(),
            8,
            "TitrateValue alignment must be 8"
        );
    }
