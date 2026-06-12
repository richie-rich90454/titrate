# Standard Library

The Titrate standard library is organized into modules under the `tt` namespace. Each module is documented in its own page — use the links below to explore.

## Core

| Module | Description |
|--------|-------------|
| [lang](../stdlib/lang) | Core types: `String`, `Integer`, `Double`, `Boolean`, `Character`, `Result`, `Iterator`, `Iterable` |
| [operator](../stdlib/operator) | Operator overloading and custom operators |
| [optional-variant](../stdlib/optional-variant) | `Optional` and `Variant` types for safe value handling |

## Collections

| Module | Description |
|--------|-------------|
| [collections](../stdlib/collections) | `ArrayList`, `HashMap`, `Set`, `Vec`, `Deque`, `PriorityQueue`, `Counter`, `StringBuilder` |
| [array](../stdlib/array) | Fixed-size arrays and array utilities |
| [hashset](../stdlib/hashset) | Hash-based set implementation |
| [heapq](../stdlib/heapq) | Heap-based priority queue operations |
| [bisect](../stdlib/bisect) | Binary search for sorted sequences |
| [itertools](../stdlib/itertools) | Iterator adapters and combinators |
| [dataclass](../stdlib/dataclass) | Decorator for auto-generating class boilerplate |

## I/O & File System

| Module | Description |
|--------|-------------|
| [io](../stdlib/io) | File I/O, `println`, `print`, and stream operations |
| [contextlib](../stdlib/contextlib) | Resource management with `with`-style contexts |

## Text & Serialization

| Module | Description |
|--------|-------------|
| [text](../stdlib/text) | Text formatting and manipulation utilities |
| [regex](../stdlib/regex) | Regular expressions: `Regex`, `Match` |
| [serialization](../stdlib/serialization) | JSON, CSV, and XML parsing and writing |
| [pprint](../stdlib/pprint) | Pretty-printing for data structures |

## Math & Science

| Module | Description |
|--------|-------------|
| [math](../stdlib/math) | Mathematical constants, functions, `NDArray`, `Matrix` |
| [complex](../stdlib/complex) | Complex number type and operations |
| [fractions](../stdlib/fractions) | Rational number (`Fraction`) type |
| [statistics](../stdlib/statistics) | Statistical functions: mean, median, variance, etc. |
| [chemistry](../stdlib/chemistry) | Computational chemistry: `Atom`, `Molecule`, `ForceField`, `MD`, `RHF` |
| [units](../stdlib/units) | Units of measure: `Base`, `Derived`, physical `Constants` |

## System & Networking

| Module | Description |
|--------|-------------|
| [system](../stdlib/system) | Environment variables, CLI args, `Sys.exec`, `Sys.exit` |
| [networking](../stdlib/networking) | TCP client/server, HTTP client |
| [concurrent](../stdlib/concurrent) | Concurrency primitives and async utilities |
| [crypto](../stdlib/crypto) | Cryptographic hashes and encryption |

## Date & Time

| Module | Description |
|--------|-------------|
| [datetime](../stdlib/datetime) | `DateTime`, `Duration`, `Time` |

## Random & Utilities

| Module | Description |
|--------|-------------|
| [functools](../stdlib/functools) | Higher-order functions: composition, partial application |
| [logging](../stdlib/logging) | Logging framework |
| [uuid](../stdlib/uuid) | UUID generation and parsing |
| [argparse](../stdlib/argparse) | Command-line argument parsing |
| [algorithms](../stdlib/algorithms) | Common algorithms: sort, search, graph traversal |

## Testing

| Module | Description |
|--------|-------------|
| [testing](../stdlib/testing) | Built-in testing framework (`Assay`) |
| [assert](../stdlib/assert) | Assertion utilities |
