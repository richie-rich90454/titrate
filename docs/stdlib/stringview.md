# stringview

The `tt.lang` module provides `StringView` — a non-owning string reference for efficient substring operations without allocation.

```titrate
import tt.lang.StringView;
```

## StringView

A lightweight view into a portion of a string. Unlike creating new substrings, a `StringView` borrows a range of characters from the source string, avoiding allocation overhead.

- `fn init(source: string, start: int, length: int)` — create view over `source` starting at `start` for `length` characters
- `slice(start: int, length: int): StringView` — create a sub-view relative to this view
- `charAt(index: int): string` — character at the given index within the view
- `length(): int` — number of characters in the view
- `equals(other: string): bool` — compare view contents with a string
- `startsWith(prefix: string): bool` — check if view starts with prefix
- `endsWith(suffix: string): bool` — check if view ends with suffix
- `contains(substring: string): bool` — check if view contains substring
- `indexOf(substring: string): int` — find first occurrence of substring; -1 if not found
- `toString(): string` — convert the view to an owned string

```titrate
let text: string = "Hello, Titrate!";
let view: StringView = new StringView(text, 7, 7);  // "Titrate"

io::println(view.toString());           // "Titrate"
io::println(Integer.toString(view.length())); // 7
io::println(Boolean.toString(view.startsWith("Tit")));  // true
io::println(Boolean.toString(view.endsWith("rate")));   // true

let sub: StringView = view.slice(0, 3);
io::println(sub.toString());  // "Tit"
```
