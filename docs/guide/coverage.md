# Code Coverage

Titrate's compiler, VM, and native backend are written in Rust. This guide
explains how to measure test coverage over that Rust code, interpret the
results, and add tests to close coverage gaps.

Coverage is the foundation of Phase 6 of the native backend roadmap: before
optimising or extending the backend, we want a quantitative picture of which
lines the existing test suite exercises.

## Prerequisites

Coverage collection is delegated to an external Rust coverage tool. Install
**one** of the following:

### Option A — cargo-tarpaulin (recommended)

`tarpaulin` is the simplest path. It builds, runs, and reports in one step.

```bash
cargo install cargo-tarpaulin
```

### Option B — grcov

`grcov` consumes the `.profraw` files produced by rustc's built-in
`-Cinstrument-coverage` flag. It is more flexible but requires a little more
setup.

```bash
cargo install grcov
rustup component add llvm-tools-preview
```

## The coverage profile

The workspace `Cargo.toml` defines a `[profile.coverage]` that inherits from
`release` but keeps full debug info and disables optimisation:

```toml
[profile.coverage]
inherits = "release"
debug = true
opt-level = 0
```

Full debug info ensures source lines map cleanly to the reported coverage,
and `opt-level = 0` prevents the compiler from inlining or eliding the very
lines you are trying to measure. Always build coverage runs against this
profile.

## Running coverage

### With pipette

The build tool exposes a `coverage` subcommand that wraps the tooling for
you. From the workspace root:

```bash
pipette coverage
```

To include the native (LLVM) test binaries in the report:

```bash
pipette coverage --native
```

`pipette coverage` will:

1. Detect an installed coverage tool (`cargo-tarpaulin` preferred, `grcov`
   as a fallback).
2. Run the workspace test suite under that tool using the `coverage`
   profile.
3. Print a per-file summary table to the terminal.
4. Write a machine-readable baseline to `coverage-summary.txt` in the
   workspace root.

The `--native` flag additionally instruments the native test binaries (the
`native_*.rs` tests in `trc/tests/`) so that coverage of `titrate_native`
and the LLVM codegen path is included.

### With cargo-tarpaulin directly

```bash
cargo tarpaulin --workspace --profile coverage --out Html --output-dir coverage/
```

Open `coverage/tarpaulin-report.html` in a browser for a line-by-line view.

### With grcov directly (PowerShell)

```powershell
$env:RUSTFLAGS = "-Cinstrument-coverage"
$env:LLVM_PROFILE_FILE = "coverage/%p-%m.profraw"
cargo test --workspace --profile coverage

grcov coverage/ --binary-path target/coverage -s . -t html -o coverage/html
```

Open `coverage/html/index.html` in a browser.

On bash/zsh, use `export RUSTFLAGS=...` instead of `$env:RUSTFLAGS`.

## Interpreting the results

The per-file summary printed by `pipette coverage` looks like this:

```
file                                               covered      total          %
----------------------------------------------------------------------------------
trc/src/lexer/scanner.rs                              412         431     95.59%
trc/src/bytecode/vm/mod.rs                           1204        1587     75.86%
...
----------------------------------------------------------------------------------
TOTAL                                               8421       10234     82.29%
```

- **covered** — lines executed by at least one test.
- **total** — instrumented lines (excludes comments, blank lines, and
  non-code).
- **%** — `covered / total * 100`.

Files below 100% are listed in [Coverage Exceptions](./coverage-exceptions)
along with the reason and a tracking note.

### What counts as "covered"

The tools measure **line coverage** by default. Branch coverage is available
with tarpaulin's `--branch` flag or grcov's branch output, but the baseline
target for Phase 6 is line coverage.

## Generating a baseline

A baseline capture script lives at `scripts/coverage-baseline.ps1`. It runs
`pipette coverage` and stashes the resulting `coverage-summary.txt` so you
can diff future runs against it:

```powershell
./scripts/coverage-baseline.ps1
```

The first run establishes `coverage-baseline.txt`. Re-running the script
overwrites `coverage-summary.txt`; diff the two files to see drift:

```powershell
git diff --no-index coverage-baseline.txt coverage-summary.txt
```

## Adding tests to improve coverage

1. **Find a gap.** Open the HTML report from `coverage/` and look for red
   (uncovered) lines.
2. **Pick the right test home.**
   - Compiler/VM internals → a `#[test]` in `trc/src/.../tests.rs` or a new
     integration test in `trc/tests/`.
   - End-to-end Titrate behaviour → a new `_test.tr` file or `test_*`
     function in `stdlib_test/`.
3. **Write the test** following the patterns in the existing test files.
   The `assay` module (`lib/tt/assay/`) provides assertions for Titrate-side
   tests.
4. **Re-run coverage** and confirm the gap closed.

## Current baseline state

The baseline is generated on demand because the coverage tools are not part
of the regular CI image. As of Phase 6 Task 6.1, the workspace builds
cleanly with **744 passing tests and 0 warnings**. The first `pipette
coverage` run after installing a tool will produce the numeric baseline;
record it in [Coverage Exceptions](./coverage-exceptions) and update the
table as coverage improves.

## See also

- [Coverage Exceptions](./coverage-exceptions) — files below 100% and why.
- [Build Tool](./build-tool) — the `pipette` command reference.
- [Compiler Architecture](./architecture) — what the instrumented code does.
