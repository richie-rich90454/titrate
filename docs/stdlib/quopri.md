# Quopri

The `tt.encoding.Quopri` module implements RFC 2045 quoted-printable encoding and decoding for carrying 8-bit data over 7-bit ASCII channels. It mirrors Python's `quopri` module, exposing `encode`/`decode` (streaming into a `StringBuilder`), `encodestring`/`decodestring` (returning the result string), and an optional `header` mode for RFC 2047 Q-encoding (space becomes underscore).

## Import

```titrate
import tt::encoding::Quopri;
```

## Functions

### encodestring

- `Quopri.encodestring(s: string, quotetabs: bool, header: bool): string` — encode `s` into quoted-printable form and return the result. `quotetabs` forces encoding of tab characters; `header` enables RFC 2047 Q-encoding (space -> underscore). Mirrors `quopri.encodestring(s, quotetabs=False, header=False)`.

```titrate
let encoded: string = Quopri.encodestring("Hello, world!", false, false);
// "Hello, world="
```

### decodestring

- `Quopri.decodestring(s: string, header: bool): string` — decode a quoted-printable string and return the original bytes (as a UTF-8 string). `header` enables RFC 2047 decoding (underscore -> space). Mirrors `quopri.decodestring(s, header=False)`.

```titrate
let decoded: string = Quopri.decodestring("Hello=2C world!", false);
// "Hello, world!"
```

### encode

- `Quopri.encode(input: string, output: StringBuilder, quotetabs: bool, header: bool): void` — read raw bytes from `input`, encode to quoted-printable, and append to `output`. Mirrors `quopri.encode(input, output, quotetabs=False, header=False)`.

### decode

- `Quopri.decode(input: string, output: StringBuilder, header: bool): void` — read quoted-printable bytes from `input` and append the decoded bytes to `output`. Mirrors `quopri.decode(input, output, header=False)`.

## Encoding Rules

- Printable ASCII `33..126` (except `=` which is `61`) stays literal
- `=` is always encoded as `=3D`
- Tab is literal unless `quotetabs` is set
- Space is literal except when immediately before a line ending (then it is encoded)
- All other bytes (control chars, 8-bit) are encoded as `=XX`
- Lines are wrapped with a soft line break `=\r\n` when they exceed 76 characters
- `\n` becomes `\r\n`; an existing `\r\n` is preserved
- In `header` mode, space becomes `_` (RFC 2047)

## Decoding Rules

- `=XX` is decoded to the byte `(hexHi << 4) | hexLo`
- `=\r\n` or `=\n` (soft line break) is consumed
- In `header` mode, `_` becomes space (RFC 2047)
- Malformed `=` sequences emit `=` literally

## Usage Example

```titrate
import tt::encoding::Quopri;
import tt::util::StringBuilder;

public fn main(): void {
    let original: string = "Café résumé — naïve";
    let encoded: string = Quopri.encodestring(original, false, false);
    io::println("Encoded: " + encoded);
    let decoded: string = Quopri.decodestring(encoded, false);
    io::println("Decoded: " + decoded);
    let sb: StringBuilder = new StringBuilder();
    Quopri.encode("plain text", sb, false, false);
    io::println("Streamed: " + sb.toString());
}
```
