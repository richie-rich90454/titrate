# `tt::ios` — Stream State & Formatting

The `Ios` module provides C++ `<ios>`-style stream state management.

## Format Flags

| Constant | Value |
|----------|-------|
| `FMT_DEC` | `1` |
| `FMT_HEX` | `2` |
| `FMT_OCT` | `4` |
| `FMT_FIXED` | `8` |
| `FMT_SCIENTIFIC` | `16` |
| `FMT_BOOLALPHA` | `32` |
| `FMT_SHOWPOINT` | `128` |
| `FMT_UPPERCASE` | `2048` |

## Stream State

| Constant | Value |
|----------|-------|
| `GOOD_STATE` | `0` |
| `EOF_STATE` | `1` |
| `FAIL_STATE` | `2` |
| `BAD_STATE` | `4` |

## IosBase

```titrate
let ios: IosBase = new IosBase();
ios.setf(FMT_HEX);
let isHex: bool = ios.isHex();
ios.setPrecision(6);
ios.setWidth(10);
```
