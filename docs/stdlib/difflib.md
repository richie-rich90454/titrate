# difflib

The `tt.text` module provides sequence comparison utilities for computing diffs between text sequences.

```titrate
import tt.text.Difflib;
```

## Difflib

Utilities for comparing sequences and generating diffs in various formats.

- `Difflib.unifiedDiff(a: ArrayList<string>, b: ArrayList<string>, fromFile: string, toFile: string): string` — generate a unified diff between two sequences
- `Difflib.contextDiff(a: ArrayList<string>, b: ArrayList<string>, fromFile: string, toFile: string): string` — generate a context diff between two sequences
- `Difflib.ratio(a: string, b: string): double` — compute similarity ratio between two strings (0.0 to 1.0)
- `Difflib.sequenceMatcher(a: ArrayList<string>, b: ArrayList<string>): ArrayList<Variant>` — find matching blocks between two sequences
- `Difflib.ndiff(a: ArrayList<string>, b: ArrayList<string>): string` — generate a delta in ndiff format
- `Difflib.restore(delta: string, which: int): ArrayList<string>` — restore a sequence from an ndiff delta (1 for first, 2 for second)

```titrate
let a: ArrayList<string> = new ArrayList<string>();
a.add("hello");
a.add("world");
let b: ArrayList<string> = new ArrayList<string>();
b.add("hello");
b.add("titrate");

let diff: string = Difflib.unifiedDiff(a, b, "old.txt", "new.txt");
io::println(diff);

let similarity: double = Difflib.ratio("hello", "hallo");
io::println(Double.toString(similarity));  // 0.8
```
