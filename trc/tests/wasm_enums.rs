//! Parity tests for enums, Result and constructor-pattern switches.

mod wasm_common;

use wasm_common::assert_parity;

#[test]
fn enum_unit_variants() {
    assert_parity(
        r#"
enum Color { Red, Green, Blue }
fn label(c: Color): string {
    switch (c) {
        case Red() => return "red";
        case Green() => return "green";
        case Blue() => return "blue";
        case _ => return "other";
    }
    return "?";
}
public fn main(): void {
    io::println(label(Red()));
    io::println(label(Green()));
    io::println(label(Blue()));
    let c = Blue();
    io::println(c.toString());
    io::println(Boolean.toString(c is Color));
}
"#,
    );
}

#[test]
fn enum_payload_switch() {
    assert_parity(
        r#"
enum Shape { Circle(double), Rect(double, double) }
fn area(s: Shape): double {
    switch (s) {
        case Circle(r) => return 3.0 * r * r;
        case Rect(w, h) => return w * h;
        case _ => return 0.0;
    }
    return 0.0;
}
public fn main(): void {
    let c = Circle(2.0);
    io::println(c.toString());
    io::println(Double.toString(area(c)));
    io::println(Double.toString(area(Rect(3.0, 4.0))));
}
"#,
    );
}

#[test]
fn result_switch() {
    assert_parity(
        r#"
fn div(a: int, b: int): Result<int, string> {
    if (b == 0) { return Err("division by zero"); }
    return Ok(a / b);
}
public fn main(): void {
    let r = div(10, 2);
    switch (r) {
        case Ok(v) => io::println("Ok value: " + Integer.toString(v));
        case Err(e) => io::println("Error: " + e);
        case _ => {}
    }
    let r2 = div(10, 0);
    switch (r2) {
        case Ok(v) => io::println("Ok value: " + Integer.toString(v));
        case Err(e) => io::println("Error: " + e);
        case _ => {}
    }
}
"#,
    );
}

#[test]
fn result_unwrap_and_propagation() {
    assert_parity(
        r#"
fn f(): Result<int, string> { return Ok(7); }
fn g(): Result<int, string> { let v = f()?; return Ok(v + 1); }
public fn main(): void {
    let r = g();
    switch (r) {
        case Ok(v) => io::println(Integer.toString(v));
        case _ => {}
    }
}
"#,
    );
}
