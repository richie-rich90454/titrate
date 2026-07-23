# Cookbook

Practical recipes for common tasks in Titrate. Each recipe presents a problem, a complete solution, and a walkthrough of how it works. Copy the code, run it, and adapt it to your needs.

## Recipe 1: Read a CSV File and Process Rows

**Problem:** You have a CSV file and need to parse and process each row.

```titrate
import tt::util::ArrayList;
import tt::io::File;

public fn readCsv(path: string): ArrayList<ArrayList<string>> {
    let f = File.open(path, "r");
    let content: string = f.readAll();
    let rows: ArrayList<ArrayList<string>> = new ArrayList<ArrayList<string>>();
    let lines: ArrayList<string> = String.split(content, "\n");
    for (line in lines) {
        if (String.length(String.trim(line)) == 0) {
            continue;
        }
        let cells: ArrayList<string> = String.split(line, ",");
        rows.add(cells);
    }
    return rows;
}

public fn sumColumn(rows: ArrayList<ArrayList<string>>, col: int): double {
    var total: double = 0.0;
    var i: int = 1;  // skip header row
    while (i < rows.size()) {
        let row: ArrayList<string> = rows.get(i);
        if (col < row.size()) {
            let value: Result<double, string> = Double.parseDouble(row.get(col));
            if (value.isOk()) {
                total = total + value.unwrap();
            }
        }
        i = i + 1;
    }
    return total;
}

public fn main(): void {
    let rows: ArrayList<ArrayList<string>> = readCsv("data.csv");
    io::println("Rows: " + Integer.toString(rows.size()));
    let total: double = sumColumn(rows, 1);
    io::println("Column 1 total: " + Double.toString(total));
}
```

**Explanation:** `readCsv` reads the file, splits by newlines, then splits each line by commas. `sumColumn` iterates rows (skipping the header), parses a specific column as a double, and accumulates the total. Using `Result` ensures parse failures do not crash the program.

## Recipe 2: Build a REST API Client

**Problem:** You need to make HTTP requests and handle JSON responses.

```titrate
import tt::util::ArrayList;
import tt::util::HashMap;
import tt::json::JsonValue;
import tt::net::HttpClient;

public fn fetchUser(id: int): Result<JsonValue, string> {
    let client: HttpClient = new HttpClient();
    let url: string = "https://api.example.com/users/" + Integer.toString(id);
    let response: Result<string, string> = client.get(url);
    if (response.isErr()) {
        return err("HTTP request failed: " + response.unwrapErr());
    }
    let body: string = response.unwrap();
    let json: Result<JsonValue, string> = JsonValue.parse(body);
    if (json.isErr()) {
        return err("JSON parse failed: " + json.unwrapErr());
    }
    return json;
}

public fn printUser(id: int): void {
    let result: Result<JsonValue, string> = fetchUser(id);
    if (result.isOk()) {
        let user: JsonValue = result.unwrap();
        io::println("Name: " + user.get("name").asStr());
        io::println("Email: " + user.get("email").asStr());
    } else {
        io::println("Error: " + result.unwrapErr());
    }
}

public fn main(): void {
    printUser(1);
}
```

**Explanation:** The `fetchUser` function composes two fallible operations — an HTTP GET and a JSON parse — using `Result` to propagate errors. Each step checks for failure and returns early with a descriptive error message. The caller (`printUser`) handles both success and error paths.

## Recipe 3: Implement a Simple Cache with HashMap

**Problem:** You need an in-memory key-value cache with expiration.

```titrate
import tt::util::HashMap;
import tt::time::DateTime;

class CacheEntry<T> {
    public T value;
    public double expiresAt;

    public fn init(value: T, expiresAt: double) {
        this.value = value;
        this.expiresAt = expiresAt;
    }
}

class Cache<T> {
    private HashMap<string, CacheEntry<T>> store;
    private double defaultTtl;

    public fn init(defaultTtl: double) {
        this.store = new HashMap<string, CacheEntry<T>>();
        this.defaultTtl = defaultTtl;
    }

    public fn put(key: string, value: T): void {
        let expiresAt: double = DateTime.now().epochMillis + this.defaultTtl;
        let entry: CacheEntry<T> = new CacheEntry<T>(value, expiresAt);
        this.store.put(key, entry);
    }

    public fn putWithTtl(key: string, value: T, ttl: double): void {
        let expiresAt: double = DateTime.now().epochMillis + ttl;
        let entry: CacheEntry<T> = new CacheEntry<T>(value, expiresAt);
        this.store.put(key, entry);
    }

    public fn get(key: string): Result<T, string> {
        let entry: CacheEntry<T> = this.store.get(key);
        if (entry == null) {
            return err("key not found: " + key);
        }
        if (DateTime.now().epochMillis > entry.expiresAt) {
            this.store.remove(key);
            return err("key expired: " + key);
        }
        return ok(entry.value);
    }

    public fn remove(key: string): void {
        this.store.remove(key);
    }

    public fn clear(): void {
        this.store = new HashMap<string, CacheEntry<T>>();
    }
}

public fn main(): void {
    let cache: Cache<string> = new Cache<string>(5000.0);
    cache.put("greeting", "Hello, world!");
    let result: Result<string, string> = cache.get("greeting");
    if (result.isOk()) {
        io::println(result.unwrap());  // Hello, world!
    }
}
```

**Explanation:** The `Cache` class wraps a `HashMap` and stores each value alongside an expiration timestamp. `get` checks whether the entry exists and whether it has expired, returning a `Result` to signal success or failure. `putWithTtl` allows per-entry TTL overrides.

## Recipe 4: Parse and Transform JSON Data

**Problem:** You have a JSON string and need to extract and transform specific fields.

```titrate
import tt::util::ArrayList;
import tt::json::JsonValue;

public fn extractNames(json: string): ArrayList<string> {
    let parsed: Result<JsonValue, string> = JsonValue.parse(json);
    if (parsed.isErr()) {
        io::println("Parse error: " + parsed.unwrapErr());
        return new ArrayList<string>();
    }
    let root: JsonValue = parsed.unwrap();
    let users: JsonValue = root.get("users");
    let names: ArrayList<string> = new ArrayList<string>();
    var i: int = 0;
    while (i < users.size()) {
        let user: JsonValue = users.get(i);
        let name: string = user.get("name").asStr();
        names.add(String.toUpperCase(name));
        i = i + 1;
    }
    return names;
}

public fn main(): void {
    let json: string = "{\"users\":[{\"name\":\"Alice\"},{\"name\":\"Bob\"}]}";
    let names: ArrayList<string> = extractNames(json);
    for (name in names) {
        io::println(name);  // ALICE, BOB
    }
}
```

**Explanation:** `JsonValue.parse` converts a JSON string into a tree of `JsonValue` nodes. You navigate the tree with `.get(key)` for objects and `.get(index)` for arrays, then extract primitive values with `.asStr()`, `.asDouble()`, etc. The `Result` return from `parse` ensures malformed JSON does not crash your program.

## Recipe 5: Build a Command-Line Calculator

**Problem:** You want an interactive calculator that evaluates arithmetic expressions.

```titrate
import tt::util::ArrayList;

public fn evaluate(tokens: ArrayList<string>): Result<double, string> {
    if (tokens.size() != 3) {
        return err("Usage: <number> <operator> <number>");
    }
    let aResult: Result<double, string> = Double.parseDouble(tokens.get(0));
    if (aResult.isErr()) {
        return err("Invalid first number: " + tokens.get(0));
    }
    let a: double = aResult.unwrap();
    let op: string = tokens.get(1);
    let bResult: Result<double, string> = Double.parseDouble(tokens.get(2));
    if (bResult.isErr()) {
        return err("Invalid second number: " + tokens.get(2));
    }
    let b: double = bResult.unwrap();

    if (op == "+") {
        return ok(a + b);
    }
    if (op == "-") {
        return ok(a - b);
    }
    if (op == "*") {
        return ok(a * b);
    }
    if (op == "/") {
        if (b == 0.0) {
            return err("division by zero");
        }
        return ok(a / b);
    }
    return err("unknown operator: " + op);
}

public fn main(): void {
    let input: string = io::readLine();
    let tokens: ArrayList<string> = String.split(input, " ");
    let result: Result<double, string> = evaluate(tokens);
    if (result.isOk()) {
        io::println("= " + Double.toString(result.unwrap()));
    } else {
        io::println("Error: " + result.unwrapErr());
    }
}
```

**Explanation:** The calculator splits input into tokens, parses the two operands, and dispatches on the operator. Every fallible step returns a `Result`, so errors like invalid numbers or division by zero are handled gracefully without exceptions.

## Recipe 6: Implement a Simple Event System

**Problem:** You need a publish-subscribe system where components communicate through events.

```titrate
import tt::util::ArrayList;
import tt::util::HashMap;

interface EventHandler {
    fn handle(event: string): void;
}

class EventBus {
    private HashMap<string, ArrayList<EventHandler>> channels;

    public fn init() {
        this.channels = new HashMap<string, ArrayList<EventHandler>>();
    }

    public fn subscribe(channel: string, handler: EventHandler): void {
        let handlers: ArrayList<EventHandler> = this.channels.get(channel);
        if (handlers == null) {
            handlers = new ArrayList<EventHandler>();
            this.channels.put(channel, handlers);
        }
        handlers.add(handler);
    }

    public fn publish(channel: string, event: string): void {
        let handlers: ArrayList<EventHandler> = this.channels.get(channel);
        if (handlers == null) {
            return;
        }
        for (handler in handlers) {
            handler.handle(event);
        }
    }
}

class Logger implements EventHandler {
    public fn handle(event: string): void {
        io::println("[LOG] " + event);
    }
}

class Alerter implements EventHandler {
    public fn handle(event: string): void {
        io::println("[ALERT] " + String.toUpperCase(event));
    }
}

public fn main(): void {
    let bus: EventBus = new EventBus();
    bus.subscribe("system", new Logger());
    bus.subscribe("system", new Alerter());
    bus.publish("system", "disk space low");
    // [LOG] disk space low
    // [ALERT] DISK SPACE LOW
}
```

**Explanation:** The `EventBus` maintains a map of channel names to handler lists. `subscribe` adds a handler to a channel, and `publish` iterates all handlers for that channel. The `EventHandler` interface decouples the bus from specific handler implementations — any class can subscribe by implementing the interface.

## Recipe 7: Work with Dates and Durations

**Problem:** You need to parse, format, and compute differences between dates.

```titrate
import tt::time::DateTime;
import tt::time::Duration;

public fn daysBetween(a: DateTime, b: DateTime): int {
    let diff: Duration = b.difference(a);
    return diff.totalDays();
}

public fn formatIso(dt: DateTime): string {
    let year: string = Integer.toString(dt.year());
    let month: string = String.substring("0" + Integer.toString(dt.month()), String.length("0" + Integer.toString(dt.month())) - 2);
    let day: string = String.substring("0" + Integer.toString(dt.day()), String.length("0" + Integer.toString(dt.day())) - 2);
    return year + "-" + month + "-" + day;
}

public fn addWeeks(dt: DateTime, weeks: int): DateTime {
    return dt.add(Duration.fromDays(weeks * 7));
}

public fn main(): void {
    let start: DateTime = DateTime.of(2026, 1, 15);
    let end: DateTime = DateTime.of(2026, 3, 20);
    io::println("Days between: " + Integer.toString(daysBetween(start, end)));
    io::println("Start: " + formatIso(start));
    let future: DateTime = addWeeks(start, 4);
    io::println("4 weeks later: " + formatIso(future));
}
```

**Explanation:** `DateTime` and `Duration` from the standard library handle calendar math. `daysBetween` computes the difference in days, `formatIso` builds an ISO 8601 string, and `addWeeks` demonstrates date arithmetic by adding a `Duration` to a `DateTime`.

## Recipe 8: Create a Custom Collection (RingBuffer)

**Problem:** You need a fixed-size circular buffer that overwrites the oldest element when full.

```titrate
import tt::util::ArrayList;

class RingBuffer<T> {
    private ArrayList<T> data;
    private int head;
    private int count;
    private int capacity;

    public fn init(capacity: int) {
        this.capacity = capacity;
        this.data = new ArrayList<T>();
        this.head = 0;
        this.count = 0;
        var i: int = 0;
        while (i < capacity) {
            this.data.add(null);
            i = i + 1;
        }
    }

    public fn push(item: T): void {
        let index: int = (this.head + this.count) % this.capacity;
        this.data.set(index, item);
        if (this.count < this.capacity) {
            this.count = this.count + 1;
        } else {
            this.head = (this.head + 1) % this.capacity;
        }
    }

    public fn size(): int {
        return this.count;
    }

    public fn get(index: int): T {
        let actual: int = (this.head + index) % this.capacity;
        return this.data.get(actual);
    }

    public fn toArrayList(): ArrayList<T> {
        let result: ArrayList<T> = new ArrayList<T>();
        var i: int = 0;
        while (i < this.count) {
            result.add(this.get(i));
            i = i + 1;
        }
        return result;
    }
}

public fn main(): void {
    let buf: RingBuffer<int> = new RingBuffer<int>(3);
    buf.push(10);
    buf.push(20);
    buf.push(30);
    buf.push(40);  // overwrites 10
    let items: ArrayList<int> = buf.toArrayList();
    for (item in items) {
        io::println(Integer.toString(item));  // 20, 30, 40
    }
}
```

**Explanation:** A ring buffer uses modular arithmetic to wrap around a fixed-size array. `push` writes to the next slot, and when the buffer is full, the `head` advances to "forget" the oldest element. `get` maps a logical index to the physical index using `(head + index) % capacity`.

## Recipe 9: File Watcher / Directory Traversal

**Problem:** You need to walk a directory tree and find files matching a pattern.

```titrate
import tt::util::ArrayList;
import tt::io::File;

public fn findFiles(dir: string, extension: string): ArrayList<string> {
    let results: ArrayList<string> = new ArrayList<string>();
    let entries: Result<ArrayList<string>, string> = File.listDir(dir);
    if (entries.isErr()) {
        io::println("Cannot read dir: " + dir);
        return results;
    }
    for (entry in entries.unwrap()) {
        let fullPath: string = dir + "/" + entry;
        if (File.isDirectory(fullPath)) {
            let subResults: ArrayList<string> = findFiles(fullPath, extension);
            for (sub in subResults) {
                results.add(sub);
            }
        } else {
            if (String.endsWith(entry, extension)) {
                results.add(fullPath);
            }
        }
    }
    return results;
}

public fn main(): void {
    let pyFiles: ArrayList<string> = findFiles("./src", ".tr");
    io::println("Found " + Integer.toString(pyFiles.size()) + " Titrate files:");
    for (f in pyFiles) {
        io::println("  " + f);
    }
}
```

**Explanation:** `findFiles` is a recursive function that lists directory entries, recurses into subdirectories, and collects files whose names end with the given extension. `File.listDir` returns a `Result` so I/O errors are handled gracefully.

## Recipe 10: Matrix Operations for Scientific Computing

**Problem:** You need basic matrix operations — creation, multiplication, and transposition.

```titrate
import tt::util::ArrayList;

class Matrix {
    public int rows;
    public int cols;
    private ArrayList<double> data;

    public fn init(rows: int, cols: int) {
        this.rows = rows;
        this.cols = cols;
        this.data = new ArrayList<double>();
        var i: int = 0;
        while (i < rows * cols) {
            this.data.add(0.0);
            i = i + 1;
        }
    }

    public fn set(r: int, c: int, value: double): void {
        this.data.set(r * this.cols + c, value);
    }

    public fn get(r: int, c: int): double {
        return this.data.get(r * this.cols + c);
    }

    public fn multiply(other: Matrix): Result<Matrix, string> {
        if (this.cols != other.rows) {
            return err("dimension mismatch: " + Integer.toString(this.cols) + " != " + Integer.toString(other.rows));
        }
        let result: Matrix = new Matrix(this.rows, other.cols);
        var i: int = 0;
        while (i < this.rows) {
            var j: int = 0;
            while (j < other.cols) {
                var sum: double = 0.0;
                var k: int = 0;
                while (k < this.cols) {
                    sum = sum + this.get(i, k) * other.get(k, j);
                    k = k + 1;
                }
                result.set(i, j, sum);
                j = j + 1;
            }
            i = i + 1;
        }
        return ok(result);
    }

    public fn transpose(): Matrix {
        let result: Matrix = new Matrix(this.cols, this.rows);
        var i: int = 0;
        while (i < this.rows) {
            var j: int = 0;
            while (j < this.cols) {
                result.set(j, i, this.get(i, j));
                j = j + 1;
            }
            i = i + 1;
        }
        return result;
    }

    public fn print(): void {
        var i: int = 0;
        while (i < this.rows) {
            var row: string = "";
            var j: int = 0;
            while (j < this.cols) {
                if (j > 0) {
                    row = row + "  ";
                }
                row = row + Double.toString(this.get(i, j));
                j = j + 1;
            }
            io::println(row);
            i = i + 1;
        }
    }
}

public fn main(): void {
    let a: Matrix = new Matrix(2, 3);
    a.set(0, 0, 1.0); a.set(0, 1, 2.0); a.set(0, 2, 3.0);
    a.set(1, 0, 4.0); a.set(1, 1, 5.0); a.set(1, 2, 6.0);

    let b: Matrix = new Matrix(3, 2);
    b.set(0, 0, 7.0);  b.set(0, 1, 8.0);
    b.set(1, 0, 9.0);  b.set(1, 1, 10.0);
    b.set(2, 0, 11.0); b.set(2, 1, 12.0);

    let product: Result<Matrix, string> = a.multiply(b);
    if (product.isOk()) {
        product.unwrap().print();
    }

    io::println("Transpose of A:");
    a.transpose().print();
}
```

**Explanation:** The `Matrix` class stores data in a flat `ArrayList<double>` using row-major order (`row * cols + col`). `multiply` checks dimension compatibility and returns a `Result` — if the inner dimensions do not match, you get an error instead of a wrong answer. `transpose` swaps rows and columns.

## Recipe 11: Build a Simple TCP Echo Server

**Problem:** You need a basic TCP server that accepts connections and echoes back messages.

```titrate
import tt::net::TcpServer;
import tt::net::TcpClient;

public fn main(): void {
    let server: TcpServer = new TcpServer();
    let bound: bool = server.bind(8080);
    if (!bound) {
        io::println("Failed to bind to port 8080");
        return;
    }
    io::println("Server listening on port 8080");

    // Accept one connection and echo back messages
    let client: TcpClient = server.accept();
    let message: string = client.receive();
    io::println("Received: " + message);
    client.send("Echo: " + message);
    client.close();
    server.close();
}
```

**Explanation:** The `TcpServer` binds to a port and accepts incoming TCP connections. Each accepted connection is represented by a `TcpClient`, which provides `receive()` and `send()` methods for communication. This pattern forms the basis of any network server built with Titrate.

## Recipe 12: Implement Retry Logic with Result

**Problem:** An operation may fail transiently — you want to retry it a few times before giving up.

```titrate
import tt::util::ArrayList;

public fn retry<T>(operation: fn(): Result<T, string>, maxAttempts: int, delayMs: int): Result<T, string> {
    var attempt: int = 0;
    while (attempt < maxAttempts) {
        let result: Result<T, string> = operation();
        if (result.isOk()) {
            return result;
        }
        attempt = attempt + 1;
        if (attempt < maxAttempts) {
            io::println("Attempt " + Integer.toString(attempt) + " failed, retrying...");
            // Note: Thread.sleep() is not yet available in the stdlib
        } else {
            io::println("Attempt " + Integer.toString(attempt) + " failed, giving up.");
        }
    }
    return err("operation failed after " + Integer.toString(maxAttempts) + " attempts");
}

public fn unreliableOperation(): Result<string, string> {
    let rand: double = Math.random();
    if (rand > 0.5) {
        return ok("success!");
    }
    return err("random failure");
}

public fn main(): void {
    let result: Result<string, string> = retry(fn(): Result<string, string> {
        return unreliableOperation();
    }, 5, 1000);
    if (result.isOk()) {
        io::println("Got: " + result.unwrap());
    } else {
        io::println("Failed: " + result.unwrapErr());
    }
}
```

**Explanation:** `retry` is a higher-order function that takes an operation (a closure returning `Result<T, string>`), a maximum number of attempts, and a delay between retries. It calls the operation repeatedly until it succeeds or the attempt limit is reached. The closure-based design means you can wrap any fallible operation with retry logic.

## What is Next?

- [Error Handling](./error-handling) — `Result`, `ok`, `err`, and the `?` operator
- [Closures](./closures) — anonymous functions and capture semantics
- [Interfaces](./interfaces) — contracts and polymorphism
- [Standard Library](./stdlib) — what is available out of the box

## Recipe: Parse a DNA Sequence and Find ORFs

```titrate
import tt::bio::Sequence;
import tt::bio::CodonTable;

public fn findORFs(dna: string): ArrayList<string> {
    let seq = Sequence.dna(dna);
    let table = CodonTable.getTable(1);
    let orfs = new ArrayList<string>();
    let s = seq.toString();
    let len = String.length(s);
    var i = 0;
    while (i + 2 < len) {
        let codon = String.substring(s, i, i + 3);
        if (table.isStart(codon)) {
            var j = i + 3;
            while (j + 2 < len) {
                let nextCodon = String.substring(s, j, j + 3);
                if (table.isStop(nextCodon)) {
                    orfs.add(String.substring(s, i, j + 3));
                    break;
                }
                j = j + 3;
            }
        }
        i = i + 3;
    }
    return orfs;
}
```

## Recipe: Compute Black-Scholes Option Price

```titrate
import tt::finance::BlackScholes;

public fn main(): void {
    let s = 100.0;   // spot price
    let k = 105.0;   // strike price
    let t = 0.25;    // time to expiry (years)
    let r = 0.05;    // risk-free rate
    let sigma = 0.2; // volatility

    let callPrice = BlackScholes.callPrice(s, k, t, r, sigma);
    let putPrice = BlackScholes.putPrice(s, k, t, r, sigma);
    let delta = BlackScholes.delta(s, k, t, r, sigma);

    io::println("Call: " + Double.toString(callPrice));
    io::println("Put:  " + Double.toString(putPrice));
    io::println("Delta: " + Double.toString(delta));
}
```

## Recipe: Run a Simple Neural Network Training

```titrate
import tt.ml.Tensor;
import tt.ml.Layer;
import tt.ml.Loss;
import tt.ml.Optimizer;
import tt.ml.Model;

public fn main(): void {
    let model = new Model();
    model.add(new Layer.Dense(2, 16));
    model.add(new Layer.ReLU());
    model.add(new Layer.Dense(16, 1));
    model.add(new Layer.Sigmoid());

    let optimizer = new Optimizer.Adam(model.parameters(), 0.01);
    let lossFn = Loss.binaryCrossEntropy;

    // Training loop
    var epoch = 0;
    while (epoch < 100) {
        let loss = model.trainBatch(inputs, targets, lossFn, optimizer);
        if (epoch % 10 == 0) {
            io::println("Epoch " + Integer.toString(epoch) + ": loss=" + Double.toString(loss));
        }
        epoch = epoch + 1;
    }
}
```

## Recipe: Stream a Large JSON File

```titrate
import tt.json.JsonStreamingParser;
import tt.json.JsonValue;

public fn main(): void {
    let parser = new JsonStreamingParser();
    var count = 0;

    parser.onKey(fn(k: string): void {
        if (k == "name") { count = count + 1; }
    });

    parser.onValue(fn(v: JsonValue): void {
        if (count % 1000 == 0) {
            io::println("Processed " + Integer.toString(count) + " records");
        }
    });

    // Feed chunks from file
    let chunk = File.readChunk("large.json", 0, 8192);
    while (String.length(chunk) > 0) {
        parser.feed(chunk);
        chunk = File.readChunk("large.json", 0, 8192);
    }
    parser.finish();
}
```

## Recipe: Render a Calendar Month (Python `calendar` parity)

**Problem:** You need to display a month's calendar in plain text or HTML, mirroring Python's `calendar` module.

```titrate
import tt.time.TextCalendar;
import tt.time.HTMLCalendar;
import tt.time.Calendar;

public fn main(): void {
    let cal = new TextCalendar(0);  // Monday-first
    io::println(cal.formatMonth(2026, 7, 4));

    let htmlCal = new HTMLCalendar(0);
    let html: string = htmlCal.formatMonth(2026, 7, true);
    File.writeFile("july.html", html);

    // Quick helpers
    io::println(Boolean.toString(Calendar.isleap(2024)));  // true
    let (firstWd, nDays): (int, int) = Calendar.monthRange(2026, 7);
    io::println("July 2026 starts on weekday " + Integer.toString(firstWd) + " and has " + Integer.toString(nDays) + " days");
}
```

## Recipe: Sniff an Image File Format (Python `imghdr` parity)

**Problem:** You need to detect the format of an image file from its bytes, not its extension.

```titrate
import tt.image.Imghdr;

public fn identify(path: string): void {
    let bytes = File.readBytes(path);
    let fmt: string = Imghdr.what(bytes);
    io::println(path + " -> " + fmt);  // e.g. "png", "jpeg", "webp", "tiff"
}
```

## Recipe: Detect Audio File Type (Python `sndhdr` parity)

```titrate
import tt.audio.Sndhdr;

public fn sniff(path: string): void {
    let bytes = File.readBytes(path);
    let (fmt, rate, channels, frames): (string, int, int, int) = Sndhdr.what(bytes);
    io::println(fmt + " " + Integer.toString(rate) + "Hz " + Integer.toString(channels) + "ch");
}
```

## Recipe: Benchmark a Function (Python `timeit` parity)

**Problem:** You want to measure the throughput of a small function, like Python's `timeit.timeit`.

```titrate
import tt.timeit.Timeit;

public fn main(): void {
    let elapsed: double = Timeit.timeit(fn(): void {
        let _ = MathAdvanced.sqrt(2.0);
    }, 100000);  // 100,000 iterations
    io::println("ns per call: " + Double.toString(elapsed * 1_000_000_000.0 / 100_000.0));

    let repeat: double = Timeit.repeat(fn(): void {
        let _ = MathAdvanced.sqrt(2.0);
    }, 5, 10000);  // 5 runs, 10,000 iterations each
    io::println("best of 5: " + Double.toString(repeat) + "s");
}
```

## Recipe: Password Hashing with `crypt` (Python `crypt` parity)

```titrate
import tt.crypto.Crypt;

public fn main(): void {
    let salt: string = Crypt.mksalt("sha512crypt");
    let hashed: string = Crypt.crypt("correct horse battery staple", salt);
    io::println("stored hash: " + hashed);

    // Verify
    let verify: string = Crypt.crypt("correct horse battery staple", hashed);
    io::println(Boolean.toString(verify == hashed));  // true
}
```

## Recipe: Topological Sort of a DAG (Python `graphlib` parity)

```titrate
import tt.graphlib.Graphlib;

public fn buildOrder(): void {
    // Dependencies: "build" depends on "test" depends on "compile"
    let graph = new HashMap<string, ArrayList<string>>();
    let deps = new ArrayList<string>();
    deps.add("compile");
    graph.put("test", deps);

    let deps2 = new ArrayList<string>();
    deps2.add("test");
    graph.put("build", deps2);

    let order: ArrayList<string> = Graphlib.topologicalSort(graph);
    // order: ["compile", "test", "build"]
    for (step in order) { io::println(step); }
}
```

## Recipe: Parallel Sort with ExecutionPolicy (C++ `<algorithm>` parity)

**Problem:** You have a large list and want to sort it using multiple threads.

```titrate
import tt.algorithms.Algorithms;
import tt.execution_policy.ExecutionPolicy;

public fn main(): void {
    let data = new ArrayList<int>();
    // ... fill with millions of values ...
    Algorithms.sort(data, ExecutionPolicy.Par);       // parallel sort
    Algorithms.forEach(data, fn(x: int): void {
        // ... process x ...
    }, ExecutionPolicy.ParUnseq);                     // vectorized + parallel
}
```

## Recipe: Cooperative Cancellation with `JThread` and `StopToken` (C++ `<thread>` parity)

```titrate
import tt.thread.JThread;
import tt.thread.StopToken;

public fn main(): void {
    let jt = new JThread(fn(token: StopToken): void {
        var i: int = 0;
        while (!token.stopRequested()) {
            io::println("working " + Integer.toString(i));
            i = i + 1;
            Thread.sleep(Duration.ofMillis(100));
        }
        io::println("stopped cleanly");
    });
    Thread.sleep(Duration.ofSeconds(2));
    jt.requestStop();   // ask the worker to exit
    jt.join();          // wait for it
}
```

## Recipe: Generator Coroutine (C++ `<coroutine>` parity)

```titrate
import tt.concurrent.Generator;

public fn main(): void {
    let fib = new Generator<int>(fn(yield: fn(int): void): void {
        var a: int = 0;
        var b: int = 1;
        while (a < 100) {
            yield(a);
            let next: int = a + b;
            a = b;
            b = next;
        }
    });
    while (fib.hasNext()) {
        io::println(Integer.toString(fib.next()));  // 0, 1, 1, 2, 3, 5, 8, ...
    }
}
```

## Recipe: std::format-style String Formatting (C++ `<format>` parity)

```titrate
import tt.format.Format;

public fn main(): void {
    let pi: double = 3.141592653589793;
    let s1: string = Format.stdFormat("pi = {:.4f}", pi);   // "pi = 3.1416"
    let s2: string = Format.stdFormat("{:>10} | {:<10}", "name", "value");
    let s3: string = Format.stdFormat("{0} and {0}", "repeat");  // "repeat and repeat"
    io::println(s1);
    io::println(s2);
    io::println(s3);
}
```

## Recipe: LZMA/XZ Compression (Python `lzma` parity)

```titrate
import tt.compression.Lzma;

public fn main(): void {
    let payload = File.readBytes("payload.bin");
    let compressed = Lzma.compressXz(payload);
    File.writeBytes("payload.xz", compressed);
    io::println(Boolean.toString(Lzma.isXz(compressed)));  // true

    let restored = Lzma.decompressXz(compressed);
    io::println(Integer.toString(restored.size()));  // matches original size
}
```

## Recipe: Smart-Pointer Interop (C++ `<memory>` parity)

```titrate
import tt.memory.UniquePtr;
import tt.memory.SharedPtr;
import tt.memory.WeakPtr;

public class Resource {
    public fn init() { io::println("acquired"); }
    public fn close() { io::println("released"); }
    public fn work(): void { io::println("working"); }
}

public fn main(): void {
    let u = UniquePtr.of<Resource>(new Resource());  // single owner
    u.get().work();

    let s1 = SharedPtr.of<Resource>(new Resource());  // refcounted
    let s2 = SharedPtr.copy<Resource>(s1);             // share ownership
    let w = WeakPtr.of<Resource>(s1);                  // non-owning observer
    // Resource is released when last SharedPtr goes out of scope
}
```

## Recipe: Streaming XML with SAX Handlers (C++ `<streambuf>` style)

```titrate
import tt.xml.XmlStreamingParser;

public fn main(): void {
    let parser = new XmlStreamingParser();
    var depth: int = 0;
    parser.onStartElement(fn(name: string, attrs: HashMap<string, string>): void {
        depth = depth + 1;
        io::println("+" + name);
    });
    parser.onEndElement(fn(name: string): void {
        depth = depth - 1;
        io::println("-" + name);
    });
    parser.onCharacters(fn(text: string): void {
        if (String.length(String.trim(text)) > 0) {
            io::println("  text: " + text);
        }
    });
    File.streamOver("data.xml", fn(chunk: string): void {
        parser.feed(chunk);
    });
    parser.finish();
}
```
