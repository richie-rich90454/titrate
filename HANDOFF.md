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
host shim in `trc/tests/wasm_common/mod.rs`. 88 tests pass. Most are true VM
parity; three string-ordering/substring/tuple-index tests use hardcoded expected
output because the VM rejects those source constructs, so they are **not** VM
parity tests. No CLI flag; use `trc::codegen::wasm::compile_to_wasm`. Unsupported
constructs are listed in `docs/guide/wasm-backend.md`.

**Native sample parity** is normalized **stdout** comparison over 123 programs;
six network/infinite demos are excluded (`native_samples_parity.rs:35-42`). Stderr
is not compared, so "byte parity" is an overstatement.

---

## 4. Trust model

There is **no runtime trust-mode separation yet**. Do not describe the current
VM as having trusted/untrusted modes. What actually exists:

- Strict analysis by default; `--compat` restores the old permissive analysis
  for files with imports. This changes compile-time checking only — it has
  **no runtime effect**.
- Unconditional hardening applied to every program: bounded reads and lengths,
  JSON depth/cycle limits, symlink-safe directory walks, poison-safe lock
  registries, socket/subprocess timeouts, and OS-seeded `Math.random()`.

A real `RuntimePolicy` with capability denial (FFI, process, network, filesystem
confinement, resource budgets) is a **remaining** item in `ROADMAP.md` Phase 4.
Scripts are still equivalent to arbitrary native code; do not run untrusted
`.tr` source in a privileged process. No external security review has been done.

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
- **Stdlib coverage report:** `docs/stdlib-coverage.json` is a source-derived inventory (571 modules, 10,957 public declarations) generated by `scripts/gen-stdlib-coverage.ps1`; it is not evidence of reference-list parity (see `docs/guide/stdlib-coverage.md`).

---

## 6. Active gap — native stdlib parity

`stdlib_runtest_native` compiles and runs 598 stdlib test programs through the
LLVM backend. **200 pass, 398 fail** after the glob-import and index-expression
fixes; the earlier baseline was 199/598. Failures include:

- explicit generic type arguments are parsed then discarded, so zero-arg
  generic factories cannot be monomorphized
- function-valued fields are mis-dispatched as virtual methods
- name-inferred native ABI return types produce corrupt payloads
- generic classes lack interface vtables; container element types are not
  resolved through monomorphized names
- LLVM call arguments are not coerced to declared parameter types, causing
  `CallInst` signature assertion aborts
- container type mismatches such as `{ i64, ptr }` to `{ ptr, ptr }`
- hard crashes (access violations, stack overflows) from the above

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
  `titrate_native/src/wrappers/*.rs`: 542 wrappers + 15 direct helpers = 557
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
