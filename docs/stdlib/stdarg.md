# StdArg

The `tt::lang::StdArg` module provides C-style varargs analogs for C `<cstdarg>` parity. It provides `VaList` wrapping `ArrayList<Variant>` iteration, plus the standard `va_start`, `va_arg`, `va_end`, `va_copy` functions.

## Import

```titrate
import tt::lang::StdArg;
```

## API Reference

### `VaList`

Iterator over a list of `Variant` arguments (analog of `va_list` in C).

**Constructor:**
- `init()` — creates an empty, inactive `VaList`

**Methods:**
- `initWithArgs(args: ArrayList<Variant>): void` — initialize with the given argument list and mark active
- `hasNext(): bool` — returns `true` if there are more arguments to read
- `next(typeName: string): Variant` — returns the next argument as a `Variant`. The `typeName` parameter is informational (used for optional type checking); the actual value is returned as a `Variant` regardless.
- `peek(): Variant` — peek at the next argument without advancing
- `position(): int` — return the current position (number of arguments consumed)
- `count(): int` — return the total number of arguments
- `remaining(): int` — return the number of remaining arguments
- `reset(): void` — reset the position to the beginning (does not reactivate if ended)
- `end(): void` — mark this `VaList` as ended (cleanup)
- `isEnded(): bool` — check if this `VaList` has been ended by `va_end`
- `isActive(): bool` — check if this `VaList` is active (started and not ended)
- `args(): ArrayList<Variant>` — return the underlying argument list
- `copy(): VaList` — create a snapshot copy of this `VaList` at the current position
- `toString(): string` — returns `"VaList(pos=..., count=..., active=...)"`

### Free Functions

#### `va_start(args: ArrayList<Variant>): VaList`

Initialize a `VaList` for iterating over the given argument list (`va_start`).

#### `va_arg(list: VaList, typeName: string): Variant`

Return the next argument from the `VaList` (`va_arg`). The `typeName` is informational and used for optional type validation.

#### `va_end(list: VaList): void`

Clean up a `VaList` after use (`va_end`).

#### `va_copy(src: VaList): VaList`

Copy a `VaList` at its current position (`va_copy`). The destination is a new `VaList` that can be iterated independently.

#### `va_listOf(args: ArrayList<Variant>): VaList`

Create a `VaList` from a list of `Variant` arguments (alias for `va_start`).

#### `va_emptyList(): VaList`

Create an empty `VaList`.

#### `va_buildArgs(): ArrayList<Variant>`

Build an argument list from individual `Variant` values (helper).

#### `va_addArg(args: ArrayList<Variant>, value: Variant): ArrayList<Variant>`

Append an argument to an argument list (builder pattern). Returns the list for chaining.

#### `va_isType(list: VaList, typeName: string): bool`

Check if the next argument matches the given type tag.

#### `va_argIfType(list: VaList, typeName: string): Variant`

Return the next argument only if it matches the given type tag; otherwise return `null` without advancing.

## Usage Examples

### Basic Varargs Summation

```titrate
import tt::lang::StdArg;
import tt::lang::Variant;
import tt::util::ArrayList;
import tt::io::IO;

public fn sum(args: ArrayList<Variant>): int {
    let ap: VaList = StdArg.va_start(args);
    var total: int = 0;
    while (ap.hasNext()) {
        let v: Variant = StdArg.va_arg(ap, "int");
        total = total + (v as int);
    }
    StdArg.va_end(ap);
    return total;
}

public fn main(): void {
    let args: ArrayList<Variant> = StdArg.va_buildArgs();
    StdArg.va_addArg(args, 1);
    StdArg.va_addArg(args, 2);
    StdArg.va_addArg(args, 3);
    IO.println("sum = " + Integer.toString(sum(args)));
}
```

### Using va_copy

```titrate
import tt::lang::StdArg;
import tt::lang::Variant;
import tt::util::ArrayList;

let args: ArrayList<Variant> = StdArg.va_buildArgs();
StdArg.va_addArg(args, "a");
StdArg.va_addArg(args, "b");
StdArg.va_addArg(args, "c");

let ap: VaList = StdArg.va_start(args);
let copy: VaList = StdArg.va_copy(ap);
io::println("original position: " + Integer.toString(ap.position()));
io::println("copy position: " + Integer.toString(copy.position()));
StdArg.va_end(ap);
StdArg.va_end(copy);
```

### Peeking and Checking Types

```titrate
import tt::lang::StdArg;
import tt::lang::Variant;
import tt::util::ArrayList;

let args: ArrayList<Variant> = StdArg.va_buildArgs();
StdArg.va_addArg(args, 42);
let ap: VaList = StdArg.va_start(args);
if (ap.hasNext()) {
    let peeked: Variant = ap.peek();
    io::println("peeked without advancing");
}
if (StdArg.va_isType(ap, "int")) {
    let val: Variant = StdArg.va_argIfType(ap, "int");
    io::println("got typed value");
}
StdArg.va_end(ap);
```

### Iterating with count and remaining

```titrate
import tt::lang::StdArg;
import tt::lang::Variant;
import tt::util::ArrayList;

let args: ArrayList<Variant> = StdArg.va_buildArgs();
StdArg.va_addArg(args, 1);
StdArg.va_addArg(args, 2);
let ap: VaList = StdArg.va_start(args);
io::println("count: " + Integer.toString(ap.count()));
io::println("remaining: " + Integer.toString(ap.remaining()));
StdArg.va_arg(ap, "int");
io::println("remaining after one: " + Integer.toString(ap.remaining()));
StdArg.va_end(ap);
```
