# system

The `tt.sys`, `tt.subprocess`, `tt.tempfile`, `tt.secrets`, `tt.copy`, `tt.argparse`, and `tt.logging` modules provide system-level operations: environment access, process execution, temporary files, secure random generation, object copying, command-line argument parsing, and logging.

```titrate
import tt.sys.Sys;
import tt.subprocess.Subprocess;
import tt.tempfile.Tempfile;
import tt.secrets.Secrets;
import tt.copy.Copy;
import tt.argparse.ArgumentParser;
import tt.logging.Logger;
```

## Sys

System-level operations backed by VM built-ins.

- `Sys.args(): ArrayList<string>` — command-line arguments
- `Sys.env(key: string): string` — get an environment variable
- `Sys.setEnv(key: string, val: string): void` — set an environment variable
- `Sys.exit(code: int): void` — terminate the process
- `Sys.workingDir(): string` — current working directory
- `Sys.sleep(ms: int): void` — sleep for milliseconds
- `Sys.platform(): string` — platform identifier
- `Sys.cpuCount(): int` — number of available CPUs
- `Sys.exec(command: string): string` — execute a system command and return output
- `Sys.pid(): int` — current process ID
- `Sys.nanoTime(): long` — high-resolution time in nanoseconds
- `Sys.userName(): string` — current user name
- `Sys.hostName(): string` — host name

```titrate
let dir = Sys.workingDir();
let platform = Sys.platform();
let cpus = Sys.cpuCount();
Sys.sleep(1000);
```

## Subprocess

System command execution.

- `Subprocess.run(command: string): int` — run a command, return exit code
- `Subprocess.exec(command: string): string` — run a command, return stdout
- `Subprocess.exitCode(): int` — exit code from the last `run`

```titrate
let output = Subprocess.exec("ls -la");
let code = Subprocess.run("echo hello");
```

## Tempfile

Temporary file and directory creation.

- `Tempfile.createTempFile(): string` — create a temporary file, return its path
- `Tempfile.createTempDir(): string` — create a temporary directory, return its path
- `Tempfile.getTempDir(): string` — system temporary directory path

```titrate
let tmpPath = Tempfile.createTempFile();
let tmpDir = Tempfile.createTempDir();
let sysTmp = Tempfile.getTempDir();
```

## Secrets

Cryptographically secure random generation.

- `Secrets.tokenHex(nBytes: int): string` — random hex token
- `Secrets.tokenBytes(nBytes: int): string` — random byte token
- `Secrets.tokenUrlSafe(nBytes: int): string` — URL-safe random token
- `Secrets.choice<T>(list: ArrayList<T>): T` — random element from a list
- `Secrets.randBelow(n: int): int` — random integer in `[0, n)`

```titrate
let token = Secrets.tokenHex(32);
let safe = Secrets.tokenUrlSafe(16);
let pick = Secrets.choice(myList);
```

## Copy

Shallow and deep copy utilities.

- `Copy.shallowCopy<T>(obj: T): T` — shallow copy of any object
- `Copy.deepCopy<T>(obj: T): T` — deep copy of any object
- `Copy.shallowCopyList<T>(list: ArrayList<T>): ArrayList<T>` — shallow copy a list
- `Copy.deepCopyList<T>(list: ArrayList<T>): ArrayList<T>` — deep copy a list
- `Copy.shallowCopyMap<K, V>(map: HashMap<K, V>): HashMap<K, V>` — shallow copy a map
- `Copy.deepCopyMap<K, V>(map: HashMap<K, V>): HashMap<K, V>` — deep copy a map

```titrate
let original = new ArrayList<int>();
original.add(1);
let shallow = Copy.shallowCopyList(original);
let deep = Copy.deepCopyList(original);
```

## ArgumentParser

Python-style command-line argument parsing.

- `fn init(name: string, description: string)` — create a parser
- `addArgument(name: string, typ: string, default: Variant, required: bool, help: string): void` — register an argument
- `parseArgs(args: ArrayList<string>): HashMap<string, Variant>` — parse arguments into a map
- `usage(): string` — generate usage string
- `help(): string` — generate help text

```titrate
let parser = new ArgumentParser("myapp", "A sample application");
parser.addArgument("name", "string", null, true, "Your name");
parser.addArgument("verbose", "bool", false, false, "Enable verbose output");
let args = parser.parseArgs(Sys.args());
```

## Logger

Python-style logging with configurable handlers.

- `fn init(name: string)` — create a logger with default level `"WARNING"`
- `fn init(name: string, level: string)` — create a logger with specified level
- `setLevel(level: string): void` — set log level (`"DEBUG"`, `"INFO"`, `"WARNING"`, `"ERROR"`, `"CRITICAL"`)
- `addHandler(handler: Handler): void` — add a log handler
- `debug(msg: string): void` — log at DEBUG level
- `info(msg: string): void` — log at INFO level
- `warning(msg: string): void` — log at WARNING level
- `error(msg: string): void` — log at ERROR level
- `critical(msg: string): void` — log at CRITICAL level

### Formatter

- `Formatter.format(level: string, message: string, timestamp: string): string` — format a log record

### ConsoleHandler

- `fn init()` — handler that prints to stderr
- `fn init(formatter: Formatter)` — handler with custom formatter

### FileHandler

- `fn init(path: string)` — handler that appends to a file
- `fn init(path: string, formatter: Formatter)` — handler with custom formatter

```titrate
let logger = new Logger("myapp", "DEBUG");
logger.addHandler(new ConsoleHandler());
logger.info("Application started");
logger.warning("Low disk space");
```

## Deepened OS Operations

- `System.getEnvironmentVariable(name: string): string` — get env variable
- `System.setEnvironmentVariable(name: string, value: string): void` — set env variable
- `System.getProcessId(): int` — current process ID
- `System.getProcessorCount(): int` — available CPU cores
- `System.getTotalMemory(): long` — total system memory
- `System.getFreeMemory(): long` — free system memory
- `System.getUptime(): long` — system uptime in milliseconds
