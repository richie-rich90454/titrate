# statistics

The `tt.statistics` module provides statistical functions for analyzing numerical data.

```titrate
import tt.statistics;
```

## Descriptive Statistics

### mean

- `statistics.mean(data: ArrayList<double>): double` — arithmetic mean (average)

```titrate
let avg = statistics.mean([1.0, 2.0, 3.0, 4.0, 5.0]);
io::println(Double.toString(avg));  // 3.0
```

### median

- `statistics.median(data: ArrayList<double>): double` — middle value when sorted

```titrate
let mid = statistics.median([1.0, 3.0, 5.0, 7.0, 9.0]);
io::println(Double.toString(mid));  // 5.0
```

### mode

- `statistics.mode(data: ArrayList<double>): double` — most frequently occurring value

```titrate
let m = statistics.mode([1.0, 2.0, 2.0, 3.0, 3.0, 3.0]);
io::println(Double.toString(m));  // 3.0
```

### variance

- `statistics.variance(data: ArrayList<double>): double` — population variance

```titrate
let v = statistics.variance([2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]);
io::println(Double.toString(v));  // 4.0
```

### pvariance

- `statistics.pvariance(data: ArrayList<double>): double` — population variance (explicit name)

### stdev

- `statistics.stdev(data: ArrayList<double>): double` — population standard deviation (square root of variance)

```titrate
let s = statistics.stdev([2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]);
io::println(Double.toString(s));  // 2.0
```

### pstdev

- `statistics.pstdev(data: ArrayList<double>): double` — population standard deviation (explicit name)

### sampleVariance

- `statistics.sampleVariance(data: ArrayList<double>): double` — sample variance (Bessel's correction, divides by N-1)

### sampleStdev

- `statistics.sampleStdev(data: ArrayList<double>): double` — sample standard deviation

## Spread and Range

### min

- `statistics.min(data: ArrayList<double>): double` — minimum value

### max

- `statistics.max(data: ArrayList<double>): double` — maximum value

### range

- `statistics.range(data: ArrayList<double>): double` — difference between max and min

```titrate
let r = statistics.range([1.0, 5.0, 3.0, 9.0, 2.0]);
io::println(Double.toString(r));  // 8.0
```

## Quantiles

### quantile

- `statistics.quantile(data: ArrayList<double>, q: double): double` — value at the given quantile (0.0–1.0)

```titrate
let q1 = statistics.quantile([1.0, 2.0, 3.0, 4.0, 5.0], 0.25);  // first quartile
```

### quartiles

- `statistics.quartiles(data: ArrayList<double>): (double, double, double)` — returns (Q1, Q2, Q3)

```titrate
let (q1, q2, q3) = statistics.quartiles([1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
```

### iqr

- `statistics.iqr(data: ArrayList<double>): double` — interquartile range (Q3 - Q1)

## Correlation and Regression

### correlation

- `statistics.correlation(x: ArrayList<double>, y: ArrayList<double>): double` — Pearson correlation coefficient (-1.0 to 1.0)

```titrate
let r = statistics.correlation([1.0, 2.0, 3.0], [2.0, 4.0, 6.0]);
io::println(Double.toString(r));  // 1.0 (perfect positive correlation)
```

### covariance

- `statistics.covariance(x: ArrayList<double>, y: ArrayList<double>): double` — population covariance

### linearRegression

- `statistics.linearRegression(x: ArrayList<double>, y: ArrayList<double>): (double, double)` — returns (slope, intercept) of best-fit line

```titrate
let (slope, intercept) = statistics.linearRegression([1.0, 2.0, 3.0], [2.0, 4.0, 6.0]);
// slope ≈ 2.0, intercept ≈ 0.0
```

### geometricMean

- `statistics.geometricMean(data: ArrayList<double>): double` — geometric mean (n-th root of the product)

```titrate
let gm = statistics.geometricMean([1.0, 2.0, 4.0, 8.0]);  // ≈ 2.828
```

### harmonicMean

- `statistics.harmonicMean(data: ArrayList<double>): double` — harmonic mean (n / Σ(1/xᵢ))

```titrate
let hm = statistics.harmonicMean([1.0, 2.0, 4.0]);  // ≈ 1.714
```

## Summary

### describe

- `statistics.describe(data: ArrayList<double>): Summary` — compute a statistical summary

```titrate
let summary = statistics.describe([1.0, 2.0, 3.0, 4.0, 5.0]);
io::println(Double.toString(summary.mean()));    // 3.0
io::println(Double.toString(summary.stdev()));   // 1.414...
io::println(Double.toString(summary.min()));     // 1.0
io::println(Double.toString(summary.max()));     // 5.0
```

### Summary

- `.count(): int` — number of data points
- `.mean(): double` — arithmetic mean
- `.stdev(): double` — standard deviation
- `.min(): double` — minimum
- `.max(): double` — maximum
- `.median(): double` — median
- `.q1(): double` — first quartile
- `.q3(): double` — third quartile

## Hypothesis Testing

- `Statistics.tTest(sample: ArrayList<double>, mu0: double): double` — one-sample t-test p-value
- `Statistics.twoSampleTTest(a: ArrayList<double>, b: ArrayList<double>): double` — two-sample t-test
- `Statistics.pairedTTest(before: ArrayList<double>, after: ArrayList<double>): double` — paired t-test
- `Statistics.chiSquaredTest(observed: ArrayList<double>, expected: ArrayList<double>): double` — chi-squared test
- `Statistics.kolmogorovSmirnovTest(sample: ArrayList<double>, cdf: fn(double): double): double` — K-S test
- `Statistics.mannWhitneyUTest(a: ArrayList<double>, b: ArrayList<double>): double` — Mann-Whitney U test
- `Statistics.wilcoxonTest(before: ArrayList<double>, after: ArrayList<double>): double` — Wilcoxon signed-rank test

## ANOVA

- `Statistics.oneWayANOVA(groups: ArrayList<ArrayList<double>>): double` — F-statistic
- `Statistics.twoWayANOVA(data: ArrayList<ArrayList<double>>, factorA: int, factorB: int): HashMap<string, double>` — two-way ANOVA
- `Statistics.tukeyHSD(groups: ArrayList<ArrayList<double>>): ArrayList<double>` — Tukey HSD post-hoc
- `Statistics.bonferroniCorrection(pValues: ArrayList<double>, alpha: double): ArrayList<bool>` — Bonferroni correction

## Bayesian Statistics

- `Statistics.betaBinomialPosterior(alpha: double, beta: double, successes: int, trials: int): (double, double)` — Beta-Binomial posterior
- `Statistics.normalNormalPosterior(priorMean: double, priorVar: double, sampleMean: double, sampleVar: double, n: int): (double, double)` — Normal-Normal posterior
- `Statistics.gammaPoissonPosterior(alpha: double, beta: double, count: int): (double, double)` — Gamma-Poisson posterior
- `Statistics.credibleInterval(alpha: double, beta: double, level: double): (double, double)` — credible interval

## MCMC

- `Mcmc.metropolisHastings(target: fn(double): double, proposal: fn(double): double, initial: double, iterations: int): ArrayList<double>` — Metropolis-Hastings sampling
- `Mcmc.gibbsSampler(conditionals: ArrayList<fn(ArrayList<double>): double>, initial: ArrayList<double>, iterations: int): ArrayList<ArrayList<double>>` — Gibbs sampling
- `Mcmc.rhat(chains: ArrayList<ArrayList<double>>): double` — R-hat convergence diagnostic
- `Mcmc.effectiveSampleSize(samples: ArrayList<double>): int` — ESS
- `Mcmc.autocorrelation(samples: ArrayList<double>, lag: int): double` — autocorrelation at lag

## Kernel Density Estimation

- `Kde.kdeGaussian(data: ArrayList<double>, x: double, bandwidth: double): double` — Gaussian KDE
- `Kde.silvermanBandwidth(data: ArrayList<double>): double` — Silverman bandwidth
- `Kde.scottBandwidth(data: ArrayList<double>): double` — Scott bandwidth
- `Kde.kdeEvaluate(data: ArrayList<double>, points: ArrayList<double>, bandwidth: double): ArrayList<double>` — evaluate KDE at points
- `Kde.kdeGrid(data: ArrayList<double>, lo: double, hi: double, n: int, bandwidth: double): ArrayList<double>` — KDE on grid

## Bootstrap

- `Bootstrap.bootstrapSample(data: ArrayList<double>): ArrayList<double>` — resample with replacement
- `Bootstrap.bootstrapCI(data: ArrayList<double>, statistic: fn(ArrayList<double>): double, confidence: double, iterations: int): (double, double)` — bootstrap confidence interval
- `Bootstrap.bootstrapBCa(data: ArrayList<double>, statistic: fn(ArrayList<double>): double, confidence: double, iterations: int): (double, double)` — BCa confidence interval
- `Bootstrap.bootstrapTest(sample1: ArrayList<double>, sample2: ArrayList<double>, iterations: int): double` — bootstrap hypothesis test

## Time Series

- `TimeSeries.acf(data: ArrayList<double>, maxLag: int): ArrayList<double>` — autocorrelation function
- `TimeSeries.pacf(data: ArrayList<double>, maxLag: int): ArrayList<double>` — partial autocorrelation
- `TimeSeries.arimaFit(data: ArrayList<double>, p: int, d: int, q: int): ArrayList<double>` — ARIMA parameters
- `TimeSeries.arimaForecast(params: ArrayList<double>, data: ArrayList<double>, steps: int): ArrayList<double>` — ARIMA forecast
- `TimeSeries.exponentialSmoothing(data: ArrayList<double>, alpha: double): ArrayList<double>` — simple exponential smoothing
- `TimeSeries.holtSmoothing(data: ArrayList<double>, alpha: double, beta: double): ArrayList<double>` — Holt's linear trend
- `TimeSeries.holtWinters(data: ArrayList<double>, alpha: double, beta: double, gamma: double, period: int): ArrayList<double>` — Holt-Winters
- `TimeSeries.seasonalDecompose(data: ArrayList<double>, period: int): HashMap<string, ArrayList<double>>` — seasonal decomposition

## Survival Analysis

- `Survival.kaplanMeier(times: ArrayList<double>, events: ArrayList<bool>): ArrayList<double>` — Kaplan-Meier estimator
- `Survival.logRankTest(group1Times: ArrayList<double>, group1Events: ArrayList<bool>, group2Times: ArrayList<double>, group2Events: ArrayList<bool>): double` — log-rank test p-value
- `Survival.hazardFunction(times: ArrayList<double>, events: ArrayList<bool>): ArrayList<double>` — hazard function
- `Survival.medianSurvival(kmEstimate: ArrayList<double>): double` — median survival time
- `Survival.coxRegression(times: ArrayList<double>, events: ArrayList<bool>, covariates: ArrayList<ArrayList<double>>): ArrayList<double>` — Cox proportional hazards
