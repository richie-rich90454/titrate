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

**Additional methods (C++ `<string_view>` parity, Phase 1-2):**

- `removePrefix(n: int): void` — shrink the view from the front by `n` characters
- `removeSuffix(n: int): void` — shrink the view from the back by `n` characters
- `substr(start: int, length: int): StringView` — return a sub-view of the given length starting at `start`
- `compare(other: StringView): int` — lexicographic comparison; negative if less, zero if equal, positive if greater
- `compare(other: string): int` — lexicographic comparison against an owned string
- `find(substring: string): int` — find first occurrence of `substring`; -1 if not found (same semantics as `indexOf`)
- `rfind(substring: string): int` — find last occurrence of `substring`; -1 if not found
- `findFirstOf(chars: string): int` — find first character that is in `chars`; -1 if none
- `findLastOf(chars: string): int` — find last character that is in `chars`; -1 if none
- `findFirstNotOf(chars: string): int` — find first character not in `chars`; -1 if none
- `findLastNotOf(chars: string): int` — find last character not in `chars`; -1 if none

```titrate
let text: string = "Hello, Titrate!";
let view: StringView = new StringView(text, 7, 7);  // "Titrate"

io::println(view.toString());           // "Titrate"
io::println(Integer.toString(view.length())); // 7
io::println(Boolean.toString(view.startsWith("Tit")));  // true
io::println(Boolean.toString(view.endsWith("rate")));   // true

let sub: StringView = view.slice(0, 3);
io::println(sub.toString());  // "Tit"

// removePrefix / removeSuffix shrink the view in place
let v: StringView = new StringView("prefix:body", 0, 11);
v.removePrefix(7);  // view now covers "body"
io::println(v.toString());  // "body"

let w: StringView = new StringView("body:trailing", 0, 13);
w.removeSuffix(9);  // view now covers "body"
io::println(w.toString());  // "body"

// find / rfind
let s: StringView = new StringView("abracadabra", 0, 11);
io::println(Integer.toString(s.find("abra")));   // 0
io::println(Integer.toString(s.rfind("abra")));  // 7

// findFirstOf / findLastOf / findFirstNotOf / findLastNotOf
io::println(Integer.toString(s.findFirstOf("cd")));     // 4
io::println(Integer.toString(s.findLastOf("cd")));      // 6
io::println(Integer.toString(s.findFirstNotOf("ab")));  // 4
io::println(Integer.toString(s.findLastNotOf("ab")));   // 10

// compare
let a: StringView = new StringView("apple", 0, 5);
let b: StringView = new StringView("banana", 0, 6);
io::println(Integer.toString(a.compare(b)));  // negative
```
