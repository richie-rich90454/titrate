# selectors

The `tt.sys.Selectors` module provides I/O multiplexing via the `Selector` class. Because true OS-level `select`/`poll`/`epoll` requires VM-level socket-handle integration, this implementation uses a polling-based approach: `select` returns all registered handles and respects the timeout by sleeping.

```titrate
import tt.sys.Selectors;
```

## Constants

- `EVENT_READ: int` — `1`, event mask for read readiness
- `EVENT_WRITE: int` — `2`, event mask for write readiness

## Selector

Monitors a set of integer I/O handles for readiness events.

**Methods:**

- `fn init()` — create a new selector with no registered handles
- `fn register(handle: int, events: int): void` — register a handle for the given event mask
- `fn unregister(handle: int): void` — remove a handle from the selector
- `fn modify(handle: int, events: int): void` — change the event mask of an already-registered handle
- `fn isRegistered(handle: int): bool` — return true if the handle is registered
- `fn select(timeout: int): ArrayList<int>` — return all registered handles; `timeout` is in milliseconds (0 = non-blocking, -1 = wait indefinitely)
- `fn selectWithEvents(timeout: int): ArrayList<int>` — select ready handles and return them with their event mask
- `fn close(): void` — close the selector and release resources
- `fn isClosed(): bool` — return true if the selector has been closed
- `fn size(): int` — return the number of registered handles
- `fn getEvents(handle: int): int` — return the event mask for a registered handle, or 0 if not registered

## Top-level Functions

- `fn createSelector(): Selector` — create and return a new `Selector` instance

```titrate
import tt.sys.Selectors;

let sel = Selectors.createSelector();
sel.register(0, Selectors.EVENT_READ);
let ready = sel.select(100);
io::println(Integer.toString(sel.size())); // 1
sel.close();
```
