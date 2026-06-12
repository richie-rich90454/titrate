# pair

The `tt.util` module provides `Pair<T, U>` — a heterogeneous two-element tuple for grouping related values.

```titrate
import tt.util.Pair;
```

## Pair

A simple container holding two values of potentially different types. Useful for returning two values from a function or storing key-value associations before they are placed in a map.

**Fields:**

- `first: T` — the first element
- `second: U` — the second element

**Methods:**

- `fn init(first: T, second: U)` — create a pair with the given values
- `toString(): string` — string representation as `"(first, second)"`
- `equals(other: Pair<T, U>): bool` — structural equality check
- `swap(): Pair<U, T>` — return a new pair with elements swapped

**Factory function:**

- `makePair<T, U>(first: T, second: U): Pair<T, U>` — create a pair without explicit type parameters

```titrate
let p: Pair<string, int> = new Pair("age", 30);
io::println(p.first);                   // "age"
io::println(Integer.toString(p.second)); // 30
io::println(p.toString());              // "(age, 30)"

let swapped: Pair<int, string> = p.swap();
io::println(swapped.toString());        // "(30, age)"

let q: Pair<string, int> = makePair("score", 95);
io::println(Boolean.toString(p.equals(q))); // false
```
