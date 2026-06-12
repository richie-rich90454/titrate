# Scientific Computing

Titrate was designed with scientific computing in mind. Whether you're crunching numbers with N-dimensional arrays, running molecular dynamics simulations, or just making sure you don't accidentally add meters to seconds — the standard library has you covered. This guide walks you through the key modules: `tt.math` for numerical operations, `tt.chem` for computational chemistry, and `tt.units` for dimensional consistency.

## NDArray

The `NDArray` type (N-dimensional array) is the foundation for numerical computing in Titrate. If you've used NumPy in Python, you'll feel right at home — it provides efficient storage and operations on multi-dimensional data.

### Creating NDArrays

```titrate
// From a flat array with shape
let a = NDArray.fromArray([1.0, 2.0, 3.0, 4.0, 5.0, 6.0], (2, 3));

// Zeros and ones
let zeros = NDArray.zeros((3, 3));
let ones = NDArray.ones((2, 4));

// Identity matrix
let eye = NDArray.eye(3);

// Filled with a constant
let filled = NDArray.filled(3.14, (2, 2));
```

### Indexing

```titrate
let a = NDArray.fromArray([1.0, 2.0, 3.0, 4.0, 5.0, 6.0], (2, 3));

// Single element
let val = a.get(0, 1);  // 2.0

// Set an element
a.set(1, 2, 99.0);

// Slice a row
let row = a.row(0);  // NDArray of shape (3,)
```

### Arithmetic

NDArrays support element-wise arithmetic with operator overloading — so you can write `a + b` instead of looping manually:

```titrate
let a = NDArray.fromArray([1.0, 2.0, 3.0], (3,));
let b = NDArray.fromArray([4.0, 5.0, 6.0], (3,));

let sum = a + b;          // [5.0, 7.0, 9.0]
let diff = a - b;         // [-3.0, -3.0, -3.0]
let scaled = a * 2.0;     // [2.0, 4.0, 6.0]
let divided = b / 2.0;    // [2.0, 2.5, 3.0]
```

### Shape and Reshape

```titrate
let a = NDArray.fromArray([1.0, 2.0, 3.0, 4.0, 5.0, 6.0], (2, 3));
io::println(a.shape().toString());  // (2, 3)

let reshaped = a.reshape((3, 2));
io::println(reshaped.shape().toString());  // (3, 2)
```

## Matrix

While `NDArray` is great for general data, the `Matrix` type wraps a 2D NDArray and provides specialized linear algebra operations. If you need matrix multiplication, decompositions, or solving systems of equations, this is the module to reach for.

### Creating Matrices

```titrate
let m = Matrix.fromArray([
    [1.0, 2.0, 3.0],
    [4.0, 5.0, 6.0],
    [7.0, 8.0, 9.0]
]);

let identity = Matrix.eye(3);
let zero = Matrix.zeros(2, 2);
```

### Matrix Multiplication

```titrate
let a = Matrix.fromArray([
    [1.0, 2.0],
    [3.0, 4.0]
]);

let b = Matrix.fromArray([
    [5.0, 6.0],
    [7.0, 8.0]
]);

let c = a.matmul(b);
// [[19.0, 22.0],
//  [43.0, 50.0]]
```

### Decompositions

```titrate
let m = Matrix.fromArray([
    [4.0, 2.0],
    [1.0, 3.0]
]);

// LU decomposition
let (l, u) = m.lu();

// QR decomposition
let (q, r) = m.qr();

// Eigenvalues
let eigenvalues = m.eigenvalues();
```

### Solving Linear Systems

```titrate
// Solve Ax = b
let a = Matrix.fromArray([
    [2.0, 1.0],
    [5.0, 3.0]
]);
let b = NDArray.fromArray([4.0, 7.0], (2,));

let x = a.solve(b);
// x ≈ [5.0, -6.0]
```

## Chemistry

Beyond pure math, Titrate's `tt.chem` module brings computational chemistry directly into your code. You can define molecules, compute energies with force fields, run molecular dynamics, and even perform quantum chemistry calculations — all without leaving Titrate.

### Molecules

```titrate
let water = new Molecule();
water.addAtom(new Atom("O", 0.0, 0.0, 0.0));
water.addAtom(new Atom("H", 0.9572, 0.0, 0.0));
water.addAtom(new Atom("H", -0.2399, 0.9270, 0.0));
water.addBond(new Bond(0, 1, 1));  // O-H single bond
water.addBond(new Bond(0, 2, 1));  // O-H single bond

io::println(water.formula());     // H2O
io::println(water.mass().toString());  // 18.015
```

### Force Fields

```titrate
let ff = new ForceField();
ff.addBondTerm(0, 1, 450.0, 0.9572);  // k, r0
ff.addBondTerm(0, 2, 450.0, 0.9572);
ff.addAngleTerm(1, 0, 2, 55.0, 104.52);  // k, theta0

let energy = ff.energy(water);
io::println("Energy: " + energy.toString());
```

### Molecular Dynamics

```titrate
let integrator = new Integrator(1.0);  // 1.0 fs timestep
let md = new MD(water, ff, integrator);
md.run(1000);  // 1000 steps

let positions = md.positions();
for (i in positions) {
    io::println(Double.toString(i));
}
```

### Quantum Chemistry

```titrate
let rhf = new RHF(water);
rhf.setBasis("STO-3G");
let energy = rhf.compute();
io::println("SCF Energy: " + energy.toString());
```

## Units of Measure

If you've ever spent hours debugging a simulation only to find you were adding joules to electronvolts, you'll appreciate this module. The `tt.units` module enforces dimensional consistency at runtime, preventing errors like adding meters to seconds.

### Base Units

```titrate
let length = Base.meter(5.0);
let time = Base.second(2.0);
let mass = Base.kilogram(3.0);
```

### Derived Units

```titrate
let speed = length / time;           // 2.5 m/s
let force = mass * speed / time;    // 3.75 kg·m/s² (Newtons)
let energy = force * length;         // J (Joules)
```

### Dimensional Consistency

The unit system prevents mismatched operations:

```titrate
let a = Base.meter(5.0);
let b = Base.second(2.0);
// let bad = a + b;  // Runtime error: cannot add m and s

let c = Base.meter(3.0);
let sum = a + c;     // OK: 8.0 m
```

::: tip Catch unit errors early
The units module is your best friend in scientific code. Use it from the start of your project — retrofitting units later is much harder than building them in from the beginning.
:::

### Physical Constants

```titrate
let c = Constants.speedOfLight;   // 299792458 m/s
let h = Constants.planck;         // 6.62607015e-34 J·s
let k = Constants.boltzmann;      // 1.380649e-23 J/K
let na = Constants.avogadro;      // 6.02214076e23 /mol
```

### Converting Units

```titrate
let distance = Base.meter(1000.0);
let inKm = distance.to(Derived.kilometer);  // 1.0 km

let temp = Base.kelvin(300.0);
let inCelsius = temp.to(Derived.celsius);    // 26.85 °C
```

## Try It Yourself

Create an NDArray representing a 3×3 grid of temperatures in Kelvin, then convert them all to Celsius using the formula `C = K - 273.15`:

```titrate
public fn main(): void {
    // Create a 3x3 NDArray of temperatures in Kelvin
    let tempsK = NDArray.fromArray(
        [273.15, 300.0, 310.0, 280.0, 290.0, 305.0, 260.0, 275.0, 320.0],
        (3, 3)
    );

    // Subtract 273.15 from every element to get Celsius
    // (Hint: use arithmetic operators on NDArray)
    let tempsC = tempsK - 273.15;

    // Print the result
    io::println(tempsC.toString());
}
```

<details>
<summary>Show solution</summary>

```titrate
public fn main(): void {
    let tempsK = NDArray.fromArray(
        [273.15, 300.0, 310.0, 280.0, 290.0, 305.0, 260.0, 275.0, 320.0],
        (3, 3)
    );

    let tempsC = tempsK - 273.15;

    io::println(tempsC.toString());
    // [[0.0, 26.85, 36.85],
    //  [6.85, 16.85, 31.85],
    //  [-13.15, 1.85, 46.85]]
}
```

</details>

## What's Next?

- [Standard Library](./stdlib) — full module reference
- [Operator Overloading](./operator-overloading) — how NDArray and Matrix use operators
- [Iterators](./iterators) — traversing data structures
