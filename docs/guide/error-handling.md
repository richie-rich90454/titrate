# Error Handling

Errors happen — files do not exist, network requests fail, users type the wrong thing. The question is: how does your language help you deal with them?

Titrate uses `Result` types instead of exceptions. Why? Because exceptions are invisible in the function signature — you cannot tell from looking at a function whether it might throw. With `Result`, the possibility of failure is right there in the type. The compiler will not let you forget to handle it. This is a small shift in mindset, but it makes your code more robust and easier to reason about.

## Result Type

A `Result<T, E>` represents either a success with a value of type `T`, or a failure with an error of type `E`:

```titrate
let result = ok(42);
```

- `ok(42)` creates a successful result containing the value `42`.
- `err("something went wrong")` creates a failed result containing the error message.

The type `Result<int, string>` means "this is either an `int` or a `string` (the error)." You always know both the success type and the error type.

## ok and err Constructors

Use `ok()` to wrap a successful value and `err()` to wrap an error:

```titrate
let good = ok(42);
let bad = err("parse failed");
```

Both have the same type — `Result<int, string>` — but they carry different information. You can check which one you have using `isOk()` and `isErr()`:

```titrate
if (good.isOk()) {
    let value = good.unwrap();
    io::println(Integer.toString(value));  // 42 (forty-two)
}

if (bad.isErr()) {
    io::println("Something went wrong!");
}
```

### Try It Yourself

Write a function that divides two numbers and returns a `Result`. If the divisor is zero, return an error:

```titrate
fn safeDivide(a: double, b: double): Result<double, string> {
    if (b == 0.0) {
        return err("division by zero");
    }
    return ok(a / b);
}

public fn main(): void {
    let result = safeDivide(10.0, 3.0);
    if (result.isOk()) {
        io::println(Double.toString(result.unwrap()));  // 3.333...
    }

    let bad = safeDivide(10.0, 0.0);
    if (bad.isErr()) {
        io::println("Error!");  // Error!
    }
}
```

Try changing the divisor to `0.0` and see how the error path works.

## Error Propagation with `?`

Manually checking `isOk()` and `isErr()` works, but it gets tedious when you have multiple operations that can fail. The `?` operator is Titrate's shorthand for "if this result is an error, return early — otherwise, give me the value":

```titrate
fn try_parse(s: string): Result<int, string> {
    let value = Integer.parseInt(s);
    let n = value?;  // returns err early if value is err
    return ok(n * 2);
}
```

Here's what `value?` does:

1. If `value` is `ok(n)`, it unwraps to `n` and execution continues.
2. If `value` is `err(e)`, it immediately returns `err(e)` from the enclosing function.

This makes error propagation feel almost effortless — you focus on the happy path, and errors bubble up automatically.

### Chaining Multiple Operations

The `?` operator really shines when you have a sequence of fallible operations:

```titrate
fn readConfig(path: string): Result<Config, string> {
    let content = readFile(path);
    let text = content?;           // return early if readFile fails
    let config = parseConfig(text);
    let result = config?;          // return early if parseConfig fails
    return ok(result);
}
```

Without `?`, you would need nested `if` blocks for every step. With `?`, the happy path reads top-to-bottom, and errors are handled automatically.

### `?` with Different Error Types

The `?` operator works best when all the errors in a function have the same type. If you have `Result<int, string>` and `Result<int, IoError>`, the `?` operator will not automatically convert between error types — you will need to convert them explicitly:

```titrate
fn process(s: string): Result<int, string> {
    let parsed = Integer.parseInt(s);
    let n = parsed?;  // same error type, works directly
    return ok(n * 2);
}
```

## Common Error Handling Patterns

### Providing a Default Value

If you have a `Result` and want to use a default value when it is an error:

```titrate
fn getWithDefault(result: Result<int, string>, default: int): int {
    if (result.isOk()) {
        return result.unwrap();
    }
    return default;
}

let value = getWithDefault(err("not found"), 0);  // zero
```

### Converting Between Error Types

When composing functions that return different error types, convert them to a common type:

```titrate
fn loadUser(id: int): Result<User, string> {
    let data = readFile("users/" + Integer.toString(id));
    let text = data?;  // propagate file errors as string
    let user = parseUser(text);
    return user;  // propagate parse errors as string
}
```

### Mapping a Success Value

Sometimes you want to transform the value inside an `ok` without unwrapping it:

```titrate
fn doubleIfOk(result: Result<int, string>): Result<int, string> {
    if (result.isOk()) {
        return ok(result.unwrap() * 2);
    }
    return result;
}
```

### Logging and Continuing

Not every error needs to stop execution. Sometimes you just want to log it and move on:

```titrate
fn processItems(items: ArrayList<string>): void {
    for (item in items) {
        let result = Integer.parseInt(item);
        if (result.isOk()) {
            io::println("Parsed: " + Integer.toString(result.unwrap()));
        } else {
            io::println("Skipped invalid: " + item);
        }
    }
}
```

## Error Chaining

When an error occurs deep in a call stack, it is helpful to know *where* it came from. You can chain context by wrapping errors with additional information:

```titrate
fn loadConfig(): Result<Config, string> {
    let data = readFile("config.txt");
    if (data.isErr()) {
        return err("Failed to load config: " + data.unwrapErr());
    }
    let config = parseConfig(data.unwrap());
    if (config.isErr()) {
        return err("Invalid config format: " + config.unwrapErr());
    }
    return config;
}
```

Each layer adds context to the error, so when it finally reaches the user, the message tells the full story: "Failed to load config: file not found" rather than just "file not found."

::: tip
When building error messages, think about what the person reading the message needs to know. A raw "null pointer" error is less helpful than "Failed to parse user record: missing 'name' field."
:::

## Throw / Try / Catch

While `Result` is the primary way to handle **recoverable** errors, Titrate also supports `throw`/`try`/`catch` for **unrecoverable** errors — situations where continuing execution is not meaningful:

```titrate
fn checkIndex(index: int, size: int): void {
    if (index < 0 || index >= size) {
        throw "IndexError: index " + Integer.toString(index) + " out of bounds for size " + Integer.toString(size);
    }
}
```

Use `try` / `catch` to handle thrown errors:

```titrate
public fn main(): void {
    try {
        checkIndex(10, 5);
    } catch (e: string) {
        io::println("Caught: " + e);
    }
}
```

Titrate also supports `try` / `finally` for cleanup code that should run regardless of whether an error was thrown:

```titrate
public fn main(): void {
    try {
        riskyOperation();
    } finally {
        io::println("cleanup always runs");
    }
}
```

::: warning
Reserve `throw`/`try`/`catch` for truly unrecoverable errors. For expected failure cases (invalid input, missing data, network timeouts), prefer `Result<T, E>` — it makes failure explicit in the type signature and forces callers to handle it.
:::

## Improved Error Messages

Alpha 0.3 introduces significantly improved compiler error messages with actionable suggestions.

### Suggestions on Errors

When the compiler detects a semantic error, it often provides a suggestion for how to fix it:

```titrate
fn example(): void {
    var x: int = 10;
    x = "hello";  // Error: type mismatch in assignment to 'x'
                   // Suggestion: expected type int, found string
}
```

Common scenarios where suggestions appear:

- **Type mismatches** — the compiler tells you what type was expected and what was found.
- **Missing return statements** — the compiler suggests adding a return statement at the end of the function body.
- **Immutable assignment** — trying to assign to a `let` variable suggests using `var` instead.
- **Invalid operator operands** — using a numeric operator on non-numeric types suggests which types are valid.

## "Did You Mean?" Hints

The compiler uses Levenshtein distance to detect misspelled identifiers. When you reference a name that does not exist but is similar to one that does, the compiler suggests the correct name:

```titrate
fn example(): void {
    let message = "hello";
    io::println(mesage);  // Error: undefined variable 'mesage'
                           // Suggestion: a similar name exists in scope: 'message'
}
```

This works for:

- **Variables** — misspelled local or global variable names
- **Functions** — misspelled function names
- **Classes and types** — misspelled type names in declarations and expressions

The compiler searches all names in the current scope and suggests the closest match within a small edit distance.

## Unused Variable Warnings

The compiler warns about variables that are declared but never used. This helps catch dead code and typos:

```titrate
fn example(): void {
    let x = 10;      // warning: unused variable: x
    let y = 20;
    io::println(Integer.toString(y));
}
```

Variables whose names start with an underscore (`_`) are exempt from this warning, which is useful for intentionally unused bindings:

```titrate
fn example(): void {
    let _unused = 10;  // no warning
    io::println("done");
}
```

## Unreachable Code Warnings

The compiler detects code that can never be executed because it appears after an unconditional `return`, `break`, or `continue`:

```titrate
fn example(): int {
    return 42;
    io::println("never reached");  // warning: unreachable code detected after return/break/continue
}
```

This also applies inside loops:

```titrate
fn loop_example(): void {
    while (true) {
        break;
        io::println("unreachable");  // warning: unreachable code detected after return/break/continue
    }
}
```

## What is Next?

- [Closures](./closures) — anonymous functions and capture semantics
- [Build Tool](./build-tool) — linting and formatting with pipette
- [Optimizations](./optimizations) — compiler optimization passes
