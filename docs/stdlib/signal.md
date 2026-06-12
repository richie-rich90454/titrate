# signal

The `tt.sys` module provides signal handling for responding to operating system signals.

```titrate
import tt.sys.Signal;
```

## Signal

Utilities for registering handlers, raising, and managing OS signals.

**Constants:**

| Constant | Value | Description |
|----------|-------|-------------|
| `Signal.SIGINT` | 2 | Interrupt (Ctrl+C) |
| `Signal.SIGTERM` | 15 | Termination |
| `Signal.SIGHUP` | 1 | Hangup |
| `Signal.SIGUSR1` | 10 | User-defined 1 |
| `Signal.SIGUSR2` | 12 | User-defined 2 |
| `Signal.SIGKILL` | 9 | Kill (cannot be caught) |
| `Signal.SIGPIPE` | 13 | Broken pipe |
| `Signal.SIGALRM` | 14 | Alarm clock |

**Methods:**

- `Signal.register(signum: int, handler: fn(): void): void` — register a handler function for the given signal
- `Signal.raise(signum: int): void` — raise (send) a signal to the current process
- `Signal.defaultHandler(signum: int): void` — reset the signal handler to the system default
- `Signal.ignore(signum: int): void` — ignore the given signal

```titrate
Signal.register(Signal.SIGINT, fn(): void {
    io::println("Caught SIGINT, shutting down gracefully...");
});

Signal.register(Signal.SIGTERM, fn(): void {
    io::println("Received SIGTERM");
});

// Raise a signal programmatically
Signal.raise(Signal.SIGUSR1);

// Reset to default behavior
Signal.defaultHandler(Signal.SIGINT);

// Ignore a signal
Signal.ignore(Signal.SIGPIPE);
```
