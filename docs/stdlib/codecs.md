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

## Additional codecs (Phase 1-2 parity)

The following codecs are registered alongside the built-in `utf-8`, `ascii`, and `latin-1`. They are usable through `Codecs.encode` / `Codecs.decode` by name.

### rot_13

The `rot_13` codec applies the ROT-13 substitution cipher to ASCII letters. It is its own inverse, so encoding and decoding are identical.

```titrate
let enc: string = Codecs.encode("hello", "rot13");  // "uryyb"
let dec: string = Codecs.decode("uryyb", "rot13");  // "hello"
```

### punycode

The `punycode` codec implements RFC 3492, used for internationalized domain names. It converts Unicode to an ASCII-safe form.

```titrate
let p: string = Codecs.encode("bücher", "punycode");  // "bcher-kva"
let back: string = Codecs.decode("bcher-kva", "punycode");  // "bücher"
```

### shift_jis and other CJK codecs

| Codec name | Description |
|------------|-------------|
| `shift_jis` | Shift-JIS (Japanese) |
| `euc_jp` | EUC-JP (Japanese) |
| `iso2022_jp` | ISO-2022-JP (Japanese) |
| `gb2312` | GB2312 (Simplified Chinese) |
| `gbk` | GBK (Simplified Chinese) |
| `gb18030` | GB18030 (Simplified Chinese) |
| `big5` | Big5 (Traditional Chinese) |
| `euc_kr` | EUC-KR (Korean) |
| `iso8859_1` … `iso8859_15` | ISO 8859 family |
| `koi8_r` | KOI8-R (Russian) |

```titrate
let jp: string = Codecs.encode("日本語", "shift_jis");
let restored: string = Codecs.decode(jp, "shift_jis");
```

### Codec aliases

`Codecs.lookup` resolves common aliases to their canonical name:

| Alias | Canonical |
|-------|-----------|
| `utf8` | `utf-8` |
| `ascii` | `ascii` |
| `latin1`, `iso8859_1`, `iso-8859-1` | `latin-1` |
| `shiftjis`, `shift-jis`, `sjis` | `shift_jis` |
| `eucjp` | `euc_jp` |
| `gb` | `gb2312` |
| `big5_tw` | `big5` |
