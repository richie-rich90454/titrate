//! Expression-generation harness: bounded random AST fragments are compiled
//! and executed on the bytecode VM. Structured compile/runtime errors are
//! acceptable; panics are failures.

use std::panic::{catch_unwind, AssertUnwindSafe};

use trc::ast::{
    Access, Declaration, Expr, FnDecl, Literal, Operator, Program, Span, Stmt, Type, UnOp, VarDecl,
};
use trc::bytecode;

use crate::common::Lcg;

const STRINGS: &[&str] = &["", "a", "abc", "hello", "42", "3.14", "true"];
const CHARS: &[char] = &['a', 'Z', '0', ' ', 'x'];
const TYPES: &[&str] = &["int", "long", "double", "bool", "string"];
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
const MAX_DEPTH: usize = 4;

fn span() -> Span {
    Span::unknown()
}

fn random_leaf(rng: &mut Lcg) -> Expr {
    match rng.range(0, 8) {
        0 => Expr::Literal(Literal::Int(rng.range(0, 200) as i64 - 100), span()),
        1 => Expr::Literal(Literal::Float(rng.range(0, 200) as f64 / 7.0 - 14.0), span()),
        2 => Expr::Literal(Literal::Bool(rng.boolean()), span()),
        3 => Expr::Literal(Literal::Char(*rng.pick(CHARS)), span()),
        4 => Expr::Literal(
            Literal::String((*rng.pick(STRINGS)).to_string()),
            span(),
        ),
        5 => Expr::Literal(Literal::Null, span()),
        6 => Expr::Identifier("a".to_string(), span()),
        _ => Expr::Identifier("b".to_string(), span()),
    }
}

fn random_expr_at(rng: &mut Lcg, depth: usize) -> Expr {
    if depth >= MAX_DEPTH {
        return random_leaf(rng);
    }
    match rng.range(0, 12) {
        0..=2 => random_leaf(rng),
        3 => {
            let op = (*rng.pick(BINARY_OPS)).clone();
            // Division and modulo always get a non-zero literal divisor.
            let rhs = if matches!(op, Operator::Div | Operator::Mod) {
                Box::new(Expr::Literal(Literal::Int(1 + rng.range(0, 16) as i64), span()))
            } else {
                Box::new(random_expr_at(rng, depth + 1))
            };
            Expr::Binary(Box::new(random_expr_at(rng, depth + 1)), op, rhs, span())
        }
        4 => Expr::Unary(
            (*rng.pick(UNARY_OPS)).clone(),
            Box::new(random_expr_at(rng, depth + 1)),
            span(),
        ),
        5 => Expr::Ternary {
            condition: Box::new(random_expr_at(rng, depth + 1)),
            then_expr: Box::new(random_expr_at(rng, depth + 1)),
            else_expr: Box::new(random_expr_at(rng, depth + 1)),
            span: span(),
        },
        6 => Expr::Cast(
            Box::new(random_expr_at(rng, depth + 1)),
            Type::simple(rng.pick(TYPES)),
            span(),
        ),
        7 => Expr::Is(
            Box::new(random_expr_at(rng, depth + 1)),
            Type::simple(rng.pick(TYPES)),
            span(),
        ),
        8 => Expr::Tuple(
            (0..rng.range(1, 3))
                .map(|_| random_expr_at(rng, depth + 1))
                .collect(),
            span(),
        ),
        9 => Expr::Range(
            Box::new(random_expr_at(rng, depth + 1)),
            Box::new(random_expr_at(rng, depth + 1)),
            span(),
        ),
        10 => Expr::RangeInclusive(
            Box::new(random_expr_at(rng, depth + 1)),
            Box::new(random_expr_at(rng, depth + 1)),
            span(),
        ),
        _ => Expr::Unit(span()),
    }
}

/// One bounded random AST fragment, usable by other harnesses and tests.
pub fn random_fragment(rng: &mut Lcg) -> Expr {
    random_expr_at(rng, 0)
}

/// A deliberately malformed fragment: a literal division by zero must be
/// reported as a structured VM error, never silently evaluated.
pub fn division_by_zero_fragment() -> Expr {
    Expr::Binary(
        Box::new(Expr::Literal(Literal::Int(1), span())),
        Operator::Div,
        Box::new(Expr::Literal(Literal::Int(0), span())),
        span(),
    )
}

/// Compile and run one fragment in a minimal `main` with two int locals.
pub fn run_fragment(expression: Expr) -> Result<Vec<String>, String> {
    let local = |name: &str| {
        Stmt::VarDecl(VarDecl {
            name: name.to_string(),
            typ: Some(Type::simple("int")),
            init: Some(Expr::Literal(Literal::Int(1), span())),
            mutable: true,
            span: span(),
        })
    };
    let program = Program {
        imports: Vec::new(),
        declarations: vec![Declaration::Function(FnDecl {
            access: Access::Public,
            name: "main".to_string(),
            type_params: Vec::new(),
            params: Vec::new(),
            return_type: Some(Type::simple("void")),
            body: vec![local("a"), local("b"), Stmt::Expr(expression)],
            sugar: false,
            where_clause: Vec::new(),
            span: span(),
        })],
    };
    let mut compiler = bytecode::Compiler::new();
    let compiled = compiler.compile(&program)?;
    let mut vm = bytecode::Vm::new();
    // Bounded fragments cannot loop; this is a backstop against hangs.
    vm.set_max_steps(20_000);
    vm.load_program(compiled);
    vm.run()?;
    Ok(vm.output)
}

pub fn run(iterations: usize, seed: u64) -> Result<(), String> {
    let mut rng = Lcg::new(seed);
    let mut succeeded = 0usize;
    for iteration in 0..iterations {
        let expression = random_fragment(&mut rng);
        let outcome = catch_unwind(AssertUnwindSafe(|| run_fragment(expression)));
        match outcome {
            Ok(Ok(_)) => succeeded += 1,
            Ok(Err(_)) => {
                // Type mismatches, invalid casts, and runtime faults are
                // structured errors and are expected from random ASTs.
            }
            Err(_) => {
                return Err(format!("iteration {}: expression fragment panicked", iteration));
            }
        }
    }
    if succeeded == 0 {
        return Err("generator produced no runnable expression fragments".to_string());
    }
    Ok(())
}
