//! Parity tests for signature-aware method dispatch: overloads and interface
//! default methods.

mod wasm_common;

use wasm_common::assert_parity;

#[test]
fn overloads_by_arity() {
    assert_parity(
        r#"
interface Adder {
    fn add(a: int): int;
    fn add(a: int, b: int): int;
}
class Calc implements Adder {
    public fn init() {}
    public fn add(a: int): int { return a + 1; }
    public fn add(a: int, b: int): int { return a + b; }
}
public fn main(): void {
    let c: Adder = new Calc();
    io::println(Integer.toString(c.add(5)));
    io::println(Integer.toString(c.add(5, 7)));
}
"#,
    );
}

#[test]
fn overloads_by_param_type() {
    assert_parity(
        r#"
interface Showable {
    fn show(x: int): string;
    fn show(x: string): string;
}
class Printer implements Showable {
    public fn init() {}
    public fn show(x: int): string { return "int:" + Integer.toString(x); }
    public fn show(x: string): string { return "str:" + x; }
}
public fn main(): void {
    let p = new Printer();
    io::println(p.show(42));
    io::println(p.show("hi"));
    let q: Showable = new Printer();
    io::println(q.show(7));
    io::println(q.show("there"));
}
"#,
    );
}

#[test]
fn interface_default_without_override() {
    assert_parity(
        r#"
interface Greeter {
    fn greet(): string { return "default"; }
}
class Plain implements Greeter {
    public fn init() {}
}
public fn main(): void {
    let g: Greeter = new Plain();
    io::println(g.greet());
}
"#,
    );
}

#[test]
fn interface_default_override_wins() {
    assert_parity(
        r#"
interface Greeter {
    fn greet(): string { return "default"; }
}
class Plain implements Greeter {
    public fn init() {}
}
class Loud implements Greeter {
    public fn init() {}
    public fn greet(): string { return "override"; }
}
public fn main(): void {
    let a: Greeter = new Plain();
    let b: Greeter = new Loud();
    io::println(a.greet());
    io::println(b.greet());
}
"#,
    );
}
