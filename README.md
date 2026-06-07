# Titrate

A systems programming language. Precise. Safe. Expressive.

```titrate
import tt::io::println;
import tt::util::ArrayList;

public fn main(): void {
    let list = new ArrayList<int>();
    list.add(42);
    list.add(7);
    list.add(13);
    list.sort();
    for n in list {
        println(Integer.toString(n));
    }
}
```

## Features

- **Bytecode VM** — compiles to optimized bytecode, 10x faster than tree-walking
- **Precise types** — byte to quad, unsigned fixed-width to 128-bit integers
- **Generics** — monomorphized at compile time, zero runtime overhead
- **Ownership** — move semantics, borrowing, region-based allocation, no GC
- **Modules** — import system with public/private visibility
- **Pattern matching** — destructuring enums, Result type, error propagation with `?`

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
