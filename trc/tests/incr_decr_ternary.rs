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

// -----------------------------------------------------------------------
// Ternary operator integration tests
// -----------------------------------------------------------------------
#[test]
fn test_ternary_true_branch() {
    let src = r#"
fn main(): void {
    let x = true ? 10 : 20;
    io::println(x);
}
"#;
    let output = run_source(src).expect("execution should succeed");
    assert_eq!(output, vec!["10"]);
}

#[test]
fn test_ternary_false_branch() {
    let src = r#"
fn main(): void {
    let x = false ? 10 : 20;
    io::println(x);
}
"#;
    let output = run_source(src).expect("execution should succeed");
    assert_eq!(output, vec!["20"]);
}

#[test]
fn test_ternary_with_comparison() {
    let src = r#"
fn main(): void {
    let a = 5;
    let b = 10;
    let max = a > b ? a : b;
    io::println(max);
}
"#;
    let output = run_source(src).expect("execution should succeed");
    assert_eq!(output, vec!["10"]);
}

#[test]
fn test_ternary_nested() {
    let src = r#"
fn main(): void {
    let x = 1;
    let result = x > 0 ? 1 : x < 0 ? -1 : 0;
    io::println(result);
}
"#;
    let output = run_source(src).expect("execution should succeed");
    assert_eq!(output, vec!["1"]);
}

#[test]
fn test_ternary_in_assignment() {
    let src = r#"
fn main(): void {
    let flag = true;
    var x = 0;
    x = flag ? 42 : 0;
    io::println(x);
}
"#;
    let output = run_source(src).expect("execution should succeed");
    assert_eq!(output, vec!["42"]);
}

// -----------------------------------------------------------------------
// Increment/decrement operator integration tests
// -----------------------------------------------------------------------
#[test]
fn test_prefix_increment() {
    let src = r#"
fn main(): void {
    var x = 5;
    ++x;
    io::println(x);
}
"#;
    let output = run_source(src).expect("execution should succeed");
    assert_eq!(output, vec!["6"]);
}

#[test]
fn test_prefix_decrement() {
    let src = r#"
fn main(): void {
    var x = 5;
    --x;
    io::println(x);
}
"#;
    let output = run_source(src).expect("execution should succeed");
    assert_eq!(output, vec!["4"]);
}

#[test]
fn test_postfix_increment() {
    let src = r#"
fn main(): void {
    var x = 5;
    x++;
    io::println(x);
}
"#;
    let output = run_source(src).expect("execution should succeed");
    assert_eq!(output, vec!["6"]);
}

#[test]
fn test_postfix_decrement() {
    let src = r#"
fn main(): void {
    var x = 5;
    x--;
    io::println(x);
}
"#;
    let output = run_source(src).expect("execution should succeed");
    assert_eq!(output, vec!["4"]);
}

#[test]
fn test_increment_in_loop() {
    let src = r#"
fn main(): void {
    var i = 0;
    while (i < 3) {
        io::println(i);
        i++;
    }
}
"#;
    let output = run_source(src).expect("execution should succeed");
    assert_eq!(output, vec!["0", "1", "2"]);
}
