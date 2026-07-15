# SourceLocation

The `tt::lang::SourceLocation` module provides C++ `<source_location>` parity. It represents a location in source code (file, line, column, function). `SourceLocation.current()` captures the call site. Because the Titrate VM does not yet expose a native `__builtin_source_location`, `current()` returns a location backed by `Traceback.Frame` when one is supplied, or a default `"unknown"` location otherwise. The API mirrors `std::source_location`.

## Import

```titrate
import tt::lang::SourceLocation;
```

## API Reference

### `SourceLocation`

Represents a source code location: file name, line, column, and function name.

**Fields:**
- `fileName: string`
- `line: int`
- `column: int`
- `functionName: string`

**Constructor:**
- `init()` — creates an empty location (all fields zeroed/empty)

**Static/Factory Methods:**
- `current(): SourceLocation` — `std::source_location::current()` analog. Returns a default `"unknown"` location since the VM does not yet expose native call-site capture.
- `fromFrame(frame: Traceback.Frame): SourceLocation` — build a `SourceLocation` from a `Traceback.Frame` (column defaults to 0)
- `make(fileName: string, line: int, column: int, functionName: string): SourceLocation` — create with explicit values

**C++ snake_case accessors (API parity):**
- `file_name(): string` — returns `fileName`
- `function_name(): string` — returns `functionName`
- `line_number(): int` — returns `line`
- `column_number(): int` — returns `column`

**camelCase accessors (Titrate convention):**
- `getFileName(): string`
- `getFunctionName(): string`
- `getLine(): int`
- `getColumn(): int`
- `setFileName(name: string): void`
- `setLine(line: int): void`
- `setColumn(column: int): void`
- `setFunctionName(name: string): void`

**Other methods:**
- `equals(other: SourceLocation): bool`
- `toString(): string` — returns `"file:line:column [function]"`

### Free Functions

#### `of(fileName: string, line: int, column: int, functionName: string): SourceLocation`

Create a `SourceLocation` with the given values.

#### `fromFrame(frame: Traceback.Frame): SourceLocation`

Create a `SourceLocation` from a `Traceback.Frame`.

#### `current(): SourceLocation`

Return a default `"unknown"` `SourceLocation` (`std::source_location::current()` analog).

#### `defaultLocation(): SourceLocation`

Return a default/empty `SourceLocation` (`std::source_location` default constructor).

#### `isValid(loc: SourceLocation): bool`

Validate that a `SourceLocation` has a meaningful value (non-empty file name that is not `"unknown"`).

## Usage Examples

### Creating a SourceLocation

```titrate
import tt::lang::SourceLocation;
import tt::io::IO;

public fn main(): void {
    let loc: SourceLocation = SourceLocation.of("main.tr", 42, 5, "main");
    IO.println(loc.file_name());       // main.tr
    IO.println(loc.line_number());     // 42
    IO.println(loc.column_number());   // 5
    IO.println(loc.function_name());   // main
    IO.println(loc.toString());        // main.tr:42:5 [main]
}
```

### Capturing the Current Location

```titrate
import tt::lang::SourceLocation;

let here: SourceLocation = SourceLocation.current();
io::println(here.file_name());   // unknown (until native capture is available)
io::println(SourceLocation.isValid(here));  // false
```

### Building from a Traceback Frame

```titrate
import tt::lang::SourceLocation;
import tt::lang::Traceback;

try {
    throw "test error";
} catch (e: string) {
    let frames: ArrayList<Traceback.Frame> = Traceback.extract();
    if (frames.size() > 0) {
        let loc: SourceLocation = SourceLocation.fromFrame(frames.get(0));
        io::println(loc.toString());  // file:line:0 [function]
    }
}
```

### Logging with Source Locations

```titrate
import tt::lang::SourceLocation;

public fn log(message: string, loc: SourceLocation): void {
    io::println("[" + loc.toString() + "] " + message);
}

public fn doWork(): void {
    let loc: SourceLocation = SourceLocation.of("worker.tr", 10, 1, "doWork");
    log("starting work", loc);
}
```
