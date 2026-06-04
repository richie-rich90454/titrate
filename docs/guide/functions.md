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
