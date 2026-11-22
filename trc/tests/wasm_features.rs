//! Parity tests for `with` bindings and `while let`.

mod wasm_common;

use wasm_common::{assert_compile_error, assert_parity, assert_parity_with_root};
use std::fs;

#[test]
fn unsupported_edges_report_explicit_errors() {
    assert_compile_error(
        r#"
public fn main(): void {
    while let x = 5 { io::println(Integer.toString(x)); }
}
"#,
        "wasm: while let requires a Result-valued expression",
    );
    assert_compile_error(
        r#"
public fn main(): void {
    let xs = new ArrayList<int>();
    xs.add(1);
    xs.reduce(fn(x: int): int => x);
}
"#,
        "wasm: ArrayList.reduce closure expects 2 parameter(s)",
    );
}

#[test]
fn with_binding_calls_close() {
    assert_parity(
        r#"
public class Res {
    public string name;
    public fn init(n: string) { this.name = n; }
    public fn close(): void { io::println("closing " + this.name); }
}
public fn main(): void {
    with (let r: Res = new Res("f")) {
        io::println("using " + r.name);
    }
    io::println("done");
}
"#,
    );
}

#[test]
fn with_simple_form() {
    assert_parity(
        r#"
public class Res {
    public fn init() {}
    public fn close(): void { io::println("closed"); }
}
public fn main(): void {
    with (new Res()) { io::println("body"); }
    io::println("after");
}
"#,
    );
}

#[test]
fn json_parse_stringify_objects() {
    assert_parity(
        r#"
public fn main(): void {
    let names = Json.parse("{\"a\": \"x\", \"b\": \"y\"}");
    io::println(Json.stringify(names));
    io::println(Integer.toString(names.size()));
    io::println(names.get("b"));

    let nums = Json.parse("{\"p\": 1, \"q\": 2}");
    io::println(Json.stringify(nums));
    io::println(Integer.toString(nums.get("p")));

    let flags = Json.parse("{\"t\": true, \"f\": false}");
    io::println(Json.stringify(flags));
    io::println(Boolean.toString(flags.get("t")));

    let arr = Json.parse("[1, 2, 3]");
    io::println(Json.stringify(arr));
    io::println(Integer.toString(arr.size()));
    io::println(Integer.toString(arr.get(0)));
}
"#,
    );
}

#[test]
fn json_scalar_roundtrip() {
    assert_parity(
        r#"
public fn main(): void {
    io::println(Json.stringify(Json.parse("42")));
    io::println(Json.stringify(Json.parse("2.5")));
    io::println(Json.stringify(Json.parse("true")));
    io::println(Json.stringify(Json.parse("\"hi\"")));
}
"#,
    );
}

#[test]
fn file_host_bindings() {
    let root = std::env::temp_dir().join(format!("wasm_file_test_{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let src = r#"
public fn main(): void {
    let p = "data.txt";
    File.write(p, "hello");
    io::println(File.readFile(p));
    File.append(p, " world");
    io::println(File.readFile(p));
    io::println(Boolean.toString(File.exists(p)));
    File.delete(p);
    io::println(Boolean.toString(File.exists(p)));
}
"#;
    assert_parity_with_root(src, root.to_str().unwrap());
    fs::remove_dir_all(&root).ok();
}

#[test]
fn while_let_int_result() {
    assert_parity(
        r#"
fn next(i: int): Result<int, string> {
    if (i < 3) { return Ok(i); }
    return Err("done");
}
public fn main(): void {
    var i: int = 0;
    while let v = next(i) {
        io::println(Integer.toString(v));
        i = i + 1;
    }
    io::println("finished");
}
"#,
    );
}

#[test]
fn generic_class_single_param() {
    assert_parity(
        r#"
public class Box<T> {
    public T value;
    public fn init(v: T) { this.value = v; }
    public fn get(): T { return this.value; }
    public fn set(v: T): void { this.value = v; }
}
public fn main(): void {
    let b = new Box<int>(42);
    io::println(Integer.toString(b.get()));
    b.set(100);
    io::println(Integer.toString(b.get()));
    let s = new Box<string>("hi");
    io::println(s.get());
}
"#,
    );
}

#[test]
fn generic_class_two_params_and_is() {
    assert_parity(
        r#"
public class Pair<A, B> {
    public A first;
    public B second;
    public fn init(a: A, b: B) { this.first = a; this.second = b; }
}
public fn main(): void {
    let p = new Pair<string, int>("age", 30);
    io::println(p.first);
    io::println(Integer.toString(p.second));
    if (p is Pair<string, int>) { io::println("yes"); } else { io::println("no"); }
}
"#,
    );
}

#[test]
fn generic_return_only_type_param() {
    assert_parity(
        r#"
fn newList<T>(): ArrayList<T> { return new ArrayList<T>(); }
fn show(xs: ArrayList<int>): void {
    io::println(Integer.toString(xs.size()));
    io::println(Integer.toString(xs.get(0)));
}
public fn main(): void {
    let xs = newList<int>();
    xs.add(7);
    show(xs);
}
"#,
    );
}

#[test]
fn while_let_string_result() {
    assert_parity(
        r#"
fn nxt(i: int): Result<string, int> {
    if (i < 2) { return Ok("s" + Integer.toString(i)); }
    return Err(9);
}
public fn main(): void {
    var i: int = 0;
    while let v = nxt(i) {
        io::println(v);
        i = i + 1;
    }
    io::println("end");
}
"#,
    );
}
