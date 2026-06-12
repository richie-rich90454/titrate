# compression

The `tt.compression` module provides data compression and archive utilities. Zlib offers raw compression, while ZipFile handles ZIP archive reading and writing.

```titrate
import tt.compression.Zlib;
import tt.compression.ZipFile;
```

## Zlib

Compression and decompression using the zlib and gzip formats.

- `Zlib.compress(data: string): string` — compress data using the zlib format
- `Zlib.decompress(data: string): string` — decompress zlib-compressed data
- `Zlib.gzipCompress(data: string): string` — compress data using the gzip format
- `Zlib.gzipDecompress(data: string): string` — decompress gzip-compressed data

```titrate
let original: string = "hello world, this is some text to compress";
let compressed: string = Zlib.compress(original);
let decompressed: string = Zlib.decompress(compressed);
io::println(decompressed);  // "hello world, this is some text to compress"

let gz: string = Zlib.gzipCompress(original);
let ungz: string = Zlib.gzipDecompress(gz);
io::println(ungz);  // "hello world, this is some text to compress"
```

## ZipFile

Read and write ZIP archive files.

- `ZipFile.open(path: string): ZipFile` — open an existing ZIP archive for reading
- `ZipFile.create(path: string): ZipFile` — create a new ZIP archive for writing
- `namelist(): ArrayList<string>` — list all entry names in the archive
- `read(name: string): string` — read the contents of an entry by name
- `write(name: string, data: string): void` — add or update an entry in the archive
- `close(): void` — close the archive

```titrate
let zf: ZipFile = ZipFile.create("archive.zip");
zf.write("hello.txt", "Hello, world!");
zf.write("data.txt", "Some data here");
zf.close();

let reader: ZipFile = ZipFile.open("archive.zip");
let names: ArrayList<string> = reader.namelist();
let content: string = reader.read("hello.txt");
io::println(content);  // "Hello, world!"
reader.close();
```
