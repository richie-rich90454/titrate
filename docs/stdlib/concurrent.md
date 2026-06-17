# concurrent

The `tt.concurrent` module provides asynchronous programming primitives with futures, channels, threads, synchronization constructs, and lock-free data structures.

```titrate
import tt.concurrent.Future;
import tt.concurrent.Channel;
import tt.concurrent.Thread;
import tt.concurrent.Mutex;
import tt.concurrent.LockGuard;
import tt.concurrent.RecursiveMutex;
import tt.concurrent.ConditionVariable;
import tt.concurrent.Semaphore;
import tt.concurrent.SharedMutex;
import tt.concurrent.OnceFlag;
import tt.concurrent.AtomicInt;
import tt.concurrent.AtomicBool;
import tt.concurrent.Promise;
import tt.concurrent.LockFreeQueue;
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
- `valid(): bool` — check if the future holds a shared state
- `waitFor(timeoutMs: int): bool` — wait for completion up to timeout milliseconds; returns true if done
- `sharedFuture(): SharedFuture<T>` — convert to a shared future that can be read multiple times

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

## Thread

A platform thread for concurrent execution.

- `fn init(runnable: fn(): void)` — create a thread with a function to run
- `start(): void` — start the thread
- `join(): void` — wait for the thread to finish
- `join(timeoutMs: int): bool` — wait up to timeout milliseconds; returns true if finished
- `isAlive(): bool` — check if the thread is still running
- `detach(): void` — detach the thread from management
- `interrupt(): void` — request thread interruption
- `isInterrupted(): bool` — check if interrupted
- `static Thread.currentThread(): Thread` — get the current thread
- `static Thread.sleep(ms: int): void` — sleep for milliseconds
- `static Thread.yield(): void` — hint to scheduler to yield

```titrate
let t: Thread = new Thread(fn(): void {
    io::println("running in thread");
    Thread.sleep(100);
    io::println("done");
});
t.start();
t.join();
io::println(Boolean.toString(t.isAlive()));  // false
```

## Mutex

A mutual exclusion lock for protecting shared state.

- `fn init()` — create an unlocked mutex
- `lock(): void` — acquire the lock (blocks until available)
- `tryLock(): bool` — try to acquire without blocking; returns true on success
- `unlock(): void` — release the lock

```titrate
let m: Mutex = new Mutex();
m.lock();
// critical section
m.unlock();
```

## LockGuard

An RAII-style lock holder that automatically releases a mutex on scope exit.

- `fn init(mutex: Mutex)` — create guard and lock the mutex
- `release(): void` — manually release the lock early

```titrate
let m: Mutex = new Mutex();
let guard: LockGuard = new LockGuard(m);
// critical section — lock released when guard goes out of scope
```

## RecursiveMutex

A mutex that can be re-locked by the same thread without deadlocking.

- `fn init()` — create an unlocked recursive mutex
- `lock(): void` — acquire the lock (re-entrant)
- `tryLock(): bool` — try to acquire without blocking
- `unlock(): void` — release one level of locking

```titrate
let rm: RecursiveMutex = new RecursiveMutex();
rm.lock();
rm.lock();  // same thread — no deadlock
rm.unlock();
rm.unlock();
```

## ConditionVariable

A condition variable for waiting on a predicate protected by a mutex.

- `fn init()` — create a condition variable
- `wait(mutex: Mutex): void` — release mutex and wait for notification
- `waitFor(mutex: Mutex, timeoutMs: int): bool` — wait with timeout; returns true if notified
- `notifyOne(): void` — wake one waiting thread
- `notifyAll(): void` — wake all waiting threads

```titrate
let m: Mutex = new Mutex();
let cv: ConditionVariable = new ConditionVariable();
m.lock();
cv.wait(m);
// re-acquired after notification
m.unlock();
```

## Semaphore

A counting semaphore for controlling access to a shared resource pool.

- `fn init(permits: int)` — create with the given number of permits
- `acquire(): void` — take a permit (blocks if none available)
- `tryAcquire(): bool` — try to take a permit without blocking
- `release(): void` — return a permit
- `availablePermits(): int` — number of currently available permits

```titrate
let sem: Semaphore = new Semaphore(3);
sem.acquire();
sem.acquire();
io::println(Integer.toString(sem.availablePermits()));  // 1
sem.release();
```

## SharedMutex

A read-write lock allowing concurrent readers or a single writer.

- `fn init()` — create an unlocked shared mutex
- `lockShared(): void` — acquire a shared (read) lock
- `tryLockShared(): bool` — try to acquire a shared lock
- `unlockShared(): void` — release a shared lock
- `lock(): void` — acquire an exclusive (write) lock
- `tryLock(): bool` — try to acquire an exclusive lock
- `unlock(): void` — release an exclusive lock

```titrate
let rw: SharedMutex = new SharedMutex();
rw.lockShared();
// multiple readers can hold shared locks simultaneously
rw.unlockShared();
rw.lock();
// exclusive — no other readers or writers
rw.unlock();
```

## OnceFlag

A flag ensuring a function is executed exactly once, even across threads.

- `fn init()` — create an unset flag
- `callOnce(fn: fn(): void): void` — execute the function only on the first call

```titrate
let flag: OnceFlag = new OnceFlag();
flag.callOnce(fn(): void {
    io::println("This runs only once");
});
flag.callOnce(fn(): void {
    io::println("This will not run");
});
```

## AtomicInt

An integer value with atomic read-modify-write operations.

- `fn init(value: int)` — create with initial value
- `get(): int` — atomically read
- `set(value: int): void` — atomically write
- `getAndSet(newValue: int): int` — atomically set and return old value
- `compareAndSet(expected: int, newValue: int): bool` — CAS; returns true if swapped
- `getAndIncrement(): int` — atomically increment and return old value
- `getAndDecrement(): int` — atomically decrement and return old value
- `getAndAdd(delta: int): int` — atomically add and return old value
- `addAndGet(delta: int): int` — atomically add and return new value
- `incrementAndGet(): int` — atomically increment and return new value
- `decrementAndGet(): int` — atomically decrement and return new value

```titrate
let counter: AtomicInt = new AtomicInt(0);
counter.incrementAndGet();
counter.addAndGet(5);
io::println(Integer.toString(counter.get()));  // 6

let old: int = counter.getAndSet(0);
io::println(Integer.toString(old));  // 6
```

## AtomicBool

A boolean value with atomic operations.

- `fn init(value: bool)` — create with initial value
- `get(): bool` — atomically read
- `set(value: bool): void` — atomically write
- `getAndSet(newValue: bool): bool` — atomically set and return old value
- `compareAndSet(expected: bool, newValue: bool): bool` — CAS; returns true if swapped

```titrate
let flag: AtomicBool = new AtomicBool(false);
let was: bool = flag.getAndSet(true);
io::println(Boolean.toString(was));  // false
io::println(Boolean.toString(flag.get()));  // true
```

## Promise

A writable single-assignment container that feeds a `Future`.

- `fn init()` — create a promise with an associated unresolved future
- `future(): Future<T>` — get the associated future
- `set(value: T): void` — resolve the promise with a value
- `setError(err: string): void` — resolve the promise with an error

```titrate
let p: Promise<int> = new Promise<int>();
let f: Future<int> = p.future();
p.set(42);
io::println(Integer.toString(f.get()));  // 42
```

## SPSCRingBuffer

Single-Producer Single-Consumer ring buffer. Optimal for pipelines where exactly one thread produces and one consumes. Capacity is rounded up to the next power of 2.

- `fn init(capacity: int)` — create with given capacity (rounded to power of 2)
- `push(value: Variant): bool` — try to push; returns false if full
- `pop(): Variant` — try to pop; returns null if empty
- `isEmpty(): bool` — check if empty
- `isFull(): bool` — check if full
- `size(): int` — current number of elements

```titrate
let rb: SPSCRingBuffer = new SPSCRingBuffer(64);
rb.push(42);
rb.push("hello");
let val: Variant = rb.pop();  // 42
let full: bool = rb.isFull(); // false
```

## MPSCRingBuffer

Multi-Producer Single-Consumer ring buffer. Multiple producers can push, but only one consumer pops. Uses CAS-like logic for slot reservation.

- `fn init(capacity: int)` — create with given capacity (rounded to power of 2)
- `push(value: Variant): bool` — try to push; returns false if full
- `pop(): Variant` — try to pop; returns null if empty
- `isEmpty(): bool` — check if empty
- `isFull(): bool` — check if full
- `size(): int` — current number of elements

```titrate
let rb: MPSCRingBuffer = new MPSCRingBuffer(128);
rb.push("message1");
rb.push("message2");
let msg: Variant = rb.pop();  // "message1"
```

## LockFreeStack

Lock-free stack (Treiber stack) using an array-backed approach. Models the Treiber CAS-based push/pop algorithm.

- `fn init()` — create an empty stack
- `push(value: Variant): void` — push a value onto the stack
- `pop(): Variant` — pop the top value; returns null if empty
- `peek(): Variant` — peek at the top value without removing; returns null if empty
- `isEmpty(): bool` — check if empty
- `size(): int` — current number of elements

```titrate
let stack: LockFreeStack = new LockFreeStack();
stack.push(10);
stack.push(20);
stack.push(30);
let top: Variant = stack.peek();  // 30
let val: Variant = stack.pop();   // 30
let remaining: int = stack.size(); // 2
```

**Factory functions:**

- `spscRingBuffer(capacity: int): SPSCRingBuffer` — create an SPSC ring buffer
- `mpscRingBuffer(capacity: int): MPSCRingBuffer` — create an MPSC ring buffer
- `lockFreeStack(): LockFreeStack` — create a lock-free stack

## Actor Model

- `Actor.init(handler: fn(string): void)` — create actor with message handler
- `Actor.tell(message: string): void` — send message to actor
- `Actor.ask(message: string): string` — send and wait for response
- `ActorSystem.create(name: string): ActorRef` — create actor in system
- `ActorSystem.shutdown(): void` — shutdown all actors

## CSP Channels

- `Channel.init(capacity: int)` — buffered channel
- `Channel.send(value: Variant): void` — send value (blocks if full)
- `Channel.receive(): Variant` — receive value (blocks if empty)
- `Channel.trySend(value: Variant): bool` — non-blocking send
- `Channel.tryReceive(): Variant` — non-blocking receive
- `Channel.select(channels: ArrayList<Channel>): (int, Variant)` — select from multiple channels

## Rate Limiter

- `RateLimiter.tokenBucket(rate: double, capacity: int): RateLimiter` — token bucket limiter
- `RateLimiter.leakyBucket(rate: double, capacity: int): RateLimiter` — leaky bucket limiter
- `RateLimiter.slidingWindow(limit: int, windowMs: int): RateLimiter` — sliding window limiter
- `RateLimiter.acquire(): bool` — try to acquire a permit
- `RateLimiter.wait(): void` — wait until permit available

## Circuit Breaker

- `CircuitBreaker.init(failureThreshold: int, recoveryTimeoutMs: int)` — create circuit breaker
- `CircuitBreaker.execute(fn(): Variant): Variant` — execute with circuit breaker protection
- `CircuitBreaker.getState(): string` — current state (closed/open/half-open)
- `CircuitBreaker.getFailureCount(): int` — current failure count
- `CircuitBreaker.reset(): void` — manually reset to closed state

## Retry with Backoff

- `Retry.exponentialBackoff(fn(): Variant, maxAttempts: int, baseDelayMs: int, maxDelayMs: int): Variant` — retry with exponential backoff
- `Retry.withJitter(fn(): Variant, maxAttempts: int, baseDelayMs: int): Variant` — retry with jitter
- `Retry.onErrors(fn(): Variant, maxAttempts: int, errorTypes: ArrayList<string>): Variant` — retry on specific errors
- `Retry.withBudget(fn(): Variant, budgetMs: int, maxAttempts: int): Variant` — retry within time budget
