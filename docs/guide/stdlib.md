# Standard Library

The Titrate standard library (`tt`) is a comprehensive collection of modules that provide essential functionality for everyday programming — from collections and I/O to math, networking, and testing. It is shipped with every Titrate installation and requires no external dependencies.

## Introduction

The standard library is designed to be:

- **Batteries-included**: Most common programming tasks can be accomplished without third-party libraries
- **Consistent**: All modules follow the same naming conventions and API patterns
- **Type-safe**: Generic types are used throughout, with full type parameter inference
- **Well-documented**: Each module has its own documentation page with examples

## How to Import Modules

Use the `import` keyword with `::` separators to bring standard library modules into scope:

```titrate
import tt::util::ArrayList;
import tt::math::Math;
import tt::json::JsonValue;
```

### Import Rules

- **`::` is only used in import statements** — everywhere else, use `.` for member access
- Each import brings a single name into scope
- Imports must appear at the top of the file, before any declarations
- You can import multiple names from the same module in separate statements

```titrate
import tt::util::ArrayList;
import tt::util::HashMap;
import tt::io::IO;
```

## Module Naming Conventions

Standard library modules follow a consistent naming pattern:

```
tt::namespace::ClassName
```

| Namespace | Purpose | Examples |
|-----------|---------|---------|
| `tt::util` | General-purpose data structures | `ArrayList`, `HashMap`, `StringBuilder` |
| `tt::math` | Mathematical operations | `Math`, `NDArray`, `Matrix` |
| `tt::io` | Input/output | `IO`, `File` |
| `tt::json` | JSON handling | `JsonValue` |
| `tt::random` | Random number generation | `Random` |
| `tt::regex` | Regular expressions | `Regex`, `Match` |
| `tt::uuid` | UUID generation | `Uuid` |
| `tt::operator` | Functional operator wrappers | `Operator` |
| `tt::contextlib` | Context managers | `Contextlib` |
| `tt::datetime` | Date and time | `DateTime`, `Duration` |
| `tt::system` | System operations | `Sys` |
| `tt::crypto` | Cryptography | `Crypto` |
| `tt::networking` | Network operations | `TcpClient`, `HttpClient` |
| `tt::concurrent` | Concurrency | `Future`, `Channel` |
| `tt::argparse` | Argument parsing | `ArgumentParser` |
| `tt::logging` | Logging | `Logger` |
| `tt::functools` | Higher-order functions | `Functools` |
| `tt::testing` | Testing framework | `Assay` |

## Core Modules

### lang — Core Types

The `lang` module provides the fundamental types that every Titrate program uses:

| Type | Description |
|------|-------------|
| `String` | Static methods for string operations (`String.length(s)`, `String.toUpperCase(s)`, etc.) |
| `Integer` | Integer parsing and formatting (`Integer.parseInt("42")`, `Integer.toString(42)`) |
| `Double` | Double parsing and formatting (`Double.parseDouble("3.14")`, `Double.toString(3.14)`) |
| `Boolean` | Boolean conversion (`Boolean.toString(true)`, `Boolean.parseBool("true")`) |
| `Character` | Character operations |
| `Result` | Error handling type with `ok()` and `err()` constructors |
| `Iterator` | Traversal interface for collections |
| `Iterable` | Interface for types that can be iterated with `for-in` |

```titrate
// String operations use static module methods
let s: string = "Hello, World!";
let len: int = String.length(s);
let upper: string = String.toUpperCase(s);
let parts: ArrayList<string> = String.split(s, ", ");

// Integer conversion
let n: int = Integer.parseInt("42");
let text: string = Integer.toString(n);

// Result type for error handling
let r: Result<int, string> = ok(42);
if (r.isOk()) {
    io::println(Integer.toString(r.unwrap()));  // 42
}
```

### operator — Functional Operator Wrappers

The `operator` module wraps built-in operators as functions, enabling higher-order programming patterns:

```titrate
import tt.operator.Operator;

let sum: int = Operator.add(3, 4);        // 7
let cmp: bool = Operator.gt(10, 5);       // true
let div: double = Operator.truediv(10.0, 3.0);  // 3.333...
```

See the [operator](../stdlib/operator) documentation for the complete function reference.

### optional-variant — Optional and Variant Types

The `optional-variant` module provides safe alternatives to null values and dynamic typing:

```titrate
// Optional — a value that may or may not be present
let maybeName: Optional<string> = Optional.of("Alice");
let empty: Optional<string> = Optional.empty();

if (maybeName.isPresent()) {
    io::println(maybeName.get());  // "Alice"
}

// Variant — a dynamically-typed value
let v: Variant = 42;
let s: Variant = "hello";
```

## Collections

Titrate provides a rich set of collection types for storing and manipulating groups of values.

### collections — Primary Data Structures

| Type | Description | When to Use |
|------|-------------|-------------|
| `ArrayList<T>` | Dynamic array | Ordered collection, fast index access |
| `HashMap<K, V>` | Key-value map | Fast lookups by key |
| `Set<T>` | Unique elements | Membership testing, deduplication |
| `Vec<T>` | Numeric vector | Mathematical vector operations |
| `Deque<T>` | Double-ended queue | FIFO/LIFO operations |
| `PriorityQueue<T>` | Priority-based ordering | Scheduling, min/max retrieval |
| `Counter<T>` | Frequency counting | Histograms, vote tallying |
| `StringBuilder` | Efficient string building | Concatenating many strings |

```titrate
import tt::util::ArrayList;
import tt::util::HashMap;

// ArrayList — ordered, indexable
let fruits: ArrayList<string> = new ArrayList<string>();
fruits.add("apple");
fruits.add("banana");
fruits.add("cherry");
io::println(fruits.get(1));  // "banana"
io::println(Integer.toString(fruits.size()));  // 3

// HashMap — key-value lookups
let scores: HashMap<string, int> = new HashMap<string, int>();
HashMap.put(scores, "Alice", 95);
HashMap.put(scores, "Bob", 87);
io::println(Integer.toString(HashMap.get(scores, "Alice")));  // 95
```

### array — Fixed-Size Arrays

For performance-critical code where the size is known at compile time:

```titrate
let buffer: int[] = new int[10];
buffer[0] = 42;
buffer[1] = 100;
```

### heapq — Heap-Based Priority Queue

Efficient min-heap operations for priority queues:

```titrate
import tt.heapq.Heapq;

let heap: ArrayList<int> = new ArrayList<int>();
Heapq.push(heap, 5);
Heapq.push(heap, 2);
Heapq.push(heap, 8);
let min: int = Heapq.pop(heap);  // 2
```

### bisect — Binary Search

Binary search operations on sorted sequences:

```titrate
import tt.bisect.Bisect;

let sorted: ArrayList<int> = new ArrayList<int>();
sorted.add(1);
sorted.add(3);
sorted.add(5);
sorted.add(7);
let pos: int = Bisect.bisectLeft(sorted, 4);  // 2
```

### itertools — Iterator Adapters

Composable iterator transformations:

```titrate
import tt.itertools.Itertools;

// Chain, zip, cycle, and more
```

### dataclass — Auto-Generated Class Boilerplate

Decorator-like pattern for reducing class boilerplate:

```titrate
import tt.dataclass.Dataclass;
```

## I/O & File System

### io — Input/Output

The `io` module provides file operations and console output:

```titrate
import tt.io.IO;

// Console output
io::println("Hello, world!");
io::print("No newline here");

// File operations
let content: string = IO.readFile("data.txt");
IO.writeFile("output.txt", "Hello, file!");

// Line-by-line reading
let lines: ArrayList<string> = IO.readLines("data.txt");
```

### contextlib — Resource Management

The `contextlib` module ensures resources are properly cleaned up:

```titrate
import tt.contextlib.Contextlib;

// Ensure a file is closed after use
let file: File = File.open("data.txt");
Contextlib.closing(file, fn(): void {
    let content: string = file.readAll();
    io::println(content);
});
// file.close() is called automatically

// Suppress exceptions from a block
Contextlib.suppress(fn(): void {
    // risky operation — errors are silently ignored
});
```

See the [contextlib](../stdlib/contextlib) documentation for more details.

## Text & Serialization

### text — Text Utilities

String formatting and manipulation:

```titrate
import tt.text.Text;

// Text formatting, alignment, wrapping
let padded: string = Text.center("hello", 10);
```

### regex — Regular Expressions

Pattern matching and text extraction:

```titrate
import tt.regex.Regex;

let pattern: Regex = new Regex("\\d+");
let m: Match = Regex.match(pattern, "abc123def");
if (m.found()) {
    io::println(m.group());  // "123"
}
```

### serialization — JSON, CSV, XML

Parsing and writing structured data formats:

```titrate
import tt.json.JsonValue;

// JSON parsing
let json: string = "{\"name\": \"Alice\", \"age\": 30}";
let parsed: JsonValue = JsonValue.parse(json);
let name: string = parsed.get("name").asStr();  // "Alice"

// JSON construction
let obj: JsonValue = JsonValue.ofObj();
obj.set("name", JsonValue.ofStr("Bob"));
obj.set("age", JsonValue.ofNum(25));
let output: string = obj.toString();
```

### pprint — Pretty Printing

Formatted output for data structures:

```titrate
import tt.pprint.Pprint;

let data: HashMap<string, int> = new HashMap<string, int>();
HashMap.put(data, "alpha", 1);
HashMap.put(data, "beta", 2);
Pprint.print(data);  // nicely formatted output
```

## Math & Science

### math — Mathematical Functions

Constants, trigonometry, logarithms, and advanced types:

```titrate
import tt.math.Math;

let pi: double = Math.PI;
let root: double = Math.sqrt(2.0);
let sinVal: double = Math.sin(Math.PI / 4.0);
let pow: double = Math.pow(2.0, 10.0);  // 1024.0
let absVal: double = Math.abs(-3.14);    // 3.14
```

### complex — Complex Numbers

Complex number arithmetic:

```titrate
import tt.complex.Complex;

let z: Complex = new Complex(3.0, 4.0);  // 3 + 4i
let magnitude: double = z.abs();          // 5.0
let conj: Complex = z.conjugate();        // 3 - 4i
```

### fractions — Rational Numbers

Exact rational arithmetic without floating-point errors:

```titrate
import tt.fractions.Fraction;

let a: Fraction = new Fraction(1, 3);
let b: Fraction = new Fraction(1, 6);
let sum: Fraction = a.add(b);  // 1/2
```

### statistics — Statistical Functions

Descriptive statistics:

```titrate
import tt.statistics.Statistics;

let data: ArrayList<double> = new ArrayList<double>();
data.add(1.0);
data.add(2.0);
data.add(3.0);
data.add(4.0);
data.add(5.0);
let avg: double = Statistics.mean(data);      // 3.0
let med: double = Statistics.median(data);    // 3.0
let vari: double = Statistics.variance(data); // 2.5
```

### chemistry — Computational Chemistry

Atom, molecule, and force field types for computational chemistry:

```titrate
import tt.chemistry.Atom;

let h: Atom = Atom.hydrogen(0.0, 0.0, 0.0);
let o: Atom = Atom.oxygen(0.0, 0.9572, 0.0);
```

### units — Units of Measure

Physical units and constants:

```titrate
import tt.units.Constants;

let c: double = Constants.speedOfLight;  // 299792458.0 m/s
```

## System & Networking

### system — System Operations

Environment variables, CLI arguments, and process control:

```titrate
import tt.system.Sys;

// Read environment variables
let home: string = Sys.getenv("HOME");

// Get command-line arguments
let args: ArrayList<string> = Sys.args();

// Execute a system command
Sys.exec("ls -la");

// Exit the program
Sys.exit(1);
```

### networking — TCP and HTTP

Network communication:

```titrate
import tt.networking.TcpClient;
import tt.networking.HttpClient;

// TCP client
let client: TcpClient = new TcpClient();
client.connect("example.com", 80);
client.send("GET / HTTP/1.0\r\n\r\n");
let response: string = client.receive();
client.close();

// HTTP client
let http: HttpClient = new HttpClient();
let body: string = http.get("https://api.example.com/data");
```

### concurrent — Concurrency

Asynchronous programming primitives:

```titrate
import tt.concurrent.Future;
import tt.concurrent.Channel;

// Futures for async computation
let f: Future<string> = Future.of(fn(): string {
    return fetchFromNetwork();
});

// Channels for message passing
let ch: Channel<string> = new Channel<string>();
ch.send("hello");
let msg: string = ch.receive();
```

### crypto — Cryptography

Hashing and encryption:

```titrate
import tt.crypto.Crypto;

let hash: string = Crypto.sha256("hello world");
let hmac: string = Crypto.hmacSha256("secret key", "message");
```

## Date & Time

### datetime — Date, Time, and Duration

Date and time manipulation:

```titrate
import tt.datetime.DateTime;
import tt.datetime.Duration;

let now: DateTime = DateTime.now();
let tomorrow: DateTime = now.plusDays(1);
let diff: Duration = now.diff(tomorrow);
io::println(Integer.toString(diff.inHours()));  // 24

// Parse and format dates
let parsed: DateTime = DateTime.parse("2025-01-15T10:30:00");
let formatted: string = DateTime.format(now, "yyyy-MM-dd");
```

## Random & Utilities

### functools — Higher-Order Functions

Function composition, partial application, and more:

```titrate
import tt.functools.Functools;

// Compose two functions
let addOne: fn(int): int = fn(x: int): int { return x + 1; };
let double: fn(int): int = fn(x: int): int { return x * 2; };
let addOneThenDouble: fn(int): int = Functools.compose(double, addOne);
let result: int = addOneThenDouble(5);  // (5 + 1) * 2 = 12
```

### logging — Logging Framework

Structured logging with levels:

```titrate
import tt.logging.Logger;

let log: Logger = new Logger("MyApp");
Logger.info(log, "Application started");
Logger.warn(log, "Low disk space");
Logger.error(log, "Connection failed");
```

### uuid — UUID Generation

Universally unique identifiers:

```titrate
import tt.uuid.Uuid;

let id: string = Uuid.uuid4();
io::println(id);  // e.g. "a1b2c3d4-e5f6-4a7b-8c9d-0e1f2a3b4c5d"

let valid: bool = Uuid.isValid(id);  // true
```

See the [uuid](../stdlib/uuid) documentation for more details.

### argparse — Command-Line Argument Parsing

Declarative CLI argument definitions:

```titrate
import tt.argparse.ArgumentParser;

let parser: ArgumentParser = new ArgumentParser("myapp");
parser.addArg("--input", "Input file path", true);
parser.addArg("--verbose", "Enable verbose output", false);
let args: HashMap<string, string> = parser.parse();
```

### algorithms — Common Algorithms

Sorting, searching, and graph traversal:

```titrate
import tt.algorithms.Algorithms;

let list: ArrayList<int> = new ArrayList<int>();
list.add(5);
list.add(2);
list.add(8);
list.add(1);
Algorithms.sort(list);
// list is now [1, 2, 5, 8]
```

## Testing

### testing — Built-in Testing Framework (Assay)

Titrate includes a testing framework called **Assay** for writing and running tests:

```titrate
import tt.testing.Assay;

Assay.describe("Math operations", fn(): void {
    Assay.it("should add correctly", fn(): void {
        Assay.assertEqual(2 + 2, 4);
    });

    Assay.it("should multiply correctly", fn(): void {
        Assay.assertEqual(3 * 4, 12);
    });
});
```

### assert — Assertion Utilities

Standalone assertion functions for validation:

```titrate
import tt.assert.Assert;

Assert.assertEqual(expected, actual);
Assert.assertTrue(condition);
Assert.assertFalse(condition);
Assert.assertNotNull(value);
```

## Choosing the Right Data Structure

Use this decision guide to pick the best collection for your needs:

```
Do you need key-value lookups?
├── Yes → HashMap<K, V>
└── No
    ├── Do you need unique elements?
    │   ├── Yes → Set<T>
    │   └── No
    │       ├── Do you need priority ordering?
    │       │   ├── Yes → PriorityQueue<T>
    │       │   └── No
    │       │       ├── Do you need FIFO/LIFO operations?
    │       │       │   ├── Yes → Deque<T>
    │       │       │   └── No → ArrayList<T>
    │       │       └── Do you need frequency counting?
    │       │           └── Yes → Counter<T>
    └── Do you need efficient string building?
        └── Yes → StringBuilder
```

### Quick Comparison

| Collection | Access | Insert | Delete | Search | Ordered |
|------------|--------|--------|--------|--------|---------|
| `ArrayList<T>` | O(1) index | O(1) amortized | O(n) | O(n) | Yes (insertion) |
| `HashMap<K, V>` | O(1) key | O(1) | O(1) | O(1) key | No |
| `Set<T>` | O(1) | O(1) | O(1) | O(1) | No |
| `Deque<T>` | O(1) ends | O(1) ends | O(1) ends | O(n) | Yes (insertion) |
| `PriorityQueue<T>` | O(1) min | O(log n) | O(log n) | O(n) | By priority |
| `StringBuilder` | O(1) append | — | — | — | Yes |

## Common Import Patterns

Most Titrate programs start with a similar set of imports. Here are common patterns:

### Basic Script

```titrate
import tt.io.IO;
```

### Data Processing

```titrate
import tt.util.ArrayList;
import tt.util.HashMap;
import tt.io.IO;
import tt.json.JsonValue;
```

### Scientific Computing

```titrate
import tt.math.Math;
import tt.math.NDArray;
import tt.math.Matrix;
import tt.statistics.Statistics;
```

### Web Service

```titrate
import tt.networking.HttpClient;
import tt.json.JsonValue;
import tt.util.HashMap;
import tt.logging.Logger;
import tt.uuid.Uuid;
```

### CLI Application

```titrate
import tt.argparse.ArgumentParser;
import tt.io.IO;
import tt.system.Sys;
import tt.logging.Logger;
```

## How to Explore the Stdlib

1. **Browse by category**: Use the navigation on the left to explore modules by functional area
2. **Search**: Use the search bar to find specific functions or types
3. **Read the source**: Standard library source code lives in `lib/tt/` and is written in Titrate itself
4. **Check examples**: The `examples/` directory contains working programs that demonstrate stdlib usage

### Module Index

#### Core

| Module | Description |
|--------|-------------|
| [lang](../stdlib/lang) | Core types: `String`, `Integer`, `Double`, `Boolean`, `Character`, `Result`, `Iterator`, `Iterable` |
| [operator](../stdlib/operator) | Functional operator wrappers for higher-order programming |
| [optional-variant](../stdlib/optional-variant) | `Optional` and `Variant` types for safe value handling |

#### Collections

| Module | Description |
|--------|-------------|
| [collections](../stdlib/collections) | `ArrayList`, `HashMap`, `Set`, `Vec`, `Deque`, `PriorityQueue`, `Counter`, `StringBuilder` |
| [array](../stdlib/array) | Fixed-size arrays and array utilities |
| [hashset](../stdlib/hashset) | Hash-based set implementation |
| [heapq](../stdlib/heapq) | Heap-based priority queue operations |
| [bisect](../stdlib/bisect) | Binary search for sorted sequences |
| [itertools](../stdlib/itertools) | Iterator adapters and combinators |
| [dataclass](../stdlib/dataclass) | Auto-generating class boilerplate |

#### I/O & File System

| Module | Description |
|--------|-------------|
| [io](../stdlib/io) | File I/O, `println`, `print`, and stream operations |
| [contextlib](../stdlib/contextlib) | Resource management with `with`-style contexts |

#### Text & Serialization

| Module | Description |
|--------|-------------|
| [text](../stdlib/text) | Text formatting and manipulation utilities |
| [regex](../stdlib/regex) | Regular expressions: `Regex`, `Match` |
| [serialization](../stdlib/serialization) | JSON, CSV, and XML parsing and writing |
| [pprint](../stdlib/pprint) | Pretty-printing for data structures |

#### Math & Science

| Module | Description |
|--------|-------------|
| [math](../stdlib/math) | Mathematical constants, functions, `NDArray`, `Matrix` |
| [complex](../stdlib/complex) | Complex number type and operations |
| [fractions](../stdlib/fractions) | Rational number (`Fraction`) type |
| [statistics](../stdlib/statistics) | Statistical functions: mean, median, variance, etc. |
| [chemistry](../stdlib/chemistry) | Computational chemistry: `Atom`, `Molecule`, `ForceField`, `MD`, `RHF` |
| [units](../stdlib/units) | Units of measure: `Base`, `Derived`, physical `Constants` |

#### System & Networking

| Module | Description |
|--------|-------------|
| [system](../stdlib/system) | Environment variables, CLI args, `Sys.exec`, `Sys.exit` |
| [networking](../stdlib/networking) | TCP client/server, HTTP client |
| [concurrent](../stdlib/concurrent) | Concurrency primitives and async utilities |
| [crypto](../stdlib/crypto) | Cryptographic hashes and encryption |

#### Date & Time

| Module | Description |
|--------|-------------|
| [datetime](../stdlib/datetime) | `DateTime`, `Duration`, `Time` |

#### Random & Utilities

| Module | Description |
|--------|-------------|
| [functools](../stdlib/functools) | Higher-order functions: composition, partial application |
| [logging](../stdlib/logging) | Logging framework |
| [uuid](../stdlib/uuid) | UUID generation and parsing |
| [argparse](../stdlib/argparse) | Command-line argument parsing |
| [algorithms](../stdlib/algorithms) | Common algorithms: sort, search, graph traversal |

#### Testing

| Module | Description |
|--------|-------------|
| [testing](../stdlib/testing) | Built-in testing framework (`Assay`) |
| [assert](../stdlib/assert) | Assertion utilities |

## New Scientific Modules

| Namespace | Purpose | Examples |
|-----------|---------|---------|
| `tt::bio` | Bioinformatics | `Sequence`, `Alignment`, `PhyloTree` |
| `tt::physics` | Physics simulation | `Particle`, `ForceField`, `NBodySimulator` |
| `tt::materials` | Materials science | `CrystalStructure`, `XRayDiffraction`, `Elasticity` |
| `tt::sigproc` | Signal processing | `FFT2`, `Filter`, `Wavelet`, `Spectrogram` |
| `tt::image` | Image processing | `Image`, `Kernel`, `Morphology`, `Threshold` |
| `tt::audio` | Audio processing | `AudioBuffer`, `Pitch`, `Mfcc` |
| `tt::ml` | Machine learning | `Tensor`, `Layer`, `Optimizer`, `Model` |
| `tt::geom` | Computational geometry | `ConvexHull`, `Delaunay`, `SpatialIndex` |
| `tt::nlp` | Natural language processing | `Tokenizer`, `Stemmer`, `Classifier` |
| `tt::crypto2` | Advanced cryptography | `AES`, `RSA`, `ECDSA`, `KDF` |
| `tt::hft` | High-frequency trading | `FixParser`, `OrderRouter`, `RiskManager` |
| `tt::sim` | Discrete-event simulation | `Simulation`, `Resource`, `Process` |

## Enhanced Modules

| Namespace | New Features |
|-----------|-------------|
| `tt::math` | Number theory, combinatorics, interval arithmetic, autodiff, special functions |
| `tt::statistics` | Hypothesis testing, ANOVA, Bayesian stats, MCMC, KDE, bootstrap, time series, survival analysis |
| `tt::random` | MT19937, PCG32, quasi-random sequences, continuous/discrete distributions |
| `tt::calculus` | Quadrature, vector calculus, symbolic differentiation, series expansion |
| `tt::chem` | PeriodicTable, ReactionBalancer, Thermochemistry, Kinetics, Electrochemistry |
| `tt::finance` | BlackScholes, binomial tree, Monte Carlo pricing, yield curve, portfolio optimization |
| `tt::xml` | XmlNamespace, XmlStreamingParser, XPath, XmlBuilder, XmlSchema, XmlCanonicalizer |
| `tt::json` | JsonStreamingParser, JsonPath, JsonPatch, JsonSchema, Json5, JsonBinary |
| `tt::lang` | DataFile, IntegerExt, LongExt, DoubleExt, ResultExt, OptionalExt, VastExt |
