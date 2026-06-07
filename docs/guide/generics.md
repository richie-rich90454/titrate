# Generics

Titrate supports generics for both classes and functions, with interface constraints and compile-time monomorphization.

## Generic Classes

Declare type parameters in angle brackets after the class name:

```titrate
class Box<T> {
    T value;

    public Box(T value) {
        this.value = value;
    }

    public fn get(): T {
        return this.value;
    }

    public fn set(newValue: T): void {
        this.value = newValue;
    }
}
```

Instantiate with concrete types:

```titrate
let intBox = new Box<int>(42);
let strBox = new Box<string>("hello");

io::println(intBox.get().toString());  // 42
io::println(strBox.get());             // hello
```

## Generic Functions

Type parameters can also appear on functions:

```titrate
fn id<T>(x: T): T {
    return x;
}

fn first<T>(a: T, b: T): T {
    return a;
}
```

Call them with explicit or inferred type arguments:

```titrate
let x = id<int>(10);       // explicit
let y = id("inferred");    // T inferred as string
```

## Interface Constraints

Restrict type parameters to types that implement a given interface:

```titrate
fn print<T: Display>(value: T): void {
    io::println(value.toString());
}

fn compare<T: Comparable>(a: T, b: T): int {
    return a.compareTo(b);
}

fn sum<T: Numeric>(a: T, b: T): T {
    return a + b;
}
```

Multiple constraints can be specified:

```titrate
fn sortAndPrint<T: Comparable + Display>(items: ArrayList<T>): void {
    items.sort();
    for (item in items) {
        io::println(item.toString());
    }
}
```

## Monomorphization

Titrate compiles generics via **monomorphization** — the compiler generates a separate copy of the generic code for each concrete type used in the program.

This means:

- **Zero runtime overhead** — generic code runs at the same speed as hand-written specialized code.
- **No boxing** — primitive types like `int` and `double` are used directly, not wrapped in objects.
- **Compile-time duplication** — the compiler creates specialized versions at compile time, so the VM never needs to reason about type parameters.

```titrate
// The compiler generates Box_int and Box_string internally
let a = new Box<int>(1);
let b = new Box<string>("x");
```

## Using Generic Collections

The standard library provides generic collections that take full advantage of monomorphization:

```titrate
import tt::util::{ArrayList, HashMap};

let list = new ArrayList<int>();
list.add(1);
list.add(2);
list.add(3);

let map = new HashMap<string, int>();
map.put("one", 1);
map.put("two", 2);

io::println(map.get("one").toString());  // 1
```
