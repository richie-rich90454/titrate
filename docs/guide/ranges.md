# Ranges

Need to count from 1 to 10? Or loop through indices 0 through N-1? Ranges give you a concise, readable way to express sequences of integers. They are the backbone of `for-in` loops in Titrate — and once you get used to them, you will wonder how you ever wrote loops without them.

## Syntax

Titrate supports two range operators:

| Operator | Name | End value |
|----------|------|-----------|
| `..` | Exclusive range | Not included |
| `..=` | Inclusive range | Included |

::: tip How to remember
Think of `..` as "up to but not including" and `..=` as "up to and including." The `=` sign means the end value *equals* part of the range.
:::

## Exclusive Range (`..`)

The `..` operator creates a range that **excludes** the end value:

```titrate
for (i in 0..5) {
    io::println(Integer.toString(i));
}
// Output: 0, 1, 2, 3, 4
```

The expression `a..b` iterates from `a` up to but not including `b`. This is the most common range pattern — perfect for zero-based indexing.

## Inclusive Range (`..=`)

The `..=` operator creates a range that **includes** the end value:

```titrate
for (i in 1..=5) {
    io::println(Integer.toString(i));
}
// Output: 1, 2, 3, 4, 5
```

The expression `a..=b` iterates from `a` through `b` inclusive. This is handy when you're counting in a "human" way (1 through 10, Monday through Friday, etc.).

## Range Type

Both `..` and `..=` expressions produce a value of type `Range`:

```titrate
let exclusive = 0..10;
let inclusive = 1..=10;
```

The `Range` type implements `Iterable<int>`, so it can be used anywhere an iterable is expected.

## Using Ranges

### In for-in Loops

Ranges are most commonly used with `for-in` to iterate over a sequence of integers:

```titrate
// Print numbers 0 through 9
for (i in 0..10) {
    io::println(Integer.toString(i));
}

// Print numbers 1 through 10
for (i in 1..=10) {
    io::println(Integer.toString(i));
}
```

### Iterating Over Collection Indices

A classic pattern: using an exclusive range to visit every index of a collection:

```titrate
let fruits = new ArrayList<string>();
fruits.add("apple");
fruits.add("banana");
fruits.add("cherry");

for (i in 0..fruits.size()) {
    io::println(Integer.toString(i) + ": " + fruits.get(i));
}
// 0: apple
// 1: banana
// 2: cherry
```

### Filtering in Loops

Ranges work with conditional logic inside loops:

```titrate
for (i in 0..10) {
    if (i % 2 == 0) {
        io::println(Integer.toString(i));  // 0, 2, 4, 6, 8
    }
}
```

### Nested Loops

Ranges are useful for nested iteration patterns, like generating a multiplication table:

```titrate
for (i in 1..=5) {
    for (j in 1..=5) {
        let product: int = i * j;
        io::println(Integer.toString(i) + " x " + Integer.toString(j) + " = " + Integer.toString(product));
    }
}
```

### Building Strings with Ranges

You can use ranges to build formatted output, like creating a numbered list:

```titrate
let lines = new ArrayList<string>();
for (i in 1..=5) {
    lines.add("Line " + Integer.toString(i));
}
for (line in lines) {
    io::println(line);
}
// Line 1
// Line 2
// Line 3
// Line 4
// Line 5
```

## Range Expressions

The start and end of a range can be any integer expression, not just literals:

```titrate
let start: int = 5;
let end: int = 10;

for (i in start..end) {
    io::println(Integer.toString(i));  // 5, 6, 7, 8, 9
}

for (i in start..=end) {
    io::println(Integer.toString(i));  // 5, 6, 7, 8, 9, 10
}
```

This is especially useful when the bounds come from user input or computed values:

```titrate
fn printRange(from: int, to: int): void {
    for (i in from..=to) {
        io::println(Integer.toString(i));
    }
}

printRange(3, 7);  // prints 3, 4, 5, 6, 7
```

## Try It Yourself

Write a program that prints a simple ASCII triangle using ranges. For a height of 5, the output should look like:

```
*
**
***
****
*****
```

```titrate
public fn main(): void {
    let height: int = 5;
    // Your code here!
    // Hint: use a range for the rows (1..=height),
    // and build each row by repeating "*" the right number of times.
}
```

<details>
<summary>Show solution</summary>

```titrate
public fn main(): void {
    let height: int = 5;
    for (row in 1..=height) {
        var line: string = "";
        for (col in 0..row) {
            line = line + "*";
        }
        io::println(line);
    }
}
```

</details>

## What's Next?

- [Iterators](./iterators) — custom iteration with the Iterable interface
- [Closures](./closures) — anonymous functions for processing ranges
- [Control Flow](./control-flow) — loops and conditional execution
