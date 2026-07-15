# StreamBuf

The `tt::io::StreamBuf` module provides a runtime analog of C++ `<streambuf>` (`basic_streambuf`). It defines a `StreamBuf` interface with the virtual `setbuf`/`seekoff`/`seekpos`/`sync`/`overflow`/`underflow`/`pbackfail` operations, get/put area pointer accessors, and three concrete implementations: `FileBuf` (wraps a `File`), `StringBuf` (wraps a string), and `PipeBuf` (wraps a `PipeReader`/`PipeWriter` pair).

## Import

```titrate
import tt::io::StreamBuf;
```

## Constants

The module exposes seek direction and open mode constants mirroring `std::ios_base::seekdir` and `std::ios_base::openmode`.

**Seek directions:**
- `SEEK_BEG: int = 0` — seek from beginning
- `SEEK_CUR: int = 1` — seek from current position
- `SEEK_END: int = 2` — seek from end

**Open modes:**
- `OPEN_IN: int = 1` — open for input
- `OPEN_OUT: int = 2` — open for output
- `OPEN_ATE: int = 4` — seek to end on open
- `OPEN_APP: int = 8` — append mode
- `OPEN_TRUNC: int = 16` — truncate on open
- `OPEN_BINARY: int = 32` — binary mode

## API Reference

### `StreamBuf` (interface)

Abstract buffer interface. Default implementations return failure values matching C++ `basic_streambuf` semantics (`traits::eof() == -1` for char streams).

**Methods (all have default implementations returning failure values):**
- `setbuf(buffer: ArrayList<string>, size: int): StreamBuf` — set the buffer used by the stream. Returns `this` on success, `null` on failure.
- `seekoff(off: int, dir: int, which: int): int` — seek by offset relative to `dir` (`SEEK_BEG`/`SEEK_CUR`/`SEEK_END`), affecting input (`which & OPEN_IN`) and/or output (`which & OPEN_OUT`). Returns the new absolute position, or `-1` on failure.
- `seekpos(pos: int, which: int): int` — seek to absolute position. Returns the new position, or `-1` on failure.
- `sync(): int` — synchronize the buffer with the associated sequence. Returns `0` on success, `-1` on failure.
- `overflow(c: int): int` — write character `c` to the output sequence. Returns `c` on success, `-1` on failure.
- `underflow(): int` — read a character from the input sequence without advancing. Returns the character, or `-1` at end of input.
- `pbackfail(c: int): int` — put back a character to the input sequence. Returns `c` on success, `-1` on failure.

### `FileBuf`

Concrete `StreamBuf` wrapping a `File`. Maintains get/put area pointers (`eback`/`gptr`/`egptr`/`pbase`/`pptr`/`epptr`) as positions into the underlying file.

**Constructors:**
- `init(path: string, mode: string)` — open a file at `path` with the given mode
- `init(file: File)` — wrap an existing `File` (not owned by the buffer)

**Pointer accessors:**
- `eback(): int`, `gptr(): int`, `egptr(): int` — get-area pointers
- `pbase(): int`, `pptr(): int`, `epptr(): int` — put-area pointers
- `setg(back: int, gptr: int, egptr: int): void` — set get-area pointers
- `setp(pbase: int, pptr: int, epptr: int): void` — set put-area pointers
- `gbump(n: int): void` — advance the get pointer by `n`
- `pbump(n: int): void` — advance the put pointer by `n`

**Methods:** `setbuf`, `seekoff`, `seekpos`, `sync`, `overflow`, `underflow`, `pbackfail` (as above), plus:
- `close(): void` — close the buffer, flushing pending writes; releases the file if owned
- `isClosed(): bool`

### `StringBuf`

Concrete `StreamBuf` wrapping an in-memory string. Supports both read and write modes; writes append to an internal `StringBuilder`.

**Constructors:**
- `init()` — empty buffer
- `init(source: string)` — buffer initialized with a source string for reading

**Pointer accessors:** Same as `FileBuf` (`eback`/`gptr`/`egptr`/`pbase`/`pptr`/`epptr`, `setg`/`setp`/`gbump`/`pbump`).

**Methods:**
- `str(s: string): void` — set the source string (replaces the get area)
- `str(): string` — return the contents of the put area as a string
- `sgetc(): int` — advance the get pointer and return the character read
- `setbuf`, `seekoff`, `seekpos`, `sync`, `overflow`, `underflow`, `pbackfail`
- `close(): void`, `isClosed(): bool`

### `PipeBuf`

Concrete `StreamBuf` wrapping a `PipeReader`/`PipeWriter` pair. Reads come from the `PipeReader`; writes go to the `PipeWriter`. Pipes do not support seeking.

**Constructors:**
- `init()` — creates a fresh pipe pair
- `init(reader: PipeReader, writer: PipeWriter)` — wraps an existing pipe pair

**Methods:**
- `getReader(): PipeReader` — returns the pipe reader
- `getWriter(): PipeWriter` — returns the pipe writer
- Pointer accessors and `setbuf`/`seekoff` (returns `-1`)/`seekpos` (returns `-1`)/`sync`/`overflow`/`underflow`/`pbackfail`
- `close(): void`, `isClosed(): bool`

### Free Functions

#### `openFile(path: string, mode: string): FileBuf`

Opens a `FileBuf` on the given path with the given mode.

#### `newStringBuf(): StringBuf`

Creates an empty `StringBuf`.

#### `newStringBuf(source: string): StringBuf`

Creates a `StringBuf` initialized with the given source string.

#### `newPipeBuf(): PipeBuf`

Creates a `PipeBuf` with a fresh pipe pair.

## Usage Examples

### Reading a File with FileBuf

```titrate
import tt::io::StreamBuf;

public fn main(): void {
    let buf: FileBuf = StreamBuf.openFile("data.txt", "r");
    let ch: int = buf.underflow();   // peek at the first character
    while (ch != -1) {
        io::println("char: " + ch);
        buf.gbump(1);
        ch = buf.underflow();
    }
    buf.close();
}
```

### In-Memory String Buffer

```titrate
import tt::io::StreamBuf;

let sb: StringBuf = StreamBuf.newStringBuf("hello world");
// Read characters
let c: int = sb.underflow();   // peek 'h' (104)
sb.gbump(1);
let c2: int = sb.underflow();  // peek 'e' (101)

// Write characters
sb.overflow(65);  // write 'A'
sb.overflow(66);  // write 'B'
io::println(sb.str());  // "AB"
```

### Pipe Buffer for Inter-Thread Communication

```titrate
import tt::io::StreamBuf;

let pb: PipeBuf = StreamBuf.newPipeBuf();
pb.overflow(72);   // write 'H'
pb.overflow(105);  // write 'i'
let ch: int = pb.underflow();  // read 'H' (72)
io::println(ch);
pb.close();
```

### Seeking Within a String Buffer

```titrate
import tt::io::StreamBuf;

let sb: StringBuf = StreamBuf.newStringBuf("abcdef");
// Seek to position 3 in the input area
sb.seekoff(3, StreamBuf.SEEK_BEG, StreamBuf.OPEN_IN);
let ch: int = sb.underflow();  // 'd' (100)
```
