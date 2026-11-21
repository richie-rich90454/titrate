# Contributing to Titrate

We are glad you want to contribute. Titrate is a young language, and every contribution, whether a bug fix, a new feature, better documentation, or a clearer error message, makes a real difference.

The authoritative contributor guide is [CONTRIBUTING.md](https://github.com/richie-rich90454/titrate/blob/main/CONTRIBUTING.md) in the repository root. This page mirrors its workflow for readers of the documentation site.

By participating you agree to the [Code of Conduct](https://github.com/richie-rich90454/titrate/blob/main/CODE_OF_CONDUCT.md). Do not open public issues for security vulnerabilities; use the private process in [SECURITY.md](https://github.com/richie-rich90454/titrate/blob/main/SECURITY.md).

## Prerequisites

- Rust 1.96 or later, installed with [rustup](https://rustup.rs).
- LLVM 22.1 development files for the native backend. The bytecode VM and the default test suite build without them. On Windows install LLVM 22.1 into `C:\Program Files\LLVM\`; on Linux install `llvm-22-dev` and `libclang-22-dev`; on macOS use `brew install llvm@22`.
- MSVC Build Tools on Windows (Visual Studio 2022 "Desktop development with C++" workload).

See [Building Titrate from source](./building-from-source.md) for platform-specific setup.

## Fork and branch

```bash
git clone https://github.com/YOUR_USERNAME/titrate.git
cd titrate
git remote add upstream https://github.com/richie-rich90454/titrate.git
git checkout -b my-feature
```

Use a descriptive branch name such as `fix-null-check` or `add-do-while`.

## Build

Run commands from the repository root. On Windows, always pass `--jobs 1`: parallel jobs can exhaust the page file (`os error 1455`). On Linux and macOS the flag is optional.

```bash
cargo build -p trc --jobs 1                  # compiler and bytecode VM
cargo build -p titrate_native --jobs 1       # native runtime; rebuild when the trc library changes
cargo build -p trc --release --jobs 1
```

## Test

These three commands are the green gates used by CI. All must pass before you open a pull request:

```bash
cargo build --all --jobs 1
cargo test --all --jobs 1
cargo clippy --all -- -D warnings
```

Focused targets for quicker iteration:

```bash
cargo test --lib                          # compiler and VM unit tests
cargo test --test stdlib_test             # standard library integration tests
cargo test --test mega_test               # end-to-end test
cargo test --test mega_test_02
cargo test --test mega_test_03
cargo test --test wasm_conformance        # plus the other wasm_* integration targets
```

The LLVM native tests are marked `#[ignore]` because they need LLVM development files, a system linker, and the release native runtime library. CI does not run them. To run them locally:

```bash
cargo build -p titrate_native --release --jobs 1
cargo test -p trc --test native_registry -- --ignored
cargo test -p trc --test native_gaps -- --ignored
cargo test -p trc --test native_mega_test -- --ignored
cargo test -p trc --test native_bench -- --ignored
```

`native_bench` is timing-based and can be noisy under load.

## Code style

### Rust

Format with `cargo fmt` and resolve every `cargo clippy --all -- -D warnings` warning. Use `Result<T, CompilerError>` instead of `unwrap()` or `expect()` on values that user input can affect. Do not leave placeholders, `TODO`, `FIXME`, `todo!()`, or `unimplemented!()` in shipped code. Add tests for new behavior.

### Titrate source

Write canonical Titrate, not the tolerated sugar forms, so the standard library demonstrates the recommended style:

- `name: Type` parameters and `fn name(params): ReturnType`.
- `let`, `var`, and `const` for declarations; `string` (lowercase), not `String`.
- `new ClassName()` for construction; `value as Type` for casts; `is` for type checks.
- `String.length(s)`, not `s.length()`; `MathAdvanced.sqrt(x)` and `MathTrig.sin(x)`, not `Math.sqrt(x)` or `Math.sin(x)`.
- `.` for module method calls (imports use `::`); `ok()`/`err()` for `Result`.
- No `static` keyword and no `Object` type: use a top-level `fn`, a generic, or `Variant`.
- One class per file, named after the class.

See `AGENTS.md` in the repository root for the full syntax specification.

### Language and formatting

Use American English everywhere (source strings, comments, documentation, and commit messages): "optimize", "color", "initialize", "behavior". Do not use emojis in files or commit messages.

## Adding a standard library test

1. Add `tests/stdlib/src/tests/<namespace>/test_<name>.tr`, following the one-test-file-per-module convention with `lib/tt/<namespace>/`.
2. Add `import tests::<namespace>::test_<name>;` to `tests/stdlib/src/main.tr` and call its `run_all_<name>_tests()` function inside `main()`. This aggregator is maintained by hand; a test that is not wired in will not run.
3. Put external reference data in `tests/fixtures/stdlib/` as JSON, CSV, or TXT. Do not hardcode reference values in `.tr` files.
4. Run `cargo test --test stdlib_test`, then `cargo test --all --jobs 1`.

## Running demos

```bash
./target/debug/trc.exe demos/foo.tr                  # Windows, bytecode VM
./target/debug/trc.exe demos/foo.tr --native --release
```

On Linux and macOS use `./target/debug/trc`. Skip `demos/file_watcher.tr`; it is an intentional infinite watcher.

## Commit conventions

- Stage and commit exactly one file per commit. When a fix spans several files, commit them one at a time in dependency order.
- Keep commits atomic and verify each with `git log -1`. Run `cargo test --all --jobs 1` after each fix.
- Write the first line in the imperative mood, with an optional body explaining *why*. Common prefixes are `fix:`, `feat:`, `docs:`, `refactor:`, `test:`, and `style:`, often with a scope such as `fix(llvm):`.

## Pull requests

1. Rebase onto the latest `main`: `git fetch upstream && git rebase upstream/main`.
2. Run the three green gates listed above.
3. Push your branch, open a pull request, and fill in the template.
4. Keep one feature or fix per pull request. Open a separate pull request for any extra improvements you find.
5. Respond to review feedback by pushing updates to the same branch.

Use the bug report, feature request, and stdlib bug issue templates for new issues.

## Documentation contributions

Fixing errors, adding examples, and improving clarity are all valuable. The docs are built with VitePress; preview them locally with:

```bash
cd docs
npm install
npm run docs:dev
```

## Thank you

Every contribution matters, whether it is a one-line fix or a major feature. Thanks for helping make Titrate better.
