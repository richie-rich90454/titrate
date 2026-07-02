---
layout: home

hero:
  name: Titrate
  text: The language for precise systems
  tagline: Memory-safe without GC. Zero-cost generics. Scientific computing built in. Write code that is as reliable as it is expressive.
  actions:
    - theme: brand
      text: Get Started
      link: /guide/getting-started
    - theme: alt
      text: Language Guide
      link: /guide/variables
    - theme: alt
      text: View on GitHub
      link: https://github.com/richie-rich90454/titrate

features:
  - icon:
      src: /icons/vm.svg
    title: Bytecode VM
    details: Compile to optimized bytecode and run on the Titrate VM with built-in garbage-free memory management. Significantly faster than tree-walking interpretation.
  - icon:
      src: /icons/compat.svg
    title: LLVM Native Backend
    details: Compile to standalone native executables via LLVM. Release-mode builds run three to six times faster than the bytecode VM for compute-bound workloads. One flag, one binary.
  - icon:
      src: /icons/shield.svg
    title: Ownership and Regions
    details: Move semantics, borrowing and region-based allocation -- memory safety without garbage collection or manual free.
  - icon:
      src: /icons/generics.svg
    title: Zero-Cost Generics
    details: Monomorphizing compiler generates specialized code for each type instance. No boxing, no vtables, no runtime overhead.
  - icon:
      src: /icons/module.svg
    title: Module System
    details: Organize code with imports, control visibility with public and private, and detect circular dependencies at compile time.
  - icon:
      src: /icons/flask.svg
    title: Scientific Computing
    details: Bioinformatics, physics simulation, materials science, signal processing, image/audio processing, ML, computational geometry and more -- all in the standard library.
  - icon:
      src: /icons/result.svg
    title: Result-Based Error Handling
    details: No exceptions, no null pointer errors. Use `Result<T, E>` with `ok()` and `err()` to make error handling explicit and type-safe at compile time.
  - icon:
      src: /icons/operator.svg
    title: Operator Overloading
    details: Define natural syntax for your types with `fn operator+`, `fn operator*` and more. Build expressive DSLs for math, physics and data.
  - icon:
      src: /icons/library.svg
    title: Rich Standard Library
    details: Collections, I/O, JSON, CSV, XML, TCP, HTTP, SHA-256, HMAC, Base64, bioinformatics, physics, ML, HFT, simulation -- everything you need, out of the box.
---

## What is Titrate?

Titrate is a systems programming language designed for scientific computing. It combines memory safety without garbage collection, zero-cost generics and a rich standard library for chemistry, physics, machine learning and data analysis. The language targets developers building computational tools who need performance, safety and expressiveness.

**Key advantages:**

- **Performance:** Native compilation via LLVM produces executables that run three to six times faster than the bytecode VM for compute-bound workloads
- **Safety:** Ownership semantics and region-based allocation provide memory safety without runtime garbage collection pauses
- **Expressiveness:** Operator overloading, closures and generics let you write natural mathematical code that compiles to efficient machine code
- **Productivity:** Comprehensive standard library includes chemistry simulations, bioinformatics, physics engines, signal processing and more—no external dependencies required

## Systems Programming Meets Scientific Computing

Titrate bridges two worlds typically at odds. Systems languages offer control but lack scientific libraries. Scientific languages offer libraries but sacrifice performance. Titrate delivers both.

**Systems programming features:**

- Memory safety through ownership and borrowing
- Zero-cost abstractions with monomorphizing generics
- Compile-time error detection with Result types
- Direct compilation to native executables

**Scientific computing features:**

- Molecular dynamics and quantum chemistry modules
- Bioinformatics tools for sequence analysis
- Physics simulations for particle systems
- Machine learning primitives and neural network layers
- Signal processing, image analysis and audio processing

The language compiles to optimized bytecode for development and native code for production. Development stays fast. Production runs fast.

## Development Timeline

Titrate evolves through focused releases with specific capabilities.

**2024**

- **Alpha 0.1** (March 15, 2024) — Initial compiler with tree-walking interpreter
- **Alpha 0.2** (June 1, 2024) — Module system and import resolution
- **Alpha 0.3** (Sept. 12, 2024) — Bytecode VM with garbage-free memory management
- **Alpha 0.4** (Dec. 3, 2024) — LLVM native backend for standalone executables

**2025**

- **Alpha 0.5** (Feb. 20, 2025) — Chemistry and physics standard library modules
- **Alpha 0.6** (May 8, 2025) — Machine learning and bioinformatics packages
- **Beta 0.7** (Planned: August 2025) — Language stabilization and API refinement
- **Release 1.0** (Planned: Q4 2025) — Production-ready compiler and documentation

Each release adds capabilities without breaking existing code. The roadmap prioritizes stability for scientific workflows.

## Quick Start

::: tip Time Estimates
- **Experienced developers** (Rust+LLVM+Git installed): **2-3 minutes**
- **Complete newcomers** (first-time setup): **15-30 minutes**
:::

Build and run your first program:

```bash
# If you have prerequisites installed
cargo build --release
echo 'public fn main(): void { io::println("Hello!"); }' > hello.tr
trc hello.tr
```

**For complete installation instructions, prerequisites, and time estimates, see the [Getting Started Guide](/guide/getting-started).**

## Language at a Glance

```titrate
// Variables -- let for mutable with inference, var for mutable with explicit type
let name = "Titrate";
var count: int = 0;

// Functions with name: Type parameter order
public fn greet(name: string): void {
    io::println("Hello, " + name);
}

// Classes with fn init() constructors
public class Point {
    public double x;
    public double y;

    public fn init(x: double, y: double) {
        this.x = x;
        this.y = y;
    }
}

// Generics with monomorphization -- zero runtime overhead
public class Box<T> {
    public T value;

    public fn init(value: T) { this.value = value; }
}

// Result-based error handling -- no exceptions
fn parseAndDouble(s: string): Result<int, string> {
    // Integer.parseInt returns int; check for parse failure
    let n: int = Integer.parseInt(s);
    // Simple check: if result is 0 but input is not "0", assume failure
    if (n == 0 && s != "0") {
        return err("not a number");
    }
    return ok(n * 2);
}

// Closures that capture by reference
let double = fn(x: int): int => x * 2;
let numbers = new ArrayList<int>();
numbers.forEach(fn(n: int): void {
    io::println(Integer.toString(double(n)));
});

// Operator overloading
public class Vec2 {
    public double x;
    public double y;

    public fn init(x: double, y: double) {
        this.x = x;
        this.y = y;
    }

    public fn operator+(other: Vec2): Vec2 {
        return new Vec2(this.x + other.x, this.y + other.y);
    }
}

// Ranges and iterators
for (i in 0..10) {
    io::println(Integer.toString(i));
}
```

## Next Steps

After running your first program, continue learning Titrate with these guides. Each section builds on the previous one, taking you from basic syntax to advanced systems programming.

### Recommended Learning Path

Follow this sequence to build a solid foundation:

1. **Variables** — Learn `let`, `var` and `const` declarations with type inference. [Variables Guide →](/guide/variables)

2. **Functions** — Define functions with `name: Type` parameters, write generic functions and understand recursion. [Functions Guide →](/guide/functions)

3. **Control Flow** — Master `if`/`else`, `while`, `for` loops and `break`/`continue` for fine-grained control. [Control Flow Guide →](/guide/control-flow)

4. **Classes** — Create classes with fields and methods, use inheritance with `extends` and implement interfaces. [Classes Guide →](/guide/classes)

5. **Generics** — Write type-safe reusable code with zero runtime overhead through monomorphization. [Generics Guide →](/guide/generics)

6. **Error Handling** — Handle failures explicitly with `Result<T, E>` and the `?` propagation operator. [Error Handling Guide →](/guide/error-handling)

7. **Ownership** — Understand memory safety without garbage collection using `Owned<T>`, borrows and regions. [Ownership Guide →](/guide/ownership)

8. **Native Compilation** — Compile to standalone executables with LLVM for maximum performance. [Native Backend Guide →](/guide/native-intro)

## Learning Path Overview

Titrate's learning curve progresses through three distinct phases. Each phase introduces new concepts while reinforcing previous skills.

### Phase 1: Basics — Core Syntax and Building Blocks

Learn the fundamentals every Titrate program needs:

| Skill | Description | Guide |
|-------|-------------|-------|
| **Variables** | Declare mutable bindings with `let` and `var`, compile-time constants with `const` and understand type inference. | [Variables](/guide/variables) |
| **Functions** | Define functions with `fn`, use `name: Type` parameter order, specify return types and write recursive functions. | [Functions](/guide/functions) |
| **Control Flow** | Branch with `if`/`else`/`else if`, loop with `while` and `for` and control iteration with `break` and `continue`. | [Control Flow](/guide/control-flow) |

After completing Phase 1, you can write standalone programs that process data, make decisions and loop over collections.

### Phase 2: Intermediate — Data Structures and Safety

Build larger programs with structured data and explicit error handling:

| Skill | Description | Guide |
|-------|-------------|-------|
| **Classes** | Bundle data and behavior, define constructors with `fn init()`, inherit with `extends` and implement interfaces. | [Classes](/guide/classes) |
| **Interfaces** | Define contracts with interface methods, implement multiple interfaces and write polymorphic code. | [Interfaces](/guide/interfaces) |
| **Enums** | Model domain data with algebraic data types, use enum variants with payloads and pattern match on values. | [Enums](/guide/enums) |
| **Generics** | Write reusable code with type parameters, apply interface constraints and understand monomorphization. | [Generics](/guide/generics) |
| **Error Handling** | Use `Result<T, E>` for explicit error handling, propagate errors with `?` and convert between error types. | [Error Handling](/guide/error-handling) |
| **Modules** | Organize code across files, use `import` with `::` syntax and control visibility with `public`/`private`. | [Modules](/guide/modules) |

After completing Phase 2, you can build multi-file projects with custom data types, safe error handling and reusable generic components.

### Phase 3: Advanced — Systems Programming and Performance

Master low-level memory management and native compilation:

| Skill | Description | Guide |
|-------|-------------|-------|
| **Ownership** | Track data ownership with `Owned<T>`, borrow with `&` and `&mut` and use regions for batch allocation. | [Ownership](/guide/ownership) |
| **Unsafe Code** | Suspend safety checks in `unsafe` blocks, interface with C libraries and implement low-level data structures. | [Ownership](/guide/ownership#unsafe-blocks) |
| **Native Compilation** | Compile to LLVM IR, generate standalone executables and optimize for compute-bound workloads. | [Native Backend](/guide/native-intro) |
| **Closures** | Write anonymous functions with `fn(x): T => expr`, capture variables by reference and pass functions as arguments. | [Closures](/guide/closures) |
| **Operator Overloading** | Define custom behavior for `+`, `-`, `*` and other operators with `fn operator<op>` syntax. | [Operator Overloading](/guide/operator-overloading) |
| **Iterators** | Implement `Iterable<T>` and `Iterator<T>`, use custom iterators in `for-in` loops and chain iterator operations. | [Iterators](/guide/iterators) |

After completing Phase 3, you can write high-performance systems code, manage memory without garbage collection and compile native executables for production deployment.

### Skill Progression Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              PHASE 1: BASICS                                │
│                                                                             │
│   Variables ──▶ Functions ──▶ Control Flow                                 │
│   (let/var)     (fn name)      (if/while/for)                               │
│                                                                             │
│   Skills: Declare values, define logic, branch and loop                    │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           PHASE 2: INTERMEDIATE                            │
│                                                                             │
│   Classes ──▶ Interfaces ──▶ Enums ──▶ Generics ──▶ Error Handling         │
│   (fn init)   (implements)   (ADTs)    (<T>)        (Result<T,E>)           │
│                                                                             │
│   Skills: Structure data, enforce contracts, handle failures explicitly    │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                            PHASE 3: ADVANCED                                │
│                                                                             │
│   Ownership ──▶ Unsafe Code ──▶ Native Compilation                         │
│   (Owned<T>)    (unsafe {})     (--native)                                  │
│                                                                             │
│   Skills: Manage memory safely, bypass checks when needed, compile to exe  │
└─────────────────────────────────────────────────────────────────────────────┘
```

### How Long Does Each Phase Take?

Most developers reach basic proficiency in Phase 1 after one to two days of practice. Phase 2 typically requires three to five days, depending on prior experience with object-oriented programming and generics. Phase 3 concepts—especially ownership—may take one to two weeks to internalize fully, particularly for developers coming from garbage-collected languages.

The guides include "Try It Yourself" exercises for hands-on practice. Working through these exercises accelerates learning and reveals common pitfalls early.

## Getting Started

::: tip Comprehensive Guide Available
This section provides a quick overview. For complete installation instructions, prerequisites, troubleshooting and detailed walkthroughs, see the [Getting Started Guide](/guide/getting-started).
:::

### Quick Overview

**Prerequisites:**
- Rust 1.70+ ([rustup.rs](https://rustup.rs/))
- LLVM development files (system package manager)
- Git

**Build and run:**

```bash
git clone https://github.com/richie-rich90454/titrate.git
cd titrate
cargo build --release

# Create hello.tr
echo 'public fn main(): void { io::println("Hello, Titrate!"); }' > hello.tr

# Run with bytecode VM
trc hello.tr

# Or compile to native executable (3-6x faster)
trc --native --release hello.tr
./hello
```

### Troubleshooting

::: info Detailed Troubleshooting
For comprehensive troubleshooting steps, error explanations and solutions, see the [Getting Started Guide](/guide/getting-started#troubleshooting).
:::

#### Build Fails with LLVM Link Error

**Problem**: The build fails with errors like `LLVM not found` or linking errors.

**Solution**: Install LLVM development packages. On Ubuntu run `sudo apt install llvm-dev libclang-dev`. On macOS run `brew install llvm`. Ensure the LLVM version is 15 or later.

#### Compiler Fails to Find Standard Library

**Problem**: Running `trc hello.tr` shows errors about missing imports or undefined modules.

**Solution**: Ensure the `lib/tt` directory exists in the Titrate repository. The standard library must be present for imports to work. Run `cargo test --test stdlib_test` to verify the standard library works correctly.

#### Native Compilation Produces No Output

**Problem**: The `--native` flag compiles but produces no executable file.

**Solution**: Check that LLVM development files are installed. The native backend requires `libclang` and LLVM libraries. Run `llvm-config --version` to verify LLVM is available.

#### Program Runs but Output Is Missing

**Problem**: The program executes but nothing prints to the console.

**Solution**: Verify your program calls `io::println` or another output function. Check that the `main` function is marked `public`. Use `public fn main(): void` as the entry point.

#### Stack Overflow or Memory Error

**Problem**: The program crashes with a stack overflow or memory allocation error.

**Solution**: Reduce recursion depth or use iterative algorithms instead. The bytecode VM has a limited stack size. For deep recursion compile with `--native --release` which uses native stack limits.

## See It in Action

### Molecular Dynamics Simulation

```titrate
import tt::chem::Atom;
import tt::chem::Molecule;
import tt::chem::ForceField;
import tt::chem::MDSimulation;
import tt::chem::VerletIntegrator;

public fn main(): void {
    let water: Molecule = new Molecule("water");
    water.addAtom(Atom.oxygen(0.0, 0.0, 0.0));
    water.addAtom(Atom.hydrogen(0.9572, 0.0, 0.0));
    water.addAtom(Atom.hydrogen(-0.2399, 0.9270, 0.0));

    let ff: ForceField = new ForceField();
    ff.addBondTerm(0, 1, 450.0, 0.9572);
    ff.addAngleTerm(1, 0, 2, 55.0, 104.52);

    let integrator: VerletIntegrator = new VerletIntegrator(1.0, "berendsen", 300.0);
    let md: MDSimulation = new MDSimulation(water, ff, integrator);
    md.run(1000);

    io::println("Energy: " + Double.toString(ff.totalEnergy(water)));
}
```

### JSON API Client with Error Handling

```titrate
import tt::json::Json;
import tt::json::JsonValue;
import tt::net::HttpClient;
import tt::lang::Integer;

public fn fetchUser(id: int): Result<JsonValue, string> {
    let client: HttpClient = new HttpClient();
    let url: string = "https://api.example.com/users/" + Integer.toString(id);
    let response: HttpResponse = client.get(url);

    if (response.getStatusCode() == 200) {
        let body: string = response.getBody();
        let parsed: JsonValue = Json.parse(body);
        if (!parsed.isNull()) { return ok(parsed); }
        return err("Failed to parse JSON");
    }
    return err("HTTP request failed with status: " + Integer.toString(response.getStatusCode()));
}
```

### Data Processing with NDArray

```titrate
import tt::math::ndarray::NDArray;
import tt::math::ndarray::NDArrayReduce;
import tt::math::ndarray::NDArrayMath;

public fn normalize(data: NDArray<double>): NDArray<double> {
    let mean: double = NDArrayReduce.mean(data);
    let std: double = NDArrayReduce.stddev(data);
    return NDArrayMath.map(data, fn(x: double): double => (x - mean) / std);
}

public fn correlation(x: NDArray<double>, y: NDArray<double>): double {
    let nx: NDArray<double> = normalize(x);
    let ny: NDArray<double> = normalize(y);
    return NDArrayMath.dot(nx, ny) / (nx.size() as double);
}
```

### Custom Collection with Generics

```titrate
public class RingBuffer<T> implements Iterable<T> {
    private ArrayList<T> data;
    private int head;
    private int count;

    public fn init(capacity: int) {
        this.data = new ArrayList<T>();
        this.head = 0;
        this.count = 0;
    }

    public fn push(item: T): void {
        this.data.add(item);
        this.count = this.count + 1;
    }

    public fn iterator(): Iterator<T> {
        return new RingBufferIterator<T>(this);
    }
}

// Use in for-in loops naturally
let buf = new RingBuffer<string>(3);
buf.push("first");
buf.push("second");
for (item in buf) {
    io::println(item);
}
```

## Why Titrate?

### Type Safety Without Runtime Cost

Titrate's monomorphizing compiler generates specialized code for each generic type instance. `ArrayList<int>` runs just as fast as hand-written code for integers -- no boxing, no type checks at runtime.

### Memory Safety Without Garbage Collection

Ownership semantics, move checking and region-based allocation give you memory safety guarantees without the pause times and overhead of a garbage collector. When a value goes out of scope, the compiler cleans it up immediately.

### Clean, Modern Syntax

Inspired by Rust and Python, Titrate uses `name: Type` parameter order, `fn` declarations and lowercase `string` from the start. But it also supports C-family sugar forms (`int x = 5`, `++i`, `ClassName(params)`) so developers from C, C++ and ECMAScript feel at home immediately.

### Scientific Computing Built in

Chemistry simulations (Atom, Molecule, ForceField, MD, RHF), bioinformatics (Sequence, Alignment, PhyloTree), physics (Particle, ForceField, NBodySimulator), machine learning (Tensor, Model, Optimizer), signal processing (FFT2, Filter, Wavelet), image processing, audio processing, computational geometry, NLP, HFT and discrete-event simulation are all part of the standard library -- not third-party packages.

### Comprehensive Standard Library

Collections (ArrayList, HashMap, HashSet, Vec), I/O (File, BufferedReader), serialization (JSON, CSV, XML), networking (TCP, HTTP), cryptography (SHA-256, HMAC, Base64) and more -- all included out of the box.

## Comparison

How does Titrate compare to other systems languages for common tasks?

<div class="comparison-grid">

<div class="comparison-card">
  <div class="comparison-header">
    <span class="lang-name titrate-lang">Titrate</span>
  </div>
  <div class="comparison-features">
    <div class="feature-row"><span class="feature-name">Memory safety</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">No garbage collector</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">Zero-cost generics</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">Simple syntax</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">Scientific computing stdlib</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">Result-based error handling</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">Operator overloading</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">Fast compile times</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">C-family sugar forms</span><span class="check">✓</span></div>
  </div>
</div>

<div class="comparison-card">
  <div class="comparison-header">
    <span class="lang-name">C</span>
  </div>
  <div class="comparison-features">
    <div class="feature-row"><span class="feature-name">Memory safety</span><span class="cross">✗</span></div>
    <div class="feature-row"><span class="feature-name">No garbage collector</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">Zero-cost generics</span><span class="cross">✗</span></div>
    <div class="feature-row"><span class="feature-name">Simple syntax</span><span class="cross">✗</span></div>
    <div class="feature-row"><span class="feature-name">Scientific computing stdlib</span><span class="cross">✗</span></div>
    <div class="feature-row"><span class="feature-name">Result-based error handling</span><span class="cross">✗</span></div>
    <div class="feature-row"><span class="feature-name">Operator overloading</span><span class="cross">✗</span></div>
    <div class="feature-row"><span class="feature-name">Fast compile times</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">C-family sugar forms</span><span class="check">✓</span></div>
  </div>
</div>

<div class="comparison-card">
  <div class="comparison-header">
    <span class="lang-name">Rust</span>
  </div>
  <div class="comparison-features">
    <div class="feature-row"><span class="feature-name">Memory safety</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">No garbage collector</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">Zero-cost generics</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">Simple syntax</span><span class="cross">✗</span></div>
    <div class="feature-row"><span class="feature-name">Scientific computing stdlib</span><span class="cross">✗</span></div>
    <div class="feature-row"><span class="feature-name">Result-based error handling</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">Operator overloading</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">Fast compile times</span><span class="cross">✗</span></div>
    <div class="feature-row"><span class="feature-name">C-family sugar forms</span><span class="cross">✗</span></div>
  </div>
</div>

<div class="comparison-card">
  <div class="comparison-header">
    <span class="lang-name">Python</span>
  </div>
  <div class="comparison-features">
    <div class="feature-row"><span class="feature-name">Memory safety</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">No garbage collector</span><span class="cross">✗</span></div>
    <div class="feature-row"><span class="feature-name">Zero-cost generics</span><span class="cross">✗</span></div>
    <div class="feature-row"><span class="feature-name">Simple syntax</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">Scientific computing stdlib</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">Result-based error handling</span><span class="cross">✗</span></div>
    <div class="feature-row"><span class="feature-name">Operator overloading</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">Fast compile times</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">C-family sugar forms</span><span class="cross">✗</span></div>
  </div>
</div>

<div class="comparison-card">
  <div class="comparison-header">
    <span class="lang-name">Julia</span>
  </div>
  <div class="comparison-features">
    <div class="feature-row"><span class="feature-name">Memory safety</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">No garbage collector</span><span class="cross">✗</span></div>
    <div class="feature-row"><span class="feature-name">Zero-cost generics</span><span class="partial">~</span></div>
    <div class="feature-row"><span class="feature-name">Simple syntax</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">Scientific computing stdlib</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">Result-based error handling</span><span class="cross">✗</span></div>
    <div class="feature-row"><span class="feature-name">Operator overloading</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">Fast compile times</span><span class="cross">✗</span></div>
    <div class="feature-row"><span class="feature-name">C-family sugar forms</span><span class="cross">✗</span></div>
  </div>
</div>

</div>

### Detailed Comparison

**C** remains the foundation of systems programming. It offers unmatched control over memory and hardware. But manual memory management invites bugs. Buffer overflows, use-after-free errors and null pointer crashes plague C projects. Generic programming requires macros or void pointers with runtime casts. Titrate provides similar low-level control through ownership semantics and unsafe blocks but makes safe code the default.

**Rust** pioneered the ownership model that Titrate adopts. Rust achieves memory safety without garbage collection through its borrow checker. The borrow checker enforces strict rules at compile time. Titrate uses a similar ownership system but with simpler syntax and faster compilation. Rust compile times often reach minutes for large projects. Titrate compiles in seconds. Rust requires learning lifetime annotations and complex trait syntax. Titrate uses straightforward `name: Type` declarations.

**Python** dominates scientific computing for its simplicity and library ecosystem. NumPy, SciPy and Pandas handle numerical work well. But Python runs 10 to 100 times slower than compiled languages. The garbage collector introduces unpredictable pauses. Error handling relies on exceptions rather than explicit Result types. Titrate matches Python's expressiveness with operator overloading and closures but compiles to native code. Titrate's standard library includes chemistry, physics and bioinformatics modules comparable to Python's scientific stack.

**Julia** targets scientific computing with JIT compilation. It approaches C performance for numerical kernels. But Julia suffers from JIT warmup delays on first execution. The garbage collector still runs. Compile times grow with package complexity. Julia uses multiple dispatch rather than traditional object-oriented methods. Titrate uses familiar class-based design with zero-cost generics through monomorphization. No warmup, no GC pauses, predictable performance from the first call.

### Why Choose Titrate

Titrate occupies a unique position among systems languages. Here is what sets it apart from competitors.

**Against C**: Titrate delivers memory safety by default. No buffer overflows, no use-after-free bugs. The ownership system cleans up values when they leave scope. Write safe code naturally, drop into unsafe blocks only when needed. C requires manual discipline for every allocation. Titrate makes safety automatic.

**Against Rust**: Titrate compiles in seconds, not minutes. The syntax stays simple without lifetime annotations or complex trait bounds. Generics work through straightforward type parameters. C-family developers recognize the sugar forms immediately. Rust demands months of study to master. Titrate feels familiar from day one.

**Against Python**: Titrate runs at native speed from the start. No interpreter overhead, no JIT warmup. The standard library includes scientific modules without external dependencies. Result types make error handling explicit and compile-time checked. Python hides errors until runtime. Titrate catches them during compilation.

**Against Julia**: Titrate avoids JIT warmup entirely. The first call runs as fast as the millionth. No garbage collector pauses interrupt simulations. Class-based design matches most developers' mental model. Julia's multiple dispatch requires relearning object-oriented patterns. Titrate keeps the familiar class and interface structure.

**The unique advantage**: Titrate combines the four things scientific developers need most. Native performance through LLVM. Memory safety without GC pauses. Zero-cost generics through monomorphization. Scientific libraries in the standard distribution. No other language delivers all four.

### Code Comparison: Molecular Dynamics Force Calculation

The same Lennard-Jones force calculation in four languages shows how Titrate balances performance and clarity.

**Titrate**

```titrate
import tt::chem::Atom;
import tt::math::MathAdvanced;

public fn ljForce(a: Atom, b: Atom): double {
    let dx: double = b.x - a.x;
    let dy: double = b.y - a.y;
    let dz: double = b.z - a.z;
    let r2: double = dx * dx + dy * dy + dz * dz;
    let r6: double = MathAdvanced.pow(r2, 3.0);
    let r12: double = r6 * r6;
    let eps: double = a.eps;
    let sig: double = a.sig;
    let sig6: double = MathAdvanced.pow(sig, 6.0);
    let sig12: double = sig6 * sig6;
    let force: double = 24.0 * eps * (2.0 * sig12 / r12 - sig6 / r6) / r2;
    return force;
}
```

**C**

```c
double lj_force(Atom* a, Atom* b) {
    double dx = b->x - a->x;
    double dy = b->y - a->y;
    double dz = b->z - a->z;
    double r2 = dx*dx + dy*dy + dz*dz;
    double r6 = r2 * r2 * r2;
    double r12 = r6 * r6;
    double eps = a->eps;
    double sig = a->sig;
    double sig6 = sig * sig * sig * sig * sig * sig;
    double sig12 = sig6 * sig6;
    double force = 24.0 * eps * (2.0 * sig12 / r12 - sig6 / r6) / r2;
    return force;
}
```

Titrate matches C's arithmetic clarity. Both compute the same formula line by line. Titrate adds type safety and automatic cleanup. C leaves the programmer responsible for pointer validity.

**Rust**

```rust
fn lj_force(a: &Atom, b: &Atom) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let dz = b.z - a.z;
    let r2 = dx * dx + dy * dy + dz * dz;
    let r6 = r2.powi(3);
    let r12 = r6 * r6;
    let eps = a.eps;
    let sig = a.sig;
    let sig6 = sig.powi(6);
    let sig12 = sig6 * sig6;
    let force = 24.0 * eps * (2.0 * sig12 / r12 - sig6 / r6) / r2;
    force
}
```

Rust requires lifetime annotations in function signatures. The borrow checker validates references at compile time. Titrate achieves equivalent safety without the annotation burden.

**Python**

```python
def lj_force(a, b):
    dx = b.x - a.x
    dy = b.y - a.y
    dz = b.z - a.z
    r2 = dx*dx + dy*dy + dz*dz
    r6 = r2 ** 3
    r12 = r6 * r6
    eps = a.eps
    sig = a.sig
    sig6 = sig ** 6
    sig12 = sig6 * sig6
    force = 24.0 * eps * (2.0 * sig12 / r12 - sig6 / r6) / r2
    return force
```

Python reads cleanly but runs through an interpreter. The first call suffers compilation overhead in NumPy-accelerated versions. Titrate executes native machine code immediately.

## Standard Library Ecosystem

Titrate ships with a comprehensive standard library organized into intuitive modules:

::: ecosystem-showcase

<!-- Core Collections -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-core">Core</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">📦</span>
<h4 class="ecosystem-card-title">tt::util</h4>
</div>
<p class="ecosystem-card-types">ArrayList, HashMap, HashSet, Vec, Stack, Queue</p>
<code class="ecosystem-card-code">let list = new ArrayList&lt;int&gt;();
list.add(42);</code>
<a class="ecosystem-card-link" href="/stdlib/util/ArrayList"></a>
</div>

<!-- I/O -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-core">Core</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">📄</span>
<h4 class="ecosystem-card-title">tt::io</h4>
</div>
<p class="ecosystem-card-types">File, BufferedReader, FileWriter, Path</p>
<code class="ecosystem-card-code">let f = File.open("data.txt");
let content = f.readAll();</code>
<a class="ecosystem-card-link" href="/stdlib/io/File"></a>
</div>

<!-- JSON -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-data">Data</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">📋</span>
<h4 class="ecosystem-card-title">tt::json</h4>
</div>
<p class="ecosystem-card-types">JsonValue, Json, JsonPath, JsonSchema</p>
<code class="ecosystem-card-code">let obj = Json.parse(text);
let v = obj.get("key");</code>
<a class="ecosystem-card-link" href="/stdlib/json/Json"></a>
</div>

<!-- XML -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-data">Data</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">🏷️</span>
<h4 class="ecosystem-card-title">tt::xml</h4>
</div>
<p class="ecosystem-card-types">XmlNode, Xml, XPath, XmlBuilder</p>
<code class="ecosystem-card-code">let doc = Xml.parse(xmlStr);
let nodes = XPath.select(doc, "//item");</code>
<a class="ecosystem-card-link" href="/stdlib/xml/Xml"></a>
</div>

<!-- Math -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-science">Science</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">🔢</span>
<h4 class="ecosystem-card-title">tt::math</h4>
</div>
<p class="ecosystem-card-types">Math, MathAdvanced, MathTrig, Special</p>
<code class="ecosystem-card-code">let x = MathAdvanced.sqrt(2.0);
let y = MathTrig.sin(Math.PI()/4);</code>
<a class="ecosystem-card-link" href="/stdlib/math/Math"></a>
</div>

<!-- Linear Algebra -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-science">Science</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">📐</span>
<h4 class="ecosystem-card-title">tt::math::linalg</h4>
</div>
<p class="ecosystem-card-types">Matrix, MatrixDecomp, SparseMatrix</p>
<code class="ecosystem-card-code">let m = Matrix.fromArray(data);
let det = MatrixDecomp.det(m);</code>
<a class="ecosystem-card-link" href="/stdlib/math/linalg/Matrix"></a>
</div>

<!-- NDArray -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-science">Science</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">📊</span>
<h4 class="ecosystem-card-title">tt::math::ndarray</h4>
</div>
<p class="ecosystem-card-types">NDArray, NDArrayReduce, NDArrayMath</p>
<code class="ecosystem-card-code">let arr = NDArray.zeros(3, 4);
let mean = NDArrayReduce.mean(arr);</code>
<a class="ecosystem-card-link" href="/stdlib/math/ndarray/NDArray"></a>
</div>

<!-- Chemistry -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-science">Science</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">🧪</span>
<h4 class="ecosystem-card-title">tt::chem</h4>
</div>
<p class="ecosystem-card-types">Atom, Molecule, ForceField, MD, RHF</p>
<code class="ecosystem-card-code">let mol = new Molecule("water");
mol.addAtom(Atom.oxygen(0, 0, 0));</code>
<a class="ecosystem-card-link" href="/stdlib/chem/Molecule"></a>
</div>

<!-- Bioinformatics -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-science">Science</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">🧬</span>
<h4 class="ecosystem-card-title">tt::bio</h4>
</div>
<p class="ecosystem-card-types">Sequence, Alignment, PhyloTree, CodonTable</p>
<code class="ecosystem-card-code">let seq = Sequence.fromDNA("ATGC");
let aligned = Alignment.align(a, b);</code>
<a class="ecosystem-card-link" href="/stdlib/bio/Sequence"></a>
</div>

<!-- Physics -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-science">Science</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">⚡</span>
<h4 class="ecosystem-card-title">tt::physics</h4>
</div>
<p class="ecosystem-card-types">Particle, ForceField, NBodySimulator, RigidBody</p>
<code class="ecosystem-card-code">let sim = new NBodySimulator();
sim.addParticle(p, mass, pos);</code>
<a class="ecosystem-card-link" href="/stdlib/physics/NBodySimulator"></a>
</div>

<!-- Materials -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-science">Science</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">💎</span>
<h4 class="ecosystem-card-title">tt::materials</h4>
</div>
<p class="ecosystem-card-types">CrystalStructure, XRayDiffraction, Elasticity</p>
<code class="ecosystem-card-code">let crystal = CrystalStructure.load("cif");
let pattern = XRayDiffraction.simulate(crystal);</code>
<a class="ecosystem-card-link" href="/stdlib/materials/CrystalStructure"></a>
</div>

<!-- Signal Processing -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-science">Science</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">📈</span>
<h4 class="ecosystem-card-title">tt::sigproc</h4>
</div>
<p class="ecosystem-card-types">FFT2, Filter, Wavelet, Spectrogram</p>
<code class="ecosystem-card-code">let spectrum = FFT2.transform(signal);
let filtered = Filter.lowpass(spectrum, cutoff);</code>
<a class="ecosystem-card-link" href="/stdlib/sigproc/FFT2"></a>
</div>

<!-- Image Processing -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-special">Special</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">🖼️</span>
<h4 class="ecosystem-card-title">tt::image</h4>
</div>
<p class="ecosystem-card-types">Image, Kernel, Morphology, Threshold</p>
<code class="ecosystem-card-code">let img = Image.load("photo.png");
let edges = Kernel.apply(img, sobel);</code>
<a class="ecosystem-card-link" href="/stdlib/image/Image"></a>
</div>

<!-- Audio Processing -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-special">Special</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">🎵</span>
<h4 class="ecosystem-card-title">tt::audio</h4>
</div>
<p class="ecosystem-card-types">AudioBuffer, Pitch, Mfcc</p>
<code class="ecosystem-card-code">let audio = AudioBuffer.load("song.wav");
let pitch = Pitch.detect(audio);</code>
<a class="ecosystem-card-link" href="/stdlib/audio/AudioBuffer"></a>
</div>

<!-- Machine Learning -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-science">Science</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">🤖</span>
<h4 class="ecosystem-card-title">tt::ml</h4>
</div>
<p class="ecosystem-card-types">Tensor, Layer, Optimizer, Model</p>
<code class="ecosystem-card-code">let model = Model.sequential([
  Layer.dense(128, "relu")
]);</code>
<a class="ecosystem-card-link" href="/stdlib/ml/Model"></a>
</div>

<!-- Computational Geometry -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-science">Science</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">🔺</span>
<h4 class="ecosystem-card-title">tt::geom</h4>
</div>
<p class="ecosystem-card-types">ConvexHull, Delaunay, SpatialIndex</p>
<code class="ecosystem-card-code">let hull = ConvexHull.compute(points);
let mesh = Delaunay.triangulate(hull);</code>
<a class="ecosystem-card-link" href="/stdlib/geom/ConvexHull"></a>
</div>

<!-- NLP -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-special">Special</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">💬</span>
<h4 class="ecosystem-card-title">tt::nlp</h4>
</div>
<p class="ecosystem-card-types">Tokenizer, Stemmer, Classifier</p>
<code class="ecosystem-card-code">let tokens = Tokenizer.tokenize(text);
let stems = Stemmer.stemAll(tokens);</code>
<a class="ecosystem-card-link" href="/stdlib/nlp/Tokenizer"></a>
</div>

<!-- High-Frequency Trading -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-special">Special</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">💹</span>
<h4 class="ecosystem-card-title">tt::hft</h4>
</div>
<p class="ecosystem-card-types">FixParser, OrderRouter, RiskManager, Backtest</p>
<code class="ecosystem-card-code">let order = FixParser.parse(msg);
Backtest.run(strategy, data);</code>
<a class="ecosystem-card-link" href="/stdlib/hft/Backtest"></a>
</div>

<!-- Discrete-Event Simulation -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-science">Science</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">⏱️</span>
<h4 class="ecosystem-card-title">tt::sim</h4>
</div>
<p class="ecosystem-card-types">Simulation, Resource, Process, Monitor</p>
<code class="ecosystem-card-code">let sim = new Simulation();
sim.addProcess(myProcess);</code>
<a class="ecosystem-card-link" href="/stdlib/sim/Simulation"></a>
</div>

<!-- Quantitative Finance -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-special">Special</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">💰</span>
<h4 class="ecosystem-card-title">tt::finance</h4>
</div>
<p class="ecosystem-card-types">BlackScholes, Portfolio, YieldCurve</p>
<code class="ecosystem-card-code">let price = BlackScholes.call(
  S, K, T, r, sigma);</code>
<a class="ecosystem-card-link" href="/stdlib/finance/BlackScholes"></a>
</div>

<!-- Advanced Cryptography -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-core">Core</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">🔐</span>
<h4 class="ecosystem-card-title">tt::crypto2</h4>
</div>
<p class="ecosystem-card-types">AES, RSA, ECDSA, KDF</p>
<code class="ecosystem-card-code">let cipher = AES.encrypt(key, data);
let sig = ECDSA.sign(privKey, msg);</code>
<a class="ecosystem-card-link" href="/stdlib/crypto2/AES"></a>
</div>

<!-- Networking -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-core">Core</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">🌐</span>
<h4 class="ecosystem-card-title">tt::net</h4>
</div>
<p class="ecosystem-card-types">TcpClient, TcpServer, HttpClient</p>
<code class="ecosystem-card-code">let client = new HttpClient();
let resp = client.get(url);</code>
<a class="ecosystem-card-link" href="/stdlib/net/HttpClient"></a>
</div>

<!-- Hash Functions -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-core">Core</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">🔒</span>
<h4 class="ecosystem-card-title">tt::crypto</h4>
</div>
<p class="ecosystem-card-types">SHA256, HMAC, Hash</p>
<code class="ecosystem-card-code">let hash = SHA256.hash(data);
let mac = HMAC.compute(key, msg);</code>
<a class="ecosystem-card-link" href="/stdlib/crypto/SHA256"></a>
</div>

<!-- Encoding -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-core">Core</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">🔡</span>
<h4 class="ecosystem-card-title">tt::encoding</h4>
</div>
<p class="ecosystem-card-types">Base64, Hex, Url</p>
<code class="ecosystem-card-code">let enc = Base64.encode(data);
let dec = Hex.decode("0x1A2B");</code>
<a class="ecosystem-card-link" href="/stdlib/encoding/Base64"></a>
</div>

<!-- Argument Parsing -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-core">Core</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">⚙️</span>
<h4 class="ecosystem-card-title">tt::argparse</h4>
</div>
<p class="ecosystem-card-types">ArgumentParser</p>
<code class="ecosystem-card-code">let parser = new ArgumentParser();
parser.addArg("--input", "file");</code>
<a class="ecosystem-card-link" href="/stdlib/argparse/ArgumentParser"></a>
</div>

<!-- CSV -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-data">Data</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">📊</span>
<h4 class="ecosystem-card-title">tt::csv</h4>
</div>
<p class="ecosystem-card-types">CsvReader, CsvWriter</p>
<code class="ecosystem-card-code">let reader = CsvReader.open("data.csv");
let row = reader.next();</code>
<a class="ecosystem-card-link" href="/stdlib/csv/CsvReader"></a>
</div>

<!-- Random -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-core">Core</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">🎲</span>
<h4 class="ecosystem-card-title">tt::random</h4>
</div>
<p class="ecosystem-card-types">Random, Prng, Sampling</p>
<code class="ecosystem-card-code">let rng = new Random();
let n = rng.nextInt(0, 100);</code>
<a class="ecosystem-card-link" href="/stdlib/random/Random"></a>
</div>

<!-- Statistics -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-science">Science</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">📉</span>
<h4 class="ecosystem-card-title">tt::statistics</h4>
</div>
<p class="ecosystem-card-types">Statistics, Bootstrap, TimeSeries</p>
<code class="ecosystem-card-code">let stats = Statistics.fromData(data);
let ci = Bootstrap.confidenceInterval(95);</code>
<a class="ecosystem-card-link" href="/stdlib/statistics/Statistics"></a>
</div>

:::

## Community and Contributing

Titrate is open source and actively developed. Join us!

- **GitHub** -- [richie-rich90454/titrate](https://github.com/richie-rich90454/titrate) -- Report issues, request features, contribute code
- **Contributing** -- Check the [contributing guide](/guide/contributing) for guidelines
- **Build and Test** -- `cargo test --lib`, `cargo test --test stdlib_test`, `cargo test --test mega_test`

```bash
# Clone and build
git clone https://github.com/richie-rich90454/titrate.git
cd titrate
cargo build --release

# Run the full test suite
cargo test --lib; cargo test --test stdlib_test; cargo test --test mega_test
```
