//! Parity tests for error handling: throw/try/catch and `?`.

mod wasm_common;

use wasm_common::{assert_compile_error, assert_parity};

#[test]
fn unsupported_constructs_report_explicit_errors() {
    assert_compile_error(
        r#"
public fn main(): void {
    while let x = 42 { io::println("x"); }
}
"#,
        "wasm: while let requires a Result-valued expression",
    );
    assert_compile_error(
        r#"
public fn main(): void {
    throw "outside try";
}
"#,
        "wasm: throw outside try/catch",
    );
}

#[test]
fn throw_and_catch_string() {
    assert_parity(
        r#"
public fn main(): void {
    try {
        io::println("before");
        throw "boom";
        io::println("unreachable");
    } catch (e: string) {
        io::println("caught: " + e);
    }
    io::println("after");
}
"#,
    );
}

#[test]
fn try_without_throw() {
    assert_parity(
        r#"
public fn main(): void {
    try {
        io::println("ok path");
    } catch (e: string) {
        io::println("should not happen");
    }
    io::println("done");
}
"#,
    );
}

#[test]
fn nested_try_catch() {
    assert_parity(
        r#"
public fn main(): void {
    try {
        try {
            throw "inner";
        } catch (e: string) {
            io::println("inner caught: " + e);
        }
        throw "outer";
    } catch (e: string) {
        io::println("outer caught: " + e);
    }
}
"#,
    );
}

#[test]
fn result_propagation_short_circuit() {
    assert_parity(
        r#"
fn fail(): Result<int, string> { return Err("bad"); }
fn g(): Result<int, string> {
    let v = fail()?;
    return Ok(v + 1);
}
public fn main(): void {
    let r = g();
    switch (r) {
        case Ok(v) => io::println("ok " + Integer.toString(v));
        case Err(e) => io::println("err " + e);
        case _ => {}
    }
}
"#,
    );
}
