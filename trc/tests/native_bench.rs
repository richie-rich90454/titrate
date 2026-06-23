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
