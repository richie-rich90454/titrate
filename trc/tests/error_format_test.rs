//! Integration tests for the error code catalog and error format.
//!
//! These tests verify that compiler errors from the lexer, parser, and
//! analyzer flow through the classification and rendering pipeline to
//! produce the standard `error[E0XXX]: <message>` format with source
//! location and caret.

use trc::errors;
use trc::lexer;
use trc::parser;

#[test]
fn test_lexer_error_full_pipeline() {
    let src = "let s = \"hello;\n";
    let err = lexer::tokenize(src).expect_err("should fail on unterminated string");
    let code = errors::classify_lexer_error(&err);
    assert_eq!(code, errors::LEX_UNTERMINATED_STRING);
    let loc = errors::parse_location_suffix(&err);
    assert!(loc.is_some(), "lexer error should have location suffix");
    let (line, col) = loc.unwrap();
    let clean = errors::strip_location_suffix(&err);
    let rendered = errors::render_error(code, &clean, "test.tr", Some(line), Some(col), None, 1);
    assert!(rendered.starts_with("error[E0001]:"));
    assert!(rendered.contains("--> test.tr"));
}

#[test]
fn test_lexer_invalid_hex_literal() {
    let src = "let x = 0xZZ;";
    let err = lexer::tokenize(src).expect_err("should fail on invalid hex");
    let code = errors::classify_lexer_error(&err);
    assert_eq!(code, errors::LEX_INVALID_HEX_LITERAL);
}

#[test]
fn test_parser_error_full_pipeline() {
    let src = "let x = 42";
    let tokens = lexer::tokenize(src).expect("should tokenize");
    let err = parser::parse(tokens).expect_err("should fail on missing semicolon");
    let code = errors::classify_parse_error(&err);
    assert_eq!(code, errors::PARSE_EXPECTED_TOKEN);
    let loc = errors::parse_location_suffix(&err);
    assert!(loc.is_some(), "parser error should have location suffix");
    let rendered = errors::render_error(code, &err, "test.tr", None, None, None, 1);
    assert!(rendered.starts_with("error[E1000]:"));
}

#[test]
fn test_parser_expected_declaration() {
    let src = ";";
    let tokens = lexer::tokenize(src).expect("should tokenize");
    let err = parser::parse(tokens).expect_err("should fail on stray semicolon");
    let code = errors::classify_parse_error(&err);
    assert_eq!(code, errors::PARSE_EXPECTED_DECLARATION);
}

#[test]
fn test_parser_missing_brace() {
    let src = "fn foo(): void {";
    let tokens = lexer::tokenize(src).expect("should tokenize");
    let err = parser::parse(tokens).expect_err("should fail on missing brace");
    let code = errors::classify_parse_error(&err);
    assert!(code.starts_with("E1"));
}

#[test]
fn test_render_full_diagnostic_with_source() {
    let source_line = "let s = \"hello;";
    let rendered = errors::render_error(
        "E0001",
        "Unterminated string",
        "test.tr",
        Some(1),
        Some(10),
        Some(source_line),
        7,
    );
    assert!(rendered.contains("error[E0001]: Unterminated string"));
    assert!(rendered.contains("--> test.tr:1:10"));
    assert!(rendered.contains("1 | let s = \"hello;"));
    assert!(rendered.contains("^^^^^^^"));
}

#[test]
fn test_render_without_source_line() {
    let rendered = errors::render_error(
        "E2000",
        "undeclared identifier: 'x'",
        "test.tr",
        Some(5),
        Some(3),
        None,
        1,
    );
    assert!(rendered.contains("error[E2000]: undeclared identifier: 'x'"));
    assert!(rendered.contains("--> test.tr:5:3"));
    assert!(!rendered.contains("^"));
}

#[test]
fn test_render_without_location() {
    let rendered = errors::render_error("E2999", "unknown error", "test.tr", None, None, None, 1);
    assert_eq!(rendered, "error[E2999]: unknown error\n");
}

#[test]
fn test_strip_location_from_lexer_error() {
    let msg = "Unterminated string at 3:5";
    assert_eq!(errors::strip_location_suffix(msg), "Unterminated string");
}

#[test]
fn test_strip_location_from_parser_error() {
    let msg = "Expected Semicolon, found Eof at 1:11";
    assert_eq!(errors::strip_location_suffix(msg), "Expected Semicolon, found Eof");
}

#[test]
fn test_classify_semantic_messages() {
    assert_eq!(errors::classify_semantic_error("undeclared identifier: 'foo'"), errors::SEM_UNDECLARED_IDENTIFIER);
    assert_eq!(errors::classify_semantic_error("type mismatch in variable 'x'"), errors::SEM_TYPE_MISMATCH);
    assert_eq!(errors::classify_semantic_error("duplicate declaration: 'foo'"), errors::SEM_DUPLICATE_DECLARATION);
}

#[test]
fn test_error_code_ranges() {
    assert!(errors::LEX_UNTERMINATED_STRING.starts_with("E0"));
    assert!(errors::LEX_UNKNOWN == "E0099");
    assert!(errors::PARSE_EXPECTED_TOKEN.starts_with("E1"));
    assert!(errors::PARSE_UNKNOWN == "E1999");
    assert!(errors::SEM_UNDECLARED_IDENTIFIER.starts_with("E2"));
    assert!(errors::SEM_UNKNOWN == "E2999");
    assert!(errors::CODEGEN_UNKNOWN.starts_with("E3"));
}
