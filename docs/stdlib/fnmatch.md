# fnmatch

The `tt.file` module provides Unix-style filename pattern matching. Fnmatch supports shell-style wildcards like `*`, `?`, and `[seq]`.

```titrate
import tt::file::Fnmatch;
```

## Fnmatch

Static methods for Unix-style filename pattern matching.

- `Fnmatch.fnmatch(name: string, pattern: string): bool` — match filename against pattern
- `Fnmatch.fnmatchCase(name: string, pattern: string): bool` — case-sensitive match
- `Fnmatch.filter(names: ArrayList<string>, pattern: string): ArrayList<string>` — filter names by pattern
- `Fnmatch.translate(pattern: string): string` — convert to regex pattern

```titrate
let match: bool = Fnmatch.fnmatch("file.txt", "*.txt");
let caseMatch: bool = Fnmatch.fnmatchCase("File.TXT", "*.txt");

let names: ArrayList<string> = new ArrayList<string>();
names.add("readme.md");
names.add("main.tr");
names.add("test.tr");
let trFiles: ArrayList<string> = Fnmatch.filter(names, "*.tr");

let regex: string = Fnmatch.translate("*.tr");
```

## filter

- `Fnmatch.filter(names: ArrayList<string>, pattern: string): ArrayList<string>` — filter names matching pattern

## translate

- `Fnmatch.translate(pattern: string): string` — translate fnmatch pattern to regex

## Case-Insensitive Matching

- `Fnmatch.fnmatchCase(name: string, pattern: string): bool` — case-sensitive match
- `Fnmatch.fnmatch(name: string, pattern: string): bool` — case-insensitive match (platform-dependent)
