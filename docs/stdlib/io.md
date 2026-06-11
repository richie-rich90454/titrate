# io

The `tt.io` and `tt.file` modules provide file system operations, buffered reading/writing, and path manipulation.

```titrate
import tt.io.IO;
import tt.io.File;
import tt.io.BufferedReader;
import tt.io.FileReader;
import tt.io.FileWriter;
import tt.file.Path;
import tt.file.Directory;
```

## IO

Static methods for standard input, output, and error streams.

- `IO.println(message: String): void` — print to stdout with newline
- `IO.print(message: String): void` — print to stdout without newline
- `IO.readLine(): String` — read a line from stdin
- `IO.readAll(): String` — read all of stdin
- `IO.eprintln(msg: String): void` — print to stderr with newline
- `IO.eprint(msg: String): void` — print to stderr without newline

```titrate
IO.println("Hello, world!");
String input = IO.readLine();
```

## File

File operations backed by VM built-ins.

- `File(path: String)` — create a file handle
- `getPath(): String` — return the file path
- `readLine(): String` — read one line
- `readAll(): String` — read entire file contents
- `write(content: String): void` — write content (overwrites)
- `append(content: String): void` — append content
- `exists(): bool` — check if file exists
- `isFile(): bool` — check if path is a file
- `isDirectory(): bool` — check if path is a directory
- `length(): int` — file size in bytes
- `delete(): bool` — delete the file
- `renameTo(newPath: String): bool` — rename the file
- `copyTo(dest: String): bool` — copy to destination
- `moveTo(dest: String): bool` — move to destination
- `close(): void` — close the file handle

```titrate
let f = new File("data.txt");
if (f.exists()) {
    String contents = f.readAll();
    IO.println(contents);
}
```

## BufferedReader

Buffered reader with line-by-line reading support. Implements `Reader`.

- `BufferedReader.open(path: String): BufferedReader` — open for reading
- `read(): String` — read next line
- `readLine(): String` — read next line
- `readAll(): String` — read entire contents
- `lines(): ArrayList<String>` — read all lines into a list
- `close(): void` — close the reader

```titrate
let reader = BufferedReader.open("input.txt");
ArrayList<String> lines = reader.lines();
reader.close();
```

## FileReader

Simple file reader backed by VM built-ins. Implements `Reader`.

- `FileReader.open(path: String): FileReader` — open for reading
- `read(): String` — read next line
- `readLine(): String` — read next line
- `readAll(): String` — read entire contents
- `close(): void` — close the reader

## FileWriter

File writer backed by VM built-ins. Implements `Writer`.

- `FileWriter.open(path: String): FileWriter` — open for writing
- `write(text: String): void` — write text
- `writeLine(text: String): void` — write text followed by newline
- `flush(): void` — flush buffered data
- `close(): void` — close the writer

```titrate
let writer = FileWriter.open("output.txt");
writer.writeLine("first line");
writer.writeLine("second line");
writer.close();
```

## Path

Path manipulation operations backed by VM built-ins.

- `Path(p: String)` — create a path handle
- `join(other: String): String` — join with another path component
- `basename(): String` — file name portion
- `dirname(): String` — directory portion
- `extension(): String` — file extension
- `exists(): bool` — check if path exists
- `isFile(): bool` — check if path is a file
- `isDir(): bool` — check if path is a directory
- `absolutePath(): String` — resolve to absolute path
- `canonicalPath(): String` — resolve to canonical path
- `resolve(other: String): String` — resolve against this path
- `relativize(other: String): String` — relative path from this to other
- `normalize(): String` — normalize path (remove `.` and `..`)
- `parent(): String` — parent directory
- `fileName(): String` — file name (alias for basename)

```titrate
let p = new Path("/home/user/docs/file.txt");
io::println(p.extension());  // "txt"
io::println(p.basename());   // "file.txt"
io::println(p.dirname());    // "/home/user/docs"
```

## Directory

Directory operations backed by VM built-ins.

- `Directory(p: String)` — create a directory handle
- `list(): ArrayList<String>` — list immediate children
- `walk(): ArrayList<String>` — list all files recursively
- `listFiles(): ArrayList<String>` — list only files
- `create(): bool` — create the directory
- `remove(): bool` — remove the directory
- `copy(dest: String): bool` — copy to destination
- `move(dest: String): bool` — move to destination
- `exists(): bool` — check if directory exists
- `size(): int` — total size in bytes
- `isEmpty(): bool` — true if empty
- `getCurrent(): String` — static: get current working directory
- `setCurrent(path: String): void` — static: change working directory

```titrate
let dir = new Directory("/tmp/project");
ArrayList<String> files = dir.walk();
```
