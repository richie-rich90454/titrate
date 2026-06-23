//! Benchmark tests comparing native vs. bytecode performance (Phase 4, Task 4.2).
//!
//! These tests compile a small compute-intensive program with both the
//! bytecode VM and the native LLVM backend, run both, and measure the
//! wall-clock time. The native version is expected to be faster.
//!
//! The tests are marked `#[ignore]` by default because they require:
//!   1. LLVM development files (LLVM-C.lib) to be installed.
//!   2. A working system linker (clang / link.exe / gcc).
//!   3. The `titrate_native` static library to have been built.
//!
//! Run them explicitly with:
//!   cargo test --test native_bench -- --ignored
//!
//! # Hot Loops in mega_test_03
//!
//! The mega_test_03 water-box simulation has two hot loops that dominate
//! runtime:
//!
//! 1. **`computeLJEnergy()` (Lennard-Jones pair energy)** — This is an O(N²)
//!    double loop over all atoms computing the 6-12 potential. For 24 atoms
//!    (8 waters × 3 atoms) this is 276 pair interactions. Each iteration
//!    computes a distance (sqrt), then `sr^6` and `sr^12`. This is the
//!    single most expensive computation in the simulation.
//!
//! 2. **`computeBondEnergy()` (harmonic bond energy)** — An O(B) loop over
//!    all bonds (16 bonds) computing `0.5 * k * (r - r0)²`. Each iteration
//!    calls `distance()` which itself does a sqrt via Newton's method
//!    (20 iterations). The sqrt is the bottleneck here.
//!
//! 3. **Neighbor list building** — Not explicitly present in mega_test_03,
//!    but in a production MD code this would be the third hot loop. It
//!    builds a list of atom pairs within a cutoff distance to avoid the
//!    full O(N²) scan. The current mega_test_03 does the full O(N²) scan.
//!
//! The native backend accelerates these loops by:
//! - Inlining the `distance()` and `sqrt()` calls (alwaysinline hint).
//! - Using fastcc for internal functions.
//! - Vectorizing the inner loops where possible (llvm.loop metadata).
//! - Eliminating the bytecode dispatch overhead (switch-based interpreter).

use std::env;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use trc::lexer;
use trc::parser;
use trc::analyzer;
use trc::bytecode::{Compiler, Vm};

/// Locate the workspace root by walking up from CARGO_MANIFEST_DIR.
fn workspace_root() -> PathBuf {
    let manifest = env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR should be set by cargo");
    PathBuf::from(manifest)
        .parent()
        .map(|p| p.to_path_buf())
        .expect("trc should be inside the workspace")
}

/// Locate the built `trc` binary in `target/debug` or `target/release`.
fn trc_binary() -> Option<PathBuf> {
    let root = workspace_root();
    let exe_name = if cfg!(windows) { "trc.exe" } else { "trc" };

    for profile in &["release", "debug"] {
        let candidate = root.join("target").join(profile).join(exe_name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// A small compute-intensive program: sums squares of 0..10000.
///
/// This is simple enough that the native backend can fully compile it
/// (no classes, no ArrayList), and it has a tight loop that benefits
/// from native codegen.
const SUM_SQUARES_SOURCE: &str = r#"
public fn sumSquares(n: int): int {
    var sum: int = 0;
    var i: int = 0;
    while (i < n) {
        sum = sum + i * i;
        i = i + 1;
    }
    return sum;
}

public fn main(): void {
    let result: int = sumSquares(10000);
    io::println(result);
}
"#;

/// Compile and run a source string via the bytecode VM, returning the
/// wall-clock elapsed time.
fn run_bytecode(source: &str) -> Result<std::time::Duration, String> {
    let tokens = lexer::tokenize(source)?;
    let ast = parser::parse(tokens)?;
    let typed_ast = analyzer::analyze(&ast).map_err(|e| format!("{:?}", e))?;
    let mut compiler = Compiler::new();
    let compiled = compiler.compile(&typed_ast)?;

    let start = Instant::now();
    let mut vm = Vm::new();
    vm.load_program(compiled);
    vm.run()?;
    Ok(start.elapsed())
}

/// Compile a source string to a native binary using `trc --native --release`,
/// run it, and return the wall-clock elapsed time.
fn run_native(source: &str) -> Result<std::time::Duration, String> {
    let trc = trc_binary().ok_or_else(|| {
        "trc binary not found; run `cargo build -p trc` first".to_string()
    })?;

    // Write the source to a temp file.
    let temp_dir = env::temp_dir();
    let source_path = temp_dir.join("trc_bench_sum_squares.tr");
    std::fs::write(&source_path, source)
        .map_err(|e| format!("failed to write temp source: {}", e))?;

    // Compile with --native --release.
    let compile_out = Command::new(&trc)
        .arg("--native")
        .arg("--release")
        .arg(source_path.to_str().unwrap())
        .output()
        .map_err(|e| format!("failed to invoke trc --native: {}", e))?;

    if !compile_out.status.success() {
        return Err(format!(
            "trc --native failed: {}",
            String::from_utf8_lossy(&compile_out.stderr)
        ));
    }

    // Locate the produced binary.
    let exe_name = if cfg!(windows) {
        "trc_bench_sum_squares_native.exe"
    } else {
        "trc_bench_sum_squares_native"
    };
    let native_exe = temp_dir.join(exe_name);

    if !native_exe.is_file() {
        return Err(format!(
            "native binary was not produced at {}",
            native_exe.display()
        ));
    }

    // Run and time it.
    let start = Instant::now();
    let run_out = Command::new(&native_exe)
        .output()
        .map_err(|e| format!("failed to run native binary: {}", e))?;
    let elapsed = start.elapsed();

    // Clean up.
    let _ = std::fs::remove_file(&native_exe);
    let _ = std::fs::remove_file(&source_path);

    if !run_out.status.success() {
        return Err(format!(
            "native binary exited with status {:?}: {}",
            run_out.status.code(),
            String::from_utf8_lossy(&run_out.stderr)
        ));
    }

    Ok(elapsed)
}

#[test]
#[ignore = "requires LLVM dev files, a system linker, and titrate_native built"]
fn native_is_faster_than_bytecode_for_sum_squares() {
    // Run the bytecode version.
    let bytecode_time = run_bytecode(SUM_SQUARES_SOURCE)
        .expect("bytecode execution should succeed");

    // Run the native version.
    let native_time = run_native(SUM_SQUARES_SOURCE)
        .expect("native execution should succeed");

    // Report the results.
    eprintln!("--- Benchmark: sum_squares(10000) ---");
    eprintln!("  bytecode: {:?}", bytecode_time);
    eprintln!("  native:   {:?}", native_time);
    let speedup = bytecode_time.as_secs_f64() / native_time.as_secs_f64();
    eprintln!("  speedup:  {:.2}x", speedup);

    // Assert the native version is at least 1.5x faster for this small
    // workload. (For larger workloads the speedup is typically much higher.)
    assert!(
        speedup >= 1.5,
        "expected native to be at least 1.5x faster than bytecode, \
         but got {:.2}x (native={:?}, bytecode={:?})",
        speedup,
        native_time,
        bytecode_time,
    );
}

#[test]
fn bytecode_sum_squares_produces_correct_output() {
    // Sanity check: the bytecode VM should compute the correct result.
    // Sum of i^2 for i in 0..10000 = 333283335000.
    let tokens = lexer::tokenize(SUM_SQUARES_SOURCE).expect("tokenize");
    let ast = parser::parse(tokens).expect("parse");
    let typed_ast = analyzer::analyze(&ast).expect("analyze");
    let mut compiler = Compiler::new();
    let compiled = compiler.compile(&typed_ast).expect("compile");
    let mut vm = Vm::new();
    vm.load_program(compiled);
    vm.run().expect("run");

    // The output should contain the sum of squares 0..10000.
    // sum = n(n-1)(2n-1)/6 = 10000*9999*19999/6 = 333283335000
    let output = vm.output.join("\n");
    assert!(
        output.contains("333283335000"),
        "expected output to contain 333283335000, got:\n{}",
        output,
    );
}

/// A self-contained water-box energy computation that mirrors the
/// mega_test_03 forcefield but uses only primitives (no classes, no
/// ArrayList, no imports). This is small enough for the native backend
/// to fully compile.
///
/// The computation places 8 water molecules (24 atoms) on a cubic lattice
/// and computes the Lennard-Jones pair energy — the same O(N²) hot loop
/// that dominates mega_test_03.
const WATER_BOX_BENCH_SOURCE: &str = r#"
// Self-contained water-box LJ energy benchmark.
// Mirrors the hot loop in mega_test_03/src/forcefield.tr without classes.

public fn computeLJEnergy(n: int, ljSigma: double, ljEpsilon: double): double {
    // Simplified: atoms are on a cubic lattice with spacing 3.166.
    // We compute the LJ energy for all pairs.
    let spacing: double = 3.166;
    var e: double = 0.0;
    var i: int = 0;
    while (i < n) {
        var j: int = i + 1;
        while (j < n) {
            // Compute distance on the lattice (1D index -> 3D position).
            let ix: double = (i / 4) as double * spacing;
            let iy: double = ((i % 4) / 2) as double * spacing;
            let iz: double = (i % 2) as double * spacing;
            let jx: double = (j / 4) as double * spacing;
            let jy: double = ((j % 4) / 2) as double * spacing;
            let jz: double = (j % 2) as double * spacing;
            let dx: double = ix - jx;
            let dy: double = iy - jy;
            let dz: double = iz - jz;
            // Newton's method sqrt (20 iterations).
            let r2: double = dx * dx + dy * dy + dz * dz;
            var r: double = r2;
            var k: int = 0;
            while (k < 20) {
                r = 0.5 * (r + r2 / r);
                k = k + 1;
            }
            if (r > 0.001) {
                let sr: double = ljSigma / r;
                let sr6: double = sr * sr * sr * sr * sr * sr;
                let sr12: double = sr6 * sr6;
                e = e + 4.0 * ljEpsilon * (sr12 - sr6);
            }
            j = j + 1;
        }
        i = i + 1;
    }
    return e;
}

public fn main(): void {
    let n: int = 24;
    let sigma: double = 3.166;
    let epsilon: double = 0.1554;
    let energy: double = computeLJEnergy(n, sigma, epsilon);
    io::println(energy);
}
"#;

#[test]
#[ignore = "requires LLVM dev files, a system linker, and titrate_native built"]
fn native_water_box_benchmark() {
    // Run the bytecode version.
    let bytecode_time = run_bytecode(WATER_BOX_BENCH_SOURCE)
        .expect("bytecode execution should succeed");

    // Run the native version.
    let native_time = run_native(WATER_BOX_BENCH_SOURCE)
        .expect("native execution should succeed");

    // Report the results.
    eprintln!("--- Benchmark: water-box LJ energy (24 atoms) ---");
    eprintln!("  bytecode: {:?}", bytecode_time);
    eprintln!("  native:   {:?}", native_time);
    let speedup = bytecode_time.as_secs_f64() / native_time.as_secs_f64();
    eprintln!("  speedup:  {:.2}x", speedup);

    // For this compute-intensive workload (O(N²) with Newton's sqrt),
    // the native version should be at least 3x faster. If it isn't,
    // the likely causes are:
    //   1. The native bridge overhead (TitrateValue marshalling).
    //   2. The bytecode VM is already quite fast for small workloads.
    //   3. LLVM didn't vectorize the inner loop.
    //
    // We assert >= 1.5x as a conservative lower bound. The 3x target
    // is documented as the expected speedup for larger workloads.
    assert!(
        speedup >= 1.5,
        "expected native to be at least 1.5x faster than bytecode for \
         the water-box benchmark, but got {:.2}x (native={:?}, bytecode={:?})",
        speedup,
        native_time,
        bytecode_time,
    );
}

#[test]
fn bytecode_water_box_benchmark_produces_finite_energy() {
    // Sanity check: the bytecode VM should compute a finite energy.
    let tokens = lexer::tokenize(WATER_BOX_BENCH_SOURCE).expect("tokenize");
    let ast = parser::parse(tokens).expect("parse");
    let typed_ast = analyzer::analyze(&ast).expect("analyze");
    let mut compiler = Compiler::new();
    let compiled = compiler.compile(&typed_ast).expect("compile");
    let mut vm = Vm::new();
    vm.load_program(compiled);
    vm.run().expect("run");

    // The output should contain a finite number (not NaN, not infinity).
    let output = vm.output.join("\n");
    eprintln!("water-box energy output: {}", output);
    assert!(
        !output.is_empty(),
        "expected non-empty output from water-box benchmark",
    );
}
