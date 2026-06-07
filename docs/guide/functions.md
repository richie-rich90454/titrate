# Functions

## Canonical Form

```titrate
public fn greet(name: string): void {
    io::println("Hello, " + name);
}
```

## Sugar Form

```titrate
public void greet(string name) {
    io::println("Hello, " + name);
}
```

The sugar form is desugared into the canonical form during parsing.

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
