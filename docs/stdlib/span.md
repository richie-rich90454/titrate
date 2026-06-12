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
