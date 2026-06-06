use std::fs;
use std::path::PathBuf;
use trc::lexer;
use trc::parser;
use trc::bytecode;

/// Helper: compile and run a multi-file Titrate program using compile_with_modules.
/// Note: The semantic analyzer is skipped for multi-file tests because it does not
/// resolve imported symbols. The compiler's own module system handles that.
fn compile_and_run_mega_test_02() -> Result<Vec<String>, String> {
    let base_dir = PathBuf::from("../mega_test_02/src");

    let source = fs::read_to_string(base_dir.join("main.tr"))
        .expect("mega_test_02/src/main.tr should exist");

    let tokens = lexer::tokenize(&source).expect("tokenization should succeed");
    let ast = parser::parse(tokens).expect("parsing should succeed");

    let mut compiler = bytecode::Compiler::new();
    let compiled = compiler.compile_with_modules(&ast, &base_dir)
        .expect("multi-file compilation should succeed");

    let mut vm = bytecode::Vm::new();
    vm.load_program(compiled);
    vm.run()?;

    Ok(vm.output)
}

#[test]
fn mega_test_02_output_matches_expected() {
    let expected = fs::read_to_string("../mega_test_02/expected_output.txt")
        .expect("expected_output.txt should exist");

    let output = compile_and_run_mega_test_02()
        .expect("mega test 0.2 execution should succeed");

    let actual: String = output.join("\n");
    let expected_trimmed = expected.trim_end().replace("\r\n", "\n");

    assert_eq!(actual, expected_trimmed, "mega test 0.2 output must match byte-for-byte");
}
