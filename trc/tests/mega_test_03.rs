use std::fs;
use std::path::PathBuf;
use trc::lexer;
use trc::parser;
use trc::bytecode;

/// Helper: compile and run the multi-file mega_test_03 program.
/// The semantic analyzer is skipped for multi-file tests because it does not
/// resolve imported symbols. The compiler's own module system handles that.
fn compile_and_run_mega_test_03() -> Result<Vec<String>, String> {
    let base_dir = PathBuf::from("../mega_test_03/src");

    let source = fs::read_to_string(base_dir.join("main.tr"))
        .expect("mega_test_03/src/main.tr should exist");

    let tokens = lexer::tokenize(&source).expect("tokenization should succeed");
    let ast = parser::parse(tokens).expect("parsing should succeed");

    let mut compiler = bytecode::Compiler::new();
    let compiled = compiler.compile_with_modules(&ast, &base_dir)
        .expect("multi-file compilation should succeed");

    let mut vm = bytecode::Vm::new();
    vm.set_working_dir(base_dir.clone());
    vm.load_program(compiled);
    vm.run()?;

    Ok(vm.output)
}

/// Compare actual output against expected output, treating `<PLACEHOLDER>`
/// in the expected file as a wildcard that matches any non-empty text on
/// that portion of the line.
fn matches_expected(actual: &str, expected: &str) -> bool {
    let actual_lines: Vec<&str> = actual.lines().collect();
    let expected_lines: Vec<&str> = expected.lines().collect();

    if actual_lines.len() != expected_lines.len() {
        return false;
    }

    for (a_line, e_line) in actual_lines.iter().zip(expected_lines.iter()) {
        // Split expected line by <PLACEHOLDER> and verify actual matches
        let mut remaining = *a_line;
        let mut first = true;
        for part in e_line.split("<PLACEHOLDER>") {
            if first {
                first = false;
            } else {
                // We just passed a <PLACEHOLDER> – skip over any non-empty
                // text in the actual line up to the next literal part.
                // The placeholder matches at least one character.
                if part.is_empty() {
                    // Trailing placeholder – matches the rest of the line
                    break;
                }
                if let Some(pos) = remaining.find(part) {
                    if pos == 0 && !first {
                        // Placeholder matched zero characters, which is not
                        // allowed – each placeholder must match something.
                    }
                    remaining = &remaining[pos + part.len()..];
                } else {
                    return false;
                }
                continue;
            }
            // First segment (before any placeholder) must match literally
            if !remaining.starts_with(part) {
                return false;
            }
            remaining = &remaining[part.len()..];
        }
    }
    true
}

#[test]
fn mega_test_03_output_matches_expected() {
    let expected = fs::read_to_string("../mega_test_03/expected_output.txt")
        .expect("expected_output.txt should exist");

    let output = compile_and_run_mega_test_03()
        .expect("mega test 0.3 execution should succeed");

    let actual: String = output.join("\n");
    let expected_trimmed = expected.trim_end().replace("\r\n", "\n");

    assert!(
        matches_expected(&actual, &expected_trimmed),
        "mega test 0.3 output must match expected (with <PLACEHOLDER> wildcards)\n--- ACTUAL ---\n{}\n--- EXPECTED ---\n{}",
        actual,
        expected_trimmed,
    );
}
