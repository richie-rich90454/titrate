# concurrent

The `tt.concurrent` module provides asynchronous programming primitives with futures and channels.

```titrate
import tt.concurrent.Future;
import tt.concurrent.Channel;
```

## Future

A placeholder for a result that will be available asynchronously.

- `fn init()` — create an unresolved future
- `isDone(): bool` — check if completed
- `get(): T` — get the value (available after completion)
- `isCancelled(): bool` — check if cancelled
- `cancel(): bool` — cancel if not done
- `cancel(mayInterrupt: bool): bool` — cancel with interrupt flag
- `complete(value: T): void` — resolve with a value
- `completeExceptionally(err: string): void` — resolve with an error
- `getError(): string` — get the error message
- `hasError(): bool` — check if completed with error

**Combinators:**
- `thenApply<R>(fn: fn(T): R): Future<R>` — transform the value when complete
- `thenCompose<R>(fn: fn(T): Future<R>): Future<R>` — chain with another future
- `exceptionally(fn: fn(string): T): Future<T>` — recover from error
- `handle<R>(fn: fn(T, string): R): Future<R>` — handle both success and error
- `whenComplete(fn: fn(T, string): void): Future<T>` — side-effect on completion

```titrate
let f = new Future<int>();
f.complete(42);
let doubled = f.thenApply(fn(v: int): int { return v * 2; });
io::println(Integer.toString(doubled.get()));  // 84
```

## Channel

A message-passing channel for communication between concurrent tasks.

- `fn init()` — create an unbounded channel
- `fn init(capacity: int)` — create a bounded channel
- `send(value: T): void` — send a value (drops if closed or full)
- `trySend(value: T): bool` — send without blocking; returns false if closed/full
- `receive(): T` — receive the next value
- `tryReceive(): T` — receive or return null if empty
- `len(): int` — number of buffered items
- `isFull(): bool` — check if at capacity
- `isEmpty(): bool` — check if empty
- `close(): void` — close the channel
- `isClosed(): bool` — check if closed
- `onClose(fn: fn(): void): void` — register callback on close

```titrate
let ch = new Channel<string>(10);
ch.send("hello");
ch.send("world");
io::println(ch.receive());  // "hello"
io::println(Integer.toString(ch.len()));      // 1
ch.close();
```
