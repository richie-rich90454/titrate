# Phase 7 DX Roadmap: Diagnostics, LSP, Formatter, Linter, REPL and Watch

**Status:** design document with one implemented foundation (structured JSON diagnostics).
**Branch:** `feat/full-productization-g0`.
**Source of truth:** `ROADMAP.md` section 13 ("Phase 7 — Performance: startup, compile latency, DX").
**Audience:** compiler and tooling contributors.

This document turns the one-line Phase 7 DX bullet into concrete, testable
milestones. It records what exists today, what was built alongside this plan,
and what still needs to be implemented. Items marked **DESIGN ONLY** are not in
the codebase; items marked **IMPLEMENTED** have landed.

---

## 1. Goals and non-goals

### Goals

- Machine-readable diagnostics that tools can consume without scraping text.
- An LSP server giving editors completion, go-to-definition, hover types,
  inline hints, and quick fixes.
- A deterministic formatter (`trc fmt`) and a linter (`trc lint`).
- Interactive evaluation (`trc --eval`) and file watching (`trc --watch`).
- Every DX feature covered by tests, with latency targets measured rather than
  guessed (the Phase 7 "Done when" clause).

### Non-goals

- No DAP debugger or time-travel in this plan; that remains a later Phase 7
  item and a separate document.
- No `--profile` flamegraphs, `pipette` lockfile/SBOM, tree-sitter grammar, or
  deterministic-build work here; they are Phase 7 items but outside DX tooling.
- No new Cargo dependencies unless a milestone explicitly justifies one. The
  diagnostics foundation deliberately has none.

---

## 2. Guiding principles

1. **One diagnostic pipeline.** Lexer, parser, analyzer, and codegen all feed
   the same `Diagnostic` value. Human and machine renderers are two views of
   that value, never two independent code paths.
2. **Stable contracts.** Error codes and the JSON key order are public
   contracts. Adding a key is a versioned change; reordering or removing one
   is a breaking change.
3. **Reuse before building.** The stable code catalog (`trc/src/errors.rs`),
   the `CompileError`/`Suggestion` types (`trc/src/analyzer/errors.rs`), and the
   AST/parser already contain most of what the tools need. Tools wrap the
   compiler; they do not reimplement it.
4. **No dependency for a few lines.** JSON escaping, simple lexical analysis
   for formatting, and file watching are written against the standard library
   unless a milestone proves a crate is necessary.
5. **Editor-first risk order.** Ship the data format editors read first
   (diagnostics), then the transport (LSP), then convenience tools (fmt/lint),
   then interactive modes (REPL/watch).

---

## 3. Foundation that already exists

### 3.1 Stable error codes (`trc/src/errors.rs`)

- Central catalog: `E0xxx` lexical, `E1xxx` parse, `E2xxx` semantic, `E3xxx`
  codegen, with per-variant constants and `*_UNKNOWN` fallbacks.
- Classifiers: `classify_lexer_error`, `classify_parse_error`,
  `classify_semantic_error` map free-form messages to stable codes.
- Location helpers: `parse_location_suffix`, `strip_location_suffix` recover
  ` at L:C` suffixes.
- Human renderer: `render_error` produces the canonical
  `error[E0XXX]: <message>` block with caret gutter.
- CLI integration: `report_error` and `report_semantic_error` in
  `trc/src/main.rs` already split `CompileError`'s `Display` output into a
  primary message plus `  help: ...` suggestion lines before rendering.

### 3.2 Error types (`trc/src/analyzer/errors.rs`)

- `CompileError { message: String, suggestions: Vec<Suggestion> }`.
- `Suggestion { message: String, replacement: Option<String> }` — the
  `replacement` field is the seed for machine-applicable fixes.
- `Display` renders suggestions as `\n  help: <hint>` lines, which is why the
  machine-readable layer must parse that shape when it receives strings.

### 3.3 Implemented in this change: `trc/src/diagnostics.rs`

**IMPLEMENTED.** A dependency-free, machine-readable serializer:

- `Span { line, column }` — 1-based source position.
- `DiagnosticSuggestion { message, replacement: Option<String> }` — a hint
  with an optional machine-applicable edit.
- `Diagnostic { code, message, line, column, suggestions }` — the canonical
  machine-readable value.
- Constructors:
  - `Diagnostic::new(code, message)` — explicit code, no span/suggestions.
  - `Diagnostic::from_display(err, span)` — takes a `CompileError`-style
    message; parses `\n  help:` lines into suggestions, recovers coordinates
    from a trailing ` at L:C` when `span` is `None` (an explicit span wins and
    the suffix is still stripped), and classifies the code.
  - `Diagnostic::from_compile_error(err, span)` — maps an analyzer
    `CompileError` directly, preserving structured `replacement` edits.
- Serialization: `Diagnostic::write_json` / `Diagnostic::to_json` and
  `to_json_array(&[Diagnostic])`. Hand-written JSON string escaping covers
  quotes, backslashes, `\b \f \n \r \t`, all `U+0000..U+001F` control chars, and
  passes UTF-8 through. Key order and `null` handling are fixed.
- Best-effort classification falls back semantic -> lexer -> parser so tools
  can serialize errors from any compiler phase. Callers needing an exact code
  can override the public `code` field.
- Unit tests cover key order, null coordinates, escaping, `help:` parsing,
  span override, structured replacements, null replacements, JSON arrays,
  appends, determinism, and parse-code classification.

`trc/src/lib.rs` exports the module with a single `pub mod diagnostics;` line.

JSON shape (contract, key order is significant):

```json
{"code":"E2000","message":"undeclared identifier: 'cont'","line":4,"column":9,"suggestions":[{"message":"did you mean 'count'?","replacement":"count"}]}
```

---

## 4. Deliverable B — Language Server (`trc-lsp`)

**DESIGN ONLY.** A separate binary crate under `trc/lsp/` (or a `trc lsp`
subcommand) that speaks LSP over stdio and wraps the existing lexer, parser,
and analyzer. It never holds compiler state in the editor; each request
re-parses the in-memory document, which is fast enough at Titrate's current
program sizes. A content-addressed TIR cache from the Phase 7 startup work can
later be reused for large workspaces.

### 4.1 Architecture

- **Transport:** JSON-RPC 2.0 over stdio (`Content-Length` framing), no async
  runtime dependency for v1 — synchronous request/response is sufficient.
- **Document store:** a `HashMap<Url, String>` of open documents, updated by
  `textDocument/didOpen`, `didChange`, `didClose`. Full-document sync for v1;
  incremental sync is a later optimization.
- **Compilation facade:** a thin `compile_document(&str) -> Compilation`
  returning `{ diagnostics: Vec<Diagnostic>, program: Option<ast::Program>,
  symbols: SymbolTable }`. It runs `tokenize`, `parse`, and `analyze`, mapping
  each error through `diagnostics::Diagnostic` and publishing
  `textDocument/publishDiagnostics` with the JSON shape from section 3.3.
- **Symbol table:** built by walking `ast::Program` (declarations, fields,
  methods, parameters, locals). This is the single source for completion,
  go-to-definition, and hover.

### 4.2 Capabilities by milestone

| Milestone | Capability | LSP methods | Done when |
|---|---|---|---|
| L1 | Diagnostics + lifecycle | `initialize`, `didOpen/Change/Close`, `publishDiagnostics` | Editing a `.tr` file shows errors with correct code, line, column, and help text; tests drive a fake client and assert published payloads. |
| L2 | Completion + hover | `textDocument/completion`, `textDocument/hover` | Completion returns symbols/keywords valid at the cursor; hover shows inferred types from the analyzer; tests cover locals, fields, and module functions. |
| L3 | Go-to-definition + references | `definition`, `references`, `documentSymbol` | Jumping to a function, class, or local lands on its declaration; tests assert exact ranges. |
| L4 | Inline hints + quick fixes | `inlayHint`, `codeAction` | Analyzer-provided `Suggestion.replacement` becomes a workspace edit; tests assert the edit's text and range. |

### 4.3 Notes on L4 quick fixes

The diagnostic JSON already carries `replacement`. The server additionally
needs the source range to replace, which v1 derives from the diagnostic's
`line`/`column` plus the token length at that position (from the lexer token
stream). If replacement ranges prove ambiguous, the diagnostic schema gains an
optional `range` key in a versioned schema bump rather than an ad-hoc field.

---

## 5. Deliverable C — Formatter (`trc fmt`)

**DESIGN ONLY.** A deterministic, AST-free pretty-printer operating on the
token stream so formatting never destroys comments or succeeds on invalid
syntax.

- Input: source text. Output: formatted source text plus a `--check` mode that
  exits non-zero and prints a diff when formatting would change the file.
- Rules: 4-space indentation, one statement per line, spaces around binary
  operators, `fn name(a: T): R` spacing per `AGENTS.md`, braces on the same
  line, trailing newline, no tabs.
- Safety: token-anchored rewriting preserves raw strings, block comments, and
  numeric underscores byte-for-byte. Idempotence is a hard invariant:
  `fmt(fmt(x)) == fmt(x)`.
- Milestones:
  - **F1** token infra + `--check` diff on a curated fixture set.
  - **F2** formatting rules for declarations, control flow, and expressions.
  - **F3** comment/raw-string preservation and idempotence fuzz test over the
    `examples/` and `lib/tt/` corpora.
  - **F4** expose as an LSP `textDocument/formatting` handler in L4.
- Done when: `trc fmt --check examples lib/tt` is clean on formatted fixtures,
  and the idempotence test passes over the whole corpus.

---

## 6. Deliverable D — Linter (`trc lint`)

**DESIGN ONLY.** A rule engine that consumes the analyzer's warning stream and
adds style/correctness rules without re-parsing.

- Reuse: `analyzer::analyze_with_mode_and_warnings` already emits warnings;
  `CompileError.suggestions` provides fix text. Lint rules wrap the same
  `Diagnostic` serializer, using a distinct code family (proposed `W4xxx` for
  warnings, added to `trc/src/errors.rs` in a versioned change).
- Rule set v1: unused variable, unused import, unreachable code (the analyzer
  already tracks `after_terminator`), naming convention checks from
  `AGENTS.md` section 15, and deprecated-pattern checks (raw generic types,
  `Math.sqrt` instead of `MathAdvanced.sqrt`).
- CLI: `trc lint [files...] [--json] [--deny <rule>]`; `--json` reuses
  `to_json_array`.
- Milestones:
  - **N1** emit warnings as `Diagnostic` values with `--json`.
  - **N2** first rule set with unit tests per rule and a fixture containing
    known violations.
  - **N3** severity config and `--deny` exit codes; LSP publishes the same
    diagnostics with a `warning` severity.
- Done when: each rule has a positive and negative test, and lint output uses
  the same JSON schema as compiler diagnostics.

---

## 7. Deliverable E — REPL and watch

**DESIGN ONLY.**

### 7.1 REPL (`trc --eval`)

- Read-eval-print over the existing bytecode VM: each line is tokenized,
  parsed, analyzed, compiled, and run in a persistent environment so
  declarations survive across lines.
- Multiline input via brace depth and raw-string state; `Ctrl-D` exits.
- Diagnostics print through the human renderer in interactive mode and JSON
  when `--diagnostics-format json` is set.
- Milestones:
  - **R1** single-expression evaluation with persistent environment and tests
    asserting values across lines.
  - **R2** declarations, functions, and multiline blocks.
  - **R3** editor niceties: prompt, history file, `help`/`clear` commands.
- Done when: a scripted session test covers state persistence and an error
  followed by a successful recovery.

### 7.2 Watch (`trc --watch <file>`)

- Run once, then watch the file (and its `import` graph) for changes and rerun
  on save, coalescing bursts.
- Implementation uses `std::fs::metadata` polling with a configurable interval
  to avoid a filesystem-watching dependency; swap in a native watcher later if
  latency demands it.
- On rebuild, publish diagnostics through the same pipeline and re-render only
  changed output.
- Milestones:
  - **W1** single-file polling rerun with debounce and a test using temp files.
  - **W2** import-graph tracking.
  - **W3** `--watch --eval` compound mode and clear-screen behavior.
- Done when: a test writes a file, observes a rerun, edits it, and observes a
  second rerun with updated diagnostics.

---

## 8. Cross-cutting CLI contract (DESIGN ONLY)

- New flags: `--diagnostics-format {human,json}` (default `human`),
  `--eval`, `--watch`, `fmt`, `lint`, `lsp` subcommands.
- Exit codes: `0` success, `1` compile/lint error, `2` usage error. `--check`
  formatter failures use `1`.
- All JSON output goes to stdout; human diagnostics stay on stderr. This keeps
  `--diagnostics-format json` pipeable.
- `--diagnostics-format json` must produce only the JSON array on stdout —
  no banners, no progress text — so CI and editors can parse it directly.
- Wiring `diagnostics::Diagnostic` into `main.rs` (replacing the `report_error`
  / `report_semantic_error` text path when JSON is selected) is **DESIGN ONLY**
  and intentionally not in `main.rs` for this change, since that file was not
  assigned to this implementation task.

---

## 9. Consolidated milestone table

| ID | Deliverable | Depends on | Status |
|---|---|---|---|
| M0 | Stable code catalog + human renderer | — | existing |
| M1 | `diagnostics.rs` JSON serializer + tests | M0 | **IMPLEMENTED** |
| M2 | CLI `--diagnostics-format json` wiring | M1 | design only |
| L1–L4 | LSP diagnostics, completion/hover, definitions, hints/fixes | M1, M2 | design only |
| F1–F4 | Formatter and `trc fmt --check` | M0 | design only |
| N1–N3 | Linter and `trc lint --json` | M1 | design only |
| R1–R3 | REPL `trc --eval` | M0 | design only |
| W1–W3 | Watch `trc --watch` | M0 | design only |
| V | Verify all features with tests; measure latency targets | all | design only |

Recommended order: M2 -> L1 -> L2/L3 -> F1/F2 -> N1/N2 -> L4 -> R1/R2 -> W1/W2
-> V, with F3/N3/R3/W3 as polish slots.

---

## 10. Testing strategy

- Diagnostics: unit tests on exact JSON strings and escaping (done in M1).
- LSP: an in-process fake client drives requests and asserts JSON-RPC responses;
  no editor needed in CI.
- Formatter: golden fixtures plus an idempotence test over `examples/` and
  `lib/tt/`.
- Linter: one positive and one negative test per rule, sharing fixture source.
- REPL/watch: scripted I/O and temp-file tests with short polling intervals.
- Verification milestone V runs the full suite and records startup, incremental
  compile, and cold compile timings against the Phase 7 targets (<50 ms first
  output, <2 s incremental, <15 s cold).

---

## 11. Implementation status summary

| Item | Status | Location |
|---|---|---|
| Stable error codes, classifiers, human renderer | existing | `trc/src/errors.rs` |
| `CompileError` / `Suggestion` | existing | `trc/src/analyzer/errors.rs` |
| Machine-readable JSON diagnostics + tests | **IMPLEMENTED** | `trc/src/diagnostics.rs` |
| Module export | **IMPLEMENTED** | `trc/src/lib.rs` (one line) |
| CLI JSON flag and wiring | design only | — |
| LSP server | design only | — |
| Formatter | design only | — |
| Linter | design only | — |
| REPL / watch | design only | — |

---

## 12. Open questions and risks

- **Replacement ranges.** Quick fixes need a byte range, which the current
  diagnostic schema does not carry. Decide whether to add a `range` key (schema
  version bump) or derive it from lexer tokens in the LSP layer.
- **Warning code family.** Confirm `W4xxx` conventions before adding lint codes
  to the shared catalog to avoid collisions with future `E3xxx` codegen codes.
- **Watch scope.** Polling the whole import graph may be costly; measure before
  committing to a native watcher dependency.
- **REPL persistence.** The bytecode VM's environment lifetime must be checked
  before promising cross-line declarations; if it cannot be kept alive, the
  REPL recompiles all prior lines each step (correct but slower).
- **Schema stability.** Freeze the JSON shape in M2 so LSP/CI consumers are not
  broken by later key additions.
