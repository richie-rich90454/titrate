# glob

The `tt.file` module provides pattern-based file matching. Glob finds files and directories using shell-style wildcards.

```titrate
import tt::file::Glob;
```

## Glob

Static methods for pattern-based file matching.

- `Glob.glob(pattern: string): ArrayList<string>` — find files matching pattern
- `Glob.globRecursive(pattern: string): ArrayList<string>` — recursive glob
- `Glob.escape(path: string): string` — escape special characters

```titrate
let files: ArrayList<string> = Glob.glob("src/**/*.tr");
let allFiles: ArrayList<string> = Glob.globRecursive("**/*.tr");
let escaped: string = Glob.escape("file[1].txt");
```
