# Operator Overloading

Want your custom types to feel as natural to use as built-in ones? Operator overloading lets you define how standard operators (`+`, `-`, `*`, `/`, etc.) behave for your own types. Instead of writing `vec.add(other)`, you can simply write `vec + other` — making your code read like math, not like API calls.

## Defining Operator Methods

Operator methods follow the naming convention `operatorX`, where `X` is the operator symbol. They are defined as instance methods inside a class. Unlike regular methods, operator methods do **not** take a `self` parameter — the left-hand operand is accessed via `this`:

```titrate
class Vec2 {
    public double x;
    public double y;

    fn operator+(other: Vec2): Vec2 {
        return new Vec2(this.x + other.x, this.y + other.y);
    }
}
```

The `this` keyword refers to the instance on the left side of the operator, and the method parameter is the right operand.

## Supported Operators

### Arithmetic Operators

| Operator | Method | Description |
|----------|--------|-------------|
| `+` | `operator+` | Addition |
| `-` | `operator-` | Subtraction |
| `*` | `operator*` | Multiplication |
| `/` | `operator/` | Division |
| `%` | `operator%` | Modulus |

Arithmetic operator methods return the same type as the class:

```titrate
fn operator+(other: Self): Self
fn operator-(other: Self): Self
fn operator*(other: Self): Self
fn operator/(other: Self): Self
fn operator%(other: Self): Self
```

### Comparison Operators

| Operator | Method | Description |
|----------|--------|-------------|
| `==` | `operator==` | Equality |
| `!=` | `operator!=` | Inequality |
| `<` | `operator<` | Less than |
| `>` | `operator>` | Greater than |
| `<=` | `operator<=` | Less than or equal |
| `>=` | `operator>=` | Greater than or equal |

Comparison operator methods return `bool`:

```titrate
fn operator==(other: Self): bool
fn operator!=(other: Self): bool
fn operator<(other: Self): bool
fn operator>(other: Self): bool
fn operator<=(other: Self): bool
fn operator>=(other: Self): bool
```

## Full Example: Vec2

Here is a complete `Vec2` class with both arithmetic and comparison operators:

```titrate
class Vec2 {
    public double x;
    public double y;

    // Constructor
    public fn init(x: double, y: double) {
        this.x = x;
        this.y = y;
    }

    // Arithmetic operators
    fn operator+(other: Vec2): Vec2 {
        return new Vec2(this.x + other.x, this.y + other.y);
    }

    fn operator-(other: Vec2): Vec2 {
        return new Vec2(this.x - other.x, this.y - other.y);
    }

    fn operator*(scalar: double): Vec2 {
        return new Vec2(this.x * scalar, this.y * scalar);
    }

    fn operator/(scalar: double): Vec2 {
        return new Vec2(this.x / scalar, this.y / scalar);
    }

    // Comparison operators
    fn operator==(other: Vec2): bool {
        return this.x == other.x && this.y == other.y;
    }

    fn operator!=(other: Vec2): bool {
        return !(this == other);
    }

    fn operator<(other: Vec2): bool {
        return this.magnitude() < other.magnitude();
    }

    fn operator<=(other: Vec2): bool {
        return this.magnitude() <= other.magnitude();
    }

    fn operator>(other: Vec2): bool {
        return this.magnitude() > other.magnitude();
    }

    fn operator>=(other: Vec2): bool {
        return this.magnitude() >= other.magnitude();
    }

    // Utility
    fn magnitude(): double {
        return MathAdvanced.sqrt(this.x * this.x + this.y * this.y);
    }

    fn toString(): string {
        return "(" + Double.toString(this.x) + ", " + Double.toString(this.y) + ")";
    }
}
```

Usage:

```titrate
let a = new Vec2(1.0, 2.0);
let b = new Vec2(3.0, 4.0);

let sum = a + b;           // Vec2(4.0, 6.0)
let diff = b - a;          // Vec2(2.0, 2.0)
let scaled = a * 3.0;      // Vec2(3.0, 6.0)
let divided = b / 2.0;     // Vec2(1.5, 2.0)

io::println(sum.toString());    // (4.0, 6.0)
io::println(diff.toString());   // (2.0, 2.0)

if (a < b) {
    io::println("a is shorter than b");
}
```

## Operator Overloading and Generics

Operator methods work with generic types. For example, a generic `Pair` class could define equality:

```titrate
class Pair<T: Comparable> {
    public T first;
    public T second;

    fn operator==(other: Pair<T>): bool {
        return this.first.compareTo(other.first) == 0
            && this.second.compareTo(other.second) == 0;
    }
}
```

## Common Operator Overloading Patterns

### Scalar Multiplication

One of the most common patterns: multiplying a vector/matrix by a scalar. The operator takes a different type than the class itself:

```titrate
fn operator*(scalar: double): Vec2 {
    return new Vec2(this.x * scalar, this.y * scalar);
}
```

This lets you write `vec * 2.0` naturally. Note that `2.0 * vec` will not work unless `double` also defines an operator — so stick with the `object * scalar` order.

### Magnitude-Based Comparison

For geometric types, it is common to compare by magnitude (length). This lets you sort vectors or find the shortest/longest:

```titrate
fn operator<(other: Vec2): bool {
    return this.magnitude() < other.magnitude();
}
```

### Chaining Operations

Because arithmetic operators return the same type, you can chain them naturally:

```titrate
let a = new Vec2(1.0, 0.0);
let b = new Vec2(0.0, 1.0);
let result = (a + b) * 2.0 - a;
// Equivalent to: a.operator+(b).operator*(2.0).operator-(a)
```

### Complex Numbers

A `Complex` class is a textbook example where operator overloading makes code dramatically more readable:

```titrate
public class Complex {
    public double real;
    public double imag;

    public fn init(real: double, imag: double) {
        this.real = real;
        this.imag = imag;
    }

    public fn operator+(other: Complex): Complex {
        return new Complex(this.real + other.real, this.imag + other.imag);
    }

    public fn operator*(other: Complex): Complex {
        return new Complex(
            this.real * other.real - this.imag * other.imag,
            this.real * other.imag + this.imag * other.real
        );
    }

    public fn operator==(other: Complex): bool {
        return this.real == other.real && this.imag == other.imag;
    }
}
```

Compare the readability:

```titrate
// With operator overloading — reads like a math textbook
let z = (a + b) * c;

// Without — you have to parse method calls
let z = a.add(b).multiply(c);
```

## When to Overload vs Use Named Methods

Operator overloading is powerful, but it is not always the right choice. Here is how to decide:

**Overload operators when:**
- The operation has an obvious mathematical meaning (`+` for addition, `*` for multiplication)
- The type represents a mathematical abstraction (vectors, matrices, complex numbers, quantities)
- Using the operator makes the code significantly more readable
- The behavior is unsurprising — users can predict what `a + b` does

**Use named methods when:**
- The operation does not have a clear mathematical analog
- The meaning could be ambiguous (does `+` on a list mean append or element-wise add?)
- You need to perform side effects (operators should be pure computations)
- The right-hand type varies in ways that would be confusing

```titrate
// Good use of operator overloading — mathematical and clear
let distance = point1 - point2;

// Better as a named method — not obvious what "subtracting" users means
let mutual = user1.getMutualFriends(user2);
```

::: warning Do not surprise your readers
If someone reads `a + b` and cannot guess what it does, use a named method instead. The goal of operator overloading is to make code *more* readable, not to show off cleverness.
:::

## Guidelines

- **Keep semantics intuitive**: `+` should feel like addition, not like subtraction. Users expect operators to behave consistently with their standard meaning.
- **Define operators in sets**: If you define `==`, define `!=` too. If you define `<`, define `>`, `<=`, and `>=`.
- **Prefer methods for uncommon operations**: If the meaning is not immediately obvious from the operator, use a named method instead.
- **Comparison operators must return `bool`**: This is enforced by the type system.
- **No `self` parameter**: Operator methods use `this` to refer to the left-hand operand, not a `self` parameter. The method signature only includes the right-hand operand.

## Try It Yourself

Create a `Money` class that supports addition and comparison. The class should store an amount (as a `double`) and support:

1. Adding two `Money` values with `+`
2. Comparing two `Money` values with `<` and `==`
3. Scaling by a number with `*` (e.g., `price * 2` for double the price)

```titrate
public class Money {
    public double amount;

    public fn init(amount: double) {
        this.amount = amount;
    }

    // Add your operator methods here!

    public fn toString(): string {
        return "$" + Double.toString(this.amount);
    }
}

// Test it:
let coffee = new Money(4.50);
let bagel = new Money(2.75);
let breakfast = coffee + bagel;
io::println(breakfast.toString());  // $7.25

if (coffee < bagel) {
    io::println("Coffee is cheaper");
} else {
    io::println("Bagel is cheaper");
}

let doubleOrder = breakfast * 2.0;
io::println(doubleOrder.toString());  // $14.5
```

<details>
<summary>Show solution</summary>

```titrate
public class Money {
    public double amount;

    public fn init(amount: double) {
        this.amount = amount;
    }

    public fn operator+(other: Money): Money {
        return new Money(this.amount + other.amount);
    }

    public fn operator*(scalar: double): Money {
        return new Money(this.amount * scalar);
    }

    public fn operator==(other: Money): bool {
        return this.amount == other.amount;
    }

    public fn operator!=(other: Money): bool {
        return !(this == other);
    }

    public fn operator<(other: Money): bool {
        return this.amount < other.amount;
    }

    public fn operator<=(other: Money): bool {
        return this.amount <= other.amount;
    }

    public fn operator>(other: Money): bool {
        return this.amount > other.amount;
    }

    public fn operator>=(other: Money): bool {
        return this.amount >= other.amount;
    }

    public fn toString(): string {
        return "$" + Double.toString(this.amount);
    }
}
```

</details>

## What is Next?

- [Closures](./closures) — anonymous functions
- [Iterators](./iterators) — custom iteration
- [Tuples](./tuples) — grouping multiple values