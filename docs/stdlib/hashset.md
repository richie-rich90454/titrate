# hashset

The `tt.util.HashSet` module provides a hash-based set collection backed by `HashMap`. Unlike the `Set<E>` type (which is backed by `ArrayList`), `HashSet<E>` offers O(1) average-case lookups for membership tests.

```titrate
import tt.util.HashSet;
```

## HashSet

A hash-based set that stores unique elements. Implements set algebra operations.

- `HashSet<E>()` — create an empty hash set
- `add(element: E): bool` — add element; returns true if it was not already present
- `remove(element: E): bool` — remove element; returns true if it was present
- `contains(element: E): bool` — check membership
- `size(): int` — number of elements
- `isEmpty(): bool` — true if empty
- `clear(): void` — remove all elements
- `toArrayList(): ArrayList<E>` — convert to a list
- `union(other: HashSet<E>): HashSet<E>` — elements in either set
- `intersection(other: HashSet<E>): HashSet<E>` — elements in both sets
- `difference(other: HashSet<E>): HashSet<E>` — elements in this but not other
- `symmetricDifference(other: HashSet<E>): HashSet<E>` — elements in exactly one set
- `isSubsetOf(other: HashSet<E>): bool` — all elements in other
- `isSupersetOf(other: HashSet<E>): bool` — contains all of other
- `forEach(action: function<void(E)>): void` — iterate with side effect
- `clone(): HashSet<E>` — shallow copy
- `equals(other: Object): bool` — structural equality
- `toString(): String` — `"HashSet{a, b, c}"`

```titrate
let a = new HashSet<int>();
a.add(1); a.add(2); a.add(3);
let b = new HashSet<int>();
b.add(2); b.add(3); b.add(4);

let common = a.intersection(b);    // HashSet{2, 3}
let all = a.union(b);              // HashSet{1, 2, 3, 4}
let diff = a.difference(b);        // HashSet{1}
io::println(a.isSubsetOf(all));    // true
```
