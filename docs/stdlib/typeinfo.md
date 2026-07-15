# TypeInfo

The `tt.lang.TypeInfo` module mirrors C++'s `<typeinfo>` header. It provides the `TypeInfo` class (returned by the `typeId` function), the `BadCast` and `BadTypeId` exceptions, and helper functions `of`, `typeId`, and `typeIdOf`.

## Import

```titrate
import tt::lang::TypeInfo;
```

## TypeInfo

`TypeInfo` describes a type at runtime. Two `TypeInfo` values are equal if they describe the same type. The runtime representation stores a canonical type name string that is unique per declared type.

### Construction

`TypeInfo` cannot be constructed directly by user code; use `typeId`, `typeIdOf`, or `of` to obtain instances.

### Methods

- `name(): string` — the implementation-defined mangled name of the type
- `rawName(): string` — the unmangled type identifier
- `equals(other: TypeInfo): bool` — true if both describe the same type
- `before(other: TypeInfo): bool` — ordering predicate (implementation-defined, useful as a `HashMap` key ordering)
- `hash(): int` — hash code suitable for `HashMap` keys
- `toString(): string` — human-readable form `"TypeInfo(<name>)"`

```titrate
let info: TypeInfo = typeId(42);
io::println(info.name());  // "int"
let same: TypeInfo = typeId(7);
io::println(Boolean.toString(info.equals(same)));  // true
```

## Top-level functions

### typeId

Return the `TypeInfo` for the runtime type of `value`.

**Parameters:** `value: Variant`
**Returns:** `TypeInfo`

```titrate
let i: TypeInfo = typeId("hello");
io::println(i.name());  // "string"
```

### typeIdOf

Return the `TypeInfo` for the declared type `T`. Useful when you have no instance but want the type info for a known type name.

**Parameters:** `typeName: string`
**Returns:** `TypeInfo`

```titrate
let i: TypeInfo = typeIdOf("ArrayList<string>");
io::println(i.name());
```

### of

Alias for `typeId`. Returns the `TypeInfo` for the runtime type of `value`.

**Parameters:** `value: Variant`
**Returns:** `TypeInfo`

## Exceptions

### BadCast

Thrown when a runtime cast to a reference type fails (the `as` operator on a reference that does not actually refer to a subobject of the target type).

- `BadCast.init(message: string)`
- `BadCast.what(): string` — the message passed at construction

### BadTypeId

Thrown when `typeId` is applied to a null or untyped value.

- `BadTypeId.init(message: string)`
- `BadTypeId.what(): string`

```titrate
try {
    let v: Variant = null;
    let i: TypeInfo = typeId(v);
} catch (e: string) {
    io::println(e);  // "BadTypeId: typeId applied to null"
}
```

## Notes

- Type identity is based on the canonical declared name; type aliases that resolve to the same underlying type compare equal.
- `TypeInfo` instances are immutable and cached by the runtime so that `typeId(x) == typeId(y)` whenever `x` and `y` share a runtime type.
- The mangled `name()` is stable across runs of the same Titrate build.
