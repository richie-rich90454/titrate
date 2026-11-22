//! Parity tests for host-backed printing, regex and OS read-only queries.

mod wasm_common;

use wasm_common::{assert_compile_error, assert_parity, assert_parity_with_root};

#[test]
fn remaining_unsupported_host_calls_report_explicit_errors() {
    assert_compile_error(
        r#"
public fn main(): void {
    let xs = new ArrayList<int>();
    xs.add(1);
    xs.forEach(42);
}
"#,
        "wasm: ArrayList.forEach requires a closure argument",
    );
}

#[test]
fn println_multifield_class_and_map() {
    assert_parity_with_root(
        r#"
import tt::util::HashMap;
public class Point {
    public int z;
    public string a;
    public int m;
    public fn init(zz: int, aa: string, mm: int) {
        this.z = zz; this.a = aa; this.m = mm;
    }
}
public fn main(): void {
    let p = new Point(3, "x", 9);
    io::println(p);
    let h = new HashMap<string, int>();
    h.put("b", 2);
    h.put("a", 1);
    io::println(h);
}
"#,
        "..",
    );
}

#[test]
fn println_collection_and_composites() {
    assert_parity(
        r#"
enum Color { Red, Green }
public fn main(): void {
    let xs = new ArrayList<int>();
    xs.add(7); xs.add(8);
    io::println(xs);
    let c = Green();
    io::println(c);
    let r: Result<int, string> = Ok(5);
    io::println(r);
    let t = (1, "a");
    io::println(t);
}
"#,
    );
}

#[test]
fn println_single_field_class() {
    assert_parity(
        r#"
class Box { public int value; public fn init(v: int) { this.value = v; } }
public fn main(): void {
    let b = new Box(42);
    io::println(b);
}
"#,
    );
}

#[test]
fn regex_match_find_replace() {
    assert_parity_with_root(
        r##"
import tt::regex::Regex;
public fn main(): void {
    io::println(Boolean.toString(Regex.match("a.c", "abc")));
    io::println(Boolean.toString(Regex.match("a.c", "ac")));
    io::println(Regex.find("a.c", "xxabcxx"));
    io::println(Regex.find("z+", "abc"));
    io::println(Regex.replace("[0-9]+", "a1b22", "#"));
}
"##,
        "../lib",
    );
}

#[test]
fn os_read_only_queries() {
    assert_parity_with_root(
        r#"
import tt::sys::Os;
public fn main(): void {
    io::println(Os.name());
    io::println(Os.arch());
    io::println(Os.family());
    io::println(Os.cpuCount() > 0 ? "cpu-count-positive" : "cpu-count-missing");
}
"#,
        "../lib",
    );
}
