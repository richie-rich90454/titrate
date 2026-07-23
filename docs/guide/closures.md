# Closures

Closures are anonymous functions that can capture variables from their enclosing scope. Think of them as lightweight, inline functions you can pass around like values. They're perfect for callbacks, transformations and any time you want to customize behavior without defining a whole named function.

If you've used lambdas in C++, arrow functions in ECMAScript, or closures in Rust, you'll find Titrate's closures familiar — with a few differences worth knowing about.

## Syntax

Titrate provides two forms of closure syntax:

**Expression form** — for single-expression bodies. Concise and readable:

```titrate
fn(params) => expr
```

**Block form** — for multi-statement bodies. Use this when you need local variables or multiple steps:

```titrate
fn(params) {
    stmt1;
    stmt2;
    return value;
}
```

### When to Use Each Form

- Use the **expression form** when the body is a single computation — it's shorter and easier to read at a glance.
- Use the **block form** when you need intermediate variables, multiple statements, or early returns.

```titrate
// Expression form — clean for simple transformations
let double = fn(x: int): int => x * 2;

// Block form — necessary for multi-step logic
let greet = fn(name: string): string {
    let message = "Hello, " + name + "!";
    return message;
};
```

::: tip
If you find yourself writing a block-form closure that's more than a few lines, consider extracting it into a named function. Named functions are easier to test, reuse and document.
:::

## Basic Usage

### Expression Closures

```titrate
let double = fn(x: int): int => x * 2;
io::println(Integer.toString(double(5)));  // 10
```

### Block Closures

```titrate
let greet = fn(name: string): string {
    let message = "Hello, " + name + "!";
    return message;
};
io::println(greet("Titrate"));  // Hello, Titrate!
```

### Try It Yourself

Write a closure that takes a string and returns its length, then use it with a list of names:

```titrate
import tt::util::ArrayList;

public fn main(): void {
    let nameLength = fn(s: string): int => String.length(s);

    let names = new ArrayList<string>();
    names.add("Alice");
    names.add("Bob");
    names.add("Carol");

    names.forEach(fn(name: string): void {
        io::println(name + " has length " + Integer.toString(nameLength(name)));
    });
}
```

Try modifying the closure to return the name in uppercase instead.

## Capturing Variables

Closures can reference variables from the enclosing scope. The captured variable's value is the value it has at the time the closure executes:

```titrate
fn makeCounter(): fn(): int {
    var count: int = 0;
    return fn(): int {
        count = count + 1;
        return count;
    };
}

let counter = makeCounter();
io::println(Integer.toString(counter()));  // 1
io::println(Integer.toString(counter()));  // 2
io::println(Integer.toString(counter()));  // 3
```

Closures capture by reference, so mutations inside the closure are visible outside:

```titrate
var total: int = 0;
let add = fn(n: int): void {
    total = total + n;
};
add(3);
add(7);
io::println(Integer.toString(total));  // 10
```

This is what makes closures so powerful — they "remember" the environment where they were created.

## Using Closures with Collections

Closures are commonly passed to collection methods. This is where they really shine — instead of writing loops, you describe *what* you want to do and the collection handles the *how*.

### ArrayList.forEach

```titrate
let names = new ArrayList<string>();
names.add("Alice");
names.add("Bob");
names.add("Carol");

names.forEach(fn(name: string): void {
    io::println("Hello, " + name);
});
```

### Filtering and Transforming

```titrate
let numbers = new ArrayList<int>();
numbers.add(1);
numbers.add(2);
numbers.add(3);
numbers.add(4);
numbers.add(5);

numbers.forEach(fn(n: int): void {
    if (n % 2 == 0) {
        io::println(Integer.toString(n));  // prints 2, 4
    }
});
```

## Closures as Function Parameters

You can declare functions that accept closures as parameters. This is the foundation of higher-order programming — functions that take functions as arguments:

```titrate
fn apply(a: int, f: fn(int): int): int {
    return f(a);
}

let result = apply(10, fn(x: int): int => x * x);
io::println(Integer.toString(result));  // 100
```

### Callback Pattern

```titrate
fn repeat(n: int, action: fn(int): void): void {
    var i: int = 0;
    while (i < n) {
        action(i);
        i = i + 1;
    }
}

repeat(3, fn(i: int): void {
    io::println("Iteration " + Integer.toString(i));
});
```

## Common Closure Patterns

### The Transformer

Pass a closure to transform data:

```titrate
fn transform<T, R>(list: ArrayList<T>, f: fn(T): R): ArrayList<R> {
    let result: ArrayList<R> = new ArrayList<R>();
    var i: int = 0;
    while (i < list.size()) {
        result.add(f(list.get(i)));
        i = i + 1;
    }
    return result;
}

let nums = new ArrayList<int>();
nums.add(1);
nums.add(2);
nums.add(3);

let strings = transform(nums, fn(n: int): string => Integer.toString(n));
```

### The Predicate

Use a closure to test a condition:

```titrate
fn findFirst<T>(list: ArrayList<T>, predicate: fn(T): bool): T {
    var i: int = 0;
    while (i < list.size()) {
        if (predicate(list.get(i))) {
            return list.get(i);
        }
        i = i + 1;
    }
    return null;
}

let items = new ArrayList<string>();
items.add("apple");
items.add("banana");
items.add("cherry");

let found = findFirst(items, fn(s: string): bool => String.length(s) > 5);
io::println(found);  // banana
```

### The Accumulator

Use a closure to build up a result:

```titrate
fn reduce<T>(list: ArrayList<T>, initial: T, f: fn(T, T): T): T {
    var result: T = initial;
    var i: int = 0;
    while (i < list.size()) {
        result = f(result, list.get(i));
        i = i + 1;
    }
    return result;
}

let nums = new ArrayList<int>();
nums.add(1);
nums.add(2);
nums.add(3);
nums.add(4);

let sum = reduce(nums, 0, fn(a: int, b: int): int => a + b);
io::println(Integer.toString(sum));  // 10
```

## Capture Semantics

Closures in Titrate capture variables **by reference** from the enclosing scope. This means the closure holds a reference to the original variable, not a copy of its value. Any mutations made inside the closure are immediately visible outside, and vice versa.

### How Captures Work at the Bytecode Level

When the compiler encounters a closure that references outer variables, it emits two opcodes:

- **`CLOSURE_NEW_CAPTURED`** — creates a new closure object, recording the function index and the number of captured variables. Operands: `u16` function index + `u8` captured count.
- **`CLOSURE_CAPTURE`** — copies a local variable's value into the closure's captured environment. Operand: `u8` local slot index.

Each captured variable becomes an **upvalue** inside the closure. The closure body accesses captured variables through `GET_UPVALUE` and `SET_UPVALUE` opcodes instead of `LOAD_LOCAL` / `STORE_LOCAL`.

### Capture Analysis

The analyzer automatically detects which variables from outer scopes are referenced inside a closure body. Only variables that are actually used are captured — unused outer variables are not included in the closure's captured environment.

```titrate
var x: int = 10;
var y: int = 20;

// Only x is captured; y is not referenced inside the closure
let closure = fn(): int {
    return x + 1;
};
```

### Shared and Mutable Captures

Because captures are by reference, multiple closures can share the same captured variable:

```titrate
fn makeCounters(): (fn(): int, fn(): void) {
    var count: int = 0;
    let increment = fn(): int {
        count = count + 1;
        return count;
    };
    let reset = fn(): void {
        count = 0;
    };
    return (increment, reset);
}

let (inc, reset) = makeCounters();
io::println(Integer.toString(inc()));  // 1
io::println(Integer.toString(inc()));  // 2
reset();
io::println(Integer.toString(inc()));  // 1
```

Both `increment` and `reset` share the same `count` variable through their captured environments.

## Borrow Checker Implications

Because closures capture variables by reference from the enclosing scope, the borrow checker enforces the same ownership rules:

- A closure that only **reads** a captured variable requires a shared borrow.
- A closure that **writes** to a captured variable requires a mutable borrow.
- You cannot have a mutable borrow and another borrow (shared or mutable) to the same variable at the same time.

```titrate
var x: int = 10;
let reader = fn(): int => x;        // shared borrow of x
let writer = fn(): void { x = 20; }; // mutable borrow of x
// Cannot use reader and writer simultaneously
```

If a closure needs to take ownership of a captured value, move it explicitly:

```titrate
fn makeGreeter(greeting: string): fn(string): string {
    let owned = greeting;  // take ownership
    return fn(name: string): string => owned + ", " + name + "!";
}

let hi = makeGreeter("Hi");
io::println(hi("Alice"));  // Hi, Alice!
```

## Type Inference

When a closure is passed directly to a function, the compiler can often infer the parameter and return types:

```titrate
let nums = new ArrayList<int>();
nums.add(1);
nums.add(2);
nums.add(3);

// Type inferred from ArrayList.forEach's signature
nums.forEach(fn(n) {
    io::println(Integer.toString(n));
});
```

## Closures vs Named Functions

When should you use a closure, and when should you write a named function? Here's a guide:

### Use a Closure When:

- **The logic is short and used once.** A one-line transformation like `fn(x: int): int => x * 2` doesn't need a name.
- **You need to capture variables from the enclosing scope.** Closures can "see" variables around them; named functions can't.
- **You're passing behavior as an argument.** Callbacks, predicates and transformers are natural closure territory.

### Use a Named Function When:

- **The logic is reused in multiple places.** A named function is easier to call from anywhere.
- **The body is more than a few lines.** Named functions are easier to read, test and debug.
- **You want to document the behavior.** A good function name is worth a thousand comments.

```titrate
// Closure — short, used inline, captures 'prefix'
var prefix: string = "Item: ";
items.forEach(fn(item: string): void {
    io::println(prefix + item);
});

// Named function — reusable, self-documenting
fn formatItem(prefix: string, item: string): string {
    return prefix + item;
}
```

## What's Next?

- [Tuples](./tuples) — grouping multiple values
- [Iterators](./iterators) — custom iteration with closures
- [Operator Overloading](./operator-overloading) — defining operators for your types
