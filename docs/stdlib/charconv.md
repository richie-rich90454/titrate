# CharConv

The `tt.lang.CharConv` module mirrors C++17's `<charconv>` header. It provides `toChars`/`fromChars` primitives for zero-allocation, locale-independent conversions between numeric types and character sequences, plus the `CharsFormat` flags (`SCIENTIFIC`, `FIXED`, `HEX`, `GENERAL`) and the `CharsResult` struct.

Unlike `Integer.toString` / `Integer.parseInt`, `CharConv` does not allocate; results are written into a caller-supplied buffer or read from a caller-supplied substring.

## Import

```titrate
import tt::lang::CharConv;
```

## Constants

`CharsFormat` bit flags:

- `SCIENTIFIC: int = 1` — use scientific notation (`1.23e+04`)
- `FIXED: int = 2` — use fixed notation (`12300`)
- `HEX: int = 4` — use hexadecimal floating-point (`0x1.4p-3`)
- `GENERAL: int = 3` — `SCIENTIFIC | FIXED`; pick whichever is shorter

## CharsResult

Returned by `toChars` functions:

- `ptr: string` — pointer to one past the last character written (or the full output if no buffer was supplied)
- `ec: int` — error code (0 = success, `EINVAL` = invalid argument, `ERANGE` = out of range)

```titrate
let r: CharsResult = toCharsDouble(3.14);
io::println(r.ptr);  // "3.14"
io::println(Integer.toString(r.ec));  // 0
```

## toChars family

### toChars

Convert an `int` to its shortest decimal representation.

**Parameters:** `value: int`
**Returns:** `CharsResult`

### toCharsBase

Convert an `int` to a string in the given `base` (2 to 36).

**Parameters:** `value: int`, `base: int`
**Returns:** `CharsResult`

### toCharsLong

Convert a `long` to its shortest decimal representation.

**Parameters:** `value: long`
**Returns:** `CharsResult`

```titrate
let r: CharsResult = toCharsLong(9223372036854775807L);
io::println(r.ptr);  // "9223372036854775807"
```

### toCharsDouble

Convert a `double` to its shortest representation that round-trips back to the same `double`.

**Parameters:** `value: double`
**Returns:** `CharsResult`

### toCharsDoubleFmt

Convert a `double` using the given `CharsFormat` flags and precision.

**Parameters:** `value: double`, `fmt: int`, `precision: int`
**Returns:** `CharsResult`

```titrate
let r: CharsResult = toCharsDoubleFmt(3.14159, FIXED, 2);
io::println(r.ptr);  // "3.14"
```

### toCharsDoublePrec

Convert a `double` with a specific precision (using `GENERAL` format).

**Parameters:** `value: double`, `precision: int`
**Returns:** `CharsResult`

## fromChars family

### fromCharsInt

Parse a decimal `int` from `s` starting at `0` and ending at `String.length(s)`.

**Parameters:** `s: string`
**Returns:** `CharsResult` (the `ptr` field is one past the last consumed character; on success the parsed value is accessible via the module's `lastParsedInt()`).

### fromCharsIntBase

Parse an `int` from `s` using the given `base`.

**Parameters:** `s: string`, `base: int`
**Returns:** `CharsResult`

### fromCharsDouble

Parse a `double` from `s` in `GENERAL` format.

**Parameters:** `s: string`
**Returns:** `CharsResult` (use `lastParsedDouble()` to read the result).

### fromCharsDoubleFmt

Parse a `double` from `s` using the given `CharsFormat` flags.

**Parameters:** `s: string`, `fmt: int`
**Returns:** `CharsResult`

## Result accessors

Because Titrate does not have out-parameters, `fromChars` results are stored in module-level slots:

- `lastParsedInt(): int` — the value parsed by the last `fromCharsInt*` call
- `lastParsedLong(): long` — the value parsed by `fromCharsLong*`
- `lastParsedDouble(): double` — the value parsed by `fromCharsDouble*`

## Round-trip helpers

### roundTripInt

Round-trip an `int` through `toChars` and `fromChars`; returns `true` if the round trip is exact.

**Parameters:** `value: int`
**Returns:** `bool`

### roundTripLong

Round-trip a `long`; returns `true` on exact round trip.

**Parameters:** `value: long`
**Returns:** `bool`

### roundTripDouble

Round-trip a `double`; returns `true` if the round trip is bit-exact.

**Parameters:** `value: double`
**Returns:** `bool`

```titrate
io::println(Boolean.toString(roundTripDouble(3.14159265358979)));  // true
io::println(Boolean.toString(roundTripInt(-42)));  // true
```

## Notes

- All conversions are locale-independent; the decimal separator is always `.`.
- `toChars` produces the shortest representation that round-trips; longer representations require explicit precision.
- `fromChars` skips leading whitespace; if no valid conversion is possible, `ec` is set to `EINVAL` and `ptr` is left unchanged.
