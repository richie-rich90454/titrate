# hft

The `tt.hft` module provides high-frequency trading infrastructure including FIX protocol parsing, smart order routing, risk management, backtesting, and latency measurement. These components are designed for low-latency, high-throughput trading systems.

```titrate
import tt.hft.FixParser;
import tt.hft.OrderRouter;
import tt.hft.RiskManager;
import tt.hft.Backtest;
import tt.hft.Latency;
```

## FixParser

FIX 4.2/4.4 protocol message parser and builder with tag-value pair handling, checksum validation, and sequence number management. Loads the FIX dictionary from `data/hft/fix_dictionary.json`.

- `fn init(version: string)` — create a parser for the given FIX version (`"4.2"` or `"4.4"`)
- `parse(message: string): HashMap<string, string>` — parse a FIX message string into tag-value pairs
- `build(fields: HashMap<string, string>): string` — build a FIX message from tag-value pairs, computing checksum and body length
- `validateChecksum(message: string): bool` — validate the FIX message checksum (tag 10)
- `getSequenceNumber(message: string): int` — extract the sequence number (tag 34) from a message
- `getMessageType(message: string): string` — extract the message type (tag 35) from a message
- `setSequenceNumber(seq: int): void` — set the outgoing sequence number counter
- `nextSequenceNumber(): int` — get and increment the outgoing sequence number
- `lookupTagName(tag: string): string` — look up a tag number's name from the FIX dictionary
- `lookupTagNumber(name: string): string` — look up a tag name's number from the FIX dictionary

```titrate
let parser: FixParser = new FixParser("4.4");

let fields: HashMap<string, string> = parser.parse("8=FIX.4.4\x0135=D\x0149=SENDER\x0156=TARGET\x0134=1\x0111=ORD001\x0155=AAPL\x0154=1\x0144=150.00\x0138=100\x0140=2\x0110=000\x01");

let msgType: string = parser.getMessageType("8=FIX.4.4\x0135=D\x01...");
let seq: int = parser.getSequenceNumber("8=FIX.4.4\x0135=D\x0134=5\x01...");
let valid: bool = parser.validateChecksum("8=FIX.4.4\x0135=D\x0110=123\x01");
```

### Building FIX Messages

```titrate
let parser: FixParser = new FixParser("4.4");
let fields: HashMap<string, string> = new HashMap<string, string>();
fields.put("35", "D");       // New Order Single
fields.put("49", "SENDER");
fields.put("56", "TARGET");
fields.put("34", Integer.toString(parser.nextSequenceNumber()));
fields.put("55", "AAPL");
fields.put("54", "1");       // Buy
fields.put("44", "150.00");
fields.put("38", "100");
fields.put("40", "2");       // Limit order

let message: string = parser.build(fields);
```

## OrderRouter

Smart order routing with venue selection, order type management, and fill tracking.

- `fn init()` — create an order router with no configured venues
- `addVenue(name: string, endpoint: string, priority: int): void` — add a trading venue with name, connection endpoint, and routing priority (lower = higher priority)
- `removeVenue(name: string): bool` — remove a venue; returns true if found
- `route(symbol: string, side: string, quantity: double, orderType: string): string` — route an order to the best venue; returns the venue name selected
- `routeToVenue(symbol: string, side: string, quantity: double, orderType: string, venue: string): bool` — route an order to a specific venue; returns true if venue exists
- `cancelOrder(orderId: string): bool` — cancel a pending order; returns true if found
- `getFill(orderId: string): HashMap<string, string>` — get fill details for an order (keys: `"venue"`, `"price"`, `"quantity"`, `"status"`)
- `getOpenOrders(): ArrayList<string>` — list IDs of all open orders
- `setOrderTypeMapping(orderType: string, fixOrdType: string): void` — map an internal order type to a FIX OrdType value

```titrate
let router: OrderRouter = new OrderRouter();
router.addVenue("NYSE", "nyse.example.com:8000", 1);
router.addVenue("NASDAQ", "nasdaq.example.com:8000", 2);
router.addVenue("BATS", "bats.example.com:8000", 3);

let venue: string = router.route("AAPL", "BUY", 100.0, "LIMIT");
let routed: bool = router.routeToVenue("MSFT", "SELL", 200.0, "MARKET", "NASDAQ");
let cancelled: bool = router.cancelOrder("ORD-001");
let fill: HashMap<string, string> = router.getFill("ORD-001");
let open: ArrayList<string> = router.getOpenOrders();
```

## RiskManager

Pre-trade and runtime risk controls including position limits, order rate limits, kill switch, max notional, and concentration limits.

- `fn init()` — create a risk manager with default limits
- `setPositionLimit(symbol: string, maxQuantity: double): void` — set the maximum position size for a symbol
- `setOrderRateLimit(maxOrdersPerSecond: int): void` — set the maximum order rate
- `setMaxNotional(maxNotional: double): void` — set the maximum notional value per order (price × quantity)
- `setConcentrationLimit(symbol: string, maxPercent: double): void` — set the maximum portfolio concentration for a symbol as a fraction (0.0 to 1.0)
- `enableKillSwitch(): void` — activate the kill switch (blocks all new orders)
- `disableKillSwitch(): void` — deactivate the kill switch
- `isKillSwitchActive(): bool` — check if the kill switch is active
- `checkOrder(symbol: string, side: string, quantity: double, price: double): bool` — pre-trade risk check; returns true if the order passes all limits
- `recordFill(symbol: string, side: string, quantity: double, price: double): void` — record a fill for position tracking
- `getPosition(symbol: string): double` — get the current net position for a symbol
- `getOrderRate(): double` — get the current order rate (orders per second)
- `reset(): void` — reset all positions and counters

```titrate
let risk: RiskManager = new RiskManager();
risk.setPositionLimit("AAPL", 10000.0);
risk.setOrderRateLimit(100);
risk.setMaxNotional(500000.0);
risk.setConcentrationLimit("AAPL", 0.10);

let allowed: bool = risk.checkOrder("AAPL", "BUY", 500.0, 150.0);
if (allowed) {
    risk.recordFill("AAPL", "BUY", 500.0, 150.0);
}

let pos: double = risk.getPosition("AAPL");
let rate: double = risk.getOrderRate();
```

### Kill Switch

```titrate
risk.enableKillSwitch();
let blocked: bool = risk.checkOrder("AAPL", "BUY", 100.0, 150.0);  // false
risk.disableKillSwitch();
```

## Backtest

Event-driven backtesting engine with historical data replay, slippage and commission models, and PnL tracking.

- `fn init()` — create a backtesting engine with default settings
- `setInitialCapital(capital: double): void` — set the starting capital
- `setSlippageModel(model: string, basisPoints: double): void` — set the slippage model (`"fixed"`, `"percentage"`, `"basis_points"`) with the corresponding parameter
- `setCommissionModel(model: string, rate: double): void` — set the commission model (`"fixed"`, `"per_share"`, `"percentage"`) with the corresponding rate
- `loadBars(symbol: string, bars: ArrayList<OHLCV>): void` — load historical OHLCV bars for a symbol
- `loadTrades(symbol: string, trades: ArrayList<Trade>): void` — load historical trade data for a symbol
- `submitOrder(symbol: string, side: string, quantity: double, orderType: string, price: double): string` — submit an order during backtest; returns order ID
- `run(): void` — run the backtest to completion
- `step(): bool` — advance one event; returns false when finished
- `pause(): void` — pause the backtest
- `stop(): void` — stop the backtest immediately
- `getPnL(): double` — get the current realized + unrealized PnL
- `getTotalReturn(): double` — get total return as a fraction of initial capital
- `getPositions(): HashMap<string, double>` — get current positions (symbol → quantity)
- `getEquityCurve(): ArrayList<double>` — get the equity curve over time
- `getTradeLog(): ArrayList<HashMap<string, string>>` — get the log of all executed trades

```titrate
let bt: Backtest = new Backtest();
bt.setInitialCapital(1000000.0);
bt.setSlippageModel("basis_points", 5.0);
bt.setCommissionModel("per_share", 0.005);

let bars: ArrayList<OHLCV> = new ArrayList<OHLCV>();
// ... load historical bars ...
bt.loadBars("AAPL", bars);

bt.submitOrder("AAPL", "BUY", 100.0, "LIMIT", 150.0);
bt.run();

let pnl: double = bt.getPnL();
let ret: double = bt.getTotalReturn();
let curve: ArrayList<double> = bt.getEquityCurve();
let positions: HashMap<string, double> = bt.getPositions();
```

### Step-by-Step Backtest

```titrate
let bt: Backtest = new Backtest();
bt.setInitialCapital(500000.0);
bt.loadBars("MSFT", bars);

while (bt.step()) {
    let pnl: double = bt.getPnL();
    if (pnl < -10000.0) {
        bt.stop();
        break;
    }
}
```

## Latency

High-resolution latency measurement with nanosecond timestamps, latency statistics, and histogram distribution.

- `fn init()` — create a latency measurement instance
- `now(): long` — get the current time in nanoseconds
- `start(label: string): void` — start a labeled timing measurement
- `stop(label: string): long` — stop the labeled measurement and return elapsed nanoseconds
- `record(label: string, nanos: long): void` — manually record a latency sample for a label
- `getMin(label: string): long` — get the minimum recorded latency (ns)
- `getMax(label: string): long` — get the maximum recorded latency (ns)
- `getMean(label: string): double` — get the mean latency (ns)
- `getMedian(label: string): double` — get the median latency (ns)
- `getP99(label: string): double` — get the 99th percentile latency (ns)
- `getP999(label: string): double` — get the 99.9th percentile latency (ns)
- `getStdDev(label: string): double` — get the standard deviation of latency (ns)
- `getCount(label: string): long` — get the number of recorded samples
- `getHistogram(label: string, buckets: int): ArrayList<long>` — get a latency histogram with the specified number of buckets
- `reset(label: string): void` — clear all recorded samples for a label
- `resetAll(): void` — clear all recorded samples for all labels

```titrate
let lat: Latency = new Latency();

lat.start("order_send");
// ... send order ...
let elapsed: long = lat.stop("order_send");

lat.record("tick_to_trade", 850);

let minNs: long = lat.getMin("order_send");
let meanNs: double = lat.getMean("order_send");
let p99Ns: double = lat.getP99("order_send");
let p999Ns: double = lat.getP999("order_send");
let count: long = lat.getCount("order_send");
let hist: ArrayList<long> = lat.getHistogram("order_send", 10);
```

### Multiple Labels

```titrate
let lat: Latency = new Latency();
lat.start("parse");
// ... parse FIX message ...
lat.stop("parse");

lat.start("route");
// ... route order ...
lat.stop("route");

lat.start("risk_check");
// ... risk check ...
lat.stop("risk_check");

let parseP99: double = lat.getP99("parse");
let routeP99: double = lat.getP99("route");
let riskP99: double = lat.getP99("risk_check");
```
