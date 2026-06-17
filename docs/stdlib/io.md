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

- `IO.println(message: string): void` — print to stdout with newline
- `IO.print(message: string): void` — print to stdout without newline
- `IO.readLine(): string` — read a line from stdin
- `IO.readAll(): string` — read all of stdin
- `IO.eprintln(msg: string): void` — print to stderr with newline
- `IO.eprint(msg: string): void` — print to stderr without newline

```titrate
IO.println("Hello, world!");
let input: string = IO.readLine();
```

## File

File operations backed by VM built-ins.

- `fn init(path: string)` — create a file handle
- `getPath(): string` — return the file path
- `readLine(): string` — read one line
- `readAll(): string` — read entire file contents
- `write(content: string): void` — write content (overwrites)
- `append(content: string): void` — append content
- `exists(): bool` — check if file exists
- `isFile(): bool` — check if path is a file
- `isDirectory(): bool` — check if path is a directory
- `length(): int` — file size in bytes
- `delete(): bool` — delete the file
- `renameTo(newPath: string): bool` — rename the file
- `copyTo(dest: string): bool` — copy to destination
- `moveTo(dest: string): bool` — move to destination
- `close(): void` — close the file handle

```titrate
let f = new File("data.txt");
if (f.exists()) {
    let contents: string = f.readAll();
    IO.println(contents);
}
```

## BufferedReader

Buffered reader with line-by-line reading support. Implements `Reader`.

- `BufferedReader.open(path: string): BufferedReader` — open for reading
- `read(): string` — read next line
- `readLine(): string` — read next line
- `readAll(): string` — read entire contents
- `lines(): ArrayList<string>` — read all lines into a list
- `close(): void` — close the reader

```titrate
let reader = BufferedReader.open("input.txt");
let lines: ArrayList<string> = reader.lines();
reader.close();
```

## FileReader

Simple file reader backed by VM built-ins. Implements `Reader`.

- `FileReader.open(path: string): FileReader` — open for reading
- `read(): string` — read next line
- `readLine(): string` — read next line
- `readAll(): string` — read entire contents
- `close(): void` — close the reader

## FileWriter

File writer backed by VM built-ins. Implements `Writer`.

- `FileWriter.open(path: string): FileWriter` — open for writing
- `write(text: string): void` — write text
- `writeLine(text: string): void` — write text followed by newline
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

- `fn init(p: string)` — create a path handle
- `join(other: string): string` — join with another path component
- `basename(): string` — file name portion
- `dirname(): string` — directory portion
- `extension(): string` — file extension
- `exists(): bool` — check if path exists
- `isFile(): bool` — check if path is a file
- `isDir(): bool` — check if path is a directory
- `absolutePath(): string` — resolve to absolute path
- `canonicalPath(): string` — resolve to canonical path
- `resolve(other: string): string` — resolve against this path
- `relativize(other: string): string` — relative path from this to other
- `normalize(): string` — normalize path (remove `.` and `..`)
- `parent(): string` — parent directory
- `fileName(): string` — file name (alias for basename)

```titrate
let p = new Path("/home/user/docs/file.txt");
io::println(p.extension());  // "txt"
io::println(p.basename());   // "file.txt"
io::println(p.dirname());    // "/home/user/docs"
```

## Directory

Directory operations backed by VM built-ins.

- `fn init(p: string)` — create a directory handle
- `list(): ArrayList<string>` — list immediate children
- `walk(): ArrayList<string>` — list all files recursively
- `listFiles(): ArrayList<string>` — list only files
- `create(): bool` — create the directory
- `remove(): bool` — remove the directory
- `copy(dest: string): bool` — copy to destination
- `move(dest: string): bool` — move to destination
- `exists(): bool` — check if directory exists
- `size(): int` — total size in bytes
- `isEmpty(): bool` — true if empty
- `getCurrent(): string` — static: get current working directory
- `setCurrent(path: string): void` — static: change working directory

```titrate
let dir = new Directory("/tmp/project");
let files: ArrayList<string> = dir.walk();
```

## FileWatcher

- `FileWatcher.watch(path: string, callback: fn(string, string): void): void` — watch directory for changes
- `FileWatcher.watchRecursive(path: string, callback: fn(string, string): void): void` — recursive directory watching
- `FileWatcher.stop(): void` — stop watching

## AsyncFile

- `AsyncFile.read(path: string, callback: fn(string): void): void` — non-blocking read
- `AsyncFile.write(path: string, content: string, callback: fn(): void): void` — non-blocking write
- `AsyncFile.readWithProgress(path: string, callback: fn(string, double): void): void` — read with progress tracking

## Pipe

- `PipeReader.init(path: string)` — create named pipe reader
- `PipeReader.read(): string` — read from pipe
- `PipeWriter.init(path: string)` — create named pipe writer
- `PipeWriter.write(data: string): void` — write to pipe

## FileLock

- `FileLock.shared(path: string): FileLock` — acquire shared lock
- `FileLock.exclusive(path: string): FileLock` — acquire exclusive lock
- `FileLock.tryLock(path: string, exclusive: bool): bool` — try to acquire lock
- `FileLock.lockWithTimeout(path: string, exclusive: bool, timeoutMs: int): bool` — lock with timeout
- `FileLock.release(): void` — release lock
