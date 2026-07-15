# Lzma

The `tt.compression.Lzma` module provides LZMA/XZ compression and decompression. It mirrors Python's `lzma` module, exposing module-level `compress`/`decompress`/`detectFormat`/`isLzma`, plus the `LZMACompressor`/`LZMADecompressor`/`LZMAFile` classes for incremental and file-based access. The implementation delegates to the native `Lzma_compress` / `Lzma_decompress` runtime functions, falling back to a pass-through wrapper when the native runtime is unavailable.

## Import

```titrate
import tt::compression::Lzma;
```

## Constants

### Container formats

- `Lzma.FORMAT_XZ: int = 0` — XZ container (default)
- `Lzma.FORMAT_ALONE: int = 1` — legacy `.lzma` alone format
- `Lzma.FORMAT_RAW: int = 2` — raw LZMA stream (no container)
- `Lzma.FORMAT_AUTO: int = 3` — auto-detect from magic bytes (default for decompression)

### Integrity checks

- `Lzma.CHECK_NONE: int = 0`
- `Lzma.CHECK_CRC32: int = 1`
- `Lzma.CHECK_CRC64: int = 4`
- `Lzma.CHECK_SHA256: int = 10`
- `Lzma.CHECK_ID_MAX: int = 15`
- `Lzma.CHECK_UNKNOWN: int = -1`
- `Lzma.CHECK_DEFAULT: int = CHECK_CRC64`

### Compression presets

- `Lzma.PRESET_DEFAULT: int = 6` — default preset (1..9)
- `Lzma.PRESET_EXTREME: int = 2147483648` — ORed with a preset for slower but better compression

## Functions

### compress

- `Lzma.compress(data: string, format: int, preset: int): string` — compress `data` using LZMA/XZ. `format` selects the container (`FORMAT_XZ` default), `preset` is the compression preset (1..9, optionally ORed with `PRESET_EXTREME`). Falls back to a pass-through wrapper if the native runtime is unavailable.
- `Lzma.compress(data: string): string` — compress with default format (XZ) and preset

```titrate
let data: string = "Hello, world! Hello, world! Hello, world!";
let compressed: string = Lzma.compress(data);
```

### decompress

- `Lzma.decompress(data: string, format: int): string` — decompress `data` previously produced by `compress()` (or by an external XZ/LZMA encoder). `format` may be `FORMAT_AUTO` to auto-detect, or a specific format.
- `Lzma.decompress(data: string): string` — decompress with auto-detected format

```titrate
let restored: string = Lzma.decompress(compressed);
```

### detectFormat

- `Lzma.detectFormat(data: string): int` — detect the container format from the leading magic bytes. Returns one of `FORMAT_XZ`, `FORMAT_ALONE`, `FORMAT_RAW`, or `FORMAT_AUTO` when the format cannot be determined.

### isLzma

- `Lzma.isLzma(data: string): bool` — true when `data` looks like an XZ or LZMA-alone stream

## Classes

### LZMACompressor

Accumulates chunks via `compress()` and produces the final stream via `flush()`. The implementation holds an internal buffer of input bytes and compresses them all at `flush()` time.

**Fields:**
- `format: int`
- `check: int`
- `preset: int`

**Constructors:**
- `init()` — defaults to `FORMAT_XZ`, `CHECK_DEFAULT`, `PRESET_DEFAULT`
- `initWithFormat(format: int, preset: int)` — explicit format and preset

**Methods:**
- `compress(data: string): string` — feed `data` into the compressor; returns whatever output is available (often empty until `flush()`). Throws if called after `flush()`.
- `flush(): string` — flush the compressor and return the final compressed output. After this call, `compress()` will throw.

```titrate
let c: LZMACompressor = new LZMACompressor();
c.compress("chunk1");
c.compress("chunk2");
let result: string = c.flush();
```

### LZMADecompressor

Accepts compressed chunks via `decompress()` and yields the decoded bytes incrementally.

**Constructors:**
- `init()` — defaults to `FORMAT_AUTO`
- `initWithFormat(format: int)` — explicit format

**Methods:**
- `decompress(data: string, eof: bool): string` — feed more compressed bytes; returns the next slice of decompressed output (may be empty). When `eof` is `true`, treats `data` as the final chunk.
- `eof(): bool` — true once the decompressor has produced all output
- `unusedData(): string` — any unused tail bytes after the end of the compressed stream

### LZMAFile

File-like sequential access to an LZMA/XZ stream.

**Constructors:**
- `init(path: string, mode: string)` — open `path` in `"r"`/`"rb"` (read, decompressing on open) or `"w"`/`"wb"` (write, accumulating into an `LZMACompressor`)

**Methods:**
- `read(size: int): string` — read up to `size` bytes from the decompressed stream (`-1` for all remaining)
- `readAll(): string` — read the entire decompressed stream
- `readline(): string` — read a single line up to and including `\n`
- `write(data: string): void` — append `data` to the write buffer (write mode only)
- `close(): void` — close the file; for write mode, flushes the compressor and writes the compressed output
- `closed(): bool` — true if the file is closed

## Usage Example

```titrate
import tt::compression::Lzma;

public fn main(): void {
    let original: string = "The quick brown fox jumps over the lazy dog. ".repeat(20);
    let compressed: string = Lzma.compress(original);
    io::println("Original: " + Integer.toString(String.length(original)) + " bytes");
    io::println("Compressed: " + Integer.toString(String.length(compressed)) + " bytes");
    let restored: string = Lzma.decompress(compressed);
    io::println("Restored matches: " + Boolean.toString(restored == original));
}
```
