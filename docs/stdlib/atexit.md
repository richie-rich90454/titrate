# atexit

The `tt.sys.Atexit` module provides exit-handler registration. Functions registered with `register` are invoked when `runExitHooks` is called, typically at program termination. Handlers run in LIFO order (last registered runs first).

```titrate
import tt.sys.Atexit;
```

## Top-level Functions

- `fn register(handler: fn(): void): void` — register a function to be called on exit
- `fn unregister(handler: fn(): void): void` — remove a previously registered exit handler
- `fn runExitHooks(): void` — run all registered handlers in reverse order (LIFO), then clear the list
- `fn handlerCount(): int` — return the number of registered handlers
- `fn clear(): void` — remove all registered handlers without running them

```titrate
import tt.sys.Atexit;

Atexit.register(fn(): void => io::println("cleaning up"));
Atexit.register(fn(): void => io::println("goodbye"));

io::println(Integer.toString(Atexit.handlerCount())); // 2

Atexit.runExitHooks();
// goodbye
// cleaning up
```
