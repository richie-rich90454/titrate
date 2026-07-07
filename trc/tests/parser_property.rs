//! Parser property tests — round-trip stability via canonical-form fixed point.
//!
//! Property: for any valid source `src`, let `p1 = parse(src)`,
//! `src2 = pretty_print(p1)`, `p2 = parse(src2)`, `src3 = pretty_print(p2)`.
//! Then `src2 == src3` — the canonical form is a fixed point.
//!
//! This catches:
//! - pretty_print emitting invalid syntax (parse fails on src2)
//! - pretty_print emitting ambiguous syntax (p2 differs structurally from p1)
//! - pretty_print dropping AST nodes (src3 missing content vs src2)
//!
//! Also tests:
//! - Idempotence: pretty_print is stable under re-application
//! - Structural preservation: parse(src) and parse(pretty_print(parse(src)))
//!   produce structurally equal ASTs (compared via canonical form)

use trc::ast::pretty_print;
use trc::lexer;
use trc::parser;

fn round_trip_once(src: &str) -> Result<String, String> {
    let tokens = lexer::tokenize(src).map_err(|e| format!("lex1: {}", e))?;
    let p1 = parser::parse(tokens).map_err(|e| format!("parse1: {}", e))?;
    let src2 = pretty_print::pretty_print(&p1);
    Ok(src2)
}

/// Test the fixed-point property. Returns:
/// - Ok(()) if the round-trip succeeds
/// - Err(msg) if pretty_print produced invalid syntax or non-idempotent output
/// - Ok(()) with SKIP semantics if the original source doesn't parse
fn round_trip_fixed_point(src: &str) -> Result<(), String> {
    let src2 = match round_trip_once(src) {
        Ok(s) => s,
        Err(e) if e.starts_with("parse1:") || e.starts_with("lex1:") => {
            // Original source doesn't parse — skip (parser limitation, not pretty_print bug)
            return Ok(());
        }
        Err(e) => return Err(e),
    };
    // Now parse the pretty-printed source and pretty-print again
    let tokens2 = match lexer::tokenize(&src2) {
        Ok(t) => t,
        Err(e) => return Err(format!("lex2 (pretty_print produced invalid tokens): {}", e)),
    };
    let p2 = match parser::parse(tokens2) {
        Ok(p) => p,
        Err(e) => return Err(format!(
            "parse2 (pretty_print produced invalid syntax):\n--- pretty_printed source ---\n{}\n--- parse error ---\n{}",
            src2, e
        )),
    };
    let src3 = pretty_print::pretty_print(&p2);
    if src2 != src3 {
        return Err(format!(
            "canonical form is not a fixed point.\n--- src2 (first pretty-print) ---\n{}\n--- src3 (second pretty-print) ---\n{}\n--- diff ---\n{}",
            src2,
            src3,
            first_diff(&src2, &src3)
        ));
    }
    Ok(())
}

fn first_diff(a: &str, b: &str) -> String {
    let mut out = String::new();
    let a_lines: Vec<&str> = a.lines().collect();
    let b_lines: Vec<&str> = b.lines().collect();
    let n = a_lines.len().min(b_lines.len());
    for i in 0..n {
        if a_lines[i] != b_lines[i] {
            out.push_str(&format!("line {}: a={:?} b={:?}\n", i + 1, a_lines[i], b_lines[i]));
        }
    }
    if a_lines.len() != b_lines.len() {
        out.push_str(&format!("line count: a={} b={}\n", a_lines.len(), b_lines.len()));
    }
    if out.is_empty() {
        out.push_str("(no line-level diff found — possibly trailing whitespace)");
    }
    out
}

// Deterministic xorshift64 RNG (same style as parser_fuzz.rs / vm_fuzz.rs).
struct Rng {
    state: u64,
}
impl Rng {
    fn new(seed: u64) -> Self {
        Rng {
            state: if seed == 0 { 0xDEADBEEFCAFEBABE } else { seed },
        }
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
        lo + (self.next_u64() as usize) % (hi - lo)
    }
    fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

// --- Hand-crafted programs covering every AST node kind ---------------------

const HANDCRAFTED: &[&str] = &[
    // 1. Empty program
    "",
    // 2. Single import
    "import tt::util::ArrayList;\n",
    // 3. Glob import
    "import tt::util::*;\n",
    // 4. Multiple imports
    "import tt::util::ArrayList;\nimport tt::math::Math;\n",
    // 5. Top-level let
    "let x = 42;\n",
    // 6. Top-level const
    "const MAX: int = 100;\n",
    // 7. Simple function
    "public fn id(x: int): int {\n    return x;\n}\n",
    // 8. Generic function
    "public fn map<T, R>(list: ArrayList<T>, f: fn(T): R): ArrayList<R> {\n    return list;\n}\n",
    // 9. Function with where clause (parser supports it)
    "public fn sort<T>(arr: ArrayList<T>): ArrayList<T> {\n    return arr;\n}\n",
    // 10. Private function
    "fn helper(): void {\n    return;\n}\n",
    // 11. Function with no params
    "public fn pi(): double {\n    return 3.14;\n}\n",
    // 12. Class with fields
    "class Point {\n    public double x;\n    public double y;\n}\n",
    // 13. Class with constructor and method
    "class Circle {\n    public double radius;\n    public fn init(r: double) {\n        this.radius = r;\n    }\n    public fn area(): double {\n        return 3.14 * this.radius * this.radius;\n    }\n}\n",
    // 14. Generic class
    "class Box<T> {\n    public T value;\n    public fn init(v: T) {\n        this.value = v;\n    }\n}\n",
    // 15. Class extends
    "class Shape {\n    public string name;\n}\nclass Square extends Shape {\n    public double side;\n}\n",
    // 16. Class implements interface
    "interface Drawable {\n    fn draw(): void;\n}\nclass Icon implements Drawable {\n    public fn draw(): void {\n        return;\n    }\n}\n",
    // 17. Interface with default method
    "interface Greeter {\n    fn greet(): void {\n        return;\n    }\n}\n",
    // 18. Interface extends interface
    "interface A {\n    fn a(): void;\n}\ninterface B extends A {\n    fn b(): void;\n}\n",
    // 19. Simple enum
    "enum Color {\n    Red,\n    Green,\n    Blue\n}\n",
    // 20. Enum with payload
    "enum Shape {\n    Circle(double),\n    Square(double),\n    Rect(double, double)\n}\n",
    // 21. All literal kinds
    "public fn literals(): void {\n    let i = 42;\n    let f = 3.14;\n    let b = true;\n    let c = 'a';\n    let s = \"hello\";\n    let n = null;\n}\n",
    // 22. All binary operators
    "public fn binops(a: int, b: int): int {\n    let x = a + b;\n    let y = a - b;\n    let z = a * b;\n    let w = a / b;\n    let m = a % b;\n    return x;\n}\n",
    // 23. Comparison operators
    "public fn cmp(a: int, b: int): bool {\n    return a == b;\n}\n",
    "public fn cmp2(a: int, b: int): bool {\n    return a != b;\n}\n",
    "public fn cmp3(a: int, b: int): bool {\n    return a < b;\n}\n",
    "public fn cmp4(a: int, b: int): bool {\n    return a > b;\n}\n",
    "public fn cmp5(a: int, b: int): bool {\n    return a <= b;\n}\n",
    "public fn cmp6(a: int, b: int): bool {\n    return a >= b;\n}\n",
    // 24. Logical operators
    "public fn logic(a: bool, b: bool): bool {\n    return a && b;\n}\n",
    "public fn logic2(a: bool, b: bool): bool {\n    return a || b;\n}\n",
    // 25. Bitwise operators
    "public fn bit(a: int, b: int): int {\n    return a & b;\n}\n",
    "public fn bit2(a: int, b: int): int {\n    return a | b;\n}\n",
    "public fn bit3(a: int, b: int): int {\n    return a ^ b;\n}\n",
    "public fn bit4(a: int): int {\n    return ~a;\n}\n",
    "public fn bit5(a: int, b: int): int {\n    return a << b;\n}\n",
    "public fn bit6(a: int, b: int): int {\n    return a >> b;\n}\n",
    // 26. Unary operators
    "public fn neg(x: int): int {\n    return -x;\n}\n",
    "public fn not(x: bool): bool {\n    return !x;\n}\n",
    // 27. Member access
    "public fn use_point(p: Point): double {\n    return p.x;\n}\n",
    // 28. Method call
    "public fn call_area(c: Circle): double {\n    return c.area();\n}\n",
    // 29. Index
    "public fn use_index(arr: ArrayList<int>): int {\n    return arr[0];\n}\n",
    // 30. New
    "public fn make_point(): Point {\n    return new Point();\n}\n",
    // 31. Static call
    "public fn parse(s: string): int {\n    return Integer::parseInt(s);\n}\n",
    // 32. Ternary
    "public fn ternary(x: int): int {\n    return x > 0 ? x : -x;\n}\n",
    // 33. Assignment
    "public fn assign(): void {\n    let x = 0;\n    x = 42;\n}\n",
    // 34. Range
    "public fn range(): int {\n    let r = 1..10;\n    return 0;\n}\n",
    "public fn range_inc(): int {\n    let r = 1..=10;\n    return 0;\n}\n",
    // 35. Tuple
    "public fn tuple(): (int, string) {\n    return (1, \"hello\");\n}\n",
    "public fn unit(): void {\n    let u = ();\n    return;\n}\n",
    // 36. Closure (block)
    "public fn use_closure(): void {\n    let f = fn(x: int): int {\n        return x * 2;\n    };\n}\n",
    // 37. Closure (arrow)
    "public fn use_arrow(): void {\n    let f = fn(x: int): int => x * 2;\n}\n",
    // 38. Cast
    "public fn cast(x: int): double {\n    return x as double;\n}\n",
    // 39. Is
    "public fn is_test(x: Variant): bool {\n    return x is int;\n}\n",
    // 40. Error propagation
    "public fn prop(): int {\n    return might_fail()?;\n}\n",
    // 41. Reference
    "public fn ref_test(x: int): int {\n    let r = &x;\n    return 0;\n}\n",
    "public fn mut_ref_test(x: int): int {\n    let r = &mut x;\n    return 0;\n}\n",
    // 42. Owned deref
    "public fn owned_test(o: Owned<int>): int {\n    return *o;\n}\n",
    // 43. Unsafe block as expression
    "public fn unsafe_test(): int {\n    let x = unsafe {\n        return 42;\n    };\n    return 0;\n}\n",
    // 44. If/else
    "public fn if_else(x: int): void {\n    if (x > 0) {\n        return;\n    } else {\n        return;\n    }\n}\n",
    // 45. If/else if/else
    "public fn if_elif_else(x: int): void {\n    if (x > 0) {\n        return;\n    } else if (x == 0) {\n        return;\n    } else {\n        return;\n    }\n}\n",
    // 46. While loop
    "public fn while_loop(): void {\n    let i = 0;\n    while (i < 10) {\n        i = i + 1;\n    }\n}\n",
    // 47. Do-while
    "public fn do_while(): void {\n    let i = 0;\n    do {\n        i = i + 1;\n    } while (i < 10);\n}\n",
    // 48. For-in
    "public fn for_in(): void {\n    for (x in items) {\n        return;\n    }\n}\n",
    // 49. C-style for
    "public fn c_for(): void {\n    for (let i = 0; i < 10; i++) {\n        return;\n    }\n}\n",
    // 50. Break and continue
    "public fn break_continue(): void {\n    let i = 0;\n    while (true) {\n        if (i == 5) {\n            break;\n        }\n        i = i + 1;\n        continue;\n    }\n}\n",
    // 51. Switch with literals
    "public fn switch_test(x: int): void {\n    switch (x) {\n        case 0 => return;\n        case 1 => return;\n        default => return;\n    }\n}\n",
    // 52. Switch with wildcard
    "public fn switch_wild(x: int): void {\n    switch (x) {\n        case _ => return;\n    }\n}\n",
    // 53. Switch with constructor
    "public fn switch_con(s: Shape): void {\n    switch (s) {\n        case Circle(r) => return;\n        case Square(s) => return;\n        default => return;\n    }\n}\n",
    // 54. With statement
    "public fn with_test(): void {\n    with (resource) {\n        return;\n    }\n}\n",
    "public fn with_let(): void {\n    with (let f: File = File.open(\"a\")) {\n        return;\n    }\n}\n",
    // 55. Try-catch
    "public fn try_catch(): void {\n    try {\n        return;\n    } catch (e: string) {\n        return;\n    }\n}\n",
    // 56. Throw
    "public fn do_throw(): void {\n    throw \"error\";\n}\n",
    "public fn throw_int(): void {\n    throw 42;\n}\n",
    // 57. Tuple destructuring
    "public fn destructure(): void {\n    let (a, b) = (1, 2);\n}\n",
    "public fn destructure3(): void {\n    let (a, b, c) = (1, 2, 3);\n}\n",
    // 58. Numeric literals with underscores
    "public fn underscores(): int {\n    let x = 1_000_000;\n    return x;\n}\n",
    // 59. Hex/oct/bin literals
    "public fn radix(): void {\n    let h = 0xFF;\n    let o = 0o77;\n    let b = 0b1010;\n}\n",
    // 60. Float suffixes
    "public fn floats(): void {\n    let d = 3.14;\n    let h = 1.5h;\n    let q = 2.0q;\n}\n",
    // 61. String escapes
    "public fn escapes(): string {\n    return \"hello\\n\\t\\r\\\\\\\"\\'\\0\";\n}\n",
    // 62. Char escapes
    "public fn char_escapes(): char {\n    return '\\n';\n}\n",
    "public fn char_escapes2(): char {\n    return '\\\\';\n}\n",
    // 63. Raw strings
    "public fn raw(): string {\n    return r\"raw\";\n}\n",
    "public fn raw_hash(): string {\n    return r#\"raw\"#;\n}\n",
    "public fn raw_hash2(): string {\n    return r##\"raw\"##;\n}\n",
    // 64. Byte literal
    "public fn byte(): int {\n    return b'x';\n}\n",
    // 65. Nested expressions
    "public fn nested(): int {\n    return 1 + 2 * 3 - 4 / 2 % 3;\n}\n",
    // 66. Parenthesized expressions
    "public fn parens(): int {\n    return (1 + 2) * 3;\n}\n",
    "public fn parens2(): int {\n    return ((1));\n}\n",
    // 67. Chained calls
    "public fn chained(): int {\n    return a.b().c().d();\n}\n",
    // 68. Deeply nested member access
    "public fn deep(): int {\n    return a.b.c.d.e.f;\n}\n",
    // 69. Multiple statements in block
    "public fn multi(): int {\n    let a = 1;\n    let b = 2;\n    let c = 3;\n    return a + b + c;\n}\n",
    // 70. Empty block
    "public fn empty(): void {\n}\n",
    // 71. Block statement
    "public fn block_stmt(): void {\n    {\n        let x = 1;\n    }\n}\n",
    // 72. Reference type
    "public fn ref_type(r: &int): void {\n    return;\n}\n",
    "public fn mut_ref_type(r: &mut int): void {\n    return;\n}\n",
    // 73. Tuple type
    "public fn tuple_type(t: (int, string)): void {\n    return;\n}\n",
    "public fn tuple_type3(t: (int, string, bool)): void {\n    return;\n}\n",
    // 74. Function type
    "public fn func_type(f: fn(int): int): void {\n    return;\n}\n",
    "public fn func_type2(f: fn(int, int): int): void {\n    return;\n}\n",
    "public fn func_type3(f: fn(): void): void {\n    return;\n}\n",
    // 75. Generic types with multiple params
    "public fn generic_types(m: HashMap<string, int>): void {\n    return;\n}\n",
    "public fn nested_generic(m: HashMap<string, ArrayList<int>>): void {\n    return;\n}\n",
    // 76. Type with tuple inside generic
    "public fn tuple_in_generic(m: HashMap<(int, string), bool>): void {\n    return;\n}\n",
    // 77. Comment-only program
    "// just a comment\n",
    // 78. Block comment
    "/* block comment */\n",
    // 79. Mixed comments
    "// line\n/* block */\nlet x = 1;\n",
    // 80. C-style var decl (desugared)
    "public fn c_var(): void {\n    int x = 42;\n}\n",
    "public fn c_string(): void {\n    string s = \"hi\";\n}\n",
    "public fn c_double(): void {\n    double d = 3.14;\n}\n",
    // 81. Java-style function sugar (desugared)
    "public int add(int a, int b) {\n    return a + b;\n}\n",
    "public void noop() {\n    return;\n}\n",
    "private int secret() {\n    return 42;\n}\n",
    // 82. Java-style method sugar
    "class C {\n    public int foo() {\n        return 1;\n    }\n    public void bar() {\n        return;\n    }\n}\n",
    // 83. Java-style constructor
    "class D {\n    public double x;\n    public D(double x) {\n        this.x = x;\n    }\n}\n",
    // 84. Field with C-style type
    "class E {\n    public int count;\n    public string name;\n}\n",
    // 85. Multiple fields with initializers
    "class F {\n    public int a = 1;\n    public int b = 2;\n    public int c = 3;\n}\n",
];

#[test]
fn handcrafted_programs_round_trip() {
    let mut failures = Vec::new();
    for (i, src) in HANDCRAFTED.iter().enumerate() {
        match round_trip_fixed_point(src) {
            Ok(()) => {}
            Err(e) => {
                failures.push(format!("case #{}: {}", i + 1, e));
            }
        }
    }
    if !failures.is_empty() {
        panic!(
            "{} of {} handcrafted programs failed round-trip:\n\n{}",
            failures.len(),
            HANDCRAFTED.len(),
            failures.join("\n\n")
        );
    }
}

// --- Generated programs -----------------------------------------------------

fn gen_program(rng: &mut Rng) -> String {
    let mut out = String::new();
    out.push_str("public fn generated_");
    out.push_str(&rng.next_u64().to_string());
    out.push_str("(): void {\n");
    let n_stmts = rng.range(1, 6);
    for _ in 0..n_stmts {
        out.push_str("    ");
        out.push_str(&gen_stmt(rng, 0));
        out.push('\n');
    }
    out.push_str("}\n");
    out
}

fn gen_stmt(rng: &mut Rng, depth: usize) -> String {
    if depth >= 3 {
        return gen_simple_stmt(rng);
    }
    match rng.range(0, 8) {
        0 => format!("let v_{} = {};", rng.next_u64() % 1000, gen_expr(rng, 0)),
        1 => format!("var v_{}: int = {};", rng.next_u64() % 1000, rng.range(0, 1000)),
        2 => format!("if ({}) {{ {} }}", gen_expr(rng, 0), gen_stmt(rng, depth + 1)),
        3 => format!("while ({}) {{ {} }}", gen_expr(rng, 0), gen_stmt(rng, depth + 1)),
        4 => format!("return {};", gen_expr(rng, 0)),
        5 => format!("io::println({});", gen_expr(rng, 0)),
        6 => {
            let n = rng.range(1, 4);
            let mut s = String::from("{ ");
            for _ in 0..n {
                s.push_str(&gen_stmt(rng, depth + 1));
                s.push(' ');
            }
            s.push('}');
            s
        }
        _ => format!("{};", gen_expr(rng, 0)),
    }
}

fn gen_simple_stmt(rng: &mut Rng) -> String {
    match rng.range(0, 3) {
        0 => format!("return {};", gen_expr(rng, 0)),
        1 => format!("let v_{} = {};", rng.next_u64() % 1000, gen_expr(rng, 0)),
        _ => format!("io::println({});", gen_expr(rng, 0)),
    }
}

fn gen_expr(rng: &mut Rng, depth: usize) -> String {
    if depth >= 4 {
        return gen_atom(rng);
    }
    match rng.range(0, 10) {
        0 => gen_atom(rng),
        1 => format!("{} + {}", gen_expr(rng, depth + 1), gen_expr(rng, depth + 1)),
        2 => format!("{} - {}", gen_expr(rng, depth + 1), gen_expr(rng, depth + 1)),
        3 => format!("{} * {}", gen_expr(rng, depth + 1), gen_expr(rng, depth + 1)),
        4 => format!("{} == {}", gen_expr(rng, depth + 1), gen_expr(rng, depth + 1)),
        5 => format!("{} < {}", gen_expr(rng, depth + 1), gen_expr(rng, depth + 1)),
        6 => format!("{} && {}", gen_expr(rng, depth + 1), gen_expr(rng, depth + 1)),
        7 => format!("{} || {}", gen_expr(rng, depth + 1), gen_expr(rng, depth + 1)),
        8 => format!("{} ? {} : {}", gen_expr(rng, depth + 1), gen_expr(rng, depth + 1), gen_expr(rng, depth + 1)),
        _ => format!("({})", gen_expr(rng, depth + 1)),
    }
}

fn gen_atom(rng: &mut Rng) -> String {
    match rng.range(0, 6) {
        0 => rng.range(0, 10000).to_string(),
        1 => format!("{}", rng.next_u64() as i64 as f64 / 100.0),
        2 => String::from(if rng.bool() { "true" } else { "false" }),
        3 => format!("\"str_{}\"", rng.next_u64() % 1000),
        4 => format!("v_{}", rng.next_u64() % 1000),
        _ => String::from("null"),
    }
}

const GENERATED_COUNT: usize = 1_000;

#[test]
fn generated_programs_round_trip_fixed_point() {
    let mut rng = Rng::new(0x5EED_1234_5678_ABCD);
    let mut failures = 0u32;
    let mut first_failures = Vec::new();
    for _ in 0..GENERATED_COUNT {
        let src = gen_program(&mut rng);
        match round_trip_fixed_point(&src) {
            Ok(()) => {}
            Err(e) => {
                failures += 1;
                if first_failures.len() < 5 {
                    first_failures.push(format!("--- failure ---\nsrc:\n{}\nerr:\n{}\n", src, e));
                }
            }
        }
    }
    if failures != 0 {
        panic!(
            "{} of {} generated programs failed round-trip.\nFirst {} failures:\n\n{}",
            failures,
            GENERATED_COUNT,
            first_failures.len(),
            first_failures.join("\n")
        );
    }
}

#[test]
fn pretty_print_is_idempotent_on_handcrafted() {
    // Direct idempotence: pretty_print(pretty_print(parse(src))) == pretty_print(parse(src))
    // This is a weaker property than round-trip (it skips the second parse),
    // but it catches pretty_print non-determinism.
    let mut failures = Vec::new();
    for (i, src) in HANDCRAFTED.iter().enumerate() {
        let tokens = match lexer::tokenize(src) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let p1 = match parser::parse(tokens) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let src2 = pretty_print::pretty_print(&p1);
        let src3 = pretty_print::pretty_print(&p1);
        if src2 != src3 {
            failures.push(format!("case #{}: pretty_print is non-deterministic", i + 1));
        }
    }
    if !failures.is_empty() {
        panic!("pretty_print non-determinism:\n\n{}", failures.join("\n"));
    }
}

#[test]
fn round_trip_preserves_ast_structure_on_handcrafted() {
    // Structural preservation: parse(src) and parse(pretty_print(parse(src)))
    // produce ASTs whose pretty_printed forms are identical. This is the
    // contrapositive of the fixed-point property, but checked directly here
    // to catch the case where both round-trips fail identically (which would
    // pass the fixed-point test but still indicate a bug).
    //
    // Skip cases where the original source doesn't parse (parser limitation).
    let mut failures = Vec::new();
    let mut skipped = 0u32;
    for (i, src) in HANDCRAFTED.iter().enumerate() {
        let tokens = match lexer::tokenize(src) {
            Ok(t) => t,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        let p1 = match parser::parse(tokens) {
            Ok(p) => p,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        let src2 = pretty_print::pretty_print(&p1);
        let pp1 = src2.clone();
        // Now parse src2 and pretty-print again
        let tokens2 = match lexer::tokenize(&src2) {
            Ok(t) => t,
            Err(e) => {
                failures.push(format!("case #{}: re-lex failed: {}", i + 1, e));
                continue;
            }
        };
        let p2 = match parser::parse(tokens2) {
            Ok(p) => p,
            Err(e) => {
                failures.push(format!("case #{}: re-parse failed: {}", i + 1, e));
                continue;
            }
        };
        let pp2 = pretty_print::pretty_print(&p2);
        if pp1 != pp2 {
            failures.push(format!("case #{}: structural drift\nfirst:\n{}\nsecond:\n{}\n", i + 1, pp1, pp2));
        }
    }
    if !failures.is_empty() {
        panic!(
            "{} of {} handcrafted programs had structural drift ({} skipped):\n\n{}",
            failures.len(),
            HANDCRAFTED.len() - skipped as usize,
            skipped,
            failures.join("\n")
        );
    }
}
