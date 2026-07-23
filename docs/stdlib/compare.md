# `tt::compare` — Three‑Way Comparison

The `Compare` module provides C++20 `operator<=>` (spaceship) semantics.

## Ordering Types

| Type | Description |
|------|-------------|
| `StrongOrdering` | Equality‑comparable, substitutable |
| `WeakOrdering` | Equivalence‑comparable, not substitutable |
| `PartialOrdering` | Some pairs may be unordered (e.g. NaN) |

## Creating Comparisons

```titrate
let ord: StrongOrdering = spaceshipInt(a, b);
let ord: PartialOrdering = spaceshipDouble(x, y);
let ord: StrongOrdering = spaceship(list, other, fn(a, b) => a - b);
```

## Querying the Result

```titrate
is_eq(ord)     // equal
is_neq(ord)    // not equal
is_lt(ord)     // less than
is_lteq(ord)   // less or equal
is_gt(ord)     // greater than
is_gteq(ord)   // greater or equal
```

Each predicate has three overloads (one per ordering type) plus a generic `_any` variant.
