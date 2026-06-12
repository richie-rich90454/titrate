# Getting Started

Welcome to Titrate! If you've ever wished for a language that combines the safety of strong types and ownership with the expressiveness of pattern matching and algebraic data types — you're in the right place. Titrate is designed to give you confidence in your code: the compiler catches whole classes of bugs before your program ever runs, while keeping the syntax clean and approachable.

Whether you're coming from Java, Python, Rust, or somewhere else entirely, this guide will walk you through everything you need to get up and running. Let's dive in!

## Installation

Build the compiler from source using Cargo:

```bash
cargo build --release
```

The compiler binary is `trc`. Once the build completes, you'll find it in `target/release/trc`. You can add it to your `PATH` or invoke it directly.

::: tip
Make sure you have [Rust and Cargo installed](https://rustup.rs/) before building. Titrate's compiler is written in Rust, so you'll need the Rust toolchain.
:::

## Your First Program

There's no better way to learn a language than writing code. Let's create the classic "Hello, World!" program — and walk through every piece of it.

Create a file called `hello.tr` and add the following:

```titrate
public fn main(): void {
    io::println("Hello, Titrate!");
}
```

Let's break this down line by line:

- **`public fn main(): void`** — This declares the entry point of your program. Every Titrate program starts executing from `main()`. The `: void` part tells the compiler this function doesn't return a value.
- **`io::println(...)`** — This calls the `println` function from the `io` module. The `::` syntax is used to access module members — you'll see this pattern a lot.
- **`"Hello, Titrate!"`** — A string literal. Titrate uses `string` (lowercase) as the type name.

## Running a Program

Save your file and run it with the compiler:

```bash
trc hello.tr
```

You should see `Hello, Titrate!` printed to your terminal. Congratulations — you've just written and run your first Titrate program!

::: tip Try It Yourself
Before moving on, try modifying the program to get comfortable:
- Change the message to print your name instead.
- Add a second `io::println` call on the next line to print something else.
- Try printing a number: `io::println(Integer.toString(42));`
:::

## Using Pipette

Titrate ships with **pipette**, a build tool and package manager. For projects larger than a single file, pipette manages compilation, dependencies, and execution.

```bash
pipette new myproject    # create a new project
pipette build            # compile to bytecode
pipette run              # build and run
```

When you run `pipette new myproject`, it creates a project directory with a standard layout, including a configuration file and a `src/` directory for your code. This is the recommended way to structure anything beyond a quick script.

See [Build Tool](./build-tool) for the full pipette guide.

## What You'll Learn

By the end of this guide, you'll be comfortable with the core building blocks of Titrate:

- **Variables** — How to declare immutable, mutable, and compile-time constant values
- **Functions** — Defining and calling functions, including generic functions
- **Classes** — Object-oriented programming, constructors, and interfaces
- **Enums** — Algebraic data types that model your domain precisely
- **Pattern Matching** — Exhaustive, type-safe branching on enum values
- **Ownership** — Memory safety without garbage collection
- **Generics** — Writing code that works across types
- **Modules** — Organizing multi-file projects with imports

## What's Next?

- [Variables](./variables) — `let`, `var`, and `const` declarations
- [Functions](./functions) — defining and calling functions, including generic functions
- [Classes](./classes) — object-oriented programming, inheritance, and interfaces
- [Enums](./enums) — algebraic data types and pattern matching
- [Generics](./generics) — type parameters, constraints, and monomorphization
- [Modules](./modules) — multi-file projects and imports
- [File I/O](./file-io) — reading and writing files
