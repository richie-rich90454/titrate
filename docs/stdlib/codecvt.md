# Codecvt

The `tt.encoding.Codecvt` module mirrors C++'s `<codecvt>` header (deprecated in C++17 but retained for parity). It provides conversion facets between UTF-8, UTF-16, and the platform's wide-character encoding, plus the `WStringConvert` and `WBufferConvert` adapters.

## Import

```titrate
import tt::encoding::Codecvt;
```

## CodecvtUtf8

Converts between the platform's wide-character type (`wchar_t`, modeled as `char` in Titrate) and UTF-8 byte sequences.

### Methods

- `CodecvtUtf8.init()` — default-construct the facet
- `out(state: int, from: char, fromEnd: char, fromNext: char, to: char, toEnd: char, toNext: char): int` — convert one wide character to UTF-8 bytes
- `in(state: int, from: string, fromEnd: int, fromNext: int, to: ArrayList<char>, toEnd: int, toNext: int): int` — convert UTF-8 bytes to wide characters
- `unshift(state: int, to: ArrayList<char>, toEnd: int, toNext: int): int` — emit any terminating sequence
- `encoding(): int` — returns `0` (variable width)
- `alwaysNoConv(): bool` — returns `false`
- `length(state: int, from: string, fromEnd: int, max: int): int` — return the number of wide characters that the byte sequence would produce
- `maxLength(): int` — maximum number of bytes per wide character (4)

### Static helpers

- `CodecvtUtf8.toBytes(wide: string): string` — convert a wide string to a UTF-8 byte string
- `CodecvtUtf8.fromString(bytes: string): string` — convert UTF-8 bytes to a wide string

```titrate
let utf8: string = CodecvtUtf8.toBytes("héllo");
```

## CodecvtUtf16

Converts between wide characters and UTF-16 byte sequences (little-endian or big-endian).

### Methods

- `CodecvtUtf16.init()` — default facet
- `CodecvtUtf16.init(endian: int)` — explicit endianness (`0` = little, `1` = big)
- `in(state: int, from: string, fromEnd: int, fromNext: int, to: ArrayList<char>, toEnd: int, toNext: int): int`
- `out(state: int, from: char, fromEnd: char, fromNext: char, to: char, toEnd: char, toNext: char): int`
- `encoding(): int` — returns `0`
- `maxLength(): int` — 4

### Static helpers

- `CodecvtUtf16.toBytes(wide: string, endian: int): string` — encode as UTF-16 bytes
- `CodecvtUtf16.fromString(bytes: string, endian: int): string` — decode UTF-16 bytes

## CodecvtUtf8Utf16

Converts directly between UTF-8 and UTF-16 without going through the wide-character type.

- `CodecvtUtf8Utf16.init()`
- `CodecvtUtf8Utf16.toUtf16(utf8: string): string` — encode UTF-8 bytes as UTF-16
- `CodecvtUtf8Utf16.fromUtf16(utf16: string): string` — decode UTF-16 to UTF-8

## WStringConvert

`WStringConvert<Codecvt>` wraps a codecvt facet and exposes the classic `from_bytes`/`to_bytes` API from `<codecvt>`.

- `WStringConvert.init(codecvt: CodecvtUtf8)` (or `CodecvtUtf16`, etc.)
- `fromBytes(s: string): string` — convert byte string to wide string
- `toBytes(wide: string): string` — convert wide string to byte string
- `fromBytes(ch: char): string` — single-character overload
- `converted(): int` — number of characters converted by the last call
- `state(): int` — current conversion state

```titrate
let conv: WStringConvert = new WStringConvert(new CodecvtUtf8());
let wide: string = conv.fromBytes("héllo");
let bytes: string = conv.toBytes(wide);
```

## WBufferConvert

`WBufferConvert<Codecvt>` adapts a `StreamBuf` so that reading/writing it transparently runs the codecvt conversion.

- `WBufferConvert.init(buf: StreamBuf, codecvt: CodecvtUtf8)`
- `rdbuf(): StreamBuf` — the wrapped buffer
- `rdbuf(buf: StreamBuf): StreamBuf` — replace the wrapped buffer
- `state(): int` — current conversion state
- `alwaysNoConv(): bool` — whether the facet is a no-op

## Factory functions

- `codecvtUtf8(): CodecvtUtf8` — construct a UTF-8 facet
- `codecvtUtf16(endian: int): CodecvtUtf16` — construct a UTF-16 facet
- `codecvtUtf8Utf16(): CodecvtUtf8Utf16` — construct a UTF-8↔UTF-16 facet
- `wstringConvert<C>(codecvt: C): WStringConvert` — construct a converter
- `wbufferConvert<C>(buf: StreamBuf, codecvt: C): WBufferConvert` — construct a buffer adapter

## Notes

- All conversions are stateless unless a multi-byte sequence is split across a buffer boundary; in that case the conversion state is preserved across calls.
- Endianness defaults to the platform's native order; pass `0` for little-endian or `1` for big-endian to force a specific order.
- The `<codecvt>` header is deprecated in C++17 in favour of `charconv`-based and `std::filesystem`-based conversions; Titrate retains the API for source compatibility.
