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
io::println(now.toString());  // "2026-06-15 14:30:00"
io::println(now.toISO());     // "2026-06-15T14:30:00"
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

## Business Calendar

- `BusinessCalendar.init(region: string)` — create calendar for region (US, EU, JP, CN, UK)
- `BusinessCalendar.isBusinessDay(date: string): bool` — check if business day
- `BusinessCalendar.businessDaysBetween(start: string, end: string): int` — count business days
- `BusinessCalendar.addBusinessDays(date: string, n: int): string` — add business days
- `BusinessCalendar.isHoliday(date: string): bool` — check if holiday
- `BusinessCalendar.getHolidays(year: int): ArrayList<string>` — list holidays for year
- Holiday data loaded from `data/datetime/holidays.json`

## Cron Expression Parser

- `Cron.parse(expression: string): CronSchedule` — parse cron expression
- `Cron.next(schedule: CronSchedule, after: string): string` — next execution time
- `Cron.schedule(schedule: CronSchedule, from: string, count: int): ArrayList<string>` — generate schedule
- `Cron.validate(expression: string): bool` — validate cron expression

## Date Range

- `DateRange.dateRange(start: string, end: string, step: int): ArrayList<string>` — generate date range
- `DateRange.periodArithmetic(date: string, period: string, n: int): string` — add/subtract period
- `DateRange.isoWeekDate(date: string): (int, int, int)` — ISO week date (year, week, day)
- `DateRange.ordinalDate(date: string): (int, int)` — ordinal date (year, day-of-year)

## Calendar (Phase 1-2 parity)

The `Calendar` class mirrors Python's `calendar` module for iterating over days of a week/month and rendering calendars as text or HTML. `TextCalendar` and `HTMLCalendar` are subclasses that produce formatted output. First weekday is configurable (default Monday = 0, Sunday = 6).

### Calendar

- `Calendar.init(firstWeekday: int)` — create a calendar with the given first weekday (0=Monday … 6=Sunday)
- `Calendar.iterWeekDates(year: int, month: int): ArrayList<DateTime>` — all dates in a month as `DateTime`s
- `Calendar.iterMonthDates(year: int, month: int): ArrayList<DateTime>` — alias of `iterWeekDates`
- `Calendar.iterMonthDays(year: int, month: int): ArrayList<int>` — day numbers, with `0` for padding days outside the month
- `Calendar.iterMonthDays2(year: int, month: int): ArrayList<(int, int)>` — `(day, weekday)` pairs including padding `0` entries
- `Calendar.iterMonthDays3(year: int, month: int): ArrayList<(int, int, int)>` — `(year, month, day)` tuples for all cells in the month grid
- `Calendar.monthDaysCalendar(year: int, month: int): ArrayList<ArrayList<int>>` — weeks as rows of day numbers (with `0` padding), using the configured first weekday
- `Calendar.monthDatesCalendar(year: int, month: int): ArrayList<ArrayList<DateTime>>` — same as `monthDaysCalendar` but as `DateTime` rows
- `Calendar.monthdays2calendar(year: int, month: int): ArrayList<ArrayList<(int, int)>>` — rows of `(day, weekday)` tuples
- `Calendar.yeardayscalendar(year: int, width: int): ArrayList<ArrayList<ArrayList<ArrayList<int>>>>` — year broken into rows of `width` months each, each month being weeks-of-days
- `Calendar.yeardatescalendar(year: int, width: int)` — same structure as `yeardayscalendar` but with `DateTime`s
- `Calendar.setFirstWeekday(weekday: int): void` — change the first weekday
- `Calendar.getFirstWeekday(): int` — return the configured first weekday

### TextCalendar

- `TextCalendar.init(firstWeekday: int)` — create a calendar that renders plain-text output
- `TextCalendar.formatMonth(year: int, month: int, width: int): string` — render a single month as multi-line text (day columns `width` characters wide)
- `TextCalendar.formatYear(year: int, width: int, lines: int, gap: int): string` — render an entire year as multi-column text
- `TextCalendar.prMonth(year: int, month: int, width: int): void` — print a month to stdout
- `TextCalendar.prYear(year: int, width: int, lines: int, gap: int): void` — print a year to stdout

### HTMLCalendar

- `HTMLCalendar.init(firstWeekday: int)` — create a calendar that renders HTML tables
- `HTMLCalendar.formatMonth(year: int, month: int, withYear: bool): string` — render a single month as an HTML table
- `HTMLCalendar.formatYear(year: int, width: int): string` — render an entire year as an HTML table with `width` months per row
- `HTMLCalendar.formatYearPage(year: int, width: int, css: string): string` — render a full standalone HTML page for the year

### Module-level helpers

| Function | Description |
|----------|-------------|
| `Calendar.weekday(year: int, month: int, day: int): int` | Day of week (0=Monday … 6=Sunday) for the given date |
| `Calendar.weekHeader(width: int): string` | Localized weekday-abbreviation header row, each column `width` chars wide |
| `Calendar.monthRange(year: int, month: int): (int, int)` | `(weekday of first day, number of days in month)` |
| `Calendar.isleap(year: int): bool` | Whether `year` is a leap year |
| `Calendar.leapdays(y1: int, y2: int): int` | Number of leap years in `[y1, y2)` |
| `Calendar.timegm(tuple: (int, int, int, int, int, int, int, int, int)): long` | Inverse of `time.gmtime` — convert a UTC time tuple to epoch seconds |
| `Calendar.setFirstWeekday(weekday: int): void` | Module-level setter for the default first weekday |

```titrate
import tt.time.Calendar;
import tt.time.TextCalendar;
import tt.time.HTMLCalendar;

let cal = new Calendar(0);  // Monday-first
let days = cal.iterMonthDays(2026, 2);  // [0, 0, 0, 0, 0, 0, 1, 2, ..., 28]

let text = new TextCalendar(0).formatMonth(2026, 2, 4);
io::println(text);

let html = new HTMLCalendar(6).formatMonth(2026, 2, true);  // Sunday-first, with year caption

io::println(Boolean.toString(Calendar.isleap(2024)));  // true
io::println(Integer.toString(Calendar.weekday(2026, 7, 15)));  // 1 (Tuesday)
```
