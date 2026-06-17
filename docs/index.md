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
      src: /icons/shield.svg
    title: Ownership and Regions
    details: Move semantics, borrowing, and region-based allocation -- memory safety without garbage collection or manual free.
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
    details: Bioinformatics, physics simulation, materials science, signal processing, image/audio processing, ML, computational geometry, and more -- all in the standard library.
  - icon:
      src: /icons/compat.svg
    title: C-family Compatibility
    details: Familiar syntax for developers from C, C++, and ECMAScript. Sugar forms like `int x = 5` and `++i` alongside canonical Titrate style.
  - icon:
      src: /icons/result.svg
    title: Result-Based Error Handling
    details: No exceptions, no null pointer errors. Use `Result<T, E>` with `ok()` and `err()` to make error handling explicit and type-safe at compile time.
  - icon:
      src: /icons/operator.svg
    title: Operator Overloading
    details: Define natural syntax for your types with `fn operator+`, `fn operator*`, and more. Build expressive DSLs for math, physics, and data.
  - icon:
      src: /icons/library.svg
    title: Rich Standard Library
    details: Collections, I/O, JSON, CSV, XML, TCP, HTTP, SHA-256, HMAC, Base64, bioinformatics, physics, ML, HFT, simulation -- everything you need, out of the box.
---

## Quick Start

```bash
# Build the compiler
cargo build --release

# Run your first program
echo 'public fn main(): void { io::println("Hello, Titrate!"); }' > hello.tr
trc hello.tr

# Or use the build tool
pipette new myproject
pipette run
```

## Language at a Glance

```titrate
// Variables -- let for immutable, var for mutable
let name: string = "Titrate";
var count: int = 0;

// Functions with name: Type parameter order
public fn greet(name: string): void {
    io::println("Hello, " + name);
}

// Classes with fn init() constructors
class Point {
    public double x;
    public double y;

    public fn init(x: double, y: double) {
        this.x = x;
        this.y = y;
    }
}

// Generics with monomorphization -- zero runtime overhead
class Box<T> {
    public T value;
    public fn init(value: T) { this.value = value; }
}

// Result-based error handling -- no exceptions
fn parse(s: string): Result<int, string> {
    let n: Result<int, string> = Integer.parseInt(s);
    if (n.isOk()) { return ok(n.unwrap() * 2); }
    return err("not a number");
}

// Closures that capture by reference
let double = fn(x: int): int => x * 2;
let numbers = new ArrayList<int>();
numbers.forEach(fn(n: int): void {
    io::println(Integer.toString(double(n)));
});

// Operator overloading
class Vec2 {
    public double x;
    public double y;
    fn operator+(self, other: Vec2): Vec2 {
        return new Vec2(self.x + other.x, self.y + other.y);
    }
}

// Ranges and iterators
for (i in 0..10) {
    io::println(Integer.toString(i));
}
```

## See It In Action

### Molecular Dynamics Simulation

```titrate
import tt::chem::Atom;
import tt::chem::Molecule;
import tt::chem::ForceField;
import tt::chem::MD;

public fn main(): void {
    let water: Molecule = new Molecule();
    water.addAtom(new Atom("O", 0.0, 0.0, 0.0));
    water.addAtom(new Atom("H", 0.9572, 0.0, 0.0));
    water.addAtom(new Atom("H", -0.2399, 0.9270, 0.0));

    let ff: ForceField = new ForceField();
    ff.addBondTerm(0, 1, 450.0, 0.9572);
    ff.addAngleTerm(1, 0, 2, 55.0, 104.52);

    let md: MD = new MD(water, ff, new Integrator(1.0));
    md.run(1000);

    io::println("Energy: " + Double.toString(ff.energy(water)));
}
```

### JSON API Client with Error Handling

```titrate
import tt::json::Json;
import tt::json::JsonValue;

public fn fetchUser(id: int): Result<JsonValue, string> {
    let client: HttpClient = new HttpClient();
    let url: string = "https://api.example.com/users/" + Integer.toString(id);
    let response: Result<string, string> = client.get(url);

    if (response.isOk()) {
        let parsed: Result<JsonValue, string> = Json.parse(response.unwrap());
        if (parsed.isOk()) { return ok(parsed.unwrap()); }
        return err("Failed to parse JSON");
    }
    return err("HTTP request failed: " + response.unwrapErr());
}
```

### Data Processing with NDArray

```titrate
import tt::ndarray::NDArray;
import tt::math::Math;

public fn normalize(data: NDArray<double>): NDArray<double> {
    let mean: double = data.mean();
    let std: double = data.std();
    return (data - mean) / std;
}

public fn correlation(x: NDArray<double>, y: NDArray<double>): double {
    let nx: NDArray<double> = normalize(x);
    let ny: NDArray<double> = normalize(y);
    return nx.dot(ny) / (nx.size() as double);
}
```

### Custom Collection with Generics

```titrate
class RingBuffer<T> implements Iterable<T> {
    private ArrayList<T> data;
    private int head;
    private int count;

    public fn init(capacity: int) {
        this.data = new ArrayList<T>();
        this.head = 0;
        this.count = 0;
    }

    fn push(self, item: T): void {
        self.data.add(item);
        self.count = self.count + 1;
    }

    fn iterator(self): Iterator<T> {
        return new RingBufferIterator<T>(self);
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

Ownership semantics, move checking, and region-based allocation give you memory safety guarantees without the pause times and overhead of a garbage collector. When a value goes out of scope, it is cleaned up immediately.

### Clean, Modern Syntax

Inspired by Rust and Python, Titrate uses `name: Type` parameter order, `fn` declarations, and lowercase `string` from the start. But it also supports C-family sugar forms (`int x = 5`, `++i`, `ClassName(params)`) so developers from C, C++, and ECMAScript feel at home immediately.

### Scientific Computing Built In

Chemistry simulations (Atom, Molecule, ForceField, MD, RHF), bioinformatics (Sequence, Alignment, PhyloTree), physics (Particle, ForceField, NBodySimulator), machine learning (Tensor, Model, Optimizer), signal processing (FFT2, Filter, Wavelet), image processing, audio processing, computational geometry, NLP, HFT, and discrete-event simulation are all part of the standard library -- not third-party packages.

### Comprehensive Standard Library

Collections (ArrayList, HashMap, HashSet, Vec), I/O (File, BufferedReader), serialization (JSON, CSV, XML), networking (TCP, HTTP), cryptography (SHA-256, HMAC, Base64), and more -- all included out of the box.

## Comparison

How does Titrate compare to other systems languages for common tasks?

| Feature | Titrate | C | Rust | Python |
|---------|:-------:|:---:|:----:|:------:|
| Memory safety | Yes | No | Yes | Yes |
| No garbage collector | Yes | Yes | Yes | No |
| Zero-cost generics | Yes | No | Yes | No |
| Simple syntax | Yes | No | No | Yes |
| Scientific computing stdlib | Yes | No | No | Yes |
| Result-based error handling | Yes | No | Yes | No |
| Operator overloading | Yes | No | Yes | Yes |
| Fast compile times | Yes | Yes | No | Yes |
| C-family sugar forms | Yes | Yes | No | No |

## Standard Library Ecosystem

Titrate ships with a comprehensive standard library organized into intuitive modules:

| Module | Key Types | Description |
|--------|-----------|-------------|
| `tt::util` | ArrayList, HashMap, HashSet, Vec, Stack, Queue | Core collections with generic support |
| `tt::io` | File, BufferedReader, BufferedWriter, Path | File I/O and filesystem operations |
| `tt::json` | JsonValue, Json, JsonPath, JsonSchema, Json5 | JSON parsing, serialization, querying, validation |
| `tt::xml` | XmlNode, Xml, XPath, XmlBuilder, XmlSchema | XML parsing, XPath, schema validation, C14N |
| `tt::math` | Math, Random, Statistics, Special | Mathematical functions, distributions, special functions |
| `tt::ndarray` | NDArray, Matrix | N-dimensional arrays and linear algebra |
| `tt::chem` | Atom, Molecule, ForceField, MD, RHF | Computational chemistry toolkit |
| `tt::bio` | Sequence, Alignment, PhyloTree, CodonTable | Bioinformatics and sequence analysis |
| `tt::physics` | Particle, ForceField, NBodySimulator, RigidBody | Physics simulation |
| `tt::materials` | CrystalStructure, XRayDiffraction, Elasticity | Materials science |
| `tt::sigproc` | FFT2, Filter, Wavelet, Spectrogram | Signal processing |
| `tt::image` | Image, Kernel, Morphology, Threshold | Image processing |
| `tt::audio` | AudioBuffer, Pitch, Mfcc | Audio processing |
| `tt::ml` | Tensor, Layer, Optimizer, Model | Machine learning |
| `tt::geom` | ConvexHull, Delaunay, SpatialIndex | Computational geometry |
| `tt::nlp` | Tokenizer, Stemmer, Classifier | Natural language processing |
| `tt::hft` | FixParser, OrderRouter, RiskManager, Backtest | High-frequency trading |
| `tt::sim` | Simulation, Resource, Process, Monitor | Discrete-event simulation |
| `tt::finance` | BlackScholes, Portfolio, YieldCurve | Quantitative finance |
| `tt::crypto2` | AES, RSA, ECDSA, KDF | Advanced cryptography |
| `tt::net` | TcpClient, TcpServer, HttpClient | Networking primitives |
| `tt::crypto` | SHA256, HMAC, Base64 | Cryptographic primitives |
| `tt::argparse` | ArgumentParser | CLI argument parsing |
| `tt::csv` | CsvReader, CsvWriter | CSV file handling |

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
