# Chrono

The `tt.time.Chrono` module provides clock types, time points, durations, and calendar types that mirror C++'s `<chrono>` header. It supports `SystemClock`, `SteadyClock`, `HighResolutionClock`, `UtcClock`, `TaiClock`, `GpsClock`, and `FileClock`, plus `TimePoint`/`Duration` arithmetic, `clock_cast`, calendar types (`Year`, `Month`, `Day`, `Weekday`, `YearMonthDay`), and the `tzdb`/`TimeZone` database.

## Import

```titrate
import tt::time::Chrono;
```

## Clock types

### SystemClock

Wall-clock time from the system's real-time clock. Suitable for timestamps but not monotonic intervals.

- `SystemClock.now(): long` — current wall-clock time in nanoseconds since the Unix epoch
- `SystemClock.toTimeT(tp: long): long` — convert a `TimePoint` value to POSIX `time_t` (seconds)
- `SystemClock.fromTimeT(t: long): long` — convert POSIX `time_t` (seconds) to a `TimePoint` value
- `SystemClock.isSteady(): bool` — returns `false` (system clock may be adjusted)

### SteadyClock

Monotonic clock that never decreases. Use for measuring intervals.

- `SteadyClock.now(): long` — current monotonic time in nanoseconds
- `SteadyClock.isSteady(): bool` — returns `true`

### HighResolutionClock

The clock with the smallest supported tick period; usually an alias for `SteadyClock`.

- `HighResolutionClock.now(): long` — current high-resolution time in nanoseconds
- `HighResolutionClock.isSteady(): bool` — returns `true`

### UtcClock / TaiClock / GpsClock / FileClock

Specialized clocks for UTC, International Atomic Time, GPS time, and file-system timestamps. Each exposes `now(): long` and can be converted via `clockCast`.

```titrate
let t0: long = SteadyClock.now();
// ... do work ...
let elapsed: long = SteadyClock.now() - t0;
io::println("elapsed ns: " + Long.toString(elapsed));
```

## Duration

`Duration` represents a time interval as a count of ticks with a given period (numerator/denominator).

- `Duration.init(ticks: long, num: long, den: long)` — construct a duration with the given count and period
- `Duration.count(): long` — the raw tick count
- `Duration.toNanos(): long` — convert to nanoseconds
- `Duration.toMicros(): long` — convert to microseconds
- `Duration.toMillis(): long` — convert to milliseconds
- `Duration.toSeconds(): double` — convert to seconds as a floating-point value
- `Duration.add(other: Duration): Duration` — sum two durations (period must match)
- `Duration.subtract(other: Duration): Duration` — difference of two durations
- `Duration.multiply(scalar: long): Duration` — multiply tick count by a scalar
- `Duration.divide(scalar: long): Duration` — divide tick count by a scalar
- `Duration.equals(other: Duration): bool` — equality
- `Duration.lessThan(other: Duration): bool` — ordering
- `Duration.toString(): string` — human-readable representation

```titrate
let d: Duration = new Duration(500, 1, 1000);  // 500 ms
io::println(Duration.toString(d.toSeconds()));  // 0.5
```

## TimePoint

A point in time defined relative to a clock's epoch.

- `TimePoint.init(epoch: long, ticks: long, num: long, den: long)` — construct from epoch, tick count, and period
- `TimePoint.timeSinceEpoch(): Duration` — duration since the epoch
- `TimePoint.add(d: Duration): TimePoint` — advance the time point
- `TimePoint.subtract(d: Duration): TimePoint` — move the time point backwards
- `TimePoint.equals(other: TimePoint): bool` — equality
- `TimePoint.lessThan(other: TimePoint): bool` — ordering

```titrate
let now: long = SystemClock.now();
let tp: TimePoint = new TimePoint(0, now, 1, 1000000000);  // ns since epoch
let later: TimePoint = tp.add(new Duration(5, 1, 1));  // +5 s
```

## clockCast

Convert a time point from one clock to another.

- `clockCast<ClockFrom, ClockTo>(tp: long): long` — convert a `TimePoint` value between clocks. All clocks share the same nanosecond epoch, so this is a straight pass-through that exists for API parity.

## Calendar types

### Year

- `Year.init(y: int)` — construct a year
- `Year.isLeap(): bool` — true if the year is a leap year
- `Year.toInt(): int` — the year value
- `Year.minValue(): int` / `Year.maxValue(): int` — bounds

### Month

- `Month.init(m: int)` — construct a month (1–12)
- `Month.toInt(): int` — the month value
- `Month.name(): string` — locale-independent English name (`"January"`…`"December"`)

### Day

- `Day.init(d: int)` — construct a day-of-month
- `Day.toInt(): int` — the day value

### Weekday

- `Weekday.init(w: int)` — construct a weekday (0=Sunday, 6=Saturday)
- `Weekday.toInt(): int` — the weekday value
- `Weekday.name(): string` — English weekday name

### YearMonthDay

- `YearMonthDay.init(y: int, m: int, d: int)` — construct a calendar date
- `YearMonthDay.fromTimePoint(tp: TimePoint): YearMonthDay` — convert a time point to a calendar date
- `YearMonthDay.year(): int` / `Month()` / `Day()` — accessors
- `YearMonthDay.toString(): string` — `"YYYY-MM-DD"`

```titrate
let ymd: YearMonthDay = new YearMonthDay(2024, 2, 29);
io::println(Boolean.toString(new Year(ymd.year()).isLeap()));  // true
```

## TimeZone database

### Tzdb

The timezone database loaded from `data/time/tzdata.json`.

- `Tzdb.listTimeZones(): ArrayList<string>` — names of all known time zones
- `Tzdb.getTimeZone(name: string): TimeZone` — look up a time zone by name
- `Tzdb.currentZone(): string` — the system's current time-zone name
- `Tzdb.version(): string` — the database version string

### TimeZone

- `TimeZone.name(): string` — the time-zone identifier
- `TimeZone.utcOffset(tp: long): long` — UTC offset in seconds for the given time point
- `TimeZone.isDst(tp: long): bool` — true if daylight-saving time is in effect

### LeapSecond

- `LeapSecond.init(date: string, value: int)` — a leap second insertion/deletion
- `LeapSecond.list(): ArrayList<LeapSecond>` — all known leap seconds

## is_clock

- `isClock(name: string): bool` — runtime check for whether a clock name exists
