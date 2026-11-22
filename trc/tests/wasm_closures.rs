//! Parity tests for closures: definition, invocation and capture.

mod wasm_common;

use wasm_common::assert_parity;

#[test]
fn closure_arrow_basic() {
    assert_parity(
        r#"
public fn main(): void {
    let f = fn(x: int): int => x + 1;
    io::println(Integer.toString(f(5)));
    io::println(Integer.toString(f(41)));
}
"#,
    );
}

#[test]
fn closure_block_body() {
    assert_parity(
        r#"
public fn main(): void {
    let sq = fn(x: int): int { return x * x; };
    io::println(Integer.toString(sq(7)));
}
"#,
    );
}

#[test]
fn closure_capture() {
    assert_parity(
        r#"
public fn main(): void {
    let n = 10;
    let f = fn(x: int): int => x + n;
    io::println(Integer.toString(f(5)));
}
"#,
    );
}

#[test]
fn closure_two_params() {
    assert_parity(
        r#"
public fn main(): void {
    let add = fn(a: int, b: int): int => a + b;
    io::println(Integer.toString(add(20, 22)));
}
"#,
    );
}
