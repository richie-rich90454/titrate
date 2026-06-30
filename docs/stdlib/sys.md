---
title: sys
description: System-level operations for Titrate — process control, environment, platform info, signals, and GC.
---

# sys

The `tt.sys` module provides system-level operations: process control, environment variables, platform information, signals, garbage collection, warnings, at-exit hooks, and file selectors. It is the umbrella module for interacting with the host operating system.

```titrate
import tt::sys::Sys;
import tt::sys::Os;
import tt::sys::Platform;
import tt::sys::Signal;
import tt::sys::Gc;
```

## Sys

General system utilities.

- `Sys.args(): ArrayList<string>` — command-line arguments
- `Sys.env(key: string): string` — environment variable value
- `Sys.setEnv(key: string, val: string): void`
- `Sys.exit(code: int): void`
- `Sys.workingDir(): string`
- `Sys.sleep(ms: int): void`
- `Sys.platform(): string`
- `Sys.cpuCount(): int`
- `Sys.exec(command: string): string`
- `Sys.pid(): int`
- `Sys.nanoTime(): long`
- `Sys.userName(): string`
- `Sys.hostName(): string`

```titrate
let cwd: string = Sys.workingDir();
io::println("Running in " + cwd);

let args: ArrayList<string> = Sys.args();
for (arg in args) {
    io::println(arg);
}
```

## Os

File system and OS operations.

- `Os.getcwd(): string`
- `Os.chdir(path: string): void`
- `Os.exists(path: string): bool`
- `Os.mkdir(path: string): Result<string, string>`
- `Os.makedirs(path: string): Result<string, string>`
- `Os.remove(path: string): Result<string, string>`
- `Os.rename(oldPath: string, newPath: string): Result<string, string>`
- `Os.rmdir(path: string): Result<string, string>`
- `Os.listdir(path: string): ArrayList<string>`
- `Os.walk(path: string): ArrayList<ArrayList<string>>`
- `Os.scandir(path: string): ArrayList<DirEntry>`
- `Os.stat(path: string): Result<HashMap<string, int>, string>`
- `Os.lstat(path: string): HashMap<string, int>`
- `Os.chmod(path: string, mode: int): Result<string, string>`
- `Os.umask(mask: int): int`
- `Os.getFileSize(path: string): int`
- `Os.symlink(original: string, link: string): Result<string, string>`
- `Os.readlink(path: string): Result<string, string>`
- `Os.link(src: string, dst: string): void`
- `Os.access(path: string, mode: int): bool`
- `Os.urandom(n: int): string`
- `Os.environ(): string`
- `Os.environMap(): HashMap<string, string>`
- `Os.getEnv(key: string): string`
- `Os.setEnv(key: string, value: string): void`
- `Os.unsetEnv(key: string): void`
- `Os.system(command: string): int`
- `Os.popen(command: string, mode: string): string`
- `Os.popenWrite(command: string, input: string): string`
- `Os.kill(pid: int, sig: int): void`
- `Os.getpid(): int`
- `Os.getppid(): int`
- `Os.uname(): string`
- `Os.strerror(code: int): string`

### Path Helpers

- `Os.pathSeparator(): string`
- `Os.separator(): string`
- `Os.linesep(): string`
- `Os.devNull(): string`

```titrate
if (Os.exists("data.txt")) {
    io::println("File exists");
}

let env: HashMap<string, string> = Os.environMap();
io::println(env.get("PATH"));
```

## Platform

Host platform information.

- `Platform.system(): string`
- `Platform.machine(): string`
- `Platform.release(): string`
- `Platform.version(): string`
- `Platform.processor(): string`
- `Platform.titrateVersion(): string`
- `Platform.node(): string`
- `Platform.architecture(): string`

```titrate
io::println(Platform.system());          // e.g. "windows"
io::println(Platform.titrateVersion());  // current Titrate version
```

## Signal

POSIX-style signal handling.

- `Signal.SIGHUP(): int`
- `Signal.SIGINT(): int`
- `Signal.SIGKILL(): int`
- `Signal.SIGTERM(): int`
- `Signal.SIGUSR1(): int`
- `Signal.SIGUSR2(): int`
- `Signal.register(signum: int, handler: fn(): void): void`
- `Signal.registerWithSignal(signum: int, handler: fn(int): void): void`
- `Signal.getSignal(signum: int): fn(int): void`
- `Signal.raise(signum: int): void`
- `Signal.defaultHandler(signum: int): fn(int): void`
- `Signal.ignoreHandler(signum: int): fn(int): void`

```titrate
Signal.register(Signal.SIGINT(), fn(): void {
    io::println("Interrupted by user");
    Sys.exit(0);
});
```

## Gc

Garbage collection control.

- `Gc.enable(): void`
- `Gc.disable(): void`
- `Gc.collect(): void`
- `Gc.isEnabled(): bool`
- `Gc.getStats(): HashMap<string, int>`
- `Gc.resetStats(): void`

```titrate
Gc.collect();
let stats: HashMap<string, int> = Gc.getStats();
```

## Warnings

- `Warnings.warn(message: string, category: string): void`
- `Warnings.filterWarnings(action: string, category: string): void`
- `Warnings.simplefilter(action: string, category: string): void`
- `Warnings.resetFilters(): void`
- `Warnings.getFilters(): ArrayList<HashMap<string, string>>`

### Warning Categories

- `WARNING`
- `USER_WARNING`
- `DEPRECATION_WARNING`

## Atexit

Register functions to run on program exit.

- `Atexit.register(handler: fn(): void): void`
- `Atexit.unregister(handler: fn(): void): void`
- `Atexit.runExitHooks(): void`
- `Atexit.handlerCount(): int`
- `Atexit.clear(): void`

```titrate
Atexit.register(fn(): void {
    io::println("Cleaning up before exit");
});
```

## Selectors

I/O multiplexing with selectors.

- `Selector.init()`
- `Selector.register(handle: int, events: int): void`
- `Selector.unregister(handle: int): void`
- `Selector.modify(handle: int, events: int): void`
- `Selector.isRegistered(handle: int): bool`
- `Selector.select(timeout: int): ArrayList<int>`
- `Selector.selectWithEvents(timeout: int): ArrayList<Variant>`
- `Selector.close(): void`
- `Selector.isClosed(): bool`
- `Selector.size(): int`
- `Selector.getEvents(handle: int): int`
- `Selectors.createSelector(): Selector`
