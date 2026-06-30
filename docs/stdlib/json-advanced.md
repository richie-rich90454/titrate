---
title: json-advanced
description: Advanced JSON processing in Titrate — streaming, JSONPath, Patch, Schema, JSON5, and MessagePack.
---

# json-advanced

The `tt.json` module provides advanced JSON processing capabilities including SAX-style streaming parsing, JSONPath queries, JSON Patch, JSON Schema validation, JSON5 support, and binary JSON encoding.

```titrate
import tt::json::JsonStreamingParser;
import tt::json::JsonPath;
import tt::json::JsonPatch;
import tt::json::JsonSchema;
import tt::json::Json5;
import tt::json::JsonBinary;
```

## JsonStreamingParser

A SAX-style streaming JSON parser that emits events as the document is read. Memory-efficient for large files since it never builds a full object tree in memory.

- `JsonStreamingParser.new(): JsonStreamingParser` — create a new parser with default settings
- `JsonStreamingParser.onStartObject(handler: fn(): void): JsonStreamingParser` — set handler for object start (`{`)
- `JsonStreamingParser.onEndObject(handler: fn(): void): JsonStreamingParser` — set handler for object end (`}`)
- `JsonStreamingParser.onStartArray(handler: fn(): void): JsonStreamingParser` — set handler for array start (`[`)
- `JsonStreamingParser.onEndArray(handler: fn(): void): JsonStreamingParser` — set handler for array end (`]`)
- `JsonStreamingParser.onKey(handler: fn(string): void): JsonStreamingParser` — set handler for object keys
- `JsonStreamingParser.onValue(handler: fn(Variant): void): JsonStreamingParser` — set handler for primitive values (string, number, bool, null)
- `JsonStreamingParser.parse(json: string): void` — parse a complete JSON string
- `JsonStreamingParser.feed(chunk: string): void` — feed a chunk of input for incremental parsing
- `JsonStreamingParser.finish(): void` — signal end of input for chunked parsing
- `JsonStreamingParser.currentPath(): string` — return the current JSON Pointer path (e.g., `/stores/0/name`)
- `JsonStreamingParser.currentDepth(): int` — return the current nesting depth
- `JsonStreamingParser.setMaxDepth(depth: int): void` — set maximum nesting depth limit (default: 256)
- `JsonStreamingParser.recoverOnErrors(recover: bool): void` — enable error recovery mode
- `JsonStreamingParser.iterator(): JsonPullIterator` — create a pull-style iterator for manual event consumption

```titrate
let parser = JsonStreamingParser.new();
parser.onKey(fn(key: string): void {
    io::println("Key: " + key);
});
parser.onValue(fn(val: Variant): void {
    io::println("Value: " + val.asString());
});
parser.parse('{"name": "Alice", "age": 30}');
```

### Chunked Input Feeding

For very large files, feed the parser incrementally:

```titrate
let parser = JsonStreamingParser.new();
parser.onStartObject(fn(): void {
    io::println("Object start");
});
parser.onKey(fn(key: string): void {
    io::println("Key: " + key);
});

// Feed chunks as they arrive
parser.feed('{"name":');
parser.feed('"Alice",');
parser.feed('"age":30}');
parser.finish();
```

### Path Tracking (JSON Pointer)

The parser tracks the current location as a JSON Pointer string:

```titrate
let parser = JsonStreamingParser.new();
parser.onValue(fn(val: Variant): void {
    io::println(parser.currentPath() + " = " + val.asString());
});
parser.parse('{"users": [{"name": "Alice"}, {"name": "Bob"}]}');
// Output:
// /users/0/name = Alice
// /users/1/name = Bob
```

### Pull-Style Iteration

Use the pull iterator for manual, demand-driven parsing:

```titrate
let parser = JsonStreamingParser.new();
let iter = parser.iterator('{"a":1,"b":2}');

while (iter.hasNext()) {
    let event = iter.next();
    // event.type: "start_object", "end_object", "key", "value", etc.
    io::println(event.type + ": " + event.value.asString());
}
```

## JsonPath

JSONPath query evaluator for selecting nodes from JSON documents using path expressions.

- `JsonPath.query(expression: string, json: string): ArrayList<Variant>` — evaluate a JSONPath expression, return matching values
- `JsonPath.queryOnValue(expression: string, value: JsonValue): ArrayList<Variant>` — evaluate against a parsed `JsonValue`
- `JsonPath.queryFirst(expression: string, json: string): Variant` — return the first match, or `null`
- `JsonPath.count(expression: string, json: string): int` — count matching nodes
- `JsonPath.compile(expression: string): CompiledJsonPath` — pre-compile an expression for repeated evaluation

```titrate
let json = '{"store": {"books": [{"title": "A", "price": 10}, {"title": "B", "price": 25}]}}';
let titles = JsonPath.query("$.store.books[*].title", json);
// titles = ["A", "B"]

let expensive = JsonPath.query("$.store.books[?(@.price > 15)]", json);
// expensive = [{"title": "B", "price": 25}]
```

### Path Expressions

| Syntax | Description |
|--------|-------------|
| `$` | Root node |
| `.child` | Child navigation |
| `[]` | Array index or bracket notation |
| `..` | Recursive descent |
| `[*]` | Wildcard — all elements/children |
| `[?(@.filter)]` | Filter expression |
| `[start:end]` | Array slice |

```titrate
// Root access
let root = JsonPath.query("$", json);

// Child navigation
let store = JsonPath.query("$.store", json);

// Array index
let first = JsonPath.query("$.store.books[0]", json);

// Recursive descent
let allTitles = JsonPath.query("$..title", json);

// Wildcard
let allBooks = JsonPath.query("$.store.books[*]", json);
```

### Filter Expressions

Filter expressions use `@` to refer to the current node:

```titrate
// Comparison filter
let cheap = JsonPath.query("$.store.books[?(@.price < 20)]", json);

// Logical operators in filters
let result = JsonPath.query("$.store.books[?(@.price > 5 && @.price < 20)]", json);

// Regex filter
let matched = JsonPath.query("$.store.books[?(@.title =~ /pattern/)]", json);
```

### Filter Functions

Built-in functions available inside filter expressions:

| Function | Description |
|----------|-------------|
| `length(@)` | Length of array or string |
| `size(@)` | Size of array or object |
| `keys(@)` | Keys of an object |
| `values(@)` | Values of an object |
| `contains(@, value)` | Check if array contains value or string contains substring |

```titrate
// Filter by array length
let multiAuthor = JsonPath.query("$.store.books[?(length(@.authors) > 1)]", json);

// Filter by key existence
let withPrice = JsonPath.query("$.store.books[?(contains(keys(@), 'price'))]", json);
```

### Array Slicing

```titrate
let json = '{"items": [0, 1, 2, 3, 4, 5]}';

let firstThree = JsonPath.query("$.items[0:3]", json);  // [0, 1, 2]
let lastTwo = JsonPath.query("$.items[-2:]", json);      // [4, 5]
let allFrom2 = JsonPath.query("$.items[2:]", json);      // [2, 3, 4, 5]
```

### Compiled Expressions

Pre-compile JSONPath expressions for repeated evaluation:

```titrate
let compiled = JsonPath.compile("$.store.books[?(@.price < 20)]");
let result1 = compiled.evaluate(json1);
let result2 = compiled.evaluate(json2);
```

## JsonPatch

RFC 6902 JSON Patch and RFC 6901 JSON Pointer implementation for manipulating JSON documents.

- `JsonPatch.apply(document: JsonValue, patch: ArrayList<PatchOp>): JsonValue` — apply a patch to a document, return the result
- `JsonPatch.applyFromString(document: JsonValue, patchJson: string): JsonValue` — apply a patch from a JSON string
- `JsonPatch.diff(original: JsonValue, modified: JsonValue): ArrayList<PatchOp>` — compute the diff between two documents
- `JsonPatch.compile(patchJson: string): ArrayList<PatchOp>` — pre-compile a patch for repeated application
- `JsonPatch.pointer(document: JsonValue, pointer: string): Variant` — evaluate a JSON Pointer, return the referenced value
- `JsonPatch.pointerSet(document: JsonValue, pointer: string, value: Variant): JsonValue` — set a value at a JSON Pointer location

### PatchOp Operations

| Operation | Description |
|-----------|-------------|
| `add` | Add a value at the target location |
| `remove` | Remove the value at the target location |
| `replace` | Replace the value at the target location |
| `move` | Move a value from one location to another |
| `copy` | Copy a value from one location to another |
| `test` | Test that a value at the location equals the expected value |

```titrate
let doc = JsonValue.parse('{"baz": "qux", "foo": "bar"}');

// Build patch operations
let patch = ArrayList.new();
patch.add(PatchOp.add("/foo", JsonValue.of("baz")));
patch.add(PatchOp.remove("/baz"));

let result = JsonPatch.apply(doc, patch);
// result = {"foo": "baz"}
```

### JSON Pointer (RFC 6901)

JSON Pointer uses `/`-separated paths to reference locations within a JSON document:

```titrate
let doc = JsonValue.parse('{"a": {"b": {"c": 42}}}');
let val = JsonPatch.pointer(doc, "/a/b/c"); // 42

// Array indexing
let arr = JsonValue.parse('{"items": [10, 20, 30]}');
let second = JsonPatch.pointer(arr, "/items/1"); // 20
```

### Diff Generation

Automatically compute the patch between two JSON documents:

```titrate
let original = JsonValue.parse('{"name": "Alice", "age": 30}');
let modified = JsonValue.parse('{"name": "Bob", "age": 30, "city": "NYC"}');

let patch = JsonPatch.diff(original, modified);
// patch contains: replace /name, add /city
```

## JsonSchema

JSON Schema validation supporting Draft 7 and Draft 2020-12. Validates JSON documents against schema definitions and produces detailed validation reports.

- `JsonSchema.new(): JsonSchema` — create a new validator (default: Draft 7)
- `JsonSchema.draft2020(): JsonSchema` — create a Draft 2020-12 validator
- `JsonSchema.loadSchema(schema: string): void` — load a JSON Schema from a string
- `JsonSchema.loadSchemaFromPath(path: string): void` — load a JSON Schema from a file path
- `JsonSchema.validate(json: string): bool` — validate a JSON string against loaded schemas
- `JsonSchema.validateValue(value: JsonValue): bool` — validate a parsed `JsonValue`
- `JsonSchema.validateWithReport(json: string): ValidationReport` — validate and return detailed report
- `JsonSchema.addSchema(uri: string, schema: string): void` — register a schema by URI for `$ref` resolution

### ValidationReport

- `ValidationReport.isValid(): bool` — whether validation passed
- `ValidationReport.errors(): ArrayList<ValidationError>` — list of validation errors
- `ValidationError.path(): string` — JSON Pointer path to the invalid node
- `ValidationError.message(): string` — human-readable error description
- `ValidationError.schemaLocation(): string` — location in the schema that triggered the error
- `ValidationError.keyword(): string` — the schema keyword that failed (e.g., `"type"`, `"minimum"`)

```titrate
let validator = JsonSchema.new();
validator.loadSchema("""
{
    "type": "object",
    "properties": {
        "name": {"type": "string"},
        "age": {"type": "integer", "minimum": 0}
    },
    "required": ["name"]
}
""");

let report = validator.validateWithReport('{"age": -5}');
if (!report.isValid()) {
    for (err in report.errors()) {
        io::println(err.path() + ": " + err.message());
    }
    // /age: must be greater than or equal to 0
    // : missing required property "name"
}
```

### Supported Validation Keywords

**Type keywords:** `type`, `enum`, `const`

**Numeric keywords:** `minimum`, `maximum`, `exclusiveMinimum`, `exclusiveMaximum`, `multipleOf`

**String keywords:** `minLength`, `maxLength`, `pattern`, `format`

**Array keywords:** `items`, `minItems`, `maxItems`, `uniqueItems`, `contains`

**Object keywords:** `properties`, `required`, `minProperties`, `maxProperties`, `additionalProperties`, `patternProperties`

**Combination keywords:** `allOf`, `anyOf`, `oneOf`, `not`

**Reference keywords:** `$ref`, `$defs` / `definitions`

```titrate
// Complex schema with composition and references
let validator = JsonSchema.new();
validator.loadSchema("""
{
    "$schema": "http://json-schema.org/draft-07/schema#",
    "type": "object",
    "properties": {
        "email": {"type": "string", "format": "email"},
        "tags": {
            "type": "array",
            "items": {"type": "string"},
            "uniqueItems": true
        }
    },
    "allOf": [
        {"required": ["email"]},
        {"not": {"required": ["password"]}}
    ]
}
""");
```

### Draft 2020-12

Use the Draft 2020-12 validator for schemas that use newer keywords:

```titrate
let validator = JsonSchema.draft2020();
validator.loadSchema("""
{
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "type": "object",
    "properties": {
        "items": {
            "type": "array",
            "prefixItems": [
                {"type": "string"},
                {"type": "integer"}
            ],
            "items": false
        }
    }
}
""");
```

## Json5

JSON5 parser and serializer supporting the JSON5 superset of JSON. JSON5 allows more human-friendly syntax while remaining compatible with standard JSON output.

- `Json5.parse(input: string): JsonValue` — parse a JSON5 string into a `JsonValue`
- `Json5.stringify(value: JsonValue): string` — serialize a `JsonValue` to standard JSON (not JSON5)
- `Json5.stringify5(value: JsonValue): string` — serialize a `JsonValue` to JSON5 format
- `Json5.isValid(input: string): bool` — check if a string is valid JSON5

### JSON5 Extensions over JSON

**Unquoted keys:**
```titrate
let obj = Json5.parse('{name: "Alice", age: 30}');
```

**Trailing commas:**
```titrate
let arr = Json5.parse('[1, 2, 3,]');
let obj = Json5.parse('{a: 1, b: 2,}');
```

**Single-quoted strings:**
```titrate
let val = Json5.parse("{'key': 'value'}");
```

**Multiline strings:**
```titrate
let text = Json5.parse('{msg: "hello\\nworld"}');
```

**Comments:**
```titrate
let obj = Json5.parse("""
{
    // User configuration
    name: "Alice", /* inline */ age: 30
}
""");
```

**Hexadecimal numbers:**
```titrate
let hex = Json5.parse('{flags: 0xFF}');
```

**Infinity and NaN:**
```titrate
let special = Json5.parse('{inf: Infinity, nan: NaN}');
```

### Serialization

```titrate
let obj = Json5.parse('{name: "Alice", age: 30}');

// Standard JSON output
let json = Json5.stringify(obj);  // {"name":"Alice","age":30}

// JSON5 output (preserves unquoted keys, trailing commas)
let json5 = Json5.stringify5(obj);
```

## JsonBinary

Binary JSON encoding using a MessagePack-compatible format for compact serialization and fast parsing.

- `JsonBinary.encode(value: JsonValue): ArrayList<int>` — encode a `JsonValue` to binary bytes
- `JsonBinary.decode(bytes: ArrayList<int>): JsonValue` — decode binary bytes to a `JsonValue`
- `JsonBinary.encodeToString(value: JsonValue): string` — encode and return as a Base64 string
- `JsonBinary.decodeFromString(encoded: string): JsonValue` — decode from a Base64 string
- `JsonBinary.encodeStreaming(value: JsonValue, output: fn(ArrayList<int>): void): void` — encode and stream output chunks
- `JsonBinary.registerExtension(typeId: int, encoder: fn(Variant): ArrayList<int>, decoder: fn(ArrayList<int>): Variant): void` — register a custom extension type

```titrate
let obj = JsonValue.parse('{"name": "Alice", "scores": [95, 87, 92]}');

// Encode to binary
let bytes = JsonBinary.encode(obj);

// Decode back
let decoded = JsonBinary.decode(bytes);

// Encode to Base64 string for transport
let b64 = JsonBinary.encodeToString(obj);
let restored = JsonBinary.decodeFromString(b64);
```

### Streaming Encoding

For large documents, stream encoded chunks to avoid building the entire byte array in memory:

```titrate
let obj = JsonValue.parse(largeJson);
JsonBinary.encodeStreaming(obj, fn(chunk: ArrayList<int>): void {
    // Write chunk to file, socket, etc.
    File.writeBytes("output.msgpack", chunk);
});
```

### Extension Types

Register custom extension types for serializing types not natively supported by JSON:

```titrate
// Register extension type 1 for datetime strings
JsonBinary.registerExtension(
    1,
    fn(val: Variant): ArrayList<int> {
        // Encode datetime to bytes
        return String.toBytes(val.asString());
    },
    fn(bytes: ArrayList<int>): Variant {
        // Decode bytes back to datetime string
        return Variant.of(String.fromBytes(bytes));
    }
);
```
