# AGENTS.md — Titrate Language Syntax Specification

This document is the authoritative syntax specification for the Titrate programming language. Follow these rules when writing or modifying `.tr` files.

## Project Structure

- **Compiler/VM**: `trc/` (Rust codebase)
- **Standard library**: `lib/tt/` (Titrate `.tr` files)
- **Examples**: `examples/` (Titrate `.tr` files)
- **Tests**: `stdlib_test/`, `mega_test_02/`, `mega_test_03/`, `mega_test.tr`

## Build & Test

```bash
cargo test --lib              # Compiler/VM unit tests
cargo test --test stdlib_test # Stdlib integration tests
cargo test --test mega_test   # End-to-end mega test
```

---

## 1. Lexical Structure

### 1.1 Comments

```titrate
// Line comment
/* Block comment */
```

### 1.2 Identifiers

Identifiers start with a letter or underscore, followed by letters, digits, or underscores.

### 1.3 Keywords

**Declarations:** `fn`, `class`, `interface`, `enum`, `let`, `var`, `const`, `import`, `module`

**Access modifiers:** `public`, `private`

**Control flow:** `if`, `else`, `while`, `for`, `do`, `switch`, `case`, `default`, `return`, `break`, `continue`, `with`, `where`

**Literals:** `true`, `false`, `null`

**OOP:** `new`, `this`, `super`, `extends`, `implements`

**Type operations:** `as`, `is`, `type`

**Result type:** `Result`, `Ok`, `Err`

**Ownership:** `Owned`, `region`, `unsafe`

**Primitive types:** `void`, `bool`, `byte`, `short`, `int`, `long`, `vast`, `uvast`, `float`, `double`, `half`, `quad`, `char`, `string`, `size`, `u8`, `u16`, `u32`, `u64`

### 1.4 Literals

```titrate
// Integer literals
42                  // decimal
0xFF                // hexadecimal
0o77                // octal
0b1010              // binary
1_000_000           // numeric underscores

// Float literals
3.14                // double (default)
1_000.5_0           // numeric underscores in floats
1.5h                // half-precision float
2.0q                // quad-precision float

// String literals
"hello"             // regular string with escapes: \n \t \r \\ \" \' \0 \b \f
r"raw string"       // raw string (no escape processing)
r#"raw string"#     // raw string with hash delimiters
r##"raw string"##   // raw string with double hash delimiters

// Character literals
'a'
'\n'

// Byte literal
b'x'                // byte value (desugared to int)
b'\n'
b'\x41'

// Boolean literals
true
false

// Null literal
null
```

### 1.5 Operators

**Arithmetic:** `+`, `-`, `*`, `/`, `%`

**Comparison:** `==`, `!=`, `<`, `>`, `<=`, `>=`

**Logical:** `&&`, `||`, `!`

**Bitwise:** `&`, `|`, `^`, `~`, `<<`, `>>`

**Assignment:** `=`, `+=`, `-=`, `*=`, `/=`, `%=`, `&=`, `|=`, `^=`, `<<=`, `>>=`

**Increment/Decrement:** `++`, `--` (prefix and postfix)

**Range:** `..` (exclusive), `..=` (inclusive)

**Ternary:** `? :`

**Error propagation:** `?` (postfix)

**Other:** `::`, `->`, `=>`, `.`, `,`, `;`, `:`, `as`, `is`

**Delimiters:** `(`, `)`, `{`, `}`, `[`, `]`

### 1.6 Operator Precedence (low to high)

1. Assignment (`=`, compound assignments) — right-associative
2. Ternary (`? :`) — right-associative
3. Range (`..`, `..=`)
4. Logical OR (`||`)
5. Logical AND (`&&`)
6. Equality (`==`, `!=`)
7. Comparison (`<`, `>`, `<=`, `>=`)
8. Bitwise (`|`, `^`, `&`, `<<`, `>>`)
9. Addition (`+`, `-`)
10. Multiplication (`*`, `/`, `%`)
11. Unary (`!`, `-`, `~`, `*`, `&`, `&mut`, `++`, `--`)
12. Postfix (call `.`, `::`, `[]`, `?`, `as`, `++`, `--`)
13. Primary (literals, identifiers, `this`, `super`, `new`, grouping, tuples)

---

## 2. Types

### 2.1 Primitive Types

| Type | Description |
|------|-------------|
| `void` | No value |
| `bool` | Boolean (`true`/`false`) |
| `byte` | 8-bit signed integer |
| `short` | 16-bit signed integer |
| `int` | 32-bit signed integer |
| `long` | 64-bit signed integer |
| `vast` | Arbitrary-precision integer |
| `uvast` | Arbitrary-precision unsigned integer |
| `float` | 32-bit floating point |
| `double` | 64-bit floating point (default float type) |
| `half` | 16-bit floating point |
| `quad` | 128-bit floating point |
| `char` | Unicode character |
| `string` | Unicode string (lowercase, NOT `String`) |
| `size` | Platform-dependent unsigned integer |
| `u8` | 8-bit unsigned integer |
| `u16` | 16-bit unsigned integer |
| `u32` | 32-bit unsigned integer |
| `u64` | 64-bit unsigned integer |

### 2.2 Composite Types

```titrate
// Generic types — always provide type parameters
ArrayList<string>
HashMap<string, int>
Result<int, string>
Optional<double>

// Tuple types
(int, string)
(double, double, double)

// Function types
fn(int): string                    // function taking int, returning string
fn(string): bool                   // function taking string, returning bool
fn(int, int): int                  // function taking two ints, returning int

// Reference types (advanced)
&int                               // immutable reference
&mut int                           // mutable reference
```

### 2.3 Type Casting

Use `value as Type`. Implicit widening (int to double) is also acceptable.

```titrate
let x: int = 42;
let d: double = x as double;       // explicit cast
let d2: double = x;                // implicit widening — also acceptable

// WRONG — Java-style cast
let d: double = (double) x;
```

### 2.4 Type Checking

Use `is`:

```titrate
if (other is Circle) { ... }

// WRONG
if (other instanceof Circle) { ... }
```

### 2.5 Null and Optional

`null` is a standard part of the type system. `Optional<T>` is an alternative for null-safe code.

```titrate
// null comparison is standard
if (x == null) { ... }
if (x != null) { ... }

// Optional is an alternative
let opt: Optional<int> = Optional.of(42);
if (opt.isPresent()) {
    let val: int = opt.get();
}
```

### 2.6 Variant (Dynamic Type)

`Variant` is a standard dynamic type for when generics are not suitable:

```titrate
public fn wrap(value: Variant): Variant { ... }

// Prefer generics when possible
public fn shallowCopy<T>(obj: T): T { ... }

// WRONG — use generics or Variant, never Object
public fn shallowCopy(obj: Object): Object { ... }
```

---

## 3. Declarations

### 3.1 Variable Declarations

| Keyword | Mutability | Typing | Example |
|---------|-----------|--------|---------|
| `let` | Mutable | Type inference | `let x = 42` |
| `var` | Mutable | Explicit type | `var x: int = 42` |
| `const let` | Immutable | Type inference | `const let x = 42` or `const x = 42` |
| `const var` | Immutable | Explicit type | `const var x: int = 42` or `const x: int = 42` |

```titrate
// let — type inference (recommended for local variables)
let name = "Alice";               // inferred as string
let y = 42;                       // inferred as int
let pi = 3.14;                    // inferred as double

// var — explicit type (recommended when type is important)
var count: int = 0;
var items: ArrayList<string> = new ArrayList<string>();

// let with explicit type — also valid (type inference with annotation)
let name: string = "Alice";

// const — compile-time constant (immutable)
// With type inference:
const MAX = 100;                  // inferred as int
const PI = 3.14;                  // inferred as double
const APP_NAME = "Titrate";       // inferred as string

// With explicit type:
const var MAX: int = 100;
public const DEFAULT_PORT: int = 8080;

// const let — also valid syntax for immutable with type inference
const let x = 42;

// C/Java-style syntax sugar — tolerated, desugared to var
int x = 42;                       // equivalent to var x: int = 42
string name = "Alice";            // equivalent to var name: string = "Alice"
double pi = 3.14;                 // equivalent to var pi: double = 3.14
```

### 3.2 Function Declarations

Canonical form uses `fn` with `name: Type` parameter order and `: ReturnType`:

```titrate
// Top-level function
public fn greet(name: string): void {
    io::println("Hello, " + name);
}

// Generic function — use single-letter type parameters
public fn map<T, R>(list: ArrayList<T>, f: fn(T): R): ArrayList<R> {
    let result = new ArrayList<R>();
    for (item in list) {
        result.add(f(item));
    }
    return result;
}

// Private function (omit public — default is private)
fn helper(x: int): int {
    return x * 2;
}
```

Java-style function sugar is tolerated (the parser desugars it):

```titrate
// Java-style — tolerated, desugared to fn form
public int add(int a, int b) { return a + b; }
// Equivalent to: public fn add(a: int, b: int): int { return a + b; }
```

**Return types must always be explicitly declared.**

### 3.3 Access Modifiers

- `public` — accessible from other modules
- `private` — accessible only within the current module
- **Default is private** — declarations without `public` or `private` are private

```titrate
public fn exported(): void { ... }    // accessible everywhere
fn internal(): void { ... }           // private (default)
```

---

## 4. Imports and Modules

### 4.1 Import Syntax

`::` is the canonical path separator. `.` is also accepted.

```titrate
// Canonical — use ::
import tt::util::ArrayList;
import tt::math::Math;
import tt::json::JsonValue;

// Also accepted — use .
import tt.util.ArrayList;
import tt.math.Math;

// Glob import — imports all public names from a module
import tt::util::*;
```

### 4.2 Module Method Calls

Both `.` and `::` work for calling module-level functions. `.` is preferred.

```titrate
// Preferred — dot notation
Integer.parseInt("42")
Double.toString(3.14)
MathAdvanced.sqrt(2.0)
MathTrig.sin(1.0)

// Also valid — :: notation
Integer::parseInt("42")
MathAdvanced::sqrt(2.0)
Result<int, string>::ok(42)
```

### 4.3 Math Module Split

The `Math` module is split across three files. Functions must be called on the correct module:

```titrate
// Math — constants and basic utilities
Math.PI()          // constant (function call, not field)
Math.E()           // constant
Math.INF()         // infinity
Math.NAN()         // not-a-number
Math.abs(x)        // absolute value
Math.fabs(x)       // float absolute value
Math.floor(x)      // floor
Math.ceil(x)       // ceiling
Math.round(x)      // rounding
Math.min(a, b)     // minimum
Math.max(a, b)     // maximum
Math.comb(n, k)    // combinations
Math.factorial(n)  // factorial
Math.erf(x)        // error function
Math.gamma(x)      // gamma function
Math.lgamma(x)     // log-gamma
Math.gcd(a, b)     // greatest common divisor
Math.lcm(a, b)     // least common multiple
Math.random()      // random number

// MathAdvanced — power, exponential, logarithm, root functions
MathAdvanced.sqrt(x)      // square root
MathAdvanced.pow(x, y)    // power
MathAdvanced.exp(x)       // exponential
MathAdvanced.ln(x)        // natural log
MathAdvanced.log2(x)      // log base 2
MathAdvanced.log10(x)     // log base 10
MathAdvanced.cbrt(x)      // cube root
MathAdvanced.hypot(a, b)  // hypotenuse

// MathTrig — trigonometric functions
MathTrig.sin(x)      // sine
MathTrig.cos(x)      // cosine
MathTrig.tan(x)      // tangent
MathTrig.asin(x)     // arc sine
MathTrig.acos(x)     // arc cosine
MathTrig.atan(x)     // arc tangent
MathTrig.atan2(y, x) // arc tangent 2
MathTrig.sinh(x)     // hyperbolic sine
MathTrig.cosh(x)     // hyperbolic cosine
MathTrig.tanh(x)     // hyperbolic tangent
```

**CRITICAL**: Calling `Math.sqrt()` or `Math.sin()` will fail at runtime — these functions do not exist on the `Math` module.

### 4.4 Module Organization

- **One class per file**, file name matches class name (e.g., `ArrayList.tr` contains `class ArrayList`)
- Top-level utility functions may reside in the same file as the primary class

---

## 5. Classes

### 5.1 Class Declaration

```titrate
public class Point {
    // fields, constructor, methods
}
```

### 5.2 Fields

Fields must have `public` or `private` access modifier. Both C-style and Titrate-style are acceptable:

```titrate
public class Point {
    // C-style (common in existing code)
    public double x;
    public double y;
    private string label;

    // Titrate-style (also valid)
    public var x: double;
    public var y: double;
    private var label: string;
}
```

### 5.3 Constructors

Both `fn init()` and `fn ClassName()` are acceptable. Parameters use `name: Type` order:

```titrate
public class Point {
    public double x;
    public double y;

    // Style 1: fn init() (preferred for new code)
    public fn init(x: double, y: double) {
        this.x = x;
        this.y = y;
    }

    // Style 2: fn ClassName() (also valid)
    public fn Point(x: double, y: double) {
        this.x = x;
        this.y = y;
    }
}
```

**Only one constructor per class is supported.** If you need alternate constructors, use factory functions:

```titrate
public class Regex {
    public string pattern;
    public string flags;

    public fn init(p: string) {
        this.pattern = p;
        this.flags = "";
    }

    // Alternate via method
    public fn initWithFlags(p: string, f: string) {
        this.pattern = p;
        this.flags = f;
    }
}

// Or top-level factory function
public fn withFlags(p: string, f: string): Regex {
    let r = new Regex(p);
    r.flags = f;
    return r;
}
```

### 5.4 Methods

Use `fn` keyword with `name: Type` parameter order and `: ReturnType`:

```titrate
public class Circle {
    public double radius;

    public fn init(r: double) {
        this.radius = r;
    }

    public fn area(): double {
        return 3.14159265 * this.radius * this.radius;
    }
}
```

Java-style method syntax is tolerated (desugared by parser):

```titrate
// Java-style — tolerated
public double area() { ... }
public void setRadius(double r) { ... }
// Equivalent to: public fn area(): double { ... }
```

### 5.5 Instance Access

Use `this.` for instance fields and methods (preferred). `self` parameter is tolerated:

```titrate
// Preferred — this.
public fn area(): double {
    return 3.14159265 * this.radius * this.radius;
}

// Tolerated — self parameter (Rust-style)
public fn area(self): double {
    return 3.14159265 * self.radius * self.radius;
}
```

### 5.6 Operator Overloading

Use `fn operator<op>` syntax:

```titrate
public class Vec2 {
    public double x;
    public double y;

    public fn init(x: double, y: double) {
        this.x = x;
        this.y = y;
    }

    public fn operator+(other: Vec2): Vec2 {
        return new Vec2(this.x + other.x, this.y + other.y);
    }

    public fn operator*(scalar: double): Vec2 {
        return new Vec2(this.x * scalar, this.y * scalar);
    }
}
```

### 5.7 No `static` Keyword

Titrate has no `static` keyword. Use top-level `fn` declarations instead:

```titrate
// CORRECT — top-level function
public fn sqrt(x: double): double {
    return Math_sqrt(x);
}

// WRONG — static method inside a class
public class Math {
    public static fn sqrt(x: double): double { ... }
}
```

### 5.8 No `hashCode()` Methods

This is a Java pattern not used in Titrate. Do not add `hashCode()` methods.

---

## 6. Interfaces

Interfaces support default method bodies (like Rust traits / Java 8 defaults):

```titrate
interface Comparable<T> {
    fn compareTo(other: T): int;

    // Default method body is allowed
    fn isGreaterThan(other: T): bool {
        return this.compareTo(other) > 0;
    }
}

public class Name implements Comparable<Name> {
    public string value;

    public fn compareTo(other: Name): int {
        return String.length(this.value) - String.length(other.value);
    }
}
```

---

## 7. Enums

Both `enum` keyword and class-based patterns are acceptable:

```titrate
// Style 1: enum keyword
enum Color {
    Red,
    Green,
    Blue
}

enum JsonValue {
    Null,
    Bool(bool),
    Number(double),
    Str(string)
}

// Style 2: class-based (using EnumValue)
public class Color extends EnumValue {
    public fn init(name: string, ordinal: int) {
        super.init(name, ordinal);
    }
}
```

---

## 8. Control Flow

### 8.1 Conditionals

Parentheses are **required** in `if`/`else if`/`else`:

```titrate
if (x > 0) {
    io::println("positive");
} else if (x == 0) {
    io::println("zero");
} else {
    io::println("negative");
}
```

### 8.2 While Loops

Parentheses are **required**:

```titrate
while (i < list.size()) {
    io::println(list.get(i));
    i = i + 1;
}
```

### 8.3 Do-While Loops

```titrate
do {
    io::println("at least once");
} while (condition);
```

### 8.4 While-Let

```titrate
while let value = iterator.next() {
    io::println(value);
}
```

### 8.5 For-In Loops

Parentheses are **required**:

```titrate
for (item in list) {
    io::println(item);
}
```

### 8.6 C-Style For Loops (Tolerated but Discouraged)

The parser supports C-style for loops, but prefer `while` or `for-in`:

```titrate
// Tolerated — C-style for loop
for (let i = 0; i < n; i++) {
    io::println(i);
}

// Preferred — while loop
var i: int = 0;
while (i < n) {
    io::println(i);
    i++;
}
```

### 8.7 Break and Continue

```titrate
while (true) {
    if (done) { break; }
    if (skip) { continue; }
}
```

### 8.8 Switch/Case with Pattern Matching

```titrate
switch (value) {
    case 0 => io::println("zero");
    case 1 => io::println("one");
    case _ => io::println("other");
}

// With enum patterns
switch (result) {
    case Ok(v) => io::println(v);
    case Err(e) => io::println("error: " + e);
}
```

### 8.9 With Statement (Resource Management)

```titrate
// Simple resource
with (resource) {
    // use resource
}

// Resource with binding
with (let f: File = File.open("data.txt")) {
    // use f
}
```

### 8.10 Ternary Operator

```titrate
let label: string = x > 0 ? "positive" : "non-positive";
```

### 8.11 Range Expressions

```titrate
let exclusive: Range = 1..10;      // 1, 2, ..., 9
let inclusive: Range = 1..=10;     // 1, 2, ..., 10
```

---

## 9. Error Handling

### 9.1 Result<T, E> (Primary for Recoverable Errors)

```titrate
let r: Result<int, string> = ok(42);
let e: Result<int, string> = err("something went wrong");

if (r.isOk()) {
    let val: int = r.unwrap();
}
```

### 9.2 throw/try/catch (For Unrecoverable Errors)

```titrate
// Throwing
throw "IndexError: out of bounds";

// Catching
try {
    riskyOperation();
} catch (e: string) {
    io::println("Caught: " + e);
}
```

### 9.3 Error Propagation Operator (`?`)

```titrate
// Propagates error if Result is Err, unwraps if Ok
let value: int = mightFail()?;
```

---

## 10. Closures and Function Types

### 10.1 Function Type Syntax

Use `fn(Args): Ret`:

```titrate
let mapper: fn(int): string = fn(x: int): string { return Integer.toString(x); };
let predicate: fn(string): bool = fn(s: string): bool { return String.length(s) > 0; };
let reducer: fn(int, int): int = fn(a: int, b: int): int { return a + b; };

// WRONG — Java-style
function<string(int)>
function<bool(string)>
```

### 10.2 Block Closures

```titrate
let square = fn(x: int): int { return x * x; };
```

### 10.3 Arrow Closures

```titrate
let square = fn(x: int): int => x * x;
let add = fn(a: int, b: int): int => a + b;
```

---

## 11. Tuples

```titrate
// Tuple type
let pair: (int, string) = (1, "hello");

// Tuple destructuring
let (a, b) = pair;

// Unit type (empty tuple)
let unit: void = ();
```

---

## 12. String Operations

### 12.1 String Concatenation

Use `+` for concatenation. String interpolation (`${expr}`) is planned but not yet implemented.

```titrate
let greeting: string = "Hello, " + name + "!";

// Planned (not yet available):
// let greeting: string = "Hello, ${name}!";
```

### 12.2 String Module Methods

Use static `String` module methods, not instance methods:

```titrate
let len: int = String.length(s);
let ch: string = String.charAt(s, 0);
let sub: string = String.substring(s, 0, 5);
let idx: int = String.indexOf(s, "hello");
let upper: string = String.toUpperCase(s);
let lower: string = String.toLowerCase(s);
let parts: ArrayList<string> = String.split(s, ",");

// WRONG — Java-style instance methods
s.length()
s.charAt(0)
s.substring(0, 5)
```

---

## 13. Generics

Always provide type parameters for generic types:

```titrate
// CORRECT
let list: ArrayList<string> = new ArrayList<string>();
let map: HashMap<string, int> = new HashMap<string, int>();
let result: Result<int, string> = Result.ok(42);

// WRONG — raw generic types
let list: ArrayList = new ArrayList();
let map: HashMap = new HashMap();
```

---

## 14. Advanced Features

### 14.1 Reference Types

```titrate
// Immutable reference
let ref: &int = &value;

// Mutable reference
let mutRef: &mut int = &mut value;
```

### 14.2 Unsafe Blocks

```titrate
unsafe {
    // low-level operations
}
```

### 14.3 Region Blocks

```titrate
region name {
    // region-allocated memory
}
```

### 14.4 Owned Type

```titrate
let owned: Owned<int> = Owned(value);
```

---

## 15. Naming Conventions

| Element | Convention | Example |
|---------|-----------|---------|
| Classes | PascalCase | `ArrayList`, `HashMap`, `HttpClient` |
| Interfaces | PascalCase | `Comparable`, `Iterable`, `Iterator` |
| Enums | PascalCase | `Color`, `JsonValue` |
| Functions | camelCase | `parseInt`, `sqrt`, `mapValues` |
| Methods | camelCase | `size()`, `add()`, `getValue()` |
| Variables | camelCase | `itemCount`, `firstName` |
| Constants | UPPER_SNAKE_CASE | `MAX_SIZE`, `DEFAULT_PORT`, `NAMESPACE_DNS` |
| Type parameters | Single uppercase letter | `<T>`, `<T, R>`, `<K, V>` |
| Private helpers | camelCase (omit `public`) | `fn helper(): void` |

---

## 16. Common Patterns

### 16.1 Factory Functions for Classes

When a class needs named constructors, define top-level factory functions:

```titrate
public class Atom {
    public string symbol;
    public int atomicNumber;
    public double mass;

    public fn init(sym: string, number: int, m: double) {
        this.symbol = sym;
        this.atomicNumber = number;
        this.mass = m;
    }
}

// Factory functions
public fn hydrogen(x: double, y: double, z: double): Atom {
    let a = new Atom("H", 1, 1.008);
    return a;
}
```

### 16.2 Utility Modules (No Class Wrapper)

For pure utility functions, use top-level `fn` declarations directly:

```titrate
// CORRECT — top-level functions
public fn sum<T>(a: NDArray<T>): double { ... }
public fn mean<T>(a: NDArray<T>): double { ... }

// WRONG — unnecessary class wrapper
public class NDArrayReduce {
    public static fn sum<T>(a: NDArray<T>): double { ... }
}
```

### 16.3 Native Function Calls

The VM exposes native functions using `ModuleName_functionName` convention:

```titrate
Math_sqrt(2.0)
String_length(s)
HashMap_new()
ArrayList_add(list, item)
```

---

## 17. Entry Point

The main entry point uses `public fn main(): void`:

```titrate
public fn main(): void {
    io::println("Hello, world!");
}
```

---

## Quick Reference: Java → Titrate Migration

| Java | Titrate |
|------|---------|
| `String name = "Alice"` | `let name = "Alice"` or `var name: string = "Alice"` |
| `int x = 5` | `let x = 5` or `var x: int = 5` |
| `public double area()` | `public fn area(): double` |
| `public void main()` | `public fn main(): void` |
| `public ClassName(params)` | `public fn init(params)` or `public fn ClassName(params)` |
| `(int) value` | `value as int` |
| `other instanceof Foo` | `other is Foo` |
| `String` (type) | `string` |
| `function<bool(T)>` | `fn(T): bool` |
| `new HashMap()` | `new HashMap<K, V>()` |
| `Math::sqrt(x)` | `MathAdvanced.sqrt(x)` |
| `Math::sin(x)` | `MathTrig.sin(x)` |
| `s.length()` | `String.length(s)` |
| `static fn foo()` | top-level `fn foo()` |
| `for (int i=0; i<n; i++)` | `while` loop or `for (item in collection)` (C-style for tolerated) |
| `Object` | generics or `Variant` |
| `ClassName::method()` | `ModuleName.method()` or `ModuleName::method()` |
| `a ? b : c` | `a ? b : c` (same) |
| `try { } catch (Exception e)` | `try { } catch (e: string)` |
| `Optional<T>` | `Optional<T>` or nullable with `null` |
