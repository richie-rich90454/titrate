# Memory

The `tt::lang::Memory` module provides C++ `<memory>` parity (smart pointers and allocator machinery). Titrate is GC-managed, so these smart pointers are thin abstractions that model C++ ownership semantics (unique, shared, weak) on top of GC-collected objects. They expose the standard surface area (`get`, `reset`, `release`, `use_count`, `swap`, `move`, `lock`) so ported C++ code keeps the same call sites.

## Import

```titrate
import tt::lang::Memory;
```

## API Reference

### `ControlBlock`

ControlBlock tracks the strong and weak reference counts for a `SharedPtr`. The weak count is the number of weak references plus one (the "self" reference from strong pointers) so that the block itself is reclaimed only when the last weak pointer is gone.

**Fields:**
- `strongCount: int`
- `weakCount: int`
- `object: Variant`

**Constructor:**
- `init(obj: Variant)` — strong=1, weak=1, object=obj

**Methods:**
- `addStrong(): void` — increment strong count
- `releaseStrong(): bool` — release a strong reference; returns `true` if the object should be destroyed (strong count dropped to zero)
- `addWeak(): void` — increment weak count
- `releaseWeak(): bool` — release a weak reference; returns `true` if the control block can be reclaimed
- `useCount(): int` — return the strong count
- `expired(): bool` — returns `true` if the strong count is zero

### `UniquePtr<T>`

`UniquePtr` owns its object exclusively. Move semantics are modeled by `move()`, which transfers ownership and nulls the source. Copying is not provided (the C++ `delete` of the copy constructor is mirrored by simply not offering a copy method).

**Constructors:**
- `init(obj: Variant)` — wrap an object
- `initEmpty()` — construct an empty `UniquePtr` (`nullptr` analog)

**Methods:**
- `get(): Variant` — return the held object (or `null`)
- `release(): Variant` — release ownership and return the held object; the `UniquePtr` becomes empty
- `reset(obj: Variant): void` — replace the held object (previous object is logically released; GC reclaims it)
- `swap(other: UniquePtr<T>): void` — swap contents with another `UniquePtr`
- `move(): UniquePtr<T>` — transfer ownership to a new `UniquePtr` and null this one
- `isNull(): bool` — returns `true` if the held object is `null`
- `isValid(): bool` — returns `true` if owning a non-null object
- `toString(): string` — returns `"UniquePtr(null)"` or `"UniquePtr(occupied)"`

### `SharedPtr<T>`

`SharedPtr` shares ownership of an object via reference counting. Every copy increments the strong count; every reset/destruction decrements it.

**Constructor:**
- `init(obj: Variant)` — create with a new control block (or `null` block if `obj` is `null`)

**Methods:**
- `get(): Variant` — return the held object (or `null` if empty/expired)
- `useCount(): int` — return the number of `SharedPtr` instances sharing the object
- `unique(): bool` — returns `true` if this is the only `SharedPtr` owning the object
- `reset(obj: Variant): void` — replace the held object; decrements the previous strong count
- `clear(): void` — clear this `SharedPtr` without assigning a new object
- `swap(other: SharedPtr<T>): void` — swap contents with another `SharedPtr`
- `share(): SharedPtr<T>` — create a new `SharedPtr` sharing the same object (increments strong count)
- `isNull(): bool` — returns `true` if the block is null or the object is null
- `rawBlock(): ControlBlock` — expose the underlying control block for `WeakPtr` construction
- `adoptBlock(block: ControlBlock): void` — adopt an existing control block whose strong count has already been incremented
- `toString(): string` — returns `"SharedPtr(null)"` or `"SharedPtr(use_count=...)"`

### `WeakPtr<T>`

`WeakPtr` holds a non-owning reference to a `SharedPtr`'s control block. `lock()` promotes to a `SharedPtr` if the object is still alive.

**Constructor:**
- `init(shared: SharedPtr<T>)` — create from a `SharedPtr`, incrementing the weak count

**Methods:**
- `expired(): bool` — returns `true` if the managed object has been destroyed
- `useCount(): int` — return the number of `SharedPtr` instances still sharing the object
- `lock(): SharedPtr<T>` — promote to a `SharedPtr` if the object is still alive; otherwise returns an empty `SharedPtr`
- `reset(): void` — release the weak reference
- `toString(): string` — returns `"WeakPtr(null)"` or `"WeakPtr(use_count=..., expired=...)"`

### `Allocator<T>` (interface)

The polymorphic allocator interface (`std::allocator` concept). Titrate is GC-managed, so `allocate`/`deallocate` are advisory bookkeeping.

- `allocate(n: int): Variant`
- `deallocate(p: Variant, n: int): void`
- `equals(other: Allocator<T>): bool`

### `DefaultAllocator<T>`

A no-op allocator that simply returns the passed value (since Titrate objects are GC-allocated). Mirrors `std::allocator<T>`.

**Constructor:**
- `init()`

**Methods:**
- `allocate(n: int): Variant` — returns `n` as a marker `Variant`
- `deallocate(p: Variant, n: int): void` — no-op (GC reclaims memory)
- `equals(other: Allocator<T>): bool`
- `toString(): string` — returns `"DefaultAllocator"`

### `AllocatorTraits`

Provides static-style helpers over an `Allocator`. Mirrors `std::allocator_traits<A>`.

**Constructor:**
- `init()`

**Methods:**
- `allocate<A, T>(alloc: A, n: int): Variant` — allocate `n` objects of type `T` using the given allocator
- `deallocate<A, T>(alloc: A, p: Variant, n: int): void` — deallocate `n` objects previously allocated
- `maxSize<T>(): int` — returns the maximum number of objects that can be allocated (effectively unbounded; returns a large sentinel)

### `PointerTraits`

Provides static-style helpers for pointer-like types. Mirrors `std::pointer_traits<Ptr>`.

**Constructor:**
- `init()`

**Methods:**
- `elementType(p: Variant): string` — returns the element type name for a pointer-like wrapper (runtime type name)
- `nullPointer(): Variant` — returns a null pointer value
- `toAddress(p: Variant): Variant` — returns a pointer to the object held by a smart-pointer-like wrapper (`UniquePtr`/`SharedPtr` aware)

### Free Functions

#### `makeUnique<T>(obj: Variant): UniquePtr<T>`

Construct a `UniquePtr` wrapping the given object.

#### `makeShared<T>(obj: Variant): SharedPtr<T>`

Construct a `SharedPtr` wrapping the given object.

#### `addressOf(obj: Variant): Variant`

Return the address of an object. In Titrate objects are references, so `addressOf` returns the object itself (its identity).

#### `align(alignment: int, size: int): int`

Align the given size up to the nearest multiple of `alignment`. Returns the aligned size.

#### `alignValue(value: long, alignment: long): long`

Align a value/address up to the given alignment. Returns the aligned value.

#### `assumeAligned(value: long, alignment: long): long`

Assert (at runtime) that the given value is aligned to the specified alignment and return it unchanged. Mirrors C++20 `std::assume_aligned`. Returns the value if aligned; otherwise returns the next-aligned value rather than throwing, to remain side-effect free.

#### `isAligned(value: long, alignment: long): bool`

Returns `true` if `value` is a multiple of `alignment`.

## Usage Examples

### UniquePtr

```titrate
import tt::lang::Memory;
import tt::io::IO;

public fn main(): void {
    let ptr: UniquePtr<string> = Memory.makeUnique<string>("hello");
    IO.println("is null: " + (ptr.isNull() ? "true" : "false"));
    let moved: UniquePtr<string> = ptr.move();
    IO.println("after move, source is null: " + (ptr.isNull() ? "true" : "false"));
    moved.reset(null);
    IO.println("after reset, moved is null: " + (moved.isNull() ? "true" : "false"));
}
```

### SharedPtr and reference counting

```titrate
import tt::lang::Memory;

let sp1: SharedPtr<int> = Memory.makeShared<int>(42);
io::println("use count: " + Integer.toString(sp1.useCount()));
let sp2: SharedPtr<int> = sp1.share();
io::println("after share, use count: " + Integer.toString(sp2.useCount()));
io::println("unique: " + (sp1.unique() ? "true" : "false"));
sp2.clear();
io::println("after clear, use count: " + Integer.toString(sp1.useCount()));
```

### WeakPtr

```titrate
import tt::lang::Memory;

let sp: SharedPtr<string> = Memory.makeShared<string>("data");
let wp: WeakPtr<string> = new WeakPtr<string>(sp);
io::println("expired: " + (wp.expired() ? "true" : "false"));
let locked: SharedPtr<string> = wp.lock();
io::println("locked is null: " + (locked.isNull() ? "true" : "false"));
sp.clear();
io::println("after clear, expired: " + (wp.expired() ? "true" : "false"));
```

### Alignment helpers

```titrate
import tt::lang::Memory;

io::println("align(8, 10) = " + Integer.toString(Memory.align(8, 10)));
io::println("alignValue(17, 8) = " + Integer.toString(Memory.alignValue(17, 8) as int));
io::println("isAligned(16, 8) = " + (Memory.isAligned(16, 8) ? "true" : "false"));
io::println("isAligned(17, 8) = " + (Memory.isAligned(17, 8) ? "true" : "false"));
```

### Allocator

```titrate
import tt::lang::Memory;

let alloc: DefaultAllocator<int> = new DefaultAllocator<int>();
let marker: Variant = alloc.allocate(4);
io::println("allocated marker: " + (marker as int));
alloc.deallocate(marker, 4);
let traits: AllocatorTraits = new AllocatorTraits();
io::println("max size: " + Integer.toString(traits.maxSize<int>()));
```
