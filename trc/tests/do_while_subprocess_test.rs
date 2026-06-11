use trc::lexer;
use trc::parser;
use trc::analyzer;
use trc::bytecode;
use trc::bytecode::Vm;
use trc::bytecode::value::Value;
use std::rc::Rc;

// ---------------------------------------------------------------------------
// Do-while loop tests
// ---------------------------------------------------------------------------

fn run_source(source: &str) -> Result<Vec<String>, String> {
    let tokens = lexer::tokenize(source)?;
    let ast = parser::parse(tokens)?;
    let typed_ast = analyzer::analyze(&ast).map_err(|errs| errs.join("\n"))?;
    bytecode::execute(&typed_ast).map_err(|e| e)
}

#[test]
fn test_do_while_executes_body_once() {
    // Body should execute at least once even though condition is false
    let source = r#"
fn main(): void {
    var x = 0;
    do {
        x = x + 1;
    } while (x < 0);
    io::println(x);
}
"#;
    let output = run_source(source).expect("execution should succeed");
    assert_eq!(output, vec!["1"], "do-while body should execute once even when condition is initially false");
}

#[test]
fn test_do_while_loops_while_true() {
    let source = r#"
fn main(): void {
    var x = 0;
    do {
        x = x + 1;
    } while (x < 3);
    io::println(x);
}
"#;
    let output = run_source(source).expect("execution should succeed");
    assert_eq!(output, vec!["3"], "do-while should loop until condition is false");
}

#[test]
fn test_do_while_vs_while_difference() {
    // while loop with false condition should never execute
    // do-while should execute once
    let source = r#"
fn main(): void {
    var a = 0;
    while (a < 0) {
        a = a + 1;
    }
    var b = 0;
    do {
        b = b + 1;
    } while (b < 0);
    io::println(a);
    io::println(b);
}
"#;
    let output = run_source(source).expect("execution should succeed");
    assert_eq!(output, vec!["0", "1"], "while should not execute, do-while should execute once");
}

#[test]
fn test_do_while_with_break() {
    let source = r#"
fn main(): void {
    var x = 0;
    do {
        x = x + 1;
        if (x == 5) {
            break;
        }
    } while (x < 100);
    io::println(x);
}
"#;
    let output = run_source(source).expect("execution should succeed");
    assert_eq!(output, vec!["5"], "do-while should break out of loop");
}

#[test]
fn test_do_while_with_continue() {
    let source = r#"
fn main(): void {
    var x = 0;
    var sum = 0;
    do {
        x = x + 1;
        if (x == 3) {
            continue;
        }
        sum = sum + x;
    } while (x < 5);
    io::println(sum);
}
"#;
    let output = run_source(source).expect("execution should succeed");
    // x goes 1,2,3,4,5; sum skips 3: 1+2+4+5 = 12
    assert_eq!(output, vec!["12"], "do-while should skip iteration with continue");
}

#[test]
fn test_do_while_continue_jumps_to_condition() {
    // This is the key semantic test: continue in a do-while must jump to
    // the condition check, NOT back to the body start. If continue jumped
    // to the body start, this would be an infinite loop because the
    // condition would never be re-evaluated.
    let source = r#"
fn main(): void {
    var x = 0;
    do {
        x = x + 1;
        if (x >= 3) {
            continue;
        }
        io::println(x);
    } while (x < 3);
}
"#;
    let output = run_source(source).expect("execution should succeed");
    // x=1: x>=3 false, print 1, check x<3 true
    // x=2: x>=3 false, print 2, check x<3 true
    // x=3: x>=3 true, continue -> jump to condition check
    //       check x<3 false -> exit loop
    assert_eq!(output, vec!["1", "2"], "continue must jump to condition check, not body start");
}

#[test]
fn test_do_while_nested() {
    let source = r#"
fn main(): void {
    var count = 0;
    var i = 0;
    do {
        var j = 0;
        do {
            count = count + 1;
            j = j + 1;
        } while (j < 2);
        i = i + 1;
    } while (i < 3);
    io::println(count);
}
"#;
    let output = run_source(source).expect("execution should succeed");
    assert_eq!(output, vec!["6"], "nested do-while should count 3*2=6 iterations");
}

#[test]
fn test_do_while_without_parens() {
    // do-while condition without parentheses
    let source = r#"
fn main(): void {
    var x = 0;
    do {
        x = x + 1;
    } while x < 3;
    io::println(x);
}
"#;
    let output = run_source(source).expect("execution should succeed");
    assert_eq!(output, vec!["3"], "do-while should work without parens around condition");
}

#[test]
fn test_do_while_trailing_semicolon() {
    // do-while with trailing semicolon (optional)
    let source = r#"
fn main(): void {
    var x = 0;
    do {
        x = x + 1;
    } while (x < 2);
    io::println(x);
}
"#;
    let output = run_source(source).expect("execution should succeed");
    assert_eq!(output, vec!["2"], "do-while with trailing semicolon should work");
}

// ---------------------------------------------------------------------------
// Tempfile native function tests
// ---------------------------------------------------------------------------

#[test]
fn test_tempfile_create_unique() {
    let mut vm = Vm::new();
    let result1 = vm.call_native_by_name("Tempfile_create", &[
        Value::String(Rc::new("test_unique_".to_string())),
    ]).unwrap();
    let result2 = vm.call_native_by_name("Tempfile_create", &[
        Value::String(Rc::new("test_unique_".to_string())),
    ]).unwrap();

    let path1 = match &result1 {
        Value::String(s) => s.to_string(),
        other => panic!("Expected String, got {:?}", other),
    };
    let path2 = match &result2 {
        Value::String(s) => s.to_string(),
        other => panic!("Expected String, got {:?}", other),
    };

    // Two calls with the same prefix should produce different paths (random suffix)
    assert_ne!(path1, path2, "Two Tempfile_create calls should produce different paths");

    // Clean up
    let _ = std::fs::remove_file(&path1);
    let _ = std::fs::remove_file(&path2);
}

#[test]
fn test_tempfile_create_dir_unique() {
    let mut vm = Vm::new();
    let result1 = vm.call_native_by_name("Tempfile_create", &[
        Value::String(Rc::new("test_dir_unique_".to_string())),
        Value::Bool(true),
    ]).unwrap();
    let result2 = vm.call_native_by_name("Tempfile_create", &[
        Value::String(Rc::new("test_dir_unique_".to_string())),
        Value::Bool(true),
    ]).unwrap();

    let path1 = match &result1 {
        Value::String(s) => s.to_string(),
        other => panic!("Expected String, got {:?}", other),
    };
    let path2 = match &result2 {
        Value::String(s) => s.to_string(),
        other => panic!("Expected String, got {:?}", other),
    };

    assert_ne!(path1, path2, "Two Tempfile_create dir calls should produce different paths");

    // Clean up
    let _ = std::fs::remove_dir_all(&path1);
    let _ = std::fs::remove_dir_all(&path2);
}

// ---------------------------------------------------------------------------
// Subprocess_exec native function tests
// ---------------------------------------------------------------------------

#[test]
fn test_subprocess_exec_captures_stdout() {
    let mut vm = Vm::new();
    let result = vm.call_native_by_name("Subprocess_exec", &[
        Value::String(Rc::new("cmd".to_string())),
        Value::String(Rc::new("/C".to_string())),
        Value::String(Rc::new("echo hello_world".to_string())),
    ]).expect("Subprocess_exec should succeed");

    match &result {
        Value::String(s) => assert!(s.contains("hello_world"), "stdout should contain 'hello_world', got: {}", s),
        other => panic!("Expected String, got {:?}", other),
    }
}

#[test]
fn test_subprocess_exec_returns_string() {
    let mut vm = Vm::new();
    let result = vm.call_native_by_name("Subprocess_exec", &[
        Value::String(Rc::new("cmd".to_string())),
        Value::String(Rc::new("/C".to_string())),
        Value::String(Rc::new("echo test_output".to_string())),
    ]).expect("Subprocess_exec should succeed");

    match &result {
        Value::String(s) => assert!(s.contains("test_output"), "stdout should contain 'test_output', got: {}", s),
        other => panic!("Expected String, got {:?}", other),
    }
}
