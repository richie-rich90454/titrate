# Fraction

The `tt.fractions` module provides the `Fraction` type for exact rational arithmetic, avoiding the rounding errors of floating-point arithmetic.

```titrate
import tt.fractions;
```

## Fraction

### Creating Fractions

- `new Fraction(numerator: long, denominator: long)` — create a fraction; the denominator must not be zero
- `Fraction.fromInt(n: long): Fraction` — create a fraction from an integer (denominator = 1)
- `Fraction.fromDouble(d: double): Fraction` — approximate a double as a fraction
- `Fraction.parse(s: string): Result<Fraction, string>` — parse a string like `"3/4"` or `"7"`

```titrate
let a = new Fraction(3, 4);       // 3/4
let b = Fraction.fromInt(5);       // 5/1
let c = Fraction.fromDouble(0.375); // 3/8
let d = Fraction.parse("7/12");    // Ok(7/12)
```

### Accessors

- `.numerator(): long` — the numerator
- `.denominator(): long` — the denominator (always positive after normalization)

```titrate
let f = new Fraction(6, 8);
io::println(f.numerator().toString());    // 3
io::println(f.denominator().toString());  // 4
```

Fractions are automatically reduced to lowest terms on creation:

```titrate
let f = new Fraction(6, 8);  // stored as 3/4
```

### Arithmetic

Fractions support operator overloading for exact arithmetic:

```titrate
let a = new Fraction(1, 3);
let b = new Fraction(1, 6);

let sum = a + b;     // 1/2
let diff = a - b;    // 1/6
let prod = a * b;    // 1/18
let quot = a / b;    // 2/1
let neg = -a;        // -1/3
```

### Comparison

- `.equals(other: Fraction): bool` — check equality
- `.compareTo(other: Fraction): int` — compare (negative if less, zero if equal, positive if greater)

Fractions also support comparison operators:

```titrate
let a = new Fraction(1, 3);
let b = new Fraction(1, 2);
let less = a < b;   // true
let eq = a == b;    // false
```

### Conversion

- `.toDouble(): double` — convert to a floating-point approximation
- `.toInt(): long` — truncate to an integer (rounds toward zero)
- `.toString(): string` — string representation (e.g. `"3/4"` or `"5"` for whole numbers)

```titrate
let f = new Fraction(3, 4);
io::println(f.toDouble().toString());  // 0.75
io::println(f.toString());              // "3/4"

let whole = new Fraction(5, 1);
io::println(whole.toString());          // "5"
```

### Properties

- `.isWhole(): bool` — true if the denominator is 1
- `.isZero(): bool` — true if the numerator is 0
- `.isPositive(): bool` — true if the value is greater than zero
- `.isNegative(): bool` — true if the value is less than zero
- `.sign(): int` — -1, 0, or 1
- `.abs(): Fraction` — absolute value
- `.reciprocal(): Fraction` — swap numerator and denominator
- `.inverse(): Fraction` — negate the fraction

```titrate
let f = new Fraction(3, 4);
io::println(f.reciprocal().toString());  // "4/3"
io::println(f.abs().toString());          // "3/4"
```

## Module Functions

### Utility

- `fractions::gcd(a: long, b: long): long` — greatest common divisor
- `fractions::lcm(a: long, b: long): long` — least common multiple

```titrate
let g = fractions::gcd(12, 8);   // 4
let l = fractions::lcm(4, 6);    // 12
```

### Approximation

- `fractions::approximate(d: double, maxDenominator: long): Fraction` — find the best fraction approximation with a bounded denominator

```titrate
let pi = fractions::approximate(3.14159265, 1000);
// 355/113 (a well-known approximation of pi)
```
