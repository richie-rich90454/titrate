# Complex

The `tt.complex` module provides the `Complex` type for complex number arithmetic, along with utility functions for complex-valued mathematics.

```titrate
import tt.complex;
```

## Complex

### Creating Complex Numbers

- `fn init(real: double, imag: double)` — create a complex number from real and imaginary parts
- `Complex.fromReal(r: double): Complex` — create a complex number with zero imaginary part
- `Complex.fromPolar(r: double, theta: double): Complex` — create from polar coordinates (magnitude and angle in radians)

```titrate
let z1 = new Complex(3.0, 4.0);       // 3 + 4i
let z2 = Complex.fromReal(5.0);         // 5 + 0i
let z3 = Complex.fromPolar(1.0, Math.PI / 4.0);  // e^(i*pi/4)
```

### Accessors

- `.real(): double` — the real part
- `.imag(): double` — the imaginary part

```titrate
let z = new Complex(3.0, 4.0);
io::println(Double.toString(z.real()));  // 3.0
io::println(Double.toString(z.imag()));  // 4.0
```

### Properties

- `.abs(): double` — magnitude (absolute value)
- `.arg(): double` — argument (phase angle in radians)
- `.norm(): double` — squared magnitude (real² + imag²)
- `.conjugate(): Complex` — complex conjugate (real - imag*i)

```titrate
let z = new Complex(3.0, 4.0);
io::println(Double.toString(z.abs()));          // 5.0
io::println(Double.toString(z.arg()));          // 0.9272... (atan2(4, 3))
io::println(Double.toString(z.norm()));         // 25.0
let c = z.conjugate();                    // 3 - 4i
```

### Arithmetic

Complex numbers support operator overloading for arithmetic:

```titrate
let a = new Complex(1.0, 2.0);
let b = new Complex(3.0, 4.0);

let sum = a + b;          // (4 + 6i)
let diff = a - b;         // (-2 - 2i)
let prod = a * b;         // (-5 + 10i)
let quot = a / b;         // (0.44 + 0.08i)
let neg = -a;             // (-1 - 2i)
```

### Comparison

- `.equals(other: Complex): bool` — check equality (both real and imaginary parts match)

### Conversion

- `.toString(): string` — string representation (e.g. `"3.0 + 4.0i"` or `"3.0 - 4.0i"`)

```titrate
let z = new Complex(3.0, -4.0);
io::println(z.toString());  // "3.0 - 4.0i"
```

## Module Functions

### Complex-valued Math

- `complex.exp(z: Complex): Complex` — e^z
- `complex.log(z: Complex): Complex` — natural logarithm
- `complex.log10(z: Complex): Complex` — base-10 logarithm
- `complex.sqrt(z: Complex): Complex` — principal square root
- `complex.pow(base: Complex, exp: Complex): Complex` — complex exponentiation

```titrate
let z = new Complex(0.0, Math.PI);  // i*pi
let result = complex.exp(z);         // e^(i*pi) ≈ -1 + 0i
```

### Trigonometric Functions

- `complex.sin(z: Complex): Complex` — complex sine
- `complex.cos(z: Complex): Complex` — complex cosine
- `complex.tan(z: Complex): Complex` — complex tangent
- `complex.asin(z: Complex): Complex` — complex arcsine
- `complex.acos(z: Complex): Complex` — complex arccosine
- `complex.atan(z: Complex): Complex` — complex arctangent

### Hyperbolic Functions

- `complex.sinh(z: Complex): Complex` — complex hyperbolic sine
- `complex.cosh(z: Complex): Complex` — complex hyperbolic cosine
- `complex.tanh(z: Complex): Complex` — complex hyperbolic tangent

## Polar Form

- `Complex.fromPolar(r: double, theta: double): Complex` — create from polar coordinates
- `Complex.abs(c: Complex): double` — magnitude
- `Complex.arg(c: Complex): double` — argument (phase angle)
- `Complex.conjugate(c: Complex): Complex` — complex conjugate

## Roots of Unity

- `Complex.rootsOfUnity(n: int): ArrayList<Complex>` — nth roots of unity
- `Complex.nthRoot(c: Complex, n: int): ArrayList<Complex>` — nth roots of complex number

## Deepened Operations

- `Complex.exp(c: Complex): Complex` — e^z
- `Complex.ln(c: Complex): Complex` — natural logarithm
- `Complex.pow(base: Complex, exp: Complex): Complex` — complex power
- `Complex.sqrt(c: Complex): Complex` — complex square root
- `Complex.sin(c: Complex): Complex` — complex sine
- `Complex.cos(c: Complex): Complex` — complex cosine
- `Complex.tan(c: Complex): Complex` — complex tangent
- `Complex.sinh(c: Complex): Complex` — complex hyperbolic sine
- `Complex.cosh(c: Complex): Complex` — complex hyperbolic cosine
