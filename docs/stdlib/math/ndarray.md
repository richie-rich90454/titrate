---
title: math::ndarray
description: Multi-dimensional arrays and numerical computing for Titrate.
---

# ndarray

The `tt.math.ndarray` module provides multi-dimensional arrays for numerical computing. `NDArray<T>` supports element access, reshaping, broadcasting, mathematical operations, reductions, and generation helpers.

```titrate
import tt::math::ndarray::NDArray;
import tt::math::ndarray::NDArrayMath;
import tt::math::ndarray::NDArrayReduce;
import tt::math::ndarray::NDArrayManip;
import tt::math::ndarray::NDArrayGen;
import tt::util::ArrayList;
```

## NDArray

The core generic array type.

- `fn init(shape: ArrayList<int>)`
- `get(indices: ArrayList<int>): T`
- `set(indices: ArrayList<int>, value: T): void`
- `getFlat(i: int): T`
- `setFlat(i: int, value: T): void`
- `get1D(i: int): T`
- `set1D(i: int, value: T): void`
- `get2D(i: int, j: int): T`
- `set2D(i: int, j: int, value: T): void`
- `get3D(i: int, j: int, k: int): T`
- `set3D(i: int, j: int, k: int, value: T): void`
- `reshape(newShape: ArrayList<int>): NDArray<T>`
- `transpose(): NDArray<T>`
- `flatten(): NDArray<T>`
- `fill(value: T): void`
- `clone(): NDArray<T>`
- `equals(other: NDArray<T>): bool`
- `size(): int`
- `ndim(): int`
- `rows(): int`
- `cols(): int`
- `totalElements(): int`
- `toString(): string`

### Factory Functions

- `NDArray.zeros(shape: ArrayList<int>): NDArray<double>`
- `NDArray.ones(shape: ArrayList<int>): NDArray<double>`
- `NDArray.filled(shape: ArrayList<int>, value: double): NDArray<double>`
- `NDArray.fromData<T>(shape: ArrayList<int>, data: ArrayList<T>): NDArray<T>`

```titrate
let shape: ArrayList<int> = new ArrayList<int>();
shape.add(2);
shape.add(3);

let a: NDArray<double> = NDArray.zeros(shape);
a.set2D(0, 1, 5.0);
a.set2D(1, 2, 7.0);

io::println(a.toString());
io::println("Total elements: " + Integer.toString(a.totalElements()));
```

## Element-Wise Math

`NDArrayMath` provides vectorized operations.

- `NDArrayMath.map<T>(a: NDArray<T>, f: fn(T): T): NDArray<T>`
- `NDArrayMath.zipMap<T>(a: NDArray<T>, other: NDArray<T>, f: fn(T, T): T): NDArray<T>`
- `NDArrayMath.dot<T>(a: NDArray<T>, other: NDArray<T>): double`
- `NDArrayMath.norm<T>(a: NDArray<T>): double`

### Arithmetic and Powers

- `ndAdd`, `ndSubtract`, `ndMultiply`, `ndDivide`, `ndMod`, `ndPower`
- `ndMaximum`, `ndMinimum`
- `ndNegative`, `ndReciprocal`, `ndSquare`
- `ndClip(a: NDArray<double>, minVal: double, maxVal: double): NDArray<double>`

### Transcendentals

- `ndSin`, `ndCos`, `ndTan`
- `ndAsin`, `ndAcos`, `ndAtan`
- `ndSinh`, `ndCosh`, `ndTanh`
- `ndAsinh`, `ndAcosh`, `ndAtanh`
- `ndExp`, `ndExp2`, `ndExpm1`
- `ndLog`, `ndLog2`, `ndLog10`, `ndLog1p`
- `ndSqrt`, `ndCbrt`

### Rounding and Signs

- `ndAbs`, `ndSign`, `ndFloor`, `ndCeil`, `ndRound`

```titrate
let b: NDArray<double> = NDArrayMath.ndAdd(a, NDArray.ones(shape));
let c: NDArray<double> = NDArrayMath.ndSqrt(b);
io::println(c.toString());
```

## Reductions

`NDArrayReduce` performs aggregate calculations.

- `sum<T>(a: NDArray<T>): double`
- `mean<T>(a: NDArray<T>): double`
- `min<T>(a: NDArray<T>): double`
- `max<T>(a: NDArray<T>): double`
- `variance<T>(a: NDArray<T>): double`
- `stddev<T>(a: NDArray<T>): double`
- `any<T>(a: NDArray<T>): bool`
- `all<T>(a: NDArray<T>): bool`
- `argMax<T>(a: NDArray<T>): int`
- `argMin<T>(a: NDArray<T>): int`
- `cumsum(a: NDArray<double>): NDArray<double>`
- `cumprod(a: NDArray<double>): NDArray<double>`
- `diff(a: NDArray<double>): NDArray<double>`
- `percentile(a: NDArray<double>, q: double): double`
- `ndMedian(a: NDArray<double>): double`

```titrate
io::println("Sum: " + Double.toString(NDArrayReduce.sum(a)));
io::println("Max: " + Double.toString(NDArrayReduce.max(a)));
```

## Manipulation

`NDArrayManip` reshapes and reorders arrays.

- `ndPad(a: NDArray<double>, widths: ArrayList<int>, mode: string): NDArray<double>`
- `ndFlip(a: NDArray<double>): NDArray<double>`
- `ndFliplr(a: NDArray<double>): NDArray<double>`
- `ndFlipud(a: NDArray<double>): NDArray<double>`
- `ndRoll(a: NDArray<double>, shift: int): NDArray<double>`
- `ndRot90(a: NDArray<double>): NDArray<double>`

## Generation

`NDArrayGen` creates regularly spaced arrays.

- `linspace(start: double, stop: double, num: int): NDArray<double>`
- `arange(start: double, stop: double, step: double): NDArray<double>`
- `logspace(start: double, stop: double, num: int, base: double): NDArray<double>`
- `logspace(start: double, stop: double, num: int): NDArray<double>`
- `geomspace(start: double, stop: double, num: int): NDArray<double>`
- `meshgrid(x: NDArray<double>, y: NDArray<double>): ArrayList<NDArray<double>>`

```titrate
let xs: NDArray<double> = NDArrayGen.linspace(0.0, 1.0, 11);
io::println(xs.toString());
```

## Broadcasting

`Broadcast` computes broadcasted shapes and aligns arrays.

- `broadcastShapes(shape1: ArrayList<int>, shape2: ArrayList<int>): ArrayList<int>`
- `broadcastTo(data: ArrayList<double>, shape: ArrayList<int>, targetShape: ArrayList<int>): ArrayList<double>`
- `broadcastArrays(a, shapeA, b, shapeB): Pair<ArrayList<double>, ArrayList<int>>`
