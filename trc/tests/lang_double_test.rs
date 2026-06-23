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
fn double_to_string() {
    let src = r#"
public fn main(): void {
    io::println(Double.toString(3.14));
    io::println(Double.toString(0.0));
    io::println(Double.toString(-1.5));
    io::println(Double.toString(100.0));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "3.14\n0\n-1.5\n100");
}

#[test]
fn double_parse() {
    let src = r#"
public fn main(): void {
    io::println(Double.toString(Double_parseDouble("3.14")));
    io::println(Double.toString(Double_parseDouble("0")));
    io::println(Double.toString(Double_parseDouble("-2.5")));
    io::println(Double.toString(Double_parseDouble("1e10")));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "3.14\n0\n-2.5\n10000000000");
}

#[test]
fn double_arithmetic() {
    let src = r#"
public fn main(): void {
    io::println(Double.toString(1.5 + 2.5));
    io::println(Double.toString(10.0 - 3.0));
    io::println(Double.toString(2.5 * 4.0));
    io::println(Double.toString(10.0 / 4.0));
    io::println(Double.toString(10.0 % 3.0));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "4\n7\n10\n2.5\n1");
}

#[test]
fn double_comparison() {
    let src = r#"
public fn main(): void {
    io::println(Boolean.toString(3.14 < 3.15));
    io::println(Boolean.toString(3.14 > 3.15));
    io::println(Boolean.toString(3.14 == 3.14));
    io::println(Boolean.toString(3.14 != 3.15));
    io::println(Boolean.toString(3.0 <= 3.0));
    io::println(Boolean.toString(3.0 >= 3.1));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "true\nfalse\ntrue\ntrue\ntrue\nfalse");
}

#[test]
fn double_is_nan() {
    let src = r#"
public fn isNaN(d: double): bool {
    return d != d;
}
public fn main(): void {
    io::println(Boolean.toString(isNaN(Math_nan())));
    io::println(Boolean.toString(isNaN(3.14)));
    io::println(Boolean.toString(isNaN(0.0)));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "true\nfalse\nfalse");
}

#[test]
fn double_is_infinite() {
    let src = r#"
public fn isInfinite(d: double): bool {
    let inf: double = Math_inf();
    let negInf: double = Math_negInf();
    return d == inf || d == negInf;
}
public fn main(): void {
    io::println(Boolean.toString(isInfinite(Math_inf())));
    io::println(Boolean.toString(isInfinite(Math_negInf())));
    io::println(Boolean.toString(isInfinite(3.14)));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "true\ntrue\nfalse");
}

#[test]
fn double_max_min() {
    let src = r#"
public fn max(a: double, b: double): double {
    if (a > b) { return a; }
    return b;
}
public fn min(a: double, b: double): double {
    if (a < b) { return a; }
    return b;
}
public fn main(): void {
    io::println(Double.toString(max(3.14, 2.71)));
    io::println(Double.toString(max(-1.0, -2.0)));
    io::println(Double.toString(min(3.14, 2.71)));
    io::println(Double.toString(min(-1.0, -2.0)));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "3.14\n-1\n2.71\n-2");
}

#[test]
fn double_compare() {
    let src = r#"
public fn compare(a: double, b: double): int {
    if (a < b) { return -1; }
    if (a > b) { return 1; }
    return 0;
}
public fn main(): void {
    io::println(Integer.toString(compare(3.14, 3.15)));
    io::println(Integer.toString(compare(3.15, 3.14)));
    io::println(Integer.toString(compare(3.14, 3.14)));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "-1\n1\n0");
}

#[test]
fn double_edge_cases() {
    let src = r#"
public fn main(): void {
    io::println(Double.toString(0.0 + 0.0));
    io::println(Double.toString(0.0 - 0.0));
    io::println(Double.toString(1.0 * 0.0));
    io::println(Double.toString(Math_inf()));
    io::println(Double.toString(Math_negInf()));
    io::println(Boolean.toString(Math_inf() > 0.0));
    io::println(Boolean.toString(Math_negInf() < 0.0));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "0\n0\n0\ninf\n-inf\ntrue\ntrue");
}

#[test]
fn double_mixed_arithmetic() {
    let src = r#"
public fn main(): void {
    io::println(Double.toString(2.5 + (3 as double)));
    io::println(Double.toString((10 as double) - 2.5));
    io::println(Double.toString((3 as double) * 2.5));
    io::println(Double.toString(7.5 / (3 as double)));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "5.5\n7.5\n7.5\n2.5");
}

#[test]
fn double_rounding_behavior() {
    let src = r#"
public fn main(): void {
    io::println(Double.toString(10.0 / 3.0));
    io::println(Double.toString(1.0 / 3.0));
    io::println(Double.toString(2.0 / 3.0));
    io::println(Double.toString(7.0 / 2.0));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "3.3333333333333335\n0.3333333333333333\n0.6666666666666666\n3.5");
}
