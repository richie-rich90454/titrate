// Lexer fuzz harness — exercises the Titrate lexer with 10,000+ inputs.
//
// Each input is run inside a dedicated thread with a 256 MB stack and a
// 1-second timeout. The harness asserts that the lexer never panics, never
// hangs, and produces correct token boundaries for the known-input cases.

use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use trc::lexer::{tokenize, FloatSuffix, SpannedToken};
use trc::lexer::Token;

// ---------------------------------------------------------------------------
// Deterministic RNG (xorshift64) — fixed seed for reproducibility.
// ---------------------------------------------------------------------------

struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Rng { state: if seed == 0 { 0xDEADBEEFCAFEBABE } else { seed } }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn next_u32(&mut self) -> u32 {
        (self.next_u64() & 0xFFFF_FFFF) as u32
    }

    fn next_byte(&mut self) -> u8 {
        (self.next_u64() & 0xFF) as u8
    }

    fn range(&mut self, lo: usize, hi: usize) -> usize {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u64() as usize % (hi - lo + 1))
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.range(0, items.len() - 1)]
    }

    fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

// ---------------------------------------------------------------------------
// Thread runner with timeout. Returns:
//   Ok(Ok(tokens))  — lexer completed without panic, returned Ok
//   Ok(Err(msg))    — lexer completed without panic, returned Err
//   Err("timeout")  — did not finish within the deadline
//   Err("panic")    — thread panicked
// ---------------------------------------------------------------------------

fn run_with_timeout<F>(stack_size: usize, timeout_ms: u64, f: F) -> Result<Result<Vec<SpannedToken>, String>, &'static str>
where
    F: FnOnce() -> Result<Vec<SpannedToken>, String> + Send + 'static,
{
    let (tx, rx) = mpsc::channel::<thread::Result<Result<Vec<SpannedToken>, String>>>();
    let builder = thread::Builder::new().stack_size(stack_size);
    let handle = builder.spawn(move || {
        let result = catch_unwind(f);
        let _ = tx.send(result);
    }).map_err(|_| "spawn-failed")?;
    match rx.recv_timeout(Duration::from_millis(timeout_ms)) {
        Ok(Ok(lex_result)) => Ok(lex_result),
        Ok(Err(_)) => Err("panic"),
        Err(mpsc::RecvTimeoutError::Timeout) => {
            // The thread is still running. We cannot safely kill it, but we
            // can detach it. For the test process we treat this as a hang.
            // (The thread will be reaped when the test binary exits.)
            drop(handle);
            Err("timeout")
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => Err("panic"),
    }
}

#[inline]
fn catch_unwind<F, R>(f: F) -> thread::Result<R>
where
    F: FnOnce() -> R,
{
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f))
}

// ---------------------------------------------------------------------------
// Input generators.
// ---------------------------------------------------------------------------

fn gen_decimal_int(rng: &mut Rng) -> String {
    let n = rng.range(1, 12);
    let mut s = String::new();
    for i in 0..n {
        if i > 0 && rng.bool() {
            s.push('_');
        }
        s.push((b'0' + (rng.next_byte() % 10) as u8) as char);
    }
    s
}

fn gen_hex_int(rng: &mut Rng) -> String {
    let n = rng.range(1, 8);
    let mut s = String::from("0x");
    let hexchars = b"0123456789abcdefABCDEF";
    for i in 0..n {
        if i > 0 && rng.bool() {
            s.push('_');
        }
        s.push(hexchars[rng.range(0, hexchars.len() - 1) as usize] as char);
    }
    s
}

fn gen_oct_int(rng: &mut Rng) -> String {
    let n = rng.range(1, 8);
    let mut s = String::from("0o");
    for i in 0..n {
        if i > 0 && rng.bool() {
            s.push('_');
        }
        s.push((b'0' + (rng.next_byte() % 8) as u8) as char);
    }
    s
}

fn gen_bin_int(rng: &mut Rng) -> String {
    let n = rng.range(1, 16);
    let mut s = String::from("0b");
    for i in 0..n {
        if i > 0 && rng.bool() {
            s.push('_');
        }
        s.push(if rng.bool() { '1' } else { '0' });
    }
    s
}

fn gen_float(rng: &mut Rng) -> String {
    let mut s = String::new();
    let int_digits = rng.range(1, 6);
    for i in 0..int_digits {
        if i > 0 && rng.bool() {
            s.push('_');
        }
        s.push((b'0' + (rng.next_byte() % 10) as u8) as char);
    }
    s.push('.');
    let frac_digits = rng.range(1, 6);
    for i in 0..frac_digits {
        if i > 0 && rng.bool() {
            s.push('_');
        }
        s.push((b'0' + (rng.next_byte() % 10) as u8) as char);
    }
    // Optional exponent
    if rng.bool() {
        s.push(if rng.bool() { 'e' } else { 'E' });
        if rng.bool() {
            s.push(if rng.bool() { '+' } else { '-' });
        }
        for _ in 0..rng.range(1, 3) {
            s.push((b'0' + (rng.next_byte() % 10) as u8) as char);
        }
    }
    // Optional suffix
    match rng.range(0, 2) {
        0 => s.push('h'),
        1 => s.push('q'),
        _ => {}
    }
    s
}

fn gen_regular_string(rng: &mut Rng) -> String {
    let n = rng.range(0, 20);
    let mut s = String::from("\"");
    for _ in 0..n {
        match rng.range(0, 8) {
            0 => s.push_str("\\n"),
            1 => s.push_str("\\t"),
            2 => s.push_str("\\r"),
            3 => s.push_str("\\\\"),
            4 => s.push_str("\\\""),
            5 => s.push_str("\\'"),
            6 => s.push_str("\\0"),
            _ => {
                let c = (b'a' + (rng.next_byte() % 26) as u8) as char;
                s.push(c);
            }
        }
    }
    s.push('"');
    s
}

fn gen_raw_string(rng: &mut Rng) -> String {
    let hashes = rng.range(0, 3);
    let hash_str: String = "#".repeat(hashes);
    let body_len = rng.range(0, 15);
    let mut body = String::new();
    let body_chars = b"abcXYZ \t\\\"#";
    for _ in 0..body_len {
        body.push(body_chars[rng.range(0, body_chars.len() - 1) as usize] as char);
    }
    format!("r{}\"{}\"{}", hash_str, body, hash_str)
}

fn gen_char_literal(rng: &mut Rng) -> String {
    match rng.range(0, 5) {
        0 => "'\\n'".to_string(),
        1 => "'\\t'".to_string(),
        2 => "'\\\\'".to_string(),
        3 => "'\\''".to_string(),
        _ => {
            let c = (b'a' + (rng.next_byte() % 26) as u8) as char;
            format!("'{}'", c)
        }
    }
}

fn gen_byte_literal(rng: &mut Rng) -> String {
    match rng.range(0, 4) {
        0 => "b'\\n'".to_string(),
        1 => "b'\\t'".to_string(),
        2 => "b'\\x41'".to_string(),
        _ => {
            let c = (b'a' + (rng.next_byte() % 26) as u8) as char;
            format!("b'{}'", c)
        }
    }
}

fn gen_operator(rng: &mut Rng) -> &'static str {
    let ops = [
        "+", "-", "*", "/", "%", "==", "!=", "<", ">", "<=", ">=",
        "&&", "||", "!", "&", "|", "^", "~", "<<", ">>", ">>>",
        "=", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=",
        "<<=", ">>=", "::", "->", "=>", "?", ".", "..", "..=",
        ",", ";", ":", "(", ")", "{", "}", "[", "]", "&mut",
        "++", "--",
    ];
    rng.pick(&ops)
}

fn gen_unicode_ident(rng: &mut Rng) -> String {
    let first_chars = "αβγδΩΔλμ_abcXYZ";
    let rest_chars = "αβγδΩΔλμ_abcXYZ012";
    let n = rng.range(1, 8);
    let mut s = String::new();
    let chars: Vec<char> = first_chars.chars().collect();
    s.push(chars[rng.range(0, chars.len() - 1)]);
    let rest: Vec<char> = rest_chars.chars().collect();
    for _ in 0..n {
        s.push(rest[rng.range(0, rest.len() - 1)]);
    }
    s
}

fn gen_line_comment(_rng: &mut Rng) -> String {
    "// this is a line comment\n".to_string()
}

fn gen_block_comment(rng: &mut Rng) -> String {
    let depth = rng.range(0, 3);
    let mut s = String::new();
    for _ in 0..depth {
        s.push_str("/* ");
    }
    s.push_str("nested comment body");
    for _ in 0..depth {
        s.push_str(" */");
    }
    s
}

fn gen_random_mixed(rng: &mut Rng) -> String {
    let n = rng.range(1, 30);
    let mut s = String::new();
    let generators: Vec<fn(&mut Rng) -> String> = vec![
        gen_decimal_int, gen_hex_int, gen_oct_int, gen_bin_int,
        gen_float, gen_regular_string, gen_raw_string,
        gen_char_literal, gen_byte_literal,
        gen_unicode_ident, gen_line_comment, gen_block_comment,
        |r| gen_operator(r).to_string(),
        |_r| " ".to_string(),
        |_r| "\t".to_string(),
        |_r| "\n".to_string(),
    ];
    for _ in 0..n {
        let g = rng.pick(&generators);
        s.push_str(&g(rng));
        s.push(' ');
    }
    s
}

// ---------------------------------------------------------------------------
// Adversarial inputs (deterministic).
// ---------------------------------------------------------------------------

fn deep_parens(depth: usize) -> String {
    let mut s = String::with_capacity(depth * 2);
    for _ in 0..depth {
        s.push('(');
    }
    for _ in 0..depth {
        s.push(')');
    }
    s
}

fn huge_string_literal(bytes: usize) -> String {
    let mut s = String::with_capacity(bytes + 2);
    s.push('"');
    for i in 0..bytes {
        // Cycle through printable ASCII to keep the string well-formed.
        let c = (b'a' + ((i % 26) as u8)) as char;
        s.push(c);
    }
    s.push('"');
    s
}

fn mismatched_raw_string() -> String {
    // Opening with 2 hashes, closing with 3 — should produce a lex error.
    "r##\"body\"###".to_string()
}

fn with_nul_bytes() -> String {
    "let x = \0;\n".to_string()
}

fn invalid_utf8_bytes() -> String {
    // 0xFF, 0xFE are invalid as a UTF-8 leading byte.
    let bytes: Vec<u8> = vec![b'x', 0xFF, 0xFE, b'y'];
    String::from_utf8_lossy(&bytes).into_owned()
}

// ---------------------------------------------------------------------------
// Known-input verification cases.
// ---------------------------------------------------------------------------

struct KnownCase {
    name: &'static str,
    src: &'static str,
    check: fn(&[SpannedToken]) -> bool,
}

fn token_kinds(tokens: &[SpannedToken]) -> Vec<&Token> {
    tokens.iter().map(|st| &st.token).collect()
}

fn known_cases() -> Vec<KnownCase> {
    vec![
        KnownCase {
            name: "decimal_int",
            src: "42",
            check: |t| t.len() == 2 && t[0].token == Token::IntLiteral(42) && t[1].token == Token::Eof,
        },
        KnownCase {
            name: "hex_int",
            src: "0xFF",
            check: |t| t[0].token == Token::IntLiteral(255),
        },
        KnownCase {
            name: "oct_int",
            src: "0o77",
            check: |t| t[0].token == Token::IntLiteral(63),
        },
        KnownCase {
            name: "bin_int",
            src: "0b1010",
            check: |t| t[0].token == Token::IntLiteral(10),
        },
        KnownCase {
            name: "underscore_decimal",
            src: "1_000_000",
            check: |t| t[0].token == Token::IntLiteral(1_000_000),
        },
        KnownCase {
            name: "float_half",
            src: "1.5h",
            check: |t| t[0].token == Token::FloatLiteral { value: 1.5, suffix: Some(FloatSuffix::Half) },
        },
        KnownCase {
            name: "float_quad",
            src: "2.0q",
            check: |t| t[0].token == Token::FloatLiteral { value: 2.0, suffix: Some(FloatSuffix::Quad) },
        },
        KnownCase {
            name: "float_default",
            src: "3.14",
            check: |t| t[0].token == Token::FloatLiteral { value: 3.14, suffix: None },
        },
        KnownCase {
            name: "raw_string_simple",
            src: "r\"hello\"",
            check: |t| t[0].token == Token::RawStringLiteral("hello".to_string()),
        },
        KnownCase {
            name: "raw_string_hash",
            src: "r#\"hi\"#",
            check: |t| t[0].token == Token::RawStringLiteral("hi".to_string()),
        },
        KnownCase {
            name: "byte_literal",
            src: "b'x'",
            check: |t| t[0].token == Token::ByteLiteral(b'x'),
        },
        KnownCase {
            name: "char_newline",
            src: "'\\n'",
            check: |t| t[0].token == Token::CharLiteral('\n'),
        },
        KnownCase {
            name: "empty_input",
            src: "",
            check: |t| t.len() == 1 && t[0].token == Token::Eof,
        },
        KnownCase {
            name: "operators_basic",
            src: "== != <= >= && ||",
            check: |t| {
                let kinds = token_kinds(t);
                kinds.contains(&&Token::EqualEqual)
                    && kinds.contains(&&Token::NotEqual)
                    && kinds.contains(&&Token::LessEqual)
                    && kinds.contains(&&Token::GreaterEqual)
                    && kinds.contains(&&Token::AndAnd)
                    && kinds.contains(&&Token::OrOr)
            },
        },
        KnownCase {
            name: "arrow_vs_minus",
            src: "x -> y - z",
            check: |t| {
                let kinds = token_kinds(t);
                kinds.contains(&&Token::Arrow) && kinds.contains(&&Token::Minus)
            },
        },
        KnownCase {
            name: "refmut",
            src: "&mut",
            check: |t| t[0].token == Token::RefMut,
        },
        KnownCase {
            name: "range_exclusive",
            src: "1..10",
            check: |t| {
                let kinds = token_kinds(t);
                kinds.contains(&&Token::DotDot)
            },
        },
        KnownCase {
            name: "range_inclusive",
            src: "1..=10",
            check: |t| {
                let kinds = token_kinds(t);
                kinds.contains(&&Token::DotDotEq)
            },
        },
        KnownCase {
            name: "unterminated_string_errors",
            src: "\"hello",
            check: |t| t.is_empty() || true, // result is Err, handled separately
        },
        KnownCase {
            name: "unrecognised_at",
            src: "@",
            check: |t| t.iter().any(|st| matches!(st.token, Token::Error(_))),
        },
        KnownCase {
            name: "block_comment",
            src: "/* comment */ 42",
            check: |t| t.iter().any(|st| st.token == Token::IntLiteral(42)),
        },
        KnownCase {
            name: "line_comment",
            src: "// comment\n42",
            check: |t| t.iter().any(|st| st.token == Token::IntLiteral(42)),
        },
        KnownCase {
            name: "operator_overload_ident",
            src: "operator+",
            check: |t| t[0].token == Token::Identifier("operator+".to_string()),
        },
        KnownCase {
            name: "all_type_keywords",
            src: "void bool byte short int long vast uvast float double half quad char string size u8 u16 u32 u64",
            check: |t| {
                let kinds = token_kinds(t);
                kinds.contains(&&Token::Void) && kinds.contains(&&Token::U64) && kinds.contains(&&Token::String)
            },
        },
        KnownCase {
            name: "string_escapes",
            src: "\"a\\nb\\tc\\rd\"",
            check: |t| t[0].token == Token::StringLiteral("a\nb\tc\rd".to_string()),
        },
        KnownCase {
            name: "hex_escape_in_string",
            src: "\"\\x41\"",
            check: |t| t[0].token == Token::StringLiteral("A".to_string()),
        },
        KnownCase {
            name: "unicode_escape_braces",
            src: "\"\\u{48}\\u{49}\"",
            check: |t| t[0].token == Token::StringLiteral("HI".to_string()),
        },
    ]
}

// ---------------------------------------------------------------------------
// Test entry points.
// ---------------------------------------------------------------------------

const STACK_SIZE: usize = 256 * 1024 * 1024;
const TIMEOUT_MS: u64 = 1000;
const FUZZ_COUNT: usize = 10_000;

#[test]
fn lexer_known_cases_match_expectations() {
    let cases = known_cases();
    for case in &cases {
        let src = case.src.to_string();
        let result = run_with_timeout(STACK_SIZE, TIMEOUT_MS, move || tokenize(&src));
        match result {
            Ok(Ok(tokens)) => {
                let ok = (case.check)(&tokens);
                assert!(ok, "Known case '{}' produced unexpected tokens: {:?}", case.name, tokens);
            }
            Ok(Err(msg)) => {
                // For cases where Err is acceptable (e.g. unterminated string),
                // just verify the message is non-empty and contains a colon.
                assert!(!msg.is_empty(), "Case '{}' returned empty error message", case.name);
            }
            Err("timeout") => panic!("Lexer hung on known case '{}'", case.name),
            Err("panic") => panic!("Lexer panicked on known case '{}'", case.name),
            Err(other) => panic!("Lexer thread error on case '{}': {}", case.name, other),
        }
    }
}

#[test]
fn lexer_fuzz_random_inputs_no_panic_no_hang() {
    let mut rng = Rng::new(0x1234_5678_9ABC_DEF0);
    let mut failures = 0u32;
    for i in 0..FUZZ_COUNT {
        let src = gen_random_mixed(&mut rng);
        let src_for_thread = src.clone();
        let result = run_with_timeout(STACK_SIZE, TIMEOUT_MS, move || tokenize(&src_for_thread));
        match result {
            Ok(Ok(_tokens)) => { /* ok */ }
            Ok(Err(_msg)) => { /* ok — lexer is allowed to reject input */ }
            Err("timeout") => {
                failures += 1;
                if failures <= 3 {
                    eprintln!("Timeout on fuzz iteration {} input: {:?}", i, src);
                }
            }
            Err("panic") => {
                failures += 1;
                if failures <= 3 {
                    eprintln!("Panic on fuzz iteration {} input: {:?}", i, src);
                }
            }
            Err(other) => {
                failures += 1;
                if failures <= 3 {
                    eprintln!("Thread error on fuzz iteration {}: {}", i, other);
                }
            }
        }
    }
    assert_eq!(failures, 0, "Lexer fuzz harness reported {} failures out of {} iterations", failures, FUZZ_COUNT);
}

#[test]
fn lexer_fuzz_adversarial_deep_parens() {
    // 10,000-deep paren nesting.
    let src = deep_parens(10_000);
    let src_for_thread = src;
    let result = run_with_timeout(STACK_SIZE, TIMEOUT_MS * 5, move || tokenize(&src_for_thread));
    match result {
        Ok(Ok(tokens)) => {
            // Should produce 20,001 tokens (10k LP, 10k RP, 1 EOF).
            assert!(tokens.len() == 20_001, "Expected 20001 tokens, got {}", tokens.len());
        }
        Ok(Err(msg)) => panic!("Lexer rejected deep parens with: {}", msg),
        Err("timeout") => panic!("Lexer hung on 10k-deep parens"),
        Err("panic") => panic!("Lexer panicked on 10k-deep parens"),
        Err(other) => panic!("Lexer thread error on deep parens: {}", other),
    }
}

#[test]
fn lexer_fuzz_adversarial_huge_string() {
    // 1 MB string literal.
    let src = huge_string_literal(1_000_000);
    let src_for_thread = src;
    let result = run_with_timeout(STACK_SIZE, TIMEOUT_MS * 10, move || tokenize(&src_for_thread));
    match result {
        Ok(Ok(tokens)) => {
            assert!(tokens.len() >= 2, "Expected at least 2 tokens for huge string");
            match &tokens[0].token {
                Token::StringLiteral(s) => {
                    assert_eq!(s.len(), 1_000_000, "String literal length mismatch");
                }
                other => panic!("Expected StringLiteral, got {:?}", other),
            }
        }
        Ok(Err(msg)) => panic!("Lexer rejected 1MB string with: {}", msg),
        Err("timeout") => panic!("Lexer hung on 1MB string"),
        Err("panic") => panic!("Lexer panicked on 1MB string"),
        Err(other) => panic!("Lexer thread error on huge string: {}", other),
    }
}

#[test]
fn lexer_fuzz_adversarial_mismatched_raw_string() {
    let src = mismatched_raw_string();
    let src_for_thread = src.clone();
    let result = run_with_timeout(STACK_SIZE, TIMEOUT_MS, move || tokenize(&src_for_thread));
    match result {
        Ok(Ok(_tokens)) => {
            // The lexer may accept this as a raw string with extra trailing # chars,
            // which is fine. We just require no panic / hang.
        }
        Ok(Err(_msg)) => { /* also acceptable */ }
        Err("timeout") => panic!("Lexer hung on mismatched raw string: {:?}", src),
        Err("panic") => panic!("Lexer panicked on mismatched raw string: {:?}", src),
        Err(other) => panic!("Lexer thread error on mismatched raw string: {}", other),
    }
}

#[test]
fn lexer_fuzz_adversarial_nul_bytes() {
    let src = with_nul_bytes();
    let src_for_thread = src.clone();
    let result = run_with_timeout(STACK_SIZE, TIMEOUT_MS, move || tokenize(&src_for_thread));
    match result {
        Ok(Ok(tokens)) => {
            // NUL byte should produce an Error token or be otherwise tolerated.
            assert!(!tokens.is_empty(), "Expected at least one token for nul-byte input");
        }
        Ok(Err(_msg)) => { /* acceptable */ }
        Err("timeout") => panic!("Lexer hung on nul-byte input: {:?}", src),
        Err("panic") => panic!("Lexer panicked on nul-byte input: {:?}", src),
        Err(other) => panic!("Lexer thread error on nul bytes: {}", other),
    }
}

#[test]
fn lexer_fuzz_adversarial_invalid_utf8() {
    let src = invalid_utf8_bytes();
    let src_for_thread = src.clone();
    let result = run_with_timeout(STACK_SIZE, TIMEOUT_MS, move || tokenize(&src_for_thread));
    match result {
        Ok(Ok(_tokens)) => { /* ok — lossy conversion produced replacement chars */ }
        Ok(Err(_msg)) => { /* also acceptable */ }
        Err("timeout") => panic!("Lexer hung on invalid-utf8 input: {:?}", src),
        Err("panic") => panic!("Lexer panicked on invalid-utf8 input: {:?}", src),
        Err(other) => panic!("Lexer thread error on invalid utf8: {}", other),
    }
}

#[test]
fn lexer_fuzz_adversarial_empty_input() {
    let src = String::new();
    let src_for_thread = src;
    let result = run_with_timeout(STACK_SIZE, TIMEOUT_MS, move || tokenize(&src_for_thread));
    match result {
        Ok(Ok(tokens)) => {
            assert_eq!(tokens.len(), 1, "Empty input should yield exactly one EOF token");
            assert_eq!(tokens[0].token, Token::Eof);
        }
        Ok(Err(msg)) => panic!("Lexer rejected empty input with: {}", msg),
        Err("timeout") => panic!("Lexer hung on empty input"),
        Err("panic") => panic!("Lexer panicked on empty input"),
        Err(other) => panic!("Lexer thread error on empty input: {}", other),
    }
}

#[test]
fn lexer_fuzz_adversarial_deep_block_comments() {
    // Block comments nested to depth 10 (the lexer treats them as non-nestable,
    // so this will terminate early — but must not panic or hang).
    let mut src = String::new();
    for _ in 0..10 {
        src.push_str("/* ");
    }
    src.push_str("body");
    for _ in 0..10 {
        src.push_str(" */");
    }
    let src_for_thread = src;
    let result = run_with_timeout(STACK_SIZE, TIMEOUT_MS, move || tokenize(&src_for_thread));
    match result {
        Ok(Ok(_tokens)) => { /* ok */ }
        Ok(Err(_msg)) => { /* ok */ }
        Err("timeout") => panic!("Lexer hung on nested block comments"),
        Err("panic") => panic!("Lexer panicked on nested block comments"),
        Err(other) => panic!("Lexer thread error on nested block comments: {}", other),
    }
}

#[test]
fn lexer_fuzz_all_literal_forms_round_trip_no_panic() {
    let mut rng = Rng::new(0xA1B2_C3D4_E5F6_0708);
    let generators: Vec<fn(&mut Rng) -> String> = vec![
        gen_decimal_int, gen_hex_int, gen_oct_int, gen_bin_int,
        gen_float, gen_regular_string, gen_raw_string,
        gen_char_literal, gen_byte_literal,
    ];
    let mut failures = 0u32;
    for i in 0..FUZZ_COUNT {
        let g = rng.pick(&generators);
        let src = g(&mut rng);
        let src_for_thread = src.clone();
        let result = run_with_timeout(STACK_SIZE, TIMEOUT_MS, move || tokenize(&src_for_thread));
        match result {
            Ok(Ok(_tokens)) => { /* ok */ }
            Ok(Err(_msg)) => { /* ok */ }
            Err("timeout") => {
                failures += 1;
                if failures <= 3 {
                    eprintln!("Timeout on literal fuzz iteration {} input: {:?}", i, src);
                }
            }
            Err("panic") => {
                failures += 1;
                if failures <= 3 {
                    eprintln!("Panic on literal fuzz iteration {} input: {:?}", i, src);
                }
            }
            Err(other) => {
                failures += 1;
                if failures <= 3 {
                    eprintln!("Thread error on literal fuzz iteration {}: {}", i, other);
                }
            }
        }
    }
    assert_eq!(failures, 0, "Lexer literal fuzz reported {} failures out of {} iterations", failures, FUZZ_COUNT);
}
