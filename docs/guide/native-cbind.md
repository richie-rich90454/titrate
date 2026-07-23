# Wrapping C Libraries

The native backend doesn't just compile Titrate to machine code — it
also needs to call the same 350+ native functions that the bytecode VM
uses for I/O, math, collections, networking, and more. This guide
explains how the native bridge works, how `@native`-annotated functions
map to C calls, and how to extend the bridge to wrap a new C library.

## How the Native Bridge Works

The bridge has three layers:

1. **`titrate_native` crate** (`titrate_native/src/lib.rs`) — a Rust
   crate that exposes `#[no_mangle] pub extern "C"` functions. These
   are the symbols the LLVM backend links against. The crate re-uses
   the bytecode VM's native function table (`lookup_builtin_native`)
   so the same 359 functions work in both backends.
2. **Native bridge codegen** (`trc/src/codegen/llvm/native_bridge.rs`)
   — the LLVM-side code that marshals Titrate values into the C-ABI
   `TitrateValue` struct, calls the wrapper, and unmarshals the result.
3. **Direct helpers** — a handful of hot-path functions
   (`titrate_println`, `titrate_string_concat`, `titrate_malloc`,
   `titrate_free`) that bypass the generic bridge for performance.

When you call `io::println("hello")` from native code, the codegen
emits a call to `titrate_println` directly (it's a direct helper). When
you call `MathTrig.sin(x)`, the codegen emits a call to
`titrate_MathTrig_sin` — a generic wrapper that packs `x` into a
`TitrateValue`, dispatches to the VM's `MathTrig_sin` implementation, and
unpacks the result.

## The C-ABI Value Model

The bridge uses a single C struct to represent any Titrate value:

```c
// titrate_native/titrate_native.h
typedef struct {
    int32_t tag;       // type tag (TV_INT, TV_DOUBLE, TV_STRING, ...)
    int32_t pad;       // alignment padding
    uint8_t  payload[16]; // type-specific data
} TitrateValue;
```

The 24-byte struct is small enough to be returned in registers on most
ABIs (x86-64 SysV returns 16-byte structs in RAX:RDX; 24-byte structs
go on the stack, which is still fast). The 16-byte payload holds:

| Type | Payload layout |
|---|---|
| `bool`, `byte`, `short`, `int`, `long` | Direct integer value, sign-extended |
| `float`, `double` | Direct IEEE-754 value |
| `char` | 32-bit Unicode scalar value |
| `string` | `{ i64 length, i8* ptr }` — UTF-8 bytes, not NUL-terminated |
| `void`, `null` | Payload unused |

Every native wrapper has the same C signature:

```c
TitrateValue titrate_<Name>(const TitrateValue* args, size_t arg_count);
```

This uniformity is what lets the bridge wrap all 359 native functions
with a single codegen pattern.

### Marshalling

Marshalling a Titrate value into a `TitrateValue` is a type-pun through
an alloca:

```llvm
; Marshal a double %x into a TitrateValue
%tv = alloca %TitrateValue
%tag.ptr = getelementptr %TitrateValue, %TitrateValue* %tv, i32 0, i32 0
store i32 10, i32* %tag.ptr              ; TV_DOUBLE = 10
%payload.ptr = getelementptr %TitrateValue, %TitrateValue* %tv, i32 0, i32 2
%d.ptr = bitcast [16 x i8]* %payload.ptr to double*
store double %x, double* %d.ptr
```

Unmarshalling is the reverse: read the tag (for runtime type checking
in debug builds), then load the payload as the expected LLVM type.

Strings are special-cased because they're already two words (`{ i64,
i8* }`) and fit directly in the 16-byte payload — no allocation needed
on either side.

## @native Annotations and Call Mapping

The codegen recognizes three call patterns and maps them to native
function names:

| Titrate call | Native name |
|---|---|
| `Math.sin(x)` | `Math_sin` |
| `Math::sin(x)` | `Math_sin` |
| `parseInt(s)` | `parseInt` |

The function `try_native_call_name` in `native_bridge.rs` does the
extraction. Once it has the native name (e.g. `Math_sin`), the codegen:

1. Converts it to the C wrapper symbol: `titrate_Math_sin`.
2. Marshals each argument into a `TitrateValue` and stores it in an
   args array.
3. Emits a `call` to the wrapper.
4. Unmarshals the returned `TitrateValue` back into the expected LLVM
   type.

The `@native` annotation (used internally by the standard library to
mark functions that should be looked up in the native table rather than
compiled from their body) is what tells the analyzer a function is a
native call. The codegen then uses the call-pattern matching above to
find the right wrapper.

## Adding New Native Functions

To add a new native function to the bridge:

### 1. Register it in the bytecode VM

Add the function to the appropriate module in
`trc/src/bytecode/vm/natives/` (e.g. `math.rs` for a math function).
The function must be registered in the lookup table that
`lookup_builtin_native` searches. Follow the existing pattern — the
function takes a `&[Value]` slice and returns a `Value`.

### 2. Verify the wrapper exists

The `titrate_native` crate's generic dispatch (`titrate_native_call`)
can invoke any registered native function by name, so for most
functions you don't need to write a dedicated wrapper. The codegen
will emit a call to `titrate_<Name>` and the linker will find it via
the generic dispatch.

For hot-path functions (called in tight loops), write a dedicated
wrapper in `titrate_native/src/wrappers.rs` to avoid the generic
dispatch overhead. The wrapper should:

- Have the signature `TitrateValue titrate_<Name>(const TitrateValue* args, size_t arg_count)`.
- Unpack the arguments from the `TitrateValue` array.
- Call the underlying native function directly.
- Pack the result into a `TitrateValue` and return it.

### 3. Test it

Add a test in `trc/tests/` that calls the new function from native
code. The `native_bench.rs` test file has examples of native-call
tests.

## The titrate_native Crate Structure

```
titrate_native/
├── Cargo.toml
├── titrate_native.h          # C header for the TitrateValue struct
└── src/
    ├── lib.rs                # Direct helpers + generic dispatch
    └── wrappers.rs           # Dedicated wrappers for hot-path functions
```

### `lib.rs` — Direct Helpers and Generic Dispatch

The direct helpers are the functions the codegen calls directly
(bypassing the generic bridge):

- `titrate_println(len, ptr)` — write a string + newline to process stdout via `io::stdout().write_all(...)` and `io::stdout().flush()`. There is no captured or buffered stdout; bytes are flushed to the process's real stdout handle immediately.
- `titrate_string_concat(a_len, a_ptr, b_len, b_ptr, out_len)` —
  concatenate two strings into a fresh buffer.
- `titrate_malloc(size)` — allocate `size` bytes, return `i8*`.
- `titrate_free(ptr)` — free a previously-allocated buffer.
- `titrate_native_call(name_ptr, name_len, args, arg_count)` —
  generic dispatch: look up `name` in the native table and call it.

The generic dispatch path serializes arguments through the
`TitrateValue` array, looks up the function by name in the bytecode
VM's native table, calls it, and serializes the result back. It's
correct for all 359 functions but slower than a dedicated wrapper.

### `wrappers.rs` — Dedicated Wrappers

Dedicated wrappers exist for functions that show up in hot loops:
`Math_sin`, `Math_cos`, `Math_sqrt`, `String_length`, `String_charAt`,
and a handful of others. Each wrapper unpacks its arguments directly
from the `TitrateValue` array, calls the underlying Rust function, and
packs the result. This avoids the generic dispatch's name lookup and
serialization overhead.

If you're profiling a native build and see `titrate_native_call` in the
hot path, that's a sign you need a dedicated wrapper for the function
being called.

## Example: Wrapping a C Library

Suppose you want to call a C library function — say, `cblas_ddot` from
BLAS — from Titrate. Here's the end-to-end process.

### 1. Declare the function in Titrate

In your `.tr` source, declare the function with `@native`:

```titrate
// Declared but not defined — the body comes from the native bridge.
@native
public fn cblas_ddot(n: int, x: Array<double>, y: Array<double>): double;
```

The analyzer treats `@native` functions as opaque: they have a
signature but no body. The codegen will emit a call to the C wrapper
instead of compiling a body.

### 2. Write the wrapper in titrate_native

In `titrate_native/src/wrappers.rs`, add a dedicated wrapper:

```rust
#[no_mangle]
pub extern "C" fn titrate_cblas_ddot(
    args: *const TitrateValue,
    arg_count: usize,
) -> TitrateValue {
    let args = unsafe { std::slice::from_raw_parts(args, arg_count) };
    let n = args[0].as_int() as i32;
    let x = args[1].as_double_slice();  // helper to extract Array<double>
    let y = args[2].as_double_slice();

    // Call into the linked C BLAS library.
    let result = unsafe {
        cblas_ddot(n, x.as_ptr(), 1, y.as_ptr(), 1)
    };

    TitrateValue::from_double(result)
}
```

You'll also need to link the C BLAS library. Add it to
`Titrate.toml`:

```toml
[native]
linker_flags = ["-lcblas"]   # or "-lblas", "-lmkl_rt", etc.
```

### 3. Call it from Titrate

```titrate
public fn main(): void {
    let x: Array<double> = Array.of(1.0, 2.0, 3.0);
    let y: Array<double> = Array.of(4.0, 5.0, 6.0);
    let dot: double = cblas_ddot(3, x, y);
    io::println("dot = " + Double.toString(dot));  // 32.0
}
```

The codegen emits a call to `titrate_cblas_ddot`, which unpacks the
arguments, calls the real C `cblas_ddot`, and packs the result. The
linker pulls in `libcblas` (or whatever you specified) to resolve the
C symbol.

### 4. Test and profile

Build with `trc --native --release` and verify the result. If the
function is in a hot loop, profile to confirm the wrapper overhead is
small relative to the C library's work. If the wrapper itself is the
bottleneck, consider inlining the C call directly into the LLVM IR
(via `extern` declaration) rather than going through the
`TitrateValue` marshalling layer.

## See Also

- [Why Native?](./native-intro) — what the native backend is and when
  to use it.
- [Building Native Binaries](./native-build) — prerequisites, flags,
  and the `[native]` section of `Titrate.toml`.
- [Ownership on LLVM](./native-ownership) — how `Owned<T>`, borrows,
  and regions lower to LLVM IR.
- [Compiler Architecture](./architecture) — how the front-end feeds
  both backends.
