//! Parity tests for interfaces, inheritance and virtual dispatch.

mod wasm_common;

use wasm_common::assert_parity;

#[test]
fn interface_dispatch() {
    assert_parity(
        r#"
interface Shape { fn area(): double; }
class Circle implements Shape {
    public double r;
    public fn init(r: double) { this.r = r; }
    public fn area(): double { return 3.14 * this.r * this.r; }
}
public fn main(): void {
    let s: Shape = new Circle(2.0);
    io::println(Double.toString(s.area()));
}
"#,
    );
}

#[test]
fn inheritance_and_super() {
    assert_parity(
        r#"
class Animal {
    public string name;
    public fn init(n: string) { this.name = n; }
    public fn speak(): string { return "..."; }
}
class Dog extends Animal {
    public fn init(n: string) { super.init(n); }
    public fn speak(): string { return "woof"; }
}
public fn main(): void {
    let d = new Dog("Rex");
    io::println(d.name);
    io::println(d.speak());
}
"#,
    );
}

#[test]
fn inherited_method_call() {
    assert_parity(
        r#"
class Base {
    public int x;
    public fn init(x: int) { this.x = x; }
    public fn value(): int { return this.x; }
}
class Child extends Base {
    public fn init(x: int) { super.init(x); }
}
public fn main(): void {
    let c = new Child(7);
    io::println(Integer.toString(c.value()));
}
"#,
    );
}
