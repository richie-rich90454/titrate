# Titrate

[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust 1.96+](https://img.shields.io/badge/Rust-1.96%2B-orange.svg)](https://www.rust-lang.org/)
[![trc 0.3.0-alpha](https://img.shields.io/badge/trc-0.3.0--alpha-green.svg)](trc/Cargo.toml)
[![Documentation](https://img.shields.io/badge/docs-online-blueviolet.svg)](https://richie-rich90454.github.io/titrate/)
[![Pull requests welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](docs/guide/contributing.md)

**Titrate is a systems programming language with a Rust compiler, a bytecode virtual machine, and an LLVM native backend. It provides static types, compile-time generics, closures, ownership with region-based allocation, pattern matching, and a large standard library with built-in scientific computing.**

Titrate is designed for programs that need precise control over data representation and memory without giving up expressive syntax. Write the same source for a quick bytecode run or compile it to a standalone native executable. The standard library covers collections, I/O, networking, serialization, and domain areas such as linear algebra, molecular dynamics, signal processing, and machine learning.

[Read the Titrate documentation](https://richie-rich90454.github.io/titrate/) | [Start with the installation guide](docs/guide/getting-started.md) | [Learn what Titrate is](docs/guide/overview.md)

## Quick Start

### Prerequisites

- **Rust 1.96 or later**, installed with [rustup](https://rustup.rs)
- **LLVM 22.1** development files, required by the native backend
  - Windows: install LLVM 22.1 into `C:\Program Files\LLVM\` and add it to `PATH`
  - macOS: `brew install llvm@22`
  - Linux: install `llvm-22-dev` and `libclang-22-dev`
- **MSVC Build Tools** on Windows: Visual Studio 2022 with the "Desktop development with C++" workload

See [building Titrate from source](docs/guide/building-from-source.md) for platform-specific setup details.

### Build and run

```bash
git clone https://github.com/richie-rich90454/titrate.git
cd titrate
cargo build --release
```

Create `hello.tr`:

```titrate
public fn main(): void {
    io::println("Hello, Titrate!");
}
```

Run it on the bytecode VM, or compile a native executable with LLVM:

```bash
# Bytecode VM (binary is in target/release from the workspace root)
./target/release/trc hello.tr          # Linux and macOS
.\target\release\trc.exe hello.tr      # Windows PowerShell

# Native executable
./target/release/trc --native --release hello.tr
```

For multi-file projects, use the `pipette` build tool:

```bash
./target/release/pipette new myproject
cd myproject
../target/release/pipette run
```

### Run the tests

```bash
cargo test --lib                # compiler and VM unit tests
cargo test --test stdlib_test   # standard library integration tests
cargo test --test mega_test     # end-to-end tests
```

More targets are listed in [testing and continuous integration](docs/guide/contributing.md).

## Feature Highlights

### Precise type system

Primitive types range from `byte` and `half` to `vast` (arbitrary precision) and `quad` (128-bit floating point). Casting is explicit with `as`; `is` checks a value's type.

```titrate
let count: int = 42;
let ratio: double = count as double;
let label: string = "pi";
```

### Compile-time generics and closures

Generic functions and classes are monomorphized during compilation. Closures capture their environment and can be stored in variables or passed as arguments.

```titrate
import tt::util::ArrayList;

public fn identity<T>(value: T): T {
    return value;
}

let square = fn(x: int): int => x * x;
let numbers = new ArrayList<int>();
numbers.add(square(5));
```

### Ownership and regions

Move semantics, borrowing, and region-based allocation give memory safety without a tracing garbage collector. Region escape analysis reports values that would outlive their region.

```titrate
let owned: Owned<int> = Owned(5);
let moved = owned;          // owned is moved

region scratch {
    let value = scratch.alloc(42);
}
```

### Explicit error handling

Use `Result<T, E>` for recoverable failures and `?` to propagate them. Pattern matching on enums reports non-exhaustive switches.

```titrate
fn parseAndDouble(text: string): Result<int, string> {
    let value = Integer.parseInt(text);
    if (value == 0 && text != "0") {
        return err("not a number");
    }
    return ok(value * 2);
}
```

### Operator overloading and pattern matching

Define operators on your own types and branch on enum payloads with `switch` and `case`.

```titrate
public fn operator+(other: Vec2): Vec2 {
    return new Vec2(this.x + other.x, this.y + other.y);
}

switch (result) {
    case Ok(value) => io::println(Integer.toString(value));
    case Err(message) => io::println(message);
}
```

### Scientific computing in the standard library

The `tt` standard library includes linear algebra (`Matrix`, SVD, QR, eigenvalues), N-dimensional arrays, chemistry and molecular dynamics, signal processing, statistics, image processing, and machine learning.

```titrate
import tt::chem::Atom;
import tt::math::MathAdvanced;

public fn ljForce(a: Atom, b: Atom): double {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let dz = b.z - a.z;
    let r2 = dx * dx + dy * dy + dz * dz;
    let r6 = MathAdvanced.pow(r2, 3.0);
    let r12 = r6 * r6;
    let sig6 = MathAdvanced.pow(a.sig, 6.0);
    let sig12 = sig6 * sig6;
    return 24.0 * a.eps * (2.0 * sig12 / r12 - sig6 / r6) / r2;
}
```

See the [standard library guide](docs/guide/stdlib.md) and the [scientific computing guide](docs/guide/scientific-computing.md) for the full set of modules.

## How Titrate Compares

| Concern | Titrate | C++ | Python |
|---------|---------|-----|--------|
| Memory safety | Ownership, borrowing, regions | Manual memory management | Garbage collector |
| Generics | Monomorphized at compile time | Templates | Runtime duck typing |
| Error handling | `Result<T, E>` and pattern matching | Exceptions and error codes | Exceptions |
| Execution | Bytecode VM plus optional LLVM native executable | Compiled native | Interpreter or third-party runtimes |
| Scientific computing | Included in `tt` standard library | External libraries | External packages |
| Parameter syntax | `fn name(value: int): int` | `int name(int value)` | `def name(value)` |

Titrate also tolerates C-style declarations and Java-style method signatures as syntax sugar; see the [syntax sugar reference](docs/guide/syntax-sugar.md).

## Project Status

The language is functional and under active development. `trc` is at version `0.3.0-alpha` and `pipette` is at `0.2.0`. The repository ships compiler and VM unit tests, standard library integration tests, end-to-end tests, and native-backend parity tests; continuous integration builds, tests, and runs the demos on Linux.

For a generated inventory of implemented modules, native functions, and tests, see the [Titrate capability summary](docs/SUMMARY.md). For release notes, see the [change log](docs/CHANGELOG.md). The project is licensed under [Apache-2.0](LICENSE).

## Documentation

- [What is Titrate?](docs/guide/overview.md) - language overview, use cases, and frequently asked questions
- [Getting started](docs/guide/getting-started.md) - installation and the first program
- [Building from source](docs/guide/building-from-source.md) - prerequisites and build commands for each platform
- [Language guide](docs/guide/variables.md) - variables, functions, classes, enums, and control flow
- [Ownership and regions](docs/guide/ownership.md) - memory safety model
- [LLVM native backend](docs/guide/native-intro.md) - native compilation and C interoperability
- [Standard library reference](docs/guide/stdlib.md) - modules shipped with Titrate
- [Frequently asked questions](docs/guide/faq.md) - common questions and troubleshooting

The same material is published at the [Titrate documentation site](https://richie-rich90454.github.io/titrate/).

## Contributing

Contributions to the compiler, virtual machine, standard library, documentation, and examples are welcome. Start with the [contributing guide](docs/guide/contributing.md), then open an issue or a pull request on [GitHub](https://github.com/richie-rich90454/titrate). Bug reports with a small `.tr` reproduction are especially useful.

## License

Titrate is released under the [Apache License 2.0](LICENSE).
