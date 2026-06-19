# serialization

The `tt.json`, `tt.csv`, and `tt.xml` modules provide parsing and serialization for common data formats.

```titrate
import tt.json.Json;
import tt.json.JsonValue;
import tt.csv.CsvReader;
import tt.csv.CsvWriter;
import tt.xml.Xml;
import tt.xml.XmlNode;
```

## JSON

### Json

Static methods for parsing and serializing JSON.

- `Json.parse(input: string): JsonValue` — parse a JSON string into a `JsonValue`
- `Json.stringify(value: JsonValue): string` — serialize a `JsonValue` to compact JSON
- `Json.prettyPrint(value: JsonValue, indent: int): string` — serialize with indentation
- `Json.stringifyColor(value: JsonValue): string` — serialize with ANSI color codes for terminal
- `Json.stringifyColorPretty(value: JsonValue, indent: int): string` — color + indentation
- `Json.stringifyWrapped(value: JsonValue, maxWidth: int): string` — smart line wrapping at column width

```titrate
let data = Json.parse("{\"name\": \"Alice\", \"age\": 30}");
let name: string = data.get("name").asString();  // "Alice"
let age: double = data.get("age").asNumber();     // 30.0
```

### JsonValue

Represents any JSON value with type-safe accessors.

**Factory methods:**
- `JsonValue.ofNull(): JsonValue` — create a null value
- `JsonValue.ofBool(b: bool): JsonValue` — create a boolean
- `JsonValue.ofNum(n: double): JsonValue` — create a number
- `JsonValue.ofStr(s: string): JsonValue` — create a string
- `JsonValue.ofArray(arr: ArrayList<JsonValue>): JsonValue` — create an array
- `JsonValue.ofObject(obj: HashMap<string, JsonValue>): JsonValue` — create an object

**Type checks:**
- `isNull(): bool`, `isBool(): bool`, `isNumber(): bool`, `isString(): bool`, `isArray(): bool`, `isObject(): bool`

**Accessors:**
- `asBool(): bool`, `asNumber(): double`, `asString(): string`, `asArray(): ArrayList<JsonValue>`, `asObject(): HashMap<string, JsonValue>`
- `get(key: string): JsonValue` — get object field
- `getAt(index: int): JsonValue` — get array element
- `hasKey(key: string): bool` — check object key
- `keys(): ArrayList<string>` — object keys
- `size(): int` — array length or object size
- `deepCopy(): JsonValue` — deep clone
- `merge(other: JsonValue): void` — merge another object into this one
- `pick(keys: ArrayList<string>): JsonValue` — select subset of object keys
- `omit(keys: ArrayList<string>): JsonValue` — exclude object keys
- `transform(f: fn(JsonValue): JsonValue): JsonValue` — recursively transform values
- `schema(): JsonValue` — infer JSON Schema from value structure
- `equals(other: JsonValue): bool` — deep equality comparison

```titrate
let arr = new ArrayList<JsonValue>();
arr.add(JsonValue.ofStr("hello"));
arr.add(JsonValue.ofNum(42));
let root = JsonValue.ofArray(arr);
io::println(Json.prettyPrint(root, 2));
```

## CSV

### CsvReader

Parse CSV text with configurable delimiter and quote character.

- `fn init()` — create with defaults (comma delimiter, double-quote, has header)
- `setDelimiter(d: string): void` — set field delimiter
- `setQuote(q: string): void` — set quote character
- `setHasHeader(h: bool): void` — set whether first row is a header
- `parse(input: string): ArrayList<ArrayList<string>>` — parse into rows
- `parseToMaps(input: string): ArrayList<HashMap<string, string>>` — parse rows into maps (keyed by header)
- `getColumn(rows: ArrayList<ArrayList<string>>, colIndex: int): ArrayList<string>` — extract a column by index
- `getColumnByName(rows: ArrayList<ArrayList<string>>, colName: string): ArrayList<string>` — extract a column by header name
- `getHeaders(input: string): ArrayList<string>` — get header row
- `skipLines(input: string, lines: int): string` — skip the first N lines

```titrate
let reader = new CsvReader();
let rows: ArrayList<HashMap<string, string>> = reader.parseToMaps("name,age\nAlice,30\nBob,25");
// [{name: "Alice", age: "30"}, {name: "Bob", age: "25"}]
```

### CsvWriter

Write CSV with configurable delimiter, quote, and newline.

- `fn init()` — create with defaults (comma, double-quote, `\n`)
- `write(rows: ArrayList<ArrayList<string>>): string` — serialize rows to CSV
- `writeWithHeaders(headers: ArrayList<string>, rows: ArrayList<ArrayList<string>>): string` — serialize with header row

```titrate
let writer = new CsvWriter();
let headers = new ArrayList<string>();
headers.add("x"); headers.add("y");
let rows = new ArrayList<ArrayList<string>>();
// ... add rows
let csv: string = writer.writeWithHeaders(headers, rows);
```

## Advanced JSON

### JsonStreamingParser

SAX-style streaming JSON parser for memory-efficient processing of large files.

- `fn init()` — create a streaming parser
- `onStartObject(handler: fn(): void): void` — register object start handler
- `onEndObject(handler: fn(): void): void` — register object end handler
- `onStartArray(handler: fn(): void): void` — register array start handler
- `onEndArray(handler: fn(): void): void` — register array end handler
- `onKey(handler: fn(string): void): void` — register key handler
- `onValue(handler: fn(JsonValue): void): void` — register value handler
- `feed(chunk: string): void` — feed a chunk of JSON text
- `finish(): void` — signal end of input
- `currentPath(): string` — current location as JSON Pointer

```titrate
let parser = new JsonStreamingParser();
parser.onKey(fn(k: string): void { io::println("key: " + k); });
parser.onValue(fn(v: JsonValue): void { io::println("value: " + Json.stringify(v)); });
parser.feed("{\"name\": \"Alice\"}");
parser.finish();
```

### JsonPath

Query JSON documents using JSON Path expressions.

- `JsonPath.query(data: JsonValue, path: string): ArrayList<JsonValue>` — evaluate a path expression
- `JsonPath.compile(path: string): JsonPathExpr` — compile for repeated evaluation
- `JsonPathExpr.evaluate(data: JsonValue): ArrayList<JsonValue>` — evaluate compiled expression

```titrate
let data = Json.parse("{\"users\": [{\"name\": \"Alice\", \"age\": 30}, {\"name\": \"Bob\", \"age\": 25}]}");
let names = JsonPath.query(data, "$.users[*].name");
// [JsonValue.ofStr("Alice"), JsonValue.ofStr("Bob")]
```

### JsonPatch

RFC 6902 JSON Patch and RFC 6901 JSON Pointer.

- `JsonPatch.apply(document: JsonValue, patch: ArrayList<JsonValue>): JsonValue` — apply patch operations
- `JsonPatch.diff(original: JsonValue, modified: JsonValue): ArrayList<JsonValue>` — compute patch
- `JsonPatch.compile(patch: ArrayList<JsonValue>): CompiledPatch` — compile for fast repeated application
- `JsonPointer.get(document: JsonValue, pointer: string): JsonValue` — get value at pointer
- `JsonPointer.set(document: JsonValue, pointer: string, value: JsonValue): JsonValue` — set value at pointer

```titrate
let original = Json.parse("{\"a\": 1}");
let modified = Json.parse("{\"a\": 2, \"b\": 3}");
let patch = JsonPatch.diff(original, modified);
let result = JsonPatch.apply(original, patch);
```

### JsonSchema

JSON Schema Draft 7 and Draft 2020-12 validation.

- `JsonSchema.validate(data: JsonValue, schema: JsonValue): ValidationReport` — validate against schema
- `JsonSchema.compile(schema: JsonValue): CompiledSchema` — compile for fast repeated validation
- `ValidationReport.isValid(): bool` — whether validation passed
- `ValidationReport.getErrors(): ArrayList<ValidationError>` — list of violations
- `ValidationError.getPath(): string` — JSON path to violation
- `ValidationError.getMessage(): string` — error description

```titrate
let schema = Json.parse("{\"type\": \"object\", \"required\": [\"name\"], \"properties\": {\"name\": {\"type\": \"string\"}}}");
let report = JsonSchema.validate(data, schema);
if (!report.isValid()) {
    for (err in report.getErrors()) {
        io::println(err.getPath() + ": " + err.getMessage());
    }
}
```

### Json5

JSON5 parser supporting relaxed syntax.

- `Json5.parse(input: string): JsonValue` — parse JSON5 text
- `Json5.stringify(value: JsonValue): string` — serialize to JSON5 format

```titrate
let data = Json5.parse("{name: 'Alice', age: 30,}");  // unquoted keys, single quotes, trailing comma
```

### JsonBinary

Binary JSON encoding (MessagePack-compatible).

- `JsonBinary.encode(value: JsonValue): ArrayList<byte>` — encode to binary
- `JsonBinary.decode(bytes: ArrayList<byte>): JsonValue` — decode from binary

```titrate
let data = Json.parse("{\"key\": \"value\"}");
let bytes = JsonBinary.encode(data);
let restored = JsonBinary.decode(bytes);
```

## Advanced XML

### XmlNamespace

XML namespace support with prefix-to-URI mapping.

- `fn init()` — create an empty namespace map
- `declare(prefix: string, uri: string): void` — declare a namespace prefix
- `resolveQName(qname: string): string` — resolve prefix:local to {URI}local
- `getURI(prefix: string): string` — get URI for a prefix
- `setDefaultNamespace(uri: string): void` — set default namespace

### XmlStreamingParser

SAX-style streaming XML parser for memory-efficient processing of large files.

- `fn init()` — create a streaming parser
- `onStartElement(handler: fn(string, HashMap<string, string>): void): void` — register start element handler
- `onEndElement(handler: fn(string): void): void` — register end element handler
- `onCharacters(handler: fn(string): void): void` — register character data handler
- `onComment(handler: fn(string): void): void` — register comment handler
- `feed(chunk: string): void` — feed a chunk of XML text
- `finish(): void` — signal end of input

### XPath

XPath 1.0 expression evaluator.

- `XPath.evaluate(node: XmlNode, expression: string): ArrayList<XmlNode>` — evaluate XPath
- `XPath.evaluateString(node: XmlNode, expression: string): string` — evaluate to string
- `XPath.evaluateNumber(node: XmlNode, expression: string): double` — evaluate to number
- `XPath.compile(expression: string): XPathExpr` — compile for repeated evaluation

```titrate
let doc = Xml.parse("<root><item id='1'>A</item><item id='2'>B</item></root>");
let items = XPath.evaluate(doc, "//item[@id='1']");
```

### XmlBuilder

Fluent XML builder API.

- `XmlBuilder.builder(): XmlBuilder` — create a new builder
- `root(tag: string): XmlBuilder` — set root element
- `elem(tag: string): XmlBuilder` — add child element
- `attr(key: string, value: string): XmlBuilder` — add attribute
- `text(content: string): XmlBuilder` — add text content
- `cdata(content: string): XmlBuilder` — add CDATA section
- `comment(content: string): XmlBuilder` — add comment
- `build(): XmlNode` — build the XML tree

```titrate
let doc = XmlBuilder.builder()
    .root("root")
    .elem("item").attr("id", "1").text("hello")
    .build();
```

### XmlSchema

XML Schema validation.

- `XmlSchema.validate(node: XmlNode, schema: XmlNode): ValidationReport` — validate against schema
- `ValidationReport.isValid(): bool` — whether validation passed
- `ValidationReport.getErrors(): ArrayList<ValidationError>` — list of violations

### XmlCanonicalizer

XML Canonicalization (C14N).

- `XmlCanonicalizer.canonicalize(node: XmlNode): string` — Canonical XML 1.0
- `XmlCanonicalizer.exclusiveCanonicalize(node: XmlNode, inclusivePrefixes: ArrayList<string>): string` — Exclusive C14N
