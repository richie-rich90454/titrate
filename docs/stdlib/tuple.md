# tuple

The `tt.lang.Tuple` module provides typed tuple classes for grouping fixed numbers of values. `Tuple2`, `Tuple3`, and `Tuple4` hold two, three, and four typed values respectively, along with factory and concatenation functions.

```titrate
import tt.lang.Tuple;
```

## Tuple2

A heterogeneous pair of values.

**Fields:**

- `_0: A` — the first element
- `_1: B` — the second element

**Methods:**

- `fn init(a: A, b: B)` — create a pair
- `fn get0(): A` — return the first element
- `fn get1(): B` — return the second element
- `fn toString(): string` — return a string such as `"(a, b)"`
- `fn equals(other: Tuple2<A, B>): bool` — structural equality check

## Tuple3

A heterogeneous triple of values.

**Fields:**

- `_0: A` — the first element
- `_1: B` — the second element
- `_2: C` — the third element

**Methods:**

- `fn init(a: A, b: B, c: C)` — create a triple
- `fn get0(): A` — return the first element
- `fn get1(): B` — return the second element
- `fn get2(): C` — return the third element
- `fn toString(): string` — return a string such as `"(a, b, c)"`
- `fn equals(other: Tuple3<A, B, C>): bool` — structural equality check

## Tuple4

A heterogeneous quadruple of values.

**Fields:**

- `_0: A` — the first element
- `_1: B` — the second element
- `_2: C` — the third element
- `_3: D` — the fourth element

**Methods:**

- `fn init(a: A, b: B, c: C, d: D)` — create a quadruple
- `fn get0(): A` — return the first element
- `fn get1(): B` — return the second element
- `fn get2(): C` — return the third element
- `fn get3(): D` — return the fourth element
- `fn toString(): string` — return a string such as `"(a, b, c, d)"`
- `fn equals(other: Tuple4<A, B, C, D>): bool` — structural equality check

## Top-level Functions

- `fn makeTuple2<A, B>(a: A, b: B): Tuple2<A, B>` — create a 2-tuple without explicit type parameters
- `fn makeTuple3<A, B, C>(a: A, b: B, c: C): Tuple3<A, B, C>` — create a 3-tuple without explicit type parameters
- `fn makeTuple4<A, B, C, D>(a: A, b: B, c: C, d: D): Tuple4<A, B, C, D>` — create a 4-tuple without explicit type parameters
- `fn tupleCat2<A, B, C, D>(t1: Tuple2<A, B>, t2: Tuple2<C, D>): Tuple4<A, B, C, D>` — concatenate two 2-tuples into a 4-tuple
- `fn tupleCat23<A, B, C, D>(t1: Tuple2<A, B>, t2: Tuple3<C, D, Variant>): Tuple4<A, B, C, D>` — concatenate a 2-tuple and a 3-tuple into a 4-tuple (truncating to 4 elements)

```titrate
import tt.lang.Tuple;

let p = Tuple.makeTuple2("age", 30);
io::println(p.toString());              // (age, 30)
io::println(Integer.toString(p.get1())); // 30

let q = Tuple.makeTuple2("score", 95);
let combined = Tuple.tupleCat2(p, q);
io::println(combined.toString());       // (age, 30, score, 95)
```
