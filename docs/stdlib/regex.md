# regex

The `tt.regex` module provides regular expression operations for pattern matching, searching, and replacement.

```titrate
import tt.regex.Regex;
import tt.regex.Match;
```

## Regex

Regular expression compiler and matcher.

- `Regex(pattern: String)` — compile a pattern
- `Regex(pattern: String, flags: String)` — compile with flags
- `Regex.compile(pattern: String): Regex` — static: compile a pattern
- `Regex.compile(pattern: String, flags: String): Regex` — static: compile with flags
- `match(input: String): bool` — test if the pattern matches anywhere in input
- `matches(input: String): bool` — test if the pattern matches the entire input
- `find(input: String): Match` — find first match; returns null if none
- `findAll(input: String): ArrayList<Match>` — find all matches
- `matchAll(input: String): ArrayList<Match>` — alias for findAll
- `replaceAll(input: String, replacement: String): String` — replace all matches
- `replaceFirst(input: String, replacement: String): String` — replace first match
- `split(input: String): ArrayList<String>` — split input by pattern
- `groupCount(): int` — number of capture groups
- `getPattern(): String` — the original pattern string
- `Regex.quote(s: String): String` — static: escape all regex metacharacters

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

- `matched: String` — the matched substring
- `start: int` — start index of the match
- `end: int` — end index of the match (exclusive)
- `groups: ArrayList<String>` — captured groups
- `getMatched(): String` — the matched text
- `getStart(): int` — start index
- `getEnd(): int` — end index
- `getGroup(i: int): String` — captured group by index
- `groupCount(): int` — number of captured groups
- `groups(): ArrayList<String>` — all captured groups
- `range(): ArrayList<int>` — [start, end] as a two-element list
