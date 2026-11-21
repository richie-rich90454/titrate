//! Parity tests for collections and for-in loops.

mod wasm_common;

use wasm_common::assert_parity;

#[test]
fn arraylist_basic() {
    assert_parity(
        r#"
public fn main(): void {
    let list = new ArrayList<int>();
    list.add(3);
    list.add(1);
    list.add(2);
    io::println(list.toString());
    io::println(Integer.toString(list.size()));
    io::println(Integer.toString(list.get(1)));
    io::println(Boolean.toString(list.contains(2)));
    io::println(Integer.toString(list.indexOf(1)));
    list.set(0, 9);
    io::println(list.toString());
    for (x in list) { io::println(Integer.toString(x)); }
}
"#,
    );
}

#[test]
fn arraylist_strings() {
    assert_parity(
        r#"
public fn main(): void {
    let xs = new ArrayList<string>();
    xs.add("alpha");
    xs.add("beta");
    io::println(xs.toString());
    for (s in xs) { io::println(s); }
    io::println(Boolean.toString(xs.contains("beta")));
}
"#,
    );
}

#[test]
fn arraylist_remove_pop_addall() {
    assert_parity(
        r#"
public fn main(): void {
    let xs = new ArrayList<int>();
    xs.add(1); xs.add(2); xs.add(3);
    io::println(Integer.toString(xs.remove(1)));
    io::println(xs.toString());
    io::println(Integer.toString(xs.removeAt(0)));
    io::println(Integer.toString(xs.pop()));
    io::println(Boolean.toString(xs.isEmpty()));
    let more = new ArrayList<int>();
    more.add(8); more.add(9);
    xs.addAll(more);
    io::println(xs.toString());
    io::println(Integer.toString(xs.lastIndexOf(9)));
    xs.reverse();
    io::println(xs.toString());
}
"#,
    );
}

#[test]
fn arraylist_max_min_strings() {
    assert_parity(
        r#"
public fn main(): void {
    let xs = new ArrayList<int>();
    xs.add(5); xs.add(2); xs.add(9);
    io::println(Integer.toString(xs.max()));
    io::println(Integer.toString(xs.min()));
    let ss = new ArrayList<string>();
    ss.add("a"); ss.add("b"); ss.add("a");
    io::println(Integer.toString(ss.lastIndexOf("a")));
}
"#,
    );
}

#[test]
fn hashmap_extended_methods() {
    assert_parity(
        r#"
public fn main(): void {
    let m = new HashMap<string, int>();
    m.put("a", 1);
    m.put("b", 2);
    io::println(Boolean.toString(m.containsValue(2)));
    io::println(Boolean.toString(m.containsValue(9)));
    io::println(Boolean.toString(m.isEmpty()));
    io::println(Integer.toString(m.remove("a")));
    io::println(Integer.toString(m.size()));
    m.clear();
    io::println(Boolean.toString(m.isEmpty()));
}
"#,
    );
}

#[test]
fn hashmap_basic() {
    assert_parity(
        r#"
public fn main(): void {
    let m = new HashMap<string, int>();
    m.put("a", 1);
    m.put("b", 2);
    m.put("a", 3);
    io::println(m.toString());
    io::println(Integer.toString(m.get("a")));
    io::println(Integer.toString(m.size()));
    io::println(Boolean.toString(m.containsKey("b")));
}
"#,
    );
}

#[test]
fn hashmap_for_in_keys() {
    assert_parity(
        r#"
public fn main(): void {
    let m = new HashMap<string, int>();
    m.put("x", 10);
    m.put("y", 20);
    for (k in m) { io::println(k); }
}
"#,
    );
}
