# pprint

The `tt.pprint` module provides pretty-printing utilities for data structures. It formats `ArrayList` and `HashMap` values with configurable indentation and line width, automatically switching between compact and multi-line output.

```titrate
import tt.pprint.Pprint;
```

## Pprint

All methods are static.

- `pformat(obj: Object): String` — format with default indent=2, width=80
- `pformat(obj: Object, indent: int): String` — format with given indent, width=80
- `pformat(obj: Object, indent: int, width: int): String` — format with given indent and width
- `pprint(obj: Object): void` — print to stdout with default indent=2
- `pprint(obj: Object, indent: int): void` — print to stdout with given indent

### Formatting rules

- `null` is rendered as `"null"`.
- `ArrayList` values try a compact `[a, b, c]` format first; if the result exceeds `width`, each element is placed on its own indented line.
- `HashMap` values try a compact `{k: v, ...}` format first; if too wide, each entry is placed on its own indented line.
- All other objects are rendered via `toString()`.

```titrate
let list = new ArrayList<int>();
list.add(1); list.add(2); list.add(3);
Pprint::pprint(list);
// [1, 2, 3]

let map = new HashMap<string, int>();
map.put("alpha", 1);
map.put("beta", 2);
Pprint::pprint(map, 4);
// {alpha: 1, beta: 2}

let formatted = Pprint::pformat(list, 2, 10);
// Multi-line if compact form exceeds 10 chars
```
