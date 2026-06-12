# event

The `tt.concurrent` module provides `Event` — a thread synchronization primitive that allows threads to wait for a signal.

```titrate
import tt.concurrent.Event;
```

## Event

A synchronization object that threads can wait on. One thread sets the event to signal one or more waiting threads to proceed. Events can be used for one-time or repeated signaling patterns.

- `fn init()` — create a new event in the unset (clear) state
- `set(): void` — set the event, releasing all waiting threads
- `clear(): void` — reset the event to the unset state
- `wait(): void` — block the current thread until the event is set
- `waitFor(timeoutMs: int): bool` — block until the event is set or the timeout expires; returns `true` if the event was set, `false` on timeout
- `isSet(): bool` — check if the event is currently set

```titrate
let ready: Event = new Event();

// In one thread:
ready.set();

// In another thread:
ready.wait();
io::println("Proceeding after event was set");

// With timeout
let done: Event = new Event();
let signaled: bool = done.waitFor(5000);
if (signaled) {
    io::println("Event was signaled");
} else {
    io::println("Timed out waiting for event");
}
```
