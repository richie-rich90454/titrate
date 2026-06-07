# Getting Started

Welcome to Titrate, a systems programming language with precise type systems, ownership semantics, and expressive pattern matching.

## Installation

```bash
cargo build --release
```

The compiler binary is `trc`.

## Hello World

```titrate
public fn main(): void {
    io::println("Hello, Titrate!");
}
```

## Running a Program

```bash
trc hello.tr
```

## Using Pipette

Titrate ships with **pipette**, a build tool and package manager. For projects larger than a single file, pipette manages compilation, dependencies, and execution.

```bash
pipette new myproject    # create a new project
pipette build            # compile to bytecode
pipette run              # build and run
```

See [Build Tool](./pipette) for the full pipette guide.

## What's Next?

- [Variables](./variables) — `let`, `var`, and `const` declarations
- [Functions](./functions) — defining and calling functions, including generic functions
- [Classes](./classes) — object-oriented programming, inheritance, and interfaces
- [Enums](./enums) — algebraic data types and pattern matching
- [Generics](./generics) — type parameters, constraints, and monomorphization
- [Modules](./modules) — multi-file projects and imports
- [File I/O](./file-io) — reading and writing files
