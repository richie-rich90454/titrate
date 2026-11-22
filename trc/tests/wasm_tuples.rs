//! Parity tests for tuples and tuple destructuring.

mod wasm_common;

use wasm_common::{assert_parity, assert_wasm_output};

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

#[test]
fn heterogeneous_tuple_index_uses_element_type() {
    // The bytecode VM rejects indexing a tuple; WASM selects the accessor by
    // the element at the requested index, as LLVM does.
    assert_wasm_output(
        r#"
public fn main(): void {
    let t = (1, "a", 2.5, true);
    io::println(Integer.toString(t[0]));
    io::println(t[1]);
    io::println(Double.toString(t[2]));
    io::println(Boolean.toString(t[3]));
}
"#,
        &["1", "a", "2.5", "true"],
    );
}
