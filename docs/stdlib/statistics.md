# statistics

The `tt.statistics` module provides statistical functions for analyzing numerical data.

```titrate
import tt.statistics;
```

## Descriptive Statistics

### mean

- `statistics::mean(data: array<double>): double` — arithmetic mean (average)

```titrate
let avg = statistics::mean([1.0, 2.0, 3.0, 4.0, 5.0]);
io::println(avg.toString());  // 3.0
```

### median

- `statistics::median(data: array<double>): double` — middle value when sorted

```titrate
let mid = statistics::median([1.0, 3.0, 5.0, 7.0, 9.0]);
io::println(mid.toString());  // 5.0
```

### mode

- `statistics::mode(data: array<double>): double` — most frequently occurring value

```titrate
let m = statistics::mode([1.0, 2.0, 2.0, 3.0, 3.0, 3.0]);
io::println(m.toString());  // 3.0
```

### variance

- `statistics::variance(data: array<double>): double` — population variance

```titrate
let v = statistics::variance([2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]);
io::println(v.toString());  // 4.0
```

### pvariance

- `statistics::pvariance(data: array<double>): double` — population variance (explicit name)

### stdev

- `statistics::stdev(data: array<double>): double` — population standard deviation (square root of variance)

```titrate
let s = statistics::stdev([2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]);
io::println(s.toString());  // 2.0
```

### pstdev

- `statistics::pstdev(data: array<double>): double` — population standard deviation (explicit name)

### sampleVariance

- `statistics::sampleVariance(data: array<double>): double` — sample variance (Bessel's correction, divides by N-1)

### sampleStdev

- `statistics::sampleStdev(data: array<double>): double` — sample standard deviation

## Spread and Range

### min

- `statistics::min(data: array<double>): double` — minimum value

### max

- `statistics::max(data: array<double>): double` — maximum value

### range

- `statistics::range(data: array<double>): double` — difference between max and min

```titrate
let r = statistics::range([1.0, 5.0, 3.0, 9.0, 2.0]);
io::println(r.toString());  // 8.0
```

## Quantiles

### quantile

- `statistics::quantile(data: array<double>, q: double): double` — value at the given quantile (0.0–1.0)

```titrate
let q1 = statistics::quantile([1.0, 2.0, 3.0, 4.0, 5.0], 0.25);  // first quartile
```

### quartiles

- `statistics::quartiles(data: array<double>): (double, double, double)` — returns (Q1, Q2, Q3)

```titrate
let (q1, q2, q3) = statistics::quartiles([1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
```

### iqr

- `statistics::iqr(data: array<double>): double` — interquartile range (Q3 - Q1)

## Correlation and Regression

### correlation

- `statistics::correlation(x: array<double>, y: array<double>): double` — Pearson correlation coefficient (-1.0 to 1.0)

```titrate
let r = statistics::correlation([1.0, 2.0, 3.0], [2.0, 4.0, 6.0]);
io::println(r.toString());  // 1.0 (perfect positive correlation)
```

### covariance

- `statistics::covariance(x: array<double>, y: array<double>): double` — population covariance

### linearRegression

- `statistics::linearRegression(x: array<double>, y: array<double>): (double, double)` — returns (slope, intercept) of best-fit line

```titrate
let (slope, intercept) = statistics::linearRegression([1.0, 2.0, 3.0], [2.0, 4.0, 6.0]);
// slope ≈ 2.0, intercept ≈ 0.0
```

## Summary

### describe

- `statistics::describe(data: array<double>): Summary` — compute a statistical summary

```titrate
let summary = statistics::describe([1.0, 2.0, 3.0, 4.0, 5.0]);
io::println(summary.mean().toString());    // 3.0
io::println(summary.stdev().toString());   // 1.414...
io::println(summary.min().toString());     // 1.0
io::println(summary.max().toString());     // 5.0
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
