use trc::lexer;
use trc::parser;
use trc::bytecode;

fn run_source(source: &str) -> Result<Vec<String>, String> {
    let tokens = lexer::tokenize(source).map_err(|e| format!("tokenize: {}", e))?;
    let ast = parser::parse(tokens).map_err(|e| format!("parse: {}", e))?;
    let mut compiler = bytecode::Compiler::new();
    let compiled = compiler.compile(&ast)?;
    let mut vm = bytecode::Vm::new();
    vm.load_program(compiled);
    vm.run()?;
    Ok(vm.output)
}

fn assert_output(output: &[String], expected: &str) {
    let actual: String = output.join("\n");
    assert_eq!(actual, expected.trim_end().replace("\r\n", "\n"));
}

#[test]
fn integer_to_string() {
    let src = r#"
public fn main(): void {
    io::println(Integer.toString(42));
    io::println(Integer.toString(0));
    io::println(Integer.toString(-1));
    io::println(Integer.toString(2147483647));
    io::println(Integer.toString(-2147483648));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "42\n0\n-1\n2147483647\n-2147483648");
}

#[test]
fn integer_parse_or() {
    let src = r#"
public fn main(): void {
    io::println(Integer.toString(Integer.parseOr("42", 0)));
    io::println(Integer.toString(Integer.parseOr("-100", 0)));
    io::println(Integer.toString(Integer.parseOr("abc", 99)));
    io::println(Integer.toString(Integer.parseOr("", 7)));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "42\n-100\n99\n7");
}

#[test]
fn integer_arithmetic() {
    let src = r#"
public fn main(): void {
    io::println(Integer.toString(10 + 3));
    io::println(Integer.toString(10 - 3));
    io::println(Integer.toString(10 * 3));
    io::println(Integer.toString(10 / 3));
    io::println(Integer.toString(10 % 3));
    io::println(Integer.toString(-10 + 3));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "13\n7\n30\n3\n1\n-7");
}

#[test]
fn integer_comparison() {
    let src = r#"
public fn main(): void {
    io::println(Boolean.toString(5 < 10));
    io::println(Boolean.toString(5 > 10));
    io::println(Boolean.toString(5 == 5));
    io::println(Boolean.toString(5 != 5));
    io::println(Boolean.toString(5 <= 5));
    io::println(Boolean.toString(5 >= 6));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "true\nfalse\ntrue\nfalse\ntrue\nfalse");
}

#[test]
fn integer_bitwise_and() {
    let src = r#"
public fn main(): void {
    io::println(Integer.toString(12 & 10));
    io::println(Integer.toString(255 & 0xF0));
    io::println(Integer.toString(0 & 0xFF));
    io::println(Integer.toString(-1 & 0));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "8\n240\n0\n0");
}

#[test]
fn integer_bitwise_or() {
    let src = r#"
public fn main(): void {
    io::println(Integer.toString(12 | 10));
    io::println(Integer.toString(0xF0 | 0x0F));
    io::println(Integer.toString(0 | 0));
    io::println(Integer.toString(128 | 1));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "14\n255\n0\n129");
}

#[test]
fn integer_bitwise_xor() {
    let src = r#"
public fn main(): void {
    io::println(Integer.toString(12 ^ 10));
    io::println(Integer.toString(255 ^ 255));
    io::println(Integer.toString(0 ^ 0));
    io::println(Integer.toString(0xFF ^ 0x0F));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "6\n0\n0\n240");
}

#[test]
fn integer_bitwise_not() {
    let src = r#"
public fn main(): void {
    io::println(Integer.toString(~0));
    io::println(Integer.toString(~(-1)));
    io::println(Integer.toString(~1));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "-1\n0\n-2");
}

#[test]
fn integer_shift_left() {
    let src = r#"
public fn main(): void {
    io::println(Integer.toString(1 << 4));
    io::println(Integer.toString(0 << 10));
    io::println(Integer.toString(255 << 1));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "16\n0\n510");
}

#[test]
fn integer_shift_right() {
    let src = r#"
public fn main(): void {
    io::println(Integer.toString(256 >> 4));
    io::println(Integer.toString(0 >> 10));
    io::println(Integer.toString(-8 >> 1));
    io::println(Integer.toString(1024 >> 3));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "16\n0\n-4\n128");
}

#[test]
fn integer_unsigned_right_shift() {
    let src = r#"
public fn main(): void {
    io::println(Integer.toString(-1 >>> 1));
    io::println(Integer.toString(256 >>> 4));
    io::println(Integer.toString(0 >>> 10));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "9223372036854775807\n16\n0");
}

#[test]
fn integer_max_min() {
    let src = r#"
public fn max(a: int, b: int): int {
    if (a > b) { return a; }
    return b;
}
public fn min(a: int, b: int): int {
    if (a < b) { return a; }
    return b;
}
public fn main(): void {
    io::println(Integer.toString(max(10, 20)));
    io::println(Integer.toString(max(-5, -10)));
    io::println(Integer.toString(min(10, 20)));
    io::println(Integer.toString(min(-5, -10)));
    io::println(Integer.toString(max(0, 0)));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "20\n-5\n10\n-10\n0");
}

#[test]
fn integer_signum() {
    let src = r#"
public fn signum(n: int): int {
    if (n < 0) { return -1; }
    if (n > 0) { return 1; }
    return 0;
}
public fn main(): void {
    io::println(Integer.toString(signum(42)));
    io::println(Integer.toString(signum(-42)));
    io::println(Integer.toString(signum(0)));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "1\n-1\n0");
}

#[test]
fn integer_to_hex_string() {
    let src = r#"
public fn hexDigit(d: int): string {
    if (d < 10) { return Integer.toString(d); }
    if (d == 10) { return "a"; }
    if (d == 11) { return "b"; }
    if (d == 12) { return "c"; }
    if (d == 13) { return "d"; }
    if (d == 14) { return "e"; }
    return "f";
}
public fn toHexString(n: int): string {
    if (n == 0) { return "0"; }
    var result: string = "";
    var val: int = n;
    while (val != 0) {
        let digit: int = val & 0xF;
        result = hexDigit(digit) + (result as string);
        val = val >> 4;
    }
    return result;
}
public fn main(): void {
    io::println(toHexString(255));
    io::println(toHexString(16));
    io::println(toHexString(0));
    io::println(toHexString(256));
    io::println(toHexString(4095));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "ff\n10\n0\n100\nfff");
}

#[test]
fn integer_to_binary_string() {
    let src = r#"
public fn toBinaryString(n: int): string {
    if (n == 0) { return "0"; }
    var result: string = "";
    var val: int = n;
    var bits: int = 0;
    while (val != 0 && bits < 32) {
        if ((val & 1) != 0) {
            result = "1" + (result as string);
        } else {
            result = "0" + (result as string);
        }
        val = val >> 1;
        bits = bits + 1;
    }
    return result;
}
public fn main(): void {
    io::println(toBinaryString(0));
    io::println(toBinaryString(1));
    io::println(toBinaryString(2));
    io::println(toBinaryString(10));
    io::println(toBinaryString(255));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "0\n1\n10\n1010\n11111111");
}

#[test]
fn integer_compare() {
    let src = r#"
public fn compare(a: int, b: int): int {
    if (a < b) { return -1; }
    if (a > b) { return 1; }
    return 0;
}
public fn main(): void {
    io::println(Integer.toString(compare(5, 10)));
    io::println(Integer.toString(compare(10, 5)));
    io::println(Integer.toString(compare(5, 5)));
    io::println(Integer.toString(compare(-5, -10)));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "-1\n1\n0\n1");
}

#[test]
fn integer_edge_cases() {
    let src = r#"
public fn main(): void {
    io::println(Integer.toString(0 + 0));
    io::println(Integer.toString(0 - 0));
    io::println(Integer.toString(1 * 0));
    io::println(Integer.toString(0 / 1));
    io::println(Integer.toString(-0));
    io::println(Boolean.toString(0 == 0));
    io::println(Boolean.toString(-1 < 0));
    io::println(Boolean.toString(0 == -0));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "0\n0\n0\n0\n0\ntrue\ntrue\ntrue");
}
