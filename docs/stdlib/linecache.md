# LineCache

The `tt.io.Linecache` module provides a Python `linecache` analog for random access to source lines: `getline`, `getlines`, `clearcache`, and `checkcache`. It caches file contents so repeated lookups by `(filename, lineno)` are cheap.

## Import

```titrate
import tt::io::Linecache;
```

## Functions

### getline

Return the 1-indexed line `lineno` from `filename`, or `""` if unavailable. Mirrors `linecache.getline(filename, lineno, module_globals=None)`.

**Parameters:** `filename: string`, `lineno: int`
**Returns:** `string`

```titrate
let line: string = getline("app.tr", 42);
io::println(line);
```

### getlines

Return all lines of `filename` as an `ArrayList` (1-indexed conceptually). Loads and caches the file on first access. Returns an empty list on failure. Mirrors `linecache.getlines(filename, module_globals=None)`.

**Parameters:** `filename: string`
**Returns:** `ArrayList<string>`

### clearcache

Clear the entire line cache. Mirrors `linecache.clearcache()`.

**Returns:** `void`

### checkcache

Check whether a cached file has changed on disk; evict it if so. Pass `""` to check all cached files. Mirrors `linecache.checkcache(filename=None)`.

**Parameters:** `filename: string`
**Returns:** `void`

```titrate
let lines = getlines("app.tr");
io::println(lines.size());
checkcache("app.tr");
clearcache();
```
