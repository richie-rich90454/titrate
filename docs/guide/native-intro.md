# Why Native?

Titrate has always shipped with a fast bytecode VM — but starting with the
LLVM native backend, you can now compile `.tr` programs all the way to
standalone native executables. This guide explains what the native backend
is, why you might want it, and how it compares to the bytecode VM.

## What Is the Native Backend?

The native backend is an alternative compiler pipeline. Instead of
translating your source to bytecode and interpreting it on Titrate's
stack-based VM, it lowers the analyzed AST to [LLVM IR][llvm-ir], runs
LLVM's optimizer, and links the result against the `titrate_native`
runtime to produce a real machine-code executable.

```
                  ┌──────────────────────────┐
   Source (.tr) ─▶│  Lexer / Parser / Analyzer │
                  └────────────┬─────────────┘
                               │
                  ┌────────────┴─────────────┐
                  ▼                            ▼
        ┌──────────────────┐        ┌──────────────────────┐
        │  Bytecode Compiler│        │  LLVM Codegen         │
        └────────┬─────────┘        │  (AST → LLVM IR)       │
                 │                   └────────┬──────────────┘
                 ▼                            ▼
        ┌──────────────────┐        ┌──────────────────────┐
        │   Titrate VM      │        │  LLVM Optimizer + Linker│
        │  (interpreter)    │        │  → standalone .exe     │
        └──────────────────┘        └──────────────────────┘
```

Both backends share the same front-end (lexer, parser, analyzer) and the
same standard library. You can switch between them with a single flag.

[llvm-ir]: https://llvm.org/docs/LangRef.html

## Why Compile to Native?

Three reasons, in roughly increasing order of importance.

### 1. Performance

The bytecode VM is a switch-based interpreter: every Titrate instruction
goes through a fetch → decode → dispatch cycle. For I/O-bound code that
cost is invisible — you're waiting on the kernel anyway. But for tight
compute loops (numerical kernels, simulations, signal processing), the
dispatch overhead dominates.

The native backend eliminates it. The hot loop becomes a block of real
machine instructions that the CPU's branch predictor, out-of-order
engine, and SIMD units can attack directly. LLVM's optimizer also
applies inlining, vectorization, and constant folding across function
boundaries — something the bytecode VM cannot do.

For compute-bound workloads the native backend is typically **3–6×
faster** than the bytecode VM, and the speedup grows with the workload
size. See [Native MD Simulation](./native-md) for benchmark numbers on
the canonical water-box kernel.

### 2. Standalone Binaries

A bytecode program needs the Titrate runtime installed to run. A native
executable is a single self-contained file: you can ship it to a user,
a CI runner, or a benchmark harness without explaining how to install
Titrate first.

This matters for:

- **Distribution** — hand a customer a binary, not a runtime.
- **Benchmarking** — your numbers won't be skewed by a different VM
  version on the benchmarking machine.
- **Embedding** — call the binary from a shell script, a Makefile, or
  another language's FFI without worrying about runtime setup.

### 3. Predictable Performance

The bytecode VM has predictable *per-instruction* cost but unpredictable
*overall* cost — a single Titrate expression can compile to many opcodes,
and the dispatch overhead varies with CPU branch-prediction state. The
native backend gives you the predictable cost model of compiled code:
instructions execute in roughly the cycles the hardware manual says they
will, and the optimizer's transformations are deterministic given a
fixed LLVM version.

## Bytecode VM vs. Native

| Aspect | Bytecode VM | Native Backend |
|---|---|---|
| Compilation time | Fast (milliseconds) | Slower (LLVM optimization + link) |
| Runtime performance | Good for I/O-bound | 3–6× faster for compute-bound |
| Startup time | Instant | Pays a fixed link-time cost |
| Distribution | Needs Titrate installed | Single standalone executable |
| Debugging | Bytecode-level traces | LLVM IR inspection, GDB/LLDB |
| Requires LLVM | No | Yes (LLVM 15+ dev files) |
| Requires linker | No | Yes (`link.exe`, `clang`, `gcc`, or `lld`) |
| Optimization | Constant folding, DCE | Full LLVM O3 pipeline |

## When to Use Each

::: tip Use the Bytecode VM when…
- You're in the inner edit-compile-run loop and want fast feedback.
- Your program is I/O-bound (file reads, network calls, console output).
- You're prototyping a new feature and want quick iteration.
- You're running tiny programs where startup time matters.
:::

::: tip Use the Native Backend when…
- Your program is compute-bound (tight loops, numerical kernels,
  simulations, signal processing).
- You're building a release artifact for distribution or benchmarking.
- You want to measure the peak performance of Titrate code.
- You need a standalone executable that doesn't require the Titrate
  runtime to be installed.
:::

A common workflow is to develop with the bytecode VM and switch to the
native backend for release builds and performance work. The
`pipette bench --compare-native` command makes side-by-side comparison
trivial.

## Quick Start

Compile and run a program natively:

```bash
# Debug build (no LLVM optimizations)
trc --native hello.tr
./hello            # or hello.exe on Windows

# Release build (enables LLVM O3-class optimizations)
trc --native --release hello.tr
./hello
```

A minimal program:

```titrate
public fn main(): void {
    io::println("Hello from native Titrate!");
}
```

That's it. The compiler handles the rest — invoking LLVM, linking
against `titrate_native`, and producing the final executable.

For a deeper dive on build flags, prerequisites, and troubleshooting,
see [Building Native Binaries](./native-build). For what's happening
under the hood when you use `Owned<T>`, `&T`, or `region` blocks, see
[Ownership on LLVM](./native-ownership). For wrapping C libraries, see
[Wrapping C Libraries](./native-cbind).

## See Also

- [Building Native Binaries](./native-build) — prerequisites, flags,
  and troubleshooting.
- [Ownership on LLVM](./native-ownership) — how `Owned<T>`, borrows,
  regions, and `unsafe` lower to LLVM IR.
- [Wrapping C Libraries](./native-cbind) — the native bridge and how to
  extend it.
- [Native MD Simulation](./native-md) — benchmarks and profiling tips
  for the canonical water-box kernel.
- [Compiler Architecture](./architecture) — how the front-end feeds both
  backends.
