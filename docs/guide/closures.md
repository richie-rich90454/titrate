# Closures

Closures are anonymous functions that can capture variables from their enclosing scope. They are useful for passing behavior into higher-order functions, callbacks, and inline transformations.

## Syntax

Titrate provides two forms of closure syntax:

**Expression form** — for single-expression bodies:

```titrate
fn(params) => expr
```

**Block form** — for multi-statement bodies:

```titrate
fn(params) {
    stmt1;
    stmt2;
    return value;
}
```

## Basic Usage

### Expression Closures

```titrate
let double = fn(x: int): int => x * 2;
io::println(double(5).toString());  // 10
```

### Block Closures

```titrate
let greet = fn(name: string): string {
    let message = "Hello, " + name + "!";
    return message;
};
io::println(greet("Titrate"));  // Hello, Titrate!
```

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
io::println(counter().toString());  // 1
io::println(counter().toString());  // 2
io::println(counter().toString());  // 3
```

Closures capture by reference, so mutations inside the closure are visible outside:

```titrate
var total: int = 0;
let add = fn(n: int): void {
    total = total + n;
};
add(3);
add(7);
io::println(total.toString());  // 10
```

## Using Closures with Collections

Closures are commonly passed to collection methods:

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
        io::println(n.toString());  // prints 2, 4
    }
});
```

## Closures as Function Parameters

You can declare functions that accept closures as parameters:

```titrate
fn apply(a: int, f: fn(int): int): int {
    return f(a);
}

let result = apply(10, fn(x: int): int => x * x);
io::println(result.toString());  // 100
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
    io::println("Iteration " + i.toString());
});
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
io::println(inc().toString());  // 1
io::println(inc().toString());  // 2
reset();
io::println(inc().toString());  // 1
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
    io::println(n.toString());
});
```

## What's Next?

- [Tuples](./tuples) — grouping multiple values
- [Iterators](./iterators) — custom iteration with closures
- [Operator Overloading](./operator-overloading) — defining operators for your types
