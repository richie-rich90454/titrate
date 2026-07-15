# MimeTypes

The `tt.net.MimeTypes` module guesses the MIME type of a file from its extension and vice versa. It mirrors Python's `mimetypes` module, loading default mappings from `net/mime_types.json` and supporting caller-supplied `.types` files. Both a module-level API (`guess_type`, `guess_extension`, `add_type`, `init`, `read_mime_types`) and an isolated `MimeTypes` class are provided.

## Import

```titrate
import tt::net::MimeTypes;
```

## Functions

### init

- `MimeTypes.init(files: ArrayList<string>): void` — reinitialize the module-level MIME database from the bundled data file plus any caller-supplied `files` (parsed via `read_mime_types`). Mirrors `mimetypes.init(files)`.

### read_mime_types

- `MimeTypes.read_mime_types(filename: string): void` — parse a file in the standard `mime.types` format (`"type/subtype ext1 ext2 ..."`, lines beginning with `#` ignored). Mirrors `mimetypes.read_mime_types(filename)`.

### guess_type

- `MimeTypes.guess_type(url: string, strict: bool): (string, string)` — return a `(type, encoding)` pair for the URL/path. `type` is empty when unknown; `encoding` is empty when not compressed. Mirrors `mimetypes.guess_type(url, strict=True)`.

```titrate
let result: (string, string) = MimeTypes.guess_type("photo.jpeg", true);
io::println(result.0);  // "image/jpeg"
```

### guess_extension

- `MimeTypes.guess_extension(mimeType: string, strict: bool): string` — return one extension (without leading dot) for the given MIME type, or `""` when unknown. Mirrors `mimetypes.guess_extension(type, strict=True)`.

### guess_all_extensions

- `MimeTypes.guess_all_extensions(mimeType: string, strict: bool): ArrayList<string>` — return all extensions (without leading dot) registered for the given MIME type. Mirrors `mimetypes.guess_all_extensions(type, strict=True)`.

### add_type

- `MimeTypes.add_type(mimeType: string, ext: string, strict: bool): void` — register a new extension mapping. `ext` may or may not begin with a leading dot. Mirrors `mimetypes.add_type(type, ext, strict=True)`.

```titrate
MimeTypes.add_type("application/x-myapp", ".myapp", true);
```

## Classes

### MimeTypes

An isolated MIME database instance for callers that want a private set of mappings (mirrors Python's `mimetypes.MimeTypes`).

**Fields:**
- `typesMap: HashMap<string, string>` — extension (lowercase, no dot) to MIME type
- `extsMap: HashMap<string, ArrayList<string>>` — MIME type (lowercase) to list of extensions
- `encsMap: HashMap<string, string>` — extension to encoding

**Constructors:**
- `init()` — load bundled defaults into the instance maps

**Methods:**
- `guess_type(url: string, strict: bool): (string, string)`
- `guess_extension(mimeType: string, strict: bool): string`
- `guess_all_extensions(mimeType: string, strict: bool): ArrayList<string>`
- `add_type(mimeType: string, ext: string, strict: bool): void`
- `read_mime_types(filename: string): void`

```titrate
let db: MimeTypes = new MimeTypes();
db.add_type("application/x-custom", ".custom", true);
let ext: string = db.guess_extension("application/x-custom", true);
```

## Usage Example

```titrate
import tt::net::MimeTypes;

public fn main(): void {
    let result: (string, string) = MimeTypes.guess_type("report.pdf", true);
    io::println("Type: " + result.0);       // "application/pdf"
    io::println("Encoding: " + result.1);    // ""
    let ext: string = MimeTypes.guess_extension("image/png", true);
    io::println("Extension: " + ext);        // "png"
}
```
