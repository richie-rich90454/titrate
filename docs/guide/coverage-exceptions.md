# Coverage Exceptions

This page tracks source files whose line coverage is **below 100%** and the
reason each gap is accepted (for now) or scheduled to be closed.

The table is regenerated whenever `pipette coverage` (or
`scripts/coverage-baseline.ps1`) is run. When you close a gap, move the row
to the "Closed gaps" section with the PR that fixed it.

## Coverage tooling status (2026-07-02)

**Status: BLOCKED on Windows.** Per Task E.1.4 of the
`verify-alpha-0.4-completion` spec, the baseline coverage run was attempted
on Windows with both supported tools and the manual LLVM-native path. All
three approaches failed due to a Rust 1.93.0 (254b59607 2026-01-19) std
library bug on Windows where `std::process::Command::output()` panics with
`Os { code: 0, kind: Uncategorized, message: "操作成功完成" }` ("The
operation completed successfully") inside proc-macro build scripts.

### Attempted tools and failure modes

| Tool | Command | Failure |
|------|---------|---------|
| `cargo-tarpaulin` v0.36.0 | `cargo install cargo-tarpaulin` | Build script of `getrandom v0.3.4` panicked with `Os { code: 0, kind: Uncategorized }`. Exit code 101. |
| `grcov` | `cargo install grcov` | Install did not complete within 30+ minutes (the same `quote v1.0.45` build script panic affects the dependency tree). Killed. |
| Manual LLVM-native | `RUSTFLAGS="-Cinstrument-coverage" cargo test --lib -p pipette --profile coverage` | Build script of `quote v1.0.45` panicked with the same `Os { code: 0 }` error. Exit code 101. |

System LLVM 23.0.0git tools (`llvm-profdata.exe`, `llvm-cov.exe`) are
available at `C:\Program Files\LLVM\bin\` and would be used to merge and
report `.profraw` files if they were produced. The `rustup component add
llvm-tools-preview` step succeeded; the Rust toolchain itself is the source
of the build script panic.

### Recommendation

Rerun `pipette coverage` on Linux or macOS where the Rust std library
`Command::output()` panic does not occur. On Windows, wait for a Rust
toolchain fix (the bug is tracked in `rust-lang/rust` for the 1.93.0 stable
channel on Windows).

Until then, the entire workspace Rust source tree (107 files across
`trc/src/`, `pipette/src/`, and `titrate_native/src/`) is pending coverage
measurement. The `coverage-summary.txt` file at the workspace root
documents the failure mode in detail.

## Active exceptions

| File | Covered | Total | % | Reason | Tracking |
|------|--------:|------:|--:|--------|----------|
| All Rust source files (`trc/src/**/*.rs`, `pipette/src/**/*.rs`, `titrate_native/src/**/*.rs`) | — | — | — | Coverage tooling unavailable on Windows (Rust 1.93.0 std::process::Command panic in proc-macro build scripts) | verify-alpha-0.4-completion Task E.1.4 |
| `lib/tt/**/*.tr` (414 stdlib source files) | — | — | — | Titrate-language source files; coverage tooling only instruments Rust code. `.tr` coverage is measured indirectly via the stdlib_runtest harness (Task A.4), which currently shows 10/444 passing due to pre-existing bugs in the test files (not the harness) | verify-alpha-0.4-completion Task A.4 |

> **Format:** one row per file. `Reason` is a short phrase such as
> `error-path only`, `unreachable defensive code`, `platform-gated`, or
> `todo: add tests`. `Tracking` is an issue number or PR link.

## Closed gaps

| File | PR | Notes |
|------|----|-------|
| _(none yet)_ | — | — |

## Acceptance policy

- **100% is the target.** Every line of compiler, VM, and native-backend
  Rust code should be exercised by a test.
- **Defensive `unreachable!` / panic paths** may be listed here with the
  reason `unreachable defensive code` rather than tested, but they must be
  genuinely unreachable — a `debug_assert!` or comment must justify it.
- **Platform-gated code** (e.g. Windows-only syscall shims) is listed with
  reason `platform-gated` and covered on the relevant platform.
- **Anything else** is a `todo: add tests` and should have an issue.

## Regenerating this table

```bash
pipette coverage
# then transcribe the below-100% rows from coverage-summary.txt
```

See [Code Coverage](./coverage) for the full workflow.
