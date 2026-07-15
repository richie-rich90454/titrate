# Graphlib

The `tt.algo.TopologicalSorter` module provides topological sorting of directed acyclic graphs with optional predecessor tracking. It mirrors Python's `graphlib.TopologicalSorter`, exposing the generic `TopologicalSorter<T>` class with `addNode`, `done`, `prepare`, `get_ready`, `static_order`, `isActive`, plus `predecessorsOf`/`successorsOf` introspection. Module-level helpers `topologicalSorter<T>()` and `topologicalSorterFromEdges<T>(edges)` mirror Python's constructor overloads.

## Import

```titrate
import tt::algo::TopologicalSorter;
```

## Classes

### `TopologicalSorter<T>`

Implements Python's `graphlib.TopologicalSorter` API. Nodes may be added with explicit predecessors; the sorter then yields nodes in dependency order via `get_ready()` and `static_order()`.

**Constructors:**
- `init()` — creates an empty sorter

**Methods:**
- `addNode(node: T, predecessors: ArrayList<T>): void` — add `node` with the given predecessor list; an empty list adds a node with no dependencies. Throws if called after `prepare()`.
- `addNodeOne(node: T, predecessor: T): void` — convenience: add `node` with a single predecessor
- `done(node: T): void` — mark `node` as processed. The node must have been returned by `get_ready()`. Once done, its successors have their pending predecessor count decremented. Throws if `node` is unknown or not in the READY state.
- `prepare(): void` — mark the sorter as ready; no more `addNode()` calls are allowed
- `get_ready(): ArrayList<T>` — return the list of nodes whose predecessors are all `done()`. Each returned node is marked READY; the caller must eventually call `done()` on it. Returns an empty list when no more nodes are ready.
- `static_order(): ArrayList<T>` — process the entire sorter to completion and return all nodes in topological order. Calls `prepare()`, then repeatedly `get_ready()` / `done()` until every node is processed.
- `isActive(): bool` — true if there are still nodes returned by `get_ready()` but not yet marked `done()`
- `nodeCount(): int` — number of nodes known to the sorter
- `predecessorsOf(node: T): ArrayList<T>` — return a defensive copy of the direct predecessors of `node`
- `successorsOf(node: T): ArrayList<T>` — return a defensive copy of the direct successors of `node`

```titrate
let sorter: TopologicalSorter<string> = new TopologicalSorter<string>();
let predsD = new ArrayList<string>();
predsD.add("a"); predsD.add("b"); predsD.add("c");
sorter.addNode("d", predsD);
sorter.addNode("c", new ArrayList<string>(["b"]));
sorter.addNode("b", new ArrayList<string>(["a"]));
sorter.addNode("a", new ArrayList<string>());
let order: ArrayList<string> = sorter.static_order();
// order: ["a", "b", "c", "d"]
```

## Functions

### topologicalSorter

- `TopologicalSorter.topologicalSorter<T>(): TopologicalSorter<T>` — construct an empty sorter (equivalent to `new TopologicalSorter<T>()`)

### topologicalSorterFromEdges

- `TopologicalSorter.topologicalSorterFromEdges<T>(edges: ArrayList<(T, T)>): TopologicalSorter<T>` — construct a sorter pre-populated from a list of `(node, predecessor)` pairs; each pair contributes one predecessor edge

```titrate
let edges = new ArrayList<(string, string)>();
edges.add(("d", "a"));
edges.add(("d", "b"));
edges.add(("c", "b"));
let sorter = TopologicalSorter.topologicalSorterFromEdges<string>(edges);
let order = sorter.static_order();
```

## Usage Example

```titrate
import tt::algo::TopologicalSorter;
import tt::util::ArrayList;

public fn main(): void {
    let sorter: TopologicalSorter<string> = new TopologicalSorter<string>();
    sorter.addNode("build", new ArrayList<string>(["compile"]));
    sorter.addNode("compile", new ArrayList<string>(["codegen"]));
    sorter.addNode("codegen", new ArrayList<string>(["parse"]));
    sorter.addNode("parse", new ArrayList<string>());
    let order: ArrayList<string> = sorter.static_order();
    var i: int = 0;
    while (i < order.size()) {
        io::println(order.get(i));
        i = i + 1;
    }
}
```
