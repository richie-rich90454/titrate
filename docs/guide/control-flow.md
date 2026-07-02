# Control Flow

Every program needs to make decisions and repeat actions. Titrate keeps control flow straightforward with familiar constructs — `if`/`else` for branching, `while` and `for` for looping, and `break`/`continue` for fine-grained loop control. No surprises, no hidden complexity.

## if / else

The `if` statement evaluates a condition and executes a block of code if the condition is true. Add `else` for the false case, and chain with `else if` for multiple conditions:

```titrate
if (x > 0) {
    io::println("positive");
} else {
    io::println("non-positive");
}
```

You can chain multiple conditions with `else if`:

```titrate
if (score >= 90) {
    io::println("A");
} else if (score >= 80) {
    io::println("B");
} else if (score >= 70) {
    io::println("C");
} else {
    io::println("F");
}
```

::: tip
Titrate requires parentheses around the condition in `if` statements. The curly braces `{ }` are also required — this prevents a whole class of dangling-else bugs.
:::

::: tip Try It Yourself
Write a program that checks a temperature variable and prints whether water would be solid, liquid, or gas at that temperature (in Celsius). Use `if`, `else if`, and `else`.
:::

## Nested Conditions

You can nest `if` statements inside each other when you need to check multiple independent conditions:

```titrate
if (isWeekend) {
    if (isSunny) {
        io::println("Go to the park!");
    } else {
        io::println("Stay in and read.");
    }
} else {
    io::println("Back to work.");
}
```

::: tip
When nesting gets deep (more than two or three levels), consider restructuring your code — perhaps using early returns, helper functions, or pattern matching on enums instead. Flat code is easier to understand than deeply nested code.
:::

## while

The `while` loop repeats a block as long as a condition is true. It's perfect when you don't know in advance how many iterations you need:

```titrate
var i: int = 0;
while (i < 10) {
    io::println(Integer.toString(i));
    i = i + 1;
}
```

A common pattern is using `while` to process input until a sentinel value is reached:

```titrate
var input: string = readNext();
while (String.length(input) > 0) {
    process(input);
    input = readNext();
}
```

::: tip Try It Yourself
Write a `while` loop that computes the sum of numbers from one to 100. Hint: Use a `var` for the sum and another for the counter.
:::

## for

The `for` loop iterates over any collection — arrays, `ArrayList`, ranges, and more. It's the idiomatic way to walk through elements when you don't need the index:

```titrate
for (item in collection) {
    io::println(item);
}
```

Use `var` to make the loop variable mutable:

```titrate
for (var item in collection) {
    item = item + 1;
}
```

You can iterate over a range of numbers too:

```titrate
for (i in 0..10) {
    io::println(Integer.toString(i));
}
```

::: tip
Prefer `for` over `while` when you're iterating over a collection. It's more concise, less error-prone (no off-by-one bugs) and clearly communicates your intent.
:::

::: tip Try It Yourself
Create an `ArrayList<string>` with a few names, then use a `for` loop to print a greeting for each one.
:::

## break and continue

Inside loops, you sometimes need more control than the condition alone provides:

- **`break`** — Immediately exits the loop entirely.
- **`continue`** — Skips the rest of the current iteration and jumps to the next one.

```titrate
while (true) {
    if (done) { break; }
    if (skip) { continue; }
    process();
}
```

A practical example — finding the first item that matches a condition:

```titrate
var found: string = "";
for (item in names) {
    if (String.length(item) > 10) {
        found = item;
        break;
    }
}
```

And using `continue` to skip unwanted items:

```titrate
for (item in numbers) {
    if (item < 0) { continue; }
    io::println(Integer.toString(item));
}
```

::: tip
Use `break` and `continue` judiciously. A single `break` or `continue` per loop is usually fine, but if you find yourself using several, consider restructuring the loop body into a helper function.
:::

## Infinite Loops

A `while (true)` loop runs forever — or until a `break` statement is hit. This pattern is common for event loops, game loops, and REPLs:

```titrate
while (true) {
    let command: string = readCommand();
    if (command == "quit") {
        break;
    }
    execute(command);
}
```

::: warning
Always make sure there's a `break` (or a `return`) inside an infinite loop. A truly infinite loop with no exit will hang your program.
:::

## Common Patterns

### Counting with while

```titrate
var count: int = 0;
var i: int = 1;
while (i <= n) {
    count = count + 1;
    i = i + 1;
}
```

### Accumulating with for

```titrate
var total: double = 0.0;
for (item in prices) {
    total = total + item;
}
io::println("Total: " + Double.toString(total));
```

### Early exit with break

```titrate
var target: int = -1;
for (item in list) {
    if (item == searchKey) {
        target = item;
        break;
    }
}
```

### Skipping with continue

```titrate
for (item in data) {
    if (item is InvalidEntry) { continue; }
    process(item);
}
```
