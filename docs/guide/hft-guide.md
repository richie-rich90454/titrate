# High-Frequency Trading with Titrate

Titrate's `tt.hft` module provides purpose-built primitives for building low-latency trading systems — from FIX protocol parsing to smart order routing, risk management, and backtesting. This guide walks through each component and ties them together in a complete market-making example.

## Getting Started with FIX Protocol

The FIX (Financial Information eXchange) protocol is the lingua franca of electronic trading. The `FixParser` class handles parsing and building FIX messages, which are structured as tag-value pairs separated by the SOH (`0x01`) delimiter.

### Parsing FIX Messages

```titrate
import tt::hft::FixParser;

public fn main(): void {
    let parser = new FixParser();

    // Parse a raw FIX message (SOH shown as | for readability)
    let raw = "8=FIX.4.4|9=112|35=D|49=SENDER|56=TARGET|11=ORD001|55=AAPL|54=1|44=150.25|38=100|10=123|";
    let msg = parser.parse(raw);

    // Access fields by tag number
    let msgType = msg.getString(35);     // "D" (New Order Single)
    let symbol = msg.getString(55);      // "AAPL"
    let side = msg.getInt(54);           // 1 (Buy)
    let price = msg.getDouble(44);       // 150.25
    let qty = msg.getInt(38);            // 100

    io::println("Type: " + msgType + " Symbol: " + symbol);
}
```

### Building FIX Messages

```titrate
let parser = new FixParser();

// Build a New Order Single
let msg = parser.createMessage();
msg.setString(8, "FIX.4.4");     // BeginString
msg.setString(35, "D");          // MsgType: New Order Single
msg.setString(49, "SENDER");     // SenderCompID
msg.setString(56, "TARGET");     // TargetCompID
msg.setString(11, "ORD001");     // ClOrdID
msg.setString(55, "AAPL");       // Symbol
msg.setInt(54, 1);               // Side: Buy
msg.setDouble(44, 150.25);       // Price
msg.setInt(38, 100);             // OrderQty

// Compute and set checksum (tag 10)
msg.computeChecksum();

let raw = msg.serialize();
```

### Checksum Validation

FIX messages include a three-digit checksum at tag 10. `FixParser` validates this automatically on `parse()`, but you can also verify manually:

```titrate
let msg = parser.parse(raw);

if (msg.validateChecksum()) {
    io::println("Checksum valid");
} else {
    io::println("Checksum INVALID — message corrupted");
}
```

## Smart Order Routing

The `OrderRouter` class manages order lifecycle across multiple venues, selecting the best execution destination based on configurable strategies.

### Venue Selection

```titrate
import tt::hft::OrderRouter;

let router = new OrderRouter();

// Register venues
router.addVenue("NYSE", 0.0005);    // venue name, fee per share
router.addVenue("NASDAQ", 0.0003);
router.addVenue("BATS", 0.0002);

// Set routing strategy
router.setStrategy("BEST_PRICE");  // or "LOWEST_FEE", "TWAP", "VWAP"
```

### Order Type Management

```titrate
// Submit a limit order
let orderId = router.submitOrder("AAPL", 1, 100, 150.25, "LIMIT", "NYSE");
//                      symbol, side, qty, price,  type,   venue

// Submit a market order
let mktId = router.submitOrder("MSFT", 2, 200, 0.0, "MARKET", "NASDAQ");
//                       side=2 (Sell), qty=200

// Cancel an order
router.cancelOrder(orderId);

// Replace (modify) an order
router.replaceOrder(orderId, 150.50, 150);
//                        new price, new qty
```

### Fill Tracking

```titrate
// After receiving an execution report from the venue
router.recordFill(orderId, 50, 150.25);  // partial fill: 50 shares @ 150.25

// Query fill state
let fills = router.getFills(orderId);
let totalFilled = router.totalFilledQty(orderId);
let avgPrice = router.averageFillPrice(orderId);
let isComplete = router.isFilled(orderId);

io::println("Filled: " + Integer.toString(totalFilled) + " @ " + Double.toString(avgPrice));
```

## Risk Management

The `RiskManager` class enforces pre-trade and post-trade risk controls. It acts as a gatekeeper — every order must pass risk checks before being routed.

### Position Limits

```titrate
import tt::hft::RiskManager;

let risk = new RiskManager();

// Set maximum position per symbol
risk.setMaxPosition("AAPL", 10000);    // max 10,000 shares
risk.setMaxPosition("MSFT", 5000);

// Set maximum order size
risk.setMaxOrderSize(500);             // no single order > 500 shares
```

### Rate Limits

```titrate
// Limit order submission rate
risk.setMaxOrdersPerSecond(50);        // max 50 orders/sec
risk.setMaxMessagesPerSecond(200);     // max 200 messages/sec
```

### Kill Switch and Max Notional

```titrate
// Set maximum notional value per order
risk.setMaxNotional(500000.0);         // $500K per order

// Kill switch — immediately halt all trading
risk.activateKillSwitch();             // blocks all new orders
// ... investigate issue ...
risk.deactivateKillSwitch();           // resume trading

// Check if trading is allowed
if (risk.isTradingAllowed()) {
    router.submitOrder("AAPL", 1, 100, 150.0, "LIMIT", "NYSE");
}
```

### Pre-Trade Risk Check

```titrate
// Validate an order before submission
let check = risk.checkOrder("AAPL", 1, 100, 150.25);
if (check.isOk()) {
    router.submitOrder("AAPL", 1, 100, 150.25, "LIMIT", "NYSE");
} else {
    io::println("Risk check failed: " + check.unwrapErr());
}
```

## Backtesting Strategies

The `Backtest` class provides an event-driven backtesting engine that replays historical data through your strategy, tracking fills with configurable slippage and commission models.

### Event-Driven Backtesting

```titrate
import tt::hft::Backtest;

public class SimpleStrategy {
    public fn onBar(bt: Backtest, bar: Bar): void {
        let mid = (bar.bid + bar.ask) / 2.0;
        if (bar.close > bar.open) {
            bt.buy(bar.symbol, 100, bar.bid);
        } else if (bar.close < bar.open) {
            bt.sell(bar.symbol, 100, bar.ask);
        }
    }
}

public fn main(): void {
    let bt = new Backtest();
    bt.setStrategy(new SimpleStrategy());
    bt.loadData("AAPL", "market_data.csv");

    let result = bt.run();
    io::println("Total PnL: " + Double.toString(result.totalPnL()));
    io::println("Sharpe: " + Double.toString(result.sharpeRatio()));
    io::println("Max Drawdown: " + Double.toString(result.maxDrawdown()));
}
```

### Slippage and Commission Models

```titrate
let bt = new Backtest();

// Set slippage model
bt.setSlippageModel("LINEAR", 0.001);  // 0.1% linear slippage

// Set commission model
bt.setCommissionModel("PER_SHARE", 0.005);  // $0.005 per share
// or: bt.setCommissionModel("PERCENT", 0.001);  // 0.1% of notional
```

### PnL Tracking

```titrate
let result = bt.run();

// Summary statistics
io::println("Total trades: " + Integer.toString(result.tradeCount()));
io::println("Win rate: " + Double.toString(result.winRate()));
io::println("Total PnL: " + Double.toString(result.totalPnL()));
io::println("Avg trade PnL: " + Double.toString(result.avgTradePnL()));

// Equity curve
let equity = result.equityCurve();
for (i in 0..equity.size()) {
    io::println(Double.toString(equity.get(i)));
}
```

## Latency Measurement

In HFT, every nanosecond counts. The `Latency` module provides nanosecond-precision timing and statistical analysis.

### Nanosecond Timestamps

```titrate
import tt::hft::Latency;

let start = Latency.now();  // nanosecond timestamp

// ... critical code path ...

let end = Latency.now();
let elapsed = end - start;
io::println("Elapsed: " + Long.toString(elapsed) + " ns");
```

### Percentile Statistics

```titrate
let lat = new Latency();

// Record multiple measurements
lat.record(1200);   // 1200 ns
lat.record(1450);
lat.record(980);
lat.record(3200);   // outlier
lat.record(1100);

// Percentile statistics
io::println("Min: " + Long.toString(lat.min()) + " ns");
io::println("P50: " + Long.toString(lat.percentile(50)) + " ns");
io::println("P95: " + Long.toString(lat.percentile(95)) + " ns");
io::println("P99: " + Long.toString(lat.percentile(99)) + " ns");
io::println("Max: " + Long.toString(lat.max()) + " ns");
io::println("Mean: " + Double.toString(lat.mean()) + " ns");
```

### Histograms

```titrate
// Generate a latency histogram for visualization
let hist = lat.histogram(10);  // 10 buckets
for (i in 0..hist.size()) {
    let bucket = hist.get(i);
    io::println(bucket.lower.toString() + "-" + bucket.upper.toString() +
                ": " + Integer.toString(bucket.count));
}
```

## End-to-End Example: Simple Market-Making Strategy

This example ties together all the HFT modules into a working market-making strategy that quotes both sides of the book, manages risk, and tracks latency.

```titrate
import tt::hft::FixParser;
import tt::hft::OrderRouter;
import tt::hft::RiskManager;
import tt::hft::Backtest;
import tt::hft::Latency;
import tt::math::Math;

public class MarketMaker {
    public string symbol;
    public double spread;
    public int qty;
    public OrderRouter router;
    public RiskManager risk;
    public Latency latency;
    public double position;

    public fn init(sym: string, sp: double, q: int) {
        this.symbol = sym;
        this.spread = sp;
        this.qty = q;
        this.router = new OrderRouter();
        this.risk = new RiskManager();
        this.latency = new Latency();
        this.position = 0.0;

        // Configure router
        this.router.addVenue("NYSE", 0.0005);
        this.router.addVenue("NASDAQ", 0.0003);
        this.router.setStrategy("BEST_PRICE");

        // Configure risk
        this.risk.setMaxPosition(sym, 5000);
        this.risk.setMaxOrderSize(q * 2);
        this.risk.setMaxNotional(250000.0);
        this.risk.setMaxOrdersPerSecond(20);
    }

    public fn onQuote(bid: double, ask: double): void {
        let start = Latency.now();

        let mid = (bid + ask) / 2.0;
        let myBid = mid - this.spread / 2.0;
        let myAsk = mid + this.spread / 2.0;

        // Risk check before quoting
        let bidCheck = this.risk.checkOrder(this.symbol, 1, this.qty, myBid);
        let askCheck = this.risk.checkOrder(this.symbol, 2, this.qty, myAsk);

        if (bidCheck.isOk()) {
            this.router.submitOrder(this.symbol, 1, this.qty, myBid, "LIMIT", "NYSE");
        }
        if (askCheck.isOk()) {
            this.router.submitOrder(this.symbol, 2, this.qty, myAsk, "LIMIT", "NYSE");
        }

        let elapsed = Latency.now() - start;
        this.latency.record(elapsed);
    }

    public fn onFill(side: int, fillQty: int, fillPrice: double): void {
        if (side == 1) {
            this.position = this.position + Double.parseDouble(Integer.toString(fillQty));
        } else {
            this.position = this.position - Double.parseDouble(Integer.toString(fillQty));
        }

        // Check if we need to reduce position
        if (Math.abs(this.position) > 3000.0) {
            io::println("WARNING: Position " + Double.toString(this.position) +
                        " exceeds threshold — reducing");
        }
    }

    public fn printStats(): void {
        io::println("=== Market Maker Stats ===");
        io::println("Position: " + Double.toString(this.position));
        io::println("Latency P50: " + Long.toString(this.latency.percentile(50)) + " ns");
        io::println("Latency P99: " + Long.toString(this.latency.percentile(99)) + " ns");
    }
}

public fn main(): void {
    let mm = new MarketMaker("AAPL", 0.05, 100);

    // Simulate incoming quotes
    mm.onQuote(150.00, 150.05);
    mm.onQuote(150.02, 150.07);
    mm.onQuote(149.98, 150.03);

    // Simulate a fill
    mm.onFill(1, 100, 150.00);

    mm.printStats();
}
```

::: tip Latency is everything
In production HFT systems, the critical path — from receiving market data to sending an order — should be measured in microseconds or less. Use the `Latency` module to identify and optimize hot paths in your strategy.
:::

## What's Next?

- [Standard Library](./stdlib) — full module reference
- [Error Handling](./error-handling) — robust error handling with `Result`
- [Optimizations](./optimizations) — performance tips for low-latency code
