# calculus

The `tt::calculus` module provides numerical differentiation, integration, root-finding, optimization, and symbolic calculus utilities. It is useful for scientific computing, engineering, and mathematical analysis.

## Numerical Differentiation

### derivative

Compute the numerical derivative of a function using central differences:

```titrate
import tt::calculus::Calculus;

// Derivative of sin(x) at x=0 (should be cos(0) = 1.0)
let result: double = Calculus.derivative(fn(x: double): double {
    return MathTrig.sin(x);
}, 0.0);
// result ≈ 1.0
```

- `derivative(f: fn(double): double, x: double): double` — derivative with default h=1e-7
- `derivative(f: fn(double): double, x: double, h: double): double` — derivative with custom step size

### secondDerivative

Compute the second derivative:

```titrate
let result: double = Calculus.secondDerivative(fn(x: double): double {
    return x * x;
}, 1.0);
// result ≈ 2.0
```

### nthDerivative

Compute the nth derivative recursively:

```titrate
let result: double = Calculus.nthDerivative(fn(x: double): double {
    return MathAdvanced.pow(x, 3.0);
}, 1.0, 3, 1e-5);
// result ≈ 6.0 (third derivative of x^3 is 6)
```

### partialDerivative

Compute partial derivatives of multivariable functions:

```titrate
let f = fn(x: double, y: double): double { return x * x + y * y; };
let dfdx: double = Calculus.partialDerivative(f, 1.0, 2.0, 0, 1e-7);
// df/dx at (1,2) ≈ 2.0
```

## Numerical Integration

### riemannSum

Compute Riemann sums with left, right, or midpoint rules:

```titrate
let area: double = Calculus.riemannSum(fn(x: double): double {
    return x * x;
}, 0.0, 1.0, 1000, "midpoint");
// area ≈ 0.333... (integral of x^2 from 0 to 1)
```

### trapezoidalRule

The trapezoidal rule for numerical integration:

```titrate
let area: double = Calculus.trapezoidalRule(fn(x: double): double {
    return x * x;
}, 0.0, 1.0, 1000);
```

### simpsonsRule

Simpson's rule for higher accuracy:

```titrate
let area: double = Calculus.simpsonsRule(fn(x: double): double {
    return x * x;
}, 0.0, 1.0, 1000);
```

### gaussianQuadrature

Gaussian quadrature for smooth functions:

```titrate
let area: double = Calculus.gaussianQuadrature(fn(x: double): double {
    return x * x;
}, 0.0, 1.0, 5);
```

## Root Finding

### bisection

Find roots using the bisection method:

```titrate
let root: double = Calculus.bisection(fn(x: double): double {
    return x * x - 2.0;
}, 0.0, 2.0, 1e-10);
// root ≈ 1.414... (√2)
```

### newtonRaphson

Find roots using Newton-Raphson (requires derivative):

```titrate
let root: double = Calculus.newtonRaphson(
    fn(x: double): double { return x * x - 2.0; },
    fn(x: double): double { return 2.0 * x; },
    1.0,
    1e-10
);
```

### secant

Find roots using the secant method:

```titrate
let root: double = Calculus.secant(fn(x: double): double {
    return x * x - 2.0;
}, 1.0, 2.0, 1e-10);
```

## Optimization

### gradientDescent

Minimize a function using gradient descent:

```titrate
let minimum: double = Calculus.gradientDescent(fn(x: double): double {
    return (x - 3.0) * (x - 3.0);
}, 0.0, 0.1, 1000);
// minimum ≈ 3.0
```

### goldenSection

Find minimum using golden section search:

```titrate
let minimum: double = Calculus.goldenSection(fn(x: double): double {
    return (x - 3.0) * (x - 3.0);
}, 0.0, 5.0, 1e-10);
```

## Taylor Series

### taylorSeries

Evaluate a Taylor series approximation:

```titrate
let coefficients: ArrayList<double> = new ArrayList<double>();
coefficients.add(1.0);   // constant term
coefficients.add(0.0);   // coefficient of x
coefficients.add(0.5);   // coefficient of x^2
// e^x ≈ 1 + x + x^2/2!

let value: double = Calculus.taylorSeries(coefficients, 1.0);
// value ≈ e^1 ≈ 2.718...
```

## ODE Solvers

### rungeKutta4

Solve ODEs using Runge-Kutta 4th order:

```titrate
// Solve dy/dx = -y, y(0) = 1
// Solution: y = e^(-x)
let solution: ArrayList<ArrayList<double>> = Calculus.rungeKutta4(
    fn(x: double, y: double): double { return -y; },
    0.0, 1.0, 5.0, 0.01
);
```

## Module Functions Reference

| Function | Description |
|----------|-------------|
| `derivative(f, x)` | First derivative at x |
| `secondDerivative(f, x)` | Second derivative at x |
| `nthDerivative(f, x, n, h)` | Nth derivative |
| `partialDerivative(f, x, y, varIndex, h)` | Partial derivative |
| `riemannSum(f, a, b, n, type)` | Riemann sum integration |
| `trapezoidalRule(f, a, b, n)` | Trapezoidal integration |
| `simpsonsRule(f, a, b, n)` | Simpson's rule integration |
| `gaussianQuadrature(f, a, b, n)` | Gaussian quadrature |
| `bisection(f, a, b, tol)` | Bisection root finding |
| `newtonRaphson(f, df, x0, tol)` | Newton-Raphson root finding |
| `secant(f, x0, x1, tol)` | Secant root finding |
| `gradientDescent(f, x0, lr, iter)` | Gradient descent optimization |
| `goldenSection(f, a, b, tol)` | Golden section optimization |
| `taylorSeries(coeffs, x)` | Taylor series evaluation |
| `rungeKutta4(f, x0, y0, xf, h)` | RK4 ODE solver |
