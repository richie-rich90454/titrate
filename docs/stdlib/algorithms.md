# algorithms

The `tt.algorithms` module provides common algorithms for sorting, searching, and transforming collections.

```titrate
import tt.algorithms;
```

## Sorting

### sort

- `algorithms.sort<T: Comparable<T>>(items: ArrayList<T>): void` — sort a list in-place in ascending order

```titrate
let list = new ArrayList<int>();
list.add(3); list.add(1); list.add(4); list.add(1); list.add(5);
algorithms.sort(list);
// list is now [1, 1, 3, 4, 5]
```

### sorted

- `algorithms.sorted<T: Comparable>(items: ArrayList<T>): ArrayList<T>` — return a new sorted list, leaving the original unchanged

```titrate
let original = new ArrayList<int>();
original.add(3); original.add(1); original.add(4);
let result = algorithms.sorted(original);
// original is still [3, 1, 4], result is [1, 3, 4]
```

### sortBy

- `algorithms.sortBy<T>(items: ArrayList<T>, key: fn(T): double): void` — sort in-place using a key function

```titrate
let names = new ArrayList<string>();
names.add("charlie"); names.add("alice"); names.add("bob");
algorithms.sortBy(names, fn(s: string): double => String.length(s));
// sorted by length: "bob", "alice", "charlie"
```

### reverse

- `algorithms.reverse<T>(items: ArrayList<T>): void` — reverse a list in-place

```titrate
let list = new ArrayList<int>();
list.add(1); list.add(2); list.add(3);
algorithms.reverse(list);
// list is now [3, 2, 1]
```

## Searching

### binarySearch

- `algorithms.binarySearch<T: Comparable<T>>(items: ArrayList<T>, target: T): int` — find `target` in a sorted list; returns the index or -1 if not found

```titrate
let list = new ArrayList<int>();
list.add(1); list.add(3); list.add(5); list.add(7); list.add(9);
let idx = algorithms.binarySearch(list, 5);  // 2
let missing = algorithms.binarySearch(list, 6);  // -1
```

### linearSearch

- `algorithms.linearSearch<T>(items: ArrayList<T>, target: T): int` — find `target` by scanning; returns the index or -1

### contains

- `algorithms.contains<T>(items: ArrayList<T>, target: T): bool` — check if a list contains the target

### find

- `algorithms.find<T>(items: ArrayList<T>, pred: fn(T): bool): T` — return the first element matching the predicate

### findIndex

- `algorithms.findIndex<T>(items: ArrayList<T>, pred: fn(T): bool): int` — return the index of the first element matching the predicate, or -1

## Set Operations

### unique

- `algorithms.unique<T>(items: ArrayList<T>): ArrayList<T>` — remove duplicates, preserving order

```titrate
let list = new ArrayList<int>();
list.add(1); list.add(3); list.add(1); list.add(2); list.add(3);
let deduped = algorithms.unique(list);
// [1, 3, 2]
```

### union

- `algorithms.union<T>(a: ArrayList<T>, b: ArrayList<T>): ArrayList<T>` — elements in either list, without duplicates

### intersection

- `algorithms.intersection<T>(a: ArrayList<T>, b: ArrayList<T>): ArrayList<T>` — elements in both lists

### difference

- `algorithms.difference<T>(a: ArrayList<T>, b: ArrayList<T>): ArrayList<T>` — elements in `a` but not in `b`

## Transformation

### flatten

- `algorithms.flatten<T>(items: ArrayList<ArrayList<T>>): ArrayList<T>` — concatenate nested lists into one

```titrate
let nested = new ArrayList<ArrayList<int>>();
// [[1, 2], [3, 4], [5]]
let flat = algorithms.flatten(nested);
// [1, 2, 3, 4, 5]
```

### zip

- `algorithms.zip<A, B>(a: ArrayList<A>, b: ArrayList<B>): ArrayList<(A, B)>` — pair elements from two lists

```titrate
let nums = new ArrayList<int>();
nums.add(1); nums.add(2); nums.add(3);
let letters = new ArrayList<string>();
letters.add("a"); letters.add("b"); letters.add("c");
let pairs = algorithms.zip(nums, letters);
// [(1, "a"), (2, "b"), (3, "c")]
```

### chunk

- `algorithms.chunk<T>(items: ArrayList<T>, size: int): ArrayList<ArrayList<T>>` — split a list into chunks of the given size

```titrate
let list = new ArrayList<int>();
list.add(1); list.add(2); list.add(3); list.add(4); list.add(5);
let chunks = algorithms.chunk(list, 2);
// [[1, 2], [3, 4], [5]]
```

### partition

- `algorithms.partition<T>(items: ArrayList<T>, pred: fn(T): bool): (ArrayList<T>, ArrayList<T>)` — split into elements matching and not matching the predicate

```titrate
let list = new ArrayList<int>();
list.add(1); list.add(2); list.add(3); list.add(4); list.add(5);
let (evens, odds) = algorithms.partition(list, fn(n: int): bool => n % 2 == 0);
// evens: [2, 4], odds: [1, 3, 5]
```

## Aggregation

### sum

- `algorithms.sum(items: ArrayList<double>): double` — sum all elements

### product

- `algorithms.product(items: ArrayList<double>): double` — multiply all elements

### count

- `algorithms.count<T>(items: ArrayList<T>, pred: fn(T): bool): int` — count elements matching the predicate

```titrate
let list = new ArrayList<int>();
list.add(1); list.add(2); list.add(3); list.add(4); list.add(5);
let evenCount = algorithms.count(list, fn(n: int): bool => n % 2 == 0);  // 2
```

## Shuffle

### shuffle

- `algorithms.shuffle<T>(items: ArrayList<T>): void` — randomly permute a list in-place

```titrate
let list = new ArrayList<int>();
list.add(1); list.add(2); list.add(3); list.add(4);
algorithms.shuffle(list);
// list is now in random order
```

### shuffled

- `algorithms.shuffled<T>(items: ArrayList<T>): ArrayList<T>` — return a new randomly permuted list

## Graph Algorithms Part 1

- `GraphAlgo.bfs(graph: Graph, start: string): ArrayList<string>` — breadth-first search
- `GraphAlgo.dfs(graph: Graph, start: string): ArrayList<string>` — depth-first search
- `GraphAlgo.dijkstra(graph: Graph, start: string): HashMap<string, double>` — shortest paths
- `GraphAlgo.bellmanFord(graph: Graph, start: string): HashMap<string, double>` — shortest paths (negative weights)
- `GraphAlgo.floydWarshall(graph: Graph): HashMap<string, HashMap<string, double>>` — all-pairs shortest paths
- `GraphAlgo.aStar(graph: Graph, start: string, goal: string, heuristic: fn(string): double): ArrayList<string>` — A* search

## Graph Algorithms Part 2

- `GraphAlgo.kruskalMST(graph: Graph): ArrayList<(string, string, double)>` — Kruskal's MST
- `GraphAlgo.primMST(graph: Graph): ArrayList<(string, string, double)>` — Prim's MST
- `GraphAlgo.topologicalSort(graph: Graph): ArrayList<string>` — topological ordering
- `GraphAlgo.stronglyConnectedComponents(graph: Graph): ArrayList<ArrayList<string>>` — SCCs
- `GraphAlgo.hasCycle(graph: Graph): bool` — cycle detection

## Graph Algorithms Part 3

- `GraphAlgo.maxFlow(graph: Graph, source: string, sink: string): double` — Ford-Fulkerson max flow
- `GraphAlgo.bipartiteMatching(graph: Graph): ArrayList<(string, string)>` — bipartite matching
- `GraphAlgo.eulerTour(graph: Graph, start: string): ArrayList<string>` — Euler tour
- `GraphAlgo.isHamiltonian(graph: Graph): bool` — Hamiltonian path check
- `GraphAlgo.graphColoring(graph: Graph, maxColors: int): HashMap<string, int>` — graph coloring

## String Algorithms

- `StringAlgo.kmp(text: string, pattern: string): ArrayList<int>` — KMP pattern matching
- `StringAlgo.rabinKarp(text: string, pattern: string): ArrayList<int>` — Rabin-Karp
- `StringAlgo.boyerMoore(text: string, pattern: string): ArrayList<int>` — Boyer-Moore
- `StringAlgo.suffixArray(text: string): ArrayList<int>` — suffix array
- `StringAlgo.lcpArray(text: string): ArrayList<int>` — LCP array
- `StringAlgo.zAlgorithm(text: string): ArrayList<int>` — Z-algorithm
- `StringAlgo.ahoCorasick(text: string, patterns: ArrayList<string>): HashMap<string, ArrayList<int>>` — Aho-Corasick

## Heap Algorithms

- `HeapAlgo.isHeap(arr: ArrayList): bool` — check if valid heap
- `HeapAlgo.isHeapUntil(arr: ArrayList): int` — first heap violation
- `HeapAlgo.makeHeap(arr: ArrayList): void` — heapify
- `HeapAlgo.pushHeap(arr: ArrayList, value: Variant): void` — push to heap
- `HeapAlgo.popHeap(arr: ArrayList): Variant` — pop from heap
- `HeapAlgo.sortHeap(arr: ArrayList): void` — heap sort

## Set Algorithms

- `SetAlgo.setUnion(a: ArrayList, b: ArrayList): ArrayList` — set union
- `SetAlgo.setIntersection(a: ArrayList, b: ArrayList): ArrayList` — set intersection
- `SetAlgo.setDifference(a: ArrayList, b: ArrayList): ArrayList` — set difference
- `SetAlgo.setSymmetricDifference(a: ArrayList, b: ArrayList): ArrayList` — symmetric difference
- `SetAlgo.includes(a: ArrayList, b: ArrayList): bool` — subset check
- `SetAlgo.nthElement(arr: ArrayList, n: int): Variant` — nth element (selection)

## C++ `<algorithm>` Parity (Phase 1-2)

The following algorithms were added for full C++ standard library parity, including parallel execution policy overloads.

### nth_element

- `algorithms.nthElement<T: Comparable<T>>(items: ArrayList<T>, n: int): void` — rearrange so the element at position `n` is the one that would be there if the list were sorted (selection algorithm)

```titrate
let list = new ArrayList<int>();
list.add(9); list.add(3); list.add(7); list.add(1); list.add(5);
algorithms.nthElement(list, 2);
// list.get(2) is now the 3rd-smallest element (5)
```

### partition_point

- `algorithms.partitionPoint<T>(items: ArrayList<T>, pred: fn(T): bool): int` — return the index of the first element for which the predicate returns false (assumes the range is already partitioned)

```titrate
let list = new ArrayList<int>();
list.add(2); list.add(4); list.add(6); list.add(1); list.add(3);
let idx = algorithms.partitionPoint(list, fn(n: int): bool => n % 2 == 0);  // 3
```

### is_sorted / is_sorted_until

- `algorithms.isSorted<T: Comparable<T>>(items: ArrayList<T>): bool` — check whether the range is sorted in ascending order
- `algorithms.isSortedUntil<T: Comparable<T>>(items: ArrayList<T>): int` — return the index of the first element that breaks the sort order

```titrate
let list = new ArrayList<int>();
list.add(1); list.add(2); list.add(3); list.add(2);
io::println(Boolean.toString(algorithms.isSorted(list)));  // false
let idx = algorithms.isSortedUntil(list);  // 3
```

### inplace_merge

- `algorithms.inplaceMerge<T: Comparable<T>>(items: ArrayList<T>, mid: int): void` — merge two consecutive sorted ranges `[0, mid)` and `[mid, size)` in-place

```titrate
let list = new ArrayList<int>();
list.add(1); list.add(3); list.add(5); list.add(2); list.add(4); list.add(6);
algorithms.inplaceMerge(list, 3);  // [1, 2, 3, 4, 5, 6]
```

### stable_partition

- `algorithms.stablePartition<T>(items: ArrayList<T>, pred: fn(T): bool): int` — partition preserving the relative order of elements within each group; returns the partition point

```titrate
let list = new ArrayList<int>();
list.add(1); list.add(2); list.add(3); list.add(4); list.add(5);
let p = algorithms.stablePartition(list, fn(n: int): bool => n % 2 == 0);
// list is [2, 4, 1, 3, 5]; p is 2
```

### sample

- `algorithms.sample<T>(items: ArrayList<T>, k: int): ArrayList<T>` — pick `k` distinct elements at random

```titrate
let list = new ArrayList<int>();
list.add(1); list.add(2); list.add(3); list.add(4); list.add(5);
let picked = algorithms.sample(list, 3);  // 3 random distinct elements
```

### partial_sort / partial_sort_copy

- `algorithms.partialSort<T: Comparable<T>>(items: ArrayList<T>, k: int): void` — sort so the first `k` elements are the smallest, in order; the remainder are unordered
- `algorithms.partialSortCopy<T: Comparable<T>>(input: ArrayList<T>, k: int): ArrayList<T>` — return a new list containing the `k` smallest elements, sorted

```titrate
let list = new ArrayList<int>();
list.add(9); list.add(3); list.add(7); list.add(1); list.add(5);
algorithms.partialSort(list, 2);
// list.get(0) and list.get(1) are the two smallest (1, 3), in order

let top2 = algorithms.partialSortCopy(list, 2);  // [1, 3]
```

### ExecutionPolicy overloads

Parallel execution policies (from `tt.concurrent.ExecutionPolicy`) provide overloads that dispatch to `ThreadPoolExecutor` for `par`. The following algorithms accept an `ExecutionPolicy` as the first argument:

- `algorithms.sort<T: Comparable<T>>(policy: ExecutionPolicy, items: ArrayList<T>): void`
- `algorithms.transform<T, R>(policy: ExecutionPolicy, items: ArrayList<T>, f: fn(T): R): ArrayList<R>`
- `algorithms.forEach<T>(policy: ExecutionPolicy, items: ArrayList<T>, f: fn(T): void): void`
- `algorithms.reduce<T>(policy: ExecutionPolicy, items: ArrayList<T>, init: T, op: fn(T, T): T): T`

```titrate
import tt.concurrent.ExecutionPolicy;

let list = new ArrayList<int>();
// ... populate list ...
algorithms.sort(ExecutionPolicy.Par(), list);  // parallel sort
```

The supported policies are `Seq()`, `Par()`, `ParUnseq()`, and `UnsequencedPolicy()`. For `Par()`, work is split across the thread pool; the others currently behave like sequential execution.
