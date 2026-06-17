# threadpool

The `tt.concurrent` module provides `ThreadPoolExecutor` — a thread pool for concurrent task execution.

```titrate
import tt.concurrent.ThreadPoolExecutor;
```

## ThreadPoolExecutor

A fixed-size thread pool that manages a pool of worker threads for executing tasks concurrently. Submitted tasks are queued and dispatched to available threads.

- `fn init(maxThreads: int)` — create a thread pool with the specified maximum number of worker threads
- `submit(task: fn(): void): void` — submit a runnable task for execution
- `submitCallable(task: fn(): Variant): Variant` — submit a callable task and return its result
- `shutdown(): void` — initiate an orderly shutdown; previously submitted tasks will still be executed
- `isShutdown(): bool` — check if the pool has been shut down
- `getActiveCount(): int` — return the approximate number of actively executing threads
- `getQueueSize(): int` — return the number of tasks waiting in the queue

```titrate
let pool: ThreadPoolExecutor = new ThreadPoolExecutor(4);

// Submit fire-and-forget tasks
pool.submit(fn(): void {
    io::println("Task 1 running");
});

pool.submit(fn(): void {
    io::println("Task 2 running");
});

io::println(Integer.toString(pool.getActiveCount())); // ~2
io::println(Integer.toString(pool.getQueueSize()));    // 0

pool.shutdown();
io::println(Boolean.toString(pool.isShutdown())); // true
```

## Work-Stealing Pool

- `ThreadPool.workStealingPool(parallelism: int): ThreadPool` — create work-stealing pool
- `ThreadPool.getScheduledExecutor(): ScheduledExecutor` — get scheduled executor

## Scheduled Executor

- `ScheduledExecutor.schedule(task: fn(): void, delayMs: int): ScheduledFuture` — schedule one-time task
- `ScheduledExecutor.scheduleAtFixedRate(task: fn(): void, initialDelayMs: int, periodMs: int): ScheduledFuture` — schedule periodic task
- `ScheduledExecutor.scheduleWithFixedDelay(task: fn(): void, initialDelayMs: int, delayMs: int): ScheduledFuture` — schedule with fixed delay

## Future Chaining

- `Future.thenApply(f: fn(Variant): Variant): Future` — transform result
- `Future.thenCompose(f: fn(Variant): Future): Future` — chain futures
- `Future.thenCombine(other: Future, f: fn(Variant, Variant): Variant): Future` — combine two futures
- `Future.exceptionally(f: fn(Variant): Variant): Future` — handle exception

## Rejection Policies

- `ThreadPool.abortPolicy(): RejectionPolicy` — reject with exception
- `ThreadPool.callerRunsPolicy(): RejectionPolicy` — run in caller thread
- `ThreadPool.discardPolicy(): RejectionPolicy` — silently discard
- `ThreadPool.discardOldestPolicy(): RejectionPolicy` — discard oldest task
