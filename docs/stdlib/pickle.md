# Pickle

The `tt.serialization.Pickle` module mirrors Python's `pickle` module. It implements a Titrate-specific TLV binary format that supports `null`, `bool`, `int`, `long`, `double`, `string`, bytes (as string), `ArrayList` (list), `HashMap<string, V>` (dict), and custom objects via the `"__class__"` convention on `JsonValue` objects.

## Import

```titrate
import tt::serialization::Pickle;
import tt::json::JsonValue;
```

## Constants

- `HIGHEST_PROTOCOL: int = 2`
- `DEFAULT_PROTOCOL: int = 2`

## Stream format

The stream begins with a 5-byte header: `"TTPK"` followed by a single-byte protocol version. Each value is then encoded as a single-character opcode followed by its payload, terminated by `.` (`STOP`):

| Opcode | Constant    | Payload                                                | Meaning            |
|--------|-------------|--------------------------------------------------------|--------------------|
| `.`    | STOP        | none                                                   | end of stream      |
| `N`    | NONE        | none                                                   | `null`             |
| `T`    | TRUE        | none                                                   | boolean `true`     |
| `F`    | FALSE       | none                                                   | boolean `false`    |
| `i`    | INT         | 4-byte LE signed int                                   | 32-bit int         |
| `l`    | LONG        | 8-byte LE signed long                                  | 64-bit long        |
| `d`    | DOUBLE      | 8-byte LE IEEE-754 double                              | double             |
| `s`    | STRING      | 4-byte LE length + UTF-8 bytes                          | string             |
| `b`    | BYTES       | 4-byte LE length + raw bytes                            | bytes (as string)  |
| `L`    | LIST        | 4-byte LE count + items                                | list               |
| `D`    | DICT        | 4-byte LE count + key/value pairs                       | dict               |
| `O`    | OBJECT      | 4-byte LE class_name length + class_name + state dict  | custom object      |
| `R`    | REF         | 4-byte LE memo id                                      | back-reference     |
| `M`    | MEMO        | 4-byte LE memo id + value                              | store in memo      |

## Errors

### PicklingError

Raised when an object cannot be pickled.

- `PicklingError.init(msg: string)`
- `message: string`
- `toString(): string` — returns `"PicklingError: <message>"`

### UnpicklingError

Raised when a pickle stream cannot be unpickled.

- `UnpicklingError.init(msg: string)`
- `message: string`
- `toString(): string` — returns `"UnpicklingError: <message>"`

## Pickler

`Pickler` serializes `JsonValue` objects into a binary stream.

- `Pickler.init()`
- `initWithProtocol(protocol: int)` — use a specific protocol version
- `getProtocol(): int`
- `clearMemo(): void` — clear the object-identity cache
- `dump(obj: JsonValue): string` — serialize `obj` and return the bytes
- `dumpToFile(obj: JsonValue, file: File): void`

```titrate
let p: Pickler = new Pickler();
let data: string = p.dump(JsonValue.ofNum(42.0));
```

## Unpickler

`Unpickler` deserializes a binary stream back into a `JsonValue`.

- `Unpickler.init()`
- `initWithData(data: string)` — initialize with the bytes to deserialize
- `getProtocol(): int`
- `load(): JsonValue` — deserialize the stream and return the root value

```titrate
let u: Unpickler = new Unpickler();
u.initWithData(data);
let v: JsonValue = u.load();
```

## Functions

### dumps

Serialize a `JsonValue` into a binary string.

**Overloads:**
- `dumps(obj: JsonValue): string` — default protocol
- `dumps(obj: JsonValue, protocol: int): string` — specific protocol

**Returns:** `string`

### loads

Deserialize a binary string into a `JsonValue`. Throws `UnpicklingError` on a malformed stream.

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
- `dump(obj: JsonValue, file: File): void`
- `dump(obj: JsonValue, path: string): void`

### load

Read a file and deserialize its contents into a `JsonValue`.

**Overloads:**
- `load(file: File): JsonValue`
- `load(path: string): JsonValue`

## Custom pickling — reducers and constructors

Titrate does not support runtime method dispatch by name, so custom pickling is implemented via a reducer registry. Callers register a reducer for a type tag; the reducer is invoked during pickling to convert an object into a `(className, state)` pair. Constructors rebuild an object from its pickled state.

### registerReducer

Register a reducer that converts an object with the given `__type__` tag into a `JsonValue` with `__class__` and state fields for pickling.

**Parameters:** `typeTag: string`, `reducer: fn(JsonValue): JsonValue`
**Returns:** `void`

### registerConstructor

Register a constructor that rebuilds an object from its pickled state.

**Parameters:** `className: string`, `constructor: fn(JsonValue): JsonValue`
**Returns:** `void`

### reduce

Apply a registered reducer for the value's `__type__` tag. Returns the original value when no reducer is registered.

**Parameters:** `value: JsonValue`
**Returns:** `JsonValue`

### construct

Apply a registered constructor to rebuild an object from its state. Returns the value unchanged when no constructor is registered.

**Parameters:** `value: JsonValue`
**Returns:** `JsonValue`

```titrate
registerReducer("Point", fn(v: JsonValue): JsonValue {
    let state: HashMap<string, JsonValue> = new HashMap<string, JsonValue>();
    state.put("x", v.get("x"));
    state.put("y", v.get("y"));
    let obj: HashMap<string, JsonValue> = new HashMap<string, JsonValue>();
    obj.put("__class__", JsonValue.ofStr("Point"));
    obj.put("__state__", JsonValue.ofObject(state));
    return JsonValue.ofObject(obj);
});
```

## Notes

- Numbers that fit losslessly in a 32-bit signed int are emitted with `INT`; all other numbers use `DOUBLE`.
- A pickled object whose `JsonValue` has a `__class__` key is emitted with the `OBJECT` opcode followed by a `DICT` opcode for its state. The state is taken from the explicit `__state__` key when present, or from all keys except `__class__` and `__state__` otherwise.
- Bytes are stored as a string and marked on unpickling with a `__bytes__` wrapper.
- The memo table enables cyclic and shared object graphs via the `REF` and `MEMO` opcodes.
