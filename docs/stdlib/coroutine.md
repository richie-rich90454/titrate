# Coroutine

The `tt::concurrent::Coroutine` module provides C++ `<coroutine>` parity. It implements coroutine primitives via a generator/iterator pattern since the Titrate VM does not yet have native stackless coroutines. The `Generator` class emulates `co_yield`/`co_await` by running a producer function that pushes values into a buffered queue. `CoroutineHandle` exposes the `std::coroutine_handle` API (`resume`, `done`, `promise`, `destroy`). `SuspendAlways`/`SuspendNever` are marker types matching `std::suspend_always` / `std::suspend_never`.

## Import

```titrate
import tt::concurrent::Coroutine;
```

## API Reference

### Suspend Markers

#### `SuspendAlways`

Suspend policy marker: always suspend at this await point (`std::suspend_always`).

**Methods:**
- `awaitReady(): bool` — returns `false`
- `awaitSuspend(handle: CoroutineHandle<Variant>): void` — no-op (suspension is emulated)
- `awaitResume(): Variant` — returns `null`
- `toString(): string` — returns `"SuspendAlways"`

#### `SuspendNever`

Suspend policy marker: never suspend (`std::suspend_never`).

**Methods:**
- `awaitReady(): bool` — returns `true`
- `awaitSuspend(handle: CoroutineHandle<Variant>): void` — never called
- `awaitResume(): Variant` — returns `null`
- `toString(): string` — returns `"SuspendNever"`

### `CoroutinePromise<T>`

The promise object backing a coroutine (`std::coroutine_traits::promise_type`).

**Methods:**
- `getReturnObject(): CoroutineHandle<T>` — returns a handle to this promise
- `initialSuspend(): SuspendAlways` — always suspend initially
- `finalSuspend(): SuspendAlways` — always suspend at the end
- `yieldValue(value: T): SuspendAlways` — store a yielded value and suspend
- `returnValue(value: T): void` — store the final return value
- `setException(message: string): void` — record an exception
- `unhandledException(): void` — record an unhandled exception
- `current(): T` — returns the current value
- `hasError(): bool` — returns true if an error occurred
- `error(): string` — returns the error message
- `setSentValue(value: Variant): void` — store a value sent by the caller
- `sentValue(): Variant` — returns the last sent value

### `CoroutineTraits<T>`

Describes the promise type for a coroutine returning `T` (`std::coroutine_traits<T, Args...>`).

**Methods:**
- `promiseType(): string` — returns `"CoroutinePromise<T>"`

### `CoroutineHandle<T>`

Opaque handle to a coroutine frame (`std::coroutine_handle<T>`).

**Methods:**
- `resume(): void` — advance the coroutine frame
- `done(): bool` — returns true if the coroutine has finished
- `promise(): CoroutinePromise<T>` — returns the backing promise
- `destroy(): void` — destroy the coroutine frame
- `isDestroyed(): bool`
- `operator==(other: CoroutineHandle<T>): bool`
- `toString(): string`

### `YieldContext<T>`

Passed to a generator's producer function. The producer calls `ctx.yield(value)` to emit a value and `ctx.sent()` to read the value most recently sent by the caller via `Generator.send()`.

**Methods:**
- `yield(value: T): void` — emit a value
- `sent(): Variant` — returns the last sent value
- `setSentValue(value: Variant): void`
- `isClosed(): bool`
- `close(): void`
- `poll(): T` — pull the next buffered value (or `null`)
- `hasBuffered(): bool`
- `buffer(): ArrayList<T>`

### `Generator<T>`

Emulates a coroutine that yields a sequence of values of type `T`. The producer function receives a `YieldContext<T>` and may call `ctx.yield(v)` any number of times.

**Constructor:**
- `init(producer: fn(YieldContext<T>): void)`

**Methods:**
- `next(): T` — advance to the next yield point and return the yielded value
- `send(value: Variant): T` — send a value into the generator and return the next yielded value
- `close(): void` — close the generator
- `isDone(): bool`
- `isClosed(): bool`
- `current(): T` — returns the most recently yielded value
- `collect(): ArrayList<T>` — collect all remaining values into an `ArrayList`
- `map<R>(f: fn(T): R): Generator<R>` — apply a mapping function, producing a new `Generator`
- `filter(predicate: fn(T): bool): Generator<T>` — filter yielded values by a predicate
- `take(n: int): Generator<T>` — take at most `n` values
- `asHandle(): CoroutineHandle<Variant>` — return a `CoroutineHandle` view for C++ API compatibility
- `resume(): void` — CoroutineHandle-compatible advancement
- `done(): bool`
- `destroy(): void`
- `toString(): string`

### `Awaitable<T>`

Wraps a `Future` and exposes the C++ await protocol.

**Methods:**
- `awaitReady(): bool` — returns true if the future is done
- `awaitSuspend(handle: CoroutineHandle<Variant>): void` — resume the handle when the future completes
- `awaitResume(): T` — returns the future's result
- `future(): Future<T>`

### Free Functions

- `awaitable<T>(future: Future<T>): Awaitable<T>` — wrap a `Future` in an `Awaitable`
- `generator<T>(producer: fn(YieldContext<T>): void): Generator<T>` — create a `Generator` from a producer function
- `iotaRange(start: int, end: int): Generator<int>` — yield integers `[start, end)`
- `iotaInfinite(start: int, step: int): Generator<int>` — yield an infinite sequence stepping by `step`
- `repeat<T>(value: T): Generator<T>` — repeat a single value indefinitely
- `fromPromise<T>(promise: CoroutinePromise<T>): CoroutineHandle<T>` — `std::coroutine_handle::from_promise`
- `noopCoroutine(): CoroutineHandle<Variant>` — create a no-op coroutine handle
- `runAsync<T>(asyncFn: fn(): T): T` — run a coroutine-style async function to completion
- `chain<T>(first: Generator<T>, second: Generator<T>): Generator<T>` — concatenate two generators
- `zip<A, B>(genA: Generator<A>, genB: Generator<B>): Generator<Pair<A, B>>` — zip two generators

## Usage Examples

### Basic Generator

```titrate
import tt::concurrent::Coroutine;
import tt::io::IO;

public fn main(): void {
    let gen: Generator<int> = Coroutine.iotaRange(1, 5);  // yields 1, 2, 3, 4
    while (!gen.isDone()) {
        let v: int = gen.next();
        if (v == null) { break; }
        IO.println(v);   // 1, 2, 3, 4
    }
}
```

### Mapping and Filtering

```titrate
import tt::concurrent::Coroutine;

let nums: Generator<int> = Coroutine.iotaRange(1, 10);
let squares: Generator<int> = nums.map(fn(x: int): int => x * x);
let evens: Generator<int> = squares.filter(fn(x: int): bool => x % 2 == 0);
let firstThree: Generator<int> = evens.take(3);

let collected: ArrayList<int> = firstThree.collect();
// collected = [4, 16, 36]  (2^2, 4^2, 6^2)
```

### Custom Producer with YieldContext

```titrate
import tt::concurrent::Coroutine;

public fn fibonacci(): Generator<int> {
    let producer: fn(YieldContext<int>): void = fn(ctx: YieldContext<int>): void {
        var a: int = 0;
        var b: int = 1;
        var i: int = 0;
        while (i < 10) {
            ctx.yield(a);
            let next: int = a + b;
            a = b;
            b = next;
            i = i + 1;
        }
    };
    return new Generator<int>(producer);
}

public fn main(): void {
    let fib: Generator<int> = fibonacci();
    let seq: ArrayList<int> = fib.collect();
    // seq = [0, 1, 1, 2, 3, 5, 8, 13, 21, 34]
    for (n in seq) { io::println(n); }
}
```

### Awaiting a Future

```titrate
import tt::concurrent::Coroutine;
import tt::concurrent::Future;

let fut: Future<int> = Future.completed(42);
let aw: Awaitable<int> = Coroutine.awaitable(fut);
if (aw.awaitReady()) {
    io::println(aw.awaitResume());  // 42
}
```

### Chaining and Zipping Generators

```titrate
import tt::concurrent::Coroutine;

let a: Generator<int> = Coroutine.iotaRange(0, 3);   // 0, 1, 2
let b: Generator<int> = Coroutine.iotaRange(10, 13); // 10, 11, 12
let zipped: Generator<Pair<int, int>> = Coroutine.zip(a, b);
let pairs: ArrayList<Pair<int, int>> = zipped.collect();
// pairs = [(0,10), (1,11), (2,12)]
```
