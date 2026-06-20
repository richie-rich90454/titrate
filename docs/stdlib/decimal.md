# decimal

The `tt.decimal` module provides arbitrary-precision decimal arithmetic using a string-based representation. It is suitable for financial, scientific, and other computations where binary floating-point rounding errors are unacceptable. The module is split across several files: `Decimal.tr` (core class and math functions), `DecimalArithmetic.tr` (low-level digit-string helpers), `DecimalContext.tr` (precision/rounding context management), and `DecimalExt.tr` (extended operations).

```titrate
import tt.decimal.Decimal;
import tt.decimal.DecimalContext;
import tt.decimal.DecimalExt;
```

## Decimal

An arbitrary-precision decimal number represented by a sign, a digit string, and a scale (number of digits after the decimal point).

**Fields:**

- `negative: bool` — true if the value is negative
- `digits: string` — the significant digits (no leading zeros, except `"0"` for zero)
- `scale: int` — number of digits after the decimal point

**Methods:**

- `fn init(value: string)` — parse a decimal from its string representation (e.g. `"-3.14"`)
- `fn add(other: Decimal): Decimal` — return the sum of this and `other`
- `fn sub(other: Decimal): Decimal` — return the difference (`this - other`)
- `fn mul(other: Decimal): Decimal` — return the product of this and `other`
- `fn div(other: Decimal): Decimal` — return the quotient (`this / other`); returns zero if `other` is zero
- `fn compareTo(other: Decimal): int` — return -1, 0, or 1
- `fn equals(other: Decimal): bool` — return true if values are equal
- `fn toString(): string` — convert to a plain decimal string
- `fn toDouble(): double` — convert to a 64-bit floating-point value
- `fn round(places: int, mode: string): Decimal` — round to the given number of decimal places using the named rounding mode
- `fn quantize(exp: Decimal): Decimal` — quantize to the same exponent as `exp` using `half_even` rounding
- `fn abs(): Decimal` — return the absolute value
- `fn negate(): Decimal` — return the negated value
- `fn stripTrailingZeros(): Decimal` — remove trailing zeros from the fractional part
- `fn signum(): int` — return -1, 0, or 1 depending on the sign
- `fn mod(other: Decimal): Decimal` — modulo (result has the same sign as this)
- `fn remainder(other: Decimal): Decimal` — remainder (result has the same sign as the divisor)

**Static methods:**

- `fn applyRounding(d: Decimal, mode: string): Decimal` — round a decimal to an integer using the named mode (`up`, `down`, `ceiling`, `floor`, `half_up`, `half_down`, `half_even`, `unnecessary`)
- `fn truncate(d: Decimal): Decimal` — truncate a decimal toward zero
- `fn pow10(n: int): Decimal` — compute `10^n` as a decimal

## RoundingMode

A holder for rounding-mode string constants loaded from `decimal/rounding_modes.json` via `DataFile`.

**Fields:**

- `UP: string`
- `DOWN: string`
- `CEILING: string`
- `FLOOR: string`
- `HALF_UP: string`
- `HALF_DOWN: string`
- `HALF_EVEN: string`
- `UNNECESSARY: string`

**Methods:**

- `fn init()` — load the rounding-mode constants from the data file

## DecimalContext

Holds a precision and rounding mode for decimal arithmetic. The canonical definition lives in `DecimalContext.tr`; a simpler variant with a `divide` method is also defined in `Decimal.tr`.

**Fields:**

- `precision: int` — number of significant digits
- `roundingMode: string` — name of the rounding mode
- `enabled: bool` — whether the context is active

**Methods:**

- `fn init(precision: int, roundingMode: string)` — create a context
- `fn withPrecision(p: int): DecimalContext` — return a new context with the given precision
- `fn withRoundingMode(mode: string): DecimalContext` — return a new context with the given rounding mode
- `fn getPrecision(): int` — return the precision
- `fn getRoundingMode(): string` — return the rounding mode
- `fn divide(a: Decimal, b: Decimal): Decimal` — divide two decimals using this context's precision and rounding mode (defined in `Decimal.tr`)

## DecimalContextManager

A stack of `DecimalContext` objects allowing nested precision/rounding scopes. The default context has precision 28 and `half_even` rounding.

**Fields:**

- `stack: ArrayList<DecimalContext>` — the context stack

**Methods:**

- `fn init()` — create a manager with a default context pushed
- `fn push(ctx: DecimalContext): void` — push a new context onto the stack
- `fn pop(): DecimalContext` — pop the top context (returns the default if only one remains)
- `fn current(): DecimalContext` — return the current (top) context

## DecimalArithmetic

Low-level digit-string arithmetic helpers used by `Decimal`. The public class is empty; the helper functions (`stripLeadingZeros`, `isZeroStr`, `compareDigitStrings`, `addDigitStrings`, `subDigitStrings`, `mulDigitStrings`, `divDigitStringByInt`, `alignScales`) are module-private and accessed internally by `Decimal`.

## Top-level Functions

Functions defined in `Decimal.tr`:

- `fn exp(x: Decimal): Decimal` — compute `e^x` using a Taylor series
- `fn ln(x: Decimal): Decimal` — compute the natural logarithm of `x`
- `fn log10(x: Decimal): Decimal` — compute the base-10 logarithm of `x`
- `fn fma(a: Decimal, b: Decimal, c: Decimal): Decimal` — fused multiply-add: computes `a*b + c`
- `fn logicalAnd(a: Decimal, b: Decimal): Decimal` — digit-wise logical AND (result digit is `min(da, db)`)
- `fn logicalOr(a: Decimal, b: Decimal): Decimal` — digit-wise logical OR (result digit is `max(da, db)`)
- `fn logicalXor(a: Decimal, b: Decimal): Decimal` — digit-wise logical XOR (result digit is `|da - db|`)
- `fn logicalInvert(a: Decimal): Decimal` — digit-wise complement (result digit is `9 - digit`)

Functions defined in `DecimalExt.tr`:

- `fn quantizeDouble(value: double, context: DecimalContext): double` — quantize a double using the given context
- `fn quantize(value: string, exponent: int, context: DecimalContext): string` — quantize a decimal string to the given exponent
- `fn rescale(value: string, newScale: int, context: DecimalContext): string` — rescale a decimal string to a new number of decimal places
- `fn toEngineeringString(value: string): string` — convert to engineering notation (exponent is a multiple of 3)
- `fn toPlainString(value: string): string` — convert to a plain string without scientific notation
- `fn logicalShift(value: string, shift: int): string` — shift the decimal point by `shift` places (positive right, negative left)
- `fn canonical(value: string): string` — return the canonical form (stripped trailing zeros, normalized sign)

## Constants

Rounding-mode string constants defined in `DecimalContext.tr`:

- `CEILING`, `FLOOR`, `UP`, `DOWN`, `HALF_UP`, `HALF_DOWN`, `HALF_EVEN`

Integer rounding-mode constants for use in switch statements:

- `ROUND_CEILING`, `ROUND_FLOOR`, `ROUND_UP`, `ROUND_DOWN`, `ROUND_HALF_UP`, `ROUND_HALF_DOWN`, `ROUND_HALF_EVEN`
