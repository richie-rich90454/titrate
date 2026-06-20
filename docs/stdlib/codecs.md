# codecs

The `tt.encoding.Codecs` module provides text encoding and decoding. It supports the built-in encodings `utf-8`, `ascii`, and `latin-1`, plus custom encodings registered at runtime. Encoding aliases (e.g. `utf8` → `utf-8`) are loaded from `encoding/codecs.json` via `DataFile`.

```titrate
import tt.encoding.Codecs;
```

## Top-level Functions

- `fn encode(s: string, encoding: string): string` — encode a string using the named encoding; non-encodable characters are replaced with `?`
- `fn decode(s: string, encoding: string): string` — decode a string using the named encoding
- `fn register(encoding: string, encoder: fn(string): string, decoder: fn(string): string): void` — register a custom encoding with its encoder and decoder functions
- `fn lookup(encoding: string): string` — return the canonical encoding name, or an empty string if the encoding is unknown

```titrate
import tt.encoding.Codecs;

let ascii = Codecs.encode("hello", "ascii");
let utf8 = Codecs.decode(ascii, "utf-8");

Codecs.register("rot13",
    fn(s: string): string => s,
    fn(s: string): string => s
);
io::println(Codecs.lookup("utf8")); // "utf-8"
```
