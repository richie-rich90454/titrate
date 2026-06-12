# mmap

The `tt.io` module provides memory-mapped file I/O. Mmap allows you to map a file directly into memory for efficient random access and modification.

```titrate
import tt::io::Mmap;
```

## Mmap

Memory-mapped file for efficient random access.

- `fn init(path: string)` — open and map a file
- `get(offset: int): int` — get byte at offset
- `set(offset: int, value: int): void` — set byte at offset
- `size(): int` — get mapped size
- `flush(): void` — flush changes to disk
- `close(): void` — unmap and close
- `isClosed(): bool` — check if closed
- `asString(): string` — read entire mapping as string

```titrate
let m = new Mmap("data.bin");
let len: int = m.size();
let firstByte: int = m.get(0);
m.set(0, 255);
m.flush();
let contents: string = m.asString();
m.close();
```
