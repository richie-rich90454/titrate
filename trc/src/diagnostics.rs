//! Machine-readable diagnostics for the Titrate compiler.
//!
//! `crate::errors` owns the stable error-code catalog and the human-readable
//! renderer. This module owns the *machine-readable* representation: a small
//! `Diagnostic` value that serializes to a stable JSON object with a fixed key
//! order. Editors, the LSP server, and CI can consume that object without
//! scraping the human-readable output.
//!
//! The JSON shape is a contract. Keys are always emitted in this order:
//!
//! ```json
//! {"code":"E2000","message":"undeclared identifier: 'x'","line":5,"column":3,"suggestions":[]}
//! ```
//!
//! - `code` — stable error code from `crate::errors` (e.g. `E2000`).
//! - `message` — primary message with any trailing ` at L:C` suffix removed.
//! - `line` — 1-based line, or `null` when unknown.
//! - `column` — 1-based column, or `null` when unknown.
//! - `suggestions` — array of `{ "message", "replacement" }`; `replacement`
//!   is `null` when the hint has no machine-applicable edit.
//!
//! No external crates are used: JSON strings are escaped by hand so the shape
//! stays dependency-free and deterministic.

use crate::analyzer::CompileError;
use crate::errors;

/// A 1-based source position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub line: usize,
    pub column: usize,
}

impl Span {
    pub fn new(line: usize, column: usize) -> Self {
        Span { line, column }
    }
}

/// One suggested fix for a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticSuggestion {
    /// Human-readable hint (the `help:` text).
    pub message: String,
    /// Exact replacement text when the hint is machine-applicable.
    pub replacement: Option<String>,
}

impl DiagnosticSuggestion {
    pub fn new(message: impl Into<String>) -> Self {
        DiagnosticSuggestion {
            message: message.into(),
            replacement: None,
        }
    }

    pub fn with_replacement(message: impl Into<String>, replacement: impl Into<String>) -> Self {
        DiagnosticSuggestion {
            message: message.into(),
            replacement: Some(replacement.into()),
        }
    }
}

/// A machine-readable compiler diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub suggestions: Vec<DiagnosticSuggestion>,
}

impl Diagnostic {
    /// Build a diagnostic from an explicit code and message.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Diagnostic {
            code: code.into(),
            message: message.into(),
            line: None,
            column: None,
            suggestions: Vec::new(),
        }
    }

    /// Build a diagnostic from a `CompileError`-style display string.
    ///
    /// `err` may embed suggestions as `\n  help: <hint>` lines (the format
    /// produced by `CompileError`'s `Display`), and may carry a trailing
    /// ` at L:C` suffix. When `span` is `None`, coordinates are recovered from
    /// that suffix; when `span` is provided it wins, but the suffix is still
    /// stripped from the stored message.
    pub fn from_display(err: &str, span: Option<Span>) -> Self {
        let mut parts = err.splitn(2, "\n  help: ");
        let raw = parts.next().unwrap_or(err);
        let suggestions = match parts.next() {
            Some(rest) => rest
                .split("\n  help: ")
                .map(DiagnosticSuggestion::new)
                .collect(),
            None => Vec::new(),
        };
        Self::assemble(raw, suggestions, span)
    }

    /// Build a diagnostic directly from an analyzer `CompileError`.
    ///
    /// Unlike `from_display`, this preserves structured `replacement` text on
    /// suggestions, so editors can offer machine-applicable fixes.
    pub fn from_compile_error(err: &CompileError, span: Option<Span>) -> Self {
        let suggestions = err
            .suggestions
            .iter()
            .map(|s| DiagnosticSuggestion {
                message: s.message.clone(),
                replacement: s.replacement.clone(),
            })
            .collect();
        Self::assemble(&err.message, suggestions, span)
    }

    fn assemble(raw: &str, suggestions: Vec<DiagnosticSuggestion>, span: Option<Span>) -> Self {
        let (line, column) = match span {
            Some(s) => (Some(s.line), Some(s.column)),
            None => match errors::parse_location_suffix(raw) {
                Some((l, c)) => (Some(l), Some(c)),
                None => (None, None),
            },
        };
        let message = errors::strip_location_suffix(raw);
        let code = classify_message(&message).to_string();
        Diagnostic {
            code,
            message,
            line,
            column,
            suggestions,
        }
    }

    /// Append this diagnostic's JSON object to `out`.
    pub fn write_json(&self, out: &mut String) {
        out.push('{');
        out.push_str("\"code\":");
        write_json_string(out, &self.code);
        out.push_str(",\"message\":");
        write_json_string(out, &self.message);
        out.push_str(",\"line\":");
        write_json_usize_option(out, self.line);
        out.push_str(",\"column\":");
        write_json_usize_option(out, self.column);
        out.push_str(",\"suggestions\":[");
        for (i, suggestion) in self.suggestions.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push('{');
            out.push_str("\"message\":");
            write_json_string(out, &suggestion.message);
            out.push_str(",\"replacement\":");
            match &suggestion.replacement {
                Some(rep) => write_json_string(out, rep),
                None => out.push_str("null"),
            }
            out.push('}');
        }
        out.push_str("]}");
    }

    /// Serialize this diagnostic to a stable JSON object string.
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        self.write_json(&mut out);
        out
    }
}

/// Serialize a slice of diagnostics to a stable JSON array string.
pub fn to_json_array(diagnostics: &[Diagnostic]) -> String {
    let mut out = String::with_capacity(32 * diagnostics.len());
    out.push('[');
    for (i, diagnostic) in diagnostics.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        diagnostic.write_json(&mut out);
    }
    out.push(']');
    out
}

/// Best-effort classification of a message that may come from any compiler
/// phase. Semantic messages take priority because `CompileError` is the primary
/// input; lexer and parser messages are recognized as a fallback.
fn classify_message(message: &str) -> &'static str {
    let semantic = errors::classify_semantic_error(message);
    if semantic != errors::SEM_UNKNOWN {
        return semantic;
    }
    let lexer = errors::classify_lexer_error(message);
    if lexer != errors::LEX_UNKNOWN {
        return lexer;
    }
    let parser = errors::classify_parse_error(message);
    if parser != errors::PARSE_UNKNOWN {
        return parser;
    }
    semantic
}

fn write_json_usize_option(out: &mut String, value: Option<usize>) {
    match value {
        Some(n) => out.push_str(&n.to_string()),
        None => out.push_str("null"),
    }
}

fn write_json_string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::Suggestion as AnalyzerSuggestion;

    #[test]
    fn json_with_span_has_fixed_key_order() {
        let d = Diagnostic {
            line: Some(5),
            column: Some(3),
            ..Diagnostic::new("E2000", "undeclared identifier: 'x'")
        };
        assert_eq!(
            d.to_json(),
            r#"{"code":"E2000","message":"undeclared identifier: 'x'","line":5,"column":3,"suggestions":[]}"#
        );
    }

    #[test]
    fn json_without_span_uses_null() {
        let d = Diagnostic::new("E2999", "unknown error");
        assert_eq!(
            d.to_json(),
            r#"{"code":"E2999","message":"unknown error","line":null,"column":null,"suggestions":[]}"#
        );
    }

    #[test]
    fn json_escapes_strings() {
        let d = Diagnostic::new("E0001", "say \"hi\"\nnext\tcol\u{01}end\\ é");
        assert_eq!(
            d.to_json(),
            r#"{"code":"E0001","message":"say \"hi\"\nnext\tcol\u0001end\\ é","line":null,"column":null,"suggestions":[]}"#
        );
    }

    #[test]
    fn from_display_parses_help_lines_and_location() {
        let d = Diagnostic::from_display(
            "undeclared identifier: 'cont' at 4:9\n  help: did you mean 'count'?\n  help: declare it first",
            None,
        );
        assert_eq!(d.code, errors::SEM_UNDECLARED_IDENTIFIER);
        assert_eq!(d.message, "undeclared identifier: 'cont'");
        assert_eq!(d.line, Some(4));
        assert_eq!(d.column, Some(9));
        assert_eq!(d.suggestions.len(), 2);
        assert_eq!(d.suggestions[0].message, "did you mean 'count'?");
        assert_eq!(d.suggestions[0].replacement, None);
        assert_eq!(d.suggestions[1].message, "declare it first");
    }

    #[test]
    fn explicit_span_overrides_message_suffix() {
        let d = Diagnostic::from_display("Invalid hex literal at 3:5", Some(Span::new(10, 2)));
        assert_eq!(d.line, Some(10));
        assert_eq!(d.column, Some(2));
        assert_eq!(d.message, "Invalid hex literal");
        assert_eq!(d.code, errors::LEX_INVALID_HEX_LITERAL);
    }

    #[test]
    fn from_compile_error_preserves_replacements() {
        let err = CompileError::new("undeclared identifier: 'coun'".to_string()).suggest(
            AnalyzerSuggestion {
                message: "did you mean 'count'?".to_string(),
                replacement: Some("count".to_string()),
            },
        );
        let d = Diagnostic::from_compile_error(&err, Some(Span::new(2, 7)));
        assert_eq!(d.suggestions.len(), 1);
        assert_eq!(d.suggestions[0].replacement.as_deref(), Some("count"));
        assert!(d.to_json().contains(r#""replacement":"count""#));
    }

    #[test]
    fn suggestion_without_replacement_is_null() {
        let d = Diagnostic {
            suggestions: vec![DiagnosticSuggestion::new("try another name")],
            ..Diagnostic::new("E2000", "msg")
        };
        assert_eq!(
            d.to_json(),
            r#"{"code":"E2000","message":"msg","line":null,"column":null,"suggestions":[{"message":"try another name","replacement":null}]}"#
        );
    }

    #[test]
    fn suggestion_with_replacement_serializes_value() {
        let d = Diagnostic {
            suggestions: vec![DiagnosticSuggestion::with_replacement("use 'x'", "x")],
            ..Diagnostic::new("E2000", "msg")
        };
        assert_eq!(
            d.to_json(),
            r#"{"code":"E2000","message":"msg","line":null,"column":null,"suggestions":[{"message":"use 'x'","replacement":"x"}]}"#
        );
    }

    #[test]
    fn json_array_is_stable() {
        assert_eq!(to_json_array(&[]), "[]");
        assert_eq!(
            to_json_array(&[
                Diagnostic::new("E0001", "first"),
                Diagnostic::new("E1000", "second"),
            ]),
            r#"[{"code":"E0001","message":"first","line":null,"column":null,"suggestions":[]},{"code":"E1000","message":"second","line":null,"column":null,"suggestions":[]}]"#
        );
    }

    #[test]
    fn write_json_appends_to_existing_buffer() {
        let d = Diagnostic::new("E0001", "x");
        let mut out = String::from("prefix:");
        d.write_json(&mut out);
        assert_eq!(&out["prefix:".len()..], d.to_json());
    }

    #[test]
    fn serialization_is_deterministic() {
        let d = Diagnostic::from_display("type mismatch at 1:1", None);
        assert_eq!(d.to_json(), d.to_json());
    }

    #[test]
    fn parser_messages_classify_to_parse_codes() {
        let d = Diagnostic::from_display("Expected Semicolon, found Eof at 1:11", None);
        assert_eq!(d.code, errors::PARSE_EXPECTED_SEMICOLON);
    }
}
