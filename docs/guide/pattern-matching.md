# Pattern Matching

Pattern matching is one of the most satisfying features in Titrate. Instead of writing chains of `if`/`else` checks and type casts, you describe the shape of the data you expect — and the compiler makes sure you have covered every case. It is like a supercharged `switch` statement that can inspect, destructure and extract data all at once.

If you have used pattern matching in Rust, Swift, or ML-family languages, you will find Titrate's approach familiar. If not, you are about to discover a tool that will change how you think about branching logic.

## Switch Statements

Titrate uses `switch` for pattern matching on enum values. At its core, you provide a value to match on and a series of `case` branches:

```titrate
enum HttpStatus {
    Ok(int),
    NotFound,
    ServerError(string),
}

switch (status) {
    case Ok(code) => io::println("success: " + Integer.toString(code));
    case NotFound => io::println("not found");
    case ServerError(msg) => io::println("error: " + msg);
}
```

Each `case` branch matches a specific enum variant. If the variant carries data (like `Ok(int)` or `ServerError(string)`), you can bind that data to a variable and use it in the branch body.

The compiler checks that every variant is covered. If you forget one, you will get a compile-time error — no more silent bugs from missed cases.

## Destructuring Data

Pattern matching does not just check *which* variant you have — it also pulls out the data inside. This is called **destructuring**, and it eliminates the need for separate "get" operations:

```titrate
enum Shape {
    Circle(double),
    Rectangle(double, double),
    Triangle(double, double, double),
}

switch (shape) {
    case Circle(r) => io::println("radius = " + Double.toString(r));
    case Rectangle(w, h) => io::println("width = " + Double.toString(w) + ", height = " + Double.toString(h));
    case Triangle(a, b, c) => io::println("sides: " + Double.toString(a) + ", " + Double.toString(b) + ", " + Double.toString(c));
}
```

The variables `r`, `w`, `h`, `a`, `b`, `c` are bound by the pattern — you do not need to call any getter methods.

## Wildcard Pattern

Use `_` when you want to match a variant but do not care about the data it carries:

```titrate
switch (status) {
    case Ok(_) => io::println("success");
    case NotFound => io::println("not found");
    case ServerError(_) => io::println("failure");
}
```

This is especially useful when you only need to know *which* variant you have, not the specific data inside.

## Matching Different Enum Types

Pattern matching works with any enum. Here are a few more examples with different shapes of data:

```titrate
enum JsonValue {
    Null,
    Bool(bool),
    Number(double),
    Str(string),
}

switch (value) {
    case Null => io::println("null");
    case Bool(b) => io::println("boolean: " + Boolean.toString(b));
    case Number(n) => io::println("number: " + Double.toString(n));
    case Str(s) => io::println("string: " + s);
}
```

```titrate
enum PaymentResult {
    Success(string),
    InsufficientFunds,
    Declined(string),
    NetworkError,
}

switch (result) {
    case Success(txId) => io::println("Paid! Transaction: " + txId);
    case InsufficientFunds => io::println("Not enough balance.");
    case Declined(reason) => io::println("Declined: " + reason);
    case NetworkError => io::println("Network issue. Try again.");
}
```

## Nested Patterns

You can match patterns within patterns — useful when your enum variants themselves contain enum values:

```titrate
enum Expr {
    Literal(double),
    Add(Expr, Expr),
    Negate(Expr),
}

switch (expr) {
    case Literal(n) => io::println("literal: " + Double.toString(n));
    case Add(Literal(a), Literal(b)) => io::println("adding two literals: " + Double.toString(a) + " + " + Double.toString(b));
    case Add(left, right) => io::println("adding expressions");
    case Negate(Literal(n)) => io::println("negating literal: " + Double.toString(n));
    case Negate(inner) => io::println("negating expression");
}
```

Notice how `Add(Literal(a), Literal(b))` matches an `Add` whose *both* sides are `Literal` values — and extracts the numbers from each. This is much more precise than checking the outer variant and then manually inspecting the inner values.

::: tip
Nested patterns are powerful, but do not overdo it. If a pattern becomes deeply nested (three or more levels), consider extracting the inner matching into a helper function for readability.
:::

## Block Bodies in Cases

When a case needs more than a single expression, use a block with curly braces:

```titrate
switch (shape) {
    case Circle(r) => {
        let area = 3.14159265 * r * r;
        let circ = 2.0 * 3.14159265 * r;
        io::println("Area: " + Double.toString(area));
        io::println("Circumference: " + Double.toString(circ));
    }
    case Rectangle(w, h) => {
        let area = w * h;
        io::println("Area: " + Double.toString(area));
    }
    case Triangle(a, b, c) => {
        let s = (a + b + c) / 2.0;
        let area = MathAdvanced.sqrt(s * (s - a) * (s - b) * (s - c));
        io::println("Area: " + Double.toString(area));
    }
}
```

## Pattern Matching Best Practices

### Handle every case

The compiler checks for exhaustiveness — make sure you have a `case` for every variant. This is a feature, not a limitation. It prevents bugs where a new variant is added to an enum but one of your `switch` statements was not updated.

### Use wildcards intentionally

`_` is convenient, but it can silently absorb new variants you add later. Prefer explicit cases when possible, and use `_` only when you genuinely do not care about the specific value.

### Keep cases focused

Each case should do one clear thing. If you find yourself writing long blocks in multiple cases, extract the logic into helper functions:

```titrate
switch (result) {
    case Success(txId) => handleSuccess(txId);
    case InsufficientFunds => handleInsufficientFunds();
    case Declined(reason) => handleDeclined(reason);
    case NetworkError => handleNetworkError();
}
```

### Prefer switch over if-else chains

When you are checking an enum value, always use `switch` instead of `if`/`else` with equality checks. `switch` gives you exhaustiveness checking, destructuring, and clearer intent:

```titrate
// PREFER — exhaustive, clear, destructures data
switch (status) {
    case Ok(code) => io::println(Integer.toString(code));
    case NotFound => io::println("404");
    case ServerError(msg) => io::println(msg);
}

// AVOID — no exhaustiveness check, no destructuring, verbose
if (status is HttpStatus.Ok) {
    io::println(Integer.toString(status.code));
} else if (status is HttpStatus.NotFound) {
    io::println("404");
} else {
    io::println(status.message);
}
```

::: tip Try It Yourself
1. Define an enum `Season` with variants `Spring`, `Summer(string)`, `Autumn`, and `Winter(double)` where `Summer` holds a holiday name and `Winter` holds the average temperature.
2. Write a `switch` that matches each variant and prints a description.
3. Try adding a nested pattern: define an enum `Weather` with `Sunny`, `Rainy(double)`, and `Stormy(Season, double)`, then write a `switch` that matches `Stormy(Winter(temp), windSpeed)` specifically.
:::