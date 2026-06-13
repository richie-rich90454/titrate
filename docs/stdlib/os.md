# os

The `tt.sys` module provides a full OS interface for interacting with the operating system, including environment variables, process information, file system operations, and system utilities.

```titrate
import tt::sys::Os;
```

## Os

Static methods for operating system interactions.

### Environment & Process

- `Os.getEnv(name: string): string` — get environment variable
- `Os.setEnv(name: string, value: string): void` — set environment variable
- `Os.environ(): string` — all environment variables as formatted string
- `Os.environMap(): HashMap<string, string>` — all environment variables as a HashMap
- `Os.cpuCount(): int` — get number of CPUs
- `Os.pid(): int` — get current process ID
- `Os.userName(): string` — get current username
- `Os.hostName(): string` — get hostname
- `Os.urandom(n: int): string` — get n random bytes as hex string
- `Os.workingDir(): string` — get current working directory
- `Os.getcwd(): string` — alias for workingDir
- `Os.changeDir(path: string): void` — change working directory
- `Os.exit(code: int): void` — exit the process
- `Os.sleep(ms: int): void` — sleep for milliseconds
- `Os.kill(pid: int, sig: int): void` — send signal to a process

### File System

- `Os.listDir(path: string): ArrayList<string>` — list directory entries
- `Os.mkdir(path: string): Result<string, string>` — create a directory
- `Os.makedirs(path: string): Result<string, string>` — recursively create directories (like mkdir -p)
- `Os.remove(path: string): Result<string, string>` — remove a file or empty directory
- `Os.rmdir(path: string): Result<string, string>` — remove a directory
- `Os.rename(oldPath: string, newPath: string): Result<string, string>` — rename/move a file or directory
- `Os.exists(path: string): bool` — check if path exists
- `Os.access(path: string, mode: int): bool` — check file accessibility (1=exists, 2=readable, 4=writable, 8=executable)
- `Os.stat(path: string): Result<HashMap<string, int>, string>` — get file/directory status
- `Os.getFileSize(path: string): int` — get file size
- `Os.chmod(path: string, mode: int): Result<string, string>` — change file permissions
- `Os.symlink(original: string, link: string): Result<string, string>` — create a symbolic link
- `Os.readlink(path: string): Result<string, string>` — read target of a symbolic link
- `Os.umask(mask: int): int` — set file creation mask

### Directory Scanning

- `Os.walk(path: string): ArrayList<ArrayList<string>>` — walk directory tree yielding (dirpath, subdirs, files) for each directory
- `Os.scandir(path: string): ArrayList<DirEntry>` — scan directory returning DirEntry objects with file metadata

### DirEntry

- `name: string` — entry name
- `isFile: bool` — whether the entry is a file
- `isDir: bool` — whether the entry is a directory
- `isSymlink: bool` — whether the entry is a symbolic link

### Platform Constants

- `Os.pathSeparator(): string` — ";" on Windows, ":" on Unix
- `Os.separator(): string` — "\\" on Windows, "/" on Unix
- `Os.linesep(): string` — "\r\n" on Windows, "\n" on Unix
- `Os.devNull(): string` — "NUL" on Windows, "/dev/null" on Unix

```titrate
let home: string = Os.getEnv("HOME");
let cpus: int = Os.cpuCount();
let pid: int = Os.pid();
let cwd: string = Os.getcwd();
Os.sleep(1000);

// Walk a directory tree
let tree: ArrayList<ArrayList<string>> = Os.walk("/path/to/dir");

// Scan a directory with metadata
let entries: ArrayList<DirEntry> = Os.scandir("/path/to/dir");
for (entry in entries) {
    if (entry.isDir) {
        io::println("DIR: " + entry.name);
    }
}

// Recursively create directories
Os.makedirs("/path/to/nested/dir");

// Create a symlink
Os.symlink("/path/to/target", "/path/to/link");

// Change permissions
Os.chmod("/path/to/file", 0o755);
```
