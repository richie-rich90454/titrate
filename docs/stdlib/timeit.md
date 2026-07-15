# Timeit

The `tt.time.Stopwatch` module provides elapsed-time measurement and benchmarking utilities. It mirrors Python's `timeit` module via the `Timer` class (which measures the execution time of a small code snippet supplied as a function, since Titrate has no `eval()`), plus the `Stopwatch` class for general monotonic-clock timing. Module-level convenience functions `timeit`, `repeat`, and `defaultTimer` mirror the Python top-level API.

## Import

```titrate
import tt::time::Stopwatch;
```

## Classes

### Stopwatch

Measures elapsed time using a monotonic clock. Supports start/stop accumulation, reset, and restart.

**Constructors:**
- `init()` — creates a stopped stopwatch with zero elapsed time

**Methods:**
- `start(): Stopwatch` — start (or resume) timing; returns `this` for chaining
- `stop(): Stopwatch` — stop timing and accumulate elapsed nanoseconds; returns `this`
- `reset(): Stopwatch` — reset elapsed time to zero and stop; returns `this`
- `restart(): Stopwatch` — reset and start in one call; returns `this`
- `elapsed(): Duration` — return the total elapsed time as a `Duration`
- `elapsedNanos(): long` — return the total elapsed time in nanoseconds
- `elapsedMillis(): double` — return the total elapsed time in milliseconds
- `elapsedSeconds(): double` — return the total elapsed time in seconds
- `isRunning(): bool` — true if the stopwatch is currently running
- `toString(): string` — human-readable form (e.g. `"42 ns"`, `"3.14 ms"`, `"1.5 s"`)

```titrate
let sw: Stopwatch = new Stopwatch();
sw.start();
// ... do work ...
sw.stop();
io::println(sw.toString());
```

### Timer

Measures the execution time of a small code snippet supplied as a function.

**Fields:**
- `stmt: fn(): void` — the statement to time
- `setup: fn(): void` — setup function run once before timing (may be `null`)
- `timerName: string` — name of the timer (default `"monotonic"`)

**Constructors:**
- `init(stmt: fn(): void)` — create a timer with no setup
- `initWithSetup(stmt: fn(): void, setup: fn(): void)` — create a timer with a setup function

**Methods:**
- `timeit(number: int): double` — run `setup` once, then time `number` executions of `stmt`; returns total elapsed seconds. `number < 1` is treated as 1.
- `repeat(repeat: int, number: int): ArrayList<double>` — run `timeit(number)` `repeat` times, returning the per-trial totals
- `autorange(): ArrayList<double>` — automatically choose `number` so that total time >= 0.2s, then run 5 trials of that number

```titrate
let t: Timer = new Timer(fn(): void => {
    let s: string = "x";
    var i: int = 0;
    while (i < 100) {
        s = s + "y";
        i = i + 1;
    }
});
let seconds: double = t.timeit(1000);
io::println(Double.toString(seconds) + " s for 1000 runs");
```

## Functions

### timeit

- `Stopwatch.timeit(stmt: fn(): void, number: int): double` — time `number` runs of `stmt` and return elapsed seconds. Mirrors `timeit.timeit(stmt, number)`.

### repeat

- `Stopwatch.repeat(stmt: fn(): void, repeat: int, number: int): ArrayList<double>` — run `repeat` trials of `number` runs each. Mirrors `timeit.repeat(stmt, repeat, number)`.

### defaultTimer

- `Stopwatch.defaultTimer(stmt: fn(): void): double` — run `stmt` once and return elapsed seconds. Mirrors `timeit.default_timer`.

## Usage Example

```titrate
import tt::time::Stopwatch;

public fn main(): void {
    let seconds: double = Stopwatch.timeit(fn(): void => {
        var i: int = 0;
        while (i < 1000) {
            i = i + 1;
        }
    }, 10000);
    io::println("10000 iterations took " + Double.toString(seconds) + " s");
    let results: ArrayList<double> = Stopwatch.repeat(fn(): void => {
        Math.sqrt(2.0);
    }, 5, 1000);
    io::println("Best of 5: " + Double.toString(results.get(0)));
}
```
