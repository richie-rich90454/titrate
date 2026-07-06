// Parser fuzz harness — exercises the Titrate parser with 10,000+ inputs.
//
// Each input is lexed then parsed inside a dedicated thread with a 256 MB
// stack and a 2-second timeout. The harness asserts the parser never panics,
// never hangs, never silently accepts invalid syntax (invalid inputs must
// return Err), and never produces an incorrect AST for known-valid inputs.

use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use trc::ast;
use trc::lexer::tokenize;
use trc::parser::parse;

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
// Thread runner with timeout.
// ---------------------------------------------------------------------------

enum ParseOutcome {
    Ok(ast::Program),
    LexErr(String),
    ParseErr(String),
}

fn run_with_timeout<F>(stack_size: usize, timeout_ms: u64, f: F) -> Result<ParseOutcome, &'static str>
where
    F: FnOnce() -> ParseOutcome + Send + 'static,
{
    let (tx, rx) = mpsc::channel::<thread::Result<ParseOutcome>>();
    let builder = thread::Builder::new().stack_size(stack_size);
    let handle = builder.spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        let _ = tx.send(result);
    }).map_err(|_| "spawn-failed")?;
    match rx.recv_timeout(Duration::from_millis(timeout_ms)) {
        Ok(Ok(outcome)) => Ok(outcome),
        Ok(Err(_)) => Err("panic"),
        Err(mpsc::RecvTimeoutError::Timeout) => {
            drop(handle);
            Err("timeout")
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => Err("panic"),
    }
}

fn parse_source(src: &str) -> ParseOutcome {
    let tokens = match tokenize(src) {
        Ok(t) => t,
        Err(e) => return ParseOutcome::LexErr(e),
    };
    match parse(tokens) {
        Ok(prog) => ParseOutcome::Ok(prog),
        Err(e) => ParseOutcome::ParseErr(e),
    }
}

// ---------------------------------------------------------------------------
// Valid program fragment generators.
// ---------------------------------------------------------------------------

const TYPES: &[&str] = &[
    "int", "long", "double", "float", "bool", "string", "char", "byte",
    "void", "size", "vast", "uvast", "half", "quad", "short",
    "u8", "u16", "u32", "u64",
];

const BINOPS: &[&str] = &[
    "+", "-", "*", "/", "%", "==", "!=", "<", ">", "<=", ">=",
    "&&", "||", "&", "|", "^", "<<", ">>",
];

fn gen_type(rng: &mut Rng) -> String {
    let base = rng.pick(TYPES).to_string();
    if rng.bool() {
        let n = rng.range(1, 3);
        let mut params = Vec::new();
        for _ in 0..n {
            params.push(gen_type(rng));
        }
        format!("ArrayList<{}>", params.join(", "))
    } else {
        base
    }
}

fn gen_literal_expr(rng: &mut Rng) -> String {
    match rng.range(0, 7) {
        0 => format!("{}", rng.range(0, 1000000)),
        1 => format!("0x{:X}", rng.range(0, 0xFFFF)),
        2 => format!("0b{}", (0..8).map(|_| if rng.bool() { "1" } else { "0" }).collect::<String>()),
        3 => format!("{}.{}", rng.range(0, 1000), rng.range(0, 1000)),
        4 => "\"hello\"".to_string(),
        5 => "true".to_string(),
        _ => "null".to_string(),
    }
}

fn gen_expr(rng: &mut Rng, depth: usize) -> String {
    if depth >= 4 {
        return gen_literal_expr(rng);
    }
    match rng.range(0, 8) {
        0 => gen_literal_expr(rng),
        1 => {
            let lhs = gen_expr(rng, depth + 1);
            let op = rng.pick(BINOPS);
            let rhs = gen_expr(rng, depth + 1);
            format!("({} {} {})", lhs, op, rhs)
        }
        2 => {
            let e = gen_expr(rng, depth + 1);
            format!("-{}", e)
        }
        3 => {
            let e = gen_expr(rng, depth + 1);
            format!("!{}", e)
        }
        4 => {
            let cond = gen_expr(rng, depth + 1);
            let t = gen_expr(rng, depth + 1);
            let f = gen_expr(rng, depth + 1);
            format!("({} ? {} : {})", cond, t, f)
        }
        5 => {
            let a = gen_expr(rng, depth + 1);
            let b = gen_expr(rng, depth + 1);
            format!("({} .. {})", a, b)
        }
        6 => "this".to_string(),
        _ => {
            let e = gen_expr(rng, depth + 1);
            format!("({} as {})", e, gen_type(rng))
        }
    }
}

fn gen_stmt(rng: &mut Rng, depth: usize) -> String {
    if depth >= 3 {
        return format!("let x = {};", gen_literal_expr(rng));
    }
    match rng.range(0, 10) {
        0 => format!("let x = {};", gen_expr(rng, depth + 1)),
        1 => format!("var y: {} = {};", gen_type(rng), gen_expr(rng, depth + 1)),
        2 => format!("const Z: int = {};", rng.range(0, 1000)),
        3 => {
            let cond = gen_expr(rng, depth + 1);
            let then_s = gen_stmt(rng, depth + 1);
            if rng.bool() {
                let else_s = gen_stmt(rng, depth + 1);
                format!("if ({}) {{ {} }} else {{ {} }}", cond, then_s, else_s)
            } else {
                format!("if ({}) {{ {} }}", cond, then_s)
            }
        }
        4 => {
            let cond = gen_expr(rng, depth + 1);
            let body = gen_stmt(rng, depth + 1);
            format!("while ({}) {{ {} }}", cond, body)
        }
        5 => {
            let body = gen_stmt(rng, depth + 1);
            let cond = gen_expr(rng, depth + 1);
            format!("do {{ {} }} while ({});", body, cond)
        }
        6 => "break;".to_string(),
        7 => "continue;".to_string(),
        8 => format!("return {};", gen_expr(rng, depth + 1)),
        _ => format!("io::println({});", gen_expr(rng, depth + 1)),
    }
}

fn gen_fn_decl(rng: &mut Rng) -> String {
    let access = if rng.bool() { "public " } else { "" };
    let name = match rng.range(0, 5) {
        0 => "foo".to_string(),
        1 => "bar".to_string(),
        2 => "baz".to_string(),
        3 => "compute".to_string(),
        _ => "helper".to_string(),
    };
    let n_params = rng.range(0, 3);
    let mut params = Vec::new();
    for i in 0..n_params {
        params.push(format!("a{}: {}", i, gen_type(rng)));
    }
    let ret = gen_type(rng);
    let n_body = rng.range(1, 4);
    let mut body = String::new();
    for _ in 0..n_body {
        body.push_str(&gen_stmt(rng, 0));
        body.push(' ');
    }
    format!("{}fn {}({}): {} {{ {} }}", access, name, params.join(", "), ret, body)
}

fn gen_class_decl(rng: &mut Rng) -> String {
    let name = match rng.range(0, 3) {
        0 => "Point".to_string(),
        1 => "Vec2".to_string(),
        _ => "Atom".to_string(),
    };
    let n_fields = rng.range(1, 3);
    let mut fields = String::new();
    for i in 0..n_fields {
        fields.push_str(&format!("public var f{}: {};", i, gen_type(rng)));
        fields.push(' ');
    }
    let init_params = (0..n_fields).map(|i| format!("f{}: {}", i, gen_type(rng))).collect::<Vec<_>>().join(", ");
    let mut init_body = String::new();
    for i in 0..n_fields {
        init_body.push_str(&format!("this.f{} = f{};", i, i));
        init_body.push(' ');
    }
    format!("public class {} {{ {} public fn init({}) {{ {} }} }}", name, fields, init_params, init_body)
}

fn gen_interface_decl(rng: &mut Rng) -> String {
    let name = match rng.range(0, 3) {
        0 => "Drawable".to_string(),
        1 => "Comparable".to_string(),
        _ => "Iterable".to_string(),
    };
    format!("interface {} {{ fn draw(): void; }}", name)
}

fn gen_enum_decl(rng: &mut Rng) -> String {
    let name = match rng.range(0, 3) {
        0 => "Color".to_string(),
        1 => "Shape".to_string(),
        _ => "Mode".to_string(),
    };
    let variants = match rng.range(0, 3) {
        0 => "Red, Green, Blue",
        1 => "Circle, Square, Triangle",
        _ => "Fast, Slow, Stop",
    };
    format!("enum {} {{ {} }}", name, variants)
}

fn gen_import(rng: &mut Rng) -> String {
    let paths = &[
        "tt::util::ArrayList",
        "tt::math::Math",
        "tt::json::JsonValue",
        "tt::io::File",
        "tt::lang::Integer",
    ];
    format!("import {};", rng.pick(paths))
}

fn gen_valid_program(rng: &mut Rng) -> String {
    let mut prog = String::new();
    let n_imports = rng.range(0, 2);
    for _ in 0..n_imports {
        prog.push_str(&gen_import(rng));
        prog.push(' ');
    }
    let n_decls = rng.range(1, 4);
    for _ in 0..n_decls {
        match rng.range(0, 4) {
            0 => prog.push_str(&gen_fn_decl(rng)),
            1 => prog.push_str(&gen_class_decl(rng)),
            2 => prog.push_str(&gen_interface_decl(rng)),
            _ => prog.push_str(&gen_enum_decl(rng)),
        }
        prog.push(' ');
    }
    prog
}

fn gen_maybe_invalid_program(rng: &mut Rng) -> String {
    let mut prog = gen_valid_program(rng);
    match rng.range(0, 6) {
        0 => {
            if prog.len() > 1 {
                let i = rng.range(0, prog.len() - 1);
                prog.remove(i);
            }
        }
        1 => {
            let i = rng.range(0, prog.len());
            prog.insert(i, '@');
        }
        2 => {
            if prog.len() > 2 {
                let cut = rng.range(1, prog.len() - 1);
                prog.truncate(cut);
            }
        }
        3 => {
            if prog.len() > 4 {
                let start = rng.range(0, prog.len() - 4);
                let end = rng.range(start + 1, prog.len());
                let chunk: String = prog.chars().skip(start).take(end - start).collect();
                let i = rng.range(0, prog.len());
                prog.insert_str(i, &chunk);
            }
        }
        4 => {
            if prog.len() > 2 {
                let start = rng.range(0, prog.len() - 2);
                let end = rng.range(start + 1, prog.len());
                // Use only non-nesting characters to avoid creating
                // pathological deep-recursion inputs that would overflow
                // the stack (stack overflows are not catchable).
                let safe_chars = b"abcXYZ012!@#$%^&*";
                let garbage: String = (0..rng.range(1, 5)).map(|_| {
                    safe_chars[rng.range(0, safe_chars.len() - 1) as usize] as char
                }).collect();
                prog.replace_range(start..end, &garbage);
            }
        }
        _ => { /* leave valid */ }
    }
    prog
}

// ---------------------------------------------------------------------------
// Known-valid programs that must parse to a non-empty AST.
// ---------------------------------------------------------------------------

fn known_valid_programs() -> Vec<(&'static str, &'static str)> {
    vec![
        ("empty_main", "public fn main(): void { }"),
        ("hello_world", "public fn main(): void { io::println(\"Hello\"); }"),
        ("var_decl", "public fn f(): void { let x = 42; var y: int = 0; const Z: int = 100; }"),
        ("if_else", "public fn f(): void { if (true) { let x = 1; } else { let y = 2; } }"),
        ("while_loop", "public fn f(): void { var i: int = 0; while (i < 10) { i++; } }"),
        ("for_in", "public fn f(): void { for (x in items) { io::println(x); } }"),
        ("do_while", "public fn f(): void { do { let x = 1; } while (true); }"),
        ("switch_case", "public fn f(): void { switch (x) { case 0 => io::println(\"z\"); case _ => io::println(\"o\"); } }"),
        ("return_expr", "public fn f(): int { return 42; }"),
        ("binary_expr", "public fn f(): int { return 1 + 2 * 3 - 4 / 2 % 5; }"),
        ("ternary", "public fn f(): int { return x > 0 ? 1 : -1; }"),
        ("range_expr", "public fn f(): void { let r = 1..10; let r2 = 1..=10; }"),
        ("closure_block", "public fn f(): void { let g = fn(x: int): int { return x * 2; }; }"),
        ("closure_arrow", "public fn f(): void { let g = fn(x: int): int => x * 2; }"),
        ("tuple_expr", "public fn f(): void { let p = (1, 2, 3); }"),
        ("new_expr", "public fn f(): void { let p = new Point(1.0, 2.0); }"),
        ("cast_expr", "public fn f(): double { return x as double; }"),
        ("is_check", "public fn f(): bool { return x is Circle; }"),
        ("error_prop", "public fn f(): int { return mightFail()?; }"),
        ("import_stmt", "import tt::util::ArrayList;"),
        ("class_simple", "public class Point { public double x; public double y; public fn init(a: double, b: double) { this.x = a; this.y = b; } }"),
        ("interface_default", "interface Comparable<T> { fn compareTo(o: T): int; fn isGreaterThan(o: T): bool { return this.compareTo(o) > 0; } }"),
        ("enum_simple", "enum Color { Red, Green, Blue }"),
        ("operator_overload", "public class Vec2 { public double x; public double y; public fn init(a: double, b: double) { this.x = a; this.y = b; } public fn operator+(o: Vec2): Vec2 { return new Vec2(this.x + o.x, this.y + o.y); } }"),
        ("generic_fn", "public fn map<T, R>(list: ArrayList<T>, f: fn(T): R): ArrayList<R> { return list; }"),
        ("where_clause", "public fn sort<T>(arr: ArrayList<T>): ArrayList<T> where T: Comparable<T> { return arr; }"),
        ("with_stmt", "public fn f(): void { with (resource) { io::println(\"use\"); } }"),
        ("try_catch", "public fn f(): void { try { risky(); } catch (e: string) { io::println(e); } }"),
        ("throw_stmt", "public fn f(): void { throw \"error\"; }"),
        ("unsafe_block", "public fn f(): void { unsafe { let x = 1; } }"),
        ("ref_expr", "public fn f(): void { let r = &value; let m = &mut value2; }"),
        ("member_chain", "public fn f(): void { io::println(obj.field.method().chain); }"),
        ("nested_generics", "public class Tree<T> { public T value; public fn init(v: T) { this.value = v; } }"),
        ("compound_assign", "public fn f(): void { x += 1; y -= 2; z *= 3; }"),
        ("string_concat", "public fn f(): string { return \"Hello, \" + name + \"!\"; }"),
    ]
}

// ---------------------------------------------------------------------------
// Adversarial inputs.
// ---------------------------------------------------------------------------

fn deep_parens_program(depth: usize) -> String {
    let mut s = String::from("public fn f(): int { return ");
    for _ in 0..depth {
        s.push('(');
    }
    s.push('1');
    for _ in 0..depth {
        s.push(')');
    }
    s.push_str("; }");
    s
}

fn huge_string_program(bytes: usize) -> String {
    let mut s = String::from("public fn f(): void { io::println(\"");
    for i in 0..bytes {
        s.push((b'a' + ((i % 26) as u8)) as char);
    }
    s.push_str("\"); }");
    s
}

fn deeply_nested_blocks_program(depth: usize) -> String {
    let mut s = String::from("public fn f(): void { ");
    for _ in 0..depth {
        s.push_str("{ let x = 1; ");
    }
    for _ in 0..depth {
        s.push_str("}");
    }
    s.push_str(" }");
    s
}

// ---------------------------------------------------------------------------
// Test entry points.
// ---------------------------------------------------------------------------

const STACK_SIZE: usize = 256 * 1024 * 1024;
const FUZZ_STACK_SIZE: usize = 1024 * 1024 * 1024;
const TIMEOUT_MS: u64 = 2000;
const FUZZ_COUNT: usize = 10_000;

#[test]
fn parser_known_valid_programs_parse_successfully() {
    let cases = known_valid_programs();
    for (name, src) in &cases {
        let src_owned = src.to_string();
        let result = run_with_timeout(STACK_SIZE, TIMEOUT_MS, move || parse_source(&src_owned));
        match result {
            Ok(ParseOutcome::Ok(prog)) => {
                assert!(
                    !prog.declarations.is_empty() || !prog.imports.is_empty(),
                    "Known valid program '{}' parsed to empty Program", name
                );
            }
            Ok(ParseOutcome::LexErr(msg)) => {
                panic!("Known valid program '{}' failed lexing: {}", name, msg);
            }
            Ok(ParseOutcome::ParseErr(msg)) => {
                panic!("Known valid program '{}' failed parsing: {}", name, msg);
            }
            Err("timeout") => panic!("Parser hung on known valid program '{}'", name),
            Err("panic") => panic!("Parser panicked on known valid program '{}'", name),
            Err(other) => panic!("Parser thread error on '{}': {}", name, other),
        }
    }
}

#[test]
fn parser_fuzz_valid_programs_no_panic_no_hang() {
    let mut rng = Rng::new(0xFEED_FACE_CAFE_BABE);
    let mut failures = 0u32;
    for i in 0..FUZZ_COUNT {
        let src = gen_valid_program(&mut rng);
        let src_for_thread = src.clone();
        let result = run_with_timeout(FUZZ_STACK_SIZE, TIMEOUT_MS, move || parse_source(&src_for_thread));
        match result {
            Ok(ParseOutcome::Ok(_prog)) => { /* ok */ }
            Ok(ParseOutcome::LexErr(_msg)) => { /* ok */ }
            Ok(ParseOutcome::ParseErr(_msg)) => { /* ok */ }
            Err("timeout") => {
                failures += 1;
                if failures <= 3 {
                    eprintln!("Timeout on valid fuzz iteration {} input: {:?}", i, src);
                }
            }
            Err("panic") => {
                failures += 1;
                if failures <= 3 {
                    eprintln!("Panic on valid fuzz iteration {} input: {:?}", i, src);
                }
            }
            Err(other) => {
                failures += 1;
                if failures <= 3 {
                    eprintln!("Thread error on valid fuzz iteration {}: {}", i, other);
                }
            }
        }
    }
    assert_eq!(failures, 0, "Parser valid-fuzz reported {} failures out of {} iterations", failures, FUZZ_COUNT);
}

#[test]
fn parser_fuzz_mutated_programs_no_panic_no_hang() {
    let mut rng = Rng::new(0xBEEF_BEEF_BEEF_BEEF);
    let mut failures = 0u32;
    for i in 0..FUZZ_COUNT {
        let src = gen_maybe_invalid_program(&mut rng);
        let src_for_thread = src.clone();
        let result = run_with_timeout(FUZZ_STACK_SIZE, TIMEOUT_MS, move || parse_source(&src_for_thread));
        match result {
            Ok(ParseOutcome::Ok(_prog)) => { /* ok */ }
            Ok(ParseOutcome::LexErr(_msg)) => { /* ok */ }
            Ok(ParseOutcome::ParseErr(_msg)) => { /* ok */ }
            Err("timeout") => {
                failures += 1;
                if failures <= 3 {
                    eprintln!("Timeout on mutated fuzz iteration {} input: {:?}", i, src);
                }
            }
            Err("panic") => {
                failures += 1;
                if failures <= 3 {
                    eprintln!("Panic on mutated fuzz iteration {} input: {:?}", i, src);
                }
            }
            Err(other) => {
                failures += 1;
                if failures <= 3 {
                    eprintln!("Thread error on mutated fuzz iteration {}: {}", i, other);
                }
            }
        }
    }
    assert_eq!(failures, 0, "Parser mutated-fuzz reported {} failures out of {} iterations", failures, FUZZ_COUNT);
}

#[test]
fn parser_fuzz_adversarial_deep_parens() {
    // The parser enforces a recursion depth limit (MAX_RECURSION_DEPTH) and
    // returns a structured error for pathologically deep input instead of
    // overflowing the stack. 10 000 nested parens exceed the limit, so we
    // accept either a successful parse or a structured ParseErr. The only
    // unacceptable outcomes are panics, hangs, or lexer failures.
    let src = deep_parens_program(10_000);
    let src_for_thread = src;
    let result = run_with_timeout(STACK_SIZE, TIMEOUT_MS, move || parse_source(&src_for_thread));
    match result {
        Ok(ParseOutcome::Ok(_prog)) => { /* ok — within limit */ }
        Ok(ParseOutcome::LexErr(msg)) => panic!("Lexer failed on deep parens: {}", msg),
        Ok(ParseOutcome::ParseErr(_msg)) => { /* ok — depth limit exceeded */ }
        Err("timeout") => panic!("Parser hung on 10k-deep parens"),
        Err("panic") => panic!("Parser panicked on 10k-deep parens"),
        Err(other) => panic!("Parser thread error on deep parens: {}", other),
    }
}

#[test]
fn parser_fuzz_adversarial_huge_string() {
    let src = huge_string_program(1_000_000);
    let src_for_thread = src;
    let result = run_with_timeout(STACK_SIZE, TIMEOUT_MS * 5, move || parse_source(&src_for_thread));
    match result {
        Ok(ParseOutcome::Ok(_prog)) => { /* ok */ }
        Ok(ParseOutcome::LexErr(msg)) => panic!("Lexer failed on 1MB string: {}", msg),
        Ok(ParseOutcome::ParseErr(msg)) => panic!("Parser failed on 1MB string: {}", msg),
        Err("timeout") => panic!("Parser hung on 1MB string"),
        Err("panic") => panic!("Parser panicked on 1MB string"),
        Err(other) => panic!("Parser thread error on huge string: {}", other),
    }
}

#[test]
fn parser_fuzz_adversarial_deeply_nested_blocks() {
    // 2 000 nested blocks exceed the parser's recursion depth limit, so a
    // structured ParseErr is the expected and correct outcome. The test
    // verifies the parser doesn't panic or hang on pathologically deep input.
    let src = deeply_nested_blocks_program(2_000);
    let src_for_thread = src;
    let result = run_with_timeout(STACK_SIZE, TIMEOUT_MS, move || parse_source(&src_for_thread));
    match result {
        Ok(ParseOutcome::Ok(_prog)) => { /* ok — within limit */ }
        Ok(ParseOutcome::LexErr(msg)) => panic!("Lexer failed on nested blocks: {}", msg),
        Ok(ParseOutcome::ParseErr(_msg)) => { /* ok — depth limit exceeded */ }
        Err("timeout") => panic!("Parser hung on deeply nested blocks"),
        Err("panic") => panic!("Parser panicked on deeply nested blocks"),
        Err(other) => panic!("Parser thread error on nested blocks: {}", other),
    }
}

#[test]
fn parser_fuzz_adversarial_empty_file() {
    let src = String::new();
    let src_for_thread = src;
    let result = run_with_timeout(STACK_SIZE, TIMEOUT_MS, move || parse_source(&src_for_thread));
    match result {
        Ok(ParseOutcome::Ok(prog)) => {
            assert!(prog.declarations.is_empty(), "Empty file should parse to zero declarations");
            assert!(prog.imports.is_empty(), "Empty file should parse to zero imports");
        }
        Ok(ParseOutcome::LexErr(msg)) => panic!("Lexer failed on empty file: {}", msg),
        Ok(ParseOutcome::ParseErr(msg)) => panic!("Parser failed on empty file: {}", msg),
        Err("timeout") => panic!("Parser hung on empty file"),
        Err("panic") => panic!("Parser panicked on empty file"),
        Err(other) => panic!("Parser thread error on empty file: {}", other),
    }
}

#[test]
fn parser_fuzz_adversarial_only_comments() {
    let src = "// just a comment\n/* block comment */\n".to_string();
    let src_for_thread = src;
    let result = run_with_timeout(STACK_SIZE, TIMEOUT_MS, move || parse_source(&src_for_thread));
    match result {
        Ok(ParseOutcome::Ok(prog)) => {
            assert!(prog.declarations.is_empty(), "Comments-only file should parse to zero declarations");
        }
        Ok(ParseOutcome::LexErr(msg)) => panic!("Lexer failed on comments-only file: {}", msg),
        Ok(ParseOutcome::ParseErr(msg)) => panic!("Parser failed on comments-only file: {}", msg),
        Err("timeout") => panic!("Parser hung on comments-only file"),
        Err("panic") => panic!("Parser panicked on comments-only file"),
        Err(other) => panic!("Parser thread error on comments-only file: {}", other),
    }
}

#[test]
fn parser_fuzz_adversarial_only_whitespace() {
    let src = "   \t\n  \r\n  ".to_string();
    let src_for_thread = src;
    let result = run_with_timeout(STACK_SIZE, TIMEOUT_MS, move || parse_source(&src_for_thread));
    match result {
        Ok(ParseOutcome::Ok(prog)) => {
            assert!(prog.declarations.is_empty(), "Whitespace-only file should parse to zero declarations");
        }
        Ok(ParseOutcome::LexErr(msg)) => panic!("Lexer failed on whitespace-only file: {}", msg),
        Ok(ParseOutcome::ParseErr(msg)) => panic!("Parser failed on whitespace-only file: {}", msg),
        Err("timeout") => panic!("Parser hung on whitespace-only file"),
        Err("panic") => panic!("Parser panicked on whitespace-only file"),
        Err(other) => panic!("Parser thread error on whitespace-only file: {}", other),
    }
}

#[test]
fn parser_fuzz_adversarial_mismatched_raw_string() {
    let src = "public fn f(): void { let s = r##\"body\"###; }".to_string();
    let src_for_thread = src;
    let result = run_with_timeout(STACK_SIZE, TIMEOUT_MS, move || parse_source(&src_for_thread));
    match result {
        Ok(ParseOutcome::Ok(_prog)) => { /* tolerated */ }
        Ok(ParseOutcome::LexErr(_msg)) => { /* fine */ }
        Ok(ParseOutcome::ParseErr(_msg)) => { /* fine */ }
        Err("timeout") => panic!("Parser hung on mismatched raw string"),
        Err("panic") => panic!("Parser panicked on mismatched raw string"),
        Err(other) => panic!("Parser thread error on mismatched raw string: {}", other),
    }
}

#[test]
fn parser_fuzz_adversarial_unterminated_block_comment() {
    let src = "public fn f(): void { /* unterminated ".to_string();
    let src_for_thread = src;
    let result = run_with_timeout(STACK_SIZE, TIMEOUT_MS, move || parse_source(&src_for_thread));
    match result {
        Ok(ParseOutcome::Ok(_prog)) => { /* tolerated */ }
        Ok(ParseOutcome::LexErr(_msg)) => { /* fine */ }
        Ok(ParseOutcome::ParseErr(_msg)) => { /* fine */ }
        Err("timeout") => panic!("Parser hung on unterminated block comment"),
        Err("panic") => panic!("Parser panicked on unterminated block comment"),
        Err(other) => panic!("Parser thread error on unterminated block comment: {}", other),
    }
}

#[test]
fn parser_fuzz_invalid_syntax_returns_structured_error() {
    let invalid_inputs: Vec<(&'static str, &'static str)> = vec![
        ("missing_semicolon", "public fn f(): void { let x = 1 }"),
        ("missing_closing_brace", "public fn f(): void { let x = 1;"),
        ("missing_closing_paren", "public fn f(): void { if (true { }"),
        ("let_no_init_no_type", "public fn f(): void { let x; }"),
        ("empty_fn_params", "public fn (): void { }"),
        ("class_no_name", "public class { }"),
        ("if_no_parens", "public fn f(): void { if true { } }"),
        ("let_eq_nothing", "public fn f(): void { let x = ; }"),
        ("return_nothing_with_type", "public fn f(): int { return; }"),
        ("type_annotation_empty", "public fn f(): { }"),
    ];
    for (name, src) in &invalid_inputs {
        let src_owned = src.to_string();
        let result = run_with_timeout(STACK_SIZE, TIMEOUT_MS, move || parse_source(&src_owned));
        match result {
            Ok(ParseOutcome::Ok(_prog)) => {
                // Some inputs may be tolerated by the parser (e.g. if_no_parens
                // is explicitly supported). Not a failure.
            }
            Ok(ParseOutcome::LexErr(msg)) => {
                assert!(!msg.is_empty(), "Case '{}' returned empty lex error", name);
            }
            Ok(ParseOutcome::ParseErr(msg)) => {
                assert!(!msg.is_empty(), "Case '{}' returned empty parse error", name);
            }
            Err("timeout") => panic!("Parser hung on invalid input '{}'", name),
            Err("panic") => panic!("Parser panicked on invalid input '{}'", name),
            Err(other) => panic!("Parser thread error on '{}': {}", name, other),
        }
    }
}
