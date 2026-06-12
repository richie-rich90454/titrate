# gzip

The `tt.compression` module provides `Gzip` — gzip compression and decompression utilities.

```titrate
import tt.compression.Gzip;
```

## Free Functions

- `Gzip.compress(data: string): string` — compress data using the gzip format
- `Gzip.decompress(data: string): string` — decompress gzip-compressed data
- `Gzip.isGzip(data: string): bool` — check if data starts with the gzip magic header bytes

```titrate
let original: string = "hello world, this is some text to compress";
let compressed: string = Gzip.compress(original);
let decompressed: string = Gzip.decompress(compressed);
io::println(decompressed);  // "hello world, this is some text to compress"

io::println(Boolean.toString(Gzip.isGzip(compressed)));  // true
io::println(Boolean.toString(Gzip.isGzip(original)));     // false
```
