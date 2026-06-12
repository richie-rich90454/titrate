# Enums

Enums (short for "enumerations") are one of Titrate's most powerful features. They let you define a type that can be one of several variants — and each variant can carry different data. If you've used algebraic data types in languages like Rust, Swift, or Haskell, you'll feel right at home. If you haven't, don't worry — this guide will walk you through everything.

Enums are the natural companion to [pattern matching](./pattern-matching), and together they form one of the most expressive and safe ways to model your program's data.

## Why Enums?

Imagine you're modeling the result of an operation. It could succeed with a value, or fail with an error message. You could use a class with a `success` flag and optional fields... but that's error-prone. What if someone forgets to check the flag? What if both fields are set?

Enums solve this elegantly: a value is **exactly one** of the defined variants — never a mix, never ambiguous. The compiler ensures you handle every case, so you can't accidentally forget one.

Use enums when:
- A value can be one of several **distinct alternatives** (success vs. failure, different kinds of shapes, different HTTP status categories)
- Each alternative might carry **different data** (a circle has a radius, a rectangle has width and height)
- You want the compiler to **guarantee** you've handled every possible case

Use classes when:
- You need objects with shared behavior and mutable state
- You need inheritance or interface implementation
- You're modeling entities with identity rather than alternatives

## Simple Enums (Without Data)

The simplest enums just list a set of named values — no data attached:

```titrate
enum Direction {
    North,
    South,
    East,
    West,
}
```

```titrate
enum Color {
    Red,
    Green,
    Blue,
}
```

These are great for representing fixed sets of options — directions, colors, states, categories.

## Enums with Data

Where enums really shine is when each variant carries its own data. This lets you model rich, structured information in a type-safe way:

```titrate
enum Shape {
    Circle(double),
    Rectangle(double, double),
    Triangle(double, double, double),
}
```

Here, a `Circle` carries one `double` (the radius), a `Rectangle` carries two (width and height), and a `Triangle` carries three (side lengths).

More examples:

```titrate
enum JsonValue {
    Null,
    Bool(bool),
    Number(double),
    Str(string),
}

enum HttpStatus {
    Ok(int),
    NotFound,
    ServerError(string),
}

enum Expr {
    Literal(double),
    Add(Expr, Expr),
    Multiply(Expr, Expr),
    Negate(Expr),
}
```

Notice the `Expr` enum — its variants reference `Expr` itself, creating a recursive data structure. This is how you model trees, expressions, and other self-referential structures.

## Mixed Enums

Enums can mix variants with and without data. This is common when some cases carry information and others don't:

```titrate
enum PaymentResult {
    Success(string),       // transaction ID
    InsufficientFunds,     // no extra data needed
    Declined(string),      // reason
    NetworkError,          // no extra data needed
}
```

```titrate
enum Option<T> {
    Some(T),
    None,
}
```

The `Option` enum is a perfect example: `Some` carries a value, and `None` represents the absence of a value. This is Titrate's way of handling nullability safely — no null pointer exceptions!

## Using Enums with Functions

Enums pair naturally with functions. You can accept enum values as parameters and return them as results:

```titrate
public fn describe(shape: Shape): string {
    switch shape {
        case Circle(r) => "circle with radius " + Double.toString(r);
        case Rectangle(w, h) => "rectangle " + Double.toString(w) + "x" + Double.toString(h);
        case Triangle(a, b, c) => "triangle with sides " + Double.toString(a) + ", " + Double.toString(b) + ", " + Double.toString(c);
    }
}
```

```titrate
public fn area(shape: Shape): double {
    switch shape {
        case Circle(r) => 3.14159265 * r * r;
        case Rectangle(w, h) => w * h;
        case Triangle(a, b, c) => {
            let s: double = (a + b + c) / 2.0;
            Math.sqrt(s * (s - a) * (s - b) * (s - c));
        }
    }
}
```

Functions that return enums are just as useful:

```titrate
public fn parseStatus(code: int): HttpStatus {
    if (code == 200) {
        return HttpStatus.Ok(code);
    }
    if (code == 404) {
        return HttpStatus.NotFound;
    }
    return HttpStatus.ServerError("unknown code: " + Integer.toString(code));
}
```

## Enum Methods

You can define methods on enums to encapsulate behavior alongside the data. Use the same `fn` syntax as classes:

```titrate
enum Shape {
    Circle(double),
    Rectangle(double, double),
    Triangle(double, double, double),
}

public fn area(shape: Shape): double {
    switch shape {
        case Circle(r) => 3.14159265 * r * r;
        case Rectangle(w, h) => w * h;
        case Triangle(a, b, c) => {
            let s: double = (a + b + c) / 2.0;
            Math.sqrt(s * (s - a) * (s - b) * (s - c));
        }
    }
}

public fn describe(shape: Shape): string {
    switch shape {
        case Circle(r) => "circle (r=" + Double.toString(r) + ")";
        case Rectangle(w, h) => "rectangle (" + Double.toString(w) + "x" + Double.toString(h) + ")";
        case Triangle(a, b, c) => "triangle (" + Double.toString(a) + "," + Double.toString(b) + "," + Double.toString(c) + ")";
    }
}
```

## Pattern Matching on Enums

Enums and pattern matching go hand in hand. Use `switch` to destructure enum variants and extract their data:

```titrate
switch shape {
    case Circle(r) => io::println("circle with radius " + Double.toString(r));
    case Rectangle(w, h) => io::println("rectangle");
    case Triangle(a, b, c) => io::println("triangle");
}
```

Use `_` (wildcard) when you don't care about the data inside a variant:

```titrate
switch status {
    case Ok(_) => io::println("success");
    case NotFound => io::println("not found");
    case ServerError(_) => io::println("failure");
}
```

For a deeper dive into pattern matching, see the [Pattern Matching](./pattern-matching) guide.

::: tip Try It Yourself
1. Define an enum `Transport` with variants `Car(int)`, `Bicycle`, and `Train(string)`. The `Car` variant holds the number of doors, and `Train` holds the line name.
2. Write a function `describe(transport: Transport): string` that uses `switch` to return a description for each variant.
3. Try creating a `JsonValue` enum that can represent `Null`, `Bool(bool)`, `Number(double)`, and `Str(string)`, then write a function that prints each variant.
:::
