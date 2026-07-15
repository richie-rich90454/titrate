# Cgi

The `tt.net.Cgi` module provides Common Gateway Interface (CGI) utilities for parsing HTML form data. It mirrors Python's `cgi` module and includes helpers for working with both `application/x-www-form-urlencoded` and `multipart/form-data` payloads.

## Import

```titrate
import tt::net::Cgi;
```

## Classes

### Field

A single form field parsed from a query string or multipart body.

**Fields:**
- `name: string` — field name
- `value: string` — field value
- `filename: string` — original filename (for file uploads)
- `contentType: string` — MIME type (for file uploads)
- `headerSection: string` — raw header block (for multipart parts)
- `isFile: bool` — whether this field represents a file upload

**Constructors:**

- `Field(name: string, value: string)` — create a simple value field

**Example:**
```titrate
let f: Field = new Field("username", "alice");
io::println(f.toString());  // Field{name=username, value=alice}
```

### ParsedHeader

Result of `parseHeader`: the main value plus any trailing parameters.

**Fields:**
- `value: string` — main header value (e.g. `"text/html"`)
- `params: HashMap<string, string>` — trailing parameters (e.g. `{charset: utf-8}`)

**Constructor:**
- `ParsedHeader(value: string)` — create with empty params map

### FieldStorage

Parses form data from a query string and/or POST body. Supports both `application/x-www-form-urlencoded` and `multipart/form-data`.

**Fields:**
- `fields: HashMap<string, ArrayList<Field>>` — parsed fields keyed by name
- `queryString: string` — raw query string
- `body: string` — raw request body
- `contentType: string` — request content type
- `method: string` — HTTP method (defaults to `"GET"`)

**Methods:**

- `initFromRequest(method: string, queryString: string, body: string, contentType: string): void` — populate from request components and parse
- `parse(): void` — parse the query string and body into fields (idempotent)
- `parseUrlEncoded(data: string): void` — parse `application/x-www-form-urlencoded` data
- `extractBoundary(): string` — pull the `boundary` parameter from a multipart content type
- `parseMultipart(body: string, boundary: string): void` — parse a multipart body using the given boundary
- `parseMultipartPart(part: string): void` — parse a single multipart part into a `Field`
- `addField(name: string, value: string): void` — add a simple value field
- `addFieldObj(field: Field): void` — add an already-constructed `Field`
- `getValue(name: string): string` — first value for a name, or `""` if absent
- `getValues(name: string): ArrayList<string>` — all values for a name
- `keys(): ArrayList<string>` — all distinct field names
- `hasKey(name: string): bool` — check whether a field is present
- `getField(name: string): Field` — first `Field` object for a name (useful for file uploads)
- `getList(name: string): ArrayList<Field>` — all `Field` objects for a name
- `length(): int` — number of distinct field names

**Example:**
```titrate
let fs: FieldStorage = new FieldStorage();
fs.initFromRequest("POST", "", "name=alice&age=30", "application/x-www-form-urlencoded");
io::println(fs.getValue("name"));  // alice
io::println(fs.getValue("age"));   // 30
```

## Functions

### parse_qs

Parse a query string into a map of name → list of values. Mirrors Python's `urllib.parse.parse_qs`.

**Parameters:** `qs: string`, `keep_blank_values: bool`
**Returns:** `HashMap<string, ArrayList<string>>`

```titrate
let m = parse_qs("a=1&a=2&b=", false);
// m["a"] = ["1", "2"], "b" omitted because blank
```

### parse_qsl

Parse a query string into a list of `(name, value)` `Field` entries. Mirrors Python's `urllib.parse.parse_qsl`.

**Parameters:** `qs: string`, `keep_blank_values: bool`
**Returns:** `ArrayList<Field>`

```titrate
let pairs = parse_qsl("a=1&b=2", false);
// pairs[0].name == "a", pairs[0].value == "1"
```

### parseHeader

Parse a header line like `"text/html; charset=utf-8"` into a `ParsedHeader`.

**Parameters:** `line: string`
**Returns:** `ParsedHeader`

```titrate
let h = parseHeader("text/html; charset=utf-8");
io::println(h.value);                // text/html
io::println(h.params.get("charset")); // utf-8
```

### escape

HTML-escape a string, replacing `&`, `<`, `>` (and optionally `"` and `'`). Mirrors Python's `cgi.escape`.

**Parameters:** `s: string`, `quote: bool`
**Returns:** `string`

```titrate
let safe = escape("<a href=\"x\">", true);
// &lt;a href=&quot;x&quot;&gt;
```

### escapeDefault

HTML-escape with `quote` defaulted to `false`.

**Parameters:** `s: string`
**Returns:** `string`

### urlencode

URL-encode a map of name → list of values into a query string. Mirrors Python's `urllib.parse.urlencode`.

**Parameters:** `data: HashMap<string, ArrayList<string>>`
**Returns:** `string`

```titrate
let data = new HashMap<string, ArrayList<string>>();
let l = new ArrayList<string>();
l.add("alice");
data.put("name", l);
io::println(urlencode(data));  // name=alice
```

### urlDecode

URL-decode a form-encoded string: `+` becomes space, `%XX` becomes the byte.

**Parameters:** `s: string`
**Returns:** `string`

```titrate
io::println(urlDecode("name=alice+smith")); // name=alice smith
```

### urlEncode

URL-encode a string for use in a query parameter. Encodes everything except `A–Z a–z 0–9 - _ . ~`.

**Parameters:** `s: string`
**Returns:** `string`

```titrate
io::println(urlEncode("hello world!"));  // hello%20world%21
```
