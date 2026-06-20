# weakref

The `tt.lang.WeakRef` module provides weak references via a global registry. True weak references require VM-level GC integration; this implementation uses a registry so that `collect()` can simulate GC by clearing all registered weak references. This makes `WeakRef` testable and usable in scenarios where manual collection is acceptable.

```titrate
import tt.lang.WeakRef;
```

## WeakRef

A weak reference to an object of type `T`. The referent can be retrieved with `get()` until the reference is cleared (either manually or by `collect()`).

**Methods:**

- `fn init(ref: T)` — create a weak reference to `ref` and register it with the global registry
- `fn get(): T` — return the referenced object, or `null` if it has been cleared or collected
- `fn clear(): void` — clear the reference manually
- `fn isCleared(): bool` — return true if the reference has been cleared

## Top-level Functions

- `fn weakRef<T>(ref: T): WeakRef<T>` — create and return a new `WeakRef` wrapping `ref`
- `fn collect(): int` — simulate garbage collection by clearing all registered live weak references; returns the number cleared
- `fn liveCount(): int` — return the number of live (non-cleared) weak references in the registry
- `fn clearRegistry(): void` — clear the global registry (for testing purposes)

```titrate
import tt.lang.WeakRef;

let ref = WeakRef.weakRef("hello");
io::println(Boolean.toString(ref.isCleared())); // false
io::println(ref.get());                          // hello

let cleared = WeakRef.collect();
io::println(Integer.toString(cleared));          // 1
io::println(Boolean.toString(ref.isCleared()));  // true
```
