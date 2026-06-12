# Tuples

Ever needed to return two values from a function without creating a whole class? That's exactly what tuples are for. Tuples group multiple values into a single compound value — they're lightweight, fixed-size, and can hold values of different types. Think of them as the duct tape of data structures: quick, handy, and perfect for those moments when you just need to stick a few things together.

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

::: tip Why the trailing comma?
Without it, `(42)` would just be a parenthesized integer expression — like how `(2 + 3) * 4` uses parentheses for grouping. The trailing comma tells the compiler "this is a tuple, not grouping."
:::

## Destructuring

Unpack a tuple into individual variables with `let`:

```titrate
let pair = (1, "hello");
let (x, y) = pair;
io::println(Integer.toString(x));  // 1
io::println(y);                     // hello
```

Use `_` to ignore elements you don't need:

```titrate
let triple = (10, 20, 30);
let (first, _, third) = triple;
io::println(Integer.toString(first));  // 10
io::println(Integer.toString(third));  // 30
```

Destructuring also works in `for-in` loops when iterating over collections of tuples:

```titrate
let pairs = new ArrayList<(int, string)>();
pairs.add((1, "one"));
pairs.add((2, "two"));

pairs.forEach(fn(pair: (int, string)): void {
    let (num, word) = pair;
    io::println(Integer.toString(num) + " = " + word);
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
io::println(Integer.toString(lo));  // 7
io::println(Integer.toString(hi));  // 42
```

### Swapping Values

```titrate
fn swap(a: int, b: int): (int, int) {
    return (b, a);
}

let (x, y) = swap(1, 2);
io::println(Integer.toString(x));  // 2
io::println(Integer.toString(y));  // 1
```

### Multiple Computed Results

A classic use case: computing both the quotient and remainder in one go.

```titrate
fn divMod(a: int, b: int): (int, int) {
    let quotient = a / b;
    let remainder = a % b;
    return (quotient, remainder);
}

let (q, r) = divMod(17, 5);
io::println(Integer.toString(q));  // 3
io::println(Integer.toString(r));  // 2
```

### Real-World Example: Parsing Coordinates

Tuples shine when you need to parse structured data and return multiple pieces at once:

```titrate
fn parsePoint(input: string): (double, double) {
    let parts = String.split(input, ",");
    let x = Double.parseDouble(parts[0]);
    let y = Double.parseDouble(parts[1]);
    return (x, y);
}

let (x, y) = parsePoint("3.5,7.2");
io::println("X: " + Double.toString(x) + ", Y: " + Double.toString(y));
// X: 3.5, Y: 7.2
```

## Accessing Elements by Index

Tuple elements can be accessed with dot-index notation:

```titrate
let point = (10, 20, 30);
io::println(Integer.toString(point.0));  // 10
io::println(Integer.toString(point.1));  // 20
io::println(Integer.toString(point.2));  // 30
```

::: tip
Dot-index access is handy for quick one-off reads, but destructuring is generally more readable — especially when you're accessing multiple elements. Compare `point.0` with `let (x, _, z) = point` — the latter tells you right away which elements you care about.
:::

## Tuples and Generics

Tuples work naturally with generic types:

```titrate
let entries = new ArrayList<(string, int)>();
entries.add(("Alice", 30));
entries.add(("Bob", 25));

entries.forEach(fn(entry: (string, int)): void {
    let (name, age) = entry;
    io::println(name + " is " + Integer.toString(age));
});
```

This pattern — a list of `(string, int)` tuples — is essentially a lightweight key-value table without needing a `HashMap`.

## When to Use Tuples vs Classes

Tuples and classes both group data, but they serve different purposes. Here's how to decide:

| Consideration | Use a Tuple | Use a Class |
|---|---|---|
| **Named fields** | No — elements are accessed by position (`_0`, `_1`) | Yes — fields have meaningful names (`width`, `height`) |
| **Methods** | None — just data | Can have methods and logic |
| **Lifetime** | Short-lived, local | Long-lived, passed around |
| **Identity** | Value-based — `(1, 2) == (1, 2)` | May need reference identity |
| **Size** | 2–4 elements | Any number of fields |

**Use tuples when:**
- Returning multiple values from a function
- Temporarily grouping data in a local scope
- The meaning of each position is obvious from context (like `(x, y)` coordinates)

**Use classes when:**
- The data has named fields that need self-documenting names
- You need methods that operate on the data
- The type is used across multiple functions or modules
- You need validation or invariants on construction

```titrate
// Tuple: fine for a quick coordinate pair
let point: (double, double) = (1.0, 2.0);

// Class: better when it has behavior or is used widely
public class Point {
    public double x;
    public double y;

    public fn init(x: double, y: double) {
        this.x = x;
        this.y = y;
    }

    public fn distanceTo(other: Point): double {
        let dx: double = this.x - other.x;
        let dy: double = this.y - other.y;
        return Math.sqrt(dx * dx + dy * dy);
    }
}
```

## Unit Type

The empty tuple `()` is the **unit type**. It has exactly one value, also written `()`. Functions that don't return a meaningful value implicitly return unit:

```titrate
fn sayHi(): void {
    io::println("Hi!");
}
// The void return is equivalent to returning ()
```

## Try It Yourself

Ready to practice? Here's a small exercise:

Write a function `stats` that takes three integer scores and returns a tuple containing:
1. The average (as an `int`, truncated)
2. The highest score
3. The lowest score

```titrate
fn stats(a: int, b: int, c: int): (int, int, int) {
    // Your code here!
    // Hint: use the minMax function pattern from above,
    // and compute the average with (a + b + c) / 3
}

// Test it:
let (avg, high, low) = stats(85, 92, 78);
io::println("Average: " + Integer.toString(avg));   // 85
io::println("High: " + Integer.toString(high));     // 92
io::println("Low: " + Integer.toString(low));       // 78
```

<details>
<summary>Show solution</summary>

```titrate
fn stats(a: int, b: int, c: int): (int, int, int) {
    let avg: int = (a + b + c) / 3;
    let high: int = a;
    if (b > high) { high = b; }
    if (c > high) { high = c; }
    let low: int = a;
    if (b < low) { low = b; }
    if (c < low) { low = c; }
    return (avg, high, low);
}
```

</details>

## What's Next?

- [Closures](./closures) — anonymous functions that capture their environment
- [Pattern Matching](./pattern-matching) — destructuring with match expressions
- [Operator Overloading](./operator-overloading) — defining operators for your types
