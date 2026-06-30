---
title: xml
description: XML parsing, building, and querying for Titrate.
---

# xml

The `tt.xml` module parses, builds, and queries XML documents. It supports DOM-style trees, namespace-aware parsing, element helpers, a fluent builder, and XPath expressions. For canonicalization, schema validation, and streaming parsing, see [xml-advanced](./xml-advanced).

```titrate
import tt::xml::Xml;
import tt::xml::XmlNode;
import tt::xml::XmlBuilder;
import tt::xml::XPath;
```

## Parsing

- `Xml.parse(input: string): XmlNode`
- `Xml.parseWithNamespaces(input: string): XmlNode`
- `Xml.parseElement(input: string): XmlElement`
- `Xml.canonicalize(node: XmlNode): string`
- `Xml.validate(node: XmlNode, schemaPath: string): ValidationResult`

```titrate
let xml: string = "<catalog><book id='1'><title>1984</title></book></catalog>";
let root: XmlNode = Xml.parse(xml);

let book: XmlNode = root.getElementByTagName("book");
io::println(book.getAttr("id"));   // "1"
io::println(book.getText());       // "1984"
```

## XmlNode

The DOM node type.

- `fn init(t: string)`
- `getTag(): string`
- `getText(): string`
- `getAttr(name: string): string`
- `setAttr(name: string, value: string): void`
- `addChild(node: XmlNode): void`
- `getChildren(): ArrayList<XmlNode>`
- `getChildrenByTag(tag: string): ArrayList<XmlNode>`
- `getChildrenByTagNS(nsURI: string, lname: string): ArrayList<XmlNode>`
- `getElementsByTagNameNS(nsURI: string, lname: string): ArrayList<XmlNode>`
- `removeChild(node: XmlNode): void`
- `replaceChild(oldNode: XmlNode, newNode: XmlNode): void`
- `textContent(): string`
- `cloneNode(deep: bool): XmlNode`
- `normalize(): void`
- `lookupNamespaceURI(prefix: string): string`
- `lookupPrefix(uri: string): string`
- `hasAttr(key: string): bool`
- `removeAttr(key: string): void`
- `toString(): string`

### Node Factory Functions

- `newTextNode(content: string): XmlNode`
- `newCDataNode(content: string): XmlNode`
- `newCommentNode(content: string): XmlNode`
- `newPINode(target: string, data: string): XmlNode`
- `newDeclarationNode(version: string, encoding: string): XmlNode`

## XmlElement

A higher-level wrapper around `XmlNode`.

- `fn init(node: XmlNode)`
- `find(tag: string): XmlElement`
- `findAll(tag: string): ArrayList<XmlElement>`
- `findNS(nsURI: string, localName: string): XmlElement`
- `findAllNS(nsURI: string, localName: string): ArrayList<XmlElement>`
- `getChildren(): ArrayList<XmlElement>`
- `getNode(): XmlNode`
- `get(name: string): string`
- `set(name: string, value: string): void`
- `hasAttrib(name: string): bool`
- `toString(): string`

```titrate
let rootElement: XmlElement = new XmlElement(root);
let firstBook: XmlElement = rootElement.find("book");
io::println(firstBook.get("id"));
```

## XmlBuilder

Fluent API for constructing XML documents.

- `fn init()`
- `declaration(version: string, encoding: string): XmlBuilder`
- `root(tag: string): XmlBuilder`
- `elem(tag: string): XmlBuilder`
- `attr(key: string, value: string): XmlBuilder`
- `text(content: string): XmlBuilder`
- `cdata(content: string): XmlBuilder`
- `comment(content: string): XmlBuilder`
- `pi(target: string, data: string): XmlBuilder`
- `ns(prefix: string, uri: string): XmlBuilder`
- `defaultNs(uri: string): XmlBuilder`
- `end(): XmlBuilder`
- `build(): XmlNode`
- `fromNode(node: XmlNode): XmlBuilder`
- `prettyPrint(indent: int): XmlBuilder`
- `minified(): XmlBuilder`
- `toString(): string`

```titrate
let doc: XmlNode = new XmlBuilder()
    .declaration("1.0", "UTF-8")
    .root("catalog")
        .elem("book").attr("id", "1")
            .elem("title").text("1984").end()
        .end()
        .elem("book").attr("id", "2")
            .elem("title").text("Animal Farm").end()
        .end()
    .build();

io::println(doc.toString());
```

## XPath

Query XML documents with XPath expressions.

- `XPath.compile(expr: string): XPathExpression`
- `XPath.selectNodes(node: XmlNode, expr: string): ArrayList<XmlNode>`
- `XPath.count(node: XmlNode, path: string): int`
- `XPath.stringValue(node: XmlNode): string`
- `XPath.numberValue(node: XmlNode): double`
- `XPath.name(node: XmlNode): string`
- `XPath.localName(node: XmlNode): string`
- `XPath.sum(node: XmlNode, path: string): double`
- `XPath.contains(s1: string, s2: string): bool`
- `XPath.startsWith(s1: string, s2: string): bool`
- `XPath.substring(s: string, start: double): string`
- `XPath.substringLen(s: string, start: double, length: double): string`
- `XPath.stringLength(s: string): int`
- `XPath.not(b: bool): bool`
- `XPath.position(): int`
- `XPath.last(): int`

### XPathExpression

- `fn init(expr: string)`
- `bindVariable(name: string, value: string): void`
- `getVariable(name: string): string`
- `hasVariable(name: string): bool`
- `clearVariables(): void`
- `evaluate(node: XmlNode): ArrayList<XmlNode>`
- `evaluateString(node: XmlNode): string`
- `evaluateNumber(node: XmlNode): double`
- `evaluateBoolean(node: XmlNode): bool`

```titrate
let titles: ArrayList<XmlNode> = XPath.selectNodes(root, "//book/title");
for (title in titles) {
    io::println(title.getText());
}

let count: int = XPath.count(root, "//book");
io::println("Books: " + Integer.toString(count));
```
