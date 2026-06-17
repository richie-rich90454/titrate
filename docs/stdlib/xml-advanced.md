# xml-advanced

The `tt.xml` module provides advanced XML processing capabilities including namespace handling, SAX-style streaming parsing, XPath evaluation, fluent XML building, schema validation, and canonicalization.

```titrate
import tt.xml.XmlNamespace;
import tt.xml.XmlStreamingParser;
import tt.xml.XPath;
import tt.xml.XmlBuilder;
import tt.xml.XmlSchema;
import tt.xml.XmlCanonicalizer;
```

## XmlNamespace

Namespace support for XML documents. `NamespaceMap` provides prefix-to-URI mapping, and `QName` resolves qualified names with namespace awareness.

- `NamespaceMap.new(): NamespaceMap` — create an empty namespace map
- `NamespaceMap.declare(prefix: string, uri: string): void` — declare a prefix-to-URI mapping
- `NamespaceMap.resolve(prefix: string): string` — resolve a prefix to its URI; returns empty string if undeclared
- `NamespaceMap.resolveQName(qname: string): string` — resolve a qualified name (`prefix:localname`) to `{URI}localname` form
- `NamespaceMap.defaultNamespace(): string` — return the default namespace URI
- `NamespaceMap.setDefaultNamespace(uri: string): void` — set the default namespace URI
- `NamespaceMap.prefixes(): ArrayList<string>` — list all declared prefixes
- `NamespaceMap.containsPrefix(prefix: string): bool` — check if a prefix is declared
- `NamespaceMap.copy(): NamespaceMap` — create a shallow copy
- `NamespaceMap.merge(other: NamespaceMap): void` — merge another namespace map into this one (other overrides)
- `NamespaceMap.remove(prefix: string): bool` — remove a prefix declaration

```titrate
let ns = NamespaceMap.new();
ns.declare("xs", "http://www.w3.org/2001/XMLSchema");
ns.declare("soap", "http://schemas.xmlsoap.org/soap/envelope/");
ns.setDefaultNamespace("http://example.com/ns");

let uri = ns.resolve("xs");           // "http://www.w3.org/2001/XMLSchema"
let qname = ns.resolveQName("xs:element"); // "{http://www.w3.org/2001/XMLSchema}element"
let defaultNs = ns.defaultNamespace();     // "http://example.com/ns"
```

### Namespace-Aware Attribute Handling

Attributes in XML do not inherit the default namespace. Prefixed attributes must be resolved explicitly:

```titrate
let attrQname = ns.resolveQName("soap:encodingStyle");
// "{http://schemas.xmlsoap.org/soap/envelope/}encodingStyle"
```

### Namespace Inheritance and Scoping

Namespace declarations are inherited by child elements. When parsing, the `XmlStreamingParser` maintains a scope stack:

```titrate
// Outer scope declares "xs"
ns.declare("xs", "http://www.w3.org/2001/XMLSchema");

// Inner element can use "xs" without re-declaring
let resolved = ns.resolve("xs"); // still valid

// Shadowing: inner scope can re-declare the same prefix
ns.declare("xs", "http://other.example.com/schema");
let shadowed = ns.resolve("xs"); // "http://other.example.com/schema"
```

## XmlStreamingParser

A SAX-style streaming XML parser that emits events as the document is read. Memory-efficient for large files since it never builds a full DOM tree in memory.

- `XmlStreamingParser.new(): XmlStreamingParser` — create a new parser with default settings
- `XmlStreamingParser.onStartElement(handler: fn(string, NamespaceMap): void): XmlStreamingParser` — set handler for element start tags (receives qualified name and namespace map)
- `XmlStreamingParser.onEndElement(handler: fn(string): void): XmlStreamingParser` — set handler for element end tags
- `XmlStreamingParser.onCharacters(handler: fn(string): void): XmlStreamingParser` — set handler for text content
- `XmlStreamingParser.onComment(handler: fn(string): void): XmlStreamingParser` — set handler for comments
- `XmlStreamingParser.onProcessingInstruction(handler: fn(string, string): void): XmlStreamingParser` — set handler for PIs (target, data)
- `XmlStreamingParser.parse(xml: string): void` — parse a complete XML string
- `XmlStreamingParser.feed(chunk: string): void` — feed a chunk of input for incremental parsing
- `XmlStreamingParser.finish(): void` — signal end of input for chunked parsing
- `XmlStreamingParser.setEncoding(encoding: string): void` — set expected encoding (default: auto-detect)
- `XmlStreamingParser.resolveEntities(resolve: bool): void` — enable or disable entity reference resolution (default: `true`)
- `XmlStreamingParser.namespaceAware(aware: bool): void` — enable namespace-aware reporting (default: `true`)
- `XmlStreamingParser.setMaxDepth(depth: int): void` — set maximum nesting depth limit
- `XmlStreamingParser.recoverOnErrors(recover: bool): void` — enable error recovery mode (continues parsing after recoverable errors)
- `XmlStreamingParser.getLineNumber(): int` — current line number in the input
- `XmlStreamingParser.getColumnNumber(): int` — current column number in the input

```titrate
let parser = XmlStreamingParser.new();
parser.onStartElement(fn(name: string, ns: NamespaceMap): void {
    io::println("Start: " + name);
});
parser.onEndElement(fn(name: string): void {
    io::println("End: " + name);
});
parser.onCharacters(fn(text: string): void {
    if (String.length(String.trim(text)) > 0) {
        io::println("Text: " + text);
    }
});
parser.parse("<root><item>Hello</item></root>");
```

### Chunked Input Feeding

For very large files, feed the parser incrementally:

```titrate
let parser = XmlStreamingParser.new();
parser.onStartElement(fn(name: string, ns: NamespaceMap): void {
    io::println("Element: " + name);
});

// Feed chunks as they arrive (e.g., from a file stream)
parser.feed("<root><item>");
parser.feed("Hello");
parser.feed("</item></root>");
parser.finish();
```

### Error Recovery

When `recoverOnErrors` is enabled, the parser attempts to continue after encountering malformed XML:

```titrate
let parser = XmlStreamingParser.new();
parser.recoverOnErrors(true);
parser.parse("<root><unclosed>text</root>"); // recovers from unclosed tag
```

## XPath

XPath 1.0 expression evaluator for selecting nodes and computing values from XML documents.

- `XPath.evaluate(expression: string, xml: string): ArrayList<string>` — evaluate an XPath expression against an XML string, return matching values
- `XPath.evaluateOnNode(expression: string, node: string): ArrayList<string>` — evaluate relative to a specific element
- `XPath.evaluateBoolean(expression: string, xml: string): bool` — evaluate and return a boolean result
- `XPath.evaluateNumber(expression: string, xml: string): double` — evaluate and return a numeric result
- `XPath.evaluateString(expression: string, xml: string): string` — evaluate and return a string result
- `XPath.count(expression: string, xml: string): int` — count matching nodes
- `XPath.compile(expression: string): CompiledXPath` — pre-compile an expression for repeated evaluation
- `XPath.withNamespaces(ns: NamespaceMap): XPath` — create an evaluator with namespace context
- `XPath.bindVariable(name: string, value: string): void` — bind a variable for $varname references
- `XPath.clearVariables(): void` — clear all bound variables

```titrate
let xml = "<books><book title='A'/><book title='B'/><book title='C'/></books>";
let titles = XPath.evaluate("//book/@title", xml);
// titles = ["A", "B", "C"]

let count = XPath.count("//book", xml); // 3
let first = XPath.evaluateString("//book[1]/@title", xml); // "A"
```

### Axes

XPath supports the following axes:

| Axis | Abbreviation | Description |
|------|-------------|-------------|
| `child` | (default) | Direct children |
| `descendant` | `//` | All descendants |
| `attribute` | `@` | Attributes |
| `self` | `.` | The context node itself |
| `parent` | `..` | The parent node |
| `ancestor` | — | All ancestors |
| `following-sibling` | — | Subsequent siblings |
| `preceding-sibling` | — | Preceding siblings |

```titrate
let children = XPath.evaluate("/root/child::*", xml);
let descendants = XPath.evaluate("//item", xml);
let attrs = XPath.evaluate("//book/@title", xml);
let selfNode = XPath.evaluate("//book[1]/self::book", xml);
let parent = XPath.evaluate("//book/parent::books", xml);
let ancestors = XPath.evaluate("//book/ancestor::*", xml);
let following = XPath.evaluate("//book[1]/following-sibling::book", xml);
let preceding = XPath.evaluate("//book[3]/preceding-sibling::book", xml);
```

### Predicates

Filter nodes with predicates in square brackets:

```titrate
// Positional predicate
let first = XPath.evaluate("//book[1]/@title", xml);

// Conditional predicate
let filtered = XPath.evaluate("//book[@price > 20]/@title", xml);

// Compound predicate
let result = XPath.evaluate("//book[@lang='en' and @price < 30]", xml);
```

### Functions

XPath 1.0 built-in functions:

**Node set functions:**
- `count(node-set)` — number of nodes
- `position()` — context position
- `last()` — context size

**String functions:**
- `string(object)` — convert to string
- `concat(s1, s2, ...)` — concatenate strings
- `contains(s1, s2)` — substring test
- `starts-with(s1, s2)` — prefix test
- `substring(s, start, len?)` — extract substring
- `normalize-space(s?)` — strip extra whitespace
- `translate(s, from, to)` — character replacement
- `name(node?)` — qualified name
- `local-name(node?)` — local name without prefix
- `namespace-uri(node?)` — namespace URI

**Boolean functions:**
- `not(bool)` — logical negation
- `boolean(object)` — convert to boolean

**Number functions:**
- `number(object)` — convert to number
- `sum(node-set)` — sum of numeric values

```titrate
let total = XPath.evaluateNumber("sum(//book/@price)", xml);
let names = XPath.evaluate("concat(//book[1]/@title, ' by ', //book[1]/@author)", xml);
let hasCheap = XPath.evaluateBoolean("//book[@price < 10]", xml);
```

### Namespace-Aware Resolution

Use namespace prefixes in XPath expressions by providing a `NamespaceMap`:

```titrate
let ns = NamespaceMap.new();
ns.declare("soap", "http://schemas.xmlsoap.org/soap/envelope/");
ns.declare("m", "http://example.com/msg");

let xpath = XPath.withNamespaces(ns);
let body = xpath.evaluate("//soap:Body/m:payload", soapXml);
```

### Compiled Expressions

Pre-compile XPath expressions for repeated evaluation:

```titrate
let compiled = XPath.compile("//book[@price > $min]/@title");
let expensive = compiled.evaluateWith(xml, HashMap.new()); // with variable bindings
```

## XmlBuilder

A fluent API for constructing XML documents programmatically.

- `XmlBuilder.builder(): XmlBuilder` — create a new builder
- `XmlBuilder.declaration(): XmlBuilder` — add XML declaration (`<?xml version="1.0" encoding="UTF-8"?>`)
- `XmlBuilder.root(name: string): XmlBuilder` — create the root element
- `XmlBuilder.elem(name: string): XmlBuilder` — add a child element
- `XmlBuilder.attr(name: string, value: string): XmlBuilder` — add an attribute to the current element
- `XmlBuilder.text(content: string): XmlBuilder` — add text content to the current element
- `XmlBuilder.cdata(content: string): XmlBuilder` — add CDATA section
- `XmlBuilder.comment(content: string): XmlBuilder` — add a comment
- `XmlBuilder.pi(target: string, data: string): XmlBuilder` — add a processing instruction
- `XmlBuilder.ns(prefix: string, uri: string): XmlBuilder` — declare a namespace on the current element
- `XmlBuilder.defaultNs(uri: string): XmlBuilder` — declare default namespace on the current element
- `XmlBuilder.up(): XmlBuilder` — move up to the parent element
- `XmlBuilder.build(): string` — generate the XML string with pretty printing
- `XmlBuilder.buildMinified(): string` — generate minified XML (no whitespace)
- `XmlBuilder.buildFragment(): string` — generate XML fragment (no declaration)

```titrate
let xml = XmlBuilder.builder()
    .declaration()
    .root("catalog")
        .attr("version", "1.0")
        .ns("cat", "http://example.com/catalog")
        .elem("book")
            .attr("id", "1")
            .attr("lang", "en")
            .elem("title").text("The Guide").up()
            .elem("price").text("29.99").up()
        .up()
        .elem("book")
            .attr("id", "2")
            .elem("title").text("Dark Matter").up()
            .elem("price").text("19.99").up()
        .up()
    .build();
```

### Namespace Declarations with Automatic Prefix Management

The builder can automatically manage namespace prefixes:

```titrate
let xml = XmlBuilder.builder()
    .root("soap:Envelope")
        .ns("soap", "http://schemas.xmlsoap.org/soap/envelope/")
        .ns("m", "http://example.com/msg")
        .elem("soap:Body")
            .elem("m:GetPrice")
                .elem("m:Item").text("Widget").up()
            .up()
        .up()
    .build();
```

### Pretty Printing vs Minified Output

```titrate
let pretty = XmlBuilder.builder()
    .root("root")
        .elem("child").text("value").up()
    .build();
// <root>
//   <child>value</child>
// </root>

let mini = XmlBuilder.builder()
    .root("root")
        .elem("child").text("value").up()
    .buildMinified();
// <root><child>value</child></root>
```

### Fragment Building

Build an XML fragment without a declaration for embedding in other documents:

```titrate
let fragment = XmlBuilder.builder()
    .elem("item")
        .attr("name", "widget")
        .text("A useful widget")
    .buildFragment();
// <item name="widget">A useful widget</item>
```

## XmlSchema

XML Schema (XSD) validation for XML documents. Supports simple and complex type validation, facet checking, and schema import/include.

- `XmlSchema.new(): XmlSchema` — create a new schema validator
- `XmlSchema.loadSchema(xsd: string): void` — load an XSD schema from a string
- `XmlSchema.loadSchemaFromPath(path: string): void` — load an XSD schema from a file path
- `XmlSchema.importSchema(namespace: string, xsd: string): void` — import a schema by namespace
- `XmlSchema.includeSchema(xsd: string): void` — include a schema (same namespace)
- `XmlSchema.validate(xml: string): bool` — validate an XML document against loaded schemas
- `XmlSchema.validateWithErrors(xml: string): ValidationReport` — validate and return detailed errors
- `XmlSchema.validateElement(elementName: string, xml: string): bool` — validate a specific element declaration

### ValidationReport

- `ValidationReport.isValid(): bool` — whether validation passed
- `ValidationReport.errors(): ArrayList<ValidationError>` — list of validation errors
- `ValidationError.path(): string` — XPath to the invalid node
- `ValidationError.message(): string` — human-readable error description
- `ValidationError.schemaLocation(): string` — location in the schema that triggered the error
- `ValidationError.severity(): string` — error severity (`"error"` or `"warning"`)

```titrate
let schema = XmlSchema.new();
schema.loadSchemaFromPath("schemas/order.xsd");

let report = schema.validateWithErrors(orderXml);
if (!report.isValid()) {
    let errors = report.errors();
    for (err in errors) {
        io::println(err.path() + ": " + err.message());
    }
}
```

### Simple Type Validation

Validate against built-in and custom simple types with facets:

```titrate
// Schema with facets: minLength, maxLength, pattern, minInclusive, maxInclusive, etc.
let schema = XmlSchema.new();
schema.loadSchema("""
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:simpleType name="AgeType">
    <xs:restriction base="xs:integer">
      <xs:minInclusive value="0"/>
      <xs:maxInclusive value="150"/>
    </xs:restriction>
  </xs:simpleType>
</xs:schema>
""");
```

### Complex Type Validation

Validate elements with complex content models:

```titrate
// Schema with sequence, choice, all compositor, attribute declarations
let schema = XmlSchema.new();
schema.loadSchema("""
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:complexType name="AddressType">
    <xs:sequence>
      <xs:element name="street" type="xs:string"/>
      <xs:element name="city" type="xs:string"/>
      <xs:element name="zip" type="xs:string"/>
    </xs:sequence>
    <xs:attribute name="country" type="xs:string" use="required"/>
  </xs:complexType>
</xs:schema>
""");
```

## XmlCanonicalizer

XML Canonicalization per W3C specifications. Produces a deterministic byte-level representation of an XML document, used in XML digital signatures.

- `XmlCanonicalizer.canonicalize(xml: string): string` — Canonical XML 1.0 (inclusive)
- `XmlCanonicalizer.exclusiveCanonicalize(xml: string): string` — Exclusive XML Canonicalization 1.0
- `XmlCanonicalizer.exclusiveCanonicalizeWithPrefixList(xml: string, prefixes: ArrayList<string>): string` — exclusive canonicalization with inclusive namespace prefix list
- `XmlCanonicalizer.canonicalizeSubset(xml: string, xpath: string): string` — canonicalize a subset selected by XPath
- `XmlCanonicalizer.sortAttributes(xml: string): string` — sort attributes in document order per canonicalization rules
- `XmlCanonicalizer.normalizeWhitespace(xml: string): string` — normalize whitespace per canonicalization rules

```titrate
let xml = "<root xmlns:a='http://a' a:attr='2' attr='1'><child/></root>";

// Canonical XML 1.0 (inclusive) — includes all in-scope namespace declarations
let canon = XmlCanonicalizer.canonicalize(xml);

// Exclusive — only includes visibly utilized namespaces
let exclusive = XmlCanonicalizer.exclusiveCanonicalize(xml);

// Exclusive with inclusive prefix list
let prefixes = ArrayList.new();
prefixes.add("a");
let withPrefixes = XmlCanonicalizer.exclusiveCanonicalizeWithPrefixList(xml, prefixes);
```

### Attribute Sorting

Canonicalization sorts attributes by namespace URI then local name:

```titrate
let sorted = XmlCanonicalizer.sortAttributes(
    "<root b:attr='2' a:attr='1' attr='0' xmlns:a='http://a' xmlns:b='http://b'/>"
);
// Attributes sorted: attr, a:attr, b:attr
```

### Whitespace Normalization

Canonicalization normalizes whitespace in attribute values and strips insignificant whitespace in text nodes:

```titrate
let normalized = XmlCanonicalizer.normalizeWhitespace(
    "<root attr='  multiple   spaces  '>  text  </root>"
);
// Attribute values: collapsed spaces; text: preserved with line-end normalization
```
