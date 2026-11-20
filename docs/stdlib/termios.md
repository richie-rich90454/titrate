# Termios

The `tt.os.Termios` module provides POSIX terminal control: `tcgetattr`, `tcsetattr`, `tcdrain`, `tcflush`, `tcflow`, `tcsendbreak`, `tcgetpgrp`, and `tcsetpgrp`, plus the standard `TCSA*`, queue-selector, flow-action, baud-rate, and `cc[]` index constants. It mirrors Python's `termios` module.

On Unix these functions call the real libc termios syscalls (`tcgetattr(3)`, `tcsetattr(3)`, `tcdrain(3)`, `tcflush(3)`, `tcsendbreak(3)`, `tcgetpgrp(2)`, `tcsetpgrp(2)`). On Windows there is no POSIX termios interface: `tcgetattr()` returns a zero-initialised `TermiosAttr` and every mutator returns `-1` without changing console state. `tcflow()` has no libc entry point and is an explicit no-op returning `-1` on all platforms.

## Import

```titrate
import tt::os::Termios;
```

## Class: TermiosAttr

Mirrors the C `struct termios` returned by `tcgetattr()`.

**Fields:**
- `iflag: int` — input flags
- `oflag: int` — output flags
- `cflag: int` — control flags
- `lflag: int` — local flags
- `ispeed: int` — input baud rate code
- `ospeed: int` — output baud rate code
- `cc: ArrayList<int>` — special-character array (17 entries)

**Methods:**
- `setIf(v: int): void`
- `setOf(v: int): void`
- `setCf(v: int): void`
- `setLf(v: int): void`
- `setIspeed(v: int): void`
- `setOspeed(v: int): void`
- `setCc(idx: int, value: int): void`
- `getCc(idx: int): int`

```titrate
let attr: TermiosAttr = new TermiosAttr();
attr.setLf(0);  // clear local flags
```

## Constants

All constants are zero-argument functions returning the platform's numeric value (loaded from `os/termios.json`).

### When selectors (`tcsetattr`)

- `TCSANOW(): int`
- `TCSADRAIN(): int`
- `TCSAFLUSH(): int`
- `TCSASOFT(): int`

### Queue selectors (`tcflush`)

- `TCIFLUSH(): int`
- `TCOFLUSH(): int`
- `TCIOFLUSH(): int`

### Flow actions (`tcflow`)

- `TCOOFF(): int`
- `TCOON(): int`
- `TCIOFF(): int`
- `TCION(): int`

### Baud rate codes

- `B0(): int`, `B50(): int`, `B75(): int`, `B110(): int`, `B134(): int`, `B150(): int`, `B200(): int`, `B300(): int`, `B600(): int`, `B1200(): int`, `B1800(): int`, `B2400(): int`, `B4800(): int`, `B9600(): int`, `B19200(): int`, `B38400(): int`

### `cc[]` indices

- `VINTR(): int`, `VQUIT(): int`, `VERASE(): int`, `VKILL(): int`, `VEOF(): int`, `VTIME(): int`, `VMIN(): int`, `VSWTC(): int`, `VSTART(): int`, `VSTOP(): int`, `VSUSP(): int`, `VEOL(): int`, `VREPRINT(): int`, `VDISCARD(): int`, `VWERASE(): int`, `VLNEXT(): int`, `VEOL2(): int`

## Functions

### tcgetattr

Get the terminal attributes for `fd`. On Unix this reads the live terminal state via `tcgetattr(3)`, including all 17 `cc[]` entries and the input/output baud rates. On Windows, returns a zero-initialised `TermiosAttr`. A non-terminal descriptor on Unix yields a zero-filled attribute set.

**Parameters:** `fd: int`
**Returns:** `TermiosAttr`

```titrate
let attr = tcgetattr(0);  // stdin
```

### tcsetattr

Set the terminal attributes for `fd` using the policy `when` (one of `TCSANOW`, `TCSADRAIN`, `TCSAFLUSH`, optionally `| TCSASOFT`). On Unix the flag words, baud rates and `cc[]` entries of `attr` are written to the kernel via `tcsetattr(3)`. On Windows this returns `-1` without changing console state.

**Parameters:** `fd: int`, `when: int`, `attr: TermiosAttr`
**Returns:** `int` — `0` on success, `-1` on error or unsupported platform

### tcsendbreak

Send a break condition on `fd` for `duration` tenths of a second.

**Parameters:** `fd: int`, `duration: int`
**Returns:** `int`

### tcdrain

Wait until all output written to `fd` has been transmitted.

**Parameters:** `fd: int`
**Returns:** `int`

### tcflush

Discard pending data on `fd`. `queueSelector` is one of `TCIFLUSH`, `TCOFLUSH`, `TCIOFLUSH`.

**Parameters:** `fd: int`, `queueSelector: int`
**Returns:** `int`

### tcflow

Suspend or resume transmission on `fd`. `action` is one of `TCOOFF`, `TCOON`, `TCIOFF`, `TCION`. libc exposes no `tcflow` entry point, so this is an explicit no-op returning `-1` on all platforms.

**Parameters:** `fd: int`, `action: int`
**Returns:** `int` — always `-1`

### tcgetpgrp

Return the foreground process group ID of the terminal `fd` via `tcgetpgrp(2)` on Unix, or `-1` on error (including a non-terminal descriptor). Returns `-1` on Windows, where process groups are not exposed through terminals.

**Parameters:** `fd: int`
**Returns:** `int`

### tcsetpgrp

Make `pgid` the foreground process group of the terminal `fd` via `tcsetpgrp(2)` on Unix. Returns `-1` on Windows, where process groups are not exposed through terminals.

**Parameters:** `fd: int`, `pgid: int`
**Returns:** `int` — `0` on success, `-1` on error or unsupported platform

```titrate
let attr = tcgetattr(0);
attr.setCc(VMIN(), 1);
attr.setCc(VTIME(), 0);
tcsetattr(0, TCSANOW(), attr);
```
