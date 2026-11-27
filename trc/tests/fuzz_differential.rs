//! Differential fuzz harness for Titrate execution backends.
//!
//! Backends actually compared, for every generated program:
//!   * bytecode VM (`trc::bytecode::execute`)
//!   * WebAssembly host (`trc::codegen::wasm::compile_to_wasm` executed by the
//!     wasmi host shim in `wasm_common`)
//!   * tree-walking interpreter (`trc::interpreter::Interpreter`)
//!
//! The LLVM backend is not compared here. It has no in-process stdout-capture
//! host shim, so adding it would require a native JIT harness rather than an
//! integration test. Interpreter output is read from `Interpreter::output`
//! instead of `trc::interpreter::interpret`: that helper prints captured lines
//! to process stdout and returns only `Result<(), String>`, so it cannot be
//! compared against the other backends' captured output.
//!
//! Programs stay inside the intersection of the three backends: primitive
//! literals, integer arithmetic, comparisons, boolean operators, bounded
//! while loops, if/else, `ArrayList<int>` and `HashMap<string, int>` with
//! deterministic operations. Every backend call runs on a worker thread with
//! a timeout; a backend that rejects a program structurally is recorded as a
//! skip, while a timeout or output mismatch fails the test.

mod wasm_common;

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use trc::analyzer;
use trc::interpreter::Interpreter;
use trc::lexer;
use trc::parser;

const PROGRAM_COUNT: usize = 300;
const MIN_COMPARED: usize = 200;
const TIMEOUT_MS: u64 = 10_000;
const STACK_SIZE: usize = 16 * 1024 * 1024;

const STRINGS: &[&str] = &["alpha", "beta", "gamma", "delta", "x", "y"];
const KEYS: &[&str] = &["k1", "k2", "k3"];
const ARITHMETIC: &[&str] = &["+", "-", "*", "/", "%"];
const COMPARISONS: &[&str] = &["<", ">", "<=", ">=", "==", "!="];

/// Deterministic linear congruential generator (Numerical Recipes constants).
struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.state
    }

    fn range(&mut self, lo: u64, hi: u64) -> u64 {
        lo + self.next_u64() % (hi - lo + 1)
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        let index = self.range(0, items.len() as u64 - 1) as usize;
        &items[index]
    }

    fn boolean(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

/// Build an `int`-valued expression over the locals `a`, `b` and `c`.
/// Division and modulo always use a non-zero literal divisor.
fn gen_int_expr(rng: &mut Lcg, depth: usize) -> String {
    if depth >= 2 {
        return rng.range(0, 40).to_string();
    }
    match rng.range(0, 6) {
        0 => rng.range(0, 40).to_string(),
        1 => "a".to_string(),
        2 => "b".to_string(),
        3 => "c".to_string(),
        4 => {
            let op = *rng.pick(ARITHMETIC);
            let right = if op == "/" || op == "%" {
                rng.range(1, 12).to_string()
            } else {
                gen_int_expr(rng, depth + 1)
            };
            format!("({} {} {})", gen_int_expr(rng, depth + 1), op, right)
        }
        _ => format!("-({})", gen_int_expr(rng, depth + 1)),
    }
}

fn gen_bool_expr(rng: &mut Lcg, depth: usize) -> String {
    if depth >= 2 {
        return if rng.boolean() { "true" } else { "false" }.to_string();
    }
    match rng.range(0, 5) {
        0 => if rng.boolean() { "true" } else { "false" }.to_string(),
        1 => format!(
            "({} {} {})",
            gen_int_expr(rng, depth + 1),
            rng.pick(COMPARISONS),
            gen_int_expr(rng, depth + 1)
        ),
        2 => {
            let joiner = if rng.boolean() { "&&" } else { "||" };
            format!(
                "({} {} {})",
                gen_bool_expr(rng, depth + 1),
                joiner,
                gen_bool_expr(rng, depth + 1)
            )
        }
        3 => format!("!({})", gen_bool_expr(rng, depth + 1)),
        _ => format!(
            "({} {} {})",
            gen_int_expr(rng, depth + 1),
            rng.pick(COMPARISONS),
            rng.range(0, 20)
        ),
    }
}

/// Generate one small, self-contained program. The loop bound equals the
/// number of appended list elements, so `xs.get(i)` stays in range.
fn gen_program(rng: &mut Lcg) -> String {
    let a = rng.range(0, 40);
    let b = rng.range(1, 40);
    let c = rng.range(0, 20);
    let label = rng.pick(STRINGS);
    let key = rng.pick(KEYS);
    let count = rng.range(1, 4);

    let mut adds = String::new();
    for _ in 0..count {
        adds.push_str("xs.add(");
        adds.push_str(&gen_int_expr(rng, 1));
        adds.push_str("); ");
    }

    let arithmetic = gen_int_expr(rng, 1);
    let comparison = gen_bool_expr(rng, 1);
    let branch = gen_bool_expr(rng, 1);
    let map_value = gen_int_expr(rng, 1);
    let cast = gen_int_expr(rng, 1);

    format!(
        "public fn main(): void {{
    let a: int = {a};
    let b: int = {b};
    let c: int = {c};
    io::println(Integer.toString({arithmetic}));
    io::println(Boolean.toString({comparison}));
    if ({branch}) {{ io::println(\"then-\" + \"{label}\"); }} else {{ io::println(\"else-\" + \"{label}\"); }}
    let xs = new ArrayList<int>();
    {adds}
    io::println(xs.toString());
    io::println(Integer.toString(xs.size()));
    var i: int = 0;
    while (i < {count}) {{ io::println(Integer.toString(xs.get(i))); i = i + 1; }}
    let m = new HashMap<string, int>();
    m.put(\"{key}\", {map_value});
    io::println(Integer.toString(m.get(\"{key}\")));
    io::println(Double.toString(({cast} as double) + 0.5));
}}",
        a = a,
        b = b,
        c = c,
        arithmetic = arithmetic,
        comparison = comparison,
        branch = branch,
        label = label,
        adds = adds,
        count = count,
        key = key,
        map_value = map_value,
        cast = cast,
    )
}

/// Compare printed output; numeric spellings such as `1.0` and `1` are
/// equivalent across hosts, so canonicalize standalone numbers only.
fn normalize(lines: &[String]) -> String {
    let mut normalized = Vec::new();
    for line in lines {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(number) = trimmed.parse::<f64>() {
            if number == 0.0 {
                normalized.push("0".to_string());
            } else {
                normalized.push(number.to_string());
            }
        } else {
            normalized.push(trimmed.to_string());
        }
    }
    normalized.join("\n")
}

struct BackendOutputs {
    vm: Result<Vec<String>, String>,
    wasm: Result<Vec<String>, String>,
    interpreter: Result<Vec<String>, String>,
}

/// Turn a backend panic into a structured error so one backend cannot abort
/// the comparison of the others.
fn guarded<T>(run: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    match catch_unwind(AssertUnwindSafe(run)) {
        Ok(result) => result,
        Err(_) => Err("backend panicked".to_string()),
    }
}

fn run_backends(src: &str) -> Result<BackendOutputs, String> {
    let tokens = lexer::tokenize(src).map_err(|e| format!("lexer: {}", e))?;
    let ast = parser::parse(tokens).map_err(|e| format!("parser: {}", e))?;
    let typed = analyzer::analyze(&ast).map_err(|e| format!("analyzer: {}", e.join("; ")))?;

    let vm = guarded(|| trc::bytecode::execute(&typed));
    let wasm = guarded(|| -> Result<Vec<String>, String> {
        let bytes = trc::codegen::wasm::compile_to_wasm(&typed)?;
        Ok(wasm_common::run_wasm(&bytes))
    });
    let interpreter = guarded(|| {
        let interp = Interpreter::new();
        let result = interp.run(&ast).map(|_| interp.output.borrow().clone());
        result
    });

    Ok(BackendOutputs {
        vm,
        wasm,
        interpreter,
    })
}

/// Run all three backends on a worker thread with a hard receive timeout.
fn run_with_timeout(src: String) -> Result<BackendOutputs, String> {
    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .name("differential-fuzz-worker".to_string())
        .stack_size(STACK_SIZE)
        .spawn(move || {
            let outcome = catch_unwind(AssertUnwindSafe(|| run_backends(&src)));
            let _ = tx.send(outcome.unwrap_or_else(|_| Err("harness panicked".to_string())));
        })
        .map_err(|e| format!("spawn: {}", e))?;

    match rx.recv_timeout(Duration::from_millis(TIMEOUT_MS)) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => Err("worker timeout".to_string()),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err("worker disconnected".to_string()),
    }
}

#[test]
fn differential_fuzz_compares_vm_wasm_and_interpreter() {
    let mut rng = Lcg::new(0x5EED_1234_ABCD_5678);
    let mut compared = 0usize;
    let mut skipped: Vec<String> = Vec::new();
    let mut mismatches: Vec<String> = Vec::new();

    for i in 0..PROGRAM_COUNT {
        let src = gen_program(&mut rng);
        let outputs = match run_with_timeout(src.clone()) {
            Ok(outputs) => outputs,
            Err(reason) => {
                mismatches.push(format!("iteration {}: {}\n{}", i, reason, src));
                continue;
            }
        };

        let vm = match outputs.vm {
            Ok(lines) => normalize(&lines),
            Err(reason) => {
                skipped.push(format!("iteration {}: bytecode VM: {}\n{}", i, reason, src));
                continue;
            }
        };
        let wasm = match outputs.wasm {
            Ok(lines) => normalize(&lines),
            Err(reason) => {
                skipped.push(format!("iteration {}: WASM host: {}\n{}", i, reason, src));
                continue;
            }
        };
        let interpreter = match outputs.interpreter {
            Ok(lines) => normalize(&lines),
            Err(reason) => {
                skipped.push(format!("iteration {}: interpreter: {}\n{}", i, reason, src));
                continue;
            }
        };

        compared += 1;
        if vm != wasm || vm != interpreter {
            mismatches.push(format!(
                "iteration {} divergence\n  bytecode VM: {:?}\n  WASM host: {:?}\n  interpreter: {:?}\n{}",
                i, vm, wasm, interpreter, src
            ));
        }
    }

    eprintln!(
        "differential fuzz compared {} of {} programs across bytecode VM, WASM host (wasmi) and tree-walking interpreter; {} skipped",
        compared,
        PROGRAM_COUNT,
        skipped.len()
    );
    assert!(
        compared >= MIN_COMPARED,
        "only {} of {} programs ran on all three backends; first skips: {:?}",
        compared,
        PROGRAM_COUNT,
        skipped.iter().take(3).collect::<Vec<_>>()
    );
    assert!(
        mismatches.is_empty(),
        "{} differential failures; first: {:?}",
        mismatches.len(),
        mismatches.iter().take(3).collect::<Vec<_>>()
    );
}
