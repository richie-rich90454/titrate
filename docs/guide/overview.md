---
description: Titrate is a systems programming language with a Rust compiler, a bytecode virtual machine, and an LLVM native backend. Overview of the type system, generics, closures, ownership, pattern matching, and standard library.
---

# What Is Titrate? A Systems Programming Language Overview

Titrate is a systems programming language for writing precise, memory-safe software with expressive syntax. Its toolchain is written in Rust: the `trc` compiler analyzes `.tr` source files and either emits bytecode for a stack-based virtual machine or generates a native executable through an LLVM backend. The language supports static types, compile-time generics, closures, ownership with region-based allocation, algebraic data types, and `Result`-based error handling.

Titrate ships a large standard library named `tt`. It covers collections, file I/O, networking, serialization, concurrency, and cryptography, along with scientific domains such as linear algebra, chemistry, physics, statistics, signal processing, and machine learning. The same source runs on the bytecode VM or compiles natively with `trc --native --release`.

## Titrate at a Glance

| Capability | What it means | Details |
|------------|---------------|---------|
| Rust compiler and bytecode VM | Analyze source once, then run it on a portable stack-based VM | [Compiler architecture](./architecture) |
| LLVM native backend | Emit a standalone native executable from the same source | [Native backend introduction](./native-intro) |
| Precise types | From `byte` and `half` to arbitrary-precision `vast` and 128-bit `quad` | [Type reference](../reference/types) |
| Generics | Monomorphized at compile time with no runtime type checks | [Generics guide](./generics) |
| Closures | Anonymous functions that capture their environment | [Closures guide](./closures) |
| Ownership and regions | Move semantics, borrowing, and region-based allocation without a tracing garbage collector | [Ownership guide](./ownership) |
| Pattern matching | `switch` on enum variants with payload extraction and exhaustiveness diagnostics | [Pattern matching guide](./pattern-matching) |
| Result-based errors | `Result<T, E>` and `?` propagation instead of exceptions | [Error handling guide](./error-handling) |
| Standard library | Collections, I/O, JSON, XML, TCP, HTTP, crypto, and scientific modules | [Standard library guide](./stdlib) |

## How Titrate Works

A Titrate program moves through a conventional compiler pipeline:

1. The lexer turns characters into tokens.
2. The parser builds an abstract syntax tree.
3. The analyzer resolves names, checks types, and tracks closures and ownership.
4. From there, the bytecode compiler emits instructions for the virtual machine, or the LLVM backend lowers the program to a native executable.

The bytecode path is the default for quick iteration. Native compilation is useful for deployment and for programs that benefit from LLVM optimization. Both paths are exercised by the repository test suites; see the [build and test instructions](./building-from-source).

## Language Basics

```titrate
import tt::util::ArrayList;

// Variables: let infers the type, var takes an explicit type
let name = "Titrate";
var count: int = 0;

let numbers = new ArrayList<int>();
numbers.add(count);

// Functions use name: Type parameters and an explicit return type
public fn greet(name: string): void {
    io::println("Hello, " + name);
}

// Classes use fn init() constructors
public class Point {
    public double x;
    public double y;

    public fn init(x: double, y: double) {
        this.x = x;
        this.y = y;
    }
}

// Generics are specialized per type at compile time
public class Box<T> {
    public T value;
    public fn init(value: T) { this.value = value; }
}

// Recoverable errors are values
fn parseAndDouble(text: string): Result<int, string> {
    let value = Integer.parseInt(text);
    if (value == 0 && text != "0") {
        return err("not a number");
    }
    return ok(value * 2);
}

// Ranges and iterators
for (i in 0..10) {
    io::println(Integer.toString(i));
}
```

For a line-by-line introduction, read [getting started with Titrate](./getting-started).

## Who Should Use Titrate

Titrate is a good fit when a program needs control over memory layout and types but would benefit from a higher-level syntax:

- **Systems programmers** who want ownership semantics, explicit types, and predictable code generation.
- **Scientific and engineering developers** who need matrices, N-dimensional arrays, statistics, and simulation code in the standard library rather than in a separate dependency stack.
- **Developers moving from C, C++, or Python** who want familiar optional syntax while adopting static types, generics, and explicit error handling.
- **Educators and experimenters** who want to inspect both a bytecode VM and an LLVM backend in the same open-source project.

## How Titrate Compares

The table describes language design choices, not benchmark results.

| Feature | Titrate | C++ | Python |
|---------|---------|-----|--------|
| Memory management | Ownership, borrowing, regions | Manual or smart pointers | Garbage collector |
| Generic code | Compile-time monomorphization | Templates | Runtime dynamic typing |
| Failure handling | `Result<T, E>` and pattern matching | Exceptions and error codes | Exceptions |
| Runtime options | Bytecode VM and optional native executable | Native executable | Interpreter and third-party runtimes |
| Scientific computing | Included in `tt` | External libraries | External packages |
| Function syntax | `fn add(a: int, b: int): int` | `int add(int a, int b)` | `def add(a, b)` |

Migration notes are available for [developers coming from C](./migration-from-c) and [developers coming from ECMAScript](./migration-from-ecmascript).

## Frequently Asked Questions

### What is Titrate used for?

Titrate targets systems programs that need precise types and explicit memory behavior, as well as scientific applications that want numerical and domain libraries close at hand. It runs on a bytecode VM for development and compiles to a native executable for deployment.

### Does Titrate use a garbage collector?

No tracing garbage collector is required for ordinary memory management. Titrate provides `Owned<T>` move semantics, borrowing, and `region` blocks, and the analyzer reports values that escape their region. The standard library also includes an optional `Gc` utility for programs that want explicit collection hooks.

### Can Titrate compile to native executables?

Yes. The LLVM backend is invoked with `trc --native`, and `trc --native --release` enables optimized code generation. See [compiling with the LLVM native backend](./native-build) for flags and platform notes.

### Is Titrate compatible with C or Python syntax?

Titrate uses its own canonical syntax, with `fn` declarations and `name: Type` parameters. It also accepts a limited set of C-style declarations and Java-style method signatures as syntax sugar. See [syntax sugar](./syntax-sugar) for the supported forms.

### What replaces exceptions?

Recoverable failures use `Result<T, E>` with `ok` and `err` constructors, `isOk()` checks, and the `?` propagation operator. The language also supports `throw`, `try`, and `catch` for unrecoverable errors.

### Does Titrate have a build tool?

Yes. `pipette` creates projects, builds and runs code, manages dependencies, formats sources, runs tests and benchmarks, and generates documentation. See the [pipette build tool guide](./build-tool).

### Where is the standard library?

The standard library source lives in `lib/tt/` and is imported with paths such as `import tt::util::ArrayList;`. Individual module references are listed in the [standard library documentation](../stdlib/lang).

### Is Titrate open source?

Yes. Titrate is published under the [Apache License 2.0](https://github.com/richie-rich90454/titrate/blob/main/LICENSE). Contributions to the compiler, virtual machine, standard library, and documentation are welcome through the [contributing guide](./contributing).

## Next Steps

- Install the toolchain with [building Titrate from source](./building-from-source).
- Write your first program with the [getting started guide](./getting-started).
- Explore types with the [type reference](../reference/types) and the [memory model](../reference/memory-model).
- Browse the [full capability summary](../SUMMARY) for a generated inventory of modules and native functions.
