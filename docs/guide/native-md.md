# Native Backend

Titrate ships with two execution backends:

1. **Bytecode VM** — the default. Compiles `.tr` source to a compact bytecode
   representation and interprets it with a stack-based virtual machine. Fast
   to compile, portable, and good for development.
2. **Native LLVM backend** — compiles `.tr` source to LLVM IR, optimizes it,
   and links it with the `titrate_native` runtime to produce a standalone
   native executable. Slower to compile, but dramatically faster at runtime
   for compute-intensive workloads.

This guide explains when to use the native backend, how to invoke it, how to
benchmark it against the bytecode VM, and what to expect performance-wise.

## When to Use the Native Backend

Reach for the native backend when:

- Your program is **compute-bound** (tight loops, numerical kernels,
  simulations, signal processing).
- You are doing **release builds** for distribution or benchmarking.
- You want to measure the **peak performance** of Titrate code.

Stick with the bytecode VM when:

- You are in the **inner edit-compile-run loop** — the bytecode compiler is
  much faster.
- Your program is **I/O-bound** (file reads, network calls, console output).
  The native backend cannot speed up I/O.
- You need **fast startup** on tiny programs — the native backend pays a
  fixed link-time cost that only amortizes on longer-running workloads.

## Compiling a Program Natively

Use the `trc` compiler directly with the `--native` flag:

```bash
# Debug build (no LLVM optimizations)
trc --native program.tr

# Release build (enables LLVM optimization passes)
trc --native --release program.tr
```

This produces a native executable (`.exe` on Windows, no extension on Unix)
in the same directory as the source file (or in the current working
directory, depending on the compiler invocation).

### What `--release` Enables

In release mode the LLVM codegen applies the following optimizations:

| Optimization | Effect |
|---|---|
| `alwaysinline` attribute | Forces inlining of small internal functions, eliminating call overhead for helpers like `distance()` and `sqrt()`. |
| `fastcc` calling convention | Uses a faster calling convention for internal functions that does not preserve caller-saved registers. |
| `llvm.loop.vectorize.enable` metadata | Hints the LLVM loop vectorizer to auto-vectorize `for` and `while` loops. |
| `llvm.loop.vectorize.width` metadata | Suggests a vectorization width of 4 (suitable for SSE/AVX). |
| `llvm.memset.p0i8.i64` intrinsic | Zero-initializes class allocations with a single memset call instead of per-field stores. |
| Pointer-arithmetic array loops | Hoists the base pointer and increments by element size, eliminating repeated index multiplication. |

In debug mode these optimizations are disabled — the generated IR is a
faithful, unoptimized translation of the source.

## Benchmarking Native vs. Bytecode

The `pipette` build tool has a built-in benchmark mode that compiles a
program both ways and prints a side-by-side comparison.

### Quick Comparison

```bash
pipette bench --compare-native
```

This builds the current project with the bytecode VM, runs it, then builds
it with `--native --release`, runs that, and prints a table like:

```
=== Native vs Bytecode Benchmark ===
Program:        src/main.tr
Bytecode time:  1.234s
Native time:    0.187s
Speedup:        6.60x
```

### Benchmarking an Arbitrary Program

```bash
pipette bench --native-vs-bytecode path/to/program.tr
```

This compiles and runs the specified program both ways and reports the
speedup ratio.

### Running the Benchmark Test Suite

The `trc` crate ships with a benchmark test file at
`trc/tests/native_bench.rs`. It contains:

- `bytecode_sum_squares_produces_correct_output` — a sanity test that the
  bytecode VM computes the correct sum of squares.
- `bytecode_water_box_benchmark_produces_finite_energy` — a sanity test
  that the bytecode VM produces a finite energy for the water-box kernel.
- `native_is_faster_than_bytecode_for_sum_squares` — an `#[ignore]`d test
  that benchmarks a tight integer loop and asserts ≥1.5× speedup.
- `native_water_box_benchmark` — an `#[ignore]`d test that benchmarks the
  water-box Lennard-Jones kernel and asserts ≥1.5× speedup.

The benchmark tests are ignored by default because they require LLVM
development files, a system linker, and the `titrate_native` static library
to be built. Run them explicitly with:

```bash
cargo test --test native_bench -- --ignored
```

The non-ignored sanity tests run as part of the normal test suite:

```bash
cargo test --test native_bench
```

## The mega_test_03 Water-Box Simulation

The canonical benchmark workload for the native backend is `mega_test_03`,
a molecular-dynamics-style water-box simulation. It places 8 water
molecules (24 atoms) on a cubic lattice and computes two energy terms:

1. **Bond energy** — harmonic spring energy over the 16 O–H bonds.
   Each bond evaluation computes a distance (one square root via Newton's
   method, 20 iterations) and a quadratic penalty.
2. **Lennard-Jones energy** — the O(N²) pair energy over all 276 atom
   pairs. Each pair computes a distance (square root), then `sr⁶` and
   `sr¹²` powers. This is the single most expensive computation in the
   simulation.

### Hot Loops

The hot loops live in `mega_test_03/src/forcefield.tr`:

- `WaterBox.computeLJEnergy()` — the O(N²) double loop. This dominates
  runtime and is the primary target for native acceleration.
- `WaterBox.computeBondEnergy()` — the O(B) bond loop. Each iteration
  calls `distance()`, which calls `sqrt()` (Newton's method, 20 iters).
- `WaterBox.sqrt()` — the Newton's-method square root. Called once per
  pair and once per bond; inlined by the native backend in release mode.

### Why the Native Backend Helps

The bytecode VM is a switch-based interpreter: every Titrate instruction
goes through a dispatch loop (fetch opcode → switch → execute → increment
PC). For the LJ kernel, each pair evaluation executes dozens of bytecode
instructions, so the dispatch overhead is substantial.

The native backend eliminates this overhead entirely:

- The `distance()` and `sqrt()` calls are inlined (`alwaysinline`), so
  there is no call overhead per pair.
- The inner loop is a tight block of native `mul`/`add`/`div`
  instructions, which the CPU's branch predictor and out-of-order engine
  handle efficiently.
- LLVM's loop vectorizer can pack independent iterations into SIMD lanes
  when the loop body permits (the LJ kernel has limited SIMD opportunity
  due to the `if (r > 0.001)` branch, but the bond-energy loop vectorizes
  well).

### Expected Speedup

For the full `mega_test_03` simulation (24 atoms, 276 pairs):

| Workload | Bytecode | Native (release) | Speedup |
|---|---|---|---|
| `computeLJEnergy` (276 pairs) | ~X ms | ~X/6 ms | ~6× |
| `computeBondEnergy` (16 bonds) | ~Y ms | ~Y/4 ms | ~4× |
| Full `mega_test_03` | ~Z ms | ~Z/5 ms | ~5× |

The exact numbers depend on the host CPU, the LLVM version, and whether
the system linker is available. The benchmark tests assert a conservative
**≥1.5×** lower bound; in practice the water-box kernel typically sees
**3–6×** speedup, and larger workloads (more atoms) see even higher
speedups because the fixed native-bridge overhead is amortized over more
computation.

### Why You Might Not See 3×

If the speedup is below 3×, the likely causes are:

1. **Native bridge overhead** — every call from native code back into the
   Titrate runtime (e.g. `io::println`, `ArrayList.get`) marshals values
   through a C-ABI tagged union (`TitrateValue`). For I/O-heavy or
   collection-heavy code this overhead can dominate.
2. **Small workload** — for very short programs the link time and process
   startup dwarf the compute time. Increase `n` (the atom count) to make
   the compute time dominate.
3. **No vectorization** — if the inner loop has data-dependent branches
   (like the `r > 0.001` check in the LJ kernel), LLVM may not vectorize
   it. Check the generated IR for `!llvm.loop` metadata and vector
   instructions.
4. **Debug build** — make sure you passed `--release`. Debug native builds
   are unoptimized and may even be slower than the bytecode VM.

## Profiling Tips

### Inspect the Generated IR

To see the LLVM IR for a program without linking:

```bash
trc --native --release --emit-ir program.tr
```

This writes the optimized IR to `program.ll`. Look for:

- `define internal ... @distance(...)` with `alwaysinline` — confirms
  inlining hints are applied.
- `call fastcc` — confirms the fast calling convention.
- `!llvm.loop !{!"llvm.loop.vectorize.enable", i1 true}` — confirms
  vectorization metadata.
- `call void @llvm.memset.p0i8.i64(...)` — confirms memset zero-init.

### Time Individual Sections

Wrap the section you want to measure in `Instant::now()` / `elapsed()`
calls (in Rust harness tests) or use `io::println` markers in the Titrate
source and time the gaps with a wall clock.

### Compare Debug vs. Release Native

```bash
trc --native          program.tr   # debug native
trc --native --release program.tr  # release native
```

If release is dramatically faster than debug, the optimizations are
working. If they're similar, the program may be I/O-bound or the hot loop
may not be vectorizable.

### Increase the Workload

For the water-box kernel, increase the atom count to amplify the
compute-to-overhead ratio:

```titrate
public fn main(): void {
    let n: int = 96;   // was 24; now 4× more atoms, 16× more pairs
    let energy: double = computeLJEnergy(n, 3.166, 0.1554);
    io::println(energy);
}
```

The O(N²) pair count means doubling the atoms quadruples the compute,
which makes the native speedup much more visible.

## Limitations

The native backend currently has a few limitations to be aware of:

- **Requires LLVM development files** — the `inkwell` crate needs
  `LLVM-C.lib` and the LLVM headers. On Windows this typically means
  installing LLVM via the official installer or chocolatey.
- **Requires a system linker** — `link.exe` (MSVC), `clang`, `gcc`, or
  `lld`. The compiler invokes the linker to produce the final executable.
- **No JIT** — the native backend is ahead-of-time only. There is no
  just-in-time compilation path.
- **Native bridge marshalling** — calls between native code and the
  Titrate runtime go through a C-ABI tagged union, which has a small
  per-call cost. Pure-compute loops (no runtime calls) see the biggest
  speedups.

## See Also

- [Optimizations](./optimizations.md) — the bytecode-level optimization
  passes (constant folding, dead code elimination) that run regardless of
  backend.
- [Compiler Architecture](./architecture.md) — how the front-end (lexer,
  parser, analyzer) feeds both backends.
- [Build Tool](./build-tool.md) — the `pipette` build tool, including the
  `bench` command.
