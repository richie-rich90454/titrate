//! Parity tests for imported stdlib modules and ownership/operator features.

mod wasm_common;

use wasm_common::{assert_parity, assert_parity_with_root};

#[test]
fn imported_math_module() {
    assert_parity_with_root(
        r#"
import tt::math::MathAdvanced;
public fn main(): void {
    io::println(Double.toString(MathAdvanced.sqrt(16.0)));
    io::println(Double.toString(MathAdvanced.pow(2.0, 5.0)));
}
"#,
        "../lib",
    );
}

#[test]
fn imported_util_arraylist() {
    assert_parity_with_root(
        r#"
import tt::util::ArrayList;
public fn main(): void {
    let list = new ArrayList<int>();
    list.add(4);
    list.add(5);
    io::println(list.toString());
    for (x in list) { io::println(Integer.toString(x)); }
}
"#,
        "../lib",
    );
}

#[test]
fn operator_overloading() {
    assert_parity(
        r#"
class Vec {
    public int x;
    public fn init(x: int) { this.x = x; }
    public fn operator+(o: Vec): Vec { return new Vec(this.x + o.x); }
}
public fn main(): void {
    let a = new Vec(2);
    let b = new Vec(3);
    let c = a + b;
    io::println(Integer.toString(c.x));
}
"#,
    );
}
