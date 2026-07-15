# Concepts

The `tt::lang::Concepts` module provides a runtime analog of C++ `<concepts>`. It exposes runtime predicate functions mirroring the standard library concepts (`same_as`, `derived_from`, `convertible_to`, `common_with`, `integral`, `signed_integral`, `unsigned_integral`, `floating_point`, `assignable_from`, `swappable`, `destructible`, `constructible_from`, `default_initializable`, `move_constructible`, `copy_constructible`, `regular`, `semiregular`, `equality_comparable`, `totally_ordered`, `movable`, `copyable`) plus iterator and range concept predicates.

Implementations delegate to `tt::lang::TypeTraits` for type categorization. Titrate is GC-managed, so object-lifetime concepts (`destructible`, `constructible_from`, etc.) use conservative heuristics that return true for all non-void types.

## Import

```titrate
import tt::lang::Concepts;
```

## API Reference

### Core Concept Predicates

- `sameAs(a: string, b: string): bool` — `same_as<T, U>`: two type names are identical
- `derivedFrom(derived: string, base: string): bool` — `derived_from<Derived, Base>`
- `convertibleTo(from: string, to: string): bool` — `convertible_to<From, To>`
- `commonWith(a: string, b: string): bool` — `common_with<T, U>`: T and U share a common type
- `commonReferenceWith(a: string, b: string): bool` — alias for `commonWith` in this runtime model

### Arithmetic Concepts

- `integral(typeName: string): bool` — `integral<T>`
- `signedIntegral(typeName: string): bool` — `signed_integral<T>`
- `unsignedIntegral(typeName: string): bool` — `unsigned_integral<T>`
- `floatingPoint(typeName: string): bool` — `floating_point<T>`
- `arithmetic(typeName: string): bool` — `arithmetic<T>` (integral or floating-point)

### Object-Lifetime Concepts

Conservative heuristics; Titrate is GC-managed.

- `destructible(typeName: string): bool` — always true for non-void types
- `constructibleFrom(typeName: string, argTypes: ArrayList<string>): bool` — true for non-void types
- `defaultInitializable(typeName: string): bool` — true for non-void, non-null, non-reference types
- `moveConstructible(typeName: string): bool` — true for non-void types
- `copyConstructible(typeName: string): bool` — true for non-void types
- `movable(typeName: string): bool` — move-constructible and move-assignable
- `copyable(typeName: string): bool` — copy-constructible, copy-assignable, and movable
- `assignableFrom(target: string, source: string): bool` — `assignable_from<T, U>`
- `swappable(typeName: string): bool` — true for non-void types
- `swappableWith(a: string, b: string): bool` — `swappable_with<T, U>`

### Comparison Concepts

- `equalityComparable(typeName: string): bool` — true for arithmetic, `string`, and `bool` types
- `equalityComparableWith(a: string, b: string): bool` — `equality_comparable_with<T, U>`
- `totallyOrdered(typeName: string): bool` — true for arithmetic types and `string`
- `totallyOrderedWith(a: string, b: string): bool` — `totally_ordered_with<T, U>`

### Composite Concepts

- `semiregular(typeName: string): bool` — `copyable<T> && default_initializable<T>`
- `regular(typeName: string): bool` — `semiregular<T> && equality_comparable<T>`

### Iterator Concepts

Heuristic: a type name is an iterator if it ends with `"Iterator"`.

- `iterator(typeName: string): bool`
- `inputIterator(typeName: string): bool`
- `outputIterator(typeName: string): bool`
- `forwardIterator(typeName: string): bool`
- `bidirectionalIterator(typeName: string): bool` — heuristic: contains `List`/`Tree`/`Map`/`Deque`
- `randomAccessIterator(typeName: string): bool` — heuristic: contains `Array`/`Vec`
- `contiguousIterator(typeName: string): bool`
- `sentinelFor(sentinel: string, iter: string): bool`
- `sizedSentinelFor(sentinel: string, iter: string): bool`
- `indirectlyReadable(typeName: string): bool`
- `indirectlyWritable(iter: string, typeName: string): bool`

### Range Concepts

Heuristic: a type name is a range if it ends with `"Range"` or is a known collection type (`ArrayList`, `HashMap`, etc.).

- `range(typeName: string): bool`
- `view(typeName: string): bool` — ends with `View` or `Range`
- `sizedRange(typeName: string): bool`
- `commonRange(typeName: string): bool`
- `inputRange(typeName: string): bool`
- `outputRange(typeName: string): bool`
- `forwardRange(typeName: string): bool`
- `bidirectionalRange(typeName: string): bool`
- `randomAccessRange(typeName: string): bool`
- `contiguousRange(typeName: string): bool`

### Callable Concepts

- `invocable(callableType: string, argTypes: ArrayList<string>): bool` — heuristic: function types start with `fn(`
- `regularInvocable(callableType: string, argTypes: ArrayList<string>): bool` — alias for `invocable`
- `predicate(callableType: string, argTypes: ArrayList<string>): bool` — invocable and returns bool

## Usage Examples

### Checking Arithmetic Concepts

```titrate
import tt::lang::Concepts;
import tt::io::IO;

public fn main(): void {
    IO.println(Concepts.integral("int"));          // true
    IO.println(Concepts.floatingPoint("double"));   // true
    IO.println(Concepts.arithmetic("string"));      // false
    IO.println(Concepts.signedIntegral("int"));     // true
    IO.println(Concepts.unsignedIntegral("u32"));   // true
}
```

### Object-Lifetime and Composite Concepts

```titrate
import tt::lang::Concepts;

io::println(Concepts.destructible("int"));          // true
io::println(Concepts.defaultInitializable("int"));  // true
io::println(Concepts.copyable("string"));           // true
io::println(Concepts.semiregular("int"));           // true
io::println(Concepts.regular("int"));               // true (semiregular + equality_comparable)
```

### Comparison Concepts

```titrate
import tt::lang::Concepts;

io::println(Concepts.equalityComparable("int"));     // true
io::println(Concepts.totallyOrdered("string"));      // true
io::println(Concepts.totallyOrderedWith("int", "long"));  // true (common type exists)
```

### Iterator and Range Concepts

```titrate
import tt::lang::Concepts;

io::println(Concepts.iterator("ArrayListIterator"));     // true
io::println(Concepts.randomAccessIterator("VecIterator")); // true (contains "Vec")
io::println(Concepts.bidirectionalIterator("LinkedListIterator")); // true (contains "List")

io::println(Concepts.range("ArrayList"));   // true
io::println(Concepts.range("HashMap"));     // true
io::println(Concepts.sizedRange("Vec"));    // true
io::println(Concepts.view("StringView"));   // true (ends with "View")
```

### Callable Concepts

```titrate
import tt::lang::Concepts;
import tt::util::ArrayList;

let argTypes: ArrayList<string> = new ArrayList<string>();
argTypes.add("int");
io::println(Concepts.invocable("fn(int): void", argTypes));  // true (starts with "fn(")
io::println(Concepts.predicate("fn(int): bool", argTypes));  // true
```
