# SyncStream

The `tt::io::SyncStream` module provides C++ `<syncstream>` parity. An `osyncstream` wraps a `Writer` with an internal buffer and flushes it atomically on destruction or explicit `flush()`. This prevents output from concurrent writers being interleaved at the destination.

## Import

```titrate
import tt::io::SyncStream;
```

## API Reference

### `SyncStream`

A buffered wrapper around a `Writer` that flushes its entire buffer atomically. Each `flush()` call writes the buffer's contents to the underlying `Writer` in a single critical section, preventing line-level interleaving with other `SyncStream`s targeting the same `Writer` (when they share the same sync mutex).

Implements the `Writer` interface.

**Constructors:**
- `init(wrapped: Writer)` — wraps the given `Writer` with a fresh internal mutex. Use `initWithMutex` when multiple `SyncStream`s must coordinate against the same destination.
- `initWithMutex(wrapped: Writer, syncMutex: Mutex)` — wraps the given `Writer` and shares the provided mutex. All `SyncStream`s sharing the same mutex serialize their flushes against the destination.

**Methods:**
- `write(text: string): void` — writes text to the internal buffer (does not flush). Throws if the stream is closed.
- `writeLine(text: string): void` — writes text followed by a newline to the internal buffer.
- `flush(): bool` — atomically flushes the buffer to the underlying `Writer`. After `flush()`, the buffer is empty. Returns true if anything was flushed.
- `close(): void` — atomically flushes the buffer and marks the stream as closed. Subsequent writes will throw. Idempotent.
- `isClosed(): bool` — returns true if the stream has been closed
- `emitted(): bool` — returns true if at least one flush has occurred since construction or the last `resetEmitFlag()` call
- `resetEmitFlag(): void` — resets the emitted flag
- `pendingLength(): int` — returns the number of characters currently buffered but not yet flushed
- `wrapped(): Writer` — returns the wrapped `Writer`
- `syncMutex(): Mutex` — returns the sync mutex used by this stream
- `discardBuffer(): void` — discards the buffered content without flushing it
- `toString(): string`

### Free Functions

#### `syncStream(wrapped: Writer): SyncStream`

Convenience constructor mirroring `std::osyncstream(buf)` where `buf` is the wrapped output buffer.

#### `syncStreamWithMutex(wrapped: Writer, syncMutex: Mutex): SyncStream`

Convenience constructor that creates a `SyncStream` sharing the given mutex.

#### `emitAtomic(wrapped: Writer, syncMutex: Mutex, text: string): void`

Atomically writes a complete string to the wrapped `Writer` using the shared mutex. Equivalent to constructing a `SyncStream`, writing, and immediately closing it.

#### `emitLineAtomic(wrapped: Writer, syncMutex: Mutex, text: string): void`

Atomically writes a string followed by a newline to the wrapped `Writer`.

## Usage Examples

### Buffered Atomic Output

```titrate
import tt::io::SyncStream;
import tt::io::IO;

public fn main(): void {
    let s: SyncStream = SyncStream.syncStream(IO.stdout());
    s.writeLine("line one");
    s.writeLine("line two");
    s.write("partial ");
    s.write("line three");
    s.flush();   // all buffered content is written atomically
    s.close();
}
```

### Coordinating Multiple Writers

When multiple `SyncStream`s target the same destination, share a mutex so their flushes serialize:

```titrate
import tt::io::SyncStream;
import tt::io::IO;
import tt::concurrent::Mutex;

let sharedMutex: Mutex = new Mutex();
let a: SyncStream = SyncStream.syncStreamWithMutex(IO.stdout(), sharedMutex);
let b: SyncStream = SyncStream.syncStreamWithMutex(IO.stdout(), sharedMutex);

a.writeLine("from stream A");
b.writeLine("from stream B");
a.flush();  // writes "from stream A\n" atomically
b.flush();  // writes "from stream B\n" atomically
```

### One-Shot Atomic Emission

For a single atomic write, use the free function:

```titrate
import tt::io::SyncStream;
import tt::io::IO;
import tt::concurrent::Mutex;

let m: Mutex = new Mutex();
SyncStream.emitLineAtomic(IO.stdout(), m, "this whole line appears at once");
```

### Discarding Buffered Content

```titrate
import tt::io::SyncStream;
import tt::io::IO;

let s: SyncStream = SyncStream.syncStream(IO.stdout());
s.writeLine("do not print this");
s.discardBuffer();
io::println(s.pendingLength());  // 0
```
