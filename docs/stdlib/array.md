# array

The `tt.util.Array` module provides a fixed-size array type. Unlike `ArrayList`, an `Array` has a fixed capacity that cannot change after creation.

```titrate
import tt.util.Array;
```

## Array

A fixed-size, indexable container. Useful when the number of elements is known ahead of time and will not change.

- `fn init(size: int)` — create an array of the given size, filled with null
- `Array.of<T>(elements: T...): Array<T>` — create from varargs
- `Array.filled<T>(size: int, value: T): Array<T>` — create an array filled with a value
- `get(index: int): T` — get element at index; throws on out-of-bounds
- `set(index: int, value: T): void` — set element at index; throws on out-of-bounds
- `size(): int` — the fixed capacity
- `isEmpty(): bool` — true if size is 0
- `fill(value: T): void` — set every slot to the given value
- `toArrayList(): ArrayList<T>` — convert to a dynamic list
- `forEach(action: fn(T): void): void` — iterate with side effect
- `map(mapper: fn(T): T): Array<T>` — transform each element into a new Array
- `clone(): Array<T>` — shallow copy
- `equals<T>(other: T): bool` — structural equality
- `toString(): string` — `"Array[a, b, c]"`

```titrate
let arr = new Array<int>(5);
arr.set(0, 10);
arr.set(1, 20);
io::println(Integer.toString(arr.get(0)));   // 10
io::println(Integer.toString(arr.size()));   // 5

let filled = Array.filled(3, 0);
filled.fill(7);
io::println(Integer.toString(filled.get(1)));  // 7

let doubled = filled.map(fn(n: int): int { return n * 2; });
io::println(Integer.toString(doubled.get(0))); // 14
```
