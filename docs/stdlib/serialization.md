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

```titrate
let data = Json.parse("{\"name\": \"Alice\", \"age\": 30}");
let name: string = data.get("name").asString();  // "Alice"
let age: double = data.get("age").asNumber();     // 30.0
```

### JsonValue

Represents any JSON value with type-safe accessors.

**Factory methods:**
- `JsonValue.null(): JsonValue` — create a null value
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

## XML

### Xml

Static method for parsing XML.

- `Xml.parse(input: string): XmlNode` — parse XML string into a tree

```titrate
let doc = Xml.parse("<root><item key=\"a\">hello</item></root>");
let val: string = doc.getChildrenByTag("item").get(0).getText();  // "hello"
```

### XmlNode

Represents an XML element with tag, attributes, children, and text.

- `fn init(tag: string)` — create a node with the given tag
- `getTag(): string` — element tag name
- `getText(): string` — text content
- `getAttr(name: string): string` — attribute value (empty string if missing)
- `setAttr(name: string, value: string): void` — set attribute
- `hasAttr(key: string): bool` — check attribute existence
- `removeAttr(key: string): void` — remove attribute
- `addChild(node: XmlNode): void` — append a child
- `getChildren(): ArrayList<XmlNode>` — all children
- `getChildrenByTag(tag: string): ArrayList<XmlNode>` — children matching tag
- `getElementByTagName(name: string): XmlNode` — first descendant matching tag
- `removeChild(node: XmlNode): void` — remove a child
- `replaceChild(oldNode: XmlNode, newNode: XmlNode): void` — replace a child
- `toString(): string` — serialize to XML
- `XmlNode.escapeText(s: string): string` — static: escape text for XML
- `XmlNode.escapeAttr(s: string): string` — static: escape attribute value for XML
