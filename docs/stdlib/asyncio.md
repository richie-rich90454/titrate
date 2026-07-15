# Asyncio

The `tt.concurrent.Asyncio` module provides a cooperative single-threaded event loop for Python `asyncio` parity: `EventLoop`, `Task`, `TimerHandle`, async `Lock`/`Semaphore`/`Queue`, `gather`/`wait`/`wait_for`, `sleep`, and a minimal async TCP transport (`StreamReader`/`StreamWriter`/`TcpTransport`). Since the Titrate VM does not yet have native stackless coroutines, async operations are emulated via closures that the event loop drives to completion.

## Import

```titrate
import tt::concurrent::Asyncio;
```

## Classes

### EventLoop

A cooperative event loop that drives `Task`s and timers until stopped or all tasks complete.

**Methods:**
- `runUntilComplete<T>(coro: fn(): T): T` — run until `coro` completes
- `runForever(): void` — run until stopped or all tasks done
- `stop(): void`
- `isRunning(): bool`
- `createTask<T>(coro: fn(): T): Task` — schedule a coroutine
- `callSoon(callback: fn(): void): void` — schedule a callback on the next iteration
- `callLater(delayMs: int, callback: fn(): void): TimerHandle` — schedule after `delayMs` ms
- `sleep(seconds: double): void` — async sleep (blocks the loop iteration)
- `gather(tasks: ArrayList<Task>): ArrayList<Variant>` — wait for all tasks; returns results
- `wait(tasks: ArrayList<Task>, timeout: double): ArrayList<ArrayList<Task>>` — wait for tasks; returns `(done, pending)` lists. `timeout` in seconds; `-1` means no timeout
- `waitFor<T>(task: Task, timeout: double): T` — wait for a single task; throws `TimeoutError`
- `taskCount(): int`
- `iteration(): long` — current loop iteration count

### Task

A unit of work scheduled on the event loop. Wraps a `fn(): Variant` coroutine.

**Methods:**
- `init(id: long, coro: fn(): Variant)`
- `isDone(): bool`
- `isCancelled(): bool`
- `getResult(): Variant` — throws if not done or errored
- `cancel(): void`
- `addDoneCallback(callback: fn(): void): void`

### TimerHandle

A handle to a callback scheduled at a future time.

**Methods:**
- `init(fireTime: long, callback: fn(): void)`
- `fire(): void`
- `cancel(): void`

### Lock

A non-reentrant cooperative lock. `acquire()` blocks the current task until the lock is free.

**Methods:** `acquire(): void`, `release(): void`, `locked(): bool`, `withLock(func: fn(): void): void`

### Semaphore

A counting semaphore for limiting concurrent access.

**Methods:** `init(value: int)`, `acquire(): void`, `release(): void`, `tryAcquire(): bool`, `value(): int`, `withPermit(func: fn(): void): void`

### Queue<T>

A FIFO queue for producer/consumer patterns across tasks.

**Methods:**
- `init()` / `init(maxSize: int)`
- `put(item: T): void` — blocks if full
- `putNowait(item: T): bool`
- `get(): T` — blocks if empty
- `getNowait(): T`
- `qsize(): int`, `empty(): bool`, `full(): bool`
- `close(): void`, `isClosed(): bool`

### StreamReader / StreamWriter / TcpTransport

A minimal async TCP transport.

- `StreamReader(handle: long)` — `read(n: int): string`, `readline(): string`, `readAll(): string`, `atEof(): bool`, `close(): void`
- `StreamWriter(handle: long)` — `write(data: string): void`, `writeln(data: string): void`, `drain(): void`, `close(): void`, `isClosed(): bool`
- `TcpTransport` — `connect(host: string, port: int): bool`, `getReader(): StreamReader`, `getWriter(): StreamWriter`, `getHandle(): long`, `isConnected(): bool`, `close(): void`

## Functions

### getEventLoop

Get the current event loop, creating one if none exists yet.

**Returns:** `EventLoop`

### newEventLoop

Create a new event loop (replacing the current default).

**Returns:** `EventLoop`

### setEventLoop

Set the default event loop to `loop`.

**Parameters:** `loop: EventLoop`

### run

Run `coro` to completion on the current event loop, then close it.

**Parameters:** `coro: fn(): T`
**Returns:** `T`

```titrate
let result: int = run(fn(): int {
    sleep(0.1);
    return 42;
});
io::println(result);  // 42
```

### createTask

Create a `Task` on the current event loop.

**Parameters:** `coro: fn(): T`
**Returns:** `Task`

### sleep

Async sleep for `seconds`. Blocks the current loop iteration.

**Parameters:** `seconds: double`

### gather / wait / waitFor / callSoon / callLater

Module-level shortcuts that delegate to the current event loop (see `EventLoop` methods).

### openTcpConnection

Open a TCP connection to `host:port`. Returns a `TcpTransport`. Throws `ConnectionError` on failure.

**Parameters:** `host: string`, `port: int`
**Returns:** `TcpTransport`

### startTcpServer

Start a TCP server on `port`, invoking `clientHandler` for each accepted connection. The handler receives a `TcpTransport`.

**Parameters:** `port: int`, `clientHandler: fn(TcpTransport): void`
