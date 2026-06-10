# Raw Strings and Literals

Titrate supports several literal formats beyond the basic string, character, and integer syntax. These include raw string literals, byte literals, and alternative numeric formats for binary, octal, and hexadecimal values.

## Raw String Literals

Raw string literals allow you to write strings without escape processing. They are useful for regular expressions, file paths, and any text that contains many backslashes or quotes.

### Basic Raw Strings

Prefix a string with `r` to create a raw string literal:

```titrate
let path = r"C:\Users\name\project";
io::println(path);  // C:\Users\name\project
```

In a raw string, backslashes are treated as literal characters — no escape sequences are processed:

```titrate
let regex = r"\d+\.\d+";
io::println(regex);  // \d+\.\d+
```

### Hash-Delimited Raw Strings

When the raw string content itself contains a double quote character, use the `r#"..."#` form. The number of `#` delimiters on each side must match:

```titrate
let json = r#"{"key": "value"}"#;
io::println(json);  // {"key": "value"}
```

For content that contains both quotes and `#` characters, add more hash delimiters:

```titrate
let complex = r##"data with "quotes" and # signs"##;
```

### Raw Strings vs Regular Strings

| Feature | Regular string | Raw string |
|---------|---------------|------------|
| Escape sequences | Processed | Not processed |
| Newlines in source | Not allowed | Not allowed |
| Double quotes | Escaped `\"` | Use `r#"..."#` |
| Backslashes | Escaped `\\` | Literal `\` |

## Byte Literals

Byte literals produce a single `byte` value (unsigned 8-bit integer) using the `b'x'` syntax:

```titrate
let newline: byte = b'\n';
let tab: byte = b'\t';
let letter: byte = b'A';
```

### Byte Escape Sequences

Byte literals support the following escape sequences:

| Escape | Value | Description |
|--------|-------|-------------|
| `b'\n'` | `0x0A` | Newline |
| `b'\t'` | `0x09` | Tab |
| `b'\r'` | `0x0D` | Carriage return |
| `b'\\'` | `0x5C` | Backslash |
| `b'\''` | `0x27` | Single quote |
| `b'\"'` | `0x22` | Double quote |
| `b'\0'` | `0x00` | Null byte |
| `b'\x41'` | `0x41` | Hex escape (any byte value) |

### Hex Escapes in Byte Literals

Use `\x` followed by two hexadecimal digits to specify any byte value:

```titrate
let null_byte: byte = b'\x00';
let del: byte = b'\x7F';
let capital_a: byte = b'\x41';  // same as b'A'
```

Byte literals can only contain ASCII characters. For Unicode code points, use character literals (`'x'`) instead.

## Numeric Literal Formats

Titrate supports several formats for writing integer literals beyond plain decimal.

### Hexadecimal (`0x`)

Prefix with `0x` (or `0X`) for hexadecimal:

```titrate
let hex_lower: int = 0xFF;    // 255
let hex_upper: int = 0xDEAD;  // 57005
let hex_mixed: int = 0xCafe;  // 51966
```

### Octal (`0o`)

Prefix with `0o` (or `0O`) for octal:

```titrate
let octal: int = 0o777;  // 511
let perms: int = 0o755;  // 493
```

### Binary (`0b`)

Prefix with `0b` (or `0B`) for binary:

```titrate
let binary: int = 0b1010;   // 10
let flags: int = 0b11110000;  // 240
let mask: int = 0B11001100;   // 204
```

### Underscore Separators

All integer literals (decimal, hex, octal, and binary) support underscore separators for readability. Underscores are ignored by the compiler:

```titrate
let million: int = 1_000_000;
let hex_color: int = 0xFF_00_FF;
let binary_mask: int = 0b1111_0000_1100_0011;
let octal_perms: int = 0o7_5_5;
```

### Summary of Integer Literal Formats

| Format | Prefix | Example | Value |
|--------|--------|---------|-------|
| Decimal | (none) | `42` | 42 |
| Hexadecimal | `0x` / `0X` | `0xFF` | 255 |
| Octal | `0o` / `0O` | `0o777` | 511 |
| Binary | `0b` / `0B` | `0b1010` | 10 |

All formats support underscore separators.

## What's Next?

- [Variables](./variables) — `let`, `var`, and `const` declarations
- [Types](../reference/types) — the full type system reference
- [Grammar](../reference/grammar) — formal grammar specification
