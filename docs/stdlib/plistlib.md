# PlistLib

The `tt.serialization.PlistLib` module mirrors Python's `plistlib` module. It reads and writes Apple property lists in both XML and binary formats. Property lists can contain `dict`, `array`, `string`, `integer`, `real`, `bool`, `null`, `data` (raw bytes stored as string), and `date` (ISO 8601 string).

## Import

```titrate
import tt::serialization::PlistLib;
import tt::json::JsonValue;
```

## Constants

- `FMT_XML: int = 1` — XML property list format
- `FMT_BINARY: int = 2` — binary property list format

## Errors

### PlistError

Raised when a plist cannot be parsed or contains unsupported types.

- `PlistError.init(msg: string)`
- `message: string`
- `toString(): string` — returns `"PlistError: <message>"`

## Functions

### load

Read a plist from the file at `path`. The format is auto-detected from the file's leading bytes (`bplist00` magic for binary, otherwise XML).

**Parameters:** `path: string`
**Returns:** `JsonValue`

### loads

Parse a plist from a string. Auto-detects XML vs binary by checking the leading `bplist` magic.

**Parameters:** `text: string`
**Returns:** `JsonValue`

```titrate
let text: string = "<?xml version=\"1.0\"?>\n<plist version=\"1.0\">\n";
text = text + "<dict><key>name</key><string>Alice</string></dict>\n</plist>";
let v: JsonValue = loads(text);
io::println(v.get("name").asString());  // Alice
```

### dump

Write `value` as a plist to the file at `path` in the given format.

**Parameters:** `value: JsonValue`, `path: string`, `fmt: int`
**Returns:** `void`

### dumps

Serialize `value` to a plist string in the given format (use `FMT_XML` or `FMT_BINARY`).

**Parameters:** `value: JsonValue`, `fmt: int`
**Returns:** `string`

```titrate
let obj: HashMap<string, JsonValue> = new HashMap<string, JsonValue>();
obj.put("name", JsonValue.ofStr("Alice"));
obj.put("age", JsonValue.ofNum(30.0));
let xml: string = dumps(JsonValue.ofObject(obj), FMT_XML);
```

## Convenience helpers

### loadXml

Read a plist from `path`, forcing the XML format.

**Parameters:** `path: string`
**Returns:** `JsonValue`

### loadBinary

Read a plist from `path`, forcing the binary format.

**Parameters:** `path: string`
**Returns:** `JsonValue`

### dumpsXml

Serialize `value` as an XML plist to a string (convenience for `dumps(value, FMT_XML)`).

**Parameters:** `value: JsonValue`
**Returns:** `string`

### dumpsBinary

Serialize `value` as a binary plist to a string (convenience for `dumps(value, FMT_BINARY)`).

**Parameters:** `value: JsonValue`
**Returns:** `string`

## XML format details

- The XML document has the form `<?xml ...?>`, `<!DOCTYPE plist ...>`, `<plist version="1.0">` ... `</plist>`.
- `dict` becomes `<dict>` with `<key>name</key>` followed by the value.
- `array` becomes `<array>` with each element.
- `bool` becomes `<true/>` or `<false/>`.
- Numbers that fit losslessly in a 32-bit signed int become `<integer>`; all other numbers become `<real>`.
- `string` becomes `<string>` (escaped for the five XML special characters).
- `data` is emitted as `<data>base64...</data>` and parsed into `{"__data__": "<b64>"}`.
- `date` is emitted as `<date>ISO 8601</date>` and parsed into `{"__date__": "<iso8601>"}`.

## Binary format details

The binary format is a Titrate-specific simplified text-based representation, **not** byte-compatible with Apple's CFBinaryPList. It uses a single-character type tag per line:

- `n` — null
- `T` / `F` — boolean
- `i<integer>` — 32-bit int
- `r<real>` — double
- `s<string>` — string (newlines escaped as `\n`, backslashes as `\\`)
- `a<count>` followed by `count` values — array
- `d<count>` followed by `count` `s<key>` + value pairs — dict

The stream begins with the `bplist00` magic header and ends with an 8-byte zero trailer.

## Notes

- The XML parser is a small tag-based parser tolerant of whitespace and attribute variations on `<plist>`.
- All five XML special characters (`<`, `>`, `&`, `"`, `'`) are escaped on output and unescaped on input.
- Numbers are emitted as `<integer>` when they fit losslessly in a 32-bit signed int; otherwise they are emitted as `<real>`.
- The `__data__` and `__date__` wrappers are preserved round-trip: an XML `<data>` element parses to `{"__data__": "<b64>"}`, which dumps back to `<data>`.
