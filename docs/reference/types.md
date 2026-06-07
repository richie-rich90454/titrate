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
| `vast` | 128 | Signed 128-bit integer |
| `uvast` | 128 | Unsigned 128-bit integer |
| `float` | 32 | 32-bit IEEE 754 |
| `double` | 64 | 64-bit IEEE 754 |
| `half` | 16 | 16-bit float (simulated) |
| `quad` | 128 | 128-bit float (simulated) |
| `char` | 32 | Unicode scalar |
| `string` | — | UTF-8 string |
| `size` | ptr | Pointer-sized unsigned |
| `u8` | 8 | Unsigned 8-bit integer |
| `u16` | 16 | Unsigned 16-bit integer |
| `u32` | 32 | Unsigned 32-bit integer |
| `u64` | 64 | Unsigned 64-bit integer |

## Composite Types

- `Owned<T>` — heap-allocated, move-semantics
- `Result<T, E>` — success or error
- `array<T>` — fixed-size array
- Class instances
- Enum instances

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
    T value;
}

fn id<T>(x: T): T {
    return x;
}
```

### Constraints

Type parameters can be constrained to types that implement one or more interfaces:

```titrate
class SortedList<T: Comparable> { ... }
fn print<T: Display>(value: T): void { ... }
fn sortAndPrint<T: Comparable + Display>(items: ArrayList<T>): void { ... }
```

Built-in constraint interfaces:

| Constraint | Requires |
|-----------|----------|
| `Display` | `toString()` method |
| `Numeric` | Arithmetic operators (`+`, `-`, `*`, `/`) |
| `Comparable` | `compareTo(other: T): int` method |

### Monomorphization

Titrate compiles generics via monomorphization. For each concrete type used, the compiler generates a specialized copy of the generic code. This means there is no runtime overhead — `ArrayList<int>` runs just as fast as a hand-written list for integers.
