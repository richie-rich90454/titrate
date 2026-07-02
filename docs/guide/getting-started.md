# Getting Started

Welcome to Titrate! If you've ever wished for a language that combines the safety of strong types and ownership with the expressiveness of pattern matching and algebraic data types — you're in the right place. Titrate is designed to give you confidence in your code: the compiler catches whole classes of bugs before your program ever runs, while keeping the syntax clean and approachable.

Whether you're coming from Java, Python, Rust, or somewhere else entirely, this guide will walk you through everything you need to get up and running. Let's dive in!

## Choose Your Path

**Two paths are available depending on your setup:**

- **Fast Path (2-3 minutes)** — If you already have Rust, LLVM, and Git installed, skip to [Fast Path for Experienced Developers](#fast-path-for-experienced-developers).
- **Complete Installation (15-30 minutes)** — If you're setting up from scratch, follow the [Complete Installation for Newcomers](#complete-installation-for-newcomers) section below.

## Fast Path for Experienced Developers

If you have Rust 1.70+, LLVM development files, and Git installed, you can build and run your first program in 2-3 minutes:

```bash
# Clone and build (1-2 minutes)
git clone https://github.com/richie-rich90454/titrate.git
cd titrate
cargo build --release

# Create and run your first program (1 minute)
echo 'public fn main(): void { io::println("Hello, Titrate!"); }' > hello.tr
trc hello.tr
```

Expected output: `Hello, Titrate!`

**Time estimate:** 2-3 minutes total. The release build takes 1-2 minutes. Creating and running hello.tr takes under 1 minute.

::: tip Already have a pre-built binary?
If you have access to a pre-built `trc` binary, you can skip the build step entirely and run hello.tr in under 1 minute.
:::

Skip to [Your First Program](#your-first-program) for a detailed walkthrough.

## Complete Installation for Newcomers

If you're starting from scratch, follow these steps to install all prerequisites and build the compiler.

### Prerequisites

Before building Titrate, ensure you have these tools installed:

1. **Rust and Cargo** — Install Rust 1.70 or later from [rustup.rs](https://rustup.rs/). Run `rustc --version` to verify.

2. **LLVM development files** — Required for the native backend. Install via your system package manager:
   - Ubuntu/Debian: `sudo apt install llvm-dev libclang-dev`
   - macOS: `brew install llvm`
   - Windows: Download from [llvm.org](https://llvm.org/) or use Visual Studio installer

3. **Git** — Clone the repository with `git clone https://github.com/richie-rich90454/titrate.git`.

::: warning Windows Users
LLVM installation on Windows requires downloading the LLVM installer from llvm.org or using the Visual Studio installer. Ensure you select the "LLVM development tools" option during installation.
:::

### Build Steps

1. **Clone the repository:**

```bash
git clone https://github.com/richie-rich90454/titrate.git
cd titrate
```

**Time:** 1-3 minutes depending on network speed.

2. **Build the compiler in release mode:**

```bash
cargo build --release
```

**Time:** 5-10 minutes on first build. The release build compiles all compiler components with optimizations.

The compiler binary `trc` is created in `target/release/`. You can add it to your PATH:

```bash
# Linux/macOS
export PATH="$PWD/target/release:$PATH"

# Windows PowerShell
$env:Path += ";$PWD\target\release"
```

3. **Verify the build:**

```bash
trc --version
```

You should see output like `trc 0.1.0` or similar.

**Total time for complete installation:** 15-30 minutes depending on your system and network speed.

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
- **`io::println(...)`** — This calls the `println` function from the `io` module. Both `::` and `.` can be used to call module-level functions (`io::println(...)` or `io.println(...)`); `::` is shown here for clarity.
- **`"Hello, Titrate!"`** — a string literal. Titrate uses `string` (lowercase) as the type name.

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

## Troubleshooting

### Build Fails with LLVM Link Error

**Problem**: The build fails with errors like `LLVM not found` or linking errors.

**Solution**: Install LLVM development packages. On Ubuntu run `sudo apt install llvm-dev libclang-dev`. On macOS run `brew install llvm`. Ensure the LLVM version is 15 or later.

### Compiler Fails to Find Standard Library

**Problem**: Running `trc hello.tr` shows errors about missing imports or undefined modules.

**Solution**: Ensure the `lib/tt` directory exists in the Titrate repository. The standard library must be present for imports to work. Run `cargo test --test stdlib_test` to verify the standard library works correctly.

### Native Compilation Produces No Output

**Problem**: The `--native` flag compiles but produces no executable file.

**Solution**: Check that LLVM development files are installed. The native backend requires `libclang` and LLVM libraries. Run `llvm-config --version` to verify LLVM is available.

### Program Runs but Output Is Missing

**Problem**: The program executes but nothing prints to the console.

**Solution**: Verify your program calls `io::println` or another output function. Check that the `main` function is marked `public`. Use `public fn main(): void` as the entry point.

### Stack Overflow or Memory Error

**Problem**: The program crashes with a stack overflow or memory allocation error.

**Solution**: Reduce recursion depth or use iterative algorithms instead. The bytecode VM has a limited stack size. For deep recursion compile with `--native --release` which uses native stack limits.
