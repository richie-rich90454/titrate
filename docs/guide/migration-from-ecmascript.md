# Migrating from ECMAScript / TypeScript

Coming from ECMAScript or TypeScript? Titrate shares some DNA — both have `let`, `const`, classes, and arrow-like function syntax. But Titrate is statically typed, compiled, and uses `Result` instead of exceptions. This guide maps the concepts you know to their Titrate equivalents.

## Variable Declarations

The keywords are the same, but Titrate requires explicit types (or lets the compiler infer them):

```titrate
// ECMAScript:              // Titrate:
let x = 5;                 let x = 5;
const y = "hello";         const y = "hello";
var z = true;              var z: bool = true;   // var = mutable
```

Key differences:
- **Types are required** (or inferred) — no `any` type, no implicit `any`
- **`let` is mutable** — unlike JS where `let` allows reassignment. In Titrate, `let` means you can reassign. Use `const` for immutable bindings.
- **`const` is compile-time** — in JS, `const` just prevents reassignment at runtime. In Titrate, `const` means the value must be computable at compile time.
- **`var` is mutable** — this is your go-to when you need to reassign with explicit type.

```titrate
// ECMAScript:
let count = 0;
count = count + 1;  // fine

// Titrate:
let count = 0;      // let is mutable
count = count + 1;  // now this works
```

## Functions

ECMAScript uses `function` or arrow syntax. Titrate uses `fn` with explicit parameter and return types:

```titrate
// ECMAScript:
function greet(name) {
    return "Hello, " + name;
}

// TypeScript:
function greet(name: string): string {
    return "Hello, " + name;
}

// Titrate:
public fn greet(name: string): string {
    return "Hello, " + name;
}
```

Key differences:
- **`fn` keyword** — not `function`
- **`name: Type` parameter order** — not `name: Type` is the same as TypeScript, but different from JS
- **`: ReturnType` after parameters** — same as TypeScript
- **`public` for visibility** — functions are module-private by default

### Arrow Functions → Closures

ECMAScript arrow functions map to Titrate closures:

```titrate
// ECMAScript:
const double = (x) => x * 2;
const greet = (name) => { return "Hello, " + name; };

// Titrate:
let double: fn(int): int = fn(x: int): int { return x * 2; };
let greet: fn(string): string = fn(name: string): string { return "Hello, " + name; };
```

Titrate closures are more verbose because they require explicit types. The function type syntax is `fn(Args): ReturnType`:

```titrate
// TypeScript:
type Mapper = (x: number) => string;

// Titrate:
let mapper: fn(int): string = fn(x: int): string { return Integer.toString(x); };
```

### Callbacks

```titrate
// ECMAScript:
const numbers = [1, 2, 3];
const doubled = numbers.map(x => x * 2);

// Titrate:
let numbers: ArrayList<int> = new ArrayList<int>();
numbers.add(1);
numbers.add(2);
numbers.add(3);

let doubled: ArrayList<int> = map(numbers, fn(x: int): int { return x * 2; });
```

## Classes

The class syntax is similar, but constructors use `fn init()` and fields need access modifiers:

```titrate
// ECMAScript / TypeScript:
class Point {
    constructor(public x: number, public y: number) {}
    distance() {
        return Math.sqrt(this.x ** 2 + this.y ** 2);
    }
}

// Titrate:
class Point {
    public double x;
    public double y;

    public fn init(x: double, y: double) {
        this.x = x;
        this.y = y;
    }

    public fn distance(): double {
        return MathAdvanced.sqrt(this.x * this.x + this.y * this.y);
    }
}
```

Key differences:
- **`fn init()` is the constructor** — not `constructor()`
- **Fields must have access modifiers** — `public` or `private`
- **`this.` is required** — for accessing instance members
- **No `static` keyword** — use top-level `fn` for static-like functions
- **Only one constructor** — use factory functions for alternate constructors

### No Static Methods

```titrate
// ECMAScript:
class MathUtils {
    static square(x) { return x * x; }
}

// Titrate — use top-level function:
public fn square(x: int): int {
    return x * x;
}
```

## Async/Await → Future-Based Concurrency

ECMAScript is built around async/await and Promises. Titrate uses a different model:

```titrate
// ECMAScript:
async function fetchUser(id) {
    const response = await fetch(`/users/${id}`);
    const data = await response.json();
    return data;
}

// Titrate — synchronous with Result:
import tt::json::JsonValue;
import tt::net::HttpClient;

public fn fetchUser(id: int): Result<JsonValue, string> {
    let response: Result<string, string> = HttpClient.get("/users/" + Integer.toString(id));
    if (response.isErr()) {
        return err(response.unwrapErr());
    }
    let data: Result<JsonValue, string> = JsonValue.parse(response.unwrap());
    if (data.isErr()) {
        return err(data.unwrapErr());
    }
    return data;
}
```

Titrate's `concurrent` module provides futures for parallel operations, but there is no `async`/`await` syntax. Error handling uses `Result` instead of try/catch around Promises.

## try/catch → Result

ECMAScript uses try/catch for error handling. Titrate uses `Result<T, E>`:

```titrate
// ECMAScript:
try {
    const data = JSON.parse(input);
    console.log(data.name);
} catch (e) {
    console.error("Parse failed:", e);
}

// Titrate:
let data: Result<JsonValue, string> = JsonValue.parse(input);
if (data.isOk()) {
    io::println(data.unwrap().get("name").asStr());
} else {
    io::println("Parse failed: " + data.unwrapErr());
}
```

The `?` operator replaces the pattern of "try this, and if it fails, propagate the error":

```titrate
// ECMAScript:
async function process(input) {
    try {
        const parsed = JSON.parse(input);
        const validated = validate(parsed);
        return transform(validated);
    } catch (e) {
        throw new Error("Processing failed: " + e.message);
    }
}

// Titrate:
public fn process(input: string): Result<string, string> {
    let parsed: JsonValue = JsonValue.parse(input)?;
    let validated: Data = validate(parsed)?;
    return ok(transform(validated));
}
```

## undefined/null → Variant / Result

ECMAScript has both `undefined` and `null`. Titrate uses `Variant` and `Result` to represent absence explicitly:

```titrate
// ECMAScript:
function findUser(id) {
    const user = users.get(id);
    if (user === undefined) {
        return null;
    }
    return user;
}

// Calling code must check:
const result = findUser(1);
if (result !== null && result !== undefined) {
    console.log(result.name);
}

// Titrate:
public fn findUser(id: int): Result<User, string> {
    let user: User = this.users.get(id);
    if (user == null) {
        return err("user not found");
    }
    return ok(user);
}

// Calling code:
let result: Result<User, string> = findUser(1);
if (result.isOk()) {
    io::println(result.unwrap().name);
}
```

The compiler ensures you check the `Result` before accessing the value — no more `undefined is not a function` errors.

## Array Methods → ArrayList with Closures

ECMAScript arrays have rich methods like `map`, `filter`, `reduce`. Titrate uses `ArrayList` with closures:

```titrate
// ECMAScript:
const nums = [1, 2, 3, 4, 5];
const evens = nums.filter(x => x % 2 === 0);
const doubled = evens.map(x => x * 2);
const sum = doubled.reduce((a, b) => a + b, 0);

// Titrate:
let nums: ArrayList<int> = new ArrayList<int>();
nums.add(1); nums.add(2); nums.add(3); nums.add(4); nums.add(5);

let evens: ArrayList<int> = filter(nums, fn(x: int): bool { return x % 2 == 0; });
let doubled: ArrayList<int> = map(evens, fn(x: int): int { return x * 2; });
let sum: int = reduce(doubled, 0, fn(a: int, b: int): int { return a + b; });
```

The standard library provides `map`, `filter`, and `reduce` as top-level generic functions that accept closures.

## Template Literals → String Concatenation

ECMAScript uses template literals with backticks. Titrate uses string concatenation:

```titrate
// ECMAScript:
const greeting = `Hello, ${name}! You are ${age} years old.`;

// Titrate:
let greeting: string = "Hello, " + name + "! You are " + Integer.toString(age) + " years old.";
```

There is no string interpolation syntax in Titrate. Use `+` for concatenation and `Integer.toString()`, `Double.toString()`, etc. for converting values to strings.

## String Methods → Static Module Methods

ECMAScript strings have instance methods. Titrate uses static `String` module methods:

```titrate
// ECMAScript:
s.length
s.charAt(0)
s.substring(0, 5)
s.indexOf("hello")
s.toUpperCase()
s.toLowerCase()
s.split(",")
s.trim()

// Titrate:
String.length(s)
String.charAt(s, 0)
String.substring(s, 0, 5)
String.indexOf(s, "hello")
String.toUpperCase(s)
String.toLowerCase(s)
String.split(s, ",")
String.trim(s)
```

This is a deliberate design choice — static methods make it clear you're calling a module function, not an instance method, and they work consistently with the type system.

## Type Casting

ECMAScript has implicit type coercion. TypeScript has `as` and angle-bracket casts. Titrate uses `as`:

```titrate
// TypeScript:
const x = 42 as unknown as string;  // unsafe

// Titrate:
let x: int = 42;
let s: string = x as string;  // explicit, type-checked
```

## Type Checking

```titrate
// ECMAScript:
if (obj instanceof MyClass) { ... }

// TypeScript:
if (typeof x === "string") { ... }

// Titrate:
if (obj is MyClass) { ... }
```

## Quick Reference Table

| ECMAScript / TypeScript | Titrate |
|------------------------|---------|
| `let x = 5` | `let x = 5` |
| `let x = 5; x = 10` | `let x = 5; x = 10` |
| `const X = 5` | `const X = 5` |
| `function foo(x: string): void` | `fn foo(x: string): void` |
| `(x) => x * 2` | `fn(x: int): int { return x * 2; }` |
| `class Point { constructor(x, y) }` | `class Point { fn init(x: double, y: double) }` |
| `static method()` | top-level `fn method()` |
| `try/catch` | `Result<T, E>` with `?` |
| `null / undefined` | `Variant.None` / `err()` |
| `arr.map(x => x * 2)` | `map(arr, fn(x: int): int { return x * 2; })` |
| `` `Hello, ${name}` `` | `"Hello, " + name` |
| `s.length` | `String.length(s)` |
| `s.toUpperCase()` | `String.toUpperCase(s)` |
| `s.split(",")` | `String.split(s, ",")` |
| `async/await` | `Result` + `concurrent` module |
| `x as string` (TS) | `x as string` |
| `obj instanceof Foo` | `obj is Foo` |
| `typeof x === "string"` | `x is string` |
| `any` | generics or `Variant` |
| `interface Foo { ... }` | `interface Foo { ... }` |
| `implements Foo` | `implements Foo` |
| `enum Color { Red }` | `enum Color { Red }` |

## What is Next?

- [Variables](./variables) — `let`, `var`, and `const` in depth
- [Functions](./functions) — function syntax and closures
- [Classes](./classes) — Titrate's approach to OOP
- [Error Handling](./error-handling) — `Result` and the `?` operator
- [FAQ](./faq) — common questions answered
