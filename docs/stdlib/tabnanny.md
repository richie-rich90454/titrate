# Tabnanny

The `tt.tooling.Tabnanny` module detects inconsistent use of tabs and spaces for indentation in source files. It mirrors Python's `tabnanny` module, exposing `check` (file-based), `checkString` (string-based), `process_tokens` (line-list-based), and the `NannyNag` exception class. Inconsistent indentation (e.g. tabs after spaces, or tabs in a file using spaces) is reported as a list of `NannyNag` issues.

## Import

```titrate
import tt::tooling::Tabnanny;
```

## Classes

### NannyNag

Exception raised when an inconsistent indentation is detected.

**Fields:**
- `message: string` — description of the issue
- `lineNumber: int` — 1-based line number where the issue was found
- `indentType: string` — `"tabs"`, `"spaces"`, or `"mixed"`

**Constructors:**
- `init(lineNumber: int, message: string, indentType: string)`

**Methods:**
- `getLineNumber(): int` — return the 1-based line number
- `getMessage(): string` — return the issue description
- `toString(): string` — returns `"NannyNag at line N: message"`

## Functions

### check

- `Tabnanny.check(filename: string): ArrayList<NannyNag>` — check a file for inconsistent indentation. Returns a list of `NannyNag` issues; an empty list means no problems. A missing file returns an empty list.

```titrate
let issues: ArrayList<NannyNag> = Tabnanny.check("source.tr");
var i: int = 0;
while (i < issues.size()) {
    io::println(issues.get(i).toString());
    i = i + 1;
}
```

### checkString

- `Tabnanny.checkString(source: string, filename: string): ArrayList<NannyNag>` — check a string of source for inconsistent indentation. `filename` is used for reporting context.

### process_tokens

- `Tabnanny.process_tokens(lines: ArrayList<string>): ArrayList<NannyNag>` — process a list of source lines for indentation issues. Detects when an indentation uses a different type (`tabs` vs `spaces`) than the rest of the file, and flags lines that mix tabs and spaces on the same line.

## Detection Rules

- The first indented line establishes the expected indent type (`tabs` or `spaces`)
- Subsequent indented lines that use a different type trigger a `NannyNag`
- Lines with both tabs and spaces in the leading whitespace trigger a `"mixed"` `NannyNag`
- Blank lines are ignored

## Usage Example

```titrate
import tt::tooling::Tabnanny;

public fn main(): void {
    let source: string = "fn foo(): void {\n    io::println(\"a\");\n\tio::println(\"b\");\n}\n";
    let issues: ArrayList<NannyNag> = Tabnanny.checkString(source, "demo.tr");
    if (issues.size() == 0) {
        io::println("No indentation issues");
    } else {
        var i: int = 0;
        while (i < issues.size()) {
            io::println(issues.get(i).toString());
            i = i + 1;
        }
    }
}
```
