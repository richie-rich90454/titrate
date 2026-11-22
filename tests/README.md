# Titrate Tests

## Layout

- `e2e/` — end-to-end `.tr` programs (`mega_test.tr`, `mega_test_02/`, `mega_test_03/`).
- `stdlib/` — the `stdlib_test` project (Cargo integration test `stdlib_test`).
  - `src/tests/` — per-module `.tr` test files.
  - `src/main.tr`, `src/suites_b.tr` — aggregators.
- `fixtures/stdlib/` — binary/text fixtures referenced by stdlib tests (working dir).
- `expected/` — expected outputs for e2e tests.

## Running

From `trc/`:

```bash
cargo test --lib                      # compiler/VM unit tests
cargo test --test stdlib_test         # stdlib integration tests
cargo test --test mega_test           # e2e mega test
cargo test --test mega_test_02
cargo test --test mega_test_03
cargo test --test native_registry -- --ignored
cargo test --test native_gaps -- --ignored
```

Fixtures are loaded with working directory `tests/fixtures/stdlib` (set by the harness).

## Archive

Historical diagnostics live in `archive/` (nothing is deleted).
