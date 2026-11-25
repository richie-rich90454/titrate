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
- Slicing: `a[start:stop:step]` for strings and arrays.
- FP: PersistentList, PersistentMap, LazySeq with take/rangeSeq added to util.
- FP: Pipe operator, HOFs (map/filter/reduce) available across collections.
- Zlib/gzip compression now functional (was stubbed).
- OS primitives (pty, termios, fcntl, resource, tty) are platform-limited with documented synthetic implementations.
- TCO: implemented for direct calls in tail position on both the bytecode VM (frame-reusing `TAIL_CALL` opcode) and the LLVM native backend (`tail` call lowering). Virtual/indirect/native calls and returns inside cleanup scopes are excluded.
- Test suite (stdlib_runtest) now runs files in parallel across available cores.
- Native tail calls are now trampolined at IR level for self-recursion, so deep recursion is stack-safe even in unoptimized `--native` builds.
- WASM backend: full language coverage including collections, enums/Result, closures, HOF callbacks, error handling, tuples, OOP, generics, modules, and File/JSON/regex/OS host bindings.

### Standard library
- DES crypt, bcrypt crypt, and FTP active mode are fully implemented as VM natives with native-backend bridges.
- Fixed reproduced correctness bugs across math/science (17), crypto/encoding (13), networking/serialization (22), collections (12), and string/I/O/regex (12).
- All six HMAC variants are RFC-correct for arbitrary byte keys and messages.
- Large-domain math functions (`tanh`, `asinh`, `acosh`, `erfc`) no longer overflow or cancel incorrectly.

### Verification
- Verified baseline (2026-11-25): `cargo build --all --jobs 1`, `cargo test --all --jobs 1` (1,527 passed, 0 failed across all binaries), and `cargo clippy --all --jobs 1 -- -D warnings` pass. Native stdlib parity remains a report-only known gap at 207/598.
- Bytecode/native parity: 123 of 123 sample programs produce byte-identical output.
- WASM tests: 89 passing tests across 16 integration test files; most compare generated output against the bytecode VM, while three use hardcoded expectations where the VM rejects the source construct (string ordering, substring, tuple indexing). Only stdout is compared and the host shim emulates some natives with fixtures.

### Documentation
- Updated README, tests/README, AGENTS.md spec, HANDOFF.
- Added CONTRIBUTING, CODE_OF_CONDUCT, SECURITY, CITATION, issue/PR templates, dependabot, and a trustworthy CI workflow.
- Added a headless demo gallery: generated SVG screenshots of real program output plus a repository stats card, with a `--check` mode enforcing freshness in CI.

### Pending
- Full C++/Python parity (out of scope for the language implementation).
