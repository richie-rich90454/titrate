# StopToken

The `tt::concurrent::StopToken` module provides C++ `<stop_token>` parity. It implements cooperative cancellation primitives: a `StopSource` owns the stop state, a `StopToken` is a shared view of that state, and a `StopCallback` registers a callback invoked when stop is requested. These primitives underpin `jthread`-style auto-cancellation.

## Import

```titrate
import tt::concurrent::StopToken;
```

## API Reference

### `StopState`

The shared state owned by a `StopSource` and observed by `StopToken`s and `StopCallback`s. Tracks the stop-requested flag and the list of registered callbacks.

**Constructor:**
- `init()`

**Methods:**
- `stopRequested(): bool` — returns true if stop has been requested
- `requestStop(): bool` — requests stop; if stop was not already requested, fires all registered callbacks. Returns true if this call actually transitioned the state to stopped.
- `register(callback: fn(): void): int` — registers a callback invoked when stop is requested. If stop is already requested, the callback fires immediately on the calling thread and `-1` is returned. Otherwise returns a registration id for deregistration.
- `deregister(id: int): bool` — deregisters the callback with the given id. Returns true if the callback was found and removed before being invoked.
- `callbackCount(): int` — returns the number of currently-registered callbacks

### `StopSource`

Owns a `StopState` and exposes `requestStop()` and `getToken()` (`std::stop_source`).

**Constructors:**
- `init()` — constructs a `StopSource` with a fresh stop state
- `initEmpty()` — constructs a `StopSource` with no associated state (`valid()` returns false)

**Methods:**
- `requestStop(): bool` — requests stop. Returns true if this source has a state and stop was not previously requested.
- `stopRequested(): bool` — returns true if stop has been requested
- `valid(): bool` — returns true if this `StopSource` owns a state
- `getToken(): StopToken` — returns a `StopToken` that observes this source's state
- `state(): StopState` — returns the underlying state (for `jthread`-style sharing)

### `StopToken`

A shared, copyable view of a `StopState` (`std::stop_token`).

**Constructor:**
- `init(state: StopState)`

**Methods:**
- `stopPossible(): bool` — returns true if this token has an associated state
- `stopRequested(): bool` — returns true if stop has been requested
- `register(callback: fn(): void): StopCallback` — registers a callback. Returns a `StopCallback` handle whose `deregister()` method removes the callback. If stop was already requested, the callback fires immediately and the returned `StopCallback` is a no-op.
- `state(): StopState` — returns the underlying state (may be null)
- `equals(other: StopToken): bool` — returns true if this token and `other` share the same state

### `StopCallback`

A handle returned by `StopToken.register()` that can be used to deregister the callback before it fires (`std::stop_callback`).

**Constructor:**
- `init(state: StopState, id: int)`

**Methods:**
- `deregister(): bool` — deregisters the callback. Returns true if the callback was successfully removed before being invoked.
- `registered(): bool` — returns true if this callback is still registered

### Free Functions

#### `stopSource(): StopSource`

Returns a new `StopSource` with a fresh stop state.

#### `stopToken(): StopToken`

Returns an empty `StopToken` with no associated state.

#### `stopRequested(token: StopToken): bool`

Returns true if the given token has been requested to stop. Equivalent to `token.stopRequested()` but usable as a free function for thread-pool dispatch loops.

#### `stopPossible(token: StopToken): bool`

Returns true if the given token might ever have stop requested (i.e., it has an associated state).

## Usage Examples

### Basic Cooperative Cancellation

```titrate
import tt::concurrent::StopToken;
import tt::io::IO;

public fn main(): void {
    let source: StopSource = StopToken.stopSource();
    let token: StopToken = source.getToken();

    IO.println("stop requested: " + token.stopRequested());  // false
    source.requestStop();
    IO.println("stop requested: " + token.stopRequested());  // true
}
```

### Registering a Stop Callback

```titrate
import tt::concurrent::StopToken;

let source: StopSource = StopToken.stopSource();
let token: StopToken = source.getToken();

// Register a callback that fires when stop is requested.
token.register(fn(): void {
    io::println("stop was requested — cleaning up");
});

// Later, request stop — the callback fires on the calling thread.
source.requestStop();
```

### Deregistering a Callback Before It Fires

```titrate
import tt::concurrent::StopToken;

let source: StopSource = StopToken.stopSource();
let token: StopToken = source.getToken();

let cb: StopCallback = token.register(fn(): void {
    io::println("this will not run");
});

// Cancel the callback before stop is requested.
if (cb.registered()) {
    cb.deregister();
}

source.requestStop();  // callback does NOT fire
```

### Polling a Stop Token in a Worker Loop

```titrate
import tt::concurrent::StopToken;
import tt::concurrent::Thread;

public fn worker(token: StopToken): void {
    while (!StopToken.stopRequested(token)) {
        // do a unit of work
        Thread.sleep(10);
    }
    io::println("worker exiting");
}

public fn main(): void {
    let source: StopSource = StopToken.stopSource();
    let t: Thread = new Thread(fn(): void => worker(source.getToken()));
    t.start();
    Thread.sleep(100);
    source.requestStop();
    t.join();
}
```
