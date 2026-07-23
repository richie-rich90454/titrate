# random

The `tt::random` module provides random number generation, probability distributions, and sampling utilities.

## Random

The core random number generator:

```titrate
import tt::random::Random;

// Seed the generator
Random.seed(12345);

// Generate random integers
let n: int = Random.nextInt(1, 100);      // 1 to 99
let n2: int = Random.nextInt(0, 10);      // 0 to 9

// Generate random doubles
let d: double = Random.nextDouble();       // 0.0 to 1.0
let d2: double = Random.nextDouble(0.0, 10.0);  // 0.0 to 10.0

// Random boolean
let b: bool = Random.nextBool();

// Random element from list
let colors: ArrayList<string> = new ArrayList<string>();
colors.add("red");
colors.add("green");
colors.add("blue");
let color: string = Random.choice(colors);
```

## Continuous Distributions

Continuous probability distributions:

```titrate
import tt::random::ContinuousDist;

// Normal (Gaussian) distribution
let normal: ContinuousDist = ContinuousDist.normal(0.0, 1.0);
let sample: double = normal.sample();

// Uniform distribution
let uniform: ContinuousDist = ContinuousDist.uniform(0.0, 1.0);

// Exponential distribution
let exp: ContinuousDist = ContinuousDist.exponential(1.0);

// Beta distribution
let beta: ContinuousDist = ContinuousDist.beta(2.0, 5.0);

// Gamma distribution
let gamma: ContinuousDist = ContinuousDist.gamma(2.0, 1.0);
```

## Discrete Distributions

Discrete probability distributions:

```titrate
import tt::random::DiscreteDist;

// Bernoulli distribution
let bern: DiscreteDist = DiscreteDist.bernoulli(0.7);
let trial: bool = bern.sample();

// Binomial distribution
let binom: DiscreteDist = DiscreteDist.binomial(10, 0.5);
let successes: int = binom.sample();

// Poisson distribution
let poisson: DiscreteDist = DiscreteDist.poisson(3.0);
let count: int = poisson.sample();

// Geometric distribution
let geom: DiscreteDist = DiscreteDist.geometric(0.3);
```

## Sampling

Advanced sampling techniques:

```titrate
import tt::random::Sampling;

// Simple random sample
let population: ArrayList<int> = new ArrayList<int>();
for (i in 0..100) {
    population.add(i);
}
let sample: ArrayList<int> = Sampling.simpleRandomSample(population, 10);

// Weighted sample
let weights: ArrayList<double> = new ArrayList<double>();
weights.add(0.5);
weights.add(0.3);
weights.add(0.2);
let weighted: ArrayList<int> = Sampling.weightedSample(population, weights, 5);
```

## Quasi-Random

Low-discrepancy sequences for deterministic sampling:

```titrate
import tt::random::QuasiRandom;

// Sobol sequence
let sobol: QuasiRandom = QuasiRandom.sobol(2);
let point: ArrayList<double> = sobol.next();

// Halton sequence
let halton: QuasiRandom = QuasiRandom.halton(2);
```

## Module Functions Reference

| Function | Description |
|----------|-------------|
| `Random.seed(s)` | Set random seed |
| `Random.nextInt(min, max)` | Random integer in range |
| `Random.nextDouble()` | Random double [0, 1) |
| `Random.nextBool()` | Random boolean |
| `Random.choice(list)` | Random element from list |
| `ContinuousDist.normal(mu, sigma)` | Normal distribution |
| `ContinuousDist.uniform(a, b)` | Uniform distribution |
| `DiscreteDist.bernoulli(p)` | Bernoulli distribution |
| `DiscreteDist.binomial(n, p)` | Binomial distribution |
| `DiscreteDist.poisson(lambda)` | Poisson distribution |
| `Sampling.simpleRandomSample(pop, n)` | Simple random sample |
| `QuasiRandom.sobol(dim)` | Sobol sequence |
