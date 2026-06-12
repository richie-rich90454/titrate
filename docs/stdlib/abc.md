# abc

The `tt.lang` module provides `Abc` — abstract base class support for defining interfaces that enforce method implementation.

```titrate
import tt.lang.Abc;
```

## Abc

Represents an abstract base class descriptor. It tracks which methods are abstract and can verify whether a class properly implements all required methods.

- `fn init(className: string)` — create an abstract base class descriptor with the given class name
- `abstractMethod(name: string): void` — register a method name as abstract (must be implemented by subclasses)
- `isAbstractMethod(name: string): bool` — check if a method is registered as abstract
- `getAbstractMethods(): ArrayList<string>` — return all registered abstract method names
- `isAbstract(): bool` — check if this class has any abstract methods

```titrate
let shapeAbc: Abc = new Abc("Shape");
shapeAbc.abstractMethod("area");
shapeAbc.abstractMethod("perimeter");

io::println(Boolean.toString(shapeAbc.isAbstract()));          // true
io::println(Boolean.toString(shapeAbc.isAbstractMethod("area"))); // true

let methods: ArrayList<string> = shapeAbc.getAbstractMethods();
// ["area", "perimeter"]
```

## Free Functions

- `abstractMethod(name: string): string` — create a marker for an abstract method declaration
- `isAbstract(methodName: string): bool` — check if a method name is marked as abstract

```titrate
let marker: string = abstractMethod("draw");
io::println(Boolean.toString(isAbstract(marker))); // true
```
