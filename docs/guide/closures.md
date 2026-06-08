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
