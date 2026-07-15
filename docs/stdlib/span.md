# span

The `tt.util` module provides `Span<T>` — a non-owning view over an `ArrayList` for efficient range operations without copying.

```titrate
import tt.util.Span;
```

## Span

A lightweight view into a contiguous range of elements in an `ArrayList`. Modifications through the span affect the underlying list, and no elements are copied when creating or slicing a span.

- `fn init(list: ArrayList<T>, start: int, length: int)` — create span over `list` starting at `start` for `length` elements
- `get(index: int): T` — get element at index within the span
- `set(index: int, value: T): void` — set element at index within the span
- `subSpan(start: int, length: int): Span<T>` — create a sub-span relative to this span
- `first(): T` — first element in the span
- `last(): T` — last element in the span
- `size(): int` — number of elements in the span
- `toArray(): ArrayList<T>` — copy span contents to a new `ArrayList`

```titrate
let list: ArrayList<int> = new ArrayList<int>();
list.add(10);
list.add(20);
list.add(30);
list.add(40);
list.add(50);

let span: Span<int> = new Span(list, 1, 3);  // [20, 30, 40]

io::println(Integer.toString(span.get(0)));  // 20
io::println(Integer.toString(span.first())); // 20
io::println(Integer.toString(span.last()));  // 40
io::println(Integer.toString(span.size()));  // 3

span.set(1, 99);
io::println(Integer.toString(list.get(2)));  // 99 — original list is modified

let sub: Span<int> = span.subSpan(1, 2);  // [99, 40]
io::println(Integer.toString(sub.get(0)));  // 99
```

## C++ `<span>` additions (Phase 1-2 parity)

### subspan (C++ name)

`subspan` is the C++-style alias for `subSpan`. It accepts an offset and an optional count; omitting the count takes the rest of the span.

- `subspan(offset: int): Span<T>` — view from `offset` to the end
- `subspan(offset: int, count: int): Span<T>` — view of `count` elements starting at `offset`

```titrate
let tail: Span<int> = span.subspan(2);   // [40]
let mid: Span<int> = span.subspan(1, 1); // [99]
```

### as_bytes / as_writable_bytes

`as_bytes` provides a read-only byte-level view of the underlying storage, mirroring `std::as_bytes`. The writable variant is `asWritableBytes`.

- `Span.asBytes(span: Span<T>): Span<byte>` — read-only byte view (const-correct)
- `Span.asWritableBytes(span: Span<T>): Span<byte>` — writable byte view

```titrate
let ints: ArrayList<int> = new ArrayList<int>();
ints.add(0x01020304);

let s: Span<int> = new Span(ints, 0, 1);
let bytes: Span<byte> = Span.asBytes(s);
// On a little-endian host: [0x04, 0x03, 0x02, 0x01]
io::println(Integer.toString(bytes.size()));  // 4 (for a 32-bit int)
```

### Fixed-extent span<T, N>

`Span<T, N>` is a fixed-extent span where the size is known at construction time and stored in the type. This mirrors `std::span<T, N>` (extent != `std::dynamic_extent`).

- `Span<T, N>.init(list: ArrayList<T>, start: int)` — create a fixed-extent span of `N` elements starting at `start`
- `Span<T, N>.size(): int` — always returns `N`
- `Span<T, N>.get(index: int): T` — access element `index` (bounds-checked against `N`)

```titrate
let list: ArrayList<int> = new ArrayList<int>();
list.add(10); list.add(20); list.add(30); list.add(40);

let fixed: Span<int, 3> = new Span<int, 3>(list, 0);  // exactly 3 elements
io::println(Integer.toString(fixed.size()));  // 3
io::println(Integer.toString(fixed.get(2)));   // 30
```
