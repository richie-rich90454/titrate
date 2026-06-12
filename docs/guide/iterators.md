# Iterators

Iterators are the universal way to walk through a sequence of values in Titrate. Whether you're looping over a list, counting through a range of numbers, or traversing a custom data structure, iterators give you a consistent, composable pattern. The best part? If you implement the `Iterable` interface on your own types, they work seamlessly with `for-in` loops — no special syntax required.

## The Iterable/Iterator Relationship

Titrate's iteration system is built on two interfaces that work together:

- **`Iterable<T>`** — a type that *can be iterated over*. It knows how to produce an iterator. Think of it as a collection or sequence.
- **`Iterator<T>`** — a cursor that walks through the values one at a time. It tracks where you are and whether there's more to come.

The relationship is simple: `Iterable` creates `Iterator`, and `Iterator` delivers values. When you write `for (item in collection)`, the `collection` is the `Iterable`, and the loop internally creates an `Iterator` to do the walking.

```
Iterable  ──creates──>  Iterator  ──yields──>  values
  (list)                  (cursor)              (items)
```

## The Iterable Interface

`Iterable` is implemented by types that can produce an iterator:

```titrate
interface Iterable<T> {
    fn iterator(self): Iterator<T>;
}
```

Any class that implements `Iterable` can be used in a `for-in` loop directly. That's the contract: implement `iterator()`, and your type becomes loopable.

## The Iterator Interface

`Iterator` provides one value at a time and tracks whether more values remain:

```titrate
interface Iterator<T> {
    fn hasNext(self): bool;
    fn next(self): T;
}
```

- `hasNext()` returns `true` if another element is available.
- `next()` returns the next element and advances the iterator.

The pattern is always the same: call `hasNext()` to check, then `next()` to get the value. Repeat until `hasNext()` returns `false`.

## Using for-in with Custom Iterables

When a type implements `Iterable`, you can iterate over it with `for-in`:

```titrate
let list = new ArrayList<string>();
list.add("red");
list.add("green");
list.add("blue");

for (color in list) {
    io::println(color);
}
```

The `for-in` loop is just syntactic sugar. Behind the scenes, it desugars into:

```titrate
let iter = list.iterator();
while (iter.hasNext()) {
    let color = iter.next();
    io::println(color);
}
```

Understanding this desugaring is helpful — it means you can always fall back to the `while` form if you need more control over the iteration process.

### Try It Yourself

Create a simple `CountDown` class that counts from a starting number down to zero, then use it in a `for-in` loop:

```titrate
class CountDown implements Iterable<int> {
    public int start;

    public fn init(start: int) {
        this.start = start;
    }

    fn iterator(self): Iterator<int> {
        return new CountDownIterator(self.start);
    }
}

class CountDownIterator implements Iterator<int> {
    private int current;

    public fn init(start: int) {
        this.current = start;
    }

    fn hasNext(self): bool {
        return self.current >= 0;
    }

    fn next(self): int {
        let val = self.current;
        self.current = self.current - 1;
        return val;
    }
}

public fn main(): void {
    let countdown = new CountDown(5);
    for (n in countdown) {
        io::println(Integer.toString(n));  // 5, 4, 3, 2, 1, 0
    }
}
```

Try changing `CountDown` to count up instead of down. What do you need to modify?

## Implementing Iterable on Your Own Types

### Range Example

Here's a more practical example — an `IntRange` that iterates from `start` to `end`:

```titrate
class IntRange implements Iterable<int> {
    private int start;
    private int end;

    public fn init(start: int, end: int) {
        this.start = start;
        this.end = end;
    }

    fn iterator(self): Iterator<int> {
        return new IntRangeIterator(self.start, self.end);
    }
}

class IntRangeIterator implements Iterator<int> {
    private int current;
    private int end;

    public fn init(start: int, end: int) {
        this.current = start;
        this.end = end;
    }

    fn hasNext(self): bool {
        return self.current < self.end;
    }

    fn next(self): int {
        let val = self.current;
        self.current = self.current + 1;
        return val;
    }
}
```

Usage:

```titrate
let range = new IntRange(0, 5);
for (i in range) {
    io::println(Integer.toString(i));  // 0, 1, 2, 3, 4
}
```

::: tip
Notice the pattern: the `Iterable` holds the data and creates a fresh `Iterator`, while the `Iterator` holds the current position and advances through the values. This separation means you can iterate over the same `Iterable` multiple times — each call to `iterator()` starts from the beginning.
:::

### Custom Collection Example

```titrate
class RingBuffer<T> implements Iterable<T> {
    private array<T> data;
    private int head;
    private int count;

    public fn init(capacity: int) {
        this.data = new array<T>(capacity);
        this.head = 0;
        this.count = 0;
    }

    fn push(self, item: T): void {
        let idx = (self.head + self.count) % self.data.length;
        self.data[idx] = item;
        self.count = self.count + 1;
    }

    fn iterator(self): Iterator<T> {
        return new RingBufferIterator<T>(self);
    }
}

class RingBufferIterator<T> implements Iterator<T> {
    private Owned<RingBuffer<T>> buffer;
    private int index;

    public fn init(buffer: RingBuffer<T>) {
        this.buffer = buffer;
        this.index = 0;
    }

    fn hasNext(self): bool {
        return self.index < self.buffer.count;
    }

    fn next(self): T {
        let idx = (self.buffer.head + self.index) % self.buffer.data.length;
        self.index = self.index + 1;
        return self.buffer.data[idx];
    }
}
```

Usage:

```titrate
let buf = new RingBuffer<string>(3);
buf.push("first");
buf.push("second");
buf.push("third");

for (item in buf) {
    io::println(item);
}
```

## Range Iterators

Titrate provides built-in range syntax that produces iterable sequences of integers. Ranges implement `Iterable<int>` and can be used directly in `for-in` loops. You don't need to define your own `IntRange` — it's built in!

### Exclusive Range (`..`)

The `..` operator creates an exclusive range — the end value is **not** included:

```titrate
for (i in 0..5) {
    io::println(Integer.toString(i));  // 0, 1, 2, 3, 4
}
```

### Inclusive Range (`..=`)

The `..=` operator creates an inclusive range — the end value **is** included:

```titrate
for (i in 1..=5) {
    io::println(Integer.toString(i));  // 1, 2, 3, 4, 5
}
```

### Range Types

Both `..` and `..=` expressions produce a value of type `Range`. The compiler infers the `Range` type from the expression:

```titrate
let exclusive: Range = 0..10;    // 0, 1, 2, ..., 9
let inclusive: Range = 1..=10;   // 1, 2, 3, ..., 10
```

### Using Ranges in for-in

Ranges are the most concise way to iterate over a sequence of integers:

```titrate
// Count from 0 to 9
for (i in 0..10) {
    io::println(Integer.toString(i));
}

// Count from 1 to 10 (inclusive)
for (i in 1..=10) {
    io::println(Integer.toString(i));
}
```

See [Ranges](./ranges) for full details on range syntax and types.

## Built-in Iterables

These standard library types implement `Iterable` out of the box:

| Type | Iterates over |
|------|--------------|
| `ArrayList<E>` | Elements in order |
| `Vec<E>` | Elements in order |
| `HashMap<K, V>` | Key-value pairs as `(K, V)` tuples |
| `array<T>` | Elements in index order |

## Common Iterator Patterns

### Filtering While Iterating

Use a conditional inside the loop to process only certain elements:

```titrate
let numbers = new ArrayList<int>();
numbers.add(1);
numbers.add(2);
numbers.add(3);
numbers.add(4);
numbers.add(5);

for (n in numbers) {
    if (n % 2 != 0) {
        io::println(Integer.toString(n));  // 1, 3, 5
    }
}
```

### Accumulating a Result

Iterate and build up a running total:

```titrate
let numbers = new ArrayList<int>();
numbers.add(10);
numbers.add(20);
numbers.add(30);

var total: int = 0;
for (n in numbers) {
    total = total + n;
}
io::println(Integer.toString(total));  // 60
```

### Searching for a Value

Stop iteration early when you find what you're looking for:

```titrate
let names = new ArrayList<string>();
names.add("Alice");
names.add("Bob");
names.add("Carol");

var found: string = "";
for (name in names) {
    if (String.length(name) > 4) {
        found = name;
        break;  // stop early — we found what we need
    }
}
io::println(found);  // Alice
```

### Iterating with Index

Sometimes you need both the index and the value. Use a counter alongside the iterator:

```titrate
let fruits = new ArrayList<string>();
fruits.add("apple");
fruits.add("banana");
fruits.add("cherry");

var i: int = 0;
for (fruit in fruits) {
    io::println(Integer.toString(i) + ": " + fruit);
    i = i + 1;
}
// 0: apple
// 1: banana
// 2: cherry
```

## Iterators and Closures

Iterators pair naturally with closures for processing pipelines:

```titrate
let numbers = new ArrayList<int>();
numbers.add(1);
numbers.add(2);
numbers.add(3);
numbers.add(4);
numbers.add(5);

for (n in numbers) {
    if (n % 2 != 0) {
        io::println(Integer.toString(n));  // 1, 3, 5
    }
}
```

You can also pass closures to collection methods for a more functional style:

```titrate
numbers.forEach(fn(n: int): void {
    if (n % 2 != 0) {
        io::println(Integer.toString(n));
    }
});
```

## What's Next?

- [Closures](./closures) — anonymous functions for callbacks and transformations
- [Operator Overloading](./operator-overloading) — defining operators for your types
- [Standard Library](./stdlib) — built-in collections and utilities
