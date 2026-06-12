# regex

The `tt.regex` module provides regular expression operations for pattern matching, searching, and replacement.

```titrate
import tt.regex.Regex;
import tt.regex.Match;
```

## Regex

Regular expression compiler and matcher.

- `fn init(pattern: string)` — compile a pattern
- `fn init(pattern: string, flags: string)` — compile with flags
- `Regex.compile(pattern: string): Regex` — static: compile a pattern
- `Regex.compile(pattern: string, flags: string): Regex` — static: compile with flags
- `match(input: string): bool` — test if the pattern matches anywhere in input
- `matches(input: string): bool` — test if the pattern matches the entire input
- `find(input: string): Match` — find first match; returns null if none
- `findAll(input: string): ArrayList<Match>` — find all matches
- `matchAll(input: string): ArrayList<Match>` — alias for findAll
- `replaceAll(input: string, replacement: string): string` — replace all matches
- `replaceFirst(input: string, replacement: string): string` — replace first match
- `split(input: string): ArrayList<string>` — split input by pattern
- `groupCount(): int` — number of capture groups
- `getPattern(): string` — the original pattern string
- `Regex.quote(s: string): string` — static: escape all regex metacharacters

```titrate
let re = Regex.compile("\\d+");
let m = re.find("abc123def");
io::println(m.matched);  // "123"
io::println(m.start);    // 3

let all = re.findAll("a1b22c333");
// [Match("1", 1, 2), Match("22", 3, 5), Match("333", 6, 9)]

let cleaned = re.replaceAll("price: 100 dollars", "X");
// "price: X dollars"
```

## Match

Represents a single regex match result.

- `matched: string` — the matched substring
- `start: int` — start index of the match
- `end: int` — end index of the match (exclusive)
- `groups: ArrayList<string>` — captured groups
- `getMatched(): string` — the matched text
- `getStart(): int` — start index
- `getEnd(): int` — end index
- `getGroup(i: int): string` — captured group by index
- `groupCount(): int` — number of captured groups
- `groups(): ArrayList<string>` — all captured groups
- `range(): ArrayList<int>` — [start, end] as a two-element list
