# Tuples

Tuples group multiple values into a single compound value. They are lightweight, fixed-size, and can hold values of different types.

## Creating Tuples

Use parentheses with comma-separated values:

```titrate
let pair = (1, "hello");
let triple = (3.14, true, 'x');
let nested = ((1, 2), (3, 4));
```

An empty tuple `()` is the **unit value**, representing the absence of a meaningful value (similar to `void` in expression position).

## Type Annotations

Tuple types are written as parenthesized, comma-separated type lists:

```titrate
let pair: (int, string) = (1, "hello");
let triple: (double, bool, char) = (3.14, true, 'x');
let nested: ((int, int), (int, int)) = ((1, 2), (3, 4));
```

Single-element tuples require a trailing comma to distinguish from a parenthesized expression:

```titrate
let single: (int,) = (42,);
```

## Destructuring

Unpack a tuple into individual variables with `let`:

```titrate
let pair = (1, "hello");
let (x, y) = pair;
io::println(x.toString());  // 1
io::println(y);              // hello
```

Use `_` to ignore elements you don't need:

```titrate
let triple = (10, 20, 30);
let (first, _, third) = triple;
io::println(first.toString());  // 10
io::println(third.toString());  // 30
```

Destructuring also works in `for-in` loops when iterating over collections of tuples:

```titrate
let pairs = new ArrayList<(int, string)>();
pairs.add((1, "one"));
pairs.add((2, "two"));

pairs.forEach(fn(pair: (int, string)): void {
    let (num, word) = pair;
    io::println(num.toString() + " = " + word);
});
```

## Tuples as Return Types

Functions can return tuples to deliver multiple values without defining a class:

```titrate
fn minMax(a: int, b: int): (int, int) {
    if (a <= b) {
        return (a, b);
    }
    return (b, a);
}

let (lo, hi) = minMax(42, 7);
io::println(lo.toString());  // 7
io::println(hi.toString());  // 42
```

### Swapping Values

```titrate
fn swap(a: int, b: int): (int, int) {
    return (b, a);
}

let (x, y) = swap(1, 2);
io::println(x.toString());  // 2
io::println(y.toString());  // 1
```

### Multiple Computed Results

```titrate
fn divMod(a: int, b: int): (int, int) {
    let quotient = a / b;
    let remainder = a % b;
    return (quotient, remainder);
}

let (q, r) = divMod(17, 5);
io::println(q.toString());  // 3
io::println(r.toString());  // 2
```

## Accessing Elements by Index

Tuple elements can be accessed with dot-index notation:

```titrate
let point = (10, 20, 30);
io::println(point.0.toString());  // 10
io::println(point.1.toString());  // 20
io::println(point.2.toString());  // 30
```

## Tuples and Generics

Tuples work naturally with generic types:

```titrate
let entries = new ArrayList<(string, int)>();
entries.add(("Alice", 30));
entries.add(("Bob", 25));

entries.forEach(fn(entry: (string, int)): void {
    let (name, age) = entry;
    io::println(name + " is " + age.toString());
});
```

## Unit Type

The empty tuple `()` is the **unit type**. It has exactly one value, also written `()`. Functions that don't return a meaningful value implicitly return unit:

```titrate
fn sayHi(): void {
    io::println("Hi!");
}
// The void return is equivalent to returning ()
```

## What's Next?

- [Closures](./closures) — anonymous functions that capture their environment
- [Pattern Matching](./pattern-matching) — destructuring with match expressions
- [Operator Overloading](./operator-overloading) — defining operators for your types
