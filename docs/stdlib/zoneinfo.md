# zoneinfo

The `tt.time` module provides IANA timezone support for accurate timezone-aware date and time operations.

```titrate
import tt.time.ZoneInfo;
```

## ZoneInfo

Represents an IANA timezone, providing UTC offset and daylight saving time information.

- `ZoneInfo.of(name: string): ZoneInfo` — get a timezone by its IANA name (e.g., `"America/New_York"`, `"Europe/London"`)
- `ZoneInfo.utc(): ZoneInfo` — get the UTC timezone
- `ZoneInfo.local(): ZoneInfo` — get the system's local timezone
- `offsetAt(timestamp: int): int` — get the UTC offset in seconds at the given Unix timestamp
- `isDst(timestamp: int): bool` — check if daylight saving time is in effect at the given timestamp
- `name(): string` — get the IANA timezone name
- `abbrev(timestamp: int): string` — get the timezone abbreviation at the given timestamp (e.g., `"EST"`, `"EDT"`)

```titrate
let tz: ZoneInfo = ZoneInfo.of("America/New_York");
io::println(tz.name());  // "America/New_York"

let utcTz: ZoneInfo = ZoneInfo.utc();
io::println(utcTz.name());  // "UTC"

let localTz: ZoneInfo = ZoneInfo.local();
let offset: int = localTz.offsetAt(1718400000);
io::println(Integer.toString(offset));  // offset in seconds

let abbrev: string = tz.abbrev(1718400000);
io::println(abbrev);  // "EDT"
```
