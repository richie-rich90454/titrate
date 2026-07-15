# Multiprocessing

The `tt.concurrent.Multiprocessing` module provides a Python `multiprocessing` analog: `Process`, `Queue`, `Pipe`, `Pool`, `Lock`, `Manager`, and the `currentProcess` / `activeChildren` helpers. Because the Titrate VM is single-process with thread-based concurrency, "processes" are modeled as `Thread`s with their own identity and a shared registry of active children.

## Import

```titrate
import tt::concurrent::Multiprocessing;
```

## Classes

### Process

Represents an activity that runs in a forked/spawned context (modeled as a thread).

**Fields:**
- `name: string`
- `target: fn(): void`
- `daemon: bool`

**Methods:**
- `init(target: fn(): void)`
- `setName(name: string): void`
- `start(): void` — spawn the process (target runs synchronously due to VM limits)
- `join(timeout: long): void` — block until the process terminates
- `terminate(): void` — best-effort termination
- `isAlive(): bool`
- `pid(): int` — process identifier
- `exitcode(): int` — `-1` if not yet terminated, `0` on success, non-zero on failure
- `isDaemon(): bool`

```titrate
let p = new Process(fn(): void {
    io::println("running in child");
});
p.start();
p.join(0);
```

### Queue

Process-safe FIFO queue backed by a Mutex-protected `ArrayList<Variant>`.

**Methods:**
- `init()`
- `put(item: Variant): void`
- `get(): Variant` — remove and return an item, or `null` if empty (non-blocking)
- `size(): int`
- `isEmpty(): bool`
- `clear(): void`

### Connection

Endpoint of a duplex pipe.

**Methods:**
- `init(inbound: Queue, outbound: Queue)`
- `send(obj: Variant): void`
- `recv(): Variant` — non-blocking; `null` if empty
- `close(): void`
- `closed(): bool`

### Pool

A pool of worker processes that consume tasks from a shared queue.

**Fields:**
- `numProcesses: int`

**Methods:**
- `init(numProcesses: int)` — uses default pool size (4) if `<= 0`
- `apply(func: fn(Variant): Variant, args: Variant): Variant` — apply a function to a single argument
- `map(func: fn(Variant): Variant, iterable: ArrayList<Variant>): ArrayList<Variant>` — apply to a list (best-effort parallel)
- `applyAsync(func: fn(Variant): Variant, args: Variant): AsyncResult` — returns immediately
- `close(): void` — prevent any more tasks
- `join(): void` — wait for outstanding tasks

### AsyncResult

Future-like object returned by `Pool.applyAsync()`.

**Methods:**
- `init()`
- `get(timeout: long): Variant` — block until ready, then return
- `ready(): bool`
- `_setResult(value: Variant): void` — internal use by `Pool`

### Lock

Re-export of `Mutex` as a multiprocessing-style lock.

**Methods:**
- `init()`
- `acquire(block: bool, timeout: long): bool`
- `release(): void`

### Manager / SharedValue / SharedList / SharedDict

`Manager` is a stub that creates shared-state wrappers backed by the in-process GC heap.

- `Manager.Value(typeCode: string, initialValue: Variant): SharedValue`
- `Manager.list(initial: ArrayList<Variant>): SharedList`
- `Manager.dict(initial: HashMap<string, Variant>): SharedDict`

`SharedValue` exposes `get(): Variant` / `set(value: Variant): void`. `SharedList` exposes `append(item: Variant): void`, `get(index: int): Variant`, `size(): int`. `SharedDict` exposes `put(key, value): void`, `get(key): Variant`.

## Functions

### currentProcess

Return the current `Process` object (named `"MainProcess"` in the main entry).

**Returns:** `Process`

### activeChildren

Return a list of child processes spawned by the current process.

**Returns:** `ArrayList<Process>`

### Pipe

Create a duplex pipe returning a pair of `Connection` endpoints.

**Parameters:** `duplex: bool`
**Returns:** `(Connection, Connection)`

```titrate
let (parentEnd, childEnd) = Pipe(true);
parentEnd.send("hello");
io::println(childEnd.recv() as string);
```

### Lock_new

Create a new `Lock`.

**Returns:** `Lock`

### Queue_new

Create a new shared `Queue`. (`maxsize` is currently ignored.)

**Parameters:** `maxsize: int`
**Returns:** `Queue`
