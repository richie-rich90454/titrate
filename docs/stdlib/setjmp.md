# SetJmp

The `tt::lang::SetJmp` module provides `setjmp`/`longjmp` analogs for C `<csetjmp>` parity. It is implemented via structured exception handling: `longjmp` throws a specially formatted exception that unwinds the stack to the `setjmp` marker, which catches it and returns the `longjmp` value.

Because Titrate functions cannot "return twice" the way C `setjmp` does, two APIs are provided:

1. **Low-level**: `setjmp(buf)` marks the buffer and returns `0`. The caller wraps risky code in `try`/`catch` and uses `isLongJmp()`/`extractLongJmpValue()` to handle the `longjmp` return.
2. **High-level**: `setjmpWithBody(buf, body)` runs the body inside a `try`/`catch` and returns `0` on normal completion or the `longjmp` value on unwind.

## Import

```titrate
import tt::lang::SetJmp;
```

## API Reference

### `SetJmpBuffer`

Buffer holding a saved continuation marker (analog of `jmp_buf`).

**Fields:**
- `id: int` — unique buffer identifier
- `active: bool` — whether the buffer is the current setjmp target
- `returnValue: int` — the return value after a longjmp
- `functionName: string`
- `fileName: string`
- `lineNumber: int`

**Constructor:**
- `init()` — creates a new buffer with a unique ID; `active` is `false`

**Methods:**
- `mark(): void` — mark this buffer as the current setjmp target
- `clear(): void` — deactivate the buffer
- `isActive(): bool` — returns `true` if the buffer is the current target
- `getId(): int` — returns the buffer's unique ID
- `toString(): string` — returns `"SetJmpBuffer(id=..., active=...)"`

### Free Functions

#### `setjmp(buf: SetJmpBuffer): int`

Set a jump marker. Returns `0` on first call. The caller should wrap code that might call `longjmp` in a `try`/`catch` block and use `isLongJmp()` to detect a `longjmp` unwind back to this buffer.

#### `longjmp(buf: SetJmpBuffer, value: int): void`

Initiate a non-local jump by throwing a specially formatted exception. The exception unwinds the stack until caught by a `setjmp` handler for `buf`. If `value` is `0`, it is converted to `1` (C semantics: `longjmp` with `0` returns `1`).

#### `setjmpWithBody(buf: SetJmpBuffer, body: fn(SetJmpBuffer): void): int`

High-level `setjmp`: run `body` inside a `try`/`catch`. Returns `0` if the body completes normally, or the `longjmp` value if `longjmp(buf, v)` is called within the body.

#### `isLongJmp(exception: string, buf: SetJmpBuffer): bool`

Check whether a caught exception string is a `longjmp` for the given buffer.

#### `isLongJmpAny(exception: string): bool`

Check whether a caught exception string is a `longjmp` (for any buffer).

#### `extractLongJmpValue(exception: string): int`

Extract the value from a `longjmp` exception string. Returns `0` if the string is not a `longjmp` exception.

#### `extractLongJmpBufferId(exception: string): int`

Extract the buffer ID from a `longjmp` exception string. Returns `-1` if the string is not a `longjmp` exception.

#### `createBuffer(): SetJmpBuffer`

Create a new `SetJmpBuffer` (factory function).

#### `resetBufferIds(): void`

Reset the global buffer ID counter (useful for testing).

## Usage Examples

### High-Level setjmpWithBody

```titrate
import tt::lang::SetJmp;
import tt::io::IO;

public fn main(): void {
    let buf: SetJmpBuffer = SetJmp.createBuffer();
    let result: int = SetJmp.setjmpWithBody(buf, fn(b: SetJmpBuffer): void {
        IO.println("before longjmp");
        SetJmp.longjmp(b, 42);
        IO.println("after longjmp (never reached)");
    });
    IO.println("setjmp returned: " + Integer.toString(result));
}
```

### Low-Level setjmp with try/catch

```titrate
import tt::lang::SetJmp;

let buf: SetJmpBuffer = SetJmp.createBuffer();
SetJmp.setjmp(buf);
try {
    SetJmp.longjmp(buf, 7);
} catch (e: string) {
    if (SetJmp.isLongJmp(e, buf)) {
        let val: int = SetJmp.extractLongJmpValue(e);
        io::println("longjmp value: " + Integer.toString(val));
    } else {
        throw e;
    }
}
```

### longjmp with Value Zero

```titrate
import tt::lang::SetJmp;

let buf: SetJmpBuffer = SetJmp.createBuffer();
let result: int = SetJmp.setjmpWithBody(buf, fn(b: SetJmpBuffer): void {
    SetJmp.longjmp(b, 0);
});
io::println("result (0 becomes 1): " + Integer.toString(result));
```

### Detecting Any longjmp Exception

```titrate
import tt::lang::SetJmp;

try {
    let buf: SetJmpBuffer = SetJmp.createBuffer();
    SetJmp.longjmp(buf, 99);
} catch (e: string) {
    if (SetJmp.isLongJmpAny(e)) {
        let id: int = SetJmp.extractLongJmpBufferId(e);
        io::println("caught longjmp from buffer " + Integer.toString(id));
    }
}
```
