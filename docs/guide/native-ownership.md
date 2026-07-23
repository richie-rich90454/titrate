# Ownership on LLVM

Titrate's ownership system — `Owned<T>`, borrows (`&T`, `&mut T`),
regions, and `unsafe` blocks — is a front-end concern in the bytecode
VM: the analyzer enforces the rules, and the VM just runs the resulting
bytecode. The LLVM native backend has to do something more interesting.
It has to *lower* those constructs to LLVM IR in a way that preserves
the safety guarantees while still producing fast machine code.

This guide explains how each ownership construct is lowered, what the
generated IR looks like, and how the design compares to Rust's.

## The Big Picture

The native backend's ownership lowering lives in
`trc/src/codegen/llvm/ownership.rs`. The core ideas are:

- **`Owned<T>` is a heap pointer** (`i8*` in LLVM IR). The pointer
  points to a heap allocation managed by `titrate_malloc` /
  `titrate_free`.
- **Each `Owned<T>` local has a drop flag** — an `i1` alloca that
  tracks whether the variable still owns its allocation.
- **Moves clear the drop flag** of the source, so the source's scope
  exit does not double-free.
- **Borrows are raw pointers** — `&T` lowers to `*const T`, `&mut T`
  lowers to `*mut T`. The borrow-checker has already run in the
  analyzer; the codegen just emits the address.
- **Regions are `alloca` + lifetime intrinsics** — `llvm.lifetime.start`
  and `llvm.lifetime.end` mark when region memory is live.
- **`unsafe` blocks are transparent to codegen** — they emit their body
  verbatim. The safety analysis was skipped in the front end; the
  codegen does not add or remove anything.

The result is that ownership has **zero runtime cost** for code that
does not use `Owned<T>`: plain `let` and `var` variables compile to plain
LLVM allocas with no drop flags, no cleanup, no overhead.

## Owned&lt;T&gt;: Drop Flags and Scope-Exit Cleanup

When you write:

```titrate
public fn process(): void {
    let data: Owned<string> = Owned("hello");
    io::println(String.length(data));
}   // data goes out of scope — automatically cleaned up
```

The codegen produces something like this (simplified for readability):

```llvm
define void @process() {
entry:
  ; Allocate space for the Owned<string> pointer and its drop flag.
  %data = alloca i8*
  %data.drop = alloca i1

  ; Construct the Owned value: malloc, store, mark as live.
  %mem = call i8* @titrate_malloc(i64 16)
  store i8* %mem, i8** %data
  store i1 true, i1* %data.drop

  ; ... body: call String.length, call io::println ...

  ; Scope exit: check the drop flag, free if still owned.
  %still_owned = load i1, i1* %data.drop
  br i1 %still_owned, label %free, label %done

free:
  %ptr = load i8*, i8** %data
  call void @titrate_free(i8* %ptr)
  br label %done

done:
  ret void
}
```

The drop flag is the key. It is a single `i1` alloca — one byte on the
stack — that records whether this variable still owns its allocation.
At scope exit, the codegen emits a conditional branch: if the flag is
still true, call `titrate_free`; otherwise skip the free.

### Moves

A move is just an assignment that clears the source's drop flag:

```titrate
let x: Owned<int> = Owned(5);
let y = x;  // x is moved to y
// x is no longer usable here — the analyzer already enforced this
```

Lowered:

```llvm
; let x = Owned(5)
%x = alloca i8*
%x.drop = alloca i1
%mem.x = call i8* @titrate_malloc(i64 4)
store i8* %mem.x, i8** %x
store i1 true, i1* %x.drop

; let y = x  (move)
%y = alloca i8*
%y.drop = alloca i1
%val = load i8*, i8** %x
store i8* %val, i8** %y
store i1 true, i1* %y.drop
store i1 false, i1* %x.drop    ; ← x no longer owns
```

At scope exit, both variables get their drop-flag check. Because `x`'s
flag is false, `titrate_free` is only called once — on `y`. No
double-free.

### Scope-Exit Cleanup Stack

The codegen maintains a **cleanup stack** of `CleanupAction` records,
one per `Owned<T>` local. Each record holds:

- `drop_flag: PointerValue` — the alloca holding the `i1` flag.
- `ptr_alloca: PointerValue` — the alloca holding the `i8*` pointer.

When a scope is entered, a `ScopeMarker` records the current stack
depth. When the scope exits, the codegen pops all cleanups above the
marker (in reverse order) and emits the drop-flag check + conditional
free for each. This ensures cleanups run in the correct order even when
scopes are nested or early-return is used.

## Borrows: &amp;T and &amp;mut T

Borrows lower to raw pointers. `&T` becomes `*const T`, `&mut T`
becomes `*mut T`. The borrow-checker has already run in the analyzer,
so the codegen does not need to insert any runtime checks — it just
emits the address of the borrowed value.

```titrate
let x: Owned<int> = Owned(42);
let r = &x;     // immutable borrow
io::println(Integer.toString(*r));
```

Lowered (simplified):

```llvm
%x = alloca i8*
%x.drop = alloca i1
; ... construct Owned(42) ...

; let r = &x
%r = alloca i8**        ; r is a pointer-to-pointer
store i8** %x, i8*** %r

; *r  (deref)
%addr = load i8**, i8*** %r
%val  = load i8*, i8**  %addr
; ... pass %val to Integer.toString ...
```

The `Expr::RefExpr` AST node computes the address of the referenced
value. For an identifier, that is the existing alloca pointer. For a
temporary (e.g. `&some_expression`), the codegen emits a fresh alloca,
stores the temporary into it, and returns its address.

### Why This Works

The borrow-checker guarantees:

- A `&T` borrow lives at most as long as the borrowed value.
- A `&mut T` borrow is exclusive — no other borrows exist during its
  lifetime.
- The borrowed value is not moved or freed while borrowed.

Because these are compile-time guarantees, the codegen can emit raw
pointers without any runtime overhead. There is no reference counting,
no borrow flag, no runtime check. The pointers are as cheap as C
pointers.

## Regions: alloca + Lifetime Intrinsics

A `region` block allocates memory with a bounded lifetime. All
allocations within the region are freed when the region ends. The
native backend lowers regions to `alloca` (stack allocation) plus
LLVM's `llvm.lifetime.start` and `llvm.lifetime.end` intrinsics.

```titrate
public fn processData(): void {
    region temp {
        let a = temp.alloc(1);
        let b = temp.alloc(2);
        // Use a, b...
    }
    // All of temp's memory is freed at once
}
```

Lowered (simplified):

```llvm
define void @processData() {
entry:
  ; Region allocations become stack allocas.
  %a = alloca i32
  %b = alloca i32

  ; llvm.lifetime.start marks when each slot becomes live.
  call void @llvm.lifetime.start.p0i8(i64 4, i8* %a.bytes)
  store i32 1, i32* %a
  call void @llvm.lifetime.start.p0i8(i64 4, i8* %b.bytes)
  store i32 2, i32* %b

  ; ... use a, b ...

  ; llvm.lifetime.end marks when each slot is no longer needed.
  call void @llvm.lifetime.end.p0i8(i64 4, i8* %a.bytes)
  call void @llvm.lifetime.end.p0i8(i64 4, i8* %b.bytes)
  ret void
}
```

The lifetime intrinsics do not *free* memory — `alloca` memory is freed
automatically when the function returns. What they do is tell LLVM's
optimizer that the slots can be reused for other allocas, which keeps
stack usage low even for functions with many regions.

The codegen tracks a `region_counter` to generate unique names for each
region's allocas, and the `OwnershipContext` records which allocas
belong to which region so cleanup can be emitted correctly.

## Unsafe Blocks: malloc/free

`unsafe` blocks suspend ownership and borrowing checks in the analyzer.
In the codegen, they are transparent — the body is emitted verbatim.
Raw memory operations use `titrate_malloc` and `titrate_free` directly.

```titrate
unsafe {
    let x: Owned<int> = Owned(5);
    let y = x;       // would normally move x
    let z = x;       // allowed in unsafe: no move check
}
```

Lowered, this produces two `Owned<int>` variables that both point to
the same allocation, with both drop flags set to true. At scope exit,
both will try to free the pointer — **this is a double-free bug**, and
it is exactly the kind of thing `unsafe` is supposed to make you
responsible for.

::: warning
`unsafe` does not disable the cleanup code — it disables the *checks*
that would have prevented you from creating a situation the cleanup
code cannot handle. If you use `unsafe` to subvert ownership, you own
the consequences. Keep `unsafe` blocks small, well-documented, and
audited.
:::

The `Expr::UnsafeBlock` AST node just emits its body statements without
any special handling. `Expr::OwnedDeref` loads the value pointed to by
an `Owned<T>` pointer — a plain `load` instruction.

## Comparison with Rust

Titrate's ownership model is inspired by Rust's, but the lowering is
deliberately simpler. Here is how they compare:

| Aspect | Rust | Titrate Native |
|---|---|---|
| Ownership representation | Fat pointer + drop glue in the type | Plain `i8*` + per-variable drop flag |
| Drop logic | Per-type `Drop` impl, called via virtual dispatch | Single `titrate_free` call, guarded by drop flag |
| Move semantics | Bitwise move, original is statically uninitialized | Bitwise move, original's drop flag cleared |
| Borrows | `&T` / `&mut T` with NLL, lifetime annotations inferred | `&T` / `&mut T` raw pointers, borrow-checker ran in analyzer |
| Regions | Lifetime parameters, no runtime equivalent | `region` blocks with `alloca` + lifetime intrinsics |
| `unsafe` | Same semantics: skips checks, codegen unchanged | Same semantics: skips checks, codegen unchanged |
| Zero-cost abstraction | Yes — no runtime overhead for safe code | Yes — plain `let`/`var` compile to plain allocas |

The big difference is that Titrate does not (yet) have per-type drop
glue. An `Owned<T>` is always a heap pointer freed with
`titrate_free`; there is no equivalent of Rust's `impl Drop for T` that
runs custom cleanup logic. This is a deliberate simplification for the
current phase — it makes the codegen much simpler while still
providing the core safety guarantee (no leaks, no double-frees, no
use-after-free for safe code).

The drop-flag approach is also simpler than Rust's "drop flag in the
type" or "zeroed flag after move" approaches. It costs one byte of
stack per `Owned<T>` local, which is negligible, and it makes the
codegen straightforward to reason about.

## See Also

- [Ownership](./ownership) — the language-level guide to `Owned<T>`,
  borrows, regions, and `unsafe`.
- [Why Native?](./native-intro) — what the native backend is and when
  to use it.
- [Wrapping C Libraries](./native-cbind) — the native bridge that
  provides `titrate_malloc` / `titrate_free`.
- [Compiler Architecture](./architecture) — how the front-end feeds
  both backends.
