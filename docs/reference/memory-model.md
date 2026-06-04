# Memory Model

## Stack vs Heap

- Local variables live on the stack by default.
- `Owned<T>` allocates on the heap via `new`.

## Ownership Rules

1. Each `Owned<T>` value has exactly one owner.
2. When the owner goes out of scope, the value is dropped.
3. Assignment transfers ownership (move semantics).
4. After a move, the source variable cannot be used.

## Borrowing

- `&x` creates an immutable borrow.
- `&mut x` creates a mutable borrow.
- Multiple immutable borrows are allowed simultaneously.
- Only one mutable borrow is allowed at a time.
- No borrows may exist while the owner is moved.

## Regions

A `region` block creates a scoped allocation arena:

```titrate
region r {
    let ptr = r.alloc(42);
    // ptr is valid only within this block
}
```

## Unsafe

`unsafe` blocks suspend ownership and borrowing checks:

```titrate
unsafe {
    let ptr = malloc(4);
    *ptr = 0xFEED;
    free(ptr);
}
```
