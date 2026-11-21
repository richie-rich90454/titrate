//! Parity tests for closure-valued collection higher-order methods.

mod wasm_common;

use wasm_common::assert_parity;

#[test]
fn arraylist_for_each() {
    assert_parity(
        r#"
public fn main(): void {
    let xs = new ArrayList<int>();
    xs.add(3); xs.add(1); xs.add(2);
    xs.forEach(fn(x: int): void { io::println("x=" + Integer.toString(x)); });
}
"#,
    );
}

#[test]
fn arraylist_map() {
    assert_parity(
        r#"
public fn main(): void {
    let xs = new ArrayList<int>();
    xs.add(3); xs.add(1); xs.add(2);
    let ys = xs.map(fn(x: int): int => x * 2);
    io::println(ys.toString());
}
"#,
    );
}

#[test]
fn arraylist_filter() {
    assert_parity(
        r#"
public fn main(): void {
    let xs = new ArrayList<int>();
    xs.add(1); xs.add(5); xs.add(10); xs.add(3);
    let ys = xs.filter(fn(x: int): bool => x > 3);
    io::println(ys.toString());
}
"#,
    );
}

#[test]
fn arraylist_reduce() {
    assert_parity(
        r#"
public fn main(): void {
    let xs = new ArrayList<int>();
    xs.add(3); xs.add(1); xs.add(2);
    let total = xs.reduce(fn(a: int, b: int): int => a + b);
    io::println(Integer.toString(total));
}
"#,
    );
}

#[test]
fn arraylist_hof_captures() {
    assert_parity(
        r#"
public fn main(): void {
    let xs = new ArrayList<int>();
    xs.add(1); xs.add(5); xs.add(10);
    let k = 4;
    let big = xs.filter(fn(x: int): bool => x > k);
    io::println(big.toString());
    let added = xs.map(fn(x: int): int => x + k);
    io::println(added.toString());
    let sum = added.reduce(fn(a: int, b: int): int => a + b);
    io::println(Integer.toString(sum));
}
"#,
    );
}

#[test]
fn arraylist_hof_strings() {
    assert_parity(
        r#"
public fn main(): void {
    let xs = new ArrayList<string>();
    xs.add("a"); xs.add("bb"); xs.add("ccc");
    let lens = xs.map(fn(s: string): int => String.length(s));
    io::println(lens.toString());
    xs.forEach(fn(s: string): void { io::println(s); });
}
"#,
    );
}

#[test]
fn empty_reduce_reports_explicit_error() {
    let src = r#"
public fn main(): void {
    let xs = new ArrayList<int>();
    let total = xs.reduce(fn(a: int, b: int): int => a + b);
    io::println(Integer.toString(total));
}
"#;
    let bytes = wasm_common::compile_wasm(src);
    let result = std::panic::catch_unwind(|| wasm_common::run_wasm(&bytes));
    let panic = result.expect_err("empty reduce must not silently produce a value");
    let message = panic
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| panic.downcast_ref::<&str>().map(|s| (*s).to_string()))
        .unwrap_or_default();
    assert!(
        message.contains("wasm: ArrayList.reduce on an empty list is unsupported"),
        "unexpected message: {}",
        message
    );
}

#[test]
fn arraylist_iterator_protocol() {
    assert_parity(
        r#"
public fn main(): void {
    let xs = new ArrayList<int>();
    xs.add(10); xs.add(20); xs.add(30);
    let it = xs.iterator();
    io::println(Boolean.toString(it.hasNext()));
    io::println(Integer.toString(it.next()));
    io::println(Integer.toString(it.next()));
    io::println(Integer.toString(it.count()));
    let rest = it.collect();
    io::println(rest.toString());
    io::println(Boolean.toString(it.hasNext()));
    it.reset();
    io::println(Boolean.toString(it.hasNext()));
}
"#,
    );
}
