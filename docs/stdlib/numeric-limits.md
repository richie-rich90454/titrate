# numeric-limits

The `tt.lang` module provides `NumericLimits` — type limits and classification for numeric values.

```titrate
import tt.lang.NumericLimits;
```

## NumericLimits

A utility class that exposes platform numeric limits and classification predicates for `int` and `double`.

**Constants:**

- `NumericLimits.intMax(): int` — INT_MAX (2^31 - 1)
- `NumericLimits.intMin(): int` — INT_MIN (-2^31)
- `NumericLimits.longMax(): int` — LONG_MAX
- `NumericLimits.longMin(): int` — LONG_MIN
- `NumericLimits.doubleMax(): double` — DBL_MAX
- `NumericLimits.doubleMin(): double` — smallest positive normalized double
- `NumericLimits.doubleEpsilon(): double` — DBL_EPSILON
- `NumericLimits.doubleInfinity(): double` — positive infinity
- `NumericLimits.doubleNaN(): double` — NaN

**Classification:**

- `NumericLimits.isFinite(x: double): bool` — check if finite
- `NumericLimits.isInfinite(x: double): bool` — check if infinite
- `NumericLimits.isNaN(x: double): bool` — check if NaN
- `NumericLimits.isSubnormal(x: double): bool` — check if subnormal
- `NumericLimits.signbit(x: double): bool` — check sign bit

**Half-precision (16-bit float):**

- `NumericLimits.halfMax(): double` — 65504.0
- `NumericLimits.halfMin(): double` — smallest positive half
- `NumericLimits.halfEpsilon(): double` — half epsilon

**Quad-precision (128-bit float):**

- `NumericLimits.quadMax(): double` — ≈ 1.1897e+4932
- `NumericLimits.quadMin(): double` — smallest positive quad
- `NumericLimits.quadEpsilon(): double` — quad epsilon

**Unsigned integers:**

- `NumericLimits.u8Max(): int` — 255
- `NumericLimits.u16Max(): int` — 65535
- `NumericLimits.u32Max(): long` — 4294967295
- `NumericLimits.u64Max(): long` — 18446744073709551615

**Additional:**

- `NumericLimits.infinity(): double` — positive infinity
- `NumericLimits.quietNaN(): double` — quiet NaN
- `NumericLimits.denormMin(): double` — smallest positive denormalized double
- `NumericLimits.lowest(): double` — most negative finite value

```titrate
io::println(Integer.toString(NumericLimits.intMax()));      // 2147483647
io::println(Integer.toString(NumericLimits.intMin()));      // -2147483648
io::println(Double.toString(NumericLimits.doubleEpsilon())); // 2.220446049250313e-16

let inf = NumericLimits.doubleInfinity();
io::println(Boolean.toString(NumericLimits.isInfinite(inf))); // true
io::println(Boolean.toString(NumericLimits.isFinite(3.14)));  // true

let nan = NumericLimits.doubleNaN();
io::println(Boolean.toString(NumericLimits.isNaN(nan)));      // true
```
