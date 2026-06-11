# math

The `tt.math` module provides mathematical constants, functions, linear algebra, multi-dimensional arrays, and random number generation.

```titrate
import tt.math.Math;
import tt.math.ndarray.NDArray;
import tt.math.linalg.Matrix;
import tt.random.Random;
```

## Math

Mathematical constants and functions.

**Constants:**
- `PI(): double` — π ≈ 3.14159
- `E(): double` — e ≈ 2.71828
- `INF(): double` — positive infinity
- `NAN(): double` — not-a-number
- `MAX_DOUBLE(): double`, `MIN_DOUBLE(): double` — double range
- `MAX_INT(): long`, `MIN_INT(): long` — integer range

**Trigonometric:**
- `sin(x)`, `cos(x)`, `tan(x)` — standard trig
- `asin(x)`, `acos(x)`, `atan(x)` — inverse trig
- `atan2(y, x)` — two-argument arctangent
- `toRadians(degrees)`, `toDegrees(radians)` — unit conversion

**Hyperbolic:**
- `sinh(x)`, `cosh(x)`, `tanh(x)` — hyperbolic functions
- `asinh(x)`, `acosh(x)`, `atanh(x)` — inverse hyperbolic

**Exponential / Logarithmic:**
- `exp(x)`, `expm1(x)` — e^x and e^x - 1
- `ln(x)`, `log10(x)`, `log2(x)` — logarithms
- `log(base, x)` — logarithm with arbitrary base
- `log1p(x)` — ln(1 + x)

**Power / Root:**
- `pow(base, exp)`, `sqrt(x)`, `cbrt(x)` — power and roots
- `hypot(a, b)` — sqrt(a² + b²)

**Rounding / Utility:**
- `abs(x)`, `absInt(x)` — absolute value
- `floor(x)`, `ceil(x)`, `round(x)` — rounding
- `min(a, b)`, `max(a, b)` — minimum / maximum
- `clamp(x, lo, hi)` — constrain to range
- `sign(x)` — sign function (-1, 0, 1)
- `random(): double` — random value in [0, 1)

**Exact arithmetic:**
- `addExact(a, b)`, `subtractExact(a, b)`, `multiplyExact(a, b)` — overflow-checked
- `incrementExact(a)`, `decrementExact(a)`, `negateExact(a)` — overflow-checked

```titrate
let angle = Math.toRadians(45.0);
let result = Math.sin(angle);  // ≈ 0.7071
let clamped = Math.clamp(15.0, 0.0, 10.0);  // 10.0
```

## NDArray

Multi-dimensional array with generic element type. Supports indexing, reshaping, slicing, broadcasting, and statistical reductions.

**Factory methods:**
- `NDArray.zeros(shape: ArrayList<int>): NDArray<double>` — zero-filled array
- `NDArray.ones(shape: ArrayList<int>): NDArray<double>` — one-filled array
- `NDArray.filled(shape: ArrayList<int>, value: double): NDArray<double>` — constant-filled
- `NDArray.fromData(shape: ArrayList<int>, data: ArrayList<T>): NDArray<T>` — from flat data

**Indexing:**
- `get(indices: ArrayList<int>): T` — multi-dimensional access
- `set(indices: ArrayList<int>, value: T): void` — multi-dimensional set
- `get1D(i: int)`, `get2D(i, j)`, `get3D(i, j, k)` — convenience accessors
- `getFlat(i: int): T` — linear index into data buffer

**Shape operations:**
- `reshape(newShape: ArrayList<int>): NDArray<T>` — reshape (shares data)
- `transpose(): NDArray<T>` — reverse axis order
- `flatten(): NDArray<T>` — collapse to 1D (shares data)
- `squeeze(): NDArray<T>` — remove size-1 dimensions
- `expandDims(): NDArray<T>` — add a dimension
- `broadcastTo(targetShape: ArrayList<int>): NDArray<T>` — broadcast
- `concat(other: NDArray<T>, axis: int): NDArray<T>` — concatenate
- `stack(other: NDArray<T>, axis: int): NDArray<T>` — stack along new axis
- `slice(starts: ArrayList<int>, ends: ArrayList<int>): NDArray<T>` — sub-array

**Reductions:**
- `sum(): double`, `mean(): double`, `min(): double`, `max(): double`
- `variance(): double`, `stddev(): double`
- `argMax(): int`, `argMin(): int`
- `dot(other: NDArray<T>): double` — dot product
- `norm(): double` — Euclidean norm
- `any(): bool`, `all(): bool` — boolean reductions

**Operators:**
- `operator+` — element-wise addition
- `operator-` — element-wise subtraction
- `operator*` — scalar multiplication

**Other:**
- `map(f: fn(T): T): NDArray<T>` — element-wise transform
- `zipMap(other: NDArray<T>, f: fn(T, T): T): NDArray<T>` — binary element-wise
- `sort(): NDArray<T>` — sorted copy (ascending)
- `argsort(): NDArray<int>` — indices that would sort
- `unique(): NDArray<T>` — unique sorted values
- `nonzero(): NDArray<int>` — indices of nonzero elements
- `where(condition: NDArray<T>): NDArray<T>` — conditional select
- `clip(lo: double, hi: double): NDArray<T>` — clip values
- `repeat(n: int): NDArray<T>`, `tile(reps: ArrayList<int>): NDArray<T>` — repetition
- `size(): int`, `ndim(): int`, `rows(): int`, `cols(): int`

```titrate
let shape = new ArrayList<int>();
shape.add(2); shape.add(3);
let a = NDArray.zeros(shape);
a.set2D(0, 0, 1.0);
a.set2D(1, 2, 5.0);
let b = NDArray.ones(shape);
let c = a + b;  // element-wise addition
io::println(c.sum().toString());  // 12.0
```

## Matrix

Wraps an `NDArray<double>` for linear algebra operations.

**Factory methods:**
- `Matrix(r: int, c: int)` — zero matrix
- `Matrix.identity(n: int): Matrix` — identity matrix
- `Matrix.zeros(r, c): Matrix`, `Matrix.ones(r, c): Matrix`
- `Matrix.fromNDArray(arr: NDArray<double>): Matrix`
- `Matrix.fromRows(rows: ArrayList<NDArray<double>>): Matrix`
- `Matrix.fromCols(cols: ArrayList<NDArray<double>>): Matrix`

**Element access:**
- `get(i, j): double`, `set(i, j, val: double): void`
- `getRow(i): NDArray<double>`, `getCol(j): NDArray<double>`
- `setRow(i, row)`, `setCol(j, col)`
- `rows(): int`, `cols(): int`

**Core operations:**
- `add(other: Matrix): Matrix` — element-wise addition
- `sub(other: Matrix): Matrix` — element-wise subtraction
- `mul(other: Matrix): Matrix` — matrix multiplication
- `scale(s: double): Matrix` — scalar multiplication
- `transpose(): Matrix` — transpose
- `trace(): double` — sum of diagonal

**Decompositions / Solvers:**
- `determinant(): double` — determinant via LU decomposition
- `inverse(): Matrix` — inverse via Gauss-Jordan
- `luDecompose(): (Matrix, Matrix)` — (L, U) decomposition
- `choleskyDecompose(): Matrix` — Cholesky factor L (A = L·Lᵀ)
- `solve(b: Matrix): Matrix` — solve Ax = b
- `gaussianElimination(): Matrix` — row echelon form

**Properties:**
- `rank(): int` — matrix rank
- `norm(): double` — Frobenius norm
- `conditionNumber(): double` — condition number
- `isSymmetric(): bool`, `isPositiveDefinite(): bool`, `isDiagonal(): bool`

**Other:**
- `cross(other: Matrix): Matrix` — 3D cross product
- `outerProduct(other: Matrix): Matrix` — outer product
- `concatHorizontal(other: Matrix): Matrix` — side-by-side
- `concatVertical(other: Matrix): Matrix` — stacked
- `subMatrix(r1, r2, c1, c2): Matrix` — extract sub-matrix

```titrate
let a = Matrix.identity(3);
let b = new Matrix(3, 1);
b.set(0, 0, 1.0); b.set(1, 0, 2.0); b.set(2, 0, 3.0);
let x = a.solve(b);  // x = b since a is identity
io::println(a.determinant().toString());  // 1.0
```

## Random

Pseudo-random number generation using Xorshift128+.

- `Random()` — create with auto seed
- `Random(seed: long)` — create with specific seed
- `nextInt(max: int): int` — random int in [0, max)
- `nextInt(min: int, max: int): int` — random int in [min, max]
- `nextLong(max: long): long` — random long in [0, max)
- `nextFloat(): float` — random float in [0, 1)
- `nextDouble(): double` — random double in [0, 1)
- `nextBool(): bool` — random boolean
- `nextGaussian(): double` — Gaussian (mean=0, stddev=1) via Box-Muller
- `nextExponential(): double` — exponential distribution (rate=1)
- `nextPoisson(lambda: double): int` — Poisson distribution
- `nextUniform(min: double, max: double): double` — uniform in [min, max)
- `shuffle(arr: ArrayList): void` — Fisher-Yates shuffle in-place
- `sample(arr: ArrayList): Object` — random element

```titrate
let rng = new Random(42);
let dice = rng.nextInt(1, 6);  // 1 to 6
let normal = rng.nextGaussian();
```
