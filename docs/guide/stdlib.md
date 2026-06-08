# Standard Library

The Titrate standard library is organized into modules under the `tt` namespace.

## tt.lang — Core Types

| Type | Description |
|------|-------------|
| `Boolean` | Wrapper for `bool` with logical utilities |
| `Character` | Wrapper for `char` with Unicode operations |
| `Integer` | Wrapper for `int` with parsing and conversion |
| `Long` | Wrapper for `long` with parsing and conversion |
| `Vast` | Wrapper for `vast` (128-bit signed integer) |
| `Uvast` | Wrapper for `uvast` (128-bit unsigned integer) |
| `Float` | Wrapper for `float` with conversion |
| `Double` | Wrapper for `double` with conversion |
| `Half` | Wrapper for `half` (16-bit float) |
| `Quad` | Wrapper for `quad` (128-bit float) |
| `Byte` | Wrapper for `byte` (8-bit signed integer) |
| `Short` | Wrapper for `short` (16-bit signed integer) |
| `String` | String operations: split, length, concat |
| `ParseError` | Error type returned by parse methods |

### Integer

- `Integer.toString(n: int): string` — convert int to string
- `Integer.parseInt(s: string): int` — parse string to int
- `Integer.parseOr(s: string, default: int): int` — parse with default on failure

### Long

- `Long.toString(n: long): string` — convert long to string
- `Long.parseLong(s: string): long` — parse string to long

### Double

- `Double.toString(d: double): string` — convert double to string
- `Double.parseDouble(s: string): double` — parse string to double

### String

- `String.split(s: string, delimiter: string): array<string>` — split a string on a delimiter
- `String.length(s: string): int` — get string length
- `String.concat(a: string, b: string): string` — concatenate two strings

## tt.util — Collections

### ArrayList

- `new ArrayList<E>()` — create a new list
- `.add(item: E): void` — add an item
- `.get(index: int): E` — get item by index
- `.set(index: int, item: E): void` — set item at index
- `.remove(index: int): E` — remove and return item at index
- `.size(): int` — get the number of items
- `.sort(): void` — sort items (strings: lexicographic)

### HashMap

- `new HashMap<K, V>()` — create a new map
- `.put(key: K, value: V): void` — insert a key-value pair
- `.get(key: K): V` — get value by key (returns null if not found)
- `.containsKey(key: K): bool` — check if key exists
- `.remove(key: K): void` — remove a key
- `.size(): int` — get the number of entries

### Vec

- `new Vec<E>()` — create a new vector
- `.push(item: E): void` — push an item
- `.pop(): E` — pop and return the last item
- `.get(index: int): E` — get item by index
- `.set(index: int, item: E): void` — set item at index
- `.size(): int` — get the number of items
- `.isEmpty(): bool` — check if empty
- `.contains(item: E): bool` — check if item exists

### Set

- `new Set<E>()` — create a new set
- `.add(item: E): void` — add an item
- `.contains(item: E): bool` — check if item exists
- `.remove(item: E): bool` — remove an item, returns whether it was present
- `.size(): int` — get the number of items
- `.isEmpty(): bool` — check if empty

### Deque

- `new Deque<E>()` — create a new double-ended queue
- `.pushFront(item: E): void` — add item to the front
- `.pushBack(item: E): void` — add item to the back
- `.popFront(): E` — remove and return the front item
- `.popBack(): E` — remove and return the back item
- `.peekFront(): E` — view the front item without removing
- `.peekBack(): E` — view the back item without removing
- `.size(): int` — get the number of items
- `.isEmpty(): bool` — check if empty

### PriorityQueue

- `new PriorityQueue<E>()` — create a new min-heap priority queue
- `.push(item: E): void` — insert an item
- `.pop(): E` — remove and return the highest-priority item
- `.peek(): E` — view the highest-priority item without removing
- `.size(): int` — get the number of items
- `.isEmpty(): bool` — check if empty

### Counter

- `new Counter<E>()` — create a new counter
- `.increment(item: E): void` — increment the count for an item
- `.increment(item: E, amount: int): void` — increment by a specific amount
- `.get(item: E): int` — get the count for an item
- `.remove(item: E): void` — remove an item from the counter
- `.size(): int` — get the number of distinct items

### StringBuilder

- `new StringBuilder()` — create a new string builder
- `.append(s: string): void` — append a string
- `.toString(): string` — build the final string

## tt.io — Input/Output

### File

- `File.readFile(path: string): Result<string, string>` — read entire file contents
- `File.writeFile(path: string, content: string): Result<void, string>` — write string to file
- `File.readLines(path: string): array<string>` — read file as array of lines

### Print Functions

- `io::println(s: string): void` — print a string followed by a newline
- `io::print(s: string): void` — print a string without a trailing newline

## tt.file — File System

### Path

- `Path.of(s: string): Path` — create a path from a string
- `.toString(): string` — get the path as a string
- `.exists(): bool` — check if the path exists
- `.isFile(): bool` — check if the path is a file
- `.isDirectory(): bool` — check if the path is a directory
- `.parent(): Path` — get the parent directory
- `.filename(): string` — get the filename component
- `.extension(): string` — get the file extension
- `.join(other: string): Path` — join with another path segment

### Directory

- `Directory.create(path: string): Result<void, string>` — create a directory
- `Directory.remove(path: string): Result<void, string>` — remove a directory
- `Directory.list(path: string): Result<array<string>, string>` — list directory contents
- `Directory.walk(path: string): array<string>` — recursively list all files

## tt.sys — System

### Sys

- `Sys.env(name: string): string` — get an environment variable
- `Sys.setEnv(name: string, value: string): void` — set an environment variable
- `Sys.exit(code: int): void` — exit the program with a status code
- `Sys.args(): array<string>` — get command-line arguments
- `Sys.time(): long` — get current time in milliseconds since epoch
- `Sys.exec(command: string): Result<string, string>` — execute a shell command

## tt.net — Networking

### TcpClient

- `new TcpClient(host: string, port: int)` — create a TCP client
- `.connect(): Result<void, string>` — connect to the server
- `.send(data: string): Result<void, string>` — send data
- `.receive(): Result<string, string>` — receive data
- `.close(): void` — close the connection

### TcpServer

- `new TcpServer(port: int)` — create a TCP server on the given port
- `.start(): Result<void, string>` — start listening
- `.accept(): Result<TcpClient, string>` — accept an incoming connection
- `.close(): void` — stop the server

### HttpClient

- `HttpClient.get(url: string): Result<string, string>` — perform an HTTP GET request
- `HttpClient.post(url: string, body: string): Result<string, string>` — perform an HTTP POST request
- `HttpClient.put(url: string, body: string): Result<string, string>` — perform an HTTP PUT request
- `HttpClient.delete(url: string): Result<string, string>` — perform an HTTP DELETE request

## tt.json — JSON

### Json

- `Json.parse(s: string): Result<JsonValue, string>` — parse a JSON string
- `Json.stringify(value: JsonValue): string` — serialize a JsonValue to a string
- `Json.stringify(value: JsonValue, indent: int): string` — pretty-print with indentation

### JsonValue

- `JsonValue.null(): JsonValue` — create a null JSON value
- `JsonValue.bool(b: bool): JsonValue` — create a boolean JSON value
- `JsonValue.number(n: double): JsonValue` — create a number JSON value
- `JsonValue.string(s: string): JsonValue` — create a string JSON value
- `JsonValue.array(items: array<JsonValue>): JsonValue` — create a JSON array
- `JsonValue.object(entries: HashMap<string, JsonValue>): JsonValue` — create a JSON object
- `.isNull(): bool` — check if null
- `.asBool(): bool` — get as boolean
- `.asNumber(): double` — get as number
- `.asString(): string` — get as string
- `.asArray(): array<JsonValue>` — get as array
- `.asObject(): HashMap<string, JsonValue>` — get as object
- `.get(key: string): JsonValue` — get field from object

## tt.csv — CSV

### CsvReader

- `new CsvReader(path: string)` — create a reader for a CSV file
- `new CsvReader(path: string, delimiter: char)` — create a reader with a custom delimiter
- `.readHeaders(): array<string>` — read the header row
- `.readRow(): Result<array<string>, string>` — read the next row
- `.readAll(): Result<array<array<string>>, string>` — read all rows

### CsvWriter

- `new CsvWriter(path: string)` — create a writer for a CSV file
- `new CsvWriter(path: string, delimiter: char)` — create a writer with a custom delimiter
- `.writeHeaders(headers: array<string>): void` — write the header row
- `.writeRow(row: array<string>): void` — write a row
- `.close(): void` — close the writer

## tt.xml — XML

### Xml

- `Xml.parse(s: string): Result<XmlNode, string>` — parse an XML string
- `Xml.stringify(node: XmlNode): string` — serialize an XmlNode to a string

### XmlNode

- `XmlNode.element(tag: string): XmlNode` — create an element node
- `XmlNode.element(tag: string, children: array<XmlNode>): XmlNode` — element with children
- `XmlNode.text(content: string): XmlNode` — create a text node
- `.tag(): string` — get the element tag name
- `.text(): string` — get the text content
- `.children(): array<XmlNode>` — get child nodes
- `.attr(name: string): string` — get an attribute value
- `.find(query: string): XmlNode` — find a node by CSS-like query
- `.findAll(query: string): array<XmlNode>` — find all matching nodes

## tt.time — Time

### DateTime

- `DateTime.now(): DateTime` — get the current date and time
- `DateTime.of(year: int, month: int, day: int): DateTime` — create from components
- `DateTime.of(year: int, month: int, day: int, hour: int, minute: int, second: int): DateTime` — create with time
- `.year(): int` — get the year
- `.month(): int` — get the month (1–12)
- `.day(): int` — get the day of month
- `.hour(): int` — get the hour
- `.minute(): int` — get the minute
- `.second(): int` — get the second
- `.format(pattern: string): string` — format using a pattern string
- `.plus(d: Duration): DateTime` — add a duration
- `.minus(d: Duration): DateTime` — subtract a duration

### Duration

- `Duration.ofSeconds(n: long): Duration` — create from seconds
- `Duration.ofMinutes(n: long): Duration` — create from minutes
- `Duration.ofHours(n: int): Duration` — create from hours
- `Duration.ofDays(n: int): Duration` — create from days
- `.toSeconds(): long` — convert to total seconds
- `.toMinutes(): long` — convert to total minutes

### Time

- `Time.now(): Time` — get the current wall-clock time
- `.toDateTime(): DateTime` — convert to DateTime

## tt.regex — Regular Expressions

### Regex

- `new Regex(pattern: string)` — compile a regular expression
- `.matches(input: string): bool` — check if the entire string matches
- `.find(input: string): Match` — find the first match
- `.findAll(input: string): array<Match>` — find all matches
- `.replace(input: string, replacement: string): string` — replace matches
- `.split(input: string): array<string>` — split on matches

### Match

- `.group(): string` — get the full match
- `.group(index: int): string` — get a captured group by index
- `.start(): int` — get the start position of the match
- `.end(): int` — get the end position of the match
- `.success(): bool` — check if the match was successful

## tt.math — Mathematics

### Math

- `Math.abs(x: double): double` — absolute value
- `Math.sqrt(x: double): double` — square root
- `Math.pow(base: double, exp: double): double` — exponentiation
- `Math.sin(x: double): double` — sine (radians)
- `Math.cos(x: double): double` — cosine (radians)
- `Math.tan(x: double): double` — tangent (radians)
- `Math.log(x: double): double` — natural logarithm
- `Math.log10(x: double): double` — base-10 logarithm
- `Math.exp(x: double): double` — exponential (e^x)
- `Math.floor(x: double): double` — floor
- `Math.ceil(x: double): double` — ceiling
- `Math.round(x: double): double` — round to nearest integer
- `Math.min(a: double, b: double): double` — minimum
- `Math.max(a: double, b: double): double` — maximum
- `Math.PI: double` — π constant
- `Math.E: double` — Euler's number

### NDArray

See [Scientific Computing](./scientific-computing) for the full NDArray guide.

- `NDArray.fromArray(data: array<double>, shape: (int, ...)): NDArray` — create from flat data and shape
- `NDArray.zeros(shape: (int, ...)): NDArray` — create zero-filled array
- `NDArray.ones(shape: (int, ...)): NDArray` — create one-filled array
- `NDArray.eye(n: int): NDArray` — create identity matrix
- `.shape(): (int, ...)` — get the array shape
- `.get(indices: int...): double` — get an element
- `.set(indices: int..., value: double): void` — set an element
- `.reshape(shape: (int, ...)): NDArray` — reshape the array

### Matrix

See [Scientific Computing](./scientific-computing) for the full Matrix guide.

- `Matrix.fromArray(data: array<array<double>>): Matrix` — create from 2D array
- `Matrix.eye(n: int): Matrix` — create identity matrix
- `Matrix.zeros(rows: int, cols: int): Matrix` — create zero matrix
- `.matmul(other: Matrix): Matrix` — matrix multiplication
- `.lu(): (Matrix, Matrix)` — LU decomposition
- `.qr(): (Matrix, Matrix)` — QR decomposition
- `.eigenvalues(): NDArray` — compute eigenvalues
- `.solve(b: NDArray): NDArray` — solve linear system Ax = b

## tt.random — Random Numbers

### Random

- `new Random()` — create with a time-based seed
- `new Random(seed: long)` — create with a specific seed
- `.nextInt(bound: int): int` — random int in [0, bound)
- `.nextDouble(): double` — random double in [0.0, 1.0)
- `.nextBool(): bool` — random boolean
- `.nextGaussian(): double` — Gaussian (normal) distribution with mean 0, std 1

## tt.assay — Testing

### Assay

Titrate's built-in testing framework. See [Build Tool](./pipette) for running tests with `pipette test`.

- `Assay.describe(name: string, fn: fn(): void): void` — define a test group
- `Assay.it(name: string, fn: fn(): void): void` — define a test case
- `Assay.beforeEach(fn: fn(): void): void` — run before each test in the group
- `Assay.afterEach(fn: fn(): void): void` — run after each test in the group

### TestRunner

- `TestRunner.run(): int` — run all tests, returns exit code (0 = pass)
- `TestRunner.runVerbose(): int` — run with verbose output

Example:

```titrate
Assay.describe("Math", fn() {
    Assay.it("adds correctly", fn() {
        let result = 2 + 2;
        Assay.expect(result == 4);
    });

    Assay.it("multiplies correctly", fn() {
        let result = 3 * 4;
        Assay.expect(result == 12);
    });
});
```

## tt.chem — Computational Chemistry

See [Scientific Computing](./scientific-computing) for the full chemistry guide.

### Atom

- `new Atom(symbol: string, x: double, y: double, z: double)` — create an atom
- `.symbol(): string` — get the element symbol
- `.x(): double` / `.y(): double` / `.z(): double` — get coordinates
- `.atomicNumber(): int` — get the atomic number
- `.mass(): double` — get the atomic mass

### Bond

- `new Bond(atom1: int, atom2: int, order: int)` — create a bond by atom indices and bond order
- `.atom1(): int` — get the first atom index
- `.atom2(): int` — get the second atom index
- `.order(): int` — get the bond order (1=single, 2=double, 3=triple)

### Molecule

- `new Molecule()` — create an empty molecule
- `.addAtom(atom: Atom): int` — add an atom, returns its index
- `.addBond(bond: Bond): void` — add a bond
- `.numAtoms(): int` — get the number of atoms
- `.numBonds(): int` — get the number of bonds
- `.formula(): string` — get the molecular formula
- `.mass(): double` — get the molecular mass

### ForceField

- `new ForceField()` — create a new force field
- `.addBondTerm(i: int, j: int, k: double, r0: double): void` — add a bond stretch term
- `.addAngleTerm(i: int, j: int, k: int, kTheta: double, theta0: double): void` — add an angle bend term
- `.energy(mol: Molecule): double` — compute the total potential energy

### Integrator

- `new Integrator(timestep: double)` — create a Verlet integrator with the given timestep (fs)
- `.timestep(): double` — get the timestep

### MD

- `new MD(mol: Molecule, ff: ForceField, integrator: Integrator)` — set up a molecular dynamics simulation
- `.run(steps: int): void` — run for the given number of steps
- `.positions(): array<(double, double, double)>` — get current atom positions

### RHF

- `new RHF(mol: Molecule)` — create a Restricted Hartree–Fock calculator
- `.setBasis(name: string): void` — set the basis set (e.g. "STO-3G")
- `.compute(): double` — run the SCF calculation, returns the energy

## tt.units — Units of Measure

See [Scientific Computing](./scientific-computing) for the full units guide.

### Base

- `Base.meter(v: double): Derived` — length in meters
- `Base.kilogram(v: double): Derived` — mass in kilograms
- `Base.second(v: double): Derived` — time in seconds
- `Base.kelvin(v: double): Derived` — temperature in kelvin
- `Base.ampere(v: double): Derived` — current in amperes
- `Base.mole(v: double): Derived` — amount in moles

### Derived

- `.to(unit: Derived): double` — convert to another unit
- Arithmetic on derived units produces new derived units (e.g. `meter / second` → m/s)

### Constants

- `Constants.speedOfLight: Derived` — 299792458 m/s
- `Constants.planck: Derived` — 6.62607015e-34 J·s
- `Constants.boltzmann: Derived` — 1.380649e-23 J/K
- `Constants.avogadro: Derived` — 6.02214076e23 /mol
- `Constants.gasConstant: Derived` — 8.314462618 J/(mol·K)
- `Constants.gravitational: Derived` — 6.67430e-11 m³/(kg·s²)
