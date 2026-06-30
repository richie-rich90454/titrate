---
title: math::linalg
description: Dense and sparse linear algebra for Titrate — matrices, decompositions, solvers, and SVD.
---

# linalg

The `tt.math.linalg` module provides dense and sparse linear algebra operations: matrices, decompositions, solvers, eigenvalues, and SVD.

```titrate
import tt::math::linalg::Matrix;
import tt::math::linalg::MatrixDecomp;
import tt::math::linalg::SparseMatrix;
```

## Matrix

Dense matrix type.

- `fn init(r: int, c: int)`
- `get(i: int, j: int): double`
- `set(i: int, j: int, val: double): void`
- `getRow(i: int): NDArray<double>`
- `getCol(j: int): NDArray<double>`
- `setRow(i: int, row: NDArray<double>): void`
- `setCol(j: int, col: NDArray<double>): void`
- `rows(): int`
- `cols(): int`
- `add(other: Matrix): Matrix`
- `sub(other: Matrix): Matrix`
- `mul(other: Matrix): Matrix`
- `scale(s: double): Matrix`
- `transpose(): Matrix`
- `trace(): double`
- `clone(): Matrix`
- `equals(other: Matrix): bool`
- `toString(): string`

### Factory Functions

- `Matrix.identity(n: int): Matrix`
- `Matrix.zeros(r: int, c: int): Matrix`
- `Matrix.ones(r: int, c: int): Matrix`
- `Matrix.fromNDArray(arr: NDArray<double>): Matrix`
- `Matrix.fromRows(rows: ArrayList<NDArray<double>>): Matrix`
- `Matrix.fromCols(cols: ArrayList<NDArray<double>>): Matrix`

```titrate
let a: Matrix = new Matrix(2, 2);
a.set(0, 0, 4.0);
a.set(0, 1, 7.0);
a.set(1, 0, 2.0);
a.set(1, 1, 6.0);

let inv: Matrix = MatrixDecomp.inverse(a);
io::println(inv.toString());
```

## Matrix Decompositions and Solvers

`MatrixDecomp` contains numerical linear algebra routines.

- `gaussianElimination(m: Matrix): Matrix`
- `luDecompose(m: Matrix): (Matrix, Matrix)`
- `solve(m: Matrix, b: Matrix): Matrix`
- `inverse(m: Matrix): Matrix`
- `choleskyDecompose(m: Matrix): Matrix`
- `eigenvalues(m: Matrix): ArrayList<double>`
- `eig(m: Matrix): ArrayList<Variant>`
- `powerIteration(m: Matrix, maxIter: int): ArrayList<Variant>`
- `qrDecompose(m: Matrix): (Matrix, Matrix)`
- `svd(m: Matrix): ArrayList<Variant>`
- `pseudoInverse(m: Matrix): Matrix`
- `rref(m: Matrix): Matrix`
- `cramersRule(a: Matrix, b: ArrayList<double>): ArrayList<double>`
- `matrixExp(m: Matrix): Matrix`
- `matrixPower(m: Matrix, n: int): Matrix`
- `kron(a: Matrix, b: Matrix): Matrix`

```titrate
let b: Matrix = new Matrix(2, 1);
b.set(0, 0, 1.0);
b.set(1, 0, 2.0);

let x: Matrix = MatrixDecomp.solve(a, b);
io::println(x.toString());
```

## Sparse Matrices

`SparseMatrix` provides CSR and CSC sparse formats.

### CSRMatrix

- `fn init(rows: int, cols: int)`
- `set(row: int, col: int, val: double): void`
- `get(row: int, col: int): double`
- `multiplyVec(x: ArrayList<double>): ArrayList<double>`
- `transpose(): CSRMatrix`
- `toDense(): ArrayList<ArrayList<double>>`
- `nonZeroCount(): int`
- `toString(): string`

### CSCMatrix

Same API as `CSRMatrix` with column-major storage.

### Sparse Solvers and Conversion

- `SparseMatrix.sparseFromDense(dense: ArrayList<ArrayList<double>>): CSRMatrix`
- `SparseMatrix.csrToCsc(csr: CSRMatrix): CSCMatrix`
- `SparseMatrix.cscToCsr(csc: CSCMatrix): CSRMatrix`
- `SparseMatrix.sparseConjugateGradient(a: CSRMatrix, b: ArrayList<double>, x0: ArrayList<double>, maxIter: int, tol: double): ArrayList<double>`
- `SparseMatrix.sparseLU(a: CSRMatrix): SparseLUResult`
- `SparseMatrix.sparseSolveLU(lu: SparseLUResult, b: ArrayList<double>): ArrayList<double>`

```titrate
let sparse: CSRMatrix = new CSRMatrix(3, 3);
sparse.set(0, 0, 2.0);
sparse.set(1, 1, 3.0);
sparse.set(2, 2, 4.0);

io::println("Non-zeros: " + Integer.toString(sparse.nonZeroCount()));
```
