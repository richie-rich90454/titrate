# Ranges

Ranges provide a concise syntax for generating sequences of integers. They are commonly used in `for-in` loops and implement the `Iterable<int>` interface.

## Syntax

Titrate supports two range operators:

| Operator | Name | End value |
|----------|------|-----------|
| `..` | Exclusive range | Not included |
| `..=` | Inclusive range | Included |

## Exclusive Range (`..`)

The `..` operator creates a range that **excludes** the end value:

```titrate
for (i in 0..5) {
    io::println(i.toString());
}
// Output: 0, 1, 2, 3, 4
```

The expression `a..b` iterates from `a` up to but not including `b`.

## Inclusive Range (`..=`)

The `..=` operator creates a range that **includes** the end value:

```titrate
for (i in 1..=5) {
    io::println(i.toString());
}
// Output: 1, 2, 3, 4, 5
```

The expression `a..=b` iterates from `a` through `b` inclusive.

## Range Type

Both `..` and `..=` expressions produce a value of type `Range`:

```titrate
let exclusive: Range = 0..10;
let inclusive: Range = 1..=10;
```

The `Range` type implements `Iterable<int>`, so it can be used anywhere an iterable is expected.

## Using Ranges

### In for-in Loops

Ranges are most commonly used with `for-in` to iterate over a sequence of integers:

```titrate
// Print numbers 0 through 9
for (i in 0..10) {
    io::println(i.toString());
}

// Print numbers 1 through 10
for (i in 1..=10) {
    io::println(i.toString());
}
```

### With Closures

Ranges work with higher-order functions that accept iterables:

```titrate
for (i in 0..5) {
    if (i % 2 == 0) {
        io::println(i.toString());  // 0, 2, 4
    }
}
```

### Nested Loops

Ranges are useful for nested iteration patterns:

```titrate
for (i in 0..3) {
    for (j in 0..3) {
        io::println(i.toString() + "," + j.toString());
    }
}
```

## Range Expressions

The start and end of a range can be any integer expression:

```titrate
let start: int = 5;
let end: int = 10;

for (i in start..end) {
    io::println(i.toString());  // 5, 6, 7, 8, 9
}

for (i in start..=end) {
    io::println(i.toString());  // 5, 6, 7, 8, 9, 10
}
```

## What's Next?

- [Iterators](./iterators) — custom iteration with the Iterable interface
- [Closures](./closures) — anonymous functions for processing ranges
- [Control Flow](./control-flow) — loops and conditional execution
