# FileInput

The `tt.io.FileInput` module provides a Python `fileinput` analog for lazy line iteration over files: `FileInput`, `input`, `filename`, `lineno`, `filelineno`, `isfirstline`, `isstdin`, `nextfile`, `close`, and `readline`.

## Import

```titrate
import tt::io::FileInput;
```

## Class: FileInput

Iterates over the lines of a list of files (or stdin if no files given).

**Fields:**
- `files: ArrayList<string>`
- `mode: string`
- `openHook: bool`
- `fileIndex: int`
- `currentFilename: string`
- `lineNumber: int` — cumulative across all files
- `fileLineNumber: int` — within the current file
- `isFirstLine: bool`
- `isStdin: bool`

**Methods:**
- `init(files: ArrayList<string>, mode: string)`
- `nextfile(): bool` — skip remainder of current file and move to next
- `nextLine(): string` — read and return the next line, or `null` at end of all input
- `close(): void` — close the input stream and reset state
- `filename(): string` — current filename, or `""` if reading from stdin / no file open
- `lineno(): int` — cumulative line number across all files
- `filelineno(): int` — line number within the current file
- `isfirstline(): bool` — `true` if the current line is the first in its file
- `isstdin(): bool` — `true` if the current line was read from stdin

```titrate
let files = new ArrayList<string>();
files.add("a.txt"); files.add("b.txt");
let fi: FileInput = new FileInput(files, "r");
var line: string = fi.nextLine();
while (line != null) {
    io::println(fi.filename() + ":" + fi.filelineno() + ": " + line);
    line = fi.nextLine();
}
fi.close();
```

## Functions

### input

Create and activate a `FileInput` over the given files. If `files` is empty or `null`, reads from stdin.

**Parameters:** `files: ArrayList<string>`, `mode: string`
**Returns:** `FileInput`

### filename / lineno / filelineno / isfirstline / isstdin

Module-level shortcuts that delegate to the currently-active `FileInput` instance. Return `""` / `0` / `false` if no input is active.

### close

Close the active `FileInput` instance.

### nextfile

Skip the remainder of the current file and move to the next. Returns `false` if no more files.

### readline

Convenience: read and return the next line from the active input. Returns `null` at end of all input.

```titrate
let files = new ArrayList<string>();
files.add("data.txt");
input(files, "r");
var line: string = readline();
while (line != null) {
    io::println(line);
    line = readline();
}
close();
```
