# unionfind

The `tt.util.UnionFind` module provides a disjoint-set (union-find) data structure with path compression and union by rank.

```titrate
import tt.util.UnionFind;
```

## UnionFind

A disjoint-set data structure that tracks a partition of elements into non-overlapping subsets. Supports near-constant-time (inverse Ackermann) `find` and `union` operations.

- `fn init(n: int)` — create n singleton sets {0}, {1}, ..., {n-1}
- `find(x: int): int` — find the root representative of element x (with path compression)
- `union(x: int, y: int): void` — merge the sets containing x and y (union by rank)
- `connected(x: int, y: int): bool` — check if x and y are in the same set
- `componentSize(x: int): int` — size of the set containing x
- `componentCount(): int` — number of disjoint sets
- `setSize(x: int): int` — alias for componentSize
- `reset(): void` — reset all elements to singleton sets

```titrate
let uf: UnionFind = new UnionFind(10);
uf.union(1, 2);
uf.union(2, 3);
uf.union(5, 6);

let same: bool = uf.connected(1, 3);   // true
let diff: bool = uf.connected(1, 5);   // false
let count: int = uf.componentCount();  // 7
let size: int = uf.componentSize(1);   // 3

uf.union(3, 5);
let now: bool = uf.connected(1, 6);    // true
let newCount: int = uf.componentCount(); // 6
```
