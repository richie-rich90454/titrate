# collections

The `tt.util` module provides generic collection data structures for storing, accessing, and manipulating groups of values. All collections implement the `Iterable` interface where applicable.

```titrate
import tt.util.ArrayList;
import tt.util.HashMap;
import tt.util.Set;
```

## ArrayList

A dynamic array backed by the VM's built-in array support. Implements `Iterable<T>`.

- `ArrayList<T>()` — create an empty list
- `add(element: T): void` — append an element
- `add(index: int, value: T): void` — insert at index
- `get(index: int): T` — get element at index
- `set(index: int, value: T): void` — set element at index
- `remove(index: int): T` — remove and return element at index
- `size(): int` — number of elements
- `contains(value: T): bool` — check membership
- `indexOf(value: T): int` — first index of value, or -1
- `lastIndexOf(element: T): int` — last index of element, or -1
- `subList(fromIndex: int, toIndex: int): ArrayList<T>` — view of range [from, to)
- `isEmpty(): bool` — true if empty
- `clear(): void` — remove all elements
- `clone(): ArrayList<T>` — shallow copy
- `addAll(other: ArrayList<T>): void` — append all from another list
- `sort(): void` — sort in-place
- `forEach(fn: fn(T): void): void` — iterate with side effect
- `map(fn: fn(T): T): ArrayList<T>` — transform each element
- `filter(fn: fn(T): bool): ArrayList<T>` — keep matching elements
- `iterator(): ArrayListIterator<T>` — return an iterator

```titrate
let list = new ArrayList<string>();
list.add("hello");
list.add("world");
io::println(list.get(0));  // "hello"
let upper = list.map(fn(s: string): string => String.toUpperCase(s));
// ["HELLO", "WORLD"]
```

## HashMap

A key-value map backed by the VM's built-in hash map. Implements `Iterable<K>`.

- `HashMap<K, V>()` — create an empty map
- `put(key: K, value: V): void` — insert or update
- `get(key: K): V` — get value, or null
- `containsKey(key: K): bool` — check key existence
- `remove(key: K): void` — remove by key
- `keys(): ArrayList<K>` — all keys
- `values(): ArrayList<V>` — all values
- `entries(): ArrayList<(K, V)>` — all key-value pairs
- `size(): int` — number of entries
- `isEmpty(): bool` — true if empty
- `clear(): void` — remove all entries
- `getOrDefault(key: K, defaultValue: V): V` — get or fallback
- `putIfAbsent(key: K, value: V): V` — insert if key missing
- `computeIfAbsent(key: K, mapper: fn(K): V): V` — compute and cache
- `merge(key: K, value: V, remapper: fn(V, V): V): V` — merge values
- `forEach(fn: fn(K, V): void): void` — iterate entries
- `map(fn: fn(K, V): (K, V)): HashMap<K, V>` — transform entries
- `filter(fn: fn(K, V): bool): HashMap<K, V>` — keep matching entries

```titrate
let scores = new HashMap<string, int>();
scores.put("alice", 95);
scores.put("bob", 87);
io::println(scores.get("alice"));  // 95
io::println(scores.getOrDefault("carol", 0));  // 0
```

## HashSet

A set collection backed by `ArrayList`. Implements `Iterable<E>`.

- `Set<E>()` — create an empty set
- `add(item: E): void` — add if not present
- `remove(item: E): bool` — remove, returns whether it was present
- `contains(item: E): bool` — check membership
- `size(): int` — number of elements
- `isEmpty(): bool` — true if empty
- `clear(): void` — remove all
- `union(other: Set<E>): Set<E>` — elements in either set
- `intersection(other: Set<E>): Set<E>` — elements in both sets
- `difference(other: Set<E>): Set<E>` — elements in this but not other
- `symmetricDifference(other: Set<E>): Set<E>` — elements in exactly one set
- `isSubsetOf(other: Set<E>): bool` — all elements in other
- `isSupersetOf(other: Set<E>): bool` — contains all of other
- `toArray(): ArrayList<E>` — convert to list
- `map(fn: fn(E): E): Set<E>` — transform elements
- `filter(fn: fn(E): bool): Set<E>` — keep matching elements

```titrate
let a = new Set<int>();
a.add(1); a.add(2); a.add(3);
let b = new Set<int>();
b.add(2); b.add(3); b.add(4);
let common = a.intersection(b);  // {2, 3}
```

## LinkedList

A doubly-linked list implementation for efficient insertion/removal at both ends.

- `LinkedList<T>()` — create an empty linked list
- `add(element: T): void` — append to end
- `addFirst(element: T): void` — prepend
- `addLast(element: T): void` — append
- `getFirst(): T` — first element
- `getLast(): T` — last element
- `removeFirst(): T` — remove and return first
- `removeLast(): T` — remove and return last
- `size(): int` — number of elements

## Vec

A compact vector type for numeric data storage.

- `Vec<T>()` — create an empty vector
- `add(element: T): void` — append
- `get(index: int): T` — access by index
- `set(index: int, value: T): void` — set by index
- `size(): int` — number of elements

## Deque

A double-ended queue supporting insertion and removal at both ends.

- `Deque<T>()` — create an empty deque
- `addFirst(element: T): void` — add to front
- `addLast(element: T): void` — add to back
- `removeFirst(): T` — remove from front
- `removeLast(): T` — remove from back
- `peekFirst(): T` — view front without removing
- `peekLast(): T` — view back without removing
- `size(): int` — number of elements

## Stack

A last-in, first-out (LIFO) stack.

- `Stack<T>()` — create an empty stack
- `push(element: T): void` — push onto stack
- `pop(): T` — pop top element
- `peek(): T` — view top without removing
- `size(): int` — number of elements
- `isEmpty(): bool` — true if empty

## Queue

A first-in, first-out (FIFO) queue.

- `Queue<T>()` — create an empty queue
- `enqueue(element: T): void` — add to back
- `dequeue(): T` — remove from front
- `peek(): T` — view front without removing
- `size(): int` — number of elements
- `isEmpty(): bool` — true if empty

## PriorityQueue

A priority-based queue where elements are dequeued by priority.

- `PriorityQueue<T>()` — create an empty priority queue
- `enqueue(element: T, priority: double): void` — add with priority
- `dequeue(): T` — remove highest-priority element
- `peek(): T` — view highest-priority element
- `size(): int` — number of elements

## Counter

A counting utility that tracks element frequencies.

- `Counter<T>()` — create an empty counter
- `add(item: T): void` — increment count for item
- `count(item: T): int` — get count for item
- `mostCommon(int n): ArrayList<(T, int)>` — top n items by count

## BitSet

A compact set of non-negative integers using bit manipulation.

- `BitSet()` — create an empty bit set
- `set(int bit): void` — set a bit
- `clear(int bit): void` — clear a bit
- `get(int bit): bool` — check if bit is set
- `flip(int bit): void` — toggle a bit

## StringBuilder

Efficient string builder for concatenating many strings.

- `StringBuilder()` — create an empty builder
- `append(s: string): void` — append a string
- `build(): string` — return the concatenated result

```titrate
let sb = new StringBuilder();
sb.append("Hello");
sb.append(" ");
sb.append("World");
io::println(sb.build());  // "Hello World"
```

## Trie

A prefix tree for efficient string operations.

- `Trie()` — create an empty trie
- `insert(word: string): void` — add a word
- `search(word: string): bool` — exact match
- `startsWith(prefix: string): bool` — check prefix existence

## Graph

A graph data structure with vertices and edges.

- `Graph()` — create an empty graph
- `addVertex(int id): void` — add a vertex
- `addEdge(int from, int to, double weight): void` — add a weighted edge
- `getNeighbors(int id): ArrayList<int>` — adjacent vertices

## OrderedDict

A HashMap that preserves insertion order.

- `OrderedDict<K, V>()` — create an empty ordered map
- `put(key: K, value: V): void` — insert or update
- `get(key: K): V` — get value by key

## defaultdict

A HashMap that supplies default values for missing keys.

- `defaultdict<K, V>(defaultValue: V)` — create with default
- `get(key: K): V` — returns value or default

## namedTuple

A tuple with named fields for readable record-like data.

- `namedTuple(fields: ArrayList<string>)` — create a named tuple type
- `get(field: string): Variant` — access field by name
