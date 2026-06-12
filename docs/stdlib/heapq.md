# heapq

The `tt.heapq` module provides min-heap operations on `ArrayList<int>`, mirroring Python's `heapq` module. The list is mutated in-place.

```titrate
import tt.heapq.Heapq;
```

## Heapq

All methods are static and operate on an `ArrayList<int>` that represents the heap.

- `heapify(arr: ArrayList<int>): void` — transform a list into a min-heap in-place
- `heappush(arr: ArrayList<int>, item: int): void` — push an item onto the heap
- `heappop(arr: ArrayList<int>): int` — pop and return the smallest item; throws if empty
- `heappushpop(arr: ArrayList<int>, item: int): int` — push then pop; more efficient than separate calls
- `heapreplace(arr: ArrayList<int>, item: int): int` — pop then push; more efficient than separate calls; throws if empty
- `nlargest(arr: ArrayList<int>, n: int): ArrayList<int>` — n largest elements, descending
- `nsmallest(arr: ArrayList<int>, n: int): ArrayList<int>` — n smallest elements, ascending

```titrate
let heap = new ArrayList<int>();
heap.add(5); heap.add(3); heap.add(8); heap.add(1);

Heapq.heapify(heap);
Heapq.heappush(heap, 2);

let smallest = Heapq.heappop(heap);  // 1
io::println(Integer.toString(smallest));

let top3 = Heapq.nlargest(heap, 3); // [8, 5, 3]
let low2 = Heapq.nsmallest(heap, 2); // [2, 3]
```
