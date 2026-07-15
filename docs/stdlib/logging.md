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
switch result {
    case Ok(content) => io::println(content),
    case Err(e) => log.error("Could not read file", e),
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

## JSON Formatter

- `Logging.jsonFormatter(): Formatter` — create JSON log formatter
- JSON output includes: timestamp, level, logger, message, thread, exception

## Syslog Handler

- `Logging.syslogHandler(host: string, port: int): Handler` — create syslog handler
- `Logging.syslogHandler(facility: string): Handler` — create local syslog handler

## Email Handler

- `Logging.emailHandler(to: string, from: string, smtpHost: string, subject: string): Handler` — email handler for critical errors

## Rotating Handler

- `Logging.rotatingFileHandler(path: string, maxSizeBytes: long, backupCount: int): Handler` — rotating file handler
- `Logging.timedRotatingFileHandler(path: string, when: string, backupCount: int): Handler` — time-based rotation

## Per-Handler Level

- `Handler.setLevel(level: string): void` — set minimum level for this handler
- `Handler.getLevel(): string` — get current level

## Async Handler

- `Logging.asyncHandler(handler: Handler, queueSize: int): Handler` — wrap handler with async queue
- Async handler uses background thread to process log records

## Additional Handlers (Phase 1-2 parity)

These handlers round out the Python `logging.handlers` parity surface:

| Handler | Description |
|---------|-------------|
| `StreamHandler` | Like `ConsoleHandler` but writes to an arbitrary stream (file-like object). `new StreamHandler(stream: Variant)` |
| `NullHandler` | Discards all records. Use at the top of a library so `logging.getLogger("mylib")` never emits `No handlers could be found` warnings. `new NullHandler()` |
| `MemoryHandler` | Buffers records in memory and flushes them to a target handler in batch. `new MemoryHandler(capacity: int, target: Handler)` and `.flush(): void` |
| `SocketHandler` | Sends pickled `LogRecord`s over a TCP socket. `new SocketHandler(host: string, port: int)` |
| `DatagramHandler` | Like `SocketHandler` but uses UDP datagrams. `new DatagramHandler(host: string, port: int)` |
| `HTTPHandler` | Posts log records via HTTP(S) to a URL. `new HTTPHandler(host: string, url: string, method: string)` |
| `SMTPHandler` | Sends emails via SMTP; alias of the `Logging.emailHandler` factory above. `new SMTPHandler(host: string, port: int, from: string, to: ArrayList<string>, subject: string)` |
| `NTEventLogHandler` | Windows event log handler. `new NTEventLogHandler(appName: string)` (Windows only) |
| `WatchedFileHandler` | Reopens the log file if it is moved/rotated by an external program. `new WatchedFileHandler(path: string)` |

```titrate
let libLog = logging.getLogger("mylib");
libLog.addHandler(new NullHandler());  // quiet by default
```

## LogRecord

A structured log record carrying all the data captured at emit time. The default handlers consume `LogRecord`s but you can build and dispatch them directly, e.g. when writing a custom handler or shipping records off-process.

- `new LogRecord(name: string, level: int, message: string)` — construct a record
- `LogRecord.getMessage(): string` — the formatted message (after `%`-style substitution if args were supplied)
- `LogRecord.getName(): string`, `LogRecord.getLevel(): int`, `LogRecord.getLevelName(): string` — basic getters
- `LogRecord.getSourceName(): string` — module/function that emitted the record
- `LogRecord.getLineNo(): int` — source line number, when available
- `LogRecord.getTimestamp(): long` — milliseconds since the Unix epoch
- `LogRecord.getThreadId(): long`, `LogRecord.getThreadName(): string` — thread info
- `LogRecord.getException(): string` — formatted exception text, if any
- `LogRecord.getArgs(): ArrayList<Variant>` — positional arguments used for `%`-style message substitution

## Filter

Filters give finer-grained control than level-based filtering. A `Filter` decides whether a `LogRecord` should be dispatched to a handler.

- `new Filter(name: string)` — filter matching records whose logger name starts with `name` (so `"myapp"` matches `"myapp.db"` too)
- `Filter.filter(record: LogRecord): bool` — return true to keep the record
- `.addFilter(filter: Filter): void` — attach a filter to a `Logger` or `Handler`
- `.removeFilter(filter: Filter): void` — remove a previously added filter

```titrate
let noisy = new Filter("mylib.noisy");
let log = logging.getLogger("mylib");
log.addFilter(noisy);  // suppress records from the noisy submodule
```

## LoggerAdapter

Adapt a `Logger` so every emitted record is enriched with extra contextual fields (e.g. a request-id), mirroring Python's `logging.LoggerAdapter`.

- `new LoggerAdapter(logger: Logger, extra: HashMap<string, Variant>)` — wrap `logger` and merge `extra` into each record
- `LoggerAdapter.info(message: string): void`, `.debug`, `.warn`, `.error`, `.fatal` — same as on `Logger`
- `LoggerAdapter.process(message: string, kwargs: HashMap<string, Variant>): (string, HashMap<string, Variant>)` — override hook to customize the message/kwargs before dispatch

```titrate
let base = logging.getLogger("http");
let extra = new HashMap<string, Variant>();
extra.put("request_id", "abc-123");
let adapter = new LoggerAdapter(base, extra);
adapter.info("Handling request");
```

## QueueHandler / QueueListener (Phase 1-2 parity)

Decouple producers from a (possibly slow) handler using an in-process queue — mirroring Python's `logging.handlers.QueueHandler` / `QueueListener`.

- `new QueueHandler(queue: Queue<LogRecord>)` — handler that simply enqueues records; never blocks producers
- `QueueHandler.emit(record: LogRecord): void` — enqueue a record (replaces `handle`)
- `new QueueListener(queue: Queue<LogRecord>, handlers: ArrayList<Handler>)` — listener thread that drains the queue and dispatches records to one or more target handlers
- `QueueListener.start(): void` — start the background listener
- `QueueListener.stop(): void` — signal the listener to stop and wait for it to drain
- `QueueListener.enqueueSentinel(): void` — push a sentinel record to wake up a blocked listener on shutdown

```titrate
let queue = new Queue<LogRecord>();
let qh = new QueueHandler(queue);
let root = logging.getLogger("");
root.addHandler(qh);

let targets = new ArrayList<Handler>();
targets.add(new FileHandler("app.log"));
targets.add(new ConsoleHandler());
let listener = new QueueListener(queue, targets);
listener.start();
// ... application runs ...
listener.stop();
```

## logging.config (Phase 1-2 parity)

Mirrors Python's `logging.config` module for declarative configuration.

| Function | Description |
|----------|-------------|
| `LoggingConfig.dictConfig(config: HashMap<string, Variant>): void` | Configure loggers, handlers, formatters, and filters from a dictionary |
| `LoggingConfig.fileConfig(path: string, defaults: HashMap<string, string>): void` | Configure from an INI-style config file |
| `LoggingConfig.listen(port: int): void` | Open a socket that accepts new configurations at runtime (mirror of `logging.config.listen`) |
| `LoggingConfig.stopListening(): void` | Stop the listener started by `listen` |

```titrate
let cfg = new HashMap<string, Variant>();
cfg.put("version", 1);
cfg.put("disable_existing_loggers", false);
LoggingConfig.dictConfig(cfg);
```

## Module-level helpers (Phase 1-2 parity)

| Function | Description |
|----------|-------------|
| `logging.log(level: int, message: string): void` | Log at a numeric level without naming the method |
| `logging.getLevelName(level: int): string` | Map a numeric level to its name (`logging.getLevelName(2)` -> `"INFO"`) |
| `logging.toLevel(name: string): int` | Map a level name back to its numeric value |
| `logging.makeLogRecord(attrs: HashMap<string, Variant>): LogRecord` | Build a `LogRecord` from an attribute map (handy when receiving pickled records over a socket) |
