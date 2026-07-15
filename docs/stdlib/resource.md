# Resource

The `tt.os.Resource` module provides Unix resource limits and usage: `getrlimit`, `setrlimit`, and `getrusage`, plus the standard `RLIMIT_*`, `RUSAGE_*`, and `RLIM_INFINITY` constants. It mirrors Python's `resource` module. On Windows there is no `getrlimit`/`setrlimit` syscall, so the getters return infinite limits (`RLIM_INFINITY`), the setters become graceful no-ops returning `-1`, and `getrusage()` returns a zero-initialised `Rusage`.

## Import

```titrate
import tt::os::Resource;
```

## Constants

All constants are zero-argument functions returning the platform's numeric value (loaded from `os/resource.json`).

### `RLIMIT_*` resource kinds

- `RLIMIT_CPU(): int`, `RLIMIT_FSIZE(): int`, `RLIMIT_DATA(): int`, `RLIMIT_STACK(): int`, `RLIMIT_CORE(): int`, `RLIMIT_RSS(): int`, `RLIMIT_NPROC(): int`, `RLIMIT_NOFILE(): int`, `RLIMIT_OFILE(): int`, `RLIMIT_MEMLOCK(): int`, `RLIMIT_AS(): int`, `RLIMIT_SBSIZE(): int`, `RLIMIT_SIGPENDING(): int`, `RLIMIT_MSGQUEUE(): int`, `RLIMIT_NICE(): int`, `RLIMIT_RTPRIO(): int`, `RLIMIT_RTTIME(): int`

### `RUSAGE_*` selectors

- `RUSAGE_SELF(): int`
- `RUSAGE_CHILDREN(): int`
- `RUSAGE_THREAD(): int`

### Infinity sentinels

- `RLIM_INFINITY(): int`
- `RLIM_SAVED_MAX(): int`
- `RLIM_SAVED_CUR(): int`

## Classes

### Rlimit

Resource limit pair returned by `getrlimit()` and accepted by `setrlimit()`. Either field may be `RLIM_INFINITY` to indicate "no limit".

**Fields:**
- `soft: int` — current (soft) limit
- `hard: int` — maximum value to which the soft limit may be raised

**Constructor:** `Rlimit(soft: int, hard: int)`

### Rusage

Resource usage record returned by `getrusage()`. Times are in seconds; fields are zero-initialised when the platform or `who` argument does not support the query.

**Fields:**
- `ruUtime: double` — user CPU time
- `ruStime: double` — system CPU time
- `ruMaxrss: int`, `ruIxrss: int`, `ruIdrss: int`, `ruIsrss: int` — memory usage
- `ruMinflt: int`, `ruMajflt: int`, `ruNswap: int` — page faults/swaps
- `ruInblock: int`, `ruOublock: int` — block I/O
- `ruMsgsnd: int`, `ruMsgrcv: int` — IPC
- `ruNsignals: int`, `ruNvcsw: int`, `ruNivcsw: int` — signals and context switches

## Functions

### getrlimit

Get the soft and hard limits for `resource` (one of `RLIMIT_*`). Returns a `Rlimit` with both fields set to `RLIM_INFINITY` on platforms that do not expose the syscall.

**Parameters:** `resource: int`
**Returns:** `Rlimit`

```titrate
let r = getrlimit(RLIMIT_NOFILE());
io::println(r.soft);
```

### setrlimit

Set the soft and hard limits for `resource`. Returns `0` on success and `-1` on error or unsupported platform.

**Overloads:**
- `setrlimit(resource: int, limits: Rlimit): int`
- `setrlimit(resource: int, soft: int, hard: int): int`

```titrate
let result = setrlimit(RLIMIT_NOFILE(), 1024, 4096);
```

### getrusage

Get resource usage for `who` (one of `RUSAGE_SELF`, `RUSAGE_CHILDREN`, `RUSAGE_THREAD`). Returns a zero-initialised `Rusage` on unsupported platforms.

**Parameters:** `who: int`
**Returns:** `Rusage`

```titrate
let u = getrusage(RUSAGE_SELF());
io::println(u.ruUtime);
```
