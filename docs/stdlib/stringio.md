# stringio

The `tt.io` module provides `StringReader` and `StringWriter` — in-memory stream classes for reading from and writing to strings.

```titrate
import tt.io.StringReader;
import tt.io.StringWriter;
```

## StringReader

A reader that consumes a string line-by-line or in full, useful for parsing text without file I/O.

- `fn init(data: string)` — create a reader from a string
- `readLine(): string` — read the next line (excluding the newline)
- `readAll(): string` — read all remaining content
- `hasNext(): bool` — check if there is more data to read
- `close(): void` — close the reader

```titrate
let reader: StringReader = new StringReader("line one\nline two\nline three");

while (reader.hasNext()) {
    io::println(reader.readLine());
}
// Output:
// line one
// line two
// line three

reader.close();
```

## StringWriter

A writer that accumulates output in an internal buffer, useful for building strings incrementally.

- `fn init()` — create an empty writer
- `write(data: string): void` — append a string
- `writeLine(data: string): void` — append a string followed by a newline
- `toString(): string` — get the accumulated content
- `close(): void` — close the writer

```titrate
let writer: StringWriter = new StringWriter();
writer.writeLine("Name: Alice");
writer.writeLine("Age: 30");
writer.write("---");
io::println(writer.toString());
// Name: Alice
// Age: 30
// ---

writer.close();
```
