//! Parity tests for tuples and tuple destructuring.

mod wasm_common;

use wasm_common::assert_parity;

#[test]
fn tuple_literal_and_display() {
    assert_parity(
        r#"
public fn main(): void {
    let t = (1, "hi", true);
    io::println(t.toString());
}
"#,
    );
}

#[test]
fn tuple_destructure() {
    assert_parity(
        r#"
public fn main(): void {
    let (a, b) = (1, "world");
    io::println(Integer.toString(a));
    io::println(b);
}
"#,
    );
}
