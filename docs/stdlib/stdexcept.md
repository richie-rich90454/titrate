# StdExcept

The `tt.lang.StdExcept` module mirrors C++'s `<stdexcept>` header. It provides the standard exception class hierarchy rooted at `Exception`, with `LogicError` and `RuntimeError` as the two main branches, plus `NestedException` and the `throwWithNested`/`rethrowIfNested` helpers.

## Import

```titrate
import tt::lang::StdExcept;
```

## Class hierarchy

```
Exception
├── LogicError
│   ├── DomainError
│   ├── InvalidArgument
│   ├── LengthError
│   └── OutOfRange
└── RuntimeError
    ├── RangeError
    ├── OverflowError
    └── UnderflowError
```

All exception classes expose the same API:

- `init(message: string)` — construct with a human-readable message
- `what(): string` — return the message
- `toString(): string` — return `"ClassName: <message>"`
- `getMessage(): string` — alias for `what`
- `setMessage(m: string): void` — mutate the message

## Exception

The root of the standard exception hierarchy.

```titrate
let e: Exception = new Exception("something went wrong");
io::println(e.what());  // "something went wrong"
```

## LogicError

The category of errors that could in principle be detected before the program runs: violations of logical preconditions or class invariants.

- `LogicError(message: string)`

### DomainError

Reports a domain error (function input outside the domain for which the function is defined).

- `DomainError(message: string)`

### InvalidArgument

Reports an invalid argument to a function.

- `InvalidArgument(message: string)`

```titrate
fn sqrt(x: double): double {
    if (x < 0.0) {
        throw new DomainError("sqrt of negative number");
    }
    return MathAdvanced.sqrt(x);
}
```

### LengthError

Reports an attempt to construct an object that would exceed a maximum size.

- `LengthError(message: string)`

### OutOfRange

Reports an argument value that is out of the allowed range (e.g., an out-of-bounds array index).

- `OutOfRange(message: string)`

```titrate
let list: ArrayList<int> = new ArrayList<int>();
list.add(1); list.add(2);
if (idx >= list.size()) {
    throw new OutOfRange("index " + Integer.toString(idx) + " out of bounds");
}
```

## RuntimeError

The category of errors that are detectable only at runtime.

- `RuntimeError(message: string)`

### RangeError

Reports a computation whose result is out of range (e.g., arithmetic overflow on a conversion).

- `RangeError(message: string)`

### OverflowError

Reports an arithmetic overflow.

- `OverflowError(message: string)`

### UnderflowError

Reports an arithmetic underflow.

- `UnderflowError(message: string)`

## NestedException

`NestedException` mixin captures a currently-active exception and stores it alongside a new one, so that handlers can rethrow the inner exception later.

- `NestedException.init(message: string, nested: Exception)`
- `nested(): Exception` — return the captured inner exception, or null
- `nestedPtr(): Exception` — alias for `nested`

```titrate
try {
    riskyInner();
} catch (e: Exception) {
    throw new NestedException("outer failed", e);
}
```

## throwWithNested

If the current exception is `Exception` or a subclass, wrap `nested` into it as the nested exception. If the current exception is not derived from `Exception`, the behavior is to terminate the program.

**Parameters:** `nested: Exception`
**Returns:** `void`

```titrate
try {
    try {
        throw new DomainError("inner");
    } catch (e: DomainError) {
        throwWithNested(new RuntimeError("outer"));
    }
} catch (outer: RuntimeError) {
    // outer.nested() == the DomainError
}
```

## rethrowIfNested

If `exception` has a nested exception attached, throw the nested exception. Otherwise no-op.

**Parameters:** `exception: Exception`
**Returns:** `void`

```titrate
try {
    doWork();
} catch (e: Exception) {
    rethrowIfNested(e);
    io::println("handled: " + e.what());
}
```

## Factory helpers

Convenience top-level constructors that build a typed exception with a formatted message:

- `logicError(msg: string): LogicError`
- `runtimeError(msg: string): RuntimeError`
- `domainError(msg: string): DomainError`
- `invalidArgument(msg: string): InvalidArgument`
- `lengthError(msg: string): LengthError`
- `outOfRange(msg: string): OutOfRange`
- `rangeError(msg: string): RangeError`
- `overflowError(msg: string): OverflowError`
- `underflowError(msg: string): UnderflowError`
