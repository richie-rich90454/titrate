# optional-variant

The `tt.lang` module provides `Optional<T>` and `Variant` — two discriminated wrapper types for representing values that may be absent or carry a type tag.

```titrate
import tt.lang.Optional;
import tt.lang.Variant;
```

## Optional

A container that may or may not hold a non-null value. Use `Optional` instead of `null` to make the possibility of absence explicit in the type system.

- `Optional<T>.empty(): Optional<T>` — create an empty Optional
- `Optional<T>.of(value: T): Optional<T>` — create an Optional with a non-null value
- `Optional<T>.ofNullable(value: T): Optional<T>` — of if non-null, otherwise empty
- `isPresent(): bool` — true if a value is present
- `isEmpty(): bool` — true if no value is present
- `get(): T` — return the value; throws if empty
- `orElse(defaultValue: T): T` — return value if present, otherwise default
- `orElseGet(supplier: function<T()>): T` — return value if present, otherwise invoke supplier
- `or(supplier: function<Optional<T>()>): Optional<T>` — return this if present, otherwise supplier result
- `map(mapper: function<U(T)>): Optional<U>` — transform the value if present
- `flatMap(mapper: function<Optional<U>(T)>): Optional<U>` — transform to an Optional if present
- `filter(predicate: function<bool(T)>): Optional<T>` — keep only if value matches predicate
- `ifPresent(action: function<void(T)>): void` — run action if value is present
- `ifPresentOrElse(action: function<void(T)>, emptyAction: function<void()>): void` — run action or emptyAction
- `equals(other: Object): bool` — structural equality
- `toString(): String` — `"Optional(value)"` or `"Optional.empty"`

```titrate
let present = Optional<int>::of(42);
io::println(present.isPresent());  // true
io::println(present.get());        // 42

let absent = Optional<int>::empty();
io::println(absent.orElse(0));     // 0

let mapped = present.map(fn(n: int): int => n * 2);
io::println(mapped.get());         // 84
```

## Variant

A tagged union that holds a value together with a string type tag. Useful for lightweight discriminated unions when a full enum is overkill.

- `Variant.of(tag: String, value: Object): Variant` — create a Variant with a tag and value
- `Variant.empty(tag: String): Variant` — create a Variant with a tag but no value
- `typeTag(): String` — return the type tag
- `hasTag(tag: String): bool` — check if the tag matches
- `get(): Object` — return the held value
- `getOrElse(tag: String, defaultValue: Object): Object` — return value if tag matches, otherwise default
- `isEmpty(): bool` — true if no value is held
- `equals(other: Object): bool` — structural equality
- `toString(): String` — `"Variant(tag, value)"` or `"Variant(tag, empty)"`

```titrate
let v = Variant.of("int", 42);
io::println(v.typeTag());           // "int"
io::println(v.hasTag("int"));       // true
io::println(v.getOrElse("int", 0)); // 42

let e = Variant.empty("none");
io::println(e.isEmpty());           // true
```
