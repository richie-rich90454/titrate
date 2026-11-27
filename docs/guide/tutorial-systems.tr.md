# Systems Programming Tutorial

This tutorial is for programs that care about *where* bytes live, *when* they are freed, and *what* machine code runs. It covers the five tools Titrate gives you for that work: ownership, regions, generics, the C foreign-function interface (FFI), and native compilation.

The examples are self-contained and use only APIs that exist in `lib/tt` and the VM native-function table. Import paths follow the `tt::module::Name` form; an import may also be written with dots (`tt.module.Name`) if you prefer.

::: tip Who this is for
You want deterministic cleanup, predictable memory use, zero-cost abstractions, or a native executable. If you mainly want to script collections and callbacks, start with [the scripting tutorial](tutorial-scripting.tr.md) instead.
:::

## Part 1 — Ownership

### The one-owner rule

`Owned<T>` wraps a value with exactly one owner. When the owner goes out of scope, the compiler emits cleanup automatically. Assigning the value to another variable *moves* it; the old name becomes invalid.

```titrate
public fn moveDemo(): void {
    let first: Owned<string> = Owned("payload");
    let second = first;          // ownership moves to second
    io::println(String.length(second));
    // io::println(String.length(first));  // ERROR: use after move
}
```

There is no `free()` and no garbage collector. The compiler inserts the cleanup at the point where the owner's scope ends:

```titrate
public fn process(): void {
    let data: Owned<string> = Owned("hello");
    io::println(String.length(data));
}   // data is cleaned up here
```

### Borrowing

A borrow gives you access to the data without transferring ownership. Use `&T` for a read-only borrow and `&mut T` for a mutable one. The original owner keeps responsibility for cleanup.

```titrate
// Read-only borrow: the caller keeps its value.
public fn dataLength(data: &Owned<string>): int {
    return String.length(data);
}

public fn borrowDemo(): void {
    let s: Owned<string> = Owned("hello");
    let len: int = dataLength(&s);
    io::println(Integer.toString(len));
    io::println(String.length(s));   // s is still valid
}
```

Several read-only borrows may coexist, but a mutable borrow excludes every other borrow and cannot coexist with a move:

```titrate
let x = Owned(42);
let r1 = &x;     // first immutable borrow
let r2 = &x;     // second immutable borrow — fine
// x = Owned(6); // ERROR: cannot move while borrowed
// let r3 = &mut x; // ERROR: mutable borrow while immutable borrows exist
```

| Operation | Allowed while borrowed? | Reason |
|-----------|-------------------------|--------|
| Another immutable borrow | Yes | Readers do not interfere |
| Mutable borrow | No | Would invalidate existing readers |
| Move or overwrite the owner | No | Would leave a dangling reference |

### Function boundaries

Choose the parameter type to say what a function does with a value:

```titrate
// Consumes the argument: the caller gives it up.
public fn consume(data: Owned<string>): void {
    io::println(String.length(data));
}

// Observes the argument: the caller keeps it.
public fn peek(data: &Owned<string>): int {
    return String.length(data);
}

public fn callBoth(): void {
    let s: Owned<string> = Owned("hello");
    let n: int = peek(&s);
    io::println(Integer.toString(n));
    consume(s);                      // s is moved here
    // consume(s);                   // ERROR: already moved
}
```

::: warning A move is not a copy
If you need the value in two places, either borrow it or construct a second value. Trying to reuse a moved name is a compile-time error, not a runtime crash.
:::

## Part 2 — Regions and unsafe code

### Regions

A `region` creates a scratch space with a single, predictable lifetime. Allocate with the region's `alloc` method; every allocation is released at once when the block ends. This is ideal for phases of computation that create many short-lived temporaries.

```titrate
public fn processBatch(): void {
    region scratch {
        let a = scratch.alloc(10);
        let b = scratch.alloc(20);
        let c = scratch.alloc(30);
        // a, b and c are valid only inside this block
        io::println(Integer.toString(a));
    }
    // All three allocations are freed together here
}
```

Regions buy you two things:

- **Predictable cleanup** — one release point instead of one per value.
- **Cache-friendly allocation** — allocations in a region are grouped, which reduces fragmentation and allocator traffic.

A practical pattern is to build a temporary index inside a region, use it to drive a computation, and let the whole structure disappear at the phase boundary:

```titrate
public fn phaseOne(): void {
    region temp {
        let offset = temp.alloc(0);
        let count = temp.alloc(8);
        var i: int = 0;
        while (i < 8) {
            let idx = temp.alloc(i);   // scratch values for this phase only
            i = i + 1;
        }
        io::println(Integer.toString(offset + count));
    }
}
```

### Unsafe blocks

`unsafe` suspends the ownership checks for a small, audited region of code. It exists for low-level work such as implementing a container, talking to foreign code, or manipulating raw addresses.

```titrate
unsafe {
    let x: Owned<int> = Owned(5);
    let y = x;       // normally a move
    let z = x;       // allowed here: move checking is suspended
}
```

Use `unsafe` when:

- the compiler cannot prove a hand-written data structure is safe;
- an FFI call returns a raw address whose lifetime you control;
- you have measured and verified a hot loop that needs the checks removed.

Do **not** use it to silence an error you do not understand. Keep each block as small as possible and document why it is sound.

::: warning You are the checker
Inside `unsafe`, memory safety is your responsibility. The compiler no longer guarantees it.
:::

## Part 3 — Generics for systems code

Generics let you write one implementation that is specialized for each concrete type. Titrate monomorphizes at compile time: `Box<int>` and `Box<string>` become separate, statically typed pieces of code with no boxing or dynamic dispatch.

### A generic container

```titrate
public class Box<T> {
    public T value;

    public fn init(value: T) {
        this.value = value;
    }

    public fn get(): T {
        return this.value;
    }

    public fn set(newValue: T): void {
        this.value = newValue;
    }
}

public fn useBoxes(): void {
    let intBox = new Box<int>(42);
    let strBox = new Box<string>("hello");
    io::println(Integer.toString(intBox.get()));
    io::println(strBox.get());
}
```

Use single uppercase letters for type parameters (`T`, `T, R`, `K, V`). Always provide type parameters when you instantiate a generic type; raw forms such as `new ArrayList()` are not valid.

### Generic functions

```titrate
public fn identity<T>(x: T): T {
    return x;
}

public fn map<T, R>(items: ArrayList<T>, f: fn(T): R): ArrayList<R> {
    let result = new ArrayList<R>();
    for (item in items) {
        result.add(f(item));
    }
    return result;
}
```

Call with an explicit or inferred type argument:

```titrate
let a = identity<int>(10);          // explicit
let b = identity("inferred");       // T inferred as string
```

### A real generic type: NDArray

`NDArray<T>` in the standard library is a generic container whose operations are implementations of the patterns above. The factory functions return concrete specializations, and the element type is preserved through indexing:

```titrate
import tt::math::ndarray::NDArray;
import tt::util::ArrayList;
import tt::lang::Double;
import tt::lang::Integer;

public fn matrixDemo(): void {
    let shape: ArrayList<int> = new ArrayList<int>();
    shape.add(2);
    shape.add(3);

    let m: NDArray<double> = zeros(shape);
    m.set2D(0, 1, 5.0);
    let v: double = m.get2D(0, 1);

    io::println(Double.toString(v));        // 5.0
    io::println(Integer.toString(m.rows())); // 2
    io::println(Integer.toString(m.cols())); // 3
}
```

`zeros` and `ones` produce `NDArray<double>`; `fromData<T>(shape, data)` produces an `NDArray<T>` for the element type you supply. Shape changes such as `reshape` and `flatten` share the same data buffer instead of copying it, so they are cheap even for large arrays.

::: tip Zero-cost, not magic
A generic function is compiled once per type used. That gives you static types and direct calls, at the cost of larger binaries when many specializations are instantiated.
:::

## Part 4 — Foreign function interface (FFI)

The `tt::ffi::Ctypes` module is a `ctypes`-style bridge to C libraries. It is backed by native VM primitives (`Ctypes_dlopen`, `Ctypes_dlsym`, `Ctypes_call`) where available and falls back to in-process emulation otherwise.

### C type descriptors

Predefined `CType` constants describe the C ABI types:

| Constant | C type | Size (bytes) |
|----------|--------|--------------|
| `c_bool` | `bool` | 1 |
| `c_char` | `char` | 1 |
| `c_short` / `c_ushort` | `short` / `unsigned short` | 2 |
| `c_int` / `c_uint` | `int` / `unsigned int` | 4 |
| `c_long` / `c_ulong` | `long` / `unsigned long` | 8 |
| `c_longlong` / `c_ulonglong` | `long long` variants | 8 |
| `c_float` | `float` | 4 |
| `c_double` | `double` | 8 |
| `c_char_p` | `char*` | 8 |
| `c_void_p` | `void*` | 8 |

You can also construct your own with `new CType(name, size, kind)`.

### Calling a C function

The reliable pattern is: load the library, resolve a symbol, declare the argument and return types, then call with a list of `Variant` arguments.

```titrate
import tt::ffi::Ctypes;
import tt::util::ArrayList;
import tt::lang::Variant;
import tt::lang::Integer;

public fn callCTime(): void {
    let lib: CDLL = CDLL_load("libc.so.6");   // "msvcrt.dll" on Windows
    let getTime: Func = lib.get("time");
    getTime.setReturnType(c_long);

    let args: ArrayList<Variant> = new ArrayList<Variant>();
    let t: Variant = getTime.call(args);
    io::println("raw time: " + Integer.toString(t.asInt()));
    lib.close();
}
```

For a function with arguments, declare them with `setArgTypes` and add a value for each one:

```titrate
import tt::ffi::Ctypes;
import tt::util::ArrayList;
import tt::lang::Variant;
import tt::lang::Integer;

public fn callCAbs(): void {
    let lib: CDLL = CDLL_load("libc.so.6");
    let cAbs: Func = lib.get("abs");

    let argTypes: ArrayList<CType> = new ArrayList<CType>();
    argTypes.add(c_int);
    cAbs.setArgTypes(argTypes);
    cAbs.setReturnType(c_int);

    let args: ArrayList<Variant> = new ArrayList<Variant>();
    args.add(-7 as Variant);
    let result: Variant = cAbs.call(args);

    io::println("abs(-7) = " + Integer.toString(result.asInt()));
    lib.close();
}
```

Use `WinDLL` instead of `CDLL` for Windows stdcall libraries, and `CDLL_load(name)` as the factory when you want the library loaded at the call site.

### Structures

Subclass `Structure` and declare fields with `_setFields`. Field values are stored as `Variant` under their names, and `sizeOf()` reports the C-style padded size.

```titrate
import tt::ffi::Ctypes;
import tt::util::ArrayList;
import tt::lang::Variant;
import tt::lang::Integer;

public class Point extends Structure {
    public fn init() {
        super.init();
        let fields: ArrayList<Field> = new ArrayList<Field>();
        fields.add(new Field("x", c_int));
        fields.add(new Field("y", c_int));
        this._setFields(fields);
    }
}

public fn structDemo(): void {
    let p = new Point();
    p.setField("x", 10 as Variant);
    p.setField("y", 20 as Variant);

    let x: Variant = p.getField("x");
    io::println("x = " + Integer.toString(x.asInt()));
    io::println("struct size = " + Integer.toString(p.sizeOf()));
}
```

`Union`, `Pointer`, `ByRef`, `pointer()`, `byref()`, `sizeof()`, `alignment()`, and `cast()` are available for the remaining C ABI shapes.

::: warning FFI is a trust boundary
The C library does not participate in Titrate's ownership rules. A wrong signature, a dangling pointer, or a mismatched calling convention can corrupt memory. Validate inputs before they cross the boundary and keep FFI calls behind a small, audited wrapper.
:::

## Part 5 — Native compilation

The default backend compiles to bytecode for the VM. The native backend compiles through LLVM to a real executable, which is what you want when startup time, binary distribution, or tight loops matter.

### Prerequisites

- LLVM 15 or newer (the compiler uses `inkwell` against `LLVM-C.lib` and the LLVM headers).
- A system linker (`link.exe` on Windows, `clang`/`gcc`/`ld` on Linux, `ld64`/`clang` on macOS; `lld` works everywhere).
- The native runtime library, built with `cargo build -p titrate_native`. It produces `libtitrate_native.a` on Unix or `titrate_native.lib` on Windows in `target/debug/`.

### Building

```bash
# Debug native build — faithful translation, no LLVM optimization passes
trc --native hello.tr

# Release native build — enables LLVM optimization passes
trc --native --release hello.tr

# Emit the LLVM IR next to the source for inspection
trc --native --emit-ir examples/hello.tr
```

On Windows the output is `hello.exe`; on Linux and macOS it is `hello`. With `pipette`, use `pipette build --native` and `pipette run --native`.

Debug builds are useful for reading the generated IR but are **not** representative of native performance. Benchmark release builds.

### What happens to ownership in native code

The front-end ownership analysis is identical in both backends; only the lowering changes:

- `Owned<T>` becomes a heap pointer managed by the native runtime allocator.
- Each `Owned<T>` local has a one-bit *drop flag*. At scope exit, codegen branches on that flag and frees the allocation only if it is still owned.
- A move clears the source's drop flag, so the source's scope exit cannot double-free.
- `&T` lowers to `*const T`, `&mut T` to `*mut T`. The borrow checker has already run, so codegen only emits the address.
- Regions become stack allocations bracketed by `llvm.lifetime.start` and `llvm.lifetime.end`.
- `unsafe` blocks are emitted verbatim; no checks are added or removed.

Plain `let` and `var` locals that do not use `Owned<T>` lower to ordinary LLVM allocas with no drop flags and no cleanup overhead. That is the "zero-cost" part: you only pay for the ownership machinery where you use it.

### Choosing a backend

| Need | Backend |
|------|---------|
| Fast edit-run loop, scripting, portability | Bytecode VM |
| Fast startup, native distribution, tight numeric loops | Native (`--native --release`) |
| Inspecting generated code | `--emit-ir` |
| Calling C libraries | Either; FFI is backed by VM primitives in both |

## Quick reference

- **Own one thing:** `Owned<T>`; move it with `=`, lend it with `&` or `&mut`.
- **Free a batch at once:** `region name { ... }` plus `name.alloc(...)`.
- **Write once, specialize many:** `fn f<T>(...)`, `class C<T>`, single-letter type parameters.
- **Call C:** `CDLL_load` → `get` → `setArgTypes` / `setReturnType` → `call`.
- **Ship a binary:** `trc --native --release file.tr`.

## Exercises

1. Write a function that takes `&Owned<string>` and another that takes `Owned<string>`. Call both and confirm which caller can reuse its value.
2. Build a `region`-scoped temporary buffer, then deliberately try to use a value from it after the block to see the lifetime rule in action.
3. Write a generic `swap<T>(box: Box<T>, newValue: T): T` and instantiate it for both `int` and `string`.
4. Wrap one C function behind a small Titrate function that validates its inputs before the FFI call.
5. Compile the same program to bytecode and to a release native binary; compare startup behavior with `--emit-ir` to see how ownership lowers.
