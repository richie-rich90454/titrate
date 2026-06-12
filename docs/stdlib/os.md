# os

The `tt.sys` module provides a full OS interface for interacting with the operating system, including environment variables, process information, and system utilities.

```titrate
import tt::sys::Os;
```

## Os

Static methods for operating system interactions.

- `Os.getEnv(name: string): string` — get environment variable
- `Os.setEnv(name: string, value: string): void` — set environment variable
- `Os.cpuCount(): int` — get number of CPUs
- `Os.pid(): int` — get current process ID
- `Os.userName(): string` — get current username
- `Os.hostName(): string` — get hostname
- `Os.urandom(n: int): string` — get n random bytes as hex string
- `Os.workingDir(): string` — get current working directory
- `Os.exit(code: int): void` — exit the process
- `Os.sleep(ms: int): void` — sleep for milliseconds

```titrate
let home: string = Os.getEnv("HOME");
let cpus: int = Os.cpuCount();
let pid: int = Os.pid();
let user: string = Os.userName();
let host: string = Os.hostName();
let rand: string = Os.urandom(16);
let cwd: string = Os.workingDir();
Os.sleep(1000);
```
