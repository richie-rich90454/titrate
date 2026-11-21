#![allow(dead_code)]
//! Shared host runtime for the WASM parity tests.
//!
//! Implements the `env` import contract declared in
//! `trc/src/codegen/wasm/runtime.rs` and mirrors the bytecode VM's
//! `Value::display_string` semantics so output comparisons are exact.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use wasmi::{Caller, Linker, Val};

const TAG_I64: i32 = 0;
const TAG_F64: i32 = 1;
const TAG_BOOL: i32 = 2;
const TAG_STR: i32 = 3;
const TAG_OBJ: i32 = 4;

#[derive(Clone)]
pub enum HV {
    Void,
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Obj(usize),
    Closure(u32),
}

pub enum HObj {
    Class {
        name: String,
        fields: HashMap<String, HV>,
        order: Vec<String>,
    },
    List(Vec<HV>),
    Map(Vec<(HV, HV)>),
    Tuple(Vec<HV>),
    Enum {
        ename: String,
        variant: String,
        fields: Vec<HV>,
    },
    Result {
        ok: bool,
        val: HV,
    },
    Closure,
}

#[derive(Default)]
pub struct Host {
    pub output: Vec<String>,
    pub objects: Vec<HObj>,
    pub error: Option<HV>,
    pub root: PathBuf,
}

impl HV {
    fn display(&self, host: &Host) -> String {
        match self {
            HV::Void => "void".to_string(),
            HV::Int(v) => v.to_string(),
            HV::Float(v) => v.to_string(),
            HV::Bool(b) => b.to_string(),
            HV::Str(s) => s.clone(),
            HV::Obj(i) => display_obj(&host.objects[*i], host),
            HV::Closure(code) => format!("<closure #{}>", code),
        }
    }

    fn eq(&self, other: &HV) -> bool {
        match (self, other) {
            (HV::Int(a), HV::Int(b)) => a == b,
            (HV::Float(a), HV::Float(b)) => a.to_bits() == b.to_bits(),
            (HV::Bool(a), HV::Bool(b)) => a == b,
            (HV::Str(a), HV::Str(b)) => a == b,
            (HV::Obj(a), HV::Obj(b)) => a == b,
            (HV::Void, HV::Void) => true,
            _ => false,
        }
    }
}

fn display_obj(obj: &HObj, host: &Host) -> String {
    match obj {
        HObj::Class { name, fields, order } => {
            let items: Vec<String> = order
                .iter()
                .filter_map(|k| fields.get(k).map(|v| format!("{}: {}", k, v.display(host))))
                .collect();
            format!("{}({})", name, items.join(", "))
        }
        HObj::List(elems) => {
            let items: Vec<String> = elems.iter().map(|e| e.display(host)).collect();
            format!("[{}]", items.join(", "))
        }
        HObj::Map(entries) => {
            let items: Vec<String> = entries
                .iter()
                .map(|(k, v)| format!("{}: {}", k.display(host), v.display(host)))
                .collect();
            format!("{{{}}}", items.join(", "))
        }
        HObj::Tuple(elems) => {
            let items: Vec<String> = elems.iter().map(|e| e.display(host)).collect();
            format!("({})", items.join(", "))
        }
        HObj::Enum { variant, fields, .. } => {
            if fields.is_empty() {
                variant.clone()
            } else {
                let items: Vec<String> = fields.iter().map(|e| e.display(host)).collect();
                format!("{}({})", variant, items.join(", "))
            }
        }
        HObj::Result { ok, val } => {
            if *ok {
                format!("Ok({})", val.display(host))
            } else {
                format!("Err({})", val.display(host))
            }
        }
        HObj::Closure => "<closure>".to_string(),
    }
}

fn json_quote(s: &str) -> String {
    let mut out = String::from("\"");
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn json_ser_hv(v: &HV, host: &Host) -> String {
    match v {
        HV::Void => "null".to_string(),
        HV::Int(n) => n.to_string(),
        HV::Float(f) => f.to_string(),
        HV::Bool(b) => b.to_string(),
        HV::Str(s) => json_quote(s),
        HV::Obj(i) => json_ser_obj(&host.objects[*i], host),
        HV::Closure(_) => "null".to_string(),
    }
}

fn json_ser_obj(obj: &HObj, host: &Host) -> String {
    match obj {
        HObj::Map(entries) => {
            let items: Vec<String> = entries
                .iter()
                .map(|(k, v)| format!("{}: {}", json_quote(&k.display(host)), json_ser_hv(v, host)))
                .collect();
            format!("{{{}}}", items.join(", "))
        }
        HObj::List(elems) => {
            let items: Vec<String> = elems.iter().map(|e| json_ser_hv(e, host)).collect();
            format!("{{\"_elements\": [{}]}}", items.join(", "))
        }
        other => json_quote(&display_obj(other, host)),
    }
}

fn from_parts(c: &Caller<'_, Host>, t: i32, i: i64, f: f64, p: i32) -> HV {
    match t {
        TAG_I64 => HV::Int(i),
        TAG_F64 => HV::Float(f),
        TAG_BOOL => HV::Bool(p != 0),
        TAG_STR => HV::Str(read_str(c, p)),
        TAG_OBJ => HV::Obj((p - 1) as usize),
        _ => HV::Void,
    }
}

fn cvt_i64(_: &mut Caller<'_, Host>, v: HV) -> i64 {
    match v { HV::Int(x) => x, _ => 0 }
}
fn cvt_f64(_: &mut Caller<'_, Host>, v: HV) -> f64 {
    match v { HV::Float(x) => x, _ => 0.0 }
}
fn cvt_bool(_: &mut Caller<'_, Host>, v: HV) -> i32 {
    match v { HV::Bool(x) => x as i32, HV::Int(x) => (x != 0) as i32, _ => 0 }
}
fn cvt_str(c: &mut Caller<'_, Host>, v: HV) -> i32 {
    match v { HV::Str(s) => alloc_str(c, &s), _ => 0 }
}
fn cvt_obj(_: &mut Caller<'_, Host>, v: HV) -> i32 {
    match v { HV::Obj(x) => (x + 1) as i32, _ => 0 }
}
fn key_str(c: &Caller<'_, Host>, p: i32) -> HV {
    HV::Str(read_str(c, p))
}
fn key_i64(_: &Caller<'_, Host>, k: i64) -> HV {
    HV::Int(k)
}

pub fn read_str(caller: &Caller<'_, Host>, ptr: i32) -> String {
    let mem = caller
        .get_export("memory")
        .expect("module must export memory")
        .into_memory()
        .expect("memory export");
    let mut len_buf = [0u8; 4];
    mem.read(caller, ptr as usize, &mut len_buf).expect("read length");
    let len = i32::from_le_bytes(len_buf) as usize;
    let mut buf = vec![0u8; len];
    mem.read(caller, ptr as usize + 4, &mut buf).expect("read bytes");
    String::from_utf8_lossy(&buf).into_owned()
}

fn file_path(c: &Caller<'_, Host>, ptr: i32) -> PathBuf {
    let name = read_str(c, ptr);
    let p = Path::new(&name);
    if p.is_absolute() || c.data().root.as_os_str().is_empty() {
        p.to_path_buf()
    } else {
        c.data().root.join(p)
    }
}

pub fn alloc_str(caller: &mut Caller<'_, Host>, s: &str) -> i32 {
    let bytes = s.as_bytes();
    let global = caller
        .get_export("heap_ptr")
        .expect("module must export heap_ptr")
        .into_global()
        .expect("global export");
    let ptr = global.get(&*caller).i32().expect("heap_ptr is i32");
    let mem = caller
        .get_export("memory")
        .expect("module must export memory")
        .into_memory()
        .expect("memory export");
    let end = ptr as usize + 4 + bytes.len();
    if end > mem.data_size(&*caller) {
        let pages = end.div_ceil(65_536);
        let current = mem.size(&*caller);
        mem.grow(&mut *caller, (pages as u64) - current)
            .expect("memory growth");
    }
    mem.write(&mut *caller, ptr as usize, &(bytes.len() as i32).to_le_bytes())
        .expect("write length");
    mem.write(&mut *caller, ptr as usize + 4, bytes).expect("write bytes");
    let next = (end + 3) & !3;
    global
        .set(&mut *caller, Val::I32(next as i32))
        .expect("update heap_ptr");
    ptr
}

fn obj_at<T>(f: impl FnOnce(&HObj) -> T, host: &Host, handle: i32) -> T {
    f(&host.objects[(handle - 1) as usize])
}

fn obj_at_mut<T>(f: impl FnOnce(&mut HObj) -> T, host: &mut Host, handle: i32) -> T {
    f(&mut host.objects[(handle - 1) as usize])
}

fn class_field_mut<'a>(host: &'a mut Host, handle: i32, name: &str) -> &'a mut HashMap<String, HV> {
    let idx = (handle - 1) as usize;
    match &mut host.objects[idx] {
        HObj::Class { fields, order, .. } => {
            if !fields.contains_key(name) {
                order.push(name.to_string());
            }
            fields
        }
        _ => panic!("field access on non-class object"),
    }
}

fn class_field(host: &Host, handle: i32, name: &str) -> HV {
    let idx = (handle - 1) as usize;
    match &host.objects[idx] {
        HObj::Class { fields, .. } => fields.get(name).cloned().unwrap_or(HV::Void),
        _ => HV::Void,
    }
}

pub fn register_host(linker: &mut Linker<Host>) {
    macro_rules! reg {
        ($name:expr, $closure:expr) => {
            linker.func_wrap("env", $name, $closure).unwrap();
        };
    }

    reg!("print", |mut c: Caller<'_, Host>, p: i32| {
        let s = read_str(&c, p);
        c.data_mut().output.push(s);
    });
    reg!("print_raw", |mut c: Caller<'_, Host>, p: i32| {
        let s = read_str(&c, p);
        let out = &mut c.data_mut().output;
        if out.is_empty() {
            out.push(s);
        } else {
            out.last_mut().unwrap().push_str(&s);
        }
    });
    reg!("print_i64", |mut c: Caller<'_, Host>, v: i64| {
        c.data_mut().output.push(v.to_string());
    });
    reg!("print_f64", |mut c: Caller<'_, Host>, v: f64| {
        c.data_mut().output.push(v.to_string());
    });
    reg!("print_bool", |mut c: Caller<'_, Host>, v: i32| {
        c.data_mut()
            .output
            .push(if v != 0 { "true" } else { "false" }.to_string());
    });
    reg!("print_char", |mut c: Caller<'_, Host>, v: i32| {
        let ch = char::from_u32(v as u32).unwrap_or('\u{FFFD}');
        c.data_mut().output.push(ch.to_string());
    });
    reg!("concat", |mut c: Caller<'_, Host>, a: i32, b: i32| -> i32 {
        let s = read_str(&c, a) + &read_str(&c, b);
        alloc_str(&mut c, &s)
    });
    reg!("str_eq", |c: Caller<'_, Host>, a: i32, b: i32| -> i32 {
        (read_str(&c, a) == read_str(&c, b)) as i32
    });
    reg!("str_len", |c: Caller<'_, Host>, a: i32| -> i32 {
        read_str(&c, a).chars().count() as i32
    });
    reg!("str_char_at", |mut c: Caller<'_, Host>, a: i32, idx: i32| -> i32 {
        let s = read_str(&c, a);
        let ch = s.chars().nth(idx as usize).map(String::from).unwrap_or_default();
        alloc_str(&mut c, &ch)
    });
    reg!(
        "str_substring",
        |mut c: Caller<'_, Host>, a: i32, start: i32, end: i32| -> i32 {
            let s = read_str(&c, a);
            let chars: Vec<char> = s.chars().collect();
            let lo = (start.max(0) as usize).min(chars.len());
            let hi = (end.max(0) as usize).min(chars.len()).max(lo);
            let out: String = chars[lo..hi].iter().collect();
            alloc_str(&mut c, &out)
        }
    );
    reg!("str_index_of", |c: Caller<'_, Host>, a: i32, b: i32| -> i32 {
        read_str(&c, a).find(&read_str(&c, b)).map(|i| i as i32).unwrap_or(-1)
    });
    reg!("str_upper", |mut c: Caller<'_, Host>, a: i32| -> i32 {
        let s = read_str(&c, a).to_uppercase();
        alloc_str(&mut c, &s)
    });
    reg!("str_lower", |mut c: Caller<'_, Host>, a: i32| -> i32 {
        let s = read_str(&c, a).to_lowercase();
        alloc_str(&mut c, &s)
    });
    reg!("str_trim", |mut c: Caller<'_, Host>, a: i32| -> i32 {
        let s = read_str(&c, a).trim().to_string();
        alloc_str(&mut c, &s)
    });
    reg!("str_starts_with", |c: Caller<'_, Host>, a: i32, b: i32| -> i32 {
        read_str(&c, a).starts_with(&read_str(&c, b)) as i32
    });
    reg!("str_ends_with", |c: Caller<'_, Host>, a: i32, b: i32| -> i32 {
        read_str(&c, a).ends_with(&read_str(&c, b)) as i32
    });
    reg!("to_string_i64", |mut c: Caller<'_, Host>, v: i64| -> i32 {
        alloc_str(&mut c, &v.to_string())
    });
    reg!("to_string_f64", |mut c: Caller<'_, Host>, v: f64| -> i32 {
        alloc_str(&mut c, &v.to_string())
    });
    reg!("to_string_bool", |mut c: Caller<'_, Host>, v: i32| -> i32 {
        alloc_str(&mut c, if v != 0 { "true" } else { "false" })
    });
    reg!("to_string_char", |mut c: Caller<'_, Host>, v: i32| -> i32 {
        let ch = char::from_u32(v as u32).unwrap_or('\u{FFFD}');
        alloc_str(&mut c, &ch.to_string())
    });
    reg!("parse_int", |c: Caller<'_, Host>, p: i32| -> i64 {
        read_str(&c, p).trim().parse::<i64>().unwrap_or(0)
    });
    reg!("parse_double", |c: Caller<'_, Host>, p: i32| -> f64 {
        read_str(&c, p).trim().parse::<f64>().unwrap_or(f64::NAN)
    });
    reg!("fmod", |_: Caller<'_, Host>, a: f64, b: f64| -> f64 { a % b });

    macro_rules! math1 {
        ($name:expr, $f:expr) => {
            linker
                .func_wrap("env", $name, |_: Caller<'_, Host>, x: f64| -> f64 {
                    ($f)(x)
                })
                .unwrap();
        };
    }
    macro_rules! math2 {
        ($name:expr, $f:expr) => {
            linker
                .func_wrap(
                    "env",
                    $name,
                    |_: Caller<'_, Host>, x: f64, y: f64| -> f64 { ($f)(x, y) },
                )
                .unwrap();
        };
    }
    math1!("math_sin", f64::sin);
    math1!("math_cos", f64::cos);
    math1!("math_tan", f64::tan);
    math1!("math_asin", f64::asin);
    math1!("math_acos", f64::acos);
    math1!("math_atan", f64::atan);
    math2!("math_atan2", f64::atan2);
    math1!("math_exp", f64::exp);
    math1!("math_ln", f64::ln);
    math1!("math_log2", f64::log2);
    math1!("math_log10", f64::log10);
    math2!("math_pow", f64::powf);
    math1!("math_cbrt", f64::cbrt);
    math2!("math_hypot", f64::hypot);
    math1!("math_abs", f64::abs);
    math1!("math_floor", f64::floor);
    math1!("math_ceil", f64::ceil);
    math1!("math_round", f64::round);
    math1!("math_sqrt", f64::sqrt);

    // --- objects -------------------------------------------------------------
    reg!("obj_new", |mut c: Caller<'_, Host>| -> i32 {
        let h = c.data().objects.len() + 1;
        c.data_mut().objects.push(HObj::Class {
            name: "Object".to_string(),
            fields: HashMap::new(),
            order: vec![],
        });
        h as i32
    });
    reg!("obj_named_new", |mut c: Caller<'_, Host>, np: i32| -> i32 {
        let n = read_str(&c, np);
        let h = c.data().objects.len() + 1;
        c.data_mut().objects.push(HObj::Class {
            name: n,
            fields: HashMap::new(),
            order: vec![],
        });
        h as i32
    });
    reg!("obj_class", |mut c: Caller<'_, Host>, h: i32| -> i32 {
        let name = obj_at(
            |o| match o {
                HObj::Class { name, .. } => name.clone(),
                HObj::Enum { ename, .. } => ename.clone(),
                _ => String::new(),
            },
            c.data(),
            h,
        );
        alloc_str(&mut c, &name)
    });
    reg!("field_set_i64", |mut c: Caller<'_, Host>, h: i32, np: i32, v: i64| {
        let n = read_str(&c, np);
        class_field_mut(c.data_mut(), h, &n).insert(n, HV::Int(v));
    });
    reg!("field_set_f64", |mut c: Caller<'_, Host>, h: i32, np: i32, v: f64| {
        let n = read_str(&c, np);
        class_field_mut(c.data_mut(), h, &n).insert(n, HV::Float(v));
    });
    reg!("field_set_bool", |mut c: Caller<'_, Host>, h: i32, np: i32, v: i32| {
        let n = read_str(&c, np);
        class_field_mut(c.data_mut(), h, &n).insert(n, HV::Bool(v != 0));
    });
    reg!("field_set_str", |mut c: Caller<'_, Host>, h: i32, np: i32, sp: i32| {
        let n = read_str(&c, np);
        let s = read_str(&c, sp);
        class_field_mut(c.data_mut(), h, &n).insert(n, HV::Str(s));
    });
    reg!("field_set_obj", |mut c: Caller<'_, Host>, h: i32, np: i32, v: i32| {
        let n = read_str(&c, np);
        class_field_mut(c.data_mut(), h, &n).insert(n, HV::Obj((v - 1) as usize));
    });
    reg!("field_get_i64", |c: Caller<'_, Host>, h: i32, np: i32| -> i64 {
        let n = read_str(&c, np);
        match class_field(c.data(), h, &n) { HV::Int(v) => v, _ => 0 }
    });
    reg!("field_get_f64", |c: Caller<'_, Host>, h: i32, np: i32| -> f64 {
        let n = read_str(&c, np);
        match class_field(c.data(), h, &n) { HV::Float(v) => v, _ => 0.0 }
    });
    reg!("field_get_bool", |c: Caller<'_, Host>, h: i32, np: i32| -> i32 {
        let n = read_str(&c, np);
        match class_field(c.data(), h, &n) { HV::Bool(v) => v as i32, _ => 0 }
    });
    reg!("field_get_str", |mut c: Caller<'_, Host>, h: i32, np: i32| -> i32 {
        let n = read_str(&c, np);
        let s = match class_field(c.data(), h, &n) { HV::Str(s) => s, _ => String::new() };
        alloc_str(&mut c, &s)
    });
    reg!("field_get_obj", |c: Caller<'_, Host>, h: i32, np: i32| -> i32 {
        let n = read_str(&c, np);
        match class_field(c.data(), h, &n) { HV::Obj(v) => (v + 1) as i32, _ => 0 }
    });

    // --- ArrayList ----------------------------------------------------------
    reg!("list_new", |mut c: Caller<'_, Host>| -> i32 {
        let h = c.data().objects.len() + 1;
        c.data_mut().objects.push(HObj::List(vec![]));
        h as i32
    });
    reg!("list_add", |mut c: Caller<'_, Host>, h: i32, t: i32, i: i64, f: f64, p: i32| {
        let v = from_parts(&c, t, i, f, p);
        obj_at_mut(|o| if let HObj::List(xs) = o { xs.push(v) }, c.data_mut(), h);
    });
    reg!("list_set", |mut c: Caller<'_, Host>, h: i32, idx: i64, t: i32, i: i64, f: f64, p: i32| {
        let v = from_parts(&c, t, i, f, p);
        obj_at_mut(|o| if let HObj::List(xs) = o {
            let k = idx as usize;
            if k < xs.len() { xs[k] = v; }
        }, c.data_mut(), h);
    });
    macro_rules! indexed_get {
        ($name:expr, $ty:ty, $cvt:ident, $default:expr) => {
            reg!($name, |mut c: Caller<'_, Host>, h: i32, idx: i64| -> $ty {
                let v = obj_at(|o| match o {
                    HObj::List(xs) => xs.get(idx as usize).cloned(),
                    HObj::Tuple(xs) => xs.get(idx as usize).cloned(),
                    HObj::Enum { fields, .. } => fields.get(idx as usize).cloned(),
                    _ => None,
                }, c.data(), h);
                $cvt(&mut c, v.unwrap_or(HV::Void))
            });
        };
    }
    indexed_get!("list_get_i64", i64, cvt_i64, 0);
    indexed_get!("list_get_f64", f64, cvt_f64, 0.0);
    indexed_get!("list_get_bool", i32, cvt_bool, 0);
    indexed_get!("list_get_str", i32, cvt_str, 0);
    indexed_get!("list_get_obj", i32, cvt_obj, 0);
    indexed_get!("enum_get_i64", i64, cvt_i64, 0);
    indexed_get!("enum_get_f64", f64, cvt_f64, 0.0);
    indexed_get!("enum_get_bool", i32, cvt_bool, 0);
    indexed_get!("enum_get_str", i32, cvt_str, 0);
    indexed_get!("enum_get_obj", i32, cvt_obj, 0);
    indexed_get!("tuple_get_i64", i64, cvt_i64, 0);
    indexed_get!("tuple_get_f64", f64, cvt_f64, 0.0);
    indexed_get!("tuple_get_bool", i32, cvt_bool, 0);
    indexed_get!("tuple_get_str", i32, cvt_str, 0);
    indexed_get!("tuple_get_obj", i32, cvt_obj, 0);
    reg!("list_size", |c: Caller<'_, Host>, h: i32| -> i64 {
        obj_at(|o| match o { HObj::List(xs) => xs.len() as i64, _ => 0 }, c.data(), h)
    });
    reg!("list_contains", |c: Caller<'_, Host>, h: i32, t: i32, i: i64, f: f64, p: i32| -> i32 {
        let v = from_parts(&c, t, i, f, p);
        obj_at(|o| match o {
            HObj::List(xs) => xs.iter().any(|x| x.eq(&v)) as i32,
            _ => 0,
        }, c.data(), h)
    });
    reg!("list_index_of", |c: Caller<'_, Host>, h: i32, t: i32, i: i64, f: f64, p: i32| -> i64 {
        let v = from_parts(&c, t, i, f, p);
        obj_at(|o| match o {
            HObj::List(xs) => xs.iter().position(|x| x.eq(&v)).map(|x| x as i64).unwrap_or(-1),
            _ => -1,
        }, c.data(), h)
    });
    reg!("list_display", |mut c: Caller<'_, Host>, h: i32| -> i32 {
        let s = obj_at(|o| display_obj(o, c.data()), c.data(), h);
        alloc_str(&mut c, &s)
    });
    reg!("list_clear", |mut c: Caller<'_, Host>, h: i32| {
        obj_at_mut(|o| if let HObj::List(xs) = o { xs.clear() }, c.data_mut(), h);
    });
    reg!("list_is_empty", |c: Caller<'_, Host>, h: i32| -> i32 {
        obj_at(|o| match o { HObj::List(xs) => xs.is_empty() as i32, _ => 1 }, c.data(), h)
    });
    macro_rules! indexed_remove {
        ($name:expr, $ty:ty, $cvt:ident) => {
            reg!($name, |mut c: Caller<'_, Host>, h: i32, idx: i64| -> $ty {
                let v = obj_at_mut(|o| match o {
                    HObj::List(xs) if idx >= 0 && (idx as usize) < xs.len() => {
                        Some(xs.remove(idx as usize))
                    }
                    _ => None,
                }, c.data_mut(), h);
                $cvt(&mut c, v.unwrap_or(HV::Void))
            });
        };
    }
    indexed_remove!("list_remove_i64", i64, cvt_i64);
    indexed_remove!("list_remove_f64", f64, cvt_f64);
    indexed_remove!("list_remove_bool", i32, cvt_bool);
    indexed_remove!("list_remove_str", i32, cvt_str);
    indexed_remove!("list_remove_obj", i32, cvt_obj);
    macro_rules! list_pop {
        ($name:expr, $ty:ty, $cvt:ident) => {
            reg!($name, |mut c: Caller<'_, Host>, h: i32| -> $ty {
                let v = obj_at_mut(|o| match o {
                    HObj::List(xs) => xs.pop(),
                    _ => None,
                }, c.data_mut(), h);
                $cvt(&mut c, v.unwrap_or(HV::Void))
            });
        };
    }
    list_pop!("list_pop_i64", i64, cvt_i64);
    list_pop!("list_pop_f64", f64, cvt_f64);
    list_pop!("list_pop_bool", i32, cvt_bool);
    list_pop!("list_pop_str", i32, cvt_str);
    list_pop!("list_pop_obj", i32, cvt_obj);
    reg!("list_add_all", |mut c: Caller<'_, Host>, h: i32, other: i32| {
        let src: Vec<HV> = obj_at(
            |o| match o { HObj::List(xs) => xs.clone(), _ => vec![] },
            c.data(), other,
        );
        obj_at_mut(|o| if let HObj::List(xs) = o { xs.extend(src) }, c.data_mut(), h);
    });
    reg!("list_last_index_of", |c: Caller<'_, Host>, h: i32, t: i32, i: i64, f: f64, p: i32| -> i64 {
        let v = from_parts(&c, t, i, f, p);
        obj_at(|o| match o {
            HObj::List(xs) => xs.iter().rposition(|x| x.eq(&v)).map(|x| x as i64).unwrap_or(-1),
            _ => -1,
        }, c.data(), h)
    });
    reg!("list_max_i64", |c: Caller<'_, Host>, h: i32| -> i64 {
        obj_at(|o| match o {
            HObj::List(xs) => xs.iter().filter_map(|x| if let HV::Int(v) = x { Some(*v) } else { None }).max().unwrap_or(0),
            _ => 0,
        }, c.data(), h)
    });
    reg!("list_max_f64", |c: Caller<'_, Host>, h: i32| -> f64 {
        obj_at(|o| match o {
            HObj::List(xs) => xs.iter().filter_map(|x| if let HV::Float(v) = x { Some(*v) } else { None }).fold(f64::NEG_INFINITY, f64::max),
            _ => 0.0,
        }, c.data(), h)
    });
    reg!("list_min_i64", |c: Caller<'_, Host>, h: i32| -> i64 {
        obj_at(|o| match o {
            HObj::List(xs) => xs.iter().filter_map(|x| if let HV::Int(v) = x { Some(*v) } else { None }).min().unwrap_or(0),
            _ => 0,
        }, c.data(), h)
    });
    reg!("list_min_f64", |c: Caller<'_, Host>, h: i32| -> f64 {
        obj_at(|o| match o {
            HObj::List(xs) => xs.iter().filter_map(|x| if let HV::Float(v) = x { Some(*v) } else { None }).fold(f64::INFINITY, f64::min),
            _ => 0.0,
        }, c.data(), h)
    });
    reg!("list_reverse", |mut c: Caller<'_, Host>, h: i32| {
        obj_at_mut(|o| if let HObj::List(xs) = o { xs.reverse() }, c.data_mut(), h);
    });

    // --- HashMap ------------------------------------------------------------
    reg!("map_new", |mut c: Caller<'_, Host>| -> i32 {
        let h = c.data().objects.len() + 1;
        c.data_mut().objects.push(HObj::Map(vec![]));
        h as i32
    });
    macro_rules! map_put {
        ($name:expr, $kt:ty, $kf:ident) => {
            reg!($name, |mut c: Caller<'_, Host>, h: i32, k: $kt, t: i32, i: i64, f: f64, p: i32| {
                let key = $kf(&c, k);
                let v = from_parts(&c, t, i, f, p);
                obj_at_mut(|o| if let HObj::Map(xs) = o {
                    if let Some(e) = xs.iter_mut().find(|(ek, _)| ek.eq(&key)) {
                        e.1 = v;
                    } else {
                        xs.push((key, v));
                    }
                }, c.data_mut(), h);
            });
        };
    }
    map_put!("map_put_str", i32, key_str);
    map_put!("map_put_i64", i64, key_i64);
    macro_rules! map_get {
        ($name:expr, $kt:ty, $kf:ident, $ty:ty, $cvt:ident) => {
            reg!($name, |mut c: Caller<'_, Host>, h: i32, k: $kt| -> $ty {
                let key = $kf(&c, k);
                let v = obj_at(|o| match o {
                    HObj::Map(xs) => xs.iter().find(|(ek, _)| ek.eq(&key)).map(|(_, v)| v.clone()),
                    _ => None,
                }, c.data(), h);
                $cvt(&mut c, v.unwrap_or(HV::Void))
            });
        };
    }
    map_get!("map_get_str_i64", i32, key_str, i64, cvt_i64);
    map_get!("map_get_str_f64", i32, key_str, f64, cvt_f64);
    map_get!("map_get_str_bool", i32, key_str, i32, cvt_bool);
    map_get!("map_get_str_str", i32, key_str, i32, cvt_str);
    map_get!("map_get_str_obj", i32, key_str, i32, cvt_obj);
    map_get!("map_get_i64_i64", i64, key_i64, i64, cvt_i64);
    map_get!("map_get_i64_f64", i64, key_i64, f64, cvt_f64);
    map_get!("map_get_i64_bool", i64, key_i64, i32, cvt_bool);
    map_get!("map_get_i64_str", i64, key_i64, i32, cvt_str);
    map_get!("map_get_i64_obj", i64, key_i64, i32, cvt_obj);
    reg!("map_size", |c: Caller<'_, Host>, h: i32| -> i64 {
        obj_at(|o| match o { HObj::Map(xs) => xs.len() as i64, _ => 0 }, c.data(), h)
    });
    macro_rules! map_contains {
        ($name:expr, $kt:ty, $kf:ident) => {
            reg!($name, |c: Caller<'_, Host>, h: i32, k: $kt| -> i32 {
                let key = $kf(&c, k);
                obj_at(|o| match o {
                    HObj::Map(xs) => xs.iter().any(|(ek, _)| ek.eq(&key)) as i32,
                    _ => 0,
                }, c.data(), h)
            });
        };
    }
    map_contains!("map_contains_str", i32, key_str);
    map_contains!("map_contains_i64", i64, key_i64);
    reg!("map_display", |mut c: Caller<'_, Host>, h: i32| -> i32 {
        let s = obj_at(|o| display_obj(o, c.data()), c.data(), h);
        alloc_str(&mut c, &s)
    });
    reg!("map_keys", |mut c: Caller<'_, Host>, h: i32| -> i32 {
        let keys: Vec<HV> = obj_at(
            |o| match o { HObj::Map(xs) => xs.iter().map(|(k, _)| k.clone()).collect(), _ => vec![] },
            c.data(), h,
        );
        let nh = c.data().objects.len() + 1;
        c.data_mut().objects.push(HObj::List(keys));
        nh as i32
    });
    reg!("map_values", |mut c: Caller<'_, Host>, h: i32| -> i32 {
        let vals: Vec<HV> = obj_at(
            |o| match o { HObj::Map(xs) => xs.iter().map(|(_, v)| v.clone()).collect(), _ => vec![] },
            c.data(), h,
        );
        let nh = c.data().objects.len() + 1;
        c.data_mut().objects.push(HObj::List(vals));
        nh as i32
    });
    macro_rules! map_contains_value_v {
        ($name:expr, $mk:expr) => {
            reg!($name, |c: Caller<'_, Host>, h: i32, _t: i32, i: i64, f: f64, p: i32| -> i32 {
                let target = $mk(i, f, &read_str(&c, p), p);
                obj_at(|o| match o {
                    HObj::Map(xs) => xs.iter().any(|(_, x)| x.eq(&target)) as i32,
                    _ => 0,
                }, c.data(), h)
            });
        };
    }
    map_contains_value_v!("map_contains_value_i64", |i, _, _, _| HV::Int(i));
    map_contains_value_v!("map_contains_value_f64", |_, f, _, _| HV::Float(f));
    map_contains_value_v!("map_contains_value_bool", |i, _, _, _| HV::Bool(i != 0));
    map_contains_value_v!("map_contains_value_str", |_, _, s: &String, _| HV::Str(s.clone()));
    map_contains_value_v!("map_contains_value_obj", |_, _, _, p: i32| HV::Obj((p - 1) as usize));
    reg!("map_clear", |mut c: Caller<'_, Host>, h: i32| {
        obj_at_mut(|o| if let HObj::Map(xs) = o { xs.clear() }, c.data_mut(), h);
    });
    reg!("map_remove_str", |mut c: Caller<'_, Host>, h: i32, k: i32| {
        let key = HV::Str(read_str(&c, k));
        obj_at_mut(|o| if let HObj::Map(xs) = o { xs.retain(|(ek, _)| !ek.eq(&key)) }, c.data_mut(), h);
    });
    reg!("map_remove_i64", |mut c: Caller<'_, Host>, h: i32, k: i64| {
        let key = HV::Int(k);
        obj_at_mut(|o| if let HObj::Map(xs) = o { xs.retain(|(ek, _)| !ek.eq(&key)) }, c.data_mut(), h);
    });

    // --- enums --------------------------------------------------------------
    reg!("enum_new", |mut c: Caller<'_, Host>, np: i32, vp: i32, n: i64| -> i32 {
        let ename = read_str(&c, np);
        let variant = read_str(&c, vp);
        let h = c.data().objects.len() + 1;
        c.data_mut().objects.push(HObj::Enum {
            ename,
            variant,
            fields: vec![HV::Void; n as usize],
        });
        h as i32
    });
    reg!("enum_set", |mut c: Caller<'_, Host>, h: i32, idx: i64, t: i32, i: i64, f: f64, p: i32| {
        let v = from_parts(&c, t, i, f, p);
        obj_at_mut(|o| if let HObj::Enum { fields, .. } = o {
            let k = idx as usize;
            if k < fields.len() { fields[k] = v; }
        }, c.data_mut(), h);
    });
    reg!("enum_variant", |mut c: Caller<'_, Host>, h: i32| -> i32 {
        let v = obj_at(|o| match o { HObj::Enum { variant, .. } => variant.clone(), _ => String::new() }, c.data(), h);
        alloc_str(&mut c, &v)
    });
    reg!("enum_display", |mut c: Caller<'_, Host>, h: i32| -> i32 {
        let s = obj_at(|o| display_obj(o, c.data()), c.data(), h);
        alloc_str(&mut c, &s)
    });

    // --- Result -------------------------------------------------------------
    reg!("result_new", |mut c: Caller<'_, Host>, ok: i32, t: i32, i: i64, f: f64, p: i32| -> i32 {
        let v = from_parts(&c, t, i, f, p);
        let h = c.data().objects.len() + 1;
        c.data_mut().objects.push(HObj::Result { ok: ok != 0, val: v });
        h as i32
    });
    reg!("result_is_ok", |c: Caller<'_, Host>, h: i32| -> i32 {
        obj_at(|o| match o { HObj::Result { ok, .. } => *ok as i32, _ => 0 }, c.data(), h)
    });
    macro_rules! result_unwrap {
        ($name:expr, $ty:ty, $cvt:ident) => {
            reg!($name, |mut c: Caller<'_, Host>, h: i32| -> $ty {
                let v = obj_at(|o| match o {
                    HObj::Result { val, .. } => Some(val.clone()),
                    _ => None,
                }, c.data(), h);
                $cvt(&mut c, v.unwrap_or(HV::Void))
            });
        };
    }
    result_unwrap!("result_unwrap_i64", i64, cvt_i64);
    result_unwrap!("result_unwrap_f64", f64, cvt_f64);
    result_unwrap!("result_unwrap_bool", i32, cvt_bool);
    result_unwrap!("result_unwrap_str", i32, cvt_str);
    result_unwrap!("result_unwrap_obj", i32, cvt_obj);
    reg!("result_display", |mut c: Caller<'_, Host>, h: i32| -> i32 {
        let s = obj_at(|o| display_obj(o, c.data()), c.data(), h);
        alloc_str(&mut c, &s)
    });

    // --- tuples -------------------------------------------------------------
    reg!("tuple_new", |mut c: Caller<'_, Host>, n: i64| -> i32 {
        let h = c.data().objects.len() + 1;
        c.data_mut().objects.push(HObj::Tuple(vec![HV::Void; n as usize]));
        h as i32
    });
    reg!("tuple_set", |mut c: Caller<'_, Host>, h: i32, idx: i64, t: i32, i: i64, f: f64, p: i32| {
        let v = from_parts(&c, t, i, f, p);
        obj_at_mut(|o| if let HObj::Tuple(xs) = o {
            let k = idx as usize;
            if k < xs.len() { xs[k] = v; }
        }, c.data_mut(), h);
    });
    reg!("tuple_display", |mut c: Caller<'_, Host>, h: i32| -> i32 {
        let s = obj_at(|o| display_obj(o, c.data()), c.data(), h);
        alloc_str(&mut c, &s)
    });

    // --- closures -----------------------------------------------------------
    reg!("closure_new", |mut c: Caller<'_, Host>, code: i64| -> i32 {
        let h = c.data().objects.len() + 1;
        c.data_mut().objects.push(HObj::Class {
            name: "ClosureEnv".to_string(),
            fields: HashMap::new(),
            order: vec![],
        });
        class_field_mut(c.data_mut(), h as i32, "__code").insert("__code".to_string(), HV::Closure(code as u32));
        h as i32
    });

    // --- error handling -----------------------------------------------------
    reg!("err_set", |mut c: Caller<'_, Host>, t: i32, i: i64, f: f64, p: i32| {
        let v = from_parts(&c, t, i, f, p);
        c.data_mut().error = Some(v);
    });
    macro_rules! err_get {
        ($name:expr, $ty:ty, $cvt:ident) => {
            reg!($name, |mut c: Caller<'_, Host>| -> $ty {
                let v = c.data().error.clone().unwrap_or(HV::Void);
                $cvt(&mut c, v)
            });
        };
    }
    err_get!("err_get_i64", i64, cvt_i64);
    err_get!("err_get_f64", f64, cvt_f64);
    err_get!("err_get_bool", i32, cvt_bool);
    err_get!("err_get_str", i32, cvt_str);
    err_get!("err_get_obj", i32, cvt_obj);
    reg!("err_flag", |c: Caller<'_, Host>| -> i32 {
        c.data().error.is_some() as i32
    });
    reg!("err_clear", |mut c: Caller<'_, Host>| {
        c.data_mut().error = None;
    });

    // --- file ---------------------------------------------------------------
    reg!("file_read", |mut c: Caller<'_, Host>, p: i32| -> i32 {
        let path = file_path(&c, p);
        let s = std::fs::read_to_string(path).unwrap_or_default();
        alloc_str(&mut c, &s)
    });
    reg!("file_write", |c: Caller<'_, Host>, p: i32, data: i32| {
        let path = file_path(&c, p);
        let s = read_str(&c, data);
        std::fs::write(path, s).unwrap();
    });
    reg!("file_append", |c: Caller<'_, Host>, p: i32, data: i32| {
        use std::io::Write;
        let path = file_path(&c, p);
        let s = read_str(&c, data);
        let mut f = std::fs::OpenOptions::new().append(true).create(true).open(path).unwrap();
        f.write_all(s.as_bytes()).unwrap();
    });
    reg!("file_exists", |c: Caller<'_, Host>, p: i32| -> i32 {
        file_path(&c, p).exists() as i32
    });
    reg!("file_delete", |c: Caller<'_, Host>, p: i32| {
        std::fs::remove_file(file_path(&c, p)).unwrap();
    });

    // --- JSON ---------------------------------------------------------------
    reg!("json_stringify_i64", |mut c: Caller<'_, Host>, v: i64| -> i32 {
        alloc_str(&mut c, &v.to_string())
    });
    reg!("json_stringify_f64", |mut c: Caller<'_, Host>, v: f64| -> i32 {
        alloc_str(&mut c, &v.to_string())
    });
    reg!("json_stringify_bool", |mut c: Caller<'_, Host>, v: i32| -> i32 {
        alloc_str(&mut c, if v != 0 { "true" } else { "false" })
    });
    reg!("json_stringify_str", |mut c: Caller<'_, Host>, p: i32| -> i32 {
        let s = json_quote(&read_str(&c, p));
        alloc_str(&mut c, &s)
    });
    reg!("json_stringify_handle", |mut c: Caller<'_, Host>, h: i32| -> i32 {
        let s = json_ser_obj(&c.data().objects[(h - 1) as usize], c.data());
        alloc_str(&mut c, &s)
    });
}

pub fn typed_ast(src: &str) -> trc::ast::Program {
    let tokens = trc::lexer::tokenize(src).expect("tokenize");
    let ast = trc::parser::parse(tokens).expect("parse");
    trc::analyzer::analyze(&ast).expect("analyze")
}

pub fn compile_wasm(src: &str) -> Vec<u8> {
    let typed = typed_ast(src);
    trc::codegen::wasm::compile_to_wasm(&typed).expect("wasm codegen")
}

pub fn run_wasm(bytes: &[u8]) -> Vec<String> {
    run_wasm_with_root(bytes, Path::new(""))
}

pub fn run_wasm_with_root(bytes: &[u8], root: &Path) -> Vec<String> {
    let engine = wasmi::Engine::default();
    let mut host = Host::default();
    host.root = root.to_path_buf();
    let mut store = wasmi::Store::new(&engine, host);
    let mut linker = wasmi::Linker::new(&engine);
    register_host(&mut linker);
    let module = wasmi::Module::new(&engine, bytes).expect("valid wasm module");
    let instance = linker
        .instantiate_and_start(&mut store, &module)
        .expect("instantiate");
    let main = instance
        .get_typed_func::<(), ()>(&store, "main")
        .expect("exported main");
    main.call(&mut store, ()).expect("run main");
    store.into_data().output
}

pub fn run_vm(src: &str) -> Vec<String> {
    let typed = typed_ast(src);
    trc::bytecode::execute(&typed).expect("bytecode execution")
}

pub fn assert_parity(src: &str) {
    let wasm = run_wasm(&compile_wasm(src));
    let vm = run_vm(src);
    assert_eq!(wasm, vm, "WASM output diverged from bytecode VM");
}

pub fn assert_parity_with_root(src: &str, root: &str) {
    let wasm = run_wasm_with_root(&compile_wasm(src), Path::new(root));
    let typed = typed_ast(src);
    let vm = trc::bytecode::execute_with_root(&typed, std::path::Path::new(root))
        .expect("bytecode execution with module root");
    assert_eq!(wasm, vm, "WASM output diverged from bytecode VM (module test)");
}

pub fn assert_compile_error(src: &str, needle: &str) {
    let typed = typed_ast(src);
    let err = trc::codegen::wasm::compile_to_wasm(&typed)
        .expect_err("expected an explicit wasm error");
    assert!(
        err.contains(needle),
        "expected error containing '{}', got '{}'",
        needle,
        err
    );
}
