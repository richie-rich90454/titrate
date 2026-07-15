# Faulthandler

The `tt.concurrent.Faulthandler` module provides fault handling for VM crashes. It mirrors Python's `faulthandler` module, exposing `enable`/`disable` to register an internal crash handler, `dumpTraceback`/`dumpTracebackLimit` to print the current call stack, and `register`/`unregister` for signal-based traceback dumps. Since the Titrate VM runs in a managed environment without POSIX signal injection, this module captures `Traceback.Frame` snapshots and dumps them on demand or when a registered fault condition is raised.

## Import

```titrate
import tt::concurrent::Faulthandler;
```

## Functions

### enable

- `Faulthandler.enable(file: string): void` — enable the fault handler. After this call, VM faults (such as unhandled exceptions and stack overflows) will dump a traceback before propagating. `file` is an optional path to write tracebacks to; `""` means stderr (via `println`).
- `Faulthandler.enableDefault(): void` — enable with default settings (stderr output)

```titrate
Faulthandler.enableDefault();
```

### disable

- `Faulthandler.disable(): void` — disable the fault handler. Subsequent faults will not dump tracebacks.

### isEnabled

- `Faulthandler.isEnabled(): bool` — true if the fault handler is currently enabled

### dumpTraceback

- `Faulthandler.dumpTraceback(allThreads: bool, chain: bool): void` — dump the current traceback to the configured output (or stderr). `allThreads` dumps all logical threads (currently a single thread). `chain` also dumps chained (causal) exceptions.

### dumpTracebackSimple

- `Faulthandler.dumpTracebackSimple(): void` — dump the current traceback (simple form, no options)

### dumpTracebackLimit

- `Faulthandler.dumpTracebackLimit(limit: int): void` — dump the traceback at most `limit` frames deep (most recent frames)

```titrate
Faulthandler.dumpTracebackLimit(10);
```

### register

- `Faulthandler.register(signum: int, file: string, allThreads: bool, chain: bool): bool` — register to dump a traceback when the given signal number is raised. Returns `true` on success. Since the VM is signal-agnostic, this is a soft registration that records intent for the fault handler to consult.

### unregister

- `Faulthandler.unregister(signum: int): void` — unregister a previously registered signal

### isRegistered

- `Faulthandler.isRegistered(signum: int): bool` — true if `signum` is currently registered for traceback dumps

### readTracebackSignum

- `Faulthandler.readTracebackSignum(signum: int): void` — read and dump the traceback produced when `signum` was last raised. In this implementation, this dumps the most recently captured traceback.

### reportFatal

- `Faulthandler.reportFatal(err: string): void` — enable traceback dumps on an explicit fault: call this to mark that an unrecoverable condition was hit. Dumps the traceback (if enabled) and re-throws `err`.

```titrate
Faulthandler.enableDefault();
Faulthandler.reportFatal("unrecoverable: out of memory");
```

## Usage Example

```titrate
import tt::concurrent::Faulthandler;

public fn main(): void {
    Faulthandler.enableDefault();
    Faulthandler.dumpTracebackSimple();
    io::println("Enabled: " + Boolean.toString(Faulthandler.isEnabled()));
    Faulthandler.register(11, "", false, false);
    io::println("SIGSEGV registered: " + Boolean.toString(Faulthandler.isRegistered(11)));
    try {
        Faulthandler.reportFatal("test fault");
    } catch (e: string) {
        io::println("Caught: " + e);
    }
}
```
