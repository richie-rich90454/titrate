# Typing

The `tt.lang.Typing` module provides runtime machinery for the type-hint protocol — `Protocol`, `TypedDict`, `Literal`, `Callable`, `Final`, `ClassVar`, `TypeAlias`, `Generic`, `TypeVar`, `Union`, `Optional`, `Any`, `Type`. Since Titrate is dynamically typed, these are runtime type-descriptor objects that capture metadata (name, args, constraints, bound) and offer runtime type-checking helpers (`isInstance`, `isSubtype`). It mirrors Python's `typing` module.

## Import

```titrate
import tt::lang::Typing;
```

## Classes

### TypeDescriptor

Base class for all typing type descriptors.

**Fields:**
- `kind: string` — descriptor kind (e.g. `"Union"`, `"Literal"`, `"TypeVar"`)
- `name: string` — human-readable name

**Constructors:**
- `init(kind: string, name: string)`

**Methods:**
- `toString(): string` — returns the name (e.g. `"Union[int, string]"`)
- `isParameterized(): bool` — default returns `false`

### AnyType extends TypeDescriptor

The `Any` type descriptor — accepts any value.

- `init()`
- `accepts(value: Variant): bool` — always returns `true`

### TypeVar extends TypeDescriptor

A type variable. Holds a name, optional upper bound (`bound=`), and optional constraint set.

**Fields:**
- `bound: Variant`
- `constraints: ArrayList<string>`

**Methods:**
- `withBound(bound: Variant): TypeVar` — set the upper bound (analog of `TypeVar("T", bound=Foo)`); returns `this` for chaining
- `addConstraint(constraint: string): void` — add a constraint (analog of `TypeVar("T", Foo, Bar)`)
- `hasBound(): bool`
- `hasConstraints(): bool`

### Generic extends TypeDescriptor

Base class for generic types. Subclasses register their type parameters via `addTypeVar()`.

**Methods:**
- `addTypeVar(param: TypeVar): void` — register a type parameter
- `setTypeArg(index: int, arg: Variant): void` — bind a concrete type argument to the i-th type parameter
- `isParameterized(): bool` — true if concrete type arguments have been bound

### Union extends TypeDescriptor

A union of types (analog of `typing.Union` / `typing.Optional`). A value is accepted if any member accepts it.

- `init()`
- `addMember(member: TypeDescriptor): void`
- `accepts(value: Variant): bool`

### Optional extends TypeDescriptor

Optional type — either the inner type or `null` (analog of `typing.Optional`).

- `init(inner: TypeDescriptor)`
- `accepts(value: Variant): bool`

### Literal extends TypeDescriptor

A literal type — accepts values equal to one of the recorded literals (analog of `typing.Literal`).

- `init()`
- `addValue(value: Variant): void`
- `accepts(value: Variant): bool`

### Callable extends TypeDescriptor

A callable type descriptor (analog of `typing.Callable`).

- `init(argTypes: ArrayList<TypeDescriptor>, returnType: TypeDescriptor)`
- `accepts(value: Variant): bool`

### Final extends TypeDescriptor

A final type — the inner type cannot be subclassed (analog of `typing.Final`).

- `init(inner: TypeDescriptor)`

### ClassVar extends TypeDescriptor

A class variable type (analog of `typing.ClassVar`).

- `init(inner: TypeDescriptor)`

### TypeAlias extends TypeDescriptor

A type alias (analog of `typing.TypeAlias`).

- `init(name: string, inner: TypeDescriptor)`

### Type extends TypeDescriptor

A class type (analog of `typing.Type`).

- `init(className: string)`

### Protocol extends TypeDescriptor

A structural protocol (analog of `typing.Protocol`).

- `init(name: string)`
- `addMethod(name: string, signature: Callable): void`

### TypedDict extends TypeDescriptor

A typed dictionary (analog of `typing.TypedDict`).

- `init(name: string)`
- `addField(name: string, type: TypeDescriptor): void`

## Functions

### Constructors

- `Typing.typeVar(name: string): TypeVar`
- `Typing.optional(inner: TypeDescriptor): Optional`
- `Typing.union(members: ArrayList<TypeDescriptor>): Union`
- `Typing.literal(values: ArrayList<Variant>): Literal`
- `Typing.callable(argTypes: ArrayList<TypeDescriptor>, returnType: TypeDescriptor): Callable`
- `Typing.final(inner: TypeDescriptor): Final`
- `Typing.classVar(inner: TypeDescriptor): ClassVar`
- `Typing.typeOf(className: string): Type`
- `Typing.protocol(name: string): Protocol`
- `Typing.typedDict(name: string): TypedDict`
- `Typing.typeAlias(name: string, inner: TypeDescriptor): TypeAlias`

### Type-checking

- `Typing.isInstance(value: Variant, descriptor: TypeDescriptor): bool` — runtime `isinstance`-style check
- `Typing.isSubtype(sub: TypeDescriptor, sup: TypeDescriptor): bool` — runtime `issubclass`-style check

### Descriptor kind predicates

- `Typing.isTypeVar(descriptor: TypeDescriptor): bool`
- `Typing.isUnion(descriptor: TypeDescriptor): bool`
- `Typing.isOptional(descriptor: TypeDescriptor): bool`
- `Typing.isLiteral(descriptor: TypeDescriptor): bool`
- `Typing.isCallable(descriptor: TypeDescriptor): bool`
- `Typing.isProtocol(descriptor: TypeDescriptor): bool`
- `Typing.isTypedDict(descriptor: TypeDescriptor): bool`
- `Typing.isFinal(descriptor: TypeDescriptor): bool`
- `Typing.isClassVar(descriptor: TypeDescriptor): bool`
- `Typing.isTypeAlias(descriptor: TypeDescriptor): bool`

### Accessors

- `Typing.getKind(descriptor: TypeDescriptor): string`
- `Typing.getName(descriptor: TypeDescriptor): string`

## Usage Example

```titrate
import tt::lang::Typing;

public fn main(): void {
    let t: TypeVar = Typing.typeVar("T");
    let opt: Optional = Typing.optional(t);
    io::println(opt.toString());  // "Optional[T]"
    let members = new ArrayList<TypeDescriptor>();
    members.add(Typing.typeOf("int"));
    members.add(Typing.typeOf("string"));
    let u: Union = Typing.union(members);
    io::println(u.toString());  // "Union[int, string]"
}
```
