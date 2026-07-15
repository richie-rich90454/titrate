# Ratio

The `tt::math::Ratio` module provides a runtime analog of C++ `<ratio>`. It represents a compile-time `std::ratio<N, D>` at runtime as a reduced `(num, den)` pair with a positive denominator. It provides arithmetic (`add`, `subtract`, `multiply`, `divide`), comparison (`equal`, `less`), reduction to lowest terms, and SI unit ratio aliases (`kilo`, `mega`, `giga`, `milli`, `micro`, `nano`).

## Import

```titrate
import tt::math::Ratio;
```

## API Reference

### `Ratio`

A rational number stored as a reduced `(numerator, denominator)` pair with a positive denominator.

**Fields:**
- `num: long` — the numerator
- `den: long` — the denominator (always positive)

**Constructors:**
- `init(num: long, den: long)` — construct from `long` numerator/denominator; normalizes the sign and reduces to lowest terms. Throws if `den == 0`.
- `init(num: int, den: int)` — construct from `int` numerator/denominator; same normalization.

**Methods:**
- `numerator(): long` — returns the numerator
- `denominator(): long` — returns the denominator (always positive)
- `toDouble(): double` — returns the ratio as a `double`
- `toString(): string` — returns `"num/den"`
- `reduce(): Ratio` — returns a new `Ratio` that is the reduced form (already reduced in `init`; returns a copy)
- `add(other: Ratio): Ratio` — returns `this + other`
- `subtract(other: Ratio): Ratio` — returns `this - other`
- `multiply(other: Ratio): Ratio` — returns `this * other`
- `divide(other: Ratio): Ratio` — returns `this / other`. Throws on division by zero.
- `equal(other: Ratio): bool` — returns true if this ratio equals `other`
- `less(other: Ratio): bool` — returns true if this ratio is less than `other`
- `lessEqual(other: Ratio): bool` — returns true if this ratio is less than or equal to `other`
- `greater(other: Ratio): bool` — returns true if this ratio is greater than `other`
- `greaterEqual(other: Ratio): bool` — returns true if this ratio is greater than or equal to `other`
- `reciprocal(): Ratio` — returns the reciprocal of this ratio. Throws if numerator is zero.
- `negate(): Ratio` — returns the negation of this ratio
- `abs(): Ratio` — returns the absolute value of this ratio
- `pow(n: int): Ratio` — returns this ratio raised to the integer power `n`
- `compareTo(other: Ratio): int` — returns `-1`, `0`, or `1`

### Free Functions

#### Construction

- `of(num: int, den: int): Ratio` — create a `Ratio` from `int` numerator/denominator
- `ofLong(num: long, den: long): Ratio` — create a `Ratio` from `long` numerator/denominator
- `ofInt(num: int): Ratio` — create a `Ratio` from an integer (denominator = 1)
- `ofDouble(x: double): Ratio` — approximate a `double` as a fraction using the continued-fraction method with bounded denominator (≤ 1,000,000)

#### Arithmetic (top-level)

- `add(a: Ratio, b: Ratio): Ratio`
- `subtract(a: Ratio, b: Ratio): Ratio`
- `multiply(a: Ratio, b: Ratio): Ratio`
- `divide(a: Ratio, b: Ratio): Ratio`
- `equal(a: Ratio, b: Ratio): bool`
- `less(a: Ratio, b: Ratio): bool`
- `reduce(r: Ratio): Ratio`

#### SI Unit Ratio Aliases

Each factory returns a fresh `Ratio` instance representing the corresponding `std::ratio` alias.

- `kilo(): Ratio` — `1000/1`
- `mega(): Ratio` — `1000000/1`
- `giga(): Ratio` — `1000000000/1`
- `tera(): Ratio` — `1000000000000/1`
- `milli(): Ratio` — `1/1000`
- `micro(): Ratio` — `1/1000000`
- `nano(): Ratio` — `1/1000000000`
- `pico(): Ratio` — `1/1000000000000`
- `centi(): Ratio` — `1/100`
- `deci(): Ratio` — `1/10`
- `deca(): Ratio` — `10/1`
- `hecto(): Ratio` — `100/1`

#### Common Constant Ratios

- `zero(): Ratio` — `0/1`
- `one(): Ratio` — `1/1`
- `half(): Ratio` — `1/2`
- `third(): Ratio` — `1/3`
- `quarter(): Ratio` — `1/4`
- `piApprox(): Ratio` — `22/7` (common approximation of π)

#### Helpers

- `equals(a: Ratio, b: Ratio): bool` — true if two ratios represent the same value
- `hash(r: Ratio): int` — a hash-style integer for a `Ratio` (useful for grouping)
- `parse(s: string): Ratio` — parse a `"num/den"` string. Throws on malformed input.

## Usage Examples

### Basic Ratio Arithmetic

```titrate
import tt::math::Ratio;
import tt::io::IO;

public fn main(): void {
    let a: Ratio = new Ratio(1, 2);   // 1/2
    let b: Ratio = new Ratio(1, 3);   // 1/3

    let sum: Ratio = a.add(b);        // 1/2 + 1/3 = 5/6
    IO.println(sum.toString());       // 5/6

    let product: Ratio = a.multiply(b);  // 1/2 * 1/3 = 1/6
    IO.println(product.toString());      // 1/6

    let quotient: Ratio = a.divide(b);   // (1/2) / (1/3) = 3/2
    IO.println(quotient.toString());     // 3/2
}
```

### SI Unit Conversion

```titrate
import tt::math::Ratio;

let km: Ratio = Ratio.kilo();     // 1000/1
let mm: Ratio = Ratio.milli();    // 1/1000

// 5 km in meters: 5 * 1000 = 5000
let meters: Ratio = Ratio.ofInt(5).multiply(km);
io::println(meters.toDouble());   // 5000.0
```

### Approximating a Double as a Fraction

```titrate
import tt::math::Ratio;

let r: Ratio = Ratio.ofDouble(3.14159);
io::println(r.toString());   // e.g. 355/113
io::println(r.toDouble());   // ~3.14159292...
```

### Comparison and Powers

```titrate
import tt::math::Ratio;

let half: Ratio = Ratio.half();      // 1/2
let quarter: Ratio = Ratio.quarter(); // 1/4

io::println(half.less(quarter));      // false (1/2 > 1/4)
io::println(quarter.less(half));      // true

let squared: Ratio = half.pow(2);     // (1/2)^2 = 1/4
io::println(squared.toString());      // 1/4

let reciprocal: Ratio = half.reciprocal();  // 2/1
io::println(reciprocal.toString());   // 2/1
```

### Parsing a Ratio from a String

```titrate
import tt::math::Ratio;

let r: Ratio = Ratio.parse("3/4");
io::println(r.toDouble());   // 0.75
io::println(r.numerator());  // 3
io::println(r.denominator()); // 4
```
