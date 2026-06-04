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

## What's Next?

- [Variables](./variables) — `let`, `var`, and `const` declarations
- [Functions](./functions) — defining and calling functions
- [Classes](./classes) — object-oriented programming
- [Enums](./enums) — algebraic data types and pattern matching
