use super::super::super::value::Value;

pub(crate) fn native_slice(args: &[Value]) -> Result<Value, String> {
    if args.len() < 4 {
        return Err("Slice_slice requires 4 arguments (obj, start, stop, step)".to_string());
    }
    let obj = &args[0];
    let start = &args[1];
    let stop = &args[2];
    let step = &args[3];

    let to_int = |v: &Value| -> Option<i64> {
        match v {
            Value::Int(n) => Some(*n as i64),
            Value::Long(n) => Some(*n),
            Value::Void => None,
            _ => None,
        }
    };

    let s = to_int(start);
    let e = to_int(stop);
    let st = to_int(step).unwrap_or(1);
    let st = if st == 0 { 1 } else { st };

    match obj {
        Value::String(rc) => {
            let s_str = rc.as_str();
            let len = s_str.chars().count() as i64;
            let mut si = s.unwrap_or(0);
            if si < 0 { si = len + si; }
            let mut ei = e.unwrap_or(len);
            if ei < 0 { ei = len + ei; }
            if si > len { si = len; }
            if ei > len { ei = len; }
            if si < 0 { si = 0; }
            if ei < 0 { ei = 0; }
            let chars: Vec<char> = s_str.chars().collect();
            let mut result = String::new();
            if st > 0 {
                let mut i = si;
                while i < ei {
                    if i >= 0 && (i as usize) < chars.len() {
                        result.push(chars[i as usize]);
                    }
                    i += st;
                }
            } else {
                let mut i = si;
                while i > ei {
                    if i >= 0 && (i as usize) < chars.len() {
                        result.push(chars[i as usize]);
                    }
                    i += st;
                }
            }
            Ok(Value::String(std::rc::Rc::new(result)))
        }
        Value::Array { elements: arr } => {
            let len = arr.len() as i64;
            let mut si = s.unwrap_or(0);
            if si < 0 { si = len + si; }
            let mut ei = e.unwrap_or(len);
            if ei < 0 { ei = len + ei; }
            if si > len { si = len; }
            if ei > len { ei = len; }
            if si < 0 { si = 0; }
            if ei < 0 { ei = 0; }
            let mut result = Vec::new();
            if st > 0 {
                let mut i = si;
                while i < ei {
                    if i >= 0 && (i as usize) < arr.len() {
                        result.push(arr[i as usize].clone());
                    }
                    i += st;
                }
            } else {
                let mut i = si;
                while i > ei {
                    if i >= 0 && (i as usize) < arr.len() {
                        result.push(arr[i as usize].clone());
                    }
                    i += st;
                }
            }
            Ok(Value::Array { elements: result })
        }
        _ => Err("Slice_slice: unsupported type".to_string()),
    }
}