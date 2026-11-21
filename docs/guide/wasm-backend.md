# WebAssembly backend

The `wasm` backend lowers a typed Titrate AST to a WebAssembly module. It is
exposed as `trc::codegen::wasm::compile_to_wasm(&Program) -> Result<Vec<u8>, String>`;
the returned bytes are a complete `.wasm` binary and export `main`, `memory`
and `heap_ptr`.

The backend now covers the feature set exercised by the parity tests below.
Remaining limitations are listed in
[Limitations](#limitations); constructs outside the supported set are rejected
with an explicit `wasm: ...` error rather than silently miscompiled.

## Building

```bash
cargo build -p trc --jobs 1
```

The encoder crate is `wasm-encoder`. The test runner uses `wasmi` as a dev
dependency.

## Using a generated module

The module is not self-contained: runtime facilities are imported from the
`env` module. A host must supply:

- a linear memory export named `memory`,
- a mutable i32 global export named `heap_ptr`, used as a bump pointer for
  dynamically produced strings,
- the functions listed in `trc/src/codegen/wasm/runtime.rs`.

Strings are represented as an i32 pointer to `[len: i32][UTF-8 bytes...]`.
String literals are placed in data segments at low addresses; the host starts
allocating at `heap_ptr` (65,536 by default), so literal data is never
overwritten.

Import groups:

- output: `print`, `print_raw`, `print_i64`, `print_f64`, `print_bool`,
  `print_char`
- strings: `concat`, `str_eq`, `str_len`, `str_char_at`, `str_substring`,
  `str_index_of`, `str_upper`, `str_lower`, `str_trim`, `str_starts_with`,
  `str_ends_with`
- conversion: `to_string_i64`, `to_string_f64`, `to_string_bool`,
  `to_string_char`, `parse_int`, `parse_double`
- math: `fmod`, `math_*`
- objects: `obj_new`, `obj_named_new`, `field_set_*`, `field_get_*`, `obj_class`,
  `obj_display_named`
- collections: `list_*`, `map_*`
- composites: `enum_*`, `result_*`, `tuple_*`, `closure_new`
- errors: `err_set`, `err_get_*`, `err_flag`, `err_clear`
- file: `file_read`, `file_write`, `file_append`, `file_exists`, `file_delete`
- JSON: `json_stringify_*`
- regex: `regex_match`, `regex_find`, `regex_replace`
- OS read-only queries: `os_name`, `os_arch`, `os_family`, `os_cpu_count`
- iterators: `iterator_new`, `iterator_collect`
- diagnostics: `wasm_unsupported` (traps on a construct that cannot be
  represented faithfully at run time)

A working host implementation is in `trc/tests/wasm_common/mod.rs`; it runs
the generated module and captures printed lines.

## Value mapping

| Titrate type | WASM representation |
|---|---|
| `byte`/`short`/`int`/`long`/`u*`/`size`/`vast` | `i64` |
| `float`/`double`/`half`/`quad` | `f64` |
| `bool` | `i32` (0 or 1) |
| `char` | `i32` (Unicode scalar) |
| `string` | `i32` pointer |
| class / interface / enum / collection / tuple / Result / closure | `i32` host object handle |

Composite values are host objects. Collection and composite storage uses a
four-word value encoding `(tag: i32, i64, f64, ptr: i32)`, with tags defined in
`runtime.rs`. Arithmetic, comparison, bitwise operations, control flow and
direct function calls are native WebAssembly; only operations needing host
state go through imports.

## Supported constructs

- primitive literals, `let`/`var`/`const`, assignment, arithmetic, comparison,
  logical/bitwise/unary operators, casts
- strings, `+` concatenation and the `String.*` helpers
- `if`/`else`, `while`, `do`/`while`, `while let` (Result payload binding),
  C-style `for`, `break`, `continue`, ternary, `switch` on literals and on
  enum/Result constructor patterns with payload binding
- `with (resource) { ... }` bindings; the resource is bound in the body and
  its `close()` method is called afterwards
- `for (x in collection)` over ArrayList, HashMap and string
- ArrayList (`add`, `get`, `set`, `size`, `length`, `contains`, `indexOf`,
  `lastIndexOf`, `remove`, `removeAt`, `pop`, `addAll`, `clear`, `isEmpty`,
  `reverse`, `max`, `min`, `toString`) and HashMap (`put`, `get`, `remove`,
  `size`, `containsKey`, `containsValue`, `clear`, `isEmpty`, `toString`,
  `keys`, `values`) with string or integer keys
- enums (unit and payload variants), `is`, `Result`/`Ok`/`Err`, `unwrap`,
  `isOk`/`isErr` and the `?` propagation operator
- tuples, tuple display and tuple destructuring
- closures with parameters, block or arrow bodies, and captured variables;
  calls use a per-signature WASM table and `call_indirect`
- closure-valued ArrayList methods (`forEach`, `filter`, `map`, `reduce`) are
  lowered inline to a WASM loop that drives `call_indirect` on the captured
  closure handle; captured variables work in the callback
- `ArrayList.iterator()` plus `hasNext`, `next`, `count`, `collect` and `reset`
  (element types limited to the scalar types listed in Value mapping)
- `io::println` of handle-backed values (collections, enums, `Result`, tuples
  and classes) using the VM's display form
- `tt.regex.Regex.match`, `find` and `replace` via host bindings
- `tt.sys.Os.name`, `arch`, `family` and `cpuCount` via host bindings
- `throw`/`try`/`catch` with a host error cell
- classes, constructors, fields, methods, `this`; interfaces; inheritance with
  inherited fields/methods, `super` calls, and vtbl-based virtual dispatch
- operator overloading (`operator+`, `-`, `*`, `/`, `%`)
- generic top-level functions via call-site monomorphization, including
  type parameters inferred from the expected type at the call site
- generic classes specialized by their concrete type arguments (mangled as
  `Box__int`, matching bytecode VM display)
- imported stdlib modules whose facilities map to existing host imports
  (`tt.math.*`, `tt.util.ArrayList`) — module resolution is performed by the
  bytecode side when verifying parity
- `Owned`, `region`, `unsafe` and references are lowered as pass-through
  expressions (no observational effect on printed output)

## Limitations

The following constructs remain unsupported. Each still fails with an explicit
`wasm: ...` diagnostic at code generation (or is excluded from parity testing
for the reason given), rather than being silently miscompiled.

- `with` general forms and `while let` general forms. The bytecode VM itself
  rejects `with` on a non-resource (`INVOKE_VIRTUAL: cannot invoke 'close' on
  42L`) and rejects `while let` on a non-`Result`. The supported shapes (class
  resource with `close()`, `Result`-valued `while let`) are lowered as before;
  other shapes produce an explicit error.
- Virtual dispatch still uses one table per method name, so overloads with
  different signatures under the same method name are not distinguished, and
  interface default methods are only reachable when a concrete implementing
  class provides the method. An experiment to alias interface defaults was
  reverted because it did not pass parity.
- Ownership, regions and unsafe blocks do not model lifetimes or arena
  reclamation. Reference display cannot be parity-tested: the bytecode VM
  rejects `io::println(r)` and `Json.stringify(r)` on a reference with
  `Cannot cast ref(0) to int`, so there is no VM output to match.
- `for-in` over a bare string cannot be parity-tested: the bytecode VM rejects
  the construct with `ARRAY_LEN: not an array: <string>`. The WASM lowering
  previously iterated code points via `str_char_at`; no byte-for-byte parity
  test is possible until the VM supports the construct.
- `io::println` of a multi-field class or map renders fields in
  declaration/insertion order. The bytecode VM uses a hash map whose iteration
  order varies between runs, so those two shapes cannot be compared
  byte-for-byte. This item is out of scope for this agent (handled separately).
  Collections, enums, `Result` and tuples have deterministic representations
  and do match.

### Closed since the previous revision

Each item below now has a byte-for-byte parity test in `trc/tests/wasm_limits.rs`
(or an updated test in `wasm_hof.rs`), all run by `cargo test --test 'wasm_*'`:

- HashMap keys: all primitive keys (bool, integer, float/double, char and
  string) with correct hashing/equality.
  Test: `hashmap_all_primitive_keys`.
- File operations: `File.size`, `File.readLines`, `File.copy` and `File.rename`
  added alongside the original five. Test: `file_extended_operations`.
- `Json.parse`: a real host JSON parser supports nested, mixed-value and
  non-literal input. Test: `json_nested_mixed_and_nonliteral`.
- Regex: module-style `findGroups`, `groupCount`, `subN` and `split` are bound
  in addition to `match`, `find` and `replace`. Test: `regex_extended_natives`.
- OS/network: module-style `getcwd`, `getenv`, `urandom` are bound; network-like
  calls use deterministic host emulation. Test:
  `os_mutating_and_random_natives_emulated`.
- `ArrayList.reduce` on an empty list yields a dynamic `null` matching the VM
  instead of trapping. Test: `empty_reduce_yields_dynamic_null`.
- Iterator methods taking closures (`forEach`, `map`, `filter`, `reduce` with
  the VM's `(init, closure)` form) are lowered, and iterator element types
  other than scalars (class handles) are supported. Tests:
  `iterator_closure_methods`, `iterator_non_scalar_elements`.
- Return-only generic type parameter inference: verified by the existing
  `generic_return_only_type_param` parity test; no wrong case was found.

## Verification

```bash
cargo test -p trc --test 'wasm_*' --jobs 1
cargo test --all --jobs 1
cargo clippy --all --jobs 1 -- -D warnings
```

The parity tests compile each source through the real lexer/parser/analyzer,
run the emitted module on `wasmi`, and compare its captured output
byte-for-byte with `bytecode::execute`. Tests are organized by feature:
`wasm_collections.rs`, `wasm_enums.rs`, `wasm_tuples.rs`, `wasm_errors.rs`,
`wasm_closures.rs`, `wasm_oo.rs`, `wasm_modules.rs`, `wasm_generics.rs`,
`wasm_features.rs`, `wasm_hof.rs` and `wasm_host.rs`.
