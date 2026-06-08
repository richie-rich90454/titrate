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
- **Ownership** — move semantics, borrowing, region-based allocation, no GC
- **Modules** — import system with public/private visibility
- **Pattern matching** — destructuring enums, Result type, error propagation with `?`

## Standard Library

| Module | Description |
|--------|-------------|
| `tt::util` | ArrayList, HashMap, Set, Deque, PriorityQueue, Counter, StringBuilder, Vec |
| `tt::io` | File I/O, println, print |
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
| `tt::lang` | Core types: Integer, Double, String, Boolean, etc. |

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

## Building

```bash
cargo build --release
```

The compiler binary is `trc`. The build tool is `pipette`.

## Running

Single file:

```bash
trc hello.tr
```

Project with pipette:

```bash
pipette new myproject
cd myproject
pipette run
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

// pattern matching
switch result {
    case Ok(val) => io::println(Integer.toString(val));
    case Err(msg) => io::println("error: " + msg);
}

// iterators
let list = new ArrayList<int>();
for (item in list) {
    io::println(Integer.toString(item));
}

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
