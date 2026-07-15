# Types

The `tt.lang.Types` module provides dynamic type-tagged wrappers that mirror Python's `types` module — `ModuleType`, `FunctionType`, `MethodType`, `GeneratorType`, `CoroutineType`, `AsyncGeneratorType`. Since Titrate values are dynamically typed, these classes serve as wrappers that capture the runtime metadata Python exposes on these built-in types (name, module, qualname, doc, etc.). The `isXType()` predicates let callers discriminate among wrapped values, and the `newXxx()` factory functions mirror `types.new_class()` / `types.new_module()`.

## Import

```titrate
import tt::lang::Types;
```

## Classes

### ModuleType

Analog of `types.ModuleType`. Represents a loaded module with a name and an attribute dictionary.

**Fields:**
- `name: string`
- `doc: string`
- `file: string`

**Constructors:**
- `init(name: string)`

**Methods:**
- `setAttr(key: string, value: Variant): void` — set an attribute (analog of `module.__dict__[key] = value`)
- `getAttr(key: string): Variant` — get an attribute, or `null` if not present
- `hasAttr(key: string): bool` — true if the module has the given attribute
- `delAttr(key: string): bool` — remove an attribute; returns `true` if removed
- `dir(): ArrayList<string>` — return all attribute names
- `toString(): string` — returns `"<module 'name'>"`

### FunctionType

Analog of `types.FunctionType`. Represents a user-defined function with its name, module, qualified name, docstring, and parameter list.

**Fields:**
- `name: string`
- `qualname: string`
- `module: string`
- `doc: string`
- `parameters: ArrayList<string>`
- `defaults: Variant`
- `closure: Variant`
- `globals: HashMap<string, Variant>`
- `annotations: HashMap<string, Variant>`

**Constructors:**
- `init(name: string)`

**Methods:**
- `setParameters(params: ArrayList<string>): void`
- `getParameters(): ArrayList<string>`
- `setAnnotation(key: string, value: Variant): void`
- `getAnnotation(key: string): Variant` — returns `null` if not present
- `toString(): string` — returns `"<function 'qualname'>"`

### MethodType

Analog of `types.MethodType`. Represents a bound method: a function paired with the instance it is bound to (`__self__` and `__func__`).

**Fields:**
- `function: FunctionType`
- `instance: Variant`
- `name: string`

**Constructors:**
- `init(function: FunctionType, instance: Variant)`

**Methods:**
- `getFunction(): FunctionType` — the bound function (`__func__`)
- `getInstance(): Variant` — the bound instance (`__self__`)
- `getName(): string` — the method name (`__name__`)
- `toString(): string` — returns `"<bound method 'name' of instance>"`

### GeneratorType

Analog of `types.GeneratorType`. Represents a generator-iterator modeled as a wrapper around a function value and a position cursor.

**Fields:**
- `function: FunctionType`
- `position: int`
- `currentValue: Variant`
- `finished: bool`
- `sentValue: Variant`

**Constructors:**
- `init(function: FunctionType)`

**Methods:**
- `next(): Variant` — advance and return the next yielded value, or `null` if exhausted
- `send(value: Variant): Variant` — send a value into the generator
- `close(): void` — close the generator (signals `GeneratorExit`)
- `throw(exc: string): Variant` — throw an exception at the current yield point
- `isFinished(): bool` — true if exhausted
- `toString(): string` — returns `"<generator object 'name'>"`

### CoroutineType

Analog of `types.CoroutineType`. Represents a coroutine created by an async function.

**Fields:**
- `function: FunctionType`
- `name: string` / `qualname: string`
- `position: int`
- `currentValue: Variant`
- `finished: bool` / `running: bool`
- `sentValue: Variant`

**Methods:**
- `send(value: Variant): Variant` — throws `StopIteration` if finished
- `throw(exc: string): Variant`
- `close(): void`
- `isFinished(): bool`
- `toString(): string` — returns `"<coroutine object 'qualname'>"`

### AsyncGeneratorType

Analog of `types.AsyncGeneratorType`. Combines async nature with yield-driven iteration.

**Methods:**
- `anext(): Variant` — advance and return the next yielded value
- `asend(value: Variant): Variant`
- `athrow(exc: string): Variant`
- `aclose(): void`
- `isFinished(): bool`
- `toString(): string` — returns `"<async generator object 'qualname'>"`

## Type-discrimination Predicates

Each predicate returns `true` if `value` is an instance of the corresponding wrapped type.

- `Types.isModuleType(value: Variant): bool`
- `Types.isFunctionType(value: Variant): bool`
- `Types.isMethodType(value: Variant): bool`
- `Types.isGeneratorType(value: Variant): bool`
- `Types.isCoroutineType(value: Variant): bool`
- `Types.isAsyncGeneratorType(value: Variant): bool`

## Factory Functions

- `Types.newModule(name: string): ModuleType` — create a new empty module (analog of `types.ModuleType(name)`)
- `Types.newFunction(name: string, moduleName: string): FunctionType` — create a new function-type descriptor (analog of `types.FunctionType`)
- `Types.newMethod(function: FunctionType, instance: Variant): MethodType` — create a new bound method (analog of `types.MethodType(function, instance)`)
- `Types.newGenerator(function: FunctionType): GeneratorType` — create a new generator wrapper
- `Types.newCoroutine(function: FunctionType, name: string): CoroutineType` — create a new coroutine wrapper
- `Types.newAsyncGenerator(function: FunctionType, name: string): AsyncGeneratorType` — create a new async-generator wrapper

## Introspection Helpers

- `Types.typeName(value: Variant): string` — return the type name (analog of `type(value).__name__`); returns `""` for null/unwrapped values
- `Types.qualname(value: Variant): string` — return the qualified name (analog of `__qualname__`)
- `Types.moduleName(value: Variant): string` — return the module name (analog of `__module__`)

## Usage Example

```titrate
import tt::lang::Types;

public fn main(): void {
    let m: ModuleType = Types.newModule("mymod");
    m.setAttr("version", "1.0");
    io::println(Boolean.toString(m.hasAttr("version")));  // true
    io::println(m.toString());                            // "<module 'mymod'>"
    let f: FunctionType = Types.newFunction("greet", "mymod");
    io::println(Types.typeName(f));                       // "function"
}
```
