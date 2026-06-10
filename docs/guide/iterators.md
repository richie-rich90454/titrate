# Iterators

Iterators provide a uniform way to traverse sequences of values. Titrate uses the `Iterable` and `Iterator` interfaces to enable `for-in` loops over custom types.

## The Iterable Interface

`Iterable` is implemented by types that can produce an iterator:

```titrate
interface Iterable<T> {
    fn iterator(self): Iterator<T>;
}
```

Any class that implements `Iterable` can be used in a `for-in` loop directly.

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

The `for-in` loop desugars into:

```titrate
let iter = list.iterator();
while (iter.hasNext()) {
    let color = iter.next();
    io::println(color);
}
```

## Implementing Iterable on Your Own Types

### Range Example

```titrate
class IntRange implements Iterable<int> {
    int start;
    int end;

    public IntRange(int start, int end) {
        this.start = start;
        this.end = end;
    }

    fn iterator(self): Iterator<int> {
        return new IntRangeIterator(self.start, self.end);
    }
}

class IntRangeIterator implements Iterator<int> {
    int current;
    int end;

    public IntRangeIterator(int start, int end) {
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
    io::println(i.toString());  // 0, 1, 2, 3, 4
}
```

### Custom Collection Example

```titrate
class RingBuffer<T> implements Iterable<T> {
    array<T> data;
    int head;
    int count;

    public RingBuffer(int capacity) {
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
    Owned<RingBuffer<T>> buffer;
    int index;

    public RingBufferIterator(RingBuffer<T> buffer) {
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

Titrate provides built-in range syntax that produces iterable sequences of integers. Ranges implement `Iterable<int>` and can be used directly in `for-in` loops.

### Exclusive Range (`..`)

The `..` operator creates an exclusive range — the end value is **not** included:

```titrate
for (i in 0..5) {
    io::println(i.toString());  // 0, 1, 2, 3, 4
}
```

### Inclusive Range (`..=`)

The `..=` operator creates an inclusive range — the end value **is** included:

```titrate
for (i in 1..=5) {
    io::println(i.toString());  // 1, 2, 3, 4, 5
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
    io::println(i.toString());
}

// Count from 1 to 10 (inclusive)
for (i in 1..=10) {
    io::println(i.toString());
}
```

See [Ranges](./ranges) for full details on range syntax and types.

## Built-in Iterables

These standard library types implement `Iterable`:

| Type | Iterates over |
|------|--------------|
| `ArrayList<E>` | Elements in order |
| `Vec<E>` | Elements in order |
| `HashMap<K, V>` | Key-value pairs as `(K, V)` tuples |
| `array<T>` | Elements in index order |

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
        io::println(n.toString());  // 1, 3, 5
    }
}
```

## What's Next?

- [Closures](./closures) — anonymous functions for callbacks and transformations
- [Operator Overloading](./operator-overloading) — defining operators for your types
- [Standard Library](./stdlib) — built-in collections and utilities
