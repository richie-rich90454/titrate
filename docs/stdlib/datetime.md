# datetime

The `tt.time` module provides date/time representation, duration calculations, and stopwatch utilities.

```titrate
import tt.time.DateTime;
import tt.time.Duration;
import tt.time.Time;
import tt.time.Stopwatch;
```

## DateTime

Represents a point in time as milliseconds since the Unix epoch.

- `fn init(ms: long)` — create from epoch millis
- `DateTime.now(): DateTime` — current date and time
- `DateTime.ofEpochMillis(ms: long): DateTime` — create from epoch millis
- `DateTime.fromISO(s: string): DateTime` — parse ISO 8601 string
- `getYear(): int`, `getMonth(): int`, `getDay(): int` — date components
- `getHour(): int`, `getMinute(): int`, `getSecond(): int` — time components
- `dayOfWeek(): int` — day of week (1=Monday, 7=Sunday)
- `dayOfYear(): int` — day of year (1-366)
- `isLeapYear(): bool` — whether the year is a leap year
- `daysInMonth(): int` — number of days in the current month
- `isBefore(other: DateTime): bool` — chronological comparison
- `isAfter(other: DateTime): bool` — chronological comparison
- `plus(d: Duration): DateTime` — add a duration
- `minus(d: Duration): DateTime` — subtract a duration
- `plusDays(n: long): DateTime` — add days
- `minusDays(n: long): DateTime` — subtract days
- `plusMonths(n: int): DateTime` — add months
- `plusYears(n: int): DateTime` — add years
- `withYear(y: int): DateTime` — copy with different year
- `withMonth(m: int): DateTime` — copy with different month
- `withDay(d: int): DateTime` — copy with different day
- `withHour(h: int): DateTime` — copy with different hour
- `withMinute(m: int): DateTime` — copy with different minute
- `withSecond(s: int): DateTime` — copy with different second
- `format(fmt: string): string` — format using strftime-like pattern
- `toISO(): string` — ISO 8601 representation
- `toString(): string` — default format `%Y-%m-%d %H:%M:%S`

```titrate
let now = DateTime.now();
io::println(now.toString());  // "2025-06-15 14:30:00"
io::println(now.toISO());     // "2025-06-15T14:30:00"
let tomorrow = now.plusDays(1);
```

## Duration

Represents a length of time in milliseconds.

- `fn init(ms: long)` — create from milliseconds
- `Duration.ofMillis(ms: long): Duration` — from milliseconds
- `Duration.ofSeconds(s: long): Duration` — from seconds
- `Duration.ofMinutes(m: long): Duration` — from minutes
- `Duration.ofHours(h: long): Duration` — from hours
- `Duration.ofDays(d: long): Duration` — from days
- `Duration.ofNanos(n: long): Duration` — from nanoseconds
- `Duration.between(start: DateTime, end: DateTime): Duration` — between two dates
- `toMillis(): long`, `toSeconds(): long`, `toMinutes(): long`, `toHours(): long`, `toDays(): long`, `toNanos(): long` — unit conversions
- `plus(other: Duration): Duration` — add durations
- `minus(other: Duration): Duration` — subtract durations
- `multipliedBy(factor: long): Duration` — scale up
- `dividedBy(divisor: long): Duration` — scale down
- `negated(): Duration` — negate
- `abs(): Duration` — absolute value
- `isNegative(): bool` — check sign
- `isZero(): bool` — check if zero
- `toString(): string` — human-readable string (e.g. `"1h 30m 0s 500ms"`)

```titrate
let d = Duration.ofHours(2).plus(Duration.ofMinutes(30));
io::println(d.toString());  // "2h 30m 0s 0ms"
```

## Time

Utility class for common time operations.

- `Time.now(): DateTime` — current date and time
- `Time.sleep(ms: long): void` — sleep for milliseconds
- `Time.sleepDuration(d: Duration): void` — sleep for a duration
- `Time.millis(): long` — current time in milliseconds
- `Time.micros(): long` — current time in microseconds
- `Time.nanos(): long` — current time in nanoseconds
- `Time.measure(fn): Duration` — measure execution time of a function
- `Time.stopwatch(): Stopwatch` — create a new stopwatch

```titrate
let elapsed = Time.measure(fn(): void {
    // some expensive computation
});
io::println("Took: " + elapsed.toString());
```

## Stopwatch

Stopwatch for measuring elapsed time.

- `fn init()` — create a new stopwatch
- `start(): Stopwatch` — start (or resume) timing
- `stop(): Stopwatch` — stop timing
- `reset(): Stopwatch` — reset elapsed time
- `elapsed(): Duration` — elapsed time so far
- `isRunning(): bool` — check if currently running

```titrate
let sw = new Stopwatch().start();
// ... do work ...
sw.stop();
io::println("Elapsed: " + sw.elapsed().toString());
```
