//! Fixed-workload comparison benchmarks: Titrate bytecode, Titrate native,
//! and equivalent Rust kernels over identical inputs.
//!
//! Every kernel is verified before any timing is recorded: the bytecode
//! output, the native process output, and the Rust baseline result must agree
//! (floating point is compared with a relative tolerance). Native compilation
//! happens in setup, outside the measured region; the native measurement does
//! include process launch because the native backend has no in-process API.
//!
//! Run with `cargo bench -p trc --bench comparison_kernels`, or through
//! `scripts/bench-comparison.ps1` which prints a table including losses.
//! `TITRATE_BENCH_SMOKE=1` runs output verification only, with no timings.
//! `TITRATE_BENCH_NO_NATIVE=1` skips the native backend entirely.

use std::fs;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use trc::bytecode::{Compiler, Vm};
use trc::{analyzer, bytecode, lexer, parser};

const SUM_N: i64 = 50_000;
const WATER_ATOMS: i32 = 48;
const WATER_SPACING: f64 = 3.166;
const WATER_SIGMA: f64 = 3.166;
const WATER_EPSILON: f64 = 0.1554;
const CONTAINER_N: i32 = 2_000;
const STRING_N: i32 = 500;
const STRING_PATTERN: &str = "abcde";
const PATTERN_LEN: i32 = 5;

#[derive(Clone, Copy)]
enum OutputKind {
    Exact,
    Float,
}

struct Kernel {
    name: &'static str,
    source: String,
    rust: fn() -> String,
    kind: OutputKind,
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

public fn main(): void {{
    io::println(sumSquares({SUM_N}));
}}
"#
    )
}

fn water_source() -> String {
    format!(
        r#"
public fn computeLJEnergy(n: int, sigma: double, epsilon: double): double {{
    let spacing: double = {WATER_SPACING};
    var e: double = 0.0;
    var i: int = 0;
    while (i < n) {{
        var j: int = i + 1;
        while (j < n) {{
            let ix: double = (i / 4) as double * spacing;
            let iy: double = ((i % 4) / 2) as double * spacing;
            let iz: double = (i % 2) as double * spacing;
            let jx: double = (j / 4) as double * spacing;
            let jy: double = ((j % 4) / 2) as double * spacing;
            let jz: double = (j % 2) as double * spacing;
            let dx: double = ix - jx;
            let dy: double = iy - jy;
            let dz: double = iz - jz;
            let r2: double = dx * dx + dy * dy + dz * dz;
            var r: double = r2;
            var k: int = 0;
            while (k < 20) {{
                r = 0.5 * (r + r2 / r);
                k = k + 1;
            }}
            if (r > 0.001) {{
                let sr: double = sigma / r;
                let sr6: double = sr * sr * sr * sr * sr * sr;
                let sr12: double = sr6 * sr6;
                e = e + 4.0 * epsilon * (sr12 - sr6);
            }}
            j = j + 1;
        }}
        i = i + 1;
    }}
    return e;
}}

public fn main(): void {{
    io::println(computeLJEnergy({WATER_ATOMS}, {WATER_SIGMA}, {WATER_EPSILON}));
}}
"#,
        WATER_SPACING = WATER_SPACING,
        WATER_ATOMS = WATER_ATOMS,
        WATER_SIGMA = WATER_SIGMA,
        WATER_EPSILON = WATER_EPSILON,
    )
}

fn container_source() -> String {
    format!(
        r#"
public fn iterateContainer(n: int): int {{
    let list: ArrayList<int> = new ArrayList<int>();
    var i: int = 0;
    while (i < n) {{
        list.add(i);
        i = i + 1;
    }}
    var sum: int = 0;
    var j: int = 0;
    while (j < list.size()) {{
        sum = sum + list.get(j);
        j = j + 1;
    }}
    return sum;
}}

public fn main(): void {{
    io::println(iterateContainer({CONTAINER_N}));
}}
"#
    )
}

fn string_source() -> String {
    format!(
        r#"
public fn processString(n: int): string {{
    var s: string = "";
    var i: int = 0;
    while (i < n) {{
        s = s + String.charAt("{STRING_PATTERN}", i % {PATTERN_LEN});
        i = i + 1;
    }}
    return s;
}}

public fn main(): void {{
    io::println(processString({STRING_N}));
}}
"#,
        STRING_PATTERN = STRING_PATTERN,
        PATTERN_LEN = PATTERN_LEN,
    )
}

fn rust_sum_squares() -> String {
    let sum: i64 = (0..SUM_N).map(|i| i * i).sum();
    sum.to_string()
}

fn rust_water() -> String {
    let spacing = WATER_SPACING;
    let mut e = 0.0_f64;
    for i in 0..WATER_ATOMS {
        let mut j = i + 1;
        while j < WATER_ATOMS {
            let ix = (i / 4) as f64 * spacing;
            let iy = ((i % 4) / 2) as f64 * spacing;
            let iz = (i % 2) as f64 * spacing;
            let jx = (j / 4) as f64 * spacing;
            let jy = ((j % 4) / 2) as f64 * spacing;
            let jz = (j % 2) as f64 * spacing;
            let dx = ix - jx;
            let dy = iy - jy;
            let dz = iz - jz;
            let r2 = dx * dx + dy * dy + dz * dz;
            let mut r = r2;
            for _ in 0..20 {
                r = 0.5 * (r + r2 / r);
            }
            if r > 0.001 {
                let sr = WATER_SIGMA / r;
                let sr6 = sr * sr * sr * sr * sr * sr;
                let sr12 = sr6 * sr6;
                e += 4.0 * WATER_EPSILON * (sr12 - sr6);
            }
            j += 1;
        }
    }
    e.to_string()
}

fn rust_container() -> String {
    let list: Vec<i32> = (0..CONTAINER_N).collect();
    let sum: i64 = list.iter().map(|&x| x as i64).sum();
    sum.to_string()
}

fn rust_string() -> String {
    let mut s = String::with_capacity(STRING_N as usize);
    for i in 0..STRING_N {
        s.push(STRING_PATTERN.as_bytes()[(i % PATTERN_LEN) as usize] as char);
    }
    s
}

fn kernels() -> Vec<Kernel> {
    vec![
        Kernel {
            name: "sum_of_squares",
            source: sum_source(),
            rust: rust_sum_squares,
            kind: OutputKind::Exact,
        },
        Kernel {
            name: "water_box",
            source: water_source(),
            rust: rust_water,
            kind: OutputKind::Float,
        },
        Kernel {
            name: "container_iteration",
            source: container_source(),
            rust: rust_container,
            kind: OutputKind::Exact,
        },
        Kernel {
            name: "string_processing",
            source: string_source(),
            rust: rust_string,
            kind: OutputKind::Exact,
        },
    ]
}

fn compile_program(source: &str) -> trc::ast::Program {
    let tokens = lexer::tokenize(source).expect("tokenize benchmark source");
    let ast = parser::parse(tokens).expect("parse benchmark source");
    analyzer::analyze(&ast)
        .unwrap_or_else(|errs| panic!("analyze benchmark source: {}", errs.join("\n")))
}

fn compile_bytecode(program: &trc::ast::Program) -> bytecode::CompiledProgram {
    let mut compiler = Compiler::new();
    compiler.compile(program).expect("compile benchmark source")
}

fn bytecode_output(program: &trc::ast::Program) -> String {
    let compiled = compile_bytecode(program);
    let mut vm = Vm::new();
    vm.load_program(compiled);
    vm.run().expect("run benchmark source on the bytecode VM");
    vm.output.join("\n")
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

/// Compile a kernel to a native executable in setup. Returns `None` (with a
/// clear warning) when the native prerequisites are absent; no timing is
/// reported for a backend whose output was never verified.
fn native_exe(kernel: &Kernel, dir: &Path) -> Option<PathBuf> {
    if std::env::var_os("TITRATE_BENCH_NO_NATIVE").is_some() {
        eprintln!("native {} skipped: TITRATE_BENCH_NO_NATIVE is set", kernel.name);
        return None;
    }
    let trc = match trc_binary() {
        Some(path) => path,
        None => {
            eprintln!(
                "native {} skipped: trc binary not found (build with `cargo build -p trc --release`)",
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
            "native {} skipped: trc --native --release failed: {}",
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

fn run_native_once(exe: &Path) -> String {
    let out = Command::new(exe)
        .output()
        .unwrap_or_else(|e| panic!("run native binary {}: {}", exe.display(), e));
    assert!(
        out.status.success(),
        "native benchmark binary failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("native output should be UTF-8")
}

fn assert_output(kernel: &str, expected: &str, actual: &str, kind: OutputKind) {
    let actual = actual.trim();
    match kind {
        OutputKind::Exact => assert_eq!(actual, expected, "{} output mismatch", kernel),
        OutputKind::Float => {
            let want: f64 = expected.parse().expect("Rust baseline should be a float");
            let got: f64 = actual
                .parse()
                .unwrap_or_else(|_| panic!("{} output is not a float: {}", kernel, actual));
            assert!(
                (got - want).abs() <= 1e-9 * want.abs().max(1.0),
                "{} output mismatch: got {}, expected {}",
                kernel,
                got,
                want
            );
        }
    }
}

fn verify(kernel: &Kernel, dir: &Path) -> Option<PathBuf> {
    let program = compile_program(&kernel.source);
    let expected = (kernel.rust)();
    let bytecode = bytecode_output(&program);
    assert_output(kernel.name, &expected, &bytecode, kernel.kind);

    let exe = native_exe(kernel, dir);
    if let Some(path) = &exe {
        let native = run_native_once(path);
        assert_output(kernel.name, &expected, &native, kernel.kind);
    }
    exe
}

fn benchmark_group(c: &mut Criterion) {
    if std::env::var_os("TITRATE_BENCH_SMOKE").is_some() {
        smoke();
        return;
    }
    let dir = std::env::temp_dir().join(format!(
        "titrate_comparison_bench_{}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("create native benchmark temp dir");

    let mut group = c.benchmark_group("comparison_kernels");
    for kernel in kernels() {
        let exe = verify(&kernel, &dir);

        group.bench_function(BenchmarkId::new(kernel.name, "rust"), |b| {
            let rust = kernel.rust;
            b.iter(|| black_box(rust()));
        });

        let program = compile_program(&kernel.source);
        group.bench_function(BenchmarkId::new(kernel.name, "titrate_bytecode"), |b| {
            b.iter_custom(|iters| {
                let mut elapsed = Duration::ZERO;
                for _ in 0..iters {
                    // Compilation is setup, not part of the measured region.
                    let compiled = compile_bytecode(&program);
                    let start = Instant::now();
                    let mut vm = Vm::new();
                    vm.load_program(compiled);
                    let result = vm.run();
                    elapsed += start.elapsed();
                    black_box(result);
                }
                elapsed
            });
        });

        if let Some(exe) = exe {
            group.bench_function(BenchmarkId::new(kernel.name, "titrate_native"), |b| {
                b.iter_custom(|iters| {
                    let mut elapsed = Duration::ZERO;
                    for _ in 0..iters {
                        // Includes process launch: the native backend has no
                        // in-process execution API to benchmark instead.
                        let start = Instant::now();
                        let out = Command::new(&exe).output();
                        elapsed += start.elapsed();
                        black_box(out);
                    }
                    elapsed
                });
            });
        }
    }
    group.finish();

    let _ = fs::remove_dir_all(&dir);
}

fn smoke() {
    let dir = std::env::temp_dir().join(format!(
        "titrate_comparison_bench_smoke_{}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("create native benchmark temp dir");
    let kernels = kernels();
    for kernel in &kernels {
        let exe = verify(kernel, &dir);
        if exe.is_some() {
            println!("smoke: {} bytecode/native/Rust outputs match", kernel.name);
        } else {
            println!("smoke: {} bytecode/Rust outputs match (native skipped)", kernel.name);
        }
    }
    let _ = fs::remove_dir_all(&dir);
    println!(
        "smoke: {}/{} kernels verified; no timings run or asserted",
        kernels.len(),
        kernels.len()
    );
}

criterion_group!(comparison_kernels, benchmark_group);
criterion_main!(comparison_kernels);
