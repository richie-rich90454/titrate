//! WebAssembly backend conformance tests.
//!
//! Each test compiles a Titrate source through the real lexer/parser/analyzer,
//! lowers it to a `.wasm` module, executes that module on the `wasmi`
//! interpreter with a small host runtime, and compares its captured output
//! byte-for-byte with the bytecode VM's output for the same source.

use std::collections::HashMap;
use wasmi::{Caller, Engine, Linker, Module, Store, Val};

#[derive(Clone)]
enum HostVal {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Obj(usize),
}

enum HostObj {
    Fields(HashMap<String, HostVal>),
    #[allow(dead_code)]
    List(Vec<HostVal>),
    #[allow(dead_code)]
    Map(HashMap<String, HostVal>),
}

#[derive(Default)]
struct Host {
    output: Vec<String>,
    objects: Vec<HostObj>,
}

fn obj_at(host: &Host, handle: i32) -> &HostObj {
    &host.objects[(handle - 1) as usize]
}

fn obj_at_mut(host: &mut Host, handle: i32) -> &mut HostObj {
    &mut host.objects[(handle - 1) as usize]
}

fn read_str(caller: &Caller<'_, Host>, ptr: i32) -> String {
    let mem = caller
        .get_export("memory")
        .expect("module must export memory")
        .into_memory()
        .expect("memory export");
    let mut len_buf = [0u8; 4];
    mem.read(&caller, ptr as usize, &mut len_buf).expect("read length");
    let len = i32::from_le_bytes(len_buf) as usize;
    let mut buf = vec![0u8; len];
    mem.read(&caller, ptr as usize + 4, &mut buf).expect("read bytes");
    String::from_utf8_lossy(&buf).into_owned()
}

fn alloc_str(caller: &mut Caller<'_, Host>, s: &str) -> i32 {
    let bytes = s.as_bytes();
    let global = caller
        .get_export("heap_ptr")
        .expect("module must export heap_ptr")
        .into_global()
        .expect("global export");
    let ptr = global.get(&caller).i32().expect("heap_ptr is i32");
    let mem = caller
        .get_export("memory")
        .expect("module must export memory")
        .into_memory()
        .expect("memory export");
    let end = ptr as usize + 4 + bytes.len();
    if end > mem.data_size(&caller) {
        let pages = end.div_ceil(65_536);
        let current = mem.size(&caller);
        mem.grow(&mut *caller, (pages as u64) - current)
            .expect("memory growth");
    }
    mem.write(&mut *caller, ptr as usize, &(bytes.len() as i32).to_le_bytes())
        .expect("write length");
    mem.write(&mut *caller, ptr as usize + 4, bytes)
        .expect("write bytes");
    let next = (end + 3) & !3;
    global
        .set(&mut *caller, Val::I32(next as i32))
        .expect("update heap_ptr");
    ptr
}

fn register_host(linker: &mut Linker<Host>) {
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

    reg!("obj_new", |mut c: Caller<'_, Host>| -> i32 {
        let handle = c.data().objects.len() + 1;
        c.data_mut().objects.push(HostObj::Fields(HashMap::new()));
        handle as i32
    });
    reg!("field_set_i64", |mut c: Caller<'_, Host>, h: i32, np: i32, v: i64| {
        let n = read_str(&c, np);
        if let HostObj::Fields(f) = obj_at_mut(c.data_mut(), h) {
            f.insert(n, HostVal::Int(v));
        }
    });
    reg!("field_set_f64", |mut c: Caller<'_, Host>, h: i32, np: i32, v: f64| {
        let n = read_str(&c, np);
        if let HostObj::Fields(f) = obj_at_mut(c.data_mut(), h) {
            f.insert(n, HostVal::Float(v));
        }
    });
    reg!("field_set_bool", |mut c: Caller<'_, Host>, h: i32, np: i32, v: i32| {
        let n = read_str(&c, np);
        if let HostObj::Fields(f) = obj_at_mut(c.data_mut(), h) {
            f.insert(n, HostVal::Bool(v != 0));
        }
    });
    reg!("field_set_str", |mut c: Caller<'_, Host>, h: i32, np: i32, sp: i32| {
        let n = read_str(&c, np);
        let s = read_str(&c, sp);
        if let HostObj::Fields(f) = obj_at_mut(c.data_mut(), h) {
            f.insert(n, HostVal::Str(s));
        }
    });
    reg!("field_set_obj", |mut c: Caller<'_, Host>, h: i32, np: i32, v: i32| {
        let n = read_str(&c, np);
        if let HostObj::Fields(f) = obj_at_mut(c.data_mut(), h) {
            f.insert(n, HostVal::Obj((v - 1) as usize));
        }
    });
    reg!("field_get_i64", |c: Caller<'_, Host>, h: i32, np: i32| -> i64 {
        let n = read_str(&c, np);
        match obj_at(c.data(), h) {
            HostObj::Fields(f) => match f.get(&n) {
                Some(HostVal::Int(x)) => *x,
                _ => 0,
            },
            _ => 0,
        }
    });
    reg!("field_get_f64", |c: Caller<'_, Host>, h: i32, np: i32| -> f64 {
        let n = read_str(&c, np);
        match obj_at(c.data(), h) {
            HostObj::Fields(f) => match f.get(&n) {
                Some(HostVal::Float(x)) => *x,
                _ => 0.0,
            },
            _ => 0.0,
        }
    });
    reg!("field_get_bool", |c: Caller<'_, Host>, h: i32, np: i32| -> i32 {
        let n = read_str(&c, np);
        match obj_at(c.data(), h) {
            HostObj::Fields(f) => match f.get(&n) {
                Some(HostVal::Bool(x)) => *x as i32,
                _ => 0,
            },
            _ => 0,
        }
    });
    reg!("field_get_str", |mut c: Caller<'_, Host>, h: i32, np: i32| -> i32 {
        let n = read_str(&c, np);
        let s = match obj_at(c.data(), h) {
            HostObj::Fields(f) => match f.get(&n) {
                Some(HostVal::Str(x)) => Some(x.clone()),
                _ => None,
            },
            _ => None,
        };
        s.map(|x| alloc_str(&mut c, &x)).unwrap_or(0)
    });
    reg!("field_get_obj", |c: Caller<'_, Host>, h: i32, np: i32| -> i32 {
        let n = read_str(&c, np);
        match obj_at(c.data(), h) {
            HostObj::Fields(f) => match f.get(&n) {
                Some(HostVal::Obj(x)) => (*x + 1) as i32,
                _ => 0,
            },
            _ => 0,
        }
    });
}

fn typed_ast(src: &str) -> trc::ast::Program {
    let tokens = trc::lexer::tokenize(src).expect("tokenize");
    let ast = trc::parser::parse(tokens).expect("parse");
    trc::analyzer::analyze(&ast).expect("analyze")
}

fn compile_wasm(src: &str) -> Vec<u8> {
    let typed = typed_ast(src);
    trc::codegen::wasm::compile_to_wasm(&typed).expect("wasm codegen")
}

fn run_wasm(bytes: &[u8]) -> Vec<String> {
    let engine = Engine::default();
    let mut store = Store::new(&engine, Host::default());
    let mut linker = Linker::new(&engine);
    register_host(&mut linker);
    let module = Module::new(&engine, bytes).expect("valid wasm module");
    let instance = linker
        .instantiate_and_start(&mut store, &module)
        .expect("instantiate");
    let main = instance
        .get_typed_func::<(), ()>(&store, "main")
        .expect("exported main");
    main.call(&mut store, ()).expect("run main");
    store.into_data().output
}

fn run_vm(src: &str) -> Vec<String> {
    let typed = typed_ast(src);
    trc::bytecode::execute(&typed).expect("bytecode execution")
}

fn assert_parity(src: &str) {
    let wasm = run_wasm(&compile_wasm(src));
    let vm = run_vm(src);
    assert_eq!(wasm, vm, "WASM output diverged from bytecode VM");
}

#[test]
fn wasm_emits_valid_magic() {
    let bytes = compile_wasm("public fn main(): void {}");
    assert_eq!(&bytes[..4], b"\0asm");
}

#[test]
fn wasm_dowhile_cfor_and_globals() {
    assert_parity(
        r#"
public fn main(): void {
    var i: int = 0;
    do {
        io::println("do " + Integer.toString(i));
        i = i + 1;
    } while (i < 3);
    for (let j = 0; j < 2; j++) {
        io::println("for " + Integer.toString(j));
    }
}
"#,
    );
}

#[test]
fn wasm_arithmetic_and_conversions() {
    assert_parity(
        r#"
public fn main(): void {
    io::println(Integer.toString(10 + 5));
    io::println(Integer.toString(20 - 8));
    io::println(Integer.toString(6 * 7));
    io::println(Integer.toString(100 / 3));
    io::println(Integer.toString(17 % 5));
    io::println(Double.toString(3.14159));
    io::println("The answer is " + Integer.toString(42));
    io::println(Double.toString(42 as double));
}
"#,
    );
}

#[test]
fn wasm_control_flow() {
    assert_parity(
        r#"
public fn main(): void {
    if (3 > 2) {
        io::println("greater");
    } else {
        io::println("not greater");
    }
    var i: int = 0;
    while (i < 3) {
        io::println(Integer.toString(i));
        i = i + 1;
    }
    var j: int = 0;
    while (true) {
        j = j + 1;
        if (j == 2) { continue; }
        if (j > 4) { break; }
        io::println("j=" + Integer.toString(j));
    }
    let label: string = 5 > 3 ? "yes" : "no";
    io::println(label);
}
"#,
    );
}

#[test]
fn wasm_functions_and_recursion() {
    assert_parity(
        r#"
fn add(a: int, b: int): int { return a + b; }
fn factorial(n: int): int {
    if (n <= 1) { return 1; }
    return n * factorial(n - 1);
}
public fn main(): void {
    io::println(Integer.toString(add(3, 4)));
    io::println(Integer.toString(factorial(6)));
}
"#,
    );
}

#[test]
fn wasm_operators_and_casts() {
    assert_parity(
        r#"
public fn main(): void {
    io::println(Integer.toString(5 & 3));
    io::println(Integer.toString(5 | 3));
    io::println(Integer.toString(5 ^ 3));
    io::println(Integer.toString(1 << 4));
    io::println(Integer.toString(16 >> 2));
    io::println(Boolean.toString(true && false));
    io::println(Boolean.toString(true || false));
    io::println(Boolean.toString(!true));
    io::println(Boolean.toString(5 > 3));
    io::println(Boolean.toString(3 >= 3));
    io::println(Boolean.toString(2 < 1));
    io::println(Boolean.toString(1 <= 1));
    io::println(Boolean.toString(5 == 5));
    io::println(Boolean.toString(4 != 5));
    io::println(Boolean.toString(-3 < 0));
}
"#,
    );
}

#[test]
fn wasm_strings_and_modules() {
    assert_parity(
        r#"
public fn main(): void {
    let s: string = "Hello, world";
    io::println(s);
    io::println(Integer.toString(String.length(s)));
    io::println(String.toUpperCase("abc"));
    io::println(String.toLowerCase("ABC"));
    io::println(String.substring("abcdef", 1, 4));
    io::println(Boolean.toString(String.startsWith("hello", "he")));
    io::println(Boolean.toString(String.endsWith("hello", "lo")));
    io::println(Integer.toString(Integer.parseInt("123")));
    io::println(Double.toString(Double.parseDouble("2.5")));
}
"#,
    );
}

#[test]
fn wasm_math() {
    assert_parity(
        r#"
public fn main(): void {
    io::println(Double.toString(MathAdvanced.sqrt(9.0)));
    io::println(Double.toString(MathAdvanced.pow(2.0, 10.0)));
    io::println(Double.toString(Math.abs(-3.5)));
    io::println(Double.toString(Math.floor(2.9)));
    io::println(Double.toString(Math.ceil(2.1)));
    io::println(Double.toString(Math.round(2.5)));
    io::println(Double.toString(Math.sin(0.0)));
}
"#,
    );
}

#[test]
fn wasm_classes() {
    assert_parity(
        r#"
class Circle {
    public double radius;
    public fn init(r: double) { this.radius = r; }
    public fn area(): double { return 3.14159 * this.radius * this.radius; }
    public fn describe(): void { io::println("Circle radius " + Double.toString(this.radius)); }
}
public fn main(): void {
    let c: Circle = new Circle(2.0);
    c.describe();
    io::println("Area: " + Double.toString(c.area()));
    c.radius = 3.0;
    io::println(Double.toString(c.radius));
}
"#,
    );
}

#[test]
fn wasm_switch_on_literals() {
    assert_parity(
        r#"
fn describe(n: int): string {
    switch (n) {
        case 0 => return "zero";
        case 1 => return "one";
        case _ => return "other";
    }
    return "other";
}
public fn main(): void {
    io::println(describe(0));
    io::println(describe(1));
    io::println(describe(9));
}
"#,
    );
}
