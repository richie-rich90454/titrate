//! Criterion benchmarks for VM startup and compile latency on fixed workloads.
//!
//! Workloads are embedded sources with a known output; each one is verified
//! through the bytecode VM before any timing runs. Setup work (tokenizing for
//! the parse benchmark, analyzing for the bytecode benchmark, and so on) is
//! done outside the measured region. No timing value is hard-coded.
//!
//! Run with `cargo bench -p trc --bench sys_latency`.
//! `TITRATE_BENCH_SMOKE=1` verifies workloads without recording timings.

use std::hint::black_box;
use std::time::{Duration, Instant};

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use trc::{analyzer, bytecode, lexer, parser};

const HELLO: &str = r#"public fn main(): void { io::println("hello"); }"#;
const LOOP_N: i64 = 2_000;
const DECL_COUNT: usize = 64;

struct Workload {
    name: &'static str,
    source: String,
    expected: String,
}

fn loop_source(n: i64) -> String {
    format!(
        r#"public fn main(): void {{
    var sum: long = 0;
    var i: long = 0;
    while (i < {n}) {{
        sum = sum + i;
        i = i + 1;
    }}
    io::println(sum);
}}"#
    )
}

fn decl_source(count: usize) -> String {
    let mut source = String::new();
    for idx in 0..count {
        source.push_str(&format!(
            "fn helper{idx}(): int {{ return {idx}; }}\n"
        ));
    }
    source.push_str(r#"public fn main(): void { io::println(helper0()); }"#);
    source
}

fn workloads() -> Vec<Workload> {
    vec![
        Workload {
            name: "hello",
            source: HELLO.to_string(),
            expected: "hello".to_string(),
        },
        Workload {
            name: "loop_2000",
            source: loop_source(LOOP_N),
            expected: (LOOP_N * (LOOP_N - 1) / 2).to_string(),
        },
        Workload {
            name: "decl_64",
            source: decl_source(DECL_COUNT),
            expected: "0".to_string(),
        },
    ]
}

fn compile_source(source: &str) -> trc::ast::Program {
    let tokens = lexer::tokenize(source).expect("tokenize benchmark workload");
    let ast = parser::parse(tokens).expect("parse benchmark workload");
    analyzer::analyze(&ast)
        .unwrap_or_else(|errors| panic!("analyze benchmark workload: {}", errors.join("\n")))
}

fn verify(workload: &Workload) {
    let output = bytecode::execute(&compile_source(&workload.source))
        .expect("execute benchmark workload on the bytecode VM");
    assert_eq!(
        output,
        vec![workload.expected.clone()],
        "{} output does not match its fixed expected value",
        workload.name
    );
}

fn smoke() {
    let list = workloads();
    for workload in &list {
        verify(workload);
        println!("verified {} -> {}", workload.name, workload.expected);
    }
    println!(
        "smoke: {}/{} workloads verified; no timings run or asserted",
        list.len(),
        list.len()
    );
}

fn benchmark(c: &mut Criterion) {
    if std::env::var_os("TITRATE_BENCH_SMOKE").is_some() {
        smoke();
        return;
    }

    let list = workloads();
    for workload in &list {
        verify(workload);
    }

    let mut group = c.benchmark_group("startup_latency");
    group.bench_function("vm_new", |b| {
        b.iter(|| {
            let vm = bytecode::Vm::new();
            black_box(vm.output.len());
        });
    });
    for workload in &list {
        let program = compile_source(&workload.source);
        group.bench_function(BenchmarkId::new("vm_new_and_load", workload.name), |b| {
            b.iter_custom(|iters| {
                let mut elapsed = Duration::ZERO;
                for _ in 0..iters {
                    // Compilation is setup; only construction and loading are
                    // measured, and each iteration gets a fresh program.
                    let mut compiler = bytecode::Compiler::new();
                    let compiled = compiler.compile(&program).expect("compile setup program");
                    let start = Instant::now();
                    let mut vm = bytecode::Vm::new();
                    vm.load_program(compiled);
                    elapsed += start.elapsed();
                    black_box(vm.output.len());
                }
                elapsed
            });
        });
    }
    group.finish();

    let mut group = c.benchmark_group("compile_latency");
    for workload in &list {
        let tokens = lexer::tokenize(&workload.source).expect("tokenize benchmark workload");
        group.bench_function(BenchmarkId::new("lex", workload.name), |b| {
            b.iter(|| {
                let result = lexer::tokenize(black_box(workload.source.as_str()));
                black_box(result);
            });
        });
        group.bench_function(BenchmarkId::new("parse", workload.name), |b| {
            b.iter(|| {
                let result = parser::parse(black_box(tokens.clone()));
                black_box(result);
            });
        });
        let ast = parser::parse(tokens).expect("parse benchmark workload");
        group.bench_function(BenchmarkId::new("analyze", workload.name), |b| {
            b.iter(|| {
                let result = analyzer::analyze(black_box(&ast));
                black_box(result);
            });
        });
        let typed = analyzer::analyze(&ast)
            .unwrap_or_else(|errors| panic!("analyze benchmark workload: {}", errors.join("\n")));
        group.bench_function(BenchmarkId::new("bytecode", workload.name), |b| {
            b.iter(|| {
                let mut compiler = bytecode::Compiler::new();
                let result = compiler.compile(black_box(&typed));
                black_box(result);
            });
        });
    }
    group.finish();

    let mut group = c.benchmark_group("pipeline_latency");
    for workload in &list {
        let typed = compile_source(&workload.source);
        group.bench_function(BenchmarkId::new("compile_and_run", workload.name), |b| {
            b.iter(|| {
                let output =
                    bytecode::execute(black_box(&typed)).expect("execute benchmark workload");
                black_box(output);
            });
        });
        group.bench_function(BenchmarkId::new("source_to_output", workload.name), |b| {
            b.iter(|| {
                let output = bytecode::execute(&compile_source(black_box(
                    workload.source.as_str(),
                )))
                .expect("execute benchmark workload");
                black_box(output);
            });
        });
    }
    group.finish();
}

criterion_group!(sys_latency, benchmark);
criterion_main!(sys_latency);
