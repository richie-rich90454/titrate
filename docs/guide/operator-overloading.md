# Operator Overloading

Operator overloading lets you define how standard operators (`+`, `-`, `*`, `/`, etc.) behave for your own types. This makes custom types feel as natural to use as built-in primitives.

## Defining Operator Methods

Operator methods follow the naming convention `operatorX`, where `X` is the operator symbol. They are defined as instance methods inside a class:

```titrate
class Vec2 {
    public double x;
    public double y;

    fn operator+(self, other: Vec2): Vec2 {
        return new Vec2(self.x + other.x, self.y + other.y);
    }
}
```

The `self` parameter refers to the instance on the left side of the operator, and the method parameter is the right operand.

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
fn operator+(self, other: Self): Self
fn operator-(self, other: Self): Self
fn operator*(self, other: Self): Self
fn operator/(self, other: Self): Self
fn operator%(self, other: Self): Self
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
fn operator==(self, other: Self): bool
fn operator!=(self, other: Self): bool
fn operator<(self, other: Self): bool
fn operator>(self, other: Self): bool
fn operator<=(self, other: Self): bool
fn operator>=(self, other: Self): bool
```

## Full Example: Vec2

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
    fn operator+(self, other: Vec2): Vec2 {
        return new Vec2(self.x + other.x, self.y + other.y);
    }

    fn operator-(self, other: Vec2): Vec2 {
        return new Vec2(self.x - other.x, self.y - other.y);
    }

    fn operator*(self, scalar: double): Vec2 {
        return new Vec2(self.x * scalar, self.y * scalar);
    }

    fn operator/(self, scalar: double): Vec2 {
        return new Vec2(self.x / scalar, self.y / scalar);
    }

    // Comparison operators
    fn operator==(self, other: Vec2): bool {
        return self.x == other.x && self.y == other.y;
    }

    fn operator!=(self, other: Vec2): bool {
        return !(self == other);
    }

    fn operator<(self, other: Vec2): bool {
        return self.magnitude() < other.magnitude();
    }

    fn operator<=(self, other: Vec2): bool {
        return self.magnitude() <= other.magnitude();
    }

    fn operator>(self, other: Vec2): bool {
        return self.magnitude() > other.magnitude();
    }

    fn operator>=(self, other: Vec2): bool {
        return self.magnitude() >= other.magnitude();
    }

    // Utility
    fn magnitude(self): double {
        return tt.math.Math.sqrt(self.x * self.x + self.y * self.y);
    }

    fn toString(self): string {
        return "(" + Double.toString(self.x) + ", " + Double.toString(self.y) + ")";
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

## The `self` Parameter

The `self` parameter in operator methods works like `this` in other methods — it refers to the left-hand operand. You must include it explicitly in the parameter list:

```titrate
// Correct
fn operator+(self, other: Self): Self { ... }

// Incorrect — missing self
fn operator+(other: Self): Self { ... }
```

## Operator Overloading and Generics

Operator methods work with generic types. For example, a generic `Pair` class could define equality:

```titrate
class Pair<T: Comparable> {
    public T first;
    public T second;

    fn operator==(self, other: Pair<T>): bool {
        return self.first.compareTo(other.first) == 0
            && self.second.compareTo(other.second) == 0;
    }
}
```

## Guidelines

- **Keep semantics intuitive**: `+` should feel like addition, not like subtraction. Users expect operators to behave consistently with their standard meaning.
- **Define operators in sets**: If you define `==`, define `!=` too. If you define `<`, define `>`, `<=`, and `>=`.
- **Prefer methods for uncommon operations**: If the meaning isn't immediately obvious from the operator, use a named method instead.
- **Comparison operators must return `bool`**: This is enforced by the type system.

## What's Next?

- [Closures](./closures) — anonymous functions
- [Iterators](./iterators) — custom iteration
- [Tuples](./tuples) — grouping multiple values
