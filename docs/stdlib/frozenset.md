# frozenset

The `tt.util.FrozenSet` module provides `FrozenSet<E>` — an immutable hash-based set backed by `HashMap`. Once constructed, a `FrozenSet` cannot be modified, making it safe to use as a map key or in other contexts requiring a stable identity. It supports the standard set-algebra operations.

```titrate
import tt.util.FrozenSet;
```

## FrozenSet

An immutable set of unique elements.

**Methods:**

- `fn init(items: ArrayList<E>)` — create a frozen set containing the given items (duplicates are discarded)
- `fn contains(item: E): bool` — return true if the element is in the set
- `fn size(): int` — return the number of elements
- `fn isEmpty(): bool` — return true if the set contains no elements
- `fn isSubset(other: FrozenSet<E>): bool` — return true if every element of this set is also in `other`
- `fn isSuperset(other: FrozenSet<E>): bool` — return true if every element of `other` is also in this set
- `fn union(other: FrozenSet<E>): FrozenSet<E>` — return a new set with elements in either set
- `fn intersection(other: FrozenSet<E>): FrozenSet<E>` — return a new set with elements in both sets
- `fn difference(other: FrozenSet<E>): FrozenSet<E>` — return a new set with elements in this set but not in `other`
- `fn symmetricDifference(other: FrozenSet<E>): FrozenSet<E>` — return a new set with elements in exactly one of the two sets
- `fn toArrayList(): ArrayList<E>` — return an `ArrayList` of all elements
- `fn equals(other: Variant): bool` — structural equality check against another `FrozenSet`
- `fn toString(): string` — return a string such as `"FrozenSet{a, b, c}"`

```titrate
import tt.util.FrozenSet;
import tt.util.ArrayList;

let items = new ArrayList<string>();
items.add("a"); items.add("b"); items.add("c");
let a = new FrozenSet<string>(items);

let more = new ArrayList<string>();
more.add("b"); more.add("c"); more.add("d");
let b = new FrozenSet<string>(more);

io::println(Boolean.toString(a.isSubset(b)));        // false
io::println(a.intersection(b).toString());           // FrozenSet{b, c}
io::println(a.union(b).toString());                   // FrozenSet{a, b, c, d}
```
