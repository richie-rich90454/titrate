# Calendar

The `tt.time.Calendar` module provides calendar-generation utilities and locale name tables. It mirrors Python's `calendar` module, exposing weekday/year-length helpers, monthly/yearly matrices (`monthcalendar`, `yeardatescalendar`, `yeardayscalendar`), name accessors (`month_name`, `month_abbr`, `day_name`, `day_abbr`), and `TextCalendar` / `HTMLCalendar` formatter classes. Day-of-week numbering follows Python: Monday=0 .. Sunday=6. Name tables are loaded from `locale/month_names.json` on first use.

## Import

```titrate
import tt::time::Calendar;
```

## Constants

- `Calendar.MONDAY: int = 0`
- `Calendar.TUESDAY: int = 1`
- `Calendar.WEDNESDAY: int = 2`
- `Calendar.THURSDAY: int = 3`
- `Calendar.FRIDAY: int = 4`
- `Calendar.SATURDAY: int = 5`
- `Calendar.SUNDAY: int = 6`

## Functions

### Leap-year and date helpers

- `Calendar.isleap(year: int): bool` — true if `year` is a leap year
- `Calendar.leapdays(y1: int, y2: int): int` — number of leap years in `[y1, y2)`
- `Calendar.firstweekday(year: int): int` — weekday (Monday=0) for January 1st of `year`
- `Calendar.yeardays(year: int): int` — total days in a year (365 or 366)
- `Calendar.monthlen(year: int, month: int): int` — number of days in a month (1..12)
- `Calendar.weekday(year: int, month: int, day: int): int` — day of week (Monday=0) for a date
- `Calendar.yearday(year: int, month: int, day: int): int` — day of year (1-based) for a date

```titrate
let leap: bool = Calendar.isleap(2024);  // true
let wd: int = Calendar.weekday(2026, 7, 15);
```

### Calendar matrices

- `Calendar.monthcalendar(year: int, month: int): ArrayList<ArrayList<int>>` — matrix of weeks for one month; each row is 7 entries, either day-of-month (>=1) or 0 (outside the month)
- `Calendar.monthdays2calendar(year: int, month: int): ArrayList<ArrayList<int>>` — list of weeks (each week is 7 day-of-month ints in Monday=0 order) for one month
- `Calendar.monthdatescalendar(year: int, month: int): ArrayList<ArrayList<DateTime>>` — matrix of weeks of `DateTime` for one month, including leading/trailing days from adjacent months
- `Calendar.yeardatescalendar(year: int, width: int): ArrayList<ArrayList<ArrayList<ArrayList<int>>>>` — calendar for an entire year grouped `width` months per row
- `Calendar.yeardayscalendar(year: int, width: int): ArrayList<ArrayList<ArrayList<int>>>` — calendar for a year as a list of months, each a list of weeks of day-of-month ints

### Formatters

- `Calendar.month(year: int, month: int, w: int, l: int): string` — pretty-print one month as multi-line text (mirrors `calendar.month`)
- `Calendar.calendar(year: int, w: int, l: int, c: int, m: int): string` — pretty-print a full year as text (mirrors `calendar.calendar`)

```titrate
let text: string = Calendar.month(2026, 7, 0, 0);
io::println(text);
```

### Name accessors

- `Calendar.setfirstweekday(weekday: int): void` — no-op (Titrate always uses Monday=0 internally); provided for API parity
- `Calendar.month_name(month: int): string` — name of the given month (1..12), English
- `Calendar.month_abbr(month: int): string` — abbreviated month name (1..12), English
- `Calendar.day_name(wd: int): string` — name of the given weekday (Monday=0 .. Sunday=6), English
- `Calendar.day_abbr(wd: int): string` — abbreviated weekday name (Monday=0 .. Sunday=6), English

## Classes

### Calendar

Base class for calendar generators.

**Fields:**
- `firstweekday: int`

**Constructors:**
- `init(firstweekday: int)`

**Methods:**
- `itermonthdates(year: int, month: int): ArrayList<DateTime>` — iterate `DateTime`s for one month, skipping padding days
- `monthdayscalendar(year: int, month: int): ArrayList<ArrayList<int>>` — matrix of weeks (each 7 day-of-month ints, 0 = padding)
- `monthdays2calendar(year: int, month: int): ArrayList<ArrayList<int>>` — same shape as `monthdayscalendar`
- `monthdatescalendar(year: int, month: int): ArrayList<ArrayList<DateTime>>` — matrix of weeks of `DateTime` including adjacent-month days
- `yeardayscalendar(year: int, width: int): ArrayList<ArrayList<ArrayList<int>>>` — year as a list of months
- `yeardays2calendar(year: int, width: int): ArrayList<ArrayList<ArrayList<ArrayList<int>>>>` — year grouped `width` months per row

### TextCalendar

Plain-text calendar formatter.

**Constructors:**
- `init(firstweekday: int)`

**Methods:**
- `formatmonth(year: int, month: int, w: int, l: int): string` — format one month as multi-line text
- `formatyear(year: int, w: int, l: int, c: int, m: int): string` — format a full year as text

### HTMLCalendar

HTML calendar formatter.

**Methods:**
- `formatmonth(year: int, month: int, withyear: bool): string` — format one month as an HTML table
- `formatyear(year: int, width: int): string` — format a full year as an HTML table

## Usage Example

```titrate
import tt::time::Calendar;

public fn main(): void {
    io::println(Calendar.month(2026, 7, 0, 0));
    let weeks: ArrayList<ArrayList<int>> = Calendar.monthcalendar(2026, 7);
    io::println(Integer.toString(weeks.size()) + " weeks");
    io::println(Calendar.month_name(7));  // "July"
    io::println(Calendar.day_name(0));    // "Monday"
}
```
