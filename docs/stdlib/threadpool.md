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
