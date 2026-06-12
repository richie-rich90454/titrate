# logging

The `tt.logging` module provides a flexible logging framework with configurable log levels, formatters, and output handlers.

```titrate
import tt.logging;
```

## Log Levels

| Level | Value | Description |
|-------|-------|-------------|
| `LogLevel.TRACE` | 0 | Fine-grained diagnostic events |
| `LogLevel.DEBUG` | 1 | Debugging information |
| `LogLevel.INFO` | 2 | General informational messages |
| `LogLevel.WARN` | 3 | Warning conditions |
| `LogLevel.ERROR` | 4 | Error conditions |
| `LogLevel.FATAL` | 5 | Fatal errors requiring shutdown |

## Logger

### Creating a Logger

- `logging.getLogger(name: string): Logger` — get or create a named logger

```titrate
let log = logging.getLogger("myapp");
```

### Setting the Log Level

- `.setLevel(level: LogLevel): void` — set the minimum level this logger will output

```titrate
let log = logging.getLogger("myapp");
log.setLevel(LogLevel.DEBUG);
```

### Logging Messages

- `.trace(message: string): void` — log at TRACE level
- `.debug(message: string): void` — log at DEBUG level
- `.info(message: string): void` — log at INFO level
- `.warn(message: string): void` — log at WARN level
- `.error(message: string): void` — log at ERROR level
- `.fatal(message: string): void` — log at FATAL level

```titrate
let log = logging.getLogger("myapp");
log.info("Application started");
log.debug("Processing item " + Integer.toString(id));
log.warn("Low memory");
log.error("Failed to open file: " + path);
```

### Logging with Exceptions

- `.error(message: string, err: string): void` — log an error with an associated error detail

```titrate
let result = File.readFile("missing.txt");
match result {
    Ok(content) => io::println(content),
    Err(e) => log.error("Could not read file", e),
}
```

## Handlers

Handlers determine where log messages are written. By default, loggers write to stdout.

### ConsoleHandler

- `new ConsoleHandler()` — create a handler that writes to stdout
- `new ConsoleHandler(level: LogLevel)` — create with a minimum level

### FileHandler

- `new FileHandler(path: string)` — create a handler that writes to a file
- `new FileHandler(path: string, level: LogLevel)` — create with a minimum level

### Adding Handlers

- `.addHandler(handler: Handler): void` — add a handler to the logger

```titrate
let log = logging.getLogger("myapp");
log.addHandler(new FileHandler("app.log"));
log.addHandler(new ConsoleHandler(LogLevel.WARN));
```

## Formatters

Formatters control the appearance of log messages.

### SimpleFormatter

- `new SimpleFormatter()` — format as `LEVEL: message`

### PatternFormatter

- `new PatternFormatter(pattern: string)` — format using a pattern string

Pattern tokens:

| Token | Replacement |
|-------|-------------|
| `%d` | Date and time |
| `%l` | Log level |
| `%n` | Logger name |
| `%m` | Log message |
| `%t` | Thread name |

```titrate
let formatter = new PatternFormatter("[%d] [%l] %n: %m");
let handler = new ConsoleHandler();
handler.setFormatter(formatter);
```

### Setting Formatters

- `.setFormatter(formatter: Formatter): void` — set the formatter on a handler

## Configuration

### Root Logger

- `logging.setRootLevel(level: LogLevel): void` — set the default level for all loggers

```titrate
logging.setRootLevel(LogLevel.INFO);
```

### Global Configuration

- `logging.configure(level: LogLevel, handler: Handler): void` — set the root level and handler in one call

## Complete Example

```titrate
import tt.logging;

public fn main(): void {
    logging.setRootLevel(LogLevel.DEBUG);

    let log = logging.getLogger("server");
    let fileHandler = new FileHandler("server.log");
    fileHandler.setFormatter(new PatternFormatter("[%d] [%l] %m"));
    log.addHandler(fileHandler);

    log.info("Server starting on port 8080");

    // ... server logic ...

    log.warn("Connection pool running low");
    log.error("Failed to bind port 8080");
}
```
