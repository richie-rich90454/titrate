# thread

The `tt.concurrent` module provides low-level concurrency primitives including threads, mutexes, condition variables, semaphores, atomic types, and promises.

```titrate
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
```

## Thread

A thread of execution that runs a task concurrently.

- `fn init(task: fn(): void)` — create a thread with the given task function
- `start(): void` — start the thread
- `join(): void` — wait for the thread to complete
- `detach(): void` — detach the thread to run independently
- `getId(): int` — get the thread's identifier
- `isAlive(): bool` — check if the thread is still running
- `Thread.sleep(ms: int): void` — sleep the current thread for the given milliseconds
- `Thread.yield_(): void` — yield the current thread's time slice
- `Thread.currentId(): int` — get the current thread's identifier

```titrate
let t: Thread = new Thread(fn(): void {
    io::println("Running in thread " + Integer.toString(Thread.currentId()));
    Thread.sleep(100);
});
t.start();
t.join();
io::println("Thread finished");
```

## Mutex

A mutual exclusion lock for protecting shared data.

- `fn init()` — create a new mutex
- `lock(): void` — acquire the lock (blocks until available)
- `unlock(): void` — release the lock
- `tryLock(): bool` — try to acquire the lock without blocking; returns true if acquired

```titrate
let mutex: Mutex = new Mutex();
mutex.lock();
// critical section
mutex.unlock();

if (mutex.tryLock()) {
    // acquired
    mutex.unlock();
}
```

## LockGuard

RAII-style lock guard that automatically manages a mutex lock.

- `fn init(mutex: Mutex)` — acquire the mutex lock on construction
- `unlock(): void` — release the lock early
- `isLocked(): bool` — check if the guard still holds the lock

```titrate
let mutex: Mutex = new Mutex();
let guard: LockGuard = new LockGuard(mutex);
// lock is held automatically
// critical section
guard.unlock();  // release early if needed
```

## RecursiveMutex

A mutex that can be locked multiple times by the same thread without deadlocking.

- `fn init()` — create a new recursive mutex
- `lock(): void` — acquire the lock (reentrant; same thread can lock multiple times)
- `unlock(): void` — release one level of locking
- `tryLock(): bool` — try to acquire without blocking; returns true if acquired

```titrate
let rmutex: RecursiveMutex = new RecursiveMutex();
rmutex.lock();
rmutex.lock();  // same thread can lock again
rmutex.unlock();
rmutex.unlock();  // must unlock same number of times
```

## ConditionVariable

A condition variable for waiting on state changes between threads.

- `fn init()` — create a new condition variable
- `wait(mutex: Mutex): void` — wait for notification (releases mutex while waiting)
- `waitFor(mutex: Mutex, ms: int): bool` — wait with a timeout in milliseconds; returns true if notified, false on timeout
- `notifyOne(): void` — wake one waiting thread
- `notifyAll(): void` — wake all waiting threads

```titrate
let mutex: Mutex = new Mutex();
let cv: ConditionVariable = new ConditionVariable();

// waiting thread
mutex.lock();
cv.wait(mutex);
mutex.unlock();

// notifying thread
cv.notifyOne();
```

## Semaphore

A counting semaphore that maintains a number of permits.

- `fn init(permits: int)` — create a semaphore with the given initial permit count
- `acquire(): void` — acquire a permit (blocks until available)
- `release(): void` — release a permit
- `tryAcquire(): bool` — try to acquire a permit without blocking; returns true if acquired
- `availablePermits(): int` — get the number of currently available permits

```titrate
let sem: Semaphore = new Semaphore(3);
sem.acquire();
// use resource
sem.release();
io::println(Integer.toString(sem.availablePermits()));  // 3
```

## SharedMutex

A reader-writer lock that allows concurrent reads or exclusive writes.

- `fn init()` — create a new shared mutex
- `sharedLock(): void` — acquire a shared (read) lock
- `sharedUnlock(): void` — release a shared (read) lock
- `uniqueLock(): void` — acquire a unique (write) lock
- `uniqueUnlock(): void` — release a unique (write) lock
- `trySharedLock(): bool` — try to acquire a shared lock without blocking
- `tryUniqueLock(): bool` — try to acquire a unique lock without blocking

```titrate
let rw: SharedMutex = new SharedMutex();

// multiple readers
rw.sharedLock();
// read data
rw.sharedUnlock();

// exclusive writer
rw.uniqueLock();
// write data
rw.uniqueUnlock();
```

## OnceFlag

A flag ensuring a function is executed exactly once across all threads.

- `fn init()` — create a new once flag
- `callOnce(fn: fn(): void): void` — execute the function only once; subsequent calls are no-ops
- `isCalled(): bool` — check if the function has been called

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

An atomic integer providing lock-free thread-safe operations.

- `fn init(value: int)` — create an atomic integer with the given initial value
- `get(): int` — read the current value
- `set(value: int): void` — write a new value
- `fetchAdd(delta: int): int` — add delta and return the previous value
- `fetchSub(delta: int): int` — subtract delta and return the previous value
- `compareAndSwap(expected: int, newValue: int): bool` — atomically set to newValue if current equals expected; returns true if swapped
- `incrementAndGet(): int` — increment and return the new value
- `decrementAndGet(): int` — decrement and return the new value

```titrate
let counter: AtomicInt = new AtomicInt(0);
let old: int = counter.fetchAdd(1);
io::println(Integer.toString(old));              // 0
io::println(Integer.toString(counter.get()));    // 1
let swapped: bool = counter.compareAndSwap(1, 10);
io::println(Boolean.toString(swapped));          // true
io::println(Integer.toString(counter.get()));    // 10
```

## AtomicBool

An atomic boolean providing lock-free thread-safe operations.

- `fn init(value: bool)` — create an atomic boolean with the given initial value
- `get(): bool` — read the current value
- `set(value: bool): void` — write a new value
- `compareAndSwap(expected: bool, newValue: bool): bool` — atomically set to newValue if current equals expected; returns true if swapped

```titrate
let flag: AtomicBool = new AtomicBool(false);
let swapped: bool = flag.compareAndSwap(false, true);
io::println(Boolean.toString(swapped));  // true
io::println(Boolean.toString(flag.get()));  // true
```

## Promise

A promise that can be resolved with a value or an error, producing an associated future.

- `fn init()` — create a new unresolved promise
- `complete(value: T): void` — resolve the promise with a value
- `completeExceptionally(err: string): void` — resolve the promise with an error
- `future(): Future<T>` — get the future associated with this promise
- `isDone(): bool` — check if the promise has been resolved

```titrate
let p: Promise<int> = new Promise<int>();
let f: Future<int> = p.future();
p.complete(42);
io::println(Boolean.toString(p.isDone()));  // true
io::println(Integer.toString(f.get()));     // 42
```

## jthread and thread::id (C++ `<thread>` parity, Phase 1-2)

The `jthread` (joining thread) automatically joins on destruction and supports cooperative cancellation through a `StopToken`. `thread::id` is the type used to identify threads, comparable with `==`.

### jthread

- `JThread(task: fn(StopToken): void)` — create a joining thread whose task receives a `StopToken`
- `JThread(task: fn(): void)` — create a joining thread with no stop token
- `join(): void` — wait for the thread to finish (also happens automatically on destruction)
- `detach(): void` — detach (cancellation auto-stop will not run after detach)
- `requestStop(): bool` — request cooperative cancellation; returns true if a stop was requested
- `getStopToken(): StopToken` — retrieve the stop token associated with this thread
- `isJoinable(): bool` — check if the thread can be joined

```titrate
import tt.concurrent.JThread;
import tt.concurrent.StopToken;

let t: JThread = new JThread(fn(st: StopToken): void {
    while (!st.stopRequested()) {
        io::println("working...");
        Thread.sleep(50);
    }
    io::println("stopped");
});
t.requestStop();
t.join();  // also happens automatically when t goes out of scope
```

### thread::id

- `ThreadId` — opaque identifier for a thread, comparable with `==` and `!=`
- `Thread.getId(): ThreadId` — return the identifier of this thread
- `Thread.currentThreadId(): ThreadId` — return the identifier of the calling thread
- `ThreadId.toString(): string` — human-readable form

```titrate
let t: Thread = new Thread(fn(): void { Thread.sleep(10); });
t.start();
let id: ThreadId = t.getId();
io::println(id.toString());
```

### yield / hardware_concurrency / sleep_for / sleep_until

These free functions and static helpers round out C++ `<thread>` parity.

- `Thread.yield(): void` — hint to the scheduler that the current thread should yield its time slice
- `Thread.hardwareConcurrency(): int` — number of hardware threads available (best-effort; returns 0 if not detectable)
- `sleepFor(durationMs: int): void` — sleep the current thread for the given duration in milliseconds (`std::this_thread::sleep_for`)
- `sleepUntil(epochMs: long): void` — sleep the current thread until the given epoch millisecond timestamp (`std::this_thread::sleep_until`)

```titrate
Thread.yield();
let cpus: int = Thread.hardwareConcurrency();
io::println(Integer.toString(cpus));

sleepFor(250);  // sleep 250 ms
```
