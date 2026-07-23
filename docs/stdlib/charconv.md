# `tt::charconv` — Character Conversion

Low‑level, zero‑allocation numeric↔string conversions (C++17 `<charconv>` parity).

## Constants

| Constant | Value |
|----------|-------|
| `SCIENTIFIC` | `1` |
| `FIXED` | `2` |
| `HEX` | `4` |
| `GENERAL` | `3` |

## to_chars

```titrate
let result: CharsResult = toChars(42);
let result: CharsResult = toCharsDouble(3.14159, SCIENTIFIC);
let result: CharsResult = toCharsDoublePrec(value, FIXED, 6);
```

## from_chars

```titrate
let result: CharsResult = fromCharsInt("42");
let result: CharsResult = fromCharsDouble("3.14");
let result: CharsResult = fromCharsIntBase("ff", 16);
```

## CharsResult

```titrate
public class CharsResult {
    public string ptr;          // pointer to end of parsed range
    public int ec;              // error code (0 = success)
    public long intValue;       // parsed integer (int overloads)
    public double doubleValue;  // parsed float (double overloads)
    public int consumed;        // number of characters consumed
}
```
