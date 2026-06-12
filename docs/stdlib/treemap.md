# treemap

The `tt.util` module provides `TreeMap<K, V>` — a sorted map backed by a balanced binary search tree with O(log n) operations.

```titrate
import tt.util.TreeMap;
```

## TreeMap

A map that keeps keys in sorted order. Supports efficient range queries and nearest-key lookups. An optional comparator can be provided for custom ordering.

- `fn init()` — create an empty tree map with natural ordering
- `fn init(comparator: fn(K, K): int)` — create an empty tree map with a custom comparator
- `put(key: K, value: V): void` — insert or update a key-value pair
- `get(key: K): V` — retrieve the value for a key
- `containsKey(key: K): bool` — check if a key exists
- `remove(key: K): void` — remove a key-value pair
- `firstKey(): K` — return the smallest key
- `lastKey(): K` — return the largest key
- `firstEntry(): Pair<K, V>` — return the entry with the smallest key
- `lastEntry(): Pair<K, V>` — return the entry with the largest key
- `ceilingKey(key: K): K` — return the smallest key greater than or equal to the given key
- `floorKey(key: K): K` — return the largest key less than or equal to the given key
- `keys(): ArrayList<K>` — return all keys in sorted order
- `values(): ArrayList<V>` — return all values in key-sorted order
- `entries(): ArrayList<Pair<K, V>>` — return all key-value pairs in sorted order
- `size(): int` — number of entries
- `isEmpty(): bool` — check if the map is empty
- `clear(): void` — remove all entries

```titrate
let scores: TreeMap<string, int> = new TreeMap<string, int>();
scores.put("Alice", 90);
scores.put("Charlie", 85);
scores.put("Bob", 95);

io::println(Integer.toString(scores.get("Bob")));          // 95
io::println(Boolean.toString(scores.containsKey("Diana"))); // false

// Keys are kept in sorted order
io::println(scores.firstKey());  // "Alice"
io::println(scores.lastKey());   // "Charlie"

// Nearest-key lookups
io::println(scores.ceilingKey("Bravo"));  // "Charlie"
io::println(scores.floorKey("Bravo"));    // "Bob"

io::println(Integer.toString(scores.size())); // 3

// Iterate in sorted order
let entries: ArrayList<Pair<string, int>> = scores.entries();
for (entry in entries) {
    io::println(entry.first + ": " + Integer.toString(entry.second));
}
```
