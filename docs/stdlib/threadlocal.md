# threadlocal

The `tt.concurrent` module provides `ThreadLocal<T>` — thread-local storage for per-thread isolated data.

```titrate
import tt.concurrent.ThreadLocal;
```

## ThreadLocal

A container that holds a separate copy of a value for each thread. Each thread accesses its own independent instance, avoiding the need for synchronization when accessing thread-specific data.

- `fn init()` — create a ThreadLocal with no initial value (returns null for unset threads)
- `fn init(initialValue: fn(): T)` — create a ThreadLocal with a factory function that produces the initial value for each thread
- `get(): T` — return the value for the current thread, creating it from the factory if necessary
- `set(value: T): void` — set the value for the current thread
- `remove(): void` — remove the value for the current thread

```titrate
let counter: ThreadLocal<int> = new ThreadLocal<int>(fn(): int {
    return 0;
});

// Each thread gets its own independent counter
counter.set(counter.get() + 1);
io::println(Integer.toString(counter.get())); // 1

// Remove the value for the current thread
counter.remove();
```
