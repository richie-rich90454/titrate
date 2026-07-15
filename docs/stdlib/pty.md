# Pty

The `tt.os.Pty` module provides a Unix pseudo-terminal interface with `openpty`, `fork`, and `spawn`. It mirrors Python's `pty` module. On platforms lacking a POSIX pty layer (notably Windows), operations degrade to graceful no-ops returning sentinel fds (`-1`) so callers can detect the lack of support without throwing.

## Import

```titrate
import tt::os::Pty;
```

## Classes

### PtyPair

A pseudo-terminal pair returned by `openpty()`.

**Fields:**
- `masterFd: int`
- `slaveFd: int` — `-1` on Windows (no pty layer)

**Constructor:** `PtyPair(master: int, slave: int)`

### ForkResult

Result of `pty.fork()`: the child's pid (`0` inside the child) and the master fd the parent should read from.

**Fields:**
- `pid: int` — `0` in child, child pid in parent, `-1` on Windows
- `masterFd: int`

**Constructor:** `ForkResult(pid: int, masterFd: int)`

## Functions

### openpty

Open a new pseudo-terminal pair. On Windows, returns a synthetic pair whose `slaveFd` is `-1` to signal that the underlying device is unavailable.

**Returns:** `PtyPair`

```titrate
let p: PtyPair = openpty();
io::println(p.masterFd);
```

### fork

Fork the current process and attach a new pseudo-terminal to the child as its controlling terminal. On Windows, returns `ForkResult(-1, -1)` because true `fork()` is unavailable; callers should fall back to `spawn()`.

**Returns:** `ForkResult`

```titrate
let r: ForkResult = fork();
if (r.pid == 0) {
    io::println("child");
} else if (r.pid > 0) {
    io::println("parent");
} else {
    io::println("unsupported on this platform");
}
```

### spawn

Spawn the process described by `argv` attached to a new pseudo-terminal and stream its output to stdout. `argv[0]` is the program to execute. Returns the process exit status, or `-1` if the spawn failed.

**Overloads:**
- `spawn(argv: ArrayList<string>): int`
- `spawn(command: string): int` — execute the single command string verbatim

```titrate
let argv = new ArrayList<string>();
argv.add("ls"); argv.add("-l");
let status: int = spawn(argv);
```

### slaveOf

Look up the slave fd paired with a given master fd, or `-1` if unknown.

**Parameters:** `masterFd: int`
**Returns:** `int`

### close

Close a previously opened pty pair, releasing the in-process tracking entry.

**Parameters:** `masterFd: int`
**Returns:** `void`

```titrate
let p = openpty();
// ... use the pty ...
close(p.masterFd);
```
