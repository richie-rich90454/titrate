# `tt::memory` — Smart Pointers & Memory Management

The `Memory` module provides C++‑style smart pointer abstractions over the GC‑managed heap.

## UniquePtr

```titrate
let ptr: UniquePtr<int> = makeUnique<int>(42);
let val: Variant = ptr.get();
ptr.release();
```

**Methods:** `get()`, `release()`, `reset(obj)`, `swap(other)`, `move()`, `isNull()`, `isValid()`

## SharedPtr

Reference‑counted shared ownership:

```titrate
let ptr: SharedPtr<int> = makeShared<int>(42);
let count: int = ptr.useCount();
let copy: SharedPtr<int> = ptr.share();
```

**Methods:** `get()`, `useCount()`, `unique()`, `reset(obj)`, `clear()`, `swap(other)`, `share()`

## WeakPtr

Non‑owning observer:

```titrate
let weak: WeakPtr<int> = new WeakPtr<int>(shared);
let locked: SharedPtr<int> = weak.lock();
```

## Allocator Interface

```titrate
class DefaultAllocator<T> implements Allocator<T> {
    allocate(n: int): Variant;
    deallocate(p: Variant, n: int): void;
}
```

## Utility Functions

- `addressOf(obj)` — get the address of any value
- `align(alignment, size)` — alignment computation
- `assumeAligned(value, alignment)` — assume‑aligned hint
