# functools

The `tt.functools` module provides higher-order functions and utilities for working with functions as values.

```titrate
import tt.functools;
```

## compose

- `functools::compose<A, B, C>(f: fn(B): C, g: fn(A): B): fn(A): C` — compose two functions: `compose(f, g)(x) == f(g(x))`

```titrate
let double = fn(n: int): int => n * 2;
let inc = fn(n: int): int => n + 1;
let doubleThenInc = functools::compose(inc, double);
io::println(doubleThenInc(3).toString());  // 7  (double 3 = 6, then inc = 7)
```

## pipe

- `functools::pipe<A, B, C>(g: fn(A): B, f: fn(B): C): fn(A): C` — left-to-right composition: `pipe(g, f)(x) == f(g(x))`

```titrate
let double = fn(n: int): int => n * 2;
let inc = fn(n: int): int => n + 1;
let incThenDouble = functools::pipe(inc, double);
io::println(incThenDouble(3).toString());  // 8  (inc 3 = 4, then double = 8)
```

## curry

- `functools::curry<A, B, C>(f: fn(A, B): C): fn(A): fn(B): C` — transform a two-argument function into a chain of single-argument functions

```titrate
let add = fn(a: int, b: int): int => a + b;
let add5 = functools::curry(add)(5);
io::println(add5(3).toString());  // 8
```

## partial

- `functools::partial<A, B, C>(f: fn(A, B): C, a: A): fn(B): C` — bind the first argument of a two-argument function

```titrate
let multiply = fn(a: int, b: int): int => a * b;
let triple = functools::partial(multiply, 3);
io::println(triple(4).toString());  // 12
```

## identity

- `functools::identity<T>(x: T): T` — return the argument unchanged

```titrate
let val = functools::identity(42);  // 42
```

## constant

- `functools::constant<T, U>(value: T): fn(U): T` — create a function that always returns `value`, ignoring its argument

```titrate
let alwaysZero = functools::constant(0);
io::println(alwaysZero(999).toString());  // 0
```

## flip

- `functools::flip<A, B, C>(f: fn(A, B): C): fn(B, A): C` — swap the argument order of a two-argument function

```titrate
let div = fn(a: int, b: int): int => a / b;
let divFlipped = functools::flip(div);
io::println(divFlipped(2, 10).toString());  // 5  (10 / 2)
```

## memoize

- `functools::memoize<A, B>(f: fn(A): B): fn(A): B` — cache function results so repeated calls with the same argument return the cached value

```titrate
let expensive = fn(n: int): int => {
    // ... some heavy computation ...
    return n * n;
};
let fast = functools::memoize(expensive);
io::println(fast(5).toString());  // 25 (computed)
io::println(fast(5).toString());  // 25 (cached)
```

## memoizeWith

- `functools::memoizeWith<A, B>(f: fn(A): B, maxSize: int): fn(A): B` — memoize with an LRU cache of the given size

```titrate
let cached = functools::memoizeWith(fn(n: int): int => n * n, 100);
```
