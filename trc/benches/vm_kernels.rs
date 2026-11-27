//! Criterion benchmarks for the Titrate bytecode VM.
//!
//! Each benchmark embeds its Titrate source as a string and runs the full
//! `bytecode::execute` pipeline (compile + VM execution). Sources are parsed
//! and analyzed once during setup so the measured region is the executable
//! pipeline, not source loading. No network or file access is involved.

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use trc::{analyzer, bytecode, lexer, parser};

const FIBONACCI: &str = r#"
fn fib(n: int): int {
    if (n <= 1) {
        return n;
    }
    return fib(n - 1) + fib(n - 2);
}

fn main(): void {
    let r: int = fib(18);
    io::println(r);
}
"#;

const SUM_OF_SQUARES: &str = r#"
fn main(): void {
    var sum = 0;
    var i = 0;
    while (i < 1000) {
        sum = sum + i * i;
        i = i + 1;
    }
    io::println(sum);
}
"#;

const STRING_CONCAT: &str = r#"
fn main(): void {
    var s = "";
    var i = 0;
    while (i < 500) {
        s = s + "x";
        i = i + 1;
    }
    io::println(s);
}
"#;

const ARRAYLIST_BUILD_ITERATE: &str = r#"
fn main(): void {
    let list: ArrayList<int> = new ArrayList<int>();
    var i = 0;
    while (i < 2000) {
        list.add(i);
        i = i + 1;
    }
    var sum = 0;
    for (x in list) {
        sum = sum + x;
    }
    io::println(sum);
}
"#;

const HASHMAP_PUT_GET: &str = r#"
fn main(): void {
    let map: HashMap<string, int> = new HashMap<string, int>();
    var i = 0;
    while (i < 400) {
        map.put(Integer.toString(i), i);
        i = i + 1;
    }
    var sum = 0;
    var j = 0;
    while (j < 400) {
        sum = sum + map.get(Integer.toString(j));
        j = j + 1;
    }
    io::println(sum);
}
"#;

const NDARRAY_NESTED_LOOPS: &str = r#"
fn main(): void {
    var acc = 0;
    var i = 0;
    while (i < 64) {
        var j = 0;
        while (j < 64) {
            var k = 0;
            while (k < 8) {
                acc = acc + (i * j + k);
                k = k + 1;
            }
            j = j + 1;
        }
        i = i + 1;
    }
    io::println(acc);
}
"#;

fn compile_source(source: &str) -> trc::ast::Program {
    let tokens = lexer::tokenize(source).expect("tokenize benchmark source");
    let ast = parser::parse(tokens).expect("parse benchmark source");
    analyzer::analyze(&ast).unwrap_or_else(|errs| panic!("analyze benchmark source: {}", errs.join("\n")))
}

fn bench_source(c: &mut Criterion, name: &str, source: &str) {
    let typed = compile_source(source);
    c.bench_function(name, |b| {
        b.iter(|| {
            let output = bytecode::execute(black_box(&typed)).expect("execute benchmark source");
            black_box(output);
        });
    });
}

fn fibonacci(c: &mut Criterion) {
    bench_source(c, "fibonacci_recursive", FIBONACCI);
}

fn sum_of_squares(c: &mut Criterion) {
    bench_source(c, "sum_of_squares", SUM_OF_SQUARES);
}

fn string_concat(c: &mut Criterion) {
    bench_source(c, "string_concat", STRING_CONCAT);
}

fn arraylist_build_iterate(c: &mut Criterion) {
    bench_source(c, "arraylist_build_iterate", ARRAYLIST_BUILD_ITERATE);
}

fn hashmap_put_get(c: &mut Criterion) {
    bench_source(c, "hashmap_put_get", HASHMAP_PUT_GET);
}

fn ndarray_nested_loops(c: &mut Criterion) {
    bench_source(c, "ndarray_nested_loops", NDARRAY_NESTED_LOOPS);
}

criterion_group!(
    vm_kernels,
    fibonacci,
    sum_of_squares,
    string_concat,
    arraylist_build_iterate,
    hashmap_put_get,
    ndarray_nested_loops
);
criterion_main!(vm_kernels);
