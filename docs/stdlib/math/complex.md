---
title: math::complex
description: Complex number arithmetic for Titrate under the math namespace.
---

# complex

The `tt.math.complex` module provides the `Complex` type for complex number arithmetic, mirroring the standalone `tt.complex` module but located under the `math` namespace.

```titrate
import tt::math::complex::Complex;
```

## Complex

### Creating Complex Numbers

- `fn init(real: double, imag: double)`
- `Complex.fromPolar(r: double, theta: double): Complex`
- `Complex.i(): Complex`
- `Complex.zero(): Complex`
- `Complex.one(): Complex`

```titrate
let z1: Complex = new Complex(3.0, 4.0);     // 3 + 4i
let z2: Complex = Complex.fromPolar(1.0, Math.PI() / 4.0);
let z3: Complex = Complex.i();               // 0 + 1i
```

### Accessors and Properties

- `.real(): double` — real part
- `.imag(): double` — imaginary part
- `.abs(): double` — magnitude
- `.arg(): double` — phase angle in radians
- `.norm(): double` — squared magnitude
- `.conj(): Complex` — complex conjugate

```titrate
let z: Complex = new Complex(3.0, 4.0);
io::println("abs = " + Double.toString(z.abs()));      // 5.0
io::println("arg = " + Double.toString(z.arg()));      // atan2(4, 3)
```

### Arithmetic

Complex numbers support operator overloading and explicit methods:

- `add(other: Complex): Complex`
- `sub(other: Complex): Complex`
- `mul(other: Complex): Complex`
- `div(other: Complex): Complex`
- `negate(): Complex`

```titrate
let a: Complex = new Complex(1.0, 2.0);
let b: Complex = new Complex(3.0, 4.0);
let sum: Complex = a + b;   // (4 + 6i)
let prod: Complex = a * b;  // (-5 + 10i)
```

### Transcendental Functions

- `exp()`, `log()`, `log10()`, `log2()`
- `sqrt()`, `pow(n: double): Complex`
- `sin()`, `cos()`, `tan()`
- `asin()`, `acos()`, `atan()`
- `sinh()`, `cosh()`, `tanh()`
- `asinh()`, `acosh()`, `atanh()`

```titrate
let z: Complex = new Complex(0.0, Math.PI());
let result: Complex = z.exp();  // approximately -1 + 0i
io::println(result.toString());
```

### Comparison and State

- `equals(other: Complex): bool`
- `isClose(other: Complex, relTol: double, absTol: double): bool`
- `isFinite(): bool`
- `isInf(): bool`
- `isNaN(): bool`
- `toString(): string`

## Module-Level Helpers

- `fromPolar(r: double, theta: double): Complex`
- `polar(r: double, theta: double): Complex`
- `zero()`, `one()`, `i()`
- `complexInf()`, `complexNan()`, `complexZero()`, `complexOne()`, `complexI()`
