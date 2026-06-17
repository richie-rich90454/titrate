# sparse-linalg

The `tt.math.linalg.SparseMatrix` module provides sparse matrix storage formats and solvers for large-scale linear algebra with many zero entries.

```titrate
import tt.math.linalg.SparseMatrix;
```

## CSRMatrix

Compressed Sparse Row format. Efficient for row-wise operations and matrix-vector multiplication.

- `fn init(rows: int, cols: int)` — create an empty CSR matrix
- `set(row: int, col: int, val: double): void` — insert or update a value (setting 0.0 removes the entry)
- `get(row: int, col: int): double` — get value (returns 0.0 if not stored)
- `multiplyVec(x: ArrayList<double>): ArrayList<double>` — sparse matrix-vector multiply (y = A·x)
- `transpose(): CSRMatrix` — transpose the matrix
- `toDense(): ArrayList<ArrayList<double>>` — convert to dense representation
- `nonZeroCount(): int` — number of stored (non-zero) entries

```titrate
let a: CSRMatrix = new CSRMatrix(3, 3);
a.set(0, 0, 1.0);
a.set(1, 1, 2.0);
a.set(2, 2, 3.0);
a.set(0, 2, 0.5);

let x: ArrayList<double> = new ArrayList<double>();
x.add(1.0); x.add(1.0); x.add(1.0);
let y: ArrayList<double> = a.multiplyVec(x);
// y = [1.5, 2.0, 3.0]
```

## CSCMatrix

Compressed Sparse Column format. Efficient for column-wise operations and transpose access.

- `fn init(rows: int, cols: int)` — create an empty CSC matrix
- `set(row: int, col: int, val: double): void` — insert or update a value
- `get(row: int, col: int): double` — get value (returns 0.0 if not stored)
- `multiplyVec(x: ArrayList<double>): ArrayList<double>` — sparse matrix-vector multiply (y = A·x)
- `transpose(): CSCMatrix` — transpose the matrix
- `toDense(): ArrayList<ArrayList<double>>` — convert to dense representation
- `nonZeroCount(): int` — number of stored (non-zero) entries

```titrate
let a: CSCMatrix = new CSCMatrix(3, 3);
a.set(0, 0, 4.0);
a.set(1, 1, 5.0);
a.set(2, 2, 6.0);
```

## Conversion Functions

- `sparseFromDense(dense: ArrayList<ArrayList<double>>): CSRMatrix` — convert a dense matrix to CSR format
- `csrToCsc(csr: CSRMatrix): CSCMatrix` — convert CSR to CSC
- `cscToCsr(csc: CSCMatrix): CSRMatrix` — convert CSC to CSR

```titrate
let dense: ArrayList<ArrayList<double>> = new ArrayList<ArrayList<double>>();
// ... populate dense ...
let csr: CSRMatrix = sparseFromDense(dense);
let csc: CSCMatrix = csrToCsc(csr);
```

## Sparse Conjugate Gradient Solver

- `sparseConjugateGradient(a: CSRMatrix, b: ArrayList<double>, x0: ArrayList<double>, maxIter: int, tol: double): ArrayList<double>` — solve Ax = b where A is symmetric positive definite

Parameters:
- `a` — CSR matrix (must be symmetric positive definite)
- `b` — right-hand side vector
- `x0` — initial guess
- `maxIter` — maximum number of iterations
- `tol` — convergence tolerance on residual norm

```titrate
let a: CSRMatrix = new CSRMatrix(3, 3);
a.set(0, 0, 4.0); a.set(1, 1, 5.0); a.set(2, 2, 6.0);

let b: ArrayList<double> = new ArrayList<double>();
b.add(4.0); b.add(5.0); b.add(6.0);

let x0: ArrayList<double> = new ArrayList<double>();
x0.add(0.0); x0.add(0.0); x0.add(0.0);

let x: ArrayList<double> = sparseConjugateGradient(a, b, x0, 100, 1e-10);
// x ≈ [1.0, 1.0, 1.0]
```

## COO Format

- `SparseCOO(rows: int, cols: int)` — coordinate format sparse matrix
- `add(row: int, col: int, value: double): void` — add entry
- `toCSR(): SparseCSR` — convert to CSR
- `toDense(): ArrayList<ArrayList<double>>` — convert to dense

## DOK Format

- `SparseDOK(rows: int, cols: int)` — dictionary-of-keys format
- `set(row: int, col: int, value: double): void` — set entry
- `get(row: int, col: int): double` — get entry
- `toCSR(): SparseCSR` — convert to CSR

## LIL Format

- `SparseLIL(rows: int, cols: int)` — list-of-lists format
- `append(row: int, col: int, value: double): void` — append entry to row
- `sortIndices(): void` — sort column indices per row
- `toCSR(): SparseCSR` — convert to CSR

## Iterative Solvers

- `IterativeSolvers.cg(a: SparseCSR, b: ArrayList<double>, x0: ArrayList<double>, tol: double, maxIter: int): ArrayList<double>` — conjugate gradient
- `IterativeSolvers.gmres(a: SparseCSR, b: ArrayList<double>, x0: ArrayList<double>, tol: double, maxIter: int, restart: int): ArrayList<double>` — GMRES
- `IterativeSolvers.bicgstab(a: SparseCSR, b: ArrayList<double>, x0: ArrayList<double>, tol: double, maxIter: int): ArrayList<double>` — BiCGSTAB

## Sparse Eigenvalues

- `SparseEigen.powerIteration(a: SparseCSR, maxIter: int, tol: double): (double, ArrayList<double>)` — largest eigenvalue and eigenvector
- `SparseEigen.arnoldi(a: SparseCSR, k: int, maxIter: int): ArrayList<double>` — k largest eigenvalues via Arnoldi
