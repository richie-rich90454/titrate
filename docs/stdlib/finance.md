# finance

The `tt.finance` module provides high-frequency trading (HFT) data structures, technical indicators, and risk metrics.

```titrate
import tt.finance.OrderBook;
import tt.finance.MarketData;
import tt.finance.Indicators;
import tt.finance.Risk;
```

## OrderBook

A limit order book with price-level aggregation for bids and asks.

### PriceLevel

- `price: double` — price at this level
- `quantity: double` — total quantity at this level
- `orderCount: int` — number of orders at this level

### Order

- `id: string` — order identifier
- `side: string` — "bid" or "ask"
- `price: double` — limit price
- `quantity: double` — order quantity
- `timestamp: double` — order timestamp

### OrderBook Methods

- `fn init()` — create an empty order book
- `addBid(order: Order): void` — add a bid order
- `addAsk(order: Order): void` — add an ask order
- `cancelOrder(id: string): bool` — cancel an order by ID; returns true if found
- `bestBid(): double` — highest bid price
- `bestAsk(): double` — lowest ask price
- `spread(): double` — best ask minus best bid
- `midPrice(): double` — midpoint between best bid and ask
- `bidDepth(levels: int): ArrayList<PriceLevel>` — top N bid levels (sorted descending)
- `askDepth(levels: int): ArrayList<PriceLevel>` — top N ask levels (sorted ascending)
- `bbo(): string` — best bid/offer as a string

```titrate
let book: OrderBook = new OrderBook();
book.addBid(new Order("b1", "bid", 100.50, 10.0, 0.0));
book.addBid(new Order("b2", "bid", 100.25, 5.0, 0.0));
book.addAsk(new Order("a1", "ask", 100.75, 8.0, 0.0));

let spread: double = book.spread();       // 0.25
let mid: double = book.midPrice();        // 100.625
let depth: ArrayList<PriceLevel> = book.bidDepth(5);
```

## MarketData

Market data structures for OHLCV bars, ticks, trades, and quotes.

### OHLCV

Open-High-Low-Close-Volume bar.

- `open: double`, `high: double`, `low: double`, `close: double`, `volume: double`, `timestamp: double`

```titrate
let bar: OHLCV = new OHLCV(100.0, 105.0, 98.0, 103.0, 10000.0, 1700000000.0);
```

### Tick

Individual price update.

- `price: double`, `quantity: double`, `timestamp: double`, `side: string`

### Trade

Executed trade record.

- `id: string`, `price: double`, `quantity: double`, `timestamp: double`, `side: string`, `orderId: string`

### Quote

Bid/ask quote with sizes.

- `bidPrice: double`, `bidSize: double`, `askPrice: double`, `askSize: double`, `timestamp: double`
- `spread(): double` — ask minus bid
- `midPrice(): double` — midpoint

```titrate
let q: Quote = new Quote(100.0, 500.0, 100.10, 300.0, 0.0);
let s: double = q.spread();      // 0.10
let m: double = q.midPrice();    // 100.05
```

## Indicators

Technical analysis indicators for time-series data.

### sma

- `sma(data: ArrayList<double>, period: int): ArrayList<double>` — Simple Moving Average

```titrate
let prices: ArrayList<double> = new ArrayList<double>();
// ... add price data ...
let movingAvg: ArrayList<double> = sma(prices, 20);
```

### ema

- `ema(data: ArrayList<double>, period: int): ArrayList<double>` — Exponential Moving Average (multiplier = 2/(period+1))

### ewma

- `ewma(data: ArrayList<double>, alpha: double): ArrayList<double>` — Exponentially Weighted Moving Average with custom smoothing factor

### rsi

- `rsi(data: ArrayList<double>, period: int): ArrayList<double>` — Relative Strength Index (0–100)

```titrate
let rsiValues: ArrayList<double> = rsi(prices, 14);
// RSI > 70: overbought, RSI < 30: oversold
```

### macd

- `macd(data: ArrayList<double>, fastPeriod: int, slowPeriod: int, signalPeriod: int): ArrayList<ArrayList<double>>` — MACD indicator

Returns three arrays: [MACD line, signal line, histogram].

```titrate
let result: ArrayList<ArrayList<double>> = macd(prices, 12, 26, 9);
let macdLine: ArrayList<double> = result.get(0);
let signalLine: ArrayList<double> = result.get(1);
let histogram: ArrayList<double> = result.get(2);
```

### bollingerBands

- `bollingerBands(data: ArrayList<double>, period: int, numStdDev: double): ArrayList<ArrayList<double>>` — Bollinger Bands

Returns three arrays: [upper band, middle band (SMA), lower band].

```titrate
let bands: ArrayList<ArrayList<double>> = bollingerBands(prices, 20, 2.0);
let upper: ArrayList<double> = bands.get(0);
let middle: ArrayList<double> = bands.get(1);
let lower: ArrayList<double> = bands.get(2);
```

### vwap

- `vwap(prices: ArrayList<double>, volumes: ArrayList<double>): ArrayList<double>` — Volume Weighted Average Price

```titrate
let vwapValues: ArrayList<double> = vwap(prices, volumes);
```

## Risk

Risk metrics for portfolio analysis and HFT.

### valueAtRisk

- `valueAtRisk(returns: ArrayList<double>, confidence: double): double` — Historical Value at Risk at the given confidence level (e.g., 0.95). Returns the loss as a positive number.

### conditionalVaR

- `conditionalVaR(returns: ArrayList<double>, confidence: double): double` — Conditional VaR (Expected Shortfall). Average loss beyond VaR.

### sharpeRatio

- `sharpeRatio(returns: ArrayList<double>, riskFreeRate: double): double` — Sharpe ratio: (mean return − risk-free rate) / standard deviation

### sortinoRatio

- `sortinoRatio(returns: ArrayList<double>, riskFreeRate: double): double` — Sortino ratio using downside deviation instead of total standard deviation

### maxDrawdown

- `maxDrawdown(equityCurve: ArrayList<double>): double` — Maximum drawdown as a fraction (0.0 to 1.0)

```titrate
let equity: ArrayList<double> = new ArrayList<double>();
equity.add(100.0); equity.add(110.0); equity.add(95.0); equity.add(105.0);
let mdd: double = maxDrawdown(equity);  // ≈ 0.136
```

### kellyCriterion

- `kellyCriterion(winRate: double, avgWin: double, avgLoss: double): double` — Optimal fraction to bet according to the Kelly criterion

### annualizedVolatility

- `annualizedVolatility(returns: ArrayList<double>, periodsPerYear: int): double` — Annualized volatility (stddev × √periods)

### beta

- `beta(returns: ArrayList<double>, marketReturns: ArrayList<double>): double` — Portfolio beta relative to market returns

```titrate
let var95: double = valueAtRisk(returns, 0.95);
let cvar95: double = conditionalVaR(returns, 0.95);
let sharpe: double = sharpeRatio(returns, 0.02);
let sortino: double = sortinoRatio(returns, 0.02);
let kelly: double = kellyCriterion(0.55, 1.5, 1.0);  // ≈ 0.183
```

## BlackScholes

- `BlackScholes.callPrice(s: double, k: double, t: double, r: double, sigma: double): double` — European call option price
- `BlackScholes.putPrice(s: double, k: double, t: double, r: double, sigma: double): double` — European put option price
- `BlackScholes.impliedVolatility(price: double, s: double, k: double, t: double, r: double, isCall: bool): double` — implied volatility
- `BlackScholes.delta(s: double, k: double, t: double, r: double, sigma: double): double` — delta
- `BlackScholes.gamma(s: double, k: double, t: double, r: double, sigma: double): double` — gamma
- `BlackScholes.theta(s: double, k: double, t: double, r: double, sigma: double): double` — theta
- `BlackScholes.vega(s: double, k: double, t: double, r: double, sigma: double): double` — vega
- `BlackScholes.rho(s: double, k: double, t: double, r: double, sigma: double): double` — rho

## BinomialTree

- `BinomialTree.crrPrice(s: double, k: double, t: double, r: double, sigma: double, steps: int, isCall: bool): double` — CRR binomial tree pricing
- `BinomialTree.crrGreeks(s: double, k: double, t: double, r: double, sigma: double, steps: int): HashMap<string, double>` — Greeks from binomial tree

## MonteCarloPricing

- `MonteCarloPricing.gbmPath(s0: double, mu: double, sigma: double, t: double, steps: int): ArrayList<double>` — geometric Brownian motion path
- `MonteCarloPricing.monteCarloCall(s: double, k: double, t: double, r: double, sigma: double, paths: int): double` — Monte Carlo call pricing
- `MonteCarloPricing.monteCarloPut(s: double, k: double, t: double, r: double, sigma: double, paths: int): double` — Monte Carlo put pricing
- `MonteCarloPricing.monteCarloAsian(s: double, k: double, t: double, r: double, sigma: double, paths: int, steps: int): double` — Asian option pricing
- `MonteCarloPricing.antitheticVariateCall(s: double, k: double, t: double, r: double, sigma: double, paths: int): double` — variance reduction
- `MonteCarloPricing.controlVariateCall(s: double, k: double, t: double, r: double, sigma: double, paths: int): double` — control variate method

## YieldCurve

- `YieldCurve.nelsonSiegel(beta0: double, beta1: double, beta2: double, tau: double, t: double): double` — Nelson-Siegel model
- `YieldCurve.svensson(beta0: double, beta1: double, beta2: double, beta3: double, tau1: double, tau2: double, t: double): double` — Svensson model
- `YieldCurve.cubicSplineInterpolate(points: ArrayList<double>, values: ArrayList<double>, x: double): double` — cubic spline interpolation
- `YieldCurve.bootstrapYieldCurve(bonds: ArrayList<HashMap<string, double>>): ArrayList<double>` — bootstrap yield curve
- `YieldCurve.forwardRate(spotRates: ArrayList<double>, t1: int, t2: int): double` — forward rate

## Portfolio

- `Portfolio.meanVarianceOptimize(returns: ArrayList<ArrayList<double>>, targetReturn: double): ArrayList<double>` — Markowitz optimization
- `Portfolio.efficientFrontier(returns: ArrayList<ArrayList<double>>, points: int): ArrayList<HashMap<string, double>>` — efficient frontier
- `Portfolio.blackLitterman(marketWeights: ArrayList<double>, covMatrix: ArrayList<ArrayList<double>>, views: ArrayList<double>, tau: double): ArrayList<double>` — Black-Litterman
- `Portfolio.riskParity(covMatrix: ArrayList<ArrayList<double>>): ArrayList<double>` — risk parity allocation
- `Portfolio.sharpeRatio(returns: ArrayList<double>, riskFreeRate: double): double` — Sharpe ratio

## FactorModel

- `FactorModel.capm(returns: ArrayList<double>, marketReturns: ArrayList<double>, riskFreeRate: double): HashMap<string, double>` — CAPM alpha/beta
- `FactorModel.famaFrench3(returns: ArrayList<double>, market: ArrayList<double>, smb: ArrayList<double>, hml: ArrayList<double>): HashMap<string, double>` — Fama-French three-factor
- `FactorModel.famaFrench5(returns: ArrayList<double>, market: ArrayList<double>, smb: ArrayList<double>, hml: ArrayList<double>, rmw: ArrayList<double>, cma: ArrayList<double>): HashMap<string, double>` — Fama-French five-factor
- `FactorModel.factorExposure(returns: ArrayList<double>, factors: ArrayList<ArrayList<double>>): ArrayList<double>` — factor exposure
- `FactorModel.alphaEstimate(returns: ArrayList<double>, factorReturns: ArrayList<ArrayList<double>>): double` — alpha estimate
