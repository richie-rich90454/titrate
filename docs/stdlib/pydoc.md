# PyDoc

The `tt.docs.PyDoc` module mirrors Python's `pydoc` module. It generates human-readable and HTML documentation for Titrate modules, classes, functions, and keywords from their docstrings and structural metadata. The output mirrors the format of `python -m pydoc`.

## Import

```titrate
import tt::docs::PyDoc;
```

## Topic

`Topic` is a documentation topic — a named cross-reference page (like `EXPRESSIONS` or `FUNCTIONS` in CPython's pydoc).

- `Topic.init(name: string, title: string, body: string)`
- `name(): string`
- `title(): string`
- `body(): string`
- `toString(): string`

## DocEntry

`DocEntry` describes a single documented declaration (function, method, field, class).

- `DocEntry.init(name: string, kind: string, signature: string, docstring: string)`
- `name(): string`
- `kind(): string` — one of `"function"`, `"method"`, `"class"`, `"field"`, `"module"`
- `signature(): string` — formatted signature, e.g., `(a: int, b: int): int`
- `docstring(): string`
- `toString(): string`

## Doc

`Doc` is the rendered documentation for a single object.

- `Doc.init(name: string, kind: string, summary: string, body: string, entries: ArrayList<DocEntry>)`
- `name(): string`
- `kind(): string`
- `summary(): string` — first paragraph of the docstring
- `body(): string` — full docstring
- `entries(): ArrayList<DocEntry>` — documented members (for classes/modules)
- `toText(): string` — plain-text rendering
- `toHtml(): string` — HTML rendering (single page, inline CSS)
- `toString(): string` — alias for `toText`

## Functions

### doc

Look up the documentation for `name` (a fully-qualified name like `"tt.util.ArrayList"` or `"tt.util.ArrayList.add"`) and return a `Doc` object. Returns null if `name` is not found.

**Parameters:** `name: string`
**Returns:** `Variant` (a `Doc` or `null`)

```titrate
let d: Variant = doc("tt.util.ArrayList");
if (d != null) {
    io::println(d.toText());
}
```

### render_doc

Render the documentation for `name` in the given `format`. `format` is one of `"text"` (default) or `"html"`.

**Parameters:** `name: string`, `format: string`
**Returns:** `string`

### locate

Resolve a dotted name into a runtime value. Mirrors `pydoc.locate`. Returns `null` if the name cannot be resolved.

**Parameters:** `name: string`
**Returns:** `Variant`

### source

Return the source code of `obj` as a string, or the empty string if unavailable. Mirrors `pydoc.source`.

**Parameters:** `obj: Variant`
**Returns:** `string`

### topics

Return the list of built-in documentation topics (string constants like `"EXPRESSIONS"`, `"FUNCTIONS"`, `"CLASSES"`). Useful for building an index page.

**Returns:** `ArrayList<string>`

### topic

Return the `Topic` object for the named topic, or null.

**Parameters:** `name: string`
**Returns:** `Variant`

### help

Interactive help: if `name` is provided, print the documentation for `name`. Otherwise enter interactive mode reading queries from the input queue.

**Parameters:** `name: string` (optional)
**Returns:** `void`

```titrate
help("tt.util.ArrayList");
```

## Rendering helpers

### renderModule

Render the documentation for a module object as a `Doc`.

**Parameters:** `mod: Variant`
**Returns:** `Doc`

### renderClass

Render the documentation for a class.

**Parameters:** `cls: Variant`
**Returns:** `Doc`

### renderFunction

Render the documentation for a function.

**Parameters:** `fn: Variant`
**Returns:** `Doc`

## HTML helpers

### htmlPage

Wrap `body` in a minimal HTML 5 document with the given `title` and inline CSS suitable for standalone viewing.

**Parameters:** `title: string`, `body: string`
**Returns:** `string`

### htmlEscape

Escape `<`, `>`, `&`, `"`, `'` for HTML.

**Parameters:** `s: string`
**Returns:** `string`

## Notes

- Documentation is generated from each object's docstring and structural metadata (`inspect.getmembers`, `inspect.signature`). Source code is included only if `inspect.getsource` returns a non-empty string.
- The plain-text rendering mirrors CPython's `pydoc.plain` formatter; the HTML rendering mirrors `pydoc.HTMLDoc`.
- `help()` requires an interactive input queue populated via `io::pushLine` or similar; without input it prints the topic index and returns.
