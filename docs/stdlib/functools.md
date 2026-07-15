# functools

The `tt.functools` module provides higher-order functions and utilities for working with functions as values.

```titrate
import tt.functools;
```

## compose

- `functools.compose<A, B, C>(f: fn(B): C, g: fn(A): B): fn(A): C` — compose two functions: `compose(f, g)(x) == f(g(x))`

```titrate
let double = fn(n: int): int => n * 2;
let inc = fn(n: int): int => n + 1;
let doubleThenInc = functools.compose(inc, double);
io::println(Integer.toString(doubleThenInc(3)));  // 7  (double 3 = 6, then inc = 7)
```

## pipe

- `functools.pipe<A, B, C>(g: fn(A): B, f: fn(B): C): fn(A): C` — left-to-right composition: `pipe(g, f)(x) == f(g(x))`

```titrate
let double = fn(n: int): int => n * 2;
let inc = fn(n: int): int => n + 1;
let incThenDouble = functools.pipe(inc, double);
io::println(Integer.toString(incThenDouble(3)));  // 8  (inc 3 = 4, then double = 8)
```

## curry

- `functools.curry<A, B, C>(f: fn(A, B): C): fn(A): fn(B): C` — transform a two-argument function into a chain of single-argument functions

```titrate
let add = fn(a: int, b: int): int => a + b;
let add5 = functools.curry(add)(5);
io::println(Integer.toString(add5(3)));  // 8
```

## partial

- `functools.partial<A, B, C>(f: fn(A, B): C, a: A): fn(B): C` — bind the first argument of a two-argument function

```titrate
let multiply = fn(a: int, b: int): int => a * b;
let triple = functools.partial(multiply, 3);
io::println(Integer.toString(triple(4)));  // 12
```

## identity

- `functools.identity<T>(x: T): T` — return the argument unchanged

```titrate
let val = functools.identity(42);  // 42
```

## constant

- `functools.constant<T, U>(value: T): fn(U): T` — create a function that always returns `value`, ignoring its argument

```titrate
let alwaysZero = functools.constant(0);
io::println(Integer.toString(alwaysZero(999)));  // 0
```

## flip

- `functools.flip<A, B, C>(f: fn(A, B): C): fn(B, A): C` — swap the argument order of a two-argument function

```titrate
let div = fn(a: int, b: int): int => a / b;
let divFlipped = functools.flip(div);
io::println(Integer.toString(divFlipped(2, 10)));  // 5  (10 / 2)
```

## memoize

- `functools.memoize<A, B>(f: fn(A): B): fn(A): B` — cache function results so repeated calls with the same argument return the cached value

```titrate
let expensive = fn(n: int): int => {
    // ... some heavy computation ...
    return n * n;
};
let fast = functools.memoize(expensive);
io::println(Integer.toString(fast(5)));  // 25 (computed)
io::println(Integer.toString(fast(5)));  // 25 (cached)
```

## memoizeWith

- `functools.memoizeWith<A, B>(f: fn(A): B, maxSize: int): fn(A): B` — memoize with an LRU cache of the given size

```titrate
let cached = functools.memoizeWith(fn(n: int): int => n * n, 100);
```

## lruCache

- `Functools.lruCache(maxSize: int, fn: fn(Variant): Variant): fn(Variant): Variant` — least-recently-used cache decorator

## cachedProperty

- `Functools.cachedProperty(fn: fn(): Variant): fn(): Variant` — cached property decorator

## partial

- `Functools.partial(fn: fn(Variant): Variant, args: ArrayList<Variant>): fn(Variant): Variant` — partial application

## reduce

- `Functools.reduce(fn: fn(Variant, Variant): Variant, iterable: ArrayList, initial: Variant): Variant` — left fold

## singledispatch

- `Functools.singledispatch(fn: fn(Variant): Variant): HashMap<string, fn(Variant): Variant>` — single-dispatch generic function

## wraps

- `Functools.wraps(wrapped: fn): fn` — preserve function metadata in decorator

## totalOrdering

- `Functools.totalOrdering(cls: Class): Class` — fill in missing comparison methods

## cmpToKey

- `Functools.cmpToKey(cmp: fn(Variant, Variant): int): fn(Variant, Variant): int` — convert cmp to key function

## Bind with placeholders (C++ `<functional>` parity, Phase 1-2)

`Bind` wraps a callable, binding some arguments eagerly and leaving placeholders for the rest. Placeholders `_1`, `_2`, ..., `_9` refer to the call-time arguments by position.

- `Bind<T>(fn: fn(...): T, args: ArrayList<Variant>): fn(...): T` — construct a bound callable
- `Functools.bind(f: fn(...): Variant, args: ArrayList<Variant>): Bind` — create a `Bind`
- `Functools._1`, `Functools._2`, ..., `Functools._9` — placeholder constants

```titrate
let sub = fn(a: int, b: int): int => a - b;

// Bind the second argument (b = 100), leaving _1 for the call site
let args: ArrayList<Variant> = new ArrayList<Variant>();
args.add(Functools._1);
args.add(100);
let minus100 = Functools.bind(sub, args);

io::println(Integer.toString(minus100(10)));  // -90  (10 - 100)

// Reorder arguments: _2 first, then _1
let args2: ArrayList<Variant> = new ArrayList<Variant>();
args2.add(Functools._2);
args2.add(Functools._1);
let flippedSub = Functools.bind(sub, args2);
io::println(Integer.toString(flippedSub(10, 100)));  // 90  (100 - 10)
```

## Ref / Cref (reference wrappers)

`Ref` and `Cref` wrap a value so it can be passed by reference through function-type boundaries, mirroring `std::ref` / `std::cref`.

- `Functools.ref<T>(value: T): Ref<T>` — mutable reference wrapper
- `Functools.cref<T>(value: T): Cref<T>` — immutable reference wrapper
- `Ref<T>.get(): T` — dereference
- `Ref<T>.set(value: T): void` — assign through the wrapper

```titrate
let counter: int = 0;
let r = Functools.ref(counter);
r.set(r.get() + 1);  // mutates the wrapped value
```

## Hash

- `Functools.hash(value: Variant): long` — compute a hash code for the value (usable as a `HashMap` key)
- `Functools.hashCombine(seed: long, value: Variant): long` — combine an existing hash with a new value (boost-style `hash_combine`)

```titrate
let h: long = Functools.hash("hello");
let combined: long = Functools.hashCombine(0L, 42);
```

## Functor classes

`Functor` is the base class for callable objects with state. Subclass it to build function objects comparable to C++ `std::function` / custom functors.

- `Functor` — base class; override `invoke(args: ArrayList<Variant>): Variant`
- `Functools.fromFunction<T>(f: fn(...): T): Functor` — wrap a plain function as a `Functor`

```titrate
public class Adder extends Functor {
    public int offset;
    public fn init(o: int) { this.offset = o; }
    public fn invoke(args: ArrayList<Variant>): Variant {
        let x: int = args.get(0) as int;
        return this.offset + x;
    }
}

let add10 = new Adder(10);
io::println(Integer.toString(add10.invoke(ArrayList_of(5)) as int));  // 15
```
