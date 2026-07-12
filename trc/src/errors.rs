//! Error code catalog and diagnostic rendering for the Titrate compiler.
//!
//! Every error variant in the lexer, parser, and analyzer is assigned a stable
//! code:
//!
//! - `E0xxx` — lexical errors (lexer)
//! - `E1xxx` — parse errors (parser)
//! - `E2xxx` — semantic errors (analyzer)
//! - `E3xxx` — codegen errors
//!
//! The `classify_*` functions map a free-form error message string to one of
//! the stable codes by matching on message prefixes or substrings. This keeps
//! the catalog centralized here rather than scattered across the compiler.
//!
//! `render_error` produces a diagnostic in the canonical format:
//!
//! ```text
//! error[E0XXX]: <message>
//!   --> file.tr:L:C
//!    |
//!  L | <source line>
//!    | ^^^
//! ```

// ---------------------------------------------------------------------------
// Lexical error codes (E0xxx)
// ---------------------------------------------------------------------------

pub const LEX_UNTERMINATED_STRING: &str = "E0001";
pub const LEX_UNTERMINATED_CHAR: &str = "E0002";
pub const LEX_INVALID_STRING_ESCAPE: &str = "E0003";
pub const LEX_INVALID_HEX_ESCAPE: &str = "E0004";
pub const LEX_INVALID_UNICODE_ESCAPE: &str = "E0005";
pub const LEX_EMPTY_UNICODE_ESCAPE: &str = "E0006";
pub const LEX_UNTERMINATED_UNICODE_ESCAPE: &str = "E0007";
pub const LEX_INVALID_CHAR_ESCAPE: &str = "E0008";
pub const LEX_INVALID_HEX_LITERAL: &str = "E0009";
pub const LEX_INVALID_OCT_LITERAL: &str = "E0010";
pub const LEX_INVALID_BIN_LITERAL: &str = "E0011";
pub const LEX_INVALID_DEC_LITERAL: &str = "E0012";
pub const LEX_INVALID_FLOAT_LITERAL: &str = "E0013";
pub const LEX_INVALID_CHAR_LITERAL: &str = "E0014";
pub const LEX_INVALID_BYTE_LITERAL: &str = "E0015";
pub const LEX_NUMBER_OUT_OF_RANGE: &str = "E0016";
pub const LEX_INVALID_RAW_STRING_DELIM: &str = "E0017";
pub const LEX_UNTERMINATED_RAW_STRING: &str = "E0018";
pub const LEX_INVALID_NUMBER_SUFFIX: &str = "E0019";
pub const LEX_INVALID_FLOAT_SUFFIX: &str = "E0020";
pub const LEX_MALFORMED_NUMBER: &str = "E0021";
pub const LEX_UNTERMINATED_BLOCK_COMMENT: &str = "E0022";
pub const LEX_UNRECOGNIZED_CHAR: &str = "E0023";
pub const LEX_UNKNOWN: &str = "E0099";

// ---------------------------------------------------------------------------
// Parse error codes (E1xxx)
// ---------------------------------------------------------------------------

pub const PARSE_EXPECTED_TOKEN: &str = "E1000";
pub const PARSE_EXPECTED_NAME: &str = "E1001";
pub const PARSE_EXPECTED_TYPE: &str = "E1002";
pub const PARSE_EXPECTED_TYPE_PARAM: &str = "E1003";
pub const PARSE_EXPECTED_WHERE_CLAUSE: &str = "E1004";
pub const PARSE_EXPECTED_DECLARATION: &str = "E1005";
pub const PARSE_EXPECTED_EXPRESSION: &str = "E1006";
pub const PARSE_EXPECTED_PATTERN: &str = "E1007";
pub const PARSE_EXPECTED_STATEMENT: &str = "E1008";
pub const PARSE_EXPECTED_CLOSE_ANGLE: &str = "E1009";
pub const PARSE_EXPECTED_SEMICOLON: &str = "E1010";
pub const PARSE_EXPECTED_BRACE: &str = "E1011";
pub const PARSE_EXPECTED_PAREN: &str = "E1012";
pub const PARSE_EXPECTED_BRACKET: &str = "E1013";
pub const PARSE_EXPECTED_COLON: &str = "E1014";
pub const PARSE_EXPECTED_COMMA: &str = "E1015";
pub const PARSE_EXPECTED_EQUALS: &str = "E1016";
pub const PARSE_EXPECTED_WHILE: &str = "E1017";
pub const PARSE_EXPECTED_IN: &str = "E1018";
pub const PARSE_EXPECTED_CATCH_OR_FINALLY: &str = "E1019";
pub const PARSE_EXPECTED_CASE_OR_DEFAULT: &str = "E1020";
pub const PARSE_INVALID_PATTERN: &str = "E1021";
pub const PARSE_UNKNOWN: &str = "E1999";

// ---------------------------------------------------------------------------
// Semantic error codes (E2xxx)
// ---------------------------------------------------------------------------

pub const SEM_UNDECLARED_IDENTIFIER: &str = "E2000";
pub const SEM_USE_OF_MOVED_VARIABLE: &str = "E2001";
pub const SEM_DUPLICATE_DECLARATION: &str = "E2002";
pub const SEM_TYPE_MISMATCH: &str = "E2003";
pub const SEM_RETURN_TYPE_MISMATCH: &str = "E2004";
pub const SEM_MISSING_RETURN: &str = "E2005";
pub const SEM_CONDITION_NOT_BOOL: &str = "E2006";
pub const SEM_FIELD_NOT_FOUND: &str = "E2007";
pub const SEM_METHOD_NOT_FOUND: &str = "E2008";
pub const SEM_NOT_AN_ENUM_VARIANT: &str = "E2009";
pub const SEM_VARIANT_NOT_IN_ENUM: &str = "E2010";
pub const SEM_TUPLE_ARITY_MISMATCH: &str = "E2011";
pub const SEM_TUPLE_NOT_TUPLE_TYPE: &str = "E2012";
pub const SEM_UNKNOWN: &str = "E2999";

// ---------------------------------------------------------------------------
// Codegen error codes (E3xxx)
// ---------------------------------------------------------------------------

pub const CODEGEN_UNKNOWN: &str = "E3999";

// ---------------------------------------------------------------------------
// Classification — map a free-form error message to a stable code.
// ---------------------------------------------------------------------------

/// Classify a lexer error message into a stable error code.
///
/// Lexer error messages are produced by `trc::lexer::tokenize` and follow
/// patterns like `"Unterminated string at L:C"` or
/// `"Invalid hex literal at L:C: <reason>"`.
pub fn classify_lexer_error(msg: &str) -> &'static str {
    if msg.starts_with("Unterminated string") {
        LEX_UNTERMINATED_STRING
    } else if msg.starts_with("Unterminated char") {
        LEX_UNTERMINATED_CHAR
    } else if msg.starts_with("Unknown char escape") || msg.starts_with("Unterminated string escape") {
        LEX_INVALID_CHAR_ESCAPE
    } else if msg.starts_with("Invalid hex escape") {
        LEX_INVALID_HEX_ESCAPE
    } else if msg.starts_with("Empty unicode escape") {
        LEX_EMPTY_UNICODE_ESCAPE
    } else if msg.starts_with("Unterminated unicode escape") {
        LEX_UNTERMINATED_UNICODE_ESCAPE
    } else if msg.starts_with("Invalid unicode escape") {
        LEX_INVALID_UNICODE_ESCAPE
    } else if msg.starts_with("Invalid hex literal") {
        LEX_INVALID_HEX_LITERAL
    } else if msg.starts_with("Invalid oct literal") || msg.starts_with("Invalid octal literal") {
        LEX_INVALID_OCT_LITERAL
    } else if msg.starts_with("Invalid bin literal") || msg.starts_with("Invalid binary literal") {
        LEX_INVALID_BIN_LITERAL
    } else if msg.starts_with("Invalid float literal") || msg.starts_with("Invalid float") {
        LEX_INVALID_FLOAT_LITERAL
    } else if msg.starts_with("Invalid char literal") {
        LEX_INVALID_CHAR_LITERAL
    } else if msg.starts_with("Invalid byte literal") {
        LEX_INVALID_BYTE_LITERAL
    } else if msg.starts_with("number out of range") || msg.contains("out of range") {
        LEX_NUMBER_OUT_OF_RANGE
    } else if msg.starts_with("Unterminated block comment") {
        LEX_UNTERMINATED_BLOCK_COMMENT
    } else if msg.starts_with("Unterminated raw string") {
        LEX_UNTERMINATED_RAW_STRING
    } else if msg.starts_with("Unrecognized character") || msg.starts_with("Unexpected character") {
        LEX_UNRECOGNIZED_CHAR
    } else {
        LEX_UNKNOWN
    }
}

/// Classify a parser error message into a stable error code.
///
/// Parser error messages are produced by `trc::parser::parse` and follow
/// patterns like `"Expected <token>, found <token> at L:C"` or
/// `"Expected declaration, found <token> at L:C"`.
pub fn classify_parse_error(msg: &str) -> &'static str {
    // Strip the location suffix before matching so the prefix checks are
    // stable regardless of whether the message ends with ` at L:C`.
    let stripped = strip_location_suffix(msg);
    if stripped.starts_with("Expected 'while'") {
        PARSE_EXPECTED_WHILE
    } else if stripped.starts_with("Expected 'in'") {
        PARSE_EXPECTED_IN
    } else if stripped.starts_with("Expected 'catch' or 'finally'") {
        PARSE_EXPECTED_CATCH_OR_FINALLY
    } else if stripped.starts_with("Expected 'case' or 'default'") {
        PARSE_EXPECTED_CASE_OR_DEFAULT
    } else if stripped.starts_with("Expected type parameter name") {
        PARSE_EXPECTED_TYPE_PARAM
    } else if stripped.starts_with("Expected type name") || stripped.starts_with("Expected type") {
        PARSE_EXPECTED_TYPE
    } else if stripped.starts_with("Expected pattern") {
        PARSE_EXPECTED_PATTERN
    } else if stripped.starts_with("Expected expression") {
        PARSE_EXPECTED_EXPRESSION
    } else if stripped.starts_with("Expected declaration") {
        PARSE_EXPECTED_DECLARATION
    } else if stripped.starts_with("Expected name") || stripped.starts_with("Expected identifier") {
        PARSE_EXPECTED_NAME
    } else if stripped.starts_with("Expected >") {
        PARSE_EXPECTED_CLOSE_ANGLE
    } else if stripped.starts_with("Expected Semicolon") || stripped.contains("Expected Semicolon") {
        PARSE_EXPECTED_SEMICOLON
    } else if stripped.starts_with("Expected LeftBrace") || stripped.contains("Expected LeftBrace") {
        PARSE_EXPECTED_BRACE
    } else if stripped.starts_with("Expected LeftParen") || stripped.contains("Expected LeftParen") {
        PARSE_EXPECTED_PAREN
    } else if stripped.starts_with("Expected LeftBracket") || stripped.contains("Expected LeftBracket") {
        PARSE_EXPECTED_BRACKET
    } else if stripped.starts_with("Expected Colon") || stripped.contains("Expected Colon") {
        PARSE_EXPECTED_COLON
    } else if stripped.starts_with("Expected Comma") || stripped.contains("Expected Comma") {
        PARSE_EXPECTED_COMMA
    } else if stripped.starts_with("Expected Equals") || stripped.contains("Expected Equals") {
        PARSE_EXPECTED_EQUALS
    } else if stripped.starts_with("Expected ") {
        // Generic "Expected X, found Y" — the most common parser error.
        PARSE_EXPECTED_TOKEN
    } else {
        PARSE_UNKNOWN
    }
}

/// Classify a semantic (analyzer) error message into a stable error code.
///
/// Analyzer error messages are produced by `trc::analyzer::analyze` and
/// follow patterns like `"undeclared identifier: 'foo'"` or
/// `"type mismatch in variable 'x': ..."`.
pub fn classify_semantic_error(msg: &str) -> &'static str {
    if msg.starts_with("undeclared identifier") {
        SEM_UNDECLARED_IDENTIFIER
    } else if msg.starts_with("use of moved variable") {
        SEM_USE_OF_MOVED_VARIABLE
    } else if msg.starts_with("duplicate declaration") {
        SEM_DUPLICATE_DECLARATION
    } else if msg.starts_with("return type mismatch") {
        SEM_RETURN_TYPE_MISMATCH
    } else if msg.starts_with("type mismatch") {
        SEM_TYPE_MISMATCH
    } else if msg.contains("is missing a return statement") || msg.contains("must return a value") {
        SEM_MISSING_RETURN
    } else if msg.contains("condition must be bool") {
        SEM_CONDITION_NOT_BOOL
    } else if msg.starts_with("field") && msg.contains("not found") {
        SEM_FIELD_NOT_FOUND
    } else if msg.starts_with("method") && msg.contains("not found") {
        SEM_METHOD_NOT_FOUND
    } else if msg.contains("is not an enum variant") {
        SEM_NOT_AN_ENUM_VARIANT
    } else if msg.starts_with("variant") && msg.contains("does not belong to enum") {
        SEM_VARIANT_NOT_IN_ENUM
    } else if msg.starts_with("tuple destructuring expects") {
        SEM_TUPLE_ARITY_MISMATCH
    } else if msg.starts_with("tuple destructuring requires a tuple type") {
        SEM_TUPLE_NOT_TUPLE_TYPE
    } else {
        SEM_UNKNOWN
    }
}

// ---------------------------------------------------------------------------
// Location suffix parsing
// ---------------------------------------------------------------------------

/// Extract a trailing ` at L:C` location suffix from an error message.
///
/// Returns `Some((line, col))` when the message ends with ` at <line>:<col>`,
/// where both `line` and `col` are decimal numbers. Returns `None` otherwise.
///
/// Examples:
/// - `"Unterminated string at 3:5"` → `Some((3, 5))`
/// - `"Expected Semicolon, found Eof at 1:11"` → `Some((1, 11))`
/// - `"undeclared identifier: 'foo'"` → `None`
pub fn parse_location_suffix(msg: &str) -> Option<(usize, usize)> {
    // Find the last occurrence of " at " — the location suffix is the text
    // after it, expected to be `<line>:<col>` with nothing trailing.
    let at_pos = msg.rfind(" at ")?;
    let suffix = &msg[at_pos + 4..];
    let colon = suffix.find(':')?;
    let line_str = &suffix[..colon];
    let col_str = &suffix[colon + 1..];
    // Reject anything with extra text after the column number.
    if col_str.contains(' ') || col_str.contains(':') {
        return None;
    }
    let line = line_str.parse::<usize>().ok()?;
    let col = col_str.parse::<usize>().ok()?;
    Some((line, col))
}

/// Remove a trailing ` at L:C` location suffix from an error message.
///
/// When the message ends with ` at <line>:<col>`, that suffix is removed and
/// the message before it is returned. Otherwise the message is returned
/// unchanged.
///
/// Examples:
/// - `"Unterminated string at 3:5"` → `"Unterminated string"`
/// - `"Expected Semicolon, found Eof at 1:11"` → `"Expected Semicolon, found Eof"`
/// - `"undeclared identifier: 'foo'"` → `"undeclared identifier: 'foo'"`
pub fn strip_location_suffix(msg: &str) -> String {
    if let Some(at_pos) = msg.rfind(" at ") {
        let suffix = &msg[at_pos + 4..];
        if let Some(colon) = suffix.find(':') {
            let line_str = &suffix[..colon];
            let col_str = &suffix[colon + 1..];
            if !col_str.contains(' ') && !col_str.contains(':') && line_str.parse::<usize>().is_ok() && col_str.parse::<usize>().is_ok() {
                return msg[..at_pos].to_string();
            }
        }
    }
    msg.to_string()
}

// ---------------------------------------------------------------------------
// Diagnostic rendering
// ---------------------------------------------------------------------------

/// Render an error in the canonical Titrate diagnostic format.
///
/// - `code` — the stable error code (e.g. `"E0001"`).
/// - `message` — the human-readable error message (location suffix already
///   stripped).
/// - `file` — the source file path.
/// - `line` — optional 1-based line number.
/// - `col` — optional 1-based column number.
/// - `source_line` — optional text of the offending source line.
/// - `span_len` — number of carets to render under the source line.
///
/// Output:
/// ```text
/// error[E0XXX]: <message>
///   --> file.tr:L:C
///    |
///  L | <source line>
///    | ^^^
/// ```
///
/// When `source_line` is `None`, the caret line is omitted. When `line` or
/// `col` is `None`, the `-->` line is omitted entirely. When all three are
/// `None`, only the `error[E0XXX]: <message>` header is emitted.
pub fn render_error(
    code: &str,
    message: &str,
    file: &str,
    line: Option<usize>,
    col: Option<usize>,
    source_line: Option<&str>,
    span_len: usize,
) -> String {
    let mut out = String::new();
    out.push_str(&format!("error[{}]: {}\n", code, message));

    if let (Some(l), Some(c)) = (line, col) {
        out.push_str(&format!("  --> {}:{}:{}\n", file, l, c));
    }

    if let Some(src) = source_line {
        let lineno_str = line.map(|l| l.to_string()).unwrap_or_default();
        let gutter_width = lineno_str.len().max(1);
        let pad = " ".repeat(gutter_width);
        out.push_str(&format!("{} |\n", pad));
        out.push_str(&format!("{} | {}\n", lineno_str, src));
        let carets = "^".repeat(span_len.max(1));
        let col_offset = col.unwrap_or(1).saturating_sub(1);
        let caret_pad = " ".repeat(col_offset);
        out.push_str(&format!("{} | {}{}\n", pad, caret_pad, carets));
    }

    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_lexer_unterminated_string() {
        let msg = "Unterminated string at 1:10";
        assert_eq!(classify_lexer_error(msg), LEX_UNTERMINATED_STRING);
    }

    #[test]
    fn test_classify_lexer_invalid_hex_literal() {
        let msg = "Invalid hex literal at 1:5: invalid digit";
        assert_eq!(classify_lexer_error(msg), LEX_INVALID_HEX_LITERAL);
    }

    #[test]
    fn test_classify_lexer_unknown_fallback() {
        let msg = "something weird at 1:1";
        assert_eq!(classify_lexer_error(msg), LEX_UNKNOWN);
    }

    #[test]
    fn test_classify_parse_expected_token() {
        let msg = "Expected Semicolon, found Eof at 1:11";
        assert_eq!(classify_parse_error(msg), PARSE_EXPECTED_SEMICOLON);
    }

    #[test]
    fn test_classify_parse_expected_declaration() {
        let msg = "Expected declaration, found Semicolon at 1:1";
        assert_eq!(classify_parse_error(msg), PARSE_EXPECTED_DECLARATION);
    }

    #[test]
    fn test_classify_parse_expected_while() {
        let msg = "Expected 'while' after do block, found Eof at 2:1";
        assert_eq!(classify_parse_error(msg), PARSE_EXPECTED_WHILE);
    }

    #[test]
    fn test_classify_parse_unknown_fallback() {
        let msg = "weird parse error at 1:1";
        assert_eq!(classify_parse_error(msg), PARSE_UNKNOWN);
    }

    #[test]
    fn test_classify_semantic_undeclared() {
        let msg = "undeclared identifier: 'foo'";
        assert_eq!(classify_semantic_error(msg), SEM_UNDECLARED_IDENTIFIER);
    }

    #[test]
    fn test_classify_semantic_type_mismatch() {
        let msg = "type mismatch in variable 'x': cannot assign int to string";
        assert_eq!(classify_semantic_error(msg), SEM_TYPE_MISMATCH);
    }

    #[test]
    fn test_classify_semantic_duplicate() {
        let msg = "duplicate declaration: 'foo' is already declared in this scope";
        assert_eq!(classify_semantic_error(msg), SEM_DUPLICATE_DECLARATION);
    }

    #[test]
    fn test_parse_location_suffix_lexer() {
        assert_eq!(parse_location_suffix("Unterminated string at 3:5"), Some((3, 5)));
    }

    #[test]
    fn test_parse_location_suffix_parser() {
        assert_eq!(
            parse_location_suffix("Expected Semicolon, found Eof at 1:11"),
            Some((1, 11))
        );
    }

    #[test]
    fn test_parse_location_suffix_none() {
        assert_eq!(parse_location_suffix("undeclared identifier: 'foo'"), None);
    }

    #[test]
    fn test_strip_location_suffix_lexer() {
        assert_eq!(strip_location_suffix("Unterminated string at 3:5"), "Unterminated string");
    }

    #[test]
    fn test_strip_location_suffix_parser() {
        assert_eq!(
            strip_location_suffix("Expected Semicolon, found Eof at 1:11"),
            "Expected Semicolon, found Eof"
        );
    }

    #[test]
    fn test_strip_location_suffix_preserves_when_no_location() {
        assert_eq!(
            strip_location_suffix("undeclared identifier: 'foo'"),
            "undeclared identifier: 'foo'"
        );
    }

    #[test]
    fn test_render_with_source_line() {
        let rendered = render_error(
            "E0001",
            "Unterminated string",
            "test.tr",
            Some(1),
            Some(10),
            Some("let s = \"hello;"),
            7,
        );
        assert!(rendered.contains("error[E0001]: Unterminated string"));
        assert!(rendered.contains("--> test.tr:1:10"));
        assert!(rendered.contains("1 | let s = \"hello;"));
        assert!(rendered.contains("^^^^^^^"));
    }

    #[test]
    fn test_render_without_source_line() {
        let rendered = render_error(
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
        assert!(!rendered.contains('^'));
    }

    #[test]
    fn test_render_without_location() {
        let rendered = render_error("E2999", "unknown error", "test.tr", None, None, None, 1);
        assert_eq!(rendered, "error[E2999]: unknown error\n");
    }

    #[test]
    fn test_error_code_ranges() {
        assert!(LEX_UNTERMINATED_STRING.starts_with("E0"));
        assert_eq!(LEX_UNKNOWN, "E0099");
        assert!(PARSE_EXPECTED_TOKEN.starts_with("E1"));
        assert_eq!(PARSE_UNKNOWN, "E1999");
        assert!(SEM_UNDECLARED_IDENTIFIER.starts_with("E2"));
        assert_eq!(SEM_UNKNOWN, "E2999");
        assert!(CODEGEN_UNKNOWN.starts_with("E3"));
    }
}
