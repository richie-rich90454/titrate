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

- `Sys.args(): ArrayList<String>` — command-line arguments
- `Sys.env(key: String): String` — get an environment variable
- `Sys.setEnv(key: String, val: String): void` — set an environment variable
- `Sys.exit(code: int): void` — terminate the process
- `Sys.workingDir(): String` — current working directory
- `Sys.sleep(ms: int): void` — sleep for milliseconds
- `Sys.platform(): String` — platform identifier
- `Sys.cpuCount(): int` — number of available CPUs
- `Sys.exec(command: String): String` — execute a system command and return output
- `Sys.pid(): int` — current process ID
- `Sys.nanoTime(): long` — high-resolution time in nanoseconds
- `Sys.userName(): String` — current user name
- `Sys.hostName(): String` — host name

```titrate
let dir = Sys::workingDir();
let platform = Sys::platform();
let cpus = Sys::cpuCount();
Sys::sleep(1000);
```

## Subprocess

System command execution.

- `Subprocess.run(command: String): int` — run a command, return exit code
- `Subprocess.exec(command: String): String` — run a command, return stdout
- `Subprocess.exitCode(): int` — exit code from the last `run`

```titrate
let output = Subprocess::exec("ls -la");
let code = Subprocess::run("echo hello");
```

## Tempfile

Temporary file and directory creation.

- `Tempfile.createTempFile(): String` — create a temporary file, return its path
- `Tempfile.createTempDir(): String` — create a temporary directory, return its path
- `Tempfile.getTempDir(): String` — system temporary directory path

```titrate
let tmpPath = Tempfile::createTempFile();
let tmpDir = Tempfile::createTempDir();
let sysTmp = Tempfile::getTempDir();
```

## Secrets

Cryptographically secure random generation.

- `Secrets.tokenHex(nBytes: int): String` — random hex token
- `Secrets.tokenBytes(nBytes: int): String` — random byte token
- `Secrets.tokenUrlSafe(nBytes: int): String` — URL-safe random token
- `Secrets.choice(list: ArrayList): Object` — random element from a list
- `Secrets.randBelow(n: int): int` — random integer in `[0, n)`

```titrate
let token = Secrets::tokenHex(32);
let safe = Secrets::tokenUrlSafe(16);
let pick = Secrets::choice(myList);
```

## Copy

Shallow and deep copy utilities.

- `Copy.shallowCopy(obj: Object): Object` — shallow copy of any object
- `Copy.deepCopy(obj: Object): Object` — deep copy of any object
- `Copy.shallowCopyList(list: ArrayList): ArrayList` — shallow copy a list
- `Copy.deepCopyList(list: ArrayList): ArrayList` — deep copy a list
- `Copy.shallowCopyMap(map: HashMap): HashMap` — shallow copy a map
- `Copy.deepCopyMap(map: HashMap): HashMap` — deep copy a map

```titrate
let original = new ArrayList<int>();
original.add(1);
let shallow = Copy::shallowCopyList(original);
let deep = Copy::deepCopyList(original);
```

## ArgumentParser

Python-style command-line argument parsing.

- `ArgumentParser(name: String, description: String)` — create a parser
- `addArgument(name: String, typ: String, default: Object, required: bool, help: String): void` — register an argument
- `parseArgs(args: ArrayList<String>): HashMap<String, Object>` — parse arguments into a map
- `usage(): String` — generate usage string
- `help(): String` — generate help text

```titrate
let parser = new ArgumentParser("myapp", "A sample application");
parser.addArgument("name", "string", null, true, "Your name");
parser.addArgument("verbose", "bool", false, false, "Enable verbose output");
let args = parser.parseArgs(Sys::args());
```

## Logger

Python-style logging with configurable handlers.

- `Logger(name: String)` — create a logger with default level `"WARNING"`
- `Logger(name: String, level: String)` — create a logger with specified level
- `setLevel(level: String): void` — set log level (`"DEBUG"`, `"INFO"`, `"WARNING"`, `"ERROR"`, `"CRITICAL"`)
- `addHandler(handler: Handler): void` — add a log handler
- `debug(msg: String): void` — log at DEBUG level
- `info(msg: String): void` — log at INFO level
- `warning(msg: String): void` — log at WARNING level
- `error(msg: String): void` — log at ERROR level
- `critical(msg: String): void` — log at CRITICAL level

### Formatter

- `Formatter.format(level: String, message: String, timestamp: String): String` — format a log record

### ConsoleHandler

- `ConsoleHandler()` — handler that prints to stderr
- `ConsoleHandler(formatter: Formatter)` — handler with custom formatter

### FileHandler

- `FileHandler(path: String)` — handler that appends to a file
- `FileHandler(path: String, formatter: Formatter)` — handler with custom formatter

```titrate
let logger = new Logger("myapp", "DEBUG");
logger.addHandler(new ConsoleHandler());
logger.info("Application started");
logger.warning("Low disk space");
```
