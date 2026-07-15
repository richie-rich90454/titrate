# TypeIndex

The `tt::lang::TypeIndex` module provides a runtime analog of C++ `<typeindex>`. It wraps a type name string in a comparable, hashable handle usable as a `HashMap` key. Instances are interned via `TypeIndex.of(typeName)` so that two `TypeIndex` values for the same type name are reference-equal (and therefore usable as `HashMap` keys without custom equality).

## Import

```titrate
import tt::lang::TypeIndex;
```

## API Reference

### `TypeIndex`

Wraps a type name string in a comparable, hashable handle.

**Field:**
- `typeName: string` — the wrapped type name

**Constructor (private):**
- `init(typeName: string)` — instances are created via the `of()` factory

**Methods:**
- `name(): string` — returns the wrapped type name
- `equals(other: TypeIndex): bool` — returns true if this `TypeIndex` wraps the same type name as `other`. (Reference equality also works because instances are interned, but this method is safe for non-interned instances too.)
- `compareTo(other: TypeIndex): int` — compares by lexicographic order of the wrapped type name. Returns `-1`, `0`, or `1`.
- `hash(): int` — returns a hash code suitable for use in hash-based containers. Matches `std::type_index::hash_code()` semantics. (Named `hash` per Titrate convention — AGENTS.md §5.8 prohibits the Java-style `hashCode` name.)
- `hash_code(): int` — C++-style alias for `hash()`. Matches `std::type_index::hash_code()`.
- `toString(): string` — returns `"typeindex(<typeName>)"`
- `is(typeName: string): bool` — returns true if this `TypeIndex` wraps the given type name

### Free Functions

#### `of(typeName: string): TypeIndex`

Returns the interned `TypeIndex` for the given type name. Two calls with the same type name return the same (reference-equal) instance. This is the canonical way to obtain a `TypeIndex`.

#### `equals(a: TypeIndex, b: TypeIndex): bool`

Returns true if two `TypeIndex` values wrap the same type name.

#### `compareTo(a: TypeIndex, b: TypeIndex): int`

Compares two `TypeIndex` values by their wrapped type name.

#### `hash(idx: TypeIndex): int`

Returns the hash code of a `TypeIndex` (0 if null).

#### `clearCache(): void`

Clears the interning cache. Existing `TypeIndex` instances remain valid but subsequent calls to `of()` will create new instances.

#### `size(): int`

Returns the number of interned `TypeIndex` values.

#### `names(): ArrayList<string>`

Returns all interned type names.

#### `isInterned(typeName: string): bool`

Returns true if a `TypeIndex` has been interned for the given type name.

## Usage Examples

### Creating and Comparing TypeIndex Values

```titrate
import tt::lang::TypeIndex;
import tt::io::IO;

public fn main(): void {
    let a: TypeIndex = TypeIndex.of("int");
    let b: TypeIndex = TypeIndex.of("int");
    let c: TypeIndex = TypeIndex.of("string");

    IO.println(a == b);           // true  (interned — same reference)
    IO.println(a.equals(b));      // true
    IO.println(a.equals(c));      // false
    IO.println(a.name());         // int
    IO.println(a.toString());     // typeindex(int)
}
```

### Using TypeIndex as a HashMap Key

```titrate
import tt::lang::TypeIndex;
import tt::util::HashMap;

let map: HashMap<TypeIndex, string> = new HashMap<TypeIndex, string>();
map.put(TypeIndex.of("int"), "integer type");
map.put(TypeIndex.of("string"), "text type");

let key: TypeIndex = TypeIndex.of("int");
io::println(map.get(key));   // "integer type"
```

### Ordering and Hashing

```titrate
import tt::lang::TypeIndex;

let a: TypeIndex = TypeIndex.of("apple");
let b: TypeIndex = TypeIndex.of("banana");

io::println(a.compareTo(b));   // -1 ("apple" < "banana")
io::println(a.hash());          // stable hash code
io::println(TypeIndex.hash(a)); // same value via free function
```

### Inspecting the Interning Registry

```titrate
import tt::lang::TypeIndex;

TypeIndex.of("int");
TypeIndex.of("string");
TypeIndex.of("bool");

io::println(TypeIndex.size());        // 3
io::println(TypeIndex.isInterned("int"));     // true
io::println(TypeIndex.isInterned("double"));  // false

// Iterate over all interned names
let names: ArrayList<string> = TypeIndex.names();
for (n in names) {
    io::println(n);
}
```
