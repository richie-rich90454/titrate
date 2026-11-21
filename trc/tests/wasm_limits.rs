//! Parity tests for limitations closed in the WASM host backend.
mod wasm_common;

use wasm_common::{assert_parity, assert_parity_with_root};
use std::fs;

#[test]
fn hashmap_all_primitive_keys() {
    assert_parity(
        r#"
public fn main(): void {
    let b = new HashMap<bool, int>();
    b.put(true, 1);
    b.put(false, 2);
    io::println(b.toString());
    io::println(Integer.toString(b.get(true)));
    io::println(Boolean.toString(b.containsKey(false)));

    let f = new HashMap<double, int>();
    f.put(1.5, 10);
    f.put(2.5, 20);
    io::println(f.toString());
    io::println(Integer.toString(f.get(1.5)));
    io::println(Boolean.toString(f.containsKey(2.5)));

    let c = new HashMap<char, int>();
    c.put('a', 1);
    c.put('b', 2);
    io::println(c.toString());
    io::println(Integer.toString(c.get('a')));
    io::println(Boolean.toString(c.containsKey('b')));
}
"#,
    );
}

#[test]
fn json_nested_mixed_and_nonliteral() {
    assert_parity(
        r#"
public fn main(): void {
    io::println(Json.stringify(Json.parse("{\"a\": [1, 2], \"b\": {\"x\": true}}")));
    io::println(Json.stringify(Json.parse("[1, \"two\", true]")));
    let s = "[1, 2, 3]";
    io::println(Json.stringify(Json.parse(s)));
    io::println(Json.stringify(Json.parse("null")));
}
"#,
    );
}

#[test]
fn file_extended_operations() {
    let root = std::env::temp_dir().join(format!("wasm_file_ext_{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let src = r#"
public fn main(): void {
    let a = "a.txt";
    let b = "b.txt";
    let c = "c.txt";
    File.write(a, "abc\ndef\n");
    io::println(Integer.toString(File.size(a)));
    io::println(File.readLines(a).toString());
    File.copy(a, b);
    io::println(Boolean.toString(File.exists(b)));
    io::println(Boolean.toString(File.rename(b, c)));
    io::println(Boolean.toString(File.exists(c)));
    File.delete(a);
    File.delete(c);
}
"#;
    assert_parity_with_root(src, root.to_str().unwrap());
    fs::remove_dir_all(&root).ok();
}

#[test]
fn regex_extended_natives() {
    assert_parity_with_root(
        r#"
import tt::regex::Regex;
public fn main(): void {
    io::println(Regex.findGroups("(a)(b)", "ab"));
    io::println(Integer.toString(Regex.groupCount("(a)(b)(c)")));
    io::println(Regex.subN("a", "X", "banana", 2));
    io::println(Regex.split(",", "a,b,c").toString());
}
"#,
        "../lib",
    );
}

#[test]
fn iterator_closure_methods() {
    assert_parity(
        r#"
public fn main(): void {
    let xs = new ArrayList<int>();
    xs.add(1); xs.add(2); xs.add(3);
    let a = xs.iterator();
    let ys = a.map(fn(x: int): int => x * 2).collect();
    io::println(ys.toString());

    let b = xs.iterator();
    let zs = b.filter(fn(x: int): bool => x > 1).collect();
    io::println(zs.toString());

    let c = xs.iterator();
    c.forEach(fn(x: int): void { io::println("v=" + Integer.toString(x)); });

    let d = xs.iterator();
    let sum = d.reduce(0, fn(a: int, b: int): int => a + b);
    io::println(Integer.toString(sum));
}
"#,
    );
}

#[test]
fn iterator_non_scalar_elements() {
    assert_parity(
        r#"
public class Box {
    public int value;
    public fn init(v: int) { this.value = v; }
}
public fn main(): void {
    let xs = new ArrayList<Box>();
    xs.add(new Box(3));
    xs.add(new Box(4));
    let it = xs.iterator();
    io::println(Integer.toString(it.next().value));
    io::println(Boolean.toString(it.hasNext()));
    io::println(Integer.toString(it.next().value));
}
"#,
    );
}

#[test]
fn os_mutating_and_random_natives_emulated() {
    assert_parity_with_root(
        r#"
import tt::sys::Os;
public fn main(): void {
    io::println(Os.getcwd());
    io::println(Os.getenv("HOME"));
    io::println(Integer.toString(String.length(Os.urandom(4))));
}
"#,
        "../lib",
    );
}
