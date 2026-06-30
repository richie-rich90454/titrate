---
title: json
description: JSON parsing, construction, and serialization in Titrate.
---

# json

The `tt.json` module provides JSON parsing, construction, and serialization. It is the primary way to read and write JSON in Titrate. For advanced features such as streaming, JSONPath, JSON Patch, JSON Schema, JSON5, and binary MessagePack encoding, see [json-advanced](./json-advanced).

```titrate
import tt::json::Json;
import tt::json::JsonValue;
```

## Parsing JSON

- `Json.parse(input: string): JsonValue` — parse a JSON string into a `JsonValue`
- `Json.load(path: string): JsonValue` — read a JSON file and parse it

```titrate
let raw: string = "{\"name\": \"Ada\", \"age\": 42, \"active\": true}";
let value: JsonValue = Json.parse(raw);

let name: string = value.get("name").asString();     // "Ada"
let age: double = value.get("age").asNumber();       // 42.0
let active: bool = value.get("active").asBool();     // true
```

## Building JSON Values

The `JsonValue` class represents any JSON value. Factory functions create values of each JSON type:

- `JsonValue.ofNull(): JsonValue`
- `JsonValue.ofBool(b: bool): JsonValue`
- `JsonValue.ofNum(n: double): JsonValue`
- `JsonValue.ofStr(s: string): JsonValue`
- `JsonValue.ofArray(arr: ArrayList<JsonValue>): JsonValue`
- `JsonValue.ofObject(obj: HashMap<string, JsonValue>): JsonValue`

```titrate
import tt::util::ArrayList;
import tt::util::HashMap;

let scores: ArrayList<JsonValue> = new ArrayList<JsonValue>();
scores.add(JsonValue.ofNum(95.0));
scores.add(JsonValue.ofNum(87.0));

let person: HashMap<string, JsonValue> = new HashMap<string, JsonValue>();
person.put("name", JsonValue.ofStr("Bob"));
person.put("scores", JsonValue.ofArray(scores));
person.put("active", JsonValue.ofBool(true));

let doc: JsonValue = JsonValue.ofObject(person);
io::println(Json.stringify(doc, 2, false));
```

## Serializing JSON

- `Json.stringify(value: JsonValue): string` — compact serialization
- `Json.stringify(value: JsonValue, indent: int, sortKeys: bool): string` — pretty or sorted output
- `Json.stringify(value: JsonValue, indent: int, sortKeys: bool, ensureAscii: bool): string` — also escape non-ASCII
- `Json.prettyPrint(value: JsonValue, indent: int): string` — pretty print alias
- `Json.dumps(value: JsonValue, sortKeys: bool, indent: int): string`

```titrate
let compact: string = Json.stringify(doc);
let pretty: string = Json.stringify(doc, 2, true);
let sorted: string = Json.stringifySorted(doc, 2);
let ascii: string = Json.stringifyAscii(doc, 2);
```

## Writing JSON to Files

- `Json.dump(value: JsonValue, path: string): void` — write compact JSON to a file
- `Json.dump(obj: Variant, filePath: string, indent: int, sortKeys: bool): void` — write a `Variant` with formatting

```titrate
Json.dump(doc, "data.json");
```

## Inspecting JsonValue

| Check | Accessor |
|-------|----------|
| `value.isNull()` | — |
| `value.isBool()` | `value.asBool()` |
| `value.isNumber()` | `value.asNumber()` |
| `value.isString()` | `value.asString()` |
| `value.isArray()` | `value.asArray()` |
| `value.isObject()` | `value.asObject()` |

- `value.get(key: string): JsonValue` — object key access, returns null-value if missing
- `value.getAt(index: int): JsonValue` — array index access
- `value.hasKey(key: string): bool`
- `value.keys(): ArrayList<string>`
- `value.size(): int`
- `value.toString(): string`

## Structural Operations

- `value.deepCopy(): JsonValue` — create a deep copy
- `value.merge(other: JsonValue): void` — merge another object into this object (mutating)
- `value.deepMerge(other: JsonValue): JsonValue` — recursive merge, returns new value
- `value.diff(other: JsonValue): ArrayList<string>` — JSON Pointer paths that differ
- `value.flatten(): JsonValue` — flatten nested objects to dot-notation keys
- `value.unflatten(): JsonValue` — reverse a flatten operation
- `value.pick(keys: ArrayList<string>): JsonValue` — new object with only selected keys
- `value.omit(keys: ArrayList<string>): JsonValue` — new object without selected keys
- `value.transform(f: fn(JsonValue): JsonValue): JsonValue` — recursively transform values
- `value.schema(): JsonValue` — infer a JSON Schema from the value

## JSON Pointer

- `value.path(pointer: string): JsonValue` — get value at a JSON Pointer (RFC 6901)
- `value.set(pointer: string, value: JsonValue): JsonValue` — set value at a pointer, returning a new value

```titrate
let doc: JsonValue = Json.parse("{\"a\": {\"b\": [10, 20, 30]}}");
let found: JsonValue = doc.path("/a/b/1");
io::println(found.asNumber());  // 20.0
```

## Case Conversion Helpers

- `Json.toCamelCase(s: string): string` — `snake_case` to `camelCase`
- `Json.toSnakeCase(s: string): string` — `camelCase` to `snake_case`
- `Json.toPascalCase(s: string): string` — to `PascalCase`

## Terminal Output

- `Json.stringifyColor(value: JsonValue): string` — colorized compact JSON
- `Json.stringifyColorPretty(value: JsonValue, indent: int): string` — colorized pretty JSON
- `Json.stringifyWrapped(value: JsonValue, maxWidth: int): string` — smart line wrapping
