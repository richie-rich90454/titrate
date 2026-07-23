# Migrating from C/C++

Coming from C or C++? This guide maps the concepts you already know to their Titrate equivalents. Titrate takes inspiration from C-family syntax but makes deliberate departures for safety and clarity — no more header files, no manual memory management, no undefined behavior from null pointers.

## Variable Declarations

C/C++ puts the type first. Titrate puts the name first, followed by a colon and the type:

```titrate
// C:                     // Titrate:
int x = 5;               let x = 5;
double pi = 3.14;        let pi = 3.14;
char c = 'a';            let c = 'a';
const int MAX = 100;     const MAX = 100;
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

Use `let` by default (mutable). Only use `const` when you need immutability.

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

The biggest shift from C/C++: **you do not manually manage memory in Titrate.**

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

Titrate uses garbage collection. There is no `malloc`, `free`, `new`/`delete`, or smart pointers. The GC reclaims memory when objects are no longer reachable.

### Ownership Hints

While the GC handles memory, Titrate lets you express ownership intent:

- **`let`** — mutable binding with type inference (like a non-const variable in C++)
- **`var`** — mutable binding with explicit type
- **`const`** — immutable binding (like a `const` variable in C++)
- **`&int` / `&mut int`** — reference types for borrowing (advisory, not enforced like Rust)

These hints help the optimizer and communicate intent, but the GC still manages the actual memory.

## Pointers

Titrate does not have raw pointers. Use references and safe alternatives:

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
        return MathAdvanced.sqrt(this.x * this.x + this.y * this.y);
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

C/C++ has no built-in type checking operator (you would use `dynamic_cast`). Titrate uses `is`:

```titrate
// C++:
if (dynamic_cast<Circle*>(shape)) { ... }

// Titrate:
if (shape is Circle) { ... }
```

## Quick Reference Table

| C/C++ | Titrate |
|-------|---------|
| `int x = 5;` | `let x = 5;` |
| `const int X = 5;` | `const X = 5;` |
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
| `#define MAX 100` | `const MAX = 100;` |
| `for (int i=0; i<n; i++)` | `var i: int = 0; while (i < n) { ... i = i + 1; }` |
| `switch/case` | `switch/case` with pattern matching |
| `try/catch` | `Result<T, E>` with `?` |
| `sizeof(T)` | (not needed — no manual memory management) |

## What is Next?

- [Variables](./variables) — `let`, `var`, and `const` in depth
- [Classes](./classes) — Titrate's approach to OOP
- [Error Handling](./error-handling) — `Result` and the `?` operator
- [Enums](./enums) — enums with associated data
- [FAQ](./faq) — common questions answered

## Scientific Computing Migration

C/C++ scientists will find Titrate's scientific modules far more ergonomic than raw C or even C++ with external libraries:

| C/C++ Library | Titrate Equivalent |
|---------------|-------------------|
| FFTW | `tt.sigproc.FFT2` |
| OpenCV | `tt.image.Image`, `tt.image.Kernel` |
| Eigen | `tt.math.linalg.Matrix`, `tt.math.ndarray.NDArray` |
| GSL | `tt.math.special.Special` |
| QuantLib | `tt.finance.BlackScholes`, `tt.finance.Portfolio` |
| RDKit | `tt.bio.Sequence`, `tt.bio.Alignment` |
| NLTK | `tt.nlp.Tokenizer`, `tt.nlp.Stemmer` |
| TensorFlow | `tt.ml.Tensor`, `tt.ml.Model` |

### Example: FFT in C vs Titrate

```c
// C with FFTW
#include <fftw3.h>
fftw_complex *in, *out;
fftw_plan p;
in = (fftw_complex*) fftw_malloc(sizeof(fftw_complex) * N);
out = (fftw_complex*) fftw_malloc(sizeof(fftw_complex) * N);
p = fftw_plan_dft_1d(N, in, out, FFTW_FORWARD, FFTW_ESTIMATE);
fftw_execute(p);
fftw_destroy_plan(p);
fftw_free(in); fftw_free(out);
```

```titrate
// Titrate
import tt.sigproc.FFT2;
let spectrum = FFT2.fft(signal);
```

### Example: Option Pricing in C vs Titrate

```c
// C with QuantLib (simplified)
#include <ql/quantlib.hpp>
using namespace QuantLib;
VanillaOption option(payoff, exercise);
option.setPricingEngine(AnalyticEuropeanEngine(stochasticProcess));
Real price = option.NPV();
```

```titrate
// Titrate
import tt::finance::BlackScholes;
let price = BlackScholes.callPrice(100.0, 105.0, 0.25, 0.05, 0.2);
```

## C Standard Library Parity

Phase 1-2 of the standard library brings the C standard library into Titrate so existing C code can be ported line-by-line. Below is a side-by-side migration reference for the most common C library facilities.

### `<stdio.h>` → `tt::io`

| C | Titrate |
|---|---------|
| `printf("x=%d\n", x);` | `io::println("x=" + Integer.toString(x));` |
| `fprintf(stderr, "err\n");` | `io::eprintln("err");` |
| `fopen(path, "r")` | `File.open(path)` (returns `Result<File, string>`) |
| `fgets(buf, n, fp)` | `file.readLine()` |
| `fread(buf, 1, n, fp)` | `file.readBytes(n)` |
| `fclose(fp)` | `file.close()` |

```titrate
import tt.io.File;

let f = File.open("data.txt");
switch f {
    case Ok(file) => {
        let lines = file.readLines();
        for (line in lines) { io::println(line); }
        file.close();
    },
    case Err(e) => io::eprintln("open failed: " + e),
}
```

### `<string.h>` → `String` module

| C | Titrate |
|---|---------|
| `strlen(s)` | `String.length(s)` |
| `strcmp(a, b)` | `String.compare(a, b)` |
| `strcpy(dst, src)` | `dst = src` (immutability; just rebind) |
| `strcat(a, b)` | `a + b` |
| `strchr(s, c)` | `String.indexOf(s, c)` |
| `strstr(hay, needle)` | `String.indexOf(hay, needle)` |
| `strtok(s, ",")` | `String.split(s, ",")` |

### `<stdlib.h>` → various Titrate modules

| C | Titrate |
|---|---------|
| `atoi(s)` / `atol(s)` | `Integer.parseInt(s)` |
| `atof(s)` | `Double.parseDouble(s)` |
| `strtol(s, &end, 10)` | `Integer.parseInt(s)` (use `try`/`catch` for invalid input) |
| `malloc(n)` / `calloc(n, sz)` | `new ArrayList<T>()` or `new T[n]` |
| `free(p)` | (automatic GC) |
| `qsort(arr, n, sz, cmp)` | `Algorithms.sort(arr)` or `Algorithms.sortWith(arr, cmp)` |
| `bsearch(&k, arr, n, sz, cmp)` | `Bisect.bisectLeft(arr, k)` |
| `rand()` | `Math.random()` |
| `srand(seed)` | `Random.seed(seed)` |
| `exit(code)` | `Sys.exit(code)` |
| `getenv("HOME")` | `Sys.env("HOME")` |

### `<math.h>` → `tt::math`

| C | Titrate |
|---|---------|
| `sqrt(x)` | `MathAdvanced.sqrt(x)` |
| `pow(x, y)` | `MathAdvanced.pow(x, y)` |
| `exp(x)` | `MathAdvanced.exp(x)` |
| `log(x)` | `MathAdvanced.ln(x)` |
| `log10(x)` | `MathAdvanced.log10(x)` |
| `sin(x)` / `cos(x)` / `tan(x)` | `MathTrig.sin(x)` / `MathTrig.cos(x)` / `MathTrig.tan(x)` |
| `floor(x)` / `ceil(x)` | `Math.floor(x)` / `Math.ceil(x)` |
| `fabs(x)` | `Math.fabs(x)` |
| `fmod(a, b)` | `a % b` |

**Critical**: Titrate splits the math surface across three modules — `Math` (constants + base utilities), `MathAdvanced` (powers, exps, logs, roots), and `MathTrig` (trig + hyperbolic). Calling `Math.sqrt()` or `Math.sin()` will fail at runtime.

### `<time.h>` → `tt::time`

| C | Titrate |
|---|---------|
| `time(NULL)` | `Time.millis()` |
| `clock()` | `Time.nanos()` |
| `difftime(t1, t2)` | `Duration.between(t1, t2).toSeconds()` |
| `localtime(&t)` | `DateTime.now()` (returns a `DateTime`) |
| `strftime(buf, n, fmt, tm)` | `dt.format(fmt)` |
| `sleep(seconds)` | `Time.sleep(seconds * 1000)` |

### `<ctype.h>` → `Character` and `tt::text`

| C | Titrate |
|---|---------|
| `isalpha(c)` | `Character.isLetter(c)` |
| `isdigit(c)` | `Character.isDigit(c)` |
| `isspace(c)` | `Character.isWhitespace(c)` |
| `toupper(c)` | `Character.toUpperCase(c)` |
| `tolower(c)` | `Character.toLowerCase(c)` |

### `<errno.h>` → `Result<T, E>`

C uses `errno` after the fact. Titrate uses `Result<T, E>` to make errors part of the type:

```c
// C
FILE *fp = fopen(path, "r");
if (fp == NULL) { perror(path); exit(1); }
```

```titrate
// Titrate
let fp = File.open(path);
switch fp {
    case Ok(f)  => useFile(f),
    case Err(e) => {
        io::eprintln(path + ": " + e);
        Sys.exit(1);
    },
}
```

### C++ `<algorithm>` → `tt::algorithms`

```cpp
// C++
#include <algorithm>
std::vector<int> v = {5, 2, 8, 1, 9, 3};
std::sort(v.begin(), v.end());
auto it = std::find(v.begin(), v.end(), 8);
bool sorted = std::is_sorted(v.begin(), v.end());
std::nth_element(v.begin(), v.begin()+3, v.end());
```

```titrate
// Titrate
import tt.algorithms.Algorithms;

let v = new ArrayList<int>();
v.add(5); v.add(2); v.add(8); v.add(1); v.add(9); v.add(3);
Algorithms.sort(v);
let i: int = Algorithms.indexOf(v, 8);
let sorted: bool = Algorithms.isSorted(v);
Algorithms.nthElement(v, 3);
```

For parallel execution, pass an `ExecutionPolicy` (`Seq`, `Par`, `ParUnseq`, `Unseq`):

```titrate
import tt.execution_policy.ExecutionPolicy;

Algorithms.sort(v, ExecutionPolicy.Par);
Algorithms.forEach(v, fn(x: int): void { io::println(Integer.toString(x)); }, ExecutionPolicy.ParUnseq);
```

### C++ `<thread>` → `tt::thread`

```cpp
// C++
#include <thread>
#include <chrono>
std::thread t([](){ /* work */ });
t.join();
std::this_thread::sleep_for(std::chrono::seconds(1));
auto hc = std::thread::hardware_concurrency();
```

```titrate
// Titrate
import tt.thread.Thread;
import tt.thread.JThread;
import tt.time.Duration;

let t = new Thread(fn(): void { /* work */ });
t.start();
t.join();
Thread.sleep(Duration.ofSeconds(1));
let hc: int = Thread.hardwareConcurrency();

// JThread auto-joins and supports cooperative cancellation via StopToken
let jt = new JThread(fn(token: StopToken): void {
    while (!token.stopRequested()) { /* work */ }
});
```

### C++ `<memory>` → `tt::memory`

```cpp
// C++
#include <memory>
std::unique_ptr<Foo> u = std::make_unique<Foo>();
std::shared_ptr<Foo> s = std::make_shared<Foo>();
std::weak_ptr<Foo> w = s;
```

```titrate
// Titrate — Titrate has GC, but the wrappers exist for interop
import tt.memory.UniquePtr;
import tt.memory.SharedPtr;
import tt.memory.WeakPtr;

let u = UniquePtr.of<Foo>(new Foo());
let s = SharedPtr.of<Foo>(new Foo());
let w = WeakPtr.of(s);
```

### C++ `<coroutine>` → `tt::concurrent`

```cpp
// C++20
#include <coroutine>
generator<int> count_up(int n) { for (int i=0; i<n; i++) co_yield i; }
```

```titrate
// Titrate
import tt.concurrent.Generator;

let g = new Generator<int>(fn(yield: fn(int): void): void {
    var i: int = 0;
    while (i < n) { yield(i); i++; }
});
while (g.hasNext()) { io::println(Integer.toString(g.next())); }
```

### C++ `<format>` → `tt::format`

```cpp
// C++20
#include <format>
std::string s = std::format("x={}, y={:.2f}", 42, 3.14159);
```

```titrate
// Titrate
import tt.format.Format;

let s: string = Format.stdFormat("x={}, y={:.2f}", 42, 3.14159);
```

### C `<setjmp.h>` / `<stdarg.h>` → interop only

These C facilities have direct interop wrappers (`tt::setjmp`, `tt::stdarg`) for FFI scenarios, but idiomatic Titrate code should prefer `Result<T, E>` (instead of `setjmp`/`longjmp`) and explicit parameter lists with `Variant`/`Variant[]` (instead of variadic C functions).
