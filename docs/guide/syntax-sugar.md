# Syntax Sugar

Titrate offers a canonical syntax and several sugar forms that make the language approachable for developers coming from C, C++, ECMAScript, and other C-family languages. All sugar forms are fully supported and will always remain available — they desugar into the same canonical form during parsing, so there is no runtime difference.

> **Tip:** The canonical Titrate style is recommended for new projects, but the sugar forms are perfectly valid. Use whichever style your team is most productive with.

## Function Declarations

### C-family Function Syntax

**Sugar:** `Type name(params) { body }`
**Canonical:** `fn name(params): Type { body }`

```titrate
// Sugar form — familiar to C/ECMAScript developers
public void greet(string name) {
    io::println("Hello, " + name);
}

// Canonical form — recommended Titrate style
public fn greet(name: string): void {
    io::println("Hello, " + name);
}
```

The sugar form uses `Type name` parameter order (C-style), while the canonical form uses `name: Type` (Pascal/Rust-style).

## Parameter Order

### C-style Parameters

**Sugar:** `Type name, Type name, ...`
**Canonical:** `name: Type, name: Type, ...`

```titrate
// Sugar form
fn add(int a, int b): int { return a + b; }

// Canonical form
fn add(a: int, b: int): int { return a + b; }
```

The sugar parameter order is available in:
- Top-level function declarations
- Class method declarations
- Constructor declarations

## Constructors

### ClassName Constructor Syntax

**Sugar:** `ClassName(params) { body }` inside a class
**Canonical:** `fn init(params) { body }`

```titrate
class Circle {
    public double radius;

    // Sugar form — familiar to C++/ECMAScript developers
    public Circle(double r) {
        this.radius = r;
    }

    // Canonical form — recommended Titrate style
    public fn init(r: double) {
        this.radius = r;
    }
}
```

Both forms produce the same constructor. `new Circle(5.0)` works with either declaration.

## Class Methods

### C-family Method Syntax

**Sugar:** `Type name(params) { body }` inside a class
**Canonical:** `fn name(params): Type { body }`

```titrate
class Circle {
    // Sugar form
    public double area() { return 3.14 * this.radius * this.radius; }

    // Canonical form
    public fn area(): double { return 3.14 * this.radius * this.radius; }
}
```

## Class Fields

### Type-first Field Declaration

**Sugar:** `Type name;` or `Type name = expr;`
**Canonical:** `public Type name;` or `public Type name = expr;`

```titrate
class Point {
    // Sugar form — type comes first, no access modifier needed
    double x;
    double y = 0.0;

    // Canonical form — access modifier required
    public double x;
    public double y = 0.0;
}
```

Sugar fields default to `public` visibility when no access modifier is specified.

## Variable Declarations

### Type-first Variable Declaration

**Sugar:** `Type name = expr;`
**Canonical:** `let name: Type = expr;` or `var name: Type = expr;`

```titrate
// Sugar form — familiar to C/ECMAScript developers
int count = 0;
string name = "Alice";

// Canonical form — recommended Titrate style
let count: int = 0;
let name: string = "Alice";
```

Sugar variable declarations are always mutable (equivalent to `var`).

## Compound Assignment Operators

**Sugar:** `x += y`, `x -= y`, `x *= y`, `x /= y`, `x %= y`
**Canonical:** `x = x + y`, `x = x - y`, etc.

```titrate
var total: int = 0;
total += 5;   // same as: total = total + 5
total -= 2;   // same as: total = total - 2
total *= 3;   // same as: total = total * 3
total /= 2;   // same as: total = total / 2
total %= 4;   // same as: total = total % 4
```

Also supported: `&=`, `|=`, `^=`, `<<=`, `>>=` for bitwise operations.

## Increment and Decrement

### Prefix

**Sugar:** `++x`, `--x`
**Canonical:** `x = x + 1`, `x = x - 1`

```titrate
var i: int = 0;
++i;   // same as: i = i + 1
--i;   // same as: i = i - 1
```

### Postfix

**Sugar:** `x++`, `x--`
**Canonical:** `x = x + 1`, `x = x - 1`

```titrate
var i: int = 0;
i++;   // same as: i = i + 1
i--;   // same as: i = i - 1
```

> **Note:** Unlike C, postfix increment/decrement in Titrate does **not** return the pre-increment value. Both `++x` and `x++` desugar to `x = x + 1`.

## Ternary Operator

**Sugar:** `condition ? then_expr : else_expr`
**Canonical:** `if/else` expression

```titrate
let max: int = a > b ? a : b;

// Equivalent with if/else:
let max: int = 0;
if (a > b) {
    max = a;
} else {
    max = b;
}
```

## Range Operators

**Sugar:** `start..end` (exclusive), `start..=end` (inclusive)
**Canonical:** Range construction

```titrate
for (i in 0..5) { }     // 0, 1, 2, 3, 4
for (i in 1..=5) { }    // 1, 2, 3, 4, 5
```

See [Ranges](./ranges) for full details.

## Namespace Access (`::`)

**Sugar:** `expr::name`
**Canonical:** `expr.name`

```titrate
// Sugar form — familiar to C++ developers
Math::sqrt(2.0)
Integer::parseInt("42")

// Canonical form — preferred in Titrate
Math.sqrt(2.0)
Integer.parseInt("42")
```

Both `::` and `.` produce the same member access. The `::` form is always available for C++ developers. In import statements, `::` is the standard separator:

```titrate
import tt::math::Math;     // standard import syntax
import tt.math.Math;        // dot notation also accepted
```

## Result Constructors

`Ok(value)` and `Err(value)` are **compiler-built-in keywords** with dedicated parsing and bytecode opcodes. `ok(value)` and `err(value)` are **standard library convenience functions**. Both are always available and produce the same `Result` values — neither is "sugar" for the other; they are two equivalent ways to construct Results.

```titrate
// Keyword form — built into the compiler, familiar to Rust developers
let good: Result<int, string> = Ok(42);
let bad: Result<int, string> = Err("failed");

// Function form — standard library convenience functions
let good: Result<int, string> = ok(42);
let bad: Result<int, string> = err("failed");
```

Both forms produce the same `Result` value.

## toString() Instance Method

**Sugar:** `obj.toString()` on primitives
**Canonical:** `Integer.toString(obj)`, `Double.toString(obj)`, etc.

```titrate
let x: int = 42;

// Sugar form — calling toString as an instance method
io::println(x.toString());

// Canonical form — calling toString as a static module method
io::println(Integer.toString(x));
```

The compiler automatically converts instance `.toString()` calls to the appropriate static module method based on the type:

| Type | Static method |
|------|--------------|
| `int` | `Integer.toString(x)` |
| `long` | `Long.toString(x)` |
| `double` | `Double.toString(x)` |
| `bool` | `Boolean.toString(x)` |
| `char` | `Char.toString(x)` |

## Enum Variant Bare Fields

**Sugar:** `Variant(Type, Type, ...)`
**Canonical:** `Variant(_0: Type, _1: Type, ...)`

```titrate
enum Shape {
    Circle(double),              // sugar: bare types
    Rectangle(double, double),
}

// Internally becomes:
// Circle(_0: double)
// Rectangle(_0: double, _1: double)
```

Access unnamed fields by position: `shape.0`, `shape.1`.

## self Parameter Type Inference

**Sugar:** `self` without a type annotation
**Canonical:** `self: Self`

```titrate
class Vec2 {
    // Sugar form — self type is inferred
    fn magnitude(self): double { ... }

    // Canonical form — explicit self type
    fn magnitude(self: Self): double { ... }
}
```

## Closure Parameter Type Inference

**Sugar:** `fn(x, y) => expr` — params without type annotations
**Canonical:** `fn(x: Type, y: Type) => expr`

```titrate
let nums = new ArrayList<int>();

// Sugar form — types inferred from context
nums.forEach(fn(n) { io::println(Integer.toString(n)); });

// Canonical form — explicit types
nums.forEach(fn(n: int): void { io::println(Integer.toString(n)); });
```

## Byte Literals

**Sugar:** `b'x'`
**Canonical:** Integer literal with the ASCII value

```titrate
let letter: byte = b'A';    // sugar: 65
let newline: byte = b'\n';  // sugar: 10
```

## Region Blocks

**Sugar:** `region name { block }`
**Canonical:** `unsafe { block }`

```titrate
region r {
    let ptr = r.alloc(42);
}

// Equivalent to:
unsafe {
    let ptr = alloc(42);
}
```

## Unit Literal

**Sugar:** `()` (empty parentheses)
**Canonical:** `Expr::Unit`

```titrate
let nothing = ();   // the unit value
```

## Import Dot Notation

**Sugar:** `import a.b.c;`
**Canonical:** `import a::b::c;`

```titrate
import tt.math.Math;       // sugar: dot notation
import tt::math::Math;     // canonical: double-colon notation
```

Both forms produce the same import.

---

## Quick Reference

| Sugar Form | Canonical Form | Familiar From |
|-----------|---------------|---------------|
| `void greet(string name)` | `fn greet(name: string): void` | C, C++, ECMAScript |
| `int x = 5` | `let x: int = 5` | C, C++, ECMAScript |
| `ClassName(params)` | `fn init(params)` | C++, ECMAScript |
| `double area()` | `fn area(): double` | C, C++, ECMAScript |
| `Type name;` (field) | `public Type name;` | C, C++, ECMAScript |
| `x += y` | `x = x + y` | C, C++, ECMAScript |
| `++x` / `x++` | `x = x + 1` | C, C++, ECMAScript |
| `cond ? a : b` | `if/else` | C, C++, ECMAScript |
| `expr::name` | `expr.name` | C++ |
| `Ok(v)` / `Err(v)` | `ok(v)` / `err(v)` | Equivalent (keyword vs. stdlib function) |
| `obj.toString()` | `Integer.toString(obj)` | ECMAScript |
| `b'x'` | `65` (ASCII value) | C, C++ |
| `import a.b.c` | `import a::b::c` | ECMAScript |
| `self` (no type) | `self: Self` | Rust |
| `fn(x) => ...` | `fn(x: Type) => ...` | ECMAScript |
| `region r { }` | `unsafe { }` | Rust |
