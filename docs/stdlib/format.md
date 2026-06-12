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
