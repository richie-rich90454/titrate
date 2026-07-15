# Reprlib

The `tt.pprint.Pprint` module provides recursive-limited `repr` generation via the `Repr` class. It mirrors Python's `reprlib` module, exposing a `Repr` class with configurable per-type limits (`maxstring`, `maxlist`, `maxdict`, etc.) and a module-level `repr` convenience function. When a structure exceeds its limit, it is truncated and an ellipsis (`...`) is appended; nested structures are also bounded by `maxlevel`.

## Import

```titrate
import tt::pprint::Pprint;
```

## Classes

### Repr

A recursive-limited repr generator. Configure per-type limits via the public fields, then call `repr(obj)`.

**Fields:**
- `maxlevel: int` — max recursion depth (default `6`)
- `maxtuple: int` — max tuple elements shown (default `6`)
- `maxlist: int` — max list elements shown (default `6`)
- `maxarray: int` — max array elements shown (default `5`)
- `maxdict: int` — max dict entries shown (default `4`)
- `maxset: int` — max set elements shown (default `6`)
- `maxfrozenset: int` — max frozenset elements shown (default `6`)
- `maxdeque: int` — max deque elements shown (default `6`)
- `maxstring: int` — max string length before truncation (default `340`)
- `maxlong: int` — max long digit count before truncation (default `40`)
- `maxother: int` — max other repr length before truncation (default `300`)

**Constructors:**
- `init()` — creates a Repr with default limits

**Methods:**
- `repr(obj: Variant): string` — main entry point: produces a recursive-limited repr of `obj`
- `repr1(obj: Variant, level: int): string` — recursive dispatch; `level` tracks remaining recursion depth. When `level` reaches 0, nested structures are replaced with `"..."`.
- `reprStr(s: string, level: int): string` — repr a string with quotes, truncating to `maxstring` chars
- `reprList(list: ArrayList<Variant>, level: int): string` — repr an `ArrayList`, truncating to `maxlist` elements with `"..."` marker
- `reprDict(map: HashMap<Variant, Variant>, level: int): string` — repr a `HashMap`, truncating to `maxdict` entries with `"..."` marker

```titrate
let r: Repr = new Repr();
r.maxlist = 3;
let list = new ArrayList<Variant>();
list.add(1); list.add(2); list.add(3); list.add(4); list.add(5);
io::println(r.repr(list));  // "[1, 2, 3, ...]"
```

## Functions

### repr

- `Pprint.repr(obj: Variant): string` — module-level convenience: repr an object using default `Repr` limits

```titrate
let s: string = Pprint.repr("a very long string ...");
```

## Usage Example

```titrate
import tt::pprint::Pprint;

public fn main(): void {
    let r: Repr = new Repr();
    r.maxstring = 10;
    r.maxlist = 3;
    let nested = new ArrayList<Variant>();
    let inner = new ArrayList<Variant>();
    inner.add("a very long string");
    nested.add(inner);
    io::println(r.repr(nested));
    let big = new ArrayList<Variant>();
    var i: int = 0;
    while (i < 100) {
        big.add(i);
        i = i + 1;
    }
    io::println(Pprint.repr(big));  // "[0, 1, 2, 3, 4, 5, ...]"
}
```
