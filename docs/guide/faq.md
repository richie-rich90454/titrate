# Frequently Asked Questions

Common questions about Titrate, answered concisely. If you don't find what you're looking for, check the [guide](./getting-started) or open an issue on GitHub.

## What is Titrate?

Titrate is a statically typed, compiled programming language designed for clarity and safety. It compiles to bytecode that runs on its own virtual machine. Titrate emphasizes readable syntax, compile-time error checking, and a type system that catches bugs before your code runs.

## How is Titrate different from Rust / Python / C++?

| Aspect | Titrate | Rust | Python | C++ |
|--------|---------|------|--------|-----|
| Typing | Static, inferred | Static, inferred | Dynamic | Static |
| Memory | GC + ownership | Ownership + borrow checker | GC | Manual |
| Compilation | Bytecode + VM | Native | Interpreted | Native |
| Error handling | Result types | Result types | Exceptions | Exceptions |
| Null safety | Optional / Variant | `Option<T>` | None | Pointers |
| Generics | Monomorphization | Monomorphization | Duck typing | Templates |

Titrate takes inspiration from Rust's `Result` type and ownership model, but uses a garbage collector alongside ownership hints rather than a full borrow checker. It compiles to bytecode (like Java) rather than native code (like Rust/C++).

## Does Titrate have garbage collection?

Yes. Titrate uses a garbage collector for memory management. However, the language also supports ownership annotations (`let` for immutable, `var` for mutable) that communicate intent and help the optimizer. The GC handles the actual memory reclamation — you don't need to manually free anything.

## Can I use Titrate for web development?

Titrate can be used for backend web services — its standard library includes HTTP client and server modules, JSON parsing, and file I/O. For frontend web development, Titrate does not currently compile to JavaScript or WebAssembly, so you'd use it on the server side only.

## How does ownership work?

Titrate's ownership system is advisory, not enforced like Rust's borrow checker. Variables declared with `let` are immutable by default — you can't reassign them. Variables declared with `var` are mutable. This helps you write code that's easier to reason about, but the garbage collector handles the actual memory lifecycle. See [Ownership](./ownership) for the full guide.

## What's the difference between `let` and `var`?

- **`let`** declares an immutable variable — once assigned, it can't be changed.
- **`var`** declares a mutable variable — it can be reassigned.
- **`const`** declares a compile-time constant — the value must be known at compile time.

```titrate
let name: string = "Alice";   // immutable
var count: int = 0;           // mutable
const MAX: int = 100;         // compile-time constant
```

Use `let` by default. Only reach for `var` when you genuinely need to reassign. Use `const` for values that should never change and can be computed at compile time.

## Why `name: Type` instead of `Type name`?

Titrate uses `name: Type` syntax for declarations, parameters, and return types. This follows the convention of languages like TypeScript, Swift, and Kotlin. It has practical benefits:

- **Readability** — the name comes first, which is what you care about most.
- **Consistency** — the colon always separates the name from the type, whether it's a variable, parameter, or return type.
- **Type inference** — when the type is omitted, the syntax still reads naturally: `let x = 5`.

## Does Titrate support exceptions?

No. Titrate uses `Result<T, E>` types for error handling instead of exceptions. This means the possibility of failure is visible in the function signature — you can't accidentally ignore an error. Use `ok(value)` for success, `err(message)` for failure, and the `?` operator to propagate errors concisely. See [Error Handling](./error-handling) for the full guide.

## How do I handle null values?

Titrate doesn't have a `null` keyword in the traditional sense. Instead, use `Variant` or `Optional` from the standard library to represent the absence of a value explicitly:

```titrate
let maybeName: Variant = Variant.of("Alice");
if (maybeName.isPresent()) {
    io::println(maybeName.getValue().asStr());
}
```

This forces you to check for the presence of a value before using it, eliminating null pointer exceptions at compile time.

## Can I call C libraries from Titrate?

Titrate supports a foreign function interface (FFI) through native function declarations. The VM exposes native functions using the `ModuleName_functionName` convention. However, directly calling arbitrary C libraries requires writing a native binding — this is an advanced feature and the API for creating custom native bindings is still evolving.

## How fast is Titrate?

Titrate compiles to bytecode that runs on a stack-based virtual machine. Performance is comparable to other bytecode-compiled languages. The compiler performs optimizations like constant folding and dead code elimination. Generic code is monomorphized (like Rust and C++), so there's no runtime overhead for generics. For most applications, Titrate's performance is more than adequate. If you need bare-metal speed, a native-compiled language like Rust or C++ would be more appropriate.

## What platforms does Titrate support?

Titrate runs on any platform where its VM can be compiled. The reference implementation is written in Rust, so it supports:

- **Windows** (x86_64)
- **macOS** (x86_64, Apple Silicon)
- **Linux** (x86_64)

The VM is portable, and adding support for additional platforms is primarily a matter of compiling the Rust codebase for the target.

## How do I contribute to Titrate?

We welcome contributions! See the [Contributing Guide](./contributing) for the full process. In short:

1. Fork the repository
2. Create a feature branch
3. Make your changes (compiler, stdlib, or docs)
4. Run `cargo test --lib` and `cargo test --test stdlib_test`
5. Submit a pull request

## What's the difference between `Ok`/`Err` and `ok()`/`err()`?

Both create `Result` values, but they differ in usage:

- **`ok(value)`** and **`err(message)`** are constructor functions — use them to create `Result` instances in your code.
- **`Ok` and `Err`** are the enum variant names used in pattern matching and type-level discussion.

```titrate
let result: Result<int, string> = ok(42);     // constructing
let error: Result<int, string> = err("fail"); // constructing

switch result {
    case Ok(v) => io::println(Integer.toString(v));  // pattern matching
    case Err(e) => io::println(e);                    // pattern matching
}
```

In practice, use `ok()`/`err()` when building results and `Ok`/`Err` when destructuring them.

## Does Titrate support concurrency?

Titrate has basic concurrency support through the `concurrent` standard library module, which provides futures and async operations. However, Titrate does not currently have an async/await syntax or a full-featured concurrency runtime. Concurrency is an active area of development.

## How do I install third-party packages?

Titrate uses `pipette` as its package manager. To install a package:

```bash
pipette install package-name
```

To add a dependency to your project:

```bash
pipette add package-name
```

Packages are hosted on the Titrate package registry. You can also reference local packages by path in your project configuration. See [Build Tool](./build-tool) for more details.

## Where can I learn more?

- [Getting Started](./getting-started) — install Titrate and write your first program
- [Language Guide](./variables) — comprehensive language reference
- [Standard Library](./stdlib) — what's available out of the box
- [Contributing](./contributing) — help improve Titrate
