# Shelve

The `tt.io.Shelve` module provides a Python `shelve` analog for persistent object storage: `Shelf`, `DbfilenameShelf`, `BsdDbShelf`, and `open`. A shelf is a persistent dictionary whose values are serialized to disk.

## Import

```titrate
import tt::io::Shelve;
```

## Classes

### Shelf

Base class: a dict-like mapping persisted to a backing store.

**Fields:**
- `filename: string`
- `flag: string`
- `writeback: bool`
- `cache: HashMap<string, Variant>`
- `dirty: HashMap<string, bool>`

**Methods:**
- `init(filename: string, flag: string)` — load the backing store into the cache
- `sync(): void` — persist the cache back to the backing store
- `get(key: string): Variant` — `null` if absent
- `put(key: string, value: Variant): void` — store; if `writeback` is `false`, syncs immediately
- `remove(key: string): void`
- `containsKey(key: string): bool`
- `keys(): ArrayList<string>`
- `size(): int`
- `close(): void` — flush any pending writes
- `items(): ArrayList<string>` — iterate keys

### DbfilenameShelf

A shelf backed by a single file using JSON serialization. Extends `Shelf`.

**Constructor:** `DbfilenameShelf(filename: string, flag: string)`

### BsdDbShelf

A shelf that delegates to an external key-value store (simulated with an in-memory `HashMap`). Extends `Shelf`.

**Fields:**
- `_db: HashMap<string, Variant>`

**Constructor:** `BsdDbShelf(db: HashMap<string, Variant>, filename: string, flag: string)`

Overrides `put` and `remove` to delegate to `_db` in addition to the base.

## Functions

### open

Open a shelf. `flag` mirrors Python: `"c"` (create/read/write, default), `"r"` (read-only), `"w"` (read/write, truncate), `"n"` (new, truncate). Throws `FileNotFoundError` for `"r"` if the file does not exist.

**Parameters:** `filename: string`, `flag: string`
**Returns:** `Shelf` (a `DbfilenameShelf`)

```titrate
let db: Shelf = open("data.shelf", "c");
db.put("name", "alice");
let v: Variant = db.get("name");
db.close();
```
