# algorithms

The `tt.algorithms` module provides common algorithms for sorting, searching, and transforming collections.

```titrate
import tt.algorithms;
```

## Sorting

### sort

- `algorithms::sort<T: Comparable>(items: ArrayList<T>): void` — sort a list in-place in ascending order

```titrate
let list = new ArrayList<int>();
list.add(3); list.add(1); list.add(4); list.add(1); list.add(5);
algorithms::sort(list);
// list is now [1, 1, 3, 4, 5]
```

### sorted

- `algorithms::sorted<T: Comparable>(items: ArrayList<T>): ArrayList<T>` — return a new sorted list, leaving the original unchanged

```titrate
let original = new ArrayList<int>();
original.add(3); original.add(1); original.add(4);
let result = algorithms::sorted(original);
// original is still [3, 1, 4], result is [1, 3, 4]
```

### sortBy

- `algorithms::sortBy<T>(items: ArrayList<T>, key: fn(T): double): void` — sort in-place using a key function

```titrate
let names = new ArrayList<string>();
names.add("charlie"); names.add("alice"); names.add("bob");
algorithms::sortBy(names, fn(s: string): double => s.length());
// sorted by length: "bob", "alice", "charlie"
```

### reverse

- `algorithms::reverse<T>(items: ArrayList<T>): void` — reverse a list in-place

```titrate
let list = new ArrayList<int>();
list.add(1); list.add(2); list.add(3);
algorithms::reverse(list);
// list is now [3, 2, 1]
```

## Searching

### binarySearch

- `algorithms::binarySearch<T: Comparable>(items: ArrayList<T>, target: T): int` — find `target` in a sorted list; returns the index or -1 if not found

```titrate
let list = new ArrayList<int>();
list.add(1); list.add(3); list.add(5); list.add(7); list.add(9);
let idx = algorithms::binarySearch(list, 5);  // 2
let missing = algorithms::binarySearch(list, 6);  // -1
```

### linearSearch

- `algorithms::linearSearch<T>(items: ArrayList<T>, target: T): int` — find `target` by scanning; returns the index or -1

### contains

- `algorithms::contains<T>(items: ArrayList<T>, target: T): bool` — check if a list contains the target

### find

- `algorithms::find<T>(items: ArrayList<T>, pred: fn(T): bool): T` — return the first element matching the predicate

### findIndex

- `algorithms::findIndex<T>(items: ArrayList<T>, pred: fn(T): bool): int` — return the index of the first element matching the predicate, or -1

## Set Operations

### unique

- `algorithms::unique<T>(items: ArrayList<T>): ArrayList<T>` — remove duplicates, preserving order

```titrate
let list = new ArrayList<int>();
list.add(1); list.add(3); list.add(1); list.add(2); list.add(3);
let deduped = algorithms::unique(list);
// [1, 3, 2]
```

### union

- `algorithms::union<T>(a: ArrayList<T>, b: ArrayList<T>): ArrayList<T>` — elements in either list, without duplicates

### intersection

- `algorithms::intersection<T>(a: ArrayList<T>, b: ArrayList<T>): ArrayList<T>` — elements in both lists

### difference

- `algorithms::difference<T>(a: ArrayList<T>, b: ArrayList<T>): ArrayList<T>` — elements in `a` but not in `b`

## Transformation

### flatten

- `algorithms::flatten<T>(items: ArrayList<ArrayList<T>>): ArrayList<T>` — concatenate nested lists into one

```titrate
let nested = new ArrayList<ArrayList<int>>();
// [[1, 2], [3, 4], [5]]
let flat = algorithms::flatten(nested);
// [1, 2, 3, 4, 5]
```

### zip

- `algorithms::zip<A, B>(a: ArrayList<A>, b: ArrayList<B>): ArrayList<(A, B)>` — pair elements from two lists

```titrate
let nums = new ArrayList<int>();
nums.add(1); nums.add(2); nums.add(3);
let letters = new ArrayList<string>();
letters.add("a"); letters.add("b"); letters.add("c");
let pairs = algorithms::zip(nums, letters);
// [(1, "a"), (2, "b"), (3, "c")]
```

### chunk

- `algorithms::chunk<T>(items: ArrayList<T>, size: int): ArrayList<ArrayList<T>>` — split a list into chunks of the given size

```titrate
let list = new ArrayList<int>();
list.add(1); list.add(2); list.add(3); list.add(4); list.add(5);
let chunks = algorithms::chunk(list, 2);
// [[1, 2], [3, 4], [5]]
```

### partition

- `algorithms::partition<T>(items: ArrayList<T>, pred: fn(T): bool): (ArrayList<T>, ArrayList<T>)` — split into elements matching and not matching the predicate

```titrate
let list = new ArrayList<int>();
list.add(1); list.add(2); list.add(3); list.add(4); list.add(5);
let (evens, odds) = algorithms::partition(list, fn(n: int): bool => n % 2 == 0);
// evens: [2, 4], odds: [1, 3, 5]
```

## Aggregation

### sum

- `algorithms::sum(items: ArrayList<double>): double` — sum all elements

### product

- `algorithms::product(items: ArrayList<double>): double` — multiply all elements

### count

- `algorithms::count<T>(items: ArrayList<T>, pred: fn(T): bool): int` — count elements matching the predicate

```titrate
let list = new ArrayList<int>();
list.add(1); list.add(2); list.add(3); list.add(4); list.add(5);
let evenCount = algorithms::count(list, fn(n: int): bool => n % 2 == 0);  // 2
```

## Shuffle

### shuffle

- `algorithms::shuffle<T>(items: ArrayList<T>): void` — randomly permute a list in-place

```titrate
let list = new ArrayList<int>();
list.add(1); list.add(2); list.add(3); list.add(4);
algorithms::shuffle(list);
// list is now in random order
```

### shuffled

- `algorithms::shuffled<T>(items: ArrayList<T>): ArrayList<T>` — return a new randomly permuted list
