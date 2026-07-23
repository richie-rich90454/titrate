# Variables

Variables are the building blocks of any program — they are how you store data, pass information around and keep track of state. Titrate gives you three distinct ways to declare a variable, and each one communicates something different to the compiler (and to anyone reading your code).

Why three forms? Each one communicates something different to the compiler. `let` keeps code concise with type inference, `var` makes types explicit, and `const` tells the compiler to compute the value at compile time as an immutable binding. This makes your code easier to understand and helps the compiler catch mistakes early.

## let — Type Inference (Mutable)

Use `let` when you want the compiler to infer the type from the value. This is the most common declaration in Titrate — it keeps your code concise while still being fully type-safe.

```titrate
let x = 42;
let greeting = "Hello";
x = 99;  // OK — let bindings are mutable by default
```

`let` infers the type from the assigned value, so you do not need to write it out when it is obvious from context.

::: tip
Use `let` as your default. The type inference keeps code concise, and the mutability gives you flexibility when you need to update the value later.
:::

## var — Explicit Type (Mutable)

Use `var` when you want to be explicit about the type. This is useful when the type is important for readability or when type inference might not give you what you expect:

```titrate
var counter: int = 0;
counter = counter + 1;
counter = counter + 1;
// counter is now two
```

```titrate
var total: double = 0.0;
for (item in prices) {
    total = total + item;
}
```

Both `let` and `var` create mutable bindings. The only difference is that `let` uses type inference while `var` requires an explicit type annotation.

## const — Compile-Time Constant (Immutable)

Use `const` for values that are known at compile time and will never, ever change. `const` creates an **immutable** binding in Titrate. The compiler can inline these values and optimize around them.

### With Type Inference

```titrate
const PI = 3.14159;              // inferred as double
const MAX_SIZE = 1024;           // inferred as int
const APP_NAME = "Titrate";      // inferred as string
```

### With Explicit Type

```titrate
const var PI: double = 3.14159;
const var MAX_SIZE: int = 1024;
const var APP_NAME: string = "Titrate";
```

### Using const let

You can also use `const let` for immutable with type inference:

```titrate
const let PI = 3.14159;
const let MAX_SIZE = 1024;
```

`const` values are computed during compilation, which means they cannot depend on runtime data:

```titrate
const SECONDS_PER_MINUTE = 60;     // OK — literal value
// const now = getCurrentTime();    // ERROR — not a compile-time value
```

## When to Use What

Here is a quick decision guide:

| Keyword | Mutability | Typing | When to use |
|---------|-----------|--------|-------------|
| `let` | Mutable | Type inference | Default choice. Concise, flexible, type-safe. |
| `var` | Mutable | Explicit type | When you want to be explicit about the type. |
| `const` | Immutable | Type inference | For fixed values like mathematical constants, configuration limits, or string literals that the compiler can embed directly. |
| `const var` | Immutable | Explicit type | When you want immutable with explicit type. |

A good rule of thumb: **start with `let`, use `var` when you need an explicit type, and use `const` for values that are truly fixed forever.**

## Type Inference

Titrate can often figure out the type of a variable from the value you assign to it. When the type is obvious from context, you can omit the type annotation:

```titrate
let name = "Titrate";       // inferred as string
let count = 42;             // inferred as int
let ratio = 3.14;           // inferred as double
let flag = true;            // inferred as bool
```

This works with more complex expressions too:

```titrate
let items = new ArrayList<string>();               // inferred as ArrayList<string>
```

::: tip
Type inference is convenient, but do not be afraid to write explicit types when they make your code clearer — especially for function parameters and return types. Explicit types act as documentation and help the compiler give you better error messages.

Note: `let x: type = y` is allowed but not recommended. Use `let x = y` for type inference, or `var x: type = y` when you need explicit typing.
:::

## Variable Scoping

Variables in Titrate are scoped to the block `{ ... }` in which they are declared. Once the block ends, the variable is no longer accessible:

```titrate
public fn main(): void {
    let x = 10;
    if (x > 5) {
        let y = 20;
        io::println(Integer.toString(x + y));  // OK: x and y are both in scope
    }
    // io::println(Integer.toString(y));  // ERROR: y is not in scope here
}
```

You can also shadow variables in inner scopes — a new declaration with the same name hides the outer one:

```titrate
let x = 10;
if (true) {
    let x = "shadowed";  // shadows the outer x
    io::println(x);              // prints "shadowed"
}
io::println(Integer.toString(x));  // prints 10 — outer x is unchanged
```

::: warning
While shadowing is allowed, use it sparingly. Overusing shadowing can make code confusing to read. If you find yourself shadowing frequently, consider using more descriptive variable names instead.
:::

::: tip Try It Yourself
Declare variables using all three keywords and experiment with what the compiler allows and does not allow:
1. Create a `let` binding and reassign it to verify it is mutable.
2. Create a `var` counter and increment it in a `while` loop.
3. Declare a `const` for the number of days in a week, then try to change it (the compiler should reject this).
4. Try omitting type annotations on different kinds of values to see what the compiler infers.
:::
