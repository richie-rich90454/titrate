# Migrating from C/C++

Coming from C or C++? This guide maps the concepts you already know to their Titrate equivalents. Titrate takes inspiration from C-family syntax but makes deliberate departures for safety and clarity — no more header files, no manual memory management, no undefined behavior from null pointers.

## Variable Declarations

C/C++ puts the type first. Titrate puts the name first, followed by a colon and the type:

```titrate
// C:                     // Titrate:
int x = 5;               let x: int = 5;
double pi = 3.14;        let pi: double = 3.14;
char c = 'a';            let c: char = 'a';
const int MAX = 100;     const MAX: int = 100;
```

Titrate also has `var` for mutable variables — use it when you need to reassign:

```titrate
// C:
int count = 0;
count = count + 1;

// Titrate:
var count: int = 0;
count = count + 1;
```

Use `let` by default (immutable). Only use `var` when you genuinely need mutation.

## Functions

C/C++ uses `Type name(params)` syntax. Titrate uses `fn name(params): ReturnType`:

```titrate
// C:
int add(int a, int b) {
    return a + b;
}

// Titrate:
public fn add(a: int, b: int): int {
    return a + b;
}
```

Key differences:
- **`fn` keyword** — every function starts with `fn`
- **`name: Type` parameter order** — not `Type name`
- **`: ReturnType` after parameters** — not before the function name
- **`public` for visibility** — functions are module-private by default
- **No forward declarations** — the compiler handles ordering

### The Entry Point

```titrate
// C:
int main(int argc, char** argv) {
    return 0;
}

// Titrate:
public fn main(): void {
    // no return value needed
}
```

Titrate's `main` returns `void`, not `int`. Command-line arguments are accessed through the `argparse` standard library module.

## Memory Management

The biggest shift from C/C++: **you don't manually manage memory in Titrate.**

```titrate
// C:
int* arr = malloc(10 * sizeof(int));
// ... use arr ...
free(arr);

// Titrate:
let arr: ArrayList<int> = new ArrayList<int>();
// ... use arr ...
// GC handles cleanup automatically
```

Titrate uses garbage collection. There's no `malloc`, `free`, `new`/`delete`, or smart pointers. The GC reclaims memory when objects are no longer reachable.

### Ownership Hints

While the GC handles memory, Titrate lets you express ownership intent:

- **`let`** — immutable binding (like a `const` reference in C++)
- **`var`** — mutable binding (like a non-const reference)
- **`&int` / `&mut int`** — reference types for borrowing (advisory, not enforced like Rust)

These hints help the optimizer and communicate intent, but the GC still manages the actual memory.

## Pointers

Titrate doesn't have raw pointers. Use references and safe alternatives:

```titrate
// C:
int x = 10;
int* ptr = &x;
*ptr = 20;

// Titrate:
var x: int = 10;
// No pointer arithmetic — use the value directly
x = 20;
```

For optional values (where C uses `NULL`), use `Variant` or `Result`:

```titrate
// C:
int* find(int* arr, int size, int target) {
    for (int i = 0; i < size; i++) {
        if (arr[i] == target) return &arr[i];
    }
    return NULL;
}

// Titrate:
public fn find(arr: ArrayList<int>, target: int): Result<int, string> {
    var i: int = 0;
    while (i < arr.size()) {
        if (arr.get(i) == target) {
            return ok(arr.get(i));
        }
        i = i + 1;
    }
    return err("not found");
}
```

## Structs → Classes

C structs become Titrate classes with `fn init()` constructors:

```titrate
// C:
typedef struct {
    double x;
    double y;
} Point;

Point point_new(double x, double y) {
    Point p;
    p.x = x;
    p.y = y;
    return p;
}

// Titrate:
class Point {
    public double x;
    public double y;

    public fn init(x: double, y: double) {
        this.x = x;
        this.y = y;
    }
}

// Usage:
let p: Point = new Point(3.0, 4.0);
```

Key differences:
- **Fields need access modifiers** — `public` or `private`
- **`fn init()` is the constructor** — not a function with the struct name
- **`this.` is required** — for accessing instance fields and methods
- **Methods go inside the class** — no separate `point_area()` functions

### Adding Methods

```titrate
// C:
double point_distance(Point* p) {
    return sqrt(p->x * p->x + p->y * p->y);
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
        return Math.sqrt(this.x * this.x + this.y * this.y);
    }
}
```

## Enums

C enums are just named integers. Titrate enums can carry data:

```titrate
// C:
enum Color { RED, GREEN, BLUE };

// Titrate:
enum Color {
    Red,
    Green,
    Blue,
}
```

With associated data (impossible in C without unions):

```titrate
// C — unsafe, no type safety:
enum ShapeType { CIRCLE, RECTANGLE };
typedef struct {
    enum ShapeType type;
    union {
        double radius;
        struct { double w; double h; };
    };
} Shape;

// Titrate — safe and explicit:
enum Shape {
    Circle(double),
    Rectangle(double, double),
}
```

See [Enums](./enums) for the full guide.

## Error Handling

C uses return codes and `errno`. C++ uses exceptions. Titrate uses `Result`:

```titrate
// C:
int divide(int a, int b, int* result) {
    if (b == 0) return -1;  // error code
    *result = a / b;
    return 0;  // success
}

// Titrate:
public fn divide(a: int, b: int): Result<int, string> {
    if (b == 0) {
        return err("division by zero");
    }
    return ok(a / b);
}
```

The `?` operator propagates errors concisely (like checking return codes but much cleaner):

```titrate
// C — tedious error checking at every step:
int rc;
int a, b, c;
rc = parse_int(str1, &a);
if (rc != 0) return rc;
rc = parse_int(str2, &b);
if (rc != 0) return rc;
c = a + b;

// Titrate — clean error propagation:
let a: int = parseInt(str1)?;
let b: int = parseInt(str2)?;
let c: int = a + b;
```

See [Error Handling](./error-handling) for the full guide.

## Header Files → Modules

C/C++ uses header files for declarations. Titrate uses modules:

```titrate
// C: math_utils.h
#ifndef MATH_UTILS_H
#define MATH_UTILS_H
int factorial(int n);
#endif

// C: math_utils.c
#include "math_utils.h"
int factorial(int n) {
    if (n <= 1) return 1;
    return n * factorial(n - 1);
}

// Titrate: math_utils.tr
public fn factorial(n: int): int {
    if (n <= 1) { return 1; }
    return n * factorial(n - 1);
}
```

```titrate
// C: main.c
#include "math_utils.h"

// Titrate: main.tr
import tt::math_utils;
```

No header files, no include guards, no separate declaration/definition. Just `import` and use.

## Preprocessor → const and Compile-Time

C/C++ uses the preprocessor for constants and conditional compilation. Titrate uses `const` and regular code:

```titrate
// C:
#define MAX_SIZE 100
#define SQUARE(x) ((x) * (x))
#ifdef DEBUG
printf("debug mode\n");
#endif

// Titrate:
const MAX_SIZE: int = 100;
fn square(x: int): int { return x * x; }
// No preprocessor — use regular conditionals
if (debug) {
    io::println("debug mode");
}
```

## Type Casting

C/C++ uses `(Type)value`. Titrate uses `value as Type`:

```titrate
// C:
double d = 3.14;
int i = (int)d;

// Titrate:
let d: double = 3.14;
let i: int = d as int;
```

## Type Checking

C/C++ has no built-in type checking operator (you'd use `dynamic_cast`). Titrate uses `is`:

```titrate
// C++:
if (dynamic_cast<Circle*>(shape)) { ... }

// Titrate:
if (shape is Circle) { ... }
```

## Quick Reference Table

| C/C++ | Titrate |
|-------|---------|
| `int x = 5;` | `let x: int = 5;` |
| `const int X = 5;` | `const X: int = 5;` |
| `int main()` | `public fn main(): void` |
| `void foo(int x)` | `fn foo(x: int): void` |
| `malloc(n * sizeof(T))` | `new ArrayList<T>()` |
| `free(ptr)` | (automatic GC) |
| `T* ptr` | `T` (no raw pointers) |
| `NULL` | `Variant.None` or `err()` |
| `struct Point { ... };` | `class Point { ... }` |
| `Point_new(x, y)` | `new Point(x, y)` |
| `p->field` | `p.field` |
| `(int)value` | `value as int` |
| `dynamic_cast<T*>(p)` | `p is T` |
| `enum Color { RED }` | `enum Color { Red }` |
| `return -1;` (error) | `return err("message");` |
| `#include "foo.h"` | `import tt::foo;` |
| `#define MAX 100` | `const MAX: int = 100;` |
| `for (int i=0; i<n; i++)` | `var i: int = 0; while (i < n) { ... i = i + 1; }` |
| `switch/case` | `switch/case` with pattern matching |
| `try/catch` | `Result<T, E>` with `?` |
| `sizeof(T)` | (not needed — no manual memory management) |

## What's Next?

- [Variables](./variables) — `let`, `var`, and `const` in depth
- [Classes](./classes) — Titrate's approach to OOP
- [Error Handling](./error-handling) — `Result` and the `?` operator
- [Enums](./enums) — enums with associated data
- [FAQ](./faq) — common questions answered
