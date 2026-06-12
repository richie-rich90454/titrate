# scheduler

The `tt.time` module provides an event scheduler for scheduling and running delayed actions.

```titrate
import tt.time.Scheduler;
```

## Scheduler

A general-purpose event scheduler that runs tasks after specified delays, with optional priority ordering.

- `fn init()` — create a new scheduler
- `enter(delay: double, action: fn(): void): int` — schedule an event after `delay` seconds; returns the event ID
- `enter(delay: double, priority: int, action: fn(): void): int` — schedule an event with a priority (lower numbers run first); returns the event ID
- `cancel(eventId: int): bool` — cancel a scheduled event; returns true if the event was found and cancelled
- `run(): void` — run all scheduled events in order
- `run(blocking: bool): void` — run events; if `blocking` is true, wait for all events to complete
- `empty(): bool` — check if there are no pending events
- `queueSize(): int` — get the number of pending events

```titrate
let sched: Scheduler = new Scheduler();

let id1: int = sched.enter(1.0, fn(): void {
    io::println("First event (1s delay)");
});

let id2: int = sched.enter(2.0, fn(): void {
    io::println("Second event (2s delay)");
});

let id3: int = sched.enter(0.5, 1, fn(): void {
    io::println("High priority event (0.5s delay)");
});

sched.cancel(id2);  // cancel the second event
io::println(Boolean.toString(sched.empty()));     // false
io::println(Integer.toString(sched.queueSize()));  // 2

sched.run();  // executes remaining events in order
```
