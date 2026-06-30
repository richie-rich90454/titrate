---
title: textwrap
description: Text wrapping, filling, indentation, and truncation utilities for Titrate.
---

# textwrap

The `tt.textwrap` module provides text wrapping, filling, indentation, and truncation utilities. It is useful for formatting console output, generating fixed-width reports, and preparing strings for display.

```titrate
import tt::textwrap::Textwrap;
```

## Textwrap

The `Textwrap` class exposes a single set of static-style methods for common text formatting tasks.

### Wrapping and Filling

- `Textwrap.wrap(text: string, width: int): ArrayList<string>` — break `text` into lines no wider than `width`
- `Textwrap.fill(text: string, width: int): string` — wrap text and join the lines with `\n`

```titrate
let text: string = "The quick brown fox jumps over the lazy dog";
let lines: ArrayList<string> = Textwrap.wrap(text, 20);
for (line in lines) {
    io::println(line);
}
// Output:
// The quick brown fox
// jumps over the lazy
// dog

let paragraph: string = Textwrap.fill(text, 20);
```

### Indentation and Dedent

- `Textwrap.indent(text: string, prefix: string): string` — add `prefix` to every non-empty line
- `Textwrap.dedent(text: string): string` — remove common leading whitespace from all lines

```titrate
let code: string = "    line one\n    line two\n        line three";
let dedented: string = Textwrap.dedent(code);
io::println(dedented);

let quoted: string = Textwrap.indent("line one\nline two", "> ");
io::println(quoted);
// > line one
// > line two
```

### Truncation

- `Textwrap.shorten(text: string, width: int): string` — truncate text to `width` characters, adding `...` when truncated

```titrate
let title: string = "A very long headline that needs to fit a narrow column";
let short: string = Textwrap.shorten(title, 30);
io::println(short);  // "A very long headline that n..."
```

## Common Use Cases

| Task | Method |
|------|--------|
| Format paragraphs | `Textwrap.fill` |
| Iterate over wrapped lines | `Textwrap.wrap` |
| Remove leading indentation from multiline strings | `Textwrap.dedent` |
| Add a quote prefix | `Textwrap.indent` |
| Fit text into a UI width | `Textwrap.shorten` |
