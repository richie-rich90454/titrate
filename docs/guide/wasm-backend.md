# WebAssembly backend

The `wasm` backend lowers a typed Titrate AST to a WebAssembly module. It is
exposed as `trc::codegen::wasm::compile_to_wasm(&Program) -> Result<Vec<u8>, String>`;
the returned bytes are a complete `.wasm` binary and export `main`, `memory`
and `heap_ptr`.

This is a **partial** backend. It is not at parity with the bytecode VM yet;
see [Unsupported constructs](#unsupported-constructs) below.

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
- objects: `obj_new`, `field_set_*`, `field_get_*`

A working host implementation is in `trc/tests/wasm_conformance.rs`; it runs
the generated module and captures printed lines.

## Value mapping

| Titrate type | WASM representation |
|---|---|
| `byte`/`short`/`int`/`long`/`u*`/`size`/`vast` | `i64` |
| `float`/`double`/`half`/`quad` | `f64` |
| `bool` | `i32` (0 or 1) |
| `char` | `i32` (Unicode scalar) |
| `string` | `i32` pointer |
| class instance | `i32` host object handle |

Arithmetic, comparison, bitwise operations, control flow, and direct function
calls are emitted as native WebAssembly instructions. Only the operations that
need host state (printing, formatting, object fields, transcendental math) go
through imports.

## Supported constructs

- primitive literals, `let`/`var`/`const` declarations, assignment
- arithmetic (`+ - * / %`), comparison, logical (`&& || !`), bitwise and unary
  operators, explicit casts via `as`
- string literals, `+` concatenation, and the `String.*` helpers listed above
- `if`/`else`, `while`, `do`/`while`, C-style `for`, `break`, `continue`,
  ternary, `switch` on integer/bool/char/string literals with wildcard
- top-level functions, recursion, and `return`
- classes: constructors, public/private fields, field read/write, instance
  methods and `this`
- `io::println`, the `Integer`/`Long`/`Double`/`Boolean`/`Char` `toString`
  helpers, `parseInt`/`parseDouble`, and `Math`/`MathAdvanced`/`MathTrig`
  functions listed in `runtime.rs`
- top-level `const`/`var` declarations with literal initializers

## Unsupported constructs

These are rejected at code-generation time with a `wasm: ...` error rather
than silently miscompiled:

- collections (`ArrayList`, `HashMap`) and `for (x in ...)`, iterator/range
  protocols, `while let`
- closures, function values, indirect calls, captured variables
- enums, `Result`/`Ok`/`Err`, constructor-pattern `switch` cases, `is`, `with`
- `throw`/`try`/`catch`, the `?` propagation operator, `unsafe`/`region`/`Owned`
- tuples, tuple destructuring, references (`&T`/`&mut T`)
- interface dispatch, inheritance, operator overloading
- imported stdlib modules (`import tt::...`), `Sys`/`File`/`Json`/`Regex`/
  networking natives, `String.split`, and the remaining native functions
- generic functions/types and monomorphization

As a result the full `tests/e2e/mega_test.tr`, `mega_test_03.tr`, and the
stdlib-heavy examples do not compile through this backend yet. The smoke tests
in `trc/tests/wasm_reach.rs` record which examples currently lower
successfully (`hello.tr`, `fibonacci.tr`).

## Verification

```bash
cargo test -p trc --test wasm_conformance --jobs 1
cargo test -p trc --test wasm_reach --jobs 1
```

`wasm_conformance.rs` compiles each source with the real lexer/parser/analyzer,
runs the emitted module on `wasmi`, and compares its captured output
byte-for-byte with `bytecode::execute` for the same source.

## Path to full parity

1. Collections: add host-backed `ArrayList`/`HashMap` handles plus element-type
   aware add/get/set/size, and lower `for-in` to indexed loops.
2. Enums/Result: represent tagged values as host objects with a variant tag;
   lower constructor patterns by extracting payloads into locals.
3. Closures: compile each literal to a function plus a captured-environment
   handle, and lower indirect calls through function references or a table.
4. Error handling: add a host-side error cell and lower `throw`/`catch` to
   traps or explicit checks.
5. Module imports: reuse the bytecode compiler's module resolution before
   lowering, then map stdlib natives onto host imports.
