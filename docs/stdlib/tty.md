# Tty

The `tt.os.Tty` module provides terminal mode helpers mirroring Python's `tty` module: `setraw`, `setcbreak`, the `raw` and `cbreak` context helpers, and `ttyspawn`.

## Import

```titrate
import tt::os::Tty;
```

## Constants

The flag constants below are zero-argument functions loaded from `os/tty.json`. They return the platform's numeric bit values.

- Input flags: `BRKINT()`, `ICRNL()`, `INPCK()`, `ISTRIP()`, `IXON()`
- Output flags: `OPOST()`
- Control flags: `CSIZE()`, `PARENB()`, `CS8()`
- Local flags: `ECHO()`, `ECHOK()`, `ECHOE()`, `ECHONL()`, `ICANON()`, `ISIG()`, `IEXTEN()`, `FLUSHO()`, `NOFLSH()`, `TOSTOP()`

## Functions

### setraw

Put the terminal attached to `fd` into raw mode: clear canonical mode, echo, signal processing and input/output post-processing, then set 8-bit characters and `VMIN = 1`, `VTIME = 0`. The previous `TermiosAttr` is returned so it can be restored later.

**Overloads:**
- `setraw(fd: int): TermiosAttr` — uses `TCSAFLUSH`
- `setraw(fd: int, when: int): TermiosAttr`

On Unix this reads and writes the real terminal mode through `Termios.tcgetattr()` / `Termios.tcsetattr()`. Windows has no POSIX termios, so a zero-initialised `TermiosAttr` is returned and the console is unchanged.

### setcbreak

Like `setraw()` but clears only `ECHO` and `ICANON`, leaving signal processing enabled.

**Overloads:**
- `setcbreak(fd: int): TermiosAttr`
- `setcbreak(fd: int, when: int): TermiosAttr`

Platform behavior matches `setraw()`.

### raw / cbreak

`raw(fd, f)` and `cbreak(fd, f)` run a callback with the terminal in the requested mode and restore the saved mode afterward, including when the callback throws.

**Parameters:** `fd: int`, `f: fn(): void`
**Returns:** `void`

### ttyspawn

Run `argv` as a real child process and return its captured stdout. `argv[0]` is the program to execute.

**Parameters:** `argv: ArrayList<string>`
**Returns:** `string` — captured output, or `""` for an empty argument list

On Unix, `ttyspawn()` currently uses the regular subprocess path; the child inherits the parent's terminal mode. A true controlling pseudo-terminal is not created. On Windows it runs the same subprocess path without a console pty. Use `tt.os.Pty.spawn()` when pseudo-terminal streaming is required.
