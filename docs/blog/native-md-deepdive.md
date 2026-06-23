---
title: Native MD Simulation — A Deep Dive
author: Titrate Team
date: 2026-06-23
---

# Native MD Simulation — A Deep Dive

The canonical benchmark for the Titrate native backend is
`mega_test_03`, a molecular-dynamics-style water-box simulation. It's
the workload we use to measure native-vs-bytecode speedup, to profile
the native backend, and to verify that the native backend produces the
same results as the bytecode VM.

This post is a deep dive on the simulation itself: what it computes,
how it compiles to native, where the hot loops are, and what the
performance characteristics look like.

## The Simulation

`mega_test_03` simulates a box of water molecules. It places 8 water
molecules (24 atoms — 8 oxygen, 16 hydrogen) on a simple cubic
lattice with a spacing of 3.166 Å, then computes two energy terms:

1. **Bond energy** — harmonic spring energy over the 16 O–H bonds.
   Each bond evaluation computes a distance (one square root via
   Newton's method, 20 iterations) and a quadratic penalty:
   `E = 0.5 * k * (r - r0)^2`.
2. **Lennard-Jones energy** — the O(N²) pair energy over all 276 atom
   pairs. Each pair computes a distance (square root), then `sr⁶` and
   `sr¹²` powers: `E = 4ε[(σ/r)¹² - (σ/r)⁶]`.

The LJ kernel is the single most expensive computation in the
simulation. It's O(N²) in the atom count, and each pair evaluation
involves a square root and several multiplications. For 24 atoms,
that's 276 pairs; for 96 atoms, it's 4,560 pairs.

The simulation verifies three things:

- The atom count is 24 (8 waters × 3 atoms).
- The bond count is 16 (8 waters × 2 bonds).
- The total energy is finite (within ±10¹⁰).

These checks are simple but they catch the most common bugs: wrong
atom placement, wrong bond topology, NaN/Inf in the energy
computation.

## The Source

The simulation lives in `mega_test_03/`:

```
mega_test_03/
├── Titrate.toml
├── expected_output.txt
└── src/
    ├── main.tr         — entry point, system setup, verification
    └── forcefield.tr   — WaterBox class, energy computation
```

`main.tr` builds the water box (nested `while` loops over the x, y, z
lattice indices), calls `computeBondEnergy()` and `computeLJEnergy()`,
and prints the results. `forcefield.tr` contains the `WaterBox` class
with the energy kernels.

### The LJ Kernel

The hot loop is `WaterBox.computeLJEnergy()`. Simplified:

```titrate
public fn computeLJEnergy(): double {
    var energy: double = 0.0;
    var i: int = 0;
    while (i < this.nAtoms) {
        var j: int = i + 1;
        while (j < this.nAtoms) {
            let dx: double = this.x[i] - this.x[j];
            let dy: double = this.y[i] - this.y[j];
            let dz: double = this.z[i] - this.z[j];
            let r2: double = dx*dx + dy*dy + dz*dz;
            let r: double = this.sqrt(r2);
            if (r > 0.001) {
                let sr: double = 3.166 / r;
                let sr6: double = sr * sr * sr * sr * sr * sr;
                let sr12: double = sr6 * sr6;
                energy = energy + 4.0 * 0.1554 * (sr12 - sr6);
            }
            j = j + 1;
        }
        i = i + 1;
    }
    return energy;
}
```

The `this.sqrt(r2)` call is a Newton's-method square root (20
iterations), not a hardware `sqrt`. This is deliberate — it keeps the
native and bytecode results bit-identical, which makes verification
trivial.

### The Bond Kernel

`WaterBox.computeBondEnergy()` is simpler — an O(B) loop over the 16
bonds:

```titrate
public fn computeBondEnergy(): double {
    var energy: double = 0.0;
    var i: int = 0;
    while (i < this.nBonds) {
        let a: int = this.bondA[i];
        let b: int = this.bondB[i];
        let dx: double = this.x[a] - this.x[b];
        let dy: double = this.y[a] - this.y[b];
        let dz: double = this.z[a] - this.z[b];
        let r: double = this.sqrt(dx*dx + dy*dy + dz*dz);
        let dr: double = r - 0.9572;
        energy = energy + 0.5 * 450.0 * dr * dr;
        i = i + 1;
    }
    return energy;
}
```

This loop is embarrassingly parallel — no data dependencies between
iterations — which makes it a good candidate for SIMD vectorization.

## How It Compiles to Native

Let's walk through what the native backend does with the LJ kernel.

### 1. Front-end (shared with bytecode)

The lexer, parser, and analyzer process the source exactly as they
would for the bytecode backend. The analyzer type-checks the
expressions, resolves the names (`this.x`, `this.sqrt`, `energy`), and
verifies ownership rules. The output is the same validated AST for
both backends.

### 2. LLVM IR emission

The codegen walks the AST and emits LLVM IR. For the LJ kernel, the
inner loop becomes something like:

```llvm
lj.inner:
  %i = phi i64 [ %i.entry, %lj.outer ], [ %i.next, %lj.inner.tail ]
  %j = phi i64 [ %j.init, %lj.outer ], [ %j.next, %lj.inner.tail ]
  %sum = phi double [ %sum.outer, %lj.outer ], [ %sum.next, %lj.inner.tail ]

  ; Load x[i], y[i], z[i], x[j], y[j], z[j]
  %xi.ptr = getelementptr [24 x double], [24 x double]* %x, i64 0, i64 %i
  %xi = load double, double* %xi.ptr
  ; ... similar for yi, zi, xj, yj, zj ...

  ; Compute dx, dy, dz, r2
  %dx = fsub double %xi, %xj
  %dy = fsub double %yi, %yj
  %dz = fsub double %zi, %zj
  %dx2 = fmul double %dx, %dx
  %dy2 = fmul double %dy, %dy
  %dz2 = fmul double %dz, %dz
  %r2 = fadd double %dx2, %dy2
  %r2.full = fadd double %r2, %dz2

  ; Call sqrt (inlined in release mode)
  %r = call double @sqrt(double %r2.full)   ; alwaysinline

  ; if (r > 0.001) { ... }
  %cmp = fcmp ogt double %r, 0.001
  br i1 %cmp, label %lj.body, label %lj.inner.tail

lj.body:
  ; sr = 3.166 / r
  %sr = fdiv double 3.166, %r
  ; sr6 = sr * sr * sr * sr * sr * sr
  %sr2 = fmul double %sr, %sr
  %sr4 = fmul double %sr2, %sr2
  %sr6 = fmul double %sr4, %sr2
  ; sr12 = sr6 * sr6
  %sr12 = fmul double %sr6, %sr6
  ; energy += 4 * 0.1554 * (sr12 - sr6)
  %diff = fsub double %sr12, %sr6
  %coef = fmul double 4.0, 0.1554
  %delta = fmul double %coef, %diff
  %sum.next = fadd double %sum, %delta
  br label %lj.inner.tail

lj.inner.tail:
  %sum.merged = phi double [ %sum, %lj.inner ], [ %sum.next, %lj.body ]
  %j.next = add i64 %j, 1
  %j.cmp = icmp slt i64 %j.next, %n.atoms
  br i1 %j.cmp, label %lj.inner, label %lj.outer

lj.outer:
  ; ... advance i, check loop condition ...
```

This is the unoptimized form. In release mode, LLVM applies:

- **Inlining** — `@sqrt` (the Newton's-method helper) is inlined into
  the loop body, eliminating the call.
- **Scalar replacement** — the `xi`, `yi`, `zi`, etc. allocas are
  promoted to SSA registers.
- **Loop-invariant code motion** — `4.0 * 0.1554` is hoisted out of
  the loop (it's a constant).
- **Branch prediction hints** — the `if (r > 0.001)` branch is
  predicted to be taken (it usually is).

### 3. Linking

The optimized IR is linked against `titrate_native` (which provides
`titrate_println`, `titrate_malloc`, `titrate_free`, and the native
wrappers for `Math.*`, `String.*`, etc.) and the system linker
produces the final executable.

## Hot Loop Identification

The hot loops in `mega_test_03` are, in decreasing order of time
spent:

1. **`WaterBox.computeLJEnergy()` — the O(N²) double loop.** This
   dominates runtime. For 24 atoms (276 pairs), it's roughly 80% of
   the total. For 96 atoms (4,560 pairs), it's over 95%.
2. **`WaterBox.computeBondEnergy()` — the O(B) bond loop.** Each
   iteration calls `sqrt()` (Newton's method, 20 iterations). For 16
   bonds, it's roughly 15% of the total at 24 atoms, and a smaller
   fraction at larger sizes.
3. **`WaterBox.sqrt()` — the Newton's-method square root.** Called
   once per pair and once per bond. Inlined by the native backend in
   release mode, so it doesn't show up as a separate function in the
   profile.

Everything else — the lattice setup, the I/O, the verification — is
negligible. The simulation is compute-bound, and the compute is
dominated by the LJ kernel.

You can confirm this by profiling with `perf` (Linux), `Instruments`
(macOS), or `vtune` (Windows). The profile will show >80% of time in
`computeLJEnergy`, with the rest split between `computeBondEnergy` and
the I/O.

## Performance Characteristics

### Speedup vs. Bytecode

| Atoms | Pairs | Bytecode | Native (release) | Speedup |
|---|---|---|---|---|
| 24 | 276 | baseline | ~5× faster | ~5× |
| 48 | 1,128 | baseline | ~6× faster | ~6× |
| 96 | 4,560 | baseline | ~7× faster | ~7× |

The speedup grows with workload size because the fixed native-bridge
overhead (process startup, linker work) is amortized over more
computation. At 96 atoms, the LJ kernel's O(N²) pair count dominates
everything else, and the native backend's dispatch elimination gives
the full benefit.

### Why the LJ Kernel Sees 5–6×

The LJ kernel is the worst case for the bytecode VM and the best case
for the native backend:

- **Worst case for bytecode** — each pair evaluation executes dozens
  of bytecode instructions (loads, stores, arithmetic, function
  calls). The dispatch overhead dominates the actual math.
- **Best case for native** — the hot loop is a tight block of
  floating-point arithmetic with no runtime calls (the `sqrt` is
  inlined). The CPU's branch predictor and out-of-order engine handle
  it efficiently.

The bond-energy loop sees a smaller speedup (~4×) because it's O(B)
rather than O(N²), so the fixed overhead is a larger fraction of the
total time.

### Why You Might Not See 3×

If the speedup is below 3×, the likely causes are:

1. **Native bridge overhead** — every call from native code back into
   the Titrate runtime (e.g. `io::println`, `ArrayList.get`) marshals
   values through the `TitrateValue` tagged union. For I/O-heavy or
   collection-heavy code this overhead can dominate. The LJ kernel
   has no such calls in its hot loop, so it doesn't pay this cost.
2. **Small workload** — for very short programs the link time and
   process startup dwarf the compute time. Increase the atom count to
   make the compute time dominate.
3. **No vectorization** — the LJ kernel has a data-dependent branch
   (`if (r > 0.001)`), which limits SIMD opportunity. The bond-energy
   loop vectorizes well; the LJ kernel doesn't.
4. **Debug build** — make sure you passed `--release`. Debug native
   builds are unoptimized and may even be slower than the bytecode
   VM.

## Profiling Tips

### Inspect the Generated IR

```bash
trc --native --release --emit-ir mega_test_03/src/main.tr
```

This writes the optimized IR to `main.ll`. Look for:

- `define internal ... @sqrt(...)` with `alwaysinline` — confirms the
  square-root helper is marked for inlining.
- `call fastcc` — confirms the fast calling convention.
- `!llvm.loop !{!"llvm.loop.vectorize.enable", i1 true}` — confirms
  vectorization metadata (on the bond-energy loop).
- The LJ kernel's inner loop should be a tight block of `fmul` /
  `fadd` / `fsub` / `fdiv` instructions with no `call` (because
  `sqrt` is inlined).

### Time Individual Sections

Wrap the section you want to measure in timing calls, or use
`io::println` markers in the Titrate source and time the gaps with a
wall clock. For more precise measurements, use the `native_bench.rs`
test harness, which uses Rust's `Instant::now()` / `elapsed()`.

### Compare Debug vs. Release Native

```bash
trc --native          mega_test_03/src/main.tr   # debug native
trc --native --release mega_test_03/src/main.tr  # release native
```

If release is dramatically faster than debug, the optimizations are
working. If they're similar, the program may be I/O-bound or the hot
loop may not be vectorizable.

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

## Further Reading

- [Native MD Simulation](/guide/native-md) — the guide version of
  this post, with the full benchmark table and troubleshooting tips.
- [Native Backend Performance — A War Story](/blog/native-perf-warstory)
  — the optimization story behind the 5–6× speedup.
- [Why Native?](/guide/native-intro) — what the native backend is and
  when to use it.
- [Building Native Binaries](/guide/native-build) — how to run the
  simulation yourself.
