# Ownership

Ownership is Titrate's answer to one of the oldest problems in programming: **how do you manage memory safely without sacrificing performance?** Most languages pick one of two approaches — garbage collection (safe but with runtime overhead) or manual memory management (fast but error-prone). Titrate offers a third path: the compiler tracks who "owns" each piece of data and enforces rules at compile time, so you get safety *and* performance with no runtime cost.

If you're coming from a garbage-collected language, ownership might feel new. If you're coming from C or C++, it'll feel like the guardrails you always wished you had. Let's walk through how it works.

## Why Ownership?

The core problem ownership solves is **memory safety without garbage collection**:

- **Garbage collection** (Java, Python, Go) automatically reclaims memory, but at a cost: unpredictable pauses, runtime overhead, and less control over when things are cleaned up.
- **Manual management** (C, C++) gives you full control, but opens the door to use-after-free, double-free, and memory leaks — bugs that can crash your program or create security vulnerabilities.
- **Ownership** (Titrate, Rust) gives the compiler enough information to insert memory cleanup at the right places automatically. No GC pauses, no manual `free()`, and the compiler catches misuse before your program ever runs.

The trade-off is that you need to think about *who owns* a value and *how long* it's valid. But once you internalize the rules, they become second nature — and the compiler is there to help you every step of the way.

## Owned Values

When you create an `Owned<T>` value, a single variable "owns" that data. Assigning it to another variable **moves** the ownership — the original variable can no longer be used:

```titrate
let x: Owned<int> = new int(5);
let y = x;  // x is moved to y
// io::println(Integer.toString(x));  // ERROR: use after move
```

This might seem restrictive at first, but it's the key insight: **there is always exactly one owner**. When the owner goes out of scope, the compiler automatically cleans up the memory. No double-free, no use-after-free.

```titrate
public fn process(): void {
    let data: Owned<string> = new string("hello");
    // data is used here...
    let length: int = String.length(data);
}   // data goes out of scope — automatically cleaned up
```

::: tip
Think of ownership like physical ownership in the real world. If you give your book to a friend, you no longer have it. If you want both of you to read it, you need a different arrangement — that's where borrowing comes in.
:::

## Borrowing

What if you want to *use* a value without taking ownership of it? That's what **borrowing** is for. A borrow creates a reference (`&`) to the owned value — you can read the data, but the original owner keeps responsibility for cleanup:

```titrate
let x: Owned<int> = new int(5);
let y = &x;      // immutable borrow — y refers to x's data
// x = new int(6);  // ERROR: cannot move while borrowed
```

While a value is borrowed, the owner can't move or modify it. This prevents dangling references — the compiler guarantees the borrowed data stays valid as long as the borrow exists.

### Multiple Immutable Borrows

You can have multiple immutable borrows at the same time — multiple readers are fine as long as nobody is writing:

```titrate
let x: Owned<int> = new int(42);
let r1 = &x;     // first immutable borrow
let r2 = &x;     // second immutable borrow — OK
// Both r1 and r2 can read x's data
```

### Borrowing Rules Summary

| Action | Allowed? | Why |
|--------|----------|-----|
| Multiple immutable borrows | Yes | Read-only access is safe to share |
| Mutable borrow while immutable borrows exist | No | Would invalidate the immutable borrows |
| Move while borrowed | No | Would create a dangling reference |

## Regions

Regions provide a structured way to allocate memory with a bounded lifetime. All allocations within a region are automatically freed when the region ends — no need to track individual cleanups:

```titrate
region r {
    let ptr = r.alloc(42);
    // ptr is valid within this region
    io::println(Integer.toString(ptr));
}
// ptr is no longer valid — all of r's memory is freed at once
```

Regions are especially useful for:

- **Batch processing** — allocate many temporary values during a computation, then free them all at once when done
- **Avoiding fragmentation** — region-based allocation is contiguous, so it's cache-friendly and has minimal overhead
- **Predictable cleanup** — you know exactly when memory is freed (when the region scope ends), unlike garbage collection

```titrate
public fn processData(): void {
    region temp {
        let a = temp.alloc(1);
        let b = temp.alloc(2);
        let c = temp.alloc(3);
        // Use a, b, c...
    }
    // All three allocations freed at once — no individual cleanup needed
}
```

::: tip
Use regions when you have a clear phase of computation that creates many temporary values. The region acts as a "scratch space" that gets wiped clean all at once.
:::

## Unsafe Blocks

`unsafe` blocks suspend ownership and borrowing checks. This is useful for low-level operations that the compiler cannot verify — like interfacing with external C code, implementing data structures with raw pointers, or doing bit-level manipulation:

```titrate
unsafe {
    // ownership rules are suspended inside this block
    let x: Owned<int> = new int(5);
    let y = x;       // would normally move x
    let z = x;       // allowed in unsafe: no move check
}
```

### When to Use unsafe

`unsafe` is a tool for specific situations, not a shortcut to avoid thinking about ownership. Use it when:

- **Interfacing with foreign functions** — calling C libraries through FFI where Titrate's ownership model doesn't apply
- **Implementing low-level data structures** — linked lists, graphs, and other structures where the compiler can't prove safety automatically
- **Performance-critical inner loops** — where you've verified safety manually and need to bypass checks for speed

Avoid `unsafe` when:

- The compiler's ownership rules are just being "annoying" — usually there's a safe way to restructure your code
- You're not sure why the compiler is rejecting your code — the error is probably telling you something important
- You haven't carefully reasoned about the safety of what you're doing

::: warning
Use `unsafe` sparingly — it disables the safety guarantees that the compiler provides. Every `unsafe` block is a place where *you* are responsible for ensuring memory safety, rather than the compiler. Keep unsafe blocks small and well-documented.
:::

## Common Ownership Patterns

### Transfer ownership into a function

When a function takes ownership of a value, the caller can no longer use it:

```titrate
public fn consume(data: Owned<string>): void {
    // data is used and then freed when this function returns
    io::println(String.length(data));
}

public fn main(): void {
    let s: Owned<string> = new string("hello");
    consume(s);
    // s is no longer valid here
}
```

### Borrow to allow continued use

When you want to use a value after passing it to a function, borrow it instead:

```titrate
public fn peek(data: &Owned<string>): int {
    return String.length(data);
}

public fn main(): void {
    let s: Owned<string> = new string("hello");
    let len: int = peek(&s);    // borrow s
    io::println(Integer.toString(len));  // s is still valid
}
```

### Region-scoped temporaries

Use regions to group temporary allocations that share a lifetime:

```titrate
public fn transform(): void {
    region scratch {
        let tmp1 = scratch.alloc(10);
        let tmp2 = scratch.alloc(20);
        // Use tmp1 and tmp2...
    }
    // Both freed at once
}
```

::: tip Try It Yourself
1. Create an `Owned<int>` value and try to use it after assigning it to another variable. Observe the compiler error.
2. Create an `Owned<string>` value, borrow it with `&`, and verify you can still use the original after the borrow.
3. Create a `region` block and allocate a few values inside it. Try to use one of the values after the region ends — what happens?
4. Write a function that takes an `Owned<int>` parameter and another that takes `&Owned<int>`. Call both from `main` and observe the difference in what you can do afterward.
:::
