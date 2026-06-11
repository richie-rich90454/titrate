use trc::lexer;
use trc::parser;
use trc::bytecode;

/// Helper: compile and run a Titrate source snippet, returning output lines.
fn run_source(src: &str) -> Result<Vec<String>, String> {
    let tokens = lexer::tokenize(src).expect("tokenization should succeed");
    let ast = parser::parse(tokens).expect("parsing should succeed");

    let mut compiler = bytecode::Compiler::new();
    let compiled = compiler.compile(&ast)?;

    let mut vm = bytecode::Vm::new();
    vm.load_program(compiled);
    vm.run()?;
    Ok(vm.output)
}

#[test]
fn test_compound_assign_add_execution() {
    let src = r#"
fn main(): void {
    var x = 10;
    x += 5;
    io::println(x);
}
"#;
    let output = run_source(src).expect("execution should succeed");
    assert_eq!(output, vec!["15"]);
}

#[test]
fn test_compound_assign_sub_execution() {
    let src = r#"
fn main(): void {
    var x = 10;
    x -= 3;
    io::println(x);
}
"#;
    let output = run_source(src).expect("execution should succeed");
    assert_eq!(output, vec!["7"]);
}

#[test]
fn test_compound_assign_mul_execution() {
    let src = r#"
fn main(): void {
    var x = 6;
    x *= 7;
    io::println(x);
}
"#;
    let output = run_source(src).expect("execution should succeed");
    assert_eq!(output, vec!["42"]);
}

#[test]
fn test_compound_assign_div_execution() {
    let src = r#"
fn main(): void {
    var x = 42;
    x /= 6;
    io::println(x);
}
"#;
    let output = run_source(src).expect("execution should succeed");
    assert_eq!(output, vec!["7"]);
}

#[test]
fn test_compound_assign_mod_execution() {
    let src = r#"
fn main(): void {
    var x = 17;
    x %= 5;
    io::println(x);
}
"#;
    let output = run_source(src).expect("execution should succeed");
    assert_eq!(output, vec!["2"]);
}

#[test]
fn test_compound_assign_bitand_execution() {
    let src = r#"
fn main(): void {
    var x = 15;
    x &= 6;
    io::println(x);
}
"#;
    let output = run_source(src).expect("execution should succeed");
    assert_eq!(output, vec!["6"]);
}

#[test]
fn test_compound_assign_bitor_execution() {
    let src = r#"
fn main(): void {
    var x = 10;
    x |= 5;
    io::println(x);
}
"#;
    let output = run_source(src).expect("execution should succeed");
    assert_eq!(output, vec!["15"]);
}

#[test]
fn test_compound_assign_bitxor_execution() {
    let src = r#"
fn main(): void {
    var x = 15;
    x ^= 9;
    io::println(x);
}
"#;
    let output = run_source(src).expect("execution should succeed");
    assert_eq!(output, vec!["6"]);
}

#[test]
fn test_compound_assign_left_shift_execution() {
    let src = r#"
fn main(): void {
    var x = 3;
    x <<= 2;
    io::println(x);
}
"#;
    let output = run_source(src).expect("execution should succeed");
    assert_eq!(output, vec!["12"]);
}

#[test]
fn test_compound_assign_right_shift_execution() {
    let src = r#"
fn main(): void {
    var x = 12;
    x >>= 2;
    io::println(x);
}
"#;
    let output = run_source(src).expect("execution should succeed");
    assert_eq!(output, vec!["3"]);
}

#[test]
fn test_compound_assign_chained() {
    let src = r#"
fn main(): void {
    var x = 10;
    x += 5;
    x *= 2;
    x -= 5;
    io::println(x);
}
"#;
    let output = run_source(src).expect("execution should succeed");
    // 10 + 5 = 15, 15 * 2 = 30, 30 - 5 = 25
    assert_eq!(output, vec!["25"]);
}

#[test]
fn test_compound_assign_in_loop() {
    let src = r#"
fn main(): void {
    var sum = 0;
    for (let i = 0; i < 5; i = i + 1) {
        sum += i;
    }
    io::println(sum);
}
"#;
    let output = run_source(src).expect("execution should succeed");
    // 0 + 1 + 2 + 3 + 4 = 10
    assert_eq!(output, vec!["10"]);
}
