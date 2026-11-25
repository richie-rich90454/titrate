# Titrate — Comprehensive Roadmap to Full Completion

**Status:** living document. Last updated 2026-11-22.
**Branch this work was performed on:** `feat/full-productization-g0`.
**Related documents:** `HANDOFF.md` (verified current state), `AGENTS.md` (authoritative syntax spec), `README.md` (public claims).

This document is the definitive, long-form plan for taking Titrate from its current state to the user's stated goal: a fully functional, zero-known-bug systems language that is the fastest at its abstraction and ease-of-use level, with a complete standard library, support for both trusted and untrusted execution, cross-platform graphics/audio, ML frameworks, and eventual OS development capability. It records what has already been completed so that later work never repeats it, and it states honestly which parts are multi-year programs rather than bug-fix passes.

---

## Table of contents

1. Vision and success criteria
2. Operating principles and non-negotiables
3. Current state (verified)
4. Target architecture
5. Workstream index
6. Phase 0 — Baseline and verification
7. Phase 1 — Correctness and safety
8. Phase 2 — Backend parity
9. Phase 3 — Verification enforcement
10. Phase 4 — Trust model and robustness
11. Phase 5 — Performance: bytecode VM
12. Phase 6 — Performance: native backend
13. Phase 7 — Performance: startup, compile latency, and DX
14. Phase 8 — JIT tier
15. Phase 9 — Standard library: foundation
16. Phase 10 — Standard library: data and science
17. Phase 11 — Standard library: network, async, system, OS
18. Phase 12 — Graphics and audio
19. Phase 13 — Machine learning
20. Phase 14 — Platform and OS readiness
21. Coverage checklist and definition of done
22. Gates and maturity levels
23. Risk register
24. Effort and sequencing
25. Decision log
26. Appendix A — inventory of completed fixes
27. Appendix B — known gaps at time of writing
28. Appendix C — command reference

---

## 1. Vision and success criteria

Titrate aims to be a single language that spans three abstraction tiers without forking the language:

- **script**: Python/JavaScript-level ease, sub-50 ms startup, instant iteration.
- **app**: Go/Rust-level systems programming with zero-GC ownership and regions, native AOT output, competitive with Rust on a published benchmark suite.
- **kernel**: freestanding systems code, no runtime dependencies, capable of booting an OS.

Success is not a feeling. It is defined by measurable contracts:

| Goal | Contract |
|---|---|
| No known bugs | Every defect discovered is fixed with a regression test, or recorded in `HANDOFF.md` as a deliberate, justified limitation. There are no undocumented failures. |
| Everything works | `cargo test --all --jobs 1` green on Windows and Linux; all three backends produce byte-identical output on the full corpus. |
| Fastest at this abstraction | Published criterion benchmarks against Rust, C++, Go, Python, and Node on a fixed suite, on fixed hardware, with real numbers including losses. |
| Complete stdlib | `docs/stdlib-coverage.json` generated from source shows 100% of the agreed reference API lists, with no unapproved exclusions. |
| Trusted and untrusted | A `RuntimePolicy` fixed at VM construction; untrusted mode is enforced by tests that prove denial of FFI, process, network, filesystem escape, and resource exhaustion. |
| OS capable | `trc --target x86_64-unknown-none` produces a bootable image with no compiler, Rust std, CRT, or LLVM-C in the output; a minimal kernel boots in QEMU. |

No engineer can honestly promise "zero issues whatsoever" for all future inputs and toolchains. The commitment is instead to the exit contract above: no *known* defect remains unaddressed, and every claim is backed by a command that actually ran.

---

## 2. Operating principles and non-negotiables

1. **One source language.** All three tiers compile the same `.tr` files; differences are profiles, not forks.
2. **One semantic source of truth.** TIR, not three independent codegens, defines program meaning.
3. **Byte-identical output.** VM, native, and WASM must agree exactly; strict floating-point semantics by default, `fast-math` only as explicit opt-in.
4. **Pure `.tr` standard library.** No logic moves into native libraries; the compiler lowers recognized patterns to optimized IR. The only native surface is a minimal, capability-tagged platform primitive layer.
5. **No silent fallbacks.** A fallback is allowed only when it is semantically correct (e.g. `Json.parse` falling back to the pure parser when the native handle is unavailable) and is documented in code.
6. **Preserve source compatibility.** No removal of existing syntax sugar, natives, or public APIs. New features are additive; `--compat` preserves old permissive analysis indefinitely.
7. **Root cause first.** No panics hidden by `catch_unwind`, no validation bypasses, no weakened tests.
8. **Evidence before claims.** HANDOFF records only commands that were run, with dates.
9. **Atomic changes.** One concern per commit, American English, no placeholder/stub/TODO.
10. **Security at trust boundaries.** Untrusted input is validated; trusted mode is explicit, not accidental.

---

## 3. Current state (verified 2026-11-22)

Completed in the first implementation pass on `feat/full-productization-g0`:

- Module-aware analyzer with strict default and `--compat`; canonical `error[E0XXX]` diagnostics; closure and generic-enum inference fixes.
- Parser fixes for `?` disambiguation, `>>>=`, and interpolation.
- VM: configurable/fatal step limit, checked operand reads, safe stack operations, `i32` jump offsets.
- Native robustness: bounded reads, UTF-8-safe hex, decompression caps, JSON depth/cycle limits, symlink-safe walks, poison-safe locks, process/socket timeouts, OS-seeded randomness.
- WASM parity: string relational ops, `null`, substring, heterogeneous tuple indexing.
- LLVM: half marshalling, generic interface default bodies, stdin natives (`IO_readLine`/`IO_readAll`), JSON fallback preserved.
- Build: honest LLVM requirement, generated header (561 wrappers + 15 helpers = 576 exports), tracked `AGENTS.md`.
- Tests: native suites un-ignored with capability detection; 123/123 native sample parity; 88 WASM tests; 9/9 native gaps; registry 1:1:1 check currently failing (HANDOFF §0).
- Docs: evidence-based `HANDOFF.md`, corrected counts and toolchain version everywhere.

Verified gates: `cargo build --all --jobs 1`, `cargo clippy --all --jobs 1 -- -D warnings`, `cargo test --all --jobs 1` (1,260 tests across the binaries reached in the latest gate log, one `native_registry` failure).

**Known gap at time of writing:** `stdlib_runtest_native` is 206/598. It is report-only in CI and documented in `HANDOFF.md` §6. Closing it is Phase 2 continuation work below.

**Not yet started:** TIR refactor, JIT, stdlib expansion, graphics/audio, ML, OS readiness, Rust benchmarks.

---

## 4. Target architecture

```
.tr source → lexer → parser → AST → module-aware analyzer (effects, refinements, ownership)
                                   ↓
                       ┌─────────── TIR ───────────┐
                       │ interned u16 names        │
                       │ field slots + shared layouts│
                       │ method slots + shared vtables│
                       │ resolved call targets      │
                       │ monomorphized generics      │
                       │ effect + capability tags   │
                       │ SIMD/parallel/GPU hints    │
                       └──┬──────────┬──────────┬───┘
                          ↓          ↓          ↓
                   bytecode VM    LLVM AOT    WASM / Component
                   + ORC JIT      + LTO/PGO   model output
                   (script)       + SIMD      (embedded)
                          ↓          ↓
                   ┌──────────────────────────┐
                   │ minimal host primitives  │
                   │ window/input/audio/GPU/  │
                   │ timer; capability-tagged │
                   └──────────────────────────┘
```

TIR is the single point where name resolution, field/method layout, capability checks, and optimization hints are decided. All backends consume it. This eliminates the class of bugs caused by three codegens independently re-deriving semantics from the AST.

---

## 5. Workstream index

| ID | Workstream | Phase(s) | Owner profile |
|---|---|---|---|
| A | Correctness and diagnostics | 1 | compiler engineer |
| B | Trust model and robustness | 4 | security engineer |
| C | Bytecode VM performance | 5 | runtime engineer |
| D | Native backend performance | 6 | LLVM engineer |
| E | Build/startup/DX | 7 | tooling engineer |
| F | JIT tier | 8 | JIT engineer |
| G | Standard library waves | 9–11 | stdlib engineers |
| H | Graphics and audio | 12 | platform engineers |
| I | Machine learning | 13 | ML engineer |
| J | OS/platform | 14 | systems engineer |
| K | Verification and fuzzing | all | test engineer |
| L | Docs and governance | all | documentation writer |

---

## 6. Phase 0 — Baseline and verification

**Goal:** never claim a gate without running it.

**Tasks:**
1. Run and record `cargo build --all --jobs 1`, `cargo test --all --jobs 1`, `cargo clippy --all --jobs 1 -- -D warnings`, `cargo build --release -p titrate_native --jobs 1`.
2. Run native parity, native gaps, registry, WASM, and stdlib harnesses; record exact counts.
3. Record toolchain versions and working-tree state in `HANDOFF.md`.
4. Refresh `HANDOFF.md` after every gate; never write intentions as facts.

**Tracker:** `docs/native-parity-status.md` records the root-cause checklist;
`scripts/classify-native-failures.ps1` emits the live failure-count table from
a `.local` harness log.

**Done when:** every claim in `HANDOFF.md` cites a command that ran, with a date.

**Status:** COMPLETE for the current pass; must be repeated after each phase.

---

## 7. Phase 1 — Correctness and safety

**Goal:** no known P0 defect.

**Completed items:**
- Import analyzer bypass (`main.rs`) replaced by module-aware analysis; `--compat` preserved.
- Phantom `trc_llvm_unavailable` cfg removed; missing LLVM is a hard, explicit error.
- Canonical E0XXX diagnostics wired into CLI.
- `?` ternary/propagation ambiguity, `>>>=`, interpolation trailing tokens.
- Step cap made configurable, fatal, reset; checked operand reads; safe stack ops.
- `i16` jump overflow eliminated; jump operands widened to `i32`.
- Closure return-type inference and generic enum assignability.
- Six shipped examples corrected for strict mode rather than weakening the checker.

**Remaining items:**
- Continue native stdlib parity (see Phase 2 continuation).
- Complete the bug sweep in Appendix B.

**Done when:** no open P0 defect; every fixed defect has a named regression test.

---

## 8. Phase 2 — Backend parity

**Goal:** VM ≡ native ≡ WASM bytes on the entire corpus.

**Completed:**
- WASM string ordering, `null`, `substring`, tuple indexing; 88 tests pass.
- LLVM stdin bridge, half marshalling, generic interface defaults.
- `native_samples_parity` 123/123; `native_gaps` 9/9.

**Continuation — native stdlib parity (206/598):**

Failure categories observed:
1. Unsupported call targets for some imported module functions.
2. Unsupported `Index` expressions on certain container results.
3. Container type mismatch `{ i64, ptr }` → `{ ptr, ptr }`.
4. Methods missing from generic-class vtables.
5. Hard crashes: access violations, stack overflows, LLVM signature assertion.

**Approach:**
- Build a complete failure inventory by dumping all 392 failures (the harness currently prints 20). Add a diagnostic env var to `stdlib_runtest.rs` that writes every failure to a file.
- Triage by root cause, not by file. A single systemic fix should resolve many failures.
- For each category: add an IR-level regression test first, then fix the codegen path, then re-run the full 598-file harness.
- Prioritize crashes (safety) over wrong output over unsupported constructs.
- Re-enable `stdlib_runtest_native` as a gating test only when 598/598 passes.

**Done when:** 598/598 native stdlib programs pass; `stdlib_runtest_native` is no longer `#[ignore]`d; no `continue-on-error` remains in CI for it.

---

## 9. Phase 3 — Verification enforcement

**Goal:** claims enforced by CI, not by hand.

- Native tests un-ignored with runtime capability detection (COMPLETE).
- Registry 1:1:1 check enforced, but currently failing on the wrapper/registry mismatch (HANDOFF §0).
- `stdlib_runtest_all` requires ≥581 files, per-file `PASS:` markers, and errors on empty synthetic mains (COMPLETE).
- CI builds release native lib then runs `cargo test --all` on Windows and Linux (COMPLETE).
- Add continuous benchmarking with a 5% regression gate.
- Add nightly fuzz runs for lexer, parser, bytecode, and native argument surfaces.
- Add differential fuzzing: generate random valid programs, execute on all three backends, compare bytes.

**Done when:** every headline claim in `README.md` is checked by CI.

---

## 10. Phase 4 — Trust model and robustness

**Completed:** capability-oriented hardening of natives, bounded reads, JSON depth/cycle limits, symlink-safe walks, poison-safe locks, timeouts, OS-seeded RNG, `--compat`.

**Remaining:**
- Formalize `RuntimePolicy` as a single struct fixed at VM construction; thread it through every native.
- Untrusted default: FFI/process/signals/registry off; network allowlist; filesystem confined to a canonicalized root with symlink-escape rejection; step/memory/depth/output/time budgets.
- `Secret` value type that refuses `display_string`, JSON serialization, and FFI conversion; zeroization.
- Effect system so untrusted mode rejects undeclared `io`/`net`/`ffi` effects at compile time.
- Constant-time crypto comparison already correct; add secure-bytes native backed by `getrandom`.
- External security review and fuzz campaign.

**Done when:** a test suite proves every denied capability is actually denied, and no user-triggerable crash/OOM/hang is known.

---

## 11. Phase 5 — Performance: bytecode VM

**Goal:** 2–5× faster VM, byte-identical output.

**Quick wins (low risk):**
- Remove `fname.clone()` on every CALL.
- Gate the full-stack closure upvalue scan behind a compiler-produced `needs_upvalues` flag.
- Pass `&[Value]` slices to natives instead of 32 `drain().collect()` allocations.
- Slice iterator `remaining` instead of cloning twice.
- `Chunk::strings: Vec<Rc<String>>` so `PUSH_STRING` does not reallocate literals.
- Unified output helper using `display_into`.

**Structural (TIR-backed):**
- Interned `u16` names for fields, methods, classes, and static calls.
- `fields: Vec<Value>` with shared, sorted class layouts; instances store `class_idx`.
- Shared per-class vtable; no vtable clone per instance.
- Precompiled field initializers as permanent functions.
- Resolved call operands patched at `load_program`.
- Decode-once run loop with cached `ip` and chunk slice.
- Box large `Value` variants behind `Rc` (88-byte slots → ~24–32).
- In-place ArrayList mutation; auxiliary hash index for HashMap while keeping insertion-order arrays.
- Zero-copy string slices.

**Done when:** criterion benchmarks show the targets; parity tests remain byte-identical.

---

## 12. Phase 6 — Performance: native backend

**Goal:** within 10–20% of Rust on the curated suite.

- O3 instead of O2 (`program.rs`).
- ThinLTO and PGO wired into `pipette build --release --profile`.
- Eliminate the 24-byte tagged-union marshal on hot paths; direct typed symbol calls.
- Full monomorphization including containers (`mono.rs`).
- Unboxed representations; alias-analysis-friendly IR.
- SIMD auto-lowering from recognized `.tr` patterns; strict FP default, `fast-math` opt-in.
- Custom calling convention after ABI correctness is proven.
- SIMD/parallel/GPU lowering hints in TIR.
- Published benchmark suite with Rust, C++, Go, Python, and Node baselines.

**Done when:** the benchmark table is published with real numbers and the 10–20% target is met on the curated suite.

---

## 13. Phase 7 — Performance: startup, compile latency, DX

**Startup:** cached TIR and precompiled stdlib; small runtime; target <50 ms to first output.
**Compile latency:** content-addressed TIR cache, incremental module compilation, parallel codegen; target <2 s incremental, <15 s cold.
**DX:** structured JSON diagnostics with machine-applicable fixes; LSP (completion, go-to-def, hover types, inline hints, quick fixes); `trc --eval`, `--watch`; DAP debugger with time-travel; built-in `--profile` flamegraphs; `trc fmt`, `trc lint`; `pipette` lockfile and SBOM; tree-sitter grammar; deterministic builds.

**Done when:** all DX features have tests and the latency targets are measured.

---

## 14. Phase 8 — JIT tier

- LLVM ORC JIT in the VM for hot functions.
- Lightweight profiling marks hot bytecode; compile TIR in background; switch to native with deopt back to VM.
- Combination of sub-50 ms startup with near-native steady state.
- Feature-flagged; VM remains fallback.

**Done when:** JIT-enabled runs are byte-identical to VM and faster on the hot-loop benchmarks.

---

## 15. Phase 9 — Standard library: foundation

**Priority order from user:** data/science first, then core collections/util, network/async, system/OS, then more.

Foundation wave: paths, iterators, datetime/time zones, decimal/fractions, struct, bisect, heapq, copy, records, enums, logging, argument parsing. All pure `.tr`, all three backends.

**Done when:** the generated coverage checklist shows 100% for the foundation reference lists.

---

## 16. Phase 10 — Standard library: data and science

Built on the existing `NDArray` core (11 files) and `Matrix`/linalg:
- Full ufunc set, broadcasting completion, structured dtypes, masked arrays, FFT, polynomial, linalg decompositions, PCG64 RNG, statistics, DataFrame-like tabular type.
- Compiler SIMD auto-lowering accelerates these without leaving `.tr`.

**Done when:** NumPy top-level API coverage is 100% on the generated checklist; numeric benchmarks published.

---

## 17. Phase 11 — Standard library: network, async, system, OS

- Async/await as language syntax; structured concurrency; `parallel_for`.
- `urllib`, HTTP client/server completion, selectors, DNS/URL utilities.
- Files, processes, logging, serialization, concurrency, compression/archive completeness.
- Capability tags on every I/O API.

**Done when:** reference API lists for CPython `http`/`urllib`/`asyncio`/`os`/`sys`/`subprocess` and Java `java.net`/`java.nio` are 100% covered.

---

## 18. Phase 12 — Graphics and audio

- Minimal host primitives: window, input, audio device, GPU buffer, timer.
- Pure `.tr` wrappers: 2D/3D rendering, sprites, shaders, audio playback/synthesis, WebAudio/SDL-style APIs.
- Cross-platform on Windows and Linux.

**Done when:** graphics/audio demos run cross-platform; no rendering or DSP logic exists outside `.tr`.

---

## 19. Phase 13 — Machine learning

- Estimator-style API, tensor/autograd surface on NDArray, training and inference benchmarks.
- Pure `.tr`; accelerated by SIMD/GPU lowering.
- scikit-learn estimator API and a small tensor/autograd surface as reference lists.

**Done when:** ML benchmarks train and infer successfully on pure `.tr` NDArray.

---

## 20. Phase 14 — Platform and OS readiness

Separate, multi-year program:
- `titrate_rt`: `no_std` bump allocator, custom `_start`, panic handler, linker script, `-nostdlib`.
- Final binary must contain no compiler, Rust std, CRT, or LLVM-C.
- `--target <triple>` and cross-compilation; target wrappers beyond X86.
- Language primitives: volatile load/store, inline asm, MMIO, port I/O, LLVM atomics with explicit orderings, `repr(C)`/`packed`/unions/section attributes.
- Real NLL borrow checker; `unsafe` scoped to genuinely unsafe operations; drop flags on all exits.
- Kernel substrate: IDT/interrupts, scheduler, page tables, drivers, boot protocol.
- Minimal kernel boots in QEMU.

**Done when:** a bootable kernel image is produced and runs in QEMU.

---

## 21. Coverage checklist and definition of done

The checklist in `docs/stdlib-coverage.json` is generated, never hand-edited, by
`pwsh scripts/gen-stdlib-coverage.ps1`. The generator scans `lib/tt/**/*.tr` for
public `fn`/`class`/`interface`/`enum` declarations and derives every count from
source; see `docs/guide/stdlib-coverage.md` for the schema and status meanings.

**The current report is an inventory, not proof of parity with the reference
lists below.** Its only status is `implemented`, meaning a declaration exists in
source. It makes no claim of Java, Python, or NumPy parity and must not be read
as a 100% result. Comparing the inventory against these reference sets is the
remaining work of phases 9–13. Reference sets:

| Domain | Reference |
|---|---|
| Core/util | CPython stdlib modules + Java SE packages |
| Collections | Java Collections Framework + Python `collections`/`itertools`/`functools` |
| Data/science | NumPy top-level + pandas-like tabular |
| Network/async | Python `http`/`urllib`/`asyncio`/`selectors` + `java.net` |
| System/OS | Python `os`/`sys`/`subprocess`/`logging` + `java.nio` |
| Graphics | raylib/SDL-style surface |
| Audio | WebAudio/SDL_mixer-style surface |
| ML | scikit-learn estimator API + tensor/autograd surface |

Status vocabulary: `implemented`, `partial`, `missing`, `explicitly-excluded` with reason. Only `implemented` is currently emitted, as a source-presence marker; the other three require the reference-list comparison that has not been done yet. "Fully expanded" = zero `missing`/`partial` outside approved exclusions, measured against those reference lists. Every API has a `.tr` test, generated doc page, cross-backend parity test, capability tag, and a benchmark if hot.

---

## 22. Gates and maturity levels

| Gate | Condition |
|---|---|
| G0 | Baseline recorded |
| G1 | No known P0 defects |
| G2 | VM≡native≡WASM≡oracle bytes on the full corpus, including stdlib |
| G3 | Untrusted isolation enforced; fuzz clean |
| G4 | VM performance targets met |
| G5 | Native performance targets met; Rust comparison published |
| G6 | Reserved |
| G7 | Stdlib coverage 100% |
| G8 | Graphics/audio cross-platform |
| G9 | ML benchmarks pass |
| G10 | QEMU kernel boot |

Levels: L0 current → L1 verified → L2 correct & safe → L3 fast VM → L4 fast native + DX → L5 stdlib complete → L6 media & ML complete → L7 platform. "Everything fully reached" = L7 plus the exit contract.

---

## 23. Risk register

1. Native stdlib parity may require deep LLVM rewrites — mitigated by full failure inventory and root-cause triage.
2. SIMD changes FP order — strict FP default plus explicit `fast-math`.
3. TIR refactor is large — land incrementally behind existing codegens with parity gates.
4. Strict analysis surfaces latent errors in shipped programs — fix the programs; `--compat` preserves old behavior.
5. Pure `.tr` numeric performance may trail tuned native libraries — compiler lowering is the mitigation; report real numbers.
6. Huge stdlib increases compile time and binary size — TIR cache and capability-tagged lazy modules.
7. GUI/audio primitives expand the host layer — keep minimal and capability-tagged.
8. Multi-year timeline — keep phases independently shippable.

---

## 24. Effort and sequencing

| Span | Estimate (solo) |
|---|---|
| L1–L2 including native stdlib parity | 4–9 months |
| L3–L4 including JIT and DX | 12–18 months |
| L5 stdlib | 15–24 months additional |
| L6 media/ML | 24–36 months additional |
| L7 OS | 2–4 years additional |

Total to L7: roughly 6–8 years solo. These are estimates, not commitments. Phases are ordered so each yields a usable product.

---

## 25. Decision log

- All three abstraction tiers; all metrics including DX.
- Zero-GC ownership/regions retained; implicit region for script mode.
- 100% source compatibility; `--compat` retains permissive analysis forever.
- Stdlib API idiomatic Titrate plus familiarity aliases.
- Stdlib source pure `.tr`; compiler auto-lowers numeric patterns to SIMD.
- Minimal host primitives approved for graphics/audio.
- Generated coverage checklist is the definition of "fully expanded".
- OS is a separate track after the core product.

---

## 26. Appendix A — inventory of completed fixes

See `HANDOFF.md` §0–§5 for the verified list. Summary: 12 correctness defects, native robustness hardening, WASM and LLVM parity fixes, generated header, honest build errors, CI native enforcement, evidence-based documentation.

---

## 27. Appendix B — known gaps at time of writing

1. **Native stdlib parity:** 206/598 (`HANDOFF.md` §6). Report-only in CI. Gating requires 598/598.
2. **TIR, JIT, stdlib expansion, graphics/audio, ML, OS:** not started; phases 8–14.
3. **Rust performance comparison:** not published; Phase 6.
4. **Security certification:** hardening implemented, no external review; Phase 4.

---

## 28. Appendix C — command reference

```
cargo build --all --jobs 1
cargo test --all --jobs 1
cargo clippy --all --jobs 1 -- -D warnings
cargo build --release -p titrate_native --jobs 1
cargo test -p trc --test native_samples_parity --jobs 1
cargo test -p trc --test native_gaps --test native_registry --jobs 1
cargo test -p trc --test stdlib_runtest --jobs 1 -- --ignored   # native parity report
pwsh scripts/gen-header.ps1
```

---

This roadmap is complete as a plan. Its execution is gated by the phases above; no phase may be reported done without its gate command recorded in `HANDOFF.md`.
