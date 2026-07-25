# CONTINUE.md — Titrate Bug Audit & Fix Progress

## Summary

- **Bytecode VM:** 0 failures (all 113 demos pass)
- **LLVM backend:** 28 failures (85/113 demos pass)
- **Unit tests:** 718/718 passing

## This Session's LLVM Fixes (7 commits)

### Commit 1: Forward-reference void return types
- Used `llvm_type_or_void` instead of `llvm_type` for forward-reference function return types

### Commit 2: Short-circuit i8→i1 coercion  
- Native bool functions return i8; short-circuit `&&`/`||` passed this to `build_conditional_branch` which requires i1
- Added i8→i1 coercion in `compile_short_circuit` (both left and right operands)
- **Fixed:** count_lines, ini_parser

### Commit 3: Instance method dispatch + struct comparisons + current_class_name
- Added native function dispatch for `MemberAccess` call targets (val.isObject(), data.isNull(), g.get(a).put(b,d), etc.)
- Fixed `infer_expr_type` for This, MemberAccess, and Call(MemberAccess) expressions
- Fixed Json.parse() type inference to return JsonValue instead of Variant
- Added final fallback struct handler for struct-vs-int/float/pointer comparisons
- Added `current_class_name` tracking for field store class resolution
- Added JsonValue instance method natives (isNull, isObject, etc.)

### Commit 4: Class compilation order + hasKey alias + revert broken import loading
- Moved `class_infos` insertion before method compilation so `this.field` stores work during method body compilation
- Added `hasKey` → `containsKey` method name alias for HashMap
- Reverted `collect_imported_declarations` (caused duplicate declaration errors when re-analyzing stdlib files)
- Fixed type mismatch in `resolved_method` match arms

### Commit 5: Struct comparisons + hasKey native + as-cast + module function lookup
- Added struct-vs-struct comparison handler (extract i64 fields for ordering)
- Added struct-vs-float comparison handler (extract i64, sitofp, compare)
- Added HashMap_hasKey as alias for HashMap_containsKey in native bridge
- Added HashMap_keys to native return type inference
- Extended compile_cast for int-to-int casts ('as' casts on non-pointers)
- Added module-level function lookup fallback in compile_call

---

## Remaining 28 LLVM Backend Failures

### Category 1: Chained method calls on containers (7 demos)
**Demos:** anagram_finder, ini_parser, levenshtein, matrix_demo, routing_demo, graph_bfs_dfs (this._adj.containsKey), flatten_json (val.keys)

**Root cause:** `g.get(a).put(b, d)` — the outer call's callee is `MemberAccess(Call(...), "put")`. The first `obj_type_name` resolution block (line 2350) handles `Call(inner_callee, ...)` but the inner callee is `MemberAccess(Identifier("g"), "get")`, not `Identifier("get")`.

**Fix needed:** In the first resolution block, add handling for `Call(MemberAccess(...), ...)` inner callees. Or better: use `self.infer_expr_type(obj)` for Call expressions directly.

### Category 2: hasKey/keys on non-typed variables (4 demos)
**Demos:** flatten_json, json_ql, todo_cli, validate_json

**Root cause:** Variables declared as `HashMap<string, string>` might not have `titrate_type` set to "HashMap" because the titrate_type uses the raw type name from the AST. The hasKey alias only works in the second resolution block.

**Fix needed:** Ensure `titrate_type` for HashMap variables returns "HashMap" (the base type name, not the full generic).

### Category 3: Struct vs struct comparisons (3 demos)
**Demos:** hex_dump, kmeans, (char_freq, find_duplicates, word_freq — "expected string struct, got IntValue")

**Root cause:** The struct-vs-struct handler was added but may not be reached because an earlier handler returns first, or the comparison operators don't match.

**Fix needed:** Debug why the handler isn't matching. May need to move it earlier in the chain.

### Category 4: Struct vs float comparison (1 demo)
**Demo:** pagerank_simple

**Root cause:** Similar to above — the struct-vs-float handler may not be reached.

### Category 5: LLVM module verification (3 demos)
**Demos:** math_eval, wc, url_decode

**Root cause:** Generated LLVM IR has type mismatches or invalid instructions.

**Fix needed:** Investigate the specific IR failures.

### Category 6: Class not found (4 demos)
**Demos:** archive_lister (ZipFile), email_validator (Regex), grep (Regex), regex_match (Regex)

**Root cause:** The LLVM backend doesn't compile imported stdlib classes (Regex, ZipFile). Reverting the `collect_imported_declarations` approach was necessary because re-analyzing stdlib files causes duplicate declarations.

**Fix needed:** Need a way to load class layouts from imported modules without re-running the analyzer. Could parse only class declarations, or cache class layouts from the analyzer phase.

### Category 7: Unknown variable JsonValue (2 demos)
**Demos:** json_merge, json_splitter

**Root cause:** `JsonValue` is used as a type in variable declarations but isn't a class that gets compiled in the LLVM backend.

**Fix needed:** Similar to Category 6 — need to load JsonValue class layout.

### Category 8: Function not found (1 demo)
**Demo:** binary_search_tree

**Root cause:** Top-level function `insertNode` called from class methods can't be found.

**Fix needed:** The module-level function lookup fallback was added but may not be working for functions compiled after the calling class.

### Category 9: Complex string concatenation (1 demo)
**Demo:** template_engine

**Root cause:** `compile_string_expr` doesn't handle deeply nested `Binary(Add, Binary(Add, ...), ...)` expressions.

**Fix needed:** Make compile_string_expr recursive for nested binary concatenations.

### Category 10: Function signature mismatch (1 demo)
**Demo:** json_stats

**Root cause:** A native function is called with wrong parameter types.

**Fix needed:** Debug which function and fix parameter type coercion.

---

## How to Continue

1. **Fix chained method calls** — The first `obj_type_name` resolution block needs to handle `Call(MemberAccess(...), ...)` by using `infer_expr_type` on the Call itself
2. **Fix titrate_type for generic containers** — Ensure HashMap/ArrayList variables have the correct base type name
3. **Fix struct comparison dispatch order** — Move the struct-vs-struct handler before handlers that return early
4. **Investigate LLVM verification failures** — Run with `--emit-ir` to see the generated IR
5. **Fix remaining categories** — each is a specific codegen fix

### Key Files
- `trc/src/codegen/llvm/mod.rs` — main codegen
- `trc/src/codegen/llvm/native_bridge.rs` — native function types
- `trc/src/codegen/llvm/types.rs` — LLVM type mapping
- `trc/src/bytecode/vm/natives/lookup.rs` — native function registry

### Testing
```bash
cargo test --lib                          # Unit tests
cargo build --release                     # Build
# Test LLVM demos
Get-ChildItem demos/*.tr | ForEach-Object {
    $out = & ./target/release/trc $_.FullName --native 2>&1 | Select-Object -First 1
    if ($out -match "error|assert") { "$($_.Name): $out" }
}
```
