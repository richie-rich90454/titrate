# Numbers

The `tt.lang.Numbers` module provides the Python `numbers` ABC hierarchy as Titrate interfaces with default method bodies, plus concrete wrappers for native numeric primitives. The hierarchy is `Number` -> `ComplexNumber` -> `RealNumber` -> `RationalNumber` -> `IntegralNumber`. Helper functions provide `isinstance`-style checks against primitives (`int`/`long`/`double`/etc.) via `Variant` type tags, and factory wrappers adapt native values to the ABCs.

## Import

```titrate
import tt::lang::Numbers;
```

## Interfaces

### Number

Root numeric interface (Python: `numbers.Number`).

**Methods:**
- `toDouble(): double`
- `toLong(): long`
- `toString(): string`
- `isInteger(): bool` — default returns `false`

### ComplexNumber extends Number

Complex number interface (Python: `numbers.Complex`).

**Methods:**
- `real(): double`
- `imag(): double`
- `conjugate(): ComplexNumber`
- `toDouble(): double` — default returns `this.real()`
- `toLong(): long` — default returns `this.real() as long`
- `toString(): string` — default formats as `"(r+ij)"`

### RealNumber extends ComplexNumber

Real number interface (Python: `numbers.Real`).

**Methods:**
- `real(): double` — default returns `this.toDouble()`
- `imag(): double` — default returns `0.0`
- `conjugate(): ComplexNumber` — default returns `this`
- `floor(): long`
- `ceil(): long`
- `truncate(): long`
- `round(precision: int): RealNumber`
- `isInteger(): bool` — default returns `true` iff `toDouble()` is integral

### RationalNumber extends RealNumber

Rational number interface (Python: `numbers.Rational`).

**Methods:**
- `numerator(): long`
- `denominator(): long`
- `floor(): long` — default returns `numerator() / denominator()`
- `ceil(): long` — default rounds toward positive infinity
- `truncate(): long` — default rounds toward zero
- `round(precision: int): RealNumber` — default returns `this`
- `toDouble(): double` — default returns `numerator() / denominator()`
- `toLong(): long` — default returns `floor()`
- `toString(): string` — default returns `"n/d"` (or `"n"` when `d == 1`)

### IntegralNumber extends RationalNumber

Integral number interface (Python: `numbers.Integral`).

**Methods:**
- `numerator(): long` — default returns `toLong()`
- `denominator(): long` — default returns `1L`
- `floor(): long` / `ceil(): long` / `truncate(): long` — default returns `toLong()`
- `round(precision: int): RealNumber` — default returns `this`
- `isInteger(): bool` — default returns `true`
- `bitwiseAnd(other: IntegralNumber): IntegralNumber`
- `bitwiseOr(other: IntegralNumber): IntegralNumber`
- `bitwiseXor(other: IntegralNumber): IntegralNumber`
- `shiftLeft(n: int): IntegralNumber`
- `shiftRight(n: int): IntegralNumber`
- `bitwiseNot(): IntegralNumber`

## Classes

### IntWrapper

Adapts a Titrate `int` to the `IntegralNumber` ABC.

- `init(value: int)`
- `value: int`

### LongWrapper

Adapts a Titrate `long` to the `IntegralNumber` ABC.

- `init(value: long)`
- `value: long`

### DoubleWrapper

Adapts a Titrate `double` to the `RealNumber` ABC. `round(precision)` performs decimal-place rounding.

- `init(value: double)`
- `value: double`

### RationalImpl

Concrete `RationalNumber` implementation. The constructor normalizes the sign (denominator positive), reduces by GCD, and throws if `den == 0`. `round(0)` uses round-half-to-even (banker's rounding) like Python.

- `init(num: long, den: long)`
- `num: long` / `den: long` (post-normalization)

```titrate
let r: RationalImpl = Numbers.rational(6L, 4L);
io::println(r.toString());  // "3/2"
io::println(Double.toString(r.toDouble()));  // "1.5"
```

## Functions

### Factory wrappers

- `Numbers.wrapInt(value: int): IntWrapper`
- `Numbers.wrapLong(value: long): LongWrapper`
- `Numbers.wrapDouble(value: double): DoubleWrapper`
- `Numbers.rational(num: long, den: long): RationalImpl`

### isinstance-style checks

These helpers examine a `Variant`'s type tag to determine whether the contained value conforms to a particular numeric ABC.

- `Numbers.isNumber(v: Variant): bool` — true for any numeric primitive (`int`, `long`, `double`, `float`, `byte`, `short`, `vast`, `uvast`, `half`, `quad`, `u8`, `u16`, `u32`, `u64`)
- `Numbers.isIntegral(v: Variant): bool` — true for integer primitives
- `Numbers.isReal(v: Variant): bool` — alias for `isNumber`
- `Numbers.isRational(v: Variant): bool` — alias for `isIntegral`
- `Numbers.isComplex(v: Variant): bool` — alias for `isNumber`

### Numeric helpers

- `Numbers.toDouble(n: Number): double` — convert any `Number` to double
- `Numbers.toLong(n: Number): long` — convert any `Number` to long, truncating fractions

### Custom type registry

- `Numbers.registerNumber(name: string, factory: fn(Variant): Number): void` — register a custom `Number` type by name
- `Numbers.lookupNumber(name: string): bool` — true if a name is registered
- `Numbers.registeredNumberNames(): ArrayList<string>` — list registered names

## Usage Example

```titrate
import tt::lang::Numbers;

public fn main(): void {
    let i: IntegralNumber = Numbers.wrapInt(42);
    let d: RealNumber = Numbers.wrapDouble(3.14);
    let r: RationalNumber = Numbers.rational(22L, 7L);
    io::println(i.toString());  // "42"
    io::println(d.toString());  // "3.14"
    io::println(r.toString());  // "22/7"
    let sum: double = i.toDouble() + d.toDouble();
    io::println(Double.toString(sum));
}
```
