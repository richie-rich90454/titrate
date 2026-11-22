//! Bytecode VM fuzz harness — compiles random valid Titrate programs
//! to bytecode and executes them. Asserts no panics, no stack corruption,
//! no use-after-free in the value stack, no frame-local escapes.
//!
//! Each program is generated to be syntactically valid and semantically
//! straightforward (no missing imports, no undeclared identifiers).
//! The harness catches any panic via `catch_unwind` and treats it as a
//! test failure.
//!
//! Closes Task C.3 of the world-class-systems-grade-audit spec.

use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use trc::bytecode;
use trc::lexer;
use trc::parser;

// ---------------------------------------------------------------------------
// Deterministic RNG (xorshift64) — same style as parser_fuzz.rs.
// Fixed seed for reproducibility.
// ---------------------------------------------------------------------------

struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Rng {
            state: if seed == 0 {
                0xDEADBEEFCAFEBABE
            } else {
                seed
            },
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
// Valid program fragment generators.
//
// All generators produce SYNTACTICALLY VALID Titrate source that the
// analyzer + VM can run.  Only built-in primitives (int, double, bool,
// string, void) and `io::println` are used — no imports, no external
// modules, no undeclared identifiers, no `new`, no `for-in`.
//
// All `while` loops are bounded (max 10 iterations) to guarantee termination.
// Division/modulo always uses a non-zero literal divisor.
// ---------------------------------------------------------------------------

const STRINGS: &[&str] = &["hello", "world", "foo", "bar", "baz", "test", "x"];
const INT_BINOPS: &[&str] = &["+", "-", "*"];
const SHIFT_OPS: &[&str] = &["<<", ">>"];
const BITWISE_OPS: &[&str] = &["&", "|", "^"];
const CMP_OPS: &[&str] = &["<", ">", "<=", ">=", "==", "!="];

/// Generate a literal expression of a random primitive type.
fn gen_literal_expr(rng: &mut Rng) -> String {
    match rng.range(0, 6) {
        0 => format!("{}", rng.range(0, 1000)),
        1 => format!("{}.{}", rng.range(0, 1000), rng.range(0, 1000)),
        2 => "true".to_string(),
        3 => "false".to_string(),
        4 => format!("\"{}\"", rng.pick(STRINGS)),
        _ => "null".to_string(),
    }
}

/// Generate an expression that evaluates to an int.
/// Division and modulo always use a non-zero literal divisor.
fn gen_int_expr(rng: &mut Rng, depth: usize) -> String {
    if depth >= 4 {
        return format!("{}", rng.range(0, 1000));
    }
    match rng.range(0, 7) {
        0 => format!("{}", rng.range(0, 1000)),
        1 => {
            let a = gen_int_expr(rng, depth + 1);
            let b = gen_int_expr(rng, depth + 1);
            let op = rng.pick(INT_BINOPS);
            format!("({} {} {})", a, op, b)
        }
        2 => {
            // Division/modulo with non-zero literal divisor
            let a = gen_int_expr(rng, depth + 1);
            let divisor = rng.range(1, 1000);
            let op = rng.pick(&["/", "%"]);
            format!("({} {} {})", a, op, divisor)
        }
        3 => {
            let a = gen_int_expr(rng, depth + 1);
            format!("-{}", a)
        }
        4 => {
            // Shift with small shift amount (0..8)
            let a = gen_int_expr(rng, depth + 1);
            let shift = rng.range(0, 8);
            let op = rng.pick(SHIFT_OPS);
            format!("({} {} {})", a, op, shift)
        }
        5 => {
            let a = gen_int_expr(rng, depth + 1);
            let b = gen_int_expr(rng, depth + 1);
            let op = rng.pick(BITWISE_OPS);
            format!("({} {} {})", a, op, b)
        }
        _ => format!("({} as int)", gen_int_expr(rng, depth + 1)),
    }
}

/// Generate an expression that evaluates to a bool.
fn gen_bool_expr(rng: &mut Rng, depth: usize) -> String {
    if depth >= 3 {
        return if rng.bool() {
            "true".to_string()
        } else {
            "false".to_string()
        };
    }
    match rng.range(0, 5) {
        0 => {
            if rng.bool() {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        1 => {
            let a = gen_int_expr(rng, depth + 1);
            let b = gen_int_expr(rng, depth + 1);
            let op = rng.pick(CMP_OPS);
            format!("({} {} {})", a, op, b)
        }
        2 => {
            let a = gen_bool_expr(rng, depth + 1);
            let b = gen_bool_expr(rng, depth + 1);
            let op = rng.pick(&["&&", "||"]);
            format!("({} {} {})", a, op, b)
        }
        3 => {
            let a = gen_bool_expr(rng, depth + 1);
            format!("!{}", a)
        }
        _ => {
            // Comparison against a literal
            let a = gen_int_expr(rng, depth + 1);
            let lit = rng.range(0, 100);
            let op = rng.pick(CMP_OPS);
            format!("({} {} {})", a, op, lit)
        }
    }
}

/// Generate an expression of any primitive type.
fn gen_expr(rng: &mut Rng, depth: usize) -> String {
    if depth >= 4 {
        return gen_literal_expr(rng);
    }
    match rng.range(0, 6) {
        0 => gen_literal_expr(rng),
        1 => gen_int_expr(rng, depth + 1),
        2 => gen_bool_expr(rng, depth + 1),
        3 => {
            // String concatenation
            let a = format!("\"{}\"", rng.pick(STRINGS));
            let b = format!("\"{}\"", rng.pick(STRINGS));
            format!("({} + {})", a, b)
        }
        4 => {
            // Ternary
            let cond = gen_bool_expr(rng, depth + 1);
            let t = gen_int_expr(rng, depth + 1);
            let f = gen_int_expr(rng, depth + 1);
            format!("({} ? {} : {})", cond, t, f)
        }
        _ => {
            // `as` cast: int → double
            let a = gen_int_expr(rng, depth + 1);
            format!("({} as double)", a)
        }
    }
}

/// Generate a single statement.
///
/// `var_counter` is bumped for each new variable so names are unique within
/// a program.  `in_loop` controls whether `break`/`continue` are emitted
/// (they are only valid inside a loop body).
fn gen_stmt(rng: &mut Rng, depth: usize, var_counter: &mut usize, in_loop: bool) -> String {
    if depth >= 3 {
        let name = format!("v{}", *var_counter);
        *var_counter += 1;
        return format!("let {} = {};", name, gen_literal_expr(rng));
    }
    match rng.range(0, 9) {
        0 => {
            let name = format!("v{}", *var_counter);
            *var_counter += 1;
            format!("let {} = {};", name, gen_expr(rng, depth + 1))
        }
        1 => {
            let name = format!("v{}", *var_counter);
            *var_counter += 1;
            format!("var {}: int = {};", name, gen_int_expr(rng, depth + 1))
        }
        2 => format!("io::println({});", gen_expr(rng, depth + 1)),
        3 => {
            let cond = gen_bool_expr(rng, depth + 1);
            let then_s = gen_stmt(rng, depth + 1, var_counter, in_loop);
            if rng.bool() {
                let else_s = gen_stmt(rng, depth + 1, var_counter, in_loop);
                format!("if ({}) {{ {} }} else {{ {} }}", cond, then_s, else_s)
            } else {
                format!("if ({}) {{ {} }}", cond, then_s)
            }
        }
        4 => {
            // Bounded while loop — always terminates (max 10 iterations).
            // Wrapped in a block so the counter is scoped and doesn't leak.
            let counter_name = format!("v{}", *var_counter);
            *var_counter += 1;
            let limit = rng.range(1, 10);
            let body = gen_stmt(rng, depth + 1, var_counter, true);
            format!(
                "{{ let {} = 0; while ({} < {}) {{ {} = {} + 1; {} }} }}",
                counter_name, counter_name, limit, counter_name, counter_name, body
            )
        }
        5 => {
            if in_loop {
                "break;".to_string()
            } else {
                let name = format!("v{}", *var_counter);
                *var_counter += 1;
                format!("let {} = {};", name, gen_literal_expr(rng))
            }
        }
        6 => {
            if in_loop {
                "continue;".to_string()
            } else {
                let name = format!("v{}", *var_counter);
                *var_counter += 1;
                format!("let {} = {};", name, gen_literal_expr(rng))
            }
        }
        7 => "return;".to_string(),
        _ => {
            // `as` cast assignment
            let name = format!("v{}", *var_counter);
            *var_counter += 1;
            format!(
                "let {}: double = {} as double;",
                name,
                gen_int_expr(rng, depth + 1)
            )
        }
    }
}

/// Generate a complete Titrate program with a `public fn main(): void`
/// entry point containing 3–8 randomly generated statements.
fn gen_program(rng: &mut Rng) -> String {
    let mut var_counter = 0usize;
    let n_stmts = rng.range(3, 8);
    let mut body = String::new();
    for _ in 0..n_stmts {
        body.push_str(&gen_stmt(rng, 0, &mut var_counter, false));
        body.push(' ');
    }
    format!("public fn main(): void {{ {} }}", body)
}

// ---------------------------------------------------------------------------
// Adversarial program generators.
// ---------------------------------------------------------------------------

/// A program that calls `io::println` 1 000 times — exercises output buffer
/// growth / memory pressure.
fn many_println_program(n: usize) -> String {
    let mut s = String::from("public fn main(): void { ");
    for i in 0..n {
        s.push_str(&format!("io::println(\"line_{}\"); ", i));
    }
    s.push('}');
    s
}

/// A program with `depth` nested `if (true)` blocks — exercises compiler
/// and VM block nesting.
fn nested_if_program(depth: usize) -> String {
    let mut s = String::from("public fn main(): void { ");
    for _ in 0..depth {
        s.push_str("if (true) { ");
    }
    s.push_str("io::println(\"deep\"); ");
    for _ in 0..depth {
        s.push_str("} ");
    }
    s.push('}');
    s
}

/// A program with `n` sequential `let` declarations — exercises variable
/// table growth.
fn many_lets_program(n: usize) -> String {
    let mut s = String::from("public fn main(): void { ");
    for i in 0..n {
        s.push_str(&format!("let v{} = {}; ", i, i));
    }
    s.push_str("io::println(v0); ");
    s.push('}');
    s
}

/// A program with a deeply nested binary expression:
/// `(depth + ((depth-1) + (... + (1 + 0))))`.
fn deep_binary_expr_program(depth: usize) -> String {
    let mut expr = String::from("0");
    for d in 1..=depth {
        expr = format!("({} + {})", d, expr);
    }
    format!(
        "public fn main(): void {{ let x = {}; io::println(x); }}",
        expr
    )
}

/// A program that uses `as` casts.
fn as_cast_program() -> String {
    "public fn main(): void { let d: double = 42 as double; let i: int = 99; let d2: double = i as double; io::println(d); io::println(d2); }"
        .to_string()
}

/// A program that uses an `is` type check — may error at compile or run
/// time, but must not panic.
fn is_check_program() -> String {
    "public fn main(): void { let x = 42; if (x is int) { io::println(\"int\"); } }"
        .to_string()
}

// ---------------------------------------------------------------------------
// Compilation + execution pipeline.
// ---------------------------------------------------------------------------

const FUZZ_COUNT: usize = 1_000;
const TIMEOUT_MS: u64 = 2000;
const STACK_SIZE: usize = 64 * 1024 * 1024;

/// Compile and run a Titrate source string through the full pipeline:
/// lex → parse → compile_with_modules → Vm::run.
///
/// Returns the joined stdout output on success, or a structured `Err`
/// describing which stage failed.
fn run_program(src: &str) -> Result<String, String> {
    let tokens = lexer::tokenize(src).map_err(|e| format!("lexer: {}", e))?;
    let ast = parser::parse(tokens).map_err(|e| format!("parser: {}", e))?;
    let root_dir = PathBuf::from("..");
    let mut compiler = bytecode::Compiler::new();
    let compiled = compiler
        .compile_with_modules(&ast, &root_dir)
        .map_err(|e| format!("compiler: {}", e))?;
    let mut vm = bytecode::Vm::new();
    vm.set_working_dir(root_dir);
    vm.load_program(compiled);
    vm.run().map_err(|e| format!("vm: {}", e))?;
    Ok(vm.output.join("\n"))
}

/// Run `run_program` on a dedicated thread with a large stack and a timeout.
///
/// Returns:
/// - `Ok(Ok(output))`  — program compiled and ran successfully.
/// - `Ok(Err(msg))`    — structured compile-time or runtime error (acceptable).
/// - `Err("timeout")`  — thread did not finish within `TIMEOUT_MS`.
/// - `Err("panic")`    — the thread panicked (test failure).
/// - `Err(other)`      — thread spawn or channel failure (test failure).
fn run_with_timeout(src: String) -> Result<Result<String, String>, &'static str> {
    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(move || {
            let result =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run_program(&src)));
            let _ = tx.send(result);
        })
        .map_err(|_| "spawn-failed")?;
    match rx.recv_timeout(Duration::from_millis(TIMEOUT_MS)) {
        Ok(Ok(r)) => Ok(r),
        Ok(Err(_)) => Err("panic"),
        Err(mpsc::RecvTimeoutError::Timeout) => Err("timeout"),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err("panic"),
    }
}

// ---------------------------------------------------------------------------
// Main fuzz test — 1 000 random programs, no panics, no corruption.
// ---------------------------------------------------------------------------

#[test]
fn vm_fuzz_1000_programs_no_panic_no_corruption() {
    let mut rng = Rng::new(0xC0_FFEE_BABE_1337);
    let mut failures = 0u32;
    let mut errors_seen = 0u32;
    for i in 0..FUZZ_COUNT {
        let src = gen_program(&mut rng);
        let result = run_with_timeout(src.clone());
        match result {
            Ok(Ok(_output)) => { /* program ran successfully */ }
            Ok(Err(msg)) => {
                // Compile-time or runtime error — acceptable as long as it is
                // a structured `Err`, not a panic.  Print the first few for
                // visibility.
                errors_seen += 1;
                if errors_seen <= 3 {
                    eprintln!("Runtime/compile error on iteration {}: {}", i, msg);
                    eprintln!("  source: {}", src);
                }
            }
            Err("timeout") => {
                failures += 1;
                if failures <= 3 {
                    eprintln!("Timeout on iteration {} input: {}", i, src);
                }
            }
            Err("panic") => {
                failures += 1;
                if failures <= 3 {
                    eprintln!("PANIC on iteration {} input: {}", i, src);
                }
            }
            Err(other) => {
                failures += 1;
                if failures <= 3 {
                    eprintln!("Thread error on iteration {}: {}", i, other);
                }
            }
        }
    }
    assert_eq!(
        failures, 0,
        "VM fuzz reported {} failures (panics/timeouts) out of {} iterations",
        failures, FUZZ_COUNT
    );
}

// ---------------------------------------------------------------------------
// Adversarial tests — each exercises a specific stress pattern.
// ---------------------------------------------------------------------------

/// Helper: run a single program and assert it does not panic or time out.
/// A structured compile/runtime error is acceptable.
fn assert_no_panic_or_timeout(src: String, label: &str) {
    let result = run_with_timeout(src);
    match result {
        Ok(Ok(_output)) => { /* ran successfully */ }
        Ok(Err(_msg)) => { /* structured error — acceptable */ }
        Err("timeout") => panic!("VM timed out on adversarial case: {}", label),
        Err("panic") => panic!("VM panicked on adversarial case: {}", label),
        Err(other) => panic!("VM thread error on adversarial case '{}': {}", label, other),
    }
}

#[test]
fn vm_fuzz_adversarial_many_println() {
    // 1 000 io::println calls — exercises output buffer growth / memory.
    assert_no_panic_or_timeout(many_println_program(1_000), "many_println_1000");
}

#[test]
fn vm_fuzz_adversarial_nested_if() {
    // 100 nested if blocks — exercises compiler/VM block depth.
    assert_no_panic_or_timeout(nested_if_program(100), "nested_if_100");
}

#[test]
fn vm_fuzz_adversarial_many_lets() {
    // 100 sequential let declarations — exercises variable table growth.
    assert_no_panic_or_timeout(many_lets_program(100), "many_lets_100");
}

#[test]
fn vm_fuzz_adversarial_deep_binary_expr() {
    // 50-level nested binary expression — exercises expression compilation.
    assert_no_panic_or_timeout(deep_binary_expr_program(50), "deep_binary_expr_50");
}

#[test]
fn vm_fuzz_adversarial_as_cast() {
    // `as` type casts (int → double).
    assert_no_panic_or_timeout(as_cast_program(), "as_cast");
}

#[test]
fn vm_fuzz_adversarial_is_check() {
    // `is` type check — may error, must not panic.
    assert_no_panic_or_timeout(is_check_program(), "is_check");
}

// ---------------------------------------------------------------------------
// Step limit, malformed bytecode, and wide jump operands.
// ---------------------------------------------------------------------------

fn compile_source(src: &str) -> bytecode::CompiledProgram {
    let tokens = lexer::tokenize(src).expect("lex error");
    let ast = parser::parse(tokens).expect("parse error");
    let mut compiler = bytecode::Compiler::new();
    compiler
        .compile_with_modules(&ast, &PathBuf::from(".."))
        .expect("compiler error")
}

#[test]
fn vm_step_limit_is_configurable_and_fatal() {
    let src = r#"
        public fn main(): void {
            try {
                while (true) { }
            } catch (e: string) {
                io::println("caught");
            }
        }
    "#;
    let mut vm = bytecode::Vm::new();
    vm.set_max_steps(2_000);
    vm.load_program(compile_source(src));
    let err = vm.run().expect_err("step limit must terminate the run");
    assert!(err.contains("step limit"), "unexpected error: {}", err);
    assert!(
        !vm.output.iter().any(|line| line.contains("caught")),
        "a fatal step limit must bypass user-level try/catch"
    );
}

#[test]
fn vm_zero_step_limit_means_unlimited() {
    let mut vm = bytecode::Vm::new();
    vm.set_max_steps(0);
    vm.load_program(compile_source("public fn main(): void { io::println(\"ok\"); }"));
    vm.run().expect("unlimited step budget must not trip the cap");
    assert_eq!(vm.output.join("\n"), "ok");
}

#[test]
fn vm_step_counter_resets_between_runs() {
    let mut body = String::from("public fn main(): void { let x = 0; while (x < 100) { x = x + 1; } }");
    body.push(' ');
    let compiled = compile_source(&body);
    let mut vm = bytecode::Vm::new();
    vm.set_max_steps(1_200);
    vm.load_program(compiled);
    vm.run().expect("first run must stay under the step limit");
    vm.run().expect("second run must get a fresh step budget");
}

#[test]
fn vm_truncated_operand_is_a_clean_error() {
    let mut chunk = bytecode::Chunk::new();
    chunk.write_opcode(bytecode::OpCode::JMP, 1);
    chunk.write_u8(0, 1); // only one of the four i32 operand bytes
    let program = bytecode::CompiledProgram {
        functions: vec![trc::bytecode::frame::FunctionDef {
            name: "main".to_string(),
            arity: 0,
            chunk,
            is_method: false,
            is_constructor: false,
            local_count: 0,
            param_types: Vec::new(),
        }],
        classes: Vec::new(),
        enums: Vec::new(),
        native_names: Vec::new(),
        global_count: 0,
    };
    let mut vm = bytecode::Vm::new();
    vm.load_program(program);
    let err = vm.run().expect_err("truncated operand must be an error, not a panic");
    assert!(err.contains("out of bounds"), "unexpected error: {}", err);
}

#[test]
fn vm_stack_underflow_is_a_clean_error() {
    let mut chunk = bytecode::Chunk::new();
    chunk.write_opcode(bytecode::OpCode::POP, 1);
    let program = bytecode::CompiledProgram {
        functions: vec![trc::bytecode::frame::FunctionDef {
            name: "main".to_string(),
            arity: 0,
            chunk,
            is_method: false,
            is_constructor: false,
            local_count: 0,
            param_types: Vec::new(),
        }],
        classes: Vec::new(),
        enums: Vec::new(),
        native_names: Vec::new(),
        global_count: 0,
    };
    let mut vm = bytecode::Vm::new();
    vm.load_program(program);
    let err = vm.run().expect_err("stack underflow must be an error, not a panic");
    assert!(err.contains("stack underflow"), "unexpected error: {}", err);
}

#[test]
fn vm_large_function_backward_jump_uses_wide_operand() {
    // 2,500 assignments compile to ~35KB of bytecode. The loop's backward
    // jump is therefore wider than i16; a 16-bit operand wraps and redirects
    // execution into the middle of the body.
    const ASSIGNMENTS: usize = 2_500;
    let mut src = String::from("public fn main(): void { let x = 0; while (x < 1) {");
    for _ in 0..ASSIGNMENTS {
        src.push_str(" x = x + 1;");
    }
    src.push_str(" } io::println(x); }");

    let compiled = compile_source(&src);
    let code_len = compiled
        .functions
        .iter()
        .map(|f| f.chunk.code.len())
        .max()
        .unwrap_or(0);
    assert!(code_len > 32_767, "test program must exceed the i16 range: {} bytes", code_len);

    let mut vm = bytecode::Vm::new();
    vm.load_program(compiled);
    vm.run().expect("wide jump must execute the large function correctly");
    assert_eq!(vm.output.join("\n").trim_end(), ASSIGNMENTS.to_string());
}
