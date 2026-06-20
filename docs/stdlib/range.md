# range

The `tt.util.Range` module provides lazy integer sequences. A `Range` represents an immutable sequence of integers from `start` to `stop` (exclusive) stepping by `step`. It is memory-efficient because it computes values on demand rather than materializing the full sequence.

```titrate
import tt.util.Range;
```

## Range

An immutable sequence of integers defined by `start`, `stop`, and `step`.

**Fields:**

- `start: int` — the first value
- `stop: int` — the exclusive upper (or lower, for negative step) bound
- `step: int` — the difference between successive values

**Methods:**

- `fn init(start: int, stop: int, step: int)` — create a range
- `fn iterator(): RangeIterator` — return an iterator over the range
- `fn contains(value: int): bool` — return true if `value` is an element of the range
- `fn size(): int` — return the number of elements in the range
- `fn isEmpty(): bool` — return true if the range contains no elements
- `fn get(index: int): int` — return the element at the given zero-based index
- `fn toArrayList(): ArrayList<int>` — materialize the range into an `ArrayList`
- `fn toString(): string` — return a string such as `"Range(0, 10)"` or `"Range(0, 10, 2)"`

## RangeIterator

Walks through a `Range` one element at a time.

**Methods:**

- `fn init(start: int, stop: int, step: int)` — create an iterator
- `fn hasNext(): bool` — return true if more elements remain
- `fn next(): int` — return the current element and advance

## Top-level Functions

- `fn range(stop: int): Range` — create a range from `0` to `stop` (exclusive) with step 1
- `fn rangeWithStart(start: int, stop: int): Range` — create a range from `start` to `stop` (exclusive) with step 1
- `fn rangeWithStep(start: int, stop: int, step: int): Range` — create a range with an explicit step

```titrate
import tt.util.Range;

let r = Range.rangeWithStep(0, 10, 2);
io::println(Integer.toString(r.size()));    // 5
io::println(Boolean.toString(r.contains(4))); // true

let iter = r.iterator();
while (iter.hasNext()) {
    io::println(Integer.toString(iter.next())); // 0, 2, 4, 6, 8
}
```
