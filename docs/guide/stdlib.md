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
| `String` | String operations: split, substring, length |
| `ParseError` | Error type returned by parse methods |

### Integer

- `Integer.toString(n: int): string` — convert int to string
- `Integer.parseInt(s: string): Result&lt;int, ParseError&gt;` — parse string to int

### Double

- `Double.toString(d: double): string` — convert double to string
- `Double.parseDouble(s: string): Result&lt;double, ParseError&gt;` — parse string to double

### String

- `String.split(s: string, delimiter: string): array&lt;string&gt;` — split a string on a delimiter
- `String.length(s: string): int` — get string length
- `String.substring(s: string, start: int, end: int): string` — extract a substring

## tt.util — Collections

### ArrayList

- `new ArrayList&lt;E&gt;()` — create a new list
- `.add(item: E): void` — add an item
- `.get(index: int): E` — get item by index
- `.set(index: int, item: E): void` — set item at index
- `.remove(index: int): E` — remove and return item at index
- `.size(): int` — get the number of items
- `.sort(): void` — sort items (strings: lexicographic)

### HashMap

- `new HashMap&lt;K, V&gt;()` — create a new map
- `.put(key: K, value: V): void` — insert a key-value pair
- `.get(key: K): V` — get value by key
- `.containsKey(key: K): bool` — check if key exists
- `.remove(key: K): void` — remove a key
- `.size(): int` — get the number of entries

### Vec

- `new Vec&lt;E&gt;()` — create a new vector (stack-allocated when possible)
- `.push(item: E): void` — push an item
- `.pop(): E` — pop and return the last item
- `.get(index: int): E` — get item by index
- `.size(): int` — get the number of items

### StringBuilder

- `new StringBuilder()` — create a new string builder
- `.append(s: string): void` — append a string
- `.appendInt(n: int): void` — append an integer
- `.appendDouble(d: double): void` — append a double
- `.toString(): string` — build the final string

## tt.io — Input/Output

### File

- `File.readFile(path: string): Result&lt;string&gt;` — read entire file contents
- `File.writeFile(path: string, content: string): Result&lt;void&gt;` — write string to file
- `File.readLines(path: string): array&lt;string&gt;` — read file as array of lines

### Print Functions

- `println(s: string): void` — print a string followed by a newline
- `print(s: string): void` — print a string without a trailing newline
