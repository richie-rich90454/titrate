---
title: LLVM Ownership Lowering — A Deep Dive
author: Titrate Team
date: 2026-06-23
---

# LLVM Ownership Lowering — A Deep Dive

Titrate's ownership system is the part of the language that most
people assume would be hard to lower to LLVM. The analyzer enforces
the rules — moves, borrows, region lifetimes — at compile time. But
the *codegen* still has to emit something that runs those rules
correctly at runtime, without giving up the performance that made
ownership attractive in the first place.

This post is a deep dive on how we did it: the design we ended up
with, the alternatives we rejected, and the lessons we learned along
the way.

## The Challenge

The front-end ownership rules are:

- An `Owned<T>` value has exactly one owner. When the owner goes out
  of scope, the value is cleaned up.
- Assigning an `Owned<T>` to another variable **moves** it — the
  original is no longer usable.
- A borrow (`&T` or `&mut T`) creates a reference to the value without
  taking ownership. The borrow-checker ensures the borrowed value
  stays valid for the borrow's lifetime.
- A `region` block allocates memory with a bounded lifetime. All
  allocations within the region are freed when the region ends.
- An `unsafe` block suspends the checks. The programmer is responsible
  for safety.

The analyzer enforces all of this before codegen runs. By the time we
emit LLVM IR, we know the program is *statically* safe. The question
is: what IR do we emit that *preserves* that safety while still being
fast?

The key tension is **moves**. If `x` owns an allocation and we move
it to `y`, both variables point to the same memory. At scope exit, we
need to free the memory exactly once. We need *some* runtime mechanism
to track which variable currently owns the allocation.

## Drop Flags and Scope-Exit Cleanup

The design we ended up with is the simplest one that works: **per-
variable drop flags**.

Every `Owned<T>` local variable gets two allocas:

1. The pointer itself (`i8*` — a heap pointer to the owned data).
2. A drop flag (`i1` — true if this variable currently owns the
   allocation, false if it's been moved away).

At construction, the pointer is stored and the flag is set to true.
At a move, the pointer is copied to the destination and the source's
flag is set to false. At scope exit, the codegen emits a conditional
branch: if the flag is true, call `titrate_free`; otherwise skip the
free.

```llvm
; let x = Owned(5)
%x = alloca i8*
%x.drop = alloca i1
%mem = call i8* @titrate_malloc(i64 4)
store i8* %mem, i8** %x
store i1 true, i1* %x.drop

; let y = x  (move)
%y = alloca i8*
%y.drop = alloca i1
%val = load i8*, i8** %x
store i8* %val, i8** %y
store i1 true, i1* %y.drop
store i1 false, i1* %x.drop    ; ← x no longer owns

; ... scope exit ...
%still_owned.x = load i1, i1* %x.drop
br i1 %still_owned.x, label %free.x, label %skip.x
free.x:
  %ptr.x = load i8*, i8** %x
  call void @titrate_free(i8* %ptr.x)
  br label %skip.x
skip.x:
%still_owned.y = load i1, i1* %y.drop
br i1 %still_owned.y, label %free.y, label %skip.y
free.y:
  %ptr.y = load i8*, i8** %y
  call void @titrate_free(i8* %ptr.y)
  br label %skip.y
skip.y:
  ret void
```

This is correct by construction: each allocation is freed exactly
once, by the variable that currently owns it.

### Why Not Drop Flags in the Type?

Rust used to have drop flags embedded in the type itself (the "drop
flag" field approach, pre-1.0). The advantage is that you don't need
per-variable allocas — the flag lives in the heap allocation. The
disadvantage is that every owned type pays the memory cost of the
flag, even if it's never moved.

We chose per-variable drop flags for two reasons:

1. **Simpler codegen** — the flag is an alloca, just like the pointer.
   No special handling for heap layout.
2. **Zero cost when unused** — a plain `let x = 5` (no `Owned<T>`)
   compiles to a plain alloca with no flag, no cleanup, no overhead.
   Only `Owned<T>` locals pay the cost.

The cost is one byte of stack per `Owned<T>` local, plus one
conditional branch at scope exit. In practice this is negligible —
`Owned<T>` is used for a small number of long-lived values, not for
every variable in the program.

### The Cleanup Stack

The codegen maintains a **cleanup stack** of `CleanupAction` records,
one per `Owned<T>` local. Each record holds the drop-flag alloca and
the pointer alloca. When a scope is entered, a `ScopeMarker` records
the current stack depth. When the scope exits, the codegen pops all
cleanups above the marker (in reverse order) and emits the drop-flag
check + conditional free for each.

This handles nested scopes and early returns correctly. A `return` in
the middle of a function emits all the cleanups for the scopes being
exited, in reverse order, before the actual `ret`.

## Moves and Borrows

Moves are easy: copy the pointer, clear the source's drop flag. The
analyzer has already verified that the source isn't used after the
move, so the cleared flag is just bookkeeping for the scope-exit
cleanup.

Borrows are even easier: they lower to **raw pointers**. `&T` becomes
`*const T`, `&mut T` becomes `*mut T`. The borrow-checker ran in the
analyzer; the codegen doesn't insert any runtime checks. The
`Expr::RefExpr` AST node computes the address of the referenced value
— for an identifier, that's the existing alloca pointer; for a
temporary, it's a fresh alloca.

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

%r = alloca i8**        ; r is a pointer-to-pointer
store i8** %x, i8*** %r

; *r
%addr = load i8**, i8*** %r
%val  = load i8*, i8**  %addr
; ... pass %val to Integer.toString ...
```

There's no reference counting, no borrow flag, no runtime check. The
pointers are as cheap as C pointers. This works because the
borrow-checker has already guaranteed:

- A `&T` borrow lives at most as long as the borrowed value.
- A `&mut T` borrow is exclusive.
- The borrowed value isn't moved or freed while borrowed.

Those are compile-time guarantees. At runtime, there's nothing to
check.

## Regions: alloca + Lifetime Intrinsics

`region` blocks are interesting because they look like they need
runtime support — "free all the memory when the region ends" — but
they actually don't. We lower them to `alloca` (stack allocation)
plus LLVM's `llvm.lifetime.start` and `llvm.lifetime.end` intrinsics.

```titrate
region temp {
    let a = temp.alloc(1);
    let b = temp.alloc(2);
    // Use a, b...
}
```

Lowered (simplified):

```llvm
%a = alloca i32
%b = alloca i32

call void @llvm.lifetime.start.p0i8(i64 4, i8* %a.bytes)
store i32 1, i32* %a
call void @llvm.lifetime.start.p0i8(i64 4, i8* %b.bytes)
store i32 2, i32* %b

; ... use a, b ...

call void @llvm.lifetime.end.p0i8(i64 4, i8* %a.bytes)
call void @llvm.lifetime.end.p0i8(i64 4, i8* %b.bytes)
ret void
```

The lifetime intrinsics don't *free* memory — `alloca` memory is freed
automatically when the function returns. What they do is tell LLVM's
optimizer that the slots can be reused for other allocas, which keeps
stack usage low even for functions with many regions.

This is a deliberate simplification. A "real" region allocator (arena
allocation, like Rust's `bumpalo`) would heap-allocate a chunk and
bump-allocate within it. That's faster than `malloc` per allocation
but adds complexity. For the current phase, `alloca` + lifetime
intrinsics gives us the correctness guarantee (region memory doesn't
escape the region) without the complexity.

## Unsafe Blocks

`unsafe` blocks are the easiest to lower: they're transparent. The
body is emitted verbatim. The safety analysis was skipped in the front
end; the codegen doesn't add or remove anything.

This means an `unsafe` block that subverts ownership can produce a
double-free at runtime — both variables' drop flags are true, both
try to free the same pointer. That's exactly the kind of bug `unsafe`
is supposed to make you responsible for. The codegen doesn't try to
save you from yourself.

## Lessons Learned

A few things we learned along the way:

### 1. Per-variable drop flags are good enough

We spent a while considering fancier schemes — drop flags in the type,
static analysis to eliminate drop flags when possible, "drop glue"
tables. In the end, per-variable drop flags were simple to implement,
simple to reason about, and fast enough that they don't show up in
profiles. The simplicity is worth the byte of stack per `Owned<T>`
local.

### 2. The borrow-checker does the heavy lifting

The codegen for borrows is trivial — raw pointers, no checks — because
the borrow-checker already did the hard work in the analyzer. This is
the right division of labor: the front-end enforces safety, the
back-end emits fast code. Trying to do borrow-checking in the codegen
would be a nightmare.

### 3. Regions don't need runtime support

We initially planned to implement region allocation with a real arena
allocator. It turned out that `alloca` + lifetime intrinsics gives us
the correctness guarantee for free, and the performance is fine for
the workloads we care about. We can always add a real arena later if
profiling shows it's needed.

### 4. `unsafe` transparency is a feature

Making `unsafe` blocks transparent to codegen means the codegen
doesn't have a special "unsafe mode" that might diverge from the safe
codegen. The same IR is emitted either way; `unsafe` just skips the
front-end checks. This keeps the codegen simple and makes `unsafe`
behavior predictable.

### 5. The cleanup stack is the key data structure

The cleanup stack — a stack of `CleanupAction` records, one per
`Owned<T>` local, with scope markers to track nesting — is the core
data structure that makes the whole thing work. It handles nested
scopes, early returns, and moves correctly, and it's simple enough to
reason about. If you're implementing something similar, start with a
cleanup stack.

## Comparison with Rust

Titrate's ownership lowering is deliberately simpler than Rust's. The
big differences:

| Aspect | Rust | Titrate Native |
|---|---|---|
| Ownership representation | Fat pointer + drop glue in the type | Plain `i8*` + per-variable drop flag |
| Drop logic | Per-type `Drop` impl, virtual dispatch | Single `titrate_free`, guarded by drop flag |
| Move semantics | Bitwise move, original statically uninitialized | Bitwise move, original's drop flag cleared |
| Regions | Lifetime parameters, no runtime equivalent | `region` blocks with `alloca` + lifetime intrinsics |

Rust's approach is more general — per-type drop glue lets types run
arbitrary cleanup logic. Titrate's approach is simpler — `Owned<T>`
is always a heap pointer freed with `titrate_free`. We may add per-type
drop glue in a future phase, but for now the simplicity is worth the
restriction.

## Further Reading

- [Ownership on LLVM](/guide/native-ownership) — the guide version of
  this post, with more code examples.
- [Ownership](/guide/ownership) — the language-level guide to
  `Owned<T>`, borrows, regions, and `unsafe`.
- [Compiler Architecture](/guide/architecture) — how the front-end
  feeds both backends.
