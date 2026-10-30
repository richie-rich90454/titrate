# Changelog

## V1 (in progress)

### Repository
- Reorganized layout: tests under `tests/`, diagnostics under `archive/` and `.local/`.
- No files deleted; everything archived.

### Language
- Added pipe operator `|>` (desugars to function call).
- Region blocks are now a distinct AST node; region-allocated values are tracked and cannot escape their region via return.
- String interpolation `${expr}` in string literals (desugars to concatenation).
- First-class range values: `a..b` / `a..=b` evaluate to tuples; `in` operator for membership.
- Test suite (stdlib_runtest) now runs files in parallel across available cores.

### Documentation
- Updated README, tests/README, AGENTS.md spec, HANDOFF.

### Pending
- String interpolation, first-class ranges, FP paradigms, WASM backend, memory-safety completion, full C++/Python parity.
