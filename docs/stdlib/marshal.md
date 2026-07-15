# Marshal

The `tt.serialization.Marshal` module mirrors Python's `marshal` module. It is a compact internal serialization format used for simple value types. Unlike `Pickle`, `Marshal` does **not** support custom object hooks and is intended for simple, homogeneous data only.

## Import

```titrate
import tt::serialization::Marshal;
import tt::json::JsonValue;
```

## Constants

- `version: int = 4` — the current marshal format version

## Stream format

The stream begins with a 4-byte magic header `"TTMA"` followed by a single-byte version. Each value is then encoded with a 1-byte type code followed by its payload, terminated by `.` (TYPE_STOP):

| Type code | Constant      | Payload                                            | Meaning        |
|-----------|---------------|----------------------------------------------------|----------------|
| `0`       | TYPE_NULL     | none                                               | null terminator |
| `N`       | TYPE_NONE    | none                                               | `null`         |
| `T`       | TYPE_TRUE    | none                                               | boolean `true` |
| `F`       | TYPE_FALSE   | none                                               | boolean `false` |
| `i`       | TYPE_INT     | 4-byte LE signed int                              | 32-bit int     |
| `l`       | TYPE_LONG    | 8-byte LE signed long                             | 64-bit long    |
| `f`       | TYPE_FLOAT   | 8-byte LE IEEE-754 double                         | double         |
| `s`       | TYPE_STRING  | 4-byte LE length + UTF-8 bytes                    | string         |
| `b`       | TYPE_BYTES   | 4-byte LE length + raw bytes                       | bytes          |
| `L`       | TYPE_LIST    | 4-byte LE count + items                            | list           |
| `t`       | TYPE_TUPLE   | 4-byte LE count + items (alias for list)           | tuple/list     |
| `D`       | TYPE_DICT    | 4-byte LE count + key/value pairs                 | dict           |
| `.`       | TYPE_STOP    | none                                               | end of stream  |

## Errors

### ValueError

Raised when a marshal stream cannot be parsed or contains unsupported types.

- `ValueError.init(msg: string)`
- `message: string`
- `toString(): string` — returns `"ValueError: <message>"`

## Functions

### dumps

Serialize a `JsonValue` into a compact binary string.

**Parameters:** `value: JsonValue`
**Returns:** `string`

```titrate
let blob: string = dumps(JsonValue.ofNum(42.0));
```

### loads

Deserialize a binary string into a `JsonValue`. Throws `ValueError` on a malformed stream.

**Parameters:** `data: string`
**Returns:** `JsonValue`

```titrate
let blob: string = dumps(JsonValue.ofStr("hello"));
let back: JsonValue = loads(blob);
io::println(back.asString());  // hello
```

### dump

Serialize a `JsonValue` and write it to a file.

**Overloads:**
- `dump(value: JsonValue, file: File): void`
- `dump(value: JsonValue, path: string): void`

### load

Read a file and deserialize its contents into a `JsonValue`.

**Overloads:**
- `load(file: File): JsonValue`
- `load(path: string): JsonValue`

```titrate
dump(JsonValue.ofNum(3.14), "data.ttm");
let v: JsonValue = load("data.ttm");
```

## Notes

- Numbers that fit losslessly in a 32-bit signed int are emitted with `INT`; all other numbers use `FLOAT`.
- The magic header is `"TTMA"` (Titrate Marshal); this distinguishes marshal streams from pickle streams (which use `"TTPK"`).
- Bytes are stored as a string and tagged on unpickling with a `__bytes__` wrapper.
- `TUPLE` is decoded identically to `LIST` — both produce a `JsonValue` array.
- The marshal format is **not** byte-compatible with CPython's marshal format; it is a Titrate-specific format that supports the same use cases.
