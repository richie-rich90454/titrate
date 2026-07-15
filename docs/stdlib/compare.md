# Compare

The `tt.lang.Compare` module mirrors C++20's `<compare>` header. It provides the three comparison category types (`PartialOrdering`, `WeakOrdering`, `StrongOrdering`), the `spaceship` operator analog, and the `is_eq`/`is_neq`/`is_lt`/`is_lteq`/`is_gt`/`is_gteq` predicate functions.

## Import

```titrate
import tt::lang::Compare;
```

## StrongOrdering

`StrongOrdering` is the result of a three-way comparison where `==` implies that the two operands are indistinguishable. It is the strongest of the three categories.

### Constants (static)

- `less(): StrongOrdering`
- `equal(): StrongOrdering`
- `greater(): StrongOrdering`

### Methods

- `init(value: int)` — internal: -1 for less, 0 for equal, 1 for greater
- `isEq(): bool`
- `isNeq(): bool`
- `isLt(): bool`
- `isLteq(): bool`
- `isGt(): bool`
- `isGteq(): bool`
- `compareTo(other: StrongOrdering): int` — ordering on the category itself

## WeakOrdering

`WeakOrdering` is the result of a three-way comparison where `==` means "equivalent" but not necessarily "identical" (e.g., case-insensitive string comparison).

### Constants (static)

- `less(): WeakOrdering`
- `equivalent(): WeakOrdering`
- `greater(): WeakOrdering`

### Methods

Same shape as `StrongOrdering`: `isEq`, `isNeq`, `isLt`, `isLteq`, `isGt`, `isGteq`, `compareTo`.

## PartialOrdering

`PartialOrdering` is the result of a three-way comparison where some pairs of values may be unordered (e.g., NaN compared to any float).

### Constants (static)

- `less(): PartialOrdering`
- `equivalent(): PartialOrdering`
- `greater(): PartialOrdering`
- `unordered(): PartialOrdering`

### Methods

- `isEq(): bool`
- `isNeq(): bool`
- `isLt(): bool`
- `isLteq(): bool`
- `isGt(): bool`
- `isGteq(): bool`
- `isUnordered(): bool`
- `compareTo(other: PartialOrdering): int`

```titrate
let r: StrongOrdering = spaceshipInt(3, 5);
if (r.isLt()) {
    io::println("3 < 5");
}
```

## spaceship functions

The `spaceship` analog of C++'s `operator<=>`.

### spaceship

Compare two `Variant` values using their natural ordering. Returns `WeakOrdering`.

**Parameters:** `a: Variant`, `b: Variant`
**Returns:** `WeakOrdering`

### spaceshipWeak

Same as `spaceship` but explicitly typed as `WeakOrdering`.

### spaceshipPartial

Compare two `Variant` values, returning `PartialOrdering` (allows unordered results).

### spaceshipInt

Compare two `int` values. Returns `StrongOrdering`.

**Parameters:** `a: int`, `b: int`
**Returns:** `StrongOrdering`

```titrate
let r: StrongOrdering = spaceshipInt(7, 7);
io::println(Boolean.toString(r.isEq()));  // true
```

### spaceshipDouble

Compare two `double` values. Returns `PartialOrdering` (because of NaN).

**Parameters:** `a: double`, `b: double`
**Returns:** `PartialOrdering`

## Predicate functions

The `<compare>` header also provides convenience predicates that take a comparison category and return a boolean. They accept any of the three category types via `Variant`.

- `isEq(c: Variant): bool` — true if `c` is `equal`/`equivalent`
- `isNeq(c: Variant): bool` — negation of `isEq`
- `isLt(c: Variant): bool` — true if `c` is `less`
- `isLteq(c: Variant): bool` — true if `c` is `less` or `equal`
- `isGt(c: Variant): bool` — true if `c` is `greater`
- `isGteq(c: Variant): bool` — true if `c` is `greater` or `equal`

```titrate
let r: PartialOrdering = spaceshipDouble(1.5, 2.0);
if (isLt(r)) {
    io::println("less");
}
```

## common_comparison_category

Given several comparison categories, return the strongest one that all of them can be converted to. For example, `StrongOrdering` and `WeakOrdering` together yield `WeakOrdering`; any `PartialOrdering` yields `PartialOrdering`.

**Parameters:** `categories: ArrayList<Variant>` (each entry is a `StrongOrdering`, `WeakOrdering`, or `PartialOrdering`)
**Returns:** `Variant` (one of the three category types)

## category_name

Return the runtime name of a comparison category instance: `"strong_ordering"`, `"weak_ordering"`, `"partial_ordering"`, or `"unknown"`.

**Parameters:** `c: Variant`
**Returns:** `string`

## Conversions

- `strongToWeak(s: StrongOrdering): WeakOrdering` — convert a strong result to weak
- `weakToPartial(w: WeakOrdering): PartialOrdering` — convert a weak result to partial
- `strongToPartial(s: StrongOrdering): PartialOrdering` — convert strong directly to partial
