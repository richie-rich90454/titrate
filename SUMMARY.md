# Titrate Language — Comprehensive Capability Summary

> **Generated:** 2026-06-21
> **Version:** trc 0.3.0-alpha / pipette 0.2.0
> **Total commits:** 1296
> **Total .tr files:** 439 | **Total .rs files:** 107 | **Documentation:** 151 .md files

---

## Table of Contents

1. [Project Overview](#1-project-overview)
2. [Language Syntax & Features](#2-language-syntax--features)
3. [Type System](#3-type-system)
4. [Compiler & VM Architecture](#4-compiler--vm-architecture)
5. [Standard Library Modules](#5-standard-library-modules)
6. [Native Function Registry](#6-native-function-registry)
7. [Testing Infrastructure](#7-testing-infrastructure)
8. [Build Tools & Package Manager](#8-build-tools--package-manager)
9. [Documentation System](#9-documentation-system)
10. [Known Limitations & Stubs](#10-known-limitations--stubs)
11. [Future Directions](#11-future-directions)

---

## 1. Project Overview

### Repository Structure

```
titrate/
├── trc/                    # Compiler & VM (Rust, 92 .rs files)
│   ├── src/lexer/          # 3 files: mod, scanner, token
│   ├── src/parser/         # 6 files: mod, declarations, expressions, patterns, statements, types
│   ├── src/ast/            # 3 files: mod, nodes, types
│   ├── src/analyzer/       # 10 files: mod, closures, errors, exprs, inference, registration, scope, stmts, tests, types
│   ├── src/interpreter/    # 9 files: mod, env, eval, execution, heap, methods, operators, value, tests
│   ├── src/bytecode/       # compiler/ (9 files) + vm/ (7 files + natives/ 26 files)
│   └── tests/              # 9 integration test files
├── pipette/                # Build tool & package manager (Rust, 15 .rs files)
├── lib/tt/                 # Standard library (419 .tr files, 65 subdirectories)
│   └── data/               # External data files (40+ JSON files, 24 subdirectories)
├── examples/               # 15 example .tr programs
├── docs/                   # VitePress documentation (151 .md files)
├── stdlib_test/            # Stdlib integration test project
├── mega_test_02/           # Multi-file end-to-end test #2
├── mega_test_03/           # Multi-file end-to-end test #3
├── linguist/               # GitHub Linguist language definitions
├── mega_test.tr            # Root-level mega test
├── mega_test_03.tr         # Root-level mega test 03
├── Cargo.toml              # Workspace root (members: trc, pipette)
└── AGENTS.md               # Authoritative syntax specification
```

### Cargo Dependencies (trc)

| Dependency | Version | Purpose |
|-----------|---------|---------|
| `chrono` | 0.4 | Date/time handling |
| `regex` | 1 | Regular expression engine |
| `md-5` | 0.10 | MD5 hashing |
| `sha1` | 0.10 | SHA-1 hashing |
| `sha2` | 0.10 | SHA-256/384/512 hashing |
| `sha3` | 0.10 | SHA-3/SHAKE hashing |
| `blake2` | 0.10 | BLAKE2 hashing |
| `crc32fast` | 1.4 | CRC32 checksums |
| `base64` | 0.22 | Base64 encoding |
| `percent-encoding` | 2 | URL encoding |
| `rand` | 0.8 | Random number generation |
| `libc` | 0.2 | Unix system calls (unix only) |

### License
Apache-2.0

---

## 2. Language Syntax & Features

### 2.1 Lexical Structure

**Comments:** `//` line comments, `/* */` block comments

**Identifiers:** Start with letter or `_`, followed by letters/digits/underscores

**Keywords (full list):**
- Declarations: `fn`, `class`, `interface`, `enum`, `let`, `var`, `const`, `import`, `module`
- Access: `public`, `private`
- Control flow: `if`, `else`, `while`, `for`, `do`, `switch`, `case`, `default`, `return`, `break`, `continue`, `with`, `where`
- Literals: `true`, `false`, `null`
- OOP: `new`, `this`, `super`, `extends`, `implements`
- Type ops: `as`, `is`, `type`
- Result: `Result`, `Ok`, `Err`
- Ownership: `Owned`, `region`, `unsafe`
- Error handling: `throw`, `try`, `catch`, `finally`
- Primitives: `void`, `bool`, `byte`, `short`, `int`, `long`, `vast`, `uvast`, `float`, `double`, `half`, `quad`, `char`, `string`, `size`, `u8`, `u16`, `u32`, `u64`

**Literals:**
- Integer: `42`, `0xFF`, `0o77`, `0b1010`, `1_000_000`
- Float: `3.14` (double default), `1.5h` (half), `2.0q` (quad), `1_000.5_0`
- String: `"hello"` with escapes `\n \t \r \\ \" \' \0 \b \f`, raw strings `r"..."`, `r#"..."#`, `r##"..."##`, interpolation `"Hello, ${name}!"`
- Unicode escapes: `\u{HHHHHH}` (braces) and `\uXXXX` (4 hex digits, no braces)
- Character: `'a'`, `'\n'`
- Byte: `b'x'`, `b'\n'`, `b'\x41'`
- Boolean: `true`, `false`
- Null: `null`

**Operators (precedence low→high):**
1. Assignment: `=`, `+=`, `-=`, `*=`, `/=`, `%=`, `&=`, `|=`, `^=`, `<<=`, `>>=` (right-assoc)
2. Ternary: `? :` (right-assoc)
3. Range: `..` (exclusive), `..=` (inclusive)
4. Logical OR: `||`
5. Logical AND: `&&`
6. Equality: `==`, `!=`
7. Comparison: `<`, `>`, `<=`, `>=`
8. Bitwise: `|`, `^`, `&`, `<<`, `>>`
9. Addition: `+`, `-`
10. Multiplication: `*`, `/`, `%`
11. Unary: `!`, `-`, `~`, `*`, `&`, `&mut`, `++`, `--`
12. Postfix: call `.`, `::`, `[]`, `?`, `as`, `is`, `++`, `--`
13. Primary: literals, identifiers, `this`, `super`, `new`, grouping, tuples

### 2.2 Declarations

**Variable declarations:**
- `let x = 42` — type inference (mutable)
- `var x: int = 42` — explicit type (mutable)
- `const MAX: int = 100` — compile-time constant (immutable)
- C-style sugar: `int x = 42` → `var x: int = 42`

**Function declarations:**
```titrate
public fn greet(name: string): void { ... }
public fn map<T, R>(list: ArrayList<T>, f: fn(T): R): ArrayList<R> { ... }
```
- Java-style sugar tolerated: `public int add(int a, int b)` → `public fn add(a: int, b: int): int`
- Return types must always be explicitly declared
- Default access is private

**Class declarations:**
- Fields require `public` or `private` access modifier
- Constructors: `fn init(params)` or `fn ClassName(params)` (one per class)
- Methods: `fn name(params): ReturnType`
- Operator overloading: `fn operator+(other: T): T`
- No `static` keyword — use top-level `fn` instead
- No `hashCode()` methods
- `this.` for instance access (preferred), `self` parameter tolerated

**Interface declarations:**
- Support default method bodies (like Rust traits / Java 8 defaults)
- Generic interfaces: `interface Comparable<T>`

**Enum declarations:**
- `enum` keyword with optional payload: `enum JsonValue { Null, Bool(bool), Number(double), Str(string) }`
- Class-based enum pattern: `class Color extends EnumValue`

### 2.3 Control Flow

- `if`/`else if`/`else` — parentheses required
- `while` (condition) — parentheses required
- `do { } while (condition);`
- `while let value = iterator.next() { }`
- `for (item in list) { }` — parentheses required
- C-style `for (let i = 0; i < n; i++)` — tolerated but discouraged
- `break`, `continue`
- `switch`/`case` with pattern matching: `case Ok(v) => ...`, `case _ => ...`
- `with (resource) { }` — resource management
- Ternary: `x > 0 ? "pos" : "neg"`
- Range: `1..10` (exclusive), `1..=10` (inclusive)

### 2.4 Error Handling

- **Result<T, E>:** `ok(42)`, `err("failed")`, `isOk()`, `unwrap()`
- **throw/try/catch:** `throw "error message"`, `try { } catch (e: string) { }`, `try { } finally { }`
- **Error propagation:** `let value = mightFail()?();`

### 2.5 Closures & Function Types

- Function type: `fn(int): string`, `fn(string): bool`, `fn(int, int): int`
- Block closure: `let square = fn(x: int): int { return x * x; };`
- Arrow closure: `let square = fn(x: int): int => x * x;`
- Arrow closure with return type: `fn(x): int => x * x`

### 2.6 Tuples

- Tuple type: `(int, string)`, `(double, double, double)`
- Tuple literal: `(1, "hello")`
- Tuple destructuring: `let (a, b) = pair;`
- Tuple field access: `t.0`, `t.1` (numeric member access)
- Unit type: `void = ()`

### 2.7 Imports & Modules

- Canonical: `import tt::util::ArrayList;`
- Dot accepted: `import tt.util.ArrayList;`
- Glob: `import tt::util::*;`
- Interspersed with declarations (imports can appear anywhere at top level)
- Module calls: `Integer.parseInt("42")` or `Integer::parseInt("42")`

### 2.8 Generics

- Always provide type parameters: `ArrayList<string>`, `HashMap<string, int>`, `Result<int, string>`
- Generic functions: `fn map<T, R>(list: ArrayList<T>, f: fn(T): R): ArrayList<R>`
- Type parameters: single uppercase letters `<T>`, `<T, R>`, `<K, V>`
- Generic type arguments in expressions with backtracking: `Result<int, string>::ok(42)`

### 2.9 Advanced Features

- **References:** `&int` (immutable), `&mut int` (mutable)
- **Unsafe blocks:** `unsafe { }`
- **Region blocks:** `region name { }`
- **Owned type:** `Owned<int>`
- **Type casting:** `value as Type`
- **Type checking:** `value is Circle`
- **Variant:** dynamic type for when generics aren't suitable
- **Optional<T>:** null-safe alternative
- **`where` as identifier:** can be used as function name
- **`Result` as identifier:** can be used in expressions

---

## 3. Type System

### 3.1 Primitive Types

| Type | Size | Description |
|------|------|-------------|
| `void` | — | No value |
| `bool` | 1 bit | Boolean |
| `byte` | 8-bit | Signed integer |
| `short` | 16-bit | Signed integer |
| `int` | 32-bit | Signed integer |
| `long` | 64-bit | Signed integer |
| `vast` | arbitrary | Arbitrary-precision integer |
| `uvast` | arbitrary | Arbitrary-precision unsigned integer |
| `float` | 32-bit | Single-precision float |
| `double` | 64-bit | Double-precision float (default) |
| `half` | 16-bit | Half-precision float |
| `quad` | 128-bit | Quad-precision float |
| `char` | Unicode | Unicode character |
| `string` | Unicode | Unicode string (lowercase) |
| `size` | platform | Platform-dependent unsigned integer |
| `u8` | 8-bit | Unsigned integer |
| `u16` | 16-bit | Unsigned integer |
| `u32` | 32-bit | Unsigned integer |
| `u64` | 64-bit | Unsigned integer |

### 3.2 Composite Types

- Generic types: `ArrayList<string>`, `HashMap<string, int>`, `Result<int, string>`, `Optional<double>`
- Tuple types: `(int, string)`, `(double, double, double)`
- Function types: `fn(int): string`, `fn(string): bool`
- Reference types: `&int`, `&mut int`

### 3.3 Type Operations

- **Casting:** `value as Type` (implicit widening `int` → `double` also accepted)
- **Type checking:** `value is Circle` (returns `bool`)
- **Null comparison:** `if (x == null)`, `if (x != null)`

---

## 4. Compiler & VM Architecture

### 4.1 Compilation Pipeline

```
Source (.tr) → Lexer → Parser → AST → Analyzer → Interpreter (tree-walking)
                                           ↓
                                      Bytecode Compiler → VM (stack-based)
```

### 4.2 Lexer (`trc/src/lexer/`)

- **scanner.rs:** Main `tokenize()` function. Handles all token types including:
  - Numeric literals (decimal, hex, octal, binary, with underscores)
  - Float literals (with `h`/`q` suffixes for half/quad precision)
  - String literals (regular, raw with `r"..."`, `r#"..."#`, `r##"..."##`)
  - Unicode escapes (`\u{HHHHHH}` and `\uXXXX`)
  - All operators and delimiters
  - Keywords (70+ recognized)

- **token.rs:** Token enum definition with 100+ variants including:
  - All keywords, literals, operators
  - `Throw`, `Try`, `Catch`, `Finally` tokens
  - `FloatSuffix` enum (`h`, `q`)

### 4.3 Parser (`trc/src/parser/`)

- **declarations.rs:** Top-level parsing. Imports and declarations can be interspersed in any order.
- **expressions.rs:** Precedence-climbing expression parser. Supports:
  - Tuple field access (`t.0`, `t.1`)
  - Generic type arguments with backtracking (`Result<T, E>::method()`)
  - `is` type check expression
  - `where` and `Result` as identifiers
  - Arrow closures with optional return type (`fn(x): int => x * x`)
  - String interpolation `${expr}` (desugars to `Binary(Add)` chain at parse time)
- **statements.rs:** Statement parsing including `throw`, `try`/`catch`/`finally`, type keyword as variable name
- **types.rs:** Type parsing (generic types, tuple types, function types, reference types)
- **patterns.rs:** Pattern matching for switch/case
- **mod.rs:** Parser entry point, `token_as_name()` for keyword-as-name desugaring

### 4.4 AST (`trc/src/ast/`)

- **nodes.rs:** Statement and expression AST nodes. Key variants:
  - Statements: `VarDecl`, `ConstDecl`, `TupleDestructure`, `Throw`, `TryCatch`
  - Expressions: `Cast`, `Is`, `Tuple`, `Unit`, `Closure`, `ErrorPropagation`, `StaticCall`, `MemberAccess`, `UnsafeBlock`, `OwnedDeref`, `RefExpr`
  - `Operator` enum with 30+ operators

### 4.5 Analyzer (`trc/src/analyzer/`)

- **scope.rs:** Symbol types and scope management
- **types.rs:** Type helpers
- **inference.rs:** Type inference for expressions (`Is` → `bool`, `Cast` → target type)
- **exprs.rs:** Expression analysis with symbol resolution and similar name suggestions
- **stmts.rs:** Statement analysis with block return analysis and terminator detection. Handles `Throw` and `TryCatch` with catch variable scoping.
- **registration.rs:** Top-level declaration registration
- **closures.rs:** Captured variable collection from expressions and statements (including `Is`, `Throw`, `TryCatch`)
- **errors.rs:** Compile error types

### 4.6 Interpreter (`trc/src/interpreter/`)

Tree-walking interpreter with:
- **eval.rs:** Expression evaluation. Handles `Is` (type checking against `ClassInstance`, `String`, `Int`, `Long`, `Double`, `Bool`, `Null`), tuple field access, `Cast`, `StaticCall`, closures, ternary, etc.
- **execution.rs:** Statement execution. Handles `Throw` (converts to error), `TryCatch` (catches errors and `ResultErr`), `With` resource binding, etc.
- **value.rs:** `Value` enum with variants: `Void`, `Null`, `Bool`, `Int`, `Long`, `Double`, `String`, `ClassInstance`, `Tuple`, `Result`, `Closure`, `Ref`, etc.
- **env.rs:** Variable scoping with parent chain
- **heap.rs:** Memory simulation for heap allocation
- **methods.rs:** Method dispatch and vtable lookup
- **operators.rs:** Operator helper functions including unsigned right shift `>>>`

### 4.7 Bytecode Compiler (`trc/src/bytecode/compiler/`)

- **mod.rs:** Lowers AST to bytecode chunks
- **expr.rs:** Expression compilation (`Is` → push bool, `Cast` → CAST opcode)
- **stmt.rs:** Statement compilation (`Throw` → compile + POP, `TryCatch` → compile both blocks)
- **generics.rs:** Monomorphization with type substitution for all expression/statement types
- **inference.rs:** Type inference helpers (`Is` → `InferredType::Bool`)
- **symbols.rs:** Local variable management
- **optimization.rs:** Constant folding, dead code elimination, unused string removal
- **resolver.rs:** Module resolution with import path caching

### 4.8 Bytecode VM (`trc/src/bytecode/vm/`)

Stack-based virtual machine:
- **step.rs:** Main instruction decode loop. Handles all opcodes including tuple field access (`GET_FIELD` on `Value::Tuple`).
- **call.rs:** Function invocation, closure upvalue resolution
- **object.rs:** Class instantiation and built-in pseudo-class handling
- **operators.rs:** Operator dispatch
- **cast.rs:** Type conversion logic

### 4.9 Native Function Modules (`trc/src/bytecode/vm/natives/`)

26 Rust files implementing 344 native functions (350 registered names with 3 aliases):

| Module | Functions | Key Operations |
|--------|-----------|----------------|
| `system.rs` | 53 | OS info, process control, filesystem, signals, env vars |
| `math.rs` | 33 | Trig, log, exp, pow, sqrt, abs, floor, ceil, round, fma |
| `socket.rs` | 31 | TCP/UDP sockets, addr info, socket options |
| `atomic.rs` | 24 | AtomicInt, AtomicLong, AtomicBool, AtomicRef |
| `hash.rs` | 22 | MD5, SHA-1/2/3, BLAKE2, CRC32, SHAKE, Hasher API |
| `net.rs` | 20 | TCP, HTTP (GET/POST/PUT/DELETE/PATCH/HEAD), DNS |
| `file.rs` | 18 | Read, write, open, close, seek, bytes, copy, delete, setModified |
| `mutex.rs` | 17 | Mutex, RecursiveMutex, SharedMutex (RwLock), OnceFlag |
| `time.rs` | 15 | Now, sleep, format, getYear/Month/Day/Hour/Minute/Second |
| `sqlite.rs` | 14 | Open, execute, query, row access, column metadata |
| `builtins.rs` | 12 | println, toString, parseInt, Ok/Err, String operations |
| `string.rs` | 11 | Trim, startsWith, endsWith, pad, case, replace, charAt |
| `regex.rs` | 9 | Match, find, replace, groupCount, fullMatch, subN |
| `thread.rs` | 8 | Spawn, join, sleep, yield, getId, currentId, detach |
| `ssl.rs` | 8 | Context, connect, send, recv, close, peer cert |
| `path.rs` | 8 | Join, exists, isFile, isDir, basename, dirname, extension |
| `directory.rs` | 7 | List, create, remove, removeTree, walk, copy, move |
| `condvar.rs` | 5 | New, wait, waitFor, notifyOne, notifyAll |
| `semaphore.rs` | 5 | New, acquire, release, tryAcquire, availablePermits |
| `encoding.rs` | 6 | Base64, Hex, URL encode/decode |
| `mmap.rs` | 6 | Open, close, get, set, size, flush |
| `zlib.rs` | 4 | Zlib and Gzip compress/decompress |
| `json.rs` | 2 | Parse and stringify |
| `subprocess.rs` | 3 | Run, exec, popenWrite (pipe input to stdin) |
| `random.rs` | 2 | Seed and nextLong |
| `tempfile.rs` | 1 | Create |

**Platform-specific implementations:**
- `Os_kill` uses `libc::kill()` syscall (Unix)
- `Os_strerror` uses `libc::strerror()` for real OS error messages (Unix)
- `Os_access` uses `libc::access()` for real permission checking (R_OK/W_OK/X_OK) (Unix)
- `Os_umask` uses `libc::umask()` (Unix) or C runtime `_umask()` (Windows)
- `Os_getppid` uses `libc::getppid()` (Unix) or toolhelp32 API (Windows)
- `Fs_totalSpace`/`Fs_freeSpace` use `libc::statvfs()` (Unix) or `GetDiskFreeSpaceExW` (Windows)
- `SharedMutex` uses `RwLock<()>` for actual reader-writer lock semantics

---

## 5. Standard Library Modules

The standard library at `lib/tt/` contains **419 .tr files** across **65 subdirectories**, plus **40+ JSON data files** in `lib/tt/data/`.

### 5.1 Core Language (`lang/` — ~50 files)

| File | Key Contents |
|------|-------------|
| `String.tr` | 60+ string operations: length, concat, case, replace, split, join, format, contains, compareTo, pad, trim, wrap, indent, dedent, partition, center, ljust, rjust, zfill, swapcase, title, capitalize, isAlpha, isDigit, lines, chunked |
| `Integer.tr` | parseInt, toString, MAX/MIN_VALUE, compare, max, min, toHexString/Binary/Octal, bitCount, highestOneBit, rotateLeft/Right |
| `Double.tr` | parse, toString, isNaN |
| `Long.tr` | parse, toString, MAX/MIN_VALUE |
| `Float.tr` | parse, toString, isNaN, isInfinite, POSITIVE/NEGATIVE_INFINITY, NaN |
| `Half.tr` / `Quad.tr` | Half/quad precision float parse/toString |
| `Byte.tr` / `Short.tr` | Byte/short parse/toString |
| `Vast.tr` / `Uvast.tr` | Arbitrary-precision integer parse/toString |
| `VastExt.tr` | vastAdd/Sub/Mul/Div/Mod/Gcd/Lcm, factorial, isProbablePrime, shiftLeft/Right, bitLength, toByteArray |
| `Boolean.tr` | toString, parseBoolean, logicalAnd/Or/Not |
| `Character.tr` / `CharacterExt.tr` | isDigit, isLetter, isWhitespace, isUpperCase/LowerCase, toUpperCase/LowerCase, getNumericValue, isAlphabetic, isISOControl |
| `Optional.tr` / `OptionalExt.tr` | empty, of, ofNullable, map, flatMap, filter, orElse, ifPresent, isPresent, isEmpty |
| `Result.tr` / `ResultExt.tr` | ok, err, andThen, orElse, unwrapOr, unwrapOrElse, expect, map, mapErr, flatten, transpose |
| `Variant.tr` / `VariantExt.tr` | of, empty, holdsAlternative, get, visit, variantIndex, variantTypeName |
| `Enum.tr` / `EnumExt.tr` | EnumValue, IntEnum, auto, resetAuto, enumValueOf, enumFromOrdinal, FlagEnum, StrEnum |
| `Tuple.tr` / `TupleExt.tr` | Tuple2<A,B>, Tuple3<A,B,C>, Tuple4<A,B,C,D>, makeTuple, tupleCat |
| `WeakRef.tr` | WeakRef<T> with registry-based GC simulation, collect(), liveCount(), clearRegistry() |
| `DataFile.tr` | load, loadCsv, loadText, exists, list, meta, validate, clearCache |
| `NumericLimits.tr` | intMax/Min, longMax/Min, doubleMax/Min/Epsilon/NaN/Inf, floatMax/Min/Epsilon, byteMax/Min, shortMax/Min, halfMax/Min/Epsilon, quadMax/Min/Epsilon, u8/u16/u32/u64Max |
| `IntegerExt.tr` | parseIntWithRadix, parseUnsignedInt, toUnsignedString, bitCount, rotateLeft/Right, clampInt |
| `LongExt.tr` | divideUnsigned, remainderUnsigned, numberOfLeadingZeros, reverse, reverseBytes, bitCountLong |
| `DoubleExt.tr` | isFinite, isInfinite, isNaN, doubleToLongBits, longBitsToDouble, toHexString, toEngineeringString |
| `StringExt.tr` / `StringUtils.tr` / `StringView.tr` | Extended string operations, wordWrap, camelToSnake, snakeToCamel, pluralize, levenshteinDistance, StringView |
| `Subprocess.tr` | Popen, SubprocessResult |
| `Traceback.tr` | Frame, format, formatFrame, extract, formatException |
| `Abc.tr` | Abstract base class |
| `AssertExt.tr` | assertEqual, assertNotEqual, assertTrue, assertFalse, assertAlmostEqual, assertRaises, assertIn, assertGreater, assertLess |
| `AssayExt.tr` | TestFixture, TestSuite, BenchmarkResult, parameterizedTest, benchmark |
| `Contextlib.tr` | ContextManager, ExitStack, closing, suppress |
| `CopyExt.tr` | CopyDispatcher, deepCopy, shallowCopy, replace |
| `DataclassExt.tr` | DataclassExt |
| `Functools.tr` | LruCache, CachedProperty, PartialFn, DispatchRegistry, lruCache, partial, reduce, cmpToKey, singledispatch, cachedProperty, wraps, totalOrdering |
| `Itertools.tr` | accumulate, compress, dropwhile, filterfalse, groupby, islice, starmap, takewhile, zipLongest |
| `Iterator.tr` | Iterator<E> interface, MappedIterator, FilteredIterator, TakeIterator, ChainIterator, ZipIterator, EnumeratedIterator |
| `LoggerExt.tr` | JsonFormatter, SyslogHandler, RotatingHandler, EmailHandler, PerLevelLogger, AsyncHandler |
| `ErrorCode.tr` | ErrorCode, genericError, systemError, ioError, networkError, databaseError |
| `ParseError.tr` | ParseError enum |
| `ArgparseExt.tr` | ArgparseExt |

### 5.2 Collections (`util/` — ~35 files)

| File | Key Contents |
|------|-------------|
| `ArrayList.tr` | ArrayList<T> with sort, reverse, copy, extend, indexOf, count, remove |
| `HashMap.tr` | HashMap<K,V> with fromKeys, setDefault, pop, merge |
| `HashSet.tr` | HashSet<E> |
| `TreeMap.tr` | TreeMap<K,V> (red-black tree) |
| `TreeSet.tr` | TreeSet<E> |
| `LinkedHashMap` | (via HashMap with insertion order tracking) |
| `OrderedDict.tr` | OrderedDict<K,V> |
| `RingDeque.tr` | RingDeque<E> — O(1) double-ended queue using ring buffer |
| `Deque.tr` | Deque — backed by RingDeque for O(1) pushLeft/popLeft |
| `LinkedList.tr` | LinkedList<E> with Node<E> |
| `ForwardList.tr` | ForwardList<T> singly-linked list |
| `Stack.tr` | Stack<E> |
| `Queue.tr` | Queue<E> |
| `PriorityQueue.tr` / `PriorityQueueExt.tr` | Heap-based priority queue |
| `BitSet.tr` | BitSet |
| `Counter.tr` | Counter |
| `defaultdict.tr` | defaultdict<K,V> |
| `ChainMap.tr` | ChainMap<K,V> |
| `Range.tr` | Range, RangeIterator |
| `FrozenSet.tr` | FrozenSet<E> |
| `Set.tr` | Set<E> |
| `Trie.tr` | Trie with TrieNode |
| `UnionFind.tr` | UnionFind (disjoint set) |
| `Pair.tr` | Pair<F,S> |
| `Span.tr` | Span<T> |
| `Vec.tr` | Vec<T> |
| `Array.tr` | Array<T> |
| `StringBuilder.tr` | StringBuilder |
| `Graph.tr` | Graph<V> |
| `GraphUtil.tr` | GraphUtil |
| `namedTuple.tr` | namedTuple |
| `UserDict.tr` / `UserList.tr` / `UserString.tr` | User-defined collection base classes |
| `ArrayListIterator.tr` | ArrayListIterator<T> |

### 5.3 Math & Science

#### Math Core (`math/`)
- `Math.tr` — PI, E, tau, INF, NAN, abs, floor, ceil, round, min, max, gcd, lcm, roundTo, floorTo, ceilTo
- `MathAdvanced.tr` — ln, log10, log2, log, log1p, exp, expm1, pow, sqrt, cbrt, hypot, fma, copySign, nextUp, nextDown, ulp, getExponent, scalb, floorDiv, floorMod, addExact, subtractExact, multiplyExact
- `MathTrig.tr` — sin, cos, tan, asin, acos, atan, atan2, sinh, cosh, tanh, asinh, acosh, atanh
- `Combinatorics.tr` — factorial, binomial, stirling1/2, bellNumber, catalanNumber, partitionNumber, derangement, multinomial, fibonacci, lucasNumber, permutations
- `NumberTheory.tr` — modPow, extendedGcd, modularInverse, chineseRemainder, isPrime, primeSieve, factorize, eulerTotient, mobius, legendreSymbol, jacobiSymbol, isCarmichael
- `Bit.tr` — popcount, floor2, ceil2
- `Autodiff.tr` — DualNumber for automatic differentiation
- `ContinuedFraction.tr` — ContinuedFraction class, sqrtContinued
- `IntervalArithmetic.tr` — Interval class
- `TensorOps.tr` — tensorContract, tensorProduct, tensorPermute, tensorTranspose, tensorSymmetrize, tensorTrace, tensorNorm, tensorReshape, tensorFlatten, tensorSlice

#### Math Subdirectories
- **`math/complex/`** — Complex.tr: Complex number class
- **`math/conic/`** — Conic.tr: Parabola, Ellipse, Hyperbola, Circle
- **`math/fft/`** — FFT.tr: fft, ifft, fftMagnitude, fftPhase, fftFreq, windowHann/Hamming/Blackman
- **`math/geometry3d/`** — AABB, Frustum, Plane, Ray, Sphere, Triangle (3D geometry)
- **`math/interpolation/`** — Interpolation.tr: CubicSpline
- **`math/linalg/`** — Matrix.tr, MatrixDecomp.tr (LU, Cholesky, QR, SVD, eigenvalues), MatrixOps.tr (cross, outerProduct, vandermonde), MatrixProps.tr (determinant, rank, condition number), SparseMatrix.tr (CSR, CSC, sparse CG, sparse LU)
- **`math/ndarray/`** — NDArray.tr, NDArrayMath.tr (50+ element-wise ops), NDArrayReduce.tr (sum, mean, min, max, variance, stddev, percentile), NDArrayGen.tr (linspace, arange, logspace), NDArrayManip.tr (flip, rot90, pad, squeeze), NDArrayShape.tr (broadcastTo), NDArraySort.tr, NDArrayAnalysis.tr (histogram, interp, cov, corrcoef), Broadcast.tr, Cumulative.tr, Einsum.tr, Indexing.tr, Manipulation.tr, Rolling.tr, SearchSort.tr
- **`math/optimization/`** — Optimization.tr: OptResult class
- **`math/pde/`** — PDE.tr: BoundaryCondition, DirichletBC, NeumannBC, RobinBC, heatEquation1D
- **`math/polynomial/`** — Polynomial.tr: Polynomial class, polyfit, fromRoots, companionMatrix, lagrangeInterpolation
- **`math/sequence/`** — Sequence.tr: arithmetic/geometric sequences, fibonacci, triangular, pascalsTriangle, sums
- **`math/special/`** — Special.tr: 60+ special functions including beta, zeta, Bessel functions (J, Y, I, K), Airy, elliptic integrals, digamma, trigamma, softmax, sigmoid, ReLU, GELU, hypergeometric functions, Legendre/Hermite/Laguerre/Chebyshev polynomials
- **`math/transform/`** — Color.tr (Color, RGBA, HSV, hex), Mat4.tr (4x4 matrices, perspective, orthographic, lookAt), Quaternion.tr (quaternion math), Transform.tr
- **`math/vec/`** — Vec2.tr, Vec3.tr, Vec4.tr (2D/3D/4D vectors)

#### Linear Algebra (`linalg/`)
- Cholesky.tr — cholesky, choleskySolve, choleskyUpdate/Downdate, isPositiveDefinite
- Eigen.tr — eigenvalues, eigenvectors, powerIteration, inverseIteration, rayleighQuotient
- MatrixProps.tr — norm1/2/Inf/Frobenius, matrixRank, nullSpace, rangeSpace, conditionNumber, determinant, trace
- Qr.tr — qrHouseholder, qrGivens, qrPivoted, rankRevealingQr
- Solve.tr — solve, solveSymmetric, leastSquares, weightedLeastSquares, conjugateGradient, gmres, bandedSolve, bicgstab
- Svd.tr — svd, thinSvd, truncatedSvd, pseudoinverse, lowRankApproximation

#### Calculus (`calculus/`)
- Calculus.tr — derivative, nthDerivative, partialDerivative, riemannSum, trapezoidRule, simpsonsRule, integrate, improperIntegral, limit, newtonsMethod, bisection, secantMethod, findMin/Max, criticalPoints, concavity, inflectionPoints, areaBetweenCurves, volume (disk/washer/shell), arcLength, parametricDerivative, polarDerivative, polarArea
- LineIntegral.tr — lineIntegral, surfaceIntegral, greensTheorem
- Quadrature.tr — gaussLegendre, gaussKronrod, simpsonAdaptive, romberg, doubleIntegral, tripleIntegral
- Series.tr — taylorSeries, laurentSeries, padeApproximation, asymptoticExpansion
- Symbolic.tr — Expr class for symbolic differentiation, simplify, expand, substitute, evaluate
- VectorCalc.tr — gradient, divergence, curl2d/3d, laplacian, hessian, jacobian, directionalDerivative

### 5.4 I/O & File System (`io/` — ~19 files)

| File | Key Contents |
|------|-------------|
| `IO.tr` | println, print, readLine, readAll, stderr, eprintln, eprint, writeStderr, flushStderr, input, open |
| `File.tr` | File class |
| `BufferedReader.tr` | BufferedReader + open |
| `FileReader.tr` / `FileWriter.tr` | FileReader/FileWriter + open |
| `FileWatcher.tr` | WatchEvent, FileWatcher + watchRecursive |
| `FileLock.tr` | FileLock + withLock, withLockTimeout |
| `Format.tr` | format, sprintf |
| `BytesIO.tr` | BytesIO class |
| `StringReader.tr` / `StringWriter.tr` | StringReader/StringWriter |
| `Pipe.tr` | PipeReader, PipeWriter, NamedPipe + createPipe |
| `Mmap.tr` / `MmapExt.tr` | Mmap, MmapExt + mmapReadOnly |
| `AsyncFile.tr` | AsyncFileReader, AsyncFileWriter + readWithProgress |
| `Tempfile.tr` | TemporaryFile, SpooledTemporaryFile + mkstemp, mkdtemp |
| `Reader.tr` / `Writer.tr` | Reader/Writer interfaces |

#### File Utilities (`file/`)
- `Directory.tr` — Directory class, getCurrent, setCurrent (uses Sys_setWorkingDir)
- `FileUtils.tr` — copy2, copyfileobj, copystat (uses File_setModified), copy, copyTree, move, rmtree, which, diskUsage, sameFile
- `Path.tr` — Path class
- `Fnmatch.tr` — fnmatch, fnmatchCase, filter, translate
- `Glob.tr` — glob, iglob, escape

### 5.5 Networking (`net/` — ~12 files)

| File | Key Contents |
|------|-------------|
| `HttpClient.tr` | HttpResponse, HttpException, HttpClient |
| `TcpServer.tr` | TcpServer (getLocalPort, setTimeout via native Net_ functions) |
| `TcpClient.tr` | TcpClient (getLocalAddress, getRemoteAddress, setTimeout via native Net_ functions) |
| `UdpSocket.tr` | UdpSocket |
| `Socket.tr` | Socket |
| `Dns.tr` | DnsRecord, DnsCache |
| `IpAddress.tr` | IpAddress, IPv4Network, IPv6Network (128-bit arithmetic via `vast` type) |
| `Ssl.tr` / `SslExt.tr` | SslContext, SslConnection, TlsConfig, TlsSession (certificatePin secure by default) |
| `WebSocket.tr` | WebSocketFrame, WebSocketClient |
| `Smtp.tr` | SmtpClient |
| `HttpUtil.tr` | CookieJar, MultipartForm, HttpCache, ConnectionPool |
| `UrlBuilder.tr` | Url class |

### 5.6 Concurrency (`concurrent/` — ~18 files)

| File | Key Contents |
|------|-------------|
| `Mutex.tr` / `RecursiveMutex.tr` | Mutex, RecursiveMutex |
| `SharedMutex` | (via mutex.rs native — RwLock for reader-writer semantics) |
| `Semaphore.tr` | Semaphore |
| `OnceFlag.tr` | OnceFlag |
| `ConditionVariable.tr` | ConditionVariable |
| `Event.tr` | Event |
| `Barrier.tr` / `Latch.tr` | Barrier, Latch |
| `Atomic.tr` | AtomicInt, AtomicLong, AtomicReference<T>, AtomicBool |
| `Channel.tr` | BufferedChannel, UnboundedChannel |
| `LockFreeQueue.tr` | SPSCRingBuffer, MPSCRingBuffer, LockFreeStack |
| `LockGuard.tr` | LockGuard |
| `Future.tr` / `Promise.tr` | Future<T>, Promise<T>, CancelledError, TimeoutError |
| `Actor.tr` | ActorMessage, ActorRef, ActorSystem, SupervisorStrategy, Actor |
| `RateLimiter.tr` | TokenBucket, LeakyBucket, SlidingWindowRateLimiter, FixedWindowRateLimiter |
| `CircuitBreaker.tr` | CircuitBreaker |
| `Retry.tr` | RetryPolicy |

### 5.7 Cryptography (`crypto/`, `crypto2/`)

- `Hash.tr` / `Hmac.tr` — MD5, SHA-1/256/384/512, SHA-3, BLAKE2, CRC32, HMAC
- `CryptoExt.tr` — Ed25519, Curve25519, ChaCha20-Poly1305, HKDF, constant-time comparison
- `Secrets.tr` — tokenBytes, tokenHex, tokenUrlSafe, randBelow, SystemRandom
- `crypto2/AES.tr` — AES block encrypt/decrypt (ECB, CBC, CTR, GCM), PKCS7 padding (fails closed for security)

### 5.8 Data Serialization

#### JSON (`json/` — 9 files)
- `Json.tr` / `JsonParser.tr` / `JsonValue.tr` — parse, stringify, registerEncoder/Decoder
- `JsonBinary.tr` — MessagePack binary JSON
- `JsonSchema.tr` — JsonSchema validation
- `JsonPatch.tr` — JsonPatch, JsonPointer, diff, compile
- `JsonPath.tr` — JsonPathExpression, compile, query
- `JsonStreamingParser.tr` — JsonEvent, JsonStreamingParser
- `Json5.tr` — Json5 class

#### XML (`xml/` — 8 files)
- `Xml.tr` / `XmlNode.tr` — parse, XmlElement, XmlNode
- `XmlBuilder.tr` — XmlBuilder, serializePretty/Minified
- `XmlCanonicalizer.tr` — XML canonicalization
- `XmlNamespace.tr` — NamespaceMap, namespace scope management
- `XmlSchema.tr` — XmlSchema validation
- `XmlStreamingParser.tr` — streaming XML parser
- `XPath.tr` — XPathExpression, selectNodes, XPath functions

#### CSV (`csv/`)
- `CsvReader.tr` — CsvReader, DictReader, Dialect, Sniffer
- `CsvWriter.tr` — CsvWriter, DictWriter

#### Other Formats
- `config/Toml.tr` — TOML parser
- `config/ConfigParser.tr` — INI-style config parser
- `html/Html.tr` — Html, HtmlElement, HtmlParser

### 5.9 Text Processing

- `pprint/Pprint.tr` — PrettyPrinter
- `textwrap/Textwrap.tr` — Textwrap
- `text/Difflib.tr` — SequenceMatcher, unifiedDiff, contextDiff, getCloseMatches
- `text/Shlex.tr` — Shlex, split, quote, join
- `text/Unicodedata.tr` — Unicode category, decimal, numeric, normalize, name, lookup, bidirectional, combining, mirrored
- `string/StringUtils.tr` — StringUtils, Template
- `encoding/Base64.tr` — Base64, Base32, Base16, Base85
- `encoding/Hex.tr` — Hex
- `encoding/Url.tr` — Url, ParseResult, parse, unparse, join
- `encoding/Codecs.tr` — encode, decode, register (custom codec registry)

### 5.10 Science & Engineering

#### Chemistry (`chem/` — ~15 files)
- `Atom.tr` / `PeriodicTable.tr` — Element data, periodic table, atomic properties
- `Bond.tr` — Bond class
- `ForceField.tr` / `ForceFieldNonbonded.tr` — Molecular force field (Lennard-Jones, Coulomb)
- `Integrator.tr` / `MD.tr` — Molecular dynamics simulation (Verlet integrator)
- `Kinetics.tr` — Reaction kinetics (zero/first/second order, Arrhenius)
- `Thermochemistry.tr` — Enthalpy, Gibbs free energy, entropy, Hess's law
- `ReactionBalancer.tr` — Chemical equation balancing, oxidation states
- `Electrochemistry.tr` — Nernst potential, cell potential
- `RHF.tr` / `RHFIntegrals.tr` / `RHFMatrix.tr` / `RHFDiag.tr` — Hartree-Fock quantum chemistry

#### Physics (`physics/` — ~6 files)
- `ForceField.tr` — Gravitational, Coulomb, Lorentz, spring, Lennard-Jones forces
- `NBodySimulator.tr` — N-body simulation (direct O(n²) + full Barnes-Hut octree with theta criterion)
- `Particle.tr` — Particle class
- `RigidBody.tr` — RigidBody class
- `UnitSystem.tr` — Unit conversion (SI, CGS, natural)
- `WaveFunction.tr` — WaveFunction class

#### Biology (`bio/` — ~6 files)
- `Alignment.tr` — Sequence alignment
- `FastaReader.tr` / `FastaWriter.tr` — FASTA format I/O
- `PhyloTree.tr` — Phylogenetic tree, Newick parser
- `RestrictionEnzyme.tr` — Restriction enzyme database
- `Sequence.tr` — DNA/protein sequence class

#### Materials Science (`materials/` — 4 files)
- `CrystalStructure.tr` — UnitCell, MillerIndices
- `Elasticity.tr` — StressTensor, StrainTensor, ElasticConstants
- `PhaseDiagram.tr` — PhasePoint, BinaryPhaseDiagram
- `XRayDiffraction.tr` — AtomPosition, XRD

#### Signal Processing (`sigproc/` — ~7 files)
- `Filter.tr` — Butterworth (arbitrary-order cascaded biquad), Chebyshev, FIR, IIR filter design
- `FFT2.tr` — FFT, IFFT, FFT frequencies, windowing
- `Convolution.tr` — 1D/2D convolution, correlation, deconvolution
- `Spectrogram.tr` — STFT, power spectral density, mel spectrogram
- `Wavelet.tr` — Haar, Morlet CWT, DWT
- `Window.tr` — Hamming, Hanning, Blackman, Bartlett, Kaiser, Gaussian windows

#### Image Processing (`image/` — 4 files)
- `Image.tr` — Image class
- `Kernel.tr` — Gaussian blur, sharpen, edge detect, Sobel, Laplacian
- `Morphology.tr` — Dilation, erosion, opening, closing, gradient, top-hat
- `Threshold.tr` — Binary threshold, Otsu, adaptive, histogram equalization
- `Transform.tr` — Affine transforms, warp

### 5.11 Finance & HFT

#### Finance (`finance/` — ~10 files)
- `BlackScholes.tr` — Call/put pricing, implied volatility, Greeks (delta, gamma, theta, vega, rho)
- `BinomialTree.tr` — CRR binomial tree pricing
- `MonteCarloPricing.tr` — GBM paths, Monte Carlo call/put/Asian, antithetic/control variates
- `Indicators.tr` — SMA, EMA, EWMA, RSI, MACD, Bollinger Bands, VWAP
- `Portfolio.tr` — Mean-variance optimization, efficient frontier, Black-Litterman, risk parity
- `Risk.tr` — VaR, CVaR, Sharpe ratio, Sortino ratio, max drawdown, Kelly criterion
- `FactorModel.tr` — CAPM, Fama-French 3/5
- `YieldCurve.tr` — Nelson-Siegel, Svensson, cubic spline, bootstrap
- `MarketData.tr` — OHLCV, Tick, Trade, Quote
- `OrderBook.tr` — PriceLevel, Order, OrderBook

#### HFT (`hft/` — 5 files)
- `Backtest.tr` — BacktestEngine, BarData, BacktestResult
- `FixParser.tr` — FIX protocol parsing/building
- `Latency.tr` — LatencyTracker
- `OrderRouter.tr` — Order, Venue, OrderRouter
- `RiskManager.tr` — RiskManager

### 5.12 Machine Learning (`ml/` — 6 files)
- `Tensor.tr` — Tensor class
- `Layer.tr` — DenseLayer, DropoutLayer, BatchNormLayer, Conv2D, RNN, LSTM
- `Loss.tr` — MSE, MAE, cross-entropy, binary CE, hinge, Huber, KL divergence
- `Optimizer.tr` — SGD, Adam, AdamW, RMSProp, AdaGrad
- `Model.tr` — SequentialModel
- `DataLoader.tr` — Dataset, DataLoader

### 5.13 NLP (`nlp/` — 5 files)
- `Tokenizer.tr` — Word/sentence tokenization, stop word removal
- `Stemmer.tr` — Porter, Lancaster, Snowball stemmers
- `Distance.tr` — Levenshtein, Jaro, Jaro-Winkler, Soundex, Metaphone
- `Vectorize.tr` — Vocabulary, TfidfVectorizer, bag of words
- `Classifier.tr` — Naive Bayes classifier, sentiment analysis

### 5.14 Statistics (`statistics/` — 6 files)
- `Statistics.tr` — NormalDist, UniformDist, ExponentialDist, PoissonDist, BinomialDist, GammaDist, BetaDist; correlation, regression, t-tests, ANOVA, chi-squared, KS test, Mann-Whitney, Wilcoxon
- `Bootstrap.tr` — Bootstrap sampling
- `Kde.tr` — Kernel density estimation (Gaussian, Silverman/Scott bandwidth)
- `Mcmc.tr` — Metropolis-Hastings, Gibbs sampling, R-hat, ESS, autocorrelation
- `Survival.tr` — Kaplan-Meier, log-rank test, Cox regression
- `TimeSeries.tr` — ACF, PACF, ARIMA, exponential smoothing, Holt-Winters, seasonal decomposition

### 5.15 System & OS (`sys/` — ~7 files)
- `Os.tr` — DirEntry, 40+ OS operations (env, kill, chmod, umask, scandir, stat, access, popen, popenWrite, etc.)
- `Sys.tr` — args, env, exit, workingDir, sleep, platform, cpuCount, exec, pid, nanoTime
- `Platform.tr` — system, machine, release, version, processor, titrateVersion
- `Signal.tr` — Signal constants (SIGHUP, SIGINT, etc.), register, raise, defaultHandler, ignoreHandler
- `Selectors.tr` — Selector class for I/O multiplexing (select, selectWithEvents with tuples)
- `Gc.tr` — enable, disable, collect, isEnabled, getStats (returns pid, enabled, collections, etc.)
- `Warnings.tr` — warn, filterWarnings, simplefilter, resetFilters
- `Atexit.tr` — register, unregister, runExitHooks, handlerCount, clear

### 5.16 Other Modules

#### Algorithms (`algo/`, `algorithms/`)
- `algo/` — Graph algorithms (BFS, DFS, Dijkstra, Bellman-Ford, Floyd-Warshall, A*), max flow, MST (Kruskal, Prim), topological sort, SCC, heap algorithms, string algorithms (KMP, Rabin-Karp, Boyer-Moore, suffix array, Z-algorithm, Aho-Corasick)
- `algorithms/Algorithms.tr` — 60+ STL-style algorithms (sort, find, binarySearch, transform, reverse, rotate, shuffle, partition, accumulate, innerProduct, nextPermutation, etc.)

#### Data Structures
- `heapq/Heapq.tr` — heapify, heappush, heappop, nlargest, nsmallest, merge
- `bisect/Bisect.tr` — bisectLeft, bisectRight, insortLeft, insortRight

#### Utilities
- `functools/Functools.tr` — partial, reduce, cache, pipe, compose, curry, memoize, lruCache, totalOrdering, singledispatch, cachedProperty
- `itertools/Itertools.tr` / `ItertoolsSeq.tr` — combinations, permutations, product, chain, cycle, islice, zipLongest, count, repeat, starmap, groupby, pairwise, batched, tee
- `argparse/ArgumentParser.tr` — Argument, ArgumentGroup, ArgumentParser, Namespace
- `contextlib/Contextlib.tr` — ContextManager, ExitStack, closing, suppress
- `copy/Copy.tr` — shallowCopy, deepCopy
- `operator/Operator.tr` — add, sub, mul, truediv, mod, eq, ne, lt, le, gt, ge, not, and, or, itemGetter, attrGetter
- `dataclass/Dataclass.tr` — DataclassInfo, create, newInstance, toString, equals, copyWith, field, asdict, replace
- `fractions/Fraction.tr` — Fraction class, fromDouble, fromString
- `decimal/` — Decimal, RoundingMode, DecimalContext, quantize, exp, ln, log10, fma
- `uuid/Uuid.tr` — UUID class, uuid4, uuid1, uuid3, uuid5
- `i18n/Locale.tr` — Locale class
- `assay/` — TestCase, TestSuite, TestRunner
- `assert/Assert.tr` — assertTrue, assertEqual, assertNull, assertInRange, etc.

#### Time (`time/` — ~8 files)
- `DateTime.tr` — DateTime, TimeZone (with tzOffsetMinutes field, toUtc, isDst, strftime %z/%Z)
- `Time.tr` — now, sleep, millis, micros, nanos, monotonic, perfCounter
- `Duration.tr` — Duration (ofMillis, ofSeconds, ofMinutes, ofHours, ofDays)
- `Stopwatch.tr` — Stopwatch
- `Cron.tr` — CronExpression, next
- `BusinessCalendar.tr` — BusinessCalendar
- `DateRange.tr` — dateRange, periodAdd/Subtract, isoWeekDate, ordinalDate
- `Scheduler.tr` — ScheduledEvent, Scheduler
- `ZoneInfo.tr` — ZoneInfo, timezone presets (utc, eastern, central, mountain, pacific, gmt, cet, jst, cst, ist, aest)

#### Units (`units/` — 5 files)
- `Base.tr` — SI base units (Meter, Second, Kilogram, Kelvin, Mole, Ampere, Candela)
- `Derived.tr` — SI derived units (Joule, Newton, Pascal, Watt, Coulomb, Volt, Ohm, Farad, Henry, Tesla, Weber, Hertz, etc.)
- `Constants.tr` — 20+ physical constants (Boltzmann, Avogadro, Planck, speed of light, etc.)
- `UnitConverter.tr` — UnitTracker, convert, dimensionalAnalysis
- `SpecialUnits.tr` — CurrencyUnit, astronomical units, parsec, Planck units

#### Compression (`compression/` — 6 files)
- `Gzip.tr` — compress, decompress, isGzip
- `Zlib.tr` — compress, decompress, gzipCompress/Decompress
- `Lz4.tr` — compress, decompress, frame compress, xxhash
- `Zstd.tr` — compress, decompress, dictionary training, streaming
- `Tar.tr` — TarEntry, TarReader, TarWriter
- `ZipFile.tr` — ZipEntry, ZipFile, ZipWriter (read/extract/write require native support)

#### Database (`db/` — 2 files)
- `Sqlite.tr` — SqliteConnection, SqliteResultSet, Row, DatabaseError
- `SqliteExt.tr` — PreparedStatement, TransactionController, BlobReader, WAL mode, backup

#### Simulation (`sim/` — 4 files)
- `Simulation.tr` — SimEvent, Simulation
- `Process.tr` — SimProcess
- `Resource.tr` — SimResource
- `Monitor.tr` — Monitor

#### Geometry (`geom/` — 5 files)
- `ConvexHull.tr` — Graham scan, quickHull, hull area/perimeter
- `Delaunay.tr` — Delaunay triangulation, Voronoi diagram
- `Polygon.tr` — Area, centroid, convexity, point-in-polygon
- `SpatialIndex.tr` — KDTree, RTree, BVH
- `Spline.tr` — BSpline, NURBS, Bezier curves

#### Random (`random/` — 6 files)
- `Random.tr` — Random class with 15+ distributions
- `Prng.tr` — MT19937, PCG32, Xoshiro256 PRNGs
- `ContinuousDist.tr` — Normal, multivariate normal, Dirichlet, Wishart, Beta, Gamma, etc.
- `DiscreteDist.tr` — Multinomial, categorical, hypergeometric, Poisson, Zipf, etc.
- `QuasiRandom.tr` — Sobol, Halton, Latin hypercube
- `Sampling.tr` — choices, sample, shuffle, reservoir, rejection, importance

### 5.17 Data Files (`lib/tt/data/` — 40+ JSON files)

Organized across 24 subdirectories:
- `bio/` — codon_tables, restriction_enzymes, scoring_matrices
- `chem/` — forcefield_params, orbital_exponents, oxidation_states, periodic_table, thermochemistry
- `color/` — named_colors
- `crypto/` — hash_algorithms
- `datetime/` — holidays, timezones
- `decimal/` — rounding_modes
- `encoding/` — base64_alphabets, codecs, escape_maps, url_safe_chars
- `hft/` — fix_dictionary
- `html/` — entities
- `io/` — format_specs
- `lang/` — error_codes, numeric_limits
- `locale/` — cldr, month_names
- `logging/` — log_levels
- `materials/` — scattering_factors, space_groups
- `math/` — bessel_coefficients, erf_coefficients, lanczos_coefficients, quadrature_nodes
- `net/` — ip_ranges
- `nlp/` — sentiment_lexicon, stop_words
- `physics/` — constants
- `schemas/` — constants_schema, data_file_schema, entities_schema, periodic_table_schema
- `string/` — charsets
- `sys/` — platform_data, signals
- `unicode/` — combining_class, composition, decomposition, mirrored
- `units/` — conversions, si_units
- `uuid/` — namespaces
- `binary/` — struct_formats

---

## 6. Native Function Registry

### Registration Mechanism

Native functions are registered in `trc/src/bytecode/vm/natives/lookup.rs` via a `match` statement mapping string names to Rust function pointers.

### Statistics
- **350** registered native function names
- **344** unique native function implementations
- **3** alias mappings (Time_millis/Time_now → native_time_now, Double_parseDouble/Double_parse → native_double_parse_double)
- **26** native module files

### Full Native Function List by Module

#### Global Builtins (5)
`println`, `toString`, `parseInt`, `Ok`, `Err`

#### File I/O — File_ (17)
readFile, writeFile, readLines, open, readLine, write, close, seek, tell, readBytes, writeBytes, lastModified, setModified, flush, size, truncate, copy, delete

#### String — String_ (11)
split, trim, length, trimStart, trimEnd, startsWith, endsWith, padLeft, padRight, toUpperCase, toLowerCase, replace, fromCharCode, charAt

#### Integer/Double/Long (5)
Integer_parseOr, Double_parseDouble, Double_parse, Long_parseLong, TypeName_of

#### Path — Path_ (8)
join, exists, isFile, isDir, basename, dirname, extension, isSymlink

#### Directory — Dir_ (7)
list, create, remove, removeTree, walk, copy, move

#### System — Sys_ (7)
args, env, setEnv, exit, workingDir, setWorkingDir, sleep

#### Environment — Env_ (3)
get, set, vars

#### Filesystem — Fs_ (6)
exists, isFile, isDir, size, totalSpace, freeSpace

#### Process — Process_ (2)
id, args

#### OS — Os_ (40)
name, arch, family, cpuCount, userName, hostName, urandom, chmod, makedirs, symlink, readlink, kill, environ, umask, scandir, environMap, getpid, getcwd, chdir, getenv, setenv, unsetenv, system, uname, getppid, strerror, removedirs, renames, replace, link, utime, lstat, access, release, version

#### Signal — Signal_ (2)
register, raise

#### Titrate/GC (2)
Titrate_version, Gc_collect

#### Network — Net_ (10)
connect, send, receive, bind, accept, close, getLocalPort, getLocalAddress, getRemoteAddress, setTimeout

#### HTTP — Http_ (8)
get, post, put, delete, patch, head, setTimeout, setFollowRedirects

#### DNS — Dns_ (2)
lookup, reverseLookup

#### Time — Time_ (14)
now, sleep, format, getYear, getMonth, getDay, getHour, getMinute, getSecond, dayOfWeek, dayOfYear, monotonic, perfCounter, epochSeconds, nanos, millis

#### Regex — Regex_ (9)
match, find, replace, groupCount, findGroups, findWithFlags, matchWithFlags, fullMatch, subN

#### Math — Math_ (33)
sin, cos, tan, asin, acos, atan, atan2, ln, log10, log2, exp, pow, sqrt, cbrt, abs, absInt, floor, ceil, round, inf, nan, maxDouble, minDouble, maxInt, minInt, nextUp, nextDown, ulp, getExponent, scalb, random, negInf, fma

#### Random — Random_ (2)
seed, nextLong

#### JSON — Json_ (2)
parse, stringify

#### Hash — Hash_/Hasher_/Hmac_ (22)
md5, sha1, sha256, sha384, sha512, sha3_256, sha3_384, sha3_512, blake2b, blake2s, crc32, sha224, sha3_224, shake128, shake256, Hasher_new/update/digest/hexDigest/reset/close, Hmac_compareDigest

#### Encoding — Base64_/Hex_/Url_ (6)
Base64_encode, Base64_decode, Hex_encode, Hex_decode, Url_encode, Url_decode

#### Subprocess — Subprocess_ (3)
run, exec, popenWrite

#### Tempfile — Tempfile_ (1)
create

#### Thread — Thread_ (8)
spawn, spawnRunnable, join, sleep, yield, getId, currentId, detach

#### Mutex — Mutex_/RecursiveMutex_/SharedMutex_/OnceFlag_ (17)
Mutex_new/lock/unlock/tryLock, RecursiveMutex_new/lock/unlock/tryLock, SharedMutex_new/sharedLock/sharedUnlock/uniqueLock/uniqueUnlock/trySharedLock/tryUniqueLock, OnceFlag_new/callOnce

#### CondVar — CondVar_ (5)
new, wait, waitFor, notifyOne, notifyAll

#### Semaphore — Semaphore_ (5)
new, acquire, release, tryAcquire, availablePermits

#### Atomic — AtomicInt_/AtomicBool_/AtomicLong_/AtomicRef_ (24)
AtomicInt_new/get/set/fetchAdd/fetchSub/compareAndSwap/fetchOr/fetchAnd/fetchXor/exchange, AtomicBool_new/get/set/compareAndSwap, AtomicLong_new/get/set/fetchAdd/fetchSub/compareAndSwap, AtomicRef_new/get/set/compareAndSwap

#### Socket — Socket_/UdpSocket_ (31)
Socket_new/connect/bind/listen/accept/send/recv/close/setTimeout/setNoDelay, UdpSocket_new/bind/sendTo/recvFrom/close/setTimeout/lastSenderHost/lastSenderPort, Socket_getAddrInfo/inetPton/inetNtop/createConnection/createServer/getLocalAddress/getRemoteAddress/getLocalPort/getRemotePort/setReuseAddr/setBroadcast/setKeepAlive/setLinger

#### SSL — Ssl_ (8)
contextNew, connect, send, recv, close, peerCertificate, contextClose, getPeerCertHash

#### SQLite — Sqlite_ (14)
open, execute, query, close, lastInsertId, nextRow, getInt, getString, getDouble, columnCount, columnName, closeResult, executePrepared, backup

#### Mmap — Mmap_ (6)
open, close, get, set, size, flush

#### Zlib/Gzip (4)
Zlib_compress, Zlib_decompress, Gzip_compress, Gzip_decompress

---

## 7. Testing Infrastructure

### Test Suites

| Suite | Tests | Command |
|-------|-------|---------|
| Compiler/VM unit tests | 574 | `cargo test --lib` |
| Stdlib parse test | 1 | `cargo test --test stdlib_parse_test` |
| Stdlib integration tests | 53 | `cargo test --test stdlib_test` |
| Mega test (end-to-end) | 1 | `cargo test --test mega_test` |
| Mega test 02 | 1 | `cargo test --test mega_test_02` |
| Mega test 03 | 1 | `cargo test --test mega_test_03` |
| Incr/decr/ternary | 1 | `cargo test --test incr_decr_ternary` |
| Do-while/subprocess | 1 | `cargo test --test do_while_subprocess_test` |
| Compound assignment | 1 | `cargo test --test compound_assign_test` |

**Total: 634 tests passing**

### Test Projects

- **`stdlib_test/`** — Multi-file project testing stdlib modules (Titrate.toml + src/main.tr)
- **`mega_test_02/`** — Multi-file project with data, shapes, sort, wordfreq modules + test data files
- **`mega_test_03/`** — Multi-file project with forcefield module
- **`mega_test.tr`** / **`mega_test_03.tr`** — Single-file end-to-end tests

### Build & Test Commands

```bash
cargo test --lib              # Compiler/VM unit tests (574)
cargo test --test stdlib_test # Stdlib integration tests (53)
cargo test --test mega_test   # End-to-end mega test (1)
cargo build                   # Build compiler + pipette
```

---

## 8. Build Tools & Package Manager

### Pipette (`pipette/` — 15 .rs files)

Titrate's build tool and package manager (v0.2.0):

| Module | Purpose |
|--------|---------|
| `main.rs` | CLI entry point |
| `lib.rs` | Library exports |
| `build.rs` | Build system |
| `run.rs` | Run .tr programs |
| `test_runner.rs` | Test runner |
| `bench.rs` | Benchmarking |
| `clean.rs` | Clean build artifacts |
| `config.rs` | Titrate.toml configuration |
| `deps.rs` | Dependency management |
| `doc.rs` | Documentation generation |
| `format.rs` | Code formatter |
| `lint.rs` | Linter |
| `project.rs` | Project scaffolding |
| `serialize.rs` | Serialization |
| `watch.rs` | File watcher for live reload |

### Project Configuration (`Titrate.toml`)

Multi-file projects use a `Titrate.toml` manifest file for configuration.

---

## 9. Documentation System

### VitePress Documentation (`docs/` — 151 .md files)

- **Site URL:** https://richie-rich90454.github.io/titrate/
- **Custom syntax highlighter:** `titrate-lang.js` for Titrate code blocks
- **Theme:** GitHub light/dark with line numbers

### Documentation Sections

| Section | Files | Content |
|---------|-------|---------|
| `docs/guide/` | 37 | Getting started, variables, functions, control-flow, classes, interfaces, enums, tuples, generics, pattern-matching, error-handling, closures, operator-overloading, iterators, ranges, ownership, modules, file-io, scientific-computing, bio/physics/ml/3d-graphics/hft/simulation guides, optimizations, syntax-sugar, stdlib, build-tool, pipette, cookbook, architecture, contributing, migration-from-c, migration-from-ecmascript |
| `docs/reference/` | 4 | Lexer tokens, grammar, types, memory model |
| `docs/stdlib/` | 109 | One doc per stdlib module |
| `docs/index.md` | 1 | Landing page |

### GitHub Linguist Support (`linguist/`)
- `languages.yml` — Language definition
- `titrate.tmLanguage.yml` — TextMate grammar for syntax highlighting

---

## 10. Known Limitations & Stubs

### Native Dependency Stubs (require external Rust crates)

| Module | Missing Dependency | Status |
|--------|-------------------|--------|
| SSL/TLS (`ssl.rs`) | `native-tls` | All functions return error stubs |
| SQLite (`sqlite.rs`) | `rusqlite` | All functions return error stubs |
| Mmap (`mmap.rs`) | `memmap2` | All functions return error stubs |
| Zlib/Gzip (`zlib.rs`) | `flate2` | All functions return error stubs |
| ZipFile (`ZipFile.tr`) | zip archive support | readEntry/extractAll/write return errors |

### VM Architecture Limitations

- **Thread_spawn:** Creates a thread but does not execute the passed closure. `Value` contains `Rc<>` which is not `Send`, so Titrate closures cannot safely cross thread boundaries. Use `ThreadPoolExecutor` or `async()` instead.
- **Signal handling:** `Signal_register` and `Signal_raise` return errors on all platforms. Real signal handling requires routing OS signals back into Titrate handlers.

---

## 11. Future Directions

### High Priority

1. **Add native dependencies** for SSL (`native-tls`), SQLite (`rusqlite`), Mmap (`memmap2`), Zlib (`flate2`) to enable full functionality
2. **Thread-safe Value type:** Migrate from `Rc<>` to `Arc<Mutex<>>` to enable true multi-threading with `Thread_spawn`
3. **Signal handling:** Implement `libc::signal`/`libc::sigaction` registration with callback routing

### Medium Priority

4. **Compiler optimizations:** More aggressive constant folding, inlining, tail call optimization
5. **Error messages:** Improve compiler error messages with better source locations and suggestions
6. **Standard library testing:** Increase test coverage for stdlib modules
7. **Documentation:** Complete API reference for all stdlib modules
8. **Package registry:** Implement package publishing and distribution for pipette

### Low Priority

9. **JIT compilation:** Just-in-time bytecode compilation for performance
10. **Foreign Function Interface (FFI):** Direct C function calls from Titrate
11. **Async/await:** Native async/await syntax for asynchronous programming
12. **Pattern matching enhancements:** More sophisticated pattern matching (guards, ranges, etc.)
13. **Macro system:** Compile-time code generation

### Recently Resolved

The following items were previously listed as limitations and have been resolved:

- **String interpolation:** `${expr}` syntax now implemented in the parser (desugars to `Binary(Add)` chain)
- **Full Barnes-Hut:** Octree-based N-body simulation with theta opening-angle criterion implemented
- **IPv6 support:** 128-bit arithmetic using `vast` type for IPv6 network operations
- **Arbitrary-order Butterworth filter:** Cascaded biquad sections with polynomial convolution
- **Denman-Beavers matrix square root:** Full iteration with Gauss-Jordan matrix inverse
- **CP/Tucker ALS decomposition:** Full MTTKRP + Gram matrix linear system solve
- **Windows platform natives:** `Os.umask`, `Os.getppid`, `Fs.totalSpace`, `Fs.freeSpace` all implemented via Windows API
- **Os.popen write mode:** `Subprocess_popenWrite` native pipes input to command stdin
- **Compiler warnings:** All 22 warnings resolved (0 warnings remaining)

---

## Appendix A: File Counts

| Category | Count |
|----------|-------|
| Total .tr files | 439 |
| Stdlib .tr files | 419 |
| Example .tr files | 15 |
| Test .tr files | 5 |
| Total .rs files | 107 |
| trc .rs files | 92 |
| pipette .rs files | 15 |
| Documentation .md files | 151 |
| JSON data files | 40+ |
| Git commits | 1296 |
| Native functions | 344 |
| Stdlib subdirectories | 65 |

## Appendix B: Naming Conventions

| Element | Convention | Example |
|---------|-----------|---------|
| Classes | PascalCase | `ArrayList`, `HashMap` |
| Interfaces | PascalCase | `Comparable`, `Iterable` |
| Enums | PascalCase | `Color`, `JsonValue` |
| Functions | camelCase | `parseInt`, `sqrt` |
| Methods | camelCase | `size()`, `add()` |
| Variables | camelCase | `itemCount`, `firstName` |
| Constants | UPPER_SNAKE_CASE | `MAX_SIZE`, `DEFAULT_PORT` |
| Type parameters | Single uppercase letter | `<T>`, `<T, R>`, `<K, V>` |

## Appendix C: Math Module Split

The `Math` module is split across three files. Functions must be called on the correct module:

- **`Math`** — Constants (PI, E, INF, NAN), abs, floor, ceil, round, min, max, gcd, lcm, comb, factorial, erf, gamma, lgamma, random
- **`MathAdvanced`** — sqrt, pow, exp, ln, log2, log10, cbrt, hypot, fma, nextUp, nextDown, ulp, getExponent, scalb
- **`MathTrig`** — sin, cos, tan, asin, acos, atan, atan2, sinh, cosh, tanh

**CRITICAL:** Calling `Math.sqrt()` or `Math.sin()` will fail at runtime — these functions do not exist on the `Math` module.
