# fileutils

The `tt.file` module provides high-level file operations for copying, moving, and inspecting files and directories.

```titrate
import tt::file::FileUtils;
```

## FileUtils

Static methods for high-level file and directory operations.

- `FileUtils.copy(src: string, dst: string): void` — copy a file
- `FileUtils.copyTree(src: string, dst: string): void` — copy a directory tree
- `FileUtils.move(src: string, dst: string): void` — move a file or directory
- `FileUtils.rmtree(path: string): void` — remove directory tree
- `FileUtils.which(name: string): string` — find executable in PATH
- `FileUtils.diskUsage(path: string): int` — get disk usage in bytes
- `FileUtils.touch(path: string): void` — create empty file or update mtime
- `FileUtils.exists(path: string): bool` — check if path exists
- `FileUtils.isFile(path: string): bool` — check if path is a file
- `FileUtils.isDir(path: string): bool` — check if path is a directory
- `FileUtils.size(path: string): int` — get file size in bytes

```titrate
FileUtils.copy("data.txt", "backup/data.txt");
FileUtils.copyTree("project/", "backup/project/");
FileUtils.move("old_name.txt", "new_name.txt");
FileUtils.rmtree("temp_build/");

let gitPath: string = FileUtils.which("git");
let usage: int = FileUtils.diskUsage("/home/user");
FileUtils.touch("new_file.txt");

let exists: bool = FileUtils.exists("config.json");
let isFile: bool = FileUtils.isFile("config.json");
let isDir: bool = FileUtils.isDir("src/");
let fileSize: int = FileUtils.size("data.bin");
```

## Extended File Utilities

- `FileUtils.copy2(src: string, dst: string): void` — copy file with metadata
- `FileUtils.copytree(src: string, dst: string): void` — copy directory tree
- `FileUtils.rmtree(path: string): void` — remove directory tree
- `FileUtils.move(src: string, dst: string): void` — move file or directory
- `FileUtils.diskUsage(path: string): HashMap<string, long>` — disk usage (total, used, free)
- `FileUtils.which(command: string): string` — find executable in PATH
