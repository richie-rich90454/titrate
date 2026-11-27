//! Deterministic fuzz harness for randomly generated expression ASTs.
//!
//! Each iteration builds a bounded `ast::Program` whose `main` function
//! evaluates one randomly generated expression. The program is compiled and
//! executed through the bytecode VM inside a worker thread with a timeout.
//! Structured compile/runtime errors are acceptable; panics and hangs are
//! failures.
//!
//! The generator is a linear congruential generator with a fixed seed, so a
//! failure is reproducible from the iteration number alone. No external
//! crates and no host commands are used.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use trc::ast::{
    Access, Declaration, Expr, FnDecl, Literal, Operator, Program, Span, Stmt, Type, UnOp, VarDecl,
};
use trc::bytecode;

/// 64-bit linear congruential generator (Numerical Recipes constants).
struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Lcg {
            state: if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed },
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.state >> 32
    }

    /// Inclusive range.
    fn range(&mut self, lo: usize, hi: usize) -> usize {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u64() as usize % (hi - lo + 1))
    }

    fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

const STRINGS: &[&str] = &["", "a", "abc", "hello", "42", "3.14", "true"];
const CHARS: &[char] = &['a', 'Z', '0', ' ', 'x'];
const BINARY_OPS: &[Operator] = &[
    Operator::Add,
    Operator::Sub,
    Operator::Mul,
    Operator::Div,
    Operator::Mod,
    Operator::Eq,
    Operator::Ne,
    Operator::Lt,
    Operator::Gt,
    Operator::Le,
    Operator::Ge,
    Operator::And,
    Operator::Or,
    Operator::BitAnd,
    Operator::BitOr,
    Operator::BitXor,
    Operator::BitShl,
    Operator::BitShr,
];
const UNARY_OPS: &[UnOp] = &[UnOp::Neg, UnOp::Not, UnOp::BitNot];
const TYPES: &[&str] = &["int", "long", "double", "bool", "string"];
const MAX_DEPTH: usize = 4;

fn span() -> Span {
    Span::unknown()
}

fn random_leaf(rng: &mut Lcg) -> Expr {
    match rng.range(0, 8) {
        0 => Expr::Literal(Literal::Int(rng.range(0, 200) as i64 - 100), span()),
        1 => Expr::Literal(Literal::Float(rng.range(0, 200) as f64 / 7.0 - 14.0), span()),
        2 => Expr::Literal(Literal::Bool(rng.bool()), span()),
        3 => Expr::Literal(Literal::Char(CHARS[rng.range(0, CHARS.len() - 1)]), span()),
        4 => Expr::Literal(
            Literal::String(STRINGS[rng.range(0, STRINGS.len() - 1)].to_string()),
            span(),
        ),
        5 => Expr::Literal(Literal::Null, span()),
        6 => Expr::Identifier("a".to_string(), span()),
        _ => Expr::Identifier("b".to_string(), span()),
    }
}

fn random_expr(rng: &mut Lcg, depth: usize) -> Expr {
    if depth >= MAX_DEPTH {
        return random_leaf(rng);
    }

    match rng.range(0, 12) {
        0..=2 => random_leaf(rng),
        3 => {
            let op = BINARY_OPS[rng.range(0, BINARY_OPS.len() - 1)].clone();
            // Division and modulo always get a non-zero literal divisor.
            let rhs = if matches!(op, Operator::Div | Operator::Mod) {
                Box::new(Expr::Literal(
                    Literal::Int(1 + rng.range(0, 16) as i64),
                    span(),
                ))
            } else {
                Box::new(random_expr(rng, depth + 1))
            };
            Expr::Binary(
                Box::new(random_expr(rng, depth + 1)),
                op,
                rhs,
                span(),
            )
        }
        4 => Expr::Unary(
            UNARY_OPS[rng.range(0, UNARY_OPS.len() - 1)].clone(),
            Box::new(random_expr(rng, depth + 1)),
            span(),
        ),
        5 => Expr::Ternary {
            condition: Box::new(random_expr(rng, depth + 1)),
            then_expr: Box::new(random_expr(rng, depth + 1)),
            else_expr: Box::new(random_expr(rng, depth + 1)),
            span: span(),
        },
        6 => Expr::Cast(
            Box::new(random_expr(rng, depth + 1)),
            Type::simple(TYPES[rng.range(0, TYPES.len() - 1)]),
            span(),
        ),
        7 => Expr::Is(
            Box::new(random_expr(rng, depth + 1)),
            Type::simple(TYPES[rng.range(0, TYPES.len() - 1)]),
            span(),
        ),
        8 => Expr::Tuple(
            (0..rng.range(1, 3))
                .map(|_| random_expr(rng, depth + 1))
                .collect(),
            span(),
        ),
        9 => Expr::Range(
            Box::new(random_expr(rng, depth + 1)),
            Box::new(random_expr(rng, depth + 1)),
            span(),
        ),
        10 => Expr::RangeInclusive(
            Box::new(random_expr(rng, depth + 1)),
            Box::new(random_expr(rng, depth + 1)),
            span(),
        ),
        _ => Expr::Unit(span()),
    }
}

fn random_program(rng: &mut Lcg) -> Program {
    let make_local = |name: &str| {
        Stmt::VarDecl(VarDecl {
            name: name.to_string(),
            typ: Some(Type::simple("int")),
            init: Some(Expr::Literal(Literal::Int(1), span())),
            mutable: true,
            span: span(),
        })
    };

    Program {
        imports: Vec::new(),
        declarations: vec![Declaration::Function(FnDecl {
            access: Access::Public,
            name: "main".to_string(),
            type_params: Vec::new(),
            params: Vec::new(),
            return_type: Some(Type::simple("void")),
            body: vec![
                make_local("a"),
                make_local("b"),
                Stmt::Expr(random_expr(rng, 0)),
            ],
            sugar: false,
            where_clause: Vec::new(),
            span: span(),
        })],
    }
}

fn run_program(program: Program) -> Result<(), String> {
    let mut compiler = bytecode::Compiler::new();
    let compiled = compiler.compile(&program)?;
    let mut vm = bytecode::Vm::new();
    // Bounded expressions cannot loop, but keep a step cap as a backstop.
    vm.set_max_steps(20_000);
    vm.load_program(compiled);
    vm.run()
}

fn run_with_timeout(program: Program) -> Result<Result<(), String>, &'static str> {
    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            let result = catch_unwind(AssertUnwindSafe(|| run_program(program)));
            let _ = tx.send(result);
        })
        .map_err(|_| "spawn-failed")?;

    match rx.recv_timeout(Duration::from_millis(1_000)) {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(_)) => Err("panic"),
        Err(mpsc::RecvTimeoutError::Timeout) => Err("timeout"),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err("panic"),
    }
}

const ITERATIONS: usize = 1_000;

#[test]
fn expression_ast_fuzz_never_panics_or_hangs() {
    let mut rng = Lcg::new(0x4558_5052_4553_5349);
    let mut failures = 0usize;
    let mut succeeded = 0usize;

    for iteration in 0..ITERATIONS {
        let program = random_program(&mut rng);
        match run_with_timeout(program) {
            Ok(Ok(())) => succeeded += 1,
            Ok(Err(_message)) => {
                // Type mismatches and invalid casts are structured errors.
            }
            Err("timeout") => {
                failures += 1;
                if failures <= 3 {
                    eprintln!("Timeout on expression fuzz iteration {}", iteration);
                }
            }
            Err("panic") => {
                failures += 1;
                if failures <= 3 {
                    eprintln!("Panic on expression fuzz iteration {}", iteration);
                }
            }
            Err(other) => {
                failures += 1;
                if failures <= 3 {
                    eprintln!("Thread error on expression fuzz iteration {}: {}", iteration, other);
                }
            }
        }
    }

    assert!(succeeded > 0, "generator produced no runnable expression programs");
    assert_eq!(
        failures, 0,
        "expression fuzz reported {} failures out of {} iterations",
        failures, ITERATIONS
    );
}
