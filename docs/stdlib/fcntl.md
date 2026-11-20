# Fcntl

The `tt.os.Fcntl` module provides Unix file-control operations (`fcntl`, `ioctl`, `flock`, `lockf`) on file descriptors. It mirrors Python's `fcntl` module.

On Unix these functions issue the real `fcntl(2)`, `ioctl(2)`, `flock(2)` and `lockf(2)` syscalls; the integer arguments are passed through unchanged and the raw syscall result is returned. `lockf()` maps the flock-style flags to `F_ULOCK`/`F_LOCK`/`F_TLOCK` and uses `lseek(2)` to honor `start` and `whence`.

Windows has none of these syscalls. `flock()` then falls back to an in-memory per-descriptor state map (an advisory lock within the process only), while `fcntl()`, `ioctl()` and `lockf()` print an unsupported-platform message and return `-1`.

## Import

```titrate
import tt::os::Fcntl;
```

## Constants

All constants are exposed as zero-argument functions returning the platform's numeric value (loaded from `os/fcntl.json`).

### Command constants

- `F_DUPFD(): int`
- `F_GETFD(): int`
- `F_SETFD(): int`
- `F_GETFL(): int`
- `F_SETFL(): int`
- `F_GETLK(): int`
- `F_SETLK(): int`
- `F_SETLKW(): int`
- `F_GETOWN(): int`
- `F_SETOWN(): int`

### File descriptor flags

- `FD_CLOEXEC(): int`

### POSIX record lock types

- `F_RDLCK(): int`
- `F_WRLCK(): int`
- `F_UNLCK(): int`

### `flock(2)` operation flags

- `LOCK_SH(): int` — shared lock
- `LOCK_EX(): int` — exclusive lock
- `LOCK_NB(): int` — non-blocking
- `LOCK_UN(): int` — unlock

## Functions

### fcntl

Perform the command `op` on file descriptor `fd`. On Unix the raw `fcntl(2)` result is returned (0 on success for set-commands, the requested value for get-commands, `-1` on syscall error). The string argument overload parses its argument as an integer with a fallback of 0. On Windows the function returns `-1`.

**Parameters:**
- `fd: int` — open file descriptor
- `op: int` — command constant
- `arg: int` (overload: `arg: string`, also a 2-arg form) — opaque argument

**Returns:** `int`

```titrate
let flags: int = fcntl(fd, F_GETFL());
```

### ioctl

Perform a device-control request on `fd`. On Unix the raw `ioctl(2)` result is returned; on Windows the function returns `-1`. The string argument overload parses its argument as an integer with a fallback of 0.

**Parameters:**
- `fd: int`
- `request: int`
- `arg: int` (overload: `arg: string`, also a 2-arg form)

**Returns:** `int`

### flock

Apply or remove an advisory lock on the open file descriptor `fd`. `operation` is one of `LOCK_SH`, `LOCK_EX`, `LOCK_UN`, optionally ORed with `LOCK_NB`. On Unix this calls `flock(2)` and `_flockState` is updated on success. On Windows there is no `flock(2)`, so only `_flockState` is updated and the call always returns `0`. Returns `0` on success, `-1` on Unix syscall error.

**Parameters:** `fd: int`, `operation: int`
**Returns:** `int`

```titrate
flock(fd, LOCK_EX());
// ... exclusive work ...
flock(fd, LOCK_UN());
```

### flockState

Query the currently-recorded `flock` operation for `fd`, or `0` if none. Useful for tests.

**Parameters:** `fd: int`
**Returns:** `int`

### lockf

Apply, test, or remove a POSIX lock region on `fd`. `length` is the number of bytes to lock starting at `start` bytes from `whence` (`0 = SEEK_SET`, `1 = SEEK_CUR`, `2 = SEEK_END`). On Unix the native uses `lseek(2)` then `lockf(2)`, mapping `LOCK_UN` to `F_ULOCK`, `LOCK_NB` to `F_TLOCK`, and otherwise `F_LOCK`. Windows returns `-1`.

**Overloads:**
- `lockf(fd: int, operation: int): int`
- `lockf(fd: int, operation: int, length: int): int`
- `lockf(fd: int, operation: int, length: int, start: int): int`
- `lockf(fd: int, operation: int, length: int, start: int, whence: int): int`

**Returns:** `0` on success, `-1` on failure

```titrate
lockf(fd, LOCK_EX(), 100, 0, 0);  // exclusive lock on first 100 bytes
```

### withFlock

Run a function while holding a `flock` on `fd`. The lock is released (`LOCK_UN`) when the function returns or throws.

**Parameters:** `fd: int`, `operation: int`, `f: fn(): void`
**Returns:** `void`

```titrate
withFlock(fd, LOCK_EX(), fn(): void {
    io::println("holding exclusive lock");
});
```
