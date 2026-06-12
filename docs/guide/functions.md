# Functions

## Canonical Form

```titrate
public fn greet(name: string): void {
    io::println("Hello, " + name);
}
```

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

## Return Values

```titrate
fn add(a: int, b: int): int {
    return a + b;
}
```

## Generic Functions

Functions can declare type parameters to operate on any type:

```titrate
fn id<T>(x: T): T {
    return x;
}

fn first<T>(a: T, b: T): T {
    return a;
}
```

Add interface constraints to restrict the type parameter:

```titrate
fn print<T: Display>(value: T): void {
    io::println(value.toString());
}

fn max<T: Comparable>(a: T, b: T): T {
    if (a.compareTo(b) >= 0) {
        return a;
    }
    return b;
}
```

See [Generics](./generics) for the full generics guide.
