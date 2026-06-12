# itertools

The `tt.itertools` module provides iterator adapters and combinators for working with sequences. These functions return lazy iterators that compute values on demand, making them efficient for large or infinite sequences.

```titrate
import tt.itertools;
```

## count

- `itertools.count(start: int, step: int): Iterator<int>` — infinite iterator starting at `start`, incrementing by `step`

```titrate
let nums = itertools.count(0, 2);
// 0, 2, 4, 6, 8, ...
```

## cycle

- `itertools.cycle<T>(iter: Iterator<T>): Iterator<T>` — repeat an iterator endlessly

```titrate
let repeated = itertools.cycle([1, 2, 3].iterator());
// 1, 2, 3, 1, 2, 3, 1, 2, 3, ...
```

## repeat

- `itertools.repeat<T>(item: T): Iterator<T>` — infinite iterator yielding the same item
- `itertools.repeat<T>(item: T, n: int): Iterator<T>` — yield `item` exactly `n` times

```titrate
let forever = itertools.repeat(0);
let limited = itertools.repeat("hi", 3);
// "hi", "hi", "hi"
```

## chain

- `itertools.chain<T>(a: Iterator<T>, b: Iterator<T>): Iterator<T>` — concatenate two iterators

```titrate
let combined = itertools.chain([1, 2].iterator(), [3, 4].iterator());
// 1, 2, 3, 4
```

## zip

- `itertools.zip<A, B>(a: Iterator<A>, b: Iterator<B>): Iterator<(A, B)>` — pair elements from two iterators

```titrate
let pairs = itertools.zip([1, 2, 3].iterator(), ["a", "b", "c"].iterator());
// (1, "a"), (2, "b"), (3, "c")
```

## enumerate

- `itertools.enumerate<T>(iter: Iterator<T>): Iterator<(int, T)>` — pair each element with its index

```titrate
for (pair in itertools.enumerate(["x", "y", "z"].iterator())) {
    io::println(Integer.toString(pair.0) + ": " + pair.1);
}
// 0: x
// 1: y
// 2: z
```

## take

- `itertools.take<T>(iter: Iterator<T>, n: int): Iterator<T>` — yield the first `n` elements

```titrate
let first5 = itertools.take(itertools.count(1, 1), 5);
// 1, 2, 3, 4, 5
```

## drop

- `itertools.drop<T>(iter: Iterator<T>, n: int): Iterator<T>` — skip the first `n` elements

```titrate
let rest = itertools.drop([1, 2, 3, 4, 5].iterator(), 2);
// 3, 4, 5
```

## filter

- `itertools.filter<T>(iter: Iterator<T>, pred: fn(T): bool): Iterator<T>` — keep elements matching a predicate

```titrate
let evens = itertools.filter([1, 2, 3, 4, 5, 6].iterator(), fn(n: int): bool => n % 2 == 0);
// 2, 4, 6
```

## map

- `itertools.map<T, U>(iter: Iterator<T>, f: fn(T): U): Iterator<U>` — transform each element

```titrate
let doubled = itertools.map([1, 2, 3].iterator(), fn(n: int): int => n * 2);
// 2, 4, 6
```

## takeWhile

- `itertools.takeWhile<T>(iter: Iterator<T>, pred: fn(T): bool): Iterator<T>` — yield elements while the predicate holds, then stop

```titrate
let prefix = itertools.takeWhile([1, 2, 3, 10, 4, 5].iterator(), fn(n: int): bool => n < 5);
// 1, 2, 3
```

## dropWhile

- `itertools.dropWhile<T>(iter: Iterator<T>, pred: fn(T): bool): Iterator<T>` — skip elements while the predicate holds, then yield the rest

```titrate
let suffix = itertools.dropWhile([1, 2, 3, 10, 4, 5].iterator(), fn(n: int): bool => n < 5);
// 10, 4, 5
```

## slice

- `itertools.slice<T>(iter: Iterator<T>, start: int, end: int): Iterator<T>` — yield elements from index `start` to `end` (exclusive)

```titrate
let mid = itertools.slice([0, 1, 2, 3, 4, 5].iterator(), 2, 5);
// 2, 3, 4
```

## flatMap

- `itertools.flatMap<T, U>(iter: Iterator<T>, f: fn(T): Iterator<U>): Iterator<U>` — map each element to an iterator and flatten

```titrate
let expanded = itertools.flatMap([1, 2, 3].iterator(), fn(n: int): Iterator<string> => itertools.repeat(Integer.toString(n), n));
// "1", "2", "2", "3", "3", "3"
```

## reduce

- `itertools.reduce<T>(iter: Iterator<T>, f: fn(T, T): T): T` — combine all elements using a binary operator

```titrate
let sum = itertools.reduce([1, 2, 3, 4].iterator(), fn(a: int, b: int): int => a + b);
// 10
```

## any / all

- `itertools.any<T>(iter: Iterator<T>, pred: fn(T): bool): bool` — true if any element satisfies the predicate
- `itertools.all<T>(iter: Iterator<T>, pred: fn(T): bool): bool` — true if all elements satisfy the predicate

```titrate
let hasEven = itertools.any([1, 3, 5, 6].iterator(), fn(n: int): bool => n % 2 == 0);  // true
let allPos = itertools.all([1, 2, 3].iterator(), fn(n: int): bool => n > 0);            // true
```

## min / max

- `itertools.min<T>(iter: Iterator<T>): T` — smallest element
- `itertools.max<T>(iter: Iterator<T>): T` — largest element

```titrate
let lo = itertools.min([3, 1, 4, 1, 5].iterator());  // 1
let hi = itertools.max([3, 1, 4, 1, 5].iterator());  // 5
```

## collect

- `itertools.collect<T>(iter: Iterator<T>): ArrayList<T>` — consume the iterator into an `ArrayList`

```titrate
let list = itertools.collect(itertools.map([1, 2, 3].iterator(), fn(n: int): int => n * n));
// ArrayList containing [1, 4, 9]
```
