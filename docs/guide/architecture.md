# Compiler Architecture

Understanding how the Titrate compiler works internally — from source text to running program. This guide is for contributors who want to modify the compiler, add language features, or just understand what happens under the hood.

## High-Level Architecture

```
  Source (.tr)
      │
      ▼
  ┌─────────┐
  │  Lexer   │  Characters → Tokens
  └────┬────┘
       │
       ▼
  ┌─────────┐
  │  Parser  │  Tokens → AST
  └────┬────┘
       │
       ▼
  ┌──────────┐
  │    AST    │  Abstract Syntax Tree
  └────┬─────┘
       │
       ▼
  ┌───────────┐
  │ Analyzer   │  Type checking, name resolution, ownership
  └──┬────┬───┘
     │    │
     │    ▼
     │  ┌──────────────────┐
     │  │  Bytecode Compiler│  AST → Bytecode (opcodes + constants)
     │  └───────┬──────────┘
     │          │
     │          ▼
     │  ┌───────────┐
     │  │     VM     │  Stack-based execution
     │  └───────────┘
     │
     ▼
  ┌────────────────┐
  │  Interpreter    │  Tree-walking execution (fast prototyping)
  └────────────────┘

  ── or, with the native backend ──

  Analyzer
      │
      ▼
  ┌──────────────────┐
  │  LLVM Codegen     │  AST → LLVM IR
  └───────┬──────────┘
          │
          ▼
  ┌──────────────────┐
  │  LLVM Optimizer   │  O3, inlining, vectorization
  └───────┬──────────┘
          │
          ▼
  ┌──────────────────┐
  │  System Linker    │  + titrate_native → standalone executable
  └──────────────────┘
```

The compiler is written in Rust and lives in `trc/src/`. Each stage is a separate module with clear responsibilities. Titrate has two execution backends — a bytecode VM (the default) and an LLVM native backend — that share the same front-end (lexer, parser, analyzer) and standard library.

## The Compilation Pipeline

### Source → Lexer → Tokens

**Module:** `trc/src/lexer/` (`scanner.rs`, `token.rs`)

The lexer reads source text character by character and produces a stream of tokens. It handles:

- **Keyword recognition** — `fn`, `let`, `var`, `class`, `if`, `while`, `return`, etc.
- **Literal parsing** — integers (`42`), floats (`3.14`), strings (`"hello"`), raw strings (`r"..."`), chars (`'a'`), booleans (`true`, `false`)
- **Operator tokenization** — `+`, `==`, `=>`, `&mut`, `..`, `..=`, compound assignments (`+=`, `-=`, etc.)
- **Operator overloading names** — `operator+`, `operator==`, etc.
- **Whitespace and comments** — skipped during tokenization
- **Error tokens** — unrecognized characters produce `Error` tokens with a message

The `Token` enum defines every token type the lexer can produce, from keywords (`Fn`, `Class`, `Interface`) to literals (`IntLiteral`, `StringLiteral`) to punctuation (`LeftBrace`, `FatArrow`).

### Tokens → Parser → AST

**Module:** `trc/src/parser/` (`declarations.rs`, `expressions.rs`, `statements.rs`, `patterns.rs`, `types.rs`)

The parser consumes the token stream and builds an Abstract Syntax Tree (AST). It uses **recursive descent** — each grammar rule corresponds to a function that calls other rule functions:

- `declarations.rs` — top-level declarations: classes, interfaces, enums, functions, imports
- `expressions.rs` — expressions with precedence climbing: binary ops, unary ops, calls, member access, closures
- `statements.rs` — statements: `let`/`var`/`const`, assignment, `if`, `while`, `for-in`, `switch`, `return`, `break`, `continue`
- `patterns.rs` — pattern matching: enum destructuring, `Ok`/`Err` matching, wildcard `_`
- `types.rs` — type expressions: named types, generics (`ArrayList<T>`), function types (`fn(int): string`), tuples

**Desugaring** happens during parsing. Some syntactic forms are transformed into simpler AST nodes before reaching the analyzer. For example, compound assignment operators (`x += 1`) may be desugared into assignment and binary operation nodes.

The AST is defined in `trc/src/ast/` (`nodes.rs`, `types.rs`).

### AST → Analyzer → Validated AST

**Module:** `trc/src/analyzer/` (`mod.rs`, `exprs.rs`, `stmts.rs`, `types.rs`, `inference.rs`, `scope.rs`, `registration.rs`, `closures.rs`, `errors.rs`)

The analyzer is the semantic pass. It walks the AST and enforces the language's rules:

- **Name resolution** — every identifier is resolved to its declaration (variable, function, class, field, etc.). The `scope.rs` module manages lexical scopes.
- **Type checking** — every expression is assigned a type, and assignments/arguments/returns are checked for compatibility. `inference.rs` handles type inference where types can be omitted.
- **Registration** — before type checking, `registration.rs` collects all top-level declarations so they can be referenced before they're defined (forward references).
- **Ownership checking** — verifies that `let` variables are not reassigned, that ownership annotations are consistent.
- **Closure analysis** — `closures.rs` determines which variables are captured by closures and whether they need to be heap-allocated.
- **Error reporting** — `errors.rs` produces typed errors with source locations and suggestions (including "did you mean?" hints using Levenshtein distance).

If the analyzer finds errors, compilation stops here. The user gets a list of errors with file, line, and column information.

### Optimizer

**Module:** `trc/src/bytecode/compiler/optimization.rs`

The optimizer runs on the analyzed AST (or during bytecode emission) and applies transformations that preserve semantics while improving performance:

- **Constant folding** — evaluate constant expressions at compile time (`3 + 4` → `7`)
- **Dead code elimination** — remove code that can never be reached (after `return`, `break`, `continue`)
- **Unused variable detection** — warn about variables that are declared but never read

These are the first optimization passes. The architecture is designed to support additional passes in the future.

### AST → Bytecode Compiler → Bytecode

**Module:** `trc/src/bytecode/compiler/` (`mod.rs`, `expr.rs`, `stmt.rs`, `chunk.rs`, `generics.rs`, `inference.rs`, `resolver.rs`, `symbols.rs`)

The bytecode compiler translates the validated AST into bytecode — a sequence of opcodes and operands that the VM can execute:

- `expr.rs` — emits bytecode for expressions (arithmetic, calls, field access, etc.)
- `stmt.rs` — emits bytecode for statements (variable declarations, control flow, etc.)
- `chunk.rs` — a `Chunk` is a unit of bytecode (a function body), containing opcodes, constant tables, and debug info
- `generics.rs` — handles monomorphization: for each concrete type used with a generic, a specialized copy is emitted
- `resolver.rs` — resolves variable references to stack slots
- `symbols.rs` — manages the symbol table for class names, method names, and field names

The output is a set of `Chunk` objects, each containing:
- An array of opcodes (see below)
- A constant table (string literals, numeric constants)
- A line number table (for error reporting)

### Bytecode → VM → Result

**Module:** `trc/src/bytecode/vm/` (`mod.rs`, `step.rs`, `call.rs`, `cast.rs`, `object.rs`, `operators.rs`)

The VM is a **stack-based interpreter**. It executes bytecode by:

1. **Pushing** values onto the operand stack
2. **Popping** values, operating on them, and **pushing** the result
3. **Jumping** to different bytecode offsets for control flow
4. **Calling** functions by creating new stack frames

Key VM components:

- `step.rs` — the main dispatch loop: reads an opcode and executes the corresponding operation
- `call.rs` — function and method call mechanics, including virtual dispatch for class methods
- `cast.rs` — type casting between numeric types and `string`
- `object.rs` — heap-allocated objects (class instances, enums, arrays, closures)
- `operators.rs` — arithmetic, comparison, and logical operators

**Native function dispatch** (`trc/src/bytecode/vm/natives/`): The VM has a set of built-in native functions that are implemented in Rust rather than Titrate bytecode. These include:

| Module | Functions |
|--------|-----------|
| `math.rs` | `Math_sqrt`, `Math_abs`, `Math_max`, etc. |
| `string.rs` | `String_length`, `String_charAt`, `String_substring`, etc. |
| `file.rs` | `File_readAll`, `File_writeAll`, `File_listDir`, etc. |
| `json.rs` | `Json_parse`, `Json_stringify` |
| `hash.rs` | `HashMap_new`, `HashMap_put`, `HashMap_get`, etc. |
| `net.rs` | HTTP client and server functions |
| `regex.rs` | Regular expression matching |
| `system.rs` | `System_currentTimeMillis`, environment variables |
| `time.rs` | Date and time operations |

Native functions are dispatched via the `CALL_NATIVE` opcode with a function index.

### Interpreter (Tree-Walking)

In addition to the bytecode compiler and VM, Titrate also supports a tree-walking interpreter for fast prototyping and development. The interpreter walks the analyzed AST directly without compiling to bytecode, making it ideal for exploratory programming and quick feedback cycles.

## The LLVM Native Backend

In addition to the bytecode VM and the tree-walking interpreter, Titrate
ships with an **LLVM native backend** that compiles `.tr` source to
standalone native executables. The native backend shares the same
front-end (lexer, parser, analyzer) and standard library as the
bytecode VM; it plugs in at the same point in the pipeline — right
after the analyzer — and produces LLVM IR instead of bytecode.

```
  Analyzer
      │
      ▼
  ┌──────────────────┐
  │  LLVM Codegen     │  AST → LLVM IR
  └───────┬──────────┘
          │
          ▼
  ┌──────────────────┐
  │  LLVM Optimizer   │  O3, inlining, vectorization
  └───────┬──────────┘
          │
          ▼
  ┌──────────────────┐
  │  System Linker    │  + titrate_native → standalone executable
  └──────────────────┘
```

### Module: `trc/src/codegen/llvm/`

The LLVM codegen lives in `trc/src/codegen/llvm/` and is organized into
focused submodules:

| File | Responsibility |
|------|----------------|
| `mod.rs` | The `LlvmBackend` entry point, top-level codegen driver |
| `types.rs` | Mapping from Titrate types to LLVM types |
| `ownership.rs` | Lowering of `Owned<T>`, borrows, regions, `unsafe` |
| `native_bridge.rs` | Marshalling and call emission for native functions |
| `vtable.rs` | Virtual dispatch tables for class methods |
| `enum_codegen.rs` | Enum construction and pattern matching |
| `tuple_codegen.rs` | Tuple construction and destructuring |
| `target_wrappers.rs` | Platform-specific wrappers (entry point, etc.) |
| `linker.rs` | Invoking the system linker to produce the executable |

### The Native Bridge

The native backend does not reimplement the 353 native functions
(`Math_sin`, `String_length`, `HashMap_put`, etc.) that the bytecode
VM uses. Instead, it links against the **`titrate_native` crate**
(`titrate_native/src/lib.rs`), which exposes them as `extern "C"`
functions with a uniform C-ABI signature.

The bridge uses a 24-byte tagged-union struct, `TitrateValue`, to
marshal values between LLVM IR and the Rust runtime:

```c
typedef struct {
    int32_t tag;        // type tag (TV_INT, TV_DOUBLE, TV_STRING, ...)
    int32_t pad;        // alignment padding
    uint8_t  payload[16]; // type-specific data
} TitrateValue;
```

Every native wrapper has the same signature:
`TitrateValue titrate_<Name>(const TitrateValue* args, size_t arg_count)`.

A handful of hot-path functions (`titrate_println`,
`titrate_string_concat`, `titrate_malloc`, `titrate_free`) bypass the
generic marshalling path and have custom signatures. The codegen
recognizes them by name and emits the right call pattern.

For the full story on the bridge — including how to wrap a new C
library — see [Wrapping C Libraries](./native-cbind).

### Ownership Lowering

The native backend lowers Titrate's ownership constructs to LLVM IR
without any runtime checks (the analyzer has already enforced the
rules):

- **`Owned<T>`** — a heap pointer (`i8*`) with a per-variable `i1`
  drop flag. Moves clear the source's drop flag. At scope exit, the
  flag is checked and `titrate_free` is called if still true.
- **`&T` / `&mut T`** — raw pointers (`*const T` / `*mut T`). The
  borrow-checker ran in the analyzer; the codegen just emits the
  address.
- **`region` blocks** — `alloca` plus `llvm.lifetime.start` /
  `llvm.lifetime.end` intrinsics.
- **`unsafe` blocks** — transparent to codegen. The body is emitted
  verbatim.

For the full story, see [Ownership on LLVM](./native-ownership).

### Release-Mode Optimizations

In release mode (`--release`), the codegen applies optimization hints
that let LLVM produce dramatically faster code:

- `alwaysinline` on small internal helpers (eliminates call overhead).
- `fastcc` calling convention for internal functions.
- `llvm.loop.vectorize.enable` / `llvm.loop.vectorize.width` metadata
  on loops.
- `llvm.memset.p0i8.i64` for zero-initializing class allocations.
- Pointer-arithmetic array loops (hoists base pointer, increments by
  element size).

For compute-bound workloads, release-mode native builds are typically
**3–6× faster** than the bytecode VM. See [Native MD Simulation](./native-md)
for benchmark numbers.

### How It Fits Alongside the Bytecode VM

The two backends are **complementary**, not competing:

- **Bytecode VM** — fast compilation, instant startup, portable. Use
  for development, I/O-bound programs, and quick iteration.
- **Native backend** — slower compilation, fixed link-time cost, but
  dramatically faster runtime for compute-bound code. Use for release
  builds, distribution, and performance-critical workloads.

Both backends share the same front-end, so a program that type-checks
will compile under either. The standard library works identically in
both (the native bridge calls the same Rust native functions the
bytecode VM uses). You can switch between them with a single flag
(`--native`), and the `pipette bench --compare-native` command makes
side-by-side benchmarking trivial.

## Instruction Set Overview

The VM has over 100 opcodes organized into categories:

| Category | Examples | Description |
|----------|----------|-------------|
| Constants | `PUSH_I32`, `PUSH_F64`, `PUSH_BOOL`, `PUSH_STRING`, `PUSH_NULL` | Push literal values |
| Stack | `POP`, `DUP`, `SWAP` | Stack manipulation |
| Arithmetic | `ADD_I32`, `SUB_F64`, `MUL_I64`, `DIV_I32` | Type-specialized math |
| Bitwise | `BITAND_I32`, `SHL_I64`, `BITNOT_I64` | Bit operations |
| Comparison | `EQ_I32`, `LT_F64`, `GE_I64`, `EQ_STRING` | Type-specialized comparisons |
| Logic | `AND`, `OR`, `NOT` | Boolean operations |
| String | `STR_CONCAT`, `STR_CONCAT_RIGHT`, `STR_CONCAT_LEFT` | String concatenation |
| Control flow | `JMP`, `JMP_IF_FALSE`, `CALL`, `RET` | Jumps and calls |
| Variables | `LOAD_LOCAL`, `STORE_LOCAL`, `LOAD_UPVALUE`, `STORE_UPVALUE` | Local and closure variables |
| Objects | `NEW`, `INVOKE_VIRTUAL`, `GET_FIELD`, `SET_FIELD` | Class instances |
| Arrays | `ARRAY_NEW`, `ARRAY_GET`, `ARRAY_SET`, `ARRAY_LEN` | Array operations |
| Ownership | `BOX_VALUE`, `UNBOX_VALUE`, `REF_IMMUTABLE`, `REF_MUTABLE`, `DEREF` | Ownership operations |
| Enums | `ENUM_NEW`, `MATCH_ENUM` | Enum construction and matching |
| Results | `RESULT_OK`, `RESULT_ERR`, `UNWRAP_OR_PROPAGATE` | Result type operations |
| Cast | `CAST` | Type conversion |
| Iteration | `ITER_NEXT` | For-in loop iteration |
| Pattern matching | `MATCH_ENUM`, `MATCH_OK`, `MATCH_ERR` | Switch/case dispatch |
| Closures | `CLOSURE_NEW`, `CLOSURE_CAPTURE`, `GET_UPVALUE`, `SET_UPVALUE` | Closure creation and capture |
| Tuples | `TUPLE_NEW`, `TUPLE_GET` | Tuple operations |
| Type check | `TYPE_CHECK` | `is` operator |
| Operator overloading | `INVOKE_OPERATOR` | Custom operator dispatch |

Each opcode has a fixed operand size. For example, `PUSH_I32` has a 4-byte operand (the integer value), `CALL` has a 3-byte operand (2-byte function index + 1-byte argument count), and `JMP` has a 2-byte operand (signed offset).

## How to Add a New Language Feature

Here is the typical workflow for adding a new feature to the compiler:

### 1. Add Tokens (Lexer)

In `trc/src/lexer/token.rs`, add a new variant to the `Token` enum. Then in `trc/src/lexer/scanner.rs`, add the logic to recognize the new token (keyword or operator).

### 2. Add AST Nodes (Parser)

In `trc/src/ast/nodes.rs`, add a new AST node type for the feature. Then in `trc/src/parser/`, add parsing logic — typically a new function in `expressions.rs` or `statements.rs` that constructs the new node.

### 3. Add Semantic Rules (Analyzer)

In `trc/src/analyzer/`, add type-checking and validation logic for the new node. This might involve:
- Adding a new case in `exprs.rs` or `stmts.rs`
- Updating `types.rs` if the feature introduces a new type
- Updating `scope.rs` if the feature affects name resolution

### 4. Add Bytecode Emission

In `trc/src/bytecode/compiler/`, add a new opcode to `opcodes.rs` (if needed) and add emission logic in `expr.rs` or `stmt.rs`.

### 5. Add VM Execution

In `trc/src/bytecode/vm/step.rs`, add a case for the new opcode in the dispatch loop.

### 6. Add Tests

- **Unit tests** in `trc/src/analyzer/tests.rs` or `trc/src/bytecode/vm/tests.rs`
- **Integration tests** in `trc/tests/stdlib_test.rs`
- **End-to-end tests** in `trc/tests/mega_test.rs`

### Example: Adding a `do-while` Loop

1. **Lexer**: Add `Do` token (already exists!)
2. **Parser**: Add `parseDoWhile()` in `statements.rs` that parses `do { body } while (condition);`
3. **Analyzer**: Add a case in `stmts.rs` — the body and condition are analyzed like any other loop
4. **Bytecode**: Emit the body, then the condition, then `JMP_IF_FALSE` back to the top
5. **VM**: No new opcode needed — existing `JMP_IF_FALSE` handles it
6. **Tests**: Add test cases in `mega_test.rs`

## Try It Yourself: Examine Bytecode Output

The compiler can dump bytecode for inspection. Try compiling a simple program and examining the output:

```titrate
public fn main(): void {
    let x = 10;
    let y = 20;
    let z = x + y;
    io::println(Integer.toString(z));
}
```

Conceptually, this compiles to something like:

```
PUSH_I32 10       // push 10
STORE_LOCAL 0     // store in slot 0 (x)
PUSH_I32 20       // push 20
STORE_LOCAL 1     // store in slot 1 (y)
LOAD_LOCAL 0      // load x
LOAD_LOCAL 1      // load y
ADD_I32           // add
STORE_LOCAL 2     // store in slot 2 (z)
LOAD_LOCAL 2      // load z
STATIC_CALL Integer.toString 1  // call toString
CALL_NATIVE io::println 1       // call println
PUSH_VOID
RET
```

Notice how the stack-based model works: values are pushed, operated on, and the result is pushed back. Variables are stored in numbered local slots.

## What is Next?

- [Optimizations](./optimizations) — compiler optimization passes
- [Why Native?](./native-intro) — what the LLVM native backend is and when to use it
- [Building Native Binaries](./native-build) — prerequisites and flags for native compilation
- [Ownership on LLVM](./native-ownership) — how `Owned<T>`, borrows, and regions lower to LLVM IR
- [Wrapping C Libraries](./native-cbind) — the native bridge and how to extend it
- [Native MD Simulation](./native-md) — benchmarks and profiling tips for the native backend
- [Contributing](./contributing) — how to contribute to the compiler
- [Grammar Reference](../reference/grammar) — formal grammar specification