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
    details: Compile to optimized bytecode and run on the Titrate VM with built-in garbage-free memory management.
  - icon:
      src: /icons/compat.svg
    title: LLVM Native Backend
    details: Compile to standalone native executables via LLVM. Release-mode builds run three to six times faster.
  - icon:
      src: /icons/shield.svg
    title: Ownership and Regions
    details: Move semantics, borrowing and region-based allocation for memory safety without garbage collection.
  - icon:
      src: /icons/generics.svg
    title: Zero-Cost Generics
    details: Monomorphizing compiler generates specialized code for each type instance. No runtime overhead.
  - icon:
      src: /icons/flask.svg
    title: Scientific Computing
    details: Bioinformatics, physics, materials science, signal processing, ML, and more in the standard library.
  - icon:
      src: /icons/result.svg
    title: Result-Based Error Handling
    details: No exceptions, no null pointer errors. Use Result<T, E> for explicit, type-safe error handling.
  - icon:
      src: /icons/operator.svg
    title: Operator Overloading
    details: Define natural syntax for your types. Build expressive DSLs for math, physics and data.
  - icon:
      src: /icons/library.svg
    title: Rich Standard Library
    details: Collections, I/O, JSON, CSV, XML, TCP, HTTP, crypto, bioinformatics, physics, ML, and more.
---

## Language at a Glance

```titrate
// Variables -- let for mutable with inference, var for explicit type
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
    let n = Integer.parseInt(s);
    if (n == 0 && s != "0") {
        return err("not a number");
    }
    return ok(n * 2);
}

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

## Quick Start

```bash
# Clone and build
git clone https://github.com/richie-rich90454/titrate.git
cd titrate
cargo build --release

# Create and run your first program
echo 'public fn main(): void { io::println("Hello, Titrate!"); }' > hello.tr
trc hello.tr

# Or compile to native executable (3-6x faster)
trc --native --release hello.tr
./hello
```

## Why Titrate?

### Type Safety Without Runtime Cost

Titrate's monomorphizing compiler generates specialized code for each generic type instance. `ArrayList<int>` runs just as fast as hand-written code for integers -- no boxing, no type checks at runtime.

### Memory Safety Without Garbage Collection

Ownership semantics, move checking and region-based allocation give you memory safety guarantees without the pause times and overhead of a garbage collector.

### Clean, Modern Syntax

Inspired by Rust and Python, Titrate uses `name: Type` parameter order, `fn` declarations and lowercase `string` from the start. C-family sugar forms (`int x = 5`, `++i`, `ClassName(params)`) make developers from C, C++ and ECMAScript feel at home immediately.

### Scientific Computing Built in

Chemistry simulations, bioinformatics, physics engines, machine learning, signal processing, image/audio processing, computational geometry, NLP, HFT and discrete-event simulation are all part of the standard library.

## Comparison

### Molecular Dynamics Force Calculation

**Titrate** - Memory safe, zero-cost generics, scientific stdlib built in:

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
    let sig6: double = MathAdvanced.pow(a.sig, 6.0);
    let sig12: double = sig6 * sig6;
    return 24.0 * a.eps * (2.0 * sig12 / r12 - sig6 / r6) / r2;
}
```

**Python** - Simple syntax, but 10-100x slower, GC pauses:

```python
def lj_force(a, b):
    dx = b.x - a.x
    dy = b.y - a.y
    dz = b.z - a.z
    r2 = dx*dx + dy*dy + dz*dz
    r6 = r2 ** 3
    r12 = r6 * r6
    sig6 = a.sig ** 6
    sig12 = sig6 * sig6
    return 24.0 * a.eps * (2.0 * sig12 / r12 - sig6 / r6) / r2
```

**C++** - Fast, but manual memory management, no safety:

```cpp
double lj_force(Atom* a, Atom* b) {
    double dx = b->x - a->x;
    double dy = b->y - a->y;
    double dz = b->z - a->z;
    double r2 = dx*dx + dy*dy + dz*dz;
    double r6 = r2 * r2 * r2;
    double r12 = r6 * r6;
    double sig6 = a->sig * a->sig * a->sig * a->sig * a->sig * a->sig;
    double sig12 = sig6 * sig6;
    return 24.0 * a->eps * (2.0 * sig12 / r12 - sig6 / r6) / r2;
}
```

| Feature | Titrate | Python | C++ |
|---------|---------|--------|-----|
| Memory Safety | Yes (ownership) | Yes (GC) | No |
| Garbage Collector | No | Yes | No |
| Zero-Cost Generics | Yes | No | Partial |
| Scientific Stdlib | Built-in | External packages | External libraries |
| Error Handling | Result<T,E> | Exceptions | Exceptions/codes |
| Compile Time | Fast | N/A (interpreted) | Slow |
| Runtime Performance | 3-6x faster than VM | Baseline | Similar to Titrate |
