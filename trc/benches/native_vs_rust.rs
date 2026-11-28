//! Criterion suite comparing Titrate native output with equivalent Rust kernels.
//!
//! Each kernel has a fixed input and a single integer output. Before any
//! timing, the bytecode VM output and the compiled native executable output
//! must both match the Rust kernel byte-for-byte. A kernel that cannot be
//! verified is reported as "not measured"; no number is invented. After the
//! Criterion groups finish, measured medians and losses are printed to stdout.
//!
//! Run with `cargo bench -p trc --bench native_vs_rust`.
//! `TITRATE_BENCH_SMOKE=1` verifies outputs only.
//! `TITRATE_BENCH_NO_NATIVE=1` skips the native backend.

use std::fs;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use trc::bytecode;

const SUM_N: i64 = 100_000;
const FIB_N: i32 = 90;
const GRID_N: i32 = 300;
const LOSS_SAMPLES: usize = 5;

struct Kernel {
    name: &'static str,
    source: String,
    rust: fn() -> String,
}

fn sum_source() -> String {
    format!(
        r#"
public fn sumSquares(n: long): long {{
    var sum: long = 0;
    var i: long = 0;
    while (i < n) {{
        sum = sum + i * i;
        i = i + 1;
    }}
    return sum;
}}
public fn main(): void {{ io::println(sumSquares({SUM_N})); }}
"#
    )
}

fn fib_source() -> String {
    format!(
        r#"
public fn fibIter(n: int): long {{
    var a: long = 0;
    var b: long = 1;
    var i: int = 0;
    while (i < n) {{
        let next: long = a + b;
        a = b;
        b = next;
        i = i + 1;
    }}
    return a;
}}
public fn main(): void {{ io::println(fibIter({FIB_N})); }}
"#
    )
}

fn grid_source() -> String {
    format!(
        r#"
public fn gridAcc(n: int): long {{
    var acc: long = 0;
    var i: int = 0;
    while (i < n) {{
        var j: int = 0;
        while (j < n) {{
            acc = acc + (i as long) * (j as long);
            j = j + 1;
        }}
        i = i + 1;
    }}
    return acc;
}}
public fn main(): void {{ io::println(gridAcc({GRID_N})); }}
"#
    )
}

fn rust_sum() -> String {
    (0..SUM_N).map(|i| i * i).sum::<i64>().to_string()
}

fn rust_fib() -> String {
    let (mut a, mut b) = (0_i64, 1_i64);
    for _ in 0..FIB_N {
        let next = a + b;
        a = b;
        b = next;
    }
    a.to_string()
}

fn rust_grid() -> String {
    let mut acc = 0_i64;
    for i in 0..GRID_N as i64 {
        for j in 0..GRID_N as i64 {
            acc += i * j;
        }
    }
    acc.to_string()
}

fn kernels() -> Vec<Kernel> {
    vec![
        Kernel {
            name: "sum_squares",
            source: sum_source(),
            rust: rust_sum,
        },
        Kernel {
            name: "fib_iter",
            source: fib_source(),
            rust: rust_fib,
        },
        Kernel {
            name: "grid_acc",
            source: grid_source(),
            rust: rust_grid,
        },
    ]
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("trc should be inside the workspace")
        .to_path_buf()
}

fn trc_binary() -> Option<PathBuf> {
    let exe = if cfg!(windows) { "trc.exe" } else { "trc" };
    for profile in &["release", "debug"] {
        let candidate = workspace_root().join("target").join(profile).join(exe);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Compile a kernel to a native executable during setup. Absent prerequisites
/// mean no timing is reported for this kernel.
fn native_exe(kernel: &Kernel, dir: &Path) -> Option<PathBuf> {
    if std::env::var_os("TITRATE_BENCH_NO_NATIVE").is_some() {
        eprintln!("native {} skipped: TITRATE_BENCH_NO_NATIVE is set", kernel.name);
        return None;
    }
    let trc = match trc_binary() {
        Some(path) => path,
        None => {
            eprintln!(
                "native {} skipped: build trc first (`cargo build -p trc --release`)",
                kernel.name
            );
            return None;
        }
    };
    let source = dir.join(format!("comp_{}.tr", kernel.name));
    fs::write(&source, &kernel.source).unwrap_or_else(|e| panic!("write native source: {e}"));
    let out = Command::new(&trc)
        .arg("--native")
        .arg("--release")
        .arg(&source)
        .output()
        .unwrap_or_else(|e| panic!("invoke trc --native: {e}"));
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        eprintln!(
            "native {} skipped: compilation failed: {}",
            kernel.name,
            stderr.chars().take(400).collect::<String>()
        );
        return None;
    }
    let exe_name = if cfg!(windows) {
        format!("comp_{}_native.exe", kernel.name)
    } else {
        format!("comp_{}_native", kernel.name)
    };
    let exe = dir.join(exe_name);
    if !exe.is_file() {
        eprintln!("native {} skipped: no binary at {}", kernel.name, exe.display());
        return None;
    }
    Some(exe)
}

fn run_native_once(path: &Path) -> String {
    let out = Command::new(path)
        .output()
        .unwrap_or_else(|e| panic!("run native binary {}: {}", path.display(), e));
    assert!(
        out.status.success(),
        "native binary failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("native output should be UTF-8")
}

fn bytecode_output(source: &str) -> String {
    bytecode::execute(&compile_source(source)).expect("execute kernel on the bytecode VM")
}

fn compile_source(source: &str) -> trc::ast::Program {
    let tokens = trc::lexer::tokenize(source).expect("tokenize kernel");
    let ast = trc::parser::parse(tokens).expect("parse kernel");
    trc::analyzer::analyze(&ast)
        .unwrap_or_else(|errors| panic!("analyze kernel: {}", errors.join("\n")))
}

/// Return the native path only after bytecode and native agree with Rust.
fn verify(kernel: &Kernel, dir: &Path) -> Option<PathBuf> {
    let expected = (kernel.rust)();
    let bytecode = bytecode_output(&kernel.source).trim().to_string();
    assert_eq!(
        bytecode, expected,
        "{} bytecode output does not match Rust",
        kernel.name
    );
    let exe = native_exe(kernel, dir);
    if let Some(path) = &exe {
        let native = run_native_once(path).trim().to_string();
        assert_eq!(
            native, expected,
            "{} native output does not match Rust",
            kernel.name
        );
    }
    exe
}

fn sample(mut run: impl FnMut()) -> Vec<Duration> {
    (0..LOSS_SAMPLES)
        .map(|_| {
            let start = Instant::now();
            run();
            start.elapsed()
        })
        .collect()
}

fn median(samples: &[Duration]) -> Duration {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    sorted[sorted.len() / 2]
}

fn print_losses(verified: &[(Kernel, Option<PathBuf>)]) {
    println!();
    println!(
        "Native vs Rust loss (medians from {} samples per backend in this process):",
        LOSS_SAMPLES
    );
    println!(
        "{:<16} {:>14} {:>14} {:>16}",
        "kernel", "rust ms", "native ms", "loss"
    );
    for (kernel, exe) in verified {
        let path = match exe {
            Some(path) => path,
            None => {
                println!(
                    "{:<16} {:>14} {:>14} {:>16}",
                    kernel.name, "measured", "not measured", "not measured"
                );
                continue;
            }
        };
        let rust = median(&sample(|| black_box((kernel.rust)())));
        let native = median(&sample(|| {
            let output = Command::new(path).output().expect("run native kernel");
            black_box(output);
        }));
        let rust_ms = rust.as_secs_f64() * 1000.0;
        let native_ms = native.as_secs_f64() * 1000.0;
        let ratio = native_ms / rust_ms;
        println!(
            "{:<16} {:>14.3} {:>14.3} {:>15.2}x ({:+.1}%)",
            kernel.name,
            rust_ms,
            native_ms,
            ratio,
            (ratio - 1.0) * 100.0
        );
    }
    println!("Loss = native median / Rust median - 1. No figure here is precomputed.");
}

fn benchmark(c: &mut Criterion) {
    let dir = std::env::temp_dir().join(format!("titrate_native_vs_rust_{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create native benchmark temp dir");

    let mut verified = Vec::new();
    for kernel in kernels() {
        let exe = verify(&kernel, &dir);
        verified.push((kernel, exe));
    }

    if std::env::var_os("TITRATE_BENCH_SMOKE").is_some() {
        for (kernel, exe) in &verified {
            if exe.is_some() {
                println!("smoke: {} bytecode/native/Rust outputs match", kernel.name);
            } else {
                println!("smoke: {} bytecode/Rust outputs match (native skipped)", kernel.name);
            }
        }
        println!(
            "smoke: {}/{} kernels verified; no timings run or asserted",
            verified.len(),
            verified.len()
        );
        let _ = fs::remove_dir_all(&dir);
        return;
    }

    let mut group = c.benchmark_group("native_vs_rust");
    for (kernel, exe) in &verified {
        group.bench_function(BenchmarkId::new(kernel.name, "rust"), |b| {
            let rust = kernel.rust;
            b.iter(|| black_box(rust()));
        });
        if let Some(path) = exe {
            group.bench_function(BenchmarkId::new(kernel.name, "titrate_native"), |b| {
                b.iter_custom(|iters| {
                    let mut elapsed = Duration::ZERO;
                    for _ in 0..iters {
                        // Includes process launch: the native backend has no
                        // in-process execution API to measure instead.
                        let start = Instant::now();
                        let output = Command::new(path).output();
                        elapsed += start.elapsed();
                        black_box(output);
                    }
                    elapsed
                });
            });
        }
    }
    group.finish();

    print_losses(&verified);
    let _ = fs::remove_dir_all(&dir);
}

criterion_group!(native_vs_rust, benchmark);
criterion_main!(native_vs_rust);
