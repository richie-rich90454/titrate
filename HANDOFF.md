# HANDOFF — Titrate compiler, VM, and backends

**Branch:** `feat/full-productization-g0`
**State as of:** 2026-11-22. Every gate below was actually run on this date.
**Scope:** canonical state document. Work listed as PENDING or KNOWN GAP is not done.

---

## 0. Ground truth

| Command (workspace root) | Result |
|---|---|
| `cargo build --all --jobs 1` | pass |
| `cargo clippy --all --jobs 1 -- -D warnings` | pass |
| `cargo build --release -p titrate_native --jobs 1` | pass |
| `cargo test -p trc --lib --jobs 1` | 816 passed, 0 failed |
| `cargo test -p trc --test examples_regression --test demos_regression --jobs 1` | pass |
| WASM suites (16 `wasm_*.rs` targets) | 88 tests, 0 failures |
| `cargo test -p trc --test native_samples_parity --jobs 1` | pass — 123/123 VM/native byte parity |
| `cargo test -p trc --test native_gaps --test native_registry --test native_hello --test native_mega_test --test native_bench --test tco_o0 --test tco_deep_recursion --test rejected_now_supported --jobs 1` | all pass (0 ignored) |
| `cargo test -p trc --test stdlib_runtest --jobs 1` | `stdlib_runtest_all` passes; native leg remains a KNOWN GAP (section 6) |

Toolchain: rustc/cargo 1.99.1; LLVM at `C:\Program Files\LLVM` reports `23.0.0git`
and builds because `trc/build.rs` links `LLVM-C` directly. LLVM 22.1 or newer is
required. On Windows use `--jobs 1`; parallel jobs can exhaust the page file.

---

## 1. Architecture

| Component | Role |
|---|---|
| `trc/` | Compiler **and** bytecode VM **and** all backends: `lexer/`, `parser/`, `ast/`, `analyzer/`, `bytecode/` (compiler + VM + natives), `codegen/llvm/` (native), `codegen/wasm/`. `interpreter/` is a legacy tree-walker retained for tests. |
| `pipette/` | Build tool / project manager. Not the VM. |
| `titrate_native/` | C-ABI runtime: `TitrateValue`, per-native wrappers, generated header. |
| `lib/tt/` | Titrate standard library source (571 modules). |
| `demos/`, `examples/` | 113 and 16 `.tr` programs. |
| `tests/stdlib/`, `tests/e2e/`, `tests/fixtures/stdlib/` | Canonical test roots; old root-level `stdlib_test/`, `mega_test*` paths no longer exist. |
| `AGENTS.md` | Authoritative language syntax specification; tracked in git. |

---

## 2. Verified gates

Run from the workspace root with `--jobs 1` on Windows. The commands in section 0
are the authoritative list. Do not report any gate as green unless you ran it.

---

## 3. Backend status

**Bytecode VM (default).** Reference implementation for output parity. Full unit
suite, demo/example regressions, `stdlib_runtest_all`, and all fuzz harnesses pass.

**Native backend (LLVM).** Now exercised end to end in CI: 123/123 sample parity,
9/9 `native_gaps`, registry 1:1:1, `native_hello`, `native_mega_test`, benchmarks,
and TCO tests all pass. Stdlib-wide parity is incomplete (section 6).

**WASM backend.** Real backend (`trc/src/codegen/wasm/`, seven source files) with a
host shim in `trc/tests/wasm_common/mod.rs`. 88 tests pass. No CLI flag; use
`trc::codegen::wasm::compile_to_wasm`. Unsupported constructs are listed in
`docs/guide/wasm-backend.md`.

---

## 4. Trust model

The VM now supports both modes:

- **Trusted** (default for existing programs): all natives available.
- **Untrusted**: strict analysis (`--compat` restores the old permissive mode),
  plus capability-gated file/network/process/FFI natives, bounded reads and
  allocations, JSON depth/cycle limits, symlink-safe directory walks, poison-safe
  lock registries, socket/subprocess timeouts, and OS-seeded randomness for
  `Math.random()`.

Security work is **not certified**. The hardening above is implemented and tested
at the unit level; no external penetration test or formal review has been done.

---

## 5. Known limitations

- **`Thread_spawn` is synchronous.** The closure runs on the calling thread;
  the OS thread is a no-op preserving the handle API (`natives/thread.rs`).
- **No tracing garbage collector by design.** Ownership, borrowing, and regions
  are the memory model; `Gc.collect()` is only a hint.
- **Self-only tail-call optimization.** Bytecode `TAIL_CALL` and LLVM
  `compile_self_tail_call` cover direct self recursion; virtual, indirect, and
  native calls are not optimized.
- **WASM unsupported constructs** are documented in `docs/guide/wasm-backend.md`.
- **Native generics gap** (section 6).
- **OS readiness:** the native backend is hosted only. There is no `no_std`
  runtime, freestanding target, cross-compilation, volatile/inline asm/MMIO, or
  kernel-grade borrow checker. Writing an OS requires the separate L7 program.

---

## 6. Active gap — native stdlib parity

`stdlib_runtest_native` compiles and runs 598 stdlib test programs through the
LLVM backend. **199 pass, 399 fail.** Failures include:

- unsupported call targets for some imported module functions
- `unsupported expression: Index(...)` on certain container results
- container type mismatches such as `{ i64, ptr }` to `{ ptr, ptr }`
- methods missing from generic-class vtables
- hard crashes (access violations, stack overflows, LLVM signature assertion)

The test is `#[ignore]`d with this status and runs in CI as a **report-only**
step (`continue-on-error`), so the gap stays visible without making the gate red.
It is tracked as the next major work item. Do not claim native stdlib parity.

---

## 7. ABI facts

- Rust `TitrateValue` is `#[repr(C)] { tag: i32, _pad: i32, payload: [u8;16] }`,
  24 bytes, 8-byte aligned. Tags: `TV_VOID=0`, `TV_NULL=1`, `TV_BOOL=2`,
  `TV_INT=5`, `TV_LONG=6`, `TV_FLOAT=9`, `TV_DOUBLE=10`, `TV_CHAR=11`,
  `TV_STRING=12`, `TV_ARRAY=13`, `TV_HALF=19`, `TV_QUAD=20`, `TV_HANDLE=21`.
- LLVM `titrate_value_type` is `{ i32, i32, [16 x i8] }`; array indexing uses
  GEP indices `[0, i]`.
- `titrate_native.h` is generated by `scripts/gen-header.ps1` from
  `titrate_native/src/wrappers/*.rs`: 540 wrappers + 15 direct helpers = 555
  exports. `native_call_out` is intentionally excluded from the uniform set
  because it uses an out-pointer signature.
- Native indices are positional in bytecode. New natives must be registered at
  the **end** of `Vm::new()`; `IO_readLine` and `IO_readAll` were added there.

---

## 8. Conventions

- American English everywhere.
- Atomic commits; one concern per commit.
- No `unwrap()`/`expect()` in the compiler pipeline where user code can trigger
  them; return errors instead.
- No placeholder, TODO, stub, or silent wrong-value fallback in compiler code.
- Keep the three backends byte-identical; run parity gates after any codegen edit.
