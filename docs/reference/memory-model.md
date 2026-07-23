# Memory Model

Titrate's memory model combines ownership safety with high-level ergonomics. Understanding how memory is allocated, tracked and freed is essential for writing efficient and correct Titrate programs.

## Stack vs Heap

Titrate distinguishes between two primary memory regions: the stack and the heap. Each has different performance characteristics and lifetime semantics.

### Stack Allocation

Local variables of fixed-size, value types are allocated on the stack by default. Stack allocation is extremely fast. It consists of simply moving the stack pointer. Memory is reclaimed automatically when the variable's scope ends.

```
┌─────────────────────────────┐
│  Stack (LIFO)               │
│  ┌───────────────────────┐  │
│  │  main() frame         │  │
│  │  ├─ x: int = 42       │  │
│  │  ├─ y: double = 3.14  │  │
│  │  └─ flag: bool = true │  │
│  └───────────────────────┘  │
│  ┌───────────────────────┐  │
│  │  greet() frame        │  │
│  │  └─ name: string      │  │
│  └───────────────────────┘  │
└─────────────────────────────┘
```

Stack-allocated values include:
- Primitive types (`int`, `double`, `bool`, `char`, `byte`, etc.)
- Local `let` and `var` bindings of value types
- Function call frames (return address, parameters, locals)

```titrate
public fn compute(): int {
    let x: int = 42;        // stack-allocated
    let y: double = 3.14;   // stack-allocated
    let flag: bool = true;  // stack-allocated
    return x;
}
```

### Heap Allocation

Use heap allocation for dynamically-sized data or data that must outlive the current scope. In Titrate, heap allocation occurs when you use `new` to create an object. The `Owned<T>` type can wrap heap-allocated values and enforce single-owner semantics, though `new` itself always returns an unwrapped value.

```
┌─────────────────────────────┐
│  Heap                       │
│  ┌─────┐  ┌─────┐  ┌────┐ │
│  │Point│  │Str  │  │Arr │ │
│  │x=1.0│  │"hi" │  │[1] │ │
│  │y=2.0│  │     │  │[2] │ │
│  └─────┘  └─────┘  └────┘ │
└─────────────────────────────┘
```

```titrate
public fn makePoint(): Point {
    let p: Point = new Point(1.0, 2.0);  // heap-allocated
    return p;  // ownership transferred to caller
}
```

## Value Types vs Reference Types

Titrate's type system distinguishes between value types and reference types, which determines how assignment and parameter passing behave.

### Value Types

Value types store their data directly. Assignment copies the entire value:

| Type Category | Examples |
|---------------|----------|
| Integers | `int`, `long`, `vast`, `byte`, `short`, `u8`, `u16`, `u32`, `u64` |
| Floating-point | `double`, `float`, `half`, `quad` |
| Boolean | `bool` |
| Character | `char` |
| Size | `size` |

```titrate
let a: int = 10;
let b: int = a;   // b gets a COPY of a's value
a = 20;            // does NOT affect b
io::println(Integer.toString(b));  // still 10
```

### Reference Types

Reference types store a pointer to heap-allocated data. Assignment behavior depends on the type:

| Type Category | Examples |
|---------------|----------|
| Classes | `ArrayList`, `HashMap`, user-defined classes |
| Strings | `string` (internally a reference to heap data) |
| Owned pointers | `Owned<T>` |

- **Class instances** and **strings** use reference counting (`Rc`): assignment creates a shared reference to the same data, incrementing the reference count.
- **`Owned<T>`** enforces single-owner semantics at compile time: assignment transfers ownership and the source becomes invalid (move semantics).

## Owned\<T\> in Detail

`Owned<T>` is Titrate's smart pointer type for heap-allocated values with single-owner semantics. It is the primary mechanism for managing heap memory safely.

### How Owned\<T\> Works

1. **Allocation**: `Owned(value)` wraps an existing value (including class instances created with `new`) with single-owner semantics
2. **Single owner**: Exactly one variable owns each `Owned<T>` at a time
3. **Automatic deallocation**: When the owner goes out of scope, the wrapped value is dropped
4. **Move semantics**: Assignment transfers ownership; the source becomes invalid

```titrate
public fn ownedExample(): void {
    let p: Owned<Point> = new Point(1.0, 2.0);
    // p owns the heap-allocated Point

    let q: Owned<Point> = p;  // ownership moves to q
    // p is now invalid — using it is a compile error

    // when q goes out of scope, the Point is freed
}
```

### Ownership Transfer Patterns

Ownership can be transferred in several ways:

```titrate
// Transfer via assignment
let a: Owned<Data> = new Data();
let b: Owned<Data> = a;  // a → b

// Transfer via function return
public fn createData(): Owned<Data> {
    let d: Owned<Data> = new Data();
    return d;  // ownership moves to caller
}

// Transfer via function parameter
public fn processData(d: Owned<Data>): void {
    // d is now owned by this function
    // when processData ends, d is dropped
}
```

## Ownership Rules

Titrate enforces four core ownership rules at compile time:

1. Each `Owned<T>` value has exactly one owner. There is never ambiguity about who is responsible for freeing memory.

2. When the owner goes out of scope, the value is dropped. The compiler automatically inserts drop calls at the end of each scope. No manual `free` is needed.

3. Assignment transfers ownership (move semantics). Unlike garbage-collected languages where assignment creates another reference, Titrate moves ownership to the new variable.

4. After a move, the source variable cannot be used. This prevents use-after-free bugs and dangling references.

```titrate
public fn ownershipDemo(): void {
    let a: Owned<Buffer> = new Buffer();

    let b: Owned<Buffer> = a;  // ownership moves: a → b

    // Compile error: a was moved
    // a.write("hello");
}
```

## Borrowing

Borrowing allows you to reference an `Owned<T>` value without taking ownership. Borrows are always temporary and scoped.

### Immutable Borrows

Use `&x` to create an immutable borrow. Multiple immutable borrows can coexist because they cannot modify the underlying data:

```titrate
public fn immutableBorrow(): void {
    let data: Owned<Data> = new Data();
    let ref1: &Data = &data;   // immutable borrow #1
    let ref2: &Data = &data;   // immutable borrow #2 — OK!
    // Both ref1 and ref2 can read data
}
```

### Mutable Borrows

Use `&mut x` to create a mutable borrow. Only one mutable borrow can exist at a time, and no immutable borrows may coexist with it:

```titrate
public fn mutableBorrow(): void {
    var data: Owned<Data> = new Data();
    let ref1: &mut Data = &mut data;  // mutable borrow — OK
    ref1.update();

    // Compile error: cannot borrow immutably while mutably borrowed
    // let ref2: &Data = &data;
}
```

### Borrowing Rules Summary

| Rule | Description |
|------|-------------|
| Multiple `&T` | Allowed — read-only access is safe to share |
| Single `&mut T` | Only one mutable borrow at a time |
| No `&T` alongside `&mut T` | Readers and writers cannot coexist |
| No borrows after move | Cannot borrow a moved value |

### Borrow Scopes

Borrows are valid only within their scope. Once a borrow goes out of scope, the original owner can be borrowed again:

```titrate
public fn borrowScoping(): void {
    var data: Owned<Data> = new Data();

    {
        let r: &mut Data = &mut data;
        r.update();
        // r goes out of scope here
    }

    // Now it is safe to borrow again
    let r2: &Data = &data;  // OK — previous mutable borrow ended
}
```

## Region-Based Allocation

A `region` block creates a scoped allocation arena. The region frees all allocated memory in bulk when it exits. This provides deterministic, high-performance memory management without the overhead of individual deallocations.

### How Regions Work

```
┌──────────────────────────────────┐
│  Region "r"                      │
│  ┌──────┐  ┌──────┐  ┌──────┐  │
│  │ obj1 │  │ obj2 │  │ obj3 │  │
│  └──────┘  └──────┘  └──────┘  │
│                                  │
│  ← All freed at once on exit →  │
└──────────────────────────────────┘
```

```titrate
region r {
    let ptr = r.alloc(42);
    let ptr2 = r.alloc(100);
    // ptr and ptr2 are valid only within this block
}
// All memory in region r is freed instantly
```

### When to Use Regions

- Batch processing: When you need many temporary objects that share a lifetime
- Game loops: Per-frame allocations that are all freed at frame end
- Parsing: Intermediate data structures that become irrelevant after parsing
- Performance-critical code: Avoiding per-object deallocation overhead

### Region vs Owned

| Feature | `Owned<T>` | `region` |
|---------|-----------|----------|
| Deallocation | Automatic per-object | Bulk at region end |
| Ownership | Single owner | Region owns all |
| Borrowing | Full borrow rules | Borrows valid within region |
| Overhead | Per-object drop | Single bulk free |
| Use case | General purpose | Batch/temporary data |

## Garbage Collection

Titrate does not use a garbage collector. Instead, it relies on:

1. Stack allocation for value types — freed automatically on scope exit
2. Ownership for heap allocation — deterministic deallocation when the owner drops
3. Regions for batch allocation — bulk deallocation when the region ends

This design provides:
- No pause times — there is no stop-the-world GC cycle
- Deterministic cleanup — you know exactly when memory is freed
- Lower memory overhead — no GC metadata or write barriers
- Predictable performance — no allocation spikes from GC promotion

### Why No GC?

Garbage collectors trade runtime overhead for developer convenience. Titrate's ownership system provides the same safety guarantees (no use-after-free, no dangling pointers) without the runtime cost. The compile-time borrow checker ensures memory safety, making a runtime GC unnecessary.

## Memory Layout of Objects

### Object Layout

Every heap-allocated class instance in Titrate is stored with three key pieces of metadata:
- class_name — identifies the class (e.g. "Point", "ArrayList")
- fields — a map of field names to their values
- vtable — a map of method names to function indices

```
┌──────────────────────────────┐
│  ClassInstance               │
│  ├─ class_name: "Point"     │
│  ├─ fields (HashMap)        │
│  │   ├─ "x" → Value::Double │
│  │   └─ "y" → Value::Double │
│  └─ vtable (method map)     │
└──────────────────────────────┘
```

### Field Layout

Fields are laid out in declaration order, with alignment padding as needed:

```titrate
public class Point {
    public double x;   // offset 0, 8 bytes
    public double y;   // offset 8, 8 bytes
}
// Total: 16 bytes (plus metadata)
```

```titrate
public class Mixed {
    public bool flag;    // offset 0, 1 byte + 7 padding
    public double value; // offset 8, 8 bytes
    public int count;    // offset 16, 4 bytes + 4 padding
}
// Total: 24 bytes (plus metadata)
```

## How Strings Are Stored

Strings in Titrate are immutable, heap-allocated reference types. Internally, strings are stored as reference-counted UTF-8 sequences:

```
┌────────────────────────────┐
│  String (Rc<String>)       │
│  └─ UTF-8 encoded bytes   │
│      ├─ 'h' (0x68)        │
│      ├─ 'e' (0x65)        │
│      ├─ 'l' (0x6C)        │
│      ├─ 'l' (0x6C)        │
│      └─ 'o' (0x6F)        │
└────────────────────────────┘
```

Key properties:
- **Immutable**: Once created, a string's contents never change
- **UTF-8 encoded**: All strings use UTF-8 internally
- **Reference-counted**: Multiple variables can share the same string data via `Rc` (reference counting), with no copying on assignment
- **O(n) character length**: `String.length(s)` counts Unicode characters, which requires iterating over the UTF-8 bytes (O(n))

```titrate
let s: string = "hello";       // string literal → heap allocation
let len: int = String.length(s); // counts Unicode characters (O(n))
let upper: string = String.toUpperCase(s); // creates a new string
```

## How Arrays Are Stored

Arrays (`ArrayList<T>`) are heap-allocated class instances with a contiguous backing buffer managed internally:

```
┌─────────────────────────────────┐
│  ArrayList<int> Instance        │
│  ├─ _elements: Array            │  ← value array (Vec<Value>)
│  │   ├─ [0] = 10               │
│  │   ├─ [1] = 20               │
│  │   ├─ [2] = 30               │
│  │   ├─ [3] = (unused)         │
│  │   ├─ ...                    │
│  │   └─ [7] = (unused)         │
│  └─ (internal capacity: 8)    │
└─────────────────────────────────┘
```

### Resizing Strategy

When the buffer is full and a new element is added:
1. A new buffer of **twice the current capacity** is allocated
2. All existing elements are copied to the new buffer
3. The old buffer is freed
4. The new element is inserted

This amortized O(1) approach means that while individual insertions may be O(n), the average cost across many insertions is constant.

```titrate
let list: ArrayList<int> = new ArrayList<int>();  // initial capacity
list.add(10);
list.add(20);
list.add(30);
io::println(Integer.toString(list.size()));  // 3
```

## Reference Counting vs Ownership

Titrate uses **ownership** as its primary memory management strategy, not reference counting. Here is how they compare:

| Aspect | Ownership (Titrate) | Reference Counting |
|--------|-------------------|-------------------|
| Cycle safety | No cycles possible | Cycles cause leaks |
| Overhead | Zero runtime cost | Increment/decrement on every copy |
| Determinism | Fully deterministic | Deterministic (but with overhead) |
| Complexity | Compile-time borrow checker | Runtime tracking |
| Thread safety | Enforced at compile time | Requires atomic ops |

### Why Ownership Over Reference Counting?

Reference counting has two major drawbacks:
1. **Cycle leaks**: If object A references B and B references A, neither's count ever reaches zero
2. **Performance overhead**: Every assignment and scope exit triggers count adjustments

Titrate's ownership model avoids both problems by making the ownership graph a tree (no cycles) and performing all analysis at compile time.

## The `unsafe` Escape Hatch

`unsafe` blocks suspend the ownership and borrowing checks, allowing direct memory manipulation. This is intended for FFI, low-level systems programming, and performance-critical code where the compiler cannot verify safety.

### Syntax

```titrate
unsafe {
    let ptr = malloc(4);
    *ptr = 0xFEED;
    free(ptr);
}
```

### When to Use `unsafe`

- FFI bindings: Calling C library functions that work with raw pointers
- Performance kernels: Tight loops where borrow-checker overhead matters
- Self-referential structures: Data structures the borrow checker cannot verify
- Interfacing with the VM: Direct memory access for native extensions

### Safety Guarantees

Inside an `unsafe` block, four checks are disabled:
- Borrow rules (multiple mutable borrows allowed)
- Move-after-use checks
- Null dereference checks
- Bounds checks on raw pointer access

You are responsible for upholding safety invariants manually. Incorrect `unsafe` code can cause segmentation faults, data corruption and undefined behavior.

### Best Practices

1. Minimize `unsafe` scope — keep blocks as small as possible
2. Document invariants — explain why the `unsafe` code is correct
3. Wrap in safe API — expose only safe functions to callers
4. Test thoroughly — `unsafe` code needs extra scrutiny

```titrate
// Good: small unsafe block, wrapped in safe function
public fn readByte(address: long): byte {
    unsafe {
        return *(address as *byte);
    }
}
```

## Performance Implications

### Stack vs Heap Performance

| Operation | Stack | Heap |
|-----------|-------|------|
| Allocation | ~one instruction | Multiple instructions |
| Deallocation | ~one instruction | Requires drop or region exit |
| Cache locality | Excellent | May cause cache misses |
| Fragmentation | None | Possible |

### Optimization Tips

1. Prefer value types when data is small and has a known lifetime
2. Use regions for batch allocations with the same lifetime
3. Avoid unnecessary `new` — stack-allocate when possible
4. Reuse objects instead of creating new ones in hot loops
5. Pre-size collections — pass initial capacity to `ArrayList` and `HashMap` when the size is known

```titrate
// Good: pre-sized collection
let list: ArrayList<int> = new ArrayList<int>();
list.ensureCapacity(10000);  // avoid repeated resizes

// Good: reuse mutable variable
var i: int = 0;
while (i < 1000) {
    io::println(Integer.toString(i));
    i = i + 1;
}
```

## Comparison with Other Languages

### Titrate vs Rust

Titrate's ownership model is inspired by Rust but simplified:

| Feature | Titrate | Rust |
|---------|---------|------|
| Ownership | `Owned<T>` with move semantics | Every value has ownership |
| Borrowing | `&T` and `&mut T` | `&T` and `&mut T` |
| Lifetimes | Implicit (region-based) | Explicit (`'a`) |
| Regions | Built-in `region` blocks | Not built-in |
| Unsafe | `unsafe` blocks | `unsafe` blocks |
| Complexity | Simpler — fewer annotations | More expressive but steeper learning curve |

Titrate omits Rust's explicit lifetime annotations in favor of region-based scoping, which is less expressive but easier to learn.

### Titrate vs C#

| Feature | Titrate | C# |
|---------|---------|------|
| Memory management | Ownership + regions | Garbage collection |
| Deterministic cleanup | Yes (scope-based) | No (finalizers deprecated) |
| Pause times | None | GC pauses possible |
| Null safety | `Optional<T>` | `Optional<T>` (library) |
| Memory overhead | Low (no GC metadata) | Higher (object headers, GC) |
| Learning curve | Moderate (ownership) | Low (GC handles everything) |

Titrate trades the simplicity of garbage collection for deterministic performance and lower memory overhead.
