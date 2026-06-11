# contextlib

The `tt.contextlib` module provides context manager utilities for the `with`-statement. It offers helpers for suppressing exceptions, redirecting output, and ensuring resources are closed.

```titrate
import tt.contextlib.Contextlib;
```

## Contextlib

All methods are static.

- `suppress(block: function<void()>): void` — run a block, silently swallowing any thrown error
- `redirectStdout(block: function<void()>): void` — run a block with stdout redirection (VM runtime hook)
- `closing(resource: Object, block: function<void()>): void` — run a block, then call `resource.close()`; the programmatic equivalent of the `with`-statement

```titrate
// Suppress errors from a block
Contextlib::suppress(fn(): void => {
    // risky operation that might throw
});

// Ensure a resource is closed after use
Contextlib::closing(file, fn(): void => {
    // work with file
});
// file.close() is called automatically
```
