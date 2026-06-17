# optimization

The `tt.math.optimization.Optimization` module provides numerical optimization algorithms for minimizing functions.

```titrate
import tt.math.optimization.Optimization;
```

## OptResult

All optimization functions return an `OptResult`:

- `x: ArrayList<double>` — the optimal point found
- `value: double` — function value at the optimal point
- `iterations: int` — number of iterations performed
- `converged: bool` — whether the algorithm converged within tolerance

## gradientDescent

- `gradientDescent(f: fn(ArrayList<double>): double, gradF: fn(ArrayList<double>): ArrayList<double>, x0: ArrayList<double>, lr: double, maxIter: int, tol: double): OptResult`

Standard gradient descent with a fixed learning rate.

Parameters:
- `f` — objective function
- `gradF` — gradient function
- `x0` — initial point
- `lr` — learning rate (step size)
- `maxIter` — maximum iterations
- `tol` — convergence tolerance on gradient norm

```titrate
// Minimize f(x) = (x-3)^2 + (y-1)^2
let result: OptResult = gradientDescent(
    fn(x: ArrayList<double>): double => {
        return (x.get(0) - 3.0) * (x.get(0) - 3.0) + (x.get(1) - 1.0) * (x.get(1) - 1.0);
    },
    fn(x: ArrayList<double>): ArrayList<double> => {
        let g = new ArrayList<double>();
        g.add(2.0 * (x.get(0) - 3.0));
        g.add(2.0 * (x.get(1) - 1.0));
        return g;
    },
    [0.0, 0.0], 0.1, 1000, 1e-8
);
// result.x ≈ [3.0, 1.0]
```

## stochasticGradientDescent

- `stochasticGradientDescent(f: fn(ArrayList<double>): double, gradF: fn(ArrayList<double>): ArrayList<double>, x0: ArrayList<double>, lr: double, maxIter: int, tol: double): OptResult`

SGD with learning rate decay (lr / (1 + 0.01 * iteration)).

Parameters are the same as `gradientDescent`.

## lbfgs

- `lbfgs(f: fn(ArrayList<double>): double, gradF: fn(ArrayList<double>): ArrayList<double>, x0: ArrayList<double>, maxIter: int, tol: double, m: int): OptResult`

L-BFGS quasi-Newton method with backtracking line search. Efficient for high-dimensional problems.

Parameters:
- `f` — objective function
- `gradF` — gradient function
- `x0` — initial point
- `maxIter` — maximum iterations
- `tol` — convergence tolerance on gradient norm
- `m` — number of correction pairs to store (typically 5–20)

```titrate
let result: OptResult = lbfgs(f, gradF, x0, 500, 1e-10, 10);
```

## nelderMead

- `nelderMead(f: fn(ArrayList<double>): double, x0: ArrayList<double>, maxIter: int, tol: double): OptResult`

Nelder-Mead simplex method. Derivative-free — no gradient function needed.

Parameters:
- `f` — objective function
- `x0` — initial point
- `maxIter` — maximum iterations
- `tol` — convergence tolerance on function value spread

```titrate
let result: OptResult = nelderMead(
    fn(x: ArrayList<double>): double => {
        return (x.get(0) - 2.0) * (x.get(0) - 2.0) + (x.get(1) + 1.0) * (x.get(1) + 1.0);
    },
    [0.0, 0.0], 1000, 1e-10
);
// result.x ≈ [2.0, -1.0]
```

## conjugateGradient

- `conjugateGradient(f: fn(ArrayList<double>): double, gradF: fn(ArrayList<double>): ArrayList<double>, x0: ArrayList<double>, maxIter: int, tol: double): OptResult`

Nonlinear conjugate gradient with Fletcher-Reeves beta and backtracking line search.

Parameters:
- `f` — objective function
- `gradF` — gradient function
- `x0` — initial point
- `maxIter` — maximum iterations
- `tol` — convergence tolerance on gradient norm

## linearProgramming

- `linearProgramming(c: ArrayList<double>, a: ArrayList<ArrayList<double>>, b: ArrayList<double>, maxIter: int): OptResult`

Simplex method for linear programming: minimize c^T x subject to Ax ≤ b, x ≥ 0.

Parameters:
- `c` — objective function coefficients
- `a` — constraint matrix (m × n)
- `b` — constraint right-hand side (m entries)
- `maxIter` — maximum pivot iterations

```titrate
// Minimize 3x + 5y subject to x + y <= 4, x + 3y <= 6, x >= 0, y >= 0
let c: ArrayList<double> = new ArrayList<double>();
c.add(3.0); c.add(5.0);

let a: ArrayList<ArrayList<double>> = new ArrayList<ArrayList<double>>();
let r1: ArrayList<double> = new ArrayList<double>(); r1.add(1.0); r1.add(1.0);
let r2: ArrayList<double> = new ArrayList<double>(); r2.add(1.0); r2.add(3.0);
a.add(r1); a.add(r2);

let b: ArrayList<double> = new ArrayList<double>();
b.add(4.0); b.add(6.0);

let result: OptResult = linearProgramming(c, a, b, 1000);
```

## Simulated Annealing

- `SimulatedAnnealing.optimize(f: fn(ArrayList<double>): double, initial: ArrayList<double>, initialTemp: double, coolingRate: double, iterations: int): ArrayList<double>` — simulated annealing

## Particle Swarm Optimization

- `PSO.optimize(f: fn(ArrayList<double>): double, bounds: ArrayList<(double, double)>, swarmSize: int, iterations: int): ArrayList<double>` — PSO

## Genetic Algorithm

- `GeneticAlgorithm.optimize(f: fn(ArrayList<double>): double, bounds: ArrayList<(double, double)>, populationSize: int, generations: int, mutationRate: double, crossoverRate: double): ArrayList<double>` — genetic algorithm

## Bayesian Optimization

- `BayesianOptimization.optimize(f: fn(ArrayList<double>): double, bounds: ArrayList<(double, double)>, iterations: int): ArrayList<double>` — Bayesian optimization with Gaussian process surrogate
