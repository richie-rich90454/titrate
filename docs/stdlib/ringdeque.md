# ringdeque

The `tt.util` module provides `RingDeque<T>` — a double-ended queue backed by a ring buffer, offering true O(1) amortized operations at both ends.

```titrate
import tt.util.RingDeque;
```

## RingDeque

A double-ended queue that uses a growable ring buffer internally. Unlike an `ArrayList`-based deque, it provides genuine O(1) push and pop at both the front and back without shifting elements.

- `fn init()` — create an empty ring deque
- `pushFront(item: T): void` — add an item to the front
- `pushBack(item: T): void` — add an item to the back
- `popFront(): T` — remove and return the front item
- `popBack(): T` — remove and return the back item
- `peekFront(): T` — return the front item without removing
- `peekBack(): T` — return the back item without removing
- `size(): int` — number of elements
- `isEmpty(): bool` — check if the deque is empty
- `contains(item: T): bool` — check if the deque contains an item
- `toArray(): ArrayList<T>` — return all elements as an ArrayList (front to back)
- `clear(): void` — remove all elements

```titrate
let dq: RingDeque<int> = new RingDeque<int>();
dq.pushBack(2);
dq.pushFront(1);
dq.pushBack(3);

io::println(Integer.toString(dq.peekFront())); // 1
io::println(Integer.toString(dq.peekBack()));  // 3
io::println(Integer.toString(dq.size()));       // 3

let front: int = dq.popFront();  // 1
let back: int = dq.popBack();    // 3
io::println(Integer.toString(dq.size()));       // 1

dq.pushFront(10);
dq.pushFront(20);

let all: ArrayList<int> = dq.toArray();
// [20, 10, 2]
```
