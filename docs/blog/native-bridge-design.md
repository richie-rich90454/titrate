---
title: Design of the Native Bridge
author: Titrate Team
date: 2026-06-23
---

# Design of the Native Bridge

The native backend compiles Titrate to machine code, but a Titrate
program is not just math — it calls `io::println`, `ArrayList.add`,
`MathTrig.sin`, `HttpClient.get`, and 349 other native functions that
live in the Rust runtime. The **native bridge** is what lets compiled
Titrate code call those functions.

This post covers the design of the bridge: the C-ABI value model, the
marshalling strategy, how we wrapped all 353 native functions, and the
performance considerations that shaped the design.

## The Problem

The bytecode VM has it easy. Its native functions take `&[Value]`
slices of the VM's tagged-union `Value` type and return a `Value`.
Everything is in the same process, using the same representation, with
no marshalling.

The native backend has it harder. The compiled Titrate code is
machine code — `double`s in floating-point registers, `i64`s in
integer registers, `i8*` pointers in general-purpose registers. The
Rust runtime expects `Value`s (a Rust enum with a tag and a payload).
Somehow we have to bridge these two worlds.

We considered three approaches:

1. **Reimplement everything in Rust as `extern "C"` functions.** Write
   `titrate_Math_sin(double) -> double`, `titrate_ArrayList_add(...)` ,
   etc., by hand. Correct, but 353 functions is a lot of boilerplate,
   and we would have to keep the Rust implementations in sync with the
   bytecode VM's.

2. **Serialize everything through a generic dispatch function.** Have
   one `titrate_native_call(name, args, arg_count)` function that
   looks up `name` in the native table and calls it. Simple, but the
   per-call overhead (name lookup, serialization, deserialization) is
   high.

3. **A hybrid: generic dispatch for most functions, dedicated wrappers
   for hot paths.** Use the generic dispatch as the default, and write
   dedicated `extern "C"` wrappers for the functions that show up in
   profiles.

We went with option 3. It gives us correctness for all 353 functions
out of the box (via generic dispatch) and lets us optimize the hot
paths incrementally.

## The C-ABI Value Model

The bridge needs a single C type that can represent any Titrate value.
We use a 24-byte tagged union:

```c
// titrate_native/titrate_native.h
typedef struct {
    int32_t tag;       // type tag (TV_INT, TV_DOUBLE, TV_STRING, ...)
    int32_t pad;       // alignment padding
    uint8_t  payload[16]; // type-specific data
} TitrateValue;
```

The 16-byte payload holds:

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

This uniformity is what lets the generic dispatch work — it can call
any wrapper without knowing the argument types at compile time.

### Why 24 bytes?

We picked 24 bytes for a few reasons:

- **Strings fit.** A Titrate string is `{ i64 length, i8* ptr }` — 16
  bytes. That fits in the payload with no allocation.
- **Most primitives fit.** `int`, `long`, `double`, `float`, `char`
  all fit in 16 bytes.
- **Returnable in registers on most ABIs.** x86-64 SysV returns
  16-byte structs in RAX:RDX. 24-byte structs go on the stack, which
  is still fast but not as fast as registers. We accepted this
  trade-off because the alternative (a separate out-parameter) would
  complicate the calling convention.
- **Aligned to 8 bytes.** The `pad` field keeps the payload at an
  8-byte boundary, which is important for `double` and `i64`.

### Why not just use the bytecode VM's `Value` type?

The VM's `Value` type is a Rust enum, which has a Rust-specific layout
that is not stable across compiler versions or platforms. Exposing it
as a C type would be fragile. The `TitrateValue` struct is a plain C
struct with a fixed layout, which is stable and ABI-compatible with
any language that can call C.

## Marshalling Strategy

Marshalling a Titrate value into a `TitrateValue` is a type-pun
through an alloca:

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

The type-pun through an alloca is necessary because the payload is a
`[16 x i8]` array, and we need to store a `double` (or `i64`, or
`{ i64, i8* }`) into it. LLVM's type system does not let you store a
`double` directly into an `[16 x i8]` array; you have to bitcast the
pointer first.

### Strings

Strings are special-cased because they are already two words (`{ i64,
i8* }`) and fit directly in the 16-byte payload. No allocation is
needed on either side — the string data itself lives in the
program's string constant pool or in a heap-allocated buffer, and the
`TitrateValue` just holds a pointer to it.

This is important for performance: string-heavy code (like
`io::println`) does not pay any marshalling overhead beyond copying two
words into the payload.

## Wrapping 353 Native Functions

The bytecode VM has 353 registered native functions, covering math,
strings, collections, I/O, networking, JSON, regex, and more. We
needed all of them to work from native code.

The generic dispatch path (`titrate_native_call`) makes this possible
without writing 353 wrappers by hand. It:

1. Takes a function name (as a string pointer + length) and an array
   of `TitrateValue` arguments.
2. Looks up the name in the bytecode VM's native table
   (`lookup_builtin_native`).
3. Deserializes the `TitrateValue` arguments into the VM's `Value`
   type.
4. Calls the native function.
5. Serializes the result `Value` back into a `TitrateValue`.
6. Returns it.

This works for all 353 functions because they all have the same
Rust-side signature (`fn(&[Value]) -> Value`). The serialization is
type-driven by the `Value` enum's variants.

### The Wrappers We Wrote by Hand

For hot-path functions, we wrote dedicated `extern "C"` wrappers in
`titrate_native/src/wrappers.rs`. These bypass the generic dispatch
and call the underlying Rust function directly, with the arguments
unpacked from the `TitrateValue` array.

The current set of dedicated wrappers covers:

- **Math** — `Math_sin`, `Math_cos`, `Math_sqrt`, `Math_abs`,
  `Math_min`, `Math_max`, and the other hot math functions.
- **String** — `String_length`, `String_charAt`, `String_substring`.
- **I/O** — `titrate_println` (a direct helper, not a wrapper).
- **Memory** — `titrate_malloc`, `titrate_free` (direct helpers).

These are the functions that show up in tight loops. The rest go
through generic dispatch, which is correct but slower.

### When to Write a New Wrapper

The rule is simple: if a function shows up in a profile of a native
build, write a dedicated wrapper for it. The generic dispatch path is
fine for functions called a few times per program (like
`HttpClient.get`); it is not fine for functions called millions of
times (like `MathTrig.sin` in a simulation).

To add a wrapper:

1. Add a `#[no_mangle] pub extern "C"` function in
   `titrate_native/src/wrappers.rs` with the standard signature.
2. Unpack the arguments from the `TitrateValue` array using the
   `as_int()`, `as_double()`, `as_string()` helpers.
3. Call the underlying Rust function.
4. Pack the result into a `TitrateValue` using `from_int()`,
   `from_double()`, etc.
5. Return it.

The codegen does not need to change — it already emits calls to
`titrate_<Name>` for every native function. If a dedicated wrapper
exists, the linker uses it; if not, the generic dispatch handles it.

## Performance Considerations

The bridge's performance is dominated by two things:

1. **Marshalling cost** — packing and unpacking `TitrateValue`s. This
   is a few loads and stores per argument, plus a tag write. For a
   function with N arguments, the marshalling cost is roughly
   `O(N)` memory operations.
2. **Dispatch cost** — for generic dispatch, the name lookup and
   `Value` serialization add a constant overhead per call.

For functions called in tight loops (like `MathTrig.sin` in a
simulation), the dedicated wrappers eliminate the dispatch cost and
minimize the marshalling cost. The wrapper unpacks the argument
directly from the `TitrateValue` payload, calls the Rust `sin`
function, and packs the result. Total overhead: a handful of loads
and stores.

For functions called occasionally (like `HttpClient.get`), the
generic dispatch overhead is invisible — the network round-trip
dominates.

### The Direct Helpers

A few functions are so hot that even the dedicated-wrapper overhead
is too much. For these, we have **direct helpers** that bypass the
`TitrateValue` marshalling entirely:

- `titrate_println(len, ptr)` — takes a string as `(i64, i8*)`, not as
  a `TitrateValue`. This is the single most-called function in most
  programs.
- `titrate_string_concat(a_len, a_ptr, b_len, b_ptr, out_len)` —
  takes two strings as `(i64, i8*)` pairs and returns a freshly
  allocated buffer.
- `titrate_malloc(size)` / `titrate_free(ptr)` — raw memory operations
  for `Owned<T>` and `unsafe` blocks.

These have custom signatures, not the standard
`TitrateValue titrate_<Name>(args, arg_count)` signature. The codegen
recognizes them by name and emits the right call pattern.

## What We Would Do Differently

In hindsight, a few things we would change:

1. **More direct helpers from the start.** We initially planned to use
   generic dispatch for everything and add direct helpers only when
   profiling demanded it. In practice, `titrate_println` and
   `titrate_string_concat` are so hot that they should have been
   direct helpers from day one. We added them late and had to
   re-benchmark.

2. **A code-generation step for wrappers.** Writing dedicated wrappers
   by hand is tedious and error-prone. A codegen step that produces
   wrappers from a function signature table would be better. We
   have not done this yet because the number of hot-path wrappers is
   still small (maybe 20), but it is on the list.

3. **A smaller `TitrateValue`.** 24 bytes is more than we need for
   most calls. A 16-byte struct (tag + 12-byte payload) would fit in
   two registers on x86-64 and be returnable in RAX:RDX. The catch is
   that strings need 16 bytes of payload (`{ i64, i8* }`), so we would
   need a separate string representation. Not worth the complexity for
   the current phase.

## Further Reading

- [Wrapping C Libraries](/guide/native-cbind) — the guide version of
  this post, with a worked example of wrapping a C library.
- [Why Native?](/guide/native-intro) — what the native backend is and
  when to use it.
- [Building Native Binaries](/guide/native-build) — prerequisites and
  flags.
