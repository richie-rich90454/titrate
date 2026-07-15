# TypeTraits

The `tt::lang::TypeTraits` module provides a runtime analog of C++ `<type_traits>`. It exposes runtime type-introspection predicates (`isIntegral`, `isFloatingPoint`, `isArithmetic`, `isPointer`, `isReference`, `isClass`, `isEnum`, `isSame`, `isBaseOf`, `isConvertible`) and type-modifier helpers (`addConst`, `removeConst`, `addPointer`, `removePointer`, `decay`, `conditional`, `enableIf`) operating on type-name strings and `Variant` type tags.

Type categorizations are loaded from `lib/tt/data/lang/type_traits.json` via `DataFile` (no hardcoded lookup tables).

## Import

```titrate
import tt::lang::TypeTraits;
```

## API Reference

### Primary Type Categories

Each predicate takes a type-name string and returns a boolean.

- `isIntegral(typeName: string): bool` — true for `bool`, `byte`, `short`, `int`, `long`, `vast`, `uvast`, `u8`, `u16`, `u32`, `u64`, `size`, `char`
- `isFloatingPoint(typeName: string): bool` — true for `float`, `double`, `half`, `quad`
- `isArithmetic(typeName: string): bool` — true for integral or floating-point types
- `isFundamental(typeName: string): bool` — true for primitive, void, or null types
- `isPrimitive(typeName: string): bool` — true for primitive types per AGENTS.md §2.1
- `isPointer(typeName: string): bool` — true if the type ends with `*`
- `isReference(typeName: string): bool` — true if the type starts with `&`
- `isArray(typeName: string): bool` — true if the type contains `[`
- `isVoid(typeName: string): bool` — true for `void`
- `isNull(typeName: string): bool` — true for `null`
- `isSignedIntegral(typeName: string): bool`
- `isUnsignedIntegral(typeName: string): bool`
- `isConst(typeName: string): bool` — true if qualified with `const`
- `isVolatile(typeName: string): bool` — true if qualified with `volatile`

### Composite Type Categories

- `isClass(typeName: string): bool` — heuristic: a PascalCase identifier that is not fundamental, pointer, reference, or array
- `isEnum(typeName: string): bool` — true if registered as an enum via `registerEnum()`
- `registerEnum(typeName: string): void` — register a type name as an enum
- `registerClass(typeName: string): void` — register a type name as a class

### Type Relationships

- `isSame(a: string, b: string): bool` — true if two type names are identical
- `isBaseOf(baseTypeName: string, derivedTypeName: string): bool` — true if `derivedTypeName` derives from `baseTypeName` (checked via the registry populated by `registerBaseOf()`)
- `registerBaseOf(baseTypeName: string, derivedTypeName: string): void` — register an inheritance relationship
- `isConvertible(fromTypeName: string, toTypeName: string): bool` — true if a value of `fromTypeName` is convertible to `toTypeName` (same type, arithmetic widening, base-of relationship, or registered conversion)
- `registerConvertible(fromTypeName: string, toTypeName: string): void` — register a conversion relationship

### Runtime Variant Checks

These predicates inspect a `Variant`'s type tag.

- `typeNameOf(value: Variant): string` — returns the type tag of a `Variant`, or `"null"` for null
- `isIntegralValue(value: Variant): bool`
- `isFloatingPointValue(value: Variant): bool`
- `isArithmeticValue(value: Variant): bool`
- `isClassValue(value: Variant): bool`
- `isEnumValue(value: Variant): bool`

### Type Modifiers

These return new type-name strings; they do not mutate the input.

- `addConst(typeName: string): string` — prepend `const `
- `removeConst(typeName: string): string` — remove a leading `const `
- `addPointer(typeName: string): string` — append `*`
- `removePointer(typeName: string): string` — remove a trailing `*`
- `addReference(typeName: string): string` — prepend `&`
- `removeReference(typeName: string): string` — remove a leading `&`
- `addVolatile(typeName: string): string` — prepend `volatile `
- `removeVolatile(typeName: string): string` — remove a leading `volatile `
- `decay(typeName: string): string` — apply C++ decay semantics: remove reference, remove cv-qualifiers, convert array of T to pointer to T
- `makeSigned(typeName: string): string` — convert unsigned to signed (`u8` → `byte`, etc.)
- `makeUnsigned(typeName: string): string` — convert signed to unsigned (`byte` → `u8`, etc.)

### Conditional / enable_if

- `conditional(condition: bool, trueType: string, falseType: string): string` — mirrors `std::conditional<B, T, F>`
- `enableIf(condition: bool, typeName: string): string` — mirrors `std::enable_if<B, T>::type`; returns empty string when condition is false

### Triviality (heuristic)

- `isTriviallyCopyable(typeName: string): bool` — true for fundamental types
- `isTriviallyDestructible(typeName: string): bool` — true for fundamental types

### Common Type / Array Traits

- `commonType(a: string, b: string): string` — returns the common type of two arithmetic types (wider wins), or the first if they match, else empty
- `rank(typeName: string): int` — number of array dimensions
- `extent(typeName: string, n: int): int` — size of the Nth array dimension (always 0; Titrate type names do not encode sizes)

### Cache / Registry Management

- `reload(): void` — force a reload of the type-traits data file (clears the cache)
- `clearRegistry(): void` — clear the in-memory registry (categories + registered enums/classes/bases)

## Usage Examples

### Checking Type Categories

```titrate
import tt::lang::TypeTraits;
import tt::io::IO;

public fn main(): void {
    IO.println(TypeTraits.isIntegral("int"));          // true
    IO.println(TypeTraits.isFloatingPoint("double"));   // true
    IO.println(TypeTraits.isArithmetic("string"));      // false
    IO.println(TypeTraits.isPointer("int*"));           // true
    IO.println(TypeTraits.isReference("&int"));         // true
    IO.println(TypeTraits.isVoid("void"));              // true
}
```

### Type Modifiers

```titrate
import tt::lang::TypeTraits;

let t: string = TypeTraits.addConst("int");        // "const int"
let t2: string = TypeTraits.removeConst(t);        // "int"
let ptr: string = TypeTraits.addPointer("long");   // "long*"
let ref: string = TypeTraits.addReference("int");  // "&int"
let d: string = TypeTraits.decay("const &int");    // "int"
```

### Registering Inheritance and Conversions

```titrate
import tt::lang::TypeTraits;

TypeTraits.registerBaseOf("Animal", "Dog");
TypeTraits.registerBaseOf("Animal", "Cat");
TypeTraits.registerConvertible("int", "double");

io::println(TypeTraits.isBaseOf("Animal", "Dog"));      // true
io::println(TypeTraits.isConvertible("int", "double")); // true
```

### Conditional Type Selection

```titrate
import tt::lang::TypeTraits;

// Choose int if condition is true, else double
let chosen: string = TypeTraits.conditional(useInt, "int", "double");

// enable_if pattern: returns "string" only if condition holds
let enabled: string = TypeTraits.enableIf(isStringMode, "string");
if (String.length(enabled) > 0) {
    io::println("enabled type: " + enabled);
}
```

### Inspecting a Variant

```titrate
import tt::lang::TypeTraits;
import tt::lang::Variant;

let v: Variant = Variant.of(42);
io::println(TypeTraits.typeNameOf(v));          // "int"
io::println(TypeTraits.isIntegralValue(v));     // true
io::println(TypeTraits.isFloatingPointValue(v)); // false
```
