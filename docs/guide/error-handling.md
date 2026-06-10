# Error Handling

Titrate uses `Result` types for recoverable errors and provides improved compiler diagnostics to help you find and fix mistakes quickly.

## Result Type

```titrate
let result: Result<int, string> = Ok(42);
```

## Error Propagation with `?`

```titrate
fn try_parse(s: string): Result<int, string> {
    let value: Result<int, string> = Integer.parseInt(s);
    let n: int = value?;  // returns Err early if value is Err
    return Ok(n * 2);
}
```

## ok and err Constructors

```titrate
let good: Result<int, string> = Ok(42);
let bad: Result<int, string> = Err("parse failed");
```

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

The compiler uses Levenshtein distance to detect misspelled identifiers. When you reference a name that doesn't exist but is similar to one that does, the compiler suggests the correct name:

```titrate
fn example(): void {
    let message: string = "hello";
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
    let x: int = 10;      // warning: unused variable: x
    let y: int = 20;
    io::println(y.toString());
}
```

Variables whose names start with an underscore (`_`) are exempt from this warning, which is useful for intentionally unused bindings:

```titrate
fn example(): void {
    let _unused: int = 10;  // no warning
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

## What's Next?

- [Closures](./closures) — anonymous functions and capture semantics
- [Build Tool](./build-tool) — linting and formatting with pipette
- [Optimizations](./optimizations) — compiler optimization passes
