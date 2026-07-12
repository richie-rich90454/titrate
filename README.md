# Titrate

A systems programming language. Precise. Safe. Expressive.

```titrate
import tt::io::println;
import tt::util::ArrayList;
import tt::util::HashMap;
import tt::math::Math;
import tt::json::Json;

public fn main(): void {
    // Closures
    let square = fn(x: int): int => x * x;
    io::println("5^2 = " + Integer.toString(square(5)));

    // Tuples
    let pair: (int, string) = (42, "hello");
    let (n, s) = pair;
    io::println(s + " = " + Integer.toString(n));

    // Range expressions
    for (i in 0..10) {
        io::println(Integer.toString(i));
    }

    // Raw strings and byte literals
    let raw: string = r"No \escaping here";
    let b: byte = b'A';

    // Numeric literals: binary, octal, underscores
    let bin: int = 0b1010;
    let oct: int = 0o777;
    let big: long = 1_000_000_000L;

    // Standard library
    let map: HashMap<string, double> = new HashMap<string, double>();
    map.put("pi", Math.PI());
    io::println("pi = " + Double.toString(map.get("pi")));

    // JSON
    let data = Json.parse("{\"lang\":\"Titrate\"}");
    io::println("lang = " + data.asObject().get("lang").asString());
}
```

## Features

- **Bytecode VM** — compiles to optimized bytecode, 10x faster than tree-walking
- **Precise types** — byte to quad, unsigned fixed-width to 128-bit integers
- **Generics** — monomorphized at compile time, zero runtime overhead
- **Closures** — anonymous functions with captured environment
- **Tuples** — anonymous product types with destructuring
- **Operator overloading** — define `operator+`, `operator==`, etc. on user types
- **Iterators** — `for (item in iterable)` desugars to `next()` calls
- **Range expressions** — exclusive `0..10` and inclusive `0..=9`
- **Exhaustive pattern matching** — compiler errors on non-exhaustive switch on enums
- **Raw strings** — `r"..."` and `r#"..."#` with no escape processing
- **Byte literals** — `b'x'` for byte values
- **Rich numeric literals** — binary `0b1010`, octal `0o777`, underscore separators `1_000_000`
- **Ownership** — move semantics, borrowing, region-based allocation, no GC
- **Modules** — import system with public/private visibility
- **Pattern matching** — destructuring enums, Result type, error propagation with `?`
- **Compiler optimizations** — constant folding, dead code elimination
- **Helpful errors** — compiler suggestions with "did you mean?" hints

## Standard Library

| Module | Description |
|--------|-------------|
| `tt::util` | ArrayList, HashMap, Set, Deque, PriorityQueue, Counter, StringBuilder, Vec, Stack, Queue, LinkedList, BitSet, Trie, Graph |
| `tt::io` | File I/O, println, print, Reader, Writer, FileReader, FileWriter, BufferedReader |
| `tt::concurrent` | Future, Channel |
| `tt::crypto` | Hash |
| `tt::encoding` | Base64, Hex, URL encoding |
| `tt::json` | JSON parser and serializer |
| `tt::csv` | CSV reader and writer |
| `tt::math` | Trig, logs, special functions, constants |
| `tt::math::linalg` | Matrix operations and decompositions |
| `tt::math::ndarray` | Multi-dimensional arrays with slicing and broadcasting |
| `tt::random` | Xorshift128+ PRNG |
| `tt::time` | DateTime, Duration, sleep |
| `tt::regex` | Regular expression matching, find, replace |
| `tt::file` | Path operations, directory walking |
| `tt::sys` | Environment variables, CLI args, process control |
| `tt::net` | TCP client/server, HTTP client |
| `tt::xml` | XML parser and writer |
| `tt::units` | Units of measure (Meter, Second, Joule, etc.) |
| `tt::assay` | Testing framework with assertions |
| `tt::lang` | Core types: Integer, Double, String, Boolean, Result, etc. |

## Scientific Computing

Titrate includes a built-in scientific computing stack:

```titrate
import tt::chem::Atom;
import tt::chem::Molecule;
import tt::chem::MD;
import tt::chem::Integrator;

public fn main(): void {
    let system = new Molecule("water_box");
    // Build a box of 1000 water molecules
    // Run 500 steps of MD simulation
    let integ = new Integrator(1.0, 300.0, 100.0);
    let engine = new MD(system, integ);
    engine.initializeVelocities();
    engine.computeForces();
    engine.run(500, 50);
}
```

- **tt.chem** — Molecular dynamics: atoms, bonds, force fields, integrators, RHF
- **tt.math.linalg** — Matrix decompositions (LU, QR, SVD, eigensystems)
- **tt.math.ndarray** — N-dimensional arrays with NumPy-style operations

## Prerequisites

- **Rust** 1.96+ (install via [rustup](https://rustup.rs))
- **LLVM 22.1** (install from [LLVM releases](https://github.com/llvm/llvm-project/releases))
  - On Windows: install to `C:\Program Files\LLVM\`
  - On macOS: `brew install llvm@22`
  - On Linux: install `llvm-22-dev` package or build from source
- **MSVC Build Tools** (Windows only)
  - Install Visual Studio 2022+ with "Desktop development with C++" workload
  - Or install the standalone [Build Tools for Visual Studio](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022)

## Setup

```bash
# Clone the repository
git clone https://github.com/your-org/titrate.git
cd titrate

# Build the compiler and tools
cd trc
cargo build --release

# Verify the build
cargo test --lib
cargo clippy --all -- -D warnings
```

## Building

```bash
cargo build --release
```

The compiler binary is `target/release/trc` (or `target\release\trc.exe` on Windows).
The build tool is `pipette` (`target/release/pipette`).

## Running

Single file:

```bash
./target/release/trc hello.tr
```

Project with pipette:

```bash
./target/release/pipette new myproject
cd myproject
../target/release/pipette run
```

### Running Tests

```bash
# Unit tests (compiler + VM)
cargo test --lib

# Standard library tests
cargo test --test stdlib_test

# End-to-end mega test
cargo test --test mega_test

# All tests
cargo test --all
```

### Packaging

To create a standalone executable:

```bash
cargo build --release
```

Copy `target/release/trc` (or `trc.exe`) to any location on your PATH. The binary is
self-contained and only requires the LLVM runtime DLLs to be accessible (e.g., `LLVM-C.dll`
on Windows, `libLLVM.dylib` on macOS, `libLLVM.so` on Linux).

For distribution, bundle the trc binary alongside:
- `lib/tt/` — the standard library source files
- The LLVM runtime library matching your build target

## Pipette Commands

```
pipette new <name>     Create a new project
pipette build          Compile the project [--release for optimized build]
pipette run            Build and run the project
pipette test           Run tests
pipette bench          Run benchmark files
pipette doc            Generate API documentation
pipette clean          Remove build output directory
pipette lint           Run the analyzer on all .tr files
pipette fmt            Format .tr source files
pipette outdated       Check for newer versions of dependencies
pipette tree           Show the dependency tree
pipette watch          Watch for changes and rebuild
```

## Language

```titrate
// variables
let x: int = 42;
var y: double = 3.14;
const Z: string = "hello";

// closures
let add = fn(a: int, b: int): int => a + b;
io::println(Integer.toString(add(3, 4)));

// tuples with destructuring
let point: (double, double) = (1.5, 2.5);
let (px, py) = point;

// operator overloading
class Vec2 {
    double x;
    double y;
    public Vec2(double x, double y) { this.x = x; this.y = y; }
    public Vec2 operator+(Vec2 other) {
        return new Vec2(this.x + other.x, this.y + other.y);
    }
}

// functions
fn add(a: int, b: int): int {
    return a + b;
}

// generics
fn id<T>(x: T): T {
    return x;
}

// classes with inheritance
class Dog extends Animal {
    string breed;

    public Dog(string name, string breed) {
        super(name);
        this.breed = breed;
    }
}

// exhaustive pattern matching
enum Direction { North(), South(), East(), West() }
switch dir {
    case North() => io::println("N");
    case South() => io::println("S");
    case East()  => io::println("E");
    case West()  => io::println("W");
}

// iterators
let list = new ArrayList<int>();
for (item in list) {
    io::println(Integer.toString(item));
}

// range expressions
for (i in 0..10) { io::println(Integer.toString(i)); }
for (i in 0..=9) { io::println(Integer.toString(i)); }

// raw strings
let pattern: string = r"\d+\.\d+";

// byte literals
let ch: byte = b'A';

// binary, octal, underscore literals
let flags: int = 0b1101_0101;
let perms: int = 0o755;
let million: int = 1_000_000;

// ownership
let owned: Owned<int> = new int(5);
let moved = owned;  // owned is moved

// regions
region r {
    let ptr = r.alloc(42);
}
```

## Documentation

See [docs/](docs/) or visit the hosted docs.

## License

Apache-2.0
