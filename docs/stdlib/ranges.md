# Ranges

The `tt::itertools::Ranges` module provides C++ `<ranges>` parity. It provides C++20 range adaptors and algorithms as eager `ArrayList`-returning functions (Titrate has no lazy coroutine-based views, so each view is materialized into an `ArrayList`). Range concept predicates are exposed as runtime boolean functions.

## Import

```titrate
import tt::itertools::Ranges;
```

## API Reference

### Views

#### `iota(start: int, count: int): ArrayList<int>`

`views::iota` — generate `[start, start+count)` as an `ArrayList<int>`.

#### `iotaInclusive(start: int, end: int): ArrayList<int>`

`views::iota` with an inclusive end bound: `[start, end]`.

#### `keys<K, V>(pairs: ArrayList<Pair<K, V>>): ArrayList<K>`

`views::keys` — extract the first element of each `Pair` in a list of pairs.

#### `values<K, V>(pairs: ArrayList<Pair<K, V>>): ArrayList<V>`

`views::values` — extract the second element of each `Pair` in a list of pairs.

#### `elements(list: ArrayList<ArrayList<Variant>>, index: int): ArrayList<Variant>`

`views::elements<N>` — extract the Nth element of each tuple-like list. Tuples are represented as `ArrayList<Variant>`; `index` selects the element.

#### `adjacent<T>(list: ArrayList<T>, n: int): ArrayList<ArrayList<T>>`

`views::adjacent<N>` — group consecutive elements into overlapping windows of size `n`. Returns a list of `n`-element `ArrayList`s.

#### `pairwise<T>(list: ArrayList<T>): ArrayList<ArrayList<T>>`

`views::pairwise` — `adjacent<2>`: consecutive overlapping pairs.

#### `join<T>(lists: ArrayList<ArrayList<T>>): ArrayList<T>`

`views::join` — flatten a list of lists into a single list.

#### `joinWith<T>(lists: ArrayList<ArrayList<T>>, separator: T): ArrayList<T>`

`views::join_with` — flatten a list of lists, inserting `separator` between groups.

#### `split<T>(list: ArrayList<T>, delimiter: T): ArrayList<ArrayList<T>>`

`views::split` — split a list into sublists at each occurrence of `delimiter`.

#### `lazySplit<T>(list: ArrayList<T>, delimiter: T): ArrayList<ArrayList<T>>`

`views::lazy_split` — same eager semantics as `split` in Titrate (no lazy coroutines). Provided for API parity.

#### `stride<T>(list: ArrayList<T>, n: int): ArrayList<T>`

`views::stride` — take every `n`th element of the list.

#### `chunk<T>(list: ArrayList<T>, n: int): ArrayList<ArrayList<T>>`

`views::chunk` — partition the list into non-overlapping chunks of size `n`. The final chunk may be smaller than `n`.

#### `slide<T>(list: ArrayList<T>, n: int): ArrayList<ArrayList<T>>`

`views::slide` — sliding window of size `n` (overlapping). Alias for `adjacent` with the same window size; provided under the C++23 name `views::slide`.

#### `common<T>(list: ArrayList<T>): ArrayList<T>`

`views::common` — convert a range to a common (iterator/sentinel-paired) range. In Titrate this is the identity on `ArrayList`.

#### `reverse<T>(list: ArrayList<T>): ArrayList<T>`

`views::reverse` — return a new list with elements in reverse order.

#### `single<T>(value: T): ArrayList<T>`

`views::single` — a range containing exactly one element.

#### `emptyRange<T>(): ArrayList<T>`

`views::empty` — an empty range of the given element type.

#### `repeat<T>(value: T, count: int): ArrayList<T>`

`views::repeat` — repeat `value` `count` times.

### Range Algorithms

#### `sort<T>(list: ArrayList<T>): ArrayList<T>`

`ranges::sort` — return a new sorted list (quicksort). Does not mutate the input.

#### `sortWithProjection<T, K>(list: ArrayList<T>, projection: fn(T): K): ArrayList<T>`

`ranges::sort` with a projection: project each element, compare projections. Uses stable selection sort by projected key.

#### `copy<T>(list: ArrayList<T>): ArrayList<T>`

`ranges::copy` — return a shallow copy of the list.

#### `copyProjection<T, K>(list: ArrayList<T>, projection: fn(T): K): ArrayList<K>`

`ranges::copy` with a projection: copy projected values into a new list.

#### `transform<T, U>(list: ArrayList<T>, fn: fn(T): U): ArrayList<U>`

`ranges::transform` — apply `fn` to each element, returning a new list.

#### `transformWithProjection<T, K, U>(list: ArrayList<T>, projection: fn(T): K, fn: fn(K): U): ArrayList<U>`

`ranges::transform` with a projection: `transform(element)` is applied to the projected value of each element.

#### `to<T>(list: ArrayList<T>): ArrayList<T>`

`ranges::to` — materialize any range-like into an `ArrayList`. In Titrate the input is already an `ArrayList`, so this is the identity (with a clone).

#### `toTransform<T, U>(list: ArrayList<T>, fn: fn(T): U): ArrayList<U>`

`ranges::to` with a transform: materialize while applying a transform.

### Range Concepts

Runtime predicate functions modeling the C++20 range concepts. They inspect the runtime shape of the value (an `ArrayList` is a sized range; a plain object is not) rather than compile-time type traits.

- `isRange(value: Variant): bool` — `range`: true if the value is an iterable list-like object
- `isView(value: Variant): bool` — `view`: true if the value is a view (always `false` in Titrate; `ArrayList`s are owning)
- `isSizedRange(value: Variant): bool` — `sized_range`: true if the range supports O(1) size queries
- `isCommonRange(value: Variant): bool` — `common_range`: true if the iterator and sentinel types are the same
- `isBidirectionalRange(value: Variant): bool` — `bidirectional_range`: true if the range supports decrement
- `isRandomAccessRange(value: Variant): bool` — `random_access_range`: true if the range supports O(1) indexed access
- `isContiguousRange(value: Variant): bool` — `contiguous_range`: true if the range's elements are stored contiguously
- `enableBorrowedRange(value: Variant): bool` — `enable_borrowed_range`: always `false` in Titrate (no borrowed ranges)

## Usage Examples

### Generating Ranges with iota

```titrate
import tt::itertools::Ranges;
import tt::io::IO;

public fn main(): void {
    let nums: ArrayList<int> = Ranges.iota(0, 5);
    IO.println("iota(0,5): " + Integer.toString(nums.size()));
    let inc: ArrayList<int> = Ranges.iotaInclusive(1, 3);
    IO.println("iotaInclusive(1,3): " + Integer.toString(inc.size()));
}
```

### Transforming and Reversing

```titrate
import tt::itertools::Ranges;
import tt::util::ArrayList;

let list = new ArrayList<int>();
list.add(1); list.add(2); list.add(3);
let doubled: ArrayList<int> = Ranges.transform<int, int>(list, fn(x: int): int => x * 2);
let reversed: ArrayList<int> = Ranges.reverse<int>(doubled);
io::println("reversed doubled: " + Integer.toString(reversed.get(0)));
```

### Chunking and Sliding Windows

```titrate
import tt::itertools::Ranges;
import tt::util::ArrayList;

let list = new ArrayList<int>();
list.add(1); list.add(2); list.add(3); list.add(4); list.add(5);
let chunks: ArrayList<ArrayList<int>> = Ranges.chunk<int>(list, 2);
io::println("chunk count: " + Integer.toString(chunks.size()));
let windows: ArrayList<ArrayList<int>> = Ranges.slide<int>(list, 3);
io::println("slide count: " + Integer.toString(windows.size()));
```

### Splitting and Joining

```titrate
import tt::itertools::Ranges;
import tt::util::ArrayList;

let list = new ArrayList<int>();
list.add(1); list.add(0); list.add(2); list.add(0); list.add(3);
let parts: ArrayList<ArrayList<int>> = Ranges.split<int>(list, 0);
io::println("split count: " + Integer.toString(parts.size()));

let lists = new ArrayList<ArrayList<int>>();
let inner1 = new ArrayList<int>(); inner1.add(1); inner1.add(2);
let inner2 = new ArrayList<int>(); inner2.add(3); inner2.add(4);
lists.add(inner1); lists.add(inner2);
let joined: ArrayList<int> = Ranges.join<int>(lists);
io::println("joined size: " + Integer.toString(joined.size()));
```

### Checking Range Concepts

```titrate
import tt::itertools::Ranges;
import tt::lang::Variant;
import tt::util::ArrayList;

let list = new ArrayList<int>();
list.add(1); list.add(2);
let v: Variant = list;
io::println("isRange: " + (Ranges.isRange(v) ? "true" : "false"));
io::println("isSizedRange: " + (Ranges.isSizedRange(v) ? "true" : "false"));
io::println("isContiguousRange: " + (Ranges.isContiguousRange(v) ? "true" : "false"));
```
