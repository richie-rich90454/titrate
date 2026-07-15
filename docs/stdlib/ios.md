# Ios

The `tt.io.Ios` module mirrors C++'s `<ios>` header. It provides `IosBase` (the base class for all stream classes), the `fmtflags`/`iostate`/`openmode`/`seekdir` flag enumerations, and `BasicIos` which integrates a stream with its underlying `StreamBuf`.

## Import

```titrate
import tt::io::Ios;
```

## IosBase

`IosBase` is the base class for all stream classes. It stores formatting flags, precision, field width, and the locale used for I/O.

### Flag constants

**fmtflags:**
- `fmtFlagsDec(): int` — decimal base (default)
- `fmtFlagsHex(): int` — hexadecimal base
- `fmtFlagsOct(): int` — octal base
- `fmtFlagsScientific(): int` — scientific float notation
- `fmtFlagsFixed(): int` — fixed-point notation
- `fmtFlagsBoolalpha(): int` — print booleans as words
- `fmtFlagsShowbase(): int` — show numeric base prefix
- `fmtFlagsShowpoint(): int` — always show decimal point
- `fmtFlagsShowpos(): int` — prefix `+` to non-negative numbers
- `fmtFlagsSkipws(): int` — skip whitespace on input
- `fmtFlagsUnitbuf(): int` — flush after each output
- `fmtFlagsUppercase(): int` — uppercase hex digits
- `fmtFlagsLeft(): int` — left-align
- `fmtFlagsRight(): int` — right-align
- `fmtFlagsInternal(): int` — pad between sign and value
- `adjustField(): int` — mask for `Left | Right | Internal`
- `baseField(): int` — mask for `Dec | Hex | Oct`
- `floatField(): int` — mask for `Scientific | Fixed`

**iostate:**
- `goodBit(): int` — no error (0)
- `badBit(): int` — irrecoverable stream error
- `failBit(): int` — format/parse failure
- `eofBit(): int` — end of input reached

**openmode:**
- `app(): int` — seek to end before each write
- `ate(): int` — open and seek to end
- `binary(): int` — binary mode
- `in(): int` — open for reading
- `out(): int` — open for writing
- `trunc(): int` — truncate existing file

**seekdir:**
- `beg(): int` — beginning of stream
- `cur(): int` — current position
- `end(): int` — end of stream

### Methods

- `flags(): int` — current format flags
- `setFlags(mask: int): int` — set the flags in `mask`, returning the previous value
- `unsetFlags(mask: int): int` — clear the flags in `mask`
- `precision(): int` — current floating-point precision
- `setPrecision(p: int): int` — set precision, returning the previous value
- `width(): int` — current field width
- `setWidth(w: int): int` — set field width for the next I/O
- `getLoc(): Locale` — current locale
- `imbue(loc: Locale): Locale` — set the locale, returning the previous one
- `xalloc(): int` — allocate a storage slot for an `iword`/`pword` (returns the index)
- `iword(index: int): long` — long storage at `index`
- `pword(index: int): Variant` — pointer storage at `index`
- `registerCallback(cb: fn(int, int): void, event: int): void` — register a callback for the given event

### Static class members

- `IosBase.init(): void` — initialize the library (called once at startup)
- `IosBase.syncWithStdio(sync: bool): bool` — toggle synchronization (always returns `true`)

## BasicIos

`BasicIos<CharT, Traits>` is the templated base class for `BasicIstream` and `BasicOstream`. It owns a `StreamBuf` pointer and tracks the stream's error state.

### State queries

- `good(): bool` — true if no error bits are set
- `eof(): bool` — true if `eofBit` is set
- `fail(): bool` — true if `failBit` or `badBit` is set
- `bad(): bool` — true if `badBit` is set
- `operatorBool(): bool` — implicit bool conversion (`!fail()`)
- `operatorNot(): bool` — `fail()`

### State manipulation

- `rdState(): int` — the raw `iostate` value
- `clear(state: int): void` — set the state (default `goodBit`)
- `setstate(state: int): void` — OR the state bits into the current state

### StreamBuf integration

- `rdbuf(): StreamBuf` — return the bound stream buffer
- `rdbuf(sb: StreamBuf): StreamBuf` — replace the bound buffer, returning the old one
- `tie(): BasicOstream` — the tied output stream flushed before input
- `tie(t: BasicOstream): BasicOstream` — set the tied stream

### Narrowing/widening

- `narrow(c: char, default: char): char` — convert a character to `char`
- `widen(c: char): char` — convert a `char` to the stream's character type

```titrate
let s: BasicIos = new BasicIos();
s.clear(s.goodBit());
io::println(Boolean.toString(s.good()));
```

## Ios typedefs

The module exposes the convenience typedefs that C++ users expect:

- `Ios = BasicIos<char, CharTraits<char>>`
- `Wios = BasicIos<wchar_t, CharTraits<wchar_t>>`
