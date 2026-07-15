# ValArray

The `tt.math.ValArray` module mirrors C++'s `<valarray>` header. It provides `ValArray<T>` — an array of numeric values with elementwise arithmetic — plus the `Slice`, `GSlice`, `IndirectArray`, `MaskArray`, and `SliceArray` proxy types used for BLAS-like subregion assignment, and transcendental overloads (`sin`, `cos`, `exp`, `log`, `abs`, `sqrt`) that apply element-wise.

## Import

```titrate
import tt::math::ValArray;
```

## ValArray

`ValArray<T>` is the primary container. It owns a contiguous array of numeric values and overloads the arithmetic operators to apply element-wise.

### Constructors

- `ValArray.init()` — empty array
- `ValArray.init(n: int)` — `n` zero-initialized elements
- `ValArray.init(value: T, n: int)` — `n` copies of `value`
- `ValArray.init(values: ArrayList<T>)` — copy from a list

### Element access

- `get(i: int): T` — read element `i`
- `set(i: int, value: T): void` — write element `i`
- `operator[](i: int): T` — alias for `get`
- `size(): int` — number of elements
- `resize(n: int): void` — resize (new elements zero-initialized)

### Arithmetic

Each operator applies element-wise and returns a new `ValArray<T>`:

- `operator+(other: ValArray<T>): ValArray<T>`
- `operator-(other: ValArray<T>): ValArray<T>`
- `operator*(other: ValArray<T>): ValArray<T>`
- `operator/(other: ValArray<T>): ValArray<T>`
- `operator+(scalar: T): ValArray<T>` — add scalar to every element
- `operator*(scalar: T): ValArray<T>` — multiply every element by scalar

### Assignment

- `assign(other: ValArray<T>): void` — copy assignment
- `assign(scalar: T): void` — fill with scalar
- `operator=(other: ValArray<T>): ValArray<T>` — copy assignment operator

### Reductions

- `sum(): T` — sum of all elements
- `min(): T` — minimum element
- `max(): T` — maximum element
- `product(): T` — product of all elements

### Subregion proxies

- `apply(f: fn(T): T): ValArray<T>` — return a new array with `f` applied to each element
- `shift(n: int): ValArray<T>` — circular shift by `n` (positive = left)
- `cshift(n: int): ValArray<T>` — circular shift (alias)
- `slice(start: int, size: int, stride: int): SliceArray<T>` — subregion proxy
- `gslice(start: int, sizes: ArrayList<int>, strides: ArrayList<int>): GSliceArray<T>` — generalized slice proxy
- `mask(m: ValArray<bool>): MaskArray<T>` — boolean-mask proxy
- `indirect(idx: ValArray<int>): IndirectArray<T>` — indirect-index proxy

```titrate
let a: ValArray<double> = new ValArray<double>(1.0, 5);
let b: ValArray<double> = new ValArray<double>(2.0, 5);
let c: ValArray<double> = a.operator+(b);  // [3, 3, 3, 3, 3]
let s: double = c.sum();  // 15.0
```

## Slice

`Slice` describes a strided subregion `[start, start+stride, start+2*stride, ...]` of size `size`.

- `Slice.init(start: int, size: int, stride: int)`
- `start(): int`
- `size(): int`
- `stride(): int`

## GSlice

`GSlice` describes a multi-dimensional strided subregion defined by a list of `(size, stride)` pairs.

- `GSlice.init(start: int, sizes: ArrayList<int>, strides: ArrayList<int>)`
- `start(): int`
- `sizes(): ArrayList<int>`
- `strides(): ArrayList<int>`

## Proxy types

`SliceArray<T>`, `GSliceArray<T>`, `MaskArray<T>`, and `IndirectArray<T>` are lightweight references into a `ValArray<T>`. They support assignment from a scalar or another `ValArray<T>`, and conversion back to a `ValArray<T>` via `toValArray()`.

```titrate
let v: ValArray<int> = new ValArray<int>(0, 10);
let s: SliceArray<int> = v.slice(0, 5, 2);  // indices 0, 2, 4, 6, 8
s.assign(7);  // set those elements to 7
```

## Transcendental overloads

The module exposes top-level functions that apply element-wise to a `ValArray<T>`:

- `ValArrayMath.sin(a: ValArray<double>): ValArray<double>`
- `ValArrayMath.cos(a: ValArray<double>): ValArray<double>`
- `ValArrayMath.tan(a: ValArray<double>): ValArray<double>`
- `ValArrayMath.exp(a: ValArray<double>): ValArray<double>`
- `ValArrayMath.log(a: ValArray<double>): ValArray<double>`
- `ValArrayMath.log10(a: ValArray<double>): ValArray<double>`
- `ValArrayMath.sqrt(a: ValArray<double>): ValArray<double>`
- `ValArrayMath.abs(a: ValArray<T>): ValArray<T>`
- `ValArrayMath.pow(a: ValArray<double>, b: ValArray<double>): ValArray<double>`
- `ValArrayMath.atan2(a: ValArray<double>, b: ValArray<double>): ValArray<double>`

```titrate
let xs: ValArray<double> = new ValArray<double>(0.0, 5);
xs.set(0, 0.0); xs.set(1, 0.5); xs.set(2, 1.0); xs.set(3, 1.5); xs.set(4, 2.0);
let sines: ValArray<double> = ValArrayMath.sin(xs);
```
