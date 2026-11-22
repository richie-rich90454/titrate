//! Parity tests for generic function specialization.

mod wasm_common;

use wasm_common::assert_parity;

#[test]
fn generic_identity_explicit() {
    assert_parity(
        r#"
fn identity<T>(x: T): T { return x; }
public fn main(): void {
    io::println(Integer.toString(identity<int>(42)));
}
"#,
    );
}

#[test]
fn generic_identity_inferred() {
    assert_parity(
        r#"
fn identity<T>(x: T): T { return x; }
public fn main(): void {
    io::println(identity("hello"));
}
"#,
    );
}

#[test]
fn generic_collection_element() {
    assert_parity(
        r#"
fn first<T>(xs: ArrayList<T>): T { return xs.get(0); }
public fn main(): void {
    let xs = new ArrayList<int>();
    xs.add(9);
    io::println(Integer.toString(first<int>(xs)));
}
"#,
    );
}
