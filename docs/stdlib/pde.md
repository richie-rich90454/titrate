# pde

The `tt.math.pde.PDE` module provides finite difference solvers for partial differential equations, including heat, wave, Poisson, and Laplace equations.

```titrate
import tt.math.pde.PDE;
```

## BoundaryCondition

Boundary conditions for PDE solvers:

- `fn init(kind: string, value: double, coefficient: double)` — create a boundary condition

**Kinds:**
- `"dirichlet"` — fixed value: u = value
- `"neumann"` — fixed flux: du/dn = value
- `"robin"` — mixed: coefficient * u + du/dn = value

```titrate
let fixed: BoundaryCondition = new BoundaryCondition("dirichlet", 100.0, 0.0);
let insulated: BoundaryCondition = new BoundaryCondition("neumann", 0.0, 0.0);
let convective: BoundaryCondition = new BoundaryCondition("robin", 25.0, 0.1);
```

## heatEquation1D

- `heatEquation1D(nx: int, dx: double, dt: double, steps: int, initial: ArrayList<double>, leftBC: BoundaryCondition, rightBC: BoundaryCondition): ArrayList<ArrayList<double>>`

Solve the 1D heat equation ∂u/∂t = α ∂²u/∂x² using Forward Euler (explicit) method. Returns the solution at each time step.

Parameters:
- `nx` — number of spatial grid points
- `dx` — spatial step size
- `dt` — time step size (stability requires dt/(dx²) ≤ 0.5)
- `steps` — number of time steps
- `initial` — initial temperature distribution
- `leftBC` — left boundary condition
- `rightBC` — right boundary condition

```titrate
let nx: int = 50;
let dx: double = 0.02;
let dt: double = 0.0001;
let initial: ArrayList<double> = new ArrayList<double>();
// Set initial condition: 0 everywhere
var i: int = 0;
while (i < nx) {
    initial.add(0.0);
    i = i + 1;
}
initial.set(nx / 2, 100.0);  // hot spot in the middle

let leftBC: BoundaryCondition = new BoundaryCondition("dirichlet", 0.0, 0.0);
let rightBC: BoundaryCondition = new BoundaryCondition("dirichlet", 0.0, 0.0);

let solution: ArrayList<ArrayList<double>> = heatEquation1D(nx, dx, dt, 1000, initial, leftBC, rightBC);
```

## heatEquation1DImplicit

- `heatEquation1DImplicit(nx: int, dx: double, dt: double, steps: int, initial: ArrayList<double>, leftBC: BoundaryCondition, rightBC: BoundaryCondition): ArrayList<ArrayList<double>>`

Solve the 1D heat equation using Backward Euler (implicit) method. Unconditionally stable — no restriction on dt/(dx²). Solves a tridiagonal system at each time step using the Thomas algorithm.

Parameters are the same as `heatEquation1D`.

## waveEquation1D

- `waveEquation1D(nx: int, dx: double, dt: double, steps: int, initial: ArrayList<double>, initialVelocity: ArrayList<double>, leftBC: BoundaryCondition, rightBC: BoundaryCondition): ArrayList<ArrayList<double>>`

Solve the 1D wave equation ∂²u/∂t² = c² ∂²u/∂x² using central differences. Stability requires the Courant number c·dt/dx ≤ 1.

Parameters:
- `nx` — number of spatial grid points
- `dx` — spatial step size
- `dt` — time step size
- `steps` — number of time steps
- `initial` — initial displacement
- `initialVelocity` — initial velocity distribution
- `leftBC` — left boundary condition
- `rightBC` — right boundary condition

```titrate
let nx: int = 100;
let dx: double = 0.01;
let dt: double = 0.005;
// ... set initial displacement and velocity ...
let solution: ArrayList<ArrayList<double>> = waveEquation1D(nx, dx, dt, 500, initial, velocity, leftBC, rightBC);
```

## poisson2D

- `poisson2D(nx: int, ny: int, dx: double, dy: double, source: ArrayList<ArrayList<double>>, bc: ArrayList<BoundaryCondition>, maxIter: int, tol: double): ArrayList<ArrayList<double>>`

Solve the 2D Poisson equation ∂²u/∂x² + ∂²u/∂y² = f(x,y) using iterative Gauss-Seidel method.

Parameters:
- `nx`, `ny` — grid dimensions
- `dx`, `dy` — spatial step sizes
- `source` — source term f(x,y) on the grid
- `bc` — boundary conditions [left, right, top, bottom]
- `maxIter` — maximum Gauss-Seidel iterations
- `tol` — convergence tolerance on max change

```titrate
let nx: int = 50;
let ny: int = 50;
let dx: double = 0.02;
let dy: double = 0.02;

// Zero source term
let source: ArrayList<ArrayList<double>> = new ArrayList<ArrayList<double>>();
// ... initialize with zeros ...

let bc: ArrayList<BoundaryCondition> = new ArrayList<BoundaryCondition>();
bc.add(new BoundaryCondition("dirichlet", 0.0, 0.0));   // left
bc.add(new BoundaryCondition("dirichlet", 0.0, 0.0));   // right
bc.add(new BoundaryCondition("dirichlet", 100.0, 0.0)); // top
bc.add(new BoundaryCondition("dirichlet", 0.0, 0.0));   // bottom

let u: ArrayList<ArrayList<double>> = poisson2D(nx, ny, dx, dy, source, bc, 10000, 1e-6);
```

## laplace2D

- `laplace2D(nx: int, ny: int, dx: double, dy: double, bc: ArrayList<BoundaryCondition>, maxIter: int, tol: double): ArrayList<ArrayList<double>>`

Solve the 2D Laplace equation ∂²u/∂x² + ∂²u/∂y² = 0. Equivalent to `poisson2D` with a zero source term.

Parameters are the same as `poisson2D` without the `source` parameter.
