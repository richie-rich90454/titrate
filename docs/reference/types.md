# Types

## Primitive Types

| Type | Size | Description |
|------|------|-------------|
| `void` | 0 | No value |
| `bool` | 1 | Boolean |
| `byte` | 8 | Signed 8-bit integer |
| `short` | 16 | Signed 16-bit integer |
| `int` | 32 | Signed 32-bit integer |
| `long` | 64 | Signed 64-bit integer |
| `vast` | — | Signed arbitrary-precision integer |
| `uvast` | — | Unsigned arbitrary-precision integer |
| `float` | 32 | 32-bit IEEE 754 |
| `double` | 64 | 64-bit IEEE 754 |
| `half` | 16 | 16-bit IEEE 754 half-precision float |
| `quad` | 128 | 128-bit IEEE 754 quad-precision float |
| `char` | 32 | Unicode scalar |
| `string` | — | UTF-8 string |
| `size` | ptr | Pointer-sized unsigned |
| `u8` | 8 | Unsigned 8-bit integer |
| `u16` | 16 | Unsigned 16-bit integer |
| `u32` | 32 | Unsigned 32-bit integer |
| `u64` | 64 | Unsigned 64-bit integer |

## Composite Types

- `Owned<T>` — single-owner smart pointer type (boxed value)
- `Result<T, E>` — success or error
- `Variant` — dynamic type that can hold values of different types at runtime
- `Array<T>` — fixed-size array (library class in `tt::util::Array`)
- Class instances
- Enum instances

## Tuple Types

Tuples are fixed-size, heterogeneous containers written as parenthesized type lists:

```titrate
let pair: (int, string) = (1, "hello");
let triple: (double, bool, char) = (3.14, true, 'x');
```

A single-element tuple requires a trailing comma: `(int,)`. The empty tuple `()` is the **unit type** (see below).

Tuple elements are accessed by zero-based index with dot notation:

```titrate
let point: (double, double, double) = (1.0, 2.0, 3.0);
let x = point.0;  // 1.0
let y = point.1;  // 2.0
```

Tuples support destructuring in `let` bindings:

```titrate
let (a, b) = (10, 20);
```

## Closure Types

Closures are anonymous functions with the type `fn(ParamTypes): ReturnType`:

```titrate
let add: fn(int, int): int = fn(a: int, b: int): int => a + b;
```

Closures can capture variables from their enclosing scope. The closure type only describes the parameter types and return type — captures are not part of the type signature:

```titrate
fn apply(x: int, f: fn(int): int): int {
    return f(x);
}

let result = apply(5, fn(n: int): int => n * 2);  // 10
```

## Unit Type

The unit type `()` has exactly one value, also written `()`. It represents the absence of a meaningful return value. Functions declared with `: void` implicitly return unit:

```titrate
fn doNothing(): void {
    // implicitly returns ()
}
```

Unit is also the result of statements that don't produce a value, such as assignments:

```titrate
let x: () = (io::println("hi"));  // println returns ()
```

## Type Casting

Use the `as` keyword to cast between compatible types:

```titrate
let big: long = 99999 as long;
let d: double = 42 as double;
let ch: char = 65 as char;
```

## Type Parameters

```titrate
ArrayList<int>
HashMap<string, double>
```

## Generic Type Parameters

Generic type parameters allow you to write code that works across multiple types while preserving type safety.

### Declaration

Type parameters are declared in angle brackets after a class or function name:

```titrate
class Box<T> {
    public T value;
}

fn id<T>(x: T): T {
    return x;
}
```

### Constraints

Type parameters can be constrained to a type that implements a specific interface:

```titrate
class SortedList<T: Comparable> { ... }
fn print<T: Display>(value: T): void { ... }
```

Constraints can also be specified via a `where` clause after the function signature:

```titrate
fn sortAndPrint<T>(items: ArrayList<T>): void where T: Comparable { ... }
fn convert<T, R>(input: T): R where T: Display, R: Numeric { ... }
```

Built-in constraint interfaces (enforced at monomorphization time by the compiler):

| Constraint | Requires |
|-----------|----------|
| `Display` | `toString()` method |
| `Numeric` | Arithmetic operators (`+`, `-`, `*`, `/`) |
| `Comparable` | `compareTo(other: T): int` method |

### Monomorphization

Titrate compiles generics via monomorphization. For each concrete type used, the compiler generates a specialized copy of the generic code. This means there is no runtime overhead — `ArrayList<int>` runs just as fast as a hand-written list for integers.

## Reference Types

Titrate supports reference types for advanced memory management:

```titrate
// Immutable reference
let ref: &int = &value;

// Mutable reference
let mutRef: &mut int = &mut value;
```

Reference types are used internally by the ownership system and are not commonly needed in application code.

## Optional Type

`Optional<T>` provides null-safe access to values that may be absent:

```titrate
let opt: Optional<int> = Optional.of(42);
if (opt.isPresent()) {
    let val: int = opt.get();
}
```

## DataFile and External Data

The `DataFile` system loads reference data from external JSON files at runtime, keeping `.tr` source files clean:

```titrate
import tt.lang.DataFile;

let data: JsonValue = DataFile.load("chem/periodic_table.json");
let meta: JsonValue = DataFile.meta("chem/periodic_table.json");
```

All data files include a `_meta` object with source, version, and description. No `.tr` file should contain more than 5 literal reference values.

## Extended Tuple Types

Beyond 2-element tuples, Titrate supports `Tuple3`, `Tuple4`, and `Tuple5`:

```titrate
let t3 = new Tuple3(1, "hello", 3.14);
let first = t3.getFirst();    // 1
let second = t3.getSecond();  // "hello"
let third = t3.getThird();    // 3.14
```

Generic (typed) versions are also available via `tt::lang::Tuple3<A, B, C>`, `tt::lang::Tuple4<A, B, C, D>`, and `tt::lang::Tuple5<A, B, C, D, E>`, using `get0()`, `get1()`, etc. for access.
