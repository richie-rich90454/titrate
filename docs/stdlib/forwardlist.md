# forwardlist

The `tt.util` module provides `ForwardList<T>` — a singly-linked list optimized for fast insertion and removal at the front.

```titrate
import tt.util.ForwardList;
import tt.util.ForwardNode;
```

## ForwardList

A singly-linked list that supports O(1) insertion and removal at the front. Unlike `ArrayList`, it does not support random access by index, but uses less memory for frequent front insertions.

- `fn init()` — create an empty list
- `pushFront(value: T): void` — insert at the front
- `popFront(): T` — remove and return the front element
- `front(): T` — get the front element without removing
- `insertAfter(node: ForwardNode<T>, value: T): ForwardNode<T>` — insert after the given node; returns the new node
- `removeAfter(node: ForwardNode<T>): T` — remove the node after the given node; returns the removed value
- `isEmpty(): bool` — check if the list is empty
- `size(): int` — number of elements
- `reverse(): void` — reverse the list in place
- `contains(value: T): bool` — check if the list contains a value
- `iterator(): ForwardNode<T>` — get the head node for manual traversal

```titrate
let list: ForwardList<int> = new ForwardList<int>();
list.pushFront(3);
list.pushFront(2);
list.pushFront(1);

io::println(Integer.toString(list.front())); // 1
io::println(Integer.toString(list.size()));  // 3

let val: int = list.popFront();
io::println(Integer.toString(val));          // 1

list.reverse();
io::println(Integer.toString(list.front())); // 3

// Manual traversal
var node: ForwardNode<int> = list.iterator();
while (node != null) {
    io::println(Integer.toString(node.value));
    node = node.next;
}
```
