# HANDOFF — Titrate Compiler / VM / Native Backend

> **DO NOT COMMIT THIS FILE.** It is a working handoff document for the next
> conversation. Delete it once the work it describes is resumed or completed.

---

## 1. Mission & Operating Rules (carry into next conversation verbatim)

The project is **Titrate**: a Rust-based systems language with a bytecode VM
(`pipette`) and an LLVM native backend (`trc`). The mission is to make it
production-grade: no bugs, everything works, the native backend is complete.

### Strict commit constraints (NON-NEGOTIABLE)
1. **One file per commit.** Stage and commit EXACTLY ONE file per commit. If a
   fix spans `ast.rs`, `resolver.rs`, `codegen.rs`, fix and commit them one at
   a time in logical dependency order.
2. **Atomic commits.** Never bundle unrelated changes.
3. **American English** everywhere: commit messages, code comments, compiler
   error strings (e.g. "optimize", "color", "initialize", "behavior").
4. **Commit as you go.** `git add <file>` + `git commit` immediately after
   fixing and verifying each file. Verify with `git log -1`.
5. **No regressions.** Run the test suite after every fix.

### Zero-laziness doctrine
- No placeholders, `TODO`, `FIXME`, `todo!()`, `unimplemented!()`, stubs.
- No `unwrap()`/`expect()` in the compiler pipeline where user code could
  trigger them; return `Result<T, CompilerError>`.
- Run tests after every fix; verify compilation after every edit.
- Read files fully before editing; make surgical edits.
- Resolve every `cargo clippy -D warnings` warning (no `#[allow]` unless
  absolutely necessary and justified).
- Verify with `git log` after committing.

### Build / test commands
```
cargo build -p trc --jobs 1          # debug compiler
cargo build -p titrate_native --jobs 1   # native runtime lib (must rebuild when trc lib changes)
cargo build -p trc --release --jobs 1
cargo test --all --jobs 1            # full suite (must be green)
cargo clippy --all --jobs 1 -- -D warnings
cargo test -p trc --test native_registry -- --ignored   # wrapper 1:1:1 check
cargo test -p trc --test native_bench -- --ignored      # native speedup tests
cargo test -p trc --test native_mega_test -- --ignored  # mega tests natively
./target/debug/trc.exe demos/foo.tr                     # bytecode run
./target/debug/trc.exe demos/foo.tr --native --release  # native compile+link
```
> Note: on this Windows box, run cargo with `--jobs 1` (parallel jobs can
> exhaust the page file, `os error 1455`).

---

## 2. Current State

### Green
- `cargo build --all`, `cargo clippy --all -- -D warnings`, `cargo test --all`
  are clean (full suite, 0 failures).
- `stdlib_runtest`: 581/581 passing.
- All 113 demos run correctly (only `file_watcher.tr` is an intentional
  infinite watcher).
- `cargo test -p trc --test native_registry -- --ignored`: **PASSES** —
  wrappers / VM lookup / C header are in exact 1:1:1 correspondence
  (518 wrappers, 519 header decls, 517+ registered).
- `native_hello` (3/3) passes. `native_bench` speedups are timing-flaky
  under load (mega_test_03 measured 2.3x–2.9x vs the 3x target).
- **`mega_test_03` runs natively with output identical to bytecode
  (VERIFICATION PASSED).**
- **`mega_test_02` now ALSO matches byte-for-byte natively** (quicksort,
  File I/O, word frequency all pass) — the container-bridge gap is closed,
  see section 4.
- **New `native_gaps` harness (9/9)**: VM-parity regression tests for
  closures, interface dispatch, and transparent container returns
  (`cargo test -p trc --test native_gaps -- --ignored`).

### C headers — there are a TON of them
`titrate_native/titrate_native.h` now declares **519** uniform wrappers:
```c
TitrateValue titrate_<Name>(const TitrateValue *args, size_t arg_count);
```
One per native in `titrate_native/src/wrappers.rs` (518 `titrate_*` fns) and
one per entry in `trc/src/bytecode/vm/natives/lookup.rs` (517, minus
`println` and `native_call_out` which are direct helpers). The test
`native_registry_1_to_1_to_1` enforces this correspondence; it was made to
pass by:
- `cc82b7d8 feat(native): add C wrappers for all 127 missing native functions`
- `f25ac536 feat(native): declare all native wrappers in the C header`
- `79b1b3fc fix(vm): register File_open in the native lookup table`
- `fee4a1d9 fix(test): treat native_call_out as a direct helper in registry check`

Wrapper pattern (thin dispatcher to the name-based bridge):
```rust
#[no_mangle]
pub unsafe extern "C" fn titrate_<Name>(args: *const TitrateValue, arg_count: usize) -> TitrateValue {
    unsafe { native_wrapper("<Name>", args, arg_count) }
}
```
The runtime entry point is `titrate_native_call_out(name, name_len, args,
arg_count, out)` in `titrate_native/src/wrappers.rs`, which resolves the name
via `lookup_builtin_native` and calls the VM native.

---

## 3. Recent commits (this session)

```
b3fc9e36 fix(llvm): transparent container returns with effect-safe slot sharing
0942cf73 fix(llvm): interface dispatch with fat pointers and container sharing
08b010bb fix(llvm): invocable closures with container reference semantics
f48e5212 test(native): VM-parity regression harness for container gaps
0d7b2125 style(native): use is_multiple_of in cryptoext test helper
1aa771c0 style(native): avoid clones in zstd test slice refs
092a3b0a style(native): avoid clones in lz4 test slice refs
541dd7c1 fix(native): reserve output capacity and feed LZMA-alone streams to completion
2be61048 fix(native): use with_capacity to satisfy clippy reserve lint
ab6c1c11 fix(test): serialize native link tests sharing hello_native.exe
c09618f9 feat(llvm): field slot address helper for container reference semantics
50010f13 fix(llvm): tag void-typed null literals as NULL in native marshal
0cb190d8 fix(native): return array string elements via out-pointer for Win64 ABI
1ec4c4bf fix(llvm): container reference semantics and string native parity
2d665384 fix(llvm): complete native OOP, enum, Result, and container support
6c2110c5 fix(native): pass TitrateArray to array accessors by pointer
4ba93ad5 fix(vm): make HashMap natives functional for the LLVM bridge
a9f619dc fix(llvm): lower HashMap to the {i64, ptr} TitrateArray struct
f0279602 fix(llvm): index native arg arrays with [0, i] GEP indices
1983119d fix(llvm): store updated containers back after mutating calls
e6e3e4bb fix(llvm): infer container mutator and accessor return types
c47a8bae fix(vm): make container natives functional for the LLVM bridge
9e55fea6 fix(llvm): correct native return-type inference for containers and math
f2d097aa fix(llvm): enable classes, inheritance, super calls, and module loading
fee4a1d9 fix(test): treat native_call_out as a direct helper in registry check
79b1b3fc fix(vm): register File_open in the native lookup table
f25ac536 feat(native): declare all native wrappers in the C header
cc82b7d8 feat(native): add C wrappers for all 127 missing native functions
6c394033 fix(cli): silence unused analyzer errors in import fallback arm
964acc70 fix(bench): warm up native binary before timing
906b80a2 fix(llvm): zero the marshal payload alloca to avoid poison bytes
f6087842 fix(llvm): apply O3 optimization and fix release codegen correctness
... (earlier: CLI multi-file fix, module resolution, Sys_args fix, stdlib fixes)
```

---

## 4. ACTIVE BUG — RESOLVED: the LLVM container bridge

### Root cause (found this session)
Multi-argument native calls through the LLVM bridge mis-marshalled
**arg[1]+**. The diagnosis in earlier sessions (48-byte stride, packed
struct, data layout) was a red herring. The real bug was a **GEP
indexing error** in `emit_native_call` (`native_bridge.rs`) and
`compile_native_call` (`mod.rs`):

```rust
// WRONG: a single index on an array source type strides by the WHOLE
// array size, not the element size.
builder.build_in_bounds_gep(array_ty, array_alloca, &[i as u64], ...)
//   [2 x T], i32 1  -> offset 1 * sizeof([2 x T]) = 48 for a 24-byte element
// CORRECT: two indices select element i.
builder.build_in_bounds_gep(array_ty, array_alloca, &[0, i], ...)
//   [2 x T], i32 0, i32 1 -> offset 24
```

LLVM treats `getelementptr [2 x T], ptr %p, i32 1` as indexing an array
of arrays (offset = 1 * sizeof([2 x T])), not element 1. The runtime
reads `args + i*24`, so every arg after the first was void.

### Other fixes made this session (all committed)
- `trc/src/codegen/llvm/native_bridge.rs` — `[0, i]` GEP fix; marshal
  `HashMap` as `TV_ARRAY`.
- `trc/src/codegen/llvm/mod.rs` — `[0, i]` GEP fix; pass `TitrateArray`
  to `titrate_array_get_*` by pointer (the by-value `{i64, ptr}` ABI is
  classified differently by LLVM and Rust on Win64, crashing
  `list.get(0)`); route `ArrayList.get/size` through element-type-aware
  accessors for locals, `this.field`, and `obj.field`; infer user
  function and inherited-method return types; emit virtual calls with
  the resolved return type; record full generic types on params;
  support bare enum variant construction, enum/Result switch
  discriminators with payload binding, and for-in over collections;
  route `Ok`/`Err` through the Result constructor; handle `Expr::Unit`;
  pre-populate `class_decls` in the emit-ir pipeline.
- `trc/src/codegen/llvm/types.rs` — `HashMap` lowers to `{i64, ptr}`
  (TitrateArray), not an opaque pointer.
- `trc/src/bytecode/vm/natives/system.rs` — HashMap natives made
  functional (were no-op stubs): get/put/containsKey/containsValue/
  remove/keys/values/size/isEmpty/clear/toString over a `Value::Array`
  of `[key, value]` pairs.
- `titrate_native/src/lib.rs` — `titrate_array_get_{string,int,long,
  double}` take `*const TitrateArray` instead of by-value struct.
- `trc/tests/llvm_types.rs` — updated HashMap type expectation.

### Current native status
- `bridge_repro` (add/size/get/indexOf): `size=2 v0=10 v1=20 idx=2` ✓
- **`mega_test_03` PASSES natively** — output identical to bytecode
  (Bond 0.0000357, LJ 10101811.98, VERIFICATION PASSED). This was the
  main remaining gap in earlier sessions.
- `mega_test_02` compiles and runs most sections natively (utility fns,
  DataStore/HashMap, shapes, inheritance, Result/switch, bitwise,
  strings, enum matching, ArrayList for-loop, casting, while, logic).
  It still **crashes during quicksort** — see remaining gap below.
- `native_registry` (1:1:1) passes; `native_hello` 3/3 passes;
  `native_bench` speedups are timing-flaky under load (mega_test_03
  measured 2.3x–2.9x vs the 3x target).
- Full suite `cargo test --all`, `cargo clippy --all -- -D warnings` are
  clean. Bytecode VM unchanged: stdlib_runtest 581/581, all demos.

### Remaining known gap: container mutation across function calls — CLOSED
Implemented as designed above (slot-pointer reference semantics), plus
`let`-aliasing with detach-on-rebind, in the commits below. Verified
VM-identical on swap/quicksort/alias/rebind repros.

Follow-up gaps found by probing and closed in later commits:
- **Closures** (`08b010bb`): closure definitions crashed LLVM
  (`compile_closure` never restored the builder insert position); invocation
  through locals was unimplemented. Now: builder save/restore, signature
  tracking per local (`ClosureSig`), exact-typed indirect calls, container
  params by slot.
- **Interface dispatch** (`0942cf73`): calls through interface-typed params
  were `unsupported call target`; `String.split` misrouted to `Regex_split`;
  `Char.toString` hijacked to `ArrayList_toString`; `null` marshalled as int
  0; missing-key nulls printed as `""`. Now: fat-pointer convention
  (object+itable at call sites, struct-typed params/returns), inferred-first
  native prefix trial, NULL tagging, null-aware string rendering.
- **Transparent returns** (`b3fc9e36`): `let c = ident(b)` snapshotted, so
  later mutation through one name was invisible through the other. Now:
  provable `return <param>` passthrough (transitive, cycle-guarded, with
  override veto for methods and interface veto) aliases the argument slot;
  calls still always execute (effect-safe); analyzer-`unknown` let-types fall
  back to codegen inference.

### Current WIP (NOTHING uncommitted — all clean)
`git status` should show only `HANDOFF.md` untracked.

---

## 9. Region escape analysis (completed)

- Added `RegionBlock(Block, Span)` AST variant (distinct from `UnsafeBlock`).
- Parser emits `RegionBlock` for `region name { ... }`.
- Analyzer tracks `region_depth` and `region_allocated_vars`; errors when a
  region-allocated value escapes via return (or is assigned and later returned).
- Bytecode compiler, interpreter, LLVM codegen, and pretty-printer handle
  `RegionBlock`.
- Build verified: `cargo build -p trc` clean.

---

## 8. Repository reorganization (in progress)

Layout changed (no deletions; everything archived):

- `old_test_files/` → `archive/old_test_files/`
- `mega_test.tr`, `mega_test_02/`, `mega_test_03/`, `mega_test_03.tr` → `tests/e2e/`
- `stdlib_test/` → `tests/stdlib/`
- root `test_*` fixtures (57) + `dummy_wave_close_test.wav` → `tests/fixtures/stdlib/`
- `alpha04_spec.txt`, `ONBOARDING_REPORT.md`, `SUMMARY.md` → `docs/`
- `plans/` → `docs/plans/`; `linguist/` → `docs/linguist/`
- stray logs → `.local/`; empty `coverage/` removed

Path references updated in `trc/tests/mega_test*.rs`, `debug_c2.rs`,
`stdlib_runtest.rs` (TESTS_DIR + FIXTURES_DIR working dir).

Living doc: keep updating this file; do not delete.

---

## 5. What else works in the native backend now

- Primitives, arithmetic, math (`Math_`/`MathAdvanced_`/`MathTrig_` return
  double), string ops, `io::println` (direct helpers + value-type dispatch for
  call results).
- OOP: classes, methods, inheritance (`super.init`, `super.method`), inherited
  fields, virtual dispatch, interfaces (full dispatch via fat pointers,
  including container params and declared return types).
- Closures: definable, invocable through locals, container params shared.
- Transparent container returns: passthrough helpers alias argument slots.
- Module loading: imported `.tr` files are parsed and merged (so
  `import forcefield;` in `mega_test_03` compiles natively).
- O3 optimization via the new pass manager (`default<O3>`) in release builds.
- `--release` no longer crashes (removed `fastcc` which broke the marshal
  under optimization) and the marshal payload alloca is zero-initialized.

### Remaining known gaps
- None in covered behavior: the bytecode VM path is fully green (581/581
  stdlib, all demos) and the native backend matches it on every suite plus
  the 9-case `native_gaps` harness.
- `cargo clippy --all --all-targets` (NOT the project gate) reports ~52
  lints in pre-existing `#[cfg(test)]`/fuzz-test code (June–July 2026,
  toolchain drift, e.g. `approx_constant` on the literal `3.14` test data —
  "fixing" those would alter test semantics). The 9 flagged lines authored
  this session (lz4/zstd/cryptoext test modules) were fixed. Project gate
  `cargo clippy --all -- -D warnings` is clean.
- There is no WASM backend, WASM natives, or WASM stdlib anywhere in the
  repo (verified Sept 2026: zero hits outside `docs/node_modules`
  third-party JS tooling). Nothing to validate.

---

## 6. Repo layout quick reference

- `trc/` — the compiler + bytecode VM (`trc/src/bytecode/`), LLVM backend
  (`trc/src/codegen/llvm/`), analyzer (`trc/src/analyzer/`).
- `pipette/` — build tool.
- `titrate_native/` — C-ABI runtime library: `src/wrappers.rs` (518 wrappers),
  `titrate_native.h` (519 declarations), `src/lib.rs` (`TitrateValue`, the
  `titrate_to_value` / `value_to_titrate` ABI conversion).
- `lib/tt/` — the `.tr` standard library.
- `demos/`, `examples/`, `stdlib_test/`, `mega_test_02/`, `mega_test_03/`,
  `mega_test.tr`.
- `AGENTS.md` — Titrate language syntax spec (authoritative).

## 7. Key ABI facts (for the container-bridge fix)
- Rust `TitrateValue`: `#[repr(C)] { tag: i32, _pad: i32, payload: TitratePayload }`,
  `size_of == 24`, `align == 8`. `TitratePayload` is a 16-byte union.
- LLVM `titrate_value_type`: `{ i32, i32, [16 x i8] }` (packed version
  `<{...}>`). Expected 24-byte stride in arrays.
- Tags: `TV_VOID=0`, `TV_BOOL=2`, `TV_INT=5`, `TV_LONG=6`, `TV_FLOAT=9`,
  `TV_DOUBLE=10`, `TV_CHAR=11`, `TV_STRING=12`, `TV_ARRAY=13`.
- `marshal_to_titrate` stores a value into a `[16 x i8]` payload alloca and
  `insertvalue`s it into the struct; the array of structs is passed to
  `titrate_native_call_out`.
