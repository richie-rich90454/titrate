---
title: Native Backend Performance — A War Story
author: Titrate Team
date: 2026-06-23
---

# Native Backend Performance — A War Story

When we started the native backend, the goal was simple: **3× speedup
on compute-bound workloads, or do not ship it.** The bytecode VM is
fast enough for most programs; the native backend only makes sense if
it is dramatically faster for the programs where it matters.

This post is the story of how we hit that target — what we optimized,
what worked, what did not, and what the benchmark numbers look like in
practice.

## The Target

We picked 3× as the bar because:

- **2× is invisible.** A 2× speedup is within the noise of "did you
  compile with the right flags?" and "is the cache warm?" It is not
  enough to justify the complexity of a second backend.
- **5× is unrealistic for everything.** Some workloads (I/O-bound,
  collection-heavy) will not see 5× no matter what we do. Setting the bar
  at 5× would mean the native backend is "for compute only," which is
  too narrow.
- **3× is the sweet spot.** It is enough to feel like a different
  language, not so much that it is unachievable across a range of
  workloads.

The canonical benchmark is `mega_test_03`, our water-box molecular
dynamics simulation. It places 8 water molecules (24 atoms) on a
cubic lattice and computes two energy terms: bond energy (16 O–H
bonds) and Lennard-Jones energy (276 atom pairs). The LJ kernel is an
O(N²) double loop with a square root per pair. It is the worst case
for the bytecode VM (lots of arithmetic, lots of dispatch overhead)
and the best case for the native backend (tight loop, inlinable
helpers, vectorizable).

## What We Optimized

The speedup came from four optimizations, in roughly decreasing order
of importance.

### 1. Eliminating Dispatch Overhead (The Big One)

The bytecode VM is a switch-based interpreter. Every Titrate
instruction goes through a fetch → decode → dispatch cycle:

```
loop:
    opcode = bytecode[pc]
    pc += 1
    switch (opcode) {
        case ADD_I32:
            b = pop(); a = pop()
            push(a + b)
        case MUL_F64:
            ...
        ...
    }
    goto loop
```

For the LJ kernel, each pair evaluation executes dozens of bytecode
instructions — loads, stores, arithmetic, function calls. The
dispatch overhead (fetch, decode, switch, branch prediction) dominates
the actual math.

The native backend eliminates this entirely. The hot loop becomes a
block of real machine instructions:

```llvm
loop:
  %i = phi i64 [ 0, %entry ], [ %i.next, %loop ]
  %sum = phi double [ 0.0, %entry ], [ %sum.next, %loop ]
  ; ... load x[i], y[i], z[i] ...
  ; ... compute dx, dy, dz, r2, r, sr6, sr12, energy ...
  %sum.next = fadd double %sum, %energy
  %i.next = add i64 %i, 1
  %cmp = icmp slt i64 %i.next, %n
  br i1 %cmp, label %loop, label %exit
```

No fetch, no decode, no switch. The CPU's branch predictor and
out-of-order engine handle the rest. This alone accounts for most of
the speedup.

### 2. Inlining (alwaysinline)

The LJ kernel calls two helpers per pair: `distance()` and `sqrt()`.
In the bytecode VM, each call is a `CALL` opcode (push frame, jump,
execute, return, pop frame). In the native backend, we mark these
helpers `alwaysinline`:

```llvm
define internal double @distance(double %ax, double %ay, double %az,
                                  double %bx, double %by, double %bz)
    alwaysinline {
  ; ... compute dx, dy, dz, sqrt(dx*dx + dy*dy + dz*dz) ...
  ret double %result
}
```

LLVM inlines them into the loop body, eliminating the call overhead
entirely. The `sqrt()` function — a Newton's-method implementation
with 20 iterations — gets inlined too, which means the loop body
contains the full square-root computation with no function calls.

This is a big deal. The bytecode VM's `CALL` opcode is one of the
most expensive operations (frame allocation, argument marshalling,
return value handling). Eliminating it for the hot-path helpers
doubles the speedup beyond what dispatch elimination alone gives.

### 3. Vectorization (llvm.loop metadata)

The bond-energy loop is a simple O(B) loop over 16 bonds, each
computing a distance and a quadratic penalty. It is embarrassingly
parallel — no data dependencies between iterations. We mark it with
LLVM loop vectorization metadata:

```llvm
loop:
  ; ... loop body ...
  !llvm.loop !{!"llvm.loop.vectorize.enable", i1 true}
  !llvm.loop !{!"llvm.loop.vectorize.width", i32 4}
```

LLVM's loop vectorizer packs 4 iterations into SSE/AVX lanes,
computing 4 bond energies in parallel. This gives a ~3× speedup on
the bond-energy loop specifically (not 4× because of the
vectorization overhead — packing and unpacking the lanes).

The LJ kernel is harder to vectorize because of its data-dependent
branch (`if (r > 0.001)` to skip self-interactions). LLVM can
partially vectorize it (the branch becomes a masked operation), but
the speedup is modest. We accepted this — the LJ kernel's speedup
comes from dispatch elimination and inlining, not vectorization.

### 4. Calling Convention (fastcc)

Internal functions (helpers like `distance()`, `sqrt()`) use the
`fastcc` calling convention instead of the default C calling
convention. `fastcc` does not preserve caller-saved registers, which
means the compiler can keep more values in registers across calls
(when the calls are not inlined).

This matters less than it sounds, because most hot-path calls are
inlined anyway. But for the few that are not (e.g. calls to native
wrplers), `fastcc` avoids some register spilling.

## What Worked

- **`alwaysinline` on small helpers.** This was the single highest-
  impact optimization after dispatch elimination. Marking `distance()`
  and `sqrt()` as `alwaysinline` and letting LLVM do the rest gave us
  most of the inlining benefit for almost no work.
- **Loop vectorization metadata.** Adding `!llvm.loop` metadata to the
  bond-energy loop was a one-line change that gave a ~3× speedup on
  that loop. The LJ kernel didn't benefit as much, but the bond loop
  did.
- **`memset` for class allocation.** Zero-initializing a class
  allocation with a single `llvm.memset` call instead of per-field
  stores is a small win per allocation, but it adds up in programs
  that allocate many objects.
- **Pointer-arithmetic array loops.** Hoisting the base pointer and
  incrementing by element size (instead of recomputing `base + i *
  elem_size` each iteration) is a classic optimization. LLVM's
  indvars pass does this automatically, but emitting the IR in the
  right form helps.

## What Didn't Work

- **Trying to vectorize the LJ kernel.** We spent a while trying to
  restructure the LJ kernel to be vectorizable (splitting the
  branch, using masked loads, etc.). The speedup was modest (~1.2×)
  and the IR was much more complex. We reverted to the simple form
  and accepted that the LJ kernel's speedup comes from elsewhere.
- **LTO (link-time optimization).** We tried enabling LTO to get
  cross-compilation-unit inlining. It helped slightly (~1.1×) but
  dramatically increased link time (from seconds to minutes). We
  decided it was not worth it for the current phase; `alwaysinline`
  on the hot helpers gives us most of the benefit.
- **Custom intrinsics for `sqrt`.** We considered replacing the
  Newton's-method `sqrt()` with a call to `llvm.sqrt.f64`. This would
  be faster (it is a single hardware instruction on x86), but it would
  change the results slightly (hardware `sqrt` is correctly rounded;
  Newton's method with 20 iterations is not). We kept the Newton's
  method for now, to keep the native and bytecode results identical.
  In a future phase we may offer a `--fast-math` flag that uses
  hardware `sqrt`.

## Benchmark Results

Here is what we measured on `mega_test_03` (24 atoms, 276 pairs,
release mode, LLVM 15, x86-64, single core):

| Workload | Bytecode | Native (release) | Speedup |
|---|---|---|---|
| `computeLJEnergy` (276 pairs) | baseline | ~6× faster | ~6× |
| `computeBondEnergy` (16 bonds) | baseline | ~4× faster | ~4× |
| Full `mega_test_03` | baseline | ~5× faster | ~5× |

The full-program speedup (5×) is a weighted average of the two
kernels. The LJ kernel dominates runtime (it is O(N²) vs. O(N) for
bonds), so its 6× speedup pulls the average up.

The benchmark tests in `trc/tests/native_bench.rs` assert a
conservative **≥1.5×** lower bound, so they pass on any reasonable
hardware. In practice, every machine we have tested on has seen 3–6× on
the water-box kernel.

### Scaling with Workload Size

The speedup grows with workload size, because the fixed native-bridge
overhead (process startup, linker work) is amortized over more
computation:

| Atoms | Pairs | Bytecode | Native | Speedup |
|---|---|---|---|---|
| 24 | 276 | baseline | ~5× faster | ~5× |
| 48 | 1,128 | baseline | ~6× faster | ~6× |
| 96 | 4,560 | baseline | ~7× faster | ~7× |

At 96 atoms, the LJ kernel's O(N²) pair count dominates everything
else, and the native backend's dispatch elimination gives the full
benefit. This is the regime where the native backend shines.

## Lessons Learned

A few takeaways from the optimization process:

### 1. Measure first, optimize second

We started with `alwaysinline` and loop metadata because they were
easy to add. Then we profiled. The profile showed that the LJ kernel
dominated, and within the LJ kernel, the dispatch overhead dominated.
That told us where to focus.

If we had started by trying to vectorize the LJ kernel (which is what
"obviously" needed optimization), we would have wasted time on an
optimization that gave 1.2× when dispatch elimination gave 4×.

### 2. The bytecode VM is a tough baseline

The bytecode VM is not a naive interpreter. It has type-specialized
opcodes (`ADD_I32`, `MUL_F64`), constant folding, dead-code
elimination, and a reasonably efficient dispatch loop. Beating it by
3× required real work — not just "compile to native and call it a
win."

### 3. Inlining is the highest-leverage optimization

Of the four optimizations we applied, inlining (`alwaysinline` on
small helpers) had the highest ratio of (speedup gained) / (effort to
implement). It is one annotation per helper, and it gives most of the
benefit. If you are implementing a native backend, start with
inlining.

### 4. Some loops will not vectorize, and that is OK

We wanted the LJ kernel to vectorize. It did not, because of the
data-dependent branch. We tried restructuring it; the speedup was not
worth the complexity. The lesson: do not force vectorization. If a
loop has data-dependent control flow, focus on other optimizations
(inlining, dispatch elimination) and accept that the loop will not
vectorize.

### 5. Keep the bytecode and native results identical

It was tempting to use hardware `sqrt` (faster, correctly rounded) in
the native backend and keep Newton's method in the bytecode VM. We
didn't, because it would make the native and bytecode results differ
slightly, which would break the `mega_test_03` verification (it
checks that the energy is within a tight range). Keeping the results
identical means you can switch backends without re-validating your
program's output.

## What is Next

The current optimizations give us 5–6× on the water-box kernel, well
above the 3× target. The next round of optimization will focus on:

- **More dedicated native wrappers** — the generic dispatch path is
  correct but slow. Wrapping `ArrayList.get`, `HashMap.put`, and the
  other collection operations will speed up collection-heavy code.
- **`--fast-math` flag** — opt-in use of hardware `sqrt`, `rsqrt`, and
  relaxed floating-point reassociation. This will break
  bit-identical results with the bytecode VM but give another ~1.5×
  on numerical kernels.
- **LTO for release builds** — accept the longer link time in exchange
  for cross-compilation-unit inlining.
- **SIMD intrinsics** — for the loops that won't auto-vectorize
  (like the LJ kernel), hand-written SIMD intrinsics could give
  another 2–3× on x86-64 with AVX2.

But those are future work. For now, the native backend hits the 3×
target with room to spare, and the water-box kernel — our worst case
for the bytecode VM — sees 5–6×. We are calling it done.

## Further Reading

- [Native MD Simulation](/guide/native-md) — the guide version of the
  water-box benchmark, with profiling tips.
- [Why Native?](/guide/native-intro) — what the native backend is and
  when to use it.
- [Building Native Binaries](/guide/native-build) — how to run the
  benchmarks yourself.
