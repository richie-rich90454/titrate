# Variables

Titrate provides three ways to declare variables: `let`, `var`, and `const`.

## let — Immutable Binding

```titrate
let x: int = 42;
```

## var — Mutable Binding

```titrate
var counter: int = 0;
counter = counter + 1;
```

## const — Compile-Time Constant

```titrate
const PI: double = 3.14159;
```

## Type Inference

When the type can be inferred from the initializer, you may omit it:

```titrate
let name = "Titrate";  // inferred as string
```
