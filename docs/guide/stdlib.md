# Standard Library

## io

- `io::println(s: string): void` — print a string followed by a newline

## Integer

- `Integer.toString(n: int): string` — convert int to string
- `Integer.parseInt(s: string): Result<int, string>` — parse string to int

## Double

- `Double.toString(d: double): string` — convert double to string

## ArrayList

- `new ArrayList<T>()` — create a new list
- `.add(item: T): void` — add an item
- `.get(index: int): T` — get item by index
- `.size(): int` — get the number of items
- `.sort(): void` — sort items (strings: lexicographic)

## HashMap

- `new HashMap<K, V>()` — create a new map
- `.put(key: K, value: V): void` — insert a key-value pair
- `.get(key: K): V` — get value by key
