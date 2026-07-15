# Syslog

The `tt.os.Syslog` module provides a Unix syslog interface with `openlog`, `syslog`, `closelog`, and `setlogmask`, plus the standard `LOG_*` priority, facility, and option constants. It mirrors Python's `syslog` module. On Windows (where the native syslog daemon is unavailable), messages are written to stderr mirroring `LOG_PERROR` behaviour, and the openlog/closelog/setlogmask state machine still works for portable code.

## Import

```titrate
import tt::os::Syslog;
```

## Constants

All constants are zero-argument functions returning the platform's numeric value (loaded from `os/syslog.json`).

### Priority levels

- `LOG_EMERG(): int`, `LOG_ALERT(): int`, `LOG_CRIT(): int`, `LOG_ERR(): int`, `LOG_WARNING(): int`, `LOG_NOTICE(): int`, `LOG_INFO(): int`, `LOG_DEBUG(): int`

### Facility codes

- `LOG_KERN(): int`, `LOG_USER(): int`, `LOG_MAIL(): int`, `LOG_DAEMON(): int`, `LOG_AUTH(): int`, `LOG_SYSLOG(): int`, `LOG_LPR(): int`, `LOG_NEWS(): int`, `LOG_UUCP(): int`, `LOG_CRON(): int`, `LOG_AUTHPRIV(): int`, `LOG_FTP(): int`, `LOG_LOCAL0(): int` … `LOG_LOCAL7(): int`

### Log options

- `LOG_PID(): int`, `LOG_CONS(): int`, `LOG_ODELAY(): int`, `LOG_NDELAY(): int`, `LOG_NOWAIT(): int`, `LOG_PERROR(): int`

## Functions

### LOG_MASK

Return the bit-mask for a single priority level, suitable for ORing into the value passed to `setlogmask()`.

**Parameters:** `priority: int`
**Returns:** `int`

### LOG_UPTO

Return the bit-mask covering all priorities up to and including `priority`.

**Parameters:** `priority: int`
**Returns:** `int`

```titrate
setlogmask(LOG_UPTO(LOG_WARNING()));
```

### openlog

Open a connection to the system logger. `ident` is a string prepended to every message; `logoption` is the OR of zero or more `LOG_*` option constants; `facility` selects the default facility.

**Parameters:** `ident: string`, `logoption: int`, `facility: int`
**Returns:** `void`

```titrate
openlog("myapp", LOG_PID(), LOG_USER());
```

### syslog

Send a message to the system logger. `priority` is the OR of a priority level and an optional facility. Overload accepts printf-style `(format, args)` supporting `%s`, `%d`, and `%%`.

**Overloads:**
- `syslog(priority: int, message: string): void`
- `syslog(priority: int, format: string, args: ArrayList<string>): void`

```titrate
syslog(LOG_INFO(), "user logged in");
syslog(LOG_ERR(), "user %s failed", args);
```

### closelog

Close the current connection to the system logger and clear the openlog state.

**Returns:** `void`

### setlogmask

Set the priority mask used to filter `syslog()` messages. Returns the previous mask. Messages whose priority bit is not set are dropped.

**Parameters:** `mask: int`
**Returns:** `int` — previous mask

### getlogmask

Return the current priority mask.

**Returns:** `int`

### ident

Return the ident string set by the most recent `openlog()` call (or `""` if none).

**Returns:** `string`

### isOpen

Return `true` if `openlog()` has been called and `closelog()` has not been called since.

**Returns:** `bool`
