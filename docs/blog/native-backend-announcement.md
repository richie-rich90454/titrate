---
title: Announcing the LLVM Native Backend
author: Titrate Team
date: 2026-06-23
---

# Announcing the LLVM Native Backend

Today we're shipping the LLVM native backend for Titrate. After four
phases of work — codegen, ownership lowering, native bridge, and
optimization — you can now compile `.tr` programs all the way to
standalone native executables. For compute-bound workloads, the
results are dramatic: **3–6× faster** than the bytecode VM, with a
single self-contained binary as the output.

This post covers what we built, why we built it, how it works at a
high level, and what's coming next.

## What We Built

Titrate has always had two ways to run code: a tree-walking
interpreter (for prototyping) and a bytecode VM (for real work). Both
share the same front-end — lexer, parser, analyzer — and the same
standard library. The native backend is a **third** execution path,
plugged in at the same point as the bytecode compiler:

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
        └──────────────────┘        │  → standalone .exe     │
                                    └──────────────────────┘
```

You opt in with a single flag:

```bash
trc --native --release program.tr
```

The compiler lowers the analyzed AST to LLVM IR, runs LLVM's
optimization pipeline, links against the `titrate_native` runtime, and
produces a native executable. No JIT, no runtime dependency — just a
binary you can ship.

## Why We Built It

The bytecode VM is fast enough for most programs. It's a switch-based
interpreter with type-specialized opcodes (`ADD_I32`, `MUL_F64`,
`EQ_STRING`), and for I/O-bound code the dispatch overhead is
invisible. But for compute-bound code — tight numerical loops,
simulations, signal processing — the dispatch overhead dominates.

The canonical example is `mega_test_03`, our water-box molecular
dynamics simulation. It places 8 water molecules (24 atoms) on a cubic
lattice and computes two energy terms: bond energy (16 O–H bonds) and
Lennard-Jones energy (276 atom pairs). The LJ kernel is an O(N²) double
loop with a square root per pair. On the bytecode VM, each pair
evaluation executes dozens of opcodes — fetch, decode, dispatch,
fetch, decode, dispatch — and the actual math is a tiny fraction of
the wall-clock time.

The native backend eliminates the dispatch overhead. The hot loop
becomes a block of real machine instructions. The `distance()` and
`sqrt()` helpers get inlined. LLVM's loop vectorizer packs independent
iterations into SIMD lanes. The result is a **5–6× speedup** on the
full water-box simulation, and the speedup grows with the workload
size.

## Performance Highlights

Here's what we measured on `mega_test_03` (24 atoms, 276 pairs,
release mode, LLVM 15, x86-64):

| Workload | Bytecode | Native (release) | Speedup |
|---|---|---|---|
| `computeLJEnergy` (276 pairs) | baseline | ~6× faster | ~6× |
| `computeBondEnergy` (16 bonds) | baseline | ~4× faster | ~4× |
| Full `mega_test_03` | baseline | ~5× faster | ~5× |

The benchmark tests in `trc/tests/native_bench.rs` assert a
conservative **≥1.5×** lower bound (so they pass on any reasonable
hardware), but in practice the water-box kernel typically sees 3–6×,
and larger workloads see even higher speedups because the fixed
native-bridge overhead is amortized over more computation.

The speedup comes from four places, in roughly decreasing order of
importance:

1. **Eliminating dispatch overhead** — no fetch/decode/dispatch cycle
   per operation. This is the big one.
2. **Inlining** — `alwaysinline` on small helpers like `distance()`
   and `sqrt()` eliminates call overhead.
3. **Vectorization** — LLVM's loop vectorizer packs independent
   iterations into SIMD lanes (the bond-energy loop vectorizes well;
   the LJ kernel has limited SIMD opportunity due to its
   data-dependent branch).
4. **Calling convention** — `fastcc` for internal functions avoids
   caller-saved register pressure.

## How It Works (High-Level)

The native backend has four major pieces, each corresponding to a
phase of development:

### Phase 0–1: Codegen and Ownership

The core codegen (`trc/src/codegen/llvm/`) walks the analyzed AST and
emits LLVM IR. It handles classes, generics (via monomorphization),
enums, tuples, operator overloading, and all the control flow
constructs. The ownership lowering (`ownership.rs`) translates
`Owned<T>`, borrows, regions, and `unsafe` blocks to LLVM constructs:
drop flags for `Owned<T>`, raw pointers for borrows, `alloca` + lifetime
intrinsics for regions.

### Phase 2: Native Bridge

The native bridge (`native_bridge.rs` + `titrate_native` crate) lets
native code call the same 353 native functions the bytecode VM uses.
The bridge uses a C-ABI tagged union (`TitrateValue`) to marshal
values between LLVM IR and the Rust runtime. Direct helpers
(`titrate_println`, `titrate_string_concat`, `titrate_malloc`,
`titrate_free`) bypass the generic bridge for hot paths.

### Phase 3: Optimization

Release mode (`--release`) turns on LLVM optimization hints:
`alwaysinline`, `fastcc`, loop vectorization metadata, memset
zero-init for class allocations, and pointer-arithmetic array loops.
These are the optimizations that produce the 5–6× speedup.

### Phase 4: Benchmarking

The `pipette bench --compare-native` command compiles a program both
ways and prints a side-by-side comparison. The `native_bench.rs` test
file has sanity tests (always run) and benchmark tests (ignored by
default, run with `--ignored`).

## What's Next

The native backend is functional and fast, but there's more to do:

- **More dedicated wrappers** — the generic dispatch path
  (`titrate_native_call`) is correct for all 353 functions but slower
  than a dedicated wrapper. We have wrappers for the hot-path math and
  string functions; we need them for collection operations
  (`ArrayList.get`, `HashMap.put`) and I/O.
- **Per-type drop glue** — currently `Owned<T>` is always a heap
  pointer freed with `titrate_free`. Rust-style `impl Drop for T` would
  let types run custom cleanup logic.
- **Link-time optimization (LTO)** — we currently rely on
  `alwaysinline` for cross-function inlining. Full LTO would let LLVM
  inline across compilation units.
- **More platforms** — we test on x86-64 Linux and Windows. ARM64
  (Apple Silicon, Graviton) should work but isn't tested.
- **JIT mode** — the native backend is ahead-of-time only. A JIT path
  would give us the best of both worlds: fast compilation for
  development, native speed for hot code.

In the meantime, try it out:

```bash
trc --native --release your_program.tr
```

If you hit a bug, file an issue. If you see a speedup that surprises
you (in either direction), we'd love to hear about it.

## Further Reading

- [Why Native?](/guide/native-intro) — introduction and when to use
  the native backend.
- [Building Native Binaries](/guide/native-build) — prerequisites,
  flags, and troubleshooting.
- [Ownership on LLVM](/guide/native-ownership) — how `Owned<T>`,
  borrows, and regions lower to LLVM IR.
- [Wrapping C Libraries](/guide/native-cbind) — the native bridge and
  how to extend it.
- [Native MD Simulation](/guide/native-md) — benchmarks and profiling
  tips for the water-box kernel.
