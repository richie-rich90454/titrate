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
| `cargo test -p trc --lib --jobs 1` | 950 passed, 0 failed, 5 ignored |
| `cargo test --all --jobs 1` | 1,260 tests across the binaries reached in the latest gate log, 1 failure (`native_registry`) |
| `cargo test -p trc --test examples_regression --test demos_regression --jobs 1` | pass |
| WASM suites (16 `wasm_*.rs` targets) | 88 tests, 0 failures |
| `cargo test -p trc --test native_samples_parity --jobs 1` | pass — 123/123 VM/native normalized stdout parity |
| `cargo test -p trc --test native_gaps --test native_registry --test native_hello --test native_mega_test --test native_bench --test tco_o0 --test tco_deep_recursion --test rejected_now_supported --jobs 1` | 1 failure (`native_registry`); remaining targets not reached in the latest gate log |
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
| `lib/tt/` | Titrate standard library source (616 modules). |
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
9/9 `native_gaps`, `native_hello`, `native_mega_test`, benchmarks, and TCO tests
pass in prior runs; registry 1:1:1 currently fails (section 0). Stdlib-wide parity
is incomplete (section 6).

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

Runtime trust-mode separation now exists via `RuntimePolicy` (`trc/src/runtime_policy.rs`):
trusted is the default; `--untrusted` enables capability denial and resource
budgets, with native dispatch gated centrally in `call_native_fn`. Confinement
is lexical, so symlink escapes are not blocked, and general VM heap growth is
bounded only indirectly. What also exists:

- Strict analysis by default; `--compat` restores the old permissive analysis
  for files with imports. This changes compile-time checking only — it has
  **no runtime effect**.
- Unconditional hardening applied to every program: bounded reads and lengths,
  JSON depth/cycle limits, symlink-safe directory walks, poison-safe lock
  registries, socket/subprocess timeouts, and OS-seeded `Math.random()`.

Capability denial is centralized rather than per-native, and no external
security review has been done. Treat untrusted mode as a best-effort barrier,
not a hardened sandbox.

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
- **Stdlib coverage report:** `docs/stdlib-coverage.json` is a source-derived inventory (616 modules, 11,534 public declarations) generated by `scripts/gen-stdlib-coverage.ps1`; it is not evidence of reference-list parity (see `docs/guide/stdlib-coverage.md`).

---

## 6. Active gap — native stdlib parity

`stdlib_runtest_native` compiles and runs 598 stdlib test programs through the
LLVM backend. **206 pass, 392 fail** in the latest measurement;
the baseline was 199/598. The latest declared-return, generic-vtable, and
type-argument work landed after that measurement and has not yet been
re-measured. Remaining known causes include:

- declared return types are now threaded at many call sites but some paths
  still use name-table inference from `native_bridge.rs`
- monomorphized classes use retained interface declarations, but default
  method bodies for specialized classes are still a known risk
- some container/interface conversions between `{ i64, ptr }` and
  `{ ptr, ptr }` remain incomplete
- recursion and mangling collisions can still overflow the native stack
- hard crashes (access violations) from the above

Do not claim native stdlib parity.

The test is `#[ignore]`d with this status and runs in CI as a **report-only**
step (`continue-on-error`), so the gap stays visible without making the gate red.
It is tracked as the next major work item.

---

## 7. ABI facts

- Rust `TitrateValue` is `#[repr(C)] { tag: i32, _pad: i32, payload: [u8;16] }`,
  24 bytes, 8-byte aligned. Tags: `TV_VOID=0`, `TV_NULL=1`, `TV_BOOL=2`,
  `TV_INT=5`, `TV_LONG=6`, `TV_FLOAT=9`, `TV_DOUBLE=10`, `TV_CHAR=11`,
  `TV_STRING=12`, `TV_ARRAY=13`, `TV_HALF=19`, `TV_QUAD=20`, `TV_HANDLE=21`.
- LLVM `titrate_value_type` is `{ i32, i32, [16 x i8] }`; array indexing uses
  GEP indices `[0, i]`.
- `titrate_native.h` is generated by `scripts/gen-header.ps1` from
  `titrate_native/src/wrappers/*.rs`: 561 wrappers + 15 direct helpers = 576
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
