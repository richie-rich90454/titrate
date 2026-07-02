# Functions

Functions are the building blocks of any Titrate program. Whether you're writing a quick utility or a complex algorithm, functions let you encapsulate logic, give it a name and reuse it throughout your code. Titrate's function syntax is clean and consistent — parameters use `name: Type` order, and return types come after the parameters with `: ReturnType`.

## Canonical Form

The standard way to write a function in Titrate uses the `fn` keyword with `name: Type` parameter order:

```titrate
public fn greet(name: string): void {
    io::println("Hello, " + name);
}
```

Let's break this down:

- **`public`** — makes the function visible outside the current module. Omit it for module-private functions.
- **`fn`** — the keyword that declares a function.
- **`greet`** — the function name.
- **`name: string`** — a parameter, with the name first, then a colon, then the type.
- **`: void`** — the return type. This function doesn't return a value.
- **`{ ... }`** — the function body.

### Try It Yourself

Write a function that takes two integers and returns their sum, then call it from `main()`:

```titrate
fn add(a: int, b: int): int {
    return a + b;
}

public fn main(): void {
    let result: int = add(3, 7);
    io::println(Integer.toString(result));  // 10
}
```

Try modifying it to take three parameters and return the sum of all three.

## Sugar Form (C-family Compatibility)

Titrate supports a sugar form that will feel familiar to developers coming from C, C++, ECMAScript, or similar languages:

```titrate
// Sugar form — familiar to C/ECMAScript developers
public void greet(string name) {
    io::println("Hello, " + name);
}
```

This is automatically desugared into the canonical `fn` form during parsing. However, **the canonical `fn` form is the recommended Titrate style**. Use the sugar form only during migration or if your team prefers C-family syntax:

```titrate
// Canonical form — recommended for all Titrate code
public fn greet(name: string): void {
    io::println("Hello, " + name);
}
```

::: tip
The sugar form exists to ease migration from C-family languages, but all new Titrate code should use the canonical `fn` form. It's more consistent with the rest of the language — constructors, methods, and closures all use `name: Type` parameter order.
:::

## Return Values

Functions that produce a value specify the return type after the parameters. Use `return` to send a value back to the caller:

```titrate
fn add(a: int, b: int): int {
    return a + b;
}
```

A function with `: void` return type doesn't need a `return` statement — control simply falls off the end. If you want to exit early from a `void` function, you can use a bare `return;`.

## Generic Functions

Functions can declare type parameters to operate on any type. This is incredibly powerful — write the logic once, and it works for `int`, `string`, or any other type:

```titrate
fn id<T>(x: T): T {
    return x;
}

fn first<T>(a: T, b: T): T {
    return a;
}
```

The `<T>` after the function name declares a type parameter. You can use `T` anywhere in the function signature and body — as a parameter type, return type, or inside the body. The compiler fills in the concrete type when you call the function.

Add interface constraints to restrict the type parameter to types that support certain operations:

```titrate
fn print<T: Display>(value: T): void {
    io::println(value.toString());
}

fn max<T: Comparable<T>>(a: T, b: T): T {
    if (a.compareTo(b) >= 0) {
        return a;
    }
    return b;
}
```

The constraint `<T: Display>` means "T can be any type, as long as it implements `Display`." This lets you safely call `value.toString()` because the compiler guarantees that `T` has that method.

::: tip
Generic functions are most useful when the same logic applies to multiple types. If you find yourself writing nearly identical functions that differ only in type, that's a sign you should use a generic function.
:::

See [Generics](./generics) for the full generics guide.

## Recursive Functions

A function can call itself — this is called recursion. Titrate supports recursion just like any other function call. The classic example is computing a factorial:

```titrate
fn factorial(n: int): int {
    if (n <= 1) {
        return 1;
    }
    return n * factorial(n - 1);
}

public fn main(): void {
    io::println(Integer.toString(factorial(5)));  // 120 (five factorial)
}
```

Recursion works because each function call gets its own stack frame with its own local variables. The key is having a **base case** (here, `n <= 1`) that stops the recursion, and a **recursive case** that moves toward the base case.

Another common pattern — recursively traversing a data structure:

```titrate
fn sumList(list: ArrayList<int>, index: int): int {
    if (index >= list.size()) {
        return 0;
    }
    return list.get(index) + sumList(list, index + 1);
}
```

::: warning
Titrate does not guarantee tail-call optimization. For very deep recursion, consider using an iterative approach (a `while` loop) instead to avoid stack overflow.
:::

## Functions vs Methods

Titrate has two places to put behavior: **top-level functions** and **class methods**. Here's how to decide which to use:

### Top-level Functions

Use top-level `fn` declarations when:

- **The function doesn't need instance state.** Utility functions like `max(a, b)` or `parseInt(s)` don't belong to any object.
- **You're writing a factory or constructor helper.** Since Titrate only allows one `fn init()` per class, factory functions are top-level by nature.
- **The function is a standalone operation.** If it takes all its inputs as parameters and doesn't need `this`, it's probably a top-level function.

```titrate
// Top-level function — no instance needed
public fn clamp(value: int, min: int, max: int): int {
    if (value < min) { return min; }
    if (value > max) { return max; }
    return value;
}
```

### Class Methods

Use methods (functions inside a class) when:

- **The function operates on instance data.** `area()` uses `this.radius` — it belongs on the `Circle` class.
- **The function is conceptually part of the object.** `speak()` is something a `Dog` does, not a standalone operation.

```titrate
// Method — uses instance data via this
public fn area(): double {
    return 3.14159 * this.radius * this.radius;
}
```

### A Simple Rule of Thumb

If the function needs `this`, it should be a method. If it doesn't, it should be a top-level function. Titrate doesn't have a `static` keyword — there's no way to put a "static" function inside a class, and that's by design. Top-level functions are the idiomatic way to write stateless logic.

## Function Overloading

Titrate does **not** support function overloading — you cannot define two functions with the same name but different parameter lists. Each function name must be unique within its scope.

If you need a function to handle different types, use **generics**:

```titrate
// Instead of overloading, use a generic function
fn identity<T>(x: T): T {
    return x;
}
```

If you need a function to handle different numbers of parameters, use **different names**:

```titrate
// Instead of overloading, use descriptive names
fn makePoint(x: double, y: double): Point {
    return new Point(x, y);
}

fn makePoint3D(x: double, y: double, z: double): Point3D {
    return new Point3D(x, y, z);
}
```

This keeps things explicit — the caller always knows exactly which function they're calling.
