# format

The `tt.io` module provides `Format` — printf-style string formatting with support for width, precision, and common format specifiers.

```titrate
import tt.io.Format;
```

## Format

A utility class for formatting strings using template placeholders, similar to C's `printf`.

**Methods:**

- `Format.format(template: string, args: ArrayList<Variant>): string` — format a template with the given arguments
- `Format.sprintf(template: string, args: ArrayList<Variant>): string` — alias for `format`
- `Format.printf(template: string, args: ArrayList<Variant>): void` — format and print to stdout

**Format specifiers:**

| Specifier | Description |
|-----------|-------------|
| `%d` | Integer (decimal) |
| `%f` | Floating-point (decimal) |
| `%s` | String |
| `%x` | Integer (hexadecimal, lowercase) |
| `%o` | Integer (octal) |
| `%b` | Integer (binary) |
| `%c` | Character |
| `%%` | Literal `%` |

**Width and precision** can be specified: `%10.2f` formats a double in a field of width 10 with 2 decimal places.

```titrate
let args: ArrayList<Variant> = new ArrayList<Variant>();
args.add(42);
args.add(3.14159);
args.add("world");

let result: string = Format.format("int=%d, pi=%.2f, hello %s", args);
io::println(result);  // "int=42, pi=3.14, hello world"

// Using printf directly
let hexArgs: ArrayList<Variant> = new ArrayList<Variant>();
hexArgs.add(255);
Format.printf("255 in hex: %x\n", hexArgs);  // "255 in hex: ff"

// Width and precision
let precise: ArrayList<Variant> = new ArrayList<Variant>();
precise.add(2.71828);
io::println(Format.format("%10.2f", precise));  // "      2.72"
```

## std::format-style formatting (Phase 1-2 parity)

In addition to the printf-style API above, `Format` provides `std::format`-compatible placeholders using `{}` braces. This mirrors C++20 `std::format`.

### Format string syntax

| Placeholder | Description |
|-------------|-------------|
| `{}` | Default formatting, positional by argument order |
| `{0}`, `{1}`, ... | Explicit argument index |
| `{:.2f}` | Precision (2 decimal places for floats) |
| `{:>10}` | Right-align in a field of width 10 |
| `{:<10}` | Left-align in a field of width 10 |
| `{:^10}` | Center-align in a field of width 10 |
| `{:>10.2f}` | Right-align, width 10, 2 decimal places |
| `{:#x}` | Hex with `0x` prefix |
| `{:x}`, `{:o}`, `{:b}` | Hex / octal / binary integer |

**Methods:**

- `Format.stdFormat(template: string, args: ArrayList<Variant>): string` — format using `{}` brace placeholders
- `Format.formatTo(out: StringBuilder, template: string, args: ArrayList<Variant>): void` — append formatted output to a `StringBuilder`
- `Format.formatToN(out: StringBuilder, n: int, template: string, args: ArrayList<Variant>): int` — write at most `n` characters; returns the number of characters that would have been written (like `std::format_to_n`)
- `Format.formattedSize(template: string, args: ArrayList<Variant>): int` — return the number of characters the formatted result would produce, without writing it

```titrate
let args: ArrayList<Variant> = new ArrayList<Variant>();
args.add(42); args.add(3.14159); args.add("world");

// Positional by order
let s1: string = Format.stdFormat("int={}, pi={:.2f}, hello {}", args);
// "int=42, pi=3.14, hello world"

// Explicit indices
let s2: string = Format.stdFormat("{2} then {0} then {1}", args);
// "world then 42 then 3.14"

// Alignment and width
let s3: string = Format.stdFormat("{:>10}", args);  // "        42"

// formatTo appends to an existing builder
let buf = new StringBuilder();
Format.formatTo(buf, "value = {:.2f}", args);  // buf now contains "value = 3.14"

// formattedSize computes the length without writing
let len: int = Format.formattedSize("{:.4f}", args);  // 6 (e.g. "3.1416")
```
