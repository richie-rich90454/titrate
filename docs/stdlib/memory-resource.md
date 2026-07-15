# MemoryResource

The `tt::lang::MemoryResource` module provides C++ `<memory_resource>` parity. It implements the polymorphic memory resource (PMR) surface area (`do_allocate` / `do_deallocate` / `do_is_equal`) plus pool and monotonic resource shapes. Because Titrate is GC-managed, these classes are thin abstractions over allocation tracking rather than real allocators — ported C++ code keeps the same call sites while the VM manages memory.

## Import

```titrate
import tt::lang::MemoryResource;
```

## API Reference

### `MemoryResource` (interface)

The polymorphic interface (`std::pmr::memory_resource`).

**Methods:**
- `allocate(bytes: long, alignment: long): Variant` — public allocate, dispatches to `do_allocate`
- `deallocate(p: Variant, bytes: long, alignment: long): void` — public deallocate, dispatches to `do_deallocate`
- `isEqual(other: MemoryResource): bool` — public equality, dispatches to `do_is_equal`
- `resourceName(): string` — returns a human-readable name for this resource

### `AllocationRecord`

Tracks one logical allocation made through a resource.

**Fields:**
- `bytes: long` — allocation size
- `alignment: long` — requested alignment
- `timestamp: long` — when the allocation was made

**Constructor:**
- `init(bytes: long, alignment: long)`

### `PolymorphicAllocator`

Wraps a `MemoryResource` and forwards allocations to it (`std::pmr::polymorphic_allocator`).

**Constructor:**
- `init(resource: MemoryResource)`

**Methods:**
- `resource(): MemoryResource` — returns the underlying memory resource
- `allocate(bytes: long): Variant` — allocate `bytes` with default alignment 8
- `allocateAligned(bytes: long, alignment: long): Variant` — allocate with explicit alignment
- `deallocate(p: Variant, bytes: long): void` — deallocate with default alignment
- `deallocateAligned(p: Variant, bytes: long, alignment: long): void` — deallocate with explicit alignment
- `equals(other: PolymorphicAllocator): bool` — compare by underlying resource
- `toString(): string`

### `MonotonicBufferResource`

A bump allocator that never frees individual blocks; all memory is released when `release()` is called (`std::pmr::monotonic_buffer_resource`). Backed by an `ArrayList` of allocation records.

**Constructors:**
- `init()` — default with initial buffer size 1024
- `initWithSize(initialSize: long)` — explicit initial buffer size hint

**Methods:**
- `allocate(bytes: long, alignment: long): Variant` — track an allocation
- `deallocate(p: Variant, bytes: long, alignment: long): void` — no-op (monotonic resources free on `release()`)
- `isEqual(other: MemoryResource): bool` — same object identity
- `resourceName(): string` — returns `"monotonic_buffer_resource"`
- `release(): void` — release all allocations at once (destructor analog)
- `numAllocations(): int` — current number of tracked allocations
- `totalAllocated(): long` — total bytes ever allocated
- `totalDeallocated(): long` — total bytes logically deallocated
- `bytesUsed(): long` — bytes currently in use

### `UnsynchronizedPoolResource`

A pool allocator that recycles fixed-size blocks without internal locking (`std::pmr::unsynchronized_pool_resource`). Blocks are bucketed by power-of-two size.

**Constructor:**
- `init()`

**Methods:**
- `allocate(bytes: long, alignment: long): Variant` — recycle a block from the pool or record a new one
- `deallocate(p: Variant, bytes: long, alignment: long): void` — return the block to the pool
- `isEqual(other: MemoryResource): bool` — same object identity
- `resourceName(): string` — returns `"unsynchronized_pool_resource"`
- `release(): void` — release all pooled memory
- `numFreeBlocks(): int` — number of free blocks across all pools
- `totalAllocated(): long`
- `totalDeallocated(): long`

### `SynchronizedPoolResource`

Same as `UnsynchronizedPoolResource` but guards every operation with a mutex (`std::pmr::synchronized_pool_resource`).

**Constructor:**
- `init()`

**Methods:** Same as `UnsynchronizedPoolResource` plus thread-safe locking. `resourceName()` returns `"synchronized_pool_resource"`.

### Free Functions

#### `newMonotonicBufferResource(): MonotonicBufferResource`

Returns a new `MonotonicBufferResource` (mirrors `std::pmr::new_monotonic_buffer_resource()`).

#### `newUnsynchronizedPoolResource(): UnsynchronizedPoolResource`

Returns a new `UnsynchronizedPoolResource`.

#### `newSynchronizedPoolResource(): SynchronizedPoolResource`

Returns a new `SynchronizedPoolResource`.

#### `getDefaultResource(): MemoryResource`

Returns the default global resource (a synchronized pool used as the implicit fallback). Mirrors `std::pmr::get_default_resource`.

#### `setDefaultResource(resource: MemoryResource): MemoryResource`

Sets the default global resource and returns the previous one. Only `SynchronizedPoolResource` instances are accepted.

## Usage Examples

### Tracking Allocations with a Monotonic Buffer

```titrate
import tt::lang::MemoryResource;
import tt::io::IO;

public fn main(): void {
    let res: MonotonicBufferResource = MemoryResource.newMonotonicBufferResource();
    res.allocate(64, 8);
    res.allocate(128, 8);
    IO.println("allocations: " + res.numAllocations());   // 2
    IO.println("bytes used: " + res.bytesUsed());          // 192
    res.release();
    IO.println("after release: " + res.numAllocations());  // 0
}
```

### Using a Pool Resource

```titrate
import tt::lang::MemoryResource;

let pool: UnsynchronizedPoolResource = MemoryResource.newUnsynchronizedPoolResource();
let block1: Variant = pool.allocate(32, 8);
pool.deallocate(block1, 32, 8);
// The block is now recycled — a subsequent same-size allocation reuses it.
let block2: Variant = pool.allocate(32, 8);
```

### Polymorphic Allocator Wrapping a Resource

```titrate
import tt::lang::MemoryResource;

let res: MonotonicBufferResource = MemoryResource.newMonotonicBufferResource();
let alloc: PolymorphicAllocator = new PolymorphicAllocator(res);
let p: Variant = alloc.allocate(64);
alloc.deallocate(p, 64);
io::println(alloc.toString());  // PolymorphicAllocator(monotonic_buffer_resource)
```

### Swapping the Default Resource

```titrate
import tt::lang::MemoryResource;

let previous: MemoryResource = MemoryResource.getDefaultResource();
let fresh: SynchronizedPoolResource = MemoryResource.newSynchronizedPoolResource();
MemoryResource.setDefaultResource(fresh);
// ... allocations via the default resource now go through `fresh` ...
MemoryResource.setDefaultResource(previous);  // restore
```
