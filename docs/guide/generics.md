# Generics

Ever written the same function twice, just for different types? That's exactly the problem generics solve. Instead of writing `printInt`, `printString`, and `printDouble`, you write one `print<T>` that works for any type. Generics let you write flexible, reusable code without sacrificing type safety — the compiler still catches type errors, but your code works across types.

Titrate supports generics for both classes and functions, with interface constraints and compile-time monomorphization.

## Generic Classes

Declare type parameters in angle brackets after the class name. The type parameter `T` can then be used anywhere inside the class — for fields, method parameters, and return types:

```titrate
class Box<T> {
    public T value;

    public fn init(value: T) {
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

io::println(Integer.toString(intBox.get()));  // 42
io::println(strBox.get());             // hello
```

When you write `new Box<int>(42)`, the compiler replaces every `T` in the `Box` class with `int`. When you write `new Box<string>("hello")`, it replaces `T` with `string`. Same class definition, two different types — no code duplication.

## Generic Functions

Type parameters can also appear on functions. This is especially useful for utility functions that should work with any type:

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

When the compiler can figure out the type from the argument, you can omit the type argument entirely. But when it's ambiguous, provide it explicitly.

### Try It Yourself

Write a generic `swap` function that takes a `Box<T>` and two values, replacing the box's value with the first argument and returning the old value:

```titrate
fn swap<T>(box: Box<T>, newValue: T): T {
    let old: T = box.get();
    box.set(newValue);
    return old;
}

public fn main(): void {
    let b: Box<string> = new Box<string>("original");
    let old: string = swap(b, "replaced");
    io::println(old);         // original
    io::println(b.get());     // replaced
}
```

Try creating a `Box<int>` and swapping its value too — the same `swap` function works for both types.

## Interface Constraints

By default, a type parameter `T` can be *any* type — but sometimes you need `T` to support certain operations. Interface constraints let you restrict the type parameter to types that implement a given interface:

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

The constraint `<T: Display>` means "T can be any type that implements `Display`." Inside the function, you can safely call any method that `Display` defines — the compiler guarantees it exists.

Multiple constraints can be specified with `+`:

```titrate
fn sortAndPrint<T: Comparable + Display>(items: ArrayList<T>): void {
    items.sort();
    for (item in items) {
        io::println(item.toString());
    }
}
```

This function only accepts types that are both `Comparable` (so they can be sorted) and `Display` (so they can be printed). If you try to call it with a type that doesn't implement both, you'll get a compile-time error.

## Monomorphization

Titrate compiles generics via **monomorphization** — the compiler generates a separate copy of the generic code for each concrete type used in the program.

This means:

- **Zero runtime overhead** — generic code runs at the same speed as hand-written specialized code.
- **No boxing** — primitive types like `int` and `double` are used directly, not wrapped in objects.
- **Compile-time duplication** — the compiler creates specialized versions at compile time, so the VM never needs to reason about type parameters.

### A Concrete Example

Consider this generic function:

```titrate
fn double<T: Numeric>(x: T): T {
    return x + x;
}
```

If your program calls `double` with both `int` and `double`:

```titrate
let a: int = double(5);        // T = int
let b: double = double(3.14);  // T = double
```

The compiler generates two specialized versions behind the scenes — conceptually something like:

```
fn double_int(x: int): int { return x + x; }
fn double_double(x: double): double { return x + x; }
```

Your source code stays clean and generic, but the VM executes fully specialized, no-overhead code. This is the same approach used by languages like Rust and C++.

```titrate
// The compiler generates Box_int and Box_string internally
let a = new Box<int>(1);
let b = new Box<string>("x");
```

## Common Generic Patterns

### The Wrapper (Container) Pattern

A generic class that holds a single value of type `T`:

```titrate
class Box<T> {
    public T value;

    public fn init(value: T) {
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

### The Collection Pattern

A generic class that holds multiple values of type `T`:

```titrate
class Stack<T> {
    private ArrayList<T> items;

    public fn init() {
        this.items = new ArrayList<T>();
    }

    public fn push(item: T): void {
        this.items.add(item);
    }

    public fn pop(): T {
        let top: T = this.items.get(this.items.size() - 1);
        this.items.remove(this.items.size() - 1);
        return top;
    }

    public fn isEmpty(): bool {
        return this.items.size() == 0;
    }
}
```

### The Transform Pattern

A generic function that converts from one type to another:

```titrate
fn map<T, R>(list: ArrayList<T>, f: fn(T): R): ArrayList<R> {
    let result: ArrayList<R> = new ArrayList<R>();
    var i: int = 0;
    while (i < list.size()) {
        result.add(f(list.get(i)));
        i = i + 1;
    }
    return result;
}
```

Notice the two type parameters — `T` for the input type, `R` for the output type. This lets you transform a list of one type into a list of another.

## Generics in the Standard Library

Titrate's standard library makes heavy use of generics. Here are some of the most commonly used generic types:

| Type | Description | Example |
|------|-------------|---------|
| `ArrayList<E>` | Dynamic array | `new ArrayList<string>()` |
| `HashMap<K, V>` | Key-value map | `new HashMap<string, int>()` |
| `Result<T, E>` | Success or error | `Result<int, string>` |
| `Stack<T>` | LIFO stack | `new Stack<int>()` |

### Using Generic Collections

```titrate
import tt::util::ArrayList;
import tt::util::HashMap;

let list = new ArrayList<int>();
list.add(1);
list.add(2);
list.add(3);

let map = new HashMap<string, int>();
map.put("one", 1);
map.put("two", 2);

io::println(Integer.toString(map.get("one")));  // 1
```

::: tip
Always provide type parameters when creating generic instances. Writing `new ArrayList()` without the type parameter is not valid in Titrate — you must write `new ArrayList<string>()`. This ensures type safety from the start.
:::
