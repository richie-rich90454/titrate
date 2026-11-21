# Pty

The `tt.os.Pty` module provides a Unix pseudo-terminal interface with `openpty`, `fork`, and `spawn`. It mirrors Python's `pty` module.

On Unix, `openpty()` opens a real master/slave pair through `posix_openpt(2)`, `grantpt(2)` and `unlockpt(2)`, and `close()` releases both descriptors. On Windows there is no POSIX pty layer: `openpty()` returns a synthetic master descriptor with `slaveFd = -1` and `close()` only clears the in-process tracking entry.

On Linux, `fork()` calls `libc::fork()` directly: the parent receives the child pid and the child receives `0`, then resumes at the call site with the VM stack and file descriptors inherited. Fork is refused with a diagnostic when VM threads are active or the process has more than one OS thread, because forking a multithreaded process can deadlock in allocator and lock state. On other platforms it returns `-1`: Windows has no operation that snapshots the caller's address space (`CreateProcess` starts from an image, and `NtCreateProcess` needs an image section), so a true fork is not representable; use `spawn()` there.

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

Open a new pseudo-terminal pair. On Unix both descriptors are real OS file descriptors and are tracked by `slaveOf()`. On Windows, returns a synthetic master and a `slaveFd` of `-1`. On Unix, a failure to allocate the pty returns `(-1, -1)`.

**Returns:** `PtyPair`

```titrate
let p: PtyPair = openpty();
io::println(p.masterFd);
```

### fork

Perform a POSIX fork on Linux, returning the child pid to the parent and `0` to the child. The child inherits the VM stack and open file descriptors (no cross-process close). The call is refused when VM threads or multiple OS threads are active; other platforms return `-1` and should use `spawn()`.

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

Close a previously opened pty pair. On Unix this closes the real master and slave descriptors as well as releasing the in-process tracking entry; on Windows only the tracking entry is cleared.

**Parameters:** `masterFd: int`
**Returns:** `void`

```titrate
let p = openpty();
// ... use the pty ...
close(p.masterFd);
```
