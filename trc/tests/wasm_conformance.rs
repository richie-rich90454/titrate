//! WebAssembly backend conformance tests.
//!
//! Each test compiles a Titrate source through the real lexer/parser/analyzer,
//! lowers it to a `.wasm` module, executes that module on the `wasmi`
//! interpreter with the shared host runtime, and compares its captured output
//! byte-for-byte with the bytecode VM's output for the same source.

mod wasm_common;

use wasm_common::{assert_parity, compile_wasm};

#[test]
fn wasm_emits_valid_magic() {
    let bytes = compile_wasm("public fn main(): void {}");
    assert_eq!(&bytes[..4], b"\0asm");
}

#[test]
fn wasm_dowhile_cfor_and_globals() {
    assert_parity(
        r#"
public fn main(): void {
    var i: int = 0;
    do {
        io::println("do " + Integer.toString(i));
        i = i + 1;
    } while (i < 3);
    for (let j = 0; j < 2; j++) {
        io::println("for " + Integer.toString(j));
    }
}
"#,
    );
}

#[test]
fn wasm_arithmetic_and_conversions() {
    assert_parity(
        r#"
public fn main(): void {
    io::println(Integer.toString(10 + 5));
    io::println(Integer.toString(20 - 8));
    io::println(Integer.toString(6 * 7));
    io::println(Integer.toString(100 / 3));
    io::println(Integer.toString(17 % 5));
    io::println(Double.toString(3.14159));
    io::println("The answer is " + Integer.toString(42));
    io::println(Double.toString(42 as double));
}
"#,
    );
}

#[test]
fn wasm_control_flow() {
    assert_parity(
        r#"
public fn main(): void {
    if (3 > 2) {
        io::println("greater");
    } else {
        io::println("not greater");
    }
    var i: int = 0;
    while (i < 3) {
        io::println(Integer.toString(i));
        i = i + 1;
    }
    var j: int = 0;
    while (true) {
        j = j + 1;
        if (j == 2) { continue; }
        if (j > 4) { break; }
        io::println("j=" + Integer.toString(j));
    }
    let label: string = 5 > 3 ? "yes" : "no";
    io::println(label);
}
"#,
    );
}

#[test]
fn wasm_functions_and_recursion() {
    assert_parity(
        r#"
fn add(a: int, b: int): int { return a + b; }
fn factorial(n: int): int {
    if (n <= 1) { return 1; }
    return n * factorial(n - 1);
}
public fn main(): void {
    io::println(Integer.toString(add(3, 4)));
    io::println(Integer.toString(factorial(6)));
}
"#,
    );
}

#[test]
fn wasm_operators_and_casts() {
    assert_parity(
        r#"
public fn main(): void {
    io::println(Integer.toString(5 & 3));
    io::println(Integer.toString(5 | 3));
    io::println(Integer.toString(5 ^ 3));
    io::println(Integer.toString(1 << 4));
    io::println(Integer.toString(16 >> 2));
    io::println(Boolean.toString(true && false));
    io::println(Boolean.toString(true || false));
    io::println(Boolean.toString(!true));
    io::println(Boolean.toString(5 > 3));
    io::println(Boolean.toString(3 >= 3));
    io::println(Boolean.toString(2 < 1));
    io::println(Boolean.toString(1 <= 1));
    io::println(Boolean.toString(5 == 5));
    io::println(Boolean.toString(4 != 5));
    io::println(Boolean.toString(-3 < 0));
}
"#,
    );
}

#[test]
fn wasm_strings_and_modules() {
    assert_parity(
        r#"
public fn main(): void {
    let s: string = "Hello, world";
    io::println(s);
    io::println(Integer.toString(String.length(s)));
    io::println(String.toUpperCase("abc"));
    io::println(String.toLowerCase("ABC"));
    io::println(String.substring("abcdef", 1, 4));
    io::println(Boolean.toString(String.startsWith("hello", "he")));
    io::println(Boolean.toString(String.endsWith("hello", "lo")));
    io::println(Integer.toString(Integer.parseInt("123")));
    io::println(Double.toString(Double.parseDouble("2.5")));
}
"#,
    );
}

#[test]
fn wasm_math() {
    assert_parity(
        r#"
public fn main(): void {
    io::println(Double.toString(MathAdvanced.sqrt(9.0)));
    io::println(Double.toString(MathAdvanced.pow(2.0, 10.0)));
    io::println(Double.toString(Math.abs(-3.5)));
    io::println(Double.toString(Math.floor(2.9)));
    io::println(Double.toString(Math.ceil(2.1)));
    io::println(Double.toString(Math.round(2.5)));
    io::println(Double.toString(Math.sin(0.0)));
}
"#,
    );
}

#[test]
fn wasm_classes() {
    assert_parity(
        r#"
class Circle {
    public double radius;
    public fn init(r: double) { this.radius = r; }
    public fn area(): double { return 3.14159 * this.radius * this.radius; }
    public fn describe(): void { io::println("Circle radius " + Double.toString(this.radius)); }
}
public fn main(): void {
    let c: Circle = new Circle(2.0);
    c.describe();
    io::println("Area: " + Double.toString(c.area()));
    c.radius = 3.0;
    io::println(Double.toString(c.radius));
}
"#,
    );
}

#[test]
fn wasm_switch_on_literals() {
    assert_parity(
        r#"
fn describe(n: int): string {
    switch (n) {
        case 0 => return "zero";
        case 1 => return "one";
        case _ => return "other";
    }
    return "other";
}
public fn main(): void {
    io::println(describe(0));
    io::println(describe(1));
    io::println(describe(9));
}
"#,
    );
}
