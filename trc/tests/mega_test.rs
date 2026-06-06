use std::fs;
use trc::lexer;
use trc::parser;
use trc::analyzer;
use trc::bytecode;

#[test]
fn mega_test_output_matches_expected() {
    let source = fs::read_to_string("../mega_test.tr")
        .expect("mega_test.tr should exist");
    let expected = fs::read_to_string("../expected_output.txt")
        .expect("expected_output.txt should exist");

    let tokens = lexer::tokenize(&source).expect("tokenization should succeed");
    let ast = parser::parse(tokens).expect("parsing should succeed");
    let typed_ast = analyzer::analyze(&ast).expect("semantic analysis should succeed");

    let output = bytecode::execute(&typed_ast).expect("bytecode execution should succeed");

    let actual: String = output.join("\n");
    let expected_trimmed = expected.trim_end().replace("\r\n", "\n");

    assert_eq!(actual, expected_trimmed, "mega test output must match byte-for-byte");
}
