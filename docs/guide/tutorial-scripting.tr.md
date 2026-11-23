# Scripting Tutorial

Titrate is just as happy as a scripting language as it is a systems language. If you are parsing text, reshaping lists, gluing together library calls, or writing a small tool, you can stay in the high-level subset and never think about memory management.

This tutorial covers that subset: type inference, the collection types, closures, and the iteration patterns that keep scripts short and fast. Everything here is ordinary Titrate — the same syntax and the same standard library used by the systems tutorial.

::: tip Prerequisite
Start with [Getting Started](getting-started.md) if you have not run a `.tr` file yet. This tutorial assumes you can compile and run a program with `trc` or with your project tool.
:::

## Part 1 — Inference and variables

### Let the compiler infer

Use `let` for most local variables. The compiler infers the type from the initializer, and the variable remains mutable:

```titrate
let name = "Alice";          // inferred as string
let count = 42;              // inferred as int
let ratio = 3.14;            // inferred as double
let active = true;           // inferred as bool
```

Use `var` when you want to write the type explicitly. This is useful for empty collections, where there is nothing to infer from:

```titrate
import tt::util::ArrayList;
import tt::util::HashMap;

var items: ArrayList<string> = new ArrayList<string>();
var scores: HashMap<string, int> = new HashMap<string, int>();
```

`let` also accepts an annotation, which is handy when the inferred type is not the one you want:

```titrate
let label: string = "worker";
let threshold: double = 10;   // integer literal widened to double
```

### Constants

`const` marks a value that cannot change. The keyword can stand alone or be paired with `let`/`var`:

```titrate
const MAX_RETRIES = 3;                  // inferred int
const APP_NAME = "logtool";             // inferred string
const var DEFAULT_PORT: int = 8080;     // explicit type
const let VERSION = "1.0";              // inferred with explicit form
```

Name constants in `UPPER_SNAKE_CASE`; name variables and functions in `camelCase`.

::: tip C-style sugar
`int x = 42;` is accepted and treated as `var x: int = 42;`. Prefer `let`/`var` in new code because the intent is clearer.
:::

### Strings and primitive operations

String concatenation uses `+`. Use the `String` module's static functions rather than Java-style instance methods:

```titrate
import tt::lang::String;
import tt::lang::Integer;

public fn stringDemo(): void {
    let s = "  Hello, Titrate  ";
    let trimmed: string = String.trim(s);
    let length: int = String.length(trimmed);
    let upper: string = String.toUpperCase(trimmed);
    let first: string = String.substring(trimmed, 0, 5);
    let parts = String.split(trimmed, ",");

    io::println(upper + " (" + Integer.toString(length) + " chars)");
    io::println(first);
    io::println(Integer.toString(parts.size()));
}
```

`String.charAt(s, i)`, `String.indexOf(s, needle)`, `String.replace(s, old, new)`, `String.contains(s, needle)`, `String.startsWith`, and `String.endsWith` are also available. For numeric conversion use `Integer.toString`, `Integer.parseInt`, `Double.toString`, and `Double.parseDouble`.

## Part 2 — Functions and control flow

### Functions

Return types are always explicit. Parameters use `name: Type` order:

```titrate
public fn clamp(value: int, low: int, high: int): int {
    if (value < low) { return low; }
    if (value > high) { return high; }
    return value;
}

fn double(x: int): int {          // private by default
    return x * 2;
}
```

### Conditionals and loops

Parentheses are required around conditions and in `for-in` headers:

```titrate
public fn branching(x: int): void {
    if (x > 0) {
        io::println("positive");
    } else if (x == 0) {
        io::println("zero");
    } else {
        io::println("negative");
    }
}

public fn loops(items: ArrayList<string>): void {
    var i: int = 0;
    while (i < items.size()) {
        io::println(items.get(i));
        i = i + 1;
    }

    for (item in items) {
        io::println(item);
    }
}
```

C-style `for (let i = 0; i < n; i++)` loops are tolerated but discouraged; prefer `while` or `for-in`. `do { ... } while (condition);` and `break` / `continue` work as you would expect.

### Ranges, ternaries, and switch

```titrate
public fn misc(x: int): void {
    let range = 1..10;                 // 1 through 9
    let inclusive = 1..=10;            // 1 through 10
    let label: string = x > 0 ? "positive" : "non-positive";

    switch (x) {
        case 0 => io::println("zero");
        case 1 => io::println("one");
        case _ => io::println("other");
    }

    io::println(label);
}
```

### Recoverable errors with Result

Fallible operations return `Result<T, E>` rather than throwing. Check it before unwrapping:

```titrate
import tt::lang::Result;
import tt::lang::Double;

// Ratios are never negative, so -1.0 is a safe sentinel for "invalid".
public fn parseRatio(text: string): Result<double, string> {
    let value: double = Double.parseOr(text, -1.0);
    if (value < 0.0) {
        return err("invalid ratio: " + text);
    }
    return ok(value);
}

public fn printRatio(text: string): void {
    let result: Result<double, string> = parseRatio(text);
    if (result.isOk()) {
        io::println("ratio = " + Double.toString(result.unwrap()));
    } else {
        io::println("error: " + result.unwrapErr());
    }
}
```

`Result` also provides `unwrapOr`, `map`, `andThen`, `orElse`, `expect`, and `isOkAnd`. Import it with `import tt::lang::Result;`. For a value that may be absent, use `Optional<T>` from `tt::lang::Optional` with `of`, `isPresent`, `get`, and `orElse`.

## Part 3 — Collections

### ArrayList

`ArrayList<T>` is the workhorse. It grows as needed and supports indexed access, insertion, removal, and higher-order operations:

```titrate
import tt::util::ArrayList;
import tt::lang::Integer;

public fn arrayListDemo(): void {
    let names: ArrayList<string> = new ArrayList<string>();
    names.add("Ada");
    names.add("Grace");
    names.add("Linus");
    names.add(1, "Barbara");        // insert at index

    let second: string = names.get(1);
    names.set(0, "Augusta");
    let found: bool = names.contains("Grace");
    let at: int = names.indexOf("Linus");
    let removed: string = names.remove(2);

    io::println("count = " + Integer.toString(names.size()));
    io::println("second = " + second);
    io::println("has Grace = " + (found ? "yes" : "no"));
    io::println("Linus at " + Integer.toString(at));
}
```

Other useful methods: `isEmpty`, `clear`, `indexOf`, `lastIndexOf`, `addAll`, `removeAll`, `retainAll`, `subList`, `clone`, `sort`, and `slice`.

### HashMap

`HashMap<K, V>` maps keys to values. Check membership before assuming a key exists, or use a default:

```titrate
import tt::util::HashMap;
import tt::util::ArrayList;
import tt::lang::Integer;

public fn wordCounts(words: ArrayList<string>): HashMap<string, int> {
    var counts: HashMap<string, int> = new HashMap<string, int>();
    for (word in words) {
        let current: int = counts.getOrDefault(word, 0);
        counts.put(word, current + 1);
    }
    return counts;
}

public fn printCounts(counts: HashMap<string, int>): void {
    let keys: ArrayList<string> = counts.keys();
    for (key in keys) {
        io::println(key + ": " + Integer.toString(counts.get(key)));
    }
}
```

`containsKey`, `values`, `entries`, `putIfAbsent`, `computeIfAbsent`, `replace`, `remove`, `pop`, and `forEach` are also available. `HashMap` implements `Iterable<K>`, so a `for-in` loop walks its keys.

### HashSet

`HashSet<E>` is for membership tests and set algebra:

```titrate
import tt::util::HashSet;
import tt::util::ArrayList;
import tt::lang::Integer;

public fn setDemo(words: ArrayList<string>): void {
    let seen: HashSet<string> = new HashSet<string>();
    for (word in words) {
        seen.add(word);
    }

    let others: HashSet<string> = new HashSet<string>();
    others.add("Ada");
    others.add("Grace");

    let both = seen.intersection(others);
    let either = seen.union(others);

    io::println("common = " + Integer.toString(both.size()));
    io::println("total distinct = " + Integer.toString(either.size()));
}
```

`difference`, `symmetricDifference`, `isSubsetOf`, `isSupersetOf`, `remove`, `contains`, `clear`, and `toArrayList` round out the set API.

## Part 4 — Closures

A closure is an anonymous function you can store in a variable, pass as an argument, or return. It may capture variables from the enclosing scope.

### Syntax

Give every parameter and the return type explicitly:

```titrate
// Expression form — one expression
let square = fn(x: int): int => x * x;

// Block form — multiple statements
let greet = fn(name: string): string {
    let message = "Hello, " + name + "!";
    return message;
};
```

Function types are written `fn(Args): Ret`, which lets you declare a variable that holds a closure:

```titrate
let mapper: fn(int): string = fn(x: int): string => Integer.toString(x);
let predicate: fn(string): bool = fn(s: string): bool => String.length(s) > 0;
let reducer: fn(int, int): int = fn(a: int, b: int): int => a + b;
```

### Closures with collections

`ArrayList` accepts closures directly for transformation and filtering:

```titrate
import tt::util::ArrayList;
import tt::lang::Integer;

public fn closureDemo(): void {
    let values: ArrayList<int> = new ArrayList<int>();
    var i: int = 1;
    while (i <= 6) {
        values.add(i);
        i = i + 1;
    }

    let doubled: ArrayList<int> = values.map(fn(x: int): int => x * 2);
    let labels: ArrayList<string> = values.mapTo<string>(
        fn(x: int): string => "n=" + Integer.toString(x)
    );
    let evens: ArrayList<int> = values.filter(fn(x: int): bool => x % 2 == 0);

    values.forEach(fn(x: int): void {
        io::println(Integer.toString(x));
    });

    io::println("evens = " + Integer.toString(evens.size()));
    io::println("first label = " + labels.get(0));
    io::println("first doubled = " + Integer.toString(doubled.get(0)));
}
```

`map` keeps the element type; `mapTo<R>` changes it. `filter` keeps elements for which the predicate returns `true`, and `forEach` runs a side effect for each element.

### Capturing variables

A closure can read variables from its defining scope:

```titrate
import tt::util::ArrayList;
import tt::lang::Integer;

public fn captureDemo(): void {
    let threshold: int = 10;
    let values: ArrayList<int> = new ArrayList<int>();
    values.add(5);
    values.add(12);
    values.add(20);

    let above = values.filter(fn(x: int): bool => x > threshold);
    io::println("above " + Integer.toString(threshold) + ": " +
                Integer.toString(above.size()));
}
```

::: tip Keep closures small
If a closure grows beyond a few lines, extract it into a named function. It becomes easier to test, reuse, and read.
:::

## Part 5 — Fast iteration

### Prefer for-in for whole-collection passes

When you need every element and not its index, `for-in` is the shortest and clearest form:

```titrate
public fn total(items: ArrayList<int>): int {
    var sum: int = 0;
    for (item in items) {
        sum = sum + item;
    }
    return sum;
}
```

Use an indexed `while` loop when you need the position, need to skip elements, or need to look ahead:

```titrate
public fn pairwise(items: ArrayList<double>): double {
    var total: double = 0.0;
    var i: int = 0;
    while (i + 1 < items.size()) {
        total = total + items.get(i) * items.get(i + 1);
        i = i + 1;
    }
    return total;
}
```

### Use the collection's own bulk operations

`map`, `mapTo`, `filter`, and `forEach` keep the loop inside the collection implementation, which is both shorter and easier for the runtime to optimize:

```titrate
import tt::util::ArrayList;
import tt::util::HashMap;
import tt::lang::Integer;

public fn histogram(values: ArrayList<int>): HashMap<int, int> {
    var counts: HashMap<int, int> = new HashMap<int, int>();
    values.forEach(fn(v: int): void {
        counts.put(v, counts.getOrDefault(v, 0) + 1);
    });
    return counts;
}
```

### Avoid repeated work in the loop condition

Compute an invariant once when the collection will not change during the loop:

```titrate
import tt::util::ArrayList;

public fn firstHalf(items: ArrayList<string>): ArrayList<string> {
    let result: ArrayList<string> = new ArrayList<string>();
    let limit: int = items.size() / 2;
    var i: int = 0;
    while (i < limit) {
        result.add(items.get(i));
        i = i + 1;
    }
    return result;
}
```

### Ranges for numeric work

A `Range` is a compact way to drive a numeric loop:

```titrate
public fn rangeSum(n: int): int {
    var total: int = 0;
    let r = 1..=n;
    // Ranges describe the sequence; an indexed while loop walks it when needed
    var i: int = 1;
    while (i <= n) {
        total = total + i;
        i = i + 1;
    }
    return total;
}
```

## Part 6 — A complete script: log summary

This program ties the pieces together. It reads server log lines, extracts a status code from each, counts codes with a `HashMap`, and prints a sorted report of code frequencies.

```titrate
import tt::util::ArrayList;
import tt::util::HashMap;
import tt::util::HashSet;
import tt::lang::String;
import tt::lang::Integer;

// Return the status field of a log line, or an empty string if absent.
fn statusOf(line: string): string {
    let parts: ArrayList<string> = String.split(line, " ");
    for (part in parts) {
        if (String.length(part) == 3) {
            let code: int = Integer.parseIntOr(part, -1);
            if (code >= 100 && code < 600) {
                return part;
            }
        }
    }
    return "";
}

public fn summarize(lines: ArrayList<string>): void {
    var counts: HashMap<string, int> = new HashMap<string, int>();
    let codes: HashSet<string> = new HashSet<string>();

    for (line in lines) {
        let status: string = statusOf(line);
        if (String.length(status) > 0) {
            counts.put(status, counts.getOrDefault(status, 0) + 1);
            codes.add(status);
        }
    }

    let distinct: ArrayList<string> = codes.toArrayList();
    let sortedCodes: ArrayList<string> = distinct.clone();
    sortedCodes.sort();

    io::println("distinct status codes: " + Integer.toString(sortedCodes.size()));
    for (code in sortedCodes) {
        io::println(code + " -> " + Integer.toString(counts.get(code)));
    }
}

public fn main(): void {
    let lines: ArrayList<string> = new ArrayList<string>();
    lines.add("GET /index 200 12ms");
    lines.add("GET /missing 404 3ms");
    lines.add("POST /login 200 20ms");
    lines.add("GET /missing 404 4ms");
    summarize(lines);
}
```

### Why this stays fast

- `let`/`var` keep values in their natural types; nothing is boxed.
- `for-in` walks collections directly with no index bookkeeping.
- `getOrDefault` avoids a separate membership check on every line.
- `HashSet` deduplicates in one pass instead of scanning a list repeatedly.
- Where closures are used (`map`, `filter`, `forEach`), the loop body stays small and sits at the collection boundary.

## Quick reference

- Declare: `let x = ...` (inferred), `var x: T = ...` (explicit), `const X = ...` (fixed).
- Conditions and `for-in` headers need parentheses: `if (x > 0)`, `for (item in items)`.
- Collections: `ArrayList<T>`, `HashMap<K, V>`, `HashSet<E>` from `tt::util`.
- Closures: `fn(x: int): int => x * 2`, or `fn(x: int): int { ... return ...; }`.
- Fallible values: `Result<T, E>` with `isOk`, `isErr`, `unwrap`, `unwrapErr`, `unwrapOr`.
- Strings: `String.length(s)`, `String.split(s, sep)`, `String.trim(s)`, not `s.length()`.

## Exercises

1. Rewrite `pairwise` using `for-in` plus an index counter, then compare readability.
2. Extend the log summary to count total requests per distinct code and print the most frequent one.
3. Write a `topN(counts: HashMap<string, int>, n: int): ArrayList<string>` helper using a closure-based sort.
4. Parse a comma-separated file into an `ArrayList<HashMap<string, string>>`, using `Result` to skip malformed rows.
5. Replace one of your indexed loops with `mapTo<R>` and confirm the output is unchanged.
