# Numeric

The `tt::numeric::Numeric` module provides C++ `<numeric>` parity. It provides `accumulate`, `inner_product`, `adjacent_difference`, `partial_sum`, `inclusive_scan`, `exclusive_scan`, `transform_reduce`, `reduce`, `midpoint`, `gcd`, and `lcm` as top-level functions. `gcd` and `lcm` are promoted here from `tt::math::Math` so that `<numeric>` consumers do not need to pull in the broader `Math` module.

## Import

```titrate
import tt::numeric::Numeric;
```

## API Reference

### accumulate

#### `accumulate<T>(list: ArrayList<T>, init: T, op: fn(T, T): T): T`

Accumulate (fold left) with a custom binary operator and initial value. Mirrors `std::accumulate(first, last, init, op)`.

#### `accumulateSum<T>(list: ArrayList<T>, init: T): T`

Accumulate with the default plus operator. Only meaningful for numeric types that support `+`. Returns `init + sum(elements)`.

### inner_product

#### `innerProduct<T>(a: ArrayList<T>, b: ArrayList<T>, init: T, mul: fn(T, T): T, add: fn(T, T): T): T`

Compute the inner product of two lists with custom `mul` and `add` operations. Mirrors `std::inner_product` with binary ops.

#### `innerProductDefault<T>(a: ArrayList<T>, b: ArrayList<T>, init: T): T`

Compute the inner product with the default `*` and `+` operators.

### adjacent_difference

#### `adjacentDifference<T>(list: ArrayList<T>, op: fn(T, T): T): ArrayList<T>`

Compute adjacent differences with a custom subtraction op. `result[0] = list[0]`; `result[i] = op(list[i], list[i-1])`.

#### `adjacentDifferenceDefault<T>(list: ArrayList<T>): ArrayList<T>`

Compute adjacent differences with the default `-` operator.

### partial_sum

#### `partialSum<T>(list: ArrayList<T>, op: fn(T, T): T): ArrayList<T>`

Compute partial sums (prefix sums) with a custom add op. `result[0] = list[0]`; `result[i] = op(result[i-1], list[i])`.

#### `partialSumDefault<T>(list: ArrayList<T>): ArrayList<T>`

Compute partial sums with the default `+` operator.

### inclusive_scan

#### `inclusiveScan<T>(list: ArrayList<T>, init: T, op: fn(T, T): T): ArrayList<T>`

Inclusive scan: `result[i]` includes `list[i]` in the sum. `result[0] = list[0]`; `result[i] = op(result[i-1], list[i])`.

#### `inclusiveScanInit<T>(list: ArrayList<T>, init: T, op: fn(T, T): T): ArrayList<T>`

Inclusive scan with an initial value that is folded into the first element. `result[0] = op(init, list[0])`; `result[i] = op(result[i-1], list[i])`.

### exclusive_scan

#### `exclusiveScan<T>(list: ArrayList<T>, init: T, op: fn(T, T): T): ArrayList<T>`

Exclusive scan: `result[i]` excludes `list[i]` from the sum. `result[0] = init`; `result[i] = op(result[i-1], list[i-1])`.

### transform_reduce

#### `transformReduce<T, R>(list: ArrayList<T>, init: R, transform: fn(T): R, reduce: fn(R, R): R): R`

Transform-reduce: apply `transform` to each element, then reduce with `init`.

#### `transformReduce2<T, U, R>(a: ArrayList<T>, b: ArrayList<U>, init: R, transform: fn(T, U): R, reduce: fn(R, R): R): R`

Transform-reduce over two lists: `reduce(add, transform(a[i], b[i]))` with `init`.

### reduce

#### `reduce<T>(list: ArrayList<T>, init: T, op: fn(T, T): T): T`

Reduce (fold left) with an initial value and binary operator. Differs from `accumulate` only by name; mirrors `std::reduce`.

#### `reduceSum<T>(list: ArrayList<T>, init: T): T`

Reduce with the default `+` operator.

### midpoint

#### `midpointInt(a: int, b: int): int`

Integer midpoint. Computes `(a + b) / 2` with overflow-safe rounding toward zero.

#### `midpointDouble(a: double, b: double): double`

Floating-point midpoint. Computes `(a + b) / 2.0`.

### gcd / lcm

#### `gcd(a: int, b: int): int`

Greatest common divisor (Euclidean algorithm). Promoted here from `Math` so that `<numeric>` consumers do not need to import `Math`. Result is always non-negative.

#### `lcm(a: int, b: int): int`

Least common multiple. Promoted here from `Math`. Returns `0` if either operand is `0`; otherwise the non-negative LCM.

## Usage Examples

### accumulate

```titrate
import tt::numeric::Numeric;
import tt::util::ArrayList;
import tt::io::IO;

public fn main(): void {
    let list = new ArrayList<int>();
    list.add(1); list.add(2); list.add(3); list.add(4);
    let sum: int = Numeric.accumulateSum<int>(list, 0);
    IO.println("sum = " + Integer.toString(sum));
    let product: int = Numeric.accumulate<int>(list, 1, fn(a: int, b: int): int => a * b);
    IO.println("product = " + Integer.toString(product));
}
```

### inner_product

```titrate
import tt::numeric::Numeric;
import tt::util::ArrayList;

let a = new ArrayList<int>(); a.add(1); a.add(2); a.add(3);
let b = new ArrayList<int>(); b.add(4); b.add(5); b.add(6);
let dot: int = Numeric.innerProductDefault<int>(a, b, 0);
io::println("dot product = " + Integer.toString(dot));
```

### partial_sum and adjacent_difference

```titrate
import tt::numeric::Numeric;
import tt::util::ArrayList;

let list = new ArrayList<int>();
list.add(1); list.add(2); list.add(3); list.add(4);
let sums: ArrayList<int> = Numeric.partialSumDefault<int>(list);
io::println("partial sums count: " + Integer.toString(sums.size()));
let diffs: ArrayList<int> = Numeric.adjacentDifferenceDefault<int>(list);
io::println("adjacent differences count: " + Integer.toString(diffs.size()));
```

### inclusive_scan and exclusive_scan

```titrate
import tt::numeric::Numeric;
import tt::util::ArrayList;

let list = new ArrayList<int>();
list.add(1); list.add(2); list.add(3); list.add(4);
let inc: ArrayList<int> = Numeric.inclusiveScan<int>(list, 0, fn(a: int, b: int): int => a + b);
io::println("inclusive scan: " + Integer.toString(inc.get(3)));
let exc: ArrayList<int> = Numeric.exclusiveScan<int>(list, 0, fn(a: int, b: int): int => a + b);
io::println("exclusive scan: " + Integer.toString(exc.get(3)));
```

### transform_reduce

```titrate
import tt::numeric::Numeric;
import tt::util::ArrayList;

let list = new ArrayList<int>();
list.add(1); list.add(2); list.add(3);
let sumSquares: int = Numeric.transformReduce<int, int>(list, 0,
    fn(x: int): int => x * x,
    fn(a: int, b: int): int => a + b);
io::println("sum of squares = " + Integer.toString(sumSquares));
```

### gcd and lcm

```titrate
import tt::numeric::Numeric;

io::println("gcd(12, 18) = " + Integer.toString(Numeric.gcd(12, 18)));
io::println("lcm(4, 6) = " + Integer.toString(Numeric.lcm(4, 6)));
io::println("midpointInt(1, 9) = " + Integer.toString(Numeric.midpointInt(1, 9)));
io::println("midpointDouble(1.0, 9.0) = " + Double.toString(Numeric.midpointDouble(1.0, 9.0)));
```
