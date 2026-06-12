# platform

The `tt.sys` module provides platform identification utilities for querying OS and hardware information.

```titrate
import tt.sys.Platform;
```

## Platform

Utilities for identifying the current runtime platform, operating system, and hardware architecture.

- `Platform.system(): string` — get the OS name (e.g., `"Windows"`, `"Linux"`, `"macOS"`)
- `Platform.machine(): string` — get the machine architecture (e.g., `"AMD64"`, `"x86_64"`)
- `Platform.release(): string` — get the OS release version
- `Platform.version(): string` — get the OS version string
- `Platform.processor(): string` — get the processor name
- `Platform.pythonCompat(): string` — get a Python-compatible platform string

```titrate
let os: string = Platform.system();
let arch: string = Platform.machine();
let release: string = Platform.release();

io::println("OS: " + os);           // "OS: Windows"
io::println("Arch: " + arch);       // "Arch: AMD64"
io::println("Release: " + release); // "Release: 10"

let compat: string = Platform.pythonCompat();
io::println("Compat: " + compat);   // e.g., "Windows-10-10.0.19045-AMD64"
```
