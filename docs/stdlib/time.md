---
title: time
description: Date/time, durations, scheduling, stopwatch, and timezone handling for Titrate.
---

# time

The `tt.time` module provides date/time representation, duration calculations, scheduling, stopwatch utilities, and timezone handling. For working with business calendars, cron expressions, and date ranges, see the `datetime` page which documents the same `tt.time` namespace.

```titrate
import tt::time::DateTime;
import tt::time::Duration;
import tt::time::Time;
import tt::time::Stopwatch;
import tt::time::Scheduler;
import tt::time::ZoneInfo;
```

## DateTime

Represents a point in time as milliseconds since the Unix epoch.

- `fn init(ms: long)`
- `DateTime.now(): DateTime`
- `DateTime.ofEpochMillis(ms: long): DateTime`
- `DateTime.fromTimestamp(seconds: double): DateTime`
- `DateTime.fromISO(s: string): DateTime`
- `DateTime.strptime(input: string, format: string): DateTime`
- `DateTime.utc(): TimeZone`

### Components

- `getYear(): int`, `getMonth(): int`, `getDay(): int`
- `getHour(): int`, `getMinute(): int`, `getSecond(): int`
- `timestamp(): double`
- `dayOfWeek(): int`
- `dayOfYear(): int`
- `weekday(): int`
- `isoweekday(): int`
- `isocalendar(): ArrayList<int>`
- `isLeapYear(): bool`
- `daysInMonth(): int`
- `monthName(): string`
- `dayOfWeekName(): string`

### Comparison and Arithmetic

- `isBefore(other: DateTime): bool`
- `isAfter(other: DateTime): bool`
- `equals(other: DateTime): bool`
- `plus(d: Duration): DateTime`
- `minus(d: Duration): DateTime`
- `plusDays(n: long): DateTime`
- `plusMonths(n: int): DateTime`
- `plusYears(n: int): DateTime`
- `minusDays(n: long): DateTime`

### Withers and Formatting

- `withYear(y: int): DateTime`
- `withMonth(m: int): DateTime`
- `withDay(d: int): DateTime`
- `withHour(h: int): DateTime`
- `withMinute(m: int): DateTime`
- `withSecond(s: int): DateTime`
- `replace(year: int, month: int, day: int, hour: int, minute: int, second: int): DateTime`
- `format(fmt: string): string`
- `strftime(format: string): string`
- `toISO(): string`
- `toString(): string`
- `parse(format: string, input: string): DateTime`

### Timezones

- `toUtc(): DateTime`
- `offsetMinutes(): int`
- `isDst(): bool`
- `astimezone(tz: TimeZone): DateTime`

```titrate
let now: DateTime = DateTime.now();
io::println(now.toISO());

let deadline: DateTime = now.plusDays(7);
let remaining: Duration = Duration.between(now, deadline);
io::println("Seconds until deadline: " + Long.toString(remaining.toSeconds()));
```

## Duration

Represents a length of time in milliseconds.

- `fn init(ms: long)`
- `Duration.ofMillis(ms: long): Duration`
- `Duration.ofSeconds(s: long): Duration`
- `Duration.ofMinutes(m: long): Duration`
- `Duration.ofHours(h: long): Duration`
- `Duration.ofDays(d: long): Duration`
- `Duration.ofNanos(n: long): Duration`
- `Duration.ofMicros(us: long): Duration`
- `Duration.between(start: DateTime, end: DateTime): Duration`

### Conversions and Arithmetic

- `toMillis(): long`
- `toSeconds(): long`
- `toMinutes(): long`
- `toHours(): long`
- `toDays(): long`
- `toNanos(): long`
- `toMicros(): long`
- `totalSeconds(): double`
- `plus(other: Duration): Duration`
- `minus(other: Duration): Duration`
- `multipliedBy(factor: long): Duration`
- `dividedBy(divisor: long): Duration`
- `negated(): Duration`
- `abs(): Duration`
- `isNegative(): bool`
- `isZero(): bool`
- `toString(): string`

```titrate
let d: Duration = Duration.ofHours(2).plus(Duration.ofMinutes(30));
io::println(d.toString());  // "2h 30m 0s 0ms"
```

## Time

Utility functions for time operations.

- `Time.now(): DateTime`
- `Time.sleep(ms: long): void`
- `Time.sleepDuration(d: Duration): void`
- `Time.millis(): long`
- `Time.micros(): long`
- `Time.nanos(): long`
- `Time.monotonic(): long`
- `Time.perfCounter(): long`
- `Time.epochSeconds(): double`
- `Time.measure(f: fn(): void): Duration`
- `Time.stopwatch(): Stopwatch`

```titrate
let elapsed: Duration = Time.measure(fn(): void {
    // some expensive computation
});
io::println("Took: " + elapsed.toString());
```

## Stopwatch

- `fn init()`
- `start(): Stopwatch`
- `stop(): Stopwatch`
- `reset(): Stopwatch`
- `restart(): Stopwatch`
- `elapsed(): Duration`
- `elapsedNanos(): long`
- `elapsedMillis(): double`
- `elapsedSeconds(): double`
- `isRunning(): bool`
- `toString(): string`

```titrate
let sw: Stopwatch = Time.stopwatch().start();
// ... work ...
sw.stop();
io::println("Elapsed: " + sw.elapsedMillis() + " ms");
```

## Scheduler

Time-based task scheduler.

- `Scheduler.init()`
- `Scheduler.enter(delay: double, action: fn(): void): ScheduledEvent`
- `Scheduler.enterWithPriority(delay: double, priority: int, action: fn(): void): ScheduledEvent`
- `Scheduler.cancel(event: ScheduledEvent): bool`
- `Scheduler.empty(): bool`
- `Scheduler.run(): void`
- `Scheduler.queue(): ArrayList<ScheduledEvent>`

## ZoneInfo

Timezone representation and conversion.

- `ZoneInfo.init(name: string, offsetHours: double)`
- `ZoneInfo.name(): string`
- `ZoneInfo.offset(): double`
- `ZoneInfo.offsetSeconds(): int`
- `ZoneInfo.toString(): string`
- `ZoneInfo.convert(dt: DateTime, fromZone: ZoneInfo): DateTime`

### Named Timezones

- `ZoneInfo.timezone(name: string): ZoneInfo`
- `ZoneInfo.utc(): ZoneInfo`
- `ZoneInfo.gmt(): ZoneInfo`
- `ZoneInfo.eastern(): ZoneInfo`
- `ZoneInfo.central(): ZoneInfo`
- `ZoneInfo.mountain(): ZoneInfo`
- `ZoneInfo.pacific(): ZoneInfo`
- `ZoneInfo.cet(): ZoneInfo`
- `ZoneInfo.jst(): ZoneInfo`
- `ZoneInfo.cst(): ZoneInfo`
- `ZoneInfo.ist(): ZoneInfo`
- `ZoneInfo.aest(): ZoneInfo`
