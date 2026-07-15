# pprint

The `tt.pprint` module provides pretty-printing utilities for data structures. It formats `ArrayList` and `HashMap` values with configurable indentation and line width, automatically switching between compact and multi-line output.

```titrate
import tt.pprint.Pprint;
```

## Pprint

All methods are static.

- `pformat(obj: Variant): string` — format with default indent=2, width=80
- `pformat(obj: Variant, indent: int): string` — format with given indent, width=80
- `pformat(obj: Variant, indent: int, width: int): string` — format with given indent and width
- `pprint(obj: Variant): void` — print to stdout with default indent=2
- `pprint(obj: Variant, indent: int): void` — print to stdout with given indent

### Formatting rules

- `null` is rendered as `"null"`.
- `ArrayList` values try a compact `[a, b, c]` format first; if the result exceeds `width`, each element is placed on its own indented line.
- `HashMap` values try a compact `{k: v, ...}` format first; if too wide, each entry is placed on its own indented line.
- All other objects are rendered via `toString()`.

```titrate
let list = new ArrayList<int>();
list.add(1); list.add(2); list.add(3);
Pprint.pprint(list);
// [1, 2, 3]

let map = new HashMap<string, int>();
map.put("alpha", 1);
map.put("beta", 2);
Pprint.pprint(map, 4);
// {alpha: 1, beta: 2}

let formatted = Pprint.pformat(list, 2, 10);
// Multi-line if compact form exceeds 10 chars
```

## Repr class (Phase 1-2 parity)

`Repr` is a configurable pretty-representation helper that mirrors Python's `pprint.Repr`. It controls how deep and how wide a recursive `repr` of nested data structures becomes, allowing you to truncate long collections.

- `Repr.init()` — create a `Repr` with defaults
- `Repr.repr(obj: Variant): string` — produce a `repr`-style string for `obj` using the current limits
- `Repr.repr1(obj: Variant, level: int): string` — recursive worker used by `repr`; `level` tracks recursion depth so deeply nested structures can be truncated

**Configurable limits** (fields on a `Repr` instance):

| Field | Default | Description |
|-------|---------|-------------|
| `maxLevel` | `6` | Maximum recursion depth; deeper nesting renders as `...` |
| `maxDict` | `4` | Maximum number of HashMap entries shown |
| `maxList` | `4` | Maximum number of ArrayList elements shown |
| `maxTuple` | `6` | Maximum number of tuple elements shown |
| `maxSet` | `4` | Maximum number of set elements shown |
| `maxString` | `30` | Maximum number of string characters shown |
| `maxLong` | `40` | Maximum number of digits of a long shown |
| `maxArray` | `5` | Maximum number of array elements shown |

```titrate
import tt.pprint.Repr;

let r = new Repr();
r.maxLevel = 3;
r.maxList = 2;
r.maxDict = 2;

let nested = new ArrayList<ArrayList<int>>();
// [[1, 2, 3], [4, 5, 6], [7, 8, 9]]

io::println(r.repr(nested));
// With maxList=2, maxLevel=3: [[1, 2, ...], [4, 5, ...], ...]
```
