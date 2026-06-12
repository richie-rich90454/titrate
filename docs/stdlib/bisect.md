# bisect

The `tt.bisect` module provides binary search insertion points for sorted arrays, mirroring Python's `bisect` module. Useful for maintaining sorted order while inserting elements.

```titrate
import tt.bisect.Bisect;
```

## Bisect

All methods are static. The `bisectLeft`/`bisectRight` functions return the index where an element should be inserted; the `insortLeft`/`insortRight` functions perform the insertion.

### Integer operations

- `bisectLeft(arr: ArrayList<int>, x: int): int` — leftmost insertion point to keep arr sorted
- `bisectRight(arr: ArrayList<int>, x: int): int` — rightmost insertion point to keep arr sorted
- `insortLeft(arr: ArrayList<int>, x: int): void` — insert x at the leftmost position
- `insortRight(arr: ArrayList<int>, x: int): void` — insert x at the rightmost position

### Double operations

- `bisectLeftD(arr: ArrayList<double>, x: double): int` — leftmost insertion point for doubles
- `bisectRightD(arr: ArrayList<double>, x: double): int` — rightmost insertion point for doubles

```titrate
let arr = new ArrayList<int>();
arr.add(1); arr.add(3); arr.add(5); arr.add(7);

let idx = Bisect.bisectLeft(arr, 4);   // 2
Bisect.insortRight(arr, 4);            // [1, 3, 4, 5, 7]

let pos = Bisect.bisectRight(arr, 3);  // 2 (after existing 3)
```
