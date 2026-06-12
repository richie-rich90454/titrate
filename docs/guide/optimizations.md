# Optimizations

Ever wondered what happens between writing Titrate code and running it? The compiler doesn't just translate your code — it makes it smarter. Titrate includes automatic optimization passes that reduce code size and improve runtime performance without changing your program's behavior. The best part? You don't have to do anything — it just works.

## Overview

When you compile a Titrate program, the compiler emits bytecode and then runs two optimization passes:

1. **Constant Folding** — evaluates constant expressions at compile time
2. **Dead Code Elimination** — removes unreachable bytecode

Both passes run on every compiled function. In release builds (`pipette build --release`), these optimizations are especially impactful.

## Constant Folding

Constant folding evaluates expressions whose operands are all compile-time constants and replaces them with the computed result. This eliminates unnecessary runtime computation.

### Numeric Constant Folding

When the compiler sees two constant values followed by an arithmetic or comparison operator, it computes the result at compile time and emits a single push instruction instead:

```titrate
// Before optimization:
let x: int = 3 + 4;

// The compiler emits:
//   PUSH_I32 3
//   PUSH_I32 4
//   ADD_I32

// After constant folding:
//   PUSH_I32 7
```

This applies to all arithmetic operators (`+`, `-`, `*`, `/`, `%`) and comparison operators on all integer and floating-point types:

```titrate
let a: int = 100 - 50;       // folded to PUSH_I32 50
let b: double = 3.14 * 2.0;  // folded to PUSH_F64 6.28
let c: bool = 10 > 5;        // folded to PUSH_BOOL true
```

### String Concatenation Folding

When two constant strings are concatenated, the compiler folds them into a single string at compile time:

```titrate
// Before optimization:
let greeting: string = "Hello, " + "World!";

// After constant folding:
//   PUSH_STRING "Hello, World!"
```

### What Gets Folded

| Expression type | Example | Folded result |
|----------------|---------|---------------|
| Integer arithmetic | `2 + 3` | `5` |
| Integer comparison | `10 > 5` | `true` |
| Float arithmetic | `1.5 * 2.0` | `3.0` |
| Float comparison | `3.14 > 2.71` | `true` |
| String concatenation | `"a" + "b"` | `"ab"` |

Only expressions where **both** operands are compile-time constants are folded. Expressions involving variables or function calls are left as-is.

::: tip Write readable code, let the compiler optimize
Don't manually pre-compute constants like `let x: int = 7` instead of `let x: int = 3 + 4`. Write the expression that makes your intent clear — the compiler will fold it anyway. `3 + 4` might represent "3 days + 4 days" in your domain, which is more meaningful than a bare `7`.
:::

## Dead Code Elimination

Dead code elimination removes bytecode that can never be executed. This reduces the size of the compiled output and avoids executing unnecessary instructions.

### Unreachable Code After Returns

Code that follows an unconditional `return` statement is removed:

```titrate
fn example(): int {
    return 42;
    io::println("never reached");  // eliminated
    return 0;                       // eliminated
}
```

### Unreachable Code After Jumps

Code that follows an unconditional `jump` (such as the jump at the end of a `while` loop) is removed if no other code targets it:

```titrate
fn loop_example(): void {
    while (true) {
        break;
        io::println("unreachable");  // eliminated
    }
}
```

### How It Works

The dead code elimination pass:

1. Decodes all instructions in the chunk.
2. Identifies jump targets — instruction indices that are the destination of some jump.
3. Marks instructions as reachable or dead by scanning linearly: after an unconditional `JMP` or `RET`, all subsequent instructions are dead until a jump target is reached.
4. Removes dead instructions and remaps jump offsets to account for the removed instructions.
5. Removes unused string table entries — strings that are no longer referenced by any `PUSH_STRING` instruction.

### Jump Target Preservation

The pass is careful to preserve code that is the target of a jump, even if it appears after an unconditional return or jump. This ensures that `if-else` branches and loop bodies remain intact:

```titrate
fn conditional(x: int): int {
    if (x > 0) {
        return 1;    // JMP after this return
    } else {
        return -1;   // this branch is a jump target, so it's preserved
    }
}
```

## Optimization Pipeline

The compiler runs optimizations in this order:

```
Source → Lexer → Parser → Analyzer → Bytecode Emission → Constant Folding → Dead Code Elimination → Final Bytecode
```

Both passes run on each compiled function independently. Constant folding runs first because it may create new opportunities for dead code elimination (e.g., folding a constant condition in an `if` may make one branch unreachable).

## Controlling Optimizations

- **Debug builds** (`pipette build`): optimizations still run, producing efficient bytecode.
- **Release builds** (`pipette build --release`): same optimization passes, but the release profile may enable additional backend optimizations in the future.

## Try It Yourself

See the optimizer in action! Consider this function:

```titrate
fn calculate(): int {
    let a: int = 10 + 20;
    let b: int = 5 * 3;
    let c: int = a + b;
    return c;
    let d: int = 999;
    return d;
}
```

Before you read the answer below, think about what the optimizer will do:
1. Which expressions will be constant-folded?
2. Which lines will be eliminated as dead code?

<details>
<summary>Show the optimized bytecode</summary>

Here's what happens step by step:

**Constant folding:**
- `10 + 20` → `30` (both operands are constants)
- `5 * 3` → `15` (both operands are constants)
- `a + b` → stays as-is at first, but since `a` and `b` are now known constants (`30` and `15`), this becomes `45`

**Dead code elimination:**
- `let d: int = 999;` — eliminated (after `return c`)
- `return d;` — eliminated (after `return c`)

The final bytecode is essentially:

```
PUSH_I32 45
RET
```

Everything collapsed into a single push and return!
</details>

## What's Next?

- [Build Tool](./build-tool) — build profiles and pipette commands
- [Error Handling](./error-handling) — compiler diagnostics and warnings
- [Closures](./closures) — capture semantics and bytecode details
