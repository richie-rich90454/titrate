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
