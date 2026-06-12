# treeset

The `tt.util` module provides `TreeSet<T>` — a sorted set backed by a balanced binary search tree with O(log n) operations.

```titrate
import tt.util.TreeSet;
```

## TreeSet

A set that keeps elements in sorted order. Supports efficient nearest-element lookups and set operations. An optional comparator can be provided for custom ordering.

- `fn init()` — create an empty tree set with natural ordering
- `fn init(comparator: fn(T, T): int)` — create an empty tree set with a custom comparator
- `add(item: T): void` — add an element to the set
- `remove(item: T): void` — remove an element from the set
- `contains(item: T): bool` — check if an element exists
- `first(): T` — return the smallest element
- `last(): T` — return the largest element
- `ceiling(item: T): T` — return the smallest element greater than or equal to the given item
- `floor(item: T): T` — return the largest element less than or equal to the given item
- `toArray(): ArrayList<T>` — return all elements in sorted order
- `union(other: TreeSet<T>): TreeSet<T>` — return a new set containing elements from both sets
- `intersection(other: TreeSet<T>): TreeSet<T>` — return a new set containing elements common to both sets
- `difference(other: TreeSet<T>): TreeSet<T>` — return a new set containing elements in this set but not the other
- `size(): int` — number of elements
- `isEmpty(): bool` — check if the set is empty
- `clear(): void` — remove all elements
- `toString(): string` — return a string representation of the set

```titrate
let set: TreeSet<int> = new TreeSet<int>();
set.add(5);
set.add(1);
set.add(9);
set.add(3);

io::println(Boolean.toString(set.contains(5)));  // true
io::println(Boolean.toString(set.contains(7)));  // false

// Elements are kept in sorted order
io::println(Integer.toString(set.first()));  // 1
io::println(Integer.toString(set.last()));   // 9

// Nearest-element lookups
io::println(Integer.toString(set.ceiling(4)));  // 5
io::println(Integer.toString(set.floor(4)));    // 3

// Set operations
let other: TreeSet<int> = new TreeSet<int>();
other.add(3);
other.add(7);

let uni: TreeSet<int> = set.union(other);
let inter: TreeSet<int> = set.intersection(other);
let diff: TreeSet<int> = set.difference(other);

io::println(Integer.toString(uni.size()));   // 5
io::println(Integer.toString(inter.size())); // 1
io::println(Integer.toString(diff.size()));  // 3
```
