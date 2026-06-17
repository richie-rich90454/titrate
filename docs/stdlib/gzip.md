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

## Streaming Decompression

- `Gzip.decompressStream(input: string, chunkSize: int): string` — streaming decompression for large files

## Multi-Member Gzip

- `Gzip.decompressAllMembers(data: ArrayList<byte>): ArrayList<ArrayList<byte>>` — decompress all gzip members in concatenated file

## Compression Level

- `Gzip.compressWithLevel(data: ArrayList<byte>, level: int): ArrayList<byte>` — compress with specified level (1-9)
