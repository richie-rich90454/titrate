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

## Type codes and byte serialization (Phase 1-2 parity)

### Type codes

`Array` exposes type codes that describe the element layout, mirroring Python's `array` module and C buffer protocols. The type code character identifies the element width and signedness.

| Type code | Element type | Size (bytes) |
|-----------|-------------|--------------|
| `'b'` | signed char | 1 |
| `'B'` | unsigned char | 1 |
| `'h'` | signed short | 2 |
| `'H'` | unsigned short | 2 |
| `'i'` | signed int | 4 |
| `'I'` | unsigned int | 4 |
| `'l'` | signed long | 8 |
| `'L'` | unsigned long | 8 |
| `'f'` | float | 4 |
| `'d'` | double | 8 |

- `Array.typeCode(): string` — return the type code character for this array's elements
- `Array.itemSize(): int` — return the byte size of one element

### fromBytes / toBytes

- `Array.fromBytes<T>(typeCode: string, bytes: ArrayList<byte>): Array<T>` — construct an `Array` by reinterpreting a byte sequence with the given type code
- `Array.toBytes(): ArrayList<byte>` — serialize the array to a flat byte list

```titrate
let arr = new Array<int>(2);
arr.set(0, 0x01020304);
arr.set(1, 0x05060708);

let bytes: ArrayList<byte> = arr.toBytes();
io::println(Integer.toString(bytes.size()));  // 8 (two 4-byte ints)

let restored = Array.fromBytes<int>("i", bytes);
io::println(Integer.toString(restored.get(0)));  // 0x01020304
io::println(Integer.toString(restored.typeCode()));  // "i"
io::println(Integer.toString(arr.itemSize()));       // 4
```
