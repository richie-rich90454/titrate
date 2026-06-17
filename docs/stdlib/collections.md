# collections

The `tt.util` module provides generic collection data structures for storing, accessing, and manipulating groups of values. All collections implement the `Iterable` interface where applicable.

```titrate
import tt.util.ArrayList;
import tt.util.HashMap;
import tt.util.Set;
```

## ArrayList

A dynamic array backed by the VM's built-in array support. Implements `Iterable<T>`.

- `new ArrayList<T>()` — create an empty list
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

- `new Vec<T>()` — create an empty vector
- `add(element: T): void` — append
- `get(index: int): T` — access by index
- `set(index: int, value: T): void` — set by index
- `size(): int` — number of elements

## Deque

A double-ended queue supporting insertion and removal at both ends.

- `new Deque<T>()` — create an empty deque
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

- `new Queue<T>()` — create an empty queue
- `enqueue(element: T): void` — add to back
- `dequeue(): T` — remove from front
- `peek(): T` — view front without removing
- `size(): int` — number of elements
- `isEmpty(): bool` — true if empty

## PriorityQueue

A priority-based queue where elements are dequeued by priority.

- `new PriorityQueue<T>()` — create an empty priority queue
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

- `new StringBuilder()` — create an empty builder
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

- `new Trie()` — create an empty trie
- `insert(word: string): void` — add a word
- `search(word: string): bool` — exact match
- `startsWith(prefix: string): bool` — check prefix existence

## Graph

A graph data structure with vertices and edges.

- `new Graph()` — create an empty graph
- `addVertex(int id): void` — add a vertex
- `addEdge(int from, int to, double weight): void` — add a weighted edge
- `getNeighbors(int id): ArrayList<int>` — adjacent vertices

## OrderedDict

A HashMap that preserves insertion order.

- `new OrderedDict<K, V>()` — create an empty ordered map
- `put(key: K, value: V): void` — insert or update
- `get(key: K): V` — get value by key

## defaultdict

A HashMap that supplies default values for missing keys.

- `new defaultdict<K, V>(defaultValue: V)` — create with default
- `get(key: K): V` — returns value or default

## namedTuple

A tuple with named fields for readable record-like data.

- `namedTuple(fields: ArrayList<string>)` — create a named tuple type
- `get(field: string): Variant` — access field by name

## Deepened HashMap

- `HashMap.fromKeys(keys: ArrayList<string>, defaultValue: Variant): HashMap<string, Variant>` — create from keys
- `HashMap.setDefault(map: HashMap, key: string, defaultValue: Variant): Variant` — set default if absent
- `HashMap.update(map: HashMap, other: HashMap): void` — merge another map
- `HashMap.pop(map: HashMap, key: string, default: Variant): Variant` — remove and return
- `HashMap.popItem(map: HashMap): (string, Variant)` — remove and return arbitrary entry
- `HashMap.keysView(map: HashMap): ArrayList<string>` — keys view
- `HashMap.valuesView(map: HashMap): ArrayList<Variant>` — values view

## Deepened ArrayList

- `ArrayList.sortWithComparator(list: ArrayList, comparator: fn(Variant, Variant): int): void` — sort with comparator
- `ArrayList.reverse(list: ArrayList): void` — reverse in place
- `ArrayList.copy(list: ArrayList): ArrayList` — shallow copy
- `ArrayList.extend(list: ArrayList, other: ArrayList): void` — extend with another list
- `ArrayList.count(list: ArrayList, element: Variant): int` — count occurrences
- `ArrayList.removeFirst(list: ArrayList, element: Variant): bool` — remove first occurrence
- `ArrayList.pop(list: ArrayList): Variant` — remove and return last element

## Deepened Counter

- `Counter.mostCommon(c: HashMap<string, int>, n: int): ArrayList<(string, int)>` — n most common
- `Counter.elements(c: HashMap<string, int>): ArrayList<string>` — expand elements
- `Counter.subtract(c: HashMap<string, int>, other: HashMap<string, int>): void` — subtract counts
- `Counter.total(c: HashMap<string, int>): int` — total count

## Deepened Deque

- `Deque.rotate(d: ArrayList, n: int): void` — rotate by n positions
- `Deque.insert(d: ArrayList, index: int, item: Variant): void` — insert at index
- `Deque.extendLeft(d: ArrayList, items: ArrayList): void` — extend to left
- `Deque.count(d: ArrayList, item: Variant): int` — count occurrences
- `Deque.removeFirst(d: ArrayList, item: Variant): void` — remove first occurrence

## Deepened BitSet

- `BitSet.and(a: BitSet, b: BitSet): BitSet` — bitwise AND
- `BitSet.or(a: BitSet, b: BitSet): BitSet` — bitwise OR
- `BitSet.xor(a: BitSet, b: BitSet): BitSet` — bitwise XOR
- `BitSet.andNot(a: BitSet, b: BitSet): BitSet` — AND NOT
- `BitSet.flip(bs: BitSet, index: int): void` — flip bit at index
- `BitSet.cardinality(bs: BitSet): int` — number of set bits
- `BitSet.nextSetBit(bs: BitSet, fromIndex: int): int` — next set bit
- `BitSet.nextClearBit(bs: BitSet, fromIndex: int): int` — next clear bit
- `BitSet.isEmpty(bs: BitSet): bool` — check if empty
- `BitSet.intersects(a: BitSet, b: BitSet): bool` — check intersection

## Deepened Trie

- `Trie.startsWith(trie: Trie, prefix: string): bool` — check prefix
- `Trie.searchExact(trie: Trie, word: string): bool` — exact match
- `Trie.autoComplete(trie: Trie, prefix: string): ArrayList<string>` — autocomplete suggestions
- `Trie.delete(trie: Trie, word: string): bool` — delete word
- `Trie.keysWithPrefix(trie: Trie, prefix: string): ArrayList<string>` — all words with prefix
- `Trie.longestPrefixOf(trie: Trie, word: string): string` — longest prefix match

## Deepened Graph

- `Graph.addVertex(g: Graph, vertex: string): void` — add vertex
- `Graph.addEdge(g: Graph, from: string, to: string, weight: double): void` — add weighted edge
- `Graph.removeVertex(g: Graph, vertex: string): void` — remove vertex
- `Graph.removeEdge(g: Graph, from: string, to: string): void` — remove edge
- `Graph.neighbors(g: Graph, vertex: string): ArrayList<string>` — adjacent vertices
- `Graph.degree(g: Graph, vertex: string): int` — vertex degree
- `Graph.shortestPath(g: Graph, from: string, to: string): ArrayList<string>` — Dijkstra shortest path
- `Graph.minimumSpanningTree(g: Graph): ArrayList<(string, string, double)>` — MST edges

## Deepened PriorityQueue

- `PriorityQueue.peek(pq: PriorityQueue): Variant` — peek at top element
- `PriorityQueue.updatePriority(pq: PriorityQueue, element: Variant, newPriority: double): void` — update priority
- `PriorityQueue.remove(pq: PriorityQueue, element: Variant): bool` — remove element
- `PriorityQueue.drainTo(pq: PriorityQueue, n: int): ArrayList<Variant>` — drain n elements
- `PriorityQueue.tryPut(pq: PriorityQueue, element: Variant, timeoutMs: int): bool` — try put with timeout
- `PriorityQueue.tryGet(pq: PriorityQueue, timeoutMs: int): Variant` — try get with timeout
